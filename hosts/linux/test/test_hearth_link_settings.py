"""Tests for hearth-link-settings (Mantel's Settings › Hearth Link) and Hearth Link's pieces outside its crate: the
calls it makes to the service, checked arguments, the code kept off the command line; the Game Mode page and the
images' user service."""
import importlib.machinery
import importlib.util
import json
import pathlib
import subprocess
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hls = load(SHELL / 'hearth-link-settings', 'hearth_link_settings')
ID = 'ab' * 32


class Requests(unittest.TestCase):
    def test_simple_commands(self):
        self.assertEqual(hls.request(['status']), ('status', {}))
        self.assertEqual(hls.request(['pair-open']), ('pair.open', {}))
        self.assertEqual(hls.request(['sync']), ('sync.now', {}))

    def test_join_takes_the_code_from_stdin(self):
        self.assertEqual(hls.request(['join', ID], '123 456\n'), ('pair.join', {'id': ID, 'code': '123456'}))
        self.assertEqual(hls.request(['join', ID + '@https://relay.example.com'], '123456')[1]['id'], ID + '@https://relay.example.com')
        with self.assertRaises(hls.Fail):
            hls.request(['join', ID], '12345')
        with self.assertRaises(hls.Fail):
            hls.request(['join', '; rm -rf /'], '123456')

    def test_short_codes(self):
        # Pairing from somewhere else: the short code on stdin, checked here (no 0/O/1/I/L, the check character).
        self.assertEqual(hls.request(['pair-open-internet']), ('pair.open', {'internet': True}))
        self.assertEqual(hls.request(['join-code'], '234 567 89a9\n'), ('pair.join', {'code': '234-567-89A9'}))
        for bad in ('234-567-89AA', '234-567-89A0', '234-567', '', 'K7M-4QX-92HC-X'):
            with self.assertRaises(hls.Fail):
                hls.request(['join-code'], bad)
        with self.assertRaises(hls.Fail):
            hls.request(['join-code', 'extra'], '234-567-89A9')

    def test_settings_are_checked(self):
        self.assertEqual(hls.request(['set', 'sync', 'off']), ('settings.set', {'sync': False}))
        self.assertEqual(hls.request(['set', 'name', 'Deck']), ('settings.set', {'name': 'Deck'}))
        with self.assertRaises(hls.Fail):
            hls.request(['set', 'streams', '9'])
        with self.assertRaises(hls.Fail):
            hls.request(['set', 'sync', 'maybe'])

    def test_copies(self):
        self.assertEqual(hls.request(['pause', 'a1b2c3d4e5f60718']), ('transfer.pause', {'id': 'a1b2c3d4e5f60718'}))
        with self.assertRaises(hls.Fail):
            hls.request(['resume', '../x'])

    def test_main_calls_the_service_and_prints_one_line(self):
        out = subprocess.CompletedProcess([], 0, stdout='{"me": {"name": "Deck"}}\n', stderr='')
        with mock.patch.object(hls.os.path, 'exists', return_value=True), mock.patch.object(hls.subprocess, 'run', return_value=out) as run, \
             mock.patch('builtins.print') as p:
            self.assertEqual(hls.main(['x', 'status']), 0)
        self.assertEqual(run.call_args[0][0][1:3], ['call', 'status'])
        self.assertEqual(json.loads(p.call_args[0][0])['me']['name'], 'Deck')
        bad = subprocess.CompletedProcess([], 1, stdout='{"error": "Hearth Link isn\'t running"}\n', stderr='')
        with mock.patch.object(hls.os.path, 'exists', return_value=True), mock.patch.object(hls.subprocess, 'run', return_value=bad), \
             mock.patch('builtins.print') as p:
            self.assertEqual(hls.main(['x', 'status']), 1)
        self.assertTrue(p.call_args[0][0].startswith('error: '))


