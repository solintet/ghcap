# Repository setup / リポジトリ初回設定

## English

Start in the project folder and execute `ghcap setup`. The wizard detects existing Git metadata, offers local initialization if necessary, asks for repository-local identity when missing, and can create a GitHub repository via `gh repo create`. The default visibility choice is Private. Review staged changes and entire history before publishing. Existing remote configuration must not be overwritten automatically. Remote connectivity failures are not equivalent to an empty remote. After a successful initial Push, check upstream status using `git branch -vv`.

The wizard is interactive; it does not silently commit or publish. It currently uses CUI prompts for parts of guided setup. Some advanced scenarios, including unusual remotes or unrelated histories, require manual resolution rather than forced Push.

## 日本語

対象プロジェクトのフォルダで`ghcap setup`を実行します。Git管理情報の有無を検査し、必要ならローカル初期化、リポジトリ内に限定した名前・メール設定、`gh repo create`によるGitHubリポジトリ作成を案内します。公開範囲の初期選択はPrivateです。公開前にStage内容と履歴全体を確認してください。既存のremote設定を自動上書きせず、通信失敗を「リモートが空」と誤認しません。初回Push後は`git branch -vv`で追跡先を確認できます。

セットアップは対話式であり、勝手なCommitや公開は行いません。案内の一部は現在CUI入力です。特殊なremote、履歴が無関係なリポジトリなどでは、Force Pushせず個別の確認・修正が必要です。

### File selection / ファイル選択

When creating the initial commit, ghcap lists candidate files by number. Enter the numbers separated by spaces, `all` to stage all listed candidates, or `none` to keep the current staging index unchanged. Before committing, it displays all staged changes, including files staged before running ghcap. A cancelled operation does not silently reset the Git index.

初回コミットでは候補ファイルを番号付きで表示します。番号を空白区切りで入力するか、`all`で候補全件、`none`で新しいStageなしを選択します。コミット前には以前からStageされていたファイルも含め、Index全体の変更を表示します。キャンセル時にIndexを勝手にリセットしません。
