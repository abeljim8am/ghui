use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    Frame,
};

use crate::app::App;
use crate::data::PrFilter;

/// Render the tab bar
pub fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab1_style = if app.pr_filter == PrFilter::MyPrs {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tab2_style = if app.pr_filter == PrFilter::ReviewRequested {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tab3_style = if matches!(app.pr_filter, PrFilter::Labels(_)) {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let loading_indicator = if app.is_loading() {
        format!("{} ", app.spinner())
    } else {
        String::new()
    };

    let repo_display = app.repo_name.as_deref().unwrap_or("unknown");
    let my_count = app.my_prs.len();
    let review_count = app.review_prs.len();
    let labels_count = app.labels_prs.len();

    let tab1_label = format!(" [1] My PRs ({}) ", my_count);
    let tab2_label = format!("[2] Review Requested ({}) ", review_count);
    let tab3_label = format!("[3] Labels ({}) ", labels_count);

    // Left side: tabs
    let left = Line::from(vec![
        Span::styled(tab1_label, tab1_style),
        Span::raw(" "),
        Span::styled(tab2_label, tab2_style),
        Span::raw(" "),
        Span::styled(tab3_label, tab3_style),
    ]);

    // Right side: loading + repo info
    let right = Line::from(vec![
        Span::styled(loading_indicator, Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{} ", repo_display),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(right.width() as u16)])
        .split(area);

    f.render_widget(left, chunks[0]);
    f.render_widget(right, chunks[1]);
}
