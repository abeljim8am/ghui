use ratatui::widgets::TableState;
use std::process::Command as ProcessCommand;

use crate::data::{
    AnnotationLevel, CheckAnnotation, JobLogs, PrFilter, WorkflowJob, WorkflowStatus,
};
use crate::icons;
use crate::services::{delete_label_filter, filter_prs, load_label_filters, save_label_filter};
use crate::utils::checkout_branch;
use crate::view::calculate_preview_positions;

use super::message::{Command, FetchResult, Message};
use super::model::App;

/// Update the application state based on a message.
/// Returns an optional command to be executed by the main loop.
pub fn update(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        // Navigation
        Message::NextItem => {
            next_item(app);
            None
        }
        Message::PreviousItem => {
            previous_item(app);
            None
        }
        Message::GoToTop => {
            if !app.filtered_indices.is_empty() {
                app.table_state.select(Some(0));
            }
            None
        }
        Message::GoToBottom => {
            if !app.filtered_indices.is_empty() {
                app.table_state.select(Some(app.filtered_indices.len() - 1));
            }
            None
        }

        // Tab switching
        Message::SwitchTab(filter) => {
            switch_filter(app, filter);
            None
        }

        // Actions
        Message::OpenSelected => {
            open_selected(app);
            None
        }
        Message::PromptCheckout => {
            prompt_checkout(app);
            None
        }
        Message::ConfirmCheckout => {
            if confirm_checkout(app) {
                return Some(Command::ExitAfterCheckout);
            }
            None
        }
        Message::CancelCheckout => {
            app.show_checkout_popup = false;
            app.pending_checkout_branch = None;
            None
        }
        Message::Refresh => {
            if matches!(app.pr_filter, PrFilter::Labels(_)) {
                let labels = app.get_active_labels();
                Some(Command::StartFetch(PrFilter::Labels(labels)))
            } else {
                Some(Command::StartFetch(app.pr_filter.clone()))
            }
        }

        // Search
        Message::EnterSearchMode => {
            app.search_mode = true;
            None
        }
        Message::ExitSearchMode { clear } => {
            exit_search_mode(app, clear);
            None
        }
        Message::SearchInput(c) => {
            search_push_char(app, c);
            None
        }
        Message::SearchBackspace => {
            search_pop_char(app);
            None
        }

        // Popups
        Message::ToggleHelp => {
            app.show_help_popup = !app.show_help_popup;
            None
        }
        Message::DismissHelp => {
            app.show_help_popup = false;
            None
        }
        Message::DismissError => {
            app.show_error_popup = false;
            None
        }

        // Labels
        Message::OpenLabelsPopup => {
            open_labels_popup(app);
            None
        }
        Message::CloseLabelsPopup => {
            app.show_labels_popup = false;
            None
        }
        Message::OpenAddLabelPopup => {
            app.show_add_label_popup = true;
            app.label_input.clear();
            app.label_scope_global = false;
            None
        }
        Message::CloseAddLabelPopup => {
            app.show_add_label_popup = false;
            app.label_input.clear();
            None
        }
        Message::LabelInput(c) => {
            app.label_input.push(c);
            None
        }
        Message::LabelBackspace => {
            app.label_input.pop();
            None
        }
        Message::ToggleLabelScope => {
            app.label_scope_global = !app.label_scope_global;
            None
        }
        Message::AddLabel => add_label(app),
        Message::DeleteSelectedLabel => delete_selected_label(app),
        Message::LabelsNext => {
            labels_next(app);
            None
        }
        Message::LabelsPrevious => {
            labels_previous(app);
            None
        }

        // Workflows view
        Message::OpenWorkflowsView => open_workflows_view(app),
        Message::CloseWorkflowsView => {
            close_workflows_view(app);
            None
        }
        Message::ActionsDataReceived(result) => {
            handle_actions_result(app, result);
            None
        }
        Message::RefreshActions => refresh_actions(app),
        Message::ActionsNextJob => {
            actions_next_job(app);
            None
        }
        Message::ActionsPreviousJob => {
            actions_previous_job(app);
            None
        }
        Message::OpenActionsInBrowser => {
            open_actions_in_browser(app);
            None
        }

        // Job logs
        Message::OpenJobLogs => open_job_logs(app),
        Message::CloseJobLogs => {
            close_job_logs(app);
            None
        }
        Message::JobLogsReceived(result) => {
            handle_job_logs_result(app, result);
            None
        }
        Message::JobLogsScrollUp => {
            if app.job_logs_scroll > 0 {
                app.job_logs_scroll = app.job_logs_scroll.saturating_sub(3);
            }
            None
        }
        Message::JobLogsScrollDown => {
            app.job_logs_scroll = app.job_logs_scroll.saturating_add(3);
            None
        }
        Message::CopyJobLogs => {
            copy_job_logs_to_clipboard(app);
            None
        }

        // Annotations view
        Message::AnnotationNext => {
            annotation_next(app);
            None
        }
        Message::AnnotationPrevious => {
            annotation_previous(app);
            None
        }
        Message::ToggleAnnotationSelection => {
            toggle_annotation_selection(app);
            None
        }
        Message::CopyAnnotations => {
            copy_annotations(app);
            None
        }

        // Preview view
        Message::OpenPreviewView => open_preview_view(app),
        Message::ClosePreviewView => {
            close_preview_view(app);
            None
        }
        Message::PreviewDataReceived(result) => {
            handle_preview_result(app, result);
            None
        }
        Message::PreviewScrollUp => {
            if app.preview_scroll > 0 {
                app.preview_scroll = app.preview_scroll.saturating_sub(3);
            }
            None
        }
        Message::PreviewScrollDown => {
            // Clamp scroll to avoid showing empty space beyond content
            let max_scroll = app.preview_total_lines.saturating_sub(5);
            let new_scroll = app.preview_scroll.saturating_add(3);
            app.preview_scroll = new_scroll.min(max_scroll);
            None
        }
        Message::PreviewNextSection => {
            preview_next_section(app);
            None
        }
        Message::PreviewPreviousSection => {
            preview_previous_section(app);
            None
        }
        Message::PreviewGoToTop => {
            app.preview_scroll = 0;
            app.preview_section_index = 0;
            None
        }
        Message::PreviewGoToBottom => {
            // Scroll to show the end of content (leave some visible lines at top)
            let visible_height = 20u16; // Approximate visible height
            if app.preview_total_lines > visible_height {
                app.preview_scroll = app.preview_total_lines.saturating_sub(visible_height);
            } else {
                app.preview_scroll = 0;
            }
            if let Some(ref data) = app.preview_data {
                if !data.comments.is_empty() {
                    app.preview_section_index = data.comments.len() - 1;
                }
            }
            None
        }

        // Clear clipboard feedback after timeout
        Message::Tick => {
            if app.loading_my_prs
                || app.loading_review_prs
                || app.loading_labels_prs
                || app.actions_loading
                || app.job_logs_loading
                || app.preview_loading
            {
                app.update_spinner();
            }
            // Clear clipboard feedback after 2 seconds
            if app.clipboard_feedback.is_some()
                && app.clipboard_feedback_time.elapsed() >= std::time::Duration::from_secs(2)
            {
                app.clipboard_feedback = None;
            }
            None
        }

        // Async results
        Message::FetchComplete(result) => handle_fetch_result(app, result),

        // System
        Message::Quit => Some(Command::Quit),
    }
}

