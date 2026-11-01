#!/usr/bin/env bash
# EN: The project's sole local build/test/install entry. Nothing is installed by default.
# JA: ローカル開発の共通入口です。明示的なinstall指定なしに導入しません。
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd "$ROOT"
MODE=${1:-build}
OFFLINE=0
for arg in "$@"; do
    case "$arg" in
      build|test|install|build-install) MODE=$arg ;;
      --offline) OFFLINE=1 ;;
      *) printf 'Usage / 使用方法: ./build-install.sh [build|test|install|build-install] [--offline]\n' >&2; exit 2 ;;
    esac
done
need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required tool / 必須ツール不足: $1" >&2; exit 127; }; }
for tool in cargo rustc; do need "$tool"; done
[[ -f Cargo.lock ]] || { echo 'Cargo.lock is required / Cargo.lockが必要です' >&2; exit 1; }
CARGO_ARGS=(--locked)
if [[ "$OFFLINE" == 1 ]]; then CARGO_ARGS+=(--offline); fi
# EN: Do not format the user's source automatically during release verification.
# JA: 検証工程はソースを勝手に書き換えず、整形の適否だけ確認します。
validate() {
    cargo fmt --all -- --check
    cargo check "${CARGO_ARGS[@]}" --all-targets
    cargo clippy "${CARGO_ARGS[@]}" --all-targets -- -D warnings
    cargo test "${CARGO_ARGS[@]}" --all-targets
}
build() {
    validate
    # EN: Remove only Cargo's known output before building; never reuse a stale binary.
    # JA: 旧バイナリを新規成果物と誤認しないため、ビルド対象のみ再生成します。
    cargo build "${CARGO_ARGS[@]}" --release
    local bin=target/release/ghcap
    [[ -x "$bin" ]] || { echo "Build did not produce executable / 実行ファイルがありません: $bin" >&2; exit 1; }
    [[ $("$bin" --version) == 'ghcap v0.0.31.0' ]] || { echo 'Binary version mismatch / 版不一致' >&2; exit 1; }
}
install_local() {
    # EN: Install into a user-writable location only after explicit confirmation.
    # JA: 明示指定されたユーザー領域へ、確認を取ってから配置します。
    local bin=target/release/ghcap
    [[ -x "$bin" ]] || { echo 'Build first / 先にビルドしてください' >&2; exit 1; }
    [[ $("$bin" --version) == 'ghcap v0.0.31.0' ]] || { echo 'Incorrect binary version / 実行ファイルの版が異なります' >&2; exit 1; }
    local destination="${HOME}/.local/bin/ghcap"
    printf 'Install to / 導入先: %s [y/N] ' "$destination"
    read -r answer
    [[ "$answer" == y || "$answer" == Y || "$answer" == yes ]] || { echo 'Cancelled / 中止'; return 0; }
    mkdir -p -- "$(dirname -- "$destination")"
    if [[ -e "$destination" ]]; then
      printf 'Existing executable will be replaced / 既存の実行ファイルを更新します [y/N] '
      read -r overwrite
      [[ "$overwrite" == y || "$overwrite" == Y || "$overwrite" == yes ]] || { echo 'Cancelled / 中止'; return 0; }
    fi
    local temp
    temp=$(mktemp "$(dirname "$destination")/.ghcap-install.XXXXXXXX")
    trap 'rm -f -- "$temp"' RETURN
    install -m 755 "$bin" "$temp"
    mv -f -- "$temp" "$destination"
    printf 'Installed / 導入完了: %s\n' "$destination"
}
case "$MODE" in
  test) validate ;;
  build) build ;;
  install) install_local ;;
  build-install) build; install_local ;;
esac
