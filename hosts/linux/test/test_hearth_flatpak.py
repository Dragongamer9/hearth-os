"""Tests for desktop/shell/hearth-flatpak (Flatpak and Bazaar for Settings › Apps and Mantel) and for Bazaar being
Hearth OS's app store: Hearth's own store (Apps, hearth-apps) is archived (archive/app-store, docs/ARCHIVED.md) and
nothing outside the archive opens, installs or checks it any more. A stand-in `flatpak` records what it was asked."""
import sys
sys.dont_write_bytecode = True
import contextlib
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

REPO = pathlib.Path(__file__).resolve().parents[3]
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'
BAZAAR = 'io.github.kolunmi.Bazaar'

FAKE_FLATPAK = r'''#!/bin/sh
echo "$*" >> "$FAKE_LOG"
case "$*" in
  "remotes --user --columns=name") exit 0 ;;
  "remotes --system --columns=name") echo flathub; exit 0 ;;
  "--version") echo "Flatpak 1.16.0"; exit 0 ;;
  list*) [ -e "$FAKE_GONE" ] || printf 'org.gimp.GIMP\tGIMP\t2.10\tuser\t300 MB\tflathub\n'; exit 0 ;;
  "uninstall --user -y --noninteractive org.gimp.GIMP") touch "$FAKE_GONE"; exit 0 ;;
  "info io.github.kolunmi.Bazaar") [ -n "$FAKE_BAZAAR" ] && exit 0; exit 1 ;;
  "run io.github.kolunmi.Bazaar") exit 0 ;;
esac
exit 0
'''


def load(name='hearth_flatpak'):
    loader = importlib.machinery.SourceFileLoader(name, str(SHELL / 'hearth-flatpak'))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Helper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-flatpak-test-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(self.home, '.local', 'share'))
        os.makedirs(os.path.join(self.home, '.cache'))
        bin_dir = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_dir)
        with open(os.path.join(bin_dir, 'flatpak'), 'w') as f:
            f.write(FAKE_FLATPAK)
        os.chmod(os.path.join(bin_dir, 'flatpak'), 0o755)
        self.log = os.path.join(self.tmp, 'log')
        env = {'HOME': self.home, 'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
               'XDG_CACHE_HOME': os.path.join(self.home, '.cache'), 'FAKE_LOG': self.log,
               'FAKE_GONE': os.path.join(self.tmp, 'gone'), 'PATH': bin_dir + os.pathsep + os.environ.get('PATH', '')}
        p = mock.patch.dict(os.environ, env)
        p.start()
        self.addCleanup(p.stop)
        self.h = load()
        self.h.SYSTEM_FLATPAK = os.path.join(self.tmp, 'var-lib-flatpak')
        self.h.SETUP_STATE = os.path.join(self.tmp, 'flatpak-setup.json')

    def asked(self):
        try:
            return open(self.log).read()
        except OSError:
            return ''

    def quiet(self, fn, *a):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            return fn(*a), out.getvalue()

    def test_ids_checked(self):
        for bad in ('', 'org', 'org.foo', 'org.foo.Bar;rm', '--user', '../org.a.b', 'org.a.b c'):
            with self.assertRaises(self.h.Fail, msg=bad):
                self.h.check_id(bad)
        self.assertEqual(self.h.check_id(BAZAAR), BAZAAR)

    def test_columns(self):
        rows = self.h.parse_columns('org.gimp.GIMP\tGIMP\t2.10\tuser\t3 MB\tflathub\nApplication ID\tName\nnot-an-id\tx\n',
                                    ['id', 'name', 'version', 'installation', 'size', 'origin'])
        self.assertEqual([r['id'] for r in rows], ['org.gimp.GIMP'])

    def test_your_folder_fine(self):
        self.assertEqual(self.h.user_dir_problems(), [])
        st = self.h.status()
        self.assertTrue(st['userWritable'])
        self.assertEqual(st['installTo'], 'user')
        self.assertTrue(st['systemRemote'] and st['flathub'])

    def test_a_folder_that_isnt_yours_is_said_plainly(self):
        os.makedirs(os.path.join(self.home, '.local', 'share', 'flatpak'))
        probs = self.h.user_dir_problems(uid=os.getuid() + 4242)
        self.assertTrue(probs)
        text = self.h.problem_text(probs)
        self.assertIn('Your own Flatpak folder can\'t be used', text)
        self.assertIn('hearth-admin home-repair', text)

    def test_bazaar_found_where_it_is(self):
        self.assertEqual(self.h.bazaar_where(), '')
        st = self.h.status()
        self.assertFalse(st['bazaar'])
        self.assertIn('Bazaar isn\'t installed yet', st['bazaarMissing'])
        os.makedirs(os.path.join(self.h.SYSTEM_FLATPAK, 'app', BAZAAR))
        self.assertEqual(self.h.bazaar_where(), 'system')
        self.assertTrue(self.h.status()['bazaar'])

    def test_setup_state_shown(self):
        self.assertIsNone(self.h.status()['setup'])
        with open(self.h.SETUP_STATE, 'w') as f:
            json.dump({'time': 1, 'ok': False, 'step': 'bazaar', 'message': 'offline'}, f)
        self.assertEqual(self.h.status()['setup']['message'], 'offline')

    def test_open_bazaar_before_its_installed(self):
        with self.assertRaises(self.h.Fail) as e:
            self.h.open_bazaar()
        self.assertIn('installs it for everyone when the PC is online', str(e.exception))

    def test_open_bazaar(self):
        os.makedirs(os.path.join(self.h.SYSTEM_FLATPAK, 'app', BAZAAR))
        if not shutil.which('setsid'):
            self.skipTest('no setsid here')
        with mock.patch.dict(os.environ, {'FAKE_BAZAAR': '1'}):
            self.assertEqual(self.h.open_bazaar()['opened'], BAZAAR)
        import time
        for _ in range(50):
            if 'run io.github.kolunmi.Bazaar' in self.asked():
                break
            time.sleep(0.1)
        self.assertIn('run io.github.kolunmi.Bazaar', self.asked())

    def test_remove(self):
        r, out = self.quiet(self.h.remove, 'org.gimp.GIMP')
        self.assertEqual(r['installed'], False)
        self.assertIn('uninstall --user -y --noninteractive org.gimp.GIMP', self.asked())
        self.assertIn('progress: 100 Removed', out)
        with self.assertRaises(self.h.Fail):
            self.quiet(self.h.remove, 'org.gimp.GIMP')

    def test_usage(self):
        with contextlib.redirect_stdout(io.StringIO()) as out, self.assertRaises(SystemExit):
            self.h.main(['x', 'remove', 'org.a.B', '--everything'])
        self.assertIn('usage', out.getvalue())


