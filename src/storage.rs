use crate::models::{Config, RepoConfig};
use etcetera::{choose_base_strategy, BaseStrategy};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tempfile::NamedTempFile;
const MAX_JSON: u64 = 2 * 1024 * 1024;
thread_local! {static REVISIONS:RefCell<HashMap<PathBuf,Option<Vec<u8>>>>=RefCell::new(HashMap::new());}
struct Session {
    _lock: File,
    id: String,
}
static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
fn base(data: bool) -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        let old = PathBuf::from(home).join("Library/Application Support/ghcap");
        if old.is_dir() {
            return Ok(old);
        }
    }
    let s = choose_base_strategy().map_err(io::Error::other)?;
    let path = if data { s.data_dir() } else { s.config_dir() };
    if !path.is_absolute() {
        return Err(io::Error::other(
            "Cannot resolve an absolute private configuration directory; no cwd fallback is used.",
        ));
    }
    Ok(path.join("ghcap"))
}
pub fn cfg_dir() -> io::Result<PathBuf> {
    base(false)
}
pub fn cfg_path() -> io::Result<PathBuf> {
    Ok(cfg_dir()?.join("config.json"))
}
pub fn repo_cfg_path(repo: &str) -> io::Result<PathBuf> {
    let name = repo
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(cfg_dir()?.join("repos").join(format!("{name}.json")))
}
pub fn runtime_dir() -> io::Result<PathBuf> {
    base(true)
}
fn private_dir(path: &Path) -> io::Result<()> {
    let new = !path.exists();
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    if new {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
fn private_open(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(nix::libc::O_NOFOLLOW);
    }
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(io::Error::other("Refusing symlink lock file"));
    }
    options.open(path)
}
fn bytes(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
    }
    match options.open(path) {
        Ok(file) => {
            if !file.metadata()?.is_file() {
                return Err(io::Error::other("Configuration must be a regular file"));
            }
            if file.metadata()?.len() > MAX_JSON {
                return Err(io::Error::other(
                    "Configuration exceeds 2 MiB limit; original file preserved.",
                ));
            }
            let mut b = Vec::new();
            file.take(MAX_JSON + 1).read_to_end(&mut b)?;
            if b.len() as u64 > MAX_JSON {
                return Err(io::Error::other("Configuration too large"));
            }
            Ok(Some(b))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing configuration parent"))?;
    private_dir(parent)?;
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    tmp.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> io::Result<T> {
    let raw = bytes(path)?;
    let value = match raw.as_deref() {
        None => T::default(),
        Some(b) => serde_json::from_slice(b).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid JSON in {}: {e}; original preserved",
                    path.display()
                ),
            )
        })?,
    };
    REVISIONS.with(|r| {
        r.borrow_mut().insert(path.to_owned(), raw);
    });
    Ok(value)
}
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    if data.len() as u64 > MAX_JSON {
        return Err(io::Error::other("Configuration too large"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing config parent"))?;
    private_dir(parent)?;
    let lock = private_open(&path.with_extension("json.lock"))?;
    let start = std::time::Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock)
                if start.elapsed() < std::time::Duration::from_secs(2) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            Err(e) => {
                return Err(io::Error::other(format!(
                    "Configuration lock unavailable: {e}"
                )))
            }
        }
    }
    let current = bytes(path)?;
    let expected = REVISIONS.with(|r| r.borrow().get(path).cloned());
    if expected.is_some_and(|v| v != current) {
        return Err(io::Error::other(
            "Configuration changed in another operation. Reload before saving; original preserved.",
        ));
    }
    if current.as_deref() != Some(&data) {
        atomic_write(path, &data)?;
    }
    REVISIONS.with(|r| {
        r.borrow_mut().insert(path.to_owned(), Some(data));
    });
    Ok(())
}
pub fn load_config() -> io::Result<Config> {
    let p = cfg_path()?;
    if let Some(b) = bytes(&p)? {
        let v: serde_json::Value = serde_json::from_slice(&b).map_err(io::Error::other)?;
        if !v.is_object()
            || !["language", "accounts", "presets", "schema_version"]
                .iter()
                .any(|k| v.get(k).is_some())
        {
            return Err(io::Error::other(
                "Unrecognized configuration schema; original preserved.",
            ));
        }
    }
    let config: Config = load_json(&p)?;
    if config.accounts.iter().any(|a| !a._legacy_token.is_empty()) {
        if let Some(original) = bytes(&p)? {
            let mut backup = tempfile::Builder::new()
                .prefix("config-legacy-private-")
                .tempfile_in(cfg_dir()?)?;
            backup.write_all(&original)?;
            backup.as_file().sync_all()?;
            let _ = backup.keep().map_err(|e| e.error)?;
        }
    }
    if config.schema_version > 1 {
        return Err(io::Error::other(
            "Newer configuration schema; this version will not overwrite it.",
        ));
    }
    Ok(config)
}
pub fn save_config(config: &Config) -> io::Result<()> {
    save_json(&cfg_path()?, config)
}
pub fn load_repo_config(name: &str) -> io::Result<RepoConfig> {
    load_json(&repo_cfg_path(name)?)
}
pub fn save_repo_config(name: &str, config: &RepoConfig) -> io::Result<()> {
    save_json(&repo_cfg_path(name)?, config)
}
pub fn new_id(prefix: &str) -> io::Result<String> {
    let file = NamedTempFile::new()?;
    Ok(format!(
        "{prefix}-{}",
        file.path()
            .file_name()
            .ok_or_else(|| io::Error::other("temp name unavailable"))?
            .to_string_lossy()
    ))
}
pub fn init_instance() -> io::Result<()> {
    let dir = runtime_dir()?;
    private_dir(&dir)?;
    let lock = private_open(&dir.join("instance.lock"))?;
    lock.try_lock().map_err(|_| {
        io::Error::other("Another ghcap instance is active. Close it before starting another.")
    })?;
    let id = new_id("instance")?;
    atomic_write(
        &dir.join("instance.json"),
        &serde_json::to_vec(&serde_json::json!({"pid":std::process::id(),"id":id}))?,
    )?;
    let _ = fs::remove_file(dir.join("command.json"));
    *SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| io::Error::other("Instance lock poisoned"))? =
        Some(Session { _lock: lock, id });
    Ok(())
}
pub fn clear_instance() {
    if let Some(session) = SESSION.get() {
        if let Ok(mut state) = session.lock() {
            if state.is_some() {
                if let Ok(dir) = runtime_dir() {
                    let _ = fs::remove_file(dir.join("instance.json"));
                    let _ = fs::remove_file(dir.join("command.json"));
                }
                *state = None;
            }
        }
    }
}
pub fn send_stop(command: &str) -> i32 {
    let run = || -> io::Result<()> {
        let dir = runtime_dir()?;
        let lock = private_open(&dir.join("instance.lock"))?;
        match lock.try_lock() {
            Ok(()) => return Err(io::Error::other("No running ghcap instance.")),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(e) => {
                return Err(io::Error::other(format!(
                    "Cannot inspect instance lock: {e}"
                )))
            }
        }
        let v: serde_json::Value = serde_json::from_slice(
            &bytes(&dir.join("instance.json"))?
                .ok_or_else(|| io::Error::other("Missing instance information"))?,
        )?;
        let id = v
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| io::Error::other("Invalid instance information"))?;
        atomic_write(
            &dir.join("command.json"),
            &serde_json::to_vec(&serde_json::json!({"command":command,"id":id}))?,
        )?;
        Ok(())
    };
    match run() {
        Ok(()) => {
            println!("Stop requested; completion is reported by the running instance.");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
pub fn read_stop() -> Option<String> {
    let session = SESSION.get()?.lock().ok()?;
    let id = &session.as_ref()?.id;
    let p = runtime_dir().ok()?.join("command.json");
    let v: serde_json::Value = serde_json::from_slice(bytes(&p).ok()??.as_slice()).ok()?;
    if v.get("id")?.as_str()? != id {
        let _ = fs::remove_file(p);
        return None;
    }
    let command = v.get("command")?.as_str()?.to_owned();
    let _ = fs::remove_file(p);
    Some(command)
}

pub fn preset_references(id: &str) -> io::Result<Vec<String>> {
    let dir = cfg_dir()?.join("repos");
    let mut used = Vec::new();
    if !dir.exists() {
        return Ok(used);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|s| s == "json") {
            let config: RepoConfig = load_json(&entry.path())?;
            if config.preset_id == id {
                used.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    Ok(used)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_is_preserved() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bad.json");
        fs::write(&p, b"broken").unwrap();
        assert!(load_json::<Config>(&p).is_err());
        assert_eq!(fs::read(p).unwrap(), b"broken");
    }
    #[test]
    fn concurrent_change_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("cfg.json");
        fs::write(&p, b"{}").unwrap();
        let _: serde_json::Value = load_json(&p).unwrap();
        fs::write(&p, b"{\"changed\":true}").unwrap();
        assert!(save_json(&p, &serde_json::json!({})).is_err());
        assert!(fs::read_to_string(p).unwrap().contains("changed"));
    }
    #[test]
    fn unchanged_save_preserves_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("cfg.json");
        let v = serde_json::json!({"a":1});
        save_json(&p, &v).unwrap();
        let before = fs::metadata(&p).unwrap().modified().unwrap();
        save_json(&p, &v).unwrap();
        assert_eq!(before, fs::metadata(p).unwrap().modified().unwrap());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_configuration_not_followed() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("private");
        fs::write(&target, b"secret").unwrap();
        let link = d.path().join("config.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(bytes(&link).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"secret");
    }
    #[cfg(unix)]
    #[test]
    fn fifo_configuration_does_not_block() {
        let d = tempfile::tempdir().unwrap();
        let fifo = d.path().join("config.json");
        nix::unistd::mkfifo(&fifo, nix::sys::stat::Mode::S_IRUSR).unwrap();
        assert!(bytes(&fifo).is_err());
    }
}
