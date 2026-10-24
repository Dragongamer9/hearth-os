"""ARCHIVED (docs/ARCHIVED.md). Tests for desktop/shell/hearth-apps when your own Flatpak installation can't be used (~/.local/share/flatpak owned
by root or an old account): the problem said plainly, the system's Flathub used for the catalog and installs, the
real reason kept when a refresh fails, and the catalog found in flatpak's appstream folders. A stand-in `flatpak`
records what it was asked."""
import sys
sys.dont_write_bytecode = True
import contextlib
import gzip
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import tempfile
import unittest
from unittest import mock

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
APPSTREAM = b'''<?xml version="1.0"?><components origin="flathub"><component type="desktop-application">
<id>org.videolan.VLC</id><name>VLC</name><summary>Media player</summary>
<bundle type="flatpak">app/org.videolan.VLC/x86_64/stable</bundle></component></components>'''

FAKE_FLATPAK = r'''#!/bin/sh
echo "$*" >> "$FAKE_LOG"
case "$*" in
  "remotes --user --columns=name") [ -n "$FAKE_USER_REMOTE" ] && echo flathub; exit 0 ;;
  "remotes --system --columns=name") [ -n "$FAKE_SYSTEM_REMOTE" ] && echo flathub; exit 0 ;;
  "update --appstream --user flathub") echo "error: Can't open file ~/.local/share/flatpak/repo/tmp: Permission denied" >&2; exit 1 ;;
  "update --appstream --system flathub")
    if [ -n "$FAKE_SYSTEM_OK" ]; then mkdir -p "$FAKE_SYS/appstream/flathub/$FAKE_ARCH/active" && cp "$FAKE_XML" "$FAKE_SYS/appstream/flathub/$FAKE_ARCH/active/appstream.xml.gz"; exit 0; fi
    echo "error: Flatpak system operation Deploy not allowed for user" >&2; exit 1 ;;
  "--version") echo "Flatpak 1.16.0"; exit 0 ;;
esac
exit 0
'''


