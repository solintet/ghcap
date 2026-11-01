//! EN: Explicit first-commit staging with no implicit `git add -A`.
//! JA: 初回コミット時のStage対象を明示し、無条件の`git add -A`を行わない設計です。
use crate::{git, process};
use std::{
    collections::BTreeSet,
    io::{self, Write},
    path::Path,
};

/// EN: Lists Git's candidate paths, lets users select which ones to stage,
/// revalidates the worktree after approval, and reviews the entire index.
/// Previously staged paths are not reset; they are shown before the final commit.
/// JA: Gitが検出した変更ファイルを一覧化し、利用者がStage対象を選択します。
/// 承認後に変更の有無を再検査し、既存のStage済みファイルも含めて最終内容を確認します。
/// キャンセルしても既存のIndexは削除しません。
pub fn review_and_stage(path: &Path) -> Result<bool, String> {
    git::preflight_commit(path)?;
    let before = git::commit_fingerprint(path)?;
    let output = process::capture(
        "git",
        &[
            "ls-files",
            "--others",
            "--modified",
            "--deleted",
            "--exclude-standard",
            "-z",
        ],
        Some(path),
    )?;
    if !output.success {
        return Err(process::failure("git ls-files", &output));
    }
    let mut paths = output
        .stdout
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    if !paths.is_empty() {
        println!("\nFiles available for staging / Stage対象候補:");
        for (i, file) in paths.iter().enumerate() {
            println!("  {:>3}) {}", i + 1, process::safe_text(file));
        }
        println!("\nEnter file numbers separated by spaces, 'all', or 'none'. / 番号を空白区切りで指定、all=全件、none=追加なし");
        print!("Files / 対象: ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut selected_text = String::new();
        io::stdin()
            .read_line(&mut selected_text)
            .map_err(|e| e.to_string())?;
        let chosen = selected_text.trim();
        let mut indices = BTreeSet::new();
        match chosen {
            "all" => {
                indices.extend(0..paths.len());
            }
            "none" | "" => {}
            _ => {
                for token in chosen.split_whitespace() {
                    let number = token
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid number / 不正な番号: {token}"))?;
                    if number == 0 || number > paths.len() {
                        return Err(format!("Out of range / 範囲外の番号: {number}"));
                    }
                    indices.insert(number - 1);
                }
            }
        }
        if !indices.is_empty() {
            println!("\nSelected files / 選択したファイル:");
            for &index in &indices {
                println!("  {}", process::safe_text(&paths[index]));
            }
            if !confirm("Stage only the selected paths? / 選択したファイルだけStageしますか？")?
            {
                return Ok(false);
            }
            // EN: Fail closed if any file changed while the user was selecting paths.
            // JA: ファイル選択中に内容が変わっていた場合は安全のため操作を停止します。
            if git::commit_fingerprint(path)? != before {
                return Err("Files changed during review; stage was not modified. / 確認中に変更が発生しました。Stageは実行していません。".into());
            }
            for index in indices {
                // EN: `--` separates pathspecs from Git options; no shell is involved.
                // JA: `--`でファイル名とGitオプションを分離し、シェルを使用しません。
                process::stream("git", &["add", "--", &paths[index]], Some(path))?;
            }
        }
    } else {
        println!("No unstaged files / 新しくStageできる変更ファイルはありません。");
    }
    let staged = process::capture("git", &["diff", "--cached", "--quiet"], Some(path))?;
    match staged.termination {
        process::Termination::Exited(0) => Ok(false),
        process::Termination::Exited(1) => {
            println!("\nComplete staged index / Stage済みのすべての変更:");
            process::stream("git", &["diff", "--cached", "--name-status"], Some(path))?;
            process::stream("git", &["diff", "--cached", "--stat"], Some(path))?;
            // EN: Whitespace errors are reported without treating them as "no changes".
            // JA: 空白異常は「差分なし」と解釈せず、元のGitエラーを表示します。
            let check = process::capture("git", &["diff", "--cached", "--check"], Some(path))?;
            if !check.success {
                return Err(process::failure("git diff --cached --check", &check));
            }
            Ok(true)
        }
        _ => Err(process::failure("git diff --cached --quiet", &staged)),
    }
}

/// EN: Confirm an index-changing action with an explicit default of No.
/// JA: GitのIndexを変更する操作はNoを既定値として確認します。
fn confirm(question: &str) -> Result<bool, String> {
    print!("{question} [y/N] ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
