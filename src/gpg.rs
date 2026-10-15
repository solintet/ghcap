use crate::{github, process};

pub fn list_secret() -> Result<String, String> {
    let output = process::capture("gpg", &["--list-secret-keys", "--keyid-format", "LONG"], None)?;
    if output.success { Ok(output.stdout) } else { Err(process::failure("gpg", &output)) }
}

pub fn secret_keys() -> Result<Vec<(String, String)>, String> {
    let output = process::capture("gpg", &["--list-secret-keys", "--with-colons", "--keyid-format", "LONG"], None)?;
    if !output.success { return Err(process::failure("gpg", &output)); }
    let mut result = Vec::new();
    let mut current_key = None;
    for line in output.stdout.lines() {
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
    let output = process::capture("gpg", &["--batch", "--quick-generate-key", uid, "ed25519", "sign", "1y"], None)?;
    if output.success { Ok(()) } else { Err(process::failure("gpg", &output)) }
}

pub fn export_armored(keyid: &str) -> Result<String, String> {
    let output = process::capture("gpg", &["--armor", "--export", keyid], None)?;
    if output.success { Ok(output.stdout) } else { Err(process::failure("gpg", &output)) }
}

pub fn register_key(keyid: &str) -> Result<(), String> { github::register_gpg(&export_armored(keyid)?) }

pub fn register() -> Result<(), String> {
    let key = secret_keys()?.into_iter().next().ok_or_else(|| "No secret GPG key found".to_string())?;
    register_key(&key.0)
}
