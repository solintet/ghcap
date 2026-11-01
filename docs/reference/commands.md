# Command reference / コマンド一覧

| Command | English | 日本語 |
|---|---|---|
| `ghcap` | Open the interactive TUI | TUIを起動 |
| `ghcap --version` | Show release version | バージョンを表示 |
| `ghcap doctor` | Inspect Git, gh, GPG availability | Git・gh・GPGを診断 |
| `ghcap setup` | Guided repository initialization | リポジトリの初回設定 |
| `ghcap commitstop` | Request Commit cancellation | Commitの中止を要求 |
| `ghcap pushstop` | Request Push cancellation | Pushの中止を要求 |
| `ghcap commitpushstop` | Request combined-operation cancellation | Commit & Pushの中止を要求 |

## English

Cancellation is a request, not proof that an external operation did not complete. Check the repository and remote status after interruption. `ghcap setup` may create a remote repository only after explicit approval.

## 日本語

中止コマンドは停止の要求であり、外部操作が完了していないことの証明にはなりません。中断後はローカルとリモートの状態を確認してください。`ghcap setup`によるGitHubリポジトリ作成には明示的な承認が必要です。
