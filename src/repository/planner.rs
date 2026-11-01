//! EN: Pure decisions are separated from commands that modify the repository.
//! JA: 状態による意思決定と、実際に状態を変更する処理を分ける。
use super::state::{HeadState, RepositorySnapshot, TrackingState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushPlan {
    NeedsCommit,
    NeedsRemote,
    FirstPush { remote: String, branch: String },
    NormalPush,
    DetachedHead,
    BrokenTracking { reason: String },
}

pub fn plan_push(snapshot: &RepositorySnapshot, selected_remote: Option<&str>) -> PushPlan {
    match &snapshot.head {
        HeadState::Unborn { .. } => PushPlan::NeedsCommit,
        HeadState::Detached { .. } => PushPlan::DetachedHead,
        HeadState::Attached { branch, .. } => {
            if snapshot.remotes.is_empty() {
                return PushPlan::NeedsRemote;
            }
            match &snapshot.tracking {
                TrackingState::Configured { .. } => PushPlan::NormalPush,
                TrackingState::Broken { reason } => PushPlan::BrokenTracking {
                    reason: reason.clone(),
                },
                TrackingState::NotConfigured => {
                    let remote = selected_remote
                        .filter(|r| snapshot.remotes.iter().any(|v| v == r))
                        .map(str::to_owned)
                        .or_else(|| {
                            snapshot
                                .remotes
                                .iter()
                                .find(|v| v.as_str() == "origin")
                                .cloned()
                        })
                        .or_else(|| {
                            (snapshot.remotes.len() == 1).then(|| snapshot.remotes[0].clone())
                        });
                    if let Some(remote) = remote {
                        PushPlan::FirstPush {
                            remote,
                            branch: branch.clone(),
                        }
                    } else {
                        PushPlan::NeedsRemote
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unborn_head_requires_commit() {
        let snapshot = RepositorySnapshot {
            head: HeadState::Unborn {
                branch: "master".into(),
            },
            tracking: TrackingState::NotConfigured,
            remotes: vec!["origin".into()],
        };
        assert_eq!(plan_push(&snapshot, None), PushPlan::NeedsCommit);
    }
    #[test]
    fn initial_push_does_not_assume_main() {
        let snapshot = RepositorySnapshot {
            head: HeadState::Attached {
                branch: "feature/test".into(),
                commit: "abcdef".into(),
            },
            tracking: TrackingState::NotConfigured,
            remotes: vec!["origin".into()],
        };
        assert_eq!(
            plan_push(&snapshot, None),
            PushPlan::FirstPush {
                remote: "origin".into(),
                branch: "feature/test".into()
            }
        );
    }
}
