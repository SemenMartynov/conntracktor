use std::os::raw::c_char;

// -----------------------------------------------------------------------------
// Constants & Definitions for the Linux Kernel ioctl API
// -----------------------------------------------------------------------------

/// The standard ioctl command code for communicating with network device drivers.
const SIOCETHTOOL: u64 = 0x8946;

// Modern ethtool API commands (GFEATURES dynamic bitmask matching).
const ETHTOOL_GSSET_INFO: u32 = 0x00000037;
const ETHTOOL_GSTRINGS: u32 = 0x0000001B;
const ETHTOOL_GFEATURES: u32 = 0x0000003A;

/// ID for the string set containing network feature names.
const ETH_SS_FEATURES: u32 = 4;

/// Defines the specific Network Interface Controller (NIC) features we want to query.
pub enum NicFeature {
    Rss,
    RxChecksum,
    TxChecksum,
    Tso,
}

// -----------------------------------------------------------------------------
// RAII Socket Wrapper
// -----------------------------------------------------------------------------

/// RAII wrapper for an unbound socket file descriptor.
///
/// Creating an unbound `AF_INET` socket allocates a kernel socket descriptor without
/// binding to any network interface or ephemeral port. This satisfies the requirements
/// for `SIOCETHTOOL` ioctls with zero network exposure and minimal OS overhead.
struct RawSocket(i32);

impl RawSocket {
    /// Allocates an unbound IPv4 datagram socket descriptor.
    fn new() -> Option<Self> {
        let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        if fd < 0 { None } else { Some(RawSocket(fd)) }
    }

    /// Returns the underlying raw file descriptor.
    fn fd(&self) -> i32 {
        self.0
    }
}

impl Drop for RawSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

// -----------------------------------------------------------------------------
// C-Compatible Data Structures for Kernel Communication
// -----------------------------------------------------------------------------

/// Struct used to request the size (number of elements) of a specific string set.
#[repr(C)]
struct EthtoolSsetInfo {
    cmd: u32,
    reserved: u32,
    sset_mask: u64,
    // The kernel will write the count of features into this array.
    data: [u32; 1],
}

/// Struct used to fetch the actual string names of supported NIC features.
#[repr(C)]
struct EthtoolGstrings {
    cmd: u32,
    string_set: u32,
    len: u32,
    // The actual string data (array of 32-byte strings) is dynamically appended
    // in memory directly after this struct by the kernel.
}

/// Struct used to fetch the active state of all NIC features via bitmasks.
#[repr(C)]
struct EthtoolGfeatures {
    cmd: u32,
    size: u32, // Number of 32-bit blocks
               // The bitmask blocks (array of EthtoolGetFeaturesBlock) are dynamically appended
               // in memory directly after this struct by the kernel.
}

/// Represents a single 32-bit block of feature flags.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct EthtoolGetFeaturesBlock {
    available: u32,
    requested: u32,
    active: u32,
    never_changed: u32,
}

/// Minimal version of the `struct ifreq` from `<net/if.h>` used for standard network ioctls.
#[repr(C)]
struct Ifreq {
    ifr_name: [c_char; 16],
    ifr_data: *mut libc::c_void,
    // We provide enough padding to match the largest member of the original C union (struct ifmap).
    // This prevents memory corruption and ensures cross-architecture compatibility.
    _pad: [u8; 16],
}

// -----------------------------------------------------------------------------
// Implementation
// -----------------------------------------------------------------------------

/// Safely copies a Rust string slice into a fixed-size, null-terminated C character array.
fn copy_if_name(if_name: &str, dest: &mut [c_char; 16]) {
    let bytes = if_name.as_bytes();
    let copy_len = bytes.len().min(15); // Ensure space for the null terminator.

    for i in 0..copy_len {
        dest[i] = bytes[i] as c_char;
    }
    // Explicitly zero-out the remaining space to prevent passing uninitialized memory to the kernel.
    for i in copy_len..16 {
        dest[i] = 0;
    }
}

/// Main entry point to query hardware capabilities directly from the Linux kernel.
///
/// This utilizes the modern `ETHTOOL_GFEATURES` API, which dynamically resolves
/// feature names to bitmask indices, ensuring compatibility with all recent kernel versions.
pub fn check_feature(if_name: &str, feature: NicFeature) -> bool {
    // Acquire an unbound raw socket file descriptor required for SIOCETHTOOL ioctls.
    let Some(sock) = RawSocket::new() else {
        return false;
    };
    let fd = sock.fd();

    match feature {
        NicFeature::Rss => {
            // Feature naming can occasionally vary across driver versions or kernel branches.
            // We check for both known variants of the Receive Side Scaling (RSS) feature.
            check_modern_feature(fd, if_name, "rx-hashing")
                || check_modern_feature(fd, if_name, "receive-hashing")
        }

        NicFeature::RxChecksum => check_modern_feature(fd, if_name, "rx-checksum"),
        NicFeature::TxChecksum => {
            check_modern_feature(fd, if_name, "tx-checksum-ip-generic")
                || check_modern_feature(fd, if_name, "tx-checksum-ipv4")
                || check_modern_feature(fd, if_name, "tx-checksum-ipv6")
        }
        NicFeature::Tso => check_modern_feature(fd, if_name, "tx-tcp-segmentation"),
    }
}

