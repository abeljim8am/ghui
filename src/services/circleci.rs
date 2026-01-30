use anyhow::Result;
use futures::future::join_all;
use serde::Deserialize;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use strip_ansi_escapes::strip_str;

use crate::data::{JobLogs, JobStep, WorkflowConclusion, WorkflowJob, WorkflowRun, WorkflowStatus};

const CIRCLECI_API_V2_BASE: &str = "https://circleci.com/api/v2";
const CIRCLECI_API_V1_BASE: &str = "https://circleci.com/api/v1.1";

/// Debug logging to /tmp/ghui_circleci_debug.log
fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/ghui_circleci_debug.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

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
    debug_log(&format!(
        "fetch_job_details_v1: owner={}, repo={}, job_number={}",
        owner, repo, job_number
    ));

    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    let client = create_client(&token)?;
    let url = format!(
        "{}/project/github/{}/{}/{}",
        CIRCLECI_API_V1_BASE, owner, repo, job_number
    );
    debug_log(&format!("  API URL: {}", url));

    let response: V1JobDetails = client.get(&url).send().await?.json().await?;

    debug_log(&format!(
        "  Response: build_num={:?}, status={:?}, steps_count={}",
        response.build_num,
        response.status,
        response.steps.as_ref().map(|s| s.len()).unwrap_or(0)
    ));

    // Log step names for debugging
    if let Some(ref steps) = response.steps {
        for (i, step) in steps.iter().enumerate() {
            debug_log(&format!(
                "    Step {}: name='{}', actions_count={}",
                i,
                step.name,
                step.actions.len()
            ));
        }
    }

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
    debug_log("========================================");
    debug_log(&format!(
        "fetch_circleci_job_logs: owner={}, repo={}, job_number={}, job_name={}",
        owner, repo, job_number, job_name
    ));

    let token = get_circleci_token()
        .ok_or_else(|| anyhow::anyhow!("CIRCLECI_TOKEN environment variable not set"))?;

    // Use v1.1 API which includes step output URLs
    let details = fetch_job_details_v1(owner, repo, job_number).await?;

    let mut structured_steps: Vec<JobStep> = Vec::new();

    if let Some(steps) = details.steps {
        // Determine if this is a parallel job (multiple actions per step)
        let num_containers = steps.first().map(|s| s.actions.len()).unwrap_or(1);
        let is_parallel = num_containers > 1;

        debug_log(&format!(
            "Processing job: is_parallel={}, num_containers={}, num_steps={}",
            is_parallel, num_containers, steps.len()
        ));

        // First pass: collect all URLs that need fetching with their coordinates
        // (step_index, action_index, url)
        let mut fetch_tasks: Vec<(usize, usize, String)> = Vec::new();
        for (step_idx, step) in steps.iter().enumerate() {
            for (action_idx, action) in step.actions.iter().enumerate() {
                if let Some(url) = &action.output_url {
                    fetch_tasks.push((step_idx, action_idx, url.clone()));
                }
            }
        }

        // Fetch all outputs in parallel
        let fetch_futures = fetch_tasks.iter().map(|(_, _, url)| {
            let url = url.clone();
            let token = token.clone();
            async move { fetch_step_output(&url, &token).await }
        });
        let fetch_results: Vec<Result<String>> = join_all(fetch_futures).await;

        // Build a map of (step_idx, action_idx) -> fetched output
        let mut output_map: std::collections::HashMap<(usize, usize), Result<String>> =
            std::collections::HashMap::new();
        for (i, result) in fetch_results.into_iter().enumerate() {
            let (step_idx, action_idx, _) = &fetch_tasks[i];
            output_map.insert((*step_idx, *action_idx), result);
        }

        if is_parallel {
            // For parallel jobs: create container hierarchy
            // Top level = containers, each container has sub_steps
            for container_idx in 0..num_containers {
                let mut container_failed = false;
                let mut container_sub_steps: Vec<JobStep> = Vec::new();

                for (step_idx, step) in steps.iter().enumerate() {
                    // Get the action for this container
                    let action = step.actions.get(container_idx);

                    let (step_status, step_failed, output) = if let Some(action) = action {
                        let action_status = action.status.as_deref().unwrap_or("unknown").to_lowercase();
                        let action_failed = action_status == "failed"
                            || action_status == "timedout"
                            || action.exit_code.map(|c| c != 0).unwrap_or(false);

                        if action_failed {
                            container_failed = true;
                        }

                        // Get output for this step/container
                        let mut output = String::new();
                        if action.output_url.is_some() {
                            match output_map.remove(&(step_idx, container_idx)) {
                                Some(Ok(step_output)) if !step_output.is_empty() => {
                                    output = step_output.trim().to_string();
                                }
                                Some(Ok(_)) => {
                                    if let Some(code) = action.exit_code {
                                        if code != 0 {
                                            output = format!("Exit code: {}", code);
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    output = format!("(Failed to fetch output: {})", e);
                                }
                                None => {}
                            }
                        } else if let Some(code) = action.exit_code {
                            if code != 0 || action_failed {
                                output = format!("Exit code: {}", code);
                            }
                        }

                        if output.is_empty() {
                            output = "(No output)".to_string();
                        }

                        (action.status.as_deref().unwrap_or("unknown").to_string(), action_failed, output)
                    } else {
                        ("skipped".to_string(), false, "(No data for this container)".to_string())
                    };

                    container_sub_steps.push(JobStep {
                        name: step.name.clone(),
                        status: step_status,
                        output,
                        is_failed: step_failed,
                        sub_steps: None,
                    });
                }

                // Create container as top-level step with sub_steps
                let container_status = if container_failed { "failed" } else { "success" };
                structured_steps.push(JobStep {
                    name: format!("Container {}", container_idx),
                    status: container_status.to_string(),
                    output: String::new(), // Container itself has no output, only sub-steps do
                    is_failed: container_failed,
                    sub_steps: Some(container_sub_steps),
                });
            }
        } else {
            // For non-parallel jobs: keep steps at top level (original behavior)
            for (step_idx, step) in steps.into_iter().enumerate() {
                let action = step.actions.first();
                let step_status = action
                    .and_then(|a| a.status.as_deref())
                    .unwrap_or("unknown")
                    .to_string();
                let exit_code = action.and_then(|a| a.exit_code);

                let is_failed = step_status.to_lowercase() == "failed"
                    || step_status.to_lowercase() == "timedout"
                    || exit_code.map(|c| c != 0).unwrap_or(false);

                // Get output
                let mut output = String::new();
                if let Some(action) = action {
                    if action.output_url.is_some() {
                        match output_map.remove(&(step_idx, 0)) {
                            Some(Ok(step_output)) if !step_output.is_empty() => {
                                output = step_output.trim().to_string();
                            }
                            Some(Ok(_)) => {
                                if let Some(code) = exit_code {
                                    if code != 0 {
                                        output = format!("Exit code: {}", code);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                output = format!("(Failed to fetch output: {})", e);
                            }
                            None => {}
                        }
                    } else if let Some(code) = exit_code {
                        if code != 0 || is_failed {
                            output = format!("Exit code: {}", code);
                        }
                    }
                }

                if output.is_empty() {
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

                structured_steps.push(JobStep {
                    name: step.name,
                    status: step_status,
                    output,
                    is_failed,
                    sub_steps: None,
                });
            }
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
                // Construct details URL with job number for proper log fetching
                let details_url = job.job_number.map(|num| {
                    format!(
                        "https://app.circleci.com/pipelines/gh/{}/{}/{}/workflows/{}/jobs/{}",
                        owner, repo, latest_pipeline.number, workflow.id, num
                    )
                });
                WorkflowJob {
                    id: job.job_number.unwrap_or(0),
                    name: job.name,
                    status: parse_circleci_status(&job.status),
                    conclusion: parse_circleci_conclusion(&job.status),
                    started_at: job.started_at,
                    completed_at: job.stopped_at,
                    details_url,
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

/// Extract CircleCI job number (build number) from a details URL
/// CircleCI URLs can be in various formats:
/// - https://circleci.com/gh/owner/repo/123 (legacy - 123 is build number)
/// - https://circleci.com/gh/owner/repo/123?query=param (legacy with query)
/// - https://app.circleci.com/pipelines/gh/owner/repo/123/workflows/abc/jobs/456 (new - 456 is build number)
/// - https://app.circleci.com/pipelines/github/owner/repo/123/workflows/abc/jobs/456
///
/// IMPORTANT: Workflow-level URLs (no /jobs/ segment) contain pipeline numbers, NOT build numbers.
/// We must NOT extract from those URLs as it would fetch the wrong job's steps.
pub fn extract_job_number_from_url(url: &str) -> Option<u64> {
    debug_log(&format!("extract_job_number_from_url called with: {}", url));

    if !url.contains("circleci.com") {
        debug_log("  -> Not a CircleCI URL, returning None");
        return None;
    }

    // Remove query string and fragment
    let url_path = url.split('?').next().unwrap_or(url);
    let url_path = url_path.split('#').next().unwrap_or(url_path);
    debug_log(&format!("  URL path (cleaned): {}", url_path));

    // Pattern 1: New format with /jobs/ path - the number after /jobs/ is the build number
    // https://app.circleci.com/pipelines/gh/owner/repo/123/workflows/abc/jobs/456
    if let Some(jobs_idx) = url_path.find("/jobs/") {
        let after_jobs = &url_path[jobs_idx + 6..];
        // Extract digits until we hit a non-digit or end
        let num_str: String = after_jobs.chars().take_while(|c| c.is_ascii_digit()).collect();
        debug_log(&format!("  Pattern 1 (/jobs/): found, after_jobs='{}', num_str='{}'", after_jobs, num_str));
        if !num_str.is_empty() {
            let result = num_str.parse().ok();
            debug_log(&format!("  -> Extracted job number: {:?}", result));
            return result;
        }
    }

    // Pattern 2: New workflow-level URL without /jobs/ - DO NOT extract the pipeline number!
    // https://app.circleci.com/pipelines/gh/owner/repo/123/workflows/abc
    // The 123 here is a pipeline number, not a build number - fetching it would show wrong steps.
    if url_path.contains("/pipelines/") {
        // This is a modern URL but without a job number - we can't determine the build number
        debug_log("  Pattern 2 (/pipelines/ without /jobs/): returning None to avoid pipeline number confusion");
        return None;
    }

    // Pattern 3: Legacy format - last numeric path segment is the build number
    // https://circleci.com/gh/owner/repo/123
    // These URLs directly point to a specific build.
    let segments: Vec<&str> = url_path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    debug_log(&format!("  Pattern 3 (legacy): segments={:?}", segments));

    // Look for the last numeric segment (the build number)
    for segment in segments.iter().rev() {
        if let Ok(num) = segment.parse::<u64>() {
            debug_log(&format!("  -> Extracted job number (legacy): {}", num));
            return Some(num);
        }
    }

    None
}

/// Check if a URL is a CircleCI URL
pub fn is_circleci_url(url: &str) -> bool {
    url.contains("circleci.com")
}
