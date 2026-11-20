"""1.16: your own menu entries that point to a missing program (hearth-stale-launchers): the owner's Fedora laptop kept
Debian's ~/.local/share/applications/steam.desktop (Exec=/usr/games/steam) over Fedora's working one. Found once at
sign-in, shown in a notice; "Use the system one" renames yours (never deletes it) and Undo puts it back. Nothing
changes without a click."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-stale-launchers'
loader = importlib.machinery.SourceFileLoader('hearth_stale_launchers', str(HELPER))
spec = importlib.util.spec_from_loader('hearth_stale_launchers', loader)
hs = importlib.util.module_from_spec(spec)
loader.exec_module(hs)


class Stale(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        t = self.t.name
        self.user = os.path.join(t, 'home', '.local', 'share')
        self.sys = os.path.join(t, 'usr', 'share')
        self.bin = os.path.join(t, 'usr', 'bin')
        for d in (os.path.join(self.user, 'applications'), os.path.join(self.sys, 'applications'), self.bin):
            os.makedirs(d)
        steam = os.path.join(self.bin, 'steam')
        with open(steam, 'w') as f:
            f.write('#!/bin/sh\n')
        os.chmod(steam, 0o755)
        self.env = mock.patch.dict(os.environ, {'XDG_DATA_HOME': self.user, 'XDG_DATA_DIRS': self.sys, 'HOME': os.path.join(t, 'home')})
        self.env.start()
        self.path = self.bin

    def tearDown(self):
        self.env.stop()
        self.t.cleanup()

    def entry(self, where, name, exec_, extra=''):
        p = os.path.join(where, 'applications', name)
        with open(p, 'w') as f:
            f.write(f'[Desktop Entry]\nType=Application\nName={name[:-8].title()}\nExec={exec_}\n{extra}')
        return p

    def test_finds_only_a_broken_copy_of_a_working_system_entry(self):
        mine = self.entry(self.user, 'steam.desktop', '/usr/games/steam %U')
        self.entry(self.sys, 'steam.desktop', f'{self.bin}/steam %U')
        # Your own app with no system entry, a working copy of yours, one you hid, and one whose system entry is broken too.
        self.entry(self.user, 'mine-only.desktop', '/opt/gone/app')
        self.entry(self.user, 'works.desktop', 'env FOO=1 steam -silent')
        self.entry(self.sys, 'works.desktop', 'steam')
        self.entry(self.user, 'hidden.desktop', '/nope', 'Hidden=true\n')
        self.entry(self.sys, 'hidden.desktop', 'steam')
        self.entry(self.user, 'both.desktop', '/nope/a')
        self.entry(self.sys, 'both.desktop', '/nope/b')
        found = hs.scan(self.path)
        self.assertEqual([e['file'] for e in found], [mine])
        self.assertEqual(found[0]['missing'], '/usr/games/steam')
        self.assertEqual(found[0]['name'], 'Steam')

    def test_use_the_system_one_renames_and_undo_puts_it_back(self):
        mine = self.entry(self.user, 'steam.desktop', '/usr/games/steam')
        self.entry(self.sys, 'steam.desktop', f'{self.bin}/steam')
        before = open(mine).read()
        with mock.patch('shutil.which', lambda p, path=None: os.path.join(self.bin, p) if p == 'steam' else None):
            r = hs.fix(mine, self.path)
        self.assertEqual(r['old'], mine + '.hearth-old')
        self.assertFalse(os.path.exists(mine))
        self.assertEqual(open(r['old']).read(), before, 'kept as it was, never deleted')
        self.assertEqual(hs.scan(self.path), [])
        self.assertEqual(hs.undo(r['old'])['file'], mine)
        self.assertEqual(open(mine).read(), before)
        # A file that isn't one it found is never touched.
        ok = self.entry(self.user, 'works.desktop', f'{self.bin}/steam')
        with self.assertRaises(hs.Fail):
            hs.fix(ok, self.path)
        self.assertTrue(os.path.exists(ok))
        # Undo never overwrites an entry that's there again.
        r = hs.fix(mine, self.path)
        self.entry(self.user, 'steam.desktop', '/new/steam')
        with self.assertRaises(hs.Fail):
            hs.undo(r['old'])
        self.assertTrue(os.path.exists(r['old']))

    def test_nothing_changes_without_a_click(self):
        mine = self.entry(self.user, 'steam.desktop', '/usr/games/steam')
        self.entry(self.sys, 'steam.desktop', f'{self.bin}/steam')
        with mock.patch.object(hs, 'notify', return_value='') as n, mock.patch('shutil.which', return_value=None):
            r = hs.check()
        self.assertEqual(r, {'found': 1, 'changed': 0})
        self.assertTrue(os.path.exists(mine))
        self.assertIn("Steam's menu entry points to a program that isn't here (/usr/games/steam)", n.call_args[0][1])
        self.assertIn(('use', 'Use the system one'), n.call_args[0][2])
        # The click, then Undo.
        with mock.patch.object(hs, 'notify', side_effect=['use', 'undo']):
            r = hs.check()
        self.assertEqual(r.get('undone'), True)
        self.assertTrue(os.path.exists(mine))
        with mock.patch.object(hs, 'notify', side_effect=['use', '']):
            r = hs.check()
        self.assertEqual(r['changed'], 1)
        self.assertTrue(os.path.exists(mine + '.hearth-old'))

    def test_runs_once_at_sign_in_on_the_fedora_edition(self):
        a = (ROOT / 'os-bases' / 'anatase' / 'system_files' / 'etc' / 'xdg' / 'autostart' / 'hearth-stale-launchers.desktop').read_text()
        self.assertIn('Exec=/usr/share/nimbus/desktop/shell/hearth-stale-launchers check', a)
        src = HELPER.read_text()
        self.assertNotIn('while True', src, 'no loop: it looks once')
        self.assertNotIn('os.remove', src, 'never deletes your entry')


if __name__ == '__main__':
    unittest.main()