// Helper functions

fn next_item(app: &mut App) {
    if app.filtered_indices.is_empty() {
        return;
    }
    let i = match app.table_state.selected() {
        Some(i) => {
            if i >= app.filtered_indices.len() - 1 {
                i
            } else {
                i + 1
            }
        }
        None => 0,
    };
    app.table_state.select(Some(i));
}

fn previous_item(app: &mut App) {
    if app.filtered_indices.is_empty() {
        return;
    }
    let i = match app.table_state.selected() {
        Some(i) => {
            if i == 0 {
                0
            } else {
                i - 1
            }
        }
        None => 0,
    };
    app.table_state.select(Some(i));
}

fn switch_filter(app: &mut App, filter: PrFilter) {
    if app.pr_filter != filter {
        app.pr_filter = filter;
        app.table_state = TableState::default();
        // Clear search when switching tabs
        app.search_mode = false;
        app.search_query.clear();
        update_filtered_indices(app);
        if !app.filtered_indices.is_empty() {
            app.table_state.select(Some(0));
        }
    }
}

fn update_filtered_indices(app: &mut App) {
    let prs = app.current_prs();
    app.filtered_indices = filter_prs(prs, &app.search_query);
}

fn open_selected(app: &App) {
    if let Some(pr) = app.selected_pr() {
        let url = format!(
            "https://github.com/{}/{}/pull/{}",
            pr.repo_owner, pr.repo_name, pr.number
        );
        let _ = ProcessCommand::new("open").arg(&url).spawn();
    }
}

