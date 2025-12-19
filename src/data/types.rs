use sea_query::Iden;
use serde::Deserialize;
use std::str::FromStr;

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
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
            CiStatus::Pending => ("● Pending", Color::Yellow),
            CiStatus::Success => ("✓ Passing", Color::Green),
            CiStatus::Failure => ("✗ Failing", Color::Red),
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

#[derive(Debug, Deserialize)]
pub struct CommitData {
    #[serde(rename = "statusCheckRollup")]
    pub status_check_rollup: Option<StatusCheckRollup>,
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
