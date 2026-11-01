//! EN: Safe, restartable initial setup for local and GitHub repositories.
//! JA: 初回設定を途中から再開可能にし、既存履歴を保護する。
use crate::{
    git, process,
    repository::{inspect, safety, state::HeadState},
    ui,
};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

/// EN: Use the ordinary terminal for prompts and command output.
/// JA: 入力と外部コマンド結果をCUIへ表示する。
pub fn run_cui() -> Result<(), String> {
    println!("\nghcap — Repository Setup / リポジトリの初回設定");
    let cwd = std::env::current_dir().map_err(|e| format!("Current directory: {e}"))?;
    let chosen = prompt(
        "Local directory / ローカルフォルダ",
        &cwd.display().to_string(),
    )?;
    let folder = PathBuf::from(chosen);
    if !folder.is_dir() {
        return Err(format!(
            "Directory does not exist / フォルダが存在しません: {}",
            folder.display()
        ));
    }
    let path = folder
        .canonicalize()
        .map_err(|e| format!("Directory: {e}"))?;
    println!("\nTarget / 対象: {}", path.display());
    if !confirm("Use this directory? / このフォルダで続行しますか？")? {
        return Ok(());
    }
    let inside = git::git_in(Some(&path), &["rev-parse", "--show-toplevel"]);
    match inside {
        Ok(top) => {
            // EN: Never initialize a nested repository or silently use a parent repository.
            // JA: 親リポジトリを誤操作したり、勝手に入れ子のリポジトリを作成しない。
            let root = PathBuf::from(top);
            if root.canonicalize().ok().as_ref() != Some(&path) {
                return Err(format!("A different parent Git repository owns this folder / 親のGitリポジトリに属しています: {}", root.display()));
            }
            println!(
                "Existing local Git repository detected / 既存のGitリポジトリを検出しました。"
            );
        }
        Err(_) => {
            // EN: An existing gitfile/worktree is not safe to reinitialize blindly.
            // JA: .gitファイル等を持つ既存ワークツリーを勝手に初期化しない。
            if path.join(".git").exists() {
                return Err("Existing .git metadata could not be read / 既存の.gitを読み取れません。自動初期化しません。".into());
            }
            let branch = prompt("Initial branch / 初期ブランチ", "master")?;
            validate_branch(&path, &branch)?;
            if !confirm("Initialize local Git repository? / Gitを初期化しますか？")? {
                return Ok(());
            }
            checked("git", &["init", "-b", &branch], &path)?;
        }
    }
    ensure_identity(&path)?;
    let snapshot = inspect::inspect(&path)?;
    if let HeadState::Unborn { .. } = snapshot.head {
        println!("\nFirst commit required / 初回コミットが必要です。");
        checked("git", &["status", "--short"], &path)?;
        println!("\nNOTICE / 注意: Review every file before publication. / 公開対象の全ファイルを確認してください。");
        println!("Files already committed may remain in history. / 一度コミットされた内容は履歴に残る可能性があります。");
        if super::staging::review_and_stage(&path)?
            && confirm(
                "Commit ALL currently staged changes? / 現在Stage済みの全変更をコミットしますか？",
            )?
        {
            let message = prompt("Commit message / コミットメッセージ", "Initial commit")?;
            if message.trim().is_empty() {
                return Err(
                    "Commit message must not be empty / メッセージを入力してください".into(),
                );
            }
            checked("git", &["commit", "-m", &message], &path)?;
        }
    }
    let snapshot = inspect::inspect(&path)?;
    println!("\nGitHub / GitHubの設定");
    println!("  1) Create a new private repository / 新規Privateリポジトリ作成");
    println!("  2) Connect an existing GitHub repository / 既存リポジトリへ接続");
    println!("  3) Local Git only / ローカルGitのみ");
    let choice = prompt("Choose 1/2/3 / 選択", "3")?;
    match choice.as_str() {
        "1" => {
            let existing = git::git_in(Some(&path), &["remote", "get-url", "origin"]);
            if let Ok(url) = existing {
                return Err(format!("origin already exists / originは既に存在します: {url}. Use connection workflow or inspect it first. / 既存の設定を確認してください。"));
            }
            if crate::dependencies::resolver::resolve_gh().is_err() {
                return Err(format!(
                    "Compatible gh is required / 対応版ghが必要です。\n{}",
                    crate::dependencies::installer::installation_hint()
                ));
            }
            let default_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("repository");
            let name = prompt("GitHub repository name / リポジトリ名", default_name)?;
            if !valid_repo_name(&name) {
                return Err("Invalid repository name / リポジトリ名が不正です".into());
            }
            let privacy = prompt(
                "Visibility: private/public / 公開設定 (private/public)",
                "private",
            )?;
            if privacy != "private" && privacy != "public" {
                return Err("Choose private or public / privateかpublicを選択してください".into());
            }
            if privacy == "public"
                && !confirm("Publish to the internet? / インターネットに公開しますか？")?
            {
                return Ok(());
            }
            if !confirm("Create this GitHub repository? / GitHub上に作成しますか？")? {
                return Ok(());
            }
            let flag = if privacy == "public" {
                "--public"
            } else {
                "--private"
            };
            checked(
                "gh",
                &[
                    "repo", "create", &name, flag, "--source", ".", "--remote", "origin",
                ],
                &path,
            )?;
            println!("Remote created / リモートを作成しました。");
        }
        "2" => {
            let url = prompt("GitHub remote URL (HTTPS/SSH) / GitHubのURL", "")?;
            if !safe_github_url(&url) {
                return Err("Only github.com HTTPS/SSH URLs are supported in this setup / この設定画面ではgithub.comのHTTPS/SSH URLのみ対応".into());
            }
            if let Ok(existing) = git::git_in(Some(&path), &["remote", "get-url", "origin"]) {
                if existing != url {
                    return Err(format!("Existing origin is different / 既存のoriginが異なります: {existing}. Nothing changed. / 変更していません。"));
                }
            } else if confirm("Add this remote as origin? / originとして登録しますか？")?
            {
                checked("git", &["remote", "add", "origin", &url], &path)?;
            } else {
                return Ok(());
            }
        }
        "3" => {
            println!("Local setup completed / ローカル設定が完了しました。");
            return Ok(());
        }
        _ => return Err("Invalid choice / 選択が不正です".into()),
    }
    if let HeadState::Unborn { .. } = snapshot.head {
        println!("First commit is required before pushing / Pushには初回コミットが必要です。");
        return Ok(());
    }
    let current = inspect::inspect(&path)?;
    let branch = match current.head {
        HeadState::Attached { branch, .. } => branch,
        HeadState::Unborn { .. } => return Ok(()),
        HeadState::Detached { .. } => {
            return Err("Detached HEAD: select a branch / ブランチを選択してください".into())
        }
    };
    match current.tracking {
        crate::repository::state::TrackingState::Configured { branch: upstream } => {
            println!("Upstream already configured / 追跡先設定済み: {upstream}");
        }
        crate::repository::state::TrackingState::Broken { reason } => {
            return Err(format!(
                "Upstream configuration is invalid / upstreamの設定に異常があります: {reason}"
            ));
        }
        crate::repository::state::TrackingState::NotConfigured => {
            safety::check_first_push(&path, "origin", &branch)?;
            if confirm(&format!(
                "Push origin/{branch} and set upstream? / 初回Pushしますか？"
            ))? {
                checked("git", &["push", "--set-upstream", "origin", &branch], &path)?;
            }
        }
    }
    println!("\nSetup finished / 初回設定を終了しました。");
    Ok(())
}

