# Release artifact formats

| Format | Generated from Debian? | Method / caveat |
|---|---:|---|
| Linux `.tar.gz`, `.deb`, `.rpm`, `.AppImage` | Yes | Native Linux build and package tools |
| Arch `.pkg.tar.zst` | Yes, via container | `makepkg` inside Arch Linux container |
| Windows x64 `.exe` | Yes | `x86_64-pc-windows-gnu` + MinGW; validate on Windows |
| Windows portable `.zip` | Yes | Contains `ghcap.exe` and documentation; winget source artifact |
| macOS Intel `.tar.gz` | On demand | Manually triggered GitHub Actions job on an Intel macOS runner |
| macOS Apple Silicon `.tar.gz` | On demand | Manually triggered GitHub Actions job on an Apple Silicon macOS runner |
| winget manifests | Yes | Generated from the Windows ZIP's actual SHA-256 and GitHub release URL; repository submission is separate |
| `SHA256SUMS` + `.sha256` sidecars | Yes | Generated from final artifact files; checksums do not attest to publisher identity |

The Windows target is the GNU ABI for cross-compilation practicality. It is not an MSVC build and does not replace native Windows CI. Intel and Apple Silicon macOS binaries are built separately; this project does not create a Universal Binary. macOS SDKs are not redistributed by this project. macOS build artifacts are unsigned and not notarized.
