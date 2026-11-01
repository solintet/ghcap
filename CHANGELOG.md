# Changelog / 変更履歴

## v0.0.31.0 — Integration candidate / 統合候補

### English

- Combined the strict Git operation and release safeguards from the v0.0.29.0 source with the guided setup and dependency diagnostics from v0.0.30.0.
- Split the legacy UI source into responsibility-oriented Rust modules rather than textual `include!` fragments.
- Added a distinct first-push path and generalized approved upstream remote validation.
- Added a four-platform GitHub Actions workflow with automated preview and tag-controlled stable publication logic.
- Updated user documentation and release text in both Japanese and English.

This is a source integration candidate; Rust compilation, native package builds and a real GitHub publication must pass before it can be described as a verified release.

### 日本語

- v0.0.29.0のGit操作・Release安全対策と、v0.0.30.0の初回セットアップ・依存関係診断を統合。
- 従来のUIコードを`include!`ではなく、責任別のRustモジュールへ分離。
- 初回Pushの経路を追加し、承認済みupstreamのremote検証を一般化。
- 4環境のGitHub Actionsと、自動開発版公開・正式タグからのリリース処理を追加。
- READMEとRelease文書を日英併記の利用者向けへ更新。

本版は統合ソース候補です。Rustコンパイル、各OSのパッケージ生成、GitHubの実際の公開処理を検証するまでは正式な動作確認済みリリースとは表記しません。

### Packaging and setup / 配布・初回設定

- Replaced Python packaging and GitHub Release publisher scripts with Bash and PowerShell implementations. / Python製の配布・Release公開処理をBash・PowerShellへ変更。
- Added explicit file selection before the initial commit and full staged-index review. / 初回コミットのファイル選択とIndex全体の確認を追加。
- Rejected incompatible GitHub CLI paths instead of silently returning to an unverified executable. / 不適合なghへの暗黙のフォールバックを禁止。
- Standardized the publisher attribution as solintet; preserved existing contributor notices. / 開発・公開名義をsolintetに統一し、既存の権利表示を維持。