class StoreArchived(unittest.TestCase):
    """Bazaar is the app store; Hearth's own one lives only in archive/app-store."""

    def test_store_files_are_in_the_archive(self):
        for f in ('hosts/linux/desktop/shell/hearth-apps', 'hosts/linux/desktop/shell/AppStore.qml',
                  '.github/scripts/hearth-store-check.sh'):
            self.assertFalse((REPO / f).exists(), f)
            self.assertTrue((REPO / 'archive' / 'app-store' / f).exists(), f)
        self.assertTrue((REPO / 'docs' / 'ARCHIVED.md').exists())

    def test_nothing_opens_or_installs_the_old_store(self):
        for p in sorted(SHELL.iterdir()):
            if p.is_file() and p.suffix in ('.qml', '') and p.name != 'hearth-flatpak':
                text = p.read_text(errors='replace')
                self.assertNotIn('AppStore {', text, p.name)
                self.assertNotIn('helper: "hearth-apps"', text, p.name)
        for b in ('anatase', 'peppermint'):
            build = (REPO / 'os-bases' / b / 'build.sh').read_text()
            self.assertNotIn('hearth-apps.desktop <<', build, b)
            self.assertNotIn('shell/hearth-apps ', build, b)
        for wf in ('base-anatase.yml', 'base-peppermint.yml'):
            text = (REPO / '.github' / 'workflows' / wf).read_text()
            self.assertNotIn('hearth-store-check.sh', text, wf)
            self.assertIn('Bazaar', text, wf)

    def test_bazaar_is_where_the_store_was(self):
        shell = (SHELL / 'shell.qml').read_text()
        self.assertIn('target: "apps"', shell)                       # old Apps links and keybinds open Bazaar
        self.assertIn('hearth-flatpak', shell)
        desk = (SHELL / 'hearth-desk').read_text()
        self.assertIn(BAZAAR, desk)                                  # pinned on the taskbar
        system = (SHELL / 'hearth-system').read_text()
        self.assertIn("'kind': 'store', 'label': 'App store', 'hearth': BAZAAR_DESKTOP", system)
        self.assertIn("BAZAAR_DESKTOP = '%s.desktop'" % BAZAAR, system)
        core = (REPO / 'hosts' / 'hearth-core' / 'src' / 'bazaar.rs').read_text()
        self.assertIn(BAZAAR, core)
        ui = (REPO / 'ui' / 'index.html').read_text()
        self.assertIn("call('bazaar.open'", ui)


if __name__ == '__main__':
    unittest.main()
