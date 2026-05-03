use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

/// Renders the user interface.
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

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

    let header_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(header, header_area);
}
