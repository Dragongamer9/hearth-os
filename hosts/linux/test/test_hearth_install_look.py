"""The look picked in the USB installer (ui/installer.html) becomes the new user's starting look: hearth-install takes it
from the plan and merges it into their desktop.json with hearth_settings_io (docs/SETTINGS-PERSISTENCE.md). Only
when a look was picked; never over a file that can't be read; never stops the install."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


H = load('hearth_install', ROOT / 'os' / 'live' / 'src' / 'hearth-install')
HIO = load('hearth_settings_io', ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth_settings_io.py')
GOOD = {'mode': 'whole', 'disk': '/dev/nvme0n1', 'user': {'name': 'sam', 'full': 'Sam', 'password': 'pw'},
        'hostname': 'sams-deck', 'timezone': 'America/Chicago', 'edition': 'peppermint'}


class FakeSystem:
    """A new system's files: /etc/passwd with the person, and their home from /etc/skel (desktop.json "defaults": 2)."""
    def __init__(self, skel='{\n  "defaults": 2\n}\n'):
        self.tmp = tempfile.TemporaryDirectory()
        self.dep = self.tmp.name
        os.makedirs(f'{self.dep}/etc')
        uid, gid = os.getuid(), os.getgid()
        with open(f'{self.dep}/etc/passwd', 'w') as f:
            f.write(f'root:x:0:0:root:/root:/bin/bash\nsam:x:{uid}:{gid}:Sam:/var/home/sam:/bin/bash\n')
        self.dir = f'{self.dep}/var/home/sam/.local/share/nimbus'
        self.path = f'{self.dir}/desktop.json'
        if skel is not None:
            os.makedirs(self.dir)
            with open(self.path, 'w') as f:
                f.write(skel)

    def read(self):
        with open(self.path) as f:
            return f.read()

    def close(self):
        self.tmp.cleanup()


class PlanLook(unittest.TestCase):
    def test_kept_when_picked(self):
        self.assertEqual(H.check_plan({**GOOD, 'look': {'name': 'synthwave', 'mode': 'light'}})['look'], {'name': 'synthwave', 'mode': 'light'})
        self.assertEqual(H.check_plan({**GOOD, 'look': {'mode': 'auto'}})['look'], {'mode': 'auto'})

    def test_none_when_not_picked(self):
        self.assertIsNone(H.check_plan(GOOD)['look'])
        self.assertIsNone(H.check_plan({**GOOD, 'look': {}})['look'])

    def test_odd_looks_are_dropped_never_a_stop(self):
        for bad in ('synthwave', 42, ['x'], {'name': '../../etc'}, {'name': 'Synthwave'}, {'mode': 'sepia'}, {'name': None}):
            self.assertIsNone(H.check_plan({**GOOD, 'look': bad})['look'], bad)
        self.assertEqual(H.check_plan({**GOOD, 'look': {'name': 'zen', 'mode': 'purple'}})['look'], {'name': 'zen'})

    def test_every_mantel_look_is_known(self):
        pal = json.load(open(ROOT / 'hosts' / 'linux' / 'desktop' / 'theme' / 'palettes.json'))
        looks = {k for k, v in pal.items() if not k.startswith('_') and isinstance(v, dict) and v.get('dark')}
        self.assertEqual(set(H.LOOK_NAMES), looks)

    def test_the_keys_mantel_reads(self):
        self.assertEqual(H.look_changes({'name': 'cabin', 'mode': 'dark'}), {'look': 'cabin', 'mode': 'dark'})
        self.assertEqual(H.look_changes({'mode': 'auto'}), {'sunset': True})   # Mantel's Sunset; mode left alone
        self.assertEqual(H.look_changes({'name': 'glass'}), {'look': 'glass'})
        self.assertEqual(H.look_changes(None), {})
        settings = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'Settings.qml').read_text()
        for key in ('look', 'mode', 'sunset'):
            self.assertIn(f'property string {key}:' if key != 'sunset' else 'property bool sunset:', settings)


