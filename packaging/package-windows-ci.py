#!/usr/bin/env python3
"""Package a tested native MSVC x64 executable with provenance and licenses."""
from pathlib import Path
import hashlib
import shutil
import subprocess
import tempfile
import zipfile

root = Path(__file__).resolve().parent.parent
version = '0.0.29.0'
binary = root / 'target/release/ghcap.exe'
assert subprocess.check_output([binary, '--version'], text=True).strip() == f'ghcap {version}'
subprocess.run([binary, '--help'], check=True, stdout=subprocess.DEVNULL)
dist = root / 'dist'
dist.mkdir(exist_ok=True)
exe = dist / f'ghcap-v{version}-windows-x64.exe'
archive = dist / f'ghcap-v{version}-windows-x64.zip'
if exe.exists() or archive.exists():
    raise RuntimeError('Refusing to overwrite existing Windows outputs')
shutil.copy2(binary, exe)
with tempfile.TemporaryDirectory(prefix='ghcap-windows-') as tmp:
    stage = Path(tmp)
    shutil.copy2(binary, stage / 'ghcap.exe')
    for name in ['README.md', 'LICENSE', 'DEPENDENCIES.md', 'THIRD_PARTY_NOTICES.md']:
        shutil.copy2(root / name, stage / name)
    shutil.copytree(root / 'THIRD_PARTY_LICENSES', stage / 'THIRD_PARTY_LICENSES')
    (stage / 'SOURCE_COMMIT.txt').write_text(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True))
    (stage / 'BUILD_INFO.txt').write_text(subprocess.check_output(['rustc', '-Vv'], text=True) + '\nCargo.lock SHA-256: ' + hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest() + '\n')
    (stage / 'INSTALL.txt').write_text('Requires Git for Windows, JSON-capable GitHub CLI, and GnuPG on PATH. Run ghcap.exe from a terminal. Windows folder-to-Release upload remains disabled pending verified no-follow support.\n')
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED) as output:
        for p in sorted(stage.rglob('*')):
            if p.is_file():
                output.write(p, p.relative_to(stage).as_posix())
    verify = stage / 'verify'
    with zipfile.ZipFile(archive) as package:
        package.extractall(verify)
    subprocess.run([verify / 'ghcap.exe', '--version'], check=True)
for p in [exe, archive]:
    Path(str(p) + '.sha256').write_text(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.name + '\n')
