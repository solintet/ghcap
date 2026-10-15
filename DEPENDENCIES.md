# Dependency notes for ghcap 0.27.4

## Newly added direct dependencies

| Crate | Version | License | Purpose |
|---|---:|---|---|
| `rust-i18n` | 4.2.2 | MIT | Compile-time translations, locale selection, and fallback |
| `walkdir` | 2.5.0 | MIT OR Unlicense | Cross-platform recursive traversal of release staging folders |

Neither newly added direct dependency uses GPL-family or MPL licensing. `walkdir` is dual-licensed; the MIT option can be selected for redistribution.

## Existing direct dependencies

The direct dependencies are pinned in `Cargo.toml`. A full transitive-license conclusion requires generating `Cargo.lock` with Cargo and inspecting the resolved graph; this source package does not claim that a complete transitive audit has been run in the current build environment.
