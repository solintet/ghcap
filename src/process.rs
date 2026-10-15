use duct::cmd;
use std::path::Path;

#[derive(Debug)]
pub struct Output {
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn capture(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output, String> {
    let mut command = cmd(program, args)
        .stdout_capture()
        .stderr_capture()
        .unchecked();
    if let Some(path) = cwd {
        command = command.dir(path);
    }
    let result = command.run().map_err(|e| format!("{program}: {e}"))?;
    Ok(Output {
        success: result.status.success(),
        code: result.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
    })
}

pub fn failure(program: &str, output: &Output) -> String {
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    format!(
        "{}\nexit code {}",
        if detail.is_empty() {
            format!("{program} failed")
        } else {
            detail.to_string()
        },
        output.code
    )
}

/// Execute a user-configured pre-push shell command with inherited terminal I/O.
/// Shell selection is isolated here so application modules do not duplicate OS branches.
pub fn run_shell(command: &str, dir: &Path) -> Result<(), String> {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", command])
    } else {
        ("sh", vec!["-c", command])
    };
    let result = cmd(program, args)
        .dir(dir)
        .unchecked()
        .run()
        .map_err(|error| format!("shell: {error}"))?;
    if result.status.success() {
        Ok(())
    } else {
        Err(format!("shell exited with status {}", result.status))
    }
}

/// Capture a command's output while displaying a periodic wait message.
pub fn capture_with_wait(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<Output, String> {
    use std::{
        io::Read,
        process::{Command, Stdio},
        sync::mpsc::{self, RecvTimeoutError},
        thread,
        time::Duration,
    };

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = cwd {
        command.current_dir(path);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{program}: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{program}: failed to capture stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("{program}: failed to capture stderr"))?;

    let stdout_thread = thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        text
    });
    let stderr_thread = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let (stop_tx, stop_rx) = mpsc::channel();
    let ticker = thread::spawn(move || loop {
        match stop_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => println!("Running... Please wait."),
        }
    });

    let status = child.wait();
    let _ = stop_tx.send(());
    let _ = ticker.join();
    let status = status.map_err(|error| format!("{program}: {error}"))?;
    let stdout = stdout_thread
        .join()
        .map_err(|_| format!("{program}: stdout reader thread failed"))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| format!("{program}: stderr reader thread failed"))?;
    Ok(Output {
        success: status.success(),
        code: status.code().unwrap_or(1),
        stdout,
        stderr,
    })
}
