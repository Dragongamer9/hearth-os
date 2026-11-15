"""1.13 Server mode › Game download cache and Ad blocker (hearth-server-dns) and each device's side (hearth-dns-client):
AdGuard's config follows your switches (stores, pauses, a device, everything else on your Wi-Fi, allowed sites; the
cache's names keep working while blocking is off), LANCache's log becomes "saved this week" and "right now", settings
are saved safely, and a device stops using the server after two missed answers and comes back when it answers."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import socket
import struct
import tempfile
import threading
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
LIB = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(LIB / name))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


dns = load('hearth-server-dns')
client = load('hearth-dns-client')
ORIG_ASK, ORIG_RUN = client.ask, client.run


class Base(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp(prefix='hearth-dns-')
        dns.CONF = os.path.join(self.t, 'etc', 'server-dns.json')
        dns.STATE = os.path.join(self.t, 'state')
        dns.RUN = os.path.join(self.t, 'run')
        dns.SYSTEMCTL = 'true'
        dns.TAILSCALE = 'false'
        os.environ['HEARTH_DNS_TEST'] = '1'

    def tearDown(self):
        shutil.rmtree(self.t, ignore_errors=True)


class Config(Base):
    def conf(self, **kw):
        c = dns.merged({}, dns.DEFAULTS)
        for k, v in kw.items():
            c[k] = dns.merged(v, c[k])
        return c

    def test_off_by_default(self):
        c, ok = dns.read_conf()
        self.assertTrue(ok)
        self.assertFalse(c['cache']['on'])
        self.assertFalse(c['ads']['on'])

    def test_cache_names_point_here_only_for_stores_that_are_on(self):
        y = dns.adguard_yaml(self.conf(cache={'on': True}), '192.168.1.10', '100.64.0.5')
        self.assertIn('"lancache.steamcontent.com"', y)
        self.assertIn('"download.epicgames.com"', y)
        self.assertNotIn('xboxlive', y)                              # Xbox is off by default (as the mockup)
        self.assertIn('answer: "192.168.1.10"', y)
        self.assertIn('"hearth-dns.hearth"\n      answer: "100.64.0.5"', y)
        self.assertIn('filters:\n  - enabled: false', y)              # the ad blocker is off: no blocking
        self.assertIn('protection_enabled: true', y)                  # ... but the names still work
        self.assertIn('    - 192.168.1.10\n    - 100.64.0.5', y)
        y = dns.adguard_yaml(self.conf(), '192.168.1.10', '')
        self.assertNotIn('steamcontent', y)

    def test_blocking_pauses_and_devices(self):
        now = time.time()
        c = self.conf(ads={'on': True, 'allow': ['shop.example.com'],
                           'devices': {'ally': {'on': True, 'pausedUntil': now + 600}, 'lan': {'on': False}, 'tv': {'on': True}}})
        y = dns.adguard_yaml(c, '192.168.1.10', '100.64.0.5', {'100.64.0.9': 'ally', '100.64.0.7': 'tv'}, now)
        self.assertIn('filters:\n  - enabled: true', y)
        self.assertIn('"@@||shop.example.com^"', y)
        self.assertIn("\"@@||*^$client='100.64.0.9'\"", y)           # the Ally, paused for a while
        self.assertNotIn('100.64.0.7', y)                            # the TV keeps blocking
        self.assertIn('$client=10.0.0.0/8|172.16.0.0/12|192.168.0.0/16', y)   # everything else on your Wi-Fi: off
        c['ads']['pausedUntil'] = now + 60
        self.assertIn('filters:\n  - enabled: false', dns.adguard_yaml(c, '192.168.1.10', '', {}, now))
        self.assertIn('filters:\n  - enabled: true', dns.adguard_yaml(c, '192.168.1.10', '', {}, now + 120))   # it ends by itself

    def test_settings_saved_safely(self):
        dns.set_conf(lambda c: dns.section(c, 'cache').update(on=True))
        dns.set_conf(lambda c: dns.section(dns.section(c, 'cache'), 'stores').update(xbox=True))
        saved = json.load(open(dns.CONF))
        self.assertEqual(saved, {'cache': {'on': True, 'stores': {'xbox': True}}})     # only what changed, no defaults
        self.assertTrue(os.path.exists(dns.CONF + '.bak'))
        with open(dns.CONF, 'w') as f:
            f.write('{nope')
        c, ok = dns.read_conf()
        self.assertFalse(ok)
        self.assertFalse(c['cache']['on'])
        self.assertEqual(open(dns.CONF).read(), '{nope')            # reading never writes over it
        dns.set_conf(lambda c: dns.section(c, 'ads').update(on=True))
        self.assertEqual(len([n for n in os.listdir(os.path.dirname(dns.CONF)) if '.unreadable-' in n]), 1)

    def test_commands_check_their_input(self):
        for args in (['store', 'origin', 'on'], ['cache-place', '/etc', '100'], ['cache-place', '/mnt/games', '5'],
                     ['allow', 'not a site'], ['device', '../x', 'on'], ['cache-empty', 'yes'], ['ads', 'maybe']):
            self.assertNotEqual(dns.main(['x'] + args), 0, args)


class CacheNumbers(Base):
    def test_log_becomes_saved_and_right_now(self):
        conf = dns.merged({'cache': {'on': True, 'dir': os.path.join(self.t, 'cache')}}, dns.DEFAULTS)
        logs = os.path.join(self.t, 'cache', 'logs')
        os.makedirs(logs)
        line = '[%s] %s / - - - [15/Nov/2026:02:00:00 +0000] "GET %s HTTP/1.1" 200 %d "-" "Valve/Steam HTTP Client 1.0" "%s" "lancache.steamcontent.com" "-"\n'
        with open(os.path.join(logs, 'access.log'), 'w') as f:
            f.write(line % ('steam', '100.64.0.9', '/depot/1/chunk/abc', 4 * 2 ** 30, 'HIT'))
            f.write(line % ('epicgames', '192.168.1.30', '/Builds/x', 2 ** 30, 'MISS'))
            f.write('garbage line\n')
        dns.steam_game = lambda path, cache: 'Baldur\'s Gate 3'
        now = time.time()
        stats = dns.cache_tick({}, conf, {}, now)
        v = dns.cache_view(stats, conf, {'100.64.0.9': 'ally'}, now)
        self.assertEqual(v['savedWeek'], 4 * 2 ** 30)
        self.assertEqual(v['internetWeek'], 2 ** 30)
        self.assertEqual(v['devicesWeek'], 5 * 2 ** 30)
        self.assertEqual(len(v['bars']), 7)
        self.assertEqual(v['bars'][-1]['saved'], 4 * 2 ** 30)
        rows = {r['device']: r for r in v['now']}
        self.assertEqual(rows['ally']['what'], 'Baldur\'s Gate 3')
        self.assertEqual(rows['ally']['from'], 'from the cache')
        self.assertEqual(rows['192.168.1.30']['what'], 'Epic Games download')
        self.assertEqual(rows['192.168.1.30']['from'], 'from the internet once')
        stats = dns.cache_tick(stats, conf, {}, now + 1)                   # nothing new: counted once
        self.assertEqual(dns.cache_view(stats, conf, {}, now + 1)['savedWeek'], 4 * 2 ** 30)
        stats = dns.cache_tick(stats, conf, {}, now + 60)                  # quiet for a while: nobody "right now"
        self.assertEqual(dns.cache_view(stats, conf, {}, now + 60)['now'], [])


class FakeDNS(threading.Thread):
    """Answers hearth-dns.hearth while .up is set."""
    def __init__(self):
        super().__init__(daemon=True)
        self.s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.s.bind(('127.0.0.1', 0))
        self.port = self.s.getsockname()[1]
        self.up = True
        self.s.settimeout(0.2)
        self.stop = False

    def run(self):
        while not self.stop:
            try:
                q, addr = self.s.recvfrom(512)
            except OSError:
                continue
            if self.up:
                self.s.sendto(q[:2] + struct.pack('>HHHHH', 0x8180, 1, 1, 0, 0) + q[12:]
                              + b'\xc0\x0c' + struct.pack('>HHIH', 1, 1, 10, 4) + bytes([127, 0, 0, 1]), addr)


class Client(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp(prefix='hearth-dnsc-')
        self.calls = []
        client.RUN = os.path.join(self.t, 'run.json')
        client.CONF = os.path.join(self.t, 'conf.json')
        self.srv = FakeDNS()
        self.srv.start()
        port = self.srv.port
        client.ask = lambda server, name=client.MARKER, timeout=0.3: ORIG_ASK(server, name, timeout, port)
        client.candidates = lambda: ['127.0.0.1']
        client.card = lambda: 'eth0'
        client.run = lambda cmd, timeout=10: (self.calls.append(cmd[1:3] + cmd[3:4]) or (0, ''))

    def tearDown(self):
        self.srv.stop = True
        shutil.rmtree(self.t, ignore_errors=True)

    def test_uses_it_while_it_answers_and_lets_go_after_two_misses(self):
        s = client.step({}, 1000)                  # finds it
        self.assertEqual(s['server'], '127.0.0.1')
        s = client.step(s, 1005)                   # uses it
        self.assertEqual(s['applied'], 'eth0')
        self.assertIn(['device', 'modify', 'eth0'], self.calls)
        self.srv.up = False
        s = client.step(s, 1010)
        self.assertEqual(s['applied'], 'eth0')     # one miss: still
        s = client.step(s, 1015)
        self.assertEqual(s['applied'], '')         # two: back to normal lookups
        self.assertIn(['device', 'reapply', 'eth0'], self.calls)
        self.srv.up = True
        s = client.step(s, 1100)
        s = client.step(s, 1105)
        self.assertEqual(s['applied'], 'eth0')     # and back again when it answers

    def test_switch_off_means_never(self):
        with open(client.CONF, 'w') as f:
            json.dump({'on': False}, f)
        s = client.step({}, 1000)
        s = client.step(s, 1005)
        self.assertEqual(s.get('applied', ''), '')
        self.assertNotIn(['device', 'modify', 'eth0'], self.calls)

    def test_nothing_answers_nothing_changes(self):
        self.srv.up = False
        s = client.step({}, 1000)
        s = client.step(s, 1005)
        self.assertEqual(s.get('server'), '')
        self.assertEqual(self.calls, [])


if __name__ == '__main__':
    unittest.main()
