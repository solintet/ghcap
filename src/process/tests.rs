//! EN: Process safety regression tests.
//! JA: 外部プロセスの安全性に関する回帰テストです。
use super::executor::append_tail;
#[cfg(unix)]
use super::executor::execute;
use super::*;
#[cfg(unix)]
use std::{
    process::Command,
    time::{Duration, Instant},
};
#[test]
fn terminal_and_credentials_are_not_replayed() {
    let s = safe_text("\x1b[31m gho_secret https://user:password@example.com/x");
    assert!(!s.contains('\x1b'));
    assert!(!s.contains("secret"));
    assert!(!s.contains("password"));
}
#[test]
fn bounded_tail() {
    let mut b = vec![1; 8];
    append_tail(&mut b, &[2; 8], 10);
    assert_eq!(b.len(), 10);
    assert_eq!(&b[2..], &[2; 8]);
}
#[cfg(unix)]
#[test]
fn capture_invalid_utf8_is_failure() {
    let o = capture("sh", &["-c", "printf '\\377'"], None).unwrap();
    assert!(!o.success);
    assert_eq!(o.termination, Termination::ReadFailed);
}
#[cfg(unix)]
#[test]
fn actual_exit_code() {
    let o = capture("sh", &["-c", "exit 7"], None).unwrap();
    assert_eq!(o.termination, Termination::Exited(7));
    assert_eq!(o.code, 7);
    let message = failure("sh", &o);
    // EN: Check the actual status and keep the diagnosis after the external output.
    // JA: 実際の終了状態を検証し、外部出力の後に診断を表示することを確認します。
    let status = message
        .find("[EXIT STATUS / 終了状態] Exited(7) (code 7)")
        .unwrap();
    let diagnosis = message.find("[GHCAP DIAGNOSIS / ghcap診断]").unwrap();
    assert!(status < diagnosis);
}
#[cfg(unix)]
#[test]
fn timeout_is_distinct() {
    let o = execute(
        "sh",
        &["-c", "sleep 10"],
        None,
        false,
        Duration::from_millis(80),
        || false,
    )
    .unwrap();
    assert_eq!(o.termination, Termination::TimedOut);
}
#[cfg(unix)]
#[test]
fn cancellation_is_distinct() {
    let started = Instant::now();
    let o = execute(
        "sh",
        &["-c", "sleep 10"],
        None,
        false,
        Duration::from_secs(5),
        || started.elapsed() > Duration::from_millis(80),
    )
    .unwrap();
    assert_eq!(o.termination, Termination::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(3));
}
#[cfg(unix)]
#[test]
fn descendant_holding_pipe_is_bounded() {
    let started = Instant::now();
    let o = capture("sh", &["-c", "sleep 10 & exit 0"], None).unwrap();
    assert_eq!(o.termination, Termination::ReadFailed);
    assert!(started.elapsed() < Duration::from_secs(5));
}
#[cfg(unix)]
#[test]
fn oversized_capture_is_not_success() {
    let o = capture("sh", &["-c", "yes x"], None).unwrap();
    assert_eq!(o.termination, Termination::OutputLimit);
    assert!(!o.success);
}
#[cfg(unix)]
#[test]
fn interactive_terminal_restores_group() {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return;
    }
    let previous = nix::unistd::tcgetpgrp(std::io::stdin()).unwrap();
    let mut command = Command::new("sh");
    command.args(["-c", "test -t 0"]);
    assert!(interactive(&mut command).unwrap());
    assert_eq!(nix::unistd::tcgetpgrp(std::io::stdin()).unwrap(), previous);
}
