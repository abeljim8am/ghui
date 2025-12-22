use sea_query::Iden;
use serde::Deserialize;
use std::str::FromStr;

use crate::icons;

pub const CACHE_VERSION: i32 = 5;

// Database table identifiers
#[derive(Iden)]
pub enum CacheMeta {
    Table,
    Key,
    Value,
}

#[derive(Iden)]
pub enum PullRequestsTable {
    Table,
    Number,
    Title,
    Branch,
    RepoOwner,
    RepoName,
    CiStatus,
    Filter,
    Author,
}

#[derive(Iden)]
pub enum LabelFiltersTable {
    Table,
    Id,
    LabelName,
    RepoOwner,
    RepoName,
}

// CI Status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CiStatus {
    Unknown,
    Pending,
    Success,
    Failure,
}

impl CiStatus {
    pub fn display(self) -> (&'static str, ratatui::style::Color) {
        use ratatui::style::Color;
        match self {
            CiStatus::Unknown => ("N/A", Color::DarkGray),
            CiStatus::Pending => (icons::CI_PENDING_DISPLAY, Color::Yellow),
            CiStatus::Success => (icons::CI_SUCCESS_DISPLAY, Color::Green),
            CiStatus::Failure => (icons::CI_FAILURE_DISPLAY, Color::Red),
        }
    }

    pub fn to_str(self) -> &'static str {
        match self {
            CiStatus::Unknown => "unknown",
            CiStatus::Pending => "pending",
            CiStatus::Success => "success",
            CiStatus::Failure => "failure",
        }
    }
}

impl FromStr for CiStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "PENDING" => CiStatus::Pending,
            "SUCCESS" => CiStatus::Success,
            "FAILURE" | "ERROR" => CiStatus::Failure,
            _ => CiStatus::Unknown,
        })
    }
}

// PR Filter
#[derive(Debug, Clone, PartialEq)]
pub enum PrFilter {
    MyPrs,
    ReviewRequested,
    Labels(Vec<String>),
}

impl PrFilter {
    pub fn to_str(&self) -> &'static str {
        match self {
            PrFilter::MyPrs => "my_prs",
            PrFilter::ReviewRequested => "review_requested",
            PrFilter::Labels(_) => "labels",
        }
    }
}

// GraphQL response types
#[derive(Debug, Deserialize)]
pub struct CommitConnection {
    pub nodes: Vec<CommitNode>,
}

#[derive(Debug, Deserialize)]
pub struct CommitNode {
    pub commit: CommitData,
}

impl CommitNode {
    pub fn oid(&self) -> Option<&str> {
        self.commit.oid.as_deref()
    }
}

#[derive(Debug, Deserialize)]
pub struct CommitData {
    #[serde(rename = "statusCheckRollup")]
    pub status_check_rollup: Option<StatusCheckRollup>,
    pub oid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatusCheckRollup {
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchGraphQLResponse {
    pub data: SearchGraphQLData,
}

#[derive(Debug, Deserialize)]
pub struct SearchGraphQLData {
    pub search: SearchConnection,
}

#[derive(Debug, Deserialize)]
pub struct SearchConnection {
    pub nodes: Vec<SearchNode>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
pub enum SearchNode {
    PullRequest {
        number: u64,
        title: String,
        #[serde(rename = "headRefName")]
        head_ref_name: String,
        commits: CommitConnection,
        author: Option<Author>,
    },
    #[serde(other)]
    Other,
}

// GitHub Actions types

/// Workflow run status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkflowStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    Requested,
    Pending,
    Unknown,
}

impl FromStr for WorkflowStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "queued" => WorkflowStatus::Queued,
            "in_progress" => WorkflowStatus::InProgress,
            "completed" => WorkflowStatus::Completed,
            "waiting" => WorkflowStatus::Waiting,
            "requested" => WorkflowStatus::Requested,
            "pending" => WorkflowStatus::Pending,
            _ => WorkflowStatus::Unknown,
        })
    }
}

/// Workflow run conclusion
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkflowConclusion {
    Success,
    Failure,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Neutral,
    Stale,
    StartupFailure,
    None,
}

impl FromStr for WorkflowConclusion {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "success" => WorkflowConclusion::Success,
            "failure" => WorkflowConclusion::Failure,
            "cancelled" => WorkflowConclusion::Cancelled,
            "skipped" => WorkflowConclusion::Skipped,
            "timed_out" => WorkflowConclusion::TimedOut,
            "action_required" => WorkflowConclusion::ActionRequired,
            "neutral" => WorkflowConclusion::Neutral,
            "stale" => WorkflowConclusion::Stale,
            "startup_failure" => WorkflowConclusion::StartupFailure,
            _ => WorkflowConclusion::None,
        })
    }
}

/// Annotation level for check annotations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationLevel {
    Notice,
    Warning,
    Failure,
}

impl FromStr for AnnotationLevel {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "WARNING" => AnnotationLevel::Warning,
            "FAILURE" => AnnotationLevel::Failure,
            _ => AnnotationLevel::Notice,
        })
    }
}

/// A check annotation (e.g., from reviewdog)
#[derive(Debug, Clone)]
pub struct CheckAnnotation {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub level: AnnotationLevel,
    pub message: String,
    pub title: Option<String>,
}

/// A job within a workflow run
#[derive(Debug, Clone)]
pub struct WorkflowJob {
    pub id: u64,
    pub name: String,
    pub status: WorkflowStatus,
    pub conclusion: Option<WorkflowConclusion>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub details_url: Option<String>,
    pub summary: Option<String>,
    pub text: Option<String>,
    pub annotations: Vec<CheckAnnotation>,
}

/// Log content for a job
#[derive(Debug, Clone)]
pub struct JobLogs {
    pub job_id: u64,
    pub job_name: String,
    pub content: String,
}

/// A workflow run (e.g., "CI", "Tests")
#[derive(Debug, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: WorkflowStatus,
    pub conclusion: Option<WorkflowConclusion>,
    pub html_url: String,
    pub jobs: Vec<WorkflowJob>,
    pub created_at: String,
    pub updated_at: String,
}

/// Container for all actions data for a PR
#[derive(Debug, Clone)]
pub struct ActionsData {
    pub pr_number: u64,
    pub workflow_runs: Vec<WorkflowRun>,
    pub error: Option<String>,
}

/// A comment on a PR (either PR body or review comment)
#[derive(Debug, Clone)]
pub struct PrComment {
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub is_pr_body: bool,
}

/// Container for PR preview data (description + comments)
#[derive(Debug, Clone)]
pub struct PreviewData {
    pub pr_number: u64,
    pub title: String,
    pub comments: Vec<PrComment>,
    pub error: Option<String>,
}
