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
    let mut command = cmd(program, args).stdout_capture().stderr_capture().unchecked();
    if let Some(path) = cwd { command = command.dir(path); }
    let result = command.run().map_err(|e| format!("{program}: {e}"))?;
    Ok(Output {
        success: result.status.success(),
        code: result.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
    })
}

pub fn failure(program: &str, output: &Output) -> String {
    let detail = if output.stderr.trim().is_empty() { output.stdout.trim() } else { output.stderr.trim() };
    format!("{}\nexit code {}", if detail.is_empty() { format!("{program} failed") } else { detail.to_string() }, output.code)
}


/// Execute a user-configured pre-push shell command with inherited terminal I/O.
/// Shell selection is isolated here so application modules do not duplicate OS branches.
pub fn run_shell(command: &str, dir: &Path) -> Result<(), String> {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", command])
    } else {
        ("sh", vec!["-c", command])
    };
    let result = cmd(program, args).dir(dir).unchecked().run()
        .map_err(|error| format!("shell: {error}"))?;
    if result.status.success() { Ok(()) }
    else { Err(format!("shell exited with status {}", result.status)) }
}
