use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::components::section_block;
use crate::platform::HardwareInfo;

/// Column width used when formatting hardware specification labels (SoC, CPU, Memory, ...).
const SPEC_LABEL_WIDTH: usize = 18;
/// Column width used when formatting capability feature labels (PPE, RSS, AES, ...).
const FEATURE_LABEL_WIDTH: usize = 18;

/// Formats a generic hardware specification line.
fn format_spec<'a>(label: &str, value: impl std::fmt::Display) -> Line<'a> {
    Line::from(vec![Span::raw(format!(
        "{:<SPEC_LABEL_WIDTH$} {}",
        label, value
    ))])
}

/// Formats the CPU specification line, appending a red warning if the frequency is modified.
fn format_cpu<'a>(hw: &HardwareInfo) -> Line<'a> {
    let base_info = if hw.cpu_arch.contains('×') {
        hw.cpu_arch.clone()
    } else {
        format!("{}× {}", hw.cpu_cores, hw.cpu_arch)
    };

    let mut spans = vec![Span::raw(format!("{:<SPEC_LABEL_WIDTH$} ", "CPU"))];

    if let Some(freq) = hw.cpu_freq_mhz {
        spans.push(Span::raw(format!("{base_info} @ {freq} MHz")));

        let expected_mhz = hw.soc.core_frequency_hz() / 1_000_000;
        let is_modified = expected_mhz > 0 && freq != expected_mhz;

        if is_modified {
            spans.push(Span::styled(" ❗", Style::default().fg(Color::Red)));
        }
    } else {
        spans.push(Span::raw(base_info));
    }

    Line::from(spans)
}

/// Formats a hardware capability feature line with a red warning on validation failure.
fn format_value<'a, T: PartialEq + std::fmt::Display>(
    label: &str,
    actual: T,
    expected: T,
) -> Line<'a> {
    let mut spans = vec![
        Span::raw(format!("  {:<FEATURE_LABEL_WIDTH$} ", label)),
        Span::raw(actual.to_string()),
    ];

    if actual != expected {
        spans.push(Span::styled(" ❗", Style::default().fg(Color::Red)));
    }

    Line::from(spans)
}

/// Formats a boolean capability feature line applying strict coloring to standard marks.
fn format_bool<'a>(label: &str, actual: bool, expected: bool) -> Line<'a> {
    let (symbol, color) = if actual {
        ("✓", Color::Green)
    } else {
        ("✗", Color::Red)
    };

    let mut spans = vec![
        Span::raw(format!("  {:<FEATURE_LABEL_WIDTH$} ", label)),
        Span::styled(symbol, Style::default().fg(color)),
    ];

    if actual != expected {
        spans.push(Span::styled(" ❗", Style::default().fg(Color::Red)));
    }

    Line::from(spans)
}

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
        format_spec("SoC", hw.display_soc()),
        format_cpu(hw),
        format_spec("Memory", hw.display_ram()),
        format_spec("Switch", &hw.switch),
        format_spec("Flash", &hw.flash),
        format_spec("Misc", &hw.misc),
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
        format_value("PPE", hw.ppe_count, hw.soc.ppe_count()),
        format_value("WED", hw.wed_count, hw.soc.wed_count()),
        format_bool("RSS", hw.rss, hw.soc.has_rss()),
        format_bool(
            "Checksum Offload",
            hw.checksum_offload,
            hw.soc.has_checksum_offload(),
        ),
        format_bool("TSO", hw.tso, hw.soc.has_tso()),
        format_bool("Multi RX Queues", hw.multi_rx, hw.soc.has_multi_rx_queues()),
    ];

    let sec_lines = vec![
        format_bool(
            "Crypto Engine",
            hw.crypto_engine,
            hw.soc.has_crypto_engine(),
        ),
        format_bool("AES", hw.aes, hw.soc.has_aes_acceleration()),
        format_bool("SHA", hw.sha, hw.soc.has_sha_acceleration()),
        format_bool("TRNG", hw.trng, hw.soc.has_trng()),
        format_bool("Secure Boot", hw.secure_boot, hw.soc.has_secure_boot()),
        format_bool("TrustZone (TEE)", hw.trustzone, hw.soc.has_trustzone()),
    ];

    f.render_widget(Paragraph::new(net_lines), feat_chunks[0]);
    f.render_widget(Paragraph::new(sec_lines), feat_chunks[1]);
}
