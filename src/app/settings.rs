//! EN: settings UI workflow with one responsibility per Rust module.
//! JA: settings の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::i18n;
use crate::models::Config;
use crate::storage;
use crate::ui::{self, Tui};
use std::io;

pub(super) fn settings(config: &mut Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "settings.change_language"),
            t(&lang, "settings.config_path"),
            t(&lang, "common.back"),
        ];
        let Some(i) = ui::select(tui, &lang, &t(&lang, "home.settings"), &items)? else {
            return Ok(());
        };
        match i {
            0 => {
                let choices = i18n::available()
                    .iter()
                    .map(|c| format!("{} ({c})", i18n::lang_name(c)))
                    .collect::<Vec<_>>();
                if let Some(i) =
                    ui::select(tui, &lang, &t(&lang, "settings.change_language"), &choices)?
                {
                    config.language = i18n::available()[i].to_string();
                    storage::save_config(config)?;
                }
            }
            1 => ui::message(
                &lang,
                &t(&lang, "settings.config_path"),
                &storage::cfg_path()?.display().to_string(),
            )?,
            _ => return Ok(()),
        }
    }
}
