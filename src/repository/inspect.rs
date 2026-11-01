//! EN: All expected Git absence states are handled explicitly; unexpected errors remain errors.
//! JA: Gitの未設定状態と実際のコマンド失敗を区別して扱う。
use super::state::{HeadState, RepositorySnapshot, TrackingState};
use crate::git;
use std::path::Path;

pub fn inspect(dir: &Path) -> Result<RepositorySnapshot, String> {
    let inside = git::git_in(Some(dir), &["rev-parse", "--is-inside-work-tree"])?;
    if inside != "true" {
        return Err("Not a working-tree repository / 作業ツリーではありません".into());
    }
    let branch = git::git_in(Some(dir), &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let commit = git::git_in(Some(dir), &["rev-parse", "--verify", "HEAD^{commit}"]);
    let head = match (branch, commit) {
        (Ok(branch), Ok(commit)) => HeadState::Attached { branch, commit },
        (Ok(branch), Err(_)) => {
            // EN: Ensure this is really unborn, not corruption of an existing branch.
            // JA: 既存ブランチの破損を初回コミット前と取り違えないため、参照を調べる。
            let refs = git::git_in(
                Some(dir),
                &[
                    "show-ref",
                    "--verify",
                    "--",
                    &format!("refs/heads/{branch}"),
                ],
            );
            if refs.is_ok() {
                return Err(
                    "Branch exists but HEAD could not be resolved / ブランチ参照に異常があります"
                        .into(),
                );
            }
            HeadState::Unborn { branch }
        }
        (Err(_), Ok(commit)) => HeadState::Detached { commit },
        (Err(error), Err(_)) => {
            return Err(format!(
                "Cannot inspect HEAD / HEADを確認できません: {error}"
            ))
        }
    };
    let remote_text = git::git_in(Some(dir), &["remote"])?;
    let remotes = remote_text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let tracking = match &head {
        HeadState::Attached { branch, .. } => {
            // EN: Determine whether tracking is configured before attempting ref resolution.
            // JA: 追跡設定の有無を調べてからGit参照を解決し、設定異常を握りつぶさない。
            let remote_key = format!("branch.{branch}.remote");
            let merge_key = format!("branch.{branch}.merge");
            // EN: Only exit code 1 means an absent Git configuration value.
            // Other errors must stop inspection, rather than masquerade as no upstream.
            // JA: Git設定の不存在だけをNoneとして扱い、その他のGitエラーは診断へ伝播させます。
            let remote = git::config(dir, &remote_key)?;
            let merge = git::config(dir, &merge_key)?;
            match (remote, merge) {
                (None, None) => TrackingState::NotConfigured,
                (Some(_), Some(_)) => match git::git_in(
                    Some(dir),
                    &[
                        "rev-parse",
                        "--abbrev-ref",
                        "--symbolic-full-name",
                        "@{upstream}",
                    ],
                ) {
                    Ok(name) => TrackingState::Configured { branch: name },
                    Err(reason) => TrackingState::Broken { reason },
                },
                _ => TrackingState::Broken {
                    reason: format!(
                        "Incomplete upstream configuration / upstream設定が不完全です ({branch})"
                    ),
                },
            }
        }
        _ => TrackingState::NotConfigured,
    };
    Ok(RepositorySnapshot {
        head,
        tracking,
        remotes,
    })
}
