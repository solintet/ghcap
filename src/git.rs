use std::process::Command;

pub fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git").args(args).output().map_err(|e| format!("git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!("{}\nexit code {}", if stderr.is_empty() { stdout } else { stderr }, output.status.code().unwrap_or(1)))
    }
}

pub fn current_repo() -> Option<String> { git(&["rev-parse", "--show-toplevel"]).ok() }

pub fn remote_origin() -> Option<String> { git(&["remote", "get-url", "origin"]).ok() }

pub fn status() -> String {
    let branch = git(&["branch", "--show-current"]).unwrap_or_else(|_| "-".into());
    let changes = git(&["status", "--short"]).unwrap_or_default();
    let upstream = git(&["rev-list", "--left-right", "--count", "@{upstream}...HEAD"]).unwrap_or_else(|_| "-".into());
    let changed_count = changes.lines().filter(|line| !line.trim().is_empty()).count();
    format!("Branch: {branch}\nChanged files: {changed_count}\nUpstream: {upstream}")
}

pub fn has_changes() -> bool { !git(&["status", "--porcelain"]).unwrap_or_default().trim().is_empty() }
