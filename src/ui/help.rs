use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::components::centered_rect;

/// Renders the help popup window.
pub fn draw(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 50, area);

    // Clear the background so elements behind it don't bleed through
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
