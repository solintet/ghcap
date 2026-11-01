# From source to GitHub Release / ソースからGitHub Releaseまで

## English — Prerequisites

- Have write permission to `solintet/ghcap` on GitHub. This guide is for the project maintainer; running the commands does not itself grant permission.
- Install Git, a compatible GitHub CLI (gh 2.98.0+), Rust/rustup, and optionally GnuPG for GPG signing.
- Never paste or share access tokens or 2FA codes.
- Do not tag/publish until the build and test checks succeed on the integration branch and the platform builds succeed.

On Debian, `apt install gh` can install an older version. Use GitHub CLI's officially documented Debian package repository at <https://github.com/cli/cli/blob/trunk/docs/install_linux.md> and verify:

```bash
gh --version
gh auth status --help
cargo --version
git --version
```

## 日本語 — 前提条件

- GitHub の `solintet/ghcap` に書き込み権限があること。この手順は管理者向けで、コマンドを実行しても権限自体は付与されません。
- Git、対応版GitHub CLI（gh 2.98.0以上）、Rust/rustup、必要ならGnuPGを用意します。
- トークンや2段階認証コードは貼り付けたり公開しないでください。
- 統合ブランチのテストと各OSのビルドが成功する前にStableタグやReleaseを公開しないでください。

Debian の `apt install gh` では古いバージョンになる場合があります。<https://github.com/cli/cli/blob/trunk/docs/install_linux.md> にある公式APTリポジトリの導入方法を使い、上記のコマンドで確認してください。

## English — 1. Authenticate / 日本語 — 1. ログイン

```bash
gh auth login --hostname github.com --web --git-protocol https
gh auth status
gh auth setup-git
```

`gh auth login` opens the device/browser authorization flow. Authenticate using your own account. `gh auth setup-git` configures Git to use the credential helper provided by gh. / `gh auth login` はブラウザを使う認証画面を開きます。本人のアカウントでログインし、`gh auth setup-git` でGitがghの認証ヘルパーを使えるようにします。

## English — 2. Protect the current repository / 日本語 — 2. 既存リポジトリの保護

Use a separate working directory; do not remove an existing `.git` directory. / 別の作業フォルダを使用し、既存の `.git` を削除しないでください。

```bash
gh repo clone solintet/ghcap "$HOME/ghcap-v31-review"
cd "$HOME/ghcap-v31-review"
git fetch origin
git switch master
git status -sb
git remote -v
```

If `git status` is not clean or `master` is absent, stop and inspect rather than resetting history. / 変更が残っている、または`master`が存在しない場合は、履歴を初期化せず状態を確認してください。

## English — 3. Copy the candidate files / 日本語 — 3. 統合版ソースを配置

Extract `ghcap-v0.0.31.0-solintet-source.zip` somewhere separate. Copy its **contents** into the clean clone. The following is an example; change `<EXTRACTED_DIR>` to the actual extracted path. It does not delete your existing files. / ZIPを別の場所に展開します。`<EXTRACTED_DIR>` を実際の展開先へ置き換え、きれいなCloneにソース一式を重ねます。既存ファイルを削除するコマンドではありません。

```bash
cp -a "<EXTRACTED_DIR>/ghcap-v0.0.31.0/." "$HOME/ghcap-v31-review/"
cd "$HOME/ghcap-v31-review"
```

Remove obsolete **tracked** workflow files only after inspecting them. The new workflow is `.github/workflows/build-release.yml`. If an old workflow such as `build-macos.yml` or `macos-ci.yml` remains, two builds may run. / 旧Actionsファイルが残る場合、内容を確認してから追跡対象を削除します。新しいActionsは `.github/workflows/build-release.yml` です。以前のファイルが残ると二重実行される場合があります。

```bash
git ls-files .github/workflows
# If an obsolete workflow is confirmed / 不要な旧ファイルと確認できた場合のみ:
# git rm .github/workflows/build-macos.yml
# git rm .github/workflows/macos-ci.yml
```

## English — 4. Verify before committing / 日本語 — 4. コミット前の検証

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
bash tests/publisher-offline.sh
bash -n packaging/*.sh
git diff --check
git status --short
```

**If any command fails, do not publish.** Fix it and rerun all required gates. / **いずれかのコマンドが失敗した場合は公開せず、修正後に必要な検査を再実行してください。**

Review `git diff` and staged contents. Do not commit secrets, local credential files, `target/`, or other machine-specific data. / `git diff`とStage対象を確認し、秘密情報や端末固有のファイル、`target/`を追加しないでください。

## English — 5. Commit and Push / 日本語 — 5. コミットとPush

```bash
git add -A
git diff --cached --stat
git diff --cached --check
git status -sb
git commit -m "Prepare ghcap v0.0.31.0 integration"
git push -u origin master
```

`git push -u` sets tracking on first Push. If rejected, check remote history instead of forcing it. / 初回Pushなら`-u`で追跡先を設定します。拒否された場合は強制Pushせずリモート履歴を調べてください。

## English — 6. Inspect automated previews / 日本語 — 6. Actionsと開発版を確認

```bash
gh run list --repo solintet/ghcap --workflow build-release.yml --limit 5
gh run watch --repo solintet/ghcap
# After an actual run ID is known / 実際のIDを確認後:
# gh run view <RUN_ID> --repo solintet/ghcap --log-failed

gh release list --repo solintet/ghcap --limit 10
```

Push to `master` triggers four native builds. All must pass before preview publication. The generated preview tag is `v0.0.31.0-pre.<commit-prefix>`. / `master`へのPushで4環境のビルドが始まり、全件成功後に開発版を公開します。タグは`v0.0.31.0-pre.<コミット接頭部>`です。

## English — 7. Stable Release approval / 日本語 — 7. Stableの公開

Review the preview Assets, SHA-256 digests, installation docs, and smoke tests on each intended OS. Only after deciding the version is ready, tag **the exact verified commit**. / 開発版のAssets、SHA-256、導入文書、各OSでの起動確認を終え、正式公開できると判断してから**検証済みのCommit**へタグを付けます。

```bash
git status -sb
git rev-parse HEAD
git tag -a v0.0.31.0 -m "ghcap v0.0.31.0"
git push origin v0.0.31.0
```

The same GitHub Actions workflow builds the tagged commit and, on success, publishes its assets as Stable. If the tag already exists or a build fails, do not move the tag or overwrite the Release. / このタグで再度Actionsがビルドし、全件成功ならStableとして公開します。タグが存在する、またはビルドに失敗した場合はタグ移動・Release上書きで対処しないでください。

```bash
gh run list --repo solintet/ghcap --workflow build-release.yml --limit 5
gh release view v0.0.31.0 --repo solintet/ghcap --json name,tagName,isPrerelease,assets,url
gh release download v0.0.31.0 --repo solintet/ghcap --pattern SHA256SUMS --dir /tmp/ghcap-release-check
```

## English — 8. Publish failure / 日本語 — 8. 公開に失敗した場合

Read the failing Actions job and the native stderr. Retest and correct the source before attempting a new version; do not force-push, reset hard or silently replace existing Release assets. / Actionsの失敗したジョブとstderrを確認してください。ソースを修正して再検証し、公開済みタグ・Assetsの強制変更は避けてください。
