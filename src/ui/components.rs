use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Padding},
};

/// Builds a standard bordered section block with an optional hint aligned to the right.
pub fn section_block<'a>(title: impl Into<Line<'a>>, hint: Option<Line<'a>>) -> Block<'a> {
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

/// Helper function to create a centered rectangle of a given percentage size.
/// Useful for rendering popups (e.g., Help).
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
