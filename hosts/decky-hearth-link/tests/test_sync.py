"""Tests for Hearth Link's sync switches (hosts/decky-hearth-link/link_sync.py, main.py's sync calls): off by default
and nothing changes then; per key, newest change wins; never a whole file; sign-ins never shared; device-specific
settings stay; Decky plugins from another device are packed for Decky's own install (paths checked)."""
import sys
sys.dont_write_bytecode = True
import base64
import json
import os
import pathlib
import shutil
import tempfile
import types
import unittest
import zipfile

os.environ['XDG_STATE_HOME'] = tempfile.mkdtemp()
os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)
HERE = pathlib.Path(__file__).resolve().parents[1]
REPO = HERE.parents[1]
sys.path.insert(0, str(REPO / 'hosts' / 'linux' / 'desktop' / 'shell'))
sys.path.insert(0, str(HERE))
import link_sync as ls  # noqa: E402

A, B = 'a' * 64, 'b' * 64
T = ls.now_ms()          # the rounds' clock, from now on


def rd(p):
    with open(p) as f:
        return json.load(f)


def wr(p, v):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        json.dump(v, f)


class Device:
    """One Hearth device's home, and its side of Hearth Link (prefs.json)."""
    def __init__(self, root, name):
        self.home = os.path.join(root, name)
        self.p = ls.Paths(self.home, os.path.join(self.home, 'homebrew/settings/decky-hearth-link'))

    def f(self, rel):
        return os.path.join(self.home, rel)


def carry(src, dst):
    """What Hearth Link does between two devices, in short: src's local keys newer than dst's land in dst's remote
    (and local), each with its time (hosts/hearth-link sync.rs has the real rules and its own tests)."""
    s = ls.hio.load_json(src.p.prefs, {})
    d = ls.hio.load_json(dst.p.prefs, {})
    for ns, kv in (s.get('local') or {}).items():
        for k, e in kv.items():
            mine = (d.get('local') or {}).get(ns, {}).get(k)
            if mine is None or e['t'] > mine['t']:
                d.setdefault('remote', {}).setdefault(ns, {})[k] = e
                d.setdefault('local', {}).setdefault(ns, {})[k] = e
    ls.hio.write_json(dst.p.prefs, d)


