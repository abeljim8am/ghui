use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::data::{AnnotationLevel, WorkflowConclusion, WorkflowStatus};
use crate::icons;

/// Render the help popup
pub fn render_help_popup(f: &mut Frame) {
    let area = f.area();
    let popup_width = 40u16;
    let popup_height = 21u16;
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
            Span::styled("w    ", Style::default().fg(Color::Yellow)),
            Span::raw("View Workflows"),
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
            let prefix = if is_selected { icons::SELECTOR } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(
                format!("{}{} {}", prefix, icons::BULLET, label.label_name),
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
            let prefix = if is_selected { icons::SELECTOR } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::styled(
                format!("{}{} {} (global)", prefix, icons::BULLET, label.label_name),
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
            Span::styled(icons::CURSOR, Style::default().fg(Color::Cyan)),
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

/// Render the workflows view as a full page
pub fn render_workflows_view(f: &mut Frame, app: &App) {
    let area = f.area();

    // Create the outer block - show refresh indicator in title if loading while data exists
    let title = if app.actions_loading && app.actions_data.is_some() {
        format!(" Workflows {} ", app.spinner())
    } else {
        " Workflows ".to_string()
    };
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split inner area into: header (2 lines), content (scrollable), footer (2 lines)
    let layout = Layout::vertical([
        Constraint::Length(2), // Header: PR info
        Constraint::Min(1),    // Content: workflows/jobs (scrollable)
        Constraint::Length(2), // Footer: key hints
    ])
    .split(inner_area);

    let header_area = layout[0];
    let content_area = layout[1];
    let footer_area = layout[2];

    // Render header (PR info) - always visible at top
    if let Some((ref title, number)) = app.workflows_pr_info {
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("PR #", Style::default().fg(Color::DarkGray)),
                Span::styled(number.to_string(), Style::default().fg(Color::Cyan)),
                Span::styled(" - ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    truncate_string(title, (area.width as usize).saturating_sub(20)),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::raw(""),
        ]);
        f.render_widget(header, header_area);
    }

    // Render footer (key hints) - always visible at bottom
    let auto_refresh_indicator = if app.actions_poll_enabled {
        Span::styled(" (auto-refreshing)", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    let footer = Paragraph::new(vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" view logs  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" refresh  "),
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::raw(" open  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" back"),
            auto_refresh_indicator,
        ]),
    ]);
    f.render_widget(footer, footer_area);

    // Build scrollable content (workflows and jobs)
    let mut content_lines: Vec<Line> = Vec::new();
    let mut selected_line_index: usize = 0;

    // Only show loading spinner if no data exists yet (initial load)
    // If data exists and we're refreshing, show existing data with spinner in title
    if app.actions_loading && app.actions_data.is_none() {
        content_lines.push(Line::from(vec![
            Span::styled(app.spinner(), Style::default().fg(Color::Yellow)),
            Span::raw(" Loading workflow runs..."),
        ]));
    } else if let Some(ref data) = app.actions_data {
        // Error display
        if let Some(ref err) = data.error {
            content_lines.push(Line::styled(
                format!("Error: {}", err),
                Style::default().fg(Color::Red),
            ));
            content_lines.push(Line::raw(""));
        }

        if data.workflow_runs.is_empty() {
            content_lines.push(Line::styled(
                "No workflow runs found",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            let mut job_index = 0;
            for run in &data.workflow_runs {
                // Workflow header
                let (status_icon, status_color) = get_workflow_status_display(run.status, run.conclusion);

                content_lines.push(Line::from(vec![
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(&run.name, Style::default().fg(Color::Cyan).bold()),
                ]));

                // Jobs
                for job in &run.jobs {
                    let is_selected = job_index == app.selected_job_index;
                    if is_selected {
                        selected_line_index = content_lines.len();
                    }
                    let prefix = if is_selected { icons::SELECTOR_INDENTED } else { "    " };

                    let (job_icon, job_color) = get_workflow_status_display(job.status, job.conclusion);

                    let style = if is_selected {
                        Style::default().fg(Color::Cyan).bold()
                    } else {
                        Style::default().fg(Color::White)
                    };

                    content_lines.push(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(job_icon, Style::default().fg(job_color)),
                        Span::raw(" "),
                        Span::styled(&job.name, style),
                    ]));

                    job_index += 1;
                }
                content_lines.push(Line::raw(""));
            }
        }
    } else {
        content_lines.push(Line::styled(
            "No data available",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Calculate scroll offset to keep selected item visible in the content area
    let visible_height = content_area.height as usize;
    let scroll_offset = if selected_line_index >= visible_height {
        (selected_line_index - visible_height + 2) as u16
    } else {
        0
    };

    let content = Paragraph::new(content_lines).scroll((scroll_offset, 0));
    f.render_widget(content, content_area);
}

/// Render the job logs view as a full page
pub fn render_job_logs_view(f: &mut Frame, app: &App) {
    // Use annotations view if we have annotations, otherwise show regular logs
    if app.annotations_view && !app.annotations.is_empty() {
        render_annotations_view(f, app);
    } else {
        render_raw_logs_view(f, app);
    }
}

/// Render the annotations view (for reviewdog, etc.)
fn render_annotations_view(f: &mut Frame, app: &App) {
    let area = f.area();

    // Get job name for title
    let title = if let Some(ref logs) = app.job_logs {
        format!(" {} ({} findings) ", logs.job_name, app.annotations.len())
    } else {
        " Annotations ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split into content and footer
    let layout = Layout::vertical([
        Constraint::Min(1),    // Content: annotations list
        Constraint::Length(2), // Footer: key hints
    ])
    .split(inner_area);

    let content_area = layout[0];
    let footer_area = layout[1];

    // Render footer with annotation-specific hints
    let footer_line = if let Some(ref feedback) = app.clipboard_feedback {
        Line::from(vec![
            Span::styled(format!("{} {}", icons::STATUS_SUCCESS, feedback), Style::default().fg(Color::Green)),
        ])
    } else {
        let selected_count = app.selected_annotations.len();
        let copy_hint = if selected_count > 0 {
            format!(" copy ({})  ", selected_count)
        } else {
            " copy all  ".to_string()
        };
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("v", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::raw(copy_hint),
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::raw(" open  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" back"),
        ])
    };
    let footer = Paragraph::new(vec![Line::raw(""), footer_line]);
    f.render_widget(footer, footer_area);

    // Build annotations list with selection
    let mut lines: Vec<Line> = Vec::new();
    let visible_height = content_area.height as usize;

    for (idx, ann) in app.annotations.iter().enumerate() {
        let is_selected = idx == app.selected_annotation_index;

        // Level indicator with color (use consistent spacing)
        let (level_icon, level_color) = match ann.level {
            AnnotationLevel::Failure => (icons::ANNOTATION_FAILURE, Color::Red),
            AnnotationLevel::Warning => (icons::ANNOTATION_WARNING, Color::Yellow),
            AnnotationLevel::Notice => (icons::ANNOTATION_NOTICE, Color::Blue),
        };

        // Check if this annotation is selected for copying
        let is_marked = app.selected_annotations.contains(&idx);

        // Selection indicator: ▶ for cursor, ● for marked, space otherwise
        let prefix = if is_selected && is_marked {
            icons::SELECTOR_MARKED
        } else if is_selected {
            icons::SELECTOR_ONLY
        } else if is_marked {
            icons::MARKED_ONLY
        } else {
            icons::UNMARKED
        };

        let highlight_style = if is_selected {
            Style::default().fg(Color::Cyan).bold()
        } else if is_marked {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        // File:line info
        let line_info = if ann.start_line == ann.end_line {
            format!("{}:{}", ann.path, ann.start_line)
        } else {
            format!("{}:{}-{}", ann.path, ann.start_line, ann.end_line)
        };

        // Main line: [prefix][icon] file:line
        lines.push(Line::from(vec![
            Span::styled(prefix, if is_marked { Style::default().fg(Color::Green) } else { Style::default() }),
            Span::styled(level_icon, Style::default().fg(level_color)),
            Span::styled(line_info, highlight_style),
        ]));

        // Message lines (indented, wrapped)
        let message_style = if is_selected {
            Style::default().fg(Color::White)
        } else if is_marked {
            Style::default().fg(Color::Green).italic()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Wrap message to fit width (account for indent)
        let indent = "      ";
        let max_line_width = (content_area.width as usize).saturating_sub(indent.len() + 1);
        let wrapped_lines = wrap_text(&ann.message, max_line_width);

        for line_text in wrapped_lines {
            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled(line_text, message_style),
            ]));
        }

        // Add blank line between annotations for readability
        lines.push(Line::raw(""));
    }

    // Calculate scroll to keep selection visible
    // Find the line index where the selected annotation starts
    let mut selected_start_line = 0;
    let indent = "      ";
    let max_line_width = (content_area.width as usize).saturating_sub(indent.len() + 1);

    for (idx, ann) in app.annotations.iter().enumerate() {
        if idx == app.selected_annotation_index {
            break;
        }
        // Count lines for this annotation: 1 header + wrapped message lines + 1 blank
        let msg_lines = wrap_text(&ann.message, max_line_width).len();
        selected_start_line += 1 + msg_lines + 1;
    }

    let scroll_offset = if selected_start_line >= visible_height.saturating_sub(3) {
        (selected_start_line.saturating_sub(visible_height.saturating_sub(6))) as u16
    } else {
        0
    };

    let content = Paragraph::new(lines).scroll((scroll_offset, 0));
    f.render_widget(content, content_area);
}

/// Render raw logs view (original behavior)
fn render_raw_logs_view(f: &mut Frame, app: &App) {
    let area = f.area();

    // Get job name for title
    let title = if let Some(ref logs) = app.job_logs {
        format!(" {} ", logs.job_name)
    } else if app.job_logs_loading {
        format!(" Loading {} ", app.spinner())
    } else {
        " Job Logs ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split into content and footer
    let layout = Layout::vertical([
        Constraint::Min(1),    // Content: logs
        Constraint::Length(2), // Footer: key hints
    ])
    .split(inner_area);

    let content_area = layout[0];
    let footer_area = layout[1];

    // Render footer
    let footer_line = if let Some(ref feedback) = app.clipboard_feedback {
        Line::from(vec![
            Span::styled(format!("{} {}", icons::STATUS_SUCCESS, feedback), Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" scroll  "),
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::raw(" copy  "),
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::raw(" open  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" back"),
        ])
    };
    let footer = Paragraph::new(vec![Line::raw(""), footer_line]);
    f.render_widget(footer, footer_area);

    // Render content
    if app.job_logs_loading {
        let loading = Paragraph::new(vec![Line::from(vec![
            Span::styled(app.spinner(), Style::default().fg(Color::Yellow)),
            Span::raw(" Loading job logs..."),
        ])]);
        f.render_widget(loading, content_area);
    } else if let Some(ref logs) = app.job_logs {
        let lines: Vec<Line> = logs
            .content
            .lines()
            .map(|line| Line::raw(line.to_string()))
            .collect();

        let content = Paragraph::new(lines)
            .scroll((app.job_logs_scroll, 0))
            .wrap(Wrap { trim: false });
        f.render_widget(content, content_area);
    } else {
        let empty = Paragraph::new("No logs available");
        f.render_widget(empty, content_area);
    }
}

/// Get display icon and color for workflow status
fn get_workflow_status_display(
    status: WorkflowStatus,
    conclusion: Option<WorkflowConclusion>,
) -> (&'static str, Color) {
    match status {
        WorkflowStatus::Completed => match conclusion {
            Some(WorkflowConclusion::Success) => (icons::STATUS_SUCCESS, Color::Green),
            Some(WorkflowConclusion::Failure) => (icons::STATUS_FAILURE, Color::Red),
            Some(WorkflowConclusion::Cancelled) => (icons::STATUS_CANCELLED, Color::Yellow),
            Some(WorkflowConclusion::Skipped) => (icons::STATUS_SKIPPED, Color::DarkGray),
            Some(WorkflowConclusion::TimedOut) => (icons::STATUS_TIMED_OUT, Color::Red),
            Some(WorkflowConclusion::ActionRequired) => (icons::STATUS_ACTION_REQUIRED, Color::Yellow),
            _ => (icons::STATUS_UNKNOWN, Color::DarkGray),
        },
        WorkflowStatus::InProgress => (icons::STATUS_IN_PROGRESS, Color::Yellow),
        WorkflowStatus::Queued => (icons::STATUS_QUEUED, Color::DarkGray),
        WorkflowStatus::Pending => (icons::STATUS_QUEUED, Color::Yellow),
        WorkflowStatus::Waiting => (icons::STATUS_WAITING, Color::DarkGray),
        _ => (icons::STATUS_UNKNOWN, Color::DarkGray),
    }
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

/// Wrap text to fit within a maximum width, breaking on word boundaries
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            // First word on the line
            if word.chars().count() > max_width {
                // Word is longer than max width, split it
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    lines.push(chunk);
                }
            } else {
                current_line = word.to_string();
            }
        } else if current_line.chars().count() + 1 + word.chars().count() <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            // Start new line
            lines.push(std::mem::take(&mut current_line));
            if word.chars().count() > max_width {
                // Word is longer than max width, split it
                let mut chars = word.chars().peekable();
                while chars.peek().is_some() {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if chars.peek().is_some() {
                        lines.push(chunk);
                    } else {
                        current_line = chunk;
                    }
                }
            } else {
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Render the bottom legend with keyboard shortcuts
pub fn render_legend(f: &mut Frame, area: Rect) {
    let legend = Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" nav  "),
        Span::styled("o", Style::default().fg(Color::Yellow)),
        Span::raw(" open  "),
        Span::styled("c", Style::default().fg(Color::Yellow)),
        Span::raw(" checkout  "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" search  "),
        Span::styled("p", Style::default().fg(Color::Yellow)),
        Span::raw(" preview  "),
        Span::styled("w", Style::default().fg(Color::Yellow)),
        Span::raw(" workflows  "),
        Span::styled("l", Style::default().fg(Color::Yellow)),
        Span::raw(" labels  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" help  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]);

    let paragraph = Paragraph::new(legend).style(Style::default().fg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

/// Render the PR preview view with markdown-rendered comments
pub fn render_preview_view(f: &mut Frame, app: &App) {
    let area = f.area();

    // Get PR info for title
    let title = if let Some((ref pr_title, pr_number)) = app.preview_pr_info {
        format!(" #{} - {} ", pr_number, truncate_string(pr_title, 60))
    } else {
        " Preview ".to_string()
    };

    // Add loading indicator if loading
    let title = if app.preview_loading {
        format!("{} {} ", app.spinner(), title.trim())
    } else {
        title
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Split into content and footer
    let layout = Layout::vertical([
        Constraint::Min(1),    // Content
        Constraint::Length(2), // Footer
    ])
    .split(inner_area);

    let content_area = layout[0];
    let footer_area = layout[1];

    // Render footer
    let footer_line = Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" scroll  "),
        Span::styled("^d/^u", Style::default().fg(Color::Yellow)),
        Span::raw(" page  "),
        Span::styled("g/G", Style::default().fg(Color::Yellow)),
        Span::raw(" top/bottom  "),
        Span::styled("o", Style::default().fg(Color::Yellow)),
        Span::raw(" open  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" back"),
    ]);
    let footer = Paragraph::new(vec![Line::raw(""), footer_line]);
    f.render_widget(footer, footer_area);

    // Render content
    if app.preview_loading && app.preview_data.is_none() {
        let loading = Paragraph::new(vec![Line::from(vec![
            Span::styled(app.spinner(), Style::default().fg(Color::Yellow)),
            Span::raw(" Loading PR preview..."),
        ])]);
        f.render_widget(loading, content_area);
    } else if let Some(ref data) = app.preview_data {
        let mut lines: Vec<Line> = Vec::new();

        for (idx, comment) in data.comments.iter().enumerate() {
            // Add separator between comments
            if idx > 0 {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "─".repeat(content_area.width as usize - 2),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::raw(""));
            }

            // Comment header
            let header_label = if comment.is_pr_body {
                "Description"
            } else {
                "Comment"
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", header_label),
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::styled(
                    format!("by {}", comment.author),
                    Style::default().fg(Color::Green),
                ),
            ]));
            lines.push(Line::raw(""));

            // Render markdown body
            let md_lines = markdown_to_lines(&comment.body, content_area.width as usize - 2);
            lines.extend(md_lines);
        }

        if lines.is_empty() {
            lines.push(Line::styled(
                "No description or comments available.",
                Style::default().fg(Color::DarkGray),
            ));
        }

        let content = Paragraph::new(lines)
            .scroll((app.preview_scroll, 0))
            .wrap(Wrap { trim: false });
        f.render_widget(content, content_area);
    } else {
        let empty = Paragraph::new("No preview data available");
        f.render_widget(empty, content_area);
    }
}

/// Convert markdown text to styled ratatui Lines (skipping images and videos)
fn markdown_to_lines(markdown: &str, _max_width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_text = String::new();

    // Style state
    let mut bold = false;
    let mut italic = false;
    let code = false;
    let mut heading_color: Option<Color> = None;
    let mut in_code_block = false;
    let mut in_image = false;
    let mut image_alt = String::new();
    let mut image_url = String::new();

    let parser = Parser::new(markdown);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    // Add blank line before headings for spacing
                    if !lines.is_empty() {
                        lines.push(Line::raw(""));
                    }
                    // Heading style - color based on level, no # prefix
                    bold = true;
                    heading_color = Some(match level {
                        pulldown_cmark::HeadingLevel::H1 => Color::Cyan,
                        pulldown_cmark::HeadingLevel::H2 => Color::Green,
                        pulldown_cmark::HeadingLevel::H3 => Color::Yellow,
                        _ => Color::Magenta,
                    });
                }
                Tag::Paragraph => {
                    if !current_spans.is_empty() || !current_text.is_empty() {
                        flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                        if !current_spans.is_empty() {
                            lines.push(Line::from(std::mem::take(&mut current_spans)));
                        }
                        lines.push(Line::raw(""));
                    }
                }
                Tag::CodeBlock(_) => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    in_code_block = true;
                }
                Tag::List(_) => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
                Tag::Item => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    current_spans.push(Span::styled("• ", Style::default().fg(Color::Yellow)));
                }
                Tag::Strong => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    bold = true;
                }
                Tag::Emphasis => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    italic = true;
                }
                Tag::Link { .. } => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                }
                Tag::Image { dest_url, .. } => {
                    // Capture image info to show as raw markdown
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    in_image = true;
                    image_url = dest_url.to_string();
                    image_alt.clear();
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    flush_heading_text(&mut current_text, &mut current_spans, heading_color);
                    bold = false;
                    heading_color = None;
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    lines.push(Line::raw(""));
                }
                TagEnd::Paragraph => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    lines.push(Line::raw(""));
                }
                TagEnd::List(_) => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
                TagEnd::Item => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
                TagEnd::Strong => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    bold = false;
                }
                TagEnd::Emphasis => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    italic = false;
                }
                TagEnd::Link => {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                }
                TagEnd::Image => {
                    // Output the image as raw markdown text
                    let raw_md = format!("![{}]({})", image_alt, image_url);
                    current_spans.push(Span::styled(
                        raw_md,
                        Style::default().fg(Color::DarkGray),
                    ));
                    in_image = false;
                    image_alt.clear();
                    image_url.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_image {
                    // Capture alt text for image
                    image_alt.push_str(&text);
                    continue;
                }
                if in_code_block {
                    // Code block - render each line with gray background
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::Gray),
                        )));
                    }
                } else {
                    current_text.push_str(&text);
                }
            }
            Event::Code(code_text) => {
                if in_image {
                    continue;
                }
                flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                current_spans.push(Span::styled(
                    format!("`{}`", code_text),
                    Style::default().fg(Color::Gray),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !in_code_block && !in_image {
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                }
            }
            Event::Html(html) => {
                // Show HTML content as raw text (video embeds, iframes, etc.)
                let html_lower = html.to_lowercase();
                if html_lower.contains("<video")
                    || html_lower.contains("<img")
                    || html_lower.contains("<iframe")
                {
                    // Show as raw HTML in gray
                    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
                    current_spans.push(Span::styled(
                        html.trim().to_string(),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
            _ => {}
        }
    }

    // Flush any remaining content
    flush_text(&mut current_text, &mut current_spans, bold, italic, code);
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

/// Flush accumulated text to spans with appropriate styling
fn flush_text(
    text: &mut String,
    spans: &mut Vec<Span<'static>>,
    bold: bool,
    italic: bool,
    _code: bool,
) {
    if text.is_empty() {
        return;
    }

    let mut style = Style::default();
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if italic {
        style = style.add_modifier(Modifier::ITALIC);
    }

    spans.push(Span::styled(std::mem::take(text), style));
}

/// Flush accumulated text for headings with color
fn flush_heading_text(
    text: &mut String,
    spans: &mut Vec<Span<'static>>,
    color: Option<Color>,
) {
    if text.is_empty() {
        return;
    }

    let style = if let Some(c) = color {
        Style::default().fg(c).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };

    spans.push(Span::styled(std::mem::take(text), style));
}

/// Calculate the line positions of each comment in the preview view
/// Returns (comment_positions, total_lines)
pub fn calculate_preview_positions(comments: &[crate::data::PrComment], width: usize) -> (Vec<u16>, u16) {
    let mut positions: Vec<u16> = Vec::new();
    let mut current_line: u16 = 0;

    for (idx, comment) in comments.iter().enumerate() {
        // Count separator lines first (3 lines if not first: blank + separator + blank)
        if idx > 0 {
            current_line += 3;
        }

        // Record the start position of this comment (at the header, after separator)
        positions.push(current_line);

        // Header line (e.g., "Description by author")
        current_line += 1;
        // Blank line after header
        current_line += 1;

        // Count markdown body lines
        let md_lines = markdown_to_lines(&comment.body, width.saturating_sub(2));
        current_line += md_lines.len() as u16;
    }

    (positions, current_line)
}
