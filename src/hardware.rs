use crate::soc::SocModel;
use std::fs;
use std::path::Path;
use sysinfo::{CpuRefreshKind, Networks, RefreshKind, System};

/// Hardware capabilities and platform feature flags.
#[derive(Debug, Default, Clone)]
pub struct HardwareInfo {
    // SoC & CPU Raw Data
    pub soc: SocModel,
    pub cpu_cores: usize,
    pub cpu_arch: String,
    pub cpu_freq_mhz: Option<u64>,

    // Memory
    pub memory_mb: Option<u64>,
    pub memory_freq_mhz: Option<u64>,

    // Storage & Peripheral Raw Data
    pub switch: String,
    pub flash: String,
    pub misc: String,

    // Networking Features
    pub ppe_count: usize,
    pub wed_count: usize,
    pub rss: bool,
    pub checksum_offload: bool,
    pub tso: bool,
    pub multi_rx: bool,

    // Security Features
    pub crypto_engine: bool,
    pub aes: bool,
    pub sha: bool,
    pub trng: bool,
    pub secure_boot: bool,
    pub trustzone: bool,
}

impl HardwareInfo {
    /// Detects hardware components and system capabilities dynamically.
    pub fn detect() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );
        let networks = Networks::new_with_refreshed_list();

        // Detect standard hardware baseline
        let soc = SocModel::detect();
        let main_iface = Self::get_main_interface(&networks);
        let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

        Self {
            // Core Components: Storing raw data instead of pre-formatted strings
            soc,
            cpu_cores: Self::detect_cpu_cores(&sys, &cpuinfo),
            cpu_arch: Self::detect_cpu_arch(&sys, &cpuinfo, &soc),
            cpu_freq_mhz: Self::detect_cpu_frequency_mhz(),

            // Memory
            memory_mb: Self::detect_memory_mb(),
            memory_freq_mhz: Self::detect_memory_freq_mhz(),

            // Storage & Peripherals
            switch: Self::detect_switch(),
            flash: Self::detect_flash(),
            misc: Self::detect_misc(),

            // Dynamic Networking Feature Checks combined with standard SoC limits
            ppe_count: soc.ppe_count().max(Self::count_platform_devices("ppe")),
            wed_count: soc.wed_count().max(Self::count_platform_devices("wed")),
            rss: Self::check_nic_feature(&main_iface, "rx-hashing")
                || Self::check_nic_feature(&main_iface, "rx-vlan-hw-parse"),
            checksum_offload: Self::check_nic_feature(&main_iface, "tx-checksum")
                || Self::check_nic_feature(&main_iface, "rx-checksum"),
            tso: Self::check_nic_feature(&main_iface, "tcp-segmentation-offload"),
            multi_rx: Self::check_multi_rx(&main_iface),

            // Dynamic Security Feature Checks
            crypto_engine: Self::count_platform_devices("crypto") > 0
                || Self::count_platform_devices("eip") > 0,
            aes: Self::check_crypto_algo("aes"),
            sha: Self::check_crypto_algo("sha"),
            trng: Self::check_trng(),
            secure_boot: Self::check_secure_boot(),
            trustzone: Path::new("/dev/tee0").exists() || Path::new("/sys/class/tee/tee0").exists(),
        }
    }

    // ========================================================================
    // General System Display
    // ========================================================================

    /// Returns a human-readable string for the System on Chip (SoC) model.
    pub fn display_soc(&self) -> String {
        self.soc.display_name().to_string()
    }

    /// Assembles CPU information into a human-readable string.
    /// Appends a red exclamation mark (❗) if an overclock/underclock is detected.
    pub fn display_cpu(&self) -> String {
        let mut output = if self.cpu_arch.contains('×') {
            self.cpu_arch.clone() // Prevents "4× 4× Cortex-A73"
        } else {
            format!("{}× {}", self.cpu_cores, self.cpu_arch)
        };

        if let Some(freq) = self.cpu_freq_mhz {
            output.push_str(&format!(" @ {} MHz", freq));

            let expected_mhz = self.soc.core_frequency_hz() / 1_000_000;
            // Modified status is calculated dynamically during presentation
            let is_modified = expected_mhz > 0 && freq != expected_mhz;

            if is_modified {
                output.push_str(" ❗");
            }
        }

        output
    }

    /// Assembles RAM information into a human-readable string.
    pub fn display_ram(&self) -> String {
        match (self.memory_mb, self.memory_freq_mhz) {
            (Some(mb), Some(freq)) => format!("{mb} MB @ {freq} MHz"),
            (Some(mb), None) => format!("{mb} MB"),
            (None, _) => String::new(),
        }
    }

    // ========================================================================
    // Networking Features Display
    // ========================================================================

    /// Returns the number of Packet Processing Engines (PPE) with a warning if it deviates from SoC spec.
    pub fn display_ppe_count(&self) -> String {
        Self::format_value(self.ppe_count, self.soc.ppe_count())
    }

    /// Returns the number of Wireless Ethernet Dispatchers (WED) with a warning if it deviates from SoC spec.
    pub fn display_wed_count(&self) -> String {
        Self::format_value(self.wed_count, self.soc.wed_count())
    }

    /// Returns whether Receive Side Scaling (RSS) is active/supported.
    pub fn display_rss(&self) -> String {
        Self::format_bool(self.rss, self.soc.has_rss())
    }

    /// Returns whether Checksum Offload is active/supported.
    pub fn display_checksum_offload(&self) -> String {
        Self::format_bool(self.checksum_offload, self.soc.has_checksum_offload())
    }

    /// Returns whether TCP Segmentation Offload (TSO) is active/supported.
    pub fn display_tso(&self) -> String {
        Self::format_bool(self.tso, self.soc.has_tso())
    }

    /// Returns whether multi-queue RX is active/supported.
    pub fn display_multi_rx(&self) -> String {
        Self::format_bool(self.multi_rx, self.soc.has_multi_rx_queues())
    }

    // ========================================================================
    // Security Features Display
    // ========================================================================

    /// Returns whether a cryptographic engine is detected.
    pub fn display_crypto_engine(&self) -> String {
        Self::format_bool(self.crypto_engine, self.soc.has_crypto_engine())
    }

    /// Returns whether AES hardware acceleration is detected.
    pub fn display_aes(&self) -> String {
        Self::format_bool(self.aes, self.soc.has_aes_acceleration())
    }

    /// Returns whether SHA hardware acceleration is detected.
    pub fn display_sha(&self) -> String {
        Self::format_bool(self.sha, self.soc.has_sha_acceleration())
    }

    /// Returns whether a hardware True Random Number Generator (TRNG) is detected.
    pub fn display_trng(&self) -> String {
        Self::format_bool(self.trng, self.soc.has_trng())
    }

    /// Returns whether Secure Boot is enabled.
    pub fn display_secure_boot(&self) -> String {
        Self::format_bool(self.secure_boot, self.soc.has_secure_boot())
    }

    /// Returns whether ARM TrustZone (TEE) is available.
    pub fn display_trustzone(&self) -> String {
        Self::format_bool(self.trustzone, self.soc.has_trustzone())
    }

    // ========================================================================
    // Internal Presentation Helpers
    // ========================================================================

    /// Formats a generic value for the UI, flagging unexpected values with a red exclamation mark.
    fn format_value<T: PartialEq + std::fmt::Display>(actual: T, expected: T) -> String {
        if actual != expected {
            format!("{actual} ❗")
        } else {
            actual.to_string()
        }
    }

    /// Formats a boolean capability for the UI ("✓" / "✗"), flagging unexpected values.
    fn format_bool(actual: bool, expected: bool) -> String {
        Self::format_value(
            if actual { "✓" } else { "✗" },
            if expected { "✓" } else { "✗" },
        )
    }

    // ========================================================================
    // Internal Hardware Detection Routines (Data Gathering)
    // ========================================================================

    /// Detects the physical number of CPU cores.
    fn detect_cpu_cores(sys: &System, cpuinfo: &str) -> usize {
        match sys.cpus().len() {
            0 => cpuinfo
                .lines()
                .filter(|l| l.starts_with("processor"))
                .count()
                .max(1),
            cores => cores,
        }
    }

    /// Detects CPU microarchitecture, falling back to SoC static properties.
    fn detect_cpu_arch(sys: &System, cpuinfo: &str, soc: &SocModel) -> String {
        sys.cpus()
            .first()
            .map(|cpu| cpu.brand().trim())
            .filter(|brand| !brand.is_empty() && !brand.to_lowercase().contains("unknown"))
            .map(String::from)
            .or_else(|| Self::parse_cpu_architecture_from_cpuinfo(cpuinfo))
            .unwrap_or_else(|| soc.architecture().to_string())
    }

    /// Fast frequency detection pipeline prioritizing Common Clock Framework (CCF).
    fn detect_cpu_frequency_mhz() -> Option<u64> {
        const TARGET_CLOCKS: &[&str] = &["armpll", "mcu_armpll", "cpu_clk", "cpu"];
        const MIN_VALID_CPU_FREQ_MHZ: u64 = 100;

        // O(1) Fast lookup via CCF debugfs clk_rate
        for clk_name in TARGET_CLOCKS {
            let path = format!("/sys/kernel/debug/clk/{}/clk_rate", clk_name);
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(hz) = content.trim().parse::<u64>() {
                    let mhz = hz / 1_000_000;
                    if mhz >= MIN_VALID_CPU_FREQ_MHZ {
                        return Some(mhz);
                    }
                }
            }
        }

        // Fallback to parsing the full clk_summary table O(N)
        if let Ok(content) = fs::read_to_string("/sys/kernel/debug/clk/clk_summary") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 && TARGET_CLOCKS.contains(&parts[0]) {
                    if let Ok(hz) = parts[4].parse::<u64>() {
                        let mhz = hz / 1_000_000;
                        if mhz >= MIN_VALID_CPU_FREQ_MHZ {
                            return Some(mhz);
                        }
                    }
                }
            }
        }

        None // Frequency is unknown dynamically
    }

    /// Helper method to identify CPU microarchitecture from /proc/cpuinfo ARM PART codes.
    fn parse_cpu_architecture_from_cpuinfo(cpuinfo: &str) -> Option<String> {
        let known_parts = [
            ("CPU part\t: 0xd03", "Cortex-A53"),
            ("CPU part\t: 0xd08", "Cortex-A73"),
            ("CPU part\t: 0xd0b", "Cortex-A76"),
            ("MIPS", "MIPS"),
        ];

        for (pattern, arch) in known_parts {
            if cpuinfo.contains(pattern) {
                return Some(arch.to_string());
            }
        }

        cpuinfo
            .lines()
            .find(|l| l.starts_with("model name") || l.starts_with("Hardware"))
            .and_then(|line| line.split_once(':'))
            .map(|(_, name)| name.trim())
            .filter(|name| !name.is_empty() && !name.to_lowercase().contains("unknown"))
            .map(String::from)
    }

    /// Detects system RAM capacity in Megabytes (MB).
    fn detect_memory_mb() -> Option<u64> {
        let meminfo = fs::read_to_string("/proc/meminfo").ok()?;

        meminfo
            .lines()
            .find(|line| line.starts_with("MemTotal:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|kb_str| kb_str.parse::<u64>().ok())
            .map(|kb| kb / 1024)
    }

    /// Detects system RAM frequency in MHz.
    fn detect_memory_freq_mhz() -> Option<u64> {
        // TODO: Implement dynamic RAM frequency detection via sysfs / devicetree
        None
    }

    /// Detects onboard flash storage size using `sysinfo`, falling back to manual sysfs.
    fn detect_flash() -> String {
        "[Placeholder]".to_string()
    }

    /// Detects integrated or discrete Ethernet switch chips present on the board.
    fn detect_switch() -> String {
        "[Placeholder]".to_string()
    }

    /// Detects peripheral capabilities and auxiliary hardware interfaces.
    fn detect_misc() -> String {
        "[Placeholder]".to_string()
    }

    fn count_platform_devices(keyword: &str) -> usize {
        let count_in_dir = |path: &str| -> usize {
            let mut found = 0;
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_lowercase();
                    // Example: "15100000.wed" or "mtk_wed"
                    if file_name.contains(keyword) {
                        found += 1;
                    }
                }
            }
            found
        };

        // Platform devices usually reside in the platform bus, but checking
        // devicetree soc node helps for hardware that hasn't bound a driver yet.
        let mut count = count_in_dir("/sys/bus/platform/devices");

        // If we found drivers, return. Otherwise fallback to devicetree topology count.
        if count == 0 {
            count = count_in_dir("/sys/firmware/devicetree/base/soc");
        }

        count
    }

    /// Identifies the primary physical network interface using `sysinfo` first.
    fn get_main_interface(networks: &Networks) -> Option<String> {
        // Try sysinfo
        for (name, _) in networks {
            if name.starts_with("eth") || name.starts_with("lan") {
                return Some(name.clone());
            }
        }

        // Fallback to manual sysfs
        ["eth0", "eth1", "lan0", "lan1"]
            .iter()
            .find(|&&opt| Path::new(&format!("/sys/class/net/{}", opt)).exists())
            .map(|s| s.to_string())
    }

    /// Placeholder: Verifies hardware offload status for a given network feature.
    fn check_nic_feature(_iface: &Option<String>, _feature: &str) -> bool {
        false
    }

    /// Checks whether multi-queue RX ring buffers are present for the primary interface.
    fn check_multi_rx(iface: &Option<String>) -> bool {
        if let Some(if_name) = iface {
            Path::new(&format!("/sys/class/net/{}/queues/rx-1", if_name)).exists()
        } else {
            false
        }
    }

    /// Inspects `/proc/crypto` for vendor-specific hardware acceleration drivers.
    fn check_crypto_algo(algo: &str) -> bool {
        if let Ok(content) = fs::read_to_string("/proc/crypto") {
            let mut is_target_algo = false;
            for line in content.lines() {
                if line.starts_with("name") && line.contains(algo) {
                    is_target_algo = true;
                } else if line.starts_with("driver") && is_target_algo {
                    let driver = line.split(':').nth(1).unwrap_or("").trim();
                    // Match vendor hardware drivers (mtk, safexcel, eip), excluding software implementations
                    if driver.contains("mtk")
                        || driver.contains("safexcel")
                        || driver.contains("eip")
                    {
                        return true;
                    }
                    is_target_algo = false;
                } else if line.is_empty() {
                    is_target_algo = false;
                }
            }
        }
        false
    }

    /// Checks for the presence of a hardware random number generator (TRNG).
    fn check_trng() -> bool {
        if let Ok(content) = fs::read_to_string("/sys/class/misc/hw_random/rng_available") {
            return content.contains("mtk") || content.contains("safexcel");
        }
        Path::new("/dev/hwrng").exists()
    }

    /// Checks for secure boot indicators in sysfs or device tree nodes.
    fn check_secure_boot() -> bool {
        Path::new("/sys/kernel/security/securelevel").exists()
            || Path::new("/sys/firmware/devicetree/base/firmware/secure-boot").exists()
    }
}
