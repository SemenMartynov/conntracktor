use sysinfo::System;

/// Contains static host identifiers that do not change during runtime.
#[derive(Default, Debug)]
pub struct HostInfo {
    pub hostname: String,
    pub os_version: String,
}

/// Contains dynamic system performance metrics updated periodically.
#[derive(Default, Debug)]
pub struct SystemStats {
    pub uptime_days: u64,
    pub uptime_hours: u64,
    pub cpu_usage: f32,
    pub used_ram_mb: u64,
    pub total_ram_mb: u64,
}

impl SystemStats {
    /// Updates system metrics from the provided `System` instance.
    pub fn update(&mut self, sys: &mut System) {
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let uptime_secs = System::uptime();
        self.uptime_days = uptime_secs / 86400;
        self.uptime_hours = (uptime_secs % 86400) / 3600;

        self.cpu_usage = sys.global_cpu_usage();
        self.used_ram_mb = sys.used_memory() / 1024 / 1024;
        self.total_ram_mb = sys.total_memory() / 1024 / 1024;
    }
}
