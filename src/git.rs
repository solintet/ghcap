use std::{path::Path, process::Command};

pub fn git(args: &[&str]) -> Result<String, String> { git_in(None, args) }

pub fn git_in(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(path) = dir { command.current_dir(path); }
    let output = command.output().map_err(|e| format!("git: {e}"))?;
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

pub fn is_repo(path: &Path) -> bool { git_in(Some(path), &["rev-parse", "--show-toplevel"]).is_ok() }
pub fn has_origin(path: &Path) -> bool { git_in(Some(path), &["remote", "get-url", "origin"]).is_ok() }
pub fn origin(path: &Path) -> Option<String> { git_in(Some(path), &["remote", "get-url", "origin"]).ok() }
pub fn init(path: &Path) -> Result<String, String> { git_in(Some(path), &["init"]) }
pub fn set_origin(path: &Path, url: &str) -> Result<String, String> {
    if has_origin(path) { git_in(Some(path), &["remote", "set-url", "origin", url]) }
    else { git_in(Some(path), &["remote", "add", "origin", url]) }
}
pub fn set_signing_key(path: &Path, key: &str) -> Result<String, String> { git_in(Some(path), &["config", "user.signingkey", key]) }

pub fn user_name(path: &Path) -> Option<String> { git_in(Some(path), &["config", "user.name"]).ok().filter(|v| !v.trim().is_empty()) }
pub fn user_email(path: &Path) -> Option<String> { git_in(Some(path), &["config", "user.email"]).ok().filter(|v| !v.trim().is_empty()) }
pub fn identity_configured(path: &Path) -> bool { user_name(path).is_some() && user_email(path).is_some() }
pub fn set_identity(path: &Path, name: &str, email: &str) -> Result<String, String> {
    git_in(Some(path), &["config", "user.name", name])?;
    git_in(Some(path), &["config", "user.email", email])
}
pub fn status_for(path: &Path) -> String {
    let branch = git_in(Some(path), &["branch", "--show-current"]).unwrap_or_else(|_| "-".into());
    let changes = git_in(Some(path), &["status", "--short"]).unwrap_or_default();
    let upstream = git_in(Some(path), &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"]).unwrap_or_else(|_| "-".into());
    let changed_count = changes.lines().filter(|line| !line.trim().is_empty()).count();
    format!("Branch: {branch}\nChanged files: {changed_count}\nUpstream: {upstream}")
}

