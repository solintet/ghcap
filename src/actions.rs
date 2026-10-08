use crate::{git, models::{Config, Repo}, storage};
use std::{path::Path, process::Command, sync::atomic::{AtomicBool, Ordering}};

pub static COMMIT_STOP: AtomicBool = AtomicBool::new(false);
pub static PUSH_STOP: AtomicBool = AtomicBool::new(false);
pub static COMMIT_PUSH_STOP: AtomicBool = AtomicBool::new(false);

pub fn poll_stop() {
    if let Some(command) = storage::read_stop() {
        match command.as_str() {
            "commitstop" => COMMIT_STOP.store(true, Ordering::SeqCst),
            "pushstop" => PUSH_STOP.store(true, Ordering::SeqCst),
            "commitpushstop" => { COMMIT_STOP.store(true, Ordering::SeqCst); PUSH_STOP.store(true, Ordering::SeqCst); COMMIT_PUSH_STOP.store(true, Ordering::SeqCst); }
            _ => {}
        }
    }
}

pub fn reset() {
    COMMIT_STOP.store(false, Ordering::SeqCst);
    PUSH_STOP.store(false, Ordering::SeqCst);
    COMMIT_PUSH_STOP.store(false, Ordering::SeqCst);
}

fn shell(command: &str, dir: &Path) -> Result<(), String> {
    let result = if cfg!(target_os = "windows") { Command::new("cmd").args(["/C", command]).current_dir(dir).status() } else { Command::new("sh").args(["-c", command]).current_dir(dir).status() };
    match result { Ok(status) if status.success() => Ok(()), Ok(status) => Err(format!("shell exited with status {status}")), Err(e) => Err(format!("shell: {e}")) }
}

pub fn commit(msg: &str, dir: &Path) -> Result<String, String> {
    poll_stop(); if COMMIT_STOP.load(Ordering::SeqCst) || COMMIT_PUSH_STOP.load(Ordering::SeqCst) { return Ok("Commit cancelled.".into()); }

    // The UI accepts literal \n as the only line-break notation. Convert it
    // immediately before invoking Git so the preset/input format stays simple.
    let commit_message = msg.replace("\\n", "\n");

    // Stage everything first. If there is still nothing staged afterwards,
    // this is not a real commit failure; it simply means there is nothing to commit.
    git::git_in(Some(dir), &["add", "-A"])?;
    let staged = git::git_in(Some(dir), &["diff", "--cached", "--quiet"]);
    if staged.is_ok() {
        return Ok("No changes to commit.".into());
    }

    poll_stop(); if COMMIT_STOP.load(Ordering::SeqCst) || COMMIT_PUSH_STOP.load(Ordering::SeqCst) { return Ok("Commit cancelled.".into()); }
    git::git_in_live(Some(dir), &["commit", "-m", &commit_message])
}

pub fn push(dir: &Path) -> Result<String, String> {
    poll_stop(); if PUSH_STOP.load(Ordering::SeqCst) || COMMIT_PUSH_STOP.load(Ordering::SeqCst) { return Ok("Push cancelled.".into()); }
    git::git_in_live(Some(dir), &["push"])
}

pub fn commit_push(repo: &Repo, _config: &Config, msg: &str, dir: &Path) -> Result<String, String> {
    let repo_config = storage::load_repo_config(&repo.full_name);
    let shell_enabled = repo_config.pre_push_enabled && !repo_config.pre_push_command.trim().is_empty();
    println!("[1/2] Commit");
    let commit_result = commit(msg, dir)?;
    if commit_result == "Commit cancelled." { return Ok(commit_result); }
    if commit_result == "No changes to commit." { println!("No changes to commit. Commitをスキップします。"); }
    if shell_enabled {
        poll_stop(); if COMMIT_PUSH_STOP.load(Ordering::SeqCst) || PUSH_STOP.load(Ordering::SeqCst) { return Ok("Commit completed; Push cancelled.".into()); }
        shell(&repo_config.pre_push_command, dir).map_err(|e| format!("Pre-push shell failed; push cancelled: {e}"))?;
    }
    println!("[2/2] Push");
    let push_result = push(dir)?;
    if push_result == "Push cancelled." { return Ok("Commit completed; Push cancelled.".into()); }
    Ok("Commit & Push completed.".into())
}