fn prompt_checkout(app: &mut App) {
    if let Some(pr) = app.selected_pr() {
        app.pending_checkout_branch = Some(pr.branch.clone());
        app.show_checkout_popup = true;
    }
}

fn confirm_checkout(app: &mut App) -> bool {
    if let Some(branch) = app.pending_checkout_branch.take() {
        app.show_checkout_popup = false;

        match checkout_branch(&branch) {
            Ok(()) => return true,
            Err(e) => {
                app.error = Some(e);
                app.show_error_popup = true;
            }
        }
    }
    false
}

fn exit_search_mode(app: &mut App, clear_query: bool) {
    app.search_mode = false;
    if clear_query {
        app.search_query.clear();
        update_filtered_indices(app);
        app.table_state = TableState::default();
        if !app.filtered_indices.is_empty() {
            app.table_state.select(Some(0));
        }
    }
}

fn search_push_char(app: &mut App, c: char) {
    app.search_query.push(c);
    update_filtered_indices(app);
    app.table_state = TableState::default();
    if !app.filtered_indices.is_empty() {
        app.table_state.select(Some(0));
    }
}

fn search_pop_char(app: &mut App) {
    app.search_query.pop();
    update_filtered_indices(app);
    app.table_state = TableState::default();
    if !app.filtered_indices.is_empty() {
        app.table_state.select(Some(0));
    }
}

fn open_labels_popup(app: &mut App) {
    app.show_labels_popup = true;
    app.labels_list_state = TableState::default();
    if !app.configured_labels.is_empty() {
        app.labels_list_state.select(Some(0));
    }
}

fn add_label(app: &mut App) -> Option<Command> {
    if app.label_input.trim().is_empty() {
        return None;
    }

    let label_name = app.label_input.trim().to_string();
    let (owner, repo) = if app.label_scope_global {
        (None, None)
    } else {
        (app.repo_owner.as_deref(), app.repo_name.as_deref())
    };

    if let Err(e) = save_label_filter(&label_name, owner, repo) {
        app.error = Some(format!("Failed to save label: {}", e));
        app.show_error_popup = true;
        return None;
    }

    // Reload labels
    reload_labels(app);
    app.show_add_label_popup = false;
    app.label_input.clear();

    // Refresh labels PR list if we're on that tab
    if matches!(app.pr_filter, PrFilter::Labels(_)) {
        let labels = app.get_active_labels();
        return Some(Command::StartFetch(PrFilter::Labels(labels)));
    }

    None
}

fn delete_selected_label(app: &mut App) -> Option<Command> {
    if let Some(selected) = app.labels_list_state.selected() {
        if let Some(label) = app.configured_labels.get(selected) {
            let id = label.id;
            if let Err(e) = delete_label_filter(id) {
                app.error = Some(format!("Failed to delete label: {}", e));
                app.show_error_popup = true;
                return None;
            }

            // Reload labels
            reload_labels(app);

            // Adjust selection
            if app.configured_labels.is_empty() {
                app.labels_list_state.select(None);
            } else if selected >= app.configured_labels.len() {
                app.labels_list_state
                    .select(Some(app.configured_labels.len() - 1));
            }

            // Refresh labels PR list if we're on that tab
            if matches!(app.pr_filter, PrFilter::Labels(_)) {
                let labels = app.get_active_labels();
                return Some(Command::StartFetch(PrFilter::Labels(labels)));
            }
        }
    }
    None
}

fn reload_labels(app: &mut App) {
    if let (Some(owner), Some(repo)) = (&app.repo_owner, &app.repo_name) {
        app.configured_labels = load_label_filters(owner, repo).unwrap_or_default();
    }
}

fn labels_next(app: &mut App) {
    if app.configured_labels.is_empty() {
        return;
    }
    let i = match app.labels_list_state.selected() {
        Some(i) => {
            if i >= app.configured_labels.len() - 1 {
                i
            } else {
                i + 1
            }
        }
        None => 0,
    };
    app.labels_list_state.select(Some(i));
}

fn labels_previous(app: &mut App) {
    if app.configured_labels.is_empty() {
        return;
    }
    let i = match app.labels_list_state.selected() {
        Some(i) => {
            if i == 0 {
                0
            } else {
                i - 1
            }
        }
        None => 0,
    };
    app.labels_list_state.select(Some(i));
}

