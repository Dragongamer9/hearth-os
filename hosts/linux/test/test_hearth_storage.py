"""Tests for Hearth Storage's pieces outside hosts/hearth-link: Mantel's switches (hearth-link-settings), Hearth Files'
hearth:// folders (read-only, copied through Hearth Link, media played from a 127.0.0.1 link), the bridge's calls,
the Game Mode page, the root helper and firewall wiring in both images."""
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
SRV = 'ab' * 32
SHARE = 'a1b2c3d4e5f6'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hls = load(SHELL / 'hearth-link-settings', 'hearth_link_settings_storage')
hf = load(SHELL / 'hearth-files', 'hearth_files_storage')


class Settings(unittest.TestCase):
    def test_switches_and_shared_folders(self):
        self.assertEqual(hls.request(['storage-set', 'server', 'on']), ('storage.set', {'server': True}))
        self.assertEqual(hls.request(['storage-set', 'smb', 'off']), ('storage.set', {'smb': False}))
        self.assertEqual(hls.request(['share-add', '/home/me/Videos']), ('storage.share.add', {'path': '/home/me/Videos'}))
        self.assertEqual(hls.request(['share-remove', SHARE]), ('storage.share.remove', {'id': SHARE}))
        self.assertEqual(hls.request(['backup-now']), ('storage.backup.now', {}))
        for bad in (['storage-set', 'folder', 'on'], ['storage-set', 'server', 'maybe'], ['share-add', 'relative'], ['share-remove', '../x']):
            with self.assertRaises(hls.Fail):
                hls.request(bad)


    def test_devices_and_removing_one(self):
        self.assertEqual(hls.request(['devices']), ('storage.devices', {}))
        self.assertEqual(hls.request(['storage-set', 'savesFollow', 'off']), ('storage.set', {'savesFollow': False}))
        self.assertEqual(hls.request(['device-remove', SRV]), ('storage.device.remove', {'id': SRV, 'backups': False, 'archive': False}))
        self.assertEqual(hls.request(['device-remove', SRV, 'backups', 'archive']), ('storage.device.remove', {'id': SRV, 'backups': True, 'archive': True}))
        for bad in (['device-remove', 'x'], ['device-remove', SRV, 'everything'], ['device-remove']):
            with self.assertRaises(hls.Fail):
                hls.request(bad)

    def test_mantel_page_has_the_sections(self):
        qml = (SHELL / 'SetLink.qml').read_text()
        for needle in ['"Servers & devices"', '"Backups & saves"', '"Saves follow you"', '["device-remove", page.removing, "backups", "archive"]', 'devJob.run(["devices"]']:
            self.assertIn(needle, qml)


class Files(unittest.TestCase):
    def setUp(self):
        self.calls = []
        self.answers = {}
        def fake(cmd, args, timeout=150):
            self.calls.append((cmd, args))
            a = self.answers.get(cmd, {})
            if isinstance(a, Exception):
                raise a
            return a
        self.p = mock.patch.object(hf, 'link_call', fake)
        self.p.start()

    def tearDown(self):
        self.p.stop()

    def test_addresses_are_checked(self):
        self.assertEqual(hf.parse_remote('hearth://%s/%s/Films/a.mkv' % (SRV, SHARE)), (SRV, SHARE, 'Films/a.mkv'))
        self.assertEqual(hf.parse_remote('hearth://%s/%s/' % (SRV, SHARE)), (SRV, SHARE, ''))
        for bad in ('hearth://%s/%s/../x' % (SRV, SHARE), 'hearth://%s/%s/.ssh' % (SRV, SHARE), 'hearth://nothex/%s/' % SHARE, '/etc', 'hearth://%s/%s/a//b' % (SRV, SHARE)):
            with self.assertRaises(hf.Fail):
                hf.parse_remote(bad)

    def test_listing_a_shared_folder(self):
        self.answers['storage.ls'] = {'share': 'Videos', 'entries': [{'name': 'Films', 'dir': True}, {'name': 'a.mkv', 'size': 5, 'mtime': 7}, {'name': '.x'}]}
        r = hf.remote_cmd('list', ['hearth://%s/%s/Films' % (SRV, SHARE), '--hidden'])
        self.assertEqual(self.calls[-1], ('storage.ls', {'server': SRV, 'share': SHARE, 'path': 'Films'}))
        self.assertFalse(r['writable'])
        self.assertEqual(r['parent'], 'hearth://%s/%s/' % (SRV, SHARE))
        self.assertEqual([e['name'] for e in r['entries']], ['Films', 'a.mkv'])
        self.assertEqual(r['entries'][1]['path'], 'hearth://%s/%s/Films/a.mkv' % (SRV, SHARE))
        self.assertEqual(hf.remote_cmd('list', ['/home']), None, 'local folders are left alone')

    def test_copying_goes_through_hearth_link_and_nothing_changes_the_server(self):
        d = tempfile.mkdtemp()
        r = hf.remote_cmd('copy', ['hearth://%s/%s/Films/a.mkv' % (SRV, SHARE), d])
        self.assertTrue(r['ok'] and r['background'])
        self.assertEqual(self.calls[-1], ('storage.get', {'server': SRV, 'share': SHARE, 'path': 'Films/a.mkv', 'place': os.path.realpath(d)}))
        self.assertEqual(hf.remote_cmd('conflicts', ['hearth://%s/%s/a' % (SRV, SHARE), d])['conflicts'], [])
        for c, args in (('move', ['hearth://%s/%s/a' % (SRV, SHARE), d]), ('copy', [d, 'hearth://%s/%s/' % (SRV, SHARE)]),
                        ('delete', ['hearth://%s/%s/a' % (SRV, SHARE)]), ('rename', ['hearth://%s/%s/a' % (SRV, SHARE), 'b'])):
            with self.assertRaises(hf.Fail):
                hf.remote_cmd(c, args)

    def test_media_plays_from_a_local_link(self):
        self.answers['storage.stream'] = {'url': 'http://127.0.0.1:4100/0123/a.mkv'}
        with mock.patch.object(hf, 'spawn') as sp:
            r = hf.remote_cmd('open', ['hearth://%s/%s/a.mkv' % (SRV, SHARE)])
        self.assertTrue(r['streaming'])
        self.assertEqual(sp.call_args[0][0][-1], 'http://127.0.0.1:4100/0123/a.mkv')
        self.answers['storage.stream'] = {'url': 'http://evil.example/x'}
        with mock.patch.object(hf, 'spawn') as sp, self.assertRaises(hf.Fail):
            hf.remote_cmd('open', ['hearth://%s/%s/a.mkv' % (SRV, SHARE)])
        sp.assert_not_called()
        r = hf.remote_cmd('open', ['hearth://%s/%s/notes.txt' % (SRV, SHARE)])
        self.assertEqual(self.calls[-1][0], 'storage.get')

    def test_places_list_the_servers_shared_folders(self):
        self.answers['storage.status'] = {'servers': [{'id': SRV, 'name': 'Basement PC', 'shares': [{'id': SHARE, 'name': 'Videos'}]}, {'id': 'cd' * 32, 'name': 'Asleep', 'shares': None}]}
        with mock.patch.object(hf.os.path, 'exists', return_value=True):
            p = hf.storage_places()
        self.assertEqual(len(p), 1)
        self.assertEqual(p[0]['label'], 'Videos on Basement PC')
        self.assertEqual(p[0]['section'], 'network')
        self.assertEqual(p[0]['path'], 'hearth://%s/%s/' % (SRV, SHARE))
        self.answers['storage.status'] = hf.Fail("Hearth Link isn't running")
        with mock.patch.object(hf.os.path, 'exists', return_value=True):
            self.assertEqual(hf.storage_places(), [])


