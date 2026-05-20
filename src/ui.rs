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
            // Header requires 3 lines: top border, content, bottom border
            Constraint::Length(3),
            Constraint::Length(2), // Connection tracking summary
            Constraint::Min(0),    // Active connections table
        ])
        .split(f.area());

    // *** Header widget ***
    let version = env!("CARGO_PKG_VERSION");

    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = sysinfo::System::long_os_version()
        .map(|s| {
            // Extract the OS distribution release string enclosed in parentheses.
            if let (Some(start), Some(end)) = (s.find('('), s.rfind(')')) {
                s[start + 1..end].to_string()
            } else {
                s
            }
        })
        .unwrap_or_else(|| "Unknown OS".to_string());

    let uptime_secs = sysinfo::System::uptime();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;

    let cpu_usage = app.sys.global_cpu_usage();

    let used_ram_mb = app.sys.used_memory() / 1024 / 1024;
    let total_ram_mb = app.sys.total_memory() / 1024 / 1024;

    let header_text = format!(
        " {} | {} | Uptime: {}d {}h | CPU: {:.0}% | RAM: {}/{} MB",
        hostname, os_version, days, hours, cpu_usage, used_ram_mb, total_ram_mb
    );

    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("🚜 Conntracktor v{}", version))
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(header, chunks[0]);

    // *** Conntrack summary widget ***
    let acc = &app.acc_status;

    let sw_color = if acc.software {
        OffloadStatus::Software.color()
    } else {
        OffloadStatus::None.color()
    };
    let ppe_color = if acc.hw_ppe {
        OffloadStatus::HardwarePpe.color()
    } else {
        OffloadStatus::None.color()
    };
    let wed_color = if acc.hw_wed {
        OffloadStatus::HardwareWed.color()
    } else {
        OffloadStatus::None.color()
    };

    let stats = &app.conntrack_stats;
    let summary_line = Line::from(vec![
        Span::raw(format!(
            "[ Connections: {} / {} ]    [ Hardware Offloaded: {} ]    System Engines: [ ",
            stats.total, stats.max, stats.hw_offloaded
        )),
        Span::styled(
            if acc.software { "SW:ON" } else { "SW:OFF" },
            Style::default().fg(sw_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            if acc.hw_ppe { "PPE:ON" } else { "PPE:OFF" },
            Style::default().fg(ppe_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            if acc.hw_wed { "WED:ON" } else { "WED:OFF" },
            Style::default().fg(wed_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ]"),
    ]);

    let stats_widget = Paragraph::new(summary_line);
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
            let cells = vec![
                Cell::from(conn.protocol.to_uppercase()),
                Cell::from(format!("{} {}", conn.src_type.icon(), conn.src_ip)),
                Cell::from(format!("{} {}", conn.dst_type.icon(), conn.dst_ip)),
                Cell::from(conn.status.clone()),
                Cell::from(conn.offload.label()).style(
                    Style::default()
                        .fg(conn.offload.color())
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
            Constraint::Min(22),    // SRC IP (fit the icon + IPv6)
            Constraint::Min(22),    // DST IP (fit the icon + IPv6)
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
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    // Dynamically calculate the page size based on the available layout area:
    // `chunks[2].height` represents the total height of the table area.
    // - 2 rows are allocated for the top and bottom borders (Borders::ALL).
    // - 2 rows are allocated for the table header (height 1 + bottom_margin 1).
    // We utilize the remaining space for rows, ensuring a minimum of 1 visible row.
    app.page_size = chunks[2].height.saturating_sub(4).max(1) as usize;

    f.render_stateful_widget(table, chunks[2], &mut app.table_state);
}
