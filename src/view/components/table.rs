use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::app::App;
use crate::data::PrFilter;
use crate::icons;

use super::popups::truncate_string;

/// Render the PR table
pub fn render_table(f: &mut Frame, app: &App, area: Rect) {
    let visible_prs = app.visible_prs();
    let show_owner = matches!(
        app.pr_filter,
        PrFilter::ReviewRequested | PrFilter::Labels(_)
    );

    let header = if show_owner {
        Row::new(vec![
            Cell::from("PR#").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("Author").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("Title").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("Branch").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("CI Status").style(Style::default().fg(Color::Yellow).bold()),
        ])
    } else {
        Row::new(vec![
            Cell::from("PR#").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("Title").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("Branch").style(Style::default().fg(Color::Yellow).bold()),
            Cell::from("CI Status").style(Style::default().fg(Color::Yellow).bold()),
        ])
    }
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = visible_prs
        .iter()
        .map(|pr| {
            let (ci_text, ci_color) = pr.ci_status.display();
            if show_owner {
                Row::new(vec![
                    Cell::from(format!("#{}", pr.number)),
                    Cell::from(pr.author.clone()).style(Style::default().fg(Color::Magenta)),
                    Cell::from(truncate_string(&pr.title, 45)),
                    Cell::from(truncate_string(&pr.branch, 22)),
                    Cell::from(ci_text).style(Style::default().fg(ci_color)),
                ])
            } else {
                Row::new(vec![
                    Cell::from(format!("#{}", pr.number)),
                    Cell::from(truncate_string(&pr.title, 50)),
                    Cell::from(truncate_string(&pr.branch, 25)),
                    Cell::from(ci_text).style(Style::default().fg(ci_color)),
                ])
            }
        })
        .collect();

    let table = if show_owner {
        let widths = [
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Min(25),
            Constraint::Length(24),
            Constraint::Length(12),
        ];
        Table::new(rows, widths)
    } else {
        let widths = [
            Constraint::Length(8),
            Constraint::Min(30),
            Constraint::Length(27),
            Constraint::Length(12),
        ];
        Table::new(rows, widths)
    }
    .header(header)
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(icons::SELECTOR);

    f.render_stateful_widget(table, area, &mut app.table_state.clone());
}
