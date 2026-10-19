use crate::{github, process};

pub fn list_secret() -> Result<String, String> {
    let output = process::capture(
        "gpg",
        &["--list-secret-keys", "--keyid-format", "LONG"],
        None,
    )?;
    if output.success {
        Ok(output.stdout)
    } else {
        Err(process::failure("gpg", &output))
    }
}

pub fn secret_keys() -> Result<Vec<(String, String)>, String> {
    let output = process::capture(
        "gpg",
        &[
            "--list-secret-keys",
            "--with-colons",
            "--keyid-format",
            "LONG",
        ],
        None,
    )?;
    if !output.success {
        return Err(process::failure("gpg", &output));
    }
    let mut result = Vec::new();
    let mut current_key = None;
    let mut pending_primary = false;
    for line in output.stdout.lines() {
        let fields: Vec<_> = line.split(':').collect();
        match fields.first().copied() {
            Some("sec") => {
                let usable = !fields
                    .get(1)
                    .is_some_and(|v| matches!(*v, "r" | "e" | "d" | "i"))
                    && fields
                        .get(11)
                        .is_some_and(|v| v.to_ascii_lowercase().contains('s'))
                    && fields
                        .get(6)
                        .and_then(|v| v.parse::<u64>().ok())
                        .is_none_or(|expiry| {
                            expiry == 0
                                || expiry
                                    > std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs()
                        });
                pending_primary = usable;
                current_key = if usable {
                    fields.get(4).map(|v| (*v).to_owned())
                } else {
                    None
                };
            }
            Some("fpr") if pending_primary => {
                if let Some(fpr) = fields.get(9) {
                    if fpr.len() >= 40 && fpr.bytes().all(|b| b.is_ascii_hexdigit()) {
                        current_key = Some((*fpr).to_owned());
                        pending_primary = false;
                    }
                }
            }
            Some("uid") => {
                if let (Some(key), Some(uid)) = (current_key.as_ref(), fields.get(9)) {
                    result.push((key.clone(), (*uid).to_string()));
                }
            }
            _ => {}
        }
    }
    result.dedup_by(|a, b| a.0 == b.0);
    Ok(result)
}

pub fn generate(uid: &str) -> Result<(), String> {
    if uid.trim().is_empty()
        || uid.starts_with('-')
        || uid.chars().any(char::is_control)
        || uid.contains("<email>")
    {
        return Err("Provide your own valid GPG identity; option-like or placeholder values are not accepted.".into());
    }
    let output = process::capture(
        "gpg",
        &[
            "--batch",
            "--quick-generate-key",
            uid,
            "ed25519",
            "sign",
            "1y",
        ],
        None,
    )?;
    if output.success {
        Ok(())
    } else {
        Err(process::failure("gpg", &output))
    }
}

pub fn export_armored(keyid: &str) -> Result<String, String> {
    let output = process::capture("gpg", &["--armor", "--export", keyid], None)?;
    if output.success {
        Ok(output.stdout)
    } else {
        Err(process::failure("gpg", &output))
    }
}

pub fn register_key(keyid: &str) -> Result<(), String> {
    github::register_gpg(&export_armored(keyid)?)
}
