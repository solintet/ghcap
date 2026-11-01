//! EN: Hand over the foreground terminal safely for interactive auth.
//! JA: 対話的な認証処理へ端末を一時的に渡し、終了時に復元します。
use super::{
    executor::{stop_child, Foreground},
    shutting_down,
};
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

pub fn interactive(command: &mut Command) -> Result<bool, String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    #[cfg(unix)]
    let _foreground = Foreground::for_child(child.id(), true);
    let started = Instant::now();
    loop {
        if shutting_down() || started.elapsed() > Duration::from_secs(15 * 60) {
            stop_child(&mut child);
            let _ = child.wait();
            return Err("Interactive authentication cancelled or timed out; inspect gh auth status before retrying.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(true)
                } else {
                    Err(format!("Interactive authentication failed: {status}"))
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                stop_child(&mut child);
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    }
}
