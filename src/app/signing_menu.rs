//! EN: signing menu UI workflow with one responsibility per Rust module.
//! JA: signing menu の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::gpg;
use crate::models::Config;
use crate::ui::{self, Tui};
use std::io;

pub(super) fn gpg_menu(tui: &mut Tui, config: &Config) -> io::Result<()> {
    let lang = config.language.clone();
    let items = vec![
        t(&lang, "gpg.list"),
        t(&lang, "gpg.generate"),
        t(&lang, "gpg.register"),
        t(&lang, "common.back"),
    ];
    let Some(i) = ui::select(tui, &lang, &t(&lang, "repo.gpg"), &items)? else {
        return Ok(());
    };
    match i {
        0 => {
            ui::cui_operation(&lang, &t(&lang, "gpg.list"), gpg::list_secret)?;
        }
        1 => {
            let uid = ui::cui_prompt(&t(&lang, "gpg.uid"), "Name <email>")?.unwrap_or_default();
            ui::cui_operation(&lang, &t(&lang, "gpg.generate"), || {
                gpg::generate(&uid).map(|_| t(&lang, "gpg.generated"))
            })?;
        }
        2 => {
            let keys = match gpg::secret_keys() {
                Ok(v) => v,
                Err(e) => return ui::message(&lang, "GPG", &e),
            };
            let items = keys
                .iter()
                .map(|(id, uid)| format!("{id} {uid}"))
                .collect::<Vec<_>>();
            if items.is_empty() {
                return ui::message(&lang, "GPG", "No usable signing key found.");
            }
            if let Some(index) = ui::select(tui, &lang, "Select public key to register", &items)? {
                if ui::cui_yes_no(&lang, "Register public key", &items[index])? {
                    ui::cui_operation(&lang, &t(&lang, "gpg.register"), || {
                        gpg::register_key(&keys[index].0).map(|_| t(&lang, "gpg.registered"))
                    })?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
