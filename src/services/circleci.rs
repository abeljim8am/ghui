use anyhow::Result;
use serde::Deserialize;
use std::env;
use strip_ansi_escapes::strip_str;

use crate::data::{JobLogs, JobStep, WorkflowConclusion, WorkflowJob, WorkflowRun, WorkflowStatus};

const CIRCLECI_API_V2_BASE: &str = "https://circleci.com/api/v2";
const CIRCLECI_API_V1_BASE: &str = "https://circleci.com/api/v1.1";

/// Get CircleCI API token from environment
pub fn get_circleci_token() -> Option<String> {
    env::var("CIRCLECI_TOKEN").ok()
}

/// Check if CircleCI is configured (token available)
pub fn is_circleci_configured() -> bool {
    get_circleci_token().is_some()
}

// API Response types - allow unused fields as they're part of the API schema

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PipelineListResponse {
    items: Vec<Pipeline>,
    next_page_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Pipeline {
    id: String,
    number: u64,
    state: String,
    created_at: String,
    updated_at: Option<String>,
    trigger: Option<PipelineTrigger>,
    vcs: Option<PipelineVcs>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PipelineTrigger {
    #[serde(rename = "type")]
    trigger_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PipelineVcs {
    branch: Option<String>,
    revision: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct WorkflowListResponse {
    items: Vec<CircleCIWorkflow>,
    next_page_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CircleCIWorkflow {
    id: String,
    name: String,
    status: String,
    created_at: String,
    stopped_at: Option<String>,
    pipeline_id: String,
    pipeline_number: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JobListResponse {
    items: Vec<CircleCIJob>,
    next_page_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CircleCIJob {
    id: String,
    job_number: Option<u64>,
    name: String,
    status: String,
    started_at: Option<String>,
    stopped_at: Option<String>,
    #[serde(rename = "type")]
    job_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JobDetailsResponse {
    web_url: Option<String>,
    name: String,
    status: String,
    started_at: Option<String>,
    stopped_at: Option<String>,
    duration: Option<u64>,
    messages: Option<Vec<JobMessage>>,
    contexts: Option<Vec<JobContext>>,
    steps: Option<Vec<V2JobStep>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JobMessage {
    #[serde(rename = "type")]
    message_type: Option<String>,
    message: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JobContext {
    name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct V2JobStep {
    name: String,
    status: String,
    actions: Vec<StepAction>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct StepAction {
    name: String,
    status: String,
    index: u64,
    step: Option<u64>,
    #[serde(rename = "type")]
    action_type: Option<String>,
    output_url: Option<String>,
    run_time_millis: Option<u64>,
    has_output: Option<bool>,
}

// V1.1 API Response types (for job step output)

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct V1JobDetails {
    steps: Option<Vec<V1Step>>,
    status: Option<String>,
    build_url: Option<String>,
    build_num: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct V1Step {
    name: String,
    actions: Vec<V1Action>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct V1Action {
    name: Option<String>,
    status: Option<String>,
    output_url: Option<String>,
    index: Option<u64>,
    step: Option<u64>,
    run_time_millis: Option<u64>,
    #[serde(rename = "type")]
    action_type: Option<String>,
    exit_code: Option<i32>,
    has_output: Option<bool>,
}

/// Create an HTTP client with CircleCI auth headers
fn create_client(token: &str) -> Result<reqwest::Client> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};

    let mut headers = HeaderMap::new();
    headers.insert("Circle-Token", HeaderValue::from_str(token)?);
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()?)
}

/// Get the project slug for CircleCI API (e.g., "github/owner/repo")
/// Note: CircleCI API v2 accepts both "gh" and "github" but "github" is more common
fn get_project_slug(owner: &str, repo: &str) -> String {
    format!("github/{}/{}", owner, repo)
}

/// Fetch pipelines for a project, optionally filtered by branch
async fn fetch_pipelines(owner: &str, repo: &str, branch: Option<&str>) -> Result<Vec<Pipeline>> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let project_slug = get_project_slug(owner, repo);

    let mut url = format!("{}/project/{}/pipeline", CIRCLECI_API_V2_BASE, project_slug);
    if let Some(b) = branch {
        url = format!("{}?branch={}", url, urlencoding::encode(b));
    }

    let response: PipelineListResponse = client.get(&url).send().await?.json().await?;

    Ok(response.items)
}

/// Fetch workflows for a specific pipeline
async fn fetch_pipeline_workflows(pipeline_id: &str) -> Result<Vec<CircleCIWorkflow>> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let url = format!("{}/pipeline/{}/workflow", CIRCLECI_API_V2_BASE, pipeline_id);

    let response: WorkflowListResponse = client.get(&url).send().await?.json().await?;

    Ok(response.items)
}

/// Fetch jobs for a specific workflow
async fn fetch_workflow_jobs(workflow_id: &str) -> Result<Vec<CircleCIJob>> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let url = format!("{}/workflow/{}/job", CIRCLECI_API_V2_BASE, workflow_id);

    let response: JobListResponse = client.get(&url).send().await?.json().await?;

    Ok(response.items)
}

/// Fetch job details including steps (v2 API - limited info)
#[allow(dead_code)]
async fn fetch_job_details_v2(owner: &str, repo: &str, job_number: u64) -> Result<JobDetailsResponse> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let project_slug = get_project_slug(owner, repo);
    let url = format!(
        "{}/project/{}/job/{}",
        CIRCLECI_API_V2_BASE, project_slug, job_number
    );

    let response: JobDetailsResponse = client.get(&url).send().await?.json().await?;

    Ok(response)
}

/// Fetch job details including steps with output URLs (v1.1 API - has step output)
async fn fetch_job_details_v1(owner: &str, repo: &str, job_number: u64) -> Result<V1JobDetails> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let url = format!(
        "{}/project/github/{}/{}/{}",
        CIRCLECI_API_V1_BASE, owner, repo, job_number
    );

    let response: V1JobDetails = client.get(&url).send().await?.json().await?;

    Ok(response)
}

/// Fetch step output from CircleCI (if available)
async fn fetch_step_output(output_url: &str, _token: &str) -> Result<String> {
    // CircleCI output URLs are S3 presigned URLs, they don't need auth
    let response = reqwest::get(output_url).await?;

    if !response.status().is_success() {
        return Ok(String::new());
    }

    // CircleCI returns output as JSON array of objects
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct OutputLine {
        message: Option<String>,
        #[serde(rename = "type")]
        output_type: Option<String>,
    }

    let text = response.text().await?;

    // Try to parse as JSON array first
    if let Ok(lines) = serde_json::from_str::<Vec<OutputLine>>(&text) {
        let output: String = lines
            .into_iter()
            .filter_map(|l| l.message)
            .collect::<Vec<_>>()
            .join("");
        if !output.is_empty() {
            // Strip ANSI escape codes to prevent display artifacts
            return Ok(strip_str(&output).to_string());
        }
    }

    // Try parsing as a single JSON object with "message" field
    #[derive(Deserialize)]
    struct SingleOutput {
        message: Option<String>,
        output: Option<String>,
    }
    if let Ok(single) = serde_json::from_str::<SingleOutput>(&text) {
        if let Some(msg) = single.message.or(single.output) {
            // Strip ANSI escape codes to prevent display artifacts
            return Ok(strip_str(&msg).to_string());
        }
    }

    // Fall back to raw text (might be plain text logs)
    if !text.trim().is_empty() && !text.starts_with('{') && !text.starts_with('[') {
        // Strip ANSI escape codes to prevent display artifacts
        return Ok(strip_str(&text).to_string());
    }

    Ok(String::new())
}

/// Fetch job logs from CircleCI by extracting step outputs using v1.1 API
/// Returns structured steps for foldable display
pub async fn fetch_circleci_job_logs(
    owner: &str,
    repo: &str,
    job_number: u64,
    job_name: &str,
) -> Result<JobLogs> {
    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    // Use v1.1 API which includes step output URLs
    let details = fetch_job_details_v1(owner, repo, job_number).await?;

    let mut structured_steps: Vec<JobStep> = Vec::new();

    if let Some(steps) = details.steps {
        for step in steps {
            // Get status and exit code from the first action
            let first_action = step.actions.first();
            let step_status = first_action
                .and_then(|a| a.status.as_deref())
                .unwrap_or("unknown")
                .to_string();
            let exit_code = first_action.and_then(|a| a.exit_code);

            let is_failed = step_status.to_lowercase() == "failed"
                || step_status.to_lowercase() == "timedout"
                || exit_code.map(|c| c != 0).unwrap_or(false);

            // Fetch output for this step from all actions
            let mut output = String::new();
            for action in &step.actions {
                // Add action name if present and different from step name
                if let Some(ref action_name) = action.name {
                    if action_name != &step.name && !action_name.is_empty() {
                        output.push_str(&format!(">> {}\n", action_name));
                    }
                }

                // Try to fetch output if URL is available
                if let Some(output_url) = &action.output_url {
                    match fetch_step_output(output_url, &token).await {
                        Ok(step_output) if !step_output.is_empty() => {
                            output.push_str(step_output.trim());
                            if !output.ends_with('\n') {
                                output.push('\n');
                            }
                        }
                        Ok(_) => {
                            // No output from URL, show exit code if available
                            if let Some(code) = action.exit_code {
                                if code != 0 {
                                    output.push_str(&format!("Exit code: {}\n", code));
                                }
                            }
                        }
                        Err(e) => {
                            output.push_str(&format!("(Failed to fetch output: {})\n", e));
                        }
                    }
                } else if let Some(code) = action.exit_code {
                    // No output URL, show exit code
                    if code != 0 || is_failed {
                        output.push_str(&format!("Exit code: {}\n", code));
                    }
                }
            }

            // If still no output, provide helpful message
            if output.trim().is_empty() {
                if is_failed {
                    output = format!(
                        "Step failed with status: {}\nExit code: {}\n\nPress 'o' to view in browser for full details.",
                        step_status,
                        exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string())
                    );
                } else {
                    output = "(No output)".to_string();
                }
            }

            // Check last line for exit status to detect failures (CI sometimes reports success even when commands fail)
            let output_indicates_failure = {
                let last_line = output.trim().lines().last().unwrap_or("").to_lowercase();
                (last_line.contains("exit status") || last_line.contains("exited with code"))
                    && !last_line.contains("exit status 0")
                    && !last_line.contains("code 0")
            };

            let is_failed = is_failed || output_indicates_failure;

            structured_steps.push(JobStep {
                name: step.name,
                status: if is_failed && step_status.to_lowercase() == "success" {
                    "failed".to_string()
                } else {
                    step_status
                },
                output,
                is_failed,
            });
        }
    }

    // Also create a fallback plain-text content for non-step views
    let content = if structured_steps.is_empty() {
        "No step information available.\n\nPress 'o' to open it in your browser.".to_string()
    } else {
        // Summary for plain text view
        let failed_count = structured_steps.iter().filter(|s| s.is_failed).count();
        let total = structured_steps.len();
        format!(
            "{} steps ({} passed, {} failed)\n\nUse j/k to navigate, Enter to expand/collapse",
            total,
            total - failed_count,
            failed_count
        )
    };

    Ok(JobLogs {
        job_id: job_number,
        job_name: job_name.to_string(),
        content,
        steps: if structured_steps.is_empty() {
            None
        } else {
            Some(structured_steps)
        },
    })
}

/// Convert CircleCI status string to WorkflowStatus
fn parse_circleci_status(status: &str) -> WorkflowStatus {
    match status.to_lowercase().as_str() {
        "running" => WorkflowStatus::InProgress,
        "success" => WorkflowStatus::Completed,
        "failed" | "failing" => WorkflowStatus::Completed,
        "canceled" | "cancelled" => WorkflowStatus::Completed,
        "on_hold" | "blocked" => WorkflowStatus::Waiting,
        "queued" | "not_run" => WorkflowStatus::Queued,
        "infrastructure_fail" | "timedout" => WorkflowStatus::Completed,
        _ => WorkflowStatus::Unknown,
    }
}

/// Convert CircleCI status to WorkflowConclusion
fn parse_circleci_conclusion(status: &str) -> Option<WorkflowConclusion> {
    match status.to_lowercase().as_str() {
        "success" => Some(WorkflowConclusion::Success),
        "failed" | "failing" => Some(WorkflowConclusion::Failure),
        "canceled" | "cancelled" => Some(WorkflowConclusion::Cancelled),
        "timedout" => Some(WorkflowConclusion::TimedOut),
        "infrastructure_fail" => Some(WorkflowConclusion::StartupFailure),
        "not_run" => Some(WorkflowConclusion::Skipped),
        "running" | "queued" | "on_hold" | "blocked" => None,
        _ => None,
    }
}

/// Fetch CircleCI workflows and jobs for a branch as WorkflowRun structures
/// This can be used to augment or replace GitHub check data with direct CircleCI data
pub async fn fetch_circleci_workflows_for_branch(
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<WorkflowRun>> {
    // Get recent pipelines for this branch
    let pipelines = fetch_pipelines(owner, repo, Some(branch)).await?;

    // Only process the most recent pipeline (pipelines are returned newest first)
    let latest_pipeline = match pipelines.into_iter().next() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    // Fetch workflows for this pipeline
    let workflows = fetch_pipeline_workflows(&latest_pipeline.id).await?;

    let mut workflow_runs = Vec::new();

    for workflow in workflows {
        // Fetch jobs for this workflow
        let jobs = match fetch_workflow_jobs(&workflow.id).await {
            Ok(j) => j,
            Err(_) => continue,
        };

        let workflow_jobs: Vec<WorkflowJob> = jobs
            .into_iter()
            .map(|job| {
                WorkflowJob {
                    id: job.job_number.unwrap_or(0),
                    name: job.name,
                    status: parse_circleci_status(&job.status),
                    conclusion: parse_circleci_conclusion(&job.status),
                    started_at: job.started_at,
                    completed_at: job.stopped_at,
                    details_url: None, // CircleCI doesn't provide this in job list
                    summary: None,
                    text: None,
                    annotations: Vec::new(),
                }
            })
            .collect();

        let workflow_run = WorkflowRun {
            id: latest_pipeline.number,
            name: format!("CircleCI: {}", workflow.name),
            status: parse_circleci_status(&workflow.status),
            conclusion: parse_circleci_conclusion(&workflow.status),
            html_url: format!(
                "https://app.circleci.com/pipelines/gh/{}/{}/{}/workflows/{}",
                owner, repo, latest_pipeline.number, workflow.id
            ),
            jobs: workflow_jobs,
            created_at: workflow.created_at,
            updated_at: workflow.stopped_at.unwrap_or_default(),
        };

        workflow_runs.push(workflow_run);
    }

    Ok(workflow_runs)
}

/// Extract CircleCI job number from a details URL
/// CircleCI URLs can be in various formats:
/// - https://circleci.com/gh/owner/repo/123 (legacy)
/// - https://circleci.com/gh/owner/repo/123?query=param (legacy with query)
/// - https://app.circleci.com/pipelines/gh/owner/repo/123/workflows/abc/jobs/456
/// - https://app.circleci.com/pipelines/github/owner/repo/123/workflows/abc/jobs/456
pub fn extract_job_number_from_url(url: &str) -> Option<u64> {
    if !url.contains("circleci.com") {
        return None;
    }

    // Remove query string and fragment
    let url_path = url.split('?').next().unwrap_or(url);
    let url_path = url_path.split('#').next().unwrap_or(url_path);

    // Pattern 1: New format with /jobs/ path
    // https://app.circleci.com/pipelines/gh/owner/repo/123/workflows/abc/jobs/456
    if let Some(jobs_idx) = url_path.find("/jobs/") {
        let after_jobs = &url_path[jobs_idx + 6..];
        // Extract digits until we hit a non-digit or end
        let num_str: String = after_jobs.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num_str.is_empty() {
            return num_str.parse().ok();
        }
    }

    // Pattern 2: Legacy format - last numeric path segment
    // https://circleci.com/gh/owner/repo/123
    let segments: Vec<&str> = url_path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // Look for the last numeric segment (might be build number)
    for segment in segments.iter().rev() {
        if let Ok(num) = segment.parse::<u64>() {
            return Some(num);
        }
    }

    None
}

/// Check if a URL is a CircleCI URL
pub fn is_circleci_url(url: &str) -> bool {
    url.contains("circleci.com")
}
