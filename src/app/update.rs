use ratatui::widgets::TableState;
use std::process::Command as ProcessCommand;

use crate::data::{
    AnnotationLevel, CheckAnnotation, JobLogs, PrFilter, WorkflowConclusion, WorkflowJob,
    WorkflowStatus,
};
use crate::icons;
use crate::services::{
    circleci_debug_log as debug_log, delete_label_filter, extract_job_number_from_url, filter_prs,
    is_circleci_configured, is_circleci_url, load_label_filters, save_label_filter,
};
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
        Message::DismissUrlPopup => {
            app.show_url_popup = None;
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
        Message::JobLogsNextStep => {
            job_logs_next_step(app);
            None
        }
        Message::JobLogsPrevStep => {
            job_logs_prev_step(app);
            None
        }
        Message::JobLogsToggleStep => {
            job_logs_toggle_step(app);
            None
        }
        Message::OpenStepInEditor => open_step_in_editor(app),
        Message::SmartCopyStepOutput => {
            smart_copy_step_output(app);
            None
        }
        Message::FullCopyStepOutput => {
            full_copy_step_output(app);
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

fn open_selected(app: &mut App) {
    if let Some(pr) = app.selected_pr() {
        let url = format!(
            "https://github.com/{}/{}/pull/{}",
            pr.repo_owner, pr.repo_name, pr.number
        );
        if let Some(display_url) = open_url(&url) {
            app.show_url_popup = Some(display_url);
        }
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

            // Find the first failed job and select it
            let mut first_failed_index: Option<usize> = None;
            let mut current_idx = 0;
            for run in &data.workflow_runs {
                for job in &run.jobs {
                    if matches!(
                        job.conclusion,
                        Some(WorkflowConclusion::Failure)
                            | Some(WorkflowConclusion::TimedOut)
                            | Some(WorkflowConclusion::StartupFailure)
                    ) {
                        first_failed_index = Some(current_idx);
                        break;
                    }
                    current_idx += 1;
                }
                if first_failed_index.is_some() {
                    break;
                }
            }
            app.selected_job_index = first_failed_index.unwrap_or(0);

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

fn open_actions_in_browser(app: &mut App) {
    let url_to_open = if let Some(ref data) = app.actions_data {
        // Find the currently selected job and open its details URL if available
        let mut current_idx = 0;
        let mut found_url: Option<String> = None;
        'outer: for run in &data.workflow_runs {
            for job in &run.jobs {
                if current_idx == app.selected_job_index {
                    // Try job-specific URL first, then fall back to run URL
                    if let Some(ref url) = job.details_url {
                        found_url = Some(url.clone());
                    } else if !run.html_url.is_empty() {
                        found_url = Some(run.html_url.clone());
                    }
                    break 'outer;
                }
                current_idx += 1;
            }
        }
        // If no jobs or selection is out of range, open the first run
        found_url.or_else(|| {
            data.workflow_runs
                .first()
                .filter(|run| !run.html_url.is_empty())
                .map(|run| run.html_url.clone())
        })
    } else {
        None
    };

    if let Some(url) = url_to_open {
        if let Some(display_url) = open_url(&url) {
            app.show_url_popup = Some(display_url);
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
                steps: None,
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
                steps: None,
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
                steps: None,
            });
            app.job_logs_loading = false;
            return None;
        }

        // No annotations or summary, fetch logs
        app.annotations_view = false;
        app.annotations.clear();
        app.job_logs_loading = true;
        app.job_logs = None;

        debug_log("========================================");
        debug_log(&format!(
            "open_job_logs: job.name={}, job.id={}, details_url={:?}",
            job.name, job.id, job.details_url
        ));

        // Check if this is a CircleCI job
        if let Some(ref details_url) = job.details_url {
            debug_log(&format!("  Checking if CircleCI URL: {}", details_url));
            debug_log(&format!(
                "  is_circleci_url={}, is_circleci_configured={}",
                is_circleci_url(details_url),
                is_circleci_configured()
            ));

            if is_circleci_url(details_url) {
                // CircleCI job detected - check if token is configured
                if !is_circleci_configured() {
                    debug_log("  -> CircleCI job but no token configured");
                    app.job_logs_loading = false;
                    app.job_logs = Some(JobLogs {
                        job_id: job.id,
                        job_name: job.name.clone(),
                        content: format!(
                            "{} CircleCI Token Required\n\n\
                            To view CircleCI job logs, set the CIRCLECI_TOKEN environment variable:\n\n\
                            1. Go to CircleCI → User Settings → Personal API Tokens\n\
                            2. Create a new token\n\
                            3. Export it: export CIRCLECI_TOKEN=your_token\n\n\
                            Press 'o' to open this job in your browser instead.",
                            icons::STATUS_ACTION_REQUIRED
                        ),
                        steps: None,
                    });
                    return None;
                }

                let job_number = extract_job_number_from_url(details_url);
                debug_log(&format!("  Extracted job_number: {:?}", job_number));

                if let Some(job_number) = job_number {
                    debug_log(&format!(
                        "  -> Using CircleCI fetch for job_number={}",
                        job_number
                    ));
                    return Some(Command::StartCircleCIJobLogsFetch(
                        owner, repo, job_number, job.name,
                    ));
                } else {
                    debug_log("  -> No job_number extracted, falling back to GitHub CLI");
                }
            }
        } else {
            debug_log("  No details_url, falling back to GitHub CLI");
        }

        // Fall back to GitHub logs via gh CLI
        debug_log(&format!(
            "  -> Using GitHub CLI fetch for job.id={}",
            job.id
        ));
        return Some(Command::StartJobLogsFetch(owner, repo, job.id, job.name));
    }
    None
}

fn close_job_logs(app: &mut App) {
    app.show_job_logs = false;
    app.job_logs = None;
    app.job_logs_loading = false;
    app.job_logs_scroll = 0;
    app.job_logs_selected_step = 0;
    app.job_logs_expanded_steps.clear();
    app.job_logs_selected_sub_step = None;
    app.job_logs_expanded_sub_steps.clear();
    app.annotations_view = false;
    app.annotations.clear();
    app.selected_annotation_index = 0;
    app.selected_annotations.clear();
}

fn handle_job_logs_result(app: &mut App, result: FetchResult) {
    match result {
        FetchResult::JobLogsSuccess(logs) => {
            // Initialize step state for foldable steps
            if let Some(ref steps) = logs.steps {
                // Default expansion state:
                // - Expand failed containers/steps only
                // - Keep successful ones closed
                let expanded: Vec<bool> = steps.iter().map(|step| step.is_failed).collect();

                // Initialize sub-step expansion state - all closed by default
                let expanded_sub_steps: Vec<Vec<bool>> = steps
                    .iter()
                    .map(|step| {
                        if let Some(ref sub_steps) = step.sub_steps {
                            // All sub-steps closed by default
                            vec![false; sub_steps.len()]
                        } else {
                            Vec::new()
                        }
                    })
                    .collect();

                // Select the first failed container, or the first container if none failed
                let selected = steps.iter().position(|s| s.is_failed).unwrap_or(0);

                // If the selected container has sub-steps, find the first failed sub-step
                let selected_sub_step = steps.get(selected).and_then(|step| {
                    step.sub_steps
                        .as_ref()
                        .and_then(|sub_steps| sub_steps.iter().position(|s| s.is_failed))
                });

                app.job_logs_expanded_steps = expanded;
                app.job_logs_expanded_sub_steps = expanded_sub_steps;
                app.job_logs_selected_step = selected;
                app.job_logs_selected_sub_step = selected_sub_step;
            } else {
                app.job_logs_expanded_steps.clear();
                app.job_logs_expanded_sub_steps.clear();
                app.job_logs_selected_step = 0;
                app.job_logs_selected_sub_step = None;
            }
            app.job_logs = Some(logs);
            app.job_logs_loading = false;
            app.job_logs_scroll = 0;
        }
        FetchResult::JobLogsError(e) => {
            app.job_logs_loading = false;
            app.error = Some(format!("Failed to load logs: {}", e));
            app.show_error_popup = true;
        }
        _ => {}
    }
}

fn job_logs_next_step(app: &mut App) {
    if let Some(ref logs) = app.job_logs {
        if let Some(ref steps) = logs.steps {
            let current_step = app.job_logs_selected_step;
            let current_sub = app.job_logs_selected_sub_step;
            let is_expanded = app
                .job_logs_expanded_steps
                .get(current_step)
                .copied()
                .unwrap_or(false);

            // Check if current step has sub-steps
            let has_sub_steps = steps
                .get(current_step)
                .and_then(|s| s.sub_steps.as_ref())
                .map(|ss| !ss.is_empty())
                .unwrap_or(false);

            let sub_steps_len = steps
                .get(current_step)
                .and_then(|s| s.sub_steps.as_ref())
                .map(|ss| ss.len())
                .unwrap_or(0);

            if has_sub_steps && is_expanded {
                // We're in a container with sub-steps that is expanded
                match current_sub {
                    None => {
                        // Currently on container, move to first sub-step
                        app.job_logs_selected_sub_step = Some(0);
                    }
                    Some(sub_idx) if sub_idx < sub_steps_len.saturating_sub(1) => {
                        // Move to next sub-step
                        app.job_logs_selected_sub_step = Some(sub_idx + 1);
                    }
                    Some(_) => {
                        // At last sub-step, move to next container
                        if current_step < steps.len().saturating_sub(1) {
                            app.job_logs_selected_step = current_step + 1;
                            app.job_logs_selected_sub_step = None;
                        }
                    }
                }
            } else {
                // Regular step or collapsed container, move to next step
                if current_step < steps.len().saturating_sub(1) {
                    app.job_logs_selected_step = current_step + 1;
                    app.job_logs_selected_sub_step = None;
                }
            }
        }
    }
}

fn job_logs_prev_step(app: &mut App) {
    if let Some(ref logs) = app.job_logs {
        if let Some(ref steps) = logs.steps {
            let current_step = app.job_logs_selected_step;
            let current_sub = app.job_logs_selected_sub_step;

            match current_sub {
                Some(sub_idx) if sub_idx > 0 => {
                    // Move to previous sub-step
                    app.job_logs_selected_sub_step = Some(sub_idx - 1);
                }
                Some(_) => {
                    // At first sub-step, move to container
                    app.job_logs_selected_sub_step = None;
                }
                None if current_step > 0 => {
                    // At container level, move to previous container
                    let prev_step = current_step - 1;
                    let prev_is_expanded = app
                        .job_logs_expanded_steps
                        .get(prev_step)
                        .copied()
                        .unwrap_or(false);
                    let prev_sub_steps_len = steps
                        .get(prev_step)
                        .and_then(|s| s.sub_steps.as_ref())
                        .map(|ss| ss.len())
                        .unwrap_or(0);

                    app.job_logs_selected_step = prev_step;

                    // If previous container is expanded and has sub-steps, select last sub-step
                    if prev_is_expanded && prev_sub_steps_len > 0 {
                        app.job_logs_selected_sub_step = Some(prev_sub_steps_len - 1);
                    } else {
                        app.job_logs_selected_sub_step = None;
                    }
                }
                None => {
                    // At first container, do nothing
                }
            }
        }
    }
}

fn job_logs_toggle_step(app: &mut App) {
    let step_idx = app.job_logs_selected_step;

    match app.job_logs_selected_sub_step {
        Some(sub_idx) => {
            // Toggle sub-step expansion
            if let Some(sub_expanded) = app.job_logs_expanded_sub_steps.get_mut(step_idx) {
                if let Some(expanded) = sub_expanded.get_mut(sub_idx) {
                    *expanded = !*expanded;
                }
            }
        }
        None => {
            // Toggle container/step expansion
            if let Some(expanded) = app.job_logs_expanded_steps.get_mut(step_idx) {
                *expanded = !*expanded;
            }
        }
    }
}

fn get_selected_step_name(app: &App) -> Option<String> {
    let logs = app.job_logs.as_ref()?;
    let steps = logs.steps.as_ref()?;
    let step = steps.get(app.job_logs_selected_step)?;

    match app.job_logs_selected_sub_step {
        Some(sub_idx) => step
            .sub_steps
            .as_ref()?
            .get(sub_idx)
            .map(|s| s.name.clone()),
        None => Some(step.name.clone()),
    }
}

fn open_step_in_editor(app: &mut App) -> Option<Command> {
    let output = match get_selected_step_output(app) {
        Some(o) if !o.is_empty() && o != "(No output)" => o,
        _ => {
            app.clipboard_feedback = Some("No output to open".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
            return None;
        }
    };

    let step_name = get_selected_step_name(app).unwrap_or_else(|| "step".to_string());
    // Sanitize step name for filename
    let safe_name: String = step_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let filename = format!("ghui_step_{}.log", safe_name);
    Some(Command::OpenInEditor(output, filename))
}

fn copy_job_logs_to_clipboard(app: &mut App) {
    if let Some(ref logs) = app.job_logs {
        if copy_to_clipboard(&logs.content) {
            app.clipboard_feedback = Some("Copied to clipboard!".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
        }
    }
}

/// Get the currently selected step's output
fn get_selected_step_output(app: &App) -> Option<String> {
    let logs = app.job_logs.as_ref()?;
    let steps = logs.steps.as_ref()?;
    let step = steps.get(app.job_logs_selected_step)?;

    match app.job_logs_selected_sub_step {
        Some(sub_idx) => {
            // Get sub-step output
            step.sub_steps
                .as_ref()?
                .get(sub_idx)
                .map(|s| s.output.clone())
        }
        None => {
            // Get container/step output
            if step.sub_steps.is_some() {
                // Container selected - combine all sub-step outputs
                let sub_steps = step.sub_steps.as_ref()?;
                let combined: String = sub_steps
                    .iter()
                    .filter(|s| !s.output.is_empty() && s.output != "(No output)")
                    .map(|s| format!("=== {} ===\n{}", s.name, s.output))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Some(combined)
            } else {
                Some(step.output.clone())
            }
        }
    }
}

/// Extract test failures/errors from Rails/minitest output
fn extract_test_failures(output: &str) -> Option<String> {
    let mut result = Vec::new();
    let mut in_failure = false;
    let mut current_failure = Vec::new();
    let mut summary_line: Option<String> = None;
    let mut exit_line: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        // Detect start of a failure/error block (e.g., "  1) Error:", "  2) Failure:")
        if (trimmed.starts_with("1)")
            || trimmed.starts_with("2)")
            || trimmed.starts_with("3)")
            || trimmed.starts_with("4)")
            || trimmed.starts_with("5)")
            || trimmed.starts_with("6)")
            || trimmed.starts_with("7)")
            || trimmed.starts_with("8)")
            || trimmed.starts_with("9)"))
            && (trimmed.contains("Error:") || trimmed.contains("Failure:"))
        {
            // Save previous failure if any
            if !current_failure.is_empty() {
                result.push(current_failure.join("\n"));
                current_failure.clear();
            }
            in_failure = true;
            current_failure.push(line.to_string());
        } else if in_failure {
            // Check if we've reached the end of the failure block
            // Empty line followed by another failure, or summary line
            if trimmed.is_empty()
                && current_failure
                    .last()
                    .map(|l| l.trim().is_empty())
                    .unwrap_or(false)
            {
                // Two consecutive empty lines - end of block
                result.push(current_failure.join("\n"));
                current_failure.clear();
                in_failure = false;
            } else if trimmed.contains(" runs, ") && trimmed.contains(" assertions, ") {
                // Summary line reached - end of failures
                result.push(current_failure.join("\n"));
                current_failure.clear();
                in_failure = false;
                summary_line = Some(line.to_string());
            } else {
                current_failure.push(line.to_string());
            }
        }

        // Capture summary line (e.g., "4869 runs, 18453 assertions, 1 failures, 2 errors, 0 skips")
        if trimmed.contains(" runs, ")
            && trimmed.contains(" assertions, ")
            && trimmed.contains(" failures, ")
        {
            summary_line = Some(trimmed.to_string());
        }

        // Capture exit status line
        if trimmed.to_lowercase().contains("exit")
            && (trimmed.contains("status") || trimmed.contains("code"))
        {
            exit_line = Some(trimmed.to_string());
        }
    }

    // Don't forget the last failure block
    if !current_failure.is_empty() {
        result.push(current_failure.join("\n"));
    }

    if result.is_empty() && summary_line.is_none() && exit_line.is_none() {
        return None;
    }

    let mut output_parts = result;
    if let Some(summary) = summary_line {
        output_parts.push(format!("\n{}", summary));
    }
    if let Some(exit) = exit_line {
        output_parts.push(exit);
    }

    Some(output_parts.join("\n\n"))
}

/// Smart copy: extract test failures/errors from the selected step
fn smart_copy_step_output(app: &mut App) {
    let output = match get_selected_step_output(app) {
        Some(o) if !o.is_empty() && o != "(No output)" => o,
        _ => {
            app.clipboard_feedback = Some("No output to copy".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
            return;
        }
    };

    // Try to extract test failures
    if let Some(failures) = extract_test_failures(&output) {
        if copy_to_clipboard(&failures) {
            app.clipboard_feedback = Some("Copied to clipboard!".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
            return;
        }
    }

    // Fallback: copy the full output
    if copy_to_clipboard(&output) {
        app.clipboard_feedback = Some("Fallback: copied full output".to_string());
        app.clipboard_feedback_time = std::time::Instant::now();
    }
}

/// Full copy: copy the entire selected step output
fn full_copy_step_output(app: &mut App) {
    let output = match get_selected_step_output(app) {
        Some(o) if !o.is_empty() && o != "(No output)" => o,
        _ => {
            app.clipboard_feedback = Some("No output to copy".to_string());
            app.clipboard_feedback_time = std::time::Instant::now();
            return;
        }
    };

    if copy_to_clipboard(&output) {
        let size = output.len();
        let msg = if size > 10000 {
            format!("Copied {} KB", size / 1024)
        } else {
            "Copied full output".to_string()
        };
        app.clipboard_feedback = Some(msg);
        app.clipboard_feedback_time = std::time::Instant::now();
    } else {
        app.clipboard_feedback = Some("Failed to copy - output too large?".to_string());
        app.clipboard_feedback_time = std::time::Instant::now();
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

/// Detects if running inside a container (devcontainer, Docker, etc.)
fn is_container() -> bool {
    std::env::var("DOCKER_CONTAINER").is_ok()
        || std::path::Path::new("/.dockerenv").exists()
        || std::path::Path::new("/run/.containerenv").exists()
}

/// Copies text to clipboard using OSC 52 escape sequence.
/// Works in terminals that support it (VS Code, iTerm2, Windows Terminal, etc.)
fn copy_via_osc52(text: &str) -> bool {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::Write;

    let encoded = STANDARD.encode(text);
    // OSC 52 sequence: ESC ] 52 ; c ; <base64> BEL
    let sequence = format!("\x1b]52;c;{}\x07", encoded);

    // Write to stdout (terminal)
    let mut stdout = std::io::stdout();
    if stdout.write_all(sequence.as_bytes()).is_err() {
        return false;
    }
    stdout.flush().is_ok()
}

fn copy_to_clipboard(text: &str) -> bool {
    // In containers, use OSC 52 which works through the terminal
    if is_container() {
        return copy_via_osc52(text);
    }

    // Native clipboard commands
    #[cfg(target_os = "macos")]
    let cmd = "pbcopy";
    #[cfg(target_os = "linux")]
    let cmd = "xclip";
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let cmd = "pbcopy"; // fallback

    let mut child = match ProcessCommand::new(cmd)
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            // Fallback to OSC 52 if native command fails
            return copy_via_osc52(text);
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
    }

    child.wait().is_ok()
}

/// Opens a URL in the browser, with container support.
/// Returns Some(url) if the URL should be displayed to the user (in containers).
fn open_url(url: &str) -> Option<String> {
    // In container, return URL for display (user can ctrl+click)
    if is_container() {
        return Some(url.to_string());
    }

    // Native open commands
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let cmd = "open"; // fallback

    let _ = ProcessCommand::new(cmd).arg(url).spawn();
    None
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
