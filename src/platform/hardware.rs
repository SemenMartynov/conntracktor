use super::SocModel;
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

    /// Assembles RAM information into a human-readable string.
    pub fn display_ram(&self) -> String {
        match (self.memory_mb, self.memory_freq_mhz) {
            (Some(mb), Some(freq)) => format!("{mb} MB @ {freq} MHz"),
            (Some(mb), None) => format!("{mb} MB"),
            (None, _) => String::new(),
        }
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

    /// Detects CPU microarchitecture, utilizing sysinfo, SoC knowledge base, and generic sysfs fallback.
    fn detect_cpu_arch(sys: &System, cpuinfo: &str, soc: &SocModel) -> String {
        sys.cpus()
            .first()
            .map(|cpu| cpu.brand().trim())
            .filter(|brand| !brand.is_empty() && !brand.to_lowercase().contains("unknown"))
            .map(String::from)
            // Delegate domain-specific hardware part matching to the SoC module
            .or_else(|| SocModel::parse_cpu_arch_from_cpuinfo(cpuinfo).map(String::from))
            // Fallback to generic Linux cpuinfo string extraction
            .or_else(|| Self::parse_generic_cpu_model(cpuinfo))
            // Ultimate fallback to static SoC properties
            .unwrap_or_else(|| soc.architecture().to_string())
    }

    /// Helper method to extract a generic CPU model name from `/proc/cpuinfo`.
    fn parse_generic_cpu_model(cpuinfo: &str) -> Option<String> {
        cpuinfo
            .lines()
            .find(|l| l.starts_with("model name") || l.starts_with("Hardware"))
            .and_then(|line| line.split_once(':'))
            .map(|(_, name)| name.trim())
            .filter(|name| !name.is_empty() && !name.to_lowercase().contains("unknown"))
            .map(String::from)
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
                if !parts.is_empty() && TARGET_CLOCKS.contains(&parts[0]) {
                    // Find the first number in the line (skipping the clock name) that
                    // resembles a valid frequency (i.e., >= 100 MHz / 100_000_000 Hz).
                    // This makes the parsing robust against column shifts across different Linux kernel versions.
                    if let Some(hz) = parts
                        .iter()
                        .skip(1)
                        .filter_map(|s| s.parse::<u64>().ok())
                        .find(|&val| val >= 100_000_000)
                    {
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
                    let file_name_str = file_name.as_str();

                    // Extract the base device name by stripping out memory addresses.
                    let core_name = if let Some((name, _addr)) = file_name_str.split_once('@') {
                        // Devicetree format: "wed@15010000" -> "wed"
                        name
                    } else if let Some((addr, name)) = file_name_str.split_once('.') {
                        // Platform bus format: "15010000.wed" -> "wed"
                        // Ensure the prefix consists only of hex digits to prevent false positives.
                        if addr.chars().all(|c| c.is_ascii_hexdigit()) {
                            name
                        } else {
                            file_name_str
                        }
                    } else {
                        // Legacy drivers fallback (e.g., "mtk_wed")
                        file_name_str
                    };

                    // Strip the vendor prefix (if present).
                    let trimmed = core_name.trim_start_matches("mtk_");

                    // Strict match verification.
                    // Ensure the string starts with the target keyword (e.g., "wed").
                    if trimmed.starts_with(keyword) {
                        let remainder = &trimmed[keyword.len()..];
                        // The remainder of the string must either be empty ("wed") or consist solely of digits ("wed0", "ppe1").
                        // This effectively filters out false matches like "wed_pcie".
                        if remainder.is_empty() || remainder.chars().all(|c| c.is_ascii_digit()) {
                            found += 1;
                        }
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
