//! EN: push menu UI workflow with one responsibility per Rust module.
//! JA: push menu の画面と操作を独立したRustモジュールとして管理します。
use super::{ensure_git_setup, gpg_recovery, is_gpg_error, t};
use crate::actions;
use crate::git;
use crate::models::{Config, Repo};
use crate::process;
use crate::repository::{inspect, planner, safety};
use crate::ui::{self, Tui};
use std::io;

pub(super) fn push_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(path) = ensure_git_setup(config, tui, repo)? else {
        return Ok(());
    };
    // EN: First push is a separate reviewed workflow; the hardened normal
    // PushPlan remains mandatory for subsequent pushes.
    // JA: 初回Pushは専用の確認付きフローに分離し、通常Pushは厳格な
    // PushPlanの検証を引き続き必須とします。
    let snapshot = match inspect::inspect(&path) {
        Ok(snapshot) => snapshot,
        Err(error) => return ui::message(&lang, "Push diagnosis / Push診断", &error),
    };
    match planner::plan_push(&snapshot, None) {
        planner::PushPlan::FirstPush { remote, branch } => {
            if let Err(error) = git::preflight_commit(&path) {
                return ui::message(&lang, "Git preflight / 事前検証", &error);
            }
            if let Err(error) = safety::check_first_push(&path, &remote, &branch) {
                return ui::message(&lang, "First Push / 初回Push", &error);
            }
            let target = format!("refs/heads/{branch}");
            let approved_head = match git::git_in(Some(&path), &["rev-parse", "HEAD"]) {
                Ok(head) => head,
                Err(error) => return ui::message(&lang, "Git HEAD", &error),
            };
            if !ui::cui_yes_no(
                &lang,
                "First Push / 初回Push",
                &format!("Remote: {remote}\nBranch: {branch}\nCommit: {approved_head}\nSet upstream without force? / 強制せず初回Pushしますか？"),
            )? { return Ok(()); }
            // EN: Recheck the state and remote before executing approved refspec.
            // JA: 承認後に状態やremoteが変わっていないか再検証します。
            let updated = inspect::inspect(&path).map_err(io::Error::other)?;
            if planner::plan_push(&updated, Some(&remote))
                != (planner::PushPlan::FirstPush {
                    remote: remote.clone(),
                    branch: branch.clone(),
                })
            {
                return ui::message(
                    &lang,
                    "Push cancelled / 中止",
                    "Repository state changed after approval / 承認後に状態が変化しました",
                );
            }
            safety::check_first_push(&path, &remote, &branch).map_err(io::Error::other)?;
            let now = git::git_in(Some(&path), &["rev-parse", "HEAD"]).map_err(io::Error::other)?;
            if now != approved_head {
                return ui::message(
                    &lang,
                    "Push cancelled / 中止",
                    "HEAD changed after approval / 承認後にHEADが変化しました",
                );
            }
            let refspec = format!("{target}:{target}");
            let _ = ui::cui_operation_result(&lang, "First Push / 初回Push", || {
                git::git_in_live(
                    Some(&path),
                    &["push", "--porcelain", "--set-upstream", &remote, &refspec],
                )
            })?;
            return Ok(());
        }
        planner::PushPlan::NeedsCommit => {
            return ui::message(
                &lang,
                "Push",
                "Create the initial commit first / 初回コミットが必要です",
            )
        }
        planner::PushPlan::NeedsRemote => return crate::workflows::setup::from_tui(),
        planner::PushPlan::DetachedHead => {
            return ui::message(
                &lang,
                "Push",
                "Select a branch before pushing / ブランチを選択してください",
            )
        }
        planner::PushPlan::BrokenTracking { reason } => return ui::message(&lang, "Push", &reason),
        planner::PushPlan::NormalPush => {}
    }
    let plan = match git::push_plan(&path) {
        Ok(v) => v,
        Err(e) => return ui::message(&lang, "Push blocked", &e),
    };
    if !ui::cui_yes_no(
        &lang,
        "Review Push",
        &format!(
            "{}\n{} -> {}\nCommit: {}\nSend without force?",
            process::safe_text(&plan.url),
            plan.branch,
            plan.target,
            plan.head
        ),
    )? {
        return Ok(());
    }
    actions::reset();
    let result = ui::cui_operation_result(&lang, &t(&lang, "repo.push"), || {
        actions::push(&path, &plan)
    })?;
    match result {
        Err(error)
            if is_gpg_error(&error)
                && gpg_recovery(tui, &path, &lang)?
                && ui::cui_yes_no(
                    &lang,
                    &t(&lang, "gpg.retry_title"),
                    &t(&lang, "gpg.retry_push"),
                )? =>
        {
            actions::reset();
            let retry = ui::cui_operation_result(&lang, &t(&lang, "repo.push"), || {
                actions::push(&path, &plan)
            })?;
            let _ = retry;
        }
        _ => {}
    }
    Ok(())
}
