use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::icons;

use super::components::{
    render_add_label_popup, render_checkout_popup, render_error_popup, render_help_popup,
    render_job_logs_view, render_labels_popup, render_legend, render_preview_view,
    render_search_bar, render_table, render_tabs, render_toast, render_workflows_view,
};

/// Main UI rendering function
pub fn ui(f: &mut Frame, app: &App) {
    // Always clear the entire screen first to prevent leftover characters
    f.render_widget(Clear, f.area());

    // If in workflows view, render it as a full page
    if app.show_workflows_view {
        if app.show_job_logs {
            render_job_logs_view(f, app);
        } else {
            render_workflows_view(f, app);
        }

        // Still render error popup over workflows view
        if app.show_error_popup {
            if let Some(ref error) = app.error {
                render_error_popup(f, error);
            }
        }
        render_toast(f, app);
        return;
    }

    // If in preview view, render it as a full page
    if app.show_preview_view {
        render_preview_view(f, app);

        // Still render error popup over preview view
        if app.show_error_popup {
            if let Some(ref error) = app.error {
                render_error_popup(f, error);
            }
        }
        render_toast(f, app);
        return;
    }

    // Calculate layout based on whether search is active
    let chunks = if app.search_mode || !app.search_query.is_empty() {
        Layout::vertical([
            Constraint::Length(1), // Tabs
            Constraint::Length(1), // Separator
            Constraint::Min(0),    // Table
            Constraint::Length(1), // Search bar
            Constraint::Length(1), // Legend
        ])
        .split(f.area())
    } else {
        Layout::vertical([
            Constraint::Length(1), // Tabs
            Constraint::Length(1), // Separator
            Constraint::Min(0),    // Table
            Constraint::Length(1), // Legend
        ])
        .split(f.area())
    };

    render_tabs(f, app, chunks[0]);

    // Separator line
    let separator = icons::SEPARATOR_CHAR.repeat(chunks[1].width as usize);
    f.render_widget(
        Paragraph::new(separator).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    render_table(f, app, chunks[2]);

    // Render search bar if in search mode or has query
    if app.search_mode || !app.search_query.is_empty() {
        render_search_bar(f, app, chunks[3]);
        render_legend(f, chunks[4]);
    } else {
        render_legend(f, chunks[3]);
    }

    // Render popups (order matters for layering)
    if app.show_help_popup {
        render_help_popup(f);
    }

    if app.show_checkout_popup {
        if let Some(ref branch) = app.pending_checkout_branch {
            render_checkout_popup(f, branch);
        }
    }

    if app.show_error_popup {
        if let Some(ref error) = app.error {
            render_error_popup(f, error);
        }
    }

    if app.show_labels_popup {
        render_labels_popup(f, app);
    }

    if app.show_add_label_popup {
        render_add_label_popup(f, app);
    }

    // Render toast notification on top of everything
    render_toast(f, app);
}
