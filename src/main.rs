use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use ghui::{ui, update, App, Command, Message, PrFilter};

/// A TUI for GitHub pull requests
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: (),

    /// Clear the local cache and exit
    #[arg(long)]
    clear_cache: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.clear_cache {
        let cache_path = ghui::get_cache_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache path"))?;
        if cache_path.exists() {
            std::fs::remove_file(&cache_path)?;
            eprintln!("Cache cleared: {}", cache_path.display());
        } else {
            eprintln!("No cache file found at: {}", cache_path.display());
        }
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    // Start fetching both lists
    app.start_fetch(PrFilter::MyPrs);
    app.start_fetch(PrFilter::ReviewRequested);

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        // Check for async fetch results
        if let Some(result) = app.check_fetch_result() {
            if let Some(cmd) = update(app, Message::FetchComplete(result)) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Check for actions fetch results
        if let Some(result) = app.check_actions_result() {
            if let Some(cmd) = update(app, Message::ActionsDataReceived(result)) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Check for job logs fetch results
        if let Some(result) = app.check_job_logs_result() {
            if let Some(cmd) = update(app, Message::JobLogsReceived(result)) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Check for CircleCI job logs fetch results
        if let Some(result) = app.check_circleci_logs_result() {
            if let Some(cmd) = update(app, Message::JobLogsReceived(result)) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Check for preview fetch results
        if let Some(result) = app.check_preview_result() {
            if let Some(cmd) = update(app, Message::PreviewDataReceived(result)) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Auto-poll actions if workflows view is open and has pending jobs
        if app.should_poll_actions() {
            if let Some(cmd) = update(app, Message::RefreshActions) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Auto-refresh main page every 30 seconds
        if app.should_refresh_main() {
            if let Some(cmd) = update(app, Message::Refresh) {
                if handle_command(app, cmd, terminal) {
                    return Ok(());
                }
            }
        }

        // Update spinner
        if let Some(cmd) = update(app, Message::Tick) {
            if handle_command(app, cmd, terminal) {
                return Ok(());
            }
        }

        // Draw UI
        terminal.draw(|f| ui(f, app))?;

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let msg = key_to_message(app, key.code, key.modifiers);
                    if let Some(msg) = msg {
                        if let Some(cmd) = update(app, msg) {
                            if handle_command(app, cmd, terminal) {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Handle a command returned from update
fn handle_command(
    app: &mut App,
    cmd: Command,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> bool {
    match cmd {
        Command::Quit => true,
        Command::ExitAfterCheckout => true,
        Command::StartFetch(filter) => {
            app.start_fetch(filter);
            false
        }
        Command::StartActionsFetch(owner, repo, pr_number, head_sha) => {
            app.start_actions_fetch(&owner, &repo, pr_number, &head_sha);
            false
        }
        Command::StartJobLogsFetch(owner, repo, job_id, job_name) => {
            app.start_job_logs_fetch(&owner, &repo, job_id, &job_name);
            false
        }
        Command::StartPreviewFetch(owner, repo, pr_number) => {
            app.start_preview_fetch(&owner, &repo, pr_number);
            false
        }
        Command::StartCircleCIJobLogsFetch(owner, repo, job_number, job_name) => {
            app.start_circleci_logs_fetch(&owner, &repo, job_number, &job_name);
            false
        }
        Command::OpenInEditor(content, filename) => {
            open_in_editor(app, terminal, &content, &filename);
            false
        }
    }
}

/// Open content in $EDITOR, properly suspending and restoring the TUI
fn open_in_editor(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    content: &str,
    filename: &str,
) {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(filename);

    // Write content to temp file
    if let Err(e) = std::fs::write(&temp_file, content) {
        app.clipboard_feedback = Some(format!("Failed to write temp file: {}", e));
        app.clipboard_feedback_time = std::time::Instant::now();
        return;
    }

    // Get editor from $EDITOR or fall back to common editors
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // Leave alternate screen and disable raw mode
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    // Open editor and wait for it to finish
    let result = std::process::Command::new(&editor).arg(&temp_file).status();

    // Re-enter alternate screen and enable raw mode
    let _ = enable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    );
    // Force a full redraw
    let _ = terminal.clear();

    // Handle result and clean up
    match result {
        Ok(_) => {
            let _ = std::fs::remove_file(&temp_file);
        }
        Err(e) => {
            app.clipboard_feedback = Some(format!("Failed to open {}: {}", editor, e));
            app.clipboard_feedback_time = std::time::Instant::now();
            let _ = std::fs::remove_file(&temp_file);
        }
    }
}

/// Convert a key press to a message based on current app state
fn key_to_message(app: &App, key: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    // Help popup - any key dismisses
    if app.show_help_popup {
        return Some(Message::DismissHelp);
    }

    // Checkout popup
    if app.show_checkout_popup {
        return match key {
            KeyCode::Char('y') | KeyCode::Enter => Some(Message::ConfirmCheckout),
            KeyCode::Char('n') | KeyCode::Esc => Some(Message::CancelCheckout),
            _ => None,
        };
    }

    // Error popup
    if app.show_error_popup {
        return match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => Some(Message::DismissError),
            _ => None,
        };
    }

    // URL popup (shown in containers when we can't open browser)
    if app.show_url_popup.is_some() {
        return match key {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => Some(Message::DismissUrlPopup),
            _ => None,
        };
    }

    // Job logs view (nested inside workflows view)
    if app.show_workflows_view && app.show_job_logs {
        // Annotations view has different keybindings
        if app.annotations_view && !app.annotations.is_empty() {
            return match key {
                KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseJobLogs),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::AnnotationNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::AnnotationPrevious),
                KeyCode::Char('v') | KeyCode::Char(' ') => Some(Message::ToggleAnnotationSelection),
                KeyCode::Char('y') => Some(Message::CopyAnnotations),
                KeyCode::Char('o') => Some(Message::OpenActionsInBrowser),
                _ => None,
            };
        }
        // Check if we have foldable steps
        let has_steps = app
            .job_logs
            .as_ref()
            .and_then(|l| l.steps.as_ref())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        if has_steps {
            // Step navigation mode
            return match key {
                KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseJobLogs),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::JobLogsNextStep),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::JobLogsPrevStep),
                KeyCode::Char(' ') => Some(Message::JobLogsToggleStep),
                KeyCode::Enter => Some(Message::OpenStepInEditor),
                KeyCode::Char('y') => Some(Message::CopyTestFailures),
                KeyCode::Char('x') => Some(Message::FullCopyStepOutput),
                KeyCode::Char('o') => Some(Message::OpenActionsInBrowser),
                _ => None,
            };
        }

        // Regular logs view (no steps)
        return match key {
            KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseJobLogs),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::JobLogsScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::JobLogsScrollUp),
            KeyCode::Char('y') => Some(Message::CopyTestFailures),
            KeyCode::Char('x') => Some(Message::FullCopyStepOutput),
            KeyCode::Char('o') => Some(Message::OpenActionsInBrowser),
            _ => None,
        };
    }

    // Workflows view
    if app.show_workflows_view {
        return match key {
            KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseWorkflowsView),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::ActionsNextJob),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::ActionsPreviousJob),
            KeyCode::Char('r') => Some(Message::RefreshActions),
            KeyCode::Char('o') => Some(Message::OpenActionsInBrowser),
            KeyCode::Enter => Some(Message::OpenJobLogs),
            _ => None,
        };
    }

    // Preview view
    if app.show_preview_view {
        // Handle Ctrl+D and Ctrl+U for half-page scrolling
        if modifiers.contains(KeyModifiers::CONTROL) {
            return match key {
                KeyCode::Char('d') => Some(Message::PreviewScrollDown),
                KeyCode::Char('u') => Some(Message::PreviewScrollUp),
                _ => None,
            };
        }
        return match key {
            KeyCode::Esc | KeyCode::Char('q') => Some(Message::ClosePreviewView),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::PreviewScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::PreviewScrollUp),
            KeyCode::Char('g') => Some(Message::PreviewGoToTop),
            KeyCode::Char('G') => Some(Message::PreviewGoToBottom),
            KeyCode::Char('o') => Some(Message::OpenSelected),
            _ => None,
        };
    }

    // Add label popup
    if app.show_add_label_popup {
        return match key {
            KeyCode::Esc => Some(Message::CloseAddLabelPopup),
            KeyCode::Enter => Some(Message::AddLabel),
            KeyCode::Backspace => Some(Message::LabelBackspace),
            KeyCode::Tab => Some(Message::ToggleLabelScope),
            KeyCode::Char(c) => Some(Message::LabelInput(c)),
            _ => None,
        };
    }

    // Labels popup
    if app.show_labels_popup {
        return match key {
            KeyCode::Esc => Some(Message::CloseLabelsPopup),
            KeyCode::Char('a') => Some(Message::OpenAddLabelPopup),
            KeyCode::Char('d') | KeyCode::Backspace => Some(Message::DeleteSelectedLabel),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::LabelsNext),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::LabelsPrevious),
            _ => None,
        };
    }

    // Search mode
    if app.search_mode {
        return match key {
            KeyCode::Esc => Some(Message::ExitSearchMode { clear: true }),
            KeyCode::Enter => Some(Message::ExitSearchMode { clear: false }),
            KeyCode::Backspace => Some(Message::SearchBackspace),
            KeyCode::Char(c) => Some(Message::SearchInput(c)),
            KeyCode::Down | KeyCode::Tab => Some(Message::NextItem),
            KeyCode::Up | KeyCode::BackTab => Some(Message::PreviousItem),
            _ => None,
        };
    }

    // Normal mode
    match key {
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('/') => Some(Message::EnterSearchMode),
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                Some(Message::ExitSearchMode { clear: true })
            } else {
                None
            }
        }
        KeyCode::Char('j') | KeyCode::Down => Some(Message::NextItem),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::PreviousItem),
        KeyCode::Char('o') => Some(Message::OpenSelected),
        KeyCode::Enter => Some(Message::OpenPreviewView),
        KeyCode::Char('c') => Some(Message::PromptCheckout),
        KeyCode::Char('r') => Some(Message::Refresh),
        KeyCode::Char('?') => Some(Message::ToggleHelp),
        KeyCode::Char('l') => Some(Message::OpenLabelsPopup),
        KeyCode::Char('w') => Some(Message::OpenWorkflowsView),
        KeyCode::Char('p') => Some(Message::OpenPreviewView),
        KeyCode::Char('1') => Some(Message::SwitchTab(PrFilter::MyPrs)),
        KeyCode::Char('2') => Some(Message::SwitchTab(PrFilter::ReviewRequested)),
        KeyCode::Char('3') => {
            let labels = app.get_active_labels();
            Some(Message::SwitchTab(PrFilter::Labels(labels)))
        }
        KeyCode::Char('g') => Some(Message::GoToTop),
        KeyCode::Char('G') => Some(Message::GoToBottom),
        _ => None,
    }
}
