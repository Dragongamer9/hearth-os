"""Tests for Settings › Remote play › Use this PC from anywhere (docs/REMOTE-DESKTOP.md): Sunshine's ports only through
Tailscale (the root helper's own nftables table, no rule no start), Hearth's low-latency Sunshine values merged in and
put back on Off only where they weren't changed since, the Desktop app, the switch saved only after turning on worked
and never by itself, pairing with Moonlight's PIN (old and new Sunshine), who is streaming and direct or relay, staying
awake, Sunshine's prep command that never fails, and everything off in the image. The system's commands are replaced
by recorders; every patched global and environment variable is put back."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIBEXEC = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
SYSTEMD = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hh = load(LIBEXEC / 'hearth-home-server-helper', 'hearth_home_server_helper_rd')
rd = load(SHELL / 'hearth-remote-desktop', 'hearth_remote_desktop_t')


class Patch(unittest.TestCase):
    def patch(self, mod, name, value):
        old = getattr(mod, name)
        setattr(mod, name, value)
        self.addCleanup(setattr, mod, name, old)

    def env(self, key, value):
        old = os.environ.get(key)
        os.environ[key] = value
        self.addCleanup(lambda: os.environ.__setitem__(key, old) if old is not None else os.environ.pop(key, None))


# ---------------------------------------------------------------------------------------------------- root: firewall

class Firewall(Patch):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.calls = []
        self.nft_ok = True
        self.enabled = set()

        def run(cmd, stdin=None, timeout=60):
            self.calls.append(list(cmd))
            if cmd[:2] == ['nft', '-f']:
                return (0, '') if self.nft_ok else (1, 'Error')
            if cmd[:2] == ['systemctl', 'is-enabled']:
                return (0, 'enabled') if cmd[2] in self.enabled else (1, 'disabled')
            if cmd[:2] == ['systemctl', 'enable']:
                self.enabled.add(cmd[-1])
            if cmd[:2] == ['systemctl', 'disable']:
                self.enabled.discard(cmd[-1])
            return 0, ''
        self.patch(hh, 'run', run)
        self.patch(hh, 'which', lambda n: '/usr/sbin/' + n)
        self.patch(hh, 'RD_NFT_FILE', os.path.join(self.d, 'etc/remote-desktop.nft'))
        self.patch(hh, 'RD_MARK', os.path.join(self.d, 'run/remote-desktop-fw'))
        self.env('PKEXEC_UID', str(os.getuid()))

    def test_sunshine_s_ports_match_its_docs(self):
        self.assertEqual(hh.sunshine_ports(), ([47984, 47989, 47990, 48010], [47998, 47999, 48000, 48002, 48010]))
        self.assertEqual(hh.sunshine_ports(50000)[0], [49995, 50000, 50001, 50021], 'a changed base port moves them all')

    def test_the_rule_lets_in_only_this_pc_and_tailscale(self):
        t = hh.rd_nft_text()
        self.assertIn('table inet hearth_remote_desktop {', t)
        self.assertIn('iifname { "lo", "tailscale0" } accept', t)
        self.assertIn('tcp dport { 47984, 47989, 47990, 48010 } counter drop', t)
        self.assertIn('udp dport { 47998, 47999, 48000, 48002, 48010 } counter drop', t)
        self.assertNotIn('hearth_home_server', t, 'its own table: the home server\'s rules are never touched')
        self.assertLess(t.index('accept'), t.index('drop'))

    def test_tailscale_only_loads_marks_and_enables_the_boot_unit(self):
        r = hh.remote_fw('tailscale', '47989')
        self.assertEqual(r['firewall'], 'tailscale-only')
        self.assertTrue(os.path.exists(hh.RD_NFT_FILE))
        self.assertEqual(open(hh.RD_MARK).read().strip(), 'tailscale-only')
        self.assertIn(hh.RD_UNIT, self.enabled)
        self.assertFalse([c for c in self.calls if c[:3] == ['systemctl', 'enable', '--now']], 'enabled for the boot only')
        r = hh.remote_fw('off')
        self.assertEqual(r['firewall'], 'open')
        self.assertFalse(os.path.exists(hh.RD_NFT_FILE) or os.path.exists(hh.RD_MARK))
        self.assertNotIn(hh.RD_UNIT, self.enabled)
        self.assertIn(['nft', 'delete', 'table', 'inet', 'hearth_remote_desktop'], self.calls)

    def test_a_rule_that_doesn_t_load_turns_nothing_on(self):
        self.nft_ok = False
        with self.assertRaises(hh.Fail):
            hh.remote_fw('tailscale', None)
        self.assertFalse(os.path.exists(hh.RD_NFT_FILE) or os.path.exists(hh.RD_MARK))
        self.assertNotIn(hh.RD_UNIT, self.enabled)

    def test_no_nft_no_start_and_bad_ports_refused(self):
        self.patch(hh, 'which', lambda n: None)
        with self.assertRaises(hh.Fail):
            hh.remote_fw('tailscale', None)
        for bad in ('80', '99999', '4798x', ''):
            with self.assertRaises(hh.Fail):
                hh.rd_base(bad)

    def test_only_through_pkexec_and_load_is_for_the_boot_unit(self):
        os.environ.pop('PKEXEC_UID')
        with self.assertRaises(hh.Fail):
            hh.remote_fw('tailscale', None)
        self.assertEqual(hh.remote_fw('load'), {'ok': True, 'firewall': 'open'}, 'nothing saved: nothing loaded')
        self.env('PKEXEC_UID', str(os.getuid()))
        self.assertEqual(hh.main(['x', 'remote-fw', 'load']), 1, 'not through pkexec')

    def test_the_boot_unit_never_holds_up_the_boot(self):
        t = (SYSTEMD / 'system' / 'hearth-remote-desktop-firewall.service').read_text()
        self.assertIn('ExecStart=-/usr/bin/python3 /usr/libexec/nimbus/hearth-home-server-helper remote-fw load', t)
        self.assertIn('ConditionPathExists=/etc/hearth/remote-desktop.nft', t)
        self.assertNotIn('Before=', t)
        self.assertIn('TimeoutStartSec=30', t)
        self.assertFalse(list((ROOT / 'os' / 'system_files' / 'etc').glob('systemd/system/*.wants/hearth-remote-desktop*')),
                         'not enabled in the image')
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('systemctl disable hearth-remote-desktop-firewall.service', b)
        self.assertIn('hearth-remote-desktop-firewall.service is on in the image', b)


# ---------------------------------------------------------------------------------------------------- user helper

class Helper(Patch):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        j = lambda *p: os.path.join(self.d, *p)
        for k, v in (('SETTINGS', j('c/nimbus/remote-desktop.json')), ('STATE', j('s/nimbus/remote-desktop.json')),
                     ('CREDS', j('data/nimbus/remote-play.json')), ('SUN_DIR', j('c/sunshine')),
                     ('SUN_CONF', j('c/sunshine/sunshine.conf')), ('SUN_APPS', j('c/sunshine/apps.json')),
                     ('USER_UNITS', j('c/systemd/user')), ('RUNTIME', j('run')), ('FW_MARK', j('fw-mark'))):
            self.patch(rd, k, v)
        self.env('XDG_STATE_HOME', j('s'))
        self.calls = []
        self.active = set()
        self.enabled = set()
        self.roots = []
        self.root_fail = None

        def run(cmd, timeout=15, env=None):
            self.calls.append(list(cmd))
            if cmd[:3] == ['systemctl', '--user', 'show']:
                return (0, 'app-dev.lizardbyte.app.Sunshine.service\nloaded\n') if cmd[-1].startswith('app-dev') else (0, 'not-found\n')
            if cmd[:3] == ['systemctl', '--user', 'is-active']:
                return (0, '') if cmd[-1] in self.active else (3, '')
            if cmd[:3] == ['systemctl', '--user', 'is-enabled']:
                return (0, '') if cmd[-1] in self.enabled else (1, '')
            if cmd[:3] in (['systemctl', '--user', 'start'], ['systemctl', '--user', 'restart']):
                self.active.add(cmd[-1])
            if cmd[:3] == ['systemctl', '--user', 'stop']:
                self.active.discard(cmd[-1])
            return 0, ''

        def root(args):
            self.roots.append(list(args))
            if self.root_fail:
                raise rd.Fail(self.root_fail)
            if args[:2] == ['remote-fw', 'tailscale']:
                open(rd.FW_MARK, 'w').write('tailscale-only\n')
            elif args[:2] == ['remote-fw', 'off'] and os.path.exists(rd.FW_MARK):
                os.remove(rd.FW_MARK)
            return {'ok': True}
        self.patch(rd, 'run', run)
        self.patch(rd, 'root', root)
        self.patch(rd, 'gpu', lambda: 'nvidia')
        self.patch(rd, 'unit_file', lambda u: '/usr/lib/systemd/user/' + u)
        self.patch(rd, 'wait_api', lambda *a, **k: True)
        self.patch(rd, 'ensure_login', lambda: True)
        self.patch(rd, 'which', lambda n: None)

    def conf(self, text=None):
        if text is not None:
            os.makedirs(rd.SUN_DIR, exist_ok=True)
            open(rd.SUN_CONF, 'w').write(text)
        return open(rd.SUN_CONF).read() if os.path.exists(rd.SUN_CONF) else ''

    # ---- settings
    def test_off_by_default_and_saved_only_when_changed(self):
        self.assertEqual(rd.load(), {'on': False, 'reach': 'tailscale', 'keepAwake': True})
        self.assertFalse(os.path.exists(rd.SETTINGS), 'reading never writes defaults')
        self.assertFalse(rd.set_key('reach', 'tailscale'), 'the default is what you have')
        self.assertFalse(os.path.exists(rd.SETTINGS))
        self.assertTrue(rd.set_key('keepAwake', 'off'))
        self.assertEqual(json.load(open(rd.SETTINGS)), {'keepAwake': False}, 'only the changed key')
        with self.assertRaises(rd.Fail):
            rd.set_key('reach', 'internet')
        status = rd.status()
        self.assertFalse(status['on'])
        self.assertEqual(json.load(open(rd.SETTINGS)), {'keepAwake': False}, 'status saved nothing')

    def test_a_damaged_settings_file_is_never_overwritten_with_defaults(self):
        os.makedirs(os.path.dirname(rd.SETTINGS))
        open(rd.SETTINGS, 'w').write('{"on": tr')
        self.assertFalse(rd.load()['on'])
        self.assertTrue(rd.set_key('keepAwake', 'off'))
        self.assertTrue([f for f in os.listdir(os.path.dirname(rd.SETTINGS)) if '.broken-' in f], 'the damaged one kept aside')

    # ---- Sunshine's config
    def test_conf_set_keeps_every_other_line(self):
        t = '# mine\nport = 47989\nencoder = vaapi\nencoder = software\n'
        self.assertEqual(rd.conf_get(t, 'encoder'), 'software', 'the last one counts, as in Sunshine')
        self.assertEqual(rd.conf_set(t, 'encoder', 'nvenc'), '# mine\nport = 47989\nencoder = nvenc\n')
        self.assertEqual(rd.conf_set(t, 'upnp', 'disabled'), t + 'upnp = disabled\n')
        self.assertEqual(rd.conf_set(t, 'encoder', None), '# mine\nport = 47989\n')
        self.assertEqual(rd.sunshine_port('port = 50000\n'), 50000)
        self.assertEqual(rd.sunshine_port('port = 80\n'), 47989)

    def test_hearth_s_values_go_in_and_come_back_out(self):
        self.conf('encoder = software\nmin_log_level = info\nglobal_prep_cmd = [{"do": "mine", "undo": "x"}]\n')
        st = {}
        self.assertTrue(rd.tune_apply(st, 'nvidia'))
        t = self.conf()
        for line in ('encoder = nvenc', 'nvenc_preset = 1', 'upnp = disabled', 'origin_web_ui_allowed = pc', 'stream_audio = enabled', 'min_log_level = info'):
            self.assertIn(line, t)
        prep = json.loads(rd.conf_get(t, 'global_prep_cmd'))
        self.assertEqual(prep[0], {'do': 'mine', 'undo': 'x'}, 'your own prep command stays')
        self.assertIn('hearth-remote-desktop session start', prep[1]['do'])
        self.assertTrue(os.path.exists(rd.SUN_CONF + '.bak'))
        self.assertFalse(rd.tune_apply(st, 'nvidia'), 'twice changes nothing')
        self.assertEqual(st['before']['encoder'], 'software', 'what was there first is remembered')
        # You change one of them yourself afterwards: that one stays when it's turned off.
        self.conf(rd.conf_set(self.conf(), 'nvenc_preset', '4'))
        rd.tune_undo(st)
        t = self.conf()
        self.assertEqual(rd.conf_get(t, 'encoder'), 'software')
        self.assertEqual(rd.conf_get(t, 'nvenc_preset'), '4')
        self.assertIsNone(rd.conf_get(t, 'upnp'))
        self.assertEqual(json.loads(rd.conf_get(t, 'global_prep_cmd')), [{'do': 'mine', 'undo': 'x'}])

    def test_vaapi_elsewhere_and_auto_without_a_card(self):
        self.assertEqual(rd.tuned('', 'vaapi')['encoder'], 'vaapi')
        self.assertNotIn('nvenc_preset', rd.tuned('', 'vaapi'))
        self.assertNotIn('encoder', rd.tuned('', ''))
        self.assertNotIn('global_prep_cmd', rd.tuned('global_prep_cmd = not json\n', ''), 'a list it can\'t read is left alone')

    def test_the_desktop_app(self):
        self.assertEqual(rd.desktop_app({}), 'sunshine-default', 'no file: Sunshine makes it with Desktop')
        os.makedirs(rd.SUN_DIR, exist_ok=True)
        json.dump({'env': {}, 'apps': [{'name': 'Steam Big Picture'}]}, open(rd.SUN_APPS, 'w'))
        st = {}
        self.assertEqual(rd.desktop_app(st), 'added')
        self.assertEqual(json.load(open(rd.SUN_APPS))['apps'][0], {'name': 'Desktop', 'image-path': 'desktop.png'})
        self.assertEqual(rd.desktop_app(st), 'present')
        self.assertTrue(st['addedDesktop'])

    # ---- on and off
    def test_on_firewall_first_then_sunshine_with_the_session(self):
        r = rd.set_cmd('on', 'on')
        self.assertTrue(r['on'])
        self.assertEqual(self.roots[0], ['remote-fw', 'tailscale', '47989'])
        unit = 'app-dev.lizardbyte.app.Sunshine.service'
        drop = rd.dropin_path(unit)
        self.assertIn(' gate', open(drop).read(), 'no rule, no start')
        self.assertTrue(os.path.islink(rd.wants_link(unit)))
        self.assertIn(unit, self.active)
        self.assertTrue(rd.load()['on'])
        self.assertIn('encoder = nvenc', self.conf())
        # Off: all of it undone, Sunshine stopped (Hearth started it, Remote play's switch isn't on).
        r = rd.set_cmd('on', 'off')
        self.assertFalse(r['on'])
        self.assertFalse(os.path.exists(drop) or os.path.lexists(rd.wants_link(unit)))
        self.assertNotIn(unit, self.active)
        self.assertEqual(self.roots[-1], ['remote-fw', 'off'])
        self.assertNotIn('nvenc', self.conf())
        self.assertFalse(rd.load()['on'])

    def test_off_keeps_sunshine_when_remote_play_has_it_on(self):
        unit = 'app-dev.lizardbyte.app.Sunshine.service'
        self.active.add(unit)
        self.enabled.add(unit)
        rd.set_cmd('on', 'on')
        rd.set_cmd('on', 'off')
        self.assertIn(unit, self.active)

    def test_a_firewall_that_fails_turns_nothing_on(self):
        self.root_fail = 'nftables (nft) isn\'t installed'
        with self.assertRaises(rd.Fail):
            rd.set_cmd('on', 'on')
        self.assertFalse(rd.load()['on'], 'not saved as on')
        self.assertFalse(os.path.exists(rd.SUN_CONF))
        self.assertFalse(os.path.exists(rd.USER_UNITS))
        self.assertFalse(self.active)

    def test_home_network_too_takes_the_rule_away(self):
        rd.set_cmd('on', 'on')
        rd.set_cmd('reach', 'home')
        self.assertEqual(self.roots[-1], ['remote-fw', 'off'])
        self.assertFalse(os.path.exists(rd.dropin_path('app-dev.lizardbyte.app.Sunshine.service')))
        self.assertEqual(rd.load()['reach'], 'home')

    def test_no_sunshine_no_change(self):
        self.patch(rd, 'sunshine_unit', lambda: None)
        with self.assertRaises(rd.Fail):
            rd.set_cmd('on', 'on')
        self.assertFalse(self.roots)

    def test_gate_waits_for_the_rule(self):
        with self.assertRaises(rd.Fail):
            rd.gate(wait=3, sleep=lambda s: None)
        open(rd.FW_MARK, 'w').write('tailscale-only\n')
        self.assertEqual(rd.gate(wait=3, sleep=lambda s: None), {'ok': True})

    # ---- pairing
    def test_pin_checks_and_new_sunshine_s_pairing_id(self):
        seen = []

        def api(method, path, body=None, login=None, timeout=8):
            seen.append((method, path, body))
            if method == 'GET':
                return {'pairings': [{'id': 'a' * 32, 'name': 'x', 'address': '100.70.1.4'}]}
            return {'status': True}
        self.patch(rd, 'api', api)
        for bad in ('123', '12a4', '', '12345'):
            with self.assertRaises(rd.Fail):
                rd.pin(bad)
        with self.assertRaises(rd.Fail):
            rd.pin('1234', 'bad;name$(x)')
        self.assertEqual(rd.pin('1234', 'Work laptop'), {'ok': True, 'paired': 'Work laptop'})
        self.assertEqual(seen[-1], ('POST', '/api/pin', {'pin': '1234', 'name': 'Work laptop', 'pairing_id': 'a' * 32}))

    def test_pin_on_older_sunshine_and_a_wrong_pin(self):
        def api(method, path, body=None, login=None, timeout=8):
            if method == 'GET':
                raise rd.Fail('Sunshine said 404')
            return {'status': False}
        self.patch(rd, 'api', api)
        with self.assertRaises(rd.Fail) as c:
            rd.pin('1234')
        self.assertIn("didn't work", str(c.exception))

    def test_the_login_is_private(self):
        rd.creds_save('hearth', 's3cret')
        self.assertEqual(os.stat(rd.CREDS).st_mode & 0o777, 0o600)
        self.assertEqual(rd.creds(), ('hearth', 's3cret'))
        self.assertNotIn('s3cret', json.dumps(rd.status()))
        self.assertFalse(os.path.exists(rd.SETTINGS) and 's3cret' in open(rd.SETTINGS).read())

    def test_unpair_checks_the_id(self):
        with self.assertRaises(rd.Fail):
            rd.unpair('../x')

    # ---- streaming now
    def test_sunshine_s_log(self):
        log = ('[2026-11-05 09:00:00.000]: Info: Creating encoder [h264_nvenc]\n'
               '[2026-11-05 09:00:01.000]: Info: Streaming bitrate is 20000000\n'
               '[2026-11-05 09:00:02.000]: Info: CLIENT CONNECTED\n')
        s = rd.log_state(log)
        self.assertTrue(s['connected'])
        self.assertEqual((s['bitrate'], s['encoder']), (20000000, 'h264_nvenc'))
        self.assertGreater(s['since'], 0)
        self.assertFalse(rd.log_state(log + '[2026-11-05 10:00:00.000]: Info: CLIENT DISCONNECTED\n')['connected'])

    def ts(self, cur='', relay='fra', tx=0):
        return {'BackendState': 'Running', 'Self': {'HostName': 'chris-hearth', 'DNSName': 'chris-hearth.tail.ts.net.', 'TailscaleIPs': ['100.72.99.97']},
                'Peer': {'k1': {'HostName': 'Work-PC', 'OS': 'windows', 'TailscaleIPs': ['100.70.1.4'], 'Online': True, 'Active': True,
                                'CurAddr': cur, 'Relay': relay, 'TxBytes': tx},
                         'k2': {'HostName': 'Phone', 'OS': 'android', 'TailscaleIPs': ['100.70.1.3'], 'Online': True, 'Active': False, 'TxBytes': 5},
                         'k3': {'HostName': 'Old', 'TailscaleIPs': ['100.70.1.9'], 'Online': False}}}

    def test_direct_or_relay_for_the_device_streaming(self):
        ts = rd.ts_info(self.ts(tx=0), t=100)
        self.assertEqual(ts['name'], 'chris-hearth')
        self.assertEqual(ts['ip'], '100.72.99.97')
        self.assertEqual([p['name'] for p in ts['peers']], ['Work-PC', 'Phone'], 'online devices only')
        ts = rd.ts_info(self.ts(tx=50_000_000), t=110)
        self.assertEqual(ts['peers'][0]['rate'], 5_000_000)
        sess = {'who': 'Work laptop', 'width': 1920, 'height': 1080, 'fps': 60}
        v = rd.stream_view({'connected': True, 'since': 5, 'bitrate': 20000000}, ts, sess)
        self.assertEqual((v['path'], v['relay'], v['who'], v['mbps']), ('relay', 'fra', 'Work laptop', 20.0))
        self.assertEqual(v['tip'], "Your work network is blocking direct connections; it will feel laggier")
        self.assertIn('720p', v['hint'])
        ts = rd.ts_info(self.ts(cur='203.0.113.5:41641', tx=100_000_000), t=120)
        v = rd.stream_view({'connected': True}, ts, sess)
        self.assertEqual(v['path'], 'direct')
        self.assertNotIn('tip', v)
        self.assertIn('1080p at 60 fps', v['hint'])
        self.assertEqual(rd.stream_view({'connected': False}, ts, sess), {'on': False})

    def test_no_tailscale_traffic_means_the_home_network(self):
        j = self.ts()
        for p in j['Peer'].values():
            p['Active'] = False
        v = rd.stream_view({'connected': True}, rd.ts_info(j, t=1), {})
        self.assertEqual(v['path'], 'home')

    def test_check_pings_only_your_own_devices(self):
        self.patch(rd, 'ts_json', lambda: self.ts())
        outs = {'100.70.1.4': 'pong from work-pc (100.70.1.4) via DERP(fra) in 84ms'}

        def run(cmd, timeout=15, env=None):
            return (0, outs[cmd[-1]]) if cmd[:2] == ['tailscale', 'ping'] else (0, '')
        self.patch(rd, 'run', run)
        r = rd.check('100.70.1.4')
        self.assertEqual((r['path'], r['relay'], r['ms']), ('relay', 'fra', 84))
        outs['100.70.1.4'] = 'pong from work-pc (100.70.1.4) via 203.0.113.5:41641 in 23ms'
        self.assertEqual(rd.check('100.70.1.4')['path'], 'direct')
        for bad in ('8.8.8.8', '100.70.1.99', 'x; rm', '100.70.1.9'):
            with self.assertRaises(rd.Fail):
                rd.check(bad)

    def test_the_prep_command_never_fails_and_cleans_what_it_s_given(self):
        for k, v in (('SUNSHINE_CLIENT_NAME', 'Work laptop$(x)'), ('SUNSHINE_CLIENT_WIDTH', '1920'), ('SUNSHINE_CLIENT_HEIGHT', '1080'),
                     ('SUNSHINE_CLIENT_FPS', '60'), ('SUNSHINE_APP_NAME', 'Desktop')):
            self.env(k, v)
        self.assertEqual(rd.session('start'), {'ok': True})
        s = rd.runtime_read('session.json')
        self.assertEqual((s['who'], s['width'], s['height'], s['fps'], s['app']), ('Work laptop(x)', 1920, 1080, 60, 'Desktop'))
        self.assertTrue([c for c in self.calls if c[:2] == ['systemd-run', '--user'] and '--unit=hearth-remote-desktop-awake' in c])
        self.assertEqual(rd.session('end'), {'ok': True})
        self.assertEqual(rd.runtime_read('session.json'), {})
        self.patch(rd, 'runtime_write', lambda *a: 1 / 0)
        self.assertEqual(rd.session('start'), {'ok': True}, 'a failing prep command would stop the stream')

    def test_stay_awake_holds_while_connected_and_lets_go(self):
        states = iter([True, True, False])
        self.patch(rd, 'streaming_now', lambda unit=None: {'connected': next(states, False)})
        started = []

        class P:
            def __init__(self, cmd, **k):
                started.append(cmd)
                self.done = False

            def poll(self):
                return None

            def terminate(self):
                self.done = True
        import subprocess
        self.patch(subprocess, 'Popen', P)
        rd.awake(sleep=lambda s: None, rounds=3)
        self.assertEqual(len(started), 1)
        self.assertEqual(started[0][:4], ['systemd-inhibit', '--what=idle:sleep', '--who=Hearth', '--why=A device is using this PC through Moonlight'])

    def test_quick_for_game_mode(self):
        self.assertEqual(rd.quick(), {'on': False, 'streaming': False})

    def test_status_without_sunshine(self):
        self.patch(rd, 'sunshine_unit', lambda: None)
        s = rd.status()
        self.assertFalse(s['installed'])
        self.assertFalse(s['running'])
        self.assertEqual(s['stream'], {'on': False})


class Wiring(unittest.TestCase):
    def test_the_card_and_its_links(self):
        q = (SHELL / 'SetRemote.qml').read_text()
        for t in ('helper: "hearth-remote-desktop"', '"In Moonlight, add "', '["pin", p, n]', '"unpair"', '["check"]', 'Stay awake while connected',
                  'Wake this PC from away', 'Work computers may not allow'):
            self.assertIn(t, q)
        self.assertIn('Use this PC from anywhere', (SHELL / 'SetHomeGaming.qml').read_text())
        self.assertIn('Use this PC from anywhere', (SHELL / 'SetHomeServices.qml').read_text())
        self.assertTrue((ROOT / 'docs' / 'REMOTE-DESKTOP.md').exists())

    def test_settings_go_through_the_safe_helper(self):
        src = (SHELL / 'hearth-remote-desktop').read_text()
        self.assertIn('sio.merge_json(SETTINGS', src)
        self.assertNotIn("open(SETTINGS, 'w')", src)
        self.assertNotIn('shell=True', src)


if __name__ == '__main__':
    unittest.main()
