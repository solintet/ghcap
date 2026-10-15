#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for command in cargo rustc dpkg-buildpackage; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "Missing build dependency: $command" >&2
        exit 1
    }
done

for command in git gh gpg; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "Missing runtime dependency: $command" >&2
        exit 1
    }
done

if [ ! -f Cargo.lock ]; then
    cargo generate-lockfile
fi

# Warnings are release blockers. This catches unused variables/dead code at compile time.
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo check --locked
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --locked
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --release --locked

if cargo clippy --version >/dev/null 2>&1; then
    RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo clippy --locked --all-targets --all-features -- -D warnings
fi

dpkg-buildpackage -us -uc -b

OUT="/tmp/ghcap-debs"
rm -rf "$OUT"
mkdir -p "$OUT"
for file in "$(dirname "$ROOT")"/ghcap_0.27.4-1_*.deb; do
    [ -f "$file" ] && cp -f "$file" "$OUT/"
done
printf '%s\n' "Built packages copied to: $OUT"
printf '%s\n' "Install with: sudo apt install $OUT/ghcap_0.27.4-1_amd64.deb"
