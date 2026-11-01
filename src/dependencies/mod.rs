//! EN: Dependency checks are feature-specific rather than an unconditional startup gate.
//! JA: 依存関係の検査は機能単位で行い、起動全体を無条件に拒否しない。
pub mod capabilities;
pub mod installer;
pub mod resolver;

/// EN: Git is needed for repository commands; gh and GPG are optional by operation.
/// JA: Git操作時のみGitを必須とし、ghとGPGは利用機能に応じて検査する。
pub fn check_required(_locale: &str) -> Result<(), String> {
    if capabilities::program_available("git") {
        Ok(())
    } else {
        Err("Git was not found. / Git が見つかりません。 Install Git and retry. / Git を導入して再実行してください。".into())
    }
}

/// EN: Display a diagnostic report without changing system software.
/// JA: システム設定を変更せず診断結果を表示する。
pub fn doctor() {
    println!("ghcap v0.0.31.0 — Dependency Doctor / 依存関係の診断");
    println!(
        "Git: {}",
        if capabilities::program_available("git") {
            "available / 利用可能"
        } else {
            "missing / 未導入"
        }
    );
    println!("GitHub CLI: {}", capabilities::gh_report());
    println!(
        "GnuPG: {}",
        if capabilities::program_available("gpg") {
            "available / 利用可能 (GPG signing)"
        } else {
            "optional / GPG署名時のみ必要"
        }
    );
    println!("Guidance / 導入案内: https://cli.github.com/ ; https://git-scm.com/");
}
