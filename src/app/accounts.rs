//! EN: accounts UI workflow with one responsibility per Rust module.
//! JA: accounts の画面と操作を独立したRustモジュールとして管理します。
use super::{repo_menu, t};
use crate::github;
use crate::models::{Account, Config, Repo};
use crate::storage;
use crate::ui::{self, ListAction, Tui};
use std::io;

pub(super) fn accounts(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
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

pub(super) fn add_account(config: &mut Config, _tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    let mut command = match github::login_command() {
        Ok(command) => command,
        Err(error) => {
            let _ = ui::cui_operation(&lang, "GitHub CLI / gh", || Err(error))?;
            return Ok(());
        }
    };
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

pub(super) fn edit_account(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
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

pub(super) fn delete_account(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
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

pub(super) fn adjust_last_account_after_move(config: &mut Config, from: usize, to: usize) {
    if config.last_account == from {
        config.last_account = to;
    } else if config.last_account == to {
        config.last_account = from;
    }
}

pub(super) fn open_account_repos(
    config: &mut Config,
    tui: &mut Tui,
    index: usize,
) -> io::Result<()> {
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

pub(super) fn repo_select(
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
