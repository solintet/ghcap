# ghcap

**Git and GitHub workflows in your terminal / ターミナルからGit・GitHubを操作**

## English

ghcap is a Rust-based terminal application for managing common Git and GitHub workflows. Navigate repository and account menus using the keyboard, review changes before committing, and keep Git command diagnostics visible.

### Features

- Interactive TUI for repository, account, preset, signing and Release operations.
- Commit review with change detection, deliberate Push destination validation and non-force Push.
- Guided local repository setup and first Push (`ghcap setup`).
- Dependency diagnostics (`ghcap doctor`).
- CUI output for operations, including native CLI errors and separate ghcap explanations.
- Support for multiple GitHub accounts through GitHub CLI.

### Platforms

The source is intended for Windows x64, Linux x86_64, macOS Intel and macOS Apple Silicon. Availability of a verified binary depends on completion of the release workflow and platform tests. Consult the Assets attached to each published Release.

### Requirements

- Git for local version-control operations.
- A compatible GitHub CLI (`gh`) for GitHub features. ghcap checks for version 2.98.0 or newer and `gh auth status --json` support. A package-manager build may be too old.
- GnuPG only when using GPG signing or GPG key management.

`ghcap doctor` reports compatibility without changing your system. **The application does not yet automatically download or update GitHub CLI**; use the [official installation instructions](https://github.com/cli/cli/blob/trunk/docs/install_linux.md) where required.

### Build / Install

From the source tree, run `./build-install.sh build` for a checked release build, `./build-install.sh test` for quality gates, or `./build-install.sh build-install` to build and install into your own `~/.local/bin` after confirmation. Use `--offline` to prevent Cargo network access. These commands never install OS packages automatically.

### Start

```sh
ghcap doctor
ghcap setup   # Guided setup in the current working directory
ghcap         # Interactive terminal menus
```

For GitHub actions, authenticate first with `gh auth login`. For initial public repository creation, review every file and commit before selecting Public. Existing remotes and Git history must be preserved.

See [Installation](docs/installation/overview.md), [First setup](docs/usage/repository-setup.md), [Commands](docs/reference/commands.md), [Troubleshooting](docs/troubleshooting/errors.md), [Security](SECURITY.md), [Contributing](CONTRIBUTING.md) and [Release process](docs/development/release-process.md).

## 日本語

ghcapはGit・GitHubの一般的な操作をターミナルで扱うRust製アプリケーションです。キーボードによるメニュー操作、Commit前の内容確認、Push先の検証、Gitコマンドの実行結果の確認ができます。

### 主な機能

- リポジトリ・GitHubアカウント・プリセット・署名・Release操作用のTUI。
- Commit前の変更確認、Push先の事前検証、強制しないPush。
- 初回セットアップと初回Pushの案内（`ghcap setup`）。
- 依存関係の診断（`ghcap doctor`）。
- CUIへのコマンド実行結果・Git本来のエラー・ghcapの診断の表示。
- GitHub CLIを利用した複数アカウントの操作。

### 対応環境

ソースはWindows x64、Linux x86_64、macOS Intel、macOS Apple Silicon向けに設計されています。実際の配布ファイルが提供されるかどうかは、対象環境のビルド・検証結果によります。各ReleaseのAssetsを確認してください。

### 依存関係

- Git：ローカルのバージョン管理操作で使用。
- GitHub CLI (`gh`)：GitHub機能で使用。ghcapは2.98.0以上と`gh auth status --json`対応を要求します。OS標準の`gh`が古い場合があります。
- GnuPG：GPG署名・GPG鍵管理を使う場合に必要です。

`ghcap doctor`でシステム設定を変えずに診断できます。**GitHub CLIを自動ダウンロード・更新する機能は未完成です。** 必要な場合は[公式導入方法](https://github.com/cli/cli/blob/trunk/docs/install_linux.md)を参照してください。

### ビルド・インストール

ソースのルートから `./build-install.sh build` で品質検査付きビルド、`./build-install.sh test` で検査、`./build-install.sh build-install` で検査・ビルド後に確認して `~/.local/bin` へ導入できます。`--offline` で Cargo の通信を禁止できます。OS依存パッケージを無断導入しません。

### 初回起動

```sh
ghcap doctor
ghcap setup   # 現在のフォルダを初回設定
ghcap         # 対話的なTUIメニューを開く
```

GitHub操作では先に`gh auth login`で認証します。Publicでリポジトリを作る場合、公開対象のファイルと履歴を必ず確認してください。既存remoteとGit履歴は保護する設計です。

詳しくは[導入](docs/installation/overview.md)、[初回設定](docs/usage/repository-setup.md)、[コマンド](docs/reference/commands.md)、[エラー対応](docs/troubleshooting/errors.md)、[セキュリティ](SECURITY.md)、[開発参加](CONTRIBUTING.md)、[リリース手順](docs/development/release-process.md)を参照してください。

## License / ライセンス

BSD-2-Clause; see [LICENSE](LICENSE). Third-party notices and license texts are included separately. / BSD-2-Clause。詳細は[LICENSE](LICENSE)をご覧ください。第三者ライセンスは別途収録しています。

## Maintainer / 開発・公開名義

**solintet** maintains and publishes ghcap through the [solintet/ghcap](https://github.com/solintet/ghcap) repository. Existing third-party notices and contributor copyrights remain applicable.

**ghcap の開発・公開名義は solintet** です。公開リポジトリは [solintet/ghcap](https://github.com/solintet/ghcap) です。既存の貢献者・第三者の著作権およびライセンス表示は引き続き適用されます。

### Release builds and publication / ビルド・公開の分離

GitHub Actionsの4環境ビルドは`Build and verify ghcap`で、Release公開は独立した`Publish ghcap from verified build`で行います。公開が失敗しても、Actionsの手動実行画面から成功済みビルドのRun IDを指定すれば、保存済みArtifactを再検証して**公開のみ再実行**できます。旧一体型Runも4つのビルドジョブがすべて成功した場合に限り救済できます。詳しくは[`RELEASE_WORKFLOW_PLAN.md`](RELEASE_WORKFLOW_PLAN.md)を参照してください。
