#!/usr/bin/env bash
# EN: Publish a separately verified build. Drafts are addressed by numeric API ID, never list visibility.
# JA: 検証済み成果物のみを公開。作成したDraftは一覧検索せず、数値IDで操作します。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
for tool in gh git jq sha256sum tar unzip curl mktemp; do command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 127; }; done
REPO=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}
SHA=${GHCAP_SOURCE_SHA:-${GITHUB_SHA:?GITHUB_SHA is required}}
REF=${GHCAP_SOURCE_REF:-${GITHUB_REF:?GITHUB_REF is required}}
MODE=${GHCAP_RELEASE_MODE:-preview}
RUN_ID=${GHCAP_BUILD_RUN_ID:-${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}}
RELEASE_ID_OVERRIDE=${GHCAP_RELEASE_ID:-}
VERSION=0.0.31.0
DIST="$ROOT/release-assets"
[[ "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || { echo 'Invalid repository' >&2; exit 1; }
[[ "$SHA" =~ ^[0-9a-fA-F]{40}$ && "$RUN_ID" =~ ^[0-9]+$ ]] || { echo 'Invalid source commit or build Run ID' >&2; exit 1; }
[[ -z "$RELEASE_ID_OVERRIDE" || "$RELEASE_ID_OVERRIDE" =~ ^[0-9]+$ ]] || { echo 'Invalid numeric Draft ID' >&2; exit 1; }
[[ -n "${GH_TOKEN:-}" ]] || { echo 'GH_TOKEN is required' >&2; exit 1; }
case "$MODE:$REF" in
  preview:refs/heads/master) TAG="v$VERSION-pre.${SHA:0:12}" ;;
  stable:refs/tags/v0.0.31.0) TAG="v$VERSION" ;;
  *) echo 'Invalid release mode/ref / 公開モードと対象が一致しません' >&2; exit 1 ;;
esac
# EN: Publisher may be newer than the build; archive the original build commit, not current master.
# JA: 公開処理だけを後から修正しても、ソースアーカイブはビルド元SHAから作ります。
[[ $(gh api "repos/$REPO/commits/$SHA" --jq .sha) == "$SHA" ]] || { echo 'Source commit is not in repository' >&2; exit 1; }
if ! git cat-file -e "$SHA^{commit}" 2>/dev/null; then
  git fetch --no-tags --depth=1 origin "$SHA"
