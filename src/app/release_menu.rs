//! EN: release menu UI workflow with one responsibility per Rust module.
//! JA: release menu の画面と操作を独立したRustモジュールとして管理します。
use super::repo_local_path;
use crate::git;
use crate::models::{Config, Repo};
use crate::ui::{self, Tui};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn release_flow(config: &Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.as_str();
    let Some(repo_path) = repo_local_path(repo) else {
        return ui::message(
            lang,
            "GitHub Release",
            "This repository has no configured local path. Run Git Setup first.",
        );
    };
    let Some(directory) = ui::cui_prompt(
        "Release staging folder (created automatically; put release-notes.md and assets inside)",
        "release-staging",
    )?
    else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    let directory = if directory.is_absolute() {
        directory
    } else {
        Path::new(&repo_path).join(directory)
    };
    if let Err(error) = fs::create_dir_all(&directory) {
        return ui::message(
            lang,
            "GitHub Release: cannot create folder",
            &format!("{}: {error}", directory.display()),
        );
    }
    let bundle = match crate::release::collect_bundle(&directory) {
        Ok(bundle) => bundle,
        Err(error) => return ui::message(lang, "GitHub Release: folder validation failed", &error),
    };
    let asset_list = bundle
        .assets
        .iter()
        .map(|p| format!("- {}", p.file_name().unwrap_or_default().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(tag) = ui::cui_prompt("Release tag (e.g. v1.0.0)", "v0.1.0")? else {
        return Ok(());
    };
    let tag = tag.trim().to_string();
    if let Err(error) = crate::release::validate_tag(&tag) {
        return ui::message(lang, "GitHub Release", &error);
    }
    let Some(title) = ui::cui_prompt("Release title", &tag)? else {
        return Ok(());
    };
    let title = title.trim().to_string();
    if title.is_empty() {
        return ui::message(lang, "GitHub Release", "Release title must not be empty.");
    }
    let kinds = vec![
        "Release".to_string(),
        "Pre-release".to_string(),
        "Draft".to_string(),
    ];
    let Some(kind_index) = ui::select(tui, lang, "Release type", &kinds)? else {
        return Ok(());
    };
    let kind = match kind_index {
        0 => crate::release::ReleaseKind::Release,
        1 => crate::release::ReleaseKind::Prerelease,
        _ => crate::release::ReleaseKind::Draft,
    };
    if let Err(error) = git::validate_root(Path::new(&repo_path)) {
        return ui::message(lang, "Release", &error);
    }
    let expected_url = format!("https://github.com/{}.git", repo.full_name);
    if !git::origin(Path::new(&repo_path)).is_some_and(|url| git::same_remote(&url, &expected_url))
    {
        return ui::message(
            lang,
            "Release",
            "Configured folder and selected GitHub repository do not match. No upload attempted.",
        );
    }
    for key in ["GH_REPO", "GH_HOST"] {
        if std::env::var_os(key).is_some() {
            return ui::message(
                lang,
                "Release",
                &format!("{key} overrides the GitHub target. Review your environment first."),
            );
        }
    }
    let target = match git::git_in(
        Some(Path::new(&repo_path)),
        &["rev-parse", "--verify", "HEAD"],
    ) {
        Ok(v) => v,
        Err(e) => return ui::message(lang, "Release", &e),
    };
    // Existing remote tags must refer to the exact reviewed source SHA.
    let refs = match git::git_in(
        Some(Path::new(&repo_path)),
        &[
            "ls-remote",
            "--tags",
            "origin",
            &format!("refs/tags/{tag}"),
            &format!("refs/tags/{tag}^{{}}"),
        ],
    ) {
        Ok(v) => v,
        Err(e) => return ui::message(lang, "Release", &e),
    };
    let existing = refs
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .next_back();
    if existing.is_some_and(|(sha, _)| sha != target) {
        return ui::message(
            lang,
            "Release",
            "Existing tag points to a different source commit. It will not be overwritten.",
        );
    }
    let notes_preview = fs::read_to_string(&bundle.notes_file)?;
    ui::message(
        lang,
        "Snapshot release notes — review before upload",
        &notes_preview,
    )?;
    let summary = format!(
        "Repository: {}\nTag: {}\nTitle: {}\nType: {}\nNotes: {}\nAssets ({}):\n{}\n\nCreate/upload now?",
        repo.full_name, tag, title, kinds[kind_index], format_args!("{} (source {target})",bundle.notes_file.display()), bundle.assets.len(), asset_list
    );
    if !ui::cui_confirm_enter(lang, "GitHub Release: review", &summary)? {
        return Ok(());
    }

    // If a release already exists, offer asset upload only. Existing assets are never clobbered.
    let view_args = [
        "release",
        "view",
        tag.as_str(),
        "--repo",
        repo.full_name.as_str(),
    ];
    ui::leave_tui()?;
    println!("\n=== Checking GitHub Release ===\n");
    let view_result = crate::process::capture_with_wait("gh", &view_args, None);
    ui::enter_tui()?;
    let view = view_result.map_err(io::Error::other)?;
    let exists = if view.success {
        true
    } else {
        let diagnostic = format!("{}\n{}", view.stdout, view.stderr).to_ascii_lowercase();
        if diagnostic.contains("release not found")
            || diagnostic.contains("no release found")
            || diagnostic.contains("http 404")
            || diagnostic.contains("not found: release")
        {
            false
        } else {
            return ui::message(
                lang,
                "GitHub Release: cannot check tag",
                &format!("{}\n{}", view.stderr.trim(), view.stdout.trim()),
            );
        }
    };
    if exists && !ui::cui_yes_no(lang, "Existing release", &format!("Release {tag} exists. Upload these snapshot assets without overwriting? Previous partial batches must be reviewed using gh first."))? {return Ok(());}
    let _ = ui::cui_operation_result(lang, "GitHub Release", || {
        if exists {
            crate::release::upload_to_existing(&repo.full_name, &tag, &bundle)
        } else {
            crate::release::create_release(&repo.full_name, &tag, &title, &bundle, kind, &target)
        }
    })?;
    Ok(())
}
