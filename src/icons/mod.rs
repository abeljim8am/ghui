//! Icons and emoji constants used throughout the UI.

// Spinner animation frames (braille characters)
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// CI Status icons
pub const CI_PENDING: &str = "●";
pub const CI_SUCCESS: &str = "✓";
pub const CI_FAILURE: &str = "✗";

// CI Status display strings (icon + text)
pub const CI_PENDING_DISPLAY: &str = "● Pending";
pub const CI_SUCCESS_DISPLAY: &str = "✓ Passing";
pub const CI_FAILURE_DISPLAY: &str = "✗ Failing";

// Selection/Navigation indicators
pub const SELECTOR: &str = "▶ ";
pub const SELECTOR_INDENTED: &str = "  ▶ ";
pub const SELECTOR_MARKED: &str = "▶● ";
pub const SELECTOR_ONLY: &str = "▶  ";
pub const MARKED_ONLY: &str = " ● ";
pub const UNMARKED: &str = "   ";

// Cursor
pub const CURSOR: &str = "█";

// Status icons for workflows
pub const STATUS_SUCCESS: &str = "✓";
pub const STATUS_FAILURE: &str = "✗";
pub const STATUS_CANCELLED: &str = "◯";
pub const STATUS_SKIPPED: &str = "-";
pub const STATUS_TIMED_OUT: &str = "⏱";
pub const STATUS_ACTION_REQUIRED: &str = "!";
pub const STATUS_IN_PROGRESS: &str = "●";
pub const STATUS_QUEUED: &str = "○";
pub const STATUS_WAITING: &str = "⋯";
pub const STATUS_UNKNOWN: &str = "?";

// Annotation level icons
pub const ANNOTATION_FAILURE: &str = "  ";
pub const ANNOTATION_WARNING: &str = "  ";
pub const ANNOTATION_NOTICE: &str = "󰋽  ";

// Annotation level labels (for formatted output)
pub const ANNOTATION_FAILURE_LABEL: &str = " FAILURE";
pub const ANNOTATION_WARNING_LABEL: &str = "  WARNING";
pub const ANNOTATION_NOTICE_LABEL: &str = "󰋽 NOTICE";

// Emoji icons for annotations
pub const EMOJI_FILE: &str = "  ";
pub const EMOJI_PIN: &str = "  ";
pub const EMOJI_MESSAGE: &str = "󰻞  ";

// List/UI elements
pub const BULLET: &str = "•";
pub const SEPARATOR_CHAR: &str = "─";
