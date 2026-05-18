use std::fs;
use std::path::Path;
use std::process::Command;

/// Status of available software and hardware packet acceleration engines.
#[derive(Debug, Clone, Default)]
pub struct AccelerationStatus {
    /// Indicates whether software flow offloading is active.
    pub software: bool,
    /// Indicates whether the Packet Processing Engine (PPE) hardware accelerator is active.
    pub hw_ppe: bool,
    /// Indicates whether the Wireless Ethernet Dispatcher (WED) is active.
    pub hw_wed: bool,
}

impl AccelerationStatus {
    /// Checks the system state by inspecting nftables configuration and the current debugfs structure.
    pub fn check_system() -> Self {
        let (sw_active, hw_flag_present) = Self::check_nftables_flowtable();

        Self {
            software: sw_active,
            // PPE requires software flow offloading to be enabled, the hardware offload flag
            // present in the flowtable, and the PPE driver to be initialized in the kernel.
            hw_ppe: sw_active && hw_flag_present && Self::is_ppe_driver_loaded(),
            hw_wed: Self::check_wed(),
        }
    }

    /// Verifies the configuration of the nftables `fw4` flowtable (`ft`).
    /// Returns a tuple: `(is_flowtable_created, is_hardware_offload_flag_set)`.
    fn check_nftables_flowtable() -> (bool, bool) {
        if let Ok(out) = Command::new("nft")
            .args(["list", "flowtable", "inet", "fw4", "ft"])
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // The `flags offload;` parameter instructs the kernel to push traffic to the hardware PPE.
                let has_hw_flag = stdout.contains("flags offload");

                return (true, has_hw_flag);
            }
        }
        (false, false)
    }

    /// Checks if the MediaTek PPE driver is initialized in the kernel.
    fn is_ppe_driver_loaded() -> bool {
        // The `mtk_eth_soc` driver manages both Ethernet and PPE on MediaTek SoCs
        // (from older chips like MT7621 up to modern ones like MT7988 / Filogic 880).
        //
        // Depending on the SoC and kernel version, the PPE debugfs paths vary:
        // - MT7981B (Filogic 820): Single block (`ppe0`).
        // - MT7986 (Filogic 830): Two blocks (`ppe0`, `ppe1`).
        // - MT7988 (Filogic 880 / Wi-Fi 7): Three blocks (`ppe0`, `ppe1`, `ppe2`).

        for i in 0..=2 {
            let ppe_paths = [
                format!("/sys/kernel/debug/ppe{}", i),
                format!("/sys/kernel/debug/eth_sys/ppe{}", i),
            ];

            if ppe_paths.iter().any(|p| Path::new(p).exists()) {
                return true;
            }
        }

        // Fallback for older legacy kernels (e.g. 5.4 / 5.10 on MT7621)
        Path::new("/sys/kernel/debug/mtk_ppe").exists()
    }

    /// Checks the status of the MediaTek Wireless Ethernet Dispatcher (WED) hardware offloading.
    fn check_wed() -> bool {
        // First, ensure WED is not globally disabled by a kernel module parameter (e.g., `wed_enable=N`).
        if !Self::is_wed_allowed_by_module() {
            return false;
        }

        // If the module allows WED, verify if the Wi-Fi driver has allocated DMA memory rings.
        // This typically occurs when AP/Client interfaces are brought up, even before any traffic flows.
        // Iterating up to 3 to cover up to Tri-band Wi-Fi 7 designs (wed0, wed1, wed2).
        for i in 0..=3 {
            let wed_indicators = [
                format!("/sys/kernel/debug/wed{}/txinfo", i), // Primary WED TX ring info (Kernel 6.6+)
                format!("/sys/kernel/debug/wed{}/rxinfo", i), // Primary WED RX ring info (Kernel 6.6+)
                format!("/sys/kernel/debug/wed{}/status", i), // Legacy WED status node (Kernel 5.4/5.15)
                format!("/sys/kernel/debug/mtk_wed{}/status", i), // Legacy vendor-specific WED status node
            ];

            for path in &wed_indicators {
                if let Ok(content) = fs::read_to_string(path) {
                    let content = content.trim();

                    // If the file exists but is empty (0 bytes), the Wi-Fi driver has not yet bound the DMA rings
                    // (e.g., hardware is present but the attach process failed or was skipped).
                    if content.is_empty() {
                        continue;
                    }

                    if path.ends_with("status") {
                        // For older kernel versions using the `status` file.
                        if content.contains("status: 1") || content.contains("state: ACTIVE") {
                            return true;
                        }
                    } else {
                        // For Kernel 6.6+: The `txinfo` and `rxinfo` nodes contain register addresses.
                        // If they contain substantial data (length > 10 bytes), the DMA memory has been allocated
                        // and the WED bridge is physically active.
                        if content.len() > 10 {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Checks for the presence and value of the `wed_enable` parameter across all relevant `mt76` modules.
    fn is_wed_allowed_by_module() -> bool {
        // List of relevant MediaTek wireless modules in Linux/OpenWrt.
        let mtk_modules = [
            "mt7915e",     // Wi-Fi 6 standard module for MT7981 / MT7986 / MT7915
            "mt798x_wmac", // Wi-Fi 6 integrated radio module in recent kernels (6.6+)
            "mt7986_wmac", // Wi-Fi 6 integrated radio module in intermediate kernels (5.15 - 6.1)
            "mt7996e",     // Wi-Fi 7 chips (e.g., MT7988)
            "mt7615e",     // Wi-Fi 5 chips
            "mt7622",      // Wi-Fi 5 chips
            "mt76_connac", // Common abstraction layer
        ];

        for mod_name in &mtk_modules {
            let param_path = format!("/sys/module/{}/parameters/wed_enable", mod_name);

            if let Ok(val) = fs::read_to_string(&param_path) {
                let val = val.trim();

                // If `wed_enable` is explicitly disabled ('N' or '0') in any active module,
                // we can guarantee WED is disabled system-wide.
                if val.eq_ignore_ascii_case("n") || val == "0" {
                    return false;
                }
            }
        }

        // Fallback logic:
        // - If the parameter is present and set to 'Y' / '1', this returns true implicitly.
        // - If the parameter is not found (e.g., the module is statically compiled into the kernel),
        //   we return true to allow the debugfs checks to make the final determination.
        true
    }
}
