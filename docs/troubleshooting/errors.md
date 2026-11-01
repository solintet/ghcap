# Troubleshooting / エラー対応

## English

- `fatal: Needed a single revision`: A requested Git revision may be missing (for example, an unconfigured upstream). Use `git status -sb`, `git branch -vv`, `git rev-parse --verify HEAD`, and `ghcap setup` as appropriate. Do not use Force Push as a general fix.
- `gh auth status --json` unavailable: Your GitHub CLI is too old for the required feature. Use `gh --version`, `gh auth status --help`, and the official GitHub CLI installation guide.
- Push rejected: Inspect the actual Git stderr and the remote branch history. Do not reset hard or overwrite the remote merely to eliminate the error.
- Broken TUI: On Unix, `stty sane` can restore normal terminal modes after a forced interruption.

When reporting a bug, provide the ghcap version, OS, the failing command and a sanitized error. Never paste tokens, keys or passwords.

## 日本語

- `fatal: Needed a single revision`：指定したGit参照が存在しない場合があります（upstream未設定など）。`git status -sb`、`git branch -vv`、`git rev-parse --verify HEAD`を確認し、必要なら`ghcap setup`を利用してください。Force Pushでの応急処置は推奨しません。
- `gh auth status --json`非対応：GitHub CLIが古い可能性があります。`gh --version`と`gh auth status --help`を確認し、公式配布元から更新してください。
- Pushが拒否された：Git本来のstderrとリモート履歴を確認してください。エラー回避のためのHard Reset・強制上書きは避けてください。
- TUIが崩れた：Unixでは強制終了後に`stty sane`で端末設定を復元できる場合があります。

不具合を報告する際はghcapのバージョン、OS、失敗したコマンド、秘密情報を除去したエラーを記載してください。トークンや秘密鍵、パスワードは共有しないでください。
