use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::icons;

/// Render the search bar
pub fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let total_count = app.current_prs().len();
    let filtered_count = app.filtered_indices.len();

    let cursor = if app.search_mode { icons::CURSOR } else { "" };
    let count_display = if app.search_query.is_empty() {
        String::new()
    } else {
        format!(" ({}/{})", filtered_count, total_count)
    };

    let search_line = Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        Span::styled(cursor, Style::default().fg(Color::Cyan)),
        Span::styled(count_display, Style::default().fg(Color::DarkGray)),
    ]);

    f.render_widget(Paragraph::new(search_line), area);
}
