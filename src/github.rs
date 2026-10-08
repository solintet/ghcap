use crate::models::{Account, Repo};
use std::process::Command;

fn gh(args: &[&str]) -> Result<String, String> {
    let output = Command::new("gh").args(args).output().map_err(|e| format!("gh: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!("{}\nexit code {}", if stderr.is_empty() { stdout } else { stderr }, output.status.code().unwrap_or(1)))
    }
}

pub fn accounts() -> Result<Vec<Account>, String> {
    let output = Command::new("gh")
        .args(["auth", "status", "--hostname", "github.com"])
        .output()
        .map_err(|e| format!("gh auth status: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    let mut accounts = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let marker = "Logged in to github.com account ";
        if let Some(rest) = trimmed.strip_prefix(marker) {
            let login = rest.split_whitespace().next().unwrap_or("");
            if !login.is_empty() && !accounts.iter().any(|a: &Account| a.name == login) {
                accounts.push(Account { name: login.to_string(), token: String::new(), label: String::new() });
            }
        }
    }

    if accounts.is_empty() {
        let login = gh(&["api", "user", "--jq", ".login"])?;
        if !login.is_empty() {
            accounts.push(Account { name: login, token: String::new(), label: String::new() });
        }
    }
    Ok(accounts)
}

pub fn switch_account(login: &str) -> Result<(), String> {
    let status = Command::new("gh")
        .args(["auth", "switch", "--hostname", "github.com", "--user", login])
        .status()
        .map_err(|e| format!("gh auth switch: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("gh auth switch failed\nexit code {}", status.code().unwrap_or(1))) }
}

pub fn login_command() -> Command {
    let mut command = Command::new("gh");
    command.args(["auth", "login", "--hostname", "github.com", "--web", "--git-protocol", "https"]);
    command
}

pub fn clone(repo: &Repo, destination: &str) -> Result<String, String> {
    if destination.trim().is_empty() { gh(&["repo", "clone", &repo.full_name]) } else { gh(&["repo", "clone", &repo.full_name, destination]) }
}

pub fn repos(account: &Account) -> Result<Vec<Repo>, String> {
    let owner = if account.name.is_empty() { gh(&["api", "user", "--jq", ".login"])? } else { account.name.clone() };
    let raw = gh(&["repo", "list", &owner, "--limit", "1000"])?;
    let mut repositories = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Showing ") { continue; }
        let name = trimmed.split_whitespace().next().unwrap_or("");
        if name.is_empty() || name.eq_ignore_ascii_case("name") || name.eq_ignore_ascii_case("repository") { continue; }
        let full_name = if name.contains('/') { name.to_string() } else { format!("{owner}/{name}") };
        if !repositories.iter().any(|r: &Repo| r.full_name == full_name) {
            let clone_url = format!("https://github.com/{full_name}.git");
            let ssh_url = format!("git@github.com:{full_name}.git");
            repositories.push(Repo {
                name: name.rsplit('/').next().unwrap_or(name).to_string(),
                full_name,
                clone_url,
                ssh_url,
                private: false,
            });
        }
    }
    Ok(repositories)
}

pub fn register_gpg(armored: &str) -> Result<(), String> {
    let result = Command::new("gh")
        .args(["api", "user/gpg_keys", "--method", "POST", "-f", &format!("armored_public_key={armored}")])
        .output()
        .map_err(|e| format!("gh api: {e}"))?;
    if result.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&result.stderr).trim().to_string()) }
}
