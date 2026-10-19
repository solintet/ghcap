use crate::{
    actions, git, github, gpg, i18n,
    models::{Account, Config, Repo},
    process, storage,
    ui::{self, ListAction, Tui},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn t(lang: &str, key: &str) -> String {
    i18n::text(lang, key)
}

pub fn run(mut config: Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "home.accounts"),
            t(&lang, "home.local"),
            t(&lang, "home.presets"),
            t(&lang, "home.settings"),
            t(&lang, "common.quit"),
        ];
        let Some(index) = ui::select(tui, &lang, &t(&lang, "home.title"), &items)? else {
            break;
        };
        match index {
            0 => accounts(&mut config, tui)?,
            1 => local_repo(&mut config, tui)?,
            2 => presets(&mut config, tui)?,
            3 => settings(&mut config, tui)?,
            _ => break,
        }
    }
    storage::save_config(&config).map_err(io::Error::other)
}

fn accounts(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let mut selected = 0usize;
    loop {
        let lang = config.language.clone();
        let mut items = config
            .accounts
            .iter()
            .map(|a| a.display_name().to_string())
            .collect::<Vec<_>>();
        items.push(t(&lang, "accounts.add"));
        let add_row_index = config.accounts.len();
        let footer = format!(
            "{}  Enter {}  a {}  e {}  d {}  Shift+↑/↓ {}  Esc {}",
            t(&lang, "key.navigate"),
            t(&lang, "accounts.open_key"),
            t(&lang, "accounts.add_key"),
            t(&lang, "accounts.edit_key"),
            t(&lang, "accounts.delete_key"),
            t(&lang, "accounts.move_key"),
            t(&lang, "common.back")
        );
        let Some((index, action)) = ui::manage_select(
            tui,
            &lang,
            &t(&lang, "accounts.title"),
            &items,
            &footer,
            &mut selected,
        )?
        else {
            return Ok(());
        };
        let add_row = index == add_row_index;
        match action {
            ListAction::Open if add_row => {
                add_account(config, tui)?;
                selected = config.accounts.len().saturating_sub(1);
            }
            ListAction::Open => open_account_repos(config, tui, index)?,
            ListAction::Add => {
                add_account(config, tui)?;
                selected = config.accounts.len().saturating_sub(1);
            }
            ListAction::Edit if !add_row => edit_account(config, tui, index)?,
            ListAction::Delete if !add_row => {
                delete_account(config, tui, index)?;
                selected = selected.min(config.accounts.len());
            }
            ListAction::MoveUp if !add_row && index > 0 => {
                config.accounts.swap(index, index - 1);
                selected = index - 1;
                adjust_last_account_after_move(config, index, index - 1);
                storage::save_config(config)?;
            }
            ListAction::MoveDown if !add_row && index + 1 < config.accounts.len() => {
                config.accounts.swap(index, index + 1);
                selected = index + 1;
                adjust_last_account_after_move(config, index, index + 1);
                storage::save_config(config)?;
            }
            ListAction::Back => return Ok(()),
            _ => {}
        }
    }
}

fn add_account(config: &mut Config, _tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    let mut command = github::login_command();
    if !ui::cui_login(&lang, &mut command, "https://github.com/login/device")? {
        return Ok(());
    }
    match github::accounts() {
        Ok(accounts) if !accounts.is_empty() => {
            // Merge gh's authenticated accounts into the saved list. Never replace
            // the user's account list: it is the source of remembered labels/order.
            let mut newly_added = None;
            for mut account in accounts {
                if let Some(existing_index) = config
                    .accounts
                    .iter()
                    .position(|saved| saved.name.eq_ignore_ascii_case(&account.name))
                {
                    // Preserve the saved label and any local metadata for existing accounts.
                    account.label = config.accounts[existing_index].label.clone();
                } else {
                    config.accounts.push(account.clone());
                    newly_added = Some(config.accounts.len() - 1);
                }
            }
            if let Some(index) = newly_added {
                config.last_account = index;
            }
            storage::save_config(config)?;
        }
        Ok(_) => {
            let _ = ui::cui_operation(&lang, &t(&lang, "common.error"), || {
                Err(t(&lang, "accounts.none"))
            })?;
        }
        Err(e) => {
            let _ = ui::cui_operation(&lang, &t(&lang, "common.error"), || Err(e))?;
        }
    }
    Ok(())
}

