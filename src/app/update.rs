use ratatui::widgets::TableState;
use std::process::Command as ProcessCommand;

use crate::data::PrFilter;
use crate::services::{delete_label_filter, filter_prs, load_label_filters, save_label_filter};
use crate::utils::checkout_branch;

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

        // Async results
        Message::FetchComplete(result) => {
            handle_fetch_result(app, result);
            None
        }

        // System
        Message::Tick => {
            if app.loading_my_prs || app.loading_review_prs || app.loading_labels_prs {
                app.update_spinner();
            }
            None
        }
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

fn handle_fetch_result(app: &mut App, result: FetchResult) {
    match result {
        FetchResult::Success(new_prs, filter) => {
            let is_current_filter = matches!(
                (&app.pr_filter, &filter),
                (PrFilter::MyPrs, PrFilter::MyPrs)
                    | (PrFilter::ReviewRequested, PrFilter::ReviewRequested)
                    | (PrFilter::Labels(_), PrFilter::Labels(_))
            );

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
        }
        FetchResult::Error(e) => {
            app.error = Some(e);
            app.show_error_popup = true;
            app.loading_my_prs = false;
            app.loading_review_prs = false;
            app.loading_labels_prs = false;
        }
    }
}
