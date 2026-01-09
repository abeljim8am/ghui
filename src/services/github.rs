use anyhow::Result;
use octocrab::Octocrab;
use std::process::Command;

use crate::data::{
    ActionsData, CheckAnnotation, CiStatus, JobLogs, PrComment, PrFilter, PreviewData, PullRequest,
    SearchGraphQLResponse, SearchNode, WorkflowConclusion, WorkflowJob, WorkflowRun,
    WorkflowStatus,
};
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

    // For Labels filter with multiple labels, we need to fetch each label separately
    // and combine results (GitHub Search doesn't support OR with label: qualifier)
    if let PrFilter::Labels(labels) = &filter {
        if labels.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch PRs for each label separately
        let mut all_prs = Vec::new();
        for label in labels {
            let query_string = format!("repo:{}/{} is:pr is:open label:\"{}\"", owner, repo, label);
            let prs = fetch_prs_for_query(&octocrab, query_string, &owner, &repo).await?;
            all_prs.extend(prs);
        }

        // Deduplicate by PR number
        all_prs.sort_by_key(|pr| pr.number);
        all_prs.dedup_by_key(|pr| pr.number);

        return Ok(all_prs);
    }

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
        PrFilter::Labels(_) => unreachable!(), // Handled above
    };

    fetch_prs_for_query(&octocrab, query_string, &owner, &repo).await
}

