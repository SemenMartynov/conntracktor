use crate::conntrack::{self, ConntrackStats};
use ratatui::widgets::TableState;
use sysinfo::System;

/// Application state.
pub struct App {
    /// Indicates whether the application should exit.
    pub should_quit: bool,
    /// Collects system statistics.
    pub sys: System,
    /// Holds connection tracking statistics.
    pub conntrack_stats: ConntrackStats,
    /// State for table selection and navigation.
    pub table_state: TableState,
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
            sys: System::new_all(),
            conntrack_stats: ConntrackStats::default(),
            table_state,
        }
    }

    /// Updates the application state. Called on every tick of the event loop.
    pub fn on_tick(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        // Fall back to default stats if fetching fails (e.g., non-Linux platform).
        self.conntrack_stats = conntrack::get_stats().unwrap_or_default();

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

    /// Signals the application to exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
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
}
