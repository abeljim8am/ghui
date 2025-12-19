use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher,
};

use crate::data::PullRequest;

/// Filter pull requests using fuzzy matching.
/// Returns the indices of matching PRs, sorted by match score (best first).
pub fn filter_prs(prs: &[PullRequest], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..prs.len()).collect();
    }

    let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    // Build list of (index, haystack) for matching
    let haystacks: Vec<(usize, String)> = prs
        .iter()
        .enumerate()
        .map(|(idx, pr)| {
            let (ci_text, _) = pr.ci_status.display();
            (
                idx,
                format!(
                    "#{} {} {} {} {}",
                    pr.number, pr.author, pr.title, pr.branch, ci_text
                ),
            )
        })
        .collect();

    // Use match_list to score all items
    let haystack_refs: Vec<&str> = haystacks.iter().map(|(_, s)| s.as_str()).collect();
    let matches = pattern.match_list(&haystack_refs, &mut matcher);

    // Convert matches back to indices, sorted by score (match_list returns sorted by score descending)
    matches
        .into_iter()
        .map(|(haystack, _score)| {
            // Find the index of this haystack
            haystacks
                .iter()
                .position(|(_, s)| s.as_str() == *haystack)
                .unwrap()
        })
        .collect()
}