fn handle_fetch_result(app: &mut App, result: FetchResult) -> Option<Command> {
    match result {
        FetchResult::Success(new_prs, filter) => {
            let is_current_filter = matches!(
                (&app.pr_filter, &filter),
                (PrFilter::MyPrs, PrFilter::MyPrs)
                    | (PrFilter::ReviewRequested, PrFilter::ReviewRequested)
                    | (PrFilter::Labels(_), PrFilter::Labels(_))
            );

            // Check if we're waiting for a PR's head_sha for the actions popup
            let pending_pr_number = app.actions_pending_pr_number;
            let mut actions_command: Option<Command> = None;

            if let Some(pr_number) = pending_pr_number {
                // Look for the PR we're waiting for in the new data
                if let Some(pr) = new_prs.iter().find(|p| p.number == pr_number) {
                    if let Some(ref head_sha) = pr.head_sha {
                        // Found it! Now we can fetch the actions
                        app.actions_pending_pr_number = None;
                        app.actions_poll_enabled = true;
                        actions_command = Some(Command::StartActionsFetch(
                            pr.repo_owner.clone(),
                            pr.repo_name.clone(),
                            pr.number,
                            head_sha.clone(),
                        ));
                    }
                }
            }

            match filter {
                PrFilter::MyPrs => {
                    app.my_prs = new_prs;
                    app.loading_my_prs = false;
                }
                PrFilter::ReviewRequested => {
                    app.review_prs = new_prs;
                    app.loading_review_prs = false;
                }
                PrFilter::Labels(_) => {
                    app.labels_prs = new_prs;
                    app.loading_labels_prs = false;
                }
            }

            // Update filtered indices if viewing this filter
            if is_current_filter {
                update_filtered_indices(app);
                if app.table_state.selected().is_none() && !app.filtered_indices.is_empty() {
                    app.table_state.select(Some(0));
                }
            }

            actions_command
        }
        FetchResult::Error(e) => {
            // If we were waiting for actions, clear the pending state
            if app.actions_pending_pr_number.is_some() {
                app.actions_pending_pr_number = None;
                app.actions_loading = false;
            }
            app.error = Some(e);
            app.show_error_popup = true;
            app.loading_my_prs = false;
            app.loading_review_prs = false;
            app.loading_labels_prs = false;
            None
        }
        // Handled separately by handle_actions_result, handle_job_logs_result, handle_preview_result
        FetchResult::ActionsSuccess(_) | FetchResult::ActionsError(_) => None,
        FetchResult::JobLogsSuccess(_) | FetchResult::JobLogsError(_) => None,
        FetchResult::PreviewSuccess(_) | FetchResult::PreviewError(_) => None,
    }
}

// Workflows view helpers

fn open_workflows_view(app: &mut App) -> Option<Command> {
    // Clone the needed data first to avoid borrow issues
    let pr_data = app.selected_pr().map(|pr| {
        (
            pr.repo_owner.clone(),
            pr.repo_name.clone(),
            pr.number,
            pr.title.clone(),
            pr.head_sha.clone(),
        )
    });

    if let Some((owner, repo, number, title, head_sha_opt)) = pr_data {
        app.show_workflows_view = true;
        app.actions_loading = true;
        app.selected_job_index = 0;
        app.actions_data = None;
        app.workflows_pr_info = Some((title, number));
        app.show_job_logs = false;
        app.job_logs = None;

        if let Some(head_sha) = head_sha_opt {
            // We have the head SHA, fetch actions directly
            app.actions_poll_enabled = true;
            app.actions_pending_pr_number = None;
            return Some(Command::StartActionsFetch(owner, repo, number, head_sha));
        } else {
            // No head SHA available (loaded from cache), trigger a PR refresh
            // The actions will be fetched once we get the updated PR data
            app.actions_pending_pr_number = Some(number);
            app.actions_poll_enabled = false;
            return Some(Command::StartFetch(app.pr_filter.clone()));
        }
    }
    None
}

fn close_workflows_view(app: &mut App) {
    app.show_workflows_view = false;
    app.actions_poll_enabled = false;
    app.actions_data = None;
    app.actions_loading = false;
    app.selected_job_index = 0;
    app.actions_pending_pr_number = None;
    app.workflows_pr_info = None;
    app.show_job_logs = false;
    app.job_logs = None;
    app.job_logs_loading = false;
}

