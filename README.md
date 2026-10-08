# ghcap 0.13.0

ghcap is a terminal UI for practical GitHub commit/push workflows.

## V0.10 focus

V0.10 keeps the stable crossterm renderer and moves GitHub authentication/repository operations behind the official `gh` CLI. It also adds repository cloning and keeps the local-repository auto-detection flow.

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
sudo apt install /tmp/ghcap-debs/ghcap_0.13.0-1_amd64.deb
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

This is **V0.10**, not V1.0. Any correction to this release must become **V0.10** rather than silently replacing V0.10.

## V0.11 changes

- Repository Clone is available **inside each repository menu**, not as a global Home feature.
- GitHub login uses `gh auth login --web`, while always displaying `https://github.com/login/device` before authentication starts.
- If the browser does not open, the displayed URL can be opened manually.
- Processing screens such as login, Clone, commit, push, Commit & Push, and GPG operations use CUI instead of TUI dialogs.
- Operation failures display an exit code when one is available.
- Normal repository/account navigation remains TUI.
