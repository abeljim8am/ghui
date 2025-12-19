use crate::data::{PrFilter, PullRequest};

/// Result from an async fetch operation
pub enum FetchResult {
    Success(Vec<PullRequest>, PrFilter),
    Error(String),
}

/// Command to be executed after update
pub enum Command {
    Quit,
    StartFetch(PrFilter),
    ExitAfterCheckout,
}

/// All possible messages/events in the application
pub enum Message {
    // Navigation
    NextItem,
    PreviousItem,
    GoToTop,
    GoToBottom,

    // Tab switching
    SwitchTab(PrFilter),

    // Actions
    OpenSelected,
    PromptCheckout,
    ConfirmCheckout,
    CancelCheckout,
    Refresh,

    // Search
    EnterSearchMode,
    ExitSearchMode { clear: bool },
    SearchInput(char),
    SearchBackspace,

    // Popups
    ToggleHelp,
    DismissHelp,
    DismissError,

    // Labels
    OpenLabelsPopup,
    CloseLabelsPopup,
    OpenAddLabelPopup,
    CloseAddLabelPopup,
    LabelInput(char),
    LabelBackspace,
    ToggleLabelScope,
    AddLabel,
    DeleteSelectedLabel,
    LabelsNext,
    LabelsPrevious,

    // Async results
    FetchComplete(FetchResult),

    // System
    Tick,
    Quit,
}