class Sync(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix='hl-sync-')
        self.a, self.b = Device(self.root, 'ally'), Device(self.root, 'desk')

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_switches_start_off_and_nothing_changes_then(self):
        a, b = self.a, self.b
        self.assertEqual(ls.switches(a.p), {'plugins': False, 'settings': False})
        wr(a.f('.local/share/nimbus/desktop.json'), {'look': 'hearth'})
        wr(b.f('.local/share/nimbus/desktop.json'), {'look': 'steam'})
        wr(b.p.prefs, {'remote': {'hearth:desktop': {'look': {'v': 'hearth', 't': 9e12}}}})
        self.assertEqual(ls.tick(a.p), {'applied': 0, 'shared': 0, 'errors': []})
        self.assertEqual(ls.tick(b.p), {'applied': 0, 'shared': 0, 'errors': []})
        self.assertFalse(os.path.exists(a.p.prefs), 'nothing shared with the switch off')
        self.assertEqual(rd(b.f('.local/share/nimbus/desktop.json')), {'look': 'steam'}, 'nothing taken with the switch off')

    def test_switches_are_remembered_one_at_a_time(self):
        p = self.a.p
        ls.set_switch(p, 'settings', True)
        ls.set_switch(p, 'plugins', True)
        ls.set_switch(p, 'plugins', False)
        self.assertEqual(ls.switches(p), {'plugins': False, 'settings': True})
        self.assertTrue(os.path.exists(p.switches() + '.bak'), 'saved the safe way (with .bak)')
        with self.assertRaises(ValueError):
            ls.set_switch(p, 'settings', 'yes')
        with self.assertRaises(ValueError):
            ls.set_switch(p, 'everything', True)

    def test_hearth_settings_follow_per_key_and_device_ones_stay(self):
        a, b = self.a, self.b
        da, db = a.f('.local/share/nimbus/desktop.json'), b.f('.local/share/nimbus/desktop.json')
        wr(da, {'look': 'hearth', 'mode': 'light', 'wallpaper': '/home/ana/Pictures/cat.png', 'screenOffMinutes': 3, 'lockOnWake': True})
        wr(db, {'look': 'steam', 'mode': 'dark', 'wallpaper': 'builtin:dunes', 'screenOffMinutes': 15, 'pointerSpeed': 1.5})
        wr(a.f('.config/nimbus/osk.json'), {'theme': 'ember', 'position': 'top'})
        wr(b.f('.config/nimbus/buttons.json'), {'press': 'menu', 'overamp': True})
        wr(a.f('.config/hearth-player/settings.json'), {'shuffle': True, 'volume': 0.2})
        for d in (a, b):
            ls.set_switch(d.p, 'settings', True)
        os.utime(db, (1000, 1000))                                # the desk's look is older
        ls.tick(a.p, A, now=T + 5_000_000)
        ls.tick(b.p, B, now=T + 5_000_000)
        carry(a, b)
        carry(b, a)
        ls.tick(a.p, A, now=T + 5_100_000)
        ls.tick(b.p, B, now=T + 5_100_000)
        ga, gb = rd(da), rd(db)
        self.assertEqual((gb['look'], gb['mode']), ('hearth', 'light'), "the newer device's look")
        self.assertEqual(gb['wallpaper'], 'builtin:dunes', "a picture file's path never travels")
        self.assertEqual(ga['wallpaper'], '/home/ana/Pictures/cat.png')
        self.assertEqual((ga['screenOffMinutes'], gb['screenOffMinutes']), (3, 15), 'power timers stay per device')
        self.assertNotIn('pointerSpeed', ga)
        self.assertTrue(ga['lockOnWake'] and 'lockOnWake' not in gb)
        self.assertEqual(rd(b.f('.config/nimbus/osk.json')), {'theme': 'ember'}, 'the keyboard look, not its position')
        self.assertEqual(rd(a.f('.config/nimbus/buttons.json')), {'press': 'menu'}, 'button press, not louder-than-100%')
        self.assertEqual(rd(b.f('.config/hearth-player/settings.json')), {'shuffle': True}, 'music settings, not the volume')
        self.assertTrue(os.path.exists(db + '.bak'), 'written the safe way')

        # A change later on the desk wins on the Ally; a key changed meanwhile on the Ally stays (per key).
        ls.hio.merge_json(db, {'mode': 'dark'})
        ls.hio.merge_json(da, {'accent': '#ff8a3d'})
        ls.tick(a.p, A, now=T + 5_200_000)
        ls.tick(b.p, B, now=T + 5_300_000)
        carry(b, a)
        carry(a, b)
        ls.tick(a.p, A, now=T + 5_400_000)
        ls.tick(b.p, B, now=T + 5_400_000)
        self.assertEqual(rd(da)['mode'], 'dark')
        self.assertEqual(rd(db)['accent'], '#ff8a3d')
        self.assertEqual(rd(da)['accent'], '#ff8a3d')
        # A round with nothing new changes nothing.
        before = (open(da).read(), open(db).read(), open(a.p.prefs).read())
        ls.tick(a.p, A, now=T + 5_500_000)
        ls.tick(b.p, B, now=T + 5_500_000)
        self.assertEqual((open(da).read(), open(db).read(), open(a.p.prefs).read()), before)

    def test_turning_it_off_stops_taking_changes(self):
        a = self.a
        d = a.f('.config/nimbus/osk.json')
        wr(d, {'theme': 'ember'})
        ls.set_switch(a.p, 'settings', True)
        ls.tick(a.p, A, now=T + 1_000_000)
        ls.set_switch(a.p, 'settings', False)
        wr(a.p.prefs, {'remote': {'hearth:osk': {'theme': {'v': 'slate', 't': T + 9_000_000}}}})
        ls.tick(a.p, A, now=T + 9_500_000)
        self.assertEqual(rd(d), {'theme': 'ember'})

    def test_decky_plugin_settings_without_sign_ins_and_plugins_listed(self):
        a, b = self.a, self.b
        for d in (a, b):
            for folder, name in (('decky-hearth-sync', 'Hearth Sync'), ('decky-hearth-link', 'Hearth Link')):
                wr(d.f('homebrew/plugins/%s/plugin.json' % folder), {'name': name})
        wr(a.f('homebrew/plugins/CSS Loader/plugin.json'), {'name': 'CSS Loader'})
        wr(a.f('homebrew/plugins/CSS Loader/package.json'), {'version': '2.1.0'})
        wr(a.f('homebrew/settings/CSS Loader/settings.json'),
           {'theme': 'Nord', 'apiToken': 'SECRET1', 'account': {'name': 'x', 'password': 'SECRET2'}, 'scale': 2})
        wr(a.f('homebrew/settings/CSS Loader/cache.json'), {'big': 'not a setting'})
        wr(a.f('homebrew/settings/decky-hearth-sync/settings.json'), {'autoAdd': False})
        wr(a.f('homebrew/settings/decky-hearth-sync/status.json'), {'adopted': True})
        wr(b.f('homebrew/settings/decky-hearth-sync/settings.json'), {'autoAdd': True, 'keep': 1})
        os.utime(b.f('homebrew/settings/decky-hearth-sync/settings.json'), (1000, 1000))
        for d in (a, b):
            ls.set_switch(d.p, 'plugins', True)
            ls.tick(d.p, A if d is a else B, now=T + 5_000_000)
        carry(a, b)
        shared = json.dumps(rd(b.p.prefs))
        for secret in ('SECRET1', 'SECRET2', 'apiToken', 'not a setting', 'adopted', 'link-sync'):
            self.assertNotIn(secret, shared)
        ls.tick(b.p, B, now=T + 5_100_000)
        got = rd(b.f('homebrew/settings/decky-hearth-sync/settings.json'))
        self.assertEqual(got, {'autoAdd': False, 'keep': 1}, "Hearth's own plugin: its settings, one key at a time")
        self.assertFalse(os.path.exists(b.f('homebrew/settings/CSS Loader')), "no settings for a plugin that isn't here")
        av = ls.available(b.p, [{'id': A, 'name': 'Ally', 'online': True}])
        self.assertEqual(av, [{'folder': 'CSS Loader', 'name': 'CSS Loader', 'version': '2.1.0', 'from': A, 'fromName': 'Ally', 'online': True}])
        # Installed later (through Decky): its settings come then, still without the sign-ins.
        wr(b.f('homebrew/plugins/CSS Loader/plugin.json'), {'name': 'CSS Loader'})
        ls.tick(b.p, B, now=T + 5_200_000)
        self.assertEqual(rd(b.f('homebrew/settings/CSS Loader/settings.json')), {'theme': 'Nord', 'scale': 2})
        self.assertEqual(ls.available(b.p, []), [])
        ls.set_switch(b.p, 'plugins', False)
        self.assertEqual(ls.available(b.p, []), [], 'nothing offered with the switch off')

    def test_a_sign_in_here_is_never_replaced(self):
        a = self.a
        wr(a.f('homebrew/plugins/X/plugin.json'), {'name': 'X'})
        wr(a.f('homebrew/settings/X/settings.json'), {'account': {'token': 'mine'}})
        ls.set_switch(a.p, 'plugins', True)
        wr(a.p.prefs, {'remote': {'decky:X:settings.json': {'account': {'v': {'name': 'other'}, 't': 9e12}, 'token': {'v': 'T', 't': 9e12}}}})
        ls.tick(a.p, A, now=T + 5_000_000)
        self.assertEqual(rd(a.f('homebrew/settings/X/settings.json')), {'account': {'token': 'mine'}})

    def test_plugin_files_are_checked_and_packed_for_decky(self):
        p = self.a.p
        files = [{'p': 'plugin.json', 'b64': base64.b64encode(b'{"name": "CSS Loader"}').decode()},
                 {'p': 'bin/run', 'x': True, 'b64': base64.b64encode(b'#!/bin/sh').decode()}]
        r = ls.pack_zip(p, 'CSS Loader', {'folder': 'CSS Loader', 'name': 'CSS Loader', 'version': '2.1.0', 'files': files})
        self.assertTrue(r['ok'], r)
        self.assertTrue(r['artifact'].startswith('file://'))
        z = zipfile.ZipFile(r['artifact'][7:])
        self.assertEqual(sorted(z.namelist()), ['CSS Loader/bin/run', 'CSS Loader/plugin.json'])
        self.assertEqual(len(r['hash']), 64)
        for bad in ('../../.bashrc', '/etc/passwd', 'a//b', 'a/./b'):
            r = ls.pack_zip(p, 'CSS Loader', {'folder': 'CSS Loader', 'files': files + [{'p': bad, 'b64': ''}]})
            self.assertFalse(r['ok'], bad)
        self.assertFalse(ls.pack_zip(p, 'decky-hearth-link', {'folder': 'decky-hearth-link', 'files': files})['ok'])
        self.assertFalse(ls.pack_zip(p, 'Other', {'folder': 'CSS Loader', 'files': files})['ok'])
        self.assertFalse(ls.pack_zip(p, 'X', {'folder': 'X', 'files': files[1:]})['ok'], 'no plugin.json')


