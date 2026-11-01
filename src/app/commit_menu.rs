//! EN: commit menu UI workflow with one responsibility per Rust module.
//! JA: commit menu の画面と操作を独立したRustモジュールとして管理します。
use super::{ensure_git_setup, gpg_recovery, is_gpg_error, t};
use crate::actions;
use crate::git;
use crate::models::{Config, Repo};
use crate::process;
use crate::storage;
use crate::ui::{self, Tui};
use std::io;
use std::path::Path;

pub(super) fn commit_message_flow(
    config: &mut Config,
    tui: &mut Tui,
    repo: &Repo,
) -> io::Result<CommitMessageOutcome> {
    loop {
        let lang = config.language.clone();
        let mut items = config
            .presets
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();
        items.push("新しいプリセットを作成".into());
        let preferred = storage::load_repo_config(&repo.full_name)?.preset_id;
        let initial = config
            .presets
            .iter()
            .position(|p| p.id == preferred)
            .unwrap_or(0);
        let Some(index) =
            ui::select_initial(tui, &lang, &t(&lang, "repo.commit"), &items, initial)?
        else {
            return Ok(CommitMessageOutcome::Back);
        };

        let initial = if index < config.presets.len() {
            config.presets[index].message.clone()
        } else {
            let name =
                ui::prompt(tui, &t(&lang, "presets.name"), "New preset")?.unwrap_or_default();
            if name.trim().is_empty() {
                continue;
            }
            let message =
                ui::prompt(tui, &t(&lang, "presets.message"), "feat: ")?.unwrap_or_default();
            let id = storage::new_id("preset")?;
            config.presets.push(crate::models::Preset {
                id,
                name,
                message: message.clone(),
            });
            storage::save_config(config)?;
            message
        };

        match ui::edit_commit_message(tui, &lang, &initial)? {
            ui::CommitMessageDecision::Commit(message) => {
                return Ok(CommitMessageOutcome::Commit(message))
            }
            ui::CommitMessageDecision::BackToPresets => continue,
            ui::CommitMessageDecision::BackToRepo => return Ok(CommitMessageOutcome::Back),
        }
    }
}

#[derive(Debug)]
pub(super) enum CommitMessageOutcome {
    Commit(String),
    Back,
}

pub(super) fn commit_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(path) = ensure_git_setup(config, tui, repo)? else {
        return Ok(());
    };
    let message = match commit_message_flow(config, tui, repo)? {
        CommitMessageOutcome::Commit(message) => message,
        CommitMessageOutcome::Back => return Ok(()),
    };
    let Some(expected) = review_commit(tui, &lang, &path)? else {
        return Ok(());
    };
    actions::reset();
    let result = ui::cui_operation_result(&lang, &t(&lang, "repo.commit"), || {
        actions::commit(&message, &path, expected)
    })?;
    if let Err(error) = result {
        if is_gpg_error(&error)
            && gpg_recovery(tui, &path, &lang)?
            && ui::cui_yes_no(
                &lang,
                &t(&lang, "gpg.retry_title"),
                &t(&lang, "gpg.retry_commit"),
            )?
        {
            actions::reset();
            let _ = ui::cui_operation_result(&lang, &t(&lang, "repo.commit"), || {
                actions::commit(&message, &path, expected)
            })?;
        }
    }
    Ok(())
}

pub(super) fn review_commit(tui: &mut Tui, lang: &str, path: &Path) -> io::Result<Option<u64>> {
    let fingerprint = match git::commit_fingerprint(path) {
        Ok(v) => v,
        Err(e) => {
            ui::message(lang, "Commit blocked", &e)?;
            return Ok(None);
        }
    };
    let status = git::git_in(Some(path), &["status", "--short", "--untracked-files=all"])
        .map_err(io::Error::other)?;
    ui::message(
        lang,
        "All-files commit — review untracked additions and deletions",
        &process::safe_text(&status),
    )?;
    if ui::cui_yes_no(lang,"Confirm all-files staging","Commit stages ALL non-ignored changes shown, including untracked files and deletions. Continue?")? {Ok(Some(fingerprint))} else {let _=tui;Ok(None)}
}
