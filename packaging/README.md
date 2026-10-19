# Release builds (Debian + macOS on demand)

`./packaging/build-linux.sh` creates Linux formats. `./packaging/build-all.sh` runs the Linux release build, adds a Windows x64 GNU `.exe` and portable ZIP, generates winget manifests, and writes SHA-256 checksums. Run both from Debian.

## Debian build dependencies

```sh
sudo apt update
sudo apt install build-essential gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 dpkg-dev debhelper rpm zstd curl podman llvm zip unzip
```

Install stable Rust and components: `rustup toolchain install stable && rustup default stable && rustup component add rustfmt clippy`. The Windows target is installed by the build script.

Set `GITHUB_REPOSITORY=OWNER/REPO` to the actual GitHub repository before running `./packaging/build-all.sh`. Optionally set `WINGET_PACKAGE_ID` to the actual package identifier.

```sh
export GITHUB_REPOSITORY=solintet/ghcap
./packaging/build-all.sh
```

The script needs network access for Cargo dependency resolution, AppImage tool retrieval when absent, and the Arch Linux container. Podman or Docker is required for Arch package generation.

## macOS (manual GitHub Actions workflow)

macOS builds are integrated into this same source tree but are **not** part of the default push/PR CI or Debian build. In GitHub, open **Actions → Build macOS → Run workflow** and select `intel`, `apple-silicon`, or `both`. Each architecture is built on a matching GitHub-hosted macOS runner and uploaded as a separate artifact; no Universal Binary is created.

- Intel: `ghcap-v0.0.29.0-macos-intel.tar.gz`
- Apple Silicon: `ghcap-v0.0.29.0-macos-apple-silicon.tar.gz`

The local helper `./packaging/build-macos.sh intel` or `./packaging/build-macos.sh apple-silicon` must be run on the matching Mac architecture. `./packaging/setup-macos.sh` checks/installs the local Rust prerequisites. GitHub Actions uses the same build helper and requires no macOS SDK on Debian.

Archives are unsigned and not notarized; Gatekeeper may warn users. Runtime requirements are Git, GitHub CLI (`gh`), and GnuPG (`gpg`).

## Verification boundary

Shell syntax and archive integrity can be checked on Debian, but macOS execution must be tested on macOS. A CI build alone does not imply the binary was manually end-to-end tested against GitHub.

## Arch package build in a Debian container

The Arch container installs `base-devel`, `git`, `github-cli`, and `gnupg` as root before invoking `makepkg` as an unprivileged user. It deliberately does not use `makepkg --syncdeps`: dependencies are installed in the container beforehand, avoiding an interactive `sudo` password prompt in a non-interactive build.
