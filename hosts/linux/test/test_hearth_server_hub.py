"""Tests for the Home server helper's Game servers, Game cache, Remote play, Media and PCs pages (docs/SERVER-HUB.md):
hearth-server-hub builds each page from what its sources answer (the board's night, from CI's fixture), keeps your
choices only through hearth_settings_io and only the ones the page offers, reads players over Steam's query and
Minecraft's list ping, schedules and Wake-on-LAN; the Home server helper's game firewall rule and system parts; and
hearth-server-dns' Game cache numbers. Nothing here touches the system: commands are replaced by recorders."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import socket
import struct
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIBEXEC = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
FIXTURE = ROOT / '.github' / 'scripts' / 'hearth-apps-rust-shots.d' / 'server-hub-fixture.py'
os.environ['TZ'] = 'America/Chicago'
import time  # noqa: E402
time.tzset()


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hub = load(SHELL / 'hearth-server-hub', 'hearth_server_hub_t')
hh = load(LIBEXEC / 'hearth-home-server-helper', 'hearth_home_server_helper_hub')
dns = load(LIBEXEC / 'hearth-server-dns', 'hearth_server_dns_hub')


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        d = self.tmp.name
        self.patch(hub, 'SETTINGS', os.path.join(d, 'cfg', 'nimbus', 'server-hub.json'))
        self.patch(hub, 'STATE', os.path.join(d, 'state', 'server-hub.json'))
        self.patch(hub, 'UNIT_DIR', os.path.join(d, 'units'))
        self.patch(hub, '_FX', None)

    def patch(self, mod, name, value):
        old = getattr(mod, name)
        setattr(mod, name, value)
        self.addCleanup(setattr, mod, name, old)

    def env(self, key, value):
        old = os.environ.get(key)
        os.environ[key] = value
        self.addCleanup(lambda: os.environ.__setitem__(key, old) if old is not None else os.environ.pop(key, None))

    def board(self):
        fx = os.path.join(self.tmp.name, 'fx.json')
        cfg = os.path.join(self.tmp.name, 'cfg')
        subprocess.run([sys.executable, str(FIXTURE), fx, cfg], check=True, env=dict(os.environ, TZ='America/Chicago'))
        self.env('HEARTH_SERVER_HUB_FIXTURE', fx)
        hub._FX = None


class Words(Base):
    def test_schedule_words_as_the_board(self):
        self.assertEqual(hub.span_words('06:00', '22:00'), '6 AM–10')
        self.assertEqual(hub.span_words('06:00', '13:00'), '6 AM–1')
        self.assertEqual(hub.span_words('16:00', '21:00'), '4–9 PM')
        self.assertEqual(hub.span_words('19:00', '23:00'), '7–11 PM')
        self.assertEqual(hub.span_words('12:00', '21:00'), 'noon–9')
        self.assertEqual(hub.span_words('00:00', '24:00'), 'all day')

    def test_sizes(self):
        self.assertEqual(hub.gb(1.9e12), '1.9 TB')
        self.assertEqual(hub.gb(1.15e12), '1.15 TB')
        self.assertEqual(hub.gb(612e9, 0), '612 GB')
        self.assertEqual(hub.gb(1e12, 0), '1 TB')
        self.assertEqual(hub.gb(212e6), '212 MB')


class Pages(Base):
    """Every page from the board's sources: the words the boards show come out of hearth-server-hub's own code."""

    def test_game_servers(self):
        self.board()
        v = hub.game_view()
        self.assertEqual(v['tag'], '3 running · 7 players')
        self.assertEqual([c['name'] for c in v['cards']], ['Valheim', 'Minecraft', 'Palworld', 'Gravesleep', 'Noclip Protocol'])
        self.assertEqual(v['cards'][0]['line'], ['3 of 10 online', 'world "Ashfjord"'])
        self.assertEqual(v['cards'][0]['sub'], 'since 11:40 PM · CPU 6% · RAM 3.1 GB')
        self.assertEqual(v['cards'][3]['sub'], 'build from your repo · starts Sat 7 PM')
        self.assertEqual(v['cards'][4]['buttons'], [['Make the server build', 'make']])
        d = v['detail']
        self.assertEqual(d['title'], 'Valheim · "Ashfjord"')
        self.assertEqual(d['sub'], 'day 214 · seed f8Kq2Lm · password on · crossplay off')
        self.assertEqual([(p['name'], p['where'], p['time']) for p in d['players']],
                         [('Dragongamer9', 'the ROG Ally', '2 h 14 min'), ('Sam', 'over Tailscale (shared)', '1 h 40 min'), ('Jordan', 'over Tailscale (shared)', '22 min')])
        self.assertEqual(d['mods']['caption'], 'Mods · 4 on · from Thunderstore, checked')
        self.assertEqual(v['invite']['shared'], 'Sam · Jordan · Dad (Minecraft only)')
        self.assertEqual(v['backups']['when'], 'every night at 4:00 AM and when it stops · kept 30')
        self.assertEqual([r['label'] for r in v['backups']['rows']], ['Ashfjord · tonight 4:00 AM', 'Ashfjord · Nov 25, stopped', 'Ashfjord · Nov 25 4:00 AM'])
        self.assertEqual(v['schedule']['rows'][0]['days'], ['6 AM–10', '6 AM–10', '', '6 AM–10', '6 AM–1', 'all day', 'all day'])
        self.assertEqual(v['week']['most'], 'most: Sat 9 PM, 9 players')
        self.assertEqual(len(v['week']['bars']), 28)
        self.assertEqual(v['use'], {'cpu': '14%', 'mem': '6.2 of 12 GB', 'worlds': '3.4 GB · in Backups'})
        self.assertEqual(v['own'][2], {'label': 'Noclip Protocol · O3DE server build', 'right': 'Set it up', 'link': True, 'id': 'noclip'})

    def test_game_cache(self):
        self.board()
        v = hub.cache_view()
        self.assertEqual(v['tag'], 'on · saved 1.9 TB of downloads')
        self.assertEqual(v['kpi'][0][1:], ['1.9 TB', 'since Sep 2 · about 41 hours of waiting'])
        self.assertEqual(v['kpi'][2][1], '84%')
        self.assertEqual(v['kpi'][3][1:], ['940 Mb/s', "to the ROG Ally · Baldur's Gate 3 update"])
        self.assertEqual(v['games'][0], {'game': "Baldur's Gate 3", 'store': 'Steam', 'cached': '148 GB', 'by': 'Nvidia PC, Ally, laptop', 'saved': '296 GB', 'used': 'now'})
        self.assertEqual(v['games'][4]['used'], 'Nov 26')
        self.assertEqual(v['games'][6]['used'], 'today')
        self.assertEqual(v['games'][7]['game'], '+ 34 more games')
        self.assertEqual(v['games'][7]['saved'], '1.15 TB')
        self.assertEqual(v['month']['hit'], '642 GB')

    def test_remote_play(self):
        self.board()
        v = hub.play_view()
        self.assertEqual(v['tag'], '1 playing · the TV')
        self.assertEqual(v['now']['title'], 'Gravesleep · on the TCL 55" TV')
        self.assertEqual(v['now']['line'], '4K · 60 fps · HDR · 38 Mb/s · delay 9 ms · Xbox controller')
        self.assertEqual(v['who'], {'on': True, 'title': 'You · on the TV', 'time': '1 h 12 min', 'line': 'Gravesleep · started 12:12 AM'})
        self.assertEqual([d['state'] for d in v['devices']], ['playing', 'ready', 'ready', 'away · 5G'])
        self.assertEqual([w['words'] for w in v['week']], ['9 h', '4 h', '2 h 10', '35 min'])

    def test_media(self):
        self.board()
        v = hub.media_view()
        self.assertEqual(v['count'], '7,412')
        self.assertEqual(v['tag'], 'Jellyfin · 1 watching')
        self.assertEqual((v['wanted']['n'], v['wanted']['more'], v['getting']['n'], v['ready']['n']), (6, 3, 3, 3))
        self.assertEqual(v['library']['rows'][1], ['TV shows', '64 · 2,190 episodes'])
        self.assertEqual(v['coming'][1], {'when': 'Sat 1:00 PM', 'what': 'Rail Journeys', 'line': 'for Dad · records from 11.2'})
        self.assertEqual(v['watching'][0], {'title': 'Ironwood (2021)', 'who': 'Dad, on his PC', 'at': '48 min in'})

    def test_pcs(self):
        self.board()
        v = hub.pcs_view()
        self.assertEqual(v['tag'], '3 PCs · one server')
        self.assertEqual([c['health'] for c in v['cards']], ['Healthy · 62° · up 9 days', "Healthy · 48° · Dad's using it", 'Healthy · 41° · lid shut'])
        self.assertEqual([c['right'] for c in v['cards']], ['186 W', 'gives 40%', '14 W'])
        share = dict((r[0], [c[1] for c in r[1]]) for r in v['share'])
        self.assertEqual(share['Storage · 25 TB'], ['22 TB', '2 TB', '1'])
        self.assertEqual(share['Game servers · 4'], ['Valheim, Palworld', 'Minecraft', 'Gravesleep'])
        self.assertEqual(share['AI models'], ['Colibri, Imagine, Kokoro', 'Ollama, Listen'])
        self.assertEqual([m[1] for m in v['moves']], ['Ollama and Listen move to the Nvidia PC', 'Minecraft moves to the Nvidia PC',
                                                      'The second backup copy waits for it', "Nothing is lost · moves back when it's on again"])
        self.assertEqual(v['week'][1]['note'], 'off twice (Nov 21 storm, Nov 23)')


class Choices(Base):
    def test_only_offered_choices_and_merged(self):
        os.makedirs(os.path.dirname(hub.SETTINGS))
        with open(hub.SETTINGS, 'w') as f:
            json.dump({'other': 1, 'games': {'schedule': {'valheim': {'Mon': ['06:00', '22:00']}}}}, f)
        self.patch(hub, 'timer_sync', lambda: None)
        with self.assertRaises(hub.Fail):
            hub.set_choice('games', 'maxMemGB', 13)
        with self.assertRaises(hub.Fail):
            hub.set_choice('games', 'rootShell', True)
        hub.set_choice('games', 'maxMemGB', 16)
        with open(hub.SETTINGS) as f:
            d = json.load(f)
        self.assertEqual(d['other'], 1)
        self.assertEqual(d['games']['maxMemGB'], 16)
        self.assertEqual(d['games']['schedule'], {'valheim': {'Mon': ['06:00', '22:00']}})
        self.assertNotIn('play', d)          # defaults are never written

    def test_broken_settings_never_overwritten_with_defaults(self):
        os.makedirs(os.path.dirname(hub.SETTINGS))
        with open(hub.SETTINGS, 'w') as f:
            f.write('{not json')
        s = hub.settings()
        self.assertEqual(s['games']['idleStop'], 30)
        if os.path.exists(hub.SETTINGS):
            with open(hub.SETTINGS) as f:
                self.assertNotIn('idleStop', f.read())

    def test_schedule_checked(self):
        self.patch(hub, 'timer_sync', lambda: None)
        with self.assertRaises(hub.Fail):
            hub.schedule('valheim', 'Mon', '22:00', '06:00')
        with self.assertRaises(hub.Fail):
            hub.schedule('valheim', 'Someday', '06:00', '08:00')
        hub.schedule('valheim', 'Sat', '00:00', '24:00')
        self.assertEqual(hub.settings()['games']['schedule']['valheim']['Sat'], ['00:00', '24:00'])
        hub.schedule('valheim', 'Sat', 'off')
        self.assertNotIn('Sat', hub.settings()['games']['schedule']['valheim'])

    def test_the_timer_is_off_until_a_rule_is_on(self):
        s = hub.sio.with_defaults({'games': {'idleStop': 0, 'pauseWhilePlaying': False}, 'play': {'freeGpu': False, 'keepGames': True, 'sleepAfter': False}}, hub.DEFAULTS)
        self.assertFalse(hub.timer_wanted(s))
        s['games']['schedule'] = {'valheim': {'Mon': ['06:00', '10:00']}}
        self.assertTrue(hub.timer_wanted(s))
        t = hub.timer_texts()['hearth-server-hub.timer']
        self.assertNotIn('OnBootSec', t)
        self.assertIn('WantedBy=timers.target', t)
        self.assertNotIn('[Install]', hub.timer_texts()['hearth-server-hub.service'])

    def test_media_only_from_what_you_own_or_free(self):
        self.patch(hub, 'timer_sync', lambda: None)
        with self.assertRaises(hub.Fail):
            hub.media_want(json.dumps({'title': 'Some film', 'kind': 'movie', 'from': 'torrent'}))
        hub.media_want(json.dumps({'title': 'Winter Line (1948)', 'kind': 'movie', 'from': 'free'}))
        self.assertEqual(hub.settings()['media']['wanted'][0]['line'], 'public domain · Internet Archive')
        with open(SHELL / 'hearth-server-hub') as f:
            src = f.read().lower()
        for bad in ('magnet:', 'torrent', 'jackett', 'prowlarr', 'sonarr', 'radarr', 'usenet'):
            self.assertNotIn(bad, src.replace('no torrents', ''))


class Network(Base):
    def test_a2s_players(self):
        srv = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        srv.bind(('127.0.0.1', 0))
        port = srv.getsockname()[1]

        def serve():
            for _ in range(3):
                data, addr = srv.recvfrom(1400)
                if data[4:5] == b'T':
                    srv.sendto(b'\xff\xff\xff\xffI\x11Hearth\x00map\x00valheim\x00Valheim\x00\x00\x00\x03\x0a', addr)
                elif data[4:9] == b'U\xff\xff\xff\xff':
                    srv.sendto(b'\xff\xff\xff\xffA\x01\x02\x03\x04', addr)
                else:
                    body = b'\x02' + b'\x00Sam\x00' + struct.pack('<lf', 0, 6000.0) + b'\x01Jordan\x00' + struct.pack('<lf', 0, 1320.0)
                    srv.sendto(b'\xff\xff\xff\xffD' + body, addr)
        th = threading.Thread(target=serve, daemon=True)
        th.start()
        n, mx, names = hub.a2s(port)
        srv.close()
        self.assertEqual((n, mx), (3, 10))
        self.assertEqual(names, [('Sam', 6000), ('Jordan', 1320)])

    def test_magic_packet(self):
        p = hub.magic_packet('aa:bb:cc:dd:ee:ff')
        self.assertEqual(len(p), 102)
        self.assertEqual(p[:6], b'\xff' * 6)
        self.assertEqual(p[6:12], bytes.fromhex('aabbccddeeff'))
        with self.assertRaises(hub.Fail):
            hub.magic_packet('')


class RootParts(unittest.TestCase):
    def test_games_rule_only_tailscale_and_home(self):
        t = hh.games_nft_text()
        self.assertIn('table inet hearth_games', t)
        self.assertIn('iifname { "lo", "tailscale0" } accept', t)
        self.assertIn('192.168.0.0/16', t)
        self.assertIn('udp dport { 2456, 2457', t)
        self.assertIn('counter drop', t)
        u = hh.games_unit_text()
        self.assertIn('ExecStart=-/usr/sbin/nft', u)        # can never fail the boot
        self.assertNotIn('Before=', u)

    def test_games_setup_and_off(self):
        calls = []
        d = tempfile.mkdtemp()
        old = (hh.run, hh.which, hh.GAMES_NFT_FILE, hh.GAMES_UNIT_FILE, hh.UNLOCK_TOOL)
        os.environ['PKEXEC_UID'] = str(os.getuid())
        try:
            hh.run = lambda c, stdin=None, timeout=60: (calls.append(c) or (0, 'install ok installed' if c[0] == 'dpkg-query' else ''))
            hh.which = lambda n: '/usr/sbin/' + n
            hh.GAMES_NFT_FILE = os.path.join(d, 'games.nft')
            hh.GAMES_UNIT_FILE = os.path.join(d, 'hearth-games-firewall.service')
            hh.UNLOCK_TOOL = os.path.join(d, 'no-unlock')
            r = hh.games_setup()
            self.assertTrue(r['ok'])
            self.assertTrue(os.path.exists(hh.GAMES_NFT_FILE))
            self.assertIn(['systemctl', 'enable', hh.GAMES_UNIT], calls)
            hh.games_off()
            self.assertFalse(os.path.exists(hh.GAMES_NFT_FILE))
            self.assertIn(['nft', 'delete', 'table', 'inet', 'hearth_games'], calls)
        finally:
            hh.run, hh.which, hh.GAMES_NFT_FILE, hh.GAMES_UNIT_FILE, hh.UNLOCK_TOOL = old
            os.environ.pop('PKEXEC_UID', None)

    def test_media_setup_never_unlocks_discs(self):
        with open(LIBEXEC / 'hearth-home-server-helper') as f:
            src = f.read()
        start = src.index('def media_setup')
        body = src[start:src.index('\n\n\n', start)]
        for bad in ('libdvdcss', 'makemkv', 'libaacs', 'keydb'):
            self.assertNotIn(bad, body.lower())


class CacheNumbers(unittest.TestCase):
    def test_each_game_and_device_counted(self):
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, 'logs'))
        line = '[steam] 192.168.1.23 / - - - [26/Nov/2026:01:20:00 -0600] "GET /depot/1086941/chunk/ab HTTP/1.1" 200 %d "-" "Valve/Steam HTTP Client 1.0" "%s" "lancache.steamcontent.com" "-"\n'
        with open(os.path.join(d, 'logs', 'access.log'), 'w') as f:
            f.write(line % (1000, 'MISS'))
            f.write(line % (3000, 'HIT'))
        conf = {'cache': {'dir': d, 'sizeGB': 1000}}
        old = dns.steam_game
        dns.steam_game = lambda path, cache: "Baldur's Gate 3"
        try:
            st = dns.cache_tick({}, conf, {'192.168.1.23': 'ROG Ally'}, now=1795000000)
        finally:
            dns.steam_game = old
        g = st['perGame']["Baldur's Gate 3"]
        self.assertEqual((g['hit'], g['miss']), (3000, 1000))
        self.assertEqual(st['total']['hit'], 3000)
        month = list(st['devMonth'])[0]
        self.assertEqual(sum(st['devMonth'][month].values()), 4000)


if __name__ == '__main__':
    unittest.main()
