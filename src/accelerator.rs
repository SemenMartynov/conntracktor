use std::path::Path;

/// Status of available software and hardware packet acceleration engines.
#[derive(Debug, Clone, Default)]
pub struct AccelerationStatus {
    pub software: bool,
    pub hw_ppe: bool,
    pub hw_wed: bool,
}

impl AccelerationStatus {
    /// Detects available kernel acceleration modules and sysfs/debugfs flags.
    pub fn check_system() -> Self {
        Self {
            software: Self::check_software_offload(),
            hw_ppe: Self::check_ppe(),
            hw_wed: Self::check_wed(),
        }
    }

    /// Checks for Linux/OpenWrt software flow offloading support (`nf_flow_table`).
    fn check_software_offload() -> bool {
        Path::new("/sys/module/nf_flow_table").exists()
    }

    /// Checks for MediaTek Packet Processing Engine (PPE) wired hardware offloading support.
    fn check_ppe() -> bool {
        Path::new("/sys/module/mtk_ppe").exists() || Path::new("/sys/kernel/debug/mtk_ppe").exists()
    }

    /// Checks for MediaTek Wireless Ethernet Dispatcher (WED) wireless offloading support (`mt76`).
    fn check_wed() -> bool {
        Path::new("/sys/kernel/debug/mt76/wed").exists()
            || Path::new("/sys/module/mt76/parameters/wed_enable").exists()
    }
}
