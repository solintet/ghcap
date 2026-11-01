//! EN: Execute only commands explicitly configured by the user.
//! JA: ユーザー自身が設定したコマンドのみをシェルで実行します。
use super::{failure, live};
use std::path::Path;

pub fn run_shell(command: &str, dir: &Path) -> Result<(), String> {
    let (program, args) = if cfg!(windows) {
        ("cmd", vec!["/C", command])
    } else {
        ("sh", vec!["-c", command])
    };
    let output = live(program, &args, Some(dir))?;
    if output.success {
        Ok(())
    } else {
        Err(failure(program, &output))
    }
}
