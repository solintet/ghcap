#!/usr/bin/env python3
"""Reject copyleft/unknown Cargo dependencies before packaging ghcap."""
import json
import subprocess
import sys

try:
    # Cargo writes progress/diagnostics to stderr. Capture stdout only so that
    # cargo metadata's JSON is never contaminated by messages such as
    # "Updating crates.io index".
    raw = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--no-deps"],
        text=True,
        stderr=None,
    )
except subprocess.CalledProcessError as exc:
    print("License check failed: cargo metadata failed.", file=sys.stderr)
    if exc.output:
        print(exc.output, file=sys.stderr)
    sys.exit(exc.returncode or 1)
except OSError as exc:
    print(f"License check failed: {exc}", file=sys.stderr)
    sys.exit(1)

metadata = json.loads(raw)
packages = metadata.get("packages", [])
if not packages:
    print("License check failed: Cargo metadata contains no packages.", file=sys.stderr)
    sys.exit(1)

for package in packages:
    name = package.get("name", "<unknown>")
    version = package.get("version", "<unknown>")
    license_expr = (package.get("license") or "").strip()
    license_file = package.get("license_file")
    if not license_expr and not license_file:
        print(f"License check failed: {name} {version} has no license metadata.", file=sys.stderr)
        sys.exit(1)

    upper = license_expr.upper()
    forbidden = ("GPL", "LGPL", "AGPL", "EUPL", "OSL", "SSPL", "MPL", "CPL", "EPL", "CC-BY-NC")
    if any(token in upper for token in forbidden):
        print(f"License check failed: forbidden license for {name} {version}: {license_expr}", file=sys.stderr)
        sys.exit(1)

print(f"License check passed: {len(packages)} Cargo packages inspected; no forbidden copyleft/non-commercial license detected.")
