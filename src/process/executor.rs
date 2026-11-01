//! EN: Spawn external programs, manage bounded output, cancellation and timeouts.
//! JA: 外部実行・出力上限・キャンセル・タイムアウトの責任を持ちます。
use super::{safe_text, shutting_down, Output, Termination};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

const CAPTURE_LIMIT: usize = 8 * 1024 * 1024;
const TAIL_LIMIT: usize = 256 * 1024;

/// EN: Run a bounded non-interactive command and report the true exit reason.
/// JA: 対話入力しないコマンドを上限制御付きで実行し、実際の終了理由を返します。
pub fn capture(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output, String> {
    capture_timeout(program, args, cwd, Duration::from_secs(60))
}
pub fn capture_timeout(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
) -> Result<Output, String> {
    execute(program, args, cwd, false, timeout, || false)
}
pub fn capture_with_wait(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<Output, String> {
    execute(
        program,
        args,
        cwd,
        true,
        Duration::from_secs(15 * 60),
        crate::actions::stop_requested,
    )
}
/// EN: Run interactive-capable operations with the same cancellation and timeout limits.
/// JA: 対話可能なコマンドも同じ中断・タイムアウト制限のもとで実行します。
pub fn live(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output, String> {
    execute(
        program,
        args,
        cwd,
        true,
        Duration::from_secs(15 * 60),
        crate::actions::stop_requested,
    )
}

pub(super) fn append_tail(dst: &mut Vec<u8>, data: &[u8], limit: usize) {
    if data.len() >= limit {
        dst.clear();
        dst.extend_from_slice(&data[data.len() - limit..]);
    } else {
        let excess = (dst.len() + data.len()).saturating_sub(limit);
        if excess > 0 {
            dst.drain(..excess);
        }
        dst.extend_from_slice(data);
    }
}
#[cfg(unix)]
pub(super) fn stop_child(child: &mut std::process::Child) {
    use nix::{
        sys::signal::{killpg, Signal},
        unistd::Pid,
    };
    let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGTERM);
    // Only the dedicated group created for this operation is addressed.
    thread::sleep(Duration::from_millis(100));
    let _ = killpg(Pid::from_raw(child.id() as i32), Signal::SIGKILL);
    let _ = child.kill();
}
#[cfg(not(unix))]
pub(super) fn stop_child(child: &mut std::process::Child) {
    let _ = child.kill();
}

#[cfg(unix)]
pub(super) struct Foreground {
    previous: Option<nix::unistd::Pid>,
    mask: nix::sys::signal::SigSet,
}
#[cfg(unix)]
impl Foreground {
    pub(super) fn for_child(pid: u32, live: bool) -> Self {
        use nix::{
            sys::signal::{pthread_sigmask, SigSet, SigmaskHow, Signal},
            unistd::{getpgrp, tcgetpgrp, tcsetpgrp, Pid},
        };
        let mut mask = SigSet::empty();
        let mut blocked = SigSet::empty();
        blocked.add(Signal::SIGTTOU);
        let _ = pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&blocked), Some(&mut mask));
        let previous = if live {
            tcgetpgrp(std::io::stdin())
                .ok()
                .filter(|p| *p == getpgrp())
                .filter(|_| tcsetpgrp(std::io::stdin(), Pid::from_raw(pid as i32)).is_ok())
        } else {
            None
        };
        if previous.is_some() {
            // A newly spawned reader can receive SIGTTIN before foreground
            // ownership is transferred. Resume only our dedicated group.
            let _ = nix::sys::signal::killpg(Pid::from_raw(pid as i32), Signal::SIGCONT);
        }
        Self { previous, mask }
    }
}
#[cfg(unix)]
impl Drop for Foreground {
    fn drop(&mut self) {
        if let Some(pid) = self.previous {
            let _ = nix::unistd::tcsetpgrp(std::io::stdin(), pid);
        }
        let _ = nix::sys::signal::pthread_sigmask(
            nix::sys::signal::SigmaskHow::SIG_SETMASK,
            Some(&self.mask),
            None,
        );
    }
}

