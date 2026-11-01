//! EN: home UI workflow with one responsibility per Rust module.
//! JA: home の画面と操作を独立したRustモジュールとして管理します。
use super::{accounts, local_repo, presets, settings};
use crate::i18n;
use crate::models::Config;
use crate::storage;
use crate::ui::{self, Tui};
use std::io;

pub(super) fn t(lang: &str, key: &str) -> String {
    i18n::text(lang, key)
}

pub fn run(mut config: Config, tui: &mut Tui) -> io::Result<()> {
    loop {
        let lang = config.language.clone();
        let items = vec![
            t(&lang, "home.accounts"),
            t(&lang, "home.local"),
            t(&lang, "home.presets"),
            t(&lang, "home.settings"),
            "Repository setup / リポジトリ初回設定".to_string(),
            t(&lang, "common.quit"),
        ];
        let Some(index) = ui::select(tui, &lang, &t(&lang, "home.title"), &items)? else {
            break;
        };
        match index {
            0 => accounts(&mut config, tui)?,
            1 => local_repo(&mut config, tui)?,
            2 => presets(&mut config, tui)?,
            3 => settings(&mut config, tui)?,
            4 => crate::workflows::setup::from_tui()?,
            _ => break,
        }
    }
    storage::save_config(&config).map_err(io::Error::other)
}
