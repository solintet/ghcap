//! EN: commit push menu UI workflow with one responsibility per Rust module.
//! JA: commit push menu の画面と操作を独立したRustモジュールとして管理します。
use super::{commit_message_flow, ensure_git_setup, review_commit, t, CommitMessageOutcome};
use crate::actions;
use crate::git;
use crate::models::{Config, Repo};
use crate::process;
use crate::storage;
use crate::ui::{self, Tui};
use std::io;

pub(super) fn commit_push(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(path) = ensure_git_setup(config, tui, repo)? else {
        return Ok(());
    };
    let message = match commit_message_flow(config, tui, repo)? {
        CommitMessageOutcome::Commit(v) => v,
        CommitMessageOutcome::Back => return Ok(()),
    };
    let rc = storage::load_repo_config(&repo.full_name)?;
    let mut policy = actions::ShellPolicy::Skip;
    if rc.pre_push_enabled && !rc.pre_push_command.trim().is_empty() {
        if rc.shell_confirmation {
            let choices = vec![
                t(&lang, "shell.commit_then_run"),
                t(&lang, "shell.run_then_commit"),
                t(&lang, "shell.skip"),
                t(&lang, "common.cancel"),
            ];
            policy = match ui::select(tui, &lang, &t(&lang, "shell.detected"), &choices)? {
                Some(0) => actions::ShellPolicy::AfterCommit,
                Some(1) => actions::ShellPolicy::BeforeCommit,
                Some(2) => actions::ShellPolicy::Skip,
                _ => return Ok(()),
            };
        } else {
            policy = actions::ShellPolicy::AfterCommit;
        }
    }
    actions::reset();
    if policy == actions::ShellPolicy::BeforeCommit {
        if ui::cui_operation_result(&lang, "Pre-commit shell", || {
            actions::shell(&rc.pre_push_command, &path)
        })?
        .is_err()
        {
            return Ok(());
        }
        policy = actions::ShellPolicy::Skip;
    }
    let plan = match git::push_plan(&path) {
        Ok(v) => v,
        Err(e) => return ui::message(&lang, "Commit & Push blocked", &e),
    };
    if !ui::cui_yes_no(
        &lang,
        "Review Push target",
        &format!(
            "{}\n{} -> {}\nA successful commit will be sent here without force. Continue?",
            process::safe_text(&plan.url),
            plan.branch,
            plan.target
        ),
    )? {
        return Ok(());
    }
    let Some(expected) = review_commit(tui, &lang, &path)? else {
        return Ok(());
    };
    let _ = ui::cui_operation_result(&lang, &t(&lang, "repo.commit_push"), || {
        actions::commit_push(repo, config, &message, &path, expected, policy, &plan)
    })?;
    Ok(())
}