fn edit_account(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    let current = config.accounts[index].label.clone();
    let account_name = config.accounts[index].name.clone();
    let default = if current.is_empty() {
        account_name.clone()
    } else {
        current
    };
    let Some(label) = ui::prompt(tui, &t(&lang, "accounts.edit_label"), &default)? else {
        return Ok(());
    };
    config.accounts[index].label = if label.trim() == account_name {
        String::new()
    } else {
        label.trim().to_string()
    };
    storage::save_config(config)
}

fn delete_account(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    let name = config.accounts[index].display_name().to_string();
    if !ui::confirm(
        tui,
        &lang,
        &t(&lang, "accounts.delete"),
        &format!("{}: {name}", t(&lang, "accounts.delete_confirm")),
    )? {
        return Ok(());
    }
    let selected_name = config
        .accounts
        .get(config.last_account)
        .map(|a| a.name.clone());
    config.accounts.remove(index);
    if let Some(name) = selected_name {
        if let Some(i) = config.accounts.iter().position(|a| a.name == name) {
            config.last_account = i;
        }
    }
    if config.accounts.is_empty() {
        config.last_account = 0;
    } else if config.last_account >= config.accounts.len() {
        config.last_account = config.accounts.len() - 1;
    }
    storage::save_config(config)
}

fn adjust_last_account_after_move(config: &mut Config, from: usize, to: usize) {
    if config.last_account == from {
        config.last_account = to;
    } else if config.last_account == to {
        config.last_account = from;
    }
}

fn open_account_repos(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    let account = config.accounts[index].clone();
    config.last_account = index;
    if !ui::cui_operation(&lang, &t(&lang, "accounts.switch"), || {
        github::switch_account(&account.name).map(|_| String::new())
    })? {
        return Ok(());
    }
    match github::repos(&account) {
        Ok(repositories) => {
            if repositories.is_empty() {
                ui::message(&lang, &t(&lang, "repos.title"), &t(&lang, "repos.empty"))?;
            } else if let Some(i) = repo_select(config, tui, &account, &repositories)? {
                let repo = repositories[i].clone();
                config.last_repo = Some(repo.full_name.clone());
                storage::save_config(config)?;
                repo_menu(config, tui, &account, &repo)?;
            }
        }
        Err(error) => {
            ui::cui_operation(&lang, &t(&lang, "common.error"), || {
                Err(format!("{}: {error}", t(&lang, "accounts.repo_failed")))
            })?;
        }
    }
    storage::save_config(config)?;
    Ok(())
}

fn repo_select(
    config: &Config,
    tui: &mut Tui,
    account: &Account,
    repositories: &[Repo],
) -> io::Result<Option<usize>> {
    let lang = config.language.clone();
    let items = repositories
        .iter()
        .map(|r| {
            format!(
                "{}{}",
                r.full_name,
                if r.private { "  [private]" } else { "" }
            )
        })
        .collect::<Vec<_>>();
    ui::select(
        tui,
        &lang,
        &format!("{}: {}", t(&lang, "repos.title"), account.name),
        &items,
    )
}