fn handle_actions_result(app: &mut App, result: FetchResult) {
    match result {
        FetchResult::ActionsSuccess(data) => {
            // Check if any jobs are still in progress
            let has_pending = data.workflow_runs.iter().any(|run| {
                matches!(
                    run.status,
                    WorkflowStatus::InProgress | WorkflowStatus::Queued | WorkflowStatus::Pending
                )
            });
            app.actions_poll_enabled = has_pending;
            app.actions_data = Some(data);
            app.actions_loading = false;
        }
        FetchResult::ActionsError(e) => {
            app.actions_loading = false;
            if let Some(ref mut data) = app.actions_data {
                data.error = Some(e);
            } else {
                app.error = Some(e);
                app.show_error_popup = true;
            }
        }
        _ => {}
    }
}

fn refresh_actions(app: &mut App) -> Option<Command> {
    if !app.show_workflows_view {
        return None;
    }

    // Clone the needed data first to avoid borrow issues
    let pr_data = app.selected_pr().map(|pr| {
        (
            pr.repo_owner.clone(),
            pr.repo_name.clone(),
            pr.number,
            pr.head_sha.clone(),
        )
    });

    if let Some((owner, repo, number, Some(head_sha))) = pr_data {
        app.actions_loading = true;
        return Some(Command::StartActionsFetch(owner, repo, number, head_sha));
    }
    None
}

fn actions_next_job(app: &mut App) {
    if let Some(ref data) = app.actions_data {
        let total_jobs: usize = data.workflow_runs.iter().map(|r| r.jobs.len()).sum();
        if app.selected_job_index < total_jobs.saturating_sub(1) {
            app.selected_job_index += 1;
        }
    }
}

fn actions_previous_job(app: &mut App) {
    if app.selected_job_index > 0 {
        app.selected_job_index -= 1;
    }
}

fn open_actions_in_browser(app: &App) {
    if let Some(ref data) = app.actions_data {
        // Find the currently selected job and open its details URL if available
        let mut current_idx = 0;
        for run in &data.workflow_runs {
            for job in &run.jobs {
                if current_idx == app.selected_job_index {
                    // Try job-specific URL first, then fall back to run URL
                    if let Some(ref url) = job.details_url {
                        let _ = ProcessCommand::new("open").arg(url).spawn();
                    } else if !run.html_url.is_empty() {
                        let _ = ProcessCommand::new("open").arg(&run.html_url).spawn();
                    }
                    return;
                }
                current_idx += 1;
            }
        }
        // If no jobs or selection is out of range, open the first run
        if let Some(run) = data.workflow_runs.first() {
            if !run.html_url.is_empty() {
                let _ = ProcessCommand::new("open").arg(&run.html_url).spawn();
            }
        }
    }
}

// Job logs helpers

fn get_selected_job(app: &App) -> Option<(String, String, WorkflowJob)> {
    // Get the selected job's full data (owner, repo, job)
    let (owner, repo) = app
        .selected_pr()
        .map(|pr| (pr.repo_owner.clone(), pr.repo_name.clone()))?;

    if let Some(ref data) = app.actions_data {
        let mut current_idx = 0;
        for run in &data.workflow_runs {
            for job in &run.jobs {
                if current_idx == app.selected_job_index {
                    return Some((owner, repo, job.clone()));
                }
                current_idx += 1;
            }
        }
    }
    None
}

/// Format annotations into readable text content
fn format_annotations(
    annotations: &[CheckAnnotation],
    summary: Option<&str>,
    text: Option<&str>,
) -> String {
    let mut content = String::new();

    // Add summary if present
    if let Some(s) = summary {
        if !s.is_empty() {
            content.push_str("=== Summary ===\n");
            content.push_str(s);
            content.push_str("\n\n");
        }
    }

    // Add text if present
    if let Some(t) = text {
        if !t.is_empty() {
            content.push_str("=== Details ===\n");
            content.push_str(t);
            content.push_str("\n\n");
        }
    }

    // Add annotations
    if !annotations.is_empty() {
        content.push_str("=== Annotations ===\n\n");
        for ann in annotations {
            let level_str = match ann.level {
                AnnotationLevel::Failure => icons::ANNOTATION_FAILURE_LABEL,
                AnnotationLevel::Warning => icons::ANNOTATION_WARNING_LABEL,
                AnnotationLevel::Notice => icons::ANNOTATION_NOTICE_LABEL,
            };

            let line_range = if ann.start_line == ann.end_line {
                format!(":{}", ann.start_line)
            } else {
                format!(":{}-{}", ann.start_line, ann.end_line)
            };

            content.push_str(&format!("{}\n", level_str));
            content.push_str(&format!(
                "  {} {}{}\n",
                icons::EMOJI_FILE,
                ann.path,
                line_range
            ));
            if let Some(ref title) = ann.title {
                content.push_str(&format!("  {} {}\n", icons::EMOJI_PIN, title));
            }
            content.push_str(&format!("  {} {}\n\n", icons::EMOJI_MESSAGE, ann.message));
        }
    }

    if content.is_empty() {
        content = "No annotations or details available.\n\nPress 'o' to open in browser for more information.".to_string();
    }

    content
}

