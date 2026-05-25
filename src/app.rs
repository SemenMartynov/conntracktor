use crate::conntrack::{self, ConntrackStats};
use crate::platform::{AccelerationStatus, HardwareInfo};
use crate::topology::Topology;
use ratatui::widgets::TableState;
use sysinfo::System;

/// Contains static host identifires that do not change during runtime.
#[derive(Default, Debug)]
pub struct HostInfo {
    pub hostname: String,
    pub os_version: String,
}

/// Contains dynamic system performance metric updated periodically.
#[derive(Default, Debug)]
pub struct SystemStats {
    pub uptime_days: u64,
    pub uptime_hours: u64,
    pub cpu_usage: f32,
    pub used_ram_mb: u64,
    pub total_ram_mb: u64,
}

/// Application state.
pub struct App {
    /// Indicates whether the application should exit.
    pub should_quit: bool,
    /// Indicates wheter the help popup shod be displayed.
    pub show_help: bool,
    /// Static host information (hostname, OS release).
    pub host_info: HostInfo,
    /// Dynamic hardware statistics (CPU, RAM, Uptime).
    pub system_stats: SystemStats,
    /// System packet acceleration status detected at startup.
    pub acc_status: AccelerationStatus,
    /// Hardware capabilities and platform feature flags.
    pub hardware_info: HardwareInfo,
    /// Collects system statistics.
    pub sys: System,
    /// Tracks network topology and endpoint classifications.
    pub topology: Topology,
    /// Holds connection tracking statistics.
    pub conntrack_stats: ConntrackStats,
    /// State for table selection and navigation.
    pub table_state: TableState,
    /// How many rows are visible in the table (used for page up/down).
    pub page_size: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new application state.
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            should_quit: false,
            show_help: false,
            host_info: HostInfo::default(),
            system_stats: SystemStats::default(),
            acc_status: AccelerationStatus::default(),
            hardware_info: HardwareInfo::default(),
            topology: Topology::new(),
            sys: System::new_all(),
            conntrack_stats: ConntrackStats::default(),
            table_state,
            page_size: 10,
        }
    }

    /// initializes system component and reads static environment data.
    pub fn init(&mut self) {
        // Fetch hardware acceleration status (e.g., executing `nft` commands).
        self.acc_status = AccelerationStatus::check_system();

        // Inspect system hardware capabilities.
        self.hardware_info = HardwareInfo::detect();

        // Retrieve static host information.
        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());

        let os_version = System::long_os_version()
            .map(|s| {
                // Extract the OS distribution release string enclosed in parentheses.
                if let (Some(start), Some(end)) = (s.find('('), s.rfind(')')) {
                    s[start + 1..end].to_string()
                } else {
                    s
                }
            })
            .unwrap_or_else(|| "Unknown OS".to_string());

        self.host_info = HostInfo {
            hostname,
            os_version,
        };
    }

    /// Updates the application state. Called on every tick of the event loop.
    pub fn on_tick(&mut self) {
        // Refresh CPU and memory hardware metrics.
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        // Calculate and format system statistics for the UI view.
        let uptime_secs = System::uptime();
        self.system_stats.uptime_days = uptime_secs / 86400;
        self.system_stats.uptime_hours = (uptime_secs % 86400) / 3600;

        self.system_stats.cpu_usage = self.sys.global_cpu_usage();
        self.system_stats.used_ram_mb = self.sys.used_memory() / 1024 / 1024;
        self.system_stats.total_ram_mb = self.sys.total_memory() / 1024 / 1024;

        // Fall back to default stats if fetching fails (e.g., non-Linux platform).
        self.conntrack_stats = conntrack::get_stats(&self.topology).unwrap_or_default();

        // Adjust selection index if the connection count decreased below the current selection.
        if let Some(selected) = self.table_state.selected() {
            let conn_count = self.conntrack_stats.connections.len();
            if conn_count == 0 {
                self.table_state.select(None);
            } else if selected >= conn_count {
                self.table_state.select(Some(conn_count - 1));
            }
        }
    }

    /// Toggle the help visualization
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Selects the next item in the connections table.
    pub fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.conntrack_stats.connections.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// Selects the previous item in the connections table.
    pub fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.conntrack_stats.connections.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    /// Selects the first item in the connections table (equivalent to 'gg' or Home).
    pub fn first(&mut self) {
        if self.conntrack_stats.connections.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    /// Selects the last item in the connections table (equivalent to 'G' or End).
    pub fn last(&mut self) {
        let count = self.conntrack_stats.connections.len();
        if count == 0 {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(count - 1));
        }
    }

    /// Jumps up by one visible page in the connections table ('PageUp').
    pub fn page_up(&mut self) {
        let count = self.conntrack_stats.connections.len();
        if count == 0 {
            self.table_state.select(None);
            return;
        }

        let current_offset = self.table_state.offset();
        let current_selected = self.table_state.selected().unwrap_or(0);
        let relative_pos = current_selected.saturating_sub(current_offset);

        if current_offset == 0 {
            // the first screen
            self.table_state.select(Some(0));
            *self.table_state.offset_mut() = 0;
        } else {
            // one screen up
            let new_offset = current_offset.saturating_sub(self.page_size);

            *self.table_state.offset_mut() = new_offset;
            self.table_state.select(Some(new_offset + relative_pos));
        }
    }

    /// Jumps down by one visible page in the connections table ('PageDown').
    pub fn page_down(&mut self) {
        let count = self.conntrack_stats.connections.len();
        if count == 0 {
            self.table_state.select(None);
            return;
        }

        let current_offset = self.table_state.offset();
        let current_selected = self.table_state.selected().unwrap_or(0);

        let relative_pos = current_selected.saturating_sub(current_offset);
        let max_offset = count.saturating_sub(self.page_size);

        if current_offset >= max_offset {
            // the last screen
            self.table_state.select(Some(count.saturating_sub(1)));
            *self.table_state.offset_mut() = max_offset;
        } else {
            // one screent down
            let new_offset = std::cmp::min(current_offset + self.page_size, max_offset);
            *self.table_state.offset_mut() = new_offset;

            let new_selected = std::cmp::min(new_offset + relative_pos, count.saturating_sub(1));
            self.table_state.select(Some(new_selected));
        }
    }

    /// Signals the application to exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