fn local_repo(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    if git::current_repo().is_none() {
        return ui::message(
            &lang,
            &t(&lang, "common.error"),
            &t(&lang, "local.not_repo"),
        );
    }
    let remote = git::remote_origin().unwrap_or_default();
    let repo = repo_from_remote(&remote);
    if repo.full_name.is_empty() {
        return ui::message(
            &lang,
            &t(&lang, "common.error"),
            &t(&lang, "local.no_remote"),
        );
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

fn repo_from_remote(remote: &str) -> Repo {
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

fn repo_menu(config: &mut Config, tui: &mut Tui, account: &Account, repo: &Repo) -> io::Result<()> {
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

fn release_flow(config: &Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
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

fn repo_local_path(repo: &Repo) -> Option<String> {
    storage::load_repo_config(&repo.full_name).ok()?.local_path
}

fn repo_path(repo: &Repo) -> io::Result<Option<PathBuf>> {
    Ok(storage::load_repo_config(&repo.full_name)?
        .local_path
        .map(PathBuf::from))
}

fn ensure_git_setup(
    config: &mut Config,
    tui: &mut Tui,
    repo: &Repo,
) -> io::Result<Option<PathBuf>> {
    let expected = format!("https://github.com/{}.git", repo.full_name);
    let mut path = repo_path(repo)?;
    if path.as_ref().is_none_or(|p| {
        !git::is_repo(p) || !git::origin(p).is_some_and(|url| git::same_remote(&url, &expected))
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

fn git_identity_setup(tui: &mut Tui, path: &Path, lang: &str) -> io::Result<bool> {
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

fn git_setup(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<bool> {
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
    let needs_remote = !old_remote
        .as_ref()
        .is_some_and(|url| git::same_remote(url, &remote));
    if needs_remote
        && old_remote.is_some()
        && !ui::cui_yes_no(
            &lang,
            "Change origin",
            &format!(
                "Folder: {}\nOld: {}\nNew: {}\nChange this repository's origin?",
                path.display(),
                process::safe_text(old_remote.as_deref().unwrap_or_default()),
                remote
            ),
        )?
    {
        return Ok(false);
    }
    if needs_remote
        && !ui::cui_operation(&lang, &t(&lang, "git.remote"), || {
            git::set_origin(&path, &remote)
        })?
    {
        return Ok(false);
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

fn clone_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
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

fn commit_message_flow(
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
enum CommitMessageOutcome {
    Commit(String),
    Back,
}

fn commit_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
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

fn push_repo(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(path) = ensure_git_setup(config, tui, repo)? else {
        return Ok(());
    };
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

fn review_commit(tui: &mut Tui, lang: &str, path: &Path) -> io::Result<Option<u64>> {
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
fn commit_push(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
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

fn is_gpg_error(error: &str) -> bool {
    let e = error.to_ascii_lowercase();
    e.contains("gpg failed to sign")
        || e.contains("gpg: signing failed")
        || e.contains("failed to sign the data")
        || e.contains("gpg signing")
        || e.contains("gpg failed")
}

fn gpg_recovery(tui: &mut Tui, path: &Path, lang: &str) -> io::Result<bool> {
    let keys = match gpg::secret_keys() {
        Ok(k) => k,
        Err(e) => {
            ui::cui_operation(lang, &t(lang, "common.error"), || Err(e))?;
            return Ok(false);
        }
    };
    if keys.is_empty() {
        let uid = ui::cui_prompt(&t(lang, "gpg.uid"), "Name <email>")?.unwrap_or_default();
        if uid.trim().is_empty() {
            return Ok(false);
        }
        if !ui::cui_operation(lang, &t(lang, "gpg.generate"), || {
            gpg::generate(&uid).map(|_| t(lang, "gpg.generated"))
        })? {
            return Ok(false);
        }
    }
    let keys = match gpg::secret_keys() {
        Ok(k) => k,
        Err(e) => {
            ui::cui_operation(lang, &t(lang, "common.error"), || Err(e))?;
            return Ok(false);
        }
    };
    let items = keys
        .iter()
        .map(|(id, uid)| format!("{id}  {uid}"))
        .collect::<Vec<_>>();
    let Some(i) = ui::select(tui, lang, &t(lang, "gpg.select"), &items)? else {
        return Ok(false);
    };
    let key = &keys[i].0;
    if !ui::cui_operation(lang, &t(lang, "gpg.register"), || {
        gpg::register_key(key)?;
        git::set_signing_key(path, key).map(|_| t(lang, "gpg.registered"))
    })? {
        return Ok(false);
    }
    Ok(true)
}

fn presets(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let mut selected = 0usize;
    loop {
        let lang = config.language.clone();
        let mut items = config
            .presets
            .iter()
            .map(|p| format!("{} — {}", p.name, p.message))
            .collect::<Vec<_>>();
        items.push(t(&lang, "presets.add"));
        let footer = format!(
            "{}  Enter {}  a {}  e {}  d {}  Shift+↑/↓ {}  Esc {}",
            t(&lang, "key.navigate"),
            t(&lang, "presets.use_key"),
            t(&lang, "presets.add_key"),
            t(&lang, "presets.edit_key"),
            t(&lang, "presets.delete_key"),
            t(&lang, "presets.move_key"),
            t(&lang, "common.back")
        );
        let Some((index, action)) = ui::manage_select(
            tui,
            &lang,
            &t(&lang, "repo.presets"),
            &items,
            &footer,
            &mut selected,
        )?
        else {
            return Ok(());
        };
        let add_row = index == config.presets.len();
        match action {
            ListAction::Open if add_row => {
                add_preset(config, tui)?;
                selected = config.presets.len().saturating_sub(1);
            }
            ListAction::Open => return Ok(()),
            ListAction::Add => add_preset(config, tui)?,
            ListAction::Edit if !add_row => edit_preset(config, tui, index)?,
            ListAction::Delete if !add_row => delete_preset(config, tui, index)?,
            ListAction::MoveUp if !add_row && index > 0 => {
                config.presets.swap(index, index - 1);
                selected -= 1;
                storage::save_config(config)?;
            }
            ListAction::MoveDown if !add_row && index + 1 < config.presets.len() => {
                config.presets.swap(index, index + 1);
                selected += 1;
                storage::save_config(config)?;
            }
            ListAction::Back => return Ok(()),
            _ => {}
        }
    }
}
fn add_preset(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(name) = ui::prompt(tui, &t(&lang, "presets.name"), "New preset")? else {
        return Ok(());
    };
    let Some(message) = ui::prompt(tui, &t(&lang, "presets.message"), "Update: ")? else {
        return Ok(());
    };
    if name.trim().is_empty() {
        return Ok(());
    }
    let id = storage::new_id("preset")?;
    config
        .presets
        .push(crate::models::Preset { id, name, message });
    storage::save_config(config)
}
fn edit_preset(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(name) = ui::prompt(tui, &t(&lang, "presets.name"), &config.presets[index].name)?
    else {
        return Ok(());
    };
    let Some(message) = ui::prompt(
        tui,
        &t(&lang, "presets.message"),
        &config.presets[index].message,
    )?
    else {
        return Ok(());
    };
    config.presets[index].name = name;
    config.presets[index].message = message;
    storage::save_config(config)
}
fn delete_preset(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    if config.presets.len() <= 1 {
        ui::message(
            &lang,
            &t(&lang, "presets.delete"),
            &t(&lang, "presets.last_forbidden"),
        )?;
        return Ok(());
    }
    let used = storage::preset_references(&config.presets[index].id)?;
    if !used.is_empty() {
        return ui::message(
            &lang,
            "Preset in use",
            &format!(
                "Choose another preset in these repository settings first:\n{}",
                used.join("\n")
            ),
        );
    }
    if ui::confirm(
        tui,
        &lang,
        &t(&lang, "presets.delete"),
        &format!(
            "{}: {}",
            t(&lang, "presets.delete_confirm"),
            config.presets[index].name
        ),
    )? {
        config.presets.remove(index);
        storage::save_config(config)?;
    }
    Ok(())
}

fn repo_settings(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let mut settings = storage::load_repo_config(&repo.full_name)?;
    loop {
        let items = vec![
            format!(
                "{}: {}",
                t(&lang, "settings.pre_shell"),
                if settings.pre_push_enabled {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            format!(
                "{}: {}",
                t(&lang, "settings.shell_command"),
                if settings.pre_push_command.is_empty() {
                    "<none>"
                } else {
                    &settings.pre_push_command
                }
            ),
            format!(
                "{}: {}",
                t(&lang, "settings.shell_confirmation"),
                if settings.shell_confirmation {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            t(&lang, "settings.preset"),
            t(&lang, "common.back"),
        ];
        let Some(i) = ui::select(tui, &lang, &t(&lang, "repo.settings"), &items)? else {
            return Ok(());
        };
        match i {
            0 => settings.pre_push_enabled = !settings.pre_push_enabled,
            1 => {
                if let Some(command) = ui::prompt(
                    tui,
                    &t(&lang, "settings.shell_command"),
                    &settings.pre_push_command,
                )? {
                    settings.pre_push_command = command;
                }
            }
            2 => settings.shell_confirmation = !settings.shell_confirmation,
            3 => {
                if config.presets.is_empty() {
                    ui::message(
                        &lang,
                        &t(&lang, "settings.preset"),
                        &t(&lang, "presets.none"),
                    )?;
                } else {
                    let names = config
                        .presets
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>();
                    if let Some(i) = ui::select(tui, &lang, &t(&lang, "settings.preset"), &names)? {
                        settings.preset_id = config.presets[i].id.clone();
                    }
                }
            }
            _ => {
                storage::save_repo_config(&repo.full_name, &settings)?;
                return Ok(());
            }
        }
        storage::save_repo_config(&repo.full_name, &settings)?;
    }
}

fn settings(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "settings.change_language"),
            t(&lang, "settings.config_path"),
            t(&lang, "common.back"),
        ];
        let Some(i) = ui::select(tui, &lang, &t(&lang, "home.settings"), &items)? else {
            return Ok(());
        };
        match i {
            0 => {
                let choices = i18n::available()
                    .iter()
                    .map(|c| format!("{} ({c})", i18n::lang_name(c)))
                    .collect::<Vec<_>>();
                if let Some(i) =
                    ui::select(tui, &lang, &t(&lang, "settings.change_language"), &choices)?
                {
                    config.language = i18n::available()[i].to_string();
                    storage::save_config(config)?;
                }
            }
            1 => ui::message(
                &lang,
                &t(&lang, "settings.config_path"),
                &storage::cfg_path()?.display().to_string(),
            )?,
            _ => return Ok(()),
        }
    }
}

fn gpg_menu(tui: &mut Tui, config: &Config) -> io::Result<()> {
    let lang = config.language.clone();
    let items = vec![
        t(&lang, "gpg.list"),
        t(&lang, "gpg.generate"),
        t(&lang, "gpg.register"),
        t(&lang, "common.back"),
    ];
    let Some(i) = ui::select(tui, &lang, &t(&lang, "repo.gpg"), &items)? else {
        return Ok(());
    };
    match i {
        0 => {
            ui::cui_operation(&lang, &t(&lang, "gpg.list"), gpg::list_secret)?;
        }
        1 => {
            let uid = ui::cui_prompt(&t(&lang, "gpg.uid"), "Name <email>")?.unwrap_or_default();
            ui::cui_operation(&lang, &t(&lang, "gpg.generate"), || {
                gpg::generate(&uid).map(|_| t(&lang, "gpg.generated"))
            })?;
        }
        2 => {
            let keys = match gpg::secret_keys() {
                Ok(v) => v,
                Err(e) => return ui::message(&lang, "GPG", &e),
            };
            let items = keys
                .iter()
                .map(|(id, uid)| format!("{id} {uid}"))
                .collect::<Vec<_>>();
            if items.is_empty() {
                return ui::message(&lang, "GPG", "No usable signing key found.");
            }
            if let Some(index) = ui::select(tui, &lang, "Select public key to register", &items)? {
                if ui::cui_yes_no(&lang, "Register public key", &items[index])? {
                    ui::cui_operation(&lang, &t(&lang, "gpg.register"), || {
                        gpg::register_key(&keys[index].0).map(|_| t(&lang, "gpg.registered"))
                    })?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::repo_from_remote;
    #[test]
    fn parses_https_remote() {
        let r = repo_from_remote("https://github.com/example/project.git");
        assert_eq!(r.full_name, "example/project");
        assert_eq!(r.name, "project");
    }
    #[test]
    fn parses_ssh_remote() {
        let r = repo_from_remote("git@github.com:example/project.git");
        assert_eq!(r.full_name, "example/project");
        assert_eq!(r.name, "project");
    }
    #[test]
    fn parses_ssh_url_remote() {
        let r = repo_from_remote("ssh://git@github.com/example/project.git");
        assert_eq!(r.full_name, "example/project");
        assert_eq!(r.name, "project");
    }
}
