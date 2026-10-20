"""Offline tests of release side effects: no GitHub credentials/network needed."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile


class PublisherSafety(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('publisher', Path(__file__).with_name('publish-prerelease.py'))
        self.p = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.p)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.p.ROOT = self.root
        self.p.DIST = self.root / 'release-assets'
        self.p.DIST.mkdir()
        (self.root / 'packaging').mkdir()
        (self.root / 'packaging/prerelease-notes.md').write_text('Candidate')
        self.sha, self.old = '1' * 40, '2' * 40
        self.tags, self.releases, self.events, self.data = {}, [], [], {}
        self.bad_digest = False
        v = self.p.VERSION
        for platform in ['macos-intel', 'macos-apple-silicon', 'linux-x86_64']:
            name = f'ghcap-v{v}-{platform}.tar.gz'
            note = 'SOURCE_COMMIT.txt' if platform.startswith('macos') else f'ghcap-v{v}-linux-x86_64/documentation/SOURCE_COMMIT.txt'
            with tarfile.open(self.p.DIST / name, 'w:gz') as t:
                b = (self.sha + '\n').encode()
                info = tarfile.TarInfo(note)
                info.size = len(b)
                t.addfile(info, io.BytesIO(b))
        exe = f'ghcap-v{v}-windows-x64.exe'
        (self.p.DIST / exe).write_bytes(b'exe')
        with zipfile.ZipFile(self.p.DIST / f'ghcap-v{v}-windows-x64.zip', 'w') as z:
            z.writestr('SOURCE_COMMIT.txt', self.sha)
            z.writestr('ghcap.exe', b'exe')
        for name in [f'ghcap_{v}-1_amd64.deb', f'ghcap-{v}-1.x86_64.rpm', f'ghcap-{v}-1-x86_64.pkg.tar.zst', f'ghcap-v{v}-linux-x86_64.AppImage']:
            (self.p.DIST / name).write_bytes(name.encode())
        for f in list(self.p.DIST.iterdir()):
            Path(str(f) + '.sha256').write_text(hashlib.sha256(f.read_bytes()).hexdigest() + '  ' + f.name + '\n')

    def existing(self, tag, sha, prerelease=True):
        self.tags[tag] = sha
        r = dict(id=len(self.releases) + 1, tag_name=tag, target_commitish=sha, prerelease=prerelease, draft=False, assets=[])
        self.releases.append(r)
        return r

    def gh(self, *a):
        if a[0] == 'api':
            url = next(x for x in a if x.startswith('repos/'))
            if '--method' in a:
                tag = next(x[4:] for x in a if x.startswith('ref=')).removeprefix('refs/tags/')
                assert tag not in self.tags
                self.tags[tag] = next(x[4:] for x in a if x.startswith('sha='))
                return '{}'
            if '/commits/' in url:
                key = url.split('/commits/')[1]
                return self.sha if key == self.sha else self.tags[key]
            if '/git/matching-refs/tags/' in url:
                tag = url.split('/tags/')[1]
                return json.dumps([{'ref': 'refs/tags/' + tag}] if tag in self.tags else [])
            if '--slurp' in a:
                return json.dumps([self.releases])
            return json.dumps(next(r for r in self.releases if str(r['id']) == url.rsplit('/', 1)[1]))
        action, tag = a[1:3]
        if action == 'create':
            assert '--draft' in a and '--prerelease' in a
            r = self.existing(tag, self.sha)
            r['draft'] = True
        else:
            r = next(r for r in self.releases if r['tag_name'] == tag)
            if action == 'upload':
                assert '--clobber' not in a
                f = Path(a[3])
                assert f.name not in {x['name'] for x in r['assets']}
                b = f.read_bytes()
                self.data[tag, f.name] = b
                digest = 'sha256:' + hashlib.sha256(b).hexdigest()
                r['assets'].append({'name': f.name, 'digest': 'sha256:invalid' if self.bad_digest else digest})
            elif action == 'download':
                name = a[a.index('--pattern') + 1]
                (Path(a[a.index('--dir') + 1]) / name).write_bytes(self.data[tag, name])
            elif action == 'edit':
                r['draft'] = False
            elif action == 'view':
                return 'https://github.com/owner/repo/releases/tag/' + tag
        self.events.append(action)
        return ''

    def execute(self):
        def archive(args, **kwargs):
            output = next(x.split('=', 1)[1] for x in args if x.startswith('--output='))
            Path(output).write_bytes(b'source archive')
        env = dict(GITHUB_REPOSITORY='owner/repo', GITHUB_SHA=self.sha, GITHUB_RUN_ID='123')
        with patch.dict(os.environ, env), patch.object(self.p, 'gh', self.gh), patch.object(self.p.subprocess, 'check_output', return_value=self.sha), patch.object(self.p.subprocess, 'run', side_effect=archive):
            self.p.main()

    def test_new_release_is_draft_until_assets_verified(self):
        self.execute()
        self.assertFalse(self.releases[0]['draft'])
        self.assertEqual(self.events[-1], 'edit')
        self.assertEqual(len(self.releases[0]['assets']), 21)

    def test_stable_release_is_preserved(self):
        stable = self.existing('v' + self.p.VERSION, self.old, False)
        self.execute()
        self.assertFalse(stable['prerelease'])
        self.assertEqual(stable['assets'], [])
        self.assertEqual(self.tags[stable['tag_name']], self.old)
        self.assertTrue(self.releases[-1]['tag_name'].endswith(self.sha[:12]))

    def test_existing_different_asset_is_never_overwritten(self):
        r = self.existing('v' + self.p.VERSION, self.sha)
        name = f'ghcap-v{self.p.VERSION}-windows-x64.exe'
        r['assets'].append({'name': name, 'digest': None})
        self.data[r['tag_name'], name] = b'other data'
        with self.assertRaises(AssertionError):
            self.execute()
        self.assertNotIn('upload', self.events)
        self.assertNotIn('edit', self.events)

    def test_wrong_provenance_stops_before_creating_tag(self):
        f = self.p.DIST / f'ghcap-v{self.p.VERSION}-windows-x64.zip'
        with zipfile.ZipFile(f, 'w') as z:
            z.writestr('SOURCE_COMMIT.txt', self.old)
            z.writestr('ghcap.exe', b'exe')
        Path(str(f) + '.sha256').write_text(hashlib.sha256(f.read_bytes()).hexdigest() + '  ' + f.name)
        with self.assertRaises(AssertionError):
            self.execute()
        self.assertEqual(self.tags, {})
        self.assertEqual(self.events, [])

    def test_server_checksum_failure_leaves_draft(self):
        self.bad_digest = True
        with self.assertRaises(AssertionError):
            self.execute()
        self.assertTrue(self.releases[0]['draft'])
        self.assertNotIn('edit', self.events)


if __name__ == '__main__':
    unittest.main()
