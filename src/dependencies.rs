use crate::i18n;
use std::{env, fs};

const REQUIRED: [(&str, &str); 3] = [("Git", "git"), ("GitHub CLI", "gh"), ("GnuPG", "gpg")];

/// Check required external tools before the TUI switches the terminal to raw mode.
/// A successful spawn is not enough: `--version` must also exit successfully.
pub fn check_required(locale: &str) -> Result<(), String> {
    let missing: Vec<(&str, &str)> = REQUIRED
        .iter()
        .copied()
        .filter(|(_, command)| !command_available(command))
        .collect();

    if missing.is_empty() {
        let help = crate::process::capture_timeout(
            "gh",
            &["auth", "status", "--help"],
            None,
            std::time::Duration::from_secs(5),
        )?;
        if !help.success || !help.stdout.contains("--json") {
            return Err("GitHub CLI lacks 'gh auth status --json'. Install a compatible, security-maintained gh release (2.98.0+ or equivalent vendor fixes). Saved accounts were preserved.".into());
        }
        return Ok(());
    }

    let mut message = String::new();
    message.push_str(&i18n::text(locale, "dependencies.title"));
    message.push_str("\n\n");
    message.push_str(&i18n::text(locale, "dependencies.missing"));
    for (display, command) in missing {
        message.push_str("\n  - ");
        message.push_str(display);
        message.push_str(" (");
        message.push_str(command);
        message.push(')');
    }
    message.push_str("\n\n");
    message.push_str(&i18n::text(locale, "dependencies.install"));
    message.push_str("\n\n");
    message.push_str(&installation_guidance(locale));
    Err(message)
}

fn command_available(command: &str) -> bool {
    crate::process::capture_timeout(
        command,
        &["--version"],
        None,
        std::time::Duration::from_secs(5),
    )
    .is_ok_and(|o| o.success)
}

fn installation_guidance(locale: &str) -> String {
    match env::consts::OS {
        "windows" => format!(
            "{}\n  winget install --id Git.Git -e\n  winget install --id GitHub.cli -e\n  winget install --id GnuPG.Gpg4win -e\n\n{}",
            i18n::text(locale, "dependencies.windows"),
            i18n::text(locale, "dependencies.docs"),
        ),
        "macos" => format!(
            "{}\n  brew install git gh gnupg\n\n{}",
            i18n::text(locale, "dependencies.macos"),
            i18n::text(locale, "dependencies.docs"),
        ),
        "linux" => linux_guidance(locale),
        _ => format!(
            "{}\n{}",
            i18n::text(locale, "dependencies.linux.generic"),
            i18n::text(locale, "dependencies.docs"),
        ),
    }
}

fn linux_guidance(locale: &str) -> String {
    let (id, id_like) = linux_distribution();
    let ids = format!("{id} {id_like}").to_lowercase();
    let (key, command) =
        if contains_any(&ids, &["debian", "ubuntu", "linuxmint", "pop", "raspbian"]) {
            (
                "dependencies.linux.debian",
                "sudo apt update && sudo apt install git gh gnupg",
            )
        } else if contains_any(
            &ids,
            &["fedora", "rhel", "centos", "rocky", "almalinux", "ol"],
        ) {
            (
                "dependencies.linux.fedora",
                "sudo dnf install git gh gnupg2",
            )
        } else if contains_any(&ids, &["arch", "manjaro", "endeavouros"]) {
            (
                "dependencies.linux.arch",
                "sudo pacman -S git github-cli gnupg",
            )
        } else if contains_any(&ids, &["opensuse", "suse", "sles"]) {
            (
                "dependencies.linux.opensuse",
                "sudo zypper install git gh gpg2",
            )
        } else {
            return format!(
                "{}\n\n{}",
                i18n::text(locale, "dependencies.linux.generic"),
                i18n::text(locale, "dependencies.docs"),
            );
        };

    format!(
        "{}\n  {}\n\n{}",
        i18n::text(locale, key),
        command,
        i18n::text(locale, "dependencies.docs"),
    )
}

fn linux_distribution() -> (String, String) {
    let Ok(contents) = fs::read_to_string("/etc/os-release") else {
        return (String::new(), String::new());
    };
    let mut id = String::new();
    let mut id_like = String::new();
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = unquote(value);
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            id_like = unquote(value);
        }
    }
    (id, id_like)
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim_matches('\'').to_owned()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    haystack
        .split_whitespace()
        .any(|word| needles.contains(&word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_distribution_families() {
        assert!(contains_any("debian ubuntu", &["debian", "ubuntu"]));
        assert!(contains_any("rhel fedora", &["fedora", "rhel"]));
        assert!(contains_any("arch manjaro", &["arch", "manjaro"]));
        assert!(!contains_any("alpine", &["debian", "fedora", "arch"]));
    }

    #[test]
    fn unquotes_os_release_values() {
        assert_eq!(unquote("\"ubuntu\""), "ubuntu");
        assert_eq!(unquote("'debian'"), "debian");
        assert_eq!(unquote("fedora"), "fedora");
    }
}
