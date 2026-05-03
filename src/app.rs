use sysinfo::System;

/// Application state.
pub struct App {
    /// Indicates whether the application should exit.
    pub should_quit: bool,
    /// Collects system statistics.
    pub sys: System,
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
        }
    }

    /// Updates the application state. Called on every tick of the event loop.
    pub fn on_tick(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
    }

    /// Signals the application to exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
