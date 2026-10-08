use crate::{actions, git, github, gpg, i18n, models::{Account, Config, Repo}, storage, ui::{self, Tui}};
use std::io;

fn t(lang: &str, key: &str) -> String { i18n::text(lang, key) }

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
        let Some(index) = ui::select(tui, &lang, &t(&lang, "home.title"), &items)? else { break };
        match index {
            0 => accounts(&mut config, tui)?,
            1 => local_repo(&mut config, tui)?,
            2 => presets(&mut config, tui)?,
            3 => settings(&mut config, tui)?,
            _ => break,
        }
    }
    storage::save_config(&config).map_err(|e| io::Error::other(e))
}

fn accounts(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let mut items = config.accounts.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
        items.push(t(&lang, "accounts.add"));
        let Some(index) = ui::select(tui, &lang, &t(&lang, "accounts.title"), &items)? else { return Ok(()) };
        if index == config.accounts.len() {
            let mut command = github::login_command();
            if ui::cui_login(&lang, &mut command, "https://github.com/login/device")? {
                match github::accounts() {
                    Ok(accounts) if !accounts.is_empty() => { config.accounts = accounts; config.last_account = config.accounts.len() - 1; storage::save_config(config)?; }
                    Ok(_) => { let _ = ui::cui_operation(&lang, &t(&lang, "common.error"), || Err(t(&lang, "accounts.none")))?; }
                    Err(e) => { let _ = ui::cui_operation(&lang, &t(&lang, "common.error"), || Err(e))?; }
                }
            }
        } else {
            let account = config.accounts[index].clone();
            config.last_account = index;
            if !ui::cui_operation(&lang, &t(&lang, "accounts.switch"), || github::switch_account(&account.name).map(|_| String::new()))? {
                continue;
            }
            match github::repos(&account) {
                Ok(repositories) => {
                    if repositories.is_empty() {
                        ui::message(tui, &lang, &t(&lang, "repos.title"), &t(&lang, "repos.empty"))?;
                    } else if let Some(repo_index) = repo_select(config, tui, &account, &repositories)? {
                        let repo = repositories[repo_index].clone();
                        config.last_repo = Some(repo.full_name.clone());
                        storage::save_config(config)?;
                        repo_menu(config, tui, &account, &repo)?;
                    }
                }
                Err(error) => { ui::cui_operation(&lang, &t(&lang, "common.error"), || Err(format!("{}: {error}", t(&lang, "accounts.repo_failed"))))?; },
            }
        }
    }
}

fn repo_select(config: &Config, tui: &mut Tui, account: &Account, repositories: &[Repo]) -> io::Result<Option<usize>> {
    let lang = config.language.clone();
    let items = repositories.iter().map(|repo| {
        format!("{}{}", repo.full_name, if repo.private { "  [private]" } else { "" })
    }).collect::<Vec<_>>();
    ui::select(tui, &lang, &format!("{}: {}", t(&lang, "repos.title"), account.name), &items)
}

fn local_repo(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    if git::current_repo().is_none() {
        return ui::message(tui, &lang, &t(&lang, "common.error"), &t(&lang, "local.not_repo"));
    }
    let remote = git::remote_origin().unwrap_or_default();
    let repo = repo_from_remote(&remote);
    if repo.full_name.is_empty() {
        return ui::message(tui, &lang, &t(&lang, "common.error"), &t(&lang, "local.no_remote"));
    }
    config.last_repo = Some(repo.full_name.clone());
    let account = config.accounts.get(config.last_account).cloned().unwrap_or_default();
    repo_menu(config, tui, &account, &repo)
}

fn repo_from_remote(remote: &str) -> Repo {
    let mut full_name = remote.trim().trim_end_matches('/').to_string();
    if let Some(rest) = full_name.strip_prefix("git@github.com:") { full_name = rest.to_string(); }
    if let Some(rest) = full_name.strip_prefix("https://github.com/") { full_name = rest.to_string(); }
    if let Some(rest) = full_name.strip_prefix("http://github.com/") { full_name = rest.to_string(); }
    if let Some(rest) = full_name.strip_prefix("ssh://git@github.com/") { full_name = rest.to_string(); }
    if full_name.ends_with(".git") { full_name.truncate(full_name.len() - 4); }
    let name = full_name.rsplit('/').next().unwrap_or("").to_string();
    Repo { name, full_name, clone_url: remote.into(), ssh_url: remote.into(), private: false }
}

