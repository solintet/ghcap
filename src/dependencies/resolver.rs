//! EN: Select a compatible GitHub CLI without modifying PATH, system packages, or credentials.
//! JA: PATH・OSのパッケージ・認証情報を変更せず、互換版のGitHub CLIを選択します。
use std::{
    env,
    path::{Path, PathBuf},
};

/// EN: A rejected override must not silently fall back to another executable.
/// Resolving the file to an absolute path also prevents later PATH substitutions.
/// JA: 明示指定された不適合な実行ファイルを、勝手に別のghへ置き換えません。
/// 絶対パスを使用してPATHの後続変更による実行先のすり替わりも防ぎます。
pub fn resolve_gh() -> Result<PathBuf, String> {
    if let Some(raw) = env::var_os("GHCAP_GH_PATH") {
        let selected = PathBuf::from(raw);
        return check(&selected).ok_or_else(|| {
            format!(
            "GHCAP_GH_PATH is not a compatible GitHub CLI / 指定されたghは互換性がありません: {}",
            selected.display()
        )
        });
    }
    let name = if cfg!(windows) { "gh.exe" } else { "gh" };
    if let Some(search) = env::var_os("PATH") {
        for directory in env::split_paths(&search) {
            // EN: Avoid loading an executable from an attacker-controlled relative PATH entry.
            // JA: 相対PATHから不正な実行ファイルを拾わないようにします。
            if !directory.is_absolute() {
                continue;
            }
            let candidate = directory.join(name);
            if let Some(valid) = check(&candidate) {
                return Ok(valid);
            }
        }
    }
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let managed = PathBuf::from(home).join(".ghcap").join("tools").join(name);
        if let Some(valid) = check(&managed) {
            return Ok(valid);
        }
    }
    Err("No compatible GitHub CLI found (requires version 2.98.0+ and auth status --json). / 対応版のghが見つかりません。ghcap doctorで確認してください。".into())
}

/// EN: The feature/version check happens before any GitHub operation.
/// JA: GitHub操作の開始前に版と機能を検査します。
fn check(path: &Path) -> Option<PathBuf> {
    if !super::capabilities::compatible_gh(path) {
        return None;
    }
    path.canonicalize()
        .ok()
        .filter(|resolved| resolved.is_file())
}
