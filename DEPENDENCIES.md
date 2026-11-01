# Dependencies / 依存関係

## English

The `ghcap doctor` command examines Git, GitHub CLI and optional GnuPG. For GitHub actions, the CLI must provide `gh auth status --json`; this integration requires gh 2.98.0+ or verified equivalent fixes. The Debian system package may be older; follow GitHub CLI's official package installation instructions. `GHCAP_GH_PATH` is an optional override for a separately installed compatible `gh`. ghcap does not download or modify packages automatically in this source revision.

## 日本語

`ghcap doctor`はGit、GitHub CLI、任意のGnuPGを診断します。GitHub機能には`gh auth status --json`対応が必要で、この統合版ではgh 2.98.0以降または同等の修正版を要求します。Debian標準パッケージが古い場合は公式GitHub CLIパッケージの導入手順を利用してください。`GHCAP_GH_PATH`で互換版`gh`を別途指定できます。このソース版はパッケージの自動ダウンロードやシステムの変更を行いません。
