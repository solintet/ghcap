# Installation / インストール

## English

Install Rust using [rustup](https://rustup.rs/) when building from source; Git is also required. In the extracted source folder, run `cargo build --release --locked` and `cargo test --locked --all-targets`. The resulting binary is `target/release/ghcap` (or `ghcap.exe` on Windows). A published Release may offer native packages for Windows x64, Linux x86_64, macOS Intel and Apple Silicon.

The GitHub CLI is required for GitHub features. On Debian, the distribution `gh` may be older than the version needed for JSON authentication status; follow the [official Linux install guide](https://github.com/cli/cli/blob/trunk/docs/install_linux.md) and verify with `gh --version` and `gh auth status --help`. GnuPG is optional unless your repository uses GPG signing.

## 日本語

ソースからビルドする場合は[rustup](https://rustup.rs/)でRustを導入し、Gitも用意してください。展開したソースフォルダで`cargo build --release --locked`と`cargo test --locked --all-targets`を実行します。実行ファイルは`target/release/ghcap`（Windowsでは`ghcap.exe`）です。公開ReleaseではWindows x64、Linux x86_64、macOS Intel／Apple Silicon向けのパッケージが提供される場合があります。

GitHubの機能にはGitHub CLIが必要です。Debianの標準`gh`はJSON認証情報表示に対応しないほど古い場合があります。[GitHub CLI公式導入手順](https://github.com/cli/cli/blob/trunk/docs/install_linux.md)で導入し、`gh --version`と`gh auth status --help`で確認してください。GPG署名を使用しない場合はGnuPGは任意です。
