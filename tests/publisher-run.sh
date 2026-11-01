#!/usr/bin/env bash
# EN: Offline tests for new and legacy build-run metadata trust decisions.
# JA: 新旧Build Runの出所・成功判定に関するオフラインテストです。
set -euo pipefail
PROJECT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
BASE=$(mktemp -d "${TMPDIR:-/tmp}/ghcap-run-metadata.XXXXXXXX")
trap 'rm -rf -- "$BASE"' EXIT
mkdir -p "$BASE/bin"
cat > "$BASE/bin/gh" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  *"actions/runs/123456/jobs?per_page=100"*)
    jq -n --arg bad "${MOCK_BAD_JOB:-}" '[[{jobs:(
      ["linux-x86_64","windows-x64","macos-intel","macos-apple-silicon"] |
      map({name:("Build " + .),conclusion:(if . == $bad then "failure" else "success" end)}))
    }]]' | jq '[.[0][0]]'
    ;;
  *"actions/runs/123456"*)
    jq -n --arg sha '0123456789abcdef0123456789abcdef01234567' \
      --arg event "${MOCK_EVENT:-push}" --arg status "${MOCK_CONCLUSION:-success}" \
      --arg flow "${MOCK_WORKFLOW:-Build and verify ghcap}" \
      --arg repo "${MOCK_REPO:-solintet/ghcap}" \
      '{status:"completed",conclusion:$status,event:$event,name:$flow,
        head_sha:$sha,head_branch:"master",head_repository:{full_name:$repo},
        path:".github/workflows/build-release.yml@refs/heads/master"}'
    ;;
  *) echo "Mock received unexpected call: $*" >&2; exit 96 ;;
esac
MOCK
chmod 755 "$BASE/bin/gh"
export PATH="$BASE/bin:$PATH" GITHUB_REPOSITORY=solintet/ghcap GHCAP_REQUEST_RUN_ID=123456 GHCAP_REQUEST_MODE=preview GITHUB_OUTPUT="$BASE/output"
bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null
[[ $(grep -c '^run_id=123456$' "$BASE/output") == 1 ]] || exit 1
: > "$BASE/output"
export MOCK_CONCLUSION=failure MOCK_WORKFLOW='Build, verify and release ghcap'
bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null || { echo 'Legacy successful jobs were rejected' >&2; exit 1; }
[[ $(grep -c '^run_id=123456$' "$BASE/output") == 1 ]] || exit 1
export MOCK_BAD_JOB=windows-x64
if bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null 2>&1; then echo 'Failed legacy job was accepted' >&2; exit 1; fi
unset MOCK_BAD_JOB
export MOCK_WORKFLOW='Build and verify ghcap'
if bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null 2>&1; then echo 'Failed modern workflow was accepted' >&2; exit 1; fi
export MOCK_CONCLUSION=success MOCK_EVENT=pull_request
if bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null 2>&1; then echo 'PR workflow was accepted' >&2; exit 1; fi
export MOCK_EVENT=push MOCK_REPO=other/ghcap
if bash "$PROJECT/packaging/resolve-build-run.sh" >/dev/null 2>&1; then echo 'Foreign repository was accepted' >&2; exit 1; fi
printf 'PASS / 成功: new successful run, legacy recovery, unsuccessful/fork/PR rejection\n'
