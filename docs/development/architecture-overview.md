# Architecture / アーキテクチャ

## English

ghcap consists of terminal presentation, Git repository state inspection, guided workflows, command adapters, and a bounded process runner. The process runner is the only normal route for Git and GitHub CLI subprocess execution. Repository inspection must distinguish absent configuration from a failed command; the latter is never silently converted into an empty value. Guided setup stages only approved files, reviews the complete Git index, and does not overwrite remotes or published history.

The GitHub CLI resolver checks the executable's version and necessary capabilities before selecting an absolute path. The runner may not silently fall back to an incompatible program. Long-running Git operations must preserve cancellation, timeout, and original error output while filtering secrets and terminal escape sequences.

Publisher scripts run in native CI environments. Each artifact must have a verified checksum and source commit, and published release assets must not be replaced automatically. Platform-specific package generators are isolated from the release publication step.

## 日本語

ghcapはターミナル表示、Git状態取得、案内型ワークフロー、各CLIアダプター、出力制限付きプロセス実行基盤から成ります。通常のGit・GitHub CLI実行は共通ランナーへ集約します。Git設定の不存在と外部コマンドの失敗を区別し、失敗を空値へ握りつぶしてはいけません。初回セットアップは承認したファイルだけをStageし、Git Index全体を確認します。既存remoteや公開済み履歴を勝手に上書きしません。

GitHub CLIの解決処理は版と必要機能を検査し、絶対パスを確定します。未対応版への暗黙のフォールバックは禁止します。長時間のGit操作では中断、タイムアウト、元のエラー出力を保持し、秘密情報と端末制御文字のみ安全な表示へ変換します。

配布スクリプトは対象OSのネイティブCIで動作し、各成果物のSHA-256とビルド元コミットを確認します。公開済みAssetの無断差し替えは禁止します。OS別パッケージ生成とGitHub Release公開を分離します。
