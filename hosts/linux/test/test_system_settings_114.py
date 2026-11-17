"""Tests for Hearth Settings' system pages added for 1.14 (KDE System Settings' areas Hearth lacked): VPN, Mobile
hotspot, Proxy and Connection details (hearth-network), Firmware & device security (hearth-firmware), Region & language
(hearth-region), Fonts, cursor & icons (hearth-lookset), Background services (hearth-services), Login screen
(hearth-loginscreen), and their root half (hearth-sysset-admin through hearth-admin). Fake nmcli / fwupdmgr / localectl /
systemctl answers; every program is run with an argument list, and secrets only ever go on stdin."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
UUID = '0f5d6a3e-1b2c-4d5e-8f90-123456789abc'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Home(unittest.TestCase):
    """A fresh HOME for each test (hearth_settings_io logs and notices under it)."""
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.env = mock.patch.dict(os.environ, {'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'),
                                                'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state')})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()


class FakeNm:
    """nmcli stand-in: answers by the arguments, remembers every call (argv and stdin)."""
    def __init__(self, answers):
        self.answers = answers
        self.calls = []

    def __call__(self, cmd, input=None, timeout=60):
        self.calls.append((list(cmd), input))
        for match, out in self.answers:
            if all(a in cmd for a in match):
                return (0, out, '') if not isinstance(out, tuple) else out
        return 0, '', ''


CONNS = ('Home WiFi:' + UUID + ':802-11-wireless:wlan0:yes:yes\n'
         'lo:11111111-2222-3333-4444-555555555555:loopback:lo:yes:no\n'
         'work\\:vpn:22222222-2222-3333-4444-555555555555:vpn::no:no\n'
         'wg-home:33333333-2222-3333-4444-555555555555:wireguard::no:no\n')


class Network(Home):
    def setUp(self):
        super().setUp()
        self.n = load(SHELL / 'hearth-network', 'hearth_network')
        self.n.PROXY_JSON = os.path.join(self.home, '.config', 'nimbus', 'proxy.json')
        self.n.PROXY_ENV = os.path.join(self.home, '.config', 'environment.d', '90-hearth-proxy.conf')
        self.n.KIOSLAVERC = os.path.join(self.home, '.config', 'kioslaverc')

    def test_connections_parse_escapes_and_skip_loopback(self):
        cs = self.n.parse_connections(CONNS)
        self.assertEqual([c['name'] for c in cs], ['Home WiFi', 'work:vpn', 'wg-home'])
        self.assertEqual([c['kind'] for c in cs], ['wifi', 'vpn', 'vpn'])
        self.assertTrue(cs[0]['active'] and cs[0]['autoconnect'])

    def test_status_lists_vpns_and_the_wifi_device(self):
        fake = FakeNm([(['connection', 'show', '-t'], CONNS), (['device'], 'wlan0:wifi:connected\neth0:ethernet:unavailable\n'),
                       (['vpn.service-type'], 'org.freedesktop.NetworkManager.openvpn\n')])
        with mock.patch.object(self.n, 'run', fake):
            st = self.n.status()
        self.assertEqual(st['wifiDevice'], 'wlan0')
        self.assertEqual([(v['name'], v['vpnType']) for v in st['vpns']], [('work:vpn', 'OpenVPN'), ('wg-home', 'WireGuard')])

    def test_vpn_password_goes_on_stdin_never_argv(self):
        fake = FakeNm([])
        with mock.patch.object(self.n, 'run', fake):
            self.n.vpn_up(UUID, {'user': 'me', 'password': 's3cret pass'})
        for argv, stdin in fake.calls:
            self.assertNotIn('s3cret pass', ' '.join(argv))
        up = [c for c in fake.calls if 'up' in c[0]][0]
        self.assertIn('passwd-file', up[0])
        self.assertEqual(up[0][up[0].index('passwd-file') + 1], '/dev/stdin')
        self.assertEqual(up[1], 'vpn.secrets.password:s3cret pass\n')
        with self.assertRaises(self.n.Fail):
            self.n.vpn_up(UUID, {'password': 'two\nlines'})
        with self.assertRaises(self.n.Fail):
            self.n.vpn_up('$(reboot)')

    def test_vpn_import_checks_the_file_and_never_autoconnects(self):
        d = os.path.join(self.home, 'Downloads'); os.makedirs(d)
        wg = os.path.join(d, 'home-vpn.conf'); open(wg, 'w').write('[Interface]\n')
        fake = FakeNm([(['import'], "Connection 'home-vpn' (%s) successfully added.\n" % UUID)])
        with mock.patch.object(self.n, 'run', fake):
            r = self.n.vpn_import(wg)
        self.assertEqual((r['uuid'], r['name'], r['kind']), (UUID, 'home-vpn', 'wireguard'))
        self.assertEqual(fake.calls[0][0], ['nmcli', 'connection', 'import', 'type', 'wireguard', 'file', wg])
        self.assertIn(['nmcli', 'connection', 'modify', 'uuid', UUID, 'connection.autoconnect', 'no'], [c[0] for c in fake.calls])
        bad = os.path.join(d, 'notes.txt'); open(bad, 'w').write('x')
        long = os.path.join(d, 'a-very-long-vpn-name-here.conf'); open(long, 'w').write('x')
        for f in (bad, long, os.path.join(d, 'missing.conf')):
            with self.assertRaises(self.n.Fail):
                self.n.vpn_import(f)
        ovpn = os.path.join(d, 'work.ovpn'); open(ovpn, 'w').write('client\n')
        self.n.VPN_PLUGINS = os.path.join(self.home, 'none')
        with self.assertRaises(self.n.Fail):
            self.n.vpn_import(ovpn)      # no OpenVPN plugin: says so instead of failing in nmcli

    def test_remove_only_removes_vpns(self):
        fake = FakeNm([(['connection.type'], '802-11-wireless\n')])
        with mock.patch.object(self.n, 'run', fake), self.assertRaises(self.n.Fail):
            self.n.vpn_remove(UUID)
        self.assertFalse(any('delete' in c[0] for c in fake.calls))

    def test_details_args_are_checked(self):
        a = self.n.details_args({'dns': ['1.1.1.1', '2606:4700:4700::1111'], 'autoDns': False, 'metered': 'yes',
                                 'mac': 'random', 'autoconnect': True, 'ipv6': 'disabled'}, '802-11-wireless')
        self.assertEqual(a, ['ipv4.dns', '1.1.1.1', 'ipv6.dns', '2606:4700:4700::1111', 'ipv4.ignore-auto-dns', 'yes',
                             'ipv6.ignore-auto-dns', 'yes', 'connection.metered', 'yes', 'connection.autoconnect', 'yes',
                             '802-11-wireless.cloned-mac-address', 'random', 'ipv6.method', 'disabled'])
        self.assertEqual(self.n.details_args({'ipv4': {'method': 'manual', 'address': '192.168.1.20', 'gateway': '192.168.1.1'}}, '802-3-ethernet'),
                         ['ipv4.method', 'manual', 'ipv4.addresses', '192.168.1.20/24', 'ipv4.gateway', '192.168.1.1'])
        self.assertEqual(self.n.details_args({'mac': 'default'}, '802-3-ethernet'), ['802-3-ethernet.cloned-mac-address', ''])
        for bad in ({'dns': ['1.1.1.1; reboot']}, {'metered': 'maybe'}, {'mac': 'aa:bb'}, {'ipv4': {'method': 'manual', 'address': 'x'}},
                    {'ipv4': {'method': 'manual', 'address': '10.0.0.2/8', 'gateway': 'nope'}}, {'ipv6': 'on'}):
            with self.assertRaises(self.n.Fail, msg=bad):
                self.n.details_args(bad, '802-11-wireless')
        with self.assertRaises(self.n.Fail):
            self.n.details_args({'mac': 'random'}, 'vpn')

    def test_details_read(self):
        show = ('connection.id:Home WiFi\nconnection.type:802-11-wireless\nconnection.autoconnect:yes\nconnection.metered:no\n'
                'ipv4.method:auto\nipv4.addresses:\nipv4.gateway:--\nipv4.dns:9.9.9.9\nipv4.ignore-auto-dns:yes\n'
                'ipv6.method:auto\nipv6.dns:\n802-11-wireless.cloned-mac-address:stable\nGENERAL.DEVICES:wlan0\n'
                'IP4.ADDRESS[1]:192.168.1.30/24\nIP4.GATEWAY:192.168.1.1\nIP4.DNS[1]:192.168.1.1\nIP4.DNS[2]:9.9.9.9\n')
        with mock.patch.object(self.n, 'run', FakeNm([(['show', 'uuid'], show)])):
            d = self.n.details(UUID)
        self.assertEqual((d['mac'], d['metered'], d['autoDns'], d['dns']), ('stable', 'no', False, ['9.9.9.9']))
        self.assertEqual(d['now'], {'device': 'wlan0', 'address': '192.168.1.30/24', 'gateway': '192.168.1.1',
                                    'dns': ['192.168.1.1', '9.9.9.9'], 'address6': ''})

    def test_hotspot_password_goes_through_the_editor_on_stdin(self):
        fake = FakeNm([(['device', '-t'], 'wlan0:wifi:connected\n'), (['-t', 'connection', 'show', '-f'], ''),
                       (['add'], "Connection 'Hearth Hotspot' (%s) successfully added.\n" % UUID)])
        with mock.patch.object(self.n, 'run', fake):
            self.n.hotspot_on({'ssid': 'Den', 'password': 'pass word 99', 'band': 'bg'})
        for argv, stdin in fake.calls:
            self.assertNotIn('pass word 99', ' '.join(argv))
        edit = [c for c in fake.calls if 'edit' in c[0]][0]
        self.assertEqual(edit[0], ['nmcli', 'connection', 'edit', 'uuid', UUID])
        self.assertEqual(edit[1], 'set 802-11-wireless-security.psk pass word 99\nsave persistent\nquit\n')
        add = [c for c in fake.calls if 'add' in c[0]][0][0]
        self.assertIn('ap', add); self.assertIn('shared', add)
        self.assertEqual(add[add.index('connection.autoconnect') + 1], 'no')   # never starts by itself
        for bad in ({'ssid': '', 'password': '12345678'}, {'ssid': 'x' * 33, 'password': '12345678'},
                    {'ssid': 'Den', 'password': 'short'}, {'ssid': 'Den', 'password': 'émoji-passé'}, {'ssid': 'Den', 'band': '6g'}):
            with self.assertRaises(self.n.Fail, msg=bad):
                self.n.hotspot_check(bad)

    def test_proxy_saved_with_settings_io_and_handed_on(self):
        with mock.patch.object(self.n, 'run', FakeNm([])), mock.patch.object(self.n.shutil, 'which', return_value=None):
            p = self.n.proxy_set({'mode': 'manual', 'http': 'http://proxy.lan:3128/', 'https': 'proxy.lan:3128', 'ignore': 'localhost, .lan'})
            self.assertEqual(p['http'], 'proxy.lan:3128')
            saved = json.load(open(self.n.PROXY_JSON))
            self.assertEqual(saved['mode'], 'manual')
            env = open(self.n.PROXY_ENV).read()
            self.assertTrue(env.startswith(self.n.MARK))
            self.assertIn('http_proxy=http://proxy.lan:3128', env)
            self.assertIn('no_proxy=localhost,.lan', env)
            kio = open(self.n.KIOSLAVERC).read()
            self.assertIn('[Proxy Settings]\nProxyType=1\n', kio)
            self.assertIn('httpProxy=http://proxy.lan 3128', kio)
            # A bad value saves nothing.
            before = open(self.n.PROXY_JSON).read()
            for bad in ({'http': 'proxy.lan'}, {'mode': 'auto', 'url': ''}, {'url': 'javascript:x'}, {'mode': 'sometimes'}):
                with self.assertRaises(self.n.Fail, msg=bad):
                    self.n.proxy_set(bad)
            self.assertEqual(open(self.n.PROXY_JSON).read(), before)
            # Off: Hearth's env file goes; kioslaverc keeps its other lines.
            with open(self.n.KIOSLAVERC, 'a') as f:
                f.write('\n[Other]\nKeep=1\n')
            self.n.proxy_set({'mode': 'none'})
            self.assertFalse(os.path.exists(self.n.PROXY_ENV))
            self.assertIn('Keep=1', open(self.n.KIOSLAVERC).read())
            self.assertIn('ProxyType=0', open(self.n.KIOSLAVERC).read())
            self.assertEqual(json.load(open(self.n.PROXY_JSON))['http'], 'proxy.lan:3128')   # merge: the rest is kept
            self.assertTrue(os.path.exists(self.n.PROXY_JSON + '.bak'))

    def test_proxy_env_of_your_own_is_left_alone(self):
        os.makedirs(os.path.dirname(self.n.PROXY_ENV))
        open(self.n.PROXY_ENV, 'w').write('http_proxy=http://mine:8080\n')
        with mock.patch.object(self.n, 'run', FakeNm([])), mock.patch.object(self.n.shutil, 'which', return_value=None):
            self.n.proxy_set({'mode': 'manual', 'http': 'proxy.lan:3128'})
            self.n.proxy_set({'mode': 'none'})
        self.assertEqual(open(self.n.PROXY_ENV).read(), 'http_proxy=http://mine:8080\n')

    def test_gsettings_commands(self):
        p = dict(self.n.PROXY_DEFAULT, mode='manual', http='proxy.lan:3128', socks='[::1]:1080')
        cmds = self.n.gsettings_cmds(p)
        self.assertIn(['org.gnome.system.proxy', 'mode', 'manual'], cmds)
        self.assertIn(['org.gnome.system.proxy.http', 'host', 'proxy.lan'], cmds)
        self.assertIn(['org.gnome.system.proxy.socks', 'host', '::1'], cmds)
        self.assertIn(['org.gnome.system.proxy.socks', 'port', '1080'], cmds)


DEVICES = {'Devices': [
    {'Name': 'System Firmware', 'DeviceId': 'a' * 40, 'Vendor': 'Framework', 'Version': '3.05', 'Plugin': 'uefi_capsule',
     'Flags': ['internal', 'updatable', 'needs-reboot']},
    {'Name': 'Unifying Receiver', 'DeviceId': 'b' * 40, 'Vendor': 'Logitech', 'Version': 'RQR12.10', 'Plugin': 'logitech_hidpp',
     'Flags': ['updatable']},
    {'Name': 'TPM', 'DeviceId': 'c' * 40, 'Version': '7.2', 'Flags': ['internal']}]}
UPDATES = {'Devices': [{'Name': 'Unifying Receiver', 'DeviceId': 'b' * 40, 'Releases': [
    {'Version': 'RQR12.11', 'Summary': 'Firmware for the receiver', 'Description': '<p>Fixes a <b>security</b> problem.</p>', 'Urgency': 'high', 'Size': 1000}]}]}


class Firmware(Home):
    def setUp(self):
        super().setUp()
        self.f = load(SHELL / 'hearth-firmware', 'hearth_firmware')

    def test_secure_boot_words(self):
        self.assertEqual(self.f.parse_sb_state('SecureBoot enabled\n'), 'on')
        self.assertEqual(self.f.parse_sb_state('SecureBoot disabled\nPlatform is in Setup Mode\n'), 'off')
        self.assertEqual(self.f.parse_sb_state("This system doesn't support Secure Boot"), 'unsupported')
        self.assertEqual(self.f.parse_sb_state('EFI variables are not supported on this system'), 'unsupported')

    def test_status_from_sysfs(self):
        sysfs = os.path.join(self.home, 'sys')
        ev = os.path.join(sysfs, 'firmware', 'efi', 'efivars'); os.makedirs(ev)
        open(os.path.join(ev, self.f.SB_VAR), 'wb').write(b'\x06\x00\x00\x00\x01')
        os.makedirs(os.path.join(sysfs, 'class', 'tpm', 'tpm0'))
        open(os.path.join(sysfs, 'class', 'tpm', 'tpm0', 'tpm_version_major'), 'w').write('2\n')
        bat = os.path.join(sysfs, 'class', 'power_supply', 'BAT0'); os.makedirs(bat)
        open(os.path.join(bat, 'type'), 'w').write('Battery\n'); open(os.path.join(bat, 'capacity'), 'w').write('81\n')
        self.f.SYS = sysfs
        with mock.patch.object(self.f.shutil, 'which', return_value=None):
            st = self.f.status()
        self.assertEqual((st['uefi'], st['secureBoot'], st['tpm']), (True, 'on', {'present': True, 'version': '2.0'}))
        self.assertEqual(st['power'], {'ac': None, 'battery': 81})
        self.f.SYS = os.path.join(self.home, 'nothing')
        with mock.patch.object(self.f.shutil, 'which', return_value=None):
            st = self.f.status()
        self.assertEqual((st['uefi'], st['secureBoot'], st['tpm']['present']), (False, 'unsupported', False))

    def test_devices_and_updates(self):
        ds = self.f.parse_devices(DEVICES, UPDATES)
        self.assertEqual([d['name'] for d in ds], ['Unifying Receiver', 'System Firmware', 'TPM'])
        self.assertEqual(ds[0]['update']['version'], 'RQR12.11')
        self.assertEqual(ds[0]['update']['description'], 'Fixes a  security  problem.')
        self.assertTrue(ds[1]['system'] and ds[1]['needsReboot'])
        self.assertFalse(ds[0]['system'])

    def test_fwupd_nothing_to_do_is_no_error(self):
        with mock.patch.object(self.f.shutil, 'which', return_value='/usr/bin/fwupdmgr'), \
             mock.patch.object(self.f, 'run', return_value=(2, 'No updatable devices\n', '')):
            self.assertEqual(self.f.fwupd_json(['get-updates']), {})

    def test_security_level(self):
        js = {'SecurityAttributes': [
            {'AppstreamId': 'org.fwupd.hsi.Tpm.Version20', 'Name': 'TPM v2.0', 'HsiLevel': 1, 'HsiResult': 'found', 'Flags': ['success']},
            {'AppstreamId': 'org.fwupd.hsi.Uefi.SecureBoot', 'Name': 'UEFI secure boot', 'HsiLevel': 1, 'HsiResult': 'enabled', 'Flags': ['success']},
            {'AppstreamId': 'org.fwupd.hsi.Iommu', 'Name': 'IOMMU', 'HsiLevel': 2, 'HsiResult': 'not-found', 'Flags': []}]}
        r = self.f.parse_security(js)
        self.assertEqual((r['level'], r['hsi']), (1, 'HSI:1'))
        self.assertEqual(r['items'][0]['name'], 'IOMMU')    # what isn't fine comes first


class SysAdmin(Home):
    def setUp(self):
        super().setUp()
        self.a = load(NIMBUS / 'hearth-sysset-admin', 'hearth_sysset_admin')

    def test_locale_args(self):
        known = {'en_US.utf8', 'de_DE.utf8', 'C.utf8'}
        self.assertEqual(self.a.locale_args(['LANG=en_US.UTF-8', 'LC_TIME=de_DE.UTF-8', 'LC_PAPER='], known),
                         ['LANG=en_US.UTF-8', 'LC_TIME=de_DE.UTF-8'])
        for bad in (['LC_TIME=de_DE.UTF-8'], ['LANG=fr_FR.UTF-8'], ['LANG=en_US.UTF-8', 'LANG=de_DE.UTF-8'],
                    ['LD_PRELOAD=/tmp/x'], ['LANG=en_US.UTF-8;reboot'], ['LANG=../../etc']):
            with self.assertRaises(ValueError, msg=bad):
                self.a.locale_args(bad, known)

    def test_set_locale_runs_localectl_with_a_list(self):
        calls = []
        def fake(cmd, input=None, timeout=600):
            calls.append(cmd)
            return (0, 'en_US.utf8\nde_DE.utf8\n') if cmd[0] == 'locale' else (0, '')
        with mock.patch.dict(os.environ, {'PKEXEC_UID': str(os.getuid())}), mock.patch.object(self.a, 'run', fake), redirect_stdout(io.StringIO()):
            self.a.set_locale(['LANG=de_DE.UTF-8'])
        self.assertEqual(calls[-1], ['localectl', 'set-locale', 'LANG=de_DE.UTF-8'])

    def test_user_type_plan(self):
        accts = [{'name': 'me', 'admin': True}, {'name': 'kid', 'admin': False}, {'name': 'dad', 'admin': True}]
        groups = {'wheel', 'kid'}
        self.assertEqual(self.a.user_type_plan('kid', 'admin', 'me', accts, groups), [['gpasswd', '-a', 'kid', 'wheel']])
        self.assertEqual(self.a.user_type_plan('kid', 'standard', 'me', accts, groups), [])
        with self.assertRaises(ValueError):
            self.a.user_type_plan('me', 'standard', 'me', accts, groups)             # never yourself
        with self.assertRaises(ValueError):
            self.a.user_type_plan('dad', 'standard', 'kid', [{'name': 'dad', 'admin': True}, {'name': 'kid', 'admin': False}], groups)   # last admin
        for bad in ('root', 'Bad Name', '-rf', 'nobody'):
            with self.assertRaises(ValueError):
                self.a.user_type_plan(bad, 'admin', 'me', accts, groups)
        with self.assertRaises(ValueError):
            self.a.user_type_plan('kid', 'root', 'me', accts, groups)

    def test_login_background(self):
        d = os.path.join(self.home, 'login')
        png = b'\x89PNG\r\n\x1a\n' + b'0' * 100
        r = self.a.login_background(png, d)
        self.assertEqual(open(r['background'], 'rb').read(), png)
        self.assertEqual(os.stat(r['background']).st_mode & 0o777, 0o644)   # the greeter (sddm) reads it
        with self.assertRaises(ValueError):
            self.a.login_background(b'<svg/>', d)
        self.assertEqual(open(r['background'], 'rb').read(), png)           # a bad picture changes nothing
        self.assertEqual(self.a.login_background(b'', d)['background'], '')
        self.assertFalse(os.path.exists(os.path.join(d, 'background')))

    def test_firmware_update_checks_the_id_and_mains(self):
        calls = []
        def fake(cmd, input=None, timeout=600):
            calls.append(cmd)
            if 'get-devices' in cmd:
                return 0, json.dumps(DEVICES)
            return 0, 'Successfully installed firmware'
        sysfs = os.path.join(self.home, 'sys'); bat = os.path.join(sysfs, 'class', 'power_supply', 'BAT0'); os.makedirs(bat)
        open(os.path.join(bat, 'type'), 'w').write('Battery\n')
        self.a.SYS = sysfs
        with mock.patch.dict(os.environ, {'PKEXEC_UID': str(os.getuid())}), mock.patch.object(self.a, 'run', fake):
            out = io.StringIO()
            with redirect_stdout(out), self.assertRaises(SystemExit):
                self.a.firmware_update('a' * 40)            # system firmware on battery: refused
            self.assertIn('charger', out.getvalue())
            self.assertFalse(any('update' in c for c in calls))
            with redirect_stdout(io.StringIO()):
                self.a.firmware_update('b' * 40)
            self.assertEqual(calls[-1], ['fwupdmgr', 'update', 'b' * 40, '-y', '--no-reboot-check'])
            with redirect_stdout(io.StringIO()), self.assertRaises(SystemExit):
                self.a.firmware_update('b' * 40 + ' --force')

    def test_hearth_admin_hands_these_on(self):
        src = (NIMBUS / 'hearth-admin').read_text()
        self.assertIn("elif cmd in ('firmware-refresh', 'firmware-update', 'locale', 'user-type', 'login-background'):", src)
        self.assertIn("'hearth-sysset-admin'", src)


class Region(Home):
    def setUp(self):
        super().setUp()
        self.r = load(SHELL / 'hearth-region', 'hearth_region')

    def test_localectl_parse(self):
        text = ('   System Locale: LANG=en_US.UTF-8\n                  LC_TIME=de_DE.UTF-8\n                  LC_PAPER=en_GB.UTF-8\n'
                '       VC Keymap: us\n      X11 Layout: us\n')
        self.assertEqual(self.r.parse_localectl(text), {'LANG': 'en_US.UTF-8', 'LC_TIME': 'de_DE.UTF-8', 'LC_PAPER': 'en_GB.UTF-8'})

    def test_locales_named_from_i18n(self):
        d = os.path.join(self.home, 'i18n'); os.makedirs(d)
        open(os.path.join(d, 'de_DE'), 'w').write('LC_IDENTIFICATION\ntitle "German locale for Germany"\nlanguage "German"\nterritory "Germany"\n')
        self.r.I18N = d
        ls = self.r.locales('C\nC.utf8\nPOSIX\nde_DE.utf8\nen_US.utf8\nen_US.UTF-8\nja_JP.eucJP\n')
        self.assertEqual([l['id'] for l in ls], ['en_US.UTF-8', 'de_DE.UTF-8', 'C.UTF-8'])
        self.assertEqual(ls[1]['name'], 'German (Germany)')
        self.assertEqual(ls[0]['name'], 'en_US')     # no i18n file: its code

    def test_preview_of_the_c_locale(self):
        p = self.r.preview('C.UTF-8')
        self.assertEqual(p['number'], '1234567.89')
        self.assertIn('12/24/26', p['date'])
        with self.assertRaises(self.r.Fail):
            self.r.preview('$(x)')


class Lookset(Home):
    def setUp(self):
        super().setUp()
        self.l = load(SHELL / 'hearth-lookset', 'hearth_lookset')
        c = os.path.join(self.home, '.config')
        self.l.CONFIG = c
        self.l.SAVED = os.path.join(c, 'nimbus', 'fonts-cursor-icons.json')
        self.l.FONTCONF = os.path.join(c, 'fontconfig', 'conf.d', '90-hearth-fonts.conf')
        self.l.CURSOR_ENV = os.path.join(c, 'environment.d', '90-hearth-cursor.conf')
        self.c = c

    def test_clean(self):
        self.assertEqual(self.l.clean({'fontSize': 11.3, 'cursorSize': 32, 'icons': None}), {'fontSize': 11.5, 'cursorSize': 32, 'icons': None})
        for bad in ({'font': 'Evil"; rm'}, {'fontSize': 100}, {'cursorSize': 25}, {'hinting': 'max'}, {'icons': '../x'},
                    {'dpi': 10}, {'mystery': 1}):
            with self.assertRaises(self.l.Fail, msg=bad):
                self.l.clean(bad)

    def test_nothing_chosen_writes_nothing(self):
        with mock.patch.object(self.l.shutil, 'which', return_value=None):
            self.assertEqual(self.l.apply(), [])
        self.assertFalse(os.path.exists(self.c))

    def test_set_saves_merges_and_hands_on_only_to_hearths_files(self):
        qt = os.path.join(self.c, 'qt6ct', 'qt6ct.conf'); os.makedirs(os.path.dirname(qt))
        open(qt, 'w').write(self.l.QT_MARK + '\n[Appearance]\nicon_theme=breeze-dark\nstyle=Breeze\n')
        qt5 = os.path.join(self.c, 'qt5ct', 'qt5ct.conf'); os.makedirs(os.path.dirname(qt5))
        open(qt5, 'w').write('[Appearance]\nicon_theme=mine\n')                       # yours: left alone
        kg = os.path.join(self.c, 'kdeglobals')
        open(kg, 'w').write('# Written by Hearth (Settings > Appearance); delete this line to keep your own settings.\n[Icons]\nTheme=breeze-dark\n')
        calls = []
        with mock.patch.object(self.l.shutil, 'which', return_value='/usr/bin/x'), \
             mock.patch.object(self.l, 'run', lambda cmd, timeout=20: (calls.append(cmd), (0, ''))[1]):
            self.l.set_({'icons': 'Papirus-Dark'})
            self.l.set_({'font': 'Inter', 'fontSize': 11})
            self.l.set_({'cursor': 'Adwaita', 'cursorSize': 32, 'antialias': 'rgba'})
        saved = json.load(open(self.l.SAVED))
        self.assertEqual(saved, {'icons': 'Papirus-Dark', 'font': 'Inter', 'fontSize': 11, 'cursor': 'Adwaita', 'cursorSize': 32, 'antialias': 'rgba'})
        q = open(qt).read()
        self.assertIn('icon_theme=Papirus-Dark', q)
        self.assertIn('style=Breeze', q)
        self.assertIn('[Fonts]\ngeneral="Inter,11,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"', q)
        self.assertEqual(open(qt5).read(), '[Appearance]\nicon_theme=mine\n')
        self.assertIn('[Icons]\nTheme=Papirus-Dark', open(kg).read())
        self.assertIn('[Mouse]\ncursorTheme=Adwaita\ncursorSize=32', open(os.path.join(self.c, 'kcminputrc')).read())
        self.assertIn('XCURSOR_THEME=Adwaita\nXCURSOR_SIZE=32', open(self.l.CURSOR_ENV).read())
        self.assertIn('<const>rgb</const>', open(self.l.FONTCONF).read())
        self.assertIn(['gsettings', 'set', 'org.gnome.desktop.interface', 'font-name', 'Inter 11'], calls)
        self.assertIn(['gsettings', 'set', 'org.gnome.desktop.interface', 'cursor-theme', 'Adwaita'], calls)
        for c in calls:
            self.assertIsInstance(c, list)
        # Back to Hearth's default: the key comes out; Hearth's fontconfig file goes.
        with mock.patch.object(self.l.shutil, 'which', return_value=None):
            self.l.set_({'antialias': None})
        self.assertNotIn('antialias', json.load(open(self.l.SAVED)))
        self.l.apply()
        self.assertFalse(os.path.exists(self.l.FONTCONF))
        self.assertTrue(os.path.exists(self.l.SAVED + '.bak'))

    def test_a_fontconfig_file_of_yours_is_kept(self):
        os.makedirs(os.path.dirname(self.l.FONTCONF))
        open(self.l.FONTCONF, 'w').write('<fontconfig/>\n')
        with mock.patch.object(self.l.shutil, 'which', return_value=None):
            self.l.set_({'hinting': 'full'})
        self.assertEqual(open(self.l.FONTCONF).read(), '<fontconfig/>\n')

    def test_theme_lists(self):
        icons = os.path.join(self.home, 'icons')
        for name, idx, cursors in (('Papirus', '[Icon Theme]\nName=Papirus\nDirectories=48x48/apps\n', False),
                                   ('Bibata', '[Icon Theme]\nName=Bibata Modern\n', True), ('hicolor', '[Icon Theme]\nDirectories=x\n', False)):
            os.makedirs(os.path.join(icons, name))
            open(os.path.join(icons, name, 'index.theme'), 'w').write(idx)
            if cursors:
                os.makedirs(os.path.join(icons, name, 'cursors'))
        self.l.ICON_DIRS = [icons]
        self.assertEqual(self.l.icon_themes(), [{'id': 'Papirus', 'name': 'Papirus'}])
        self.assertEqual(self.l.cursor_themes(), [{'id': 'Bibata', 'name': 'Bibata Modern'}])
        self.assertEqual(self.l.font_families('Noto Sans,Noto Sans Regular\nHack\n.LastResort\nHack\n'), ['Hack', 'Noto Sans'])

    def test_apply_sh_runs_it_last(self):
        sh = (ROOT / 'hosts' / 'linux' / 'desktop' / 'theme' / 'apply.sh').read_text()
        self.assertLess(sh.index('cursor-theme breeze_cursors'), sh.index('hearth-lookset" apply'))


class Services(Home):
    def setUp(self):
        super().setUp()
        self.s = load(SHELL / 'hearth-services', 'hearth_services')

    def test_list(self):
        files = ('syncthing.service enabled enabled\nhearth-companion.service enabled enabled\npipewire.service enabled enabled\n'
                 'foo@.service disabled enabled\nstatic-thing.service static -\nmpd.service disabled enabled\n')
        show = ('Id=syncthing.service\nDescription=Syncthing\nActiveState=active\nSubState=running\nUnitFileState=enabled\n\n'
                'Id=mpd.service\nDescription=Music Player Daemon\nActiveState=inactive\nSubState=dead\nUnitFileState=disabled\n')
        def fake(cmd, timeout=30):
            return (0, files, '') if 'list-unit-files' in cmd else (0, show, '')
        with mock.patch.object(self.s, 'run', fake):
            rows = self.s.services()['services']
        self.assertEqual([(r['id'], r['locked']) for r in rows],
                         [('mpd.service', False), ('syncthing.service', False), ('hearth-companion.service', True), ('pipewire.service', True)])
        self.assertTrue(rows[1]['running'] and rows[1]['enabled'])

    def test_set(self):
        calls = []
        def fake(cmd, timeout=30):
            calls.append(cmd)
            if 'list-unit-files' in cmd:
                return 0, 'syncthing.service enabled enabled\n', ''
            if 'is-enabled' in cmd:
                return 0, 'disabled\n', ''
            return 0, '', ''
        with mock.patch.object(self.s, 'run', fake):
            self.s.set_('syncthing.service', False)
            self.assertIn(['systemctl', '--user', 'disable', '--now', 'syncthing.service'], calls)
            self.assertFalse(any('mask' in c for c in calls))
            for bad in ('pipewire.service', 'hearth-companion.service', 'x; reboot.service', 'nimbus-shell.service', 'thing.timer'):
                with self.assertRaises(self.s.Fail, msg=bad):
                    self.s.set_(bad, False)
            with self.assertRaises(self.s.Fail):
                self.s.set_('other.service', True)          # not one of yours

    def test_a_service_for_everyone_is_masked_for_you(self):
        calls = []
        def fake(cmd, timeout=30):
            calls.append(cmd)
            if 'list-unit-files' in cmd:
                return 0, 'geoclue-agent.service enabled enabled\n', ''
            if 'is-enabled' in cmd:
                return 0, 'enabled\n', ''
            return 0, '', ''
        with mock.patch.object(self.s, 'run', fake):
            self.s.set_('geoclue-agent.service', False)
        self.assertEqual(calls[-1], ['systemctl', '--user', 'mask', '--now', 'geoclue-agent.service'])


class LoginScreen(Home):
    def setUp(self):
        super().setUp()
        self.ls = load(SHELL / 'hearth-loginscreen', 'hearth_loginscreen')
        self.ls.BACKGROUND = os.path.join(self.home, 'bg')

    def test_set_sends_the_bytes_on_stdin(self):
        pic = os.path.join(self.home, 'p.jpg'); open(pic, 'wb').write(b'\xff\xd8\xff' + b'1' * 50)
        seen = {}
        class P:
            returncode = 0; stdout = b'{"ok": true}\n'
        def fake(cmd, input=None, capture_output=True, timeout=60):
            seen['cmd'], seen['input'] = cmd, input
            return P()
        with mock.patch.object(self.ls.subprocess, 'run', fake):
            self.ls.set_(pic)
        self.assertEqual(seen['cmd'], ['pkexec', self.ls.ADMIN, 'login-background'])
        self.assertEqual(seen['input'], b'\xff\xd8\xff' + b'1' * 50)
        txt = os.path.join(self.home, 'p.txt'); open(txt, 'w').write('hello')
        with self.assertRaises(self.ls.Fail):
            self.ls.set_(txt)

    def test_the_theme_falls_back_to_the_fireside(self):
        qml = (ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'sddm' / 'themes' / 'hearth' / 'Main.qml').read_text()
        self.assertIn('source: "file:///var/lib/hearth/login-screen/background"', qml)
        self.assertIn('visible: status === Image.Ready', qml)
        self.assertIn('source: "ember-glow.png"', qml)


class Pages(unittest.TestCase):
    NEW = ('SetVpn.qml', 'SetHotspot.qml', 'SetProxy.qml', 'SetNetDetails.qml', 'SetFirmware.qml', 'SetRegion.qml',
           'SetFonts.qml', 'SetServices.qml', 'SetLoginScreen.qml')

    def test_every_page_is_registered(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        for f in self.NEW:
            self.assertIn('file: "%s"' % f, reg)

    def test_pages_change_things_only_from_a_press(self):
        for f in self.NEW:
            t = (SHELL / f).read_text()
            self.assertEqual(re.findall(r'\bSettings\.\w+\s*=(?!=)', t), [], f)
            self.assertNotIn('Component.onCompleted: save', t, f)
            # Opening a page or its refresh timer only reads: the writing commands never run from there.
            writes = r'"(details-set|proxy-set|hotspot-on|hotspot-off|vpn-up|vpn-down|vpn-remove|vpn-import|set|locale|user-type|firmware-update|firmware-refresh)"'
            for line in t.splitlines():
                if 'Component.onCompleted' in line or ('onTriggered' in line and 'page.pick(' not in line):
                    self.assertNotRegex(line, writes, (f, line.strip()))

    def test_no_secret_is_put_on_a_command_line_in_qml(self):
        for f in ('SetVpn.qml', 'SetHotspot.qml'):
            t = (SHELL / f).read_text()
            self.assertNotRegex(t, r'\[[^\]]*(vpass|pass)\.text[^\]]*\]', f)


if __name__ == '__main__':
    unittest.main()