class WriteLook(unittest.TestCase):
    def setUp(self):
        self.sys = FakeSystem()

    def tearDown(self):
        self.sys.close()

    def test_merged_into_the_new_users_file(self):
        self.assertTrue(H.write_look(self.sys.dep, 'sam', {'name': 'synthwave', 'mode': 'light'}, HIO))
        self.assertEqual(json.loads(self.sys.read()), {'defaults': 2, 'look': 'synthwave', 'mode': 'light'})
        # the file before is kept, the person owns everything written
        self.assertEqual(json.load(open(self.sys.path + '.bak')), {'defaults': 2})
        for p in (self.sys.path, self.sys.path + '.bak', f'{self.sys.dir}/.desktop.json.lock'):
            if os.path.exists(p):
                self.assertEqual(os.stat(p).st_uid, os.getuid(), p)
        self.assertFalse([f for f in os.listdir(self.sys.dir) if f.endswith('.tmp')])

    def test_nothing_written_when_no_look_was_picked(self):
        before = self.sys.read()
        for look in (None, {}, {'name': 'nope'}):
            self.assertFalse(H.write_look(self.sys.dep, 'sam', look, HIO))
            self.assertFalse(H.apply_look(self.sys.dep, 'sam', look, HIO))
        self.assertEqual(self.sys.read(), before)
        self.assertFalse(os.path.exists(self.sys.path + '.bak'))

    def test_never_clobbers_other_keys(self):
        with open(self.sys.path, 'w') as f:
            json.dump({'defaults': 2, 'lockOnWake': True, 'accent': '#ff0000', 'look': 'steam'}, f)
        H.write_look(self.sys.dep, 'sam', {'mode': 'auto'}, HIO)
        self.assertEqual(json.loads(self.sys.read()), {'defaults': 2, 'lockOnWake': True, 'accent': '#ff0000', 'look': 'steam', 'sunset': True})

    def test_never_writes_over_a_file_that_cant_be_read(self):
        for broken in ('{"defaults": 2', '', '[1, 2]', '\xff\xfe'):
            with open(self.sys.path, 'w', encoding='latin-1') as f:
                f.write(broken)
            self.assertFalse(H.write_look(self.sys.dep, 'sam', {'name': 'zen'}, HIO))
            with open(self.sys.path, encoding='latin-1') as f:
                self.assertEqual(f.read(), broken)
            self.assertFalse(os.path.exists(self.sys.path + '.bak'))

    def test_no_helper_or_no_user_writes_nothing(self):
        before = self.sys.read()
        self.assertFalse(H.write_look(self.sys.dep, 'nobody-here', {'name': 'zen'}, HIO))
        orig = H.SETTINGS_IO
        try:
            H.SETTINGS_IO = '/nonexistent/hearth_settings_io.py'
            self.assertFalse(H.write_look(self.sys.dep, 'sam', {'name': 'zen'}))
        finally:
            H.SETTINGS_IO = orig
        self.assertEqual(self.sys.read(), before)

    def test_the_new_systems_own_helper_is_used(self):
        shell = os.path.dirname(self.sys.dep + H.SETTINGS_IO)
        os.makedirs(shell)
        with open(ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth_settings_io.py') as src, open(self.sys.dep + H.SETTINGS_IO, 'w') as dst:
            dst.write(src.read())
        self.assertTrue(H.write_look(self.sys.dep, 'sam', {'name': 'cabin'}))
        self.assertEqual(json.loads(self.sys.read())['look'], 'cabin')

    def test_a_failing_look_never_stops_the_install(self):
        class Broken:
            def merge_json(self, *a):
                raise OSError('disk full')
        before = self.sys.read()
        self.assertFalse(H.apply_look(self.sys.dep, 'sam', {'name': 'zen'}, Broken()))
        self.assertEqual(self.sys.read(), before)


class SigninAlways(unittest.TestCase):
    """Sign-in "always" (the password after sleep) used to write the new user's desktop.json over, dropping
    /etc/skel's "defaults": 2. Now it's merged like the look."""
    def setUp(self):
        self.sys = FakeSystem()

    def tearDown(self):
        self.sys.close()

    def test_merged_never_written_over(self):
        H.signin_choice(self.sys.dep, 'sam', 'always', HIO)
        self.assertEqual(json.loads(self.sys.read()), {'defaults': 2, 'lockOnWake': True})
        self.assertTrue(os.path.exists(f'{self.sys.dep}/etc/nimbus/no-autologin'))

    def test_with_a_picked_look_both_stay(self):
        H.signin_choice(self.sys.dep, 'sam', 'always', HIO)
        self.assertTrue(H.apply_look(self.sys.dep, 'sam', {'name': 'zen', 'mode': 'dark'}, HIO))
        self.assertEqual(json.loads(self.sys.read()), {'defaults': 2, 'lockOnWake': True, 'look': 'zen', 'mode': 'dark'})

    def test_auto_and_start_leave_the_file_alone(self):
        before = self.sys.read()
        for choice in ('auto', 'start'):
            H.signin_choice(self.sys.dep, 'sam', choice, HIO)
        self.assertEqual(self.sys.read(), before)

    def test_never_over_a_file_that_cant_be_read(self):
        with open(self.sys.path, 'w') as f:
            f.write('{"defaults": 2')
        H.signin_choice(self.sys.dep, 'sam', 'always', HIO)
        self.assertEqual(self.sys.read(), '{"defaults": 2')


class NoSkelFile(unittest.TestCase):
    def test_made_with_only_the_picked_keys(self):
        s = FakeSystem(skel=None)
        try:
            self.assertTrue(H.write_look(s.dep, 'sam', {'name': 'gaming', 'mode': 'dark'}, HIO))
            self.assertEqual(json.loads(s.read()), {'look': 'gaming', 'mode': 'dark'})
        finally:
            s.close()


if __name__ == '__main__':
    unittest.main()
