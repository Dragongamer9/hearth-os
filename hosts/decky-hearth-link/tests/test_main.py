"""Tests for Hearth Link's Decky backend (hosts/decky-hearth-link/main.py). No Steam, no Decky: a stand-in `decky`
module and a stand-in Hearth Link on a real Unix socket."""
import sys
sys.dont_write_bytecode = True
import asyncio
import importlib.util
import json
import os
import pathlib
import shutil
import socket
import subprocess
import tempfile
import threading
import types
import unittest

HERE = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE.parents[1] / 'hosts' / 'linux' / 'desktop' / 'shell'))   # hearth_settings_io (packed next to main.py)
os.environ.setdefault('XDG_STATE_HOME', tempfile.mkdtemp())
LOG = []
A = 'a' * 64
B = 'b' * 64


def load(tmp):
    fake = types.ModuleType('decky')
    fake.DECKY_USER_HOME = os.path.join(tmp, 'home')
    fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(tmp, 'settings')
    fake.logger = types.SimpleNamespace(info=LOG.append)
    sys.modules['decky'] = fake
    spec = importlib.util.spec_from_file_location('hearth_link_main', HERE / 'main.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


FULL = {
    'me': {'id': A, 'name': 'Ally', 'ticket': 'SECRET-TICKET', 'relay': 'https://relay.example', 'port': 4242, 'kind': 'handheld'},
    'settings': {'relayUrl': 'https://relay.example'},
    'pairing': {'code': '123456', 'until': 99, 'triesLeft': 3, 'netCode': 'AB12-CD34', 'netUntil': 100},
    'peers': [{'id': B, 'name': 'Desk', 'online': True, 'lan': True, 'lastSeen': 5, 'signedIn': {'steam': True}}],
    'nearby': [{'id': 'c' * 64, 'name': 'Laptop', 'pairing': True, 'addr': '192.168.1.9:4242'}],
    'transfers': [{'id': '0123456789abcdef', 'peer': B, 'game': 'steam:620', 'title': 'Portal 2', 'store': 'steam', 'state': 'copying',
                   'total': 100, 'done': 40, 'speed': 10, 'eta': 6, 'place': '/home/you/.steam', 'relay': False, 'from': 'Desk'}],
    'offers': [{'id': 'fedcba9876543210', 'peer': B, 'from': 'Desk', 'game': {'id': 'gog:1', 'title': 'Celeste', 'store': 'gog', 'size': 9, 'meta': {'x': 1}}}],
    'storage': {'config': {'server': False, 'savesFollow': True, 'folder': '/srv'}, 'user': 'you',
                'servers': [{'id': B, 'name': 'Desk', 'free': 5, 'canWake': True, 'at': 3, 'shares': [{'path': '/srv/x'}]}], 'jobs': []},
    'sync': {'fields': 3, 'lastChange': 7}, 'playing': False,
}


class FakeLink:
    """A Hearth Link stand-in: answers each request line from `answers` and keeps what it was asked."""
    def __init__(self, path, answers):
        self.path, self.answers, self.got = path, answers, []
        os.makedirs(os.path.dirname(path), mode=0o700, exist_ok=True)
        os.chmod(os.path.dirname(path), 0o700)
        self.srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.srv.bind(path)
        self.srv.listen(4)
        threading.Thread(target=self.loop, daemon=True).start()

    def loop(self):
        while True:
            try:
                c, _ = self.srv.accept()
            except OSError:
                return
            f = c.makefile('rb')
            req = json.loads(f.readline())
            self.got.append(req)
            a = self.answers.get(req['cmd'], {'ok': False, 'error': 'Unknown command'})
            c.sendall((json.dumps(dict(a, id=req['id'])) + '\n').encode())
            c.close()

    def close(self):
        self.srv.close()


class Backend(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hl-')
        os.makedirs(os.path.join(self.tmp, 'home'))
        self.m = load(self.tmp)
        self.sock = os.path.join(self.tmp, 'run', 'hearth-link', 'link.sock')
        os.environ['HEARTH_LINK_SOCKET'] = self.sock

    def tearDown(self):
        os.environ.pop('HEARTH_LINK_SOCKET', None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_only_allowed_commands_with_checked_arguments(self):
        r = self.m.request
        for bad in ('unpair', 'settings.set', 'storage.set', 'storage.smb.show', 'storage.smb.password', 'ticket',
                    'storage.device.remove', 'storage.share.add', 'ai.ask', 'fleet.run', '', None):
            with self.assertRaises(ValueError, msg=bad):
                r(bad, {})
        self.assertEqual(r('send', {'peer': B, 'game': 'steam:620', 'extra': '/etc/passwd'}), {'peer': B, 'game': 'steam:620'})
        with self.assertRaises(ValueError):
            r('send', {'peer': '../x', 'game': 'steam:620'})
        with self.assertRaises(ValueError):
            r('get', {'peer': B, 'game': 'no colon'})
        with self.assertRaises(ValueError):
            r('transfer.cancel', {'id': '../../x'})
        self.assertEqual(r('get', {'peer': B, 'game': 'steam:620', 'title': 'Portal\n2', 'place': '/'}), {'peer': B, 'game': 'steam:620', 'title': 'Portal2'})
        self.assertEqual(r('sync.now', {}), {})
        self.assertEqual(r('pair.open', {'internet': 'yes'}), {'internet': False}, 'only a real true')
        self.assertEqual(r('pair.join', {'id': B, 'code': '123456'}), {'id': B, 'code': '123456'})
        self.assertEqual(r('pair.join', {'code': 'AB12-CD34'}), {'code': 'AB12-CD34'})
        with self.assertRaises(ValueError):
            r('pair.join', {'id': B, 'code': 'AB12-CD34'})
        with self.assertRaises(ValueError):
            r('pair.join', {'code': '12"; rm'})
        with self.assertRaises(ValueError):
            r('storage.wake', {})

    def test_status_is_short_and_has_no_secrets(self):
        link = FakeLink(self.sock, {'status': {'ok': True, 'data': FULL}})
        try:
            s = self.m.status()
        finally:
            link.close()
        dump = json.dumps(s)
        for secret in ('SECRET-TICKET', 'relay.example', '192.168.1.9', '/srv', '/home/you', 'signedIn', 'meta', '4242'):
            self.assertNotIn(secret, dump, secret)
        self.assertTrue(s['running'])
        self.assertEqual(s['me']['name'], 'Ally')
        self.assertEqual(s['peers'][0], {'id': B, 'name': 'Desk', 'online': True, 'lan': True, 'lastSeen': 5})
        self.assertEqual(s['pairing']['code'], '123456')
        self.assertEqual(s['offers'][0]['title'], 'Celeste')
        self.assertEqual(s['transfers'][0]['done'], 40)
        self.assertTrue(s['storage']['servers'][0]['canWake'])
        self.assertTrue(s['storage']['savesFollow'])
        self.assertEqual(s['lastChange'], 7)
        # Kept for the read-only view.
        last = json.loads(pathlib.Path(self.tmp, 'settings', 'last.json').read_text())
        self.assertEqual(last['status']['peers'][0]['name'], 'Desk')
        self.assertNotIn('SECRET', json.dumps(last))

    def test_not_running_gives_the_last_status_read_only(self):
        s = self.m.status()
        self.assertFalse(s['running'])
        self.assertIn("isn't running", s['error'])
        self.assertIsNone(s['last'])
        self.m.save_last(self.m.short(FULL))
        s = self.m.status()
        self.assertFalse(s['running'])
        self.assertEqual(s['last']['status']['peers'][0]['name'], 'Desk')
        r = self.m.call('send', {'peer': B, 'game': 'steam:620'})
        self.assertEqual((r['ok'], r['running']), (False, False))

    def test_a_socket_that_isnt_this_users_own_is_never_used(self):
        os.makedirs(os.path.dirname(self.sock), mode=0o755)
        os.chmod(os.path.dirname(self.sock), 0o755)   # a folder others can get into: not Hearth Link's
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        srv.bind(self.sock)
        srv.listen(1)
        try:
            self.assertFalse(self.m.link_safe(self.sock))
            self.assertIsNone(self.m.link_call('status', {}))
        finally:
            srv.close()

    def test_calls_go_through_and_errors_come_back(self):
        link = FakeLink(self.sock, {'transfer.pause': {'ok': True, 'data': None},
                                    'send': {'ok': False, 'error': 'Not a paired device'},
                                    'local.games': {'ok': True, 'data': {'games': [{'id': 'steam:620', 'title': 'Portal 2', 'store': 'steam', 'size': 9, 'hasPrefix': False},
                                                                                   {'id': 'bad', 'title': 'x'}]}}})
        try:
            self.assertEqual(self.m.call('transfer.pause', {'id': '0123456789abcdef'}), {'ok': True, 'data': None})
            self.assertEqual(self.m.call('send', {'peer': B, 'game': 'steam:620'}), {'ok': False, 'error': 'Not a paired device'})
            g = self.m.call('local.games', {})
            self.assertEqual(g['data']['games'], [{'id': 'steam:620', 'title': 'Portal 2', 'store': 'steam', 'size': 9}])
            self.assertEqual(self.m.call('unpair', {'id': B})['error'], self.m.NOT_ALLOWED)
        finally:
            link.close()
        self.assertEqual([x['cmd'] for x in link.got], ['transfer.pause', 'send', 'local.games'], 'unpair never reached Hearth Link')
        self.assertEqual(link.got[0]['args'], {'id': '0123456789abcdef'})

    def test_storage_sections(self):
        link = FakeLink(self.sock, {
            'storage.cache.status': {'ok': True, 'data': {'enabled': True, 'using': {'id': B, 'name': 'Desk'}, 'jobs': [],
                                                          'available': [{'game': 'steam:1', 'title': 'Hades', 'size': 5}], 'updates': []}},
            'storage.saves.status': {'ok': True, 'data': {'follow': True, 'server': B, 'games': [{'id': 'steam:1', 'title': 'Hades', 'at': 4}, {'id': 'gog:2', 'title': 'Celeste', 'at': 9, 'conflict': {'x': 1}}]}},
            'storage.updates.status': {'ok': False, 'error': 'nope'},
            'storage.devices': {'ok': True, 'data': {'devices': [{'id': A, 'name': 'Ally', 'me': True, 'online': True, 'battery': {'percent': 12, 'charging': False}, 'free': 1, 'total': 100, 'boot': {'x': 1}}], 'serverName': 'Desk'}},
        })
        try:
            s = self.m.storage()
        finally:
            link.close()
        self.assertTrue(s['running'])
        self.assertEqual(s['cache']['using'], 'Desk')
        self.assertEqual(s['cache']['available'][0]['title'], 'Hades')
        self.assertEqual([g['title'] for g in s['saves']['games']], ['Celeste', 'Hades'], 'newest first')
        self.assertTrue(s['saves']['games'][0]['conflict'])
        self.assertEqual(s['updates']['error'], 'nope', 'one section failing leaves the others')
        self.assertEqual(s['health']['devices'][0]['battery'], 12)
        self.assertNotIn('boot', json.dumps(s['health']))
        self.assertFalse(self.m.storage()['running'])

    def test_start_link_only_starts_the_users_own_unit(self):
        seen = []
        self.m.PROGRAM = os.path.join(self.tmp, 'hearth-link')
        self.assertFalse(self.m.start_link(run=lambda *a, **k: seen.append(a))['ok'], 'not installed: nothing run')
        self.assertEqual(seen, [])
        open(self.m.PROGRAM, 'w').close()
        r = self.m.start_link(run=lambda a, **k: seen.append(a) or types.SimpleNamespace(returncode=0, stderr=''))
        self.assertTrue(r['ok'])
        self.assertEqual(seen[0][1:], ['--user', 'start', 'hearth-link.service'])

    def test_home_server_tile_asks_only_its_three_commands_with_a_timeout(self):
        prog = os.path.join(self.tmp, 'hearth-home-server')
        self.m.HOME_SERVER = prog
        self.assertEqual(self.m.home_server(['quick']), {'available': False}, 'no helper: no tile, nothing run')
        open(prog, 'w').close()
        seen = []
        quick = {'available': True, 'host': 'nvidia-pc', 'lightMode': True, 'playing': True, 'resumed': False, 'paused': ['backups'],
                 'shares': 3, 'people': 1, 'usage': {'used': 1e12, 'total': 4e12}, 'backups': {'count': 1, 'lastOk': 5, 'waiting': ['Games'],
                 'failed': [], 'running': False, 'secret': 'x'}, 'alerts': ['A drive is failing'], 'setupDone': True, 'settings': {'x': 1},
                 'ai': {'on': True, 'state': 'unloaded', 'models': ['secret']}}

        def run(a, **k):
            seen.append((a, k))
            out = json.dumps(quick) if a[-1] == 'quick' else '{"changed": true}'
            return types.SimpleNamespace(returncode=0, stdout='log: x\n' + out + '\n', stderr='')
        r = self.m.home_server(['quick'], run=run)
        self.assertTrue(r['available'])
        self.assertEqual(r['backups']['waiting'], ['Games'])
        self.assertNotIn('settings', r, 'only the tile\'s fields')
        self.assertNotIn('secret', json.dumps(r))
        self.assertEqual(r['ai'], {'on': True, 'state': 'unloaded'}, 'the AI service in a word, nothing else')
        self.assertEqual(self.m.ai_short({'on': True, 'state': 'weird'}), {'on': False, 'state': 'off'})
        self.assertEqual(self.m.ai_short(None), {'on': False, 'state': 'off'})
        self.assertEqual(r['remote'], {'on': False, 'streaming': False, 'who': '', 'since': 0}, 'no remote desktop from an older helper')
        self.assertEqual(self.m.remote_short({'on': True, 'streaming': True, 'who': 'Work\x07 laptop', 'since': 7, 'ip': '100.1.2.3'}),
                         {'on': True, 'streaming': True, 'who': 'Work laptop', 'since': 7})
        self.assertFalse(self.m.remote_short({'on': False, 'streaming': True})['streaming'])
        self.assertEqual(seen[0][0][1:], [prog, 'quick'])
        self.assertEqual(seen[0][1]['timeout'], self.m.HOME_SERVER_TIMEOUT)
        self.assertTrue(self.m.home_server(['quick-set', 'lightMode', 'off'], run=run)['ok'])
        self.assertTrue(self.m.home_server(['resume', 'on'], run=run)['ok'])
        for bad in (['set', 'tsOnly', 'on'], ['quick-set', 'tsOnly', 'on'], ['resume', 'yes'], ['person-add', 'x'], ['apply'], 'quick', []):
            n = len(seen)
            self.assertEqual(self.m.home_server(bad, run=run)['error'], self.m.NOT_ALLOWED, bad)
            self.assertEqual(len(seen), n, 'nothing run')
        boom = lambda a, **k: (_ for _ in ()).throw(subprocess.TimeoutExpired(a, 10))
        self.assertEqual(self.m.home_server(['quick'], run=boom)['available'], False, 'a helper that hangs only hides the tile')
        err = lambda a, **k: types.SimpleNamespace(returncode=1, stdout='error: Home server comes with the next update\n', stderr='')
        self.assertFalse(self.m.home_server(['quick'], run=err)['available'])

    def test_plugin_methods(self):
        p = self.m.Plugin()
        loop = asyncio.new_event_loop()
        try:
            self.assertFalse(loop.run_until_complete(p.status())['running'])
            self.assertEqual(loop.run_until_complete(p.call('ticket', {}))['error'], self.m.NOT_ALLOWED)
        finally:
            loop.close()


if __name__ == '__main__':
    unittest.main()