/// EN: Restore TUI even if setup returns an error.
/// JA: 初回設定が失敗してもTUI表示を復元する。
pub fn from_tui() -> io::Result<()> {
    ui::leave_tui()?;
    let result = run_cui();
    match &result {
        Ok(()) => println!("\nCompleted / 処理終了"),
        Err(error) => eprintln!("\n[GHCAP DIAGNOSIS / ghcap 診断]\n{error}"),
    }
    println!("\nPress Enter / Enterで戻ります");
    let mut line = String::new();
    let input_result = io::stdin().read_line(&mut line);
    let terminal_result = ui::enter_tui();
    input_result?;
    terminal_result?;
    Ok(())
}

/// EN: Configure identity locally only; existing global settings remain untouched.
/// JA: Gitのユーザー情報は対象リポジトリだけに設定し、グローバル設定は変更しない。
fn ensure_identity(path: &Path) -> Result<(), String> {
    let name = git::user_name(path);
    let email = git::user_email(path);
    if name.is_some() && email.is_some() {
        println!("Git identity configured / Gitのユーザー情報は設定済みです。");
        return Ok(());
    }
    let name = match name {
        Some(name) => name,
        None => prompt("Git user.name / 表示名", "")?,
    };
    let email = match email {
        Some(email) => email,
        None => prompt("Git user.email / メールアドレス", "")?,
    };
    if name.trim().is_empty() || email.trim().is_empty() || email.contains(char::is_whitespace) {
        return Err("Name and email are required / Gitの名前とメールアドレスが必要です".into());
    }
    if !confirm("Save identity in this repository only? / このリポジトリだけに保存しますか？")?
    {
        return Err(
            "Git identity was not configured / Gitユーザー情報は設定されませんでした".into(),
        );
    }
    checked("git", &["config", "--local", "user.name", &name], path)?;
    checked("git", &["config", "--local", "user.email", &email], path)?;
    Ok(())
}

/// EN: Requests explicit input instead of silently accepting destructive defaults.
/// JA: リスクのある既定操作を無断実行せず、入力と確認を必須にする。
fn prompt(label: &str, default: &str) -> Result<String, String> {
    print!(
        "{label}{}: ",
        if default.is_empty() {
            String::new()
        } else {
            format!(" [{default}]")
        }
    );
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let cleaned = line.trim();
    Ok(if cleaned.is_empty() {
        default.to_string()
    } else {
        cleaned.to_string()
    })
}
fn confirm(question: &str) -> Result<bool, String> {
    print!("{question} [y/N] ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
fn checked(program: &str, args: &[&str], path: &Path) -> Result<(), String> {
    process::stream(program, args, Some(path)).map(|_| ())
}
fn validate_branch(path: &Path, name: &str) -> Result<(), String> {
    let output = process::capture("git", &["check-ref-format", "--branch", name], Some(path))?;
    if output.success {
        Ok(())
    } else {
        Err(process::failure("git check-ref-format", &output))
    }
}
fn valid_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}
fn safe_github_url(url: &str) -> bool {
    let path = if let Some(rest) = url.strip_prefix("https://github.com/") {
        rest
    } else if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else {
        return false;
    };
    !path.contains(char::is_whitespace)
        && !path.contains('@')
        && !path.contains("..")
        && path.split('/').filter(|v| !v.is_empty()).count() == 2
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_names_are_conservative() {
        assert!(valid_repo_name("ghcap"));
        assert!(!valid_repo_name("repo name"));
    }
}
