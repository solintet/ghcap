//! EN: Explicit states prevent confusing unborn HEAD with a broken repository.
//! JA: 初回コミット前とGit管理情報の異常を別の状態として扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadState {
    Unborn { branch: String },
    Attached { branch: String, commit: String },
    Detached { commit: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackingState {
    NotConfigured,
    Configured { branch: String },
    Broken { reason: String },
}

#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    pub head: HeadState,
    pub tracking: TrackingState,
    pub remotes: Vec<String>,
}
