use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Row, Table},
};

use super::components::section_block;
use crate::app::App;
use crate::conntrack::OffloadStatus;

/// Renders the active connections table widget.
/// Needs `&mut App` to mutate `table_state` and calculate `page_size`.
pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
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
    ]);

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
    .block(section_block(title_left, Some(title_right)))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    // Dynamically calculate the page size based on the available layout area:
    // `area.height` represents the total height of the table area.
    // - 2 rows are allocated for the top and bottom borders (Borders::ALL).
    // - 2 rows are allocated for the table header (height 1 + bottom_margin 1).
    // We utilize the remaining space for rows, ensuring a minimum of 1 visible row.
    app.page_size = area.height.saturating_sub(4).max(1) as usize;

    f.render_stateful_widget(table, area, &mut app.table_state);
}
