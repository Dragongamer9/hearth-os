"""Tests for "Share this device's AI" (docs/AI-SHARE.md): settings off by default and written per key (never defaults
over a damaged file), the share code kept only as a hash, the AI services found by asking them, what a device reports
(playing, on battery, plugged in, a service on this device only), every device's list on the Home server (Tailscale,
devices seen before, Hearth Link, the order), the server over a real socket (only GET, only this device and Tailscale,
the share code), where it listens (Tailscale only with the firewall rule), the root helper's nftables rule, the units
and the image's checks, and the Settings page's wiring. Nothing here patches a global without putting it back."""
import sys
sys.dont_write_bytecode = True
import http.client
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import stat
import tempfile
import threading
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIBEXEC = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
UNITS = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ais = load(SHELL / 'hearth-ai-share', 'hearth_ai_share_t')
helper = load(LIBEXEC / 'hearth-ai-share-helper', 'hearth_ai_share_helper_t')

TS = {'running': True, 'dns': 'nvidia-pc.tail4f2a.ts.net', 'name': 'Nvidia-PC', 'ips': ['100.84.12.7', 'fd7a:115c:a1e0::7'], 'peers': []}


def fake_get(answers):
    return lambda port, path, timeout=1.5: answers.get((port, path))


OLLAMA = {(11434, '/api/tags'): {'models': [{'name': 'qwen3:30b-a3b'}, {'name': 'qwen3:14b'}]}, (11434, '/api/ps'): {'models': [{'name': 'qwen3:30b-a3b'}]}}
KOKORO = {(8880, '/v1/audio/voices'): {'voices': ['af_bella', 'am_adam']}, (8880, '/v1/models'): {'data': [{'id': 'kokoro'}]}}
COMFY = {(8188, '/system_stats'): {'system': {'os': 'posix'}}, (8188, '/models/checkpoints'): ['sdxl.safetensors'], (8188, '/queue'): {'queue_running': [], 'queue_pending': []}}
WHISPER = {(8000, '/v1/models'): {'data': [{'id': 'Systran/faster-whisper-small'}]}}
LISTEN_ALL = [('0A', '0.0.0.0', p, '0.0.0.0') for p in (11434, 8880, 8188, 8000)]


