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
        s = (SHELL / 'SettingsApp.qml').read_text()
        self.assertIn('link: "SetLink.qml"', s)
        self.assertTrue((SHELL / 'SetLink.qml').exists())

    def test_game_mode_page_and_library_entry(self):
        ui = (ROOT / 'ui' / 'index.html').read_text()
        for needle in ["function pageLink()", "['link','refresh','Hearth Link']", 'data-act="lkGet"', "call('link.status'", "call('link.ui.sync'"]:
            self.assertIn(needle, ui)

    def test_both_images_install_the_service(self):
        for ed in ('anatase', 'peppermint'):
            b = (ROOT / 'os-bases' / ed / 'build.sh').read_text()
            self.assertIn('ExecStart=/usr/bin/hearth-link serve', b)
            self.assertIn('default.target.wants/hearth-link.service', b)


if __name__ == '__main__':
    unittest.main()