fn open_job_logs(app: &mut App) -> Option<Command> {
    if let Some((owner, repo, job)) = get_selected_job(app) {
        app.show_job_logs = true;
        app.job_logs_scroll = 0;

        // Check if we have annotations from GraphQL (reviewdog, etc.)
        if !job.annotations.is_empty() {
            // Use annotations view for structured display
            app.annotations_view = true;
            app.annotations = job.annotations.clone();
            app.selected_annotation_index = 0;
            app.job_logs = Some(JobLogs {
                job_id: job.id,
                job_name: job.name.clone(),
                content: String::new(), // Not used in annotations view
            });
            app.job_logs_loading = false;
            return None;
        }

        // Check if this looks like a reviewdog report with no findings
        // Reviewdog summaries contain "Findings (0)" when there are no issues
        let is_empty_reviewdog = job
            .summary
            .as_ref()
            .map(|s| s.contains("reviewdog") && s.contains("Findings (0)"))
            .unwrap_or(false);

        if is_empty_reviewdog {
            // Show a clean "no issues" message instead of raw markdown
            app.annotations_view = false;
            app.annotations.clear();
            app.job_logs = Some(JobLogs {
                job_id: job.id,
                job_name: job.name.clone(),
                content: format!(
                    "{} No issues found.\n\nPress 'o' to view details in browser.",
                    icons::STATUS_SUCCESS
                ),
            });
            app.job_logs_loading = false;
            return None;
        }

        // Check if we have summary/text (but no annotations)
        let has_summary = job.summary.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let has_text = job.text.as_ref().map(|s| !s.is_empty()).unwrap_or(false);

        if has_summary || has_text {
            // Use raw text view for summary/text without annotations
            app.annotations_view = false;
            app.annotations.clear();
            let content = format_annotations(&[], job.summary.as_deref(), job.text.as_deref());
            app.job_logs = Some(JobLogs {
                job_id: job.id,
                job_name: job.name.clone(),
                content,
            });
            app.job_logs_loading = false;
            return None;
        }

        // No annotations or summary, fetch logs via gh CLI
        app.annotations_view = false;
        app.annotations.clear();
        app.job_logs_loading = true;
        app.job_logs = None;
        return Some(Command::StartJobLogsFetch(owner, repo, job.id, job.name));
    }
    None
}

fn close_job_logs(app: &mut App) {
    app.show_job_logs = false;
    app.job_logs = None;
    app.job_logs_loading = false;
    app.job_logs_scroll = 0;
    app.annotations_view = false;
    app.annotations.clear();
    app.selected_annotation_index = 0;
    app.selected_annotations.clear();
}

fn handle_job_logs_result(app: &mut App, result: FetchResult) {
    match result {
        FetchResult::JobLogsSuccess(logs) => {
            app.job_logs = Some(logs);
            app.job_logs_loading = false;
        }
        FetchResult::JobLogsError(e) => {
            app.job_logs_loading = false;
            app.error = Some(format!("Failed to load logs: {}", e));
            app.show_error_popup = true;
        }
        _ => {}
    }
}

