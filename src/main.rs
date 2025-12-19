use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use ghui::{ui, update, App, Command, Message, PrFilter};

fn main() -> Result<()> {
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
                if handle_command(app, cmd) {
                    return Ok(());
                }
            }
        }

        // Update spinner
        if let Some(cmd) = update(app, Message::Tick) {
            if handle_command(app, cmd) {
                return Ok(());
            }
        }

        // Draw UI
        terminal.draw(|f| ui(f, app))?;

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let msg = key_to_message(app, key.code);
                    if let Some(msg) = msg {
                        if let Some(cmd) = update(app, msg) {
                            if handle_command(app, cmd) {
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
fn handle_command(app: &mut App, cmd: Command) -> bool {
    match cmd {
        Command::Quit => true,
        Command::ExitAfterCheckout => true,
        Command::StartFetch(filter) => {
            app.start_fetch(filter);
            false
        }
    }
}

/// Convert a key press to a message based on current app state
fn key_to_message(app: &App, key: KeyCode) -> Option<Message> {
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
        KeyCode::Char('o') | KeyCode::Enter => Some(Message::OpenSelected),
        KeyCode::Char('c') => Some(Message::PromptCheckout),
        KeyCode::Char('r') => Some(Message::Refresh),
        KeyCode::Char('?') => Some(Message::ToggleHelp),
        KeyCode::Char('l') => Some(Message::OpenLabelsPopup),
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
