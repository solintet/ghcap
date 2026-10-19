# Architecture — v0.0.29.0 candidate

- main: 起動前検査、インスタンス所有権、端末復元guard。
- app/ui: 既存メニューと確認、描画・入力。副作用の多いappの分割は未完了。
- actions: Commit結果の型、確認fingerprint、明示ShellPolicy、Push計画の照合。
- git: root・操作状態・実効Push設定検査。statusはporcelain v2を1回取得。
- process: 標準プロセスAPI、出力上限、期限、制御文字対策、Unix専用プロセスグループと停止。
- storage: schema、サイズ上限、排他・同時更新検出、atomic replaceとfsync、インスタンス単位の停止要求。
- release: 明示repo/SHA、タグ検査、Unix no-follow、私有スナップショット、引数上限に応じた分割upload。
- github/gpg/dependencies: CLI JSON・能力・終了コード・署名鍵属性の検査。

外部コマンドの実行にshell解釈を使うのは、ユーザーが設定し許可したシェル操作だけです。信頼したGit hooks・外部ツールの任意コード実行は隔離しません。設定保存とGit操作全体を単一トランザクションにはできないため、部分完了は表示し、巻戻しやforce pushを自動実行しません。

UIスレッド上の同期処理、Windows固有の停止/ACL/初回描画、実MacのIME・最小OS・公証は追加検証が必要です。
