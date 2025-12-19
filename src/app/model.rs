use anyhow::Result;
use ratatui::widgets::TableState;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::data::{LabelFilter, PrFilter, PullRequest, SPINNER_FRAMES};
use crate::services::{fetch_prs_graphql, load_cache, load_label_filters, save_cache};
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

    // Spinner state
    pub spinner_idx: usize,
    pub last_spinner_update: Instant,
}

impl App {
    pub fn new() -> Result<Self> {
        let (fetch_tx, fetch_rx) = mpsc::channel::<PrFilter>();
        let (result_tx, result_rx) = mpsc::channel::<FetchResult>();

        // Spawn background thread for fetching
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
            error: None,
            pending_checkout_branch: None,
            label_input: String::new(),
            label_scope_global: false,
            labels_list_state: TableState::default(),
            repo_owner: owner,
            repo_name,
            fetch_tx,
            result_rx,
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
        let _ = self.fetch_tx.send(filter);
    }

    pub fn check_fetch_result(&mut self) -> Option<FetchResult> {
        self.result_rx.try_recv().ok()
    }
}