class Base(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.d, True)
        env = mock.patch.dict(os.environ, {'XDG_STATE_HOME': os.path.join(self.d, 'state'), 'XDG_CONFIG_HOME': os.path.join(self.d, 'config')})
        env.start()
        self.addCleanup(env.stop)
        os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)
        for k, v in (('SETTINGS', 'config/nimbus/ai-share.json'), ('STATE', 'state/nimbus/ai-share.json'), ('SEEN', 'cache/ai-share-devices.json'),
                     ('HOME_SERVER', 'config/nimbus/home-server.json'), ('FW_STATE', 'run/ai-share-firewall.json'), ('FW_FILE', 'etc/ai-share.nft'),
                     ('APPS_DIR', 'apps'), ('SYS_ROOT', 'sys')):
            self.patch(ais, k, os.path.join(self.d, v))
        self.patch(ais, 'game_running', lambda: False)
        self.patch(ais, 'version', lambda: '1.4')
        ais._TS.update(at=0, v=None)
        self.addCleanup(ais._TS.update, at=0, v=None)

    def patch(self, obj, name, value):
        p = mock.patch.object(obj, name, value)
        p.start()
        self.addCleanup(p.stop)

    def write(self, rel, text):
        p = os.path.join(self.d, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def settings(self):
        with open(ais.SETTINGS) as f:
            return json.load(f)

    def found(self, answers=None, table=None):
        a = {}
        for x in (OLLAMA, KOKORO, COMFY, WHISPER):
            a.update(x)
        return ais.detect(TS['ips'], get=fake_get(answers if answers is not None else a), table=LISTEN_ALL if table is None else table)


class Settings(Base):
    def test_off_by_default_and_nothing_written(self):
        cfg = ais.load()
        self.assertFalse(cfg['on'] or cfg['hub'] or cfg['lan'] or cfg['codeOn'])
        self.assertTrue(cfg['pluggedOnly'] and cfg['notWhilePlaying'])
        doc = ais.document(cfg, TS, self.found())
        self.assertEqual((doc['reason'], doc['services'], doc['available']), ('off', [], False))
        self.assertFalse(os.path.exists(ais.SETTINGS), 'reading never writes')

    def test_only_the_changed_key_is_written(self):
        self.assertFalse(ais.set_key('notWhilePlaying', 'on'), 'a default you already have is not written')
        self.assertFalse(os.path.exists(ais.SETTINGS))
        self.assertTrue(ais.set_key('on', 'on'))
        self.assertEqual(self.settings(), {'on': True})
        self.assertTrue(ais.set_key('voice', 'off'))
        self.assertEqual(self.settings(), {'on': True, 'share': {'voice': False}})
        self.assertFalse(ais.set_key('voice', 'off'))
        for k, v in (('nope', 'on'), ('on', 'yes'), ('on', ''), ('priority', 'on')):
            with self.assertRaises(ais.Fail):
                ais.set_key(k, v)
        self.assertTrue(os.path.exists(ais.SETTINGS + '.bak'))

    def test_merge_keeps_what_others_wrote(self):
        self.write('config/nimbus/ai-share.json', json.dumps({'lan': True, 'future': 7}))
        ais.set_key('on', 'on')
        self.assertEqual(self.settings(), {'lan': True, 'future': 7, 'on': True})

    def test_never_defaults_over_a_damaged_file(self):
        self.write('config/nimbus/ai-share.json', '{"on": tr')
        self.assertFalse(ais.peek()['on'])
        with open(ais.SETTINGS) as f:
            self.assertEqual(f.read(), '{"on": tr', 'the server only reads: a damaged file stays as it is')
        self.write('config/nimbus/ai-share.json.bak', json.dumps({'on': True, 'hub': True}))
        self.assertTrue(ais.load()['hub'], 'put back from .bak')

    def test_share_code_only_as_a_hash(self):
        with self.assertRaises(ais.Fail):
            ais.set_key('codeOn', 'on')
        for bad in ('short', 'has space inside', 'x' * 65, 'tab\there1'):
            with self.assertRaises(ais.Fail):
                ais.code_set(bad)
        with mock.patch.object(ais, 'make_hash', lambda c, rounds=200000: _hash(c)):
            ais.code_set('Dragon-Fire-2026')
        self.assertTrue(ais.load()['codeOn'])
        for p in (ais.STATE, ais.SETTINGS):
            with open(p) as f:
                self.assertNotIn('Dragon-Fire-2026', f.read())
        self.assertEqual(stat.S_IMODE(os.stat(ais.STATE).st_mode), 0o600)
        h = ais.code_hash()
        self.assertTrue(ais.check_hash('Dragon-Fire-2026', h))
        self.assertFalse(ais.check_hash('dragon-fire-2026', h))
        self.assertFalse(ais.check_hash('x', 'garbage'))
        ais.code_clear()
        self.assertEqual(ais.code_hash(), '')
        self.assertFalse(ais.load()['codeOn'])


def _hash(code):
    import hashlib
    salt = b'0123456789abcdef'
    return 'pbkdf2-sha256$1000$%s$%s' % (salt.hex(), hashlib.pbkdf2_hmac('sha256', code.encode(), salt, 1000).hex())


class Detect(Base):
    def test_finds_each_service_by_asking_it(self):
        f = {s['engine']: s for s in self.found()}
        self.assertEqual(set(f), {'ollama', 'kokoro-fastapi', 'comfyui', 'whisper'})
        self.assertEqual(f['ollama']['models'], ['qwen3:30b-a3b', 'qwen3:14b'])
        self.assertEqual(f['ollama']['loaded'], ['qwen3:30b-a3b'])
        self.assertEqual(f['kokoro-fastapi']['voices'], ['af_bella', 'am_adam'])
        self.assertEqual(f['comfyui']['models'], ['sdxl.safetensors'])
        self.assertEqual(f['whisper']['models'], ['Systran/faster-whisper-small'])
        self.assertTrue(all(s['healthy'] and s['reachable'] and not s['busy'] for s in f.values()))

    def test_something_else_on_the_port_isnt_taken_for_it(self):
        other = {(11434, '/api/tags'): {'hello': 1}, (8000, '/v1/models'): {'data': [{'id': 'gpt-4o'}]}, (8880, '/v1/audio/voices'): None}
        self.assertEqual(self.found(other), [])
        self.assertEqual(self.found(table=[]), [], 'nothing listening: nothing asked, nothing found')

    def test_installed_hearth_app_not_answering_is_listed_unhealthy(self):
        os.makedirs(os.path.join(ais.APPS_DIR, 'ollama'))
        f = self.found({}, table=[])
        self.assertEqual([(s['engine'], s['healthy']) for s in f], [('ollama', False)])

    def test_on_this_device_only_and_busy(self):
        table = [('0A', '127.0.0.1', 11434, '0.0.0.0'), ('0A', '100.84.12.7', 8880, '0.0.0.0'), ('01', '100.84.12.7', 8880, '100.70.1.2'),
                 ('01', '127.0.0.1', 11434, '127.0.0.1')]
        f = {s['engine']: s for s in self.found(table=table)}
        self.assertFalse(f['ollama']['reachable'])
        self.assertFalse(f['ollama']['busy'], 'a connection from this device itself is not another device using it')
        self.assertTrue(f['kokoro-fastapi']['reachable'] and f['kokoro-fastapi']['busy'])
        busy = dict(COMFY)
        busy[(8188, '/queue')] = {'queue_running': [[1]], 'queue_pending': []}
        self.assertTrue(self.found(busy)[0]['busy'])

    def test_proc_net_tcp_lines(self):
        self.write('sys/proc/net/tcp', '  sl  local_address rem_address   st\n'
                   '   0: 00000000:2CAA 00000000:0000 0A 0 0\n   1: 0100007F:22B0 00000000:0000 0A 0 0\n'
                   '   2: 070C5464:22B0 0201460A:C350 01 0 0\n')
        self.write('sys/proc/net/tcp6', '  sl  local_address rem_address   st\n'
                   '   0: 00000000000000000000000000000000:1FFC 00000000000000000000000000000000:0000 0A 0 0\n')
        t = ais.tcp_table()
        self.assertIn(('0A', '0.0.0.0', 11434, '0.0.0.0'), t)
        self.assertIn(('0A', '127.0.0.1', 8880, '0.0.0.0'), t)
        self.assertIn(('01', '100.84.12.7', 8880, '10.70.1.2'), t)
        self.assertIn(('0A', '::', 8188, '::'), t)

    def test_local_get_over_a_real_socket_only_json(self):
        from http.server import BaseHTTPRequestHandler, HTTPServer

        class H(BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def do_GET(self):
                b = b'{"models": []}' if self.path == '/api/tags' else b'not json'
                self.send_response(200)
                self.send_header('Content-Length', str(len(b)))
                self.end_headers()
                self.wfile.write(b)
        s = HTTPServer(('127.0.0.1', 0), H)
        threading.Thread(target=s.serve_forever, daemon=True).start()
        self.addCleanup(s.server_close)
        self.addCleanup(s.shutdown)
        port = s.server_address[1]
        self.assertEqual(ais.local_get(port, '/api/tags'), {'models': []})
        self.assertIsNone(ais.local_get(port, '/other'))
        self.assertIsNone(ais.local_get(1, '/api/tags'))


class Report(Base):
    def setUp(self):
        super().setUp()
        self.cfg = ais.load()
        self.cfg['on'] = True

    def battery(self, cap, status, online):
        self.write('sys/sys/class/power_supply/BAT0/type', 'Battery')
        self.write('sys/sys/class/power_supply/BAT0/capacity', str(cap))
        self.write('sys/sys/class/power_supply/BAT0/status', status)
        self.write('sys/sys/class/power_supply/ACAD/type', 'Mains')
        self.write('sys/sys/class/power_supply/ACAD/online', '1' if online else '0')
        self.write('sys/sys/class/power_supply/ctrl/type', 'Battery')
        self.write('sys/sys/class/power_supply/ctrl/scope', 'Device')
        self.write('sys/sys/class/power_supply/ctrl/capacity', '5')

    def test_the_protocol_document(self):
        self.write('sys/sys/class/drm/card1/device/vendor', '0x10de')
        doc = ais.document(self.cfg, TS, self.found())
        for k in ('protocol', 'device', 'os', 'version', 'gaming', 'busy', 'services', 'available', 'reason', 'pluggedIn', 'updated'):
            self.assertIn(k, doc)
        self.assertEqual((doc['protocol'], doc['os'], doc['device'], doc['version']), (1, 'hearth', 'nvidia-pc', '1.4'))
        self.assertTrue(doc['available'])
        self.assertNotIn('battery', doc, 'a desktop PC has no battery field')
        self.assertEqual(doc['gpu'], {'vendor': 'nvidia'})
        chat = next(s for s in doc['services'] if s['kind'] == 'chat')
        self.assertEqual(chat['url'], 'http://nvidia-pc.tail4f2a.ts.net:11434')
        self.assertEqual(chat['openai'], 'http://nvidia-pc.tail4f2a.ts.net:11434/v1')
        img = next(s for s in doc['services'] if s['kind'] == 'image')
        self.assertEqual((img['engine'], img['url']), ('comfyui', 'http://nvidia-pc.tail4f2a.ts.net:8188'))
        self.assertNotIn('openai', img)
        self.assertEqual(sorted(s['kind'] for s in doc['services']), ['chat', 'image', 'stt', 'voice'])
        text = json.dumps(doc)
        for secret in ('pbkdf2', 'codeHash', 'password', 'token'):
            self.assertNotIn(secret, text)

    def test_per_service_switches(self):
        self.cfg['share'] = dict(self.cfg['share'], image=False, stt=False)
        doc = ais.document(self.cfg, TS, self.found())
        self.assertEqual(sorted(s['kind'] for s in doc['services']), ['chat', 'voice'])

    def test_playing_says_so_and_pauses_when_chosen(self):
        self.patch(ais, 'game_running', lambda: True)
        doc = ais.document(self.cfg, TS, self.found())
        self.assertTrue(doc['gaming'])
        self.assertEqual((doc['available'], doc['reason']), (False, 'playing'))
        self.cfg['notWhilePlaying'] = False
        doc = ais.document(self.cfg, TS, self.found())
        self.assertTrue(doc['gaming'] and doc['available'], 'still says it is playing so clients can route elsewhere')

    def test_handheld_on_battery(self):
        self.battery(64, 'Discharging', False)
        doc = ais.document(self.cfg, TS, self.found())
        self.assertEqual(doc['battery'], {'percent': 64, 'charging': False})
        self.assertEqual((doc['pluggedIn'], doc['available'], doc['reason']), (False, False, 'battery'))
        self.cfg['pluggedOnly'] = False
        self.assertTrue(ais.document(self.cfg, TS, self.found())['available'])

    def test_handheld_plugged_in(self):
        self.battery(90, 'Charging', True)
        doc = ais.document(self.cfg, TS, self.found())
        self.assertEqual((doc['battery'], doc['pluggedIn'], doc['available']), ({'percent': 90, 'charging': True}, True, True))

    def test_only_on_this_device_is_not_healthy_for_others(self):
        doc = ais.document(self.cfg, TS, self.found(OLLAMA, table=[('0A', '127.0.0.1', 11434, '0.0.0.0')]))
        self.assertEqual((doc['services'][0]['healthy'], doc['services'][0]['reachable']), (False, False))
        self.assertFalse(doc['available'])

    def test_game_running_reads_gamescope_or_the_playing_file(self):
        self.patch(ais, 'game_running', _real_game_running)
        self.write('sys/proc/123/comm', 'bash')
        os.makedirs(os.path.join(self.d, 'sys/run/user/1000'))
        self.assertFalse(ais.game_running())
        self.write('sys/run/user/1000/hearth-playing', '')
        self.assertTrue(ais.game_running())
        os.remove(os.path.join(self.d, 'sys/run/user/1000/hearth-playing'))
        self.write('sys/proc/456/comm', 'gamescope')
        self.assertTrue(ais.game_running())


_real_game_running = ais.game_running


def peer_doc(device, vendor=None, gaming=False, services=None):
    d = {'protocol': 1, 'device': device, 'name': device.upper(), 'os': 'hearth', 'version': '1.4', 'gaming': gaming, 'busy': False,
         'available': not gaming, 'reason': 'playing' if gaming else '', 'pluggedIn': True,
         'services': services if services is not None else [{'kind': 'voice', 'engine': 'kokoro-fastapi', 'url': 'http://%s.tail4f2a.ts.net:8880' % device,
                                                              'healthy': True, 'voices': ['af_bella']}]}
    if vendor:
        d['gpu'] = {'vendor': vendor}
    return d


class Devices(Base):
    def ts(self):
        return dict(TS, peers=[{'name': 'ROG-Ally', 'dns': 'rog-ally.tail4f2a.ts.net', 'os': 'linux', 'online': True, 'ips': ['100.70.1.2']},
                               {'name': 'Phone', 'dns': 'phone.tail4f2a.ts.net', 'os': 'android', 'online': True, 'ips': ['100.70.1.3']},
                               {'name': 'Laptop', 'dns': 'laptop.tail4f2a.ts.net', 'os': 'linux', 'online': True, 'ips': ['100.70.1.5']}])

    def test_every_device_in_order(self):
        asked = []
        docs = {'100.70.1.2': peer_doc('rog-ally', 'amd', gaming=True), '100.70.1.5': peer_doc('laptop', 'intel')}
        me = ais.document(dict(ais.load(), on=True), TS, self.found())
        me['gpu'] = {'vendor': 'nvidia'}
        out = ais.devices(ais.load(), fetch=lambda ip, code: asked.append(ip) or docs.get(ip), links=lambda: [{'name': 'Living-Room', 'online': False}],
                          ts=self.ts(), me=me)
        self.assertEqual(sorted(asked), ['100.70.1.2', '100.70.1.5'], 'only Linux devices are asked, not the phone')
        self.assertEqual(out['priority'], ['nvidia-pc', 'rog-ally', 'laptop', 'living-room'])
        self.assertEqual(out['protocol'], 1)
        by = {d['device']: d for d in out['devices']}
        self.assertTrue(by['rog-ally']['gaming'] and by['rog-ally']['online'])
        self.assertEqual((by['living-room']['via'], by['living-room']['online'], by['living-room']['reason']), ('hearth link', False, 'offline'))
        self.assertEqual([d['rank'] for d in out['devices']], [1, 2, 3, 4])
        # The Ally goes offline: it's still listed, from what it said before.
        out = ais.devices(ais.load(), fetch=lambda ip, code: None, links=lambda: [], ts=self.ts(), me=me)
        by = {d['device']: d for d in out['devices']}
        self.assertEqual((by['rog-ally']['online'], by['rog-ally']['reason'], by['rog-ally']['via']), (False, 'offline', 'seen before'))
        self.assertEqual(by['rog-ally']['services'][0]['voices'], ['af_bella'])

    def test_your_order_wins_and_moves(self):
        docs = {'100.70.1.2': peer_doc('rog-ally', 'amd'), '100.70.1.5': peer_doc('laptop', 'intel')}
        me = dict(ais.document(dict(ais.load(), on=True), TS, self.found()), gpu={'vendor': 'nvidia'})
        self.patch(ais, 'ts_status', lambda fresh=False: self.ts())
        self.patch(ais, 'gpu', lambda: {'vendor': 'nvidia'})
        ais.devices(ais.load(), fetch=lambda ip, code: docs.get(ip), links=lambda: [], ts=self.ts(), me=me)
        r = ais.move('laptop', 'up')
        self.assertEqual(r['priority'], ['nvidia-pc', 'laptop', 'rog-ally'])
        self.assertEqual(self.settings(), {'priority': ['nvidia-pc', 'laptop', 'rog-ally']})
        out = ais.devices(ais.load(), fetch=lambda ip, code: docs.get(ip), links=lambda: [], ts=self.ts(), me=me)
        self.assertEqual(out['priority'], ['nvidia-pc', 'laptop', 'rog-ally'])
        self.assertFalse(ais.move('nvidia-pc', 'up')['changed'], 'the first stays first')
        for bad in (('../x', 'up'), ('laptop', 'left'), ('', 'up'), ('Laptop PC', 'down')):
            with self.assertRaises(ais.Fail):
                ais.move(*bad)
        ais.order_reset()
        self.assertEqual(self.settings(), {})

    def test_another_devices_answer_is_checked(self):
        self.assertIsNone(ais.clean_doc({'protocol': 1, 'device': 'x', 'os': 'windows'}))
        self.assertIsNone(ais.clean_doc({'device': 'x', 'os': 'hearth'}), 'no protocol')
        self.assertIsNone(ais.clean_doc('nope'))
        bad = peer_doc('ally', services=[{'kind': 'chat', 'url': 'javascript:alert(1)'}, {'kind': 'shell', 'url': 'http://a:1'},
                                         {'kind': 'chat', 'engine': 'ollama', 'url': 'http://ally.ts.net:11434', 'openai': 'http://evil/v1',
                                          'models': ['ok', 7, 'x' * 500], 'healthy': True}])
        bad['extra'] = 'dropped'
        c = ais.clean_doc(bad)
        self.assertEqual(len(c['services']), 1)
        self.assertNotIn('openai', c['services'][0])
        self.assertEqual(c['services'][0]['models'], ['ok'])
        self.assertNotIn('extra', c)

    def test_only_tailscale_addresses_are_asked(self):
        with mock.patch.object(ais.http.client, 'HTTPConnection', side_effect=AssertionError('no connection')):
            self.assertIsNone(ais.peer_get('192.168.1.20'))
            self.assertIsNone(ais.peer_get('8.8.8.8'))


class ServerTest(Base):
    def start(self, cfg):
        self.write('config/nimbus/ai-share.json', json.dumps(cfg))
        self.patch(ais, 'ts_status', lambda fresh=False: TS)
        self.patch(ais, 'detect', lambda ts_ips=(), get=None, table=None: [])
        self.patch(ais.Handler, 'cache', {'devices': (0, None, '')})
        self.patch(ais.Handler, 'fails', {})
        s = ais.Server4(('127.0.0.1', 0), ais.Handler)
        threading.Thread(target=s.serve_forever, daemon=True).start()
        self.addCleanup(s.server_close)
        self.addCleanup(s.shutdown)
        return s.server_address[1]

    def get(self, port, path, method='GET', headers=None):
        c = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
        c.request(method, path, headers=headers or {})
        r = c.getresponse()
        body = r.read()
        c.close()
        return r.status, json.loads(body) if body else None

    def test_answers_read_only(self):
        port = self.start({'on': True})
        st, doc = self.get(port, '/v1/hearth/ai')
        self.assertEqual((st, doc['os'], doc['protocol'], doc['device']), (200, 'hearth', 1, 'nvidia-pc'))
        self.assertEqual(self.get(port, '/v1/hearth/ai/devices')[0], 404, 'only when "find my devices here" is on')
        self.assertEqual(self.get(port, '/etc/passwd')[0], 404)
        self.assertEqual(self.get(port, '/v1/hearth/ai/../../x')[0], 404)
        for m in ('POST', 'PUT', 'DELETE'):
            self.assertEqual(self.get(port, '/v1/hearth/ai', m)[0], 405)

    def test_devices_on_the_home_server(self):
        self.patch(ais, 'devices', lambda cfg, code='': {'protocol': 1, 'devices': [], 'code': code})
        port = self.start({'on': False, 'hub': True})
        st, out = self.get(port, '/v1/hearth/ai/devices')
        self.assertEqual((st, out['protocol']), (200, 1))
        st, doc = self.get(port, '/v1/hearth/ai')
        self.assertEqual((st, doc['reason'], doc['services']), (200, 'off', []))

    def test_share_code(self):
        self.write('state/nimbus/ai-share.json', json.dumps({'codeHash': _hash('Dragon-Fire-2026')}))
        self.patch(ais.time, 'sleep', lambda s: None)
        port = self.start({'on': True, 'codeOn': True})
        self.assertEqual(self.get(port, '/v1/hearth/ai')[0], 401)
        self.assertEqual(self.get(port, '/v1/hearth/ai', headers={'X-Hearth-Share-Code': 'wrong-code-123'})[0], 401)
        self.assertEqual(self.get(port, '/v1/hearth/ai', headers={'X-Hearth-Share-Code': 'Dragon-Fire-2026'})[0], 200)
        self.assertEqual(self.get(port, '/v1/hearth/ai', headers={'Authorization': 'Bearer Dragon-Fire-2026'})[0], 200)

    def test_who_may_ask(self):
        for ip in ('127.0.0.1', '::1', '100.70.1.2', 'fd7a:115c:a1e0::5', '::ffff:100.70.1.2'):
            self.assertTrue(ais.allowed(ip, False), ip)
        for ip in ('192.168.1.20', '10.0.0.4', '8.8.8.8', 'fe80::1', 'garbage'):
            self.assertFalse(ais.allowed(ip, False), ip)
        self.assertTrue(ais.allowed('192.168.1.20', True), 'only when you chose your home network')

    def test_where_it_listens(self):
        cfg = ais.load()
        self.assertEqual(ais.want_addresses(cfg, TS, False), ['127.0.0.1'], 'no firewall rule: this device only')
        self.assertEqual(ais.want_addresses(cfg, TS, True), ['127.0.0.1', '100.84.12.7', 'fd7a:115c:a1e0::7'])
        self.assertEqual(ais.want_addresses(dict(cfg, lan=True), TS, False), ['0.0.0.0'])
        self.assertEqual(ais.want_addresses(cfg, dict(TS, ips=['192.168.1.5']), True), ['127.0.0.1'])

    def test_serve_stops_when_sharing_is_off(self):
        self.assertEqual(ais.serve(port=0, once=True), {})

    def test_firewall_state_is_read_only(self):
        self.assertFalse(ais.firewall_ok())
        self.write('run/ai-share-firewall.json', json.dumps({'ok': True}))
        self.assertTrue(ais.firewall_ok())


class Apply(Base):
    def setUp(self):
        super().setUp()
        self.calls = []
        self.patch(ais, 'root', lambda a: self.calls.append(['root'] + a) or {'ok': True})
        self.patch(ais, 'systemctl_user', lambda *a: self.calls.append(['user'] + list(a)) or 0)

    def test_on_loads_the_rule_then_starts(self):
        ais.apply(dict(ais.load(), on=True))
        self.assertEqual(self.calls, [['root', 'firewall', 'on'], ['user', 'enable', '--now', 'hearth-ai-share.service']])

    def test_off_stops_and_never_asks_root_for_nothing(self):
        ais.apply(ais.load())
        self.assertEqual(self.calls, [['user', 'disable', '--now', 'hearth-ai-share.service']])
        self.write('etc/ai-share.nft', 'x')
        self.calls.clear()
        ais.apply(ais.load())
        self.assertEqual(self.calls[0], ['root', 'firewall', 'off'])

    def test_a_failed_rule_never_changes_a_setting(self):
        self.patch(ais, 'root', lambda a: {'ok': False, 'error': 'no nft'})
        ais.set_key('on', 'on')
        before = self.settings()
        r = ais.apply()
        self.assertEqual(r['firewall'], 'failed')
        self.assertIn('this device only', r['note'])
        self.assertEqual(self.settings(), before)


class RootHelper(Base):
    def setUp(self):
        super().setUp()
        for k, v in (('NFT_FILE', 'etc/hearth/ai-share.nft'), ('STATE', 'run/hearth/ai-share-firewall.json')):
            self.patch(helper, k, os.path.join(self.d, v))
        self.ran = []
        self.code = 0
        self.patch(helper, 'run', lambda a: self.ran.append(a) or (self.code, ''))
        self.patch(helper, 'which', lambda n: '/usr/sbin/' + n)
        self.patch(helper.os, 'geteuid', lambda: 0)

    def test_the_rule(self):
        t = helper.nft_text()
        self.assertIn('table inet hearth_ai_share {', t)
        self.assertIn('iifname { "lo", "tailscale0" } accept', t)
        self.assertIn('tcp dport 8189 counter drop', t)
        self.assertIn('delete table inet hearth_ai_share', t, 'loading twice replaces it')
        self.assertNotIn('11434', t, 'only its own port; the services keep their own rules')

    def test_on_and_off(self):
        self.assertTrue(helper.main(['firewall', 'on'])['ok'])
        self.assertTrue(os.path.exists(helper.NFT_FILE))
        self.assertIn(['/usr/sbin/nft', '-f', helper.NFT_FILE], self.ran)
        self.assertIn(['systemctl', 'enable', '--now', 'hearth-ai-share-firewall.timer'], self.ran)
        with open(helper.STATE) as f:
            self.assertTrue(json.load(f)['ok'])
        self.ran.clear()
        self.assertTrue(helper.main(['firewall', 'off'])['ok'])
        self.assertFalse(os.path.exists(helper.NFT_FILE) or os.path.exists(helper.STATE))
        self.assertIn(['/usr/sbin/nft', 'delete', 'table', 'inet', 'hearth_ai_share'], self.ran)
        self.assertIn(['systemctl', 'disable', '--now', 'hearth-ai-share-firewall.timer'], self.ran)

    def test_a_rule_that_fails_is_said(self):
        self.code = 1
        r = helper.main(['firewall', 'on'])
        self.assertFalse(r['ok'])
        with open(helper.STATE) as f:
            self.assertFalse(json.load(f)['ok'])

    def test_load_without_a_file_does_nothing(self):
        self.assertEqual(helper.main(['firewall-load']), {'ok': True, 'loaded': False})
        self.assertEqual(self.ran, [])
        for bad in (['firewall'], ['firewall', 'maybe'], ['rm', '-rf'], []):
            self.assertFalse(helper.main(bad)['ok'])


class Wiring(unittest.TestCase):
    def read(self, p):
        return (ROOT / p).read_text()

    def test_units_off_and_never_in_the_way(self):
        user = self.read('os/system_files/usr/lib/systemd/user/hearth-ai-share.service')
        self.assertIn('ExecStart=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-ai-share serve', user)
        self.assertNotRegex(user, r'(?m)^(Requires|Before|RequiredBy)=')
        fw = self.read('os/system_files/usr/lib/systemd/system/hearth-ai-share-firewall.service')
        self.assertNotIn('[Install]', fw, 'only its timer starts it')
        self.assertIn('ConditionPathExists=/etc/hearth/ai-share.nft', fw)
        self.assertNotRegex(fw, r'(?m)^(Requires|Before|WantedBy|RequiredBy)=')
        timer = self.read('os/system_files/usr/lib/systemd/system/hearth-ai-share-firewall.timer')
        self.assertIn('OnBootSec=15s', timer)

    def test_the_image_checks_them(self):
        b = self.read('os-bases/peppermint/build.sh')
        self.assertIn('systemctl --global disable hearth-ai-share.service', b)
        self.assertIn('systemctl disable hearth-ai-share-firewall.timer', b)
        self.assertIn('hearth-ai-share-firewall.timer is on in the image', b)
        y = self.read('.github/workflows/base-peppermint.yml')
        self.assertIn('AI sharing is installed and off', y)
        self.assertIn('hearth-ai-share-helper', self.read('os/system_files/usr/share/polkit-1/rules.d/49-hearth-storage.rules'))

    def test_settings_page_wiring(self):
        app = self.read('hosts/linux/desktop/shell/SettingsApp.qml')
        self.assertIn('{ id: "hs-aidevices"', app)
        self.assertIn('"hs-aidevices": "SetHomeAIDevices.qml"', app)
        page = self.read('hosts/linux/desktop/shell/SetHomeAIDevices.qml')
        self.assertIn('helper: "hearth-ai-share"', page)
        self.assertNotIn('setText(', page)
        for cmd in ('"set", k', '"devices"', '"status"', '"move"', '"code-set"', '"order-reset"'):
            self.assertIn(cmd, page)
        for p in ('SetHomeAI.qml', 'SetLink.qml'):
            self.assertIn('SettingsNav.openPage("hs-aidevices")', self.read('hosts/linux/desktop/shell/' + p))
        self.assertIn('ai-share.json', self.read('hosts/linux/test/test_settings_persistence.py'))

    def test_no_shell_anywhere(self):
        for p in ('hosts/linux/desktop/shell/hearth-ai-share', 'os/system_files/usr/libexec/nimbus/hearth-ai-share-helper'):
            t = self.read(p)
            self.assertNotIn('shell=True', t)
            self.assertNotIn('os.system(', t)
            self.assertNotRegex(t, r"\['(sh|bash)', '-c'")

    def test_docs(self):
        d = self.read('docs/AI-SHARE.md')
        for k in ('/v1/hearth/ai', '/v1/hearth/ai/devices', '"protocol": 1', '8189', 'X-Hearth-Share-Code'):
            self.assertIn(k, d)


if __name__ == '__main__':
    unittest.main()
