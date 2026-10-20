#!/usr/bin/env python3
"""Package the already tested native Linux x86_64 binary; never delete old outputs."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
VERSION = '0.0.29.0'
DIST = ROOT / 'dist'
TOOL = ('https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage', 'ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0')
RUNTIME = ('https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64', '2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d')


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def download(spec, dest):
    with urllib.request.urlopen(spec[0], timeout=120) as response, dest.open('xb') as output:
        shutil.copyfileobj(response, output)
    if hashlib.sha256(dest.read_bytes()).hexdigest() != spec[1]:
        raise RuntimeError('Official AppImage tool/runtime checksum mismatch')
    dest.chmod(0o755)


def main():
    os.chdir(ROOT)
    if os.uname().machine != 'x86_64':
        raise RuntimeError('This package recipe requires native Linux x86_64')
    binary = ROOT / 'target/release/ghcap'
    assert subprocess.check_output([binary, '--version'], text=True).strip() == f'ghcap {VERSION}'
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    epoch = int(subprocess.check_output(['git', 'show', '-s', '--format=%ct', 'HEAD'], text=True))
    os.environ['SOURCE_DATE_EPOCH'] = str(epoch)
    DIST.mkdir(exist_ok=True)
    names = [f'ghcap-v{VERSION}-linux-x86_64.tar.gz', f'ghcap_{VERSION}-1_amd64.deb', f'ghcap-{VERSION}-1.x86_64.rpm', f'ghcap-{VERSION}-1-x86_64.pkg.tar.zst', f'ghcap-v{VERSION}-linux-x86_64.AppImage']
    if any((DIST / n).exists() for n in names):
        raise RuntimeError('Refusing to overwrite existing Linux packages')
    with tempfile.TemporaryDirectory(prefix='ghcap-linux-') as scratch:
        work = Path(scratch)
        payload = work / 'payload'
        bindir = payload / 'usr/bin'
        docs = payload / 'usr/share/doc/ghcap'
        bindir.mkdir(parents=True)
        docs.mkdir(parents=True)
        shutil.copy2(binary, bindir / 'ghcap')
        (bindir / 'ghcap').chmod(0o755)
        for name in ['README.md', 'LICENSE', 'DEPENDENCIES.md', 'THIRD_PARTY_NOTICES.md']:
            shutil.copy2(ROOT / name, docs / name)
        shutil.copytree(ROOT / 'THIRD_PARTY_LICENSES', docs / 'THIRD_PARTY_LICENSES')
        (docs / 'SOURCE_COMMIT.txt').write_text(commit + '\n')
        (docs / 'BUILD_INFO.txt').write_text(subprocess.check_output(['rustc', '-Vv'], text=True) + '\nCargo.lock SHA-256: ' + hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest() + '\nLinux x86_64, built on Ubuntu 22.04; glibc >= 2.35 required.\n')
        archive_dir = work / f'ghcap-v{VERSION}-linux-x86_64'
        archive_dir.mkdir()
        shutil.copy2(binary, archive_dir / 'ghcap')
        shutil.copytree(docs, archive_dir / 'documentation')
        (archive_dir / 'INSTALL.txt').write_text('Requires git, JSON-capable GitHub CLI, and gpg. Install ghcap into ~/.local/bin. Linux x86_64; glibc >= 2.35.\n')
        with tarfile.open(DIST / names[0], 'w:gz') as tar:
            tar.add(archive_dir, arcname=archive_dir.name)
        # Debian: package prebuilt payload, without rebuilding/changing the source.
        deb = work / 'deb'
        shutil.copytree(payload, deb)
        (deb / 'DEBIAN').mkdir()
        (deb / 'DEBIAN/control').write_text(f'Package: ghcap\nVersion: {VERSION}-1\nArchitecture: amd64\nMaintainer: ghcap contributors <noreply@example.com>\nSection: utils\nPriority: optional\nDepends: libc6 (>= 2.35), libgcc-s1, git, gh, gpg\nDescription: GitHub workflow terminal UI\n Safety candidate; external Git, GitHub CLI and GnuPG required.\n')
        run('dpkg-deb', '--root-owner-group', '--build', deb, DIST / names[1])
        run('dpkg-deb', '--info', DIST / names[1])
        unpacked = work / 'deb-unpacked'
        run('dpkg-deb', '--extract', DIST / names[1], unpacked)
        run(unpacked / 'usr/bin/ghcap', '--version')
        # RPM with automatically generated ELF requirements and external tools.
        rpm = work / 'rpmbuild'
        for d in ['BUILD', 'BUILDROOT', 'RPMS', 'SOURCES', 'SPECS', 'SRPMS']:
            (rpm / d).mkdir(parents=True)
        spec = rpm / 'SPECS/ghcap.spec'
        spec.write_text(f'''Name: ghcap
Version: {VERSION}
Release: 1
Summary: GitHub workflow terminal UI
License: BSD-2-Clause
URL: https://github.com/solintet/ghcap
BuildArch: x86_64
Requires: git, gh, gnupg2
%description
GitHub workflow terminal UI safety candidate.
%install
mkdir -p %{{buildroot}}/usr
cp -a {payload}/usr/. %{{buildroot}}/usr/
%files
/usr/bin/ghcap
/usr/share/doc/ghcap
''')
        run('rpmbuild', '--define', f'_topdir {rpm}', '-bb', spec)
        shutil.copy2(rpm / 'RPMS/x86_64' / names[2], DIST / names[2])
        run('rpm', '-qp', '--info', DIST / names[2])
        # Pacman accepts a .PKGINFO plus payload; no install hooks are included.
        arch = work / 'pacman'
        shutil.copytree(payload, arch)
        size = sum(p.stat().st_size for p in arch.rglob('*') if p.is_file())
        (arch / '.PKGINFO').write_text(f'pkgname = ghcap\npkgbase = ghcap\npkgver = {VERSION}-1\npkgdesc = GitHub workflow terminal UI safety candidate\nurl = https://github.com/solintet/ghcap\nbuilddate = {epoch}\npackager = ghcap contributors\nsize = {size}\narch = x86_64\nlicense = BSD-2-Clause\ndepend = glibc>=2.35\ndepend = gcc-libs\ndepend = git\ndepend = github-cli\ndepend = gnupg\n')
        plain_tar = work / 'pacman.tar'
        run('bsdtar', '--uid', '0', '--gid', '0', '-cf', plain_tar, '-C', arch, '.PKGINFO', 'usr')
        run('zstd', '-T2', '-19', plain_tar, '-o', DIST / names[3])
        run('bsdtar', '-tf', DIST / names[3])
        # AppImage: fixed official tool AND runtime, hash checked before execution.
        tool, runtime = work / 'appimagetool', work / 'runtime-x86_64'
        download(TOOL, tool)
        download(RUNTIME, runtime)
        appdir = work / 'AppDir'
        shutil.copytree(payload, appdir)
        (appdir / 'AppRun').write_text('#!/bin/sh\nHERE="$(dirname "$(readlink -f "$0")")"\nexec "$HERE/usr/bin/ghcap" "$@"\n')
        (appdir / 'AppRun').chmod(0o755)
        (appdir / 'ghcap.desktop').write_text('[Desktop Entry]\nType=Application\nName=ghcap\nExec=ghcap\nIcon=ghcap\nTerminal=true\nCategories=Development;Utility;\n')
        (appdir / 'ghcap.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#222"/><path d="M12 18h40M12 32h40M12 46h28" stroke="white" stroke-width="5"/></svg>\n')
        env = dict(os.environ, ARCH='x86_64', APPIMAGE_EXTRACT_AND_RUN='1')
        run(tool, '--runtime-file', runtime, '--no-appstream', appdir, DIST / names[4], env=env)
        run(DIST / names[4], '--version', env=env)
        run(DIST / names[4], '--help', env=env, stdout=subprocess.DEVNULL)
        # Verify RPM and pacman payloads independently of their metadata readers.
        for name in [names[2], names[3]]:
            extract = work / ('extract-' + name)
            extract.mkdir()
            run('bsdtar', '-xf', DIST / name, '-C', extract)
            run(extract / 'usr/bin/ghcap', '--version')
    for name in names:
        p = DIST / name
        (DIST / (name + '.sha256')).write_text(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + name + '\n')


if __name__ == '__main__':
    main()
