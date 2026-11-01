# ghcap v0.0.31.0: independent build and release (2026-11-01)

Source: user-supplied `ghcap-master.zip`, compared with `ghcap-actions-41295147663(1).log`.

## Diagnosed log failure

All four build artifacts were downloaded, their checksums passed, 9 platform deliverables were validated, a preview tag was created, and a Draft Release creation command ran. The subsequent authenticated list query did **not** show the just-created Draft (`Draft not found in release list` at 09:59:43 UTC). This is a Release lookup failure, **not** a Rust compilation failure.

## Remediation

- Build and Release are now two separate workflows. Publisher `workflow_dispatch` can use a previous run ID (including legacy failed all-in-one runs only where all 4 Build matrix jobs succeeded).
- Source commit and Run identity are compared to the GitHub run metadata, OS archives, and SHA256 files before upload.
- New Drafts are created using GitHub REST API POST, whose numeric Release ID is preserved. Asset upload, state checks and publication use that ID.
- A known Draft ID can be supplied manually to recover a Draft that GitHub's release-list endpoint does not show.
- Already uploaded assets are reused only after SHA256 digest confirmation; released assets/tags are never overwritten.
- Rust cache remains in place with platform-specific cache keys; removed redundant `cargo check` but retained `cargo fmt`, `cargo clippy`, and `cargo test` on each native runner.
- `packaging/publish-release.sh` continues to publish 21 files (9 deliverables + 9 checksums + source archive + its checksum + SHA256SUMS).

## Verification in this sandbox

- `ruby` YAML parse: success (both workflows).
- `bash -n`: success (packaging and tests).
- `tests/publisher-offline.sh`: success (valid file, checksum tamper, archive/exe tamper, stale artifact, source mismatch).
- `tests/publisher-state.sh`: success (immutable published Release, tag mismatch).
- `tests/publisher-preview.sh`: success (7 asset uploads then intentional interruption; recovery using ID, digest reuse, 21 assets, publish).
- `tests/publisher-run.sh`: success (new Run, legacy Run, failed platform, failed current workflow, PR, foreign repo).
- Actual GitHub Actions network publication: NOT tested (requires GH_TOKEN and repository permissions).
- Rust compilation: NOT tested in this environment (Cargo/rustc unavailable). Unmodified application source is preserved from the supplied ZIP.
