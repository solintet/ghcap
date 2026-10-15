# ghcap 0.27.4 architecture

## Design rule

Use a maintained library or official CLI for generic behavior. ghcap-specific code should only express workflow policy and glue. Do not implement Git internals, GitHub authentication, terminal rendering, or line editing from scratch.

## Responsibilities

| Area | Implementation | Boundary |
|---|---|---|
| Terminal widgets and layout | Ratatui | Screens render state; no direct cursor arithmetic in app logic |
| Terminal input/backend | Crossterm | Key events and terminal mode; backend for Ratatui |
| Single-line editing | Rustyline | Prefilled commit message and ordinary prompts |
| CLI options/help/version | clap | Stop commands and standard CLI metadata |
| Captured external processes | duct via `process` | Shared stdout/stderr/status handling for `git`, `gh`, and `gpg` |
| Long-running streamed Git commands | `git_in_live` | Preserves real-time output and the one-second heartbeat |
| Config/data directories | etcetera | OS-specific directory resolution; old macOS Application Support path is preserved when present |
| Config serialization / CLI JSON | Serde + serde_json | Typed config representation, validation, and machine-readable `gh` output |
| Localization | rust-i18n | Compile-time translation lookup, JSON locale files, selected locale and English fallback |
| Recursive directory traversal | walkdir | Release assets, no symlink following, and pruning metadata directories |
| Safe replacement writes | tempfile | Temporary file in destination directory, sync, persist |
| Top-level errors | anyhow | Add context at the application boundary |
| GitHub operations | official `gh` CLI | Login, account switching, repositories via `--json`, clone, GPG registration |
| Git operations | official `git` CLI | Init, status, add, commit, push, remotes, identity |
| GPG operations | `gpg` CLI | Key listing, generation, export |

## Why some custom code remains

The sequence “select account → select repository → prepare local folder → choose preset → confirm commit → push” is ghcap's product behavior and does not map to one general-purpose crate. That logic remains in app/actions modules, but UI rendering, input editing, argument parsing, serialization, path discovery, and ordinary subprocess capture are delegated. The live Git path uses `std::process` intentionally because it must stream both output pipes and display a heartbeat while the child process is still running.

## Data safety

- Malformed main config JSON returns an error and is not replaced with defaults.
- Malformed repository config JSON is also returned as an error rather than silently defaulted.
- Writes use a temporary file in the target directory and persist it over the target; a failed persist does not first delete the old file.
- Account addition merges newly authenticated accounts into the saved list instead of replacing the list.

## Verification required before release

The source archive can be inspected without Rust, but a release is not considered verified until `cargo check --locked` and `cargo test --locked` pass. CI should run those checks on Linux, Windows, and macOS. GitHub auth, Clone, Commit, Push, and pre-push shell behavior require end-to-end tests with disposable test repositories.
