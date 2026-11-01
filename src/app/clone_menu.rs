//! EN: clone menu UI workflow with one responsibility per Rust module.
//! JA: clone menu の画面と操作を独立したRustモジュールとして管理します。
use super::{repo_path, t};
use crate::git;
use crate::github;
use crate::models::{Config, Repo};
use crate::storage;
use crate::ui::{self, Tui};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn clone_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let default = repo_path(repo)?
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| repo.name.clone());
    let Some(destination) = ui::prompt(tui, &t(&lang, "clone.destination"), &default)? else {
        return Ok(());
    };
    let destination_path = if Path::new(&destination).is_absolute() {
        PathBuf::from(&destination)
    } else {
        std::env::current_dir()?.join(&destination)
    };
    let result = ui::cui_operation_result(&lang, &t(&lang, "clone.title"), || {
        github::clone(repo, &destination)
    });
    if result?.is_ok() {
        let actual = fs::canonicalize(&destination_path).unwrap_or(destination_path);
        if !git::is_repo(&actual) {
            ui::message(&lang, &t(&lang, "common.error"), &t(&lang, "clone.not_git"))?;
            return Ok(());
        }
        let mut rc = storage::load_repo_config(&repo.full_name)?;
        rc.local_path = Some(actual.display().to_string());
        storage::save_repo_config(&repo.full_name, &rc)?;
    }
    Ok(())
}
