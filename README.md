# ghcap 0.27.4

ghcap is a cross-platform terminal workflow manager for GitHub repositories. It delegates GitHub operations to the official `gh` CLI, Git operations to `git`, and signing-key operations to `gpg`; ghcap does not implement GitHub authentication or Git internals itself.

## Architecture

- **Ratatui + Crossterm:** terminal layout, widgets, redraws and cross-platform terminal events.
- **Rustyline:** line editing, including prefilled commit-message presets and ordinary prompts. Commit message line breaks are entered as literal `\n`.
- **clap:** CLI parsing, generated help and version output.
- **duct:** shared captured subprocess execution for `git`, `gh` and `gpg`; the live-output Git path remains streamed so progress output and GPG/network diagnostics stay visible.
- **etcetera:** OS-appropriate config/data directory resolution without the `option-ext` MPL-2.0 dependency.
- **Serde / serde_json:** typed config serialization and machine-readable `gh` JSON responses. Invalid config JSON is reported instead of silently replaced by defaults.
- **rust-i18n:** compile-time localization with the existing per-language JSON files retained for Crowdin; runtime language selection and English fallback use the library.
- **walkdir:** cross-platform recursive Release staging-folder traversal, without following symbolic links.
- **anyhow:** top-level error context.

The app-specific code remains responsible for workflows that are unique to ghcap: account/preset management, repository association, menu navigation, and Commit/Push sequencing.

## Features

- Multiple GitHub accounts, account labels, selection, add/edit/delete/reorder.
- Repository listing and cloning through `gh`, using JSON output rather than parsing human-readable tables.
- GitHub account discovery uses `gh auth status --json hosts` rather than parsing human-readable status lines.
- Local Git repository detection, initialization, origin and identity setup.
- Commit presets and repository-specific pre-push hooks.
- Commit, Push, and Commit & Push through `git`.
- GitHub Release creation from a staging folder: reads `release-notes.md`, recursively uploads all other regular files, and supports Release / Pre-release / Draft. Existing release uploads never use `--clobber`.
- One-second `Running... Please wait.` heartbeat during silent live Git commands.
- GPG key listing/generation/registration and recovery after signing failures.
- Japanese, English, Korean, and Simplified Chinese translations.
- Stop commands: `ghcap commitstop`, `ghcap pushstop`, `ghcap commitpushstop`.
- Atomic config writes and preservation of malformed config files for manual recovery.

## Build

Requires Rust 1.85 or newer, plus `git`, GitHub CLI (`gh`), and optionally `gpg` for signing-key operations.

```sh
cargo generate-lockfile
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo build --release --locked
```

On Debian, use `packaging/build-deb.sh` to build a `.deb` package.

## Cross-platform scope

The Rust/TUI/config layers target Linux, Windows, and macOS. Real GitHub workflows also require compatible versions of `git` and `gh` on the host. CI builds on all three OS families are still required before claiming release-level support; the source architecture alone does not prove every OS has passed end-to-end tests.

## License

BSD-2-Clause. Third-party crates retain their own licenses. The newly added direct dependencies `rust-i18n` (MIT) and `walkdir` (MIT or Unlicense) do not use GPL/MPL licensing. The complete transitive dependency graph still needs a Cargo-based license audit before redistribution.


See [RELEASE_WORKFLOW_PLAN.md](RELEASE_WORKFLOW_PLAN.md) for the planned build/export-to-GitHub-Release workflow.
