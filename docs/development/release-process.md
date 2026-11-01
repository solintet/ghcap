# Release process / リリース運用

## English

1. Test the source locally and commit it on `master`. Every Push to `master` starts the native build matrix and, only if all builds succeed, publishes an immutable-by-policy preview tag `v0.0.31.0-pre.<sha>`.
2. Verify checksums, executables, README and reported operating-system limitations. Do not equate CI success with a full independent audit.
3. Once approved, create and Push the exact version tag `v0.0.31.0` on the verified Commit. The tag workflow packages the exact revision and publishes a Stable Release only after all checks pass.
4. The publisher refuses to move existing tags, replace different Assets, or change existing published releases. Use a new version/tag for corrections.

The publisher expects `GITHUB_SHA`, `GITHUB_REPOSITORY`, `GITHUB_REF`, `GITHUB_RUN_ID`, `GH_TOKEN` and `GHCAP_RELEASE_MODE` from the Actions job.

## 日本語

1. ローカルでソースを検証し、`master`にコミットします。`master`へのPushでネイティブビルドが実行され、全件成功時のみ`v0.0.31.0-pre.<sha>`の開発版を公開します。
2. チェックサム、実行ファイル、README、OS別の制限を確認します。CI成功だけを総合監査の完了とみなしません。
3. 正式公開を承認したら、検証済みCommitに正確なタグ`v0.0.31.0`を作成・Pushします。全チェック成功後にStable Releaseを公開します。
4. 公開スクリプトは既存タグ移動、異なるAssetsの上書き、公開済みReleaseの変更を拒否します。修正版は新しいバージョン・タグで公開してください。

Actionsから`GITHUB_SHA`、`GITHUB_REPOSITORY`、`GITHUB_REF`、`GITHUB_RUN_ID`、`GH_TOKEN`、`GHCAP_RELEASE_MODE`を受け取ります。

## 2026-11-01: independent publish + recovery / 公開のみ再実行

Build and Release are separate Actions workflows. `Build and verify ghcap` creates native artifacts for Linux x86_64, Windows x64, macOS Intel and macOS Apple Silicon. `Publish ghcap from verified build` downloads these artifacts using their original run ID, checks the source commit and per-asset hashes, creates or resumes a Draft via its numeric GitHub release ID and publishes it without rebuilding.

To retry an existing build, manually dispatch `Publish ghcap from verified build` and specify its original `build_run_id` and `release_mode`. For a Draft that is not visible in the Release listing, optionally provide `existing_release_id`. Original monolithic build Runs that ended in release-publication failure can be recovered only if **all four Build jobs succeeded**. See [`RELEASE_WORKFLOW_PLAN.md`](../../RELEASE_WORKFLOW_PLAN.md) for full instructions.
