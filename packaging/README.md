# Packaging / 配布物の生成

## English

ghcap is published by **solintet**. Four native GitHub Actions jobs build Linux x86_64, Windows x64, macOS Intel and macOS Apple Silicon. `cargo fmt`, `cargo check`, `cargo clippy` and `cargo test` must all succeed before native packaging begins.

Packaging is implemented using Bash, PowerShell, the operating system's native package tools, and GitHub CLI. **Python is not required.** Linux generates tar.gz, deb, RPM, Pacman and AppImage; Windows generates exe and ZIP; macOS generates tar.gz for each architecture. Release publication validates the exact source commit and every individual checksum before creating or uploading a GitHub Release.

`packaging/package-linux-ci.sh` packages an already-built native Linux binary; `packaging/package-windows-ci.ps1` packages an already-built native Windows binary. `packaging/build-macos.sh` builds the native macOS archive. `packaging/validate-release-assets.sh` and `tests/publisher-offline.sh` work without GitHub authentication. `packaging/publish-release.sh` publishes only after validation and requires GitHub Actions environment variables.

Developer-specific legacy packaging paths remain available; use the native Actions pipeline for official binaries. Distribution publishing requires deliberate GitHub authorization. Packages must not be described as tested solely because the scripts exist.

## 日本語

ghcap の開発・公開元は **solintet** です。GitHub Actions は Linux x86_64、Windows x64、macOS Intel、macOS Apple Silicon の4環境でビルドします。パッケージ化の前に Rust の整形・静的解析・Clippy・テストをすべて成功させます。

配布処理には Bash、PowerShell、各OSの標準パッケージツール、GitHub CLI を使用します。**Python は不要です。** Linux は tar.gz・deb・RPM・Pacman・AppImage、Windows は exe・ZIP、macOS はアーキテクチャ別 tar.gz を生成します。GitHub へ公開する前に、各成果物のチェックサムとビルド元コミットを照合します。

`packaging/package-linux-ci.sh` と `packaging/package-windows-ci.ps1` は検証済みのネイティブバイナリを梱包します。macOS は `packaging/build-macos.sh` を使用します。`packaging/validate-release-assets.sh` と `tests/publisher-offline.sh` は GitHub 認証なしで実行できます。`packaging/publish-release.sh` は検証済み成果物と GitHub Actions の環境変数が必要です。

従来の開発者向け配布経路は引き続き利用できますが、公式バイナリにはネイティブ Actions の検査経路を使用します。公開には明示的な GitHub 認可が必要です。スクリプトがあるだけで検証済みとは扱いません。

## Separate publish pipeline / BuildとReleaseの分離（v0.0.31.0修正版）

- Build workflow: `.github/workflows/build-release.yml` (`Build and verify ghcap`)
- Publisher workflow: `.github/workflows/publish-release.yml` (`Publish ghcap from verified build`)
- Publish from an existing run: Actions → **Publish ghcap from verified build** → **Run workflow** → input original `build_run_id`.
- Optional Draft recovery: input `existing_release_id` for a known numeric Draft ID. No guessing / 作成済みDraftの数値IDを確認してから指定。
- The publisher validates exactly nine build artifacts and 21 Release files using SHA256 and embedded source commit provenance, creates Drafts with the REST API, and publishes by numeric ID; **no Rust build or test compilation** occurs during publication.
- A failed *legacy* all-in-one run may be used only when all four original `Build ...` matrix jobs were successful and its artifacts are still retained.
- Rust cache is an optimization, not a guarantee of a zero-second rebuild. Distinct native runners have distinct target directories and may recompile dependencies after cache expiry, Rust-version changes, or source changes.

See [`RELEASE_WORKFLOW_PLAN.md`](../RELEASE_WORKFLOW_PLAN.md) for Japanese and English recovery instructions.
