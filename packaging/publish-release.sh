#!/usr/bin/env bash
# EN: Publish verified artifacts without moving tags or replacing published releases.
# JA: タグを移動せず、公開済みReleaseを変更せずに検証済み成果物だけを公開します。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"
for tool in gh git jq sha256sum tar unzip mktemp; do command -v "$tool" >/dev/null || { echo "Missing tool / ツール不足: $tool" >&2; exit 127; }; done
REPO=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}
SHA=${GITHUB_SHA:?GITHUB_SHA is required}
REF=${GITHUB_REF:?GITHUB_REF is required}
MODE=${GHCAP_RELEASE_MODE:-preview}
RUN_ID=${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}
VERSION=0.0.31.0
DIST="$ROOT/release-assets"
[[ "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || { echo 'Invalid GitHub repository / 不正なリポジトリ' >&2; exit 1; }
[[ "$SHA" =~ ^[0-9a-fA-F]{40}$ ]] || { echo 'Invalid commit / 不正なSHA' >&2; exit 1; }
case "$MODE:$REF" in
  preview:refs/heads/master) TAG="v$VERSION-pre.${SHA:0:12}" ;;
  stable:refs/tags/v0.0.31.0) TAG="v$VERSION" ;;
  *) echo 'Invalid release mode/ref / 公開モードと参照が一致しません' >&2; exit 1 ;;
esac
[[ $(git rev-parse --verify HEAD) == "$SHA" ]] || { echo 'Checkout SHA mismatch / ソースが別のコミットです' >&2; exit 1; }
[[ $(gh api "repos/$REPO/commits/$SHA" --jq .sha) == "$SHA" ]] || { echo 'Remote commit differs / リモートコミットが異なります' >&2; exit 1; }
"$ROOT/packaging/validate-release-assets.sh" "$DIST" "$SHA"
SOURCE="ghcap-v${VERSION}-source.tar.gz"
[[ ! -e "$DIST/$SOURCE" ]] || { echo 'Refusing to overwrite source / 既存ソースの上書きを拒否' >&2; exit 1; }
git archive --format=tar.gz "--prefix=ghcap-v${VERSION}/" "--output=$DIST/$SOURCE" "$SHA"
(cd "$DIST" && sha256sum "$SOURCE" > "$SOURCE.sha256")
# EN: Generate one checksums index only after every individual checksum has passed.
# JA: 個別ファイルの検証完了後にのみSHA256SUMSを生成します。
[[ ! -e "$DIST/SHA256SUMS" ]] || { echo 'Refusing to overwrite SHA256SUMS / チェックサムを上書きしません' >&2; exit 1; }
cat "$DIST"/*.sha256 > "$DIST/SHA256SUMS"

# EN: Distinguish absent tags from API errors; an existing tag is never rewritten.
# JA: タグ未作成とAPI失敗を区別し、既存タグは一切移動しません。
refs=$(gh api "repos/$REPO/git/matching-refs/tags/$TAG")
ref_sha=$(jq -r --arg tag "refs/tags/$TAG" '[.[] | select(.ref == $tag) | .object.sha] | first // empty' <<<"$refs")
if [[ "$MODE" == stable && -z "$ref_sha" ]]; then echo 'Stable tag must exist / 正式タグが存在しません' >&2; exit 1; fi
if [[ -n "$ref_sha" ]]; then
  actual_commit=$(gh api "repos/$REPO/commits/$TAG" --jq .sha)
  [[ "$actual_commit" == "$SHA" ]] || { echo 'Tag points elsewhere; refusing to move / タグの移動を拒否します' >&2; exit 1; }
else
  gh api --method POST "repos/$REPO/git/refs" -f "ref=refs/tags/$TAG" -f "sha=$SHA" >/dev/null
fi
releases=$(gh api --paginate "repos/$REPO/releases?per_page=100" --slurp)
existing=$(jq -c --arg tag "$TAG" '[.[][] | select(.tag_name == $tag)] | first // empty' <<<"$releases")
if [[ -n "$existing" ]]; then
  [[ $(jq -r .draft <<<"$existing") == true ]] || { echo 'Published Release is immutable / 公開済みReleaseは変更しません' >&2; exit 1; }
  if [[ "$MODE" == preview ]]; then
    [[ $(jq -r .prerelease <<<"$existing") == true ]] || { echo 'Draft has wrong prerelease state' >&2; exit 1; }
  else
    [[ $(jq -r .prerelease <<<"$existing") == false ]] || { echo 'Draft has wrong stable state' >&2; exit 1; }
  fi
fi
WORK=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-release.XXXXXXXX")
trap 'rm -rf -- "$WORK"' EXIT
if [[ "$MODE" == stable ]]; then
  NOTES="$ROOT/RELEASE_NOTES_v0.0.31.0.md"
else
  NOTES="$ROOT/packaging/prerelease-notes.md"
fi
[[ -s "$NOTES" ]] || { echo 'Missing notes / Release Notesがありません' >&2; exit 1; }
cat -- "$NOTES" > "$WORK/notes.md"
printf '\n\nSource Commit: `%s`\nActions: https://github.com/%s/actions/runs/%s\n' "$SHA" "$REPO" "$RUN_ID" >> "$WORK/notes.md"
if [[ -z "$existing" ]]; then
  args=(release create "$TAG" --repo "$REPO" --verify-tag --target "$SHA" --draft --title "ghcap $TAG" --notes-file "$WORK/notes.md")
  if [[ "$MODE" == preview ]]; then args+=(--prerelease); fi
  gh "${args[@]}" >/dev/null
fi
# EN: Server-side draft state and target are revalidated after creation.
# JA: Draft作成後にもサーバー側タグと対象コミットを再確認します。
[[ $(gh api "repos/$REPO/commits/$TAG" --jq .sha) == "$SHA" ]] || { echo 'Tag moved unexpectedly / タグが変化しました' >&2; exit 1; }
current=$(gh api "repos/$REPO/releases/tags/$TAG")
[[ $(jq -r .draft <<<"$current") == true ]] || { echo 'Release is no longer a draft / Draftではありません' >&2; exit 1; }
release_id=$(jq -r .id <<<"$current")
shopt -s nullglob
for path in "$DIST"/*; do
  name=${path##*/}
  if jq -e --arg name "$name" '.assets[] | select(.name == $name)' <<<"$current" >/dev/null; then
    # EN: Existing assets are only reused after byte-for-byte digest validation.
    # JA: 既存Assetはダウンロードしてハッシュが一致する場合にだけ再利用します。
    mkdir -p "$WORK/existing/$name"
    gh release download "$TAG" --repo "$REPO" --pattern "$name" --dir "$WORK/existing/$name"
    [[ $(sha256sum "$path" | cut -d ' ' -f1) == $(sha256sum "$WORK/existing/$name/$name" | cut -d ' ' -f1) ]] || { echo "Existing asset differs / 既存Assetが異なります: $name" >&2; exit 1; }
  else
    gh release upload "$TAG" "$path" --repo "$REPO"
  fi
