//! EN: Retain original process termination while streaming CLI diagnostics.
//! JA: CLIの実行経過を表示するときも本来の終了理由を保持します。
use super::{failure, live, safe_text};
use std::path::Path;

/// EN: Stream an operation through the hardened v29 process engine, keeping its
/// timeout, cancellation, terminal escaping and diagnostic handling.
/// JA: v29のタイムアウト・中断・制御文字対策を維持したまま、
/// 外部コマンドの実行経過と診断をCUIで表示します。
pub fn stream(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    println!(
        "\n[COMMAND / 実行コマンド] $ {} {}",
        safe_text(program),
        safe_text(&args.join(" "))
    );
    let result = live(program, args, cwd)?;
    println!("[EXIT STATUS / 終了状態] {:?}", result.termination);
    if result.success {
        Ok(result.stdout)
    } else {
        Err(failure(program, &result))
    }
}
