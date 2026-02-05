use crate::data::{ActionsData, JobLogs, PrFilter, PreviewData, PullRequest};

/// Result from an async fetch operation
pub enum FetchResult {
    Success(Vec<PullRequest>, PrFilter),
    Error(String),
    ActionsSuccess(ActionsData),
    ActionsError(String),
    JobLogsSuccess(JobLogs),
    JobLogsError(String),
    PreviewSuccess(PreviewData),
    PreviewError(String),
}

/// Command to be executed after update
pub enum Command {
    Quit,
    StartFetch(PrFilter),
    ExitAfterCheckout,
    StartActionsFetch(String, String, u64, String), // owner, repo, pr_number, head_sha
    StartJobLogsFetch(String, String, u64, String), // owner, repo, job_id, job_name
    StartCircleCIJobLogsFetch(String, String, u64, String), // owner, repo, job_number, job_name
    StartPreviewFetch(String, String, u64),         // owner, repo, pr_number
    OpenInEditor(String, String),                   // content, filename
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
    DismissUrlPopup,

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

    // Workflows view
    OpenWorkflowsView,
    CloseWorkflowsView,
    ActionsDataReceived(FetchResult),
    RefreshActions,
    ActionsNextJob,
    ActionsPreviousJob,
    OpenActionsInBrowser,

    // Job logs
    OpenJobLogs,
    CloseJobLogs,
    JobLogsReceived(FetchResult),
    JobLogsScrollUp,
    JobLogsScrollDown,
    CopyJobLogs,
    JobLogsNextStep,
    JobLogsPrevStep,
    JobLogsToggleStep,
    OpenStepInEditor,    // Enter - open step output in $EDITOR
    CopyTestFailures, // y - copy test failures from API
    FullCopyStepOutput,  // x - copy full step output

    // Annotations view (reviewdog, etc.)
    AnnotationNext,
    AnnotationPrevious,
    ToggleAnnotationSelection,
    CopyAnnotations,

    // Preview view
    OpenPreviewView,
    ClosePreviewView,
    PreviewDataReceived(FetchResult),
    PreviewScrollUp,
    PreviewScrollDown,
    PreviewNextSection,
    PreviewPreviousSection,
    PreviewGoToTop,
    PreviewGoToBottom,

    // Async results
    FetchComplete(FetchResult),

    // System
    Tick,
    Quit,
}
