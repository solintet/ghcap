use crate::{github, models::Account};
use std::process::Command;

pub fn list_secret() -> Result<String, String> {
    let output = Command::new("gpg").args(["--list-secret-keys", "--keyid-format", "LONG"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).into()) }
    else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn generate(uid: &str) -> Result<(), String> {
    let output = Command::new("gpg").args(["--batch", "--quick-generate-key", uid, "ed25519", "sign", "1y"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn first_secret_armored() -> Result<String, String> {
    let list = Command::new("gpg").args(["--list-secret-keys", "--with-colons"]).output().map_err(|e| e.to_string())?;
    if !list.status.success() { return Err(format!("{}\nexit code {}", String::from_utf8_lossy(&list.stderr).trim(), list.status.code().unwrap_or(1))); }
    let text = String::from_utf8_lossy(&list.stdout);
    let keyid = text.lines().find_map(|line| {
        let fields: Vec<_> = line.split(':').collect();
        (fields.first() == Some(&"sec")).then(|| fields.get(4).copied()).flatten()
    }).ok_or_else(|| "No secret GPG key found".to_string())?;
    let output = Command::new("gpg").args(["--armor", "--export", keyid]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).into()) } else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn register(account: &Account) -> Result<(), String> {
    github::register_gpg(&first_secret_armored()?)
}
