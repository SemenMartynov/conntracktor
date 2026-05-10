use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::App;
use crate::conntrack::OffloadStatus;

/// Renders the user interface.
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header
            Constraint::Length(2), // Connection tracking summary
            Constraint::Min(0),    // Active connections table
        ])
        .split(f.area());

    // *** Header widget ***
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

    // *** Conntrack summary widget ***
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

    // *** Active connections table widget ***
    let header_cells = ["PROTO", "SOURCE IP", "DESTINATION IP", "STATUS", "OFFLOAD"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header_row = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .conntrack_stats
        .connections
        .iter()
        .map(|conn| {
            let (offload_str, offload_color) = match conn.offload {
                OffloadStatus::HwOffload => ("[HW_OFFLOAD]", Color::Green),
                OffloadStatus::Cpu => ("CPU", Color::Gray),
            };

            let cells = vec![
                Cell::from(conn.protocol.to_uppercase()),
                Cell::from(conn.src_ip.clone()),
                Cell::from(conn.dst_ip.clone()),
                Cell::from(conn.status.clone()),
                Cell::from(offload_str).style(
                    Style::default()
                        .fg(offload_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ];

            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(7),  // PROTO
            Constraint::Length(16), // SRC IP
            Constraint::Length(16), // DST IP
            Constraint::Length(15), // STATUS
            Constraint::Min(15),    // OFFLOAD
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Connections "),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);
}
