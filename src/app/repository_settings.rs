//! EN: repository settings UI workflow with one responsibility per Rust module.
//! JA: repository settings の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::models::{Config, Repo};
use crate::storage;
use crate::ui::{self, Tui};
use std::io;

pub(super) fn repo_settings(config: &mut Config, tui: &mut Tui, repo: &Repo) -> io::Result<()> {
    let lang = config.language.clone();
    let mut settings = storage::load_repo_config(&repo.full_name)?;
    loop {
        let items = vec![
            format!(
                "{}: {}",
                t(&lang, "settings.pre_shell"),
                if settings.pre_push_enabled {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            format!(
                "{}: {}",
                t(&lang, "settings.shell_command"),
                if settings.pre_push_command.is_empty() {
                    "<none>"
                } else {
                    &settings.pre_push_command
                }
            ),
            format!(
                "{}: {}",
                t(&lang, "settings.shell_confirmation"),
                if settings.shell_confirmation {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            t(&lang, "settings.preset"),
            t(&lang, "common.back"),
        ];
        let Some(i) = ui::select(tui, &lang, &t(&lang, "repo.settings"), &items)? else {
            return Ok(());
        };
        match i {
            0 => settings.pre_push_enabled = !settings.pre_push_enabled,
            1 => {
                if let Some(command) = ui::prompt(
                    tui,
                    &t(&lang, "settings.shell_command"),
                    &settings.pre_push_command,
                )? {
                    settings.pre_push_command = command;
                }
            }
            2 => settings.shell_confirmation = !settings.shell_confirmation,
            3 => {
                if config.presets.is_empty() {
                    ui::message(
                        &lang,
                        &t(&lang, "settings.preset"),
                        &t(&lang, "presets.none"),
                    )?;
                } else {
                    let names = config
                        .presets
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>();
                    if let Some(i) = ui::select(tui, &lang, &t(&lang, "settings.preset"), &names)? {
                        settings.preset_id = config.presets[i].id.clone();
                    }
                }
            }
            _ => {
                storage::save_repo_config(&repo.full_name, &settings)?;
                return Ok(());
            }
        }
        storage::save_repo_config(&repo.full_name, &settings)?;
    }
}
