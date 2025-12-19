use std::process::Command;

pub fn get_current_repo() -> Option<(String, String)> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_url(&url)
}

pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    // Handle SSH: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.strip_prefix("git@github.com:")?;
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle HTTPS: https://github.com/owner/repo.git
    if url.contains("github.com") {
        let path = url.split("github.com").nth(1)?;
        let path = path.trim_start_matches('/').trim_start_matches(':');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

/// Checkout a branch using jj or git depending on the repository type.
/// Returns Ok(true) if checkout succeeded and the app should exit.
/// Returns Ok(false) if checkout failed (error will be set).
/// Returns the error message if checkout failed.
pub fn checkout_branch(branch: &str) -> Result<(), String> {
    // Check if repo uses jj by looking for .jj directory
    let has_jj = std::path::Path::new(".jj").exists();

    let result = if has_jj {
        Command::new("jj")
            .args(["new", &format!("{}@origin", branch)])
            .output()
    } else {
        Command::new("git").args(["switch", branch]).output()
    };

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(e) => Err(format!("Failed to checkout: {}", e)),
    }
}
