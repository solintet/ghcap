//! EN: Refuse implicit overwrites and unsafe first-push targets.
//! JA: 最初のPushでリモート履歴を黙って上書きしないための検査。
use crate::git;
use std::path::Path;

pub fn check_first_push(dir: &Path, remote: &str, branch: &str) -> Result<(), String> {
    // EN: Check remote reachability; a network error must NOT be treated as an empty repository.
    // JA: 接続失敗を空のリモートと誤認しない。
    let refname = format!("refs/heads/{branch}");
    let output = git::git_in(Some(dir), &["ls-remote", "--heads", remote, &refname])?;
    if !output.trim().is_empty() {
        return Err(format!("Remote branch already exists / 送信先ブランチが既に存在します: {remote}/{branch}. Inspect and integrate history before pushing. / 履歴を確認してください。"));
    }
    Ok(())
}
