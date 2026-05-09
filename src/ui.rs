use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

/// Renders the user interface.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header
            Constraint::Length(2), // Connection tracking summary
            Constraint::Min(0),    // Main content area
        ])
        .split(f.area());

    // Header widget
    let version = env!("CARGO_PKG_VERSION");

    let uptime_secs = sysinfo::System::uptime();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;

    let cpu_usage = app.sys.global_cpu_usage();

    let used_ram_mb = app.sys.used_memory() / 1024 / 1024;
    let total_ram_mb = app.sys.total_memory() / 1024 / 1024;

    let header_text = format!(
        "🚜 Conntracktor v{} | Uptime: {}d {}h | CPU: {:.0}% | RAM: {}/{} MB",
        version, days, hours, cpu_usage, used_ram_mb, total_ram_mb
    );

    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(header, chunks[0]);

    // Conntrack summary widget
    let stats = &app.conntrack_stats;

    let stats_text = format!(
        "[ Total Connections: {} / {} ]    [ Hardware Offloaded: {} ]",
        stats.total, stats.max, stats.hw_offloaded
    );

    let stats_widget = Paragraph::new(Line::from(vec![Span::styled(
        stats_text,
        Style::default().fg(Color::White),
    )]));

    f.render_widget(stats_widget, chunks[1]);
}