class Wiring(unittest.TestCase):
    def test_mantel_lists_the_page(self):
        s = (SHELL / 'settings-pages.js').read_text()   # Hearth Settings' page registry (the redesign)
        self.assertIn('{ id: "link", file: "SetLink.qml", cat: "link"', s)
        self.assertTrue((SHELL / 'SetLink.qml').exists())

    def test_game_mode_page_and_library_entry(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for needle in ["function pageLink()", "['link','refresh','Hearth Link']", 'data-act="lkGet"', "call('link.status'", "call('link.ui.sync'"]:
            self.assertIn(needle, ui)

    def test_my_devices_and_friends_devices(self):
        # Pairing a friend's device: the last word "friend" (mine: false); without it, one of your devices (the
        # service's default). Only pairing takes it.
        self.assertEqual(hls.request(['pair-open', 'friend']), ('pair.open', {'mine': False}))
        self.assertEqual(hls.request(['pair-open-internet', 'friend']), ('pair.open', {'internet': True, 'mine': False}))
        self.assertEqual(hls.request(['join', ID, 'friend'], '123456'), ('pair.join', {'id': ID, 'code': '123456', 'mine': False}))
        self.assertEqual(hls.request(['join-code', 'friend'], '234-567-89A9'), ('pair.join', {'code': '234-567-89A9', 'mine': False}))
        self.assertNotIn('mine', hls.request(['join', ID], '123456')[1])
        self.assertEqual(hls.request(['make-friend', ID]), ('peer.set', {'id': ID, 'friend': True}))
        for bad in (['make-friend', '; rm -rf /'], ['unpair', ID, 'friend'], ['status', 'friend']):
            with self.assertRaises(hls.Fail):
                hls.request(bad, '123456')
        # Both screens ask, and mark a friend's device and one paired through another of yours.
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for needle in ["function lkWhose(", "{label:'My device'", "{label:'A friend\\'s device'", "'link.pair.open', {mine}", "mine: LK.joinMine!==false",
                       "p.friend?'Friend\\'s device · '", "paired through ${esc(p.introducedBy)}", "lkCall('link.peer.set',{id:p.id, friend:true}"]:
            self.assertIn(needle, ui)
        qml = (SHELL / 'SetLink.qml').read_text()
        for needle in ['property bool friendMode: false', 'title: "Pairing a friend\'s device"', 'page.f(["pair-open"])', 'act.run(page.f(args)', 'modelData.friend ? "Friend\'s device · "', 'paired through " + modelData.introducedBy']:
            self.assertIn(needle, qml)
        # The service: what a friend's device may ask, and that your devices share their lists.
        fam = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'family.rs').read_text()
        self.assertIn('pub const FRIEND_OPS: &[&str] = &["hello", "ping", "unpair", "games", "manifest", "hashes", "chunks", "offer"];', fam)
        node = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'node.rs').read_text()
        self.assertIn('if !crate::family::friend_may(&op) && !crate::lock(&self.peers).is_mine(&peer)', node)
        self.assertIn('"family.sync" => self.clone().family_answer(peer, req).await', node)

    def test_easy_pairing_in_both_modes(self):
        # Game Mode: the devices on this network to pick, the number pad for the 6-digit code, short codes typed
        # with the same check as the service; Mantel the same.
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for needle in ["mode:'number'", "call('link.pair.join'", "'link.pair.open', {internet:true}", "const LK_ABC = '23456789ABCDEFGHJKMNPQRSTUVWXYZ'", 'Wrong code', 'Paired ✓ with']:
            self.assertIn(needle, ui)
        qml = (SHELL / 'SetLink.qml').read_text()
        for needle in ['"pair-open-internet"', '["join-code"]', 'readonly property string abc: "23456789ABCDEFGHJKMNPQRSTUVWXYZ"', 'Paired ✓ with']:
            self.assertIn(needle, qml)
        rs = (ROOT / 'hosts' / 'hearth-link' / 'src' / 'shortcode.rs').read_text()
        self.assertIn('pub const ALPHABET: &[u8; 31] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";', rs)
        self.assertEqual(hls.ABC, '23456789ABCDEFGHJKMNPQRSTUVWXYZ')

    def test_both_images_install_the_service(self):
        for ed in ('anatase', 'peppermint'):
            b = (ROOT / 'os-bases' / ed / 'build.sh').read_text()
            self.assertIn('ExecStart=/usr/bin/hearth-link serve', b)
            self.assertIn('default.target.wants/hearth-link.service', b)


if __name__ == '__main__':
    unittest.main()
