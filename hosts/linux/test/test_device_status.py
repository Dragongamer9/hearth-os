"""Settings › Hearth Link › Your devices (docs/HEARTH-LINK.md "Your devices"): hearth-device-status (the status JSON's
shape, nothing shared when the switch is off or desktop.json can't be read, the last hour kept only while sharing,
no secrets), Hearth Link's status.get (paired devices only), and the page (stand-in devices, writes nothing)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LINK = ROOT / 'hosts' / 'hearth-link' / 'src'
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import devices_sample  # noqa: E402


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_device_status', str(SHELL / 'hearth-device-status'))
    spec = importlib.util.spec_from_loader('hearth_device_status', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Stat:
    def __init__(self, total_gb, free_gb):
        self.f_frsize = 4096
        self.f_blocks = int(total_gb * 1e9 / 4096)
        self.f_bavail = int(free_gb * 1e9 / 4096)


class Helper(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.tmp = tempfile.mkdtemp()
        self.env = mock.patch.dict(os.environ, {'HOME': self.tmp, 'XDG_RUNTIME_DIR': self.tmp, 'XDG_CACHE_HOME': os.path.join(self.tmp, 'cache')})
        self.env.start()
        self.settings = os.path.join(self.tmp, '.local', 'share', 'nimbus', 'desktop.json')
        os.makedirs(os.path.dirname(self.settings))

    def tearDown(self):
        self.env.stop()
        import shutil
        shutil.rmtree(self.tmp, ignore_errors=True)

    def write_settings(self, text):
        with open(self.settings, 'w') as f:
            f.write(text)

    def test_sharing_default_on_never_written_off_when_unreadable(self):
        self.assertTrue(self.m.sharing())                       # no file: the default
        self.assertFalse(os.path.exists(self.settings))          # ...and nothing written
        self.write_settings('{"look": "steam"}')
        self.assertTrue(self.m.sharing())                        # no key: the default
        self.write_settings('{"shareStatus": false}')
        self.assertFalse(self.m.sharing())
        self.write_settings('{"shareStatus": true}')
        self.assertTrue(self.m.sharing())
        self.write_settings('{"shareSta')                         # damaged: we can't tell what you chose
        self.assertFalse(self.m.sharing())
        self.write_settings('[1, 2]')
        self.assertFalse(self.m.sharing())

    def test_off_answers_nothing_but_that(self):
        self.write_settings('{"shareStatus": false}')
        self.assertEqual(self.m.status(), {'protocol': 1, 'shared': False})

    def fake_device(self):
        m = self.m
        p = [mock.patch.object(m.stats, 'cpu_times', side_effect=[(10, 100), (30, 200)]),
             mock.patch.object(m.stats, 'hwmon_temps', return_value={'cpu': 61, 'gpu': 66}),
             mock.patch.object(m.stats, 'memory', return_value=(9.8, 15.4)),
             mock.patch.object(m.stats, 'gpu_busy', return_value=74),
             mock.patch.object(m.share, 'battery', return_value=({'percent': 46, 'charging': False}, False)),
             mock.patch.object(m.share, 'gpu', return_value={'vendor': 'amd', 'vramMB': 4096}),
             mock.patch.object(m.share, 'version', return_value='1.6'),
             mock.patch.object(m, 'amd_vram', return_value=(3100, 4096)),
             mock.patch.object(m, 'procs', return_value=[('gamescope-wl', 'gamescope -e'), ('reaper', 'reaper SteamLaunch AppId=1145350 -- game')]),
             mock.patch.object(m, 'steam_libraries', return_value=[]),
             mock.patch.object(m, 'drives', return_value=[{'name': 'System', 'used': 300.0, 'total': 1000.0}]),
             mock.patch.object(m, 'uptime', return_value=9240),
             mock.patch.object(m, 'kind', return_value='handheld'),
             mock.patch.object(m.time, 'sleep')]
        for x in p:
            x.start()
            self.addCleanup(x.stop)

    def test_status_shape(self):
        self.fake_device()
        s = self.m.status(now_ms=1_000_000)
        for k in ('protocol', 'shared', 'name', 'model', 'kind', 'version', 'mode', 'playing', 'battery', 'cpu', 'gpu', 'ram',
                  'drives', 'uptime', 'history', 'updated'):
            self.assertIn(k, s)
        self.assertEqual(s['protocol'], 1)
        self.assertTrue(s['shared'])
        self.assertEqual(s['mode'], 'game')
        self.assertEqual(s['playing'], {'title': 'A Steam game', 'appid': 1145350})
        self.assertEqual(s['battery'], {'percent': 46, 'charging': False, 'pluggedIn': False})
        self.assertEqual(s['cpu'], {'use': 20, 'temp': 61})
        self.assertEqual(s['gpu'], {'vendor': 'amd', 'use': 74, 'temp': 66, 'vramUsed': 3100, 'vramTotal': 4096})
        self.assertEqual(s['ram'], {'used': 9.8, 'total': 15.4})
        self.assertEqual(s['uptime'], 9240)
        json.dumps(s)
        # No secrets: no addresses, accounts, keys, paths.
        text = json.dumps(s).lower()
        for bad in ('"ip', 'address', 'mac', 'password', 'token', 'key', '/home', 'user'):
            self.assertNotIn(bad, text, bad)

    def test_modes_and_playing(self):
        m = self.m
        self.assertEqual(m.mode([('labwc', 'labwc')]), 'desktop')
        self.assertEqual(m.mode([('gamescope', 'gamescope')]), 'game')
        self.assertEqual(m.mode([('systemd', '')]), 'server')
        lib = os.path.join(self.tmp, 'steamapps')
        os.makedirs(lib)
        with open(os.path.join(lib, 'appmanifest_1091500.acf'), 'w') as f:
            f.write('"AppState" { "appid" "1091500" "name" "Cyberpunk 2077" }')
        self.assertEqual(m.playing([('reaper', 'SteamLaunch AppId=1091500 --')], [lib]), {'title': 'Cyberpunk 2077', 'appid': 1091500})
        self.assertIsNone(m.playing([('reaper', 'SteamLaunch AppId=1628350 --')], [lib]))   # the runtime isn't a game
        open(os.path.join(self.tmp, 'hearth-playing'), 'w').close()
        self.assertEqual(m.playing([], [lib]), {'title': 'A game', 'appid': None})

    def test_drives_once_each_named_without_paths(self):
        mounts = '\n'.join(['/dev/nvme0n1p3 / btrfs rw 0 0', '/dev/nvme0n1p3 /home btrfs rw 0 0', '/dev/nvme0n1p2 /boot ext4 rw 0 0',
                            '/dev/sda1 /run/media/chris/Games ext4 rw 0 0', 'tmpfs /tmp tmpfs rw 0 0', '/dev/sdb1 /srv/secret ext4 rw 0 0'])
        sizes = {'/': Stat(2000, 1500), '/run/media/chris/Games': Stat(4000, 600)}
        d = self.m.drives(mounts, statvfs=lambda p: sizes[p])
        self.assertEqual([x['name'] for x in d], ['System', 'Games'])
        self.assertEqual(d[0]['total'], 2000.0)
        self.assertEqual(d[1]['used'], 3400.0)

    def test_history_only_while_sharing_and_only_the_last_hour(self):
        path = os.path.join(self.tmp, 'h.json')
        make = lambda t: {'t': t, 'cpu': 5}
        hour = 3600 * 1000
        self.m.sample(now_ms=10 * hour, path=path, make=make)
        self.m.sample(now_ms=10 * hour + 60000, path=path, make=make)
        self.assertEqual(len(self.m.history(10 * hour + 60000, path)), 2)
        self.assertEqual(len(self.m.history(11 * hour + 30000, path)), 1)    # the first is more than an hour old
        self.write_settings('{"shareStatus": false}')
        self.assertEqual(self.m.sample(now_ms=10 * hour + 120000, path=path, make=make), {'kept': False})
        self.assertFalse(os.path.exists(path))                              # off: nothing kept

    def test_offline_devices_keep_their_last_numbers(self):
        live = {'devices': [{'id': 'a', 'name': 'PC', 'online': True, 'lastSeen': 5, 'shared': True, 'status': {'mode': 'game'}},
                            {'id': 'b', 'name': 'Laptop', 'online': False, 'lastSeen': 3}]}
        out, cache = self.m.merge(live, {'b': {'status': {'mode': 'desktop'}, 'at': 3}}, {'a': {'shares': [{'id': '1', 'name': 'Media'}], 'wake': True}})
        self.assertEqual(cache['a']['status'], {'mode': 'game'})
        self.assertEqual(out[1]['status'], {'mode': 'desktop'})
        self.assertTrue(out[1]['stale'])
        self.assertEqual(out[0]['shares'][0]['name'], 'Media')

    def test_sample_file_for_the_page(self):
        p = os.path.join(self.tmp, 's.json')
        with open(p, 'w') as f:
            json.dump(devices_sample.sample(), f)
        with mock.patch.dict(os.environ, {'HEARTH_DEVICES_SAMPLE': p}):
            d = self.m.devices()
        kinds = {x['name']: (x['online'], x['status']['mode'], x['status']['kind']) for x in d['devices']}
        self.assertEqual(kinds['ROG Ally'], (True, 'game', 'handheld'))
        self.assertEqual(kinds['chris-hearth'], (True, 'game', 'desktop'))
        self.assertFalse(kinds['Laptop'][0])

    def test_wake_takes_only_a_device_id(self):
        with self.assertRaises(self.m.Fail):
            self.m.main(['wake', '--help'])
        with self.assertRaises(self.m.Fail):
            self.m.main(['wake', 'a; rm -rf /'])


class Link(unittest.TestCase):
    def test_paired_devices_only_over_hearth_link(self):
        st = (LINK / 'status.rs').read_text()
        node = (LINK / 'node.rs').read_text()
        # A connection from a device that isn't paired is refused before any request is read...
        self.assertIn('alpn != crate::ALPN_LINK || !crate::lock(&self.peers).is_paired(&remote)', node)
        self.assertIn('o if o.starts_with("status.") => self.clone().status_answer(peer, op, req).await,', node)
        self.assertIn('c if c.starts_with("status.") => self.status_command(c, a).await,', node)
        # ...and status.get checks again, and runs the helper only for a paired device.
        get = st[st.index('"status.get" =>'):st.index('_ => Err', st.index('"status.get" =>'))]
        self.assertIn('is_paired(peer)', get)
        self.assertIn('if paired {', get)
        self.assertIn('if !paired { return Err("Not a paired device".into()); }', st)
        # Not shared: only that, whatever the helper said.
        self.assertIn('Ok(_) => Ok(json!({"protocol": 1, "shared": false})),', st)
        # No port opened for this: nothing listens (only Hearth Link's own connection).
        self.assertNotIn('TcpListener', st)
        self.assertNotIn('bind(', st)


class Page(unittest.TestCase):
    def test_registered_under_hearth_link_with_a_home_card(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        self.assertIn('{ id: "devices", file: "SetDevices.qml", cat: "link", parent: "link", label: "Your devices"', reg)
        home = (SHELL / 'SetHome.qml').read_text()
        self.assertIn('title: "Your devices"; linkText: "Open"; linkTarget: "devices"', home)
        link = (SHELL / 'SetLink.qml').read_text()
        self.assertIn('SettingsNav.openPage("devices")', link)
        self.assertIn('title: "Let my other devices see this device\'s status"', link)

    def test_opening_it_writes_nothing(self):
        page = (SHELL / 'SetDevices.qml').read_text()
        writes = re.findall(r'\bSettings\.\w+\s*=(?!=)', page)
        self.assertEqual(writes, ['Settings.shareStatus ='])
        self.assertIn('onToggled: v => Settings.shareStatus = v', page)
        s = (SHELL / 'Settings.qml').read_text()
        self.assertIn('property bool shareStatus: true', s)
        self.assertIn('"shareStatus"', s)
        self.assertIn('onShareStatusChanged: save("shareStatus")', s)

    def test_page_shows_what_was_asked(self):
        page = (SHELL / 'SetDevices.qml').read_text()
        for need in ('"Offline or asleep · seen "', 'game: "Game Mode", desktop: "Desktop Mode", server: "Server mode"', '"Playing " + s.playing.title',
                     '"Battery " + s.battery.percent', '"RAM "', 'vramTotal', 'page.cur.status.drives', 'upText(page.cur.status.uptime)',
                     'title: "The last hour"', 'key: "cpu"', 'key: "battery"', 'title: "Wake it"', 'title: "Remote desktop"', '"hearth://"'):
            self.assertIn(need, page, need)


if __name__ == '__main__':
    unittest.main()
