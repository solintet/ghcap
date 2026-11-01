# Quality gates / 品質検査

## English

Before a source change is published as a general-use release, run `./build-install.sh build`, `bash tests/publisher-offline.sh`, and all four native GitHub Actions jobs. In addition to the Rust unit and integration tests, validate native package contents, source commit provenance, authentication flows, Git repository setup, first and normal Push, cancellation, GPG functionality where available, and GitHub Release handling. Preserve existing configurations and do not modify historical release tags or assets.

A successful syntax check, static review, Cargo build, automated test suite, and real operating-system validation are different kinds of evidence. Document the completed scope separately. If any mandatory quality gate fails, do not publish a Stable Release. The absence of a test is not proof of correct operation.

## 日本語

ソース変更を一般向けに正式公開する前に、`./build-install.sh build`、`bash tests/publisher-offline.sh`、4環境のGitHub Actionsを実行します。Rustの単体・統合テストに加え、ネイティブ配布物の展開、ビルド元コミット、認証、初回セットアップ、初回・通常Push、中断、利用可能な環境でのGPG、GitHub Release操作を検査します。既存設定を保護し、過去のタグ・公開済みAssetは変更しません。

構文検査、静的レビュー、Cargoビルド、自動テスト、OS実環境の動作検証は別々の証拠です。完了した範囲を分けて記録してください。必要な検査が失敗した状態でStableを公開してはいけません。未検証は正常動作の証明にはなりません。