fi
[[ $(git rev-parse "$SHA^{commit}") == "$SHA" ]] || { echo 'Unavailable source commit' >&2; exit 1; }
"$ROOT/packaging/validate-release-assets.sh" "$DIST" "$SHA"
SOURCE="ghcap-v${VERSION}-source.tar.gz"
[[ ! -e "$DIST/$SOURCE" && ! -e "$DIST/SHA256SUMS" ]] || { echo 'Refusing to overwrite previously prepared files' >&2; exit 1; }
git archive --format=tar.gz "--prefix=ghcap-v${VERSION}/" "--output=$DIST/$SOURCE" "$SHA"
(cd "$DIST" && sha256sum "$SOURCE" > "$SOURCE.sha256")
(cd "$DIST" && cat -- ./*.sha256 > SHA256SUMS)
[[ $(find "$DIST" -maxdepth 1 -type f | wc -l) == 21 ]] || { echo 'Release asset count mismatch' >&2; exit 1; }
# EN: Tag references are checked before creating a new one. Existing refs are never moved.
# JA: 既存タグの移動や公開済みReleaseの変更は禁止します。
refs=$(gh api "repos/$REPO/git/matching-refs/tags")
ref_sha=$(jq -r --arg tag "refs/tags/$TAG" '[.[] | select(.ref == $tag) | .object.sha] | first // empty' <<<"$refs")
if [[ -n "$ref_sha" ]]; then
  actual_commit=$(gh api "repos/$REPO/commits/$TAG" --jq .sha)
  [[ "$actual_commit" == "$SHA" ]] || { echo 'Existing tag points to another commit' >&2; exit 1; }
else
  [[ "$MODE" == preview ]] || { echo 'Stable tag must already exist' >&2; exit 1; }
  gh api --method POST "repos/$REPO/git/refs" -f "ref=refs/tags/$TAG" -f "sha=$SHA" >/dev/null
fi
WORK=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-release.XXXXXXXX")
trap 'rm -rf -- "$WORK"' EXIT
if [[ "$MODE" == stable ]]; then NOTES="$ROOT/RELEASE_NOTES_v0.0.31.0.md"; else NOTES="$ROOT/packaging/prerelease-notes.md"; fi
[[ -s "$NOTES" ]] || { echo 'Missing Release Notes' >&2; exit 1; }
cat "$NOTES" > "$WORK/notes.md"
printf '\n\nSource Commit: `%s`\nSource Build: https://github.com/%s/actions/runs/%s\n' "$SHA" "$REPO" "$RUN_ID" >> "$WORK/notes.md"
prerelease=false
[[ "$MODE" == preview ]] && prerelease=true
# EN: Optional ID override recovers a pre-existing invisible draft without guessing.
# JA: 既存Draftが一覧に出ない場合に数値IDを指定して安全に再開できます。
if [[ -n "$RELEASE_ID_OVERRIDE" ]]; then
  release_id=$RELEASE_ID_OVERRIDE
  current=$(gh api "repos/$REPO/releases/$release_id")
else
  releases=$(gh api --paginate "repos/$REPO/releases?per_page=100" --slurp)
  current=$(jq -c --arg tag "$TAG" '[.[][] | select(.tag_name == $tag)] | first // empty' <<<"$releases")
  if [[ -n "$current" ]]; then
    release_id=$(jq -r .id <<<"$current")
  else
    # EN: Creation returns the authoritative release ID. Do not immediately scan an eventually consistent list.
    # JA: POSTの応答IDを保持し、作成直後の一覧再検索には依存しません。
    jq -n --arg tag "$TAG" --arg sha "$SHA" --arg title "ghcap $TAG" \
       --rawfile notes "$WORK/notes.md" --argjson pre "$prerelease" \
       '{tag_name:$tag,target_commitish:$sha,name:$title,body:$notes,draft:true,prerelease:$pre,generate_release_notes:false}' > "$WORK/create.json"
    echo "[publisher] Creating draft by REST API / IDを取得してDraft作成: $TAG"
    if ! current=$(gh api --method POST "repos/$REPO/releases" --input "$WORK/create.json"); then
      echo 'Create failed. If a hidden Draft exists, rerun with existing_release_id (numeric ID).' >&2
      exit 1
    fi
    release_id=$(jq -r .id <<<"$current")
  fi
fi
[[ "$release_id" =~ ^[0-9]+$ ]] || { echo 'Release creation did not return a numeric ID' >&2; exit 1; }
# EN: All subsequent reads/writes use ID: independent of tag lookup and release list propagation.
# JA: 以後はRelease一覧・タグ検索を使わず数値IDで検証・アップロード・公開します。
# EN: Direct by-ID reads may briefly lag creation. Retry only this lightweight API call.
# JA: 作成直後のID照会は一時的に反映待ちとなる可能性があるため、短時間だけ再試行します。
current=''
for attempt in 1 2 3 4 5; do
  if current=$(gh api "repos/$REPO/releases/$release_id"); then break; fi
  [[ "$attempt" != 5 ]] || { echo 'Created Release ID is not readable' >&2; exit 1; }
  sleep 1
done
jq -e --arg tag "$TAG" --argjson pre "$prerelease" \
   '.tag_name==$tag and .draft==true and .prerelease==$pre' <<<"$current" >/dev/null || {
  echo 'Release is published, mismatched, or not a draft; refusing mutation / Draftの状態が不正' >&2; exit 1;
}
[[ $(gh api "repos/$REPO/commits/$TAG" --jq .sha) == "$SHA" ]] || { echo 'Tag changed unexpectedly' >&2; exit 1; }
# EN: Never delete, replace or blindly trust existing assets. Verify GitHub's SHA-256 digest.
# JA: 再実行時は既存AssetのSHA256を比較し、削除・上書きしません。
shopt -s nullglob
for asset in "$DIST"/*; do
  name=${asset##*/}
  [[ "$name" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "Unsafe asset name: $name" >&2; exit 1; }
  current=$(gh api "repos/$REPO/releases/$release_id")
  existing=$(jq -c --arg n "$name" '[.assets[] | select(.name==$n)] | first // empty' <<<"$current")
  expected="sha256:$(sha256sum "$asset" | cut -d ' ' -f1)"
  if [[ -n "$existing" ]]; then
    actual=$(jq -r '.digest // empty' <<<"$existing")
    [[ "$actual" == "$expected" ]] || { echo "Existing asset checksum missing/different: $name" >&2; exit 1; }
    echo "[publisher] Reusing verified asset / 既存Assetを再利用: $name"
    continue
  fi
  echo "[publisher] Uploading / アップロード: $name"
  # EN: Use the documented releases/<numeric ID>/assets endpoint instead of a tag-based CLI lookup.
  # JA: タグ経由ではなく数値Release IDのRESTアップロードAPIを利用します。
  curl --fail-with-body --silent --show-error \
       --request POST --header "Authorization: Bearer $GH_TOKEN" \
       --header 'Accept: application/vnd.github+json' \
       --header 'Content-Type: application/octet-stream' \
       --data-binary "@$asset" \
       "https://uploads.github.com/repos/$REPO/releases/$release_id/assets?name=$name" > "$WORK/upload.json"
  jq -e --arg n "$name" '.name==$n and (.id | type)=="number"' "$WORK/upload.json" >/dev/null || {
    echo "Unexpected upload response: $name" >&2; exit 1;
  }
done
# EN: Refuse to publish incomplete/extra assets; compare all 21 server digests.
# JA: 21ファイル全件のサーバー側チェックサムを検証してから公開します。
current=$(gh api "repos/$REPO/releases/$release_id")
[[ $(jq '.assets|length' <<<"$current") == 21 ]] || { echo 'Unexpected number of uploaded assets' >&2; exit 1; }
for asset in "$DIST"/*; do
  name=${asset##*/}
  expected="sha256:$(sha256sum "$asset" | cut -d ' ' -f1)"
  actual=$(jq -r --arg n "$name" '[.assets[] | select(.name==$n) | .digest] | first // empty' <<<"$current")
  [[ "$actual" == "$expected" ]] || { echo "Remote SHA256 failed: $name" >&2; exit 1; }
done
jq -e --arg tag "$TAG" '.tag_name==$tag and .draft==true' <<<"$current" >/dev/null || { echo 'Draft state changed' >&2; exit 1; }
jq -n --rawfile notes "$WORK/notes.md" --argjson pre "$prerelease" \
  '{draft:false,prerelease:$pre,body:$notes,make_latest:"false"}' > "$WORK/publish.json"
current=$(gh api --method PATCH "repos/$REPO/releases/$release_id" --input "$WORK/publish.json")
jq -e --arg tag "$TAG" '.tag_name==$tag and .draft==false' <<<"$current" >/dev/null || {
  echo 'GitHub did not confirm publication' >&2; exit 1;
}
echo "Published / 公開成功: https://github.com/$REPO/releases/tag/$TAG"
