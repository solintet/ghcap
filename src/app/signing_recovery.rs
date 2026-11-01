//! EN: signing recovery UI workflow with one responsibility per Rust module.
//! JA: signing recovery の画面と操作を独立したRustモジュールとして管理します。
use super::t;
use crate::git;
use crate::gpg;
use crate::ui::{self, Tui};
use std::io;
use std::path::Path;

pub(super) fn is_gpg_error(error: &str) -> bool {
    let e = error.to_ascii_lowercase();
    e.contains("gpg failed to sign")
        || e.contains("gpg: signing failed")
        || e.contains("failed to sign the data")
        || e.contains("gpg signing")
        || e.contains("gpg failed")
}

pub(super) fn gpg_recovery(tui: &mut Tui, path: &Path, lang: &str) -> io::Result<bool> {
    let keys = match gpg::secret_keys() {
        Ok(k) => k,
        Err(e) => {
            ui::cui_operation(lang, &t(lang, "common.error"), || Err(e))?;
            return Ok(false);
        }
    };
    if keys.is_empty() {
        let uid = ui::cui_prompt(&t(lang, "gpg.uid"), "Name <email>")?.unwrap_or_default();
        if uid.trim().is_empty() {
            return Ok(false);
        }
        if !ui::cui_operation(lang, &t(lang, "gpg.generate"), || {
            gpg::generate(&uid).map(|_| t(lang, "gpg.generated"))
        })? {
            return Ok(false);
        }
    }
    let keys = match gpg::secret_keys() {
        Ok(k) => k,
        Err(e) => {
            ui::cui_operation(lang, &t(lang, "common.error"), || Err(e))?;
            return Ok(false);
        }
    };
    let items = keys
        .iter()
        .map(|(id, uid)| format!("{id}  {uid}"))
        .collect::<Vec<_>>();
    let Some(i) = ui::select(tui, lang, &t(lang, "gpg.select"), &items)? else {
        return Ok(false);
    };
    let key = &keys[i].0;
    if !ui::cui_operation(lang, &t(lang, "gpg.register"), || {
        gpg::register_key(key)?;
        git::set_signing_key(path, key).map(|_| t(lang, "gpg.registered"))
    })? {
        return Ok(false);
    }
    Ok(true)
}