class Wiring(unittest.TestCase):
    def test_game_mode_page(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for needle in ["function pageStorage()", "['storage','lib','Hearth Storage']", "if(/^stg[A-Z]/.test(a)) return stgAct(a, d);",
                       "'Move to Hearth Storage…'", "call('link.storage.servers'", "case 'stgSmbPw'", "secret:true"]:
            self.assertIn(needle, ui)

    def test_saves_follow_you_and_device_health(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        # Before a game: asked briefly, never in the way (a timer starts it anyway), and both kept when both changed.
        for needle in ["call('link.storage.saves.before'", "res({action:'slow'}), 6000", "stgSaveAsk(g, r, mod, false)", "call('link.storage.saves.pull'",
                       "call('link.storage.devices'", "call('link.storage.device.remove'", "call('link.storage.device.usage'",
                       "call('link.storage.cache.status'", "call('link.storage.updates.status'", "'storage.cache.set'", "'storage.updates.set'"]:
            self.assertIn(needle, ui)
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'link.rs').read_text()
        for c in ('"storage.saves.before"', '"storage.saves.pull"', '"storage.saves.resolve"', '"storage.devices"', '"storage.device.remove"', 'pub fn game_closed'):
            self.assertIn(c, rs)
        self.assertIn('crate::link::game_closed(&r.id)', (ROOT / 'hosts' / 'hearth-core' / 'src' / 'library.rs').read_text())
        saves = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'saves.rs').read_text()
        self.assertIn('pub const CHECK: Duration = Duration::from_millis(2500)', saves)
        # Device health only reads hearth-boot-guard's files.
        health = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'health.rs').read_text()
        self.assertNotIn('Command::new', health)
        self.assertNotIn('write(', health.split('#[cfg(test)]')[0].replace('write_atomic(&self.dir.join("devices.json")', ''))

    def test_bridge_passes_the_calls_on(self):
        rs = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'link.rs').read_text()
        for c in ('"storage.ls"', '"storage.get"', '"storage.archive"', '"storage.restore"', '"storage.backup.now"', '"storage.status"', '"storage.play"'):
            self.assertIn(c, rs)
        self.assertIn('http://127.0.0.1:', rs)

    def test_off_by_default_and_firewall_like_hearth_link(self):
        rs = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'storage.rs').read_text()
        self.assertIn('server: false', rs)
        self.assertIn('away: false, smb: false', rs)
        fw = (ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-link-firewall').read_text()
        self.assertIn("if app == 'smb':", fw)
        rules = (ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'rules.d' / '49-hearth-storage.rules').read_text()
        self.assertIn('/usr/libexec/nimbus/hearth-storage-helper', rules)
        self.assertIn('subject.local && subject.active && subject.isInGroup("wheel")', rules)
        for ed in ('anatase', 'peppermint'):
            b = (ROOT / 'os-bases' / ed / 'build.sh').read_text()
            self.assertIn('chmod 755 /usr/libexec/nimbus/hearth-storage-helper', b)

    def test_files_shows_the_section(self):
        qml = (SHELL / 'FileManager.qml').read_text()
        self.assertIn('network: "Hearth Storage"', qml)


if __name__ == '__main__':
    unittest.main()