class Backend(unittest.TestCase):
    """main.py's side: the saves switch, the sync calls and getting a plugin."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hl-')
        fake = types.ModuleType('decky')
        fake.DECKY_USER_HOME = os.path.join(self.tmp, 'home')
        fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(self.tmp, 'home/homebrew/settings/decky-hearth-link')
        fake.logger = types.SimpleNamespace(info=lambda m: None)
        sys.modules['decky'] = fake
        import importlib.util
        spec = importlib.util.spec_from_file_location('hearth_link_main_sync', HERE / 'main.py')
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)
        self.sent = []

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def link(self, answers):
        def f(cmd, args):
            self.sent.append((cmd, args))
            return answers.get(cmd, {'ok': False, 'error': 'Unknown command'})
        return f

    def test_saves_switch_sends_only_saves_follow(self):
        m = self.m
        r = m.call('storage.saves.set', {'follow': False, 'server': True, 'smb': True}, self.link({'storage.set': {'ok': True, 'data': {}}}))
        self.assertTrue(r['ok'])
        self.assertEqual(self.sent, [('storage.set', {'savesFollow': False})])
        for bad in ({}, {'follow': 'no'}, {'follow': 0}):
            self.assertFalse(m.call('storage.saves.set', bad, self.link({}))['ok'], bad)
        self.assertEqual(len(self.sent), 1)
        with self.assertRaises(ValueError):
            m.request('storage.set', {'savesFollow': False})

    def test_sync_calls(self):
        m = self.m
        st = {'ok': True, 'data': {'me': {'id': A, 'name': 'Ally'}, 'peers': [{'id': B, 'name': 'Desk', 'online': True}]}}
        info = m.sync_info(self.link({'status': st}))
        self.assertEqual(info['switches'], {'plugins': False, 'settings': False})
        self.assertEqual(m.sync_round(self.link({'status': st})), {'ok': True, 'skipped': True})
        self.assertFalse(m.set_sync('plugins', 'on')['ok'])
        self.assertTrue(m.set_sync('plugins', True)['ok'])
        self.assertFalse(m.fetch_plugin('../x', self.link({'status': st}))['ok'])
        self.assertIn('None of your devices', m.fetch_plugin('CSS Loader', self.link({'status': st}))['error'])
        ls.hio.write_json(m.sync_paths().prefs, {'remote': {'decky:plugins': {'CSS Loader': {'v': {'name': 'CSS Loader', 'version': '2', 'from': B}, 't': 5}}}})
        files = [{'p': 'plugin.json', 'b64': base64.b64encode(b'{"name": "CSS Loader"}').decode()}]
        r = m.fetch_plugin('CSS Loader', self.link({'status': st, 'decky.fetch': {'ok': True, 'data': {'folder': 'CSS Loader', 'name': 'CSS Loader', 'files': files}}}))
        self.assertTrue(r['ok'], r)
        self.assertIn(('decky.fetch', {'peer': B, 'folder': 'CSS Loader'}), self.sent)


if __name__ == '__main__':
    unittest.main()
