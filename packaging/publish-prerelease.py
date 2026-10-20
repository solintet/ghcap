#!/usr/bin/env python3
"""Create a draft, upload verified artifacts, then expose it as a Pre-release.
Never move an existing tag, replace an asset, or modify an existing Stable release.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = '0.0.29.0'
TAG = 'v' + VERSION
DIST = ROOT / 'release-assets'


def gh(*args):
    return subprocess.check_output(['gh', *args], cwd=ROOT, text=True).strip()


def main():
    global TAG
    repo, commit = os.environ['GITHUB_REPOSITORY'], os.environ['GITHUB_SHA']
    assert gh('api', f'repos/{repo}/commits/{commit}', '--jq', '.sha') == commit
    assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == commit
    expected = {
        f'ghcap-v{VERSION}-macos-intel.tar.gz', f'ghcap-v{VERSION}-macos-apple-silicon.tar.gz',
        f'ghcap-v{VERSION}-windows-x64.exe', f'ghcap-v{VERSION}-windows-x64.zip',
        f'ghcap-v{VERSION}-linux-x86_64.tar.gz', f'ghcap_{VERSION}-1_amd64.deb',
        f'ghcap-{VERSION}-1.x86_64.rpm', f'ghcap-{VERSION}-1-x86_64.pkg.tar.zst',
        f'ghcap-v{VERSION}-linux-x86_64.AppImage',
    }
    actual = {p.name for p in DIST.iterdir() if p.is_file() and not p.name.endswith('.sha256')}
    assert actual == expected, (actual, expected)
    for name in sorted(expected):
        p = DIST / name
        digest = hashlib.sha256(p.read_bytes()).hexdigest()
        assert Path(str(p) + '.sha256').read_text().split() == [digest, name], name
    # Verify native package provenance, not merely the names of the artifacts.
    for platform in ['macos-intel', 'macos-apple-silicon', 'linux-x86_64']:
        with tarfile.open(DIST / f'ghcap-v{VERSION}-{platform}.tar.gz') as package:
            note = 'SOURCE_COMMIT.txt' if platform.startswith('macos') else f'ghcap-v{VERSION}-linux-x86_64/documentation/SOURCE_COMMIT.txt'
            assert package.extractfile(note).read().decode().strip() == commit
    with zipfile.ZipFile(DIST / f'ghcap-v{VERSION}-windows-x64.zip') as package:
        assert package.read('SOURCE_COMMIT.txt').decode().strip() == commit
        assert hashlib.sha256(package.read('ghcap.exe')).digest() == hashlib.sha256((DIST / f'ghcap-v{VERSION}-windows-x64.exe').read_bytes()).digest()
    source = DIST / f'ghcap-v{VERSION}-source.tar.gz'
    subprocess.run(['git', 'archive', '--format=tar.gz', f'--prefix=ghcap-v{VERSION}/', f'--output={source}', 'HEAD'], cwd=ROOT, check=True)
    expected.add(source.name)
    sums = []
    for name in sorted(expected):
        digest = hashlib.sha256((DIST / name).read_bytes()).hexdigest()
        line = digest + '  ' + name + '\n'
        (DIST / (name + '.sha256')).write_text(line)
        sums.append(line)
    (DIST / 'SHA256SUMS').write_text(''.join(sums))
    # A missing release must be distinguished from API/authentication errors.
    releases = json.loads(gh('api', '--paginate', f'repos/{repo}/releases?per_page=100', '--slurp'))
    release = next((r for page in releases for r in page if r['tag_name'] == TAG), None)
    refs = json.loads(gh('api', f'repos/{repo}/git/matching-refs/tags/{TAG}'))
    exact = [r for r in refs if r['ref'] == f'refs/tags/{TAG}']
    if (exact and gh('api', f'repos/{repo}/commits/{TAG}', '--jq', '.sha') != commit) or (release and not release['prerelease']):
        # Preserve prior builds: publish a uniquely identified preview instead.
        TAG = 'v' + VERSION + '-pre.' + commit[:12]
        refs = json.loads(gh('api', f'repos/{repo}/git/matching-refs/tags/{TAG}'))
        exact = [r for r in refs if r['ref'] == f'refs/tags/{TAG}']
        release = next((r for page in releases for r in page if r['tag_name'] == TAG), None)
    if exact:
        assert gh('api', f'repos/{repo}/commits/{TAG}', '--jq', '.sha') == commit, 'Existing tag points to another commit; refusing to move it'
    if release is not None:
        assert release['prerelease'], 'Refusing to modify a Stable release'
        if not exact:
            assert release['draft'] and release['target_commitish'] == commit, 'Existing release has no verified tag'
    if not exact:
        gh('api', '--method', 'POST', f'repos/{repo}/git/refs', '-f', f'ref=refs/tags/{TAG}', '-f', f'sha={commit}')
    notes = (ROOT / 'packaging/prerelease-notes.md').read_text()
    notes += f'\n\nSource Commit: `{commit}`\n\nActions: https://github.com/{repo}/actions/runs/{os.environ["GITHUB_RUN_ID"]}\n'
    with tempfile.TemporaryDirectory(prefix='ghcap-publish-') as tmp:
        notes_file = Path(tmp) / 'notes.md'
        notes_file.write_text(notes)
        if release is None:
            gh('release', 'create', TAG, '--repo', repo, '--target', commit, '--draft', '--prerelease', '--title', f'ghcap {TAG} — Pre-release', '--notes-file', str(notes_file))
        # Re-check the exact tag after creation, before exposing any release.
        assert gh('api', f'repos/{repo}/commits/{TAG}', '--jq', '.sha') == commit
        current = json.loads(gh('api', '--paginate', f'repos/{repo}/releases?per_page=100', '--slurp'))
        release = next(r for page in current for r in page if r['tag_name'] == TAG)
        existing = {a['name']: a for a in release['assets']}
        for p in sorted(DIST.iterdir()):
            assert p.is_file()
            if p.name in existing:
                target = Path(tmp) / 'existing' / p.name
                target.parent.mkdir(exist_ok=True)
                gh('release', 'download', TAG, '--repo', repo, '--pattern', p.name, '--dir', str(target.parent))
                assert hashlib.sha256(target.read_bytes()).digest() == hashlib.sha256(p.read_bytes()).digest(), f'Existing asset differs: {p.name}; never clobber'
        for p in sorted(DIST.iterdir()):
            if p.name not in existing:
                gh('release', 'upload', TAG, str(p), '--repo', repo)
        uploaded = json.loads(gh('api', f'repos/{repo}/releases/{release["id"]}'))
        remote_assets = {a['name']: a for a in uploaded['assets']}
        for p in sorted(DIST.iterdir()):
            remote = remote_assets[p.name]
            expected_digest = 'sha256:' + hashlib.sha256(p.read_bytes()).hexdigest()
            if remote.get('digest'):
                assert remote['digest'] == expected_digest, f'Server digest differs: {p.name}'
            else:
                verify = Path(tmp) / 'verify'
                verify.mkdir(exist_ok=True)
                gh('release', 'download', TAG, '--repo', repo, '--pattern', p.name, '--dir', str(verify))
                assert hashlib.sha256((verify / p.name).read_bytes()).hexdigest() == expected_digest[7:]
        gh('release', 'edit', TAG, '--repo', repo, '--draft=false', '--prerelease', '--latest=false', '--notes-file', str(notes_file))
    print(gh('release', 'view', TAG, '--repo', repo, '--json', 'url', '--jq', '.url'))


if __name__ == '__main__':
    main()