done
# EN: Verify each uploaded asset against the server's digest or downloaded bytes.
# JA: アップロード後もサーバーのダイジェストか実ファイルで照合します。
current=$(gh api "repos/$REPO/releases/$release_id")
for path in "$DIST"/*; do
  name=${path##*/}
  digest=$(jq -r --arg name "$name" '[.assets[] | select(.name==$name) | .digest] | first // empty' <<<"$current")
  actual="sha256:$(sha256sum "$path" | cut -d ' ' -f1)"
  if [[ -n "$digest" ]]; then
    [[ "$digest" == "$actual" ]] || { echo "Remote digest mismatch / サーバー照合失敗: $name" >&2; exit 1; }
  else
    mkdir -p "$WORK/verify/$name"
    gh release download "$TAG" --repo "$REPO" --pattern "$name" --dir "$WORK/verify/$name"
    [[ $(sha256sum "$WORK/verify/$name/$name" | cut -d ' ' -f1) == "${actual#sha256:}" ]] || { echo "Remote file mismatch / 配布ファイル不一致: $name" >&2; exit 1; }
  fi
done
if [[ "$MODE" == preview ]]; then
  gh release edit "$TAG" --repo "$REPO" --draft=false --prerelease --latest=false --notes-file "$WORK/notes.md"
else
  gh release edit "$TAG" --repo "$REPO" --draft=false --prerelease=false --notes-file "$WORK/notes.md"
fi
gh release view "$TAG" --repo "$REPO" --json url --jq .url
