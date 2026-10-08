# ghcap 0.22.0

ghcap is a terminal UI for practical GitHub commit/push workflows.

## V0.22 focus

V0.22 keeps the stable crossterm renderer and moves GitHub authentication/repository operations behind the official `gh` CLI. It also adds repository cloning and keeps the local-repository auto-detection flow.

### Dependency design

The TUI renderer now uses `crossterm` directly. Ratatui was removed because its dependency chain introduced `instability`/`darling` version resolution that was incompatible with the project's Rust 1.85 baseline. ghcap does not need a widget framework to render its relatively small menu system.

The build script deliberately does **not** run `cargo update -p ...` for transitive crates. If `Cargo.lock` exists, it is honored with `cargo build --locked`; otherwise Cargo generates it once for the source tree and the build immediately uses that lockfile.

## Features

- Modular Rust source rather than a monolithic `main.rs`.
- Bounded crossterm TUI layout with scrolling selection.
- Four JSON locales: English, Japanese, Simplified Chinese and Korean.
- Runtime language switching with English fallback for missing keys.
- GitHub authentication through the official `gh` CLI browser/device login flow.
- Authenticated GitHub account detection without storing a Personal Access Token.
- GitHub repository listing and selection through `gh`.
- Repository cloning through `gh repo clone`.
- Current local Git repository detection and GitHub remote parsing.
- Real commit, push and commit & push operations through `git`.
- Commit presets: add, edit, reorder, delete and repository selection.
- Repository-specific pre-push shell configuration.
- Stop commands: `commitstop`, `pushstop`, `commitpushstop`.
- GPG key listing, generation and GitHub registration.
- Atomic JSON configuration writes.

## Build on Debian

The project declares Rust 1.85 as its baseline.

```sh
chmod +x packaging/build-deb.sh
./packaging/build-deb.sh

# The script also copies the generated packages to /tmp/ghcap-debs.
sudo apt install /tmp/ghcap-debs/ghcap_0.22.0-1_amd64.deb
```

The resulting `.deb` is written to the parent directory of the source tree by `dpkg-buildpackage`.

## Runtime

```sh
ghcap
```

Stop commands can be sent from another terminal while ghcap is running:

```sh
ghcap commitstop
ghcap pushstop
ghcap commitpushstop
```

## Localization

Translations are JSON files under `locales/` and are installed under `/usr/share/ghcap/locales/`. Built-in copies are compiled into the binary, so the application still has translations if the installed locale directory is unavailable. User overrides can be placed under the ghcap configuration directory.

## Version policy

This is **V0.22**, not V1.0.


## License

ghcap is released under the BSD-2-Clause License. See `LICENSE`. Third-party dependencies retain their respective licenses. The release build checks Cargo dependency license metadata and rejects forbidden copyleft/non-commercial licenses.

## V0.22 changes

- Account selection returns to the V0.13-style direct Enter-to-open behavior.
- Accounts and presets support TUI key operations: `a` add, `e` edit, `d` delete, `Shift+↑/↓` reorder.
- Accounts can have a local display label without changing the GitHub login name.
- Each repository can store its local Git folder.
- If the configured folder has no `.git`, ghcap initializes it automatically.
- The repository GitHub `origin` remote is configured automatically during Git setup.
- Commit, Push and Commit & Push automatically enter Git setup when the repository has not been prepared, then return to the requested operation.
- Clone records the cloned local folder as the repository's Git folder.
- GPG signing failures can enter GPG key registration/setup from the failed operation. After setup, ghcap asks whether to retry Push.
- Release package/version references are synchronized to 0.22.0.
- Commit starts directly from the preset list; there is no redundant message-selection screen.
- Selecting a preset opens a direct CUI editor with the preset text already inserted.
- Shift+Enter inserts a newline; Enter opens Commit confirmation.
- Commit confirmation supports Yes, No (return to repository screen), and Edit (return to the editor).
- Esc returns exactly one screen at a time throughout the Commit workflow.
- GitHub account discovery uses normal `gh auth status` output rather than requiring `--json`.
