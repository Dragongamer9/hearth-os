"""Tests for hearth-admin's commands for Hearth's system tools (hosts/hearth-tools): confirm-password and wifi-psk
(Passwords & Keys), printer-pause/resume/cancel-job (Printers & Scanner) and journal-read (Logs). Argument checks
above all, with a fake command runner and a fake password checker; and the Welcome & Tips help index made from
tools/build-docs.py."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import subprocess
import types
import unittest
from contextlib import redirect_stdout
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class FakeRunner:
    def __init__(self, answers=None):
        self.calls = []
        self.answers = answers or {}

    def __call__(self, cmd, input=None, timeout=120, env=None):
        self.calls.append(list(cmd))
        for key, ans in self.answers.items():
            if tuple(cmd[:len(key)]) == key:
                return ans
        return 0, ''


class SystemTools(unittest.TestCase):
    UUID = '0f6a1c2e-3b4d-4e5f-8a9b-0c1d2e3f4a5b'

    def setUp(self):
        self.a = load(NIMBUS / 'hearth-admin', 'hearth_admin_tools')
        self.run = FakeRunner()
        self.a.admin_run = self.run
        me = types.SimpleNamespace(pw_name='sam', pw_uid=1000, pw_dir='/var/home/sam')
        self.a.caller = lambda: me
        self.a.time.sleep = lambda s: None
        self.checked = []

    def chkpwd(self, good):
        def fake(cmd, input=None, capture_output=False, **kw):
            self.checked.append((cmd, input))
            return subprocess.CompletedProcess(cmd, 0 if input == (good + '\0').encode() else 1, b'', b'')
        return mock.patch.object(self.a.subprocess, 'run', fake)

    def call(self, cmd, args, stdin=''):
        out = io.StringIO()
        with redirect_stdout(out), mock.patch.object(sys, 'stdin', io.StringIO(stdin)):
            try:
                self.a.system_tools(cmd, args)
                code = 0
            except SystemExit as e:
                code = e.code
        return code, out.getvalue()

    def test_password_is_checked_as_the_caller(self):
        with self.chkpwd('right'):
            code, out = self.call('confirm-password', [], json.dumps({'password': 'right'}) + '\n')
            self.assertEqual(code, 0, out)
            self.assertEqual(json.loads(out.strip().splitlines()[-1]), {'ok': True})
            self.assertEqual(self.checked[0][0][:4], ['runuser', '-u', 'sam', '--'])
            code, out = self.call('confirm-password', [], json.dumps({'password': 'wrong'}) + '\n')
            self.assertEqual(code, 1)
            self.assertIn('error: the password is incorrect', out)
            for bad in ('', 'nope', '{}', json.dumps({'password': ''})):
                self.assertEqual(self.call('confirm-password', [], bad + '\n')[0], 1, bad)
            self.assertEqual(self.call('confirm-password', ['extra'], json.dumps({'password': 'right'}))[0], 1)

    def test_wifi_password_needs_your_password_and_a_real_uuid(self):
        self.run.answers = {('nmcli', '-g', 'connection.type'): (0, '802-11-wireless\n'),
                            ('nmcli', '-s', '-g', '802-11-wireless-security.psk'): (0, 'hunter22\n')}
        with self.chkpwd('right'):
            code, out = self.call('wifi-psk', [self.UUID], json.dumps({'password': 'right'}) + '\n')
            self.assertEqual(code, 0, out)
            self.assertEqual(json.loads(out.strip().splitlines()[-1]), {'ok': True, 'psk': 'hunter22', 'kind': 'wpa'})
            self.assertIn(['nmcli', '-s', '-g', '802-11-wireless-security.psk', 'connection', 'show', 'uuid', self.UUID], self.run.calls)
            self.run.calls.clear()
            code, out = self.call('wifi-psk', [self.UUID], json.dumps({'password': 'wrong'}) + '\n')
            self.assertEqual(code, 1)
            self.assertNotIn('hunter22', out)
            self.assertEqual(self.run.calls, [], 'NetworkManager must not be asked before the password is right')
            for bad in ('--help', self.UUID + 'x', 'x' * 36, self.UUID.replace('-', ';', 1)):
                self.assertEqual(self.call('wifi-psk', [bad], json.dumps({'password': 'right'}))[0], 1, bad)
        self.run.answers = {('nmcli', '-g', 'connection.type'): (0, '802-3-ethernet\n')}
        with self.chkpwd('right'):
            self.assertEqual(self.call('wifi-psk', [self.UUID], json.dumps({'password': 'right'}))[0], 1)

    def test_printers(self):
        self.run.answers = {('lpstat', '-v'): (0, 'device for HP: ipp://10.0.0.5/ipp/print\n')}
        code, out = self.call('printer-pause', ['HP'])
        self.assertEqual(code, 0, out)
        self.assertEqual(self.run.calls[-1][0], 'cupsdisable')
        self.assertEqual(self.run.calls[-1][-1], 'HP')
        self.assertEqual(self.call('printer-resume', ['HP'])[0], 0)
        self.assertEqual(self.run.calls[-1], ['cupsenable', 'HP'])
        self.assertEqual(self.call('printer-pause', ['Nope'])[0], 1)
        self.assertEqual(self.call('printer-pause', ['-x'])[0], 1)
        self.assertEqual(self.call('printer-cancel-job', ['HP-12'])[0], 0)
        self.assertEqual(self.run.calls[-1], ['cancel', 'HP-12'])
        for bad in ('-a', 'HP', 'HP-x', 'HP-12;rm'):
            self.assertEqual(self.call('printer-cancel-job', [bad])[0], 1, bad)

    def test_journal_read_arguments(self):
        ja = self.a.journal_args
        self.assertEqual(ja(['--boot', '-1', '--priority', '3', '--lines', '500', '--ident', 'hhd', '--since-min', '60']),
                         ['journalctl', '--no-pager', '-o', 'json', '-n', '500', '-b', '-1', '-p', '3', '-t', 'hhd', '--since=-60min'])
        self.assertEqual(ja(['--lines', '10', '--boot', '0', '--priority', '7'])[:7], ['journalctl', '--no-pager', '-o', 'json', '-n', '10', '-b'])
        for bad in ([], ['--boot', '0'], ['--boot', '1', '--priority', '3', '--lines', '5'], ['--boot', '0', '--priority', '8', '--lines', '5'],
                    ['--boot', '0', '--priority', '3', '--lines', '99999'], ['--boot', '0', '--priority', '3', '--lines', '5', '--ident', 'a b'],
                    ['--boot', '0', '--priority', '3', '--lines', '5', '--file', '/etc/shadow'], ['--boot', '0', '--boot', '0', '--priority', '3', '--lines', '5'],
                    ['--boot', '0', '--priority', '3', '--lines']):
            self.assertIsNone(ja(bad), bad)
        with mock.patch.object(self.a.os, 'execvp') as ex:
            self.call('journal-read', ['--boot', '0', '--priority', '4', '--lines', '100'])
            ex.assert_called_once()
            self.assertEqual(ex.call_args[0][0], 'journalctl')

    def test_main_routes_to_system_tools(self):
        self.assertTrue(set(self.a.SYSTEM_TOOLS_COMMANDS) >= {'confirm-password', 'wifi-psk', 'journal-read'})
        src = (NIMBUS / 'hearth-admin').read_text()
        self.assertIn('if cmd in SYSTEM_TOOLS_COMMANDS:', src)


class HelpIndex(unittest.TestCase):
    def test_welcome_help_comes_from_the_docs(self):
        out = subprocess.run([sys.executable, str(ROOT / 'tools' / 'build-docs.py'), '--help-json', '-'], capture_output=True, text=True, check=True).stdout
        data = json.loads(out)
        ids = {(s['page'], s['id']) for s in data['sections']}
        self.assertIn(('troubleshooting', 'logs'), ids)
        self.assertIn(('using', 'apps'), ids)
        self.assertGreater(len(data['controls']), 5)
        self.assertTrue(all(s['text'] and '<' not in s['title'] for s in data['sections']))
        self.assertNotIn('releases', {s['page'] for s in data['sections']})


if __name__ == '__main__':
    unittest.main()
