use crate::accelerator::AccelerationStatus;
use crate::conntrack::{self, ConntrackStats};
use crate::topology::Topology;
use ratatui::widgets::TableState;
use sysinfo::System;

/// Application state.
pub struct App {
    /// Indicates whether the application should exit.
    pub should_quit: bool,
    /// System packet acceleration status detected at startup.
    pub acc_status: AccelerationStatus,
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
        let acc_status = AccelerationStatus::check_system();

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            should_quit: false,
            acc_status,
            topology: Topology::new(),
            sys: System::new_all(),
            conntrack_stats: ConntrackStats::default(),
            table_state,
            page_size: 10,
        }
    }

    /// Updates the application state. Called on every tick of the event loop.
    pub fn on_tick(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

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
}
