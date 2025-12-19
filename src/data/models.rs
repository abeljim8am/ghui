use super::types::CiStatus;

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub ci_status: CiStatus,
    pub author: String,
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LabelFilter {
    pub id: i64,
    pub label_name: String,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
}

impl LabelFilter {
    pub fn is_global(&self) -> bool {
        self.repo_owner.is_none() && self.repo_name.is_none()
    }
}
