mod components;
mod connections;
mod hardware;
mod header;
mod help;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::App;

/// Main UI orchestrator.
/// Sets up the layout structure and delegates rendering to specific modules.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Define global layout constraints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Header requires 3 lines: top border, content, bottom border
            Constraint::Length(3),
            Constraint::Length(16), // Hardware Panel Widget
            Constraint::Min(0),     // Active connections table
        ])
        .split(f.area());

    // Delegate drawing to specific modules
    header::draw(f, chunks[0], &app.host_info, &app.system_stats);
    hardware::draw(f, chunks[1], &app.hardware_info);

    // Pass `app` entirely as it needs mutable access to state and dynamic calculations
    connections::draw(f, chunks[2], app);

    // Render overlays (drawn on top of the layout)
    if app.show_help {
        help::draw(f, f.area());
    }
}