/// Dynamically resolves and queries features exactly like the `ethtool` userspace utility does.
///
/// The execution pipeline consists of 5 distinct phases (utilizing 3 kernel `ioctl` calls):
/// - Phase 1 (ioctl): Request the count of features supported by the network driver.
/// - Phase 2 (ioctl): Fetch the array of feature string names into a dynamic buffer.
/// - Phase 3 (CPU):   Scan the string array to resolve the target feature's bit index.
/// - Phase 4 (ioctl): Fetch the active feature state bitmasks from the kernel.
/// - Phase 5 (CPU):   Perform a bitwise check to verify if the feature bit is active.
fn check_modern_feature(fd: i32, if_name: &str, feature_name: &str) -> bool {
    // -------------------------------------------------------------------------
    // Phase 1: Request the total number of features the driver currently tracks.
    // -------------------------------------------------------------------------
    let mut sset_info = EthtoolSsetInfo {
        cmd: ETHTOOL_GSSET_INFO,
        reserved: 0,
        sset_mask: 1 << ETH_SS_FEATURES,
        data: [0],
    };

    let mut req: Ifreq = unsafe { std::mem::zeroed() };
    copy_if_name(if_name, &mut req.ifr_name);
    req.ifr_data = &mut sset_info as *mut _ as *mut libc::c_void;

    // The `as _` cast handles pointer sizing differences between 32-bit (c_int) and 64-bit (c_ulong) systems.
    if unsafe { libc::ioctl(fd, SIOCETHTOOL as _, &mut req) } != 0 {
        return false;
    }

    let feature_count = sset_info.data[0] as usize;
    if feature_count == 0 {
        return false;
    }

    // -------------------------------------------------------------------------
    // Phase 2: Allocate a dynamic buffer and fetch all feature string names.
    // -------------------------------------------------------------------------
    // Each feature name is represented as a strictly 32-byte long character array.
    let gstrings_size = std::mem::size_of::<EthtoolGstrings>() + feature_count * 32;
    let mut gstrings_buf = vec![0u8; gstrings_size];

    let gstrings = unsafe { &mut *(gstrings_buf.as_mut_ptr() as *mut EthtoolGstrings) };
    gstrings.cmd = ETHTOOL_GSTRINGS;
    gstrings.string_set = ETH_SS_FEATURES;
    gstrings.len = feature_count as u32;

    req.ifr_data = gstrings_buf.as_mut_ptr() as *mut libc::c_void;
    if unsafe { libc::ioctl(fd, SIOCETHTOOL as _, &mut req) } != 0 {
        return false;
    }

    // -------------------------------------------------------------------------
    // Phase 3: Scan the returned strings to find the bit index of our requested feature.
    // -------------------------------------------------------------------------
    let mut feature_idx = None;
    let strings_ptr = unsafe {
        gstrings_buf
            .as_ptr()
            .add(std::mem::size_of::<EthtoolGstrings>())
    };

    for i in 0..feature_count {
        // Calculate the exact memory address for the i-th string.
        let str_ptr = unsafe { strings_ptr.add(i * 32) };
        let slice = unsafe { std::slice::from_raw_parts(str_ptr, 32) };

        // Locate the null-terminator. Ethtool strings are padded with trailing null bytes.
        let name_len = slice.iter().position(|&c| c == 0).unwrap_or(32);

        if let Ok(name) = std::str::from_utf8(&slice[..name_len]) {
            if name == feature_name {
                feature_idx = Some(i);
                break; // Found our feature, we can stop scanning.
            }
        }
    }

    // If the driver does not expose this feature, we consider it disabled.
    let Some(idx) = feature_idx else {
        return false;
    };

    // -------------------------------------------------------------------------
    // Phase 4: Fetch the feature bitmask blocks.
    // -------------------------------------------------------------------------
    // Features are packed into 32-bit blocks. We calculate how many blocks we need.
    let blocks_count = (feature_count + 31) / 32;
    let gfeatures_size = std::mem::size_of::<EthtoolGfeatures>()
        + blocks_count * std::mem::size_of::<EthtoolGetFeaturesBlock>();

    let mut gfeatures_buf = vec![0u8; gfeatures_size];
    let gfeatures = unsafe { &mut *(gfeatures_buf.as_mut_ptr() as *mut EthtoolGfeatures) };
    gfeatures.cmd = ETHTOOL_GFEATURES;
    gfeatures.size = blocks_count as u32;

    req.ifr_data = gfeatures_buf.as_mut_ptr() as *mut libc::c_void;
    if unsafe { libc::ioctl(fd, SIOCETHTOOL as _, &mut req) } != 0 {
        return false;
    }

    // -------------------------------------------------------------------------
    // Phase 5: Verify if the bit corresponding to our feature index is set to active.
    // -------------------------------------------------------------------------
    let blocks_ptr = unsafe {
        gfeatures_buf
            .as_ptr()
            .add(std::mem::size_of::<EthtoolGfeatures>()) as *const EthtoolGetFeaturesBlock
    };

    // Jump to the correct 32-bit block.
    let block = unsafe { &*blocks_ptr.add(idx / 32) };

    // Extract the exact bit representing our feature.
    (block.active & (1 << (idx % 32))) != 0
}
