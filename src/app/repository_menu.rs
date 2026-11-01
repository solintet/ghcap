//! EN: repository menu UI workflow with one responsibility per Rust module.
//! JA: repository menu の画面と操作を独立したRustモジュールとして管理します。
use super::{
    clone_repo, commit_push, commit_repo, git_setup, gpg_menu, presets, push_repo, release_flow,
    repo_settings, t,
};
use crate::git;
use crate::models::{Account, Config, Repo};
use crate::storage;
use crate::ui::{self, Tui};
use std::io;
use std::path::Path;

pub(super) fn local_repo(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    if git::current_repo().is_none() {
        return crate::workflows::setup::from_tui();
    }
    let remote = git::remote_origin().unwrap_or_default();
    let repo = repo_from_remote(&remote);
    if repo.full_name.is_empty() {
        // EN: A local-only repository should open guided setup instead of a dead-end.
        // JA: リモート未接続でも行き止まりにせず、初回セットアップへ案内します。
        return crate::workflows::setup::from_tui();
    }
    config.last_repo = Some(repo.full_name.clone());
    let account = config
        .accounts
        .get(config.last_account)
        .cloned()
        .unwrap_or_default();
    let mut rc = storage::load_repo_config(&repo.full_name)?;
    rc.local_path = git::current_repo();
    storage::save_repo_config(&repo.full_name, &rc)?;
    repo_menu(config, tui, &account, &repo)
}

pub(super) fn repo_from_remote(remote: &str) -> Repo {
    let full_name = git::github_identity(remote).unwrap_or_default();
    let name = full_name.rsplit('/').next().unwrap_or_default().to_owned();
    Repo {
        name,
        full_name,
        clone_url: remote.into(),
        ssh_url: remote.into(),
        private: false,
    }
}

pub(super) fn repo_menu(
    config: &mut Config,
    tui: &mut Tui,
    account: &Account,
    repo: &Repo,
) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "repo.commit_push"),
            t(&lang, "repo.commit"),
            t(&lang, "repo.push"),
            t(&lang, "repo.status"),
            t(&lang, "repo.gpg"),
            t(&lang, "repo.presets"),
            t(&lang, "repo.settings"),
            t(&lang, "repo.git_setup"),
            t(&lang, "repo.info"),
            t(&lang, "repo.clone"),
            "GitHub Release (folder upload)".to_string(),
            t(&lang, "common.back"),
        ];
        let Some(index) = ui::select(tui, &lang, &repo.full_name, &items)? else {
            return Ok(());
        };
        match index {
            0 => commit_push(config, tui, repo)?,
            1 => commit_repo(config, tui, repo)?,
            2 => push_repo(config, tui, repo)?,
            3 => {
                let rc = storage::load_repo_config(&repo.full_name)?;
                if let Some(path) = rc.local_path {
                    ui::message(
                        &lang,
                        &t(&lang, "repo.status"),
                        &git::status_for(Path::new(&path)),
                    )?;
                } else {
                    ui::message(
                        &lang,
                        &t(&lang, "repo.status"),
                        &t(&lang, "git.not_configured"),
                    )?;
                }
            }
            4 => gpg_menu(tui, config)?,
            5 => presets(config, tui)?,
            6 => repo_settings(config, tui, repo)?,
            7 => {
                let _ = git_setup(config, tui, repo)?;
            }
            8 => {
                let message = format!(
                    "{}: {}\n{}: {}\nClone: {}",
                    t(&lang, "info.account"),
                    account.name,
                    t(&lang, "info.repository"),
                    repo.full_name,
                    repo.clone_url
                );
                ui::message(&lang, &t(&lang, "repo.info"), &message)?;
            }
            9 => clone_repo(config, tui, repo)?,
            10 => release_flow(config, tui, repo)?,
            _ => return Ok(()),
        }
    }
}
