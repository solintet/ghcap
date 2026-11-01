(
set -euo pipefail

REPO="solintet/ghcap"
cd "/home/yellowpc/ドキュメント/Github/solintet/ghcap/"

# 1. 環境・リポジトリ確認
test "$(git branch --show-current)" = "master"
test -f ".github/workflows/build-release.yml"
test -f ".github/workflows/publish-release.yml"
test -f "packaging/resolve-build-run.sh"
gh auth status

git fetch origin master
git merge-base --is-ancestor origin/master HEAD || {
    echo "リモート側に未統合の変更があります。Push中止。"
    exit 1
}

# 2. Shell構文検査
bash -n build-install.sh packaging/*.sh tests/*.sh

# 3. Release公開処理のテスト
bash tests/publisher-offline.sh
bash tests/publisher-state.sh
bash tests/publisher-preview.sh
bash tests/publisher-run.sh

# 4. Rust品質検査・テスト・Releaseビルド
./build-install.sh build
./target/release/ghcap --version

# 5. Git Commit
git add -A
git diff --cached --check
git diff --cached --stat

PUSHED=0
if ! git diff --cached --quiet; then
    read -r -p "上記の変更をGitHubへPushしますか？ [y/N] " YES
    test "$YES" = "y" || exit 1

    git commit -m "Separate build and release workflows"
    git push origin master
    PUSHED=1
else
    echo "新しいコミット対象はありません。"
fi

SHA="$(git rev-parse HEAD)"
BUILD_RUN=""
BUILD_EVENT="push"

# 6. Pushで自動起動したBuild Actionsを取得
if [ "$PUSHED" = 1 ]; then
    for i in $(seq 1 20); do
        BUILD_RUN=$(gh run list -R "$REPO" \
            --workflow build-release.yml \
            --commit "$SHA" \
            --event push \
            --json databaseId \
            --jq '.[0].databaseId // empty')
        [ -n "$BUILD_RUN" ] && break
        sleep 3
    done
fi

# 自動起動が確認できなければ、手動実行
if [ -z "$BUILD_RUN" ]; then
    BUILD_EVENT="workflow_dispatch"
    gh workflow run build-release.yml \
        -R "$REPO" --ref master

    for i in $(seq 1 30); do
        BUILD_RUN=$(gh run list -R "$REPO" \
            --workflow build-release.yml \
            --commit "$SHA" \
            --event workflow_dispatch \
            --json databaseId \
            --jq '.[0].databaseId // empty')
        [ -n "$BUILD_RUN" ] && break
        sleep 3
    done
fi

test -n "$BUILD_RUN" || {
    echo "Build Runを確認できませんでした。"
    exit 1
}

# 7. 4環境のビルド完了まで監視
echo "Build Run ID: $BUILD_RUN"
gh run watch "$BUILD_RUN" -R "$REPO" --exit-status

# 8. 手動Buildの場合はPublishも手動起動
if [ "$BUILD_EVENT" = "workflow_dispatch" ]; then
    gh workflow run publish-release.yml \
        -R "$REPO" \
        --ref master \
        -f build_run_id="$BUILD_RUN" \
        -f release_mode=preview
    PUBLISH_EVENT="workflow_dispatch"
else
    PUBLISH_EVENT="workflow_run"
fi

# 9. Release公開Actionsを監視
PUBLISH_RUN=""
for i in $(seq 1 30); do
    PUBLISH_RUN=$(gh run list -R "$REPO" \
        --workflow publish-release.yml \
        --commit "$SHA" \
        --event "$PUBLISH_EVENT" \
        --json databaseId \
        --jq '.[0].databaseId // empty')
    [ -n "$PUBLISH_RUN" ] && break
    sleep 3
done

test -n "$PUBLISH_RUN" || {
    echo "Publish Runが見つかりません。Actions画面を確認してください。"
    exit 1
}

gh run watch "$PUBLISH_RUN" -R "$REPO" --exit-status

# 10. 最終確認
gh release list -R "$REPO" --limit 5
echo "Build・Test・Push・Actions・Preview Release完了"
)
