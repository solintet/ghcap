# ghcap v0.0.31.0

## English

This release combines guided repository setup with additional safeguards for Git operations, configuration storage, process execution, and GitHub Release asset handling.

### Changes

- Guided repository initialization and initial Push, alongside existing Commit and Push menus.
- Native Git CLI error output and distinct ghcap diagnostics.
- Dependency status command: `ghcap doctor`.
- Explicit Push destination checks and protection against unexpectedly replacing remotes.
- Native packaging workflows for Linux x86_64, Windows x64, macOS Intel and macOS Apple Silicon.
- Bilingual user and developer documentation.

### Requirements and limitations

Git is required for local operations, and a compatible GitHub CLI is required for GitHub operations. GPG support depends on the user's signing configuration. Dependency auto-installation is not yet complete. Review the package Assets and checksums before installing.

## 日本語

このリリースは、リポジトリ初回セットアップと、Git操作・設定保存・プロセス実行・GitHub Release資産の保護を統合します。

### 変更内容

- リポジトリ初期化と初回Pushの案内を追加。既存のCommit・Push画面も維持。
- Git本来のエラー出力とghcapによる診断を区別して表示。
- 依存関係を調べる`ghcap doctor`を追加。
- Push先の明示的な検証と、既存remoteの意図しない変更防止。
- Linux x86_64、Windows x64、macOS Intel／Apple Silicon向け配布ワークフロー。
- 日英併記の利用者向け・開発者向け文書。

### 依存関係・制限

ローカル操作にはGit、GitHub操作には対応版GitHub CLIが必要です。GPGの要否は署名設定によります。依存関係の自動導入は未完成です。導入前にAssetsとチェックサムを確認してください。

### Publisher / 開発・公開元

**solintet** — [github.com/solintet/ghcap](https://github.com/solintet/ghcap).

### Build tooling / ビルド・配布処理

The packaging and release pipeline uses Bash, PowerShell, the operating system's package tools and GitHub CLI. Release asset validation no longer depends on Python. Initial setup offers explicit file selection before staging.

配布・公開工程は Bash、PowerShell、各OSのパッケージツール、GitHub CLIを使用し、Pythonに依存しません。初回セットアップではファイルを選択してからStageします。
