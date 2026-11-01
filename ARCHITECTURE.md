# Architecture / アーキテクチャ

## English

UI responsibilities are split into independent `src/app/*.rs` Rust modules (`mod`, not textual `include!`). `src/repository/` inspects HEAD and tracking state and plans first Push. `src/workflows/setup.rs` provides the opt-in CUI setup wizard. `src/process/` retains bounded output, timeout, signal handling, and safe diagnostics from the strict source branch. `src/storage.rs`, `src/git.rs` and `src/release.rs` preserve historically important guards; further division of those files into smaller modules is planned and should not remove guards. `src/dependencies/` diagnoses compatible system or managed GitHub CLI binaries, but does not currently perform automatic installation.

The highest-risk boundaries are remote destination selection, commit staging, local configuration persistence, subprocess output and Release file snapshots. Regression tests should cover each boundary.

## 日本語

UIは独立したRustモジュール`src/app/*.rs`に分割しています（単純な`include!`連結は使用しません）。`src/repository/`がHEAD・追跡状態を調査して初回Pushを判断し、`src/workflows/setup.rs`が承認式のCUIセットアップを提供します。`src/process/`には厳格版の出力制限・タイムアウト・シグナル処理・安全な診断を維持しています。`src/storage.rs`、`src/git.rs`、`src/release.rs`は過去の重要な検証を保持しており、今後の詳細モジュール分割でも保護を削除しません。`src/dependencies/`はシステム版・専用版ghの互換性を診断しますが、自動インストールは未実装です。

安全性上、特に重要なのはPush先の決定、CommitのStage、設定保存、外部出力、Releaseファイルのスナップショットです。それぞれに回帰テストを設けるべきです。
