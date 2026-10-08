use crate::github;
use std::process::Command;

pub fn list_secret() -> Result<String, String> {
    let output = Command::new("gpg").args(["--list-secret-keys", "--keyid-format", "LONG"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).into()) } else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn secret_keys() -> Result<Vec<(String, String)>, String> {
    let output = Command::new("gpg").args(["--list-secret-keys", "--with-colons", "--keyid-format", "LONG"]).output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))); }
    let mut result = Vec::new();
    let mut current_key = None;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields: Vec<_> = line.split(':').collect();
        match fields.first().copied() {
            Some("sec") => current_key = fields.get(4).map(|v| (*v).to_string()),
            Some("uid") => if let (Some(key), Some(uid)) = (current_key.as_ref(), fields.get(9)) { result.push((key.clone(), (*uid).to_string())); },
            _ => {}
        }
    }
    result.dedup_by(|a,b| a.0 == b.0);
    Ok(result)
}

pub fn generate(uid: &str) -> Result<(), String> {
    let output = Command::new("gpg").args(["--batch", "--quick-generate-key", uid, "ed25519", "sign", "1y"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn export_armored(keyid: &str) -> Result<String, String> {
    let output = Command::new("gpg").args(["--armor", "--export", keyid]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).into()) } else { Err(format!("{}\nexit code {}", String::from_utf8_lossy(&output.stderr).trim(), output.status.code().unwrap_or(1))) }
}

pub fn register_key(keyid: &str) -> Result<(), String> { github::register_gpg(&export_armored(keyid)?) }

pub fn register() -> Result<(), String> {
    let key = secret_keys()?.into_iter().next().ok_or_else(|| "No secret GPG key found".to_string())?;
    register_key(&key.0)
}
