//! EN: presets UI workflow with one responsibility per Rust module.
//! JA: presets の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::models::Config;
use crate::storage;
use crate::ui::{self, ListAction, Tui};
use std::io;

pub(super) fn presets(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let mut selected = 0usize;
    loop {
        let lang = config.language.clone();
        let mut items = config
            .presets
            .iter()
            .map(|p| format!("{} — {}", p.name, p.message))
            .collect::<Vec<_>>();
        items.push(t(&lang, "presets.add"));
        let footer = format!(
            "{}  Enter {}  a {}  e {}  d {}  Shift+↑/↓ {}  Esc {}",
            t(&lang, "key.navigate"),
            t(&lang, "presets.use_key"),
            t(&lang, "presets.add_key"),
            t(&lang, "presets.edit_key"),
            t(&lang, "presets.delete_key"),
            t(&lang, "presets.move_key"),
            t(&lang, "common.back")
        );
        let Some((index, action)) = ui::manage_select(
            tui,
            &lang,
            &t(&lang, "repo.presets"),
            &items,
            &footer,
            &mut selected,
        )?
        else {
            return Ok(());
        };
        let add_row = index == config.presets.len();
        match action {
            ListAction::Open if add_row => {
                add_preset(config, tui)?;
                selected = config.presets.len().saturating_sub(1);
            }
            ListAction::Open => return Ok(()),
            ListAction::Add => add_preset(config, tui)?,
            ListAction::Edit if !add_row => edit_preset(config, tui, index)?,
            ListAction::Delete if !add_row => delete_preset(config, tui, index)?,
            ListAction::MoveUp if !add_row && index > 0 => {
                config.presets.swap(index, index - 1);
                selected -= 1;
                storage::save_config(config)?;
            }
            ListAction::MoveDown if !add_row && index + 1 < config.presets.len() => {
                config.presets.swap(index, index + 1);
                selected += 1;
                storage::save_config(config)?;
            }
            ListAction::Back => return Ok(()),
            _ => {}
        }
    }
}
pub(super) fn add_preset(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(name) = ui::prompt(tui, &t(&lang, "presets.name"), "New preset")? else {
        return Ok(());
    };
    let Some(message) = ui::prompt(tui, &t(&lang, "presets.message"), "Update: ")? else {
        return Ok(());
    };
    if name.trim().is_empty() {
        return Ok(());
    }
    let id = storage::new_id("preset")?;
    config
        .presets
        .push(crate::models::Preset { id, name, message });
    storage::save_config(config)
}
pub(super) fn edit_preset(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    let Some(name) = ui::prompt(tui, &t(&lang, "presets.name"), &config.presets[index].name)?
    else {
        return Ok(());
    };
    let Some(message) = ui::prompt(
        tui,
        &t(&lang, "presets.message"),
        &config.presets[index].message,
    )?
    else {
        return Ok(());
    };
    config.presets[index].name = name;
    config.presets[index].message = message;
    storage::save_config(config)
}
pub(super) fn delete_preset(config: &mut Config, tui: &mut Tui, index: usize) -> io::Result<()> {
    let lang = config.language.clone();
    if config.presets.len() <= 1 {
        ui::message(
            &lang,
            &t(&lang, "presets.delete"),
            &t(&lang, "presets.last_forbidden"),
        )?;
        return Ok(());
    }
    let used = storage::preset_references(&config.presets[index].id)?;
    if !used.is_empty() {
        return ui::message(
            &lang,
            "Preset in use",
            &format!(
                "Choose another preset in these repository settings first:\n{}",
                used.join("\n")
            ),
        );
    }
    if ui::confirm(
        tui,
        &lang,
        &t(&lang, "presets.delete"),
        &format!(
            "{}: {}",
            t(&lang, "presets.delete_confirm"),
            config.presets[index].name
        ),
    )? {
        config.presets.remove(index);
        storage::save_config(config)?;
    }
    Ok(())
}
