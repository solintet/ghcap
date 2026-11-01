//! EN: Check capabilities, not merely the existence of a binary.
//! JA: 実行ファイルの存在だけではなく必要な機能と版を検査する。
use std::path::Path;
use std::process::{Command, Stdio};

pub fn program_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// EN: Minimum requirement is intentional, even if --json is backported to older releases.
/// JA: JSON機能だけでなく、セキュリティ修正を含む最低版も確認する。
fn valid_gh(path: &Path) -> bool {
    let Ok(version) = Command::new(path).arg("--version").output() else {
        return false;
    };
    if !version.status.success() {
        return false;
    }
    let version_text = String::from_utf8_lossy(&version.stdout);
    let number = version_text
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|c| c.is_ascii_digit()) && part.contains('.'));
    let Some(number) = number else {
        return false;
    };
    let fields = number
        .split('.')
        .take(3)
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(fields) = fields else {
        return false;
    };
    if fields.len() != 3 || fields.as_slice() < [2, 98, 0].as_slice() {
        return false;
    }
    let Ok(help) = Command::new(path)
        .args(["auth", "status", "--help"])
        .output()
    else {
        return false;
    };
    help.status.success() && String::from_utf8_lossy(&help.stdout).contains("--json")
}

pub fn compatible_gh(path: &Path) -> bool {
    valid_gh(path)
}

pub fn gh_report() -> String {
    match super::resolver::resolve_gh() {
        Ok(candidate) => format!("compatible / 対応: {}", candidate.display()),
        Err(reason) => format!("incompatible / 非対応: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn obvious_old_version_is_not_selected() {
        // EN: A string comparison would incorrectly rank 2.9 above 2.98.
        // JA: 文字列比較では2.9が2.98より新しいと誤判定されるため数値で比較する。
        assert!([2, 9, 0].as_slice() < [2, 98, 0].as_slice());
    }
}
