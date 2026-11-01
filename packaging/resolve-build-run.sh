#!/usr/bin/env bash
# EN: Bind release publication to a completed, successful run from this repository.
# JA: 公開対象を本リポジトリで成功したビルドRunに固定します。
set -euo pipefail
for tool in gh jq; do command -v "$tool" >/dev/null || { echo "Missing: $tool" >&2; exit 127; }; done
REPO=${GITHUB_REPOSITORY:?}
RUN_ID=${GHCAP_REQUEST_RUN_ID:?}
REQUEST_MODE=${GHCAP_REQUEST_MODE:-auto}
[[ "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ && "$RUN_ID" =~ ^[0-9]+$ ]] || { echo 'Invalid repository or Run ID' >&2; exit 1; }
[[ "$REQUEST_MODE" == auto || "$REQUEST_MODE" == preview || "$REQUEST_MODE" == stable ]] || { echo 'Invalid release mode' >&2; exit 1; }
info=$(gh api "repos/$REPO/actions/runs/$RUN_ID")
jq -e --arg repo "$REPO" '
  .status == "completed" and
  .head_repository.full_name == $repo and
  (.event == "push" or .event == "workflow_dispatch") and
  (.path | startswith(".github/workflows/build-release.yml@")) and
  (.head_sha | test("^[0-9a-fA-F]{40}$"))
' <<<"$info" >/dev/null || { echo 'Build is not a trusted ghcap workflow / 信頼できるビルドではありません' >&2; exit 1; }
conclusion=$(jq -r .conclusion <<<"$info")
workflow_name=$(jq -r .name <<<"$info")
if [[ "$conclusion" == success ]]; then
  [[ "$workflow_name" == 'Build and verify ghcap' || "$workflow_name" == 'Build, verify and release ghcap' ]] || {
    echo 'Unexpected build workflow name' >&2; exit 1;
  }
fi
if [[ "$conclusion" != success ]]; then
  # EN: The original single workflow may have failed *only after* all four builds succeeded.
  # JA: 旧一体型Workflowで4ビルド成功後に公開だけ失敗したRunの救済処理です。
  [[ -z "${GHCAP_AUTO_RUN_ID:-}" && "$conclusion" == failure &&
     "$workflow_name" == 'Build, verify and release ghcap' ]] || {
    echo 'Build run did not succeed / Build Runが成功していません' >&2; exit 1;
  }
  jobs=$(gh api --paginate "repos/$REPO/actions/runs/$RUN_ID/jobs?per_page=100" --slurp)
  jq -e '
    [ .[] | .jobs[] | select(.name | startswith("Build ")) |
       {name:.name, conclusion:.conclusion} ] as $builds |
    ([$builds[].name] | unique | length) == 4 and
    (all($builds[]; .conclusion == "success")) and
    ([$builds[].name] | sort) ==
       (["Build linux-x86_64","Build windows-x64","Build macos-intel","Build macos-apple-silicon"] | sort)
  ' <<<"$jobs" >/dev/null || { echo 'Legacy run lacks four successful native build jobs' >&2; exit 1; }
  echo 'Legacy build recovery / 旧Runのビルド成果物を再利用'
fi
sha=$(jq -r .head_sha <<<"$info")
branch=$(jq -r '.head_branch // ""' <<<"$info")
path=$(jq -r .path <<<"$info")
event=$(jq -r .event <<<"$info")
if [[ -n "${GHCAP_AUTO_RUN_ID:-}" ]]; then
  [[ "$GHCAP_AUTO_RUN_ID" == "$RUN_ID" && "$event" == push ]] || { echo 'Automatic build Run ID mismatch' >&2; exit 1; }
fi
if [[ "$REQUEST_MODE" == auto ]]; then
  case "$branch" in
    master) mode=preview ;;
    v0.0.31.0) mode=stable ;;
    '')
      [[ "$path" == '.github/workflows/build-release.yml@refs/tags/v0.0.31.0' ]] || { echo 'Unsupported source ref' >&2; exit 1; }
      mode=stable
      ;;
    *) echo 'Automatic publishing is limited to master and the stable tag' >&2; exit 1 ;;
  esac
else
  mode=$REQUEST_MODE
fi
if [[ "$mode" == preview ]]; then
  [[ "$branch" == master && "$path" == *'@refs/heads/master' ]] || { echo 'Preview requires a master build' >&2; exit 1; }
  ref=refs/heads/master
else
  # EN: Manual release from master build is allowed only when the exact immutable tag points to its SHA.
  # JA: Stableは正確なタグがビルド元SHAを指すときだけ許可します。
  tag_sha=$(gh api "repos/$REPO/commits/v0.0.31.0" --jq .sha)
  [[ "$tag_sha" == "$sha" ]] || { echo 'Stable tag does not point to build commit' >&2; exit 1; }
  [[ "$path" == *'@refs/tags/v0.0.31.0' || "$path" == *'@refs/heads/master' ]] || { echo 'Stable source must be stable tag or master' >&2; exit 1; }
  ref=refs/tags/v0.0.31.0
fi
[[ -n "${GITHUB_OUTPUT:-}" ]] || { echo 'GITHUB_OUTPUT is required' >&2; exit 1; }
{
  printf 'run_id=%s\n' "$RUN_ID"
  printf 'sha=%s\n' "$sha"
  printf 'mode=%s\n' "$mode"
  printf 'ref=%s\n' "$ref"
} >> "$GITHUB_OUTPUT"
printf 'Trusted build / 検証済みBuild: run=%s sha=%s mode=%s\n' "$RUN_ID" "$sha" "$mode"
