use crate::{
    git,
    models::{Config, Repo},
    process, storage,
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};
pub static COMMIT_STOP: AtomicBool = AtomicBool::new(false);
pub static PUSH_STOP: AtomicBool = AtomicBool::new(false);
pub static COMMIT_PUSH_STOP: AtomicBool = AtomicBool::new(false);
static STAGE: AtomicU8 = AtomicU8::new(0);
pub fn poll_stop() {
    if let Some(c) = storage::read_stop() {
        match c.as_str() {
            "commitstop" => COMMIT_STOP.store(true, Ordering::SeqCst),
            "pushstop" => PUSH_STOP.store(true, Ordering::SeqCst),
            "commitpushstop" => COMMIT_PUSH_STOP.store(true, Ordering::SeqCst),
            _ => {}
        }
    }
}
pub fn stop_requested() -> bool {
    poll_stop();
    COMMIT_PUSH_STOP.load(Ordering::SeqCst)
        || match STAGE.load(Ordering::SeqCst) {
            1 => COMMIT_STOP.load(Ordering::SeqCst),
            2 | 3 => PUSH_STOP.load(Ordering::SeqCst),
            _ => false,
        }
}
pub fn reset() {
    COMMIT_STOP.store(false, Ordering::SeqCst);
    PUSH_STOP.store(false, Ordering::SeqCst);
    COMMIT_PUSH_STOP.store(false, Ordering::SeqCst);
    STAGE.store(0, Ordering::SeqCst);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPolicy {
    BeforeCommit,
    AfterCommit,
    Skip,
}
#[derive(Debug, PartialEq, Eq)]
enum CommitOutcome {
    NoChanges,
    Committed(String),
}
fn do_commit(msg: &str, dir: &Path, expected: u64) -> Result<CommitOutcome, String> {
    STAGE.store(1, Ordering::SeqCst);
    if stop_requested() {
        return Err("Commit cancelled before staging.".into());
    }
    if git::commit_fingerprint(dir)? != expected {
        return Err("Files changed since review. Nothing was staged; review again.".into());
    }
    git::git_in_live(Some(dir), &["add", "-A"])?;
    if git::commit_fingerprint(dir)? != expected {
        return Err("Files changed while staging. No Commit was attempted; inspect the staged index using Git.".into());
    }
    let unstaged = process::capture("git", &["diff", "--quiet"], Some(dir))?;
    if !matches!(unstaged.termination, process::Termination::Exited(0)) || !unstaged.success {
        return Err("Worktree and index differ after staging. No Commit was attempted; review with Git (including clean filters).".into());
    }
    let staged = process::capture("git", &["diff", "--cached", "--quiet"], Some(dir))?;
    match staged.termination {
        process::Termination::Exited(0) => return Ok(CommitOutcome::NoChanges),
        process::Termination::Exited(1) => {}
        _ => return Err(process::failure("git diff", &staged)),
    }
    if stop_requested() {
        return Err("Commit cancelled; files were already staged. No rollback was attempted; inspect the index using Git.".into());
    }
    git::git_in_live(Some(dir), &["commit", "-m", &msg.replace("\\n", "\n")])?;
    Ok(CommitOutcome::Committed(git::git_in(
        Some(dir),
        &["rev-parse", "HEAD"],
    )?))
}
pub fn commit(msg: &str, dir: &Path, expected: u64) -> Result<String, String> {
    match do_commit(msg, dir, expected)? {
        CommitOutcome::NoChanges => Ok("No changes to commit.".into()),
        CommitOutcome::Committed(sha) => Ok(format!("Commit completed: {sha}")),
    }
}
pub fn push(dir: &Path, plan: &git::PushPlan) -> Result<String, String> {
    STAGE.store(2, Ordering::SeqCst);
    if stop_requested() {
        return Err("Push cancelled before sending.".into());
    }
    git::execute_push(dir, plan)
}
pub fn commit_push(
    repo: &Repo,
    _config: &Config,
    msg: &str,
    dir: &Path,
    expected: u64,
    policy: ShellPolicy,
    approved: &git::PushPlan,
) -> Result<String, String> {
    let rc = storage::load_repo_config(&repo.full_name).map_err(|e| e.to_string())?;
    execute_sequence(msg, dir, expected, policy, &rc, approved)
}
fn execute_sequence(
    msg: &str,
    dir: &Path,
    expected: u64,
    policy: ShellPolicy,
    rc: &crate::models::RepoConfig,
    approved: &git::PushPlan,
) -> Result<String, String> {
    let before = git::push_plan(dir)?;
    if &before != approved {
        return Err(
            "Push target or HEAD changed after review; no Commit or Push was attempted.".into(),
        );
    }
    let shell = rc.pre_push_enabled && !rc.pre_push_command.trim().is_empty();
    if policy == ShellPolicy::BeforeCommit && shell {
        STAGE.store(3, Ordering::SeqCst);
        if stop_requested() {
            return Err("Cancelled before shell execution.".into());
        }
        process::run_shell(&rc.pre_push_command, dir)?;
    }
    // A pre-commit shell may change files: force a fresh review instead of
    // silently widening the user's already approved commit.
    let outcome = do_commit(msg, dir, expected)?;
    let sha = match outcome {
        CommitOutcome::Committed(s) => s,
        CommitOutcome::NoChanges => before.head.clone(),
    };
    if policy == ShellPolicy::AfterCommit && shell {
        STAGE.store(3, Ordering::SeqCst);
        if stop_requested() {
            return Err(format!("Commit {sha} exists; Push cancelled before shell."));
        }
        process::run_shell(&rc.pre_push_command, dir).map_err(|e| {
            format!("Commit {sha} exists; shell failed and Push was not attempted: {e}")
        })?;
    }
    let after = git::push_plan(dir)?;
    if before.url != after.url
        || before.target != after.target
        || before.remote != after.remote
        || before.branch != after.branch
        || after.head != sha
    {
        return Err(format!(
            "Commit {sha} exists; Push target or HEAD changed. No Push was attempted."
        ));
    }
    push(dir,&after).map_err(|e|format!("Commit {sha} exists. Push did not report success; remote may already have received it. Inspect remote before retrying.\n{e}"))?;
    Ok(format!("Commit & Push completed: {sha}"))
}

pub fn shell(command: &str, path: &Path) -> Result<String, String> {
    STAGE.store(3, Ordering::SeqCst);
    if stop_requested() {
        return Err("Shell cancelled.".into());
    }
    process::run_shell(command, path)
        .map(|_| "Shell completed; review generated changes before committing.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_review_does_not_stage() {
        let d = git::tests::repo();
        std::fs::write(d.path().join("new"), "old").unwrap();
        let fingerprint = git::commit_fingerprint(d.path()).unwrap();
        std::fs::write(d.path().join("new"), "new").unwrap();
        assert!(commit("review", d.path(), fingerprint).is_err());
        assert!(
            git::git_in(Some(d.path()), &["diff", "--cached", "--name-only"])
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn reviewed_commit_has_sha() {
        let d = git::tests::repo();
        std::fs::write(d.path().join("new"), "new").unwrap();
        let fingerprint = git::commit_fingerprint(d.path()).unwrap();
        assert!(commit("review", d.path(), fingerprint)
            .unwrap()
            .contains("Commit completed:"));
    }
    #[cfg(unix)]
    #[test]
    fn skip_never_executes_enabled_shell() {
        let d = git::tests::repo();
        let _bare = git::tests::remote(&d);
        std::fs::write(d.path().join("new"), "new").unwrap();
        let fp = git::commit_fingerprint(d.path()).unwrap();
        let rc = crate::models::RepoConfig {
            pre_push_enabled: true,
            pre_push_command: "printf forbidden > shell-marker".into(),
            ..Default::default()
        };
        execute_sequence(
            "skip",
            d.path(),
            fp,
            ShellPolicy::Skip,
            &rc,
            &git::push_plan(d.path()).unwrap(),
        )
        .unwrap();
        assert!(!d.path().join("shell-marker").exists());
    }
    #[test]
    fn reviewed_target_change_does_not_commit() {
        let d = git::tests::repo();
        let _remote = git::tests::remote(&d);
        std::fs::write(d.path().join("new"), "hello").unwrap();
        let fp = git::commit_fingerprint(d.path()).unwrap();
        let approved = git::push_plan(d.path()).unwrap();
        git::git_in(
            Some(d.path()),
            &["config", "remote.origin.pushurl", "/different"],
        )
        .unwrap();
        let rc = crate::models::RepoConfig::default();
        assert!(
            execute_sequence("blocked", d.path(), fp, ShellPolicy::Skip, &rc, &approved).is_err()
        );
        assert_eq!(
            git::git_in(Some(d.path()), &["rev-parse", "HEAD"]).unwrap(),
            approved.head
        );
        assert!(
            git::git_in(Some(d.path()), &["diff", "--cached", "--name-only"])
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn initial_commit_supported() {
        let d = tempfile::tempdir().unwrap();
        git::git_in(Some(d.path()), &["init", "-b", "master"]).unwrap();
        git::git_in(Some(d.path()), &["config", "user.name", "Test"]).unwrap();
        git::git_in(
            Some(d.path()),
            &["config", "user.email", "test@example.invalid"],
        )
        .unwrap();
        git::git_in(Some(d.path()), &["config", "commit.gpgsign", "false"]).unwrap();
        git::git_in(Some(d.path()), &["config", "core.hooksPath", "/dev/null"]).unwrap();
        std::fs::write(d.path().join("first"), "hello").unwrap();
        let fp = git::commit_fingerprint(d.path()).unwrap();
        assert!(commit("initial", d.path(), fp)
            .unwrap()
            .starts_with("Commit completed:"));
    }
    #[test]
    fn rename_and_partial_staging_can_be_reviewed() {
        let d = git::tests::repo();
        std::fs::rename(d.path().join("file"), d.path().join("日本語 renamed")).unwrap();
        std::fs::write(d.path().join("new"), "first").unwrap();
        git::git_in(Some(d.path()), &["add", "new"]).unwrap();
        std::fs::write(d.path().join("new"), "reviewed second").unwrap();
        let fp = git::commit_fingerprint(d.path()).unwrap();
        assert!(commit("rename", d.path(), fp).is_ok());
        assert_eq!(
            git::git_in(Some(d.path()), &["show", "HEAD:new"]).unwrap(),
            "reviewed second"
        );
    }
}
