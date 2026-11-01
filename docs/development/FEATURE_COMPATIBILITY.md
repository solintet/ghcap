# Feature compatibility / 機能対応表

## English

This table maps existing user workflows to the revised source architecture. It describes intended compatibility, **not** independent confirmation that every workflow has passed native execution testing.

| User workflow | Implementation | Migration note |
|---|---|---|
| TUI navigation / menus | `src/app/`, `src/ui.rs` | Preserved existing keyboard navigation and presets |
| GitHub accounts / login | `src/github.rs`, `src/app/accounts.rs` | Resolve the compatible `gh` path before interactive login; preserve saved labels |
| Repository setup | `src/workflows/setup.rs`, `src/workflows/staging.rs` | Added explicit initial staging selection; do not rewrite existing remote |
| Commit / Commit & Push | `src/actions.rs`, `src/git.rs` | Retains staged review fingerprint and `ShellPolicy` |
| First / normal Push | `src/repository/`, `src/git.rs` | First Push is guided; normal Push retains destination checks |
| GitHub Releases | `src/release.rs`, `src/app/release_menu.rs` | Preserves asset snapshot checks and duplicate protection |
| GPG / signing | `src/gpg.rs`, `src/app/signing_menu.rs` | Keeps existing optional signing functionality |
| Presets / configuration | `src/storage.rs`, `src/models.rs` | Preserves JSON migration and existing labels |
| CUI output / cancellation | `src/process/`, `src/ui.rs` | Preserves exit conditions, timeouts, cancellation and safe diagnostic output |
| Dependency diagnostics | `src/dependencies/` | `ghcap doctor` checks compatibility without modifying the OS |
| Package generation | `packaging/` | Bash/PowerShell replaces Python scripts; native verification still required |
| CI and publication | `.github/workflows/build-release.yml` | Four native platforms and draft-first publication |

## 日本語

この表は従来の利用者向け機能と、改訂版の実装箇所を対応付けたものです。**全機能の実機動作が確認済みという意味ではありません。**

| 機能 | 実装箇所 | 移行方針 |
|---|---|---|
| TUIメニュー | `src/app/`, `src/ui.rs` | 従来のキー操作とプリセットを維持 |
| GitHubアカウント・ログイン | `src/github.rs`, `src/app/accounts.rs` | 互換版ghを検査し、保存済みラベルを保護 |
| 初回セットアップ | `src/workflows/setup.rs`, `src/workflows/staging.rs` | Stage対象の選択を追加し、既存remoteは変更しない |
| Commit / Commit & Push | `src/actions.rs`, `src/git.rs` | 変更内容の再検証と`ShellPolicy`を保持 |
| 初回・通常Push | `src/repository/`, `src/git.rs` | 初回Pushは案内、通常Pushは送信先検査を保持 |
| GitHub Release | `src/release.rs`, `src/app/release_menu.rs` | Assetスナップショットと重複保護を維持 |
| GPG署名 | `src/gpg.rs`, `src/app/signing_menu.rs` | 必要時のみ利用する署名機能を維持 |
| プリセット・設定 | `src/storage.rs`, `src/models.rs` | JSONの移行とユーザーデータを保護 |
| CUI出力・中断 | `src/process/`, `src/ui.rs` | 元エラー・タイムアウト・中断の管理を維持 |
| 依存関係診断 | `src/dependencies/` | `ghcap doctor`で互換性を検査、OSは変更しない |
| 配布物生成 | `packaging/` | PythonからBash/PowerShellへ移行。ネイティブ検証が必要 |
| CI・自動公開 | `.github/workflows/build-release.yml` | 4環境とDraft先行公開方式を維持 |
