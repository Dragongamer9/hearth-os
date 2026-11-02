"""Quick Access › Hearth › Start in Steam Game Mode (hosts/decky-hearth/main.py): the plugin's backend reads and writes
HEARTH_STEAM_DEFAULT in Hearth's session.conf the way hearth-core and nimbus-session do. No Steam, no Decky."""
import sys
sys.dont_write_bytecode = True
import asyncio
import importlib.util
import os
import pathlib
import shutil
import subprocess
import tempfile
import types
import unittest

HERE = pathlib.Path(__file__).resolve().parents[1]
ROOT = HERE.parents[1]


def load(tmp):
    fake = types.ModuleType('decky')
    fake.DECKY_USER_HOME = os.path.join(tmp, 'home')
    fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(tmp, 'settings')
    fake.DECKY_PLUGIN_DIR = os.path.join(tmp, 'plugin')
    fake.logger = types.SimpleNamespace(info=lambda *a: None)
    sys.modules['decky'] = fake
    spec = importlib.util.spec_from_file_location('hearth_decky_main_sd', HERE / 'main.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


class SteamDefault(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='decky-hearth-sd-')
        self.m = load(self.tmp)
        self.home = os.path.join(self.tmp, 'home')
        self.conf = os.path.join(self.home, '.config', 'nimbus', 'session.conf')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_read_and_write_keep_the_rest(self):
        m = self.m
        self.assertFalse(m.read_steam_default(''))
        self.assertTrue(m.read_steam_default('export HEARTH_STEAM_DEFAULT="1"\n'))
        self.assertFalse(m.read_steam_default('HEARTH_STEAM_DEFAULT=1\nHEARTH_STEAM_DEFAULT=0\n'))
        t = m.write_steam_default('NIMBUS_GAMESCOPE_ARGS="-W 1280"', True)
        self.assertEqual(t, 'NIMBUS_GAMESCOPE_ARGS="-W 1280"\nHEARTH_STEAM_DEFAULT=1\n')
        t = m.write_steam_default(t, False)
        self.assertEqual(t, 'NIMBUS_GAMESCOPE_ARGS="-W 1280"\nHEARTH_STEAM_DEFAULT=0\n')

    def test_turn_off_from_quick_access_then_on_again(self):
        os.makedirs(os.path.dirname(self.conf))
        with open(self.conf, 'w') as f:
            f.write('HEARTH_DESKTOP=plasma\nHEARTH_STEAM_DEFAULT=1\n')
        state = os.path.join(self.home, '.local', 'state', 'nimbus')
        os.makedirs(state)
        p = self.m.Plugin()
        loop = asyncio.new_event_loop()
        self.assertTrue(loop.run_until_complete(p.get_steam_default()))
        r = loop.run_until_complete(p.set_steam_default(False))
        self.assertEqual(r, {'ok': True, 'on': False})
        self.assertEqual(open(self.conf).read(), 'HEARTH_DESKTOP=plasma\nHEARTH_STEAM_DEFAULT=0\n')
        for n in ('steam-default-off', 'steam-default-fails'):
            open(os.path.join(state, n), 'w').write('x\n')
        r = loop.run_until_complete(p.set_steam_default(True))
        self.assertTrue(r['on'])
        self.assertFalse(os.path.exists(os.path.join(state, 'steam-default-off')), 'turning on again clears the note')
        self.assertFalse(os.path.exists(os.path.join(state, 'steam-default-fails')))

    def test_the_session_no_longer_reads_the_setting_but_shares_the_pause(self):
        # Starting in Steam Game Mode is always on now (the plugin no longer shows the switch): nimbus-session ignores
        # HEARTH_STEAM_DEFAULT, and its two-failures pause is the same steam-default-off note that turning it on clears.
        text = (ROOT / 'hosts/linux/session/nimbus-session').read_text()
        self.assertNotIn('HEARTH_STEAM_DEFAULT', text)
        self.assertIn('SD_OFF="$SD_STATE/steam-default-off"', text)
        self.assertIn('SD_FAILS="$SD_STATE/steam-default-fails"', text)


if __name__ == '__main__':
    unittest.main()
