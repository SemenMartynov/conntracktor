use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::components::section_block;
use crate::platform::HardwareInfo;

/// Column width used when formatting hardware specification labels (SoC, CPU, Memory, ...).
const SPEC_LABEL_WIDTH: usize = 18;
/// Column width used when formatting capability feature labels (PPE, RSS, AES, ...).
const FEATURE_LABEL_WIDTH: usize = 18;

/// Renders the Hardware Panel widget.
pub fn draw(f: &mut Frame, area: Rect, hw: &HardwareInfo) {
    let hardware_block = section_block(
        " Hardware ",
        Some(Line::from(
            " [ Hardware only. Software support may be disabled ] ",
        )),
    );

    let inner_area = hardware_block.inner(area);
    f.render_widget(hardware_block, area);

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
}