def load(name='hearth_apps'):
    loader = importlib.machinery.SourceFileLoader(name, str(SHELL / 'hearth-apps'))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Access(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-apps-test-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(self.home, '.local', 'share'))
        os.makedirs(os.path.join(self.home, '.cache'))
        bin_dir = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_dir)
        with open(os.path.join(bin_dir, 'flatpak'), 'w') as f:
            f.write(FAKE_FLATPAK)
        os.chmod(os.path.join(bin_dir, 'flatpak'), 0o755)
        xml = os.path.join(self.tmp, 'appstream.xml.gz')
        with gzip.open(xml, 'wb') as f:
            f.write(APPSTREAM)
        self.env = mock.patch.dict(os.environ, {
            'HOME': self.home, 'PATH': bin_dir + os.pathsep + os.environ.get('PATH', ''), 'XDG_RUNTIME_DIR': os.path.join(self.tmp, 'run'),
            'FAKE_LOG': os.path.join(self.tmp, 'log'), 'FAKE_SYS': os.path.join(self.tmp, 'sys'), 'FAKE_XML': xml})
        self.env.start()
        for k in ('XDG_DATA_HOME', 'XDG_CACHE_HOME', 'FAKE_USER_REMOTE', 'FAKE_SYSTEM_REMOTE', 'FAKE_SYSTEM_OK'):
            os.environ.pop(k, None)
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        self.h = load()
        os.environ['FAKE_ARCH'] = self.h.arch()
        self.h.SYSTEM_FLATPAK = os.path.join(self.tmp, 'sys')

    def tearDown(self):
        self.env.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def calls(self):
        try:
            return pathlib.Path(os.environ['FAKE_LOG']).read_text().splitlines()
        except OSError:
            return []

    def not_mine(self):
        """~/.local/share/flatpak as if root owned it: the check compares owners with the id it's given."""
        os.makedirs(os.path.join(self.home, '.local', 'share', 'flatpak', 'repo'))
        return os.getuid() + 777

    def test_fine_when_everything_is_yours(self):
        self.assertEqual(self.h.user_dir_problems(), [])
        self.assertTrue(self.h.user_usable())

    def test_problem_said_plainly(self):
        other = self.not_mine()
        probs = self.h.user_dir_problems(uid=other)
        self.assertIn(os.path.join(self.home, '.local', 'share', 'flatpak'), [p['path'] for p in probs])
        text = self.h.problem_text(probs)
        self.assertIn('~/.local/share/flatpak', text)
        self.assertIn('not you', text)
        self.assertIn('hearth-admin home-repair', text)
        self.assertEqual(self.h.problem_text([]), '')

    def test_a_cache_folder_that_isnt_yours_doesnt_stop_installs(self):
        other = os.getuid() + 777
        only_cache = [p for p in self.h.user_dir_problems(uid=other) if p['path'] == os.path.join(self.home, '.cache')]
        self.assertTrue(only_cache)
        self.assertNotIn(os.path.join(self.home, '.cache'), [p['path'] for p in self.h.user_dir_problems(uid=other, cache=False)])

    def test_refresh_falls_back_to_the_systems_flathub(self):
        self.h.user_dir_problems = lambda uid=None, cache=True: [{'path': os.path.join(self.home, '.local/share/flatpak'), 'owner': 'root', 'uid': 0}]
        os.environ['FAKE_SYSTEM_REMOTE'] = '1'
        os.environ['FAKE_SYSTEM_OK'] = '1'
        with contextlib.redirect_stdout(io.StringIO()):
            r = self.h.refresh()
        self.assertEqual(r['apps'], 1)
        self.assertIn('belongs to root', r['warning'])
        self.assertIn('update --appstream --system flathub', self.calls())
        self.assertNotIn('update --appstream --user flathub', self.calls())   # not even tried
        self.assertNotIn('remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo', self.calls())
        self.assertEqual(self.h.install_target(), 'system')

    def test_refresh_failure_keeps_the_real_reasons(self):
        os.environ['FAKE_USER_REMOTE'] = '1'
        os.environ['FAKE_SYSTEM_REMOTE'] = '1'
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(self.h.Fail) as e:
            self.h.refresh()
        msg = str(e.exception)
        self.assertIn('Permission denied', msg)
        self.assertIn('not allowed for user', msg)
        self.assertIn('Permission denied', self.h.last_error()['error'])
        st = self.h.status()
        self.assertIn('Permission denied', st['lastError']['error'])
        self.assertTrue(st['userRemote'] and st['systemRemote'])
        self.assertEqual(st['version'], 'Flatpak 1.16.0')

    def test_no_flathub_anywhere(self):
        self.h.user_dir_problems = lambda uid=None, cache=True: [{'path': '/x', 'owner': 'root', 'uid': 0}]
        self.assertIsNone(self.h.install_target())
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(self.h.Fail) as e:
            self.h.install('org.videolan.VLC')
        self.assertIn('no Flathub', str(e.exception))

    def test_install_goes_to_the_system_when_yours_cant_be_used(self):
        self.h.user_dir_problems = lambda uid=None, cache=True: [{'path': '/x', 'owner': 'root', 'uid': 0}]
        os.environ['FAKE_SYSTEM_REMOTE'] = '1'
        seen = []
        self.h.run_with_progress = lambda cmd, what: (seen.append(cmd), (0, ''))[1]
        self.h.installed_map = lambda: {'org.videolan.VLC': {}}
        with contextlib.redirect_stdout(io.StringIO()):
            r = self.h.install('org.videolan.VLC')
        self.assertEqual(seen, [['flatpak', 'install', '--system', '-y', 'flathub', 'org.videolan.VLC']])
        self.assertEqual(r['installation'], 'system')

    def test_install_retries_system_on_permission_denied(self):
        os.environ['FAKE_USER_REMOTE'] = '1'
        os.environ['FAKE_SYSTEM_REMOTE'] = '1'
        seen = []
        have = {}

        def fake(cmd, what):
            seen.append(cmd)
            if '--user' in cmd:
                return 1, 'opening repo: Permission denied'
            have['org.videolan.VLC'] = {}
            return 0, ''
        self.h.run_with_progress = fake
        self.h.installed_map = lambda: have
        with contextlib.redirect_stdout(io.StringIO()):
            r = self.h.install('org.videolan.VLC')
        self.assertEqual([c[2] for c in seen], ['--user', '--system'])
        self.assertEqual(r['installation'], 'system')

    def test_catalog_without_the_active_link(self):
        d = os.path.join(self.tmp, 'sys', 'appstream', 'flathub', self.h.arch(), '0123abcd')
        os.makedirs(d)
        with gzip.open(os.path.join(d, 'appstream.xml.gz'), 'wb') as f:
            f.write(APPSTREAM)
        self.assertIn(d, self.h.appstream_dirs())
        apps, _ = self.h.catalog()
        self.assertIn('org.videolan.VLC', apps)

    def test_catalog_cache_moves_when_cache_cant_be_written(self):
        d = os.path.join(self.home, '.local', 'share', 'flatpak', 'appstream', 'flathub', self.h.arch(), 'active')
        os.makedirs(d)
        with gzip.open(os.path.join(d, 'appstream.xml.gz'), 'wb') as f:
            f.write(APPSTREAM)
        blocker = os.path.join(self.tmp, 'not-a-folder')
        open(blocker, 'w').close()
        self.h.CACHE = os.path.join(blocker, 'nimbus', 'apps')     # can't be made
        apps, det = self.h.catalog()
        self.assertIn('org.videolan.VLC', apps)
        self.assertTrue(det.startswith(os.environ['XDG_RUNTIME_DIR']), det)


if __name__ == '__main__':
    unittest.main()
