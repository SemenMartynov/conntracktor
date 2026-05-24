use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table},
};

use crate::app::App;
use crate::conntrack::OffloadStatus;

/// Column width used when formatting hardware specification labels (SoC, CPU, Memory, ...).
const SPEC_LABEL_WIDTH: usize = 18;
/// Column width used when formatting capability feature labels (PPE, RSS, AES, ...).
const FEATURE_LABEL_WIDTH: usize = 18;

/// Builds a standard bordered section block
fn section_block<'a>(title: impl Into<Line<'a>>, hint: Option<Line<'a>>) -> Block<'a> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title.into());

    if let Some(hint) = hint {
        block = block.title(hint.right_aligned());
    }

    block
}

/// Renders the user interface.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Layout constraints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Header requires 3 lines: top border, content, bottom border
            Constraint::Length(3),
            Constraint::Length(16), // Hardware Panel Widget
            Constraint::Min(0),     // Active connections table
        ])
        .split(f.area());

    // *** Header widget ***
    let version = env!("CARGO_PKG_VERSION");

    let header_text = format!(
        "{} │ {} │ Uptime: {}d {}h │ CPU: {:.0}% │ RAM: {}/{} MB",
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
    .block(section_block(
        format!(" 🚜 Conntracktor v{} ", version),
        Some(Line::from(" [ press ? for help ] ")),
    ));

    f.render_widget(header, chunks[0]);

    // *** Hardware Panel Widget ***
    let hw = &app.hardware_info;

    let hardware_block = section_block(
        " Hardware ",
        Some(Line::from(
            " [ Hardware only. Software support may be disabled ] ",
        )),
    );

    let inner_area = hardware_block.inner(chunks[1]);
    f.render_widget(hardware_block, chunks[1]);

    // Split the inner hardware panel into sub-sections:
    // 6 rows (specifications), 1 blank line, 1 row (feature section headers), 6 rows (capability indicators)
    let hw_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(6),
        ])
        .split(inner_area);

    // Hardware specifications
    let spec_lines = vec![
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "SoC",
            hw.display_soc()
        ))]),
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "CPU",
            hw.display_cpu()
        ))]),
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "Memory",
            hw.display_ram()
        ))]),
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "Switch", hw.switch
        ))]),
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "Flash", hw.flash
        ))]),
        Line::from(vec![Span::raw(format!(
            "{:<SPEC_LABEL_WIDTH$} {}",
            "Misc", hw.misc
        ))]),
    ];
    f.render_widget(Paragraph::new(spec_lines), hw_chunks[0]);

    // Feature section headers (Networking & Security)
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(hw_chunks[2]);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Networking",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        header_chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Security",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        header_chunks[1],
    );

    // Capability feature matrix
    let feat_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(hw_chunks[3]);

    let net_lines = vec![
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "PPE",
            hw.display_ppe_count()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "WED",
            hw.display_wed_count()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "RSS",
            hw.display_rss()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "Checksum Offload",
            hw.display_checksum_offload()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "TSO",
            hw.display_tso()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "Multi RX Queues",
            hw.display_multi_rx()
        )),
    ];

    let sec_lines = vec![
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "Crypto Engine",
            hw.display_crypto_engine()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "AES",
            hw.display_aes()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "SHA",
            hw.display_sha()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "TRNG",
            hw.display_trng()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "Secure Boot",
            hw.display_secure_boot()
        )),
        Line::from(format!(
            "  {:<FEATURE_LABEL_WIDTH$} {}",
            "TrustZone (TEE)",
            hw.display_trustzone()
        )),
    ];

    f.render_widget(Paragraph::new(net_lines), feat_chunks[0]);
    f.render_widget(Paragraph::new(sec_lines), feat_chunks[1]);

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
    let popup_v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let popup_h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_v_chunks[1]);

    popup_h_chunks[1]
}
