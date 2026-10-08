use std::{io::{BufRead, BufReader}, path::Path, process::{Command, Stdio}, sync::mpsc::{self, RecvTimeoutError}, thread, time::Duration};

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



/// Run git while streaming stdout/stderr directly to the CUI and also retain
/// the text for error classification (for example GPG failures).
pub fn git_in_live(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(path) = dir { command.current_dir(path); }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| format!("git: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "git: failed to capture stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "git: failed to capture stderr".to_string())?;

    let out_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut text = String::new();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    println!("{line}");
                    text.push_str(&line);
                    text.push('\n');
                }
                Err(e) => {
                    text.push_str(&format!("stdout read error: {e}\n"));
                    break;
                }
            }
        }
        text
    });
    let err_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut text = String::new();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    eprintln!("{line}");
                    text.push_str(&line);
                    text.push('\n');
                }
                Err(e) => {
                    text.push_str(&format!("stderr read error: {e}\n"));
                    break;
                }
            }
        }
        text
    });

    // Git may legitimately stay silent for a while (authentication, network,
    // GPG, hooks, etc.). Keep the CUI visibly alive once per second.
    let (stop_tx, stop_rx) = mpsc::channel();
    let ticker = thread::spawn(move || loop {
        match stop_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => println!("Running... Please wait."),
        }
    });

    let status = child.wait().map_err(|e| format!("git: {e}"));
    let _ = stop_tx.send(());
    let _ = ticker.join();
    let status = status?;
    let stdout_text = out_thread.join().unwrap_or_default();
    let stderr_text = err_thread.join().unwrap_or_default();
    if status.success() {
        Ok(String::new())
    } else {
        let combined = if stderr_text.trim().is_empty() { stdout_text.trim().to_string() } else { stderr_text.trim().to_string() };
        Err(format!("{}\nexit code {}", combined, status.code().unwrap_or(1)))
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

