//! EN: tests UI workflow with one responsibility per Rust module.
//! JA: tests の画面と操作を独立したRustモジュールとして管理します。

use super::repository_menu::repo_from_remote;
#[test]
fn parses_https_remote() {
    let r = repo_from_remote("https://github.com/example/project.git");
    assert_eq!(r.full_name, "example/project");
    assert_eq!(r.name, "project");
}
#[test]
fn parses_ssh_remote() {
    let r = repo_from_remote("git@github.com:example/project.git");
    assert_eq!(r.full_name, "example/project");
    assert_eq!(r.name, "project");
}
#[test]
fn parses_ssh_url_remote() {
    let r = repo_from_remote("ssh://git@github.com/example/project.git");
    assert_eq!(r.full_name, "example/project");
    assert_eq!(r.name, "project");
}
