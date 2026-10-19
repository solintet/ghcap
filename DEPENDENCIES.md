# Dependencies — v0.0.29.0

Git、認証JSON機能に対応するGitHub CLI、GnuPGが必要です。起動前に終了コードを含むversion検査とghの能力検査を行い、不足時は端末をraw modeへ変更する前に停止します。インストール・OS自体の更新は自動で行いません。

macOSではHomebrew等で `git gh gnupg` を導入し、`gh auth login` で認証してください。GitHub CLIの更新時は公式のセキュリティ告知とOS側の更新情報を確認してください。能力検査はすべてのセキュリティ修正の保証ではありません。

Rust依存はCargo.tomlに固定し、Cargo.lockを同梱します。anyhowは1.0.104、Ratatuiは0.30.2、解決済みlruは0.18.5です。旧版の対象依存を更新しました。すべての脆弱性が存在しないことの証明ではありません。Cargo.lockの変更時にはテストとアドバイザリ再検査を行ってください。

外部プロセスはstd::process、プロセスグループ・no-followファイル操作にはUnix上でnix、終了通知にsignal-hookを使用します。UIはRatatui/Crossterm、入力はRustyline、保存はSerdeとtempfileです。設定fsyncを維持し、不要な保存を省略します。

第三者ライセンスの宣言一覧と収集した本文はTHIRD_PARTY_NOTICES.mdおよびTHIRD_PARTY_LICENSES/にあります。対象外OS・ビルド依存も含む保守的な一覧で、未収集項目は一覧末尾に明記しています。正式再配布前に未収集本文の確認が必要です。
