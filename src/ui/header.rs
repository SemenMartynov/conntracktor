use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::components::section_block;
use crate::platform::{HostInfo, SystemStats};

/// Renders the header containing app version and host/system stats.
pub fn draw(f: &mut Frame, area: Rect, host_info: &HostInfo, system_stats: &SystemStats) {
    let version = env!("CARGO_PKG_VERSION");

    let header_text = format!(
        "{} │ {} │ Uptime: {}d {}h │ CPU: {:.0}% │ RAM: {}/{} MB",
        host_info.hostname,
        host_info.os_version,
        system_stats.uptime_days,
        system_stats.uptime_hours,
        system_stats.cpu_usage,
        system_stats.used_ram_mb,
        system_stats.total_ram_mb,
    );

    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(section_block(
        format!(" 🚜 Conntracktor v{} ", version),
        Some(Line::from(" [ press ? for help ] ")),
    ));

    f.render_widget(header, area);
}
