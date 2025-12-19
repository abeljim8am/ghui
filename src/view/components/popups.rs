use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

/// Render the help popup
pub fn render_help_popup(f: &mut Frame) {
    let area = f.area();
    let popup_width = 40u16;
    let popup_height = 20u16;
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let help_lines = vec![
        Line::from(vec![
            Span::styled("/    ", Style::default().fg(Color::Yellow)),
            Span::raw("Fuzzy search"),
        ]),
        Line::from(vec![
            Span::styled("1    ", Style::default().fg(Color::Yellow)),
            Span::raw("My Pull Requests"),
        ]),
        Line::from(vec![
            Span::styled("2    ", Style::default().fg(Color::Yellow)),
            Span::raw("Review Requested"),
        ]),
        Line::from(vec![
            Span::styled("3    ", Style::default().fg(Color::Yellow)),
            Span::raw("Labels"),
        ]),
        Line::from(vec![
            Span::styled("l    ", Style::default().fg(Color::Yellow)),
            Span::raw("Manage labels"),
        ]),
        Line::from(vec![
            Span::styled("j/↓  ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("k/↑  ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("g/G  ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to top/bottom"),
        ]),
        Line::from(vec![
            Span::styled("o/⏎  ", Style::default().fg(Color::Yellow)),
            Span::raw("Open PR in browser"),
        ]),
        Line::from(vec![
            Span::styled("c    ", Style::default().fg(Color::Yellow)),
            Span::raw("Checkout branch"),
        ]),
        Line::from(vec![
            Span::styled("r    ", Style::default().fg(Color::Yellow)),
            Span::raw("Refresh"),
        ]),
        Line::from(vec![
            Span::styled("q    ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::raw(""),
        Line::from("Press any key to close").centered(),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .title(" Help ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(help, popup_area);
}

/// Render the checkout confirmation popup
pub fn render_checkout_popup(f: &mut Frame, branch: &str) {
    let area = f.area();
    let popup_width = 50u16;
    let popup_height = 7u16;
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let content = vec![
        Line::raw(""),
        Line::from(format!("Checkout branch: {}", branch)).centered(),
        Line::raw(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("y", Style::default().fg(Color::Green).bold()),
            Span::raw(" to confirm or "),
            Span::styled("n", Style::default().fg(Color::Red).bold()),
            Span::raw(" to cancel"),
        ])
        .centered(),
    ];

    let popup = Paragraph::new(content).block(
        Block::default()
            .title(" Checkout ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(popup, popup_area);
}

/// Render the error popup
pub fn render_error_popup(f: &mut Frame, error: &str) {
    let area = f.area();
    let popup_width = (area.width * 60 / 100).max(40).min(area.width - 4);
    let popup_height = 7u16;
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let error_paragraph = Paragraph::new(error)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(" Error ")
                .title_style(Style::default().fg(Color::Red).bold())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(error_paragraph, popup_area);

    let hint_area = Rect {
        x: popup_area.x,
        y: popup_area.y + popup_area.height,
        width: popup_area.width,
        height: 1,
    };

    if hint_area.y < area.height {
        let hint = Line::from(vec![
            Span::raw("Press "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" to dismiss"),
        ])
        .centered();
        f.render_widget(hint, hint_area);
    }
}

/// Render the labels management popup
pub fn render_labels_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 50u16;
    let popup_height = 16u16;
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    // Build content
    let repo_display = match (&app.repo_owner, &app.repo_name) {
        (Some(o), Some(r)) => format!("{}/{}", o, r),
        _ => "unknown".to_string(),
    };

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Repo: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&repo_display, Style::default().fg(Color::White)),
        ]),
        Line::raw(""),
    ];

    // Separate repo-specific and global labels
    let repo_labels: Vec<_> = app
        .configured_labels
        .iter()
        .filter(|l| !l.is_global())
        .collect();
    let global_labels: Vec<_> = app
        .configured_labels
        .iter()
        .filter(|l| l.is_global())
        .collect();

    if !repo_labels.is_empty() {
        lines.push(Line::styled(
            "Repo labels:",
            Style::default().fg(Color::Yellow),
        ));
        for (i, label) in repo_labels.iter().enumerate() {
            let is_selected = app.labels_list_state.selected() == Some(i);
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(
                format!("{}• {}", prefix, label.label_name),
                style,
            ));
        }
        lines.push(Line::raw(""));
    }

    if !global_labels.is_empty() {
        lines.push(Line::styled(
            "Global labels:",
            Style::default().fg(Color::Yellow),
        ));
        let offset = repo_labels.len();
        for (i, label) in global_labels.iter().enumerate() {
            let is_selected = app.labels_list_state.selected() == Some(offset + i);
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(
                format!("{}• {} (global)", prefix, label.label_name),
                style,
            ));
        }
        lines.push(Line::raw(""));
    }

    if app.configured_labels.is_empty() {
        lines.push(Line::styled(
            "No labels configured",
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::raw(""));
    }

    // Hint line
    lines.push(Line::from(vec![
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(" add  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" delete  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" close"),
    ]));

    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Labels ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(popup, popup_area);
}

/// Render the add label popup
pub fn render_add_label_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_width = 45u16;
    let popup_height = 10u16;
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let scope_repo = if app.label_scope_global { "[ ]" } else { "[x]" };
    let scope_global = if app.label_scope_global { "[x]" } else { "[ ]" };

    let content = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("Label: ", Style::default().fg(Color::Yellow)),
            Span::styled(&app.label_input, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]),
        Line::raw(""),
        Line::styled("Scope:", Style::default().fg(Color::Yellow)),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(scope_repo, Style::default().fg(Color::Green)),
            Span::raw(" This repo only"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(scope_global, Style::default().fg(Color::Green)),
            Span::raw(" Global (all repos)"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" toggle  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ]),
    ];

    let popup = Paragraph::new(content).block(
        Block::default()
            .title(" Add Label ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(popup, popup_area);
}

/// Calculate a centered rectangle within an area
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);

    Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

/// Truncate a string to a maximum length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
