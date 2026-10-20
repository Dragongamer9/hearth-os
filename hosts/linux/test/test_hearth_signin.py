"""Tests for desktop/shell/hearth-signin: Game Mode's store sign-in request, taken by Desktop Mode (fresh and for a
known store only, used once), and handed to a windowed Hearth that's already open."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import tempfile
import time
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-signin'
AUTOSTART = ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc' / 'autostart'
STORES_RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'stores.rs'
HANDOFF_RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'signin_handoff.rs'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_signin', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_signin', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-signin-test-')
        os.environ['HOME'] = os.path.join(self.tmp, 'home')
        os.environ['XDG_STATE_HOME'] = os.path.join(self.tmp, 'state')
        os.environ['XDG_RUNTIME_DIR'] = os.path.join(self.tmp, 'run')
        os.environ['XDG_DATA_HOME'] = os.path.join(self.tmp, 'data')
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        self.s = load()

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def request(self, store, at):
        p = self.s.request_path()
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            json.dump({'store': store, 'at': at}, f)
        return p


class Freshness(Base):
    def test_where_game_mode_writes_it(self):
        self.assertEqual(self.s.request_path(), os.path.join(self.tmp, 'state', 'hearth', 'desktop-signin.json'))
        del os.environ['XDG_STATE_HOME']
        self.assertEqual(self.s.request_path(), os.path.join(self.tmp, 'home', '.local', 'state', 'hearth', 'desktop-signin.json'))

    def test_only_requests_under_ten_minutes_old(self):
        now = 1_800_000_000
        r = lambda store, at: json.dumps({'store': store, 'at': at})
        self.assertEqual(self.s.parse(r('epic', now), now), 'epic')
        self.assertEqual(self.s.parse(r('steam', now - 599), now), 'steam')
        self.assertIsNone(self.s.parse(r('gog', now - 600), now))
        self.assertIsNone(self.s.parse(r('gog', now - 86400), now))
        self.assertEqual(self.s.parse(r('xbox', now + 30), now), 'xbox')       # the clock a little ahead
        self.assertIsNone(self.s.parse(r('xbox', now + 3600), now))
        self.assertEqual(self.s.parse(r('epic', (now - 5) * 1000), now), 'epic')   # milliseconds
        self.assertIsNone(self.s.parse(r('epic', now - 61), now, self.s.ROUTE_MAX_AGE))

    def test_only_stores_hearth_knows(self):
        now = 1_800_000_000
        for bad in ('amazon', 'Steam', 'epic ', '../epic', '', None, 5, ['epic']):
            self.assertIsNone(self.s.parse(json.dumps({'store': bad, 'at': now}), now), bad)
        for text in ('', 'nope', '[]', '"epic"', '{"store": "epic"}', '{"store": "epic", "at": "now"}', '{"store": "epic", "at": true}'):
            self.assertIsNone(self.s.parse(text, now), text)

    def test_the_same_stores_as_the_rust_host(self):
        m = re.search(r'pub const STORES: \[&str; \d+\] = \[([^\]]*)\]', STORES_RS.read_text())
        self.assertTrue(m)
        self.assertEqual(tuple(re.findall(r'"([a-z]+)"', m.group(1))), self.s.STORES)
        self.assertEqual(self.s.VALID, self.s.STORES + ('steam',))
        rs = HANDOFF_RS.read_text()
        self.assertIn('pub const MAX_AGE_SECS: u64 = %d;' % self.s.MAX_AGE, rs)
        self.assertIn('pub const ROUTE_MAX_AGE_SECS: u64 = %d;' % self.s.ROUTE_MAX_AGE, rs)
        self.assertIn('join("desktop-signin.json")', rs)
        self.assertIn('join("hearth-signin-route.json")', rs)


class Take(Base):
    def test_taken_once(self):
        p = self.request('gog', time.time())
        self.assertEqual(self.s.take(), 'gog')
        self.assertFalse(os.path.exists(p))
        self.assertIsNone(self.s.take())
        self.assertEqual(os.listdir(os.path.dirname(p)), [])

    def test_a_stale_or_bad_request_is_removed_too(self):
        p = self.request('gog', time.time() - 3600)
        self.assertIsNone(self.s.take())
        self.assertFalse(os.path.exists(p))
        p = self.request('amazon', time.time())
        self.assertIsNone(self.s.take())
        self.assertFalse(os.path.exists(p))

    def test_no_request(self):
        self.assertIsNone(self.s.take())


class Start(Base):
    def fake_proc(self, procs):
        proc = os.path.join(self.tmp, 'proc')
        for pid, argv in procs.items():
            d = os.path.join(proc, str(pid))
            os.makedirs(d)
            with open(os.path.join(d, 'cmdline'), 'wb') as f:
                f.write(b'\0'.join(a.encode() for a in argv) + b'\0')
        os.makedirs(os.path.join(proc, 'self'), exist_ok=True)
        return proc

    def test_finds_a_windowed_hearth(self):
        proc = self.fake_proc({10: ['/usr/bin/nimbus'], 11: ['quickshell', '-p', 'x'], 12: ['/usr/bin/nimbus', '--windowed']})
        self.assertEqual(self.s.running_hearth(proc), 12)
        proc2 = os.path.join(self.tmp, 'proc2')
        os.rename(proc, proc2)
        os.makedirs(proc)
        self.assertIsNone(self.s.running_hearth(proc))     # nothing running
        self.assertIsNone(self.s.running_hearth(proc2, uid=os.getuid() + 1))   # someone else's

    def test_full_screen_hearth_is_not_a_window(self):
        proc = self.fake_proc({20: ['/usr/lib/nimbus/nimbus'], 21: ['vim', '--windowed']})
        self.assertIsNone(self.s.running_hearth(proc))

    def test_opens_hearth_at_the_sign_in(self):
        app = os.path.join(self.tmp, 'nimbus')
        with open(app, 'w') as f:
            f.write('#!/bin/sh\n')
        os.chmod(app, 0o755)
        os.environ['NIMBUS_APP'] = app
        self.request('epic', time.time())
        with mock.patch.object(self.s, 'running_hearth', return_value=None), mock.patch.object(self.s.subprocess, 'Popen') as popen:
            self.assertEqual(self.s.desktop_start(wait=0), 0)
        popen.assert_called_once()
        self.assertEqual(popen.call_args[0][0], [app, '--windowed', '--signin', 'epic'])
        self.assertFalse(os.path.exists(self.s.request_path()))

    def test_hands_it_to_the_hearth_window_that_is_open(self):
        self.request('steam', time.time())
        with mock.patch.object(self.s, 'running_hearth', return_value=4242), mock.patch.object(self.s.subprocess, 'Popen') as popen:
            self.assertEqual(self.s.desktop_start(wait=0), 0)
        popen.assert_not_called()
        with open(self.s.route_path()) as f:
            v = json.load(f)
        self.assertEqual(v['store'], 'steam')
        self.assertEqual(self.s.parse(json.dumps(v), time.time(), self.s.ROUTE_MAX_AGE), 'steam')

    def test_nothing_asked_nothing_opened(self):
        with mock.patch.object(self.s.subprocess, 'Popen') as popen:
            self.assertEqual(self.s.desktop_start(wait=0), 0)
        popen.assert_not_called()
        self.assertFalse(os.path.exists(self.s.route_path()))

    def test_stale_request_opens_nothing(self):
        self.request('epic', time.time() - 700)
        with mock.patch.object(self.s.subprocess, 'Popen') as popen:
            self.assertEqual(self.s.desktop_start(wait=0), 0)
        popen.assert_not_called()

    def test_desktop_mode_runs_it(self):
        text = AUTOSTART.read_text()
        self.assertRegex(text, r'python3 "\$SHARE/desktop/shell/hearth-signin" desktop-start[^\n]*&\n')


if __name__ == '__main__':
    unittest.main()
