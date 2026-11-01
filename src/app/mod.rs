//! EN: UI workflows are separate Rust modules, rather than preprocessor includes.
//! JA: UI機能をinclude!ではなく独立したRustモジュールで分割します。
mod accounts;
mod clone_menu;
mod commit_menu;
mod commit_push_menu;
mod home;
mod presets;
mod push_menu;
mod release_menu;
mod repository_menu;
mod repository_settings;
mod settings;
mod setup_menu;
mod signing_menu;
mod signing_recovery;
#[cfg(test)]
mod tests;

use accounts::accounts;
use clone_menu::clone_repo;
use commit_menu::{commit_message_flow, commit_repo, review_commit, CommitMessageOutcome};
use commit_push_menu::commit_push;
pub use home::run;
use home::t;
use presets::presets;
use push_menu::push_repo;
use release_menu::release_flow;
use repository_menu::{local_repo, repo_menu};
use repository_settings::repo_settings;
use settings::settings;
use setup_menu::{ensure_git_setup, git_setup, repo_local_path, repo_path};
use signing_menu::gpg_menu;
use signing_recovery::{gpg_recovery, is_gpg_error};