fn copy_job_logs_to_clipboard(app: &mut App) {
    if let Some(ref logs) = app.job_logs {
        if copy_to_clipboard(&logs.content) {
            app.clipboard_feedback = Some("Copied to clipboard!".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
        }
    }
}

// Annotation view helpers

fn annotation_next(app: &mut App) {
    if !app.annotations.is_empty() && app.selected_annotation_index < app.annotations.len() - 1 {
        app.selected_annotation_index += 1;
    }
}

fn annotation_previous(app: &mut App) {
    if app.selected_annotation_index > 0 {
        app.selected_annotation_index -= 1;
    }
}

/// Format a single annotation for clipboard in a concise, useful format
fn format_annotation_for_clipboard(ann: &CheckAnnotation) -> String {
    // Format: file:line message
    // e.g., "config/initializers/01_sentry.rb:62 [Correctable] Lint/UnusedBlockArgument: Unused block argument..."
    format!("{}:{} {}", ann.path, ann.start_line, ann.message)
}

fn toggle_annotation_selection(app: &mut App) {
    if app.annotations.is_empty() {
        return;
    }

    let idx = app.selected_annotation_index;
    if let Some(pos) = app.selected_annotations.iter().position(|&i| i == idx) {
        // Already selected, remove it
        app.selected_annotations.remove(pos);
    } else {
        // Not selected, add it
        app.selected_annotations.push(idx);
    }
}

fn copy_annotations(app: &mut App) {
    if app.annotations.is_empty() {
        return;
    }

    // If nothing is selected, copy all annotations
    // If some are selected, copy only those
    let (text, count) = if app.selected_annotations.is_empty() {
        let text: String = app
            .annotations
            .iter()
            .map(format_annotation_for_clipboard)
            .collect::<Vec<_>>()
            .join("\n");
        (text, app.annotations.len())
    } else {
        let mut indices = app.selected_annotations.clone();
        indices.sort();
        let text: String = indices
            .iter()
            .filter_map(|&i| app.annotations.get(i))
            .map(format_annotation_for_clipboard)
            .collect::<Vec<_>>()
            .join("\n");
        (text, indices.len())
    };

    if copy_to_clipboard(&text) {
        let msg = if count == 1 {
            "Copied 1 finding to clipboard!".to_string()
        } else {
            format!("Copied {} findings to clipboard!", count)
        };
        app.clipboard_feedback = Some(msg);
        app.clipboard_feedback_time = std::time::Instant::now();
    }
}

fn copy_to_clipboard(text: &str) -> bool {
    let mut child = match ProcessCommand::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }

    child.wait().is_ok()
}

// Preview view helpers

fn open_preview_view(app: &mut App) -> Option<Command> {
    // Clone the needed data first to avoid borrow issues
    let pr_data = app.selected_pr().map(|pr| {
        (
            pr.repo_owner.clone(),
            pr.repo_name.clone(),
            pr.number,
            pr.title.clone(),
        )
    });

    if let Some((owner, repo, number, title)) = pr_data {
        app.show_preview_view = true;
        app.preview_loading = true;
        app.preview_data = None;
        app.preview_scroll = 0;
        app.preview_pr_info = Some((title, number));
        return Some(Command::StartPreviewFetch(owner, repo, number));
    }
    None
}

fn close_preview_view(app: &mut App) {
    app.show_preview_view = false;
    app.preview_data = None;
    app.preview_loading = false;
    app.preview_scroll = 0;
    app.preview_section_index = 0;
    app.preview_comment_positions.clear();
    app.preview_total_lines = 0;
    app.preview_pr_info = None;
}

fn preview_next_section(app: &mut App) {
    let num_comments = app.preview_comment_positions.len();
    if num_comments > 0 && app.preview_section_index < num_comments - 1 {
        app.preview_section_index += 1;
        // Use stored position directly
        if let Some(&pos) = app.preview_comment_positions.get(app.preview_section_index) {
            app.preview_scroll = pos;
        }
    }
}

fn preview_previous_section(app: &mut App) {
    if app.preview_section_index > 0 {
        app.preview_section_index -= 1;
        // Use stored position directly
        if let Some(&pos) = app.preview_comment_positions.get(app.preview_section_index) {
            app.preview_scroll = pos;
        }
    }
}

fn handle_preview_result(app: &mut App, result: FetchResult) {
    match result {
        FetchResult::PreviewSuccess(data) => {
            // Calculate comment positions for navigation
            // Use a reasonable default width (80 columns) for position calculation
            let (positions, total_lines) = calculate_preview_positions(&data.comments, 80);
            app.preview_comment_positions = positions;
            app.preview_total_lines = total_lines;
            app.preview_data = Some(data);
            app.preview_loading = false;
        }
        FetchResult::PreviewError(e) => {
            app.preview_loading = false;
            app.error = Some(e);
            app.show_error_popup = true;
        }
        _ => {}
    }
}
