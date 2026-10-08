use crate::models::{Config, RepoConfig};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs::{self, File}, io::{self, Write}, path::{Path, PathBuf}};

#[cfg(unix)]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

pub fn cfg_dir() -> PathBuf { dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("ghcap") }
pub fn cfg_path() -> PathBuf { cfg_dir().join("config.json") }
pub fn repo_cfg_path(repo: &str) -> PathBuf { cfg_dir().join("repos").join(format!("{}.json", safe_name(repo))) }
pub fn runtime_dir() -> PathBuf { dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("ghcap") }

fn safe_name(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect()
}

fn restrict_file(path: &Path) -> io::Result<()> {
    #[cfg(unix)] { fs::set_permissions(path, Permissions::from_mode(0o600))?; }
    Ok(())
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        let mut file = File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    restrict_file(&tmp)?;
    if path.exists() { fs::remove_file(path)?; }
    fs::rename(tmp, path)?;
    restrict_file(path)
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> io::Result<T> {
    if !path.exists() { return Ok(T::default()); }
    let text = fs::read_to_string(path)?;
    match serde_json::from_str(&text) {
        Ok(value) => Ok(value),
        Err(_) => Ok(T::default()),
    }
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    atomic_write(path, &data)
}

pub fn load_config() -> io::Result<Config> { load_json(&cfg_path()) }
pub fn save_config(config: &Config) -> io::Result<()> { save_json(&cfg_path(), config) }
pub fn load_repo_config(name: &str) -> RepoConfig { load_json(&repo_cfg_path(name)).unwrap_or_default() }
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