pub(super) fn execute<F: FnMut() -> bool>(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    live: bool,
    timeout: Duration,
    mut cancelled: F,
) -> Result<Output, String> {
    // EN: Resolve managed gh without changing PATH or system installations.
    // JA: PATHやシステムを変更せず、利用可能な専用ghを解決します。
    let executable = if program == "gh" {
        crate::dependencies::resolver::resolve_gh()?
    } else {
        std::path::PathBuf::from(program)
    };
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(if live {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if program == "git" {
        command.env("GIT_TERMINAL_PROMPT", "0");
    }
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", safe_text(program)))?;
    #[cfg(unix)]
    let _foreground = Foreground::for_child(child.id(), live);
    let pipes = (|| -> Result<_, String> {
        let stdout = child.stdout.take().ok_or("stdout pipe unavailable")?;
        #[cfg(unix)]
        {
            use nix::fcntl::{fcntl, FcntlArg, OFlag};
            fcntl(&stdout, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(|e| e.to_string())?;
        }
        let stderr = child.stderr.take().ok_or("stderr pipe unavailable")?;
        #[cfg(unix)]
        {
            use nix::fcntl::{fcntl, FcntlArg, OFlag};
            fcntl(&stderr, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(|e| e.to_string())?;
        }
        Ok((stdout, stderr))
    })();
    let (stdout, stderr) = match pipes {
        Ok(pipes) => pipes,
        Err(e) => {
            stop_child(&mut child);
            let _ = child.wait();
            return Err(e);
        }
    };
    let reader_stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::sync_channel(16);
    let mut readers = Vec::new();
    for (index, mut pipe) in [
        (0, Box::new(stdout) as Box<dyn Read + Send>),
        (1, Box::new(stderr) as Box<dyn Read + Send>),
    ] {
        let tx = tx.clone();
        let stop = reader_stop.clone();
        readers.push(thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                match pipe.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send((index, Ok(buf[..n].to_vec()))).is_err() {
                            return;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) => {
                        let _ = tx.send((index, Err(e.to_string())));
                        break;
                    }
                }
            }
        }));
    }
    drop(tx);
    let start = Instant::now();
    let mut heartbeat = Instant::now();
    let mut buffers = [Vec::new(), Vec::new()];
    let mut displayed = [Vec::new(), Vec::new()];
    let mut status = None;
    let mut exited = None;
    let mut termination = None;
    let mut disconnected = false;
    loop {
        if termination.is_none() && (shutting_down() || cancelled()) {
            termination = Some(Termination::Cancelled);
            stop_child(&mut child);
        }
        if termination.is_none() && start.elapsed() > timeout {
            termination = Some(Termination::TimedOut);
            stop_child(&mut child);
        }
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok((index, Ok(bytes))) => {
                let limit = if live { TAIL_LIMIT } else { CAPTURE_LIMIT };
                if !live && buffers[index].len() + bytes.len() > limit && termination.is_none() {
                    termination = Some(Termination::OutputLimit);
                    stop_child(&mut child);
                }
                append_tail(&mut buffers[index], &bytes, limit);
                if live {
                    displayed[index].extend_from_slice(&bytes);
                    while let Some(end) = displayed[index].iter().position(|b| *b == b'\n') {
                        let line: Vec<_> = displayed[index].drain(..=end).collect();
                        // Bound rendering as well as retention, including oversized lines.
                        let line = String::from_utf8_lossy(&line);
                        let safe = safe_text(&line);
                        let _ = if index == 0 {
                            std::io::stdout().write_all(safe.as_bytes())
                        } else {
                            std::io::stderr().write_all(safe.as_bytes())
                        };
                    }
                    if displayed[index].len() > 32 * 1024 {
                        displayed[index].clear();
                        let _ = std::io::stderr().write_all(b"[oversized progress line omitted]\n");
                    }
                }
            }
            Ok((_, Err(_))) => {
                if termination.is_none() {
                    termination = Some(Termination::ReadFailed);
                    stop_child(&mut child);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => disconnected = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    exited = Some(Instant::now());
                }
                Ok(None) => {}
                Err(e) => {
                    stop_child(&mut child);
                    let _ = child.wait();
                    reader_stop.store(true, Ordering::SeqCst);
                    drop(rx);
                    for reader in readers {
                        #[cfg(unix)]
                        {
                            let _ = reader.join();
                        }
                        #[cfg(not(unix))]
                        if reader.is_finished() {
                            let _ = reader.join();
                        }
                    }
                    return Err(format!("{program}: wait failed: {e}"));
                }
            }
        }
        if status.is_some() && disconnected {
            break;
        }
        if exited.is_some_and(|t: Instant| t.elapsed() > Duration::from_secs(2)) {
            if termination.is_none() {
                termination = Some(Termination::ReadFailed);
                stop_child(&mut child);
            }
            break;
        }
        if live && heartbeat.elapsed() > Duration::from_secs(1) {
            eprintln!("Running... Please wait.");
            heartbeat = Instant::now();
        }
    }
    let status = match status {
        Some(s) => s,
        None => child.wait().map_err(|e| format!("{program}: {e}"))?,
    };
    reader_stop.store(true, Ordering::SeqCst);
    drop(rx);
    // Readers unblock when the owned process group closes its pipes. Do not
    // block the UI forever on a detached descendant that retained a descriptor.
    for reader in readers {
        #[cfg(unix)]
        {
            let _ = reader.join();
        }
        #[cfg(not(unix))]
        if reader.is_finished() {
            let _ = reader.join();
        }
    }
    if live {
        for bytes in displayed {
            if !bytes.is_empty() {
                eprint!("{}", safe_text(&String::from_utf8_lossy(&bytes)));
            }
        }
    }
    let decode = |b: Vec<u8>| {
        if live {
            // The bounded diagnostic tail can begin inside a UTF-8 code point.
            return Ok(String::from_utf8_lossy(&b).into_owned());
        }
        String::from_utf8(b).map_err(|_| {
            "non-UTF-8 output; operation result cannot be safely interpreted".to_string()
        })
    };
    let mut stderr = match decode(std::mem::take(&mut buffers[1])) {
        Ok(s) => s,
        Err(e) => {
            termination.get_or_insert(Termination::ReadFailed);
            e
        }
    };
    let stdout = match decode(std::mem::take(&mut buffers[0])) {
        Ok(s) => s,
        Err(e) => {
            termination.get_or_insert(Termination::ReadFailed);
            stderr.push_str(&format!("\n{e}"));
            String::new()
        }
    };
    let termination = termination.unwrap_or_else(|| {
        status
            .code()
            .map_or(Termination::Signal, Termination::Exited)
    });
    let success = status.success() && matches!(termination, Termination::Exited(0));
    Ok(Output {
        success,
        code: status.code().unwrap_or(1),
        stdout,
        stderr,
        termination,
    })
}
