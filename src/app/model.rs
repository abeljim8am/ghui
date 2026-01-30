use anyhow::Result;
use ratatui::widgets::TableState;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::data::{
    ActionsData, CheckAnnotation, JobLogs, LabelFilter, PrFilter, PreviewData, PullRequest,
    SPINNER_FRAMES,
};
use crate::services::{
    fetch_actions_for_pr, fetch_job_logs, fetch_pr_preview, fetch_prs_graphql, load_cache,
    load_label_filters, save_cache,
};
use crate::utils::get_current_repo;

use super::message::FetchResult;

pub struct App {
    // Data state
    pub my_prs: Vec<PullRequest>,
    pub review_prs: Vec<PullRequest>,
    pub labels_prs: Vec<PullRequest>,
    pub configured_labels: Vec<LabelFilter>,

    // Filter/View state
    pub pr_filter: PrFilter,
    pub table_state: TableState,
    pub filtered_indices: Vec<usize>,

    // Search state
    pub search_mode: bool,
    pub search_query: String,

    // Loading state
    pub loading_my_prs: bool,
    pub loading_review_prs: bool,
    pub loading_labels_prs: bool,

    // Popup state
    pub show_help_popup: bool,
    pub show_checkout_popup: bool,
    pub show_error_popup: bool,
    pub show_labels_popup: bool,
    pub show_add_label_popup: bool,

    // Workflows view state
    pub show_workflows_view: bool,
    pub actions_data: Option<ActionsData>,
    pub actions_loading: bool,
    pub selected_job_index: usize,
    pub actions_poll_enabled: bool,
    pub last_actions_poll: Instant,
    pub actions_pending_pr_number: Option<u64>, // PR we're waiting to get head_sha for
    pub workflows_pr_info: Option<(String, u64)>, // (title, number) for display

    // Main page auto-refresh state
    pub last_main_refresh: Instant,

    // Job logs state
    pub show_job_logs: bool,
    pub job_logs: Option<JobLogs>,
    pub job_logs_loading: bool,
    pub job_logs_scroll: u16,

    // Annotations view state (for reviewdog, etc.)
    pub annotations_view: bool, // true if viewing annotations, false for raw logs
    pub annotations: Vec<CheckAnnotation>, // current annotations being displayed
    pub selected_annotation_index: usize,
    pub selected_annotations: Vec<usize>, // indices of selected annotations for copying

    // Preview view state
    pub show_preview_view: bool,
    pub preview_data: Option<PreviewData>,
    pub preview_loading: bool,
    pub preview_scroll: u16,
    pub preview_section_index: usize,
    pub preview_comment_positions: Vec<u16>, // line positions of each comment start
    pub preview_total_lines: u16,
    pub preview_pr_info: Option<(String, u64)>, // (title, number) for display

    // Clipboard feedback
    pub clipboard_feedback: Option<String>,
    pub clipboard_feedback_time: Instant,

    // URL popup (for container environments where we can't open browser)
    pub show_url_popup: Option<String>,

    // Error state
    pub error: Option<String>,

    // Checkout state
    pub pending_checkout_branch: Option<String>,

    // Label input state
    pub label_input: String,
    pub label_scope_global: bool,
    pub labels_list_state: TableState,

    // Repository info
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,

    // Async communication
    pub fetch_tx: Sender<PrFilter>,
    pub result_rx: Receiver<FetchResult>,

    // Actions async communication
    pub actions_tx: Sender<(String, String, u64, String)>, // owner, repo, pr_number, head_sha
    pub actions_rx: Receiver<FetchResult>,

    // Job logs async communication
    pub job_logs_tx: Sender<(String, String, u64, String)>, // owner, repo, job_id, job_name
    pub job_logs_rx: Receiver<FetchResult>,

    // Preview async communication
    pub preview_tx: Sender<(String, String, u64)>, // owner, repo, pr_number
    pub preview_rx: Receiver<FetchResult>,

    // Spinner state
    pub spinner_idx: usize,
    pub last_spinner_update: Instant,
}

