use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub label: String,
}

impl Account {
    pub fn display_name(&self) -> &str { if self.label.trim().is_empty() { &self.name } else { &self.label } }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub private: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub preset_id: String,
    pub pre_push_enabled: bool,
    pub pre_push_command: String,
    pub shell_confirmation: bool,
    #[serde(default)]
    pub local_path: Option<String>,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self { preset_id: "normal".into(), pre_push_enabled: false, pre_push_command: String::new(), shell_confirmation: true, local_path: None }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub language: String,
    pub accounts: Vec<Account>,
    pub presets: Vec<Preset>,
    pub last_account: usize,
    pub last_repo: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: crate::i18n::detect_language(), accounts: Vec::new(), presets: vec![
                Preset { id: "normal".into(), name: "Normal update".into(), message: "chore: ".into() },
                Preset { id: "fix".into(), name: "Bug fix".into(), message: "fix: ".into() },
                Preset { id: "feature".into(), name: "Feature".into(), message: "feat: ".into() },
            ], last_account: 0, last_repo: None,
        }
    }
}
