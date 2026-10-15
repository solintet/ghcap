# ghcap 0.0.29.8

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

On Debian/Ubuntu, `./packaging/build-linux.sh` generates Linux packages. `./packaging/build-all.sh` extends this to a Windows x64 `.exe`, winget manifests, and a complete SHA-256 inventory. macOS source and CI are integrated in this repository, but macOS builds run only when manually requested through GitHub Actions. Intel and Apple Silicon artifacts are separate; no Universal Binary is generated. Install the build prerequisites listed in `packaging/README.md`. The Arch package is built inside an Arch Linux container, so Debian remains the host OS; Docker or Podman and network access are required for that step. The script runs the Rust release checks before creating any packages. A missing `Cargo.lock` is generated before validation; network access may be needed for the first dependency resolution.

## Cross-platform scope

The macOS CI workflow at `.github/workflows/macos-ci.yml` runs on pushes, pull requests, and manual dispatch. It builds, tests, and packages separate Intel and Apple Silicon binaries on native macOS runners, then uploads the archives and SHA-256 checksums as workflow artifacts. Linux and Windows workflows are not included in this macOS-only CI configuration. Real GitHub workflows also require compatible versions of `git` and `gh` on the host. CI builds do not replace end-to-end testing against real accounts and repositories.

## License

BSD-2-Clause. Third-party crates retain their own licenses. The newly added direct dependencies `rust-i18n` (MIT) and `walkdir` (MIT or Unlicense) do not use GPL/MPL licensing. The complete transitive dependency graph still needs a Cargo-based license audit before redistribution.


See [RELEASE_WORKFLOW_PLAN.md](RELEASE_WORKFLOW_PLAN.md) for the planned build/export-to-GitHub-Release workflow.

> Version note: the public release tag and displayed application version are `v0.0.29.8`. Cargo package metadata uses `0.29.8` because Cargo requires a three-component SemVer package version; the CLI explicitly displays the requested four-component release version.

## Required external tools

The normal ghcap TUI requires **Git** (`git`), **GitHub CLI** (`gh`), and **GnuPG** (`gpg`). At startup, ghcap checks that each command can run with `--version`. If a tool is missing or cannot start, ghcap exits before switching the terminal to raw mode and prints the missing command(s) plus localized installation guidance for the detected operating system or Linux distribution.

- Windows: install Git, GitHub CLI, and Gpg4win.
- macOS: download the matching Intel or Apple Silicon artifact from the manually triggered GitHub Actions workflow; runtime dependencies are Git, GitHub CLI, and GnuPG.
- Debian/Ubuntu: `sudo apt update && sudo apt install git gh gnupg`
- Fedora/RHEL family: `sudo dnf install git gh gnupg2`
- Arch/Manjaro: `sudo pacman -S git github-cli gnupg`

The `commitstop`, `pushstop`, and `commitpushstop` control commands do not require these tools to be available, so they can still signal an already-running ghcap instance.