/// Helper function to fetch PRs for a given search query
async fn fetch_prs_for_query(
    octocrab: &Octocrab,
    query_string: String,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequest>> {
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
                                    oid
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

            let first_commit = commits.nodes.first();

            let ci_status = first_commit
                .and_then(|c| c.commit.status_check_rollup.as_ref())
                .map(|s| s.state.parse().unwrap())
                .unwrap_or(CiStatus::Unknown);

            let head_sha = first_commit.and_then(|c| c.oid()).map(|s| s.to_string());

            let author_login = author
                .map(|a| a.login)
                .unwrap_or_else(|| "unknown".to_string());

            prs.push(PullRequest {
                number,
                title,
                branch: head_ref_name,
                repo_owner: owner.to_string(),
                repo_name: repo.to_string(),
                ci_status,
                author: author_login,
                head_sha,
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

/// Fetch all checks (GitHub Actions, CircleCI, etc.) for a specific PR
pub async fn fetch_actions_for_pr(
    owner: &str,
    repo: &str,
    pr_number: u64,
    _head_sha: &str, // Not used directly, we fetch via PR number
) -> Result<ActionsData> {
    let token = get_github_token()?;
    let octocrab = Octocrab::builder().personal_token(token).build()?;

    // Use GraphQL to get all check suites and check runs for the PR's latest commit
    // This includes GitHub Actions, CircleCI, and any other CI providers
    let query = r#"
        query($owner: String!, $repo: String!, $prNumber: Int!) {
            repository(owner: $owner, name: $repo) {
                pullRequest(number: $prNumber) {
                    commits(last: 1) {
                        nodes {
                            commit {
                                checkSuites(first: 50) {
                                    nodes {
                                        app {
                                            name
                                        }
                                        conclusion
                                        status
                                        url
                                        checkRuns(first: 50) {
                                            nodes {
                                                databaseId
                                                name
                                                conclusion
                                                status
                                                detailsUrl
                                                startedAt
                                                completedAt
                                                text
                                                summary
                                                annotations(first: 50) {
                                                    nodes {
                                                        path
                                                        location {
                                                            start {
                                                                line
                                                            }
                                                            end {
                                                                line
                                                            }
                                                        }
                                                        annotationLevel
                                                        message
                                                        title
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                status {
                                    contexts {
                                        context
                                        state
                                        targetUrl
                                        createdAt
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let response: serde_json::Value = octocrab
        .graphql(&serde_json::json!({
            "query": query,
            "variables": {
                "owner": owner,
                "repo": repo,
                "prNumber": pr_number as i64
            }
        }))
        .await?;

    let workflow_runs = parse_checks_response(&response)?;

    Ok(ActionsData {
        pr_number,
        workflow_runs,
        error: None,
    })
}

fn parse_checks_response(response: &serde_json::Value) -> Result<Vec<WorkflowRun>> {
    let mut runs = Vec::new();

    let commit = response
        .pointer("/data/repository/pullRequest/commits/nodes/0/commit")
        .ok_or_else(|| anyhow::anyhow!("No commit data found"))?;

    // Parse check suites (GitHub Actions, CircleCI checks, etc.)
    if let Some(check_suites) = commit
        .pointer("/checkSuites/nodes")
        .and_then(|v| v.as_array())
    {
        for (idx, suite) in check_suites.iter().enumerate() {
            let app_name = suite
                .pointer("/app/name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown App")
                .to_string();

            let suite_status = suite
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("QUEUED");

            let suite_conclusion = suite.get("conclusion").and_then(|v| v.as_str());

            let url = suite
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Parse check runs within this suite
            let mut jobs = Vec::new();
            if let Some(check_runs) = suite.pointer("/checkRuns/nodes").and_then(|v| v.as_array()) {
                for run in check_runs {
                    let database_id = run.get("databaseId").and_then(|v| v.as_u64()).unwrap_or(0);

                    let name = run
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let status_str = run
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("QUEUED");

                    let conclusion_str = run.get("conclusion").and_then(|v| v.as_str());

                    let started_at = run
                        .get("startedAt")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let completed_at = run
                        .get("completedAt")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let status = parse_check_status(status_str);
                    let conclusion = conclusion_str.map(parse_check_conclusion);

                    let details_url = run
                        .get("detailsUrl")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let summary = run
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let text = run
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Parse annotations
                    let mut annotations = Vec::new();
                    if let Some(annotation_nodes) =
                        run.pointer("/annotations/nodes").and_then(|v| v.as_array())
                    {
                        for ann in annotation_nodes {
                            let path = ann
                                .get("path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let start_line = ann
                                .pointer("/location/start/line")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;

                            let end_line = ann
                                .pointer("/location/end/line")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(start_line as u64)
                                as u32;

                            let level_str = ann
                                .get("annotationLevel")
                                .and_then(|v| v.as_str())
                                .unwrap_or("NOTICE");

                            let message = ann
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let title = ann
                                .get("title")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            annotations.push(CheckAnnotation {
                                path,
                                start_line,
                                end_line,
                                level: level_str.parse().unwrap(),
                                message,
                                title,
                            });
                        }
                    }

                    jobs.push(WorkflowJob {
                        id: database_id,
                        name,
                        status,
                        conclusion,
                        started_at,
                        completed_at,
                        details_url,
                        summary,
                        text,
                        annotations,
                    });
                }
            }

            // Only add suites that have check runs or are meaningful
            if !jobs.is_empty() {
                let status = parse_check_status(suite_status);
                let conclusion = suite_conclusion.map(parse_check_conclusion);

                runs.push(WorkflowRun {
                    id: idx as u64,
                    name: app_name,
                    status,
                    conclusion,
                    html_url: url,
                    jobs,
                    created_at: String::new(),
                    updated_at: String::new(),
                });
            }
        }
    }

    // Parse legacy commit statuses (some CI systems use this instead of checks)
    if let Some(contexts) = commit
        .pointer("/status/contexts")
        .and_then(|v| v.as_array())
    {
        if !contexts.is_empty() {
            let mut status_jobs = Vec::new();

            for context in contexts {
                let name = context
                    .get("context")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let state = context
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("PENDING");

                let created_at = context
                    .get("createdAt")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let (status, conclusion) = parse_commit_status_state(state);

                let target_url = context
                    .get("targetUrl")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                status_jobs.push(WorkflowJob {
                    id: 0,
                    name,
                    status,
                    conclusion,
                    started_at: created_at,
                    completed_at: None,
                    details_url: target_url,
                    summary: None,
                    text: None,
                    annotations: Vec::new(),
                });
            }

            if !status_jobs.is_empty() {
                // Determine overall status from jobs
                let has_pending = status_jobs.iter().any(|j| {
                    matches!(
                        j.status,
                        WorkflowStatus::Pending | WorkflowStatus::InProgress
                    )
                });
                let has_failure = status_jobs
                    .iter()
                    .any(|j| matches!(j.conclusion, Some(WorkflowConclusion::Failure)));

                let (overall_status, overall_conclusion) = if has_pending {
                    (WorkflowStatus::InProgress, None)
                } else if has_failure {
                    (WorkflowStatus::Completed, Some(WorkflowConclusion::Failure))
                } else {
                    (WorkflowStatus::Completed, Some(WorkflowConclusion::Success))
                };

                runs.push(WorkflowRun {
                    id: 999,
                    name: "Commit Statuses".to_string(),
                    status: overall_status,
                    conclusion: overall_conclusion,
                    html_url: String::new(),
                    jobs: status_jobs,
                    created_at: String::new(),
                    updated_at: String::new(),
                });
            }
        }
    }

    Ok(runs)
}

fn parse_check_status(status: &str) -> WorkflowStatus {
    match status.to_uppercase().as_str() {
        "QUEUED" => WorkflowStatus::Queued,
        "IN_PROGRESS" => WorkflowStatus::InProgress,
        "COMPLETED" => WorkflowStatus::Completed,
        "WAITING" => WorkflowStatus::Waiting,
        "REQUESTED" => WorkflowStatus::Requested,
        "PENDING" => WorkflowStatus::Pending,
        _ => WorkflowStatus::Unknown,
    }
}

fn parse_check_conclusion(conclusion: &str) -> WorkflowConclusion {
    match conclusion.to_uppercase().as_str() {
        "SUCCESS" => WorkflowConclusion::Success,
        "FAILURE" => WorkflowConclusion::Failure,
        "CANCELLED" => WorkflowConclusion::Cancelled,
        "SKIPPED" => WorkflowConclusion::Skipped,
        "TIMED_OUT" => WorkflowConclusion::TimedOut,
        "ACTION_REQUIRED" => WorkflowConclusion::ActionRequired,
        "NEUTRAL" => WorkflowConclusion::Neutral,
        "STALE" => WorkflowConclusion::Stale,
        "STARTUP_FAILURE" => WorkflowConclusion::StartupFailure,
        _ => WorkflowConclusion::None,
    }
}

fn parse_commit_status_state(state: &str) -> (WorkflowStatus, Option<WorkflowConclusion>) {
    match state.to_uppercase().as_str() {
        "PENDING" => (WorkflowStatus::Pending, None),
        "SUCCESS" => (WorkflowStatus::Completed, Some(WorkflowConclusion::Success)),
        "FAILURE" => (WorkflowStatus::Completed, Some(WorkflowConclusion::Failure)),
        "ERROR" => (WorkflowStatus::Completed, Some(WorkflowConclusion::Failure)),
        _ => (WorkflowStatus::Unknown, None),
    }
}

/// Fetch job logs using `gh` CLI (avoids auth complexity)
pub fn fetch_job_logs(owner: &str, repo: &str, job_id: u64, job_name: &str) -> Result<JobLogs> {
    // If job_id is 0, this is likely a third-party check (CircleCI, etc.) without GitHub logs
    if job_id == 0 {
        return Ok(JobLogs {
            job_id,
            job_name: job_name.to_string(),
            content: "No logs available for this check.\n\nPress 'o' to open it in your browser."
                .to_string(),
        });
    }

    // Use gh CLI to fetch job logs - simpler than dealing with GitHub API auth for logs
    let output = Command::new("gh")
        .args([
            "run",
            "view",
            "--repo",
            &format!("{}/{}", owner, repo),
            "--job",
            &job_id.to_string(),
            "--log",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If the job doesn't have logs yet or is a non-GitHub check, return a helpful message
        if stderr.contains("not found") || stderr.contains("no logs") {
            return Ok(JobLogs {
                job_id,
                job_name: job_name.to_string(),
                content: "No logs available for this check.\n\nThe job may not have produced logs yet, or logs may have expired.".to_string(),
            });
        }
        anyhow::bail!("Failed to fetch job logs: {}", stderr);
    }

    let content = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(JobLogs {
        job_id,
        job_name: job_name.to_string(),
        content: if content.is_empty() {
            "No log output available.".to_string()
        } else {
            content
        },
    })
}

/// Fetch PR body and comments for the preview view
pub async fn fetch_pr_preview(owner: &str, repo: &str, pr_number: u64) -> Result<PreviewData> {
    let token = get_github_token()?;
    let octocrab = Octocrab::builder().personal_token(token).build()?;

    // GraphQL query to get PR body, comments, and reviews
    let query = r#"
        query($owner: String!, $repo: String!, $prNumber: Int!) {
            repository(owner: $owner, name: $repo) {
                pullRequest(number: $prNumber) {
                    title
                    body
                    author {
                        login
                    }
                    createdAt
                    comments(first: 100) {
                        nodes {
                            author {
                                login
                            }
                            body
                            createdAt
                        }
                    }
                    reviews(first: 100) {
                        nodes {
                            author {
                                login
                            }
                            body
                            state
                            createdAt
                        }
                    }
                }
            }
        }
    "#;

    let response: serde_json::Value = octocrab
        .graphql(&serde_json::json!({
            "query": query,
            "variables": {
                "owner": owner,
                "repo": repo,
                "prNumber": pr_number as i64
            }
        }))
        .await?;

    let pr = response
        .get("data")
        .and_then(|d| d.get("repository"))
        .and_then(|r| r.get("pullRequest"))
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch PR data"))?;

    let title = pr
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let body = pr
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .to_string();

    let author = pr
        .get("author")
        .and_then(|a| a.get("login"))
        .and_then(|l| l.as_str())
        .unwrap_or("unknown")
        .to_string();

    let created_at = pr
        .get("createdAt")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    let mut comments = Vec::new();

    // Add PR body as first "comment"
    if !body.is_empty() {
        comments.push(PrComment {
            author: author.clone(),
            body,
            created_at: created_at.clone(),
            is_pr_body: true,
        });
    }

    // Collect all comments and reviews with timestamps for sorting
    let mut all_items: Vec<(String, PrComment)> = Vec::new();

    // Add actual comments
    if let Some(comment_nodes) = pr
        .get("comments")
        .and_then(|c| c.get("nodes"))
        .and_then(|n| n.as_array())
    {
        for comment in comment_nodes {
            let comment_author = comment
                .get("author")
                .and_then(|a| a.get("login"))
                .and_then(|l| l.as_str())
                .unwrap_or("unknown")
                .to_string();

            let comment_body = comment
                .get("body")
                .and_then(|b| b.as_str())
                .unwrap_or("")
                .to_string();

            let comment_created = comment
                .get("createdAt")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            if !comment_body.is_empty() {
                all_items.push((
                    comment_created.clone(),
                    PrComment {
                        author: comment_author,
                        body: comment_body,
                        created_at: comment_created,
                        is_pr_body: false,
                    },
                ));
            }
        }
    }

    // Add reviews (Request Changes, Approved, etc.)
    if let Some(review_nodes) = pr
        .get("reviews")
        .and_then(|r| r.get("nodes"))
        .and_then(|n| n.as_array())
    {
        for review in review_nodes {
            let review_author = review
                .get("author")
                .and_then(|a| a.get("login"))
                .and_then(|l| l.as_str())
                .unwrap_or("unknown")
                .to_string();

            let review_body = review
                .get("body")
                .and_then(|b| b.as_str())
                .unwrap_or("")
                .to_string();

            let review_state = review
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let review_created = review
                .get("createdAt")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            // Skip empty "COMMENTED" reviews (just noise)
            if review_state == "COMMENTED" && review_body.is_empty() {
                continue;
            }

            // Format review state for display (using nerdfont icons)
            let state_prefix = match review_state.as_str() {
                "APPROVED" => format!("{} Approved", crate::icons::REVIEW_APPROVED),
                "CHANGES_REQUESTED" => format!(
                    "{} Changes Requested",
                    crate::icons::REVIEW_CHANGES_REQUESTED
                ),
                "COMMENTED" => format!("{} Review", crate::icons::REVIEW_COMMENTED),
                "DISMISSED" => format!("{} Dismissed", crate::icons::REVIEW_DISMISSED),
                _ => continue, // Skip unknown/pending states with no useful info
            };

            // Include reviews even if body is empty (to show the approval/request status)
            let display_body = if review_body.is_empty() {
                format!("_{}_", state_prefix)
            } else {
                format!("**{}**\n\n{}", state_prefix, review_body)
            };

            all_items.push((
                review_created.clone(),
                PrComment {
                    author: review_author,
                    body: display_body,
                    created_at: review_created,
                    is_pr_body: false,
                },
            ));
        }
    }

    // Sort by created_at timestamp
    all_items.sort_by(|a, b| a.0.cmp(&b.0));

    // Add sorted items to comments
    for (_, comment) in all_items {
        comments.push(comment);
    }

    Ok(PreviewData {
        pr_number,
        title,
        comments,
        error: None,
    })
}
