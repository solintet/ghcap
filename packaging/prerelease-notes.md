# ghcap v0.0.29.0 — Pre-release

安全性改善候補版です。正式Stableではありません。

## 配布物

- Windows x64: exe / ライセンス・説明書を含むportable zip
- Linux x86_64: tar.gz / deb / rpm / pacman (.pkg.tar.zst) / AppImage
- macOS: Intel / Apple Silicon の個別tar.gz
- 同じCommitのソースと、全配布物のSHA-256

LinuxはUbuntu 22.04上でビルドし、glibc 2.35以上を必要とします。AppImageをFUSEなしで動かす場合は `APPIMAGE_EXTRACT_AND_RUN=1 ./ファイル名.AppImage` を使用してください。deb/rpm/pacmanは各ディストリビューション用で、対応外のOSへ無理に導入しないでください。

## 改善・注意点

危険なPush設定の拒否、競合の保護、Commit前後の確認、Skip時のシェル実行抑止、設定の排他とfsync、子プロセスの上限・停止、端末復元等を改善しています。全監査項目の解消や脆弱性ゼロを保証する版ではありません。

Git・認証JSONに対応する更新済みGitHub CLI・GnuPGを別途インストールしてください。Windows版のアプリ内フォルダ→Release公開は、安全なno-follow対応が検証されるまで停止しています。今回のPre-release公開はGitHub Actionsが行います。

Developer ID署名・公証、Windowsコード署名はしていません。最低OS、実認証・署名・IME、全パッケージ管理ツールでの導入・削除、ライセンス資料の最終確認など、追加検証が残っています。過去タグや既存成果物は上書きしません。同じ版の別Commitには `-pre.<Commitの先頭12桁>` のタグを使用します。