fn repo_menu(config: &mut Config, tui: &mut Tui, account: &Account, repo: &Repo) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "repo.commit_push"), t(&lang, "repo.commit"), t(&lang, "repo.push"),
            t(&lang, "repo.status"), t(&lang, "repo.gpg"), t(&lang, "repo.presets"),
            t(&lang, "repo.settings"), t(&lang, "repo.info"), t(&lang, "repo.clone"), t(&lang, "common.back"),
        ];
        let Some(index) = ui::select(tui, &lang, &repo.full_name, &items)? else { return Ok(()) };
        match index {
            0 => commit_push(config, tui, repo)?,
            1 => {
                actions::reset();
                let message = ui::prompt(tui, &t(&lang, "commit.message"), &default_message(config, repo))?.unwrap_or_default();
                ui::cui_operation(&lang, &t(&lang, "repo.commit"), || actions::commit(&message))?;
            }
            2 => { actions::reset(); ui::cui_operation(&lang, &t(&lang, "repo.push"), actions::push)?; }
            3 => ui::message(tui, &lang, &t(&lang, "repo.status"), &git::status())?,
            4 => gpg_menu(tui, config, account)?,
            5 => presets(config, tui)?,
            6 => repo_settings(config, tui, repo)?,
            7 => {
                let message = format!("{}: {}\n{}: {}\nClone: {}", t(&lang, "info.account"), account.name, t(&lang, "info.repository"), repo.full_name, repo.clone_url);
                ui::message(tui, &lang, &t(&lang, "repo.info"), &message)?;
            }
            8 => clone_repo(config, tui, repo)?,
            _ => return Ok(()),
        }
    }
}

fn clone_repo(config: &mut Config, _tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let destination = match ui::cui_prompt(&t(&lang, "clone.destination"), &repo.name)? {
        Some(value) => value,
        None => return Ok(()),
    };
    let _ = ui::cui_operation(&lang, &t(&lang, "clone.title"), || github::clone(repo, &destination))?;
    Ok(())
}

fn default_message(config: &Config, repo: &Repo) -> String {
    let repo_config = storage::load_repo_config(&repo.full_name);
    config.presets.iter().find(|p| p.id == repo_config.preset_id).or_else(|| config.presets.first()).map(|p| p.message.clone()).unwrap_or_else(|| "Update: ".into())
}

fn commit_push(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let message = ui::cui_prompt(&t(&lang, "commit.message"), &default_message(config, repo))?.unwrap_or_default();
    let repo_config = storage::load_repo_config(&repo.full_name);

    if repo_config.pre_push_enabled && !repo_config.pre_push_command.trim().is_empty() && repo_config.shell_confirmation {
        let choices = vec![t(&lang, "shell.commit_then_run"), t(&lang, "shell.run_then_commit"), t(&lang, "shell.skip"), t(&lang, "common.cancel")];
        let Some(choice) = ui::select(tui, &lang, &t(&lang, "shell.detected"), &choices)? else { return Ok(()) };
        match choice {
            0 => { let _ = ui::cui_operation(&lang, &t(&lang, "repo.commit_push"), || {
                let commit = actions::commit(&message)?;
                if commit == "Commit cancelled." { return Ok(commit); }
                run_shell(&repo_config.pre_push_command)?;
                actions::push()
            })?; },
            1 => { let _ = ui::cui_operation(&lang, &t(&lang, "repo.commit_push"), || {
                run_shell(&repo_config.pre_push_command)?;
                actions::commit(&message)?;
                actions::push()
            })?; },
            2 => { let _ = ui::cui_operation(&lang, &t(&lang, "repo.commit_push"), || actions::commit(&message).and_then(|_| actions::push()))?; },
            _ => {}
        }
    } else {
        let _ = ui::cui_operation(&lang, &t(&lang, "repo.commit_push"), || actions::commit_push(repo, config, &message))?;
    }
    Ok(())
}

fn run_shell(command: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/C", command]).status().map_err(|e| e.to_string()).and_then(|s| if s.success() { Ok(()) } else { Err(format!("status {s}")) }) }
    #[cfg(not(target_os = "windows"))]
    { std::process::Command::new("sh").args(["-c", command]).status().map_err(|e| e.to_string()).and_then(|s| if s.success() { Ok(()) } else { Err(format!("status {s}")) }) }
}

fn presets(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let mut items = config.presets.iter().map(|p| format!("{} — {}", p.name, p.message)).collect::<Vec<_>>();
        items.push(t(&lang, "presets.add"));
        let Some(index) = ui::select(tui, &lang, &t(&lang, "repo.presets"), &items)? else { return Ok(()) };
        if index == config.presets.len() {
            let name = ui::prompt(tui, &t(&lang, "presets.name"), "New preset")?.unwrap_or_default();
            let message = ui::prompt(tui, &t(&lang, "presets.message"), "Update: ")?.unwrap_or_default();
            let id = format!("preset-{}", config.presets.len() + 1);
            config.presets.push(crate::models::Preset { id, name, message });
        } else {
            let choices = vec![t(&lang, "presets.edit"), t(&lang, "presets.up"), t(&lang, "presets.down"), t(&lang, "presets.delete"), t(&lang, "common.back")];
            match ui::select(tui, &lang, &t(&lang, "presets.action"), &choices)? {
                Some(0) => {
                    let name = ui::prompt(tui, &t(&lang, "presets.name"), &config.presets[index].name)?.unwrap_or_default();
                    let message = ui::prompt(tui, &t(&lang, "presets.message"), &config.presets[index].message)?.unwrap_or_default();
                    config.presets[index].name = name; config.presets[index].message = message;
                }
                Some(1) if index > 0 => config.presets.swap(index, index - 1),
                Some(2) if index + 1 < config.presets.len() => config.presets.swap(index, index + 1),
                Some(3) => { config.presets.remove(index); }
                _ => {}
            }
        }
        storage::save_config(config)?;
    }
}

