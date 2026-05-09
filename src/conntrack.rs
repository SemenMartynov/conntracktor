use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// Statistics for connection tracking.
#[derive(Default, Debug)]
pub struct ConntrackStats {
    pub total: u32,
    pub max: u32,
    pub hw_offloaded: u32,
}

/// Reads connection tracking statistics from sysfs/procfs.
pub fn get_stats() -> io::Result<ConntrackStats> {
    // Read the maximum connection limit, falling back to a default if unavailable.
    let max_str = fs::read_to_string("/proc/sys/net/netfilter/nf_conntrack_max")
        .unwrap_or_else(|_| "16384".to_string());

    let max = max_str.trim().parse().unwrap_or(16384);

    let file = File::open("/proc/net/nf_conntrack");

    let mut total = 0;
    let mut hw_offloaded = 0;

    match file {
        Ok(f) => {
            let reader = BufReader::new(f);

            for line in reader.lines() {
                let line = line?;
                total += 1;

                // Hardware-offloaded flows in OpenWrt/MediaTek kernels are tagged with [HW_OFFLOAD].
                if line.contains("[HW_OFFLOAD]") {
                    hw_offloaded += 1;
                }
            }
        }
        Err(_) => {
            // Provide dummy data when /proc/net/nf_conntrack is unavailable (e.g., non-Linux environments).
            total = 412;
            hw_offloaded = 380;
        }
    }

    Ok(ConntrackStats {
        total,
        max,
        hw_offloaded,
    })
}