impl App {
    pub fn new() -> Result<Self> {
        let (fetch_tx, fetch_rx) = mpsc::channel::<PrFilter>();
        let (result_tx, result_rx) = mpsc::channel::<FetchResult>();

        // Spawn background thread for fetching PRs
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Ok(filter) = fetch_rx.recv() {
                let result = rt.block_on(fetch_prs_graphql(filter.clone()));
                let msg = match result {
                    Ok(prs) => {
                        // Get owner/repo from the first PR or current repo
                        if let Some((owner, repo)) = prs
                            .first()
                            .map(|pr| (pr.repo_owner.clone(), pr.repo_name.clone()))
                            .or_else(get_current_repo)
                        {
                            if let Err(e) = save_cache(&prs, &owner, &repo, filter.clone()) {
                                eprintln!("Failed to save cache: {}", e);
                            }
                        }
                        FetchResult::Success(prs, filter)
                    }
                    Err(e) => FetchResult::Error(format!("{}", e)),
                };
                if result_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Channel for actions fetching
        let (actions_tx, actions_rx_internal) = mpsc::channel::<(String, String, u64, String)>();
        let (actions_result_tx, actions_rx) = mpsc::channel::<FetchResult>();

        // Spawn background thread for fetching Actions
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Ok((owner, repo, pr_number, head_sha)) = actions_rx_internal.recv() {
                let result = rt.block_on(fetch_actions_for_pr(&owner, &repo, pr_number, &head_sha));
                let msg = match result {
                    Ok(data) => FetchResult::ActionsSuccess(data),
                    Err(e) => FetchResult::ActionsError(format!("{}", e)),
                };
                if actions_result_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Channel for job logs fetching
        let (job_logs_tx, job_logs_rx_internal) = mpsc::channel::<(String, String, u64, String)>();
        let (job_logs_result_tx, job_logs_rx) = mpsc::channel::<FetchResult>();

        // Spawn background thread for fetching job logs
        thread::spawn(move || {
            while let Ok((owner, repo, job_id, job_name)) = job_logs_rx_internal.recv() {
                let result = fetch_job_logs(&owner, &repo, job_id, &job_name);
                let msg = match result {
                    Ok(logs) => FetchResult::JobLogsSuccess(logs),
                    Err(e) => FetchResult::JobLogsError(format!("{}", e)),
                };
                if job_logs_result_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Channel for preview fetching
        let (preview_tx, preview_rx_internal) = mpsc::channel::<(String, String, u64)>();
        let (preview_result_tx, preview_rx) = mpsc::channel::<FetchResult>();

        // Spawn background thread for fetching PR preview/comments
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Ok((owner, repo, pr_number)) = preview_rx_internal.recv() {
                let result = rt.block_on(fetch_pr_preview(&owner, &repo, pr_number));
                let msg = match result {
                    Ok(data) => FetchResult::PreviewSuccess(data),
                    Err(e) => FetchResult::PreviewError(format!("{}", e)),
                };
                if preview_result_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // Get repo info for loading cache
        let (owner, repo_name) = get_current_repo().unzip();

        // Load caches
        let my_prs = match (&owner, &repo_name) {
            (Some(o), Some(r)) => load_cache(o, r, PrFilter::MyPrs).unwrap_or_default(),
            _ => Vec::new(),
        };
        let review_prs = match (&owner, &repo_name) {
            (Some(o), Some(r)) => load_cache(o, r, PrFilter::ReviewRequested).unwrap_or_default(),
            _ => Vec::new(),
        };
        let labels_prs = match (&owner, &repo_name) {
            (Some(o), Some(r)) => load_cache(o, r, PrFilter::Labels(vec![])).unwrap_or_default(),
            _ => Vec::new(),
        };

        // Load configured labels
        let configured_labels = match (&owner, &repo_name) {
            (Some(o), Some(r)) => load_label_filters(o, r).unwrap_or_default(),
            _ => Vec::new(),
        };

        let mut table_state = TableState::default();
        if !my_prs.is_empty() {
            table_state.select(Some(0));
        }

        let filtered_indices: Vec<usize> = (0..my_prs.len()).collect();

        Ok(Self {
            my_prs,
            review_prs,
            labels_prs,
            configured_labels,
            pr_filter: PrFilter::MyPrs,
            table_state,
            filtered_indices,
            search_mode: false,
            search_query: String::new(),
            loading_my_prs: true,
            loading_review_prs: true,
            loading_labels_prs: false,
            show_help_popup: false,
            show_checkout_popup: false,
            show_error_popup: false,
            show_labels_popup: false,
            show_add_label_popup: false,
            show_workflows_view: false,
            actions_data: None,
            actions_loading: false,
            selected_job_index: 0,
            actions_poll_enabled: false,
            last_actions_poll: Instant::now(),
            actions_pending_pr_number: None,
            workflows_pr_info: None,
            last_main_refresh: Instant::now(),
            show_job_logs: false,
            job_logs: None,
            job_logs_loading: false,
            job_logs_scroll: 0,
            annotations_view: false,
            annotations: Vec::new(),
            selected_annotation_index: 0,
            selected_annotations: Vec::new(),
            show_preview_view: false,
            preview_data: None,
            preview_loading: false,
            preview_scroll: 0,
            preview_section_index: 0,
            preview_comment_positions: Vec::new(),
            preview_total_lines: 0,
            preview_pr_info: None,
            clipboard_feedback: None,
            clipboard_feedback_time: Instant::now(),
            show_url_popup: None,
            error: None,
            pending_checkout_branch: None,
            label_input: String::new(),
            label_scope_global: false,
            labels_list_state: TableState::default(),
            repo_owner: owner,
            repo_name,
            fetch_tx,
            result_rx,
            actions_tx,
            actions_rx,
            job_logs_tx,
            job_logs_rx,
            preview_tx,
            preview_rx,
            spinner_idx: 0,
            last_spinner_update: Instant::now(),
        })
    }

    // Getters

    pub fn current_prs(&self) -> &Vec<PullRequest> {
        match &self.pr_filter {
            PrFilter::MyPrs => &self.my_prs,
            PrFilter::ReviewRequested => &self.review_prs,
            PrFilter::Labels(_) => &self.labels_prs,
        }
    }

    pub fn is_loading(&self) -> bool {
        match &self.pr_filter {
            PrFilter::MyPrs => self.loading_my_prs,
            PrFilter::ReviewRequested => self.loading_review_prs,
            PrFilter::Labels(_) => self.loading_labels_prs,
        }
    }

    pub fn get_active_labels(&self) -> Vec<String> {
        self.configured_labels
            .iter()
            .map(|lf| lf.label_name.clone())
            .collect()
    }

    pub fn visible_prs(&self) -> Vec<&PullRequest> {
        let prs = self.current_prs();
        self.filtered_indices
            .iter()
            .filter_map(|&idx| prs.get(idx))
            .collect()
    }

    pub fn selected_pr(&self) -> Option<&PullRequest> {
        let prs = self.current_prs();
        self.table_state
            .selected()
            .and_then(|sel| self.filtered_indices.get(sel))
            .and_then(|&idx| prs.get(idx))
    }

    pub fn spinner(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_idx]
    }

    // Spinner update

    pub fn update_spinner(&mut self) {
        if self.last_spinner_update.elapsed() >= Duration::from_millis(80) {
            self.spinner_idx = (self.spinner_idx + 1) % SPINNER_FRAMES.len();
            self.last_spinner_update = Instant::now();
        }
    }

    // Fetch management

    pub fn start_fetch(&mut self, filter: PrFilter) {
        match &filter {
            PrFilter::MyPrs => self.loading_my_prs = true,
            PrFilter::ReviewRequested => self.loading_review_prs = true,
            PrFilter::Labels(_) => self.loading_labels_prs = true,
        }
        self.error = None;
        self.show_error_popup = false;
        self.last_main_refresh = Instant::now();
        let _ = self.fetch_tx.send(filter);
    }

    pub fn check_fetch_result(&mut self) -> Option<FetchResult> {
        self.result_rx.try_recv().ok()
    }

    // Actions fetch management

    pub fn start_actions_fetch(&mut self, owner: &str, repo: &str, pr_number: u64, head_sha: &str) {
        self.actions_loading = true;
        self.last_actions_poll = Instant::now();
        let _ = self.actions_tx.send((
            owner.to_string(),
            repo.to_string(),
            pr_number,
            head_sha.to_string(),
        ));
    }

    pub fn check_actions_result(&mut self) -> Option<FetchResult> {
        self.actions_rx.try_recv().ok()
    }

    // Job logs fetch management

    pub fn start_job_logs_fetch(&mut self, owner: &str, repo: &str, job_id: u64, job_name: &str) {
        self.job_logs_loading = true;
        self.job_logs = None;
        self.job_logs_scroll = 0;
        let _ = self.job_logs_tx.send((
            owner.to_string(),
            repo.to_string(),
            job_id,
            job_name.to_string(),
        ));
    }

    pub fn check_job_logs_result(&mut self) -> Option<FetchResult> {
        self.job_logs_rx.try_recv().ok()
    }

    pub fn should_poll_actions(&self) -> bool {
        self.show_workflows_view
            && self.actions_poll_enabled
            && !self.actions_loading
            && self.last_actions_poll.elapsed() >= Duration::from_secs(30)
    }

    pub fn should_refresh_main(&self) -> bool {
        // Only auto-refresh when on the main page (not in any special views or popups)
        !self.show_workflows_view
            && !self.show_preview_view
            && !self.show_help_popup
            && !self.show_checkout_popup
            && !self.show_error_popup
            && !self.show_labels_popup
            && !self.show_add_label_popup
            && !self.is_loading()
            && self.last_main_refresh.elapsed() >= Duration::from_secs(30)
    }

    // Preview fetch management

    pub fn start_preview_fetch(&mut self, owner: &str, repo: &str, pr_number: u64) {
        self.preview_loading = true;
        self.preview_data = None;
        self.preview_scroll = 0;
        let _ = self
            .preview_tx
            .send((owner.to_string(), repo.to_string(), pr_number));
    }

    pub fn check_preview_result(&mut self) -> Option<FetchResult> {
        self.preview_rx.try_recv().ok()
    }
}
