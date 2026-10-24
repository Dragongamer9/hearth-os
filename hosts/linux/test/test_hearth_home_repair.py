"""Tests for os/system_files/usr/libexec/nimbus/hearth-home-repair (the folders Flatpak, Apps and the Debian box write
to given back to their account; container id ranges) and hearth-admin's home-repair (only ever the caller)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import pwd
import shutil
import tempfile
import types
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
NIMBUS = REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hr = load(NIMBUS / 'hearth-home-repair', 'hearth_home_repair')
ha = load(NIMBUS / 'hearth-admin', 'hearth_admin_for_repair')


def st(uid, mode=0o040755, nlink=1):
    return types.SimpleNamespace(st_uid=uid, st_gid=uid, st_mode=mode, st_nlink=nlink)


class Rules(unittest.TestCase):
    def setUp(self):
        self.protected = hr.PROTECTED
        hr.PROTECTED = True

    def tearDown(self):
        hr.PROTECTED = self.protected

    def test_who_gets_fixed(self):
        known = lambda u: u in (0, 1000, 1001)
        self.assertFalse(hr.should_fix(st(1000), 1000, known))           # already theirs
        self.assertTrue(hr.should_fix(st(0), 1000, known))               # root's folder
        self.assertTrue(hr.should_fix(st(0, 0o100644), 1000, known))     # root's file
        self.assertTrue(hr.should_fix(st(5555), 1000, known))            # an account that's gone
        self.assertFalse(hr.should_fix(st(1001), 1000, known))           # another person's: never
        self.assertFalse(hr.should_fix(st(0, 0o100644, nlink=2), 1000, known))   # maybe a link to a system file
        self.assertFalse(hr.should_fix(st(0, 0o120777, nlink=2), 1000, known))

    def test_root_files_left_when_hardlinks_arent_protected(self):
        hr.PROTECTED = False
        self.assertFalse(hr.should_fix(st(0, 0o100644), 1000, lambda u: True))
        self.assertTrue(hr.should_fix(st(0), 1000, lambda u: True))      # folders still

    def test_which_homes(self):
        base = os.path.realpath('/home')
        self.assertEqual(hr.valid_home('/home/alice', 'alice'), os.path.join(base, 'alice'))
        for bad in ('', 'home/alice', '/', '/home', '/home/', '/etc', '/root', '/home/../etc', '/srv/alice', '/var/lib/x',
                    '/home/alice/../../etc', '/home/a/b'):
            self.assertIsNone(hr.valid_home(bad, 'alice'), bad)

    def test_people(self):
        pw = lambda n, u, h: types.SimpleNamespace(pw_name=n, pw_uid=u, pw_gid=u, pw_dir=h)
        got = hr.people([pw('root', 0, '/root'), pw('sddm', 990, '/var/lib/sddm'), pw('alice', 1000, '/home/alice'),
                         pw('nobody', 65534, '/nonexistent'), pw('svc', 1500, '/opt/svc')])
        self.assertEqual([p.pw_name for p in got], ['alice'])


class Walk(unittest.TestCase):
    """The walk itself, in dry-run mode: as an ordinary user the test can't make files owned by root, so the files'
    real owner (the test's own account) plays "an account that's gone" and the repair works for another id."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-repair-test-')
        self.home = os.path.join(self.tmp, 'alice')
        for d in ('.local/share/flatpak/repo/objects/ab', '.local/share/flatpak/appstream/flathub', '.local/share/applications',
                  '.cache/nimbus/apps', '.config', 'Documents/deep', '.cache/other'):
            os.makedirs(os.path.join(self.home, d))
        for f in ('.local/share/flatpak/repo/config', '.local/share/flatpak/repo/objects/ab/cd.file', 'Documents/deep/x.txt',
                  '.cache/other/y', '.config/z'):
            open(os.path.join(self.home, f), 'w').close()
        self.outside = os.path.join(self.tmp, 'outside')
        os.makedirs(os.path.join(self.outside, 'inner'))
        os.symlink(self.outside, os.path.join(self.home, '.var'))
        self.me = os.getuid()
        self.lines = []

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_repair(self):
        r = hr.Repair(uid=self.me + 4242, gid=self.me + 4242, dry=True, uid_known=lambda u: u != self.me,
                      gid_known=lambda g: True, log=self.lines.append)
        return r.run(self.home)

    def test_only_the_listed_folders(self):
        changed = set(self.run_repair())
        self.assertTrue({'~', '~/.local', '~/.local/share', '~/.local/share/flatpak', '~/.local/share/flatpak/repo/config',
                         '~/.local/share/flatpak/repo/objects/ab/cd.file', '~/.local/share/applications', '~/.cache',
                         '~/.cache/nimbus', '~/.cache/nimbus/apps', '~/.config'} <= changed, changed)
        # Not listed: Documents, ~/.cache/other, what's inside ~/.config (only the folder itself).
        self.assertFalse(any(c.startswith('~/Documents') or c.startswith('~/.cache/other') or c == '~/.config/z' for c in changed), changed)
        self.assertTrue(all('would fix' in l for l in self.lines))

    def test_symlinks_never_followed(self):
        os.makedirs(os.path.join(self.outside, '.local-target', 'share'))
        shutil.rmtree(os.path.join(self.home, '.cache'))
        os.symlink(os.path.join(self.outside, 'inner'), os.path.join(self.home, '.cache'))
        changed = self.run_repair()
        self.assertFalse(any('inner' in c or c.startswith('~/.cache/') for c in changed), changed)
        # The symlink itself may be "fixed" (lchown never touches what it points to); nothing behind it is.
        self.assertFalse(any(c.startswith('~/.var/') for c in changed), changed)

    def test_symlink_inside_a_deep_folder(self):
        os.symlink('/etc', os.path.join(self.home, '.local/share/flatpak/evil'))
        changed = self.run_repair()
        self.assertFalse(any(c.startswith('~/.local/share/flatpak/evil/') for c in changed), changed)

    @unittest.skipUnless(os.geteuid() == 0, 'needs root to make files owned by others')
    def test_real_chown_as_root(self):
        dead = 54321
        while True:
            try:
                pwd.getpwuid(dead)
                dead += 1
            except KeyError:
                break
        nobody = pwd.getpwnam('nobody').pw_uid if 'nobody' in [p.pw_name for p in pwd.getpwall()] else None
        target = 4000
        fp = os.path.join(self.home, '.local/share/flatpak')
        os.chown(self.home, target, target)
        os.chown(os.path.join(self.home, '.local'), 0, 0)
        os.chown(fp, dead, dead)
        os.chown(os.path.join(fp, 'repo/config'), 0, 0)
        if nobody is not None:
            os.chown(os.path.join(fp, 'repo/objects/ab/cd.file'), nobody, nobody)
        os.chown(os.path.join(self.home, 'Documents/deep/x.txt'), 0, 0)
        os.chown(self.outside, 0, 0)
        r = hr.Repair(uid=target, gid=target, log=self.lines.append)
        r.run(self.home)
        self.assertEqual(os.stat(os.path.join(self.home, '.local')).st_uid, target)
        self.assertEqual(os.stat(fp).st_uid, target)
        self.assertEqual(os.stat(os.path.join(fp, 'repo/config')).st_uid, target)
        if nobody is not None:
            self.assertEqual(os.stat(os.path.join(fp, 'repo/objects/ab/cd.file')).st_uid, nobody)   # someone else's
        self.assertEqual(os.stat(os.path.join(self.home, 'Documents/deep/x.txt')).st_uid, 0)          # not listed
        self.assertEqual(os.stat(self.outside).st_uid, 0)                                            # behind a symlink


class SubIds(unittest.TestCase):
    def setUp(self):
        self.etc = tempfile.mkdtemp(prefix='hearth-subid-test-')

    def tearDown(self):
        shutil.rmtree(self.etc, ignore_errors=True)

    def test_parse_and_ranges(self):
        e = hr.parse_subids('alice:100000:65536\nbob:165536:65536\n# x\nbad\n')
        self.assertEqual(e, [('alice', 100000, 65536), ('bob', 165536, 65536)])
        self.assertTrue(hr.has_range(e, 'alice', 1000))
        self.assertFalse(hr.has_range(e, 'carol', 1002))
        self.assertTrue(hr.has_range([('1002', 300000, 65536)], 'carol', 1002))
        self.assertFalse(hr.has_range([('carol', 300000, 100)], 'carol', 1002))   # too small for podman
        self.assertEqual(hr.free_range(e), 231072)
        self.assertEqual(hr.free_range([]), 100000)
        self.assertEqual(hr.free_range([('x', 100000, 65536), ('y', 300000, 65536)]), 165536)
        self.assertEqual(hr.free_range([('x', 120000, 10)]), 165536)

    def test_ensure_adds_once(self):
        with open(os.path.join(self.etc, 'subuid'), 'w') as f:
            f.write('alice:100000:65536')       # no newline at the end
        lines = []
        self.assertTrue(hr.ensure_subids('bob', 1001, etc=self.etc, log=lines.append))
        self.assertEqual(pathlib.Path(self.etc, 'subuid').read_text(), 'alice:100000:65536\nbob:165536:65536\n')
        self.assertEqual(pathlib.Path(self.etc, 'subgid').read_text(), 'bob:100000:65536\n')   # made
        self.assertFalse(hr.ensure_subids('bob', 1001, etc=self.etc, log=lines.append))
        self.assertTrue(hr.ensure_subids('alice', 1000, etc=self.etc, dry=True, log=lines.append))   # no subgid range yet

    def test_dry_run_writes_nothing(self):
        hr.ensure_subids('bob', 1001, etc=self.etc, dry=True, log=lambda m: None)
        self.assertFalse(os.path.exists(os.path.join(self.etc, 'subuid')))


class AdminCommand(unittest.TestCase):
    def test_only_the_caller(self):
        getpw = lambda uid: types.SimpleNamespace(pw_name='alice' if uid == 1000 else 'x')
        self.assertEqual(ha.home_repair_target('1000', getpw), 'alice')
        for bad in (None, '', 'abc', '0', '999', '65534', '-1', '1000 '):
            with self.assertRaises(ValueError, msg=repr(bad)):
                ha.home_repair_target(bad, getpw)

        def missing(uid):
            raise KeyError(uid)
        with self.assertRaises(ValueError):
            ha.home_repair_target('1000', missing)


if __name__ == '__main__':
    unittest.main()
