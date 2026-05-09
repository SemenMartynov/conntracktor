use crate::conntrack::{self, ConntrackStats};
use sysinfo::System;

/// Application state.
pub struct App {
    /// Indicates whether the application should exit.
    pub should_quit: bool,
    /// Collects system statistics.
    pub sys: System,
    /// Holds connection tracking statistics.
    pub conntrack_stats: ConntrackStats,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new application state.
    pub fn new() -> Self {
        Self {
            should_quit: false,
            sys: System::new_all(),
            conntrack_stats: ConntrackStats::default(),
        }
    }

    /// Updates the application state. Called on every tick of the event loop.
    pub fn on_tick(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        // Fall back to default stats if fetching fails (e.g., non-Linux platform).
        self.conntrack_stats = conntrack::get_stats().unwrap_or_default();
    }

    /// Signals the application to exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
