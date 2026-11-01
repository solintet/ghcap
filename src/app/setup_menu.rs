//! EN: setup menu UI workflow with one responsibility per Rust module.
//! JA: setup menu の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::git;
use crate::models::{Config, Repo};
use crate::process;
use crate::storage;
use crate::ui::{self, Tui};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn repo_local_path(repo: &Repo) -> Option<String> {
    storage::load_repo_config(&repo.full_name).ok()?.local_path
}

pub(super) fn repo_path(repo: &Repo) -> io::Result<Option<PathBuf>> {
    Ok(storage::load_repo_config(&repo.full_name)?
        .local_path
        .map(PathBuf::from))
}

pub(super) fn ensure_git_setup(
    config: &mut Config,
    tui: &mut Tui,
    repo: &Repo,
) -> io::Result<Option<PathBuf>> {
    let expected = format!("https://github.com/{}.git", repo.full_name);
    let mut path = repo_path(repo)?;
    if path.as_ref().is_none_or(|p| {
        if !git::is_repo(p) {
            return true;
        }
        let Ok(names) = git::git_in(Some(p), &["remote"]) else {
            return true;
        };
        !names.lines().any(|remote| {
            git::git_in(Some(p), &["remote", "get-url", remote])
                .is_ok_and(|url| git::same_remote(&url, &expected))
        })
    }) {
        if !git_setup(config, tui, repo)? {
            return Ok(None);
        }
        path = repo_path(repo)?;
    }
    let Some(path) = path else {
        return Ok(None);
    };
    if !git::identity_configured(&path) && !git_identity_setup(tui, &path, &config.language)? {
        return Ok(None);
    }
    Ok(Some(path))
}

pub(super) fn git_identity_setup(tui: &mut Tui, path: &Path, lang: &str) -> io::Result<bool> {
    let name_default = git::user_name(path).unwrap_or_default();
    let email_default = git::user_email(path).unwrap_or_default();
    let Some(name) = ui::prompt(tui, "Git user.name", &name_default)? else {
        return Ok(false);
    };
    if name.trim().is_empty() {
        return Ok(false);
    }
    let Some(email) = ui::prompt(tui, "Git user.email", &email_default)? else {
        return Ok(false);
    };
    if email.trim().is_empty() {
        return Ok(false);
    }
    let result = ui::cui_operation_result(lang, "Git identity", || {
        git::set_identity(path, name.trim(), email.trim())
    });
    Ok(result?.is_ok())
}

pub(super) fn git_setup(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<bool> {
    let lang = config.language.clone();
    let mut settings = storage::load_repo_config(&repo.full_name)?;
    let default = settings.local_path.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.join(&repo.name).display().to_string())
            .unwrap_or_else(|| repo.name.clone())
    });
    let Some(input) = ui::prompt(tui, &t(&lang, "git.folder"), &default)? else {
        return Ok(false);
    };
    let path = PathBuf::from(input.trim());
    if path.as_os_str().is_empty() {
        return Ok(false);
    }
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(&path) {
            ui::cui_operation(&lang, &t(&lang, "common.error"), || {
                Err(format!("{}: {e}", t(&lang, "git.create_folder_failed")))
            })?;
            return Ok(false);
        }
    }
    if git::is_repo(&path) {
        if let Err(error) = git::validate_root(&path) {
            ui::message(&lang, "Git folder", &error)?;
            return Ok(false);
        }
    }
    if !git::is_repo(&path)
        && !ui::cui_operation(&lang, &t(&lang, "git.init"), || git::init(&path))?
    {
        return Ok(false);
    }
    if let Err(error) = git::validate_root(&path) {
        ui::message(&lang, "Git folder", &error)?;
        return Ok(false);
    }
    let remote = format!("https://github.com/{}.git", repo.full_name);
    let old_remote = git::origin(&path);
    match old_remote {
        Some(ref current) if git::same_remote(current, &remote) => {
            // EN: Preserve the existing transport and remote URL verbatim.
            // JA: 既存の接続方式とリモートURLを一切書き換えません。
        }
        Some(current) => {
            return ui::message(
                &lang,
                "Remote already configured / 既存リモート",
                &format!("Existing origin / 現在のorigin: {}\nExpected / 想定: {}\nNo configuration was changed / 設定は変更していません。",
                    process::safe_text(&current), remote),
            ).map(|_| false);
        }
        None => {
            if !ui::cui_yes_no(&lang, "Add origin / originを登録", &remote)? {
                return Ok(false);
            }
            if !ui::cui_operation(&lang, &t(&lang, "git.remote"), || {
                git::set_origin(&path, &remote)
            })? {
                return Ok(false);
            }
        }
    }
    let path = fs::canonicalize(&path).unwrap_or(path);
    settings.local_path = Some(path.display().to_string());
    storage::save_repo_config(&repo.full_name, &settings)?;
    ui::message(
        &lang,
        &t(&lang, "git.ready"),
        &format!(
            "{}\n{}: {}",
            t(&lang, "git.ready_message"),
            t(&lang, "git.folder"),
            path.display()
        ),
    )?;
    Ok(true)
}
