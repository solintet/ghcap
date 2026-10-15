use crate::models::{Config, RepoConfig};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io::{self, Write}, path::{Path, PathBuf}};
use etcetera::{choose_base_strategy, BaseStrategy};
use tempfile::NamedTempFile;

#[cfg(unix)]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

fn base_config_dir() -> Option<PathBuf> {
    let strategy = choose_base_strategy().ok()?;
    Some(strategy.config_dir())
}

fn base_data_dir() -> Option<PathBuf> {
    let strategy = choose_base_strategy().ok()?;
    Some(strategy.data_dir())
}

// Preserve the old directories-based macOS location when an existing install uses it.
// New installs use etcetera's native platform convention.
#[cfg(target_os = "macos")]
fn legacy_macos_app_support() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).map(|home| home.join("Library").join("Application Support"))
}

pub fn cfg_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(legacy) = legacy_macos_app_support().map(|p| p.join("ghcap")) {
        if legacy.exists() { return legacy; }
    }
    base_config_dir().map(|p| p.join("ghcap")).unwrap_or_else(|| PathBuf::from(".").join("ghcap"))
}
pub fn cfg_path() -> PathBuf { cfg_dir().join("config.json") }
pub fn repo_cfg_path(repo: &str) -> PathBuf { cfg_dir().join("repos").join(format!("{}.json", safe_name(repo))) }
pub fn runtime_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(legacy) = legacy_macos_app_support().map(|p| p.join("ghcap")) {
        if legacy.exists() { return legacy; }
    }
    base_data_dir().map(|p| p.join("ghcap")).unwrap_or_else(|| PathBuf::from(".").join("ghcap"))
}

fn safe_name(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect()
}

fn restrict_file(path: &Path) -> io::Result<()> {
    #[cfg(unix)] { fs::set_permissions(path, Permissions::from_mode(0o600))?; }
    Ok(())
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    restrict_file(tmp.path())?;
    // Persist replaces the destination atomically where the platform supports it;
    // unlike remove-then-rename, a failed persist never deletes the previous config.
    tmp.persist(path).map_err(|error| error.error)?;
    restrict_file(path)
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> io::Result<T> {
    if !path.exists() { return Ok(T::default()); }
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid JSON in {}: {error}; original file was preserved", path.display()),
    ))
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    atomic_write(path, &data)
}

pub fn load_config() -> io::Result<Config> { load_json(&cfg_path()) }
pub fn save_config(config: &Config) -> io::Result<()> { save_json(&cfg_path(), config) }
pub fn load_repo_config(name: &str) -> io::Result<RepoConfig> { load_json(&repo_cfg_path(name)) }
pub fn save_repo_config(name: &str, config: &RepoConfig) -> io::Result<()> { save_json(&repo_cfg_path(name), config) }

pub fn init_instance() -> io::Result<()> {
    fs::create_dir_all(runtime_dir())?;
    save_json(&runtime_dir().join("instance.json"), &serde_json::json!({"pid": std::process::id()}))
}

pub fn clear_instance() {
    let dir = runtime_dir();
    let _ = fs::remove_file(dir.join("instance.json"));
    let _ = fs::remove_file(dir.join("command.json"));
}

pub fn send_stop(command: &str) -> i32 {
    let dir = runtime_dir();
    if fs::create_dir_all(&dir).is_err() { eprintln!("cannot create runtime directory"); return 1; }
    if !dir.join("instance.json").exists() { eprintln!("No running ghcap instance."); return 1; }
    if save_json(&dir.join("command.json"), &serde_json::json!({"command": command})).is_err() { return 1; }
    println!("{command} sent.");
    0
}

pub fn read_stop() -> Option<String> {
    let path = runtime_dir().join("command.json");
    let text = fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let command = value.get("command")?.as_str()?.to_string();
    let _ = fs::remove_file(path);
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_json_is_reported_and_preserved() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.json");
        let original = b"{ definitely not valid json";
        fs::write(&path, original).expect("write malformed config");
        let result: io::Result<serde_json::Value> = load_json(&path);
        assert_eq!(result.expect_err("malformed JSON must be an error").kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(&path).expect("read original config"), original);
    }

    #[test]
    fn missing_json_uses_default() {
        let dir = tempfile::tempdir().expect("temp dir");
        let value: serde_json::Value = load_json(&dir.path().join("missing.json")).expect("default config");
        assert_eq!(value, serde_json::Value::Null);
    }
}
