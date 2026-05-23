use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use crate::app::App;
use crate::conntrack::OffloadStatus;

/// Renders the user interface.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Determine the dynamic height of the engines panel based on the hardware capabilities.
    // E.g., MT7986 has up to 2 engines, requiring 2 lines of content + 2 lines for top/bottom borders.
    let soc = &app.host_info.soc_model;
    let max_engine_lines = std::cmp::max(soc.ppe_count(), soc.wed_count()).max(1) as u16;
    let engines_panel_height = max_engine_lines + 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Header requires 3 lines: top border, content, bottom border
            Constraint::Length(3),
            Constraint::Length(engines_panel_height), // Dynamic engines status panel
            Constraint::Min(0),                       // Active connections table
        ])
        .split(f.area());

    // *** Header widget ***
    let version = env!("CARGO_PKG_VERSION");

    let header_text = format!(
        " {} │ {} │ Uptime: {}d {}h │ CPU: {:.0}% │ RAM: {}/{} MB ",
        app.host_info.hostname,
        app.host_info.os_version,
        app.system_stats.uptime_days,
        app.system_stats.uptime_hours,
        app.system_stats.cpu_usage,
        app.system_stats.used_ram_mb,
        app.system_stats.total_ram_mb,
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
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" 🚜 Conntracktor v{} ", version))
            .title(Line::from(" [ press ? for help ] ").right_aligned()),
    );

    f.render_widget(header, chunks[0]);

    // *** Engines Status Panel Widget ***
    let acc = &app.acc_status;

    let sw_color = if acc.software {
        OffloadStatus::Software.color()
    } else {
        OffloadStatus::None.color()
    };
    let ppe_color = if acc.hw_ppe {
        OffloadStatus::Hardware.color()
    } else {
        OffloadStatus::None.color()
    };
    let wed_color = if acc.hw_wed {
        Color::Cyan
    } else {
        OffloadStatus::None.color()
    };

    // Construct a multi-colored title for the engines panel.
    let engines_title_line = Line::from(vec![
        Span::raw(format!(" {} — Engines: [", soc.display_name())),
        Span::styled(
            if acc.software { "SW:ON" } else { "SW:OFF" },
            Style::default().fg(sw_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("] ["),
        Span::styled(
            if acc.hw_ppe { "PPE:ON" } else { "PPE:OFF" },
            Style::default().fg(ppe_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("] ["),
        Span::styled(
            if acc.hw_wed { "WED:ON" } else { "WED:OFF" },
            Style::default().fg(wed_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("] "),
    ]);

    // The inner content will be implemented later!!
    let engines_panel = Paragraph::new("").block(
        Block::default()
            .borders(Borders::ALL)
            .title(engines_title_line),
    );

    f.render_widget(engines_panel, chunks[1]);

    // *** Active connections table widget ***
    let stats = &app.conntrack_stats;

    let title_left = format!(" Active Connections [ {} / {} ] ", stats.total, stats.max);

    let title_right = Line::from(vec![
        Span::raw("[CPU: "),
        Span::styled(
            stats.cpu.to_string(),
            Style::default()
                .fg(OffloadStatus::None.color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("] [SW: "),
        Span::styled(
            stats.software.to_string(),
            Style::default()
                .fg(OffloadStatus::Software.color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("] [PPE: "),
        Span::styled(
            stats.hardware.to_string(),
            Style::default()
                .fg(OffloadStatus::Hardware.color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("] "),
    ])
    .right_aligned();

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
            .title(title_left)
            .title(title_right),
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

    // *** Help Popup Widget ***
    if app.show_help {
        let popup_area = centered_rect(50, 50, f.area());

        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let popup_text = Paragraph::new("\n   (Help content will be here...)")
            .block(popup_block)
            .style(Style::default().fg(Color::White));

        f.render_widget(popup_text, popup_area);
    }
}

/// Helper function to create the popup window.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
