# Development conventions / 開発規約

## English

The project is published under **solintet**. The executable, package and command retain the name **ghcap**. Do not attribute ghcap to MusicMenker. Historical contributor copyrights and third-party licenses are not replaced when changing product publisher metadata.

Maintain meaningful English and Japanese comments describing unusual invariants, I/O, external constraints and security decisions. Keep one responsibility per Rust module even if a module is small; avoid `include!` as a substitute for proper module boundaries. Do not silently swallow Git errors; preserve exit codes, stdout and stderr while redacting credentials and unsafe terminal escapes. Do not use Python in build, test, deployment or packaging automation.

The displayed product version is `vA.B.C.D` and may differ from Cargo's three-part version. Preserve user settings, branch names and remote URLs unless a user authorizes a change. Workflows and Release notes must be written for people unfamiliar with the development conversation. Native builds and regression tests are required for a stable release.

## 日本語

本プロジェクトの開発・公開名義は **solintet** です。実行ファイル、パッケージ、コマンド名は **ghcap** を維持します。MusicMenker名義として扱いません。製品の公開元を変更する場合でも、従来の貢献者・第三者ライセンスの権利表示は勝手に変更しません。

非自明な条件、副作用、外部制約、安全性判断を説明する日英コメントを使用します。小さなファイルでも責任が異なれば分割し、`include!` による疑似的な分割は行いません。Gitのエラーを握りつぶさず、終了コード・stdout・stderrを保持します。ただし認証情報と危険な制御文字は安全な表示へ変換します。ビルド・テスト・配布・公開にPythonを使用しません。

正式な製品表示は`vA.B.C.D`とし、3要素が必要なCargo内部版と区別します。ユーザーの設定、ブランチ、remoteは許可なく変更しません。ワークフローとRelease Notesは開発の経緯を知らない人に向けて書きます。Stableにはネイティブビルドと回帰テストが必要です。
