use crate::{models::{Account, Repo}, process};
use std::process::Command;
use serde_json::Value;

fn gh(args: &[&str]) -> Result<String, String> {
    let output = process::capture("gh", args, None)?;
    if output.success { Ok(output.stdout.trim().to_string()) } else { Err(process::failure("gh", &output)) }
}

pub fn accounts() -> Result<Vec<Account>, String> {
    let output = process::capture("gh", &["auth", "status", "--json", "hosts"], None)?;
    let mut accounts = Vec::new();
    if let Ok(value) = serde_json::from_str::<Value>(&output.stdout) {
        if let Some(hosts) = value.get("hosts").and_then(Value::as_object) {
            if let Some(users) = hosts.get("github.com").and_then(Value::as_array) {
                for user in users {
                    let Some(login) = user.get("login").and_then(Value::as_str) else { continue; };
                    if login.is_empty() || accounts.iter().any(|account: &Account| account.name == login) {
                        continue;
                    }
                    accounts.push(Account { name: login.to_owned(), token: String::new(), label: String::new() });
                }
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
    let output = process::capture("gh", &["auth", "switch", "--hostname", "github.com", "--user", login], None)?;
    if output.success { Ok(()) } else { Err(process::failure("gh auth switch", &output)) }
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
    let raw = gh(&["repo", "list", &owner, "--limit", "1000", "--json", "name,nameWithOwner,url,sshUrl,isPrivate"])?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse GitHub repository JSON: {error}"))?;
    let rows = value.as_array().ok_or_else(|| "GitHub repository response was not a JSON array".to_string())?;
    let mut repositories = Vec::with_capacity(rows.len());
    for row in rows {
        let full_name = row.get("nameWithOwner").and_then(Value::as_str).unwrap_or("").trim();
        if full_name.is_empty() { continue; }
        let name = row.get("name").and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| full_name.rsplit('/').next().unwrap_or(full_name).to_owned());
        let web_url = row.get("url").and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("https://github.com/{full_name}"));
        let clone_url = if web_url.ends_with(".git") { web_url } else { format!("{web_url}.git") };
        let ssh_url = row.get("sshUrl").and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("git@github.com:{full_name}.git"));
        repositories.push(Repo {
            name,
            full_name: full_name.to_owned(),
            clone_url,
            ssh_url,
            private: row.get("isPrivate").and_then(Value::as_bool).unwrap_or(false),
        });
    }
    Ok(repositories)
}

pub fn register_gpg(armored: &str) -> Result<(), String> {
    let output = process::capture("gh", &["api", "user/gpg_keys", "--method", "POST", "-f", &format!("armored_public_key={armored}")], None)?;
    if output.success { Ok(()) } else { Err(process::failure("gh api", &output)) }
}
