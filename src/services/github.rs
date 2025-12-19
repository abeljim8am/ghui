use anyhow::Result;
use octocrab::Octocrab;
use std::process::Command;

use crate::data::{CiStatus, PrFilter, PullRequest, SearchGraphQLResponse, SearchNode};
use crate::utils::get_current_repo;

pub fn get_github_token() -> Result<String> {
    let output = Command::new("gh").args(["auth", "token"]).output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get GitHub token. Run 'gh auth login' first.");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_current_user() -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get current user");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn fetch_prs_graphql(filter: PrFilter) -> Result<Vec<PullRequest>> {
    let (owner, repo) =
        get_current_repo().ok_or_else(|| anyhow::anyhow!("Not in a GitHub repository"))?;

    let token = get_github_token()?;
    let octocrab = Octocrab::builder().personal_token(token).build()?;

    // Use search instead of repository.pullRequests + client-side filtering.
    // This avoids missing older PRs when a repo has many open PRs.
    let query_string = match &filter {
        PrFilter::MyPrs => {
            let current_user = get_current_user()?;
            format!(
                "repo:{}/{} is:pr is:open author:{}",
                owner, repo, current_user
            )
        }
        PrFilter::ReviewRequested => {
            let current_user = get_current_user()?;
            format!(
                "repo:{}/{} is:pr is:open review-requested:{}",
                owner, repo, current_user
            )
        }
        PrFilter::Labels(labels) => {
            if labels.is_empty() {
                return Ok(Vec::new());
            }
            // Build query with labels: repo:owner/repo is:pr is:open label:label1 label:label2
            let label_query: String = labels
                .iter()
                .map(|l| format!("label:\"{}\"", l))
                .collect::<Vec<_>>()
                .join(" ");
            format!("repo:{}/{} is:pr is:open {}", owner, repo, label_query)
        }
    };

    let query = r#"
        query($queryString: String!, $after: String) {
            search(query: $queryString, type: ISSUE, first: 100, after: $after) {
                nodes {
                    __typename
                    ... on PullRequest {
                        number
                        title
                        headRefName
                        author {
                            login
                        }
                        commits(last: 1) {
                            nodes {
                                commit {
                                    statusCheckRollup {
                                        state
                                    }
                                }
                            }
                        }
                    }
                }
                pageInfo {
                    hasNextPage
                    endCursor
                }
            }
        }
    "#;

    let mut prs = Vec::new();
    let mut after: Option<String> = None;

    // Cap the number of PRs we'll accumulate to avoid runaway pagination.
    const MAX_RESULTS: usize = 500;

    loop {
        let response: SearchGraphQLResponse = octocrab
            .graphql(&serde_json::json!({
                "query": query,
                "variables": {
                    "queryString": query_string,
                    "after": after
                }
            }))
            .await?;

        for node in response.data.search.nodes {
            let (number, title, head_ref_name, commits, author) = match node {
                SearchNode::PullRequest {
                    number,
                    title,
                    head_ref_name,
                    commits,
                    author,
                } => (number, title, head_ref_name, commits, author),
                SearchNode::Other => continue,
            };

            let ci_status = commits
                .nodes
                .first()
                .and_then(|c| c.commit.status_check_rollup.as_ref())
                .map(|s| s.state.parse().unwrap())
                .unwrap_or(CiStatus::Unknown);

            let author_login = author
                .map(|a| a.login)
                .unwrap_or_else(|| "unknown".to_string());

            prs.push(PullRequest {
                number,
                title,
                branch: head_ref_name,
                repo_owner: owner.clone(),
                repo_name: repo.clone(),
                ci_status,
                author: author_login,
            });
        }

        if prs.len() >= MAX_RESULTS {
            break;
        }

        if !response.data.search.page_info.has_next_page {
            break;
        }

        after = response.data.search.page_info.end_cursor;
        if after.is_none() {
            break;
        }
    }

    Ok(prs)
}