fn repo_settings(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let mut settings = storage::load_repo_config(&repo.full_name);
    loop {
        let items = vec![
            format!("{}: {}", t(&lang, "settings.pre_shell"), if settings.pre_push_enabled { "ON" } else { "OFF" }),
            format!("{}: {}", t(&lang, "settings.shell_command"), if settings.pre_push_command.is_empty() { "<none>" } else { &settings.pre_push_command }),
            format!("{}: {}", t(&lang, "settings.shell_confirmation"), if settings.shell_confirmation { "ON" } else { "OFF" }),
            t(&lang, "settings.preset"),
            t(&lang, "common.back"),
        ];
        let Some(index) = ui::select(tui, &lang, &t(&lang, "repo.settings"), &items)? else { return Ok(()) };
        match index {
            0 => settings.pre_push_enabled = !settings.pre_push_enabled,
            1 => settings.pre_push_command = ui::prompt(tui, &t(&lang, "settings.shell_command"), &settings.pre_push_command)?.unwrap_or_default(),
            2 => settings.shell_confirmation = !settings.shell_confirmation,
            3 => {
                if config.presets.is_empty() { ui::message(tui, &lang, &t(&lang, "settings.preset"), &t(&lang, "presets.none"))?; }
                else {
                    let names = config.presets.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
                    if let Some(i) = ui::select(tui, &lang, &t(&lang, "settings.preset"), &names)? { settings.preset_id = config.presets[i].id.clone(); }
                }
            }
            _ => { storage::save_repo_config(&repo.full_name, &settings)?; return Ok(()); }
        }
        storage::save_repo_config(&repo.full_name, &settings)?;
    }
}

fn settings(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![t(&lang, "settings.change_language"), t(&lang, "settings.config_path"), t(&lang, "common.back")];
        let Some(index) = ui::select(tui, &lang, &t(&lang, "home.settings"), &items)? else { return Ok(()) };
        match index {
            0 => {
                let choices = i18n::available().iter().map(|code| format!("{} ({code})", i18n::lang_name(code))).collect::<Vec<_>>();
                if let Some(i) = ui::select(tui, &lang, &t(&lang, "settings.change_language"), &choices)? {
                    config.language = i18n::available()[i].to_string();
                    storage::save_config(config)?;
                }
            }
            1 => ui::message(tui, &lang, &t(&lang, "settings.config_path"), &storage::cfg_path().display().to_string())?,
            _ => return Ok(()),
        }
    }
}

fn gpg_menu(tui: &mut Tui, config: &Config, account: &Account) -> io::Result<()> {
    let lang = config.language.clone();
    let items = vec![t(&lang, "gpg.list"), t(&lang, "gpg.generate"), t(&lang, "gpg.register"), t(&lang, "common.back")];
    let Some(index) = ui::select(tui, &lang, &t(&lang, "repo.gpg"), &items)? else { return Ok(()) };
    match index {
        0 => { ui::cui_operation(&lang, &t(&lang, "gpg.list"), gpg::list_secret)?; }
        1 => {
            let uid = ui::cui_prompt(&t(&lang, "gpg.uid"), "Name <email>")?.unwrap_or_default();
            ui::cui_operation(&lang, &t(&lang, "gpg.generate"), || gpg::generate(&uid).map(|_| t(&lang, "gpg.generated")))?;
        }
        2 => {
            ui::cui_operation(&lang, &t(&lang, "gpg.register"), || gpg::register(account).map(|_| t(&lang, "gpg.registered")))?;
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
        let repo = repo_from_remote("https://github.com/example/project.git");
        assert_eq!(repo.full_name, "example/project");
        assert_eq!(repo.name, "project");
    }

    #[test]
    fn parses_ssh_remote() {
        let repo = repo_from_remote("git@github.com:example/project.git");
        assert_eq!(repo.full_name, "example/project");
        assert_eq!(repo.name, "project");
    }

    #[test]
    fn parses_ssh_url_remote() {
        let repo = repo_from_remote("ssh://git@github.com/example/project.git");
        assert_eq!(repo.full_name, "example/project");
        assert_eq!(repo.name, "project");
    }
}
