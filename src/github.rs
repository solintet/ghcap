use crate::models::{Account, Repo};
use serde_json::Value;
use std::process::{Command, Stdio};

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

pub fn available() -> bool {
    Command::new("gh").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

pub fn accounts() -> Result<Vec<Account>, String> {
    let raw = gh(&["auth", "status", "--json", "hosts"])?;
    let value: Value = serde_json::from_str(&raw).map_err(|e| format!("gh auth JSON: {e}"))?;
    let mut accounts = Vec::new();
    if let Some(hosts) = value.get("hosts").and_then(Value::as_object) {
        if let Some(users) = hosts.get("github.com").and_then(Value::as_array) {
            for user in users {
                if let Some(login) = user.get("login").and_then(Value::as_str) {
                    if !accounts.iter().any(|a: &Account| a.name == login) {
                        accounts.push(Account { name: login.to_string(), token: String::new() });
                    }
                }
            }
        }
    }
    if accounts.is_empty() {
        let login = gh(&["api", "user", "--jq", ".login"])?;
        if !login.is_empty() { accounts.push(Account { name: login, token: String::new() }); }
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

pub fn verify(account: &Account) -> Result<String, String> {
    let login = gh(&["api", "user", "--jq", ".login"])?;
    if account.name.is_empty() || account.name == login { Ok(login) } else { Ok(login) }
}

pub fn repos(account: &Account) -> Result<Vec<Repo>, String> {
    let owner = if account.name.is_empty() { gh(&["api", "user", "--jq", ".login"])? } else { account.name.clone() };
    let raw = gh(&["repo", "list", &owner, "--limit", "1000", "--json", "nameWithOwner,isPrivate,url,sshUrl"])?;
    let value: Value = serde_json::from_str(&raw).map_err(|e| format!("GitHub JSON: {e}"))?;
    let array = value.as_array().ok_or_else(|| "gh returned a non-array repository response".to_string())?;
    Ok(array.iter().filter_map(|item| Some(Repo {
        name: item.get("nameWithOwner")?.as_str()?.rsplit('/').next().unwrap_or("").to_string(),
        full_name: item.get("nameWithOwner")?.as_str()?.to_string(),
        clone_url: item.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
        ssh_url: item.get("sshUrl").and_then(Value::as_str).unwrap_or("").to_string(),
        private: item.get("isPrivate").and_then(Value::as_bool).unwrap_or(false),
    })).collect())
}

pub fn register_gpg(armored: &str) -> Result<(), String> {
    let result = Command::new("gh")
        .args(["api", "user/gpg_keys", "--method", "POST", "-f", &format!("armored_public_key={armored}")])
        .output()
        .map_err(|e| format!("gh api: {e}"))?;
    if result.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&result.stderr).trim().to_string()) }
}
