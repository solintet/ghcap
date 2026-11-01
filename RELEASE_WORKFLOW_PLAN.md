# ghcap release workflow / 公開工程（Build・Publish分離版）

## 日本語

- **Build and verify ghcap** (`.github/workflows/build-release.yml`): Push/PR/手動実行で4環境のビルド・テスト・梱包を行い、Artifacts（90日保持）を保存する。ここではGitHub Releaseを操作しない。
- **Publish ghcap from verified build** (`.github/workflows/publish-release.yml`): `workflow_run` で成功した同一リポジトリの`master`ビルドに対してPreviewを公開する。タグ経由のStable公開は条件が成立する場合に限り自動実行する。実行元がPR・fork・失敗の場合は自動公開しない。
- **手動で公開のみ再実行**: GitHub Actionsの「Publish ghcap from verified build」→「Run workflow」を開き、`build_run_id`に **元のビルドのRun ID**、`release_mode`に`preview`または`stable`を入力する。ビルド工程は動かない。
- **旧一体型ワークフローからの復旧**: `41295147663`などの古いRunはReleaseで失敗していても4つの`Build <platform>`ジョブがすべて成功しArtifactが残っている場合に限り手動再利用可能。旧Runの全体結論`failure`を無条件に許可するわけではない。
- **既存Draft再開**: GitHubのRelease一覧に古いDraftが表示されない場合、必要に応じて`existing_release_id`にその**数値ID**を指定する。IDはGitHubのDraft編集ページのURLなどから確認する。IDを使ってタグ名・Draft状態・既存AssetのSHA256を検証し、同一のものだけ再利用する。
- **Safety**: 対象RunのGitHub API上の出所とSHAを検査し、各OSのArchiveに埋め込まれたSource CommitとSHA256を再照合する。ソースArchiveはそのビルドSHAから生成する。公開済みReleaseの内容は書き換えない。

### 旧Runの復旧例

1. 更新した`publish-release.yml`、`packaging/publish-release.sh`、`packaging/resolve-build-run.sh`等を`master`に反映する。
2. Actions → **Publish ghcap from verified build** → **Run workflow**を選ぶ。
3. `build_run_id=41295147663`、`release_mode=preview`、`existing_release_id=`（空欄）で開始する。まずAPIで旧Runの4つのビルドジョブを検証し、残っているArtifactsを取得する。
4. 既存Draftと衝突する場合は、そのDraftの数値Release IDを指定して公開処理だけ再実行する。見つからないDraftのIDを推測しない。

**注意**: Artifactsの保存期限が過ぎた場合、ビルド成果物は再利用できない。このときだけ再ビルドが必要。

## English

The build workflow creates four verified artifacts and never touches Releases. The publisher is a separate `workflow_run`/`workflow_dispatch` workflow which downloads by the **exact source run ID**. An administrator can retry publishing without rebuilding. The legacy recovery path permits a failed former monolithic run only if all four named build jobs succeeded and the provenance checks pass. Newly created Draft IDs are taken from the REST creation response and used for all subsequent API calls; existing Draft IDs can be supplied manually for recovery. Published Releases and mismatched assets are protected from overwrite.

### Time and cache expectations

A previous run's stored artifacts enable publication-only retries without any Rust compilation. `Swatinem/rust-cache` improves dependency reuse on future builds but does **not** make changed-source builds instantaneous: quality gates, final linkage, packaging, and native verification still execute. Each matrix OS/architecture has an independent cache; GitHub-hosted runners are ephemeral. `cargo check` is not needed as a separate gate when the per-platform `cargo clippy --all-targets` and `cargo test --all-targets` jobs already compile all targets.
