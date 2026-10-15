use rust_i18n::t;
use std::env;

/// Detect the preferred language from standard Unix locale environment variables.
pub fn detect_language() -> String {
    for value in [
        env::var("LC_ALL").ok(),
        env::var("LANGUAGE").ok(),
        env::var("LANG").ok(),
    ]
    .into_iter()
    .flatten()
    {
        let language = value.to_lowercase();
        if language.starts_with("ja") {
            return "ja".into();
        }
        if language.starts_with("zh") {
            return "zh-CN".into();
        }
        if language.starts_with("ko") {
            return "ko".into();
        }
        if language.starts_with("en") {
            return "en".into();
        }
    }
    "en".into()
}

/// Resolve a translation through rust-i18n. Keys are enumerated so extraction
/// tools can see every static key and unknown keys retain the old key-as-text fallback.
pub fn text(locale: &str, key: &str) -> String {
    match key {
        "language.en" => t!("language.en", locale = locale).to_string(),
        "language.ja" => t!("language.ja", locale = locale).to_string(),
        "language.zh-CN" => t!("language.zh-CN", locale = locale).to_string(),
        "language.ko" => t!("language.ko", locale = locale).to_string(),
        "home.title" => t!("home.title", locale = locale).to_string(),
        "home.accounts" => t!("home.accounts", locale = locale).to_string(),
        "home.local" => t!("home.local", locale = locale).to_string(),
        "home.presets" => t!("home.presets", locale = locale).to_string(),
        "home.settings" => t!("home.settings", locale = locale).to_string(),
        "common.quit" => t!("common.quit", locale = locale).to_string(),
        "common.back" => t!("common.back", locale = locale).to_string(),
        "common.cancel" => t!("common.cancel", locale = locale).to_string(),
        "common.error" => t!("common.error", locale = locale).to_string(),
        "common.done" => t!("common.done", locale = locale).to_string(),
        "key.navigate" => t!("key.navigate", locale = locale).to_string(),
        "key.select_back" => t!("key.select_back", locale = locale).to_string(),
        "key.message_close" => t!("key.message_close", locale = locale).to_string(),
        "accounts.title" => t!("accounts.title", locale = locale).to_string(),
        "accounts.add" => t!("accounts.add", locale = locale).to_string(),
        "accounts.added" => t!("accounts.added", locale = locale).to_string(),
        "accounts.auth_failed" => t!("accounts.auth_failed", locale = locale).to_string(),
        "accounts.repo_failed" => t!("accounts.repo_failed", locale = locale).to_string(),
        "repos.title" => t!("repos.title", locale = locale).to_string(),
        "repos.empty" => t!("repos.empty", locale = locale).to_string(),
        "local.not_repo" => t!("local.not_repo", locale = locale).to_string(),
        "local.no_remote" => t!("local.no_remote", locale = locale).to_string(),
        "repo.commit_push" => t!("repo.commit_push", locale = locale).to_string(),
        "repo.commit" => t!("repo.commit", locale = locale).to_string(),
        "repo.push" => t!("repo.push", locale = locale).to_string(),
        "repo.status" => t!("repo.status", locale = locale).to_string(),
        "repo.gpg" => t!("repo.gpg", locale = locale).to_string(),
        "repo.presets" => t!("repo.presets", locale = locale).to_string(),
        "repo.settings" => t!("repo.settings", locale = locale).to_string(),
        "repo.info" => t!("repo.info", locale = locale).to_string(),
        "commit.message" => t!("commit.message", locale = locale).to_string(),
        "info.account" => t!("info.account", locale = locale).to_string(),
        "info.repository" => t!("info.repository", locale = locale).to_string(),
        "presets.add" => t!("presets.add", locale = locale).to_string(),
        "presets.name" => t!("presets.name", locale = locale).to_string(),
        "presets.message" => t!("presets.message", locale = locale).to_string(),
        "presets.edit" => t!("presets.edit", locale = locale).to_string(),
        "presets.up" => t!("presets.up", locale = locale).to_string(),
        "presets.down" => t!("presets.down", locale = locale).to_string(),
        "presets.delete" => t!("presets.delete", locale = locale).to_string(),
        "presets.action" => t!("presets.action", locale = locale).to_string(),
        "presets.none" => t!("presets.none", locale = locale).to_string(),
        "settings.language" => t!("settings.language", locale = locale).to_string(),
        "settings.change_language" => t!("settings.change_language", locale = locale).to_string(),
        "settings.config_path" => t!("settings.config_path", locale = locale).to_string(),
        "settings.pre_shell" => t!("settings.pre_shell", locale = locale).to_string(),
        "settings.shell_command" => t!("settings.shell_command", locale = locale).to_string(),
        "settings.shell_confirmation" => {
            t!("settings.shell_confirmation", locale = locale).to_string()
        }
        "settings.preset" => t!("settings.preset", locale = locale).to_string(),
        "shell.detected" => t!("shell.detected", locale = locale).to_string(),
        "shell.commit_then_run" => t!("shell.commit_then_run", locale = locale).to_string(),
        "shell.run_then_commit" => t!("shell.run_then_commit", locale = locale).to_string(),
        "shell.skip" => t!("shell.skip", locale = locale).to_string(),
        "shell.failed_cancelled" => t!("shell.failed_cancelled", locale = locale).to_string(),
        "gpg.register" => t!("gpg.register", locale = locale).to_string(),
        "gpg.generate" => t!("gpg.generate", locale = locale).to_string(),
        "gpg.list" => t!("gpg.list", locale = locale).to_string(),
        "gpg.uid" => t!("gpg.uid", locale = locale).to_string(),
        "gpg.registered" => t!("gpg.registered", locale = locale).to_string(),
        "gpg.generated" => t!("gpg.generated", locale = locale).to_string(),
        "accounts.none" => t!("accounts.none", locale = locale).to_string(),
        "clone.title" => t!("clone.title", locale = locale).to_string(),
        "clone.destination" => t!("clone.destination", locale = locale).to_string(),
        "clone.done" => t!("clone.done", locale = locale).to_string(),
        "repo.clone" => t!("repo.clone", locale = locale).to_string(),
        "login.url_notice" => t!("login.url_notice", locale = locale).to_string(),
        "login.browser_optional" => t!("login.browser_optional", locale = locale).to_string(),
        "login.start" => t!("login.start", locale = locale).to_string(),
        "login.failed" => t!("login.failed", locale = locale).to_string(),
        "common.exit_code" => t!("common.exit_code", locale = locale).to_string(),
        "common.terminated" => t!("common.terminated", locale = locale).to_string(),
        "key.cui_return" => t!("key.cui_return", locale = locale).to_string(),
        "accounts.switch" => t!("accounts.switch", locale = locale).to_string(),
        "accounts.action" => t!("accounts.action", locale = locale).to_string(),
        "accounts.open" => t!("accounts.open", locale = locale).to_string(),
        "accounts.up" => t!("accounts.up", locale = locale).to_string(),
        "accounts.down" => t!("accounts.down", locale = locale).to_string(),
        "accounts.delete" => t!("accounts.delete", locale = locale).to_string(),
        "common.yes" => t!("common.yes", locale = locale).to_string(),
        "common.no" => t!("common.no", locale = locale).to_string(),
        "accounts.open_key" => t!("accounts.open_key", locale = locale).to_string(),
        "accounts.add_key" => t!("accounts.add_key", locale = locale).to_string(),
        "accounts.edit_key" => t!("accounts.edit_key", locale = locale).to_string(),
        "accounts.delete_key" => t!("accounts.delete_key", locale = locale).to_string(),
        "accounts.move_key" => t!("accounts.move_key", locale = locale).to_string(),
        "accounts.edit_label" => t!("accounts.edit_label", locale = locale).to_string(),
        "accounts.delete_confirm" => t!("accounts.delete_confirm", locale = locale).to_string(),
        "git.folder" => t!("git.folder", locale = locale).to_string(),
        "git.init" => t!("git.init", locale = locale).to_string(),
        "git.remote" => t!("git.remote", locale = locale).to_string(),
        "git.create_folder_failed" => t!("git.create_folder_failed", locale = locale).to_string(),
        "git.ready" => t!("git.ready", locale = locale).to_string(),
        "git.ready_message" => t!("git.ready_message", locale = locale).to_string(),
        "git.not_configured" => t!("git.not_configured", locale = locale).to_string(),
        "repo.git_setup" => t!("repo.git_setup", locale = locale).to_string(),
        "gpg.select" => t!("gpg.select", locale = locale).to_string(),
        "gpg.retry_title" => t!("gpg.retry_title", locale = locale).to_string(),
        "gpg.retry_push" => t!("gpg.retry_push", locale = locale).to_string(),
        "presets.use_key" => t!("presets.use_key", locale = locale).to_string(),
        "presets.add_key" => t!("presets.add_key", locale = locale).to_string(),
        "presets.edit_key" => t!("presets.edit_key", locale = locale).to_string(),
        "presets.delete_key" => t!("presets.delete_key", locale = locale).to_string(),
        "presets.move_key" => t!("presets.move_key", locale = locale).to_string(),
        "presets.delete_confirm" => t!("presets.delete_confirm", locale = locale).to_string(),
        "presets.last_forbidden" => t!("presets.last_forbidden", locale = locale).to_string(),
        "gpg.retry_commit" => t!("gpg.retry_commit", locale = locale).to_string(),
        "clone.not_git" => t!("clone.not_git", locale = locale).to_string(),
        "login.browser_ready" => t!("login.browser_ready", locale = locale).to_string(),
        "login.browser_open_key" => t!("login.browser_open_key", locale = locale).to_string(),
        "dependencies.title" => t!("dependencies.title", locale = locale).to_string(),
        "dependencies.missing" => t!("dependencies.missing", locale = locale).to_string(),
        "dependencies.install" => t!("dependencies.install", locale = locale).to_string(),
        "dependencies.windows" => t!("dependencies.windows", locale = locale).to_string(),
        "dependencies.macos" => t!("dependencies.macos", locale = locale).to_string(),
        "dependencies.linux.debian" => t!("dependencies.linux.debian", locale = locale).to_string(),
        "dependencies.linux.fedora" => t!("dependencies.linux.fedora", locale = locale).to_string(),
        "dependencies.linux.arch" => t!("dependencies.linux.arch", locale = locale).to_string(),
        "dependencies.linux.opensuse" => {
            t!("dependencies.linux.opensuse", locale = locale).to_string()
        }
        "dependencies.linux.generic" => {
            t!("dependencies.linux.generic", locale = locale).to_string()
        }
        "dependencies.docs" => t!("dependencies.docs", locale = locale).to_string(),
        _ => key.to_owned(),
    }
}

pub fn lang_name(locale: &str) -> String {
    text(locale, &format!("language.{locale}"))
}

pub fn available() -> &'static [&'static str] {
    &["en", "ja", "zh-CN", "ko"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_translations_in_selected_locale() {
        assert_eq!(text("en", "common.quit"), "Quit");
        assert_eq!(text("ja", "common.quit"), "終了");
    }

    #[test]
    fn unknown_key_is_returned_unchanged() {
        assert_eq!(text("ja", "missing.key"), "missing.key");
    }
}
