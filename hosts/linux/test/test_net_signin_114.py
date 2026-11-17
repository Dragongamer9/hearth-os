"""Tests for 1.14's network and sign-in work (KDE's missing parts, docs/KDE-FEATURES-1.14.md): hearth-network's VPNs by
hand, 802.1X, mobile broadband, Wi-Fi QR codes and Duplicate; hearth-firewall; fingerprint sign-in (hearth-fingerprint,
hearth-fingerprint-pam, the PAM files); Vaults (hearth-vaults); SSH and GPG passphrases (hearth-askpass, pinentry-hearth);
the power menu's extra choices (hearth-session-actions); and the pages that use them. Fake programs throughout: no
nmcli, firewall, fprintd or gocryptfs runs, and every secret is checked to go on stdin only."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
PAMD = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'etc' / 'pam.d'
UUID = '0f5d6a3e-1b2c-4d5e-8f90-123456789abc'
NEW = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Home(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.env = mock.patch.dict(os.environ, {'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'),
                                                'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
                                                'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state'),
                                                'XDG_RUNTIME_DIR': os.path.join(self.home, 'run')})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()


class FakeNm:
    def __init__(self, answers):
        self.answers = answers
        self.calls = []

    def __call__(self, cmd, input=None, timeout=60):
        self.calls.append((list(cmd), input))
        for match, out in self.answers:
            if all(a in cmd for a in match):
                return (0, out, '') if not isinstance(out, tuple) else out
        return 0, '', ''

    def argv_text(self):
        return ' '.join(' '.join(c[0]) for c in self.calls)


def plugins_dir(home, names):
    d = os.path.join(home, 'vpn')
    os.makedirs(d, exist_ok=True)
    for n in names:
        with open(os.path.join(d, 'nm-%s-service.name' % n), 'w') as f:
            f.write('[VPN Connection]\nname=%s\nservice=org.freedesktop.NetworkManager.%s\n' % (n, n))
    return d


class Network(Home):
    def setUp(self):
        super().setUp()
        self.n = load(SHELL / 'hearth-network', 'hearth_network_114b')

    def test_vpn_kinds_follow_the_installed_plugins(self):
        self.n.VPN_PLUGINS = plugins_dir(self.home, ['l2tp', 'libreswan', 'openvpn'])
        types = {t['id']: t for t in self.n.vpn_types()}
        self.assertEqual(set(types), {'ipsec', 'l2tp', 'pptp', 'openconnect', 'sstp', 'fortisslvpn'})
        self.assertTrue(types['l2tp']['installed'] and types['ipsec']['installed'])
        self.assertEqual(types['ipsec']['plugin'], 'libreswan')
        self.assertFalse(types['pptp']['installed'])
        self.assertEqual(types['pptp']['package'], 'network-manager-pptp')
        self.assertTrue(self.n.plugins()['openvpn'])

    def test_a_missing_plugin_says_which_one_and_adds_nothing(self):
        self.n.VPN_PLUGINS = plugins_dir(self.home, [])
        fake = FakeNm([])
        with mock.patch.object(self.n, 'run', fake), self.assertRaises(self.n.Fail) as e:
            self.n.vpn_add({'type': 'sstp', 'name': 'Work', 'gateway': 'vpn.example.com', 'user': 'me', 'password': 'pw'})
        self.assertIn('network-manager-sstp', str(e.exception))
        self.assertEqual(fake.calls, [])

    def test_vpn_by_hand_puts_the_password_on_stdin_only(self):
        self.n.VPN_PLUGINS = plugins_dir(self.home, ['l2tp', 'pptp', 'sstp', 'fortisslvpn', 'strongswan', 'openconnect'])
        for typ in ('l2tp', 'pptp', 'sstp', 'fortisslvpn', 'ipsec', 'openconnect'):
            fake = FakeNm([(['add'], "Connection 'Work' (%s) successfully added.\n" % NEW)])
            with mock.patch.object(self.n, 'run', fake):
                r = self.n.vpn_add({'type': typ, 'name': 'Work', 'gateway': 'vpn.example.com', 'user': 'me',
                                    'password': 'p,a\\ss w0rd', 'psk': 'sh4red key'})
            self.assertEqual(r['uuid'], NEW, typ)
            self.assertNotIn('p,a', fake.argv_text(), typ)
            self.assertNotIn('sh4red', fake.argv_text(), typ)
            add = fake.calls[0][0]
            self.assertEqual(add[:6], ['nmcli', 'connection', 'add', 'type', 'vpn', 'vpn-type'], typ)
            self.assertIn('connection.autoconnect', add)
            self.assertEqual(add[add.index('connection.autoconnect') + 1], 'no')
            if typ != 'openconnect':
                edit = [c for c in fake.calls if 'edit' in c[0]][0]
                self.assertIn('set vpn.secrets ', edit[1])
                self.assertIn('p\\,a\\\\ss w0rd', edit[1], typ)      # ',' and '\\' escaped for nmcli's list syntax
                self.assertTrue(edit[1].endswith('save persistent\nquit\n'))
            else:
                self.assertFalse(any('edit' in c[0] for c in fake.calls))
        for bad in ({'type': 'l2tp', 'name': 'W', 'gateway': 'bad host; reboot'}, {'type': 'l2tp', 'name': 'W', 'gateway': 'h', 'password': 'two\nlines'},
                    {'type': 'openconnect', 'name': 'W', 'gateway': 'h', 'protocol': 'evil'}, {'type': 'nope', 'gateway': 'h'},
                    {'type': 'l2tp', 'name': 'W', 'gateway': 'h', 'password': ' spaced '}):
            with mock.patch.object(self.n, 'run', FakeNm([])), self.assertRaises(self.n.Fail, msg=bad):
                self.n.vpn_add(bad)

    def test_8021x_is_checked_and_its_password_goes_to_the_editor(self):
        ca = os.path.join(self.home, 'ca.pem'); open(ca, 'w').write('x')
        fake = FakeNm([(['connection.type'], '802-11-wireless\n'), (['show', 'uuid'], 'connection.id:Work\nconnection.type:802-11-wireless\n')])
        with mock.patch.object(self.n, 'run', fake):
            self.n.enterprise_set(UUID, {'eap': 'peap', 'identity': 'me@school.edu', 'phase2': 'mschapv2', 'caCert': ca,
                                         'domain': 'radius.school.edu', 'password': 'S3cret!'})
        mod = [c[0] for c in fake.calls if 'modify' in c[0]][0]
        for k, v in (('802-11-wireless-security.key-mgmt', 'wpa-eap'), ('802-1x.eap', 'peap'), ('802-1x.identity', 'me@school.edu'),
                     ('802-1x.ca-cert', ca), ('802-1x.phase2-auth', 'mschapv2'), ('802-1x.password-flags', '0')):
            self.assertEqual(mod[mod.index(k) + 1], v)
        self.assertNotIn('S3cret', fake.argv_text())
        self.assertIn('set 802-1x.password S3cret!\n', [c for c in fake.calls if 'edit' in c[0]][0][1])
        for bad in ({'eap': 'leap', 'identity': 'x'}, {'eap': 'peap', 'identity': ''}, {'eap': 'tls', 'identity': 'x'},
                    {'eap': 'peap', 'identity': 'x', 'caCert': '/nope.pem'}, {'eap': 'ttls', 'identity': 'x', 'phase2': 'evil'}):
            with self.assertRaises(self.n.Fail, msg=bad):
                self.n.enterprise_props(bad)
        # Ask each time: nothing kept.
        props, sec = self.n.enterprise_props({'eap': 'ttls', 'identity': 'x', 'password': 'pw', 'askEachTime': True})
        self.assertEqual(props[props.index('802-1x.password-flags') + 1], '2')
        self.assertEqual(sec, [])

    def test_8021x_off_only_for_wired(self):
        fake = FakeNm([(['connection.type'], '802-11-wireless\n')])
        with mock.patch.object(self.n, 'run', fake), self.assertRaises(self.n.Fail):
            self.n.enterprise_set(UUID, {'off': True})
        fake = FakeNm([(['connection.type'], '802-3-ethernet\n')])
        with mock.patch.object(self.n, 'run', fake):
            self.n.enterprise_set(UUID, {'off': True})
        self.assertIn(['nmcli', 'connection', 'modify', 'uuid', UUID, 'remove', '802-1x'], [c[0] for c in fake.calls])

    def test_mobile_broadband(self):
        fake = FakeNm([(['add'], "Connection 'Phone' (%s) successfully added.\n" % NEW)])
        with mock.patch.object(self.n, 'run', fake):
            self.n.mobile_add({'name': 'Phone', 'apn': 'fast.t-mobile.com', 'password': 'pw1', 'pin': '1234', 'roaming': False})
        add = fake.calls[0][0]
        self.assertEqual(add[:9], ['nmcli', 'connection', 'add', 'type', 'gsm', 'ifname', '*', 'con-name', 'Phone'])
        self.assertEqual(add[add.index('gsm.apn') + 1], 'fast.t-mobile.com')
        self.assertEqual(add[add.index('gsm.home-only') + 1], 'yes')
        self.assertNotIn('1234', fake.argv_text())
        self.assertNotIn('pw1', fake.argv_text())
        edit = [c for c in fake.calls if 'edit' in c[0]][0][1]
        self.assertIn('set gsm.pin 1234\n', edit)
        self.assertIn('set gsm.password pw1\n', edit)
        for bad in ({'apn': 'x y'}, {'apn': 'ok', 'pin': '12'}, {'apn': ''}):
            with mock.patch.object(self.n, 'run', FakeNm([])), self.assertRaises(self.n.Fail, msg=bad):
                self.n.mobile_add(bad)

    def test_details_read_mtu_8021x_and_mobile(self):
        show = ('connection.id:Office\nconnection.type:802-3-ethernet\n802-3-ethernet.mtu:1492\n802-1x.eap:peap\n'
                '802-1x.identity:me\n802-1x.password-flags:0 (none)\n802-1x.phase2-auth:mschapv2\nipv4.method:auto\n')
        with mock.patch.object(self.n, 'run', FakeNm([(['show', 'uuid'], show)])):
            d = self.n.details(UUID)
        self.assertEqual(d['mtu'], 1492)
        self.assertEqual((d['enterprise']['eap'], d['enterprise']['identity'], d['enterprise']['asks']), ('peap', 'me', False))
        self.assertNotIn('password', json.dumps(d['enterprise']).replace('"asks"', ''))
        show = 'connection.id:Phone\nconnection.type:gsm\ngsm.apn:internet\ngsm.home-only:yes\n'
        with mock.patch.object(self.n, 'run', FakeNm([(['show', 'uuid'], show)])):
            d = self.n.details(UUID)
        self.assertEqual(d['kind'], 'mobile')
        self.assertEqual(d['mobile'], {'apn': 'internet', 'user': '', 'roaming': False, 'number': ''})
        self.assertEqual(self.n.details_args({'mtu': 0}, '802-3-ethernet'), ['802-3-ethernet.mtu', 'auto'])
        self.assertEqual(self.n.details_args({'mtu': '1400'}, '802-11-wireless'), ['802-11-wireless.mtu', '1400'])
        for bad in (100, 'x', 99999):
            with self.assertRaises(self.n.Fail):
                self.n.details_args({'mtu': bad}, '802-3-ethernet')

    def test_wifi_qr_text_escapes_and_refuses_enterprise(self):
        self.assertEqual(self.n.wifi_qr_text('My;Net', 'WPA', 'p:a"s\\s'), 'WIFI:T:WPA;S:My\\;Net;P:p\\:a\\"s\\\\s;;')
        self.assertEqual(self.n.wifi_qr_text('Cafe', 'nopass', ''), 'WIFI:T:nopass;S:Cafe;;')
        self.assertEqual(self.n.wifi_qr_text('Hidden', 'SAE', 'pw', hidden=True), 'WIFI:T:SAE;S:Hidden;P:pw;H:true;;')
        with self.assertRaises(self.n.Fail):
            self.n.wifi_qr_text('eduroam', 'enterprise', '')

    def test_wifi_qr_goes_through_qrencodes_stdin(self):
        show = 'connection.id:Home\nconnection.type:802-11-wireless\n802-11-wireless.ssid:Home\n802-11-wireless-security.key-mgmt:wpa-psk\n'
        ascii_qr = '##  ##\n  ##  \n'
        fake = FakeNm([(['-t', 'connection', 'show', 'uuid'], show), (['-s', '-g'], 'hunter22\n'), (['qrencode'], ascii_qr)])
        with mock.patch.object(self.n, 'run', fake), mock.patch.object(self.n.shutil, 'which', lambda n: '/usr/bin/' + n):
            r = self.n.wifi_qr(UUID)
        self.assertEqual(r['rows'], ['101', '010'])
        self.assertEqual((r['ssid'], r['security'], r['size']), ('Home', 'WPA', 2))
        q = [c for c in fake.calls if c[0][0] == 'qrencode'][0]
        self.assertNotIn('hunter22', ' '.join(q[0]))
        self.assertEqual(q[1], 'WIFI:T:WPA;S:Home;P:hunter22;;')

    def test_duplicate_names_the_copy_and_never_autoconnects(self):
        conns = 'Home:%s:802-11-wireless:wlan0:yes:yes\nHome (copy):11111111-2222-3333-4444-555555555555:802-11-wireless::no:no\n' % UUID
        fake = FakeNm([(['NAME,UUID,TYPE,DEVICE,ACTIVE,AUTOCONNECT'], conns),
                       (['clone'], "Home (%s) cloned as Home (copy) 2 (%s).\n" % (UUID, NEW))])
        with mock.patch.object(self.n, 'run', fake):
            r = self.n.duplicate(UUID)
        self.assertEqual(r, {'uuid': NEW, 'name': 'Home (copy) 2'})
        self.assertIn(['nmcli', 'connection', 'modify', 'uuid', NEW, 'connection.autoconnect', 'no'], [c[0] for c in fake.calls])

    def test_status_reports_devices_and_kinds(self):
        self.n.VPN_PLUGINS = plugins_dir(self.home, ['pptp'])
        fake = FakeNm([(['NAME,UUID,TYPE,DEVICE,ACTIVE,AUTOCONNECT'], 'Phone:%s:gsm::no:yes\n' % UUID),
                       (['DEVICE,TYPE,STATE'], 'wlan0:wifi:connected\neth0:ethernet:unavailable\ncdc-wdm0:gsm:disconnected\n')])
        with mock.patch.object(self.n, 'run', fake):
            st = self.n.status()
        self.assertTrue(st['hasModem'] and st['hasEthernet'])
        self.assertEqual(st['connections'][0]['kind'], 'mobile')
        self.assertTrue(next(t for t in st['vpnTypes'] if t['id'] == 'pptp')['installed'])


class Firewall(unittest.TestCase):
    def setUp(self):
        self.f = load(NIMBUS / 'hearth-firewall', 'hearth_firewall')

    def test_ufw_rules_group_v6_twins_and_local_networks(self):
        text = ('Status: active\n\n     To                         Action      From\n     --                         ------      ----\n'
                '[ 1] 22/tcp                     ALLOW IN    Anywhere                   # SSH (kept by Hearth)\n'
                '[ 2] 1714:1764/tcp              ALLOW IN    192.168.0.0/16             # hearth-connect\n'
                '[ 3] 1714:1764/tcp              ALLOW IN    10.0.0.0/8                 # hearth-connect\n'
                '[ 4] Anywhere on tailscale0     ALLOW IN    Anywhere                   # Tailscale (kept by Hearth)\n'
                '[ 5] 22/tcp (v6)                ALLOW IN    Anywhere (v6)              # SSH (kept by Hearth)\n')
        rules = self.f.ufw_rules(text)
        ssh = next(r for r in rules if r['port'] == '22')
        self.assertEqual(ssh['nums'], [1, 5])
        self.assertEqual(ssh['from'], 'any')
        conn = next(r for r in rules if r['port'] == '1714-1764')
        self.assertEqual((conn['from'], conn['hearth'], conn['nums']), ('local', True, [2, 3]))
        self.assertEqual(conn['label'], 'Hearth Connect (while it runs)')

    def test_firewalld_rules(self):
        rich = ''.join('rule family="ipv4" source address="%s" port port="25565" protocol="tcp" accept\n' % n for n in self.f.LOCAL_V4)
        rules = self.f.fwd_rules('8080/tcp', 'ssh dhcpv6-client', rich)
        ids = [r['id'] for r in rules]
        self.assertEqual(ids, ['service:ssh', 'service:dhcpv6-client', 'port:8080/tcp', 'rich:25565/tcp'])
        self.assertEqual(rules[-1]['from'], 'local')

    def test_rules_are_checked(self):
        self.assertEqual(self.f.check_rule({'port': '6000:6010', 'proto': 'both', 'from': 'any'})[:3], ('6000-6010', ['tcp', 'udp'], 'any'))
        for bad in ({'port': '0'}, {'port': '70000'}, {'port': '10-5'}, {'port': '80; reboot'}, {'port': '80', 'proto': 'icmp'},
                    {'port': '80', 'from': '0.0.0.0/0'}, {'port': '80', 'label': 'x"; rm'}):
            with self.assertRaises(self.f.Fail, msg=bad):
                self.f.check_rule(bad)

    def test_turning_on_keeps_ssh_and_tailscale_and_ssh_cant_be_closed(self):
        calls = []
        def run(cmd, input=None, timeout=60):
            calls.append(cmd)
            if cmd[:2] == ['systemctl', 'is-active']:
                return (0, '') if 'ssh.service' in cmd else (3, '')
            if cmd[:2] == ['ufw', 'status']:
                return 0, '[ 1] 22/tcp                     ALLOW IN    Anywhere\n'
            return 0, ''
        with mock.patch.object(self.f, 'run', run), mock.patch.object(self.f, 'which', lambda n: '/usr/sbin/ufw' if n == 'ufw' else None), \
                mock.patch.object(self.f.os, 'geteuid', lambda: 0), mock.patch.object(self.f, 'ufw_enabled', lambda: True):
            self.f.turn_on()
            flat = [' '.join(c) for c in calls]
            i_ssh = next(i for i, c in enumerate(flat) if c.startswith('ufw allow proto tcp from any to any port 22'))
            i_ts = next(i for i, c in enumerate(flat) if c.startswith('ufw allow in on tailscale0'))
            i_on = flat.index('ufw --force enable')
            self.assertLess(i_ssh, i_on)
            self.assertLess(i_ts, i_on)
            with self.assertRaises(self.f.Fail):
                self.f.remove({'id': 'ufw:22/tcp@any'})

    def test_changes_need_root(self):
        with mock.patch.object(self.f.os, 'geteuid', lambda: 1000):
            for fn in (self.f.turn_on, self.f.turn_off):
                with self.assertRaises(self.f.Fail):
                    fn()

    def test_polkit_asks_for_opening_up(self):
        rules = (ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'rules.d' / '49-hearth-admin.rules').read_text()
        self.assertIn('firewall-(off|allow|incoming)', rules)
        admin = (NIMBUS / 'hearth-admin').read_text()
        self.assertIn("'firewall-status', 'firewall-on', 'firewall-off', 'firewall-allow', 'firewall-remove', 'firewall-incoming'", admin)


class Fingerprint(unittest.TestCase):
    """The PAM side: never the only way in (fingerprint-pam.yml runs the real stacks with a test account)."""
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.pamd = os.path.join(self.tmp.name, 'pam.d')
        self.lists = os.path.join(self.tmp.name, 'fp')
        os.makedirs(self.pamd)
        for f in ('hearth-fingerprint', 'hearth-fingerprint-polkit'):
            open(os.path.join(self.pamd, f), 'w').write((PAMD / f).read_text())
        self.polkit = ('#%PAM-1.0\n\n@include common-auth\n@include common-account\n@include common-password\n'
                       'session       required   pam_env.so readenv=1 user_readenv=0\n@include common-session-noninteractive\n')
        open(os.path.join(self.pamd, 'polkit-1'), 'w').write(self.polkit)
        self.env = mock.patch.dict(os.environ, {'HEARTH_PAMD': self.pamd, 'HEARTH_FP_LISTS': self.lists,
                                                'HEARTH_VENDOR_PAMD': os.path.join(self.tmp.name, 'vendor')})
        self.env.start()
        self.p = load(NIMBUS / 'hearth-fingerprint-pam', 'hearth_fingerprint_pam')

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def test_the_pam_files_can_never_block_the_password(self):
        inc = (PAMD / 'hearth-fingerprint-polkit').read_text()
        lines = [l for l in inc.splitlines() if l.strip() and not l.startswith('#')]
        self.assertEqual(len(lines), 2)
        self.assertRegex(lines[0], r'^auth\s+\[success=ignore default=1\]\s+pam_listfile\.so .*file=/etc/nimbus/fingerprint/polkit onerr=fail$')
        self.assertRegex(lines[1], r'^auth\s+\[success=done default=ignore\]\s+pam_fprintd\.so ')
        lock = (PAMD / 'hearth-fingerprint').read_text()
        self.assertIn('file=/etc/nimbus/fingerprint/lock', lock)
        self.assertNotIn('pam_unix', lock)       # the lock screen's fingerprint service never takes a password

    def test_install_check_uninstall_gives_polkit_back_byte_for_byte(self):
        self.p.install()
        t = open(os.path.join(self.pamd, 'polkit-1')).read()
        self.assertLess(t.index('hearth-fingerprint-polkit'), t.index('@include common-auth'))
        self.assertEqual(self.p.check()['ok'], True)
        self.p.install()      # twice: still one line
        self.assertEqual(open(os.path.join(self.pamd, 'polkit-1')).read().count('hearth-fingerprint-polkit'), 1)
        self.assertEqual(open(os.path.join(self.pamd, 'kde-fingerprint')).read(), (PAMD / 'hearth-fingerprint').read_text())
        self.p.uninstall()
        self.assertEqual(open(os.path.join(self.pamd, 'polkit-1')).read(), self.polkit)
        self.assertFalse(os.path.exists(os.path.join(self.pamd, 'kde-fingerprint')))

    def test_check_finds_a_broken_setup(self):
        open(os.path.join(self.pamd, 'polkit-1'), 'w').write(self.p.INCLUDE + '\n')
        self.assertFalse(self.p.check()['ok'])

    def test_places_only_change_the_list(self):
        import pwd
        me = pwd.getpwuid(os.getuid()).pw_name
        before = {f: open(os.path.join(self.pamd, f)).read() for f in os.listdir(self.pamd)}
        self.assertEqual(self.p.places_set(me, {'polkit': True}), {'lock': False, 'polkit': True})
        self.assertEqual(self.p.places_set(me, {'lock': True, 'polkit': False}), {'lock': True, 'polkit': False})
        self.assertEqual({f: open(os.path.join(self.pamd, f)).read() for f in os.listdir(self.pamd)}, before)
        self.assertEqual(self.p.names('lock'), [me])
        for bad in ('root; reboot', '../etc', ''):
            with self.assertRaises(self.p.Fail):
                self.p.places_set(bad, {'lock': True})

    def test_user_helper_reads_fprintd(self):
        h = load(SHELL / 'hearth-fingerprint', 'hearth_fingerprint')
        r = h.parse_list('Using device /net/reactivated/Fprint/Device/0\nFingerprints for user sam on Goodix MOC Fingerprint Sensor (press):\n - #0: right-index-finger\n - #1: left-thumb\n')
        self.assertEqual(r, {'reader': True, 'device': 'Goodix MOC Fingerprint Sensor', 'fingers': ['right-index-finger', 'left-thumb']})
        self.assertFalse(h.parse_list('No devices available\n')['reader'])
        with self.assertRaises(h.Fail):
            h.check_finger('$(reboot)')

    def test_build_takes_fprintds_own_profile_out(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('pam-auth-update --package --remove fprintd', b)
        self.assertIn('grep -qs pam_fprintd /etc/pam.d/common-auth', b)
        self.assertIn('"$FP" uninstall', b)

    def test_lock_screen_keeps_the_password(self):
        lock = (SHELL / 'Lock.qml').read_text()
        self.assertIn('config: "hearth-fingerprint"', lock)
        self.assertIn('PamContext {\n        id: pam\n', lock)         # the password's own conversation is still there
        self.assertNotRegex(lock, r'id: fpPam[\s\S]{0,400}respond\(root\.typed\)')   # the finger's never gets the password


class Vaults(Home):
    def setUp(self):
        super().setUp()
        self.v = load(SHELL / 'hearth-vaults', 'hearth_vaults')

    def fake(self, mounted=()):
        calls = []
        def run(cmd, input=None, timeout=120, env=None):
            calls.append((cmd, input, env))
            if cmd[0] == 'gocryptfs' and '-init' not in cmd and input != 'right horse\n':
                return 12, 'Password incorrect.'
            return 0, ''
        return calls, run

    def test_create_unlock_lock_and_the_password_only_on_stdin(self):
        calls, run = self.fake()
        with mock.patch.object(self.v, 'run', run), mock.patch.object(self.v, 'which', lambda n: '/usr/bin/' + n):
            r = self.v.create({'name': 'Private', 'backend': 'gocryptfs', 'password': 'right horse', 'idle': 5})
            vid = r['vaults'][0]['id']
            self.assertEqual(vid, 'private')
            for cmd, inp, env in calls:
                self.assertNotIn('right horse', ' '.join(cmd))
            self.assertEqual(calls[0][0][:3], ['gocryptfs', '-init', '-q'])
            self.assertIn('-idle', calls[1][0])
            self.assertEqual(calls[1][0][calls[1][0].index('-idle') + 1], '5m')
            with self.assertRaises(self.v.Fail) as e:
                self.v.do_unlock(dict(self.v.find(vid), open=False), 'wrong')
            self.assertIn("isn't right", str(e.exception))
        saved = json.load(open(os.path.join(self.home, '.config', 'nimbus', 'vaults.json')))
        self.assertEqual(saved['vaults']['private'], {'name': 'Private', 'backend': 'gocryptfs', 'idle': 5, 'lockOnLock': True})
        self.assertNotIn('right horse', json.dumps(saved))

    def test_cryfs_is_non_interactive_with_its_idle_lock(self):
        cmd, env = self.v.unlock_cmd({'backend': 'cryfs', 'data': '/d', 'mount': '/m'}, 15)
        self.assertEqual(cmd, ['cryfs', '--unmount-idle', '15', '/d', '/m'])
        self.assertEqual(env['CRYFS_FRONTEND'], 'noninteractive')

    def test_names_and_passwords_are_checked(self):
        with mock.patch.object(self.v, 'which', lambda n: '/usr/bin/' + n):
            for bad in ({'name': '../x', 'password': 'longenough'}, {'name': 'Ok', 'password': 'short'},
                        {'name': 'Ok', 'password': 'two\nlines'}, {'name': 'Ok', 'password': 'longenough', 'backend': 'veracrypt'},
                        {'name': 'Ok', 'password': 'longenough', 'idle': 99999}):
                with self.assertRaises(self.v.Fail, msg=bad):
                    self.v.create(bad)

    def test_forget_keeps_the_encrypted_files(self):
        calls, run = self.fake()
        with mock.patch.object(self.v, 'run', run), mock.patch.object(self.v, 'which', lambda n: '/usr/bin/' + n):
            self.v.create({'name': 'Tax', 'password': 'right horse'})
            data = os.path.join(self.home, '.local', 'share', 'hearth-vaults', 'tax')
            r = self.v.forget('tax')
        self.assertTrue(os.path.isdir(data))
        self.assertEqual(r['kept'], data)
        self.assertEqual(r['vaults'], [])

    def test_mount_table_reading(self):
        self.assertTrue(self.v.mounted('/home/sam/Vaults/My Stuff', 'gocryptfs /home/sam/Vaults/My\\040Stuff fuse.gocryptfs rw 0 0\n'))
        self.assertFalse(self.v.mounted('/home/sam/Vaults/Other', 'gocryptfs /home/sam/Vaults/My\\040Stuff fuse.gocryptfs rw 0 0\n'))


class AskPass(Home):
    def test_pinentry_speaks_assuan(self):
        pe = load(NIMBUS / 'pinentry-hearth', 'pinentry_hearth')
        asked = []
        def ask(kind, title, desc, prompt):
            asked.append((kind, title, desc, prompt))
            return True, 'p%ss 100%'
        out = io.StringIO()
        s = pe.Session(ask, out)
        for line in ('OPTION ttyname=/dev/pts/1', 'SETTITLE GnuPG', 'SETDESC Please enter%0Athe passphrase', 'SETPROMPT Passphrase:',
                     'GETPIN', 'BYE'):
            if not s.handle(line):
                break
        lines = out.getvalue().splitlines()
        self.assertEqual(asked, [('secret', 'GnuPG', 'Please enter\nthe passphrase', 'Passphrase:')])
        self.assertIn('D p%25ss 100%25', lines)
        self.assertEqual(lines[-1], 'OK closing connection')
        out2 = io.StringIO()
        s2 = pe.Session(lambda *a: (False, ''), out2)
        s2.handle('GETPIN')
        self.assertTrue(out2.getvalue().startswith('ERR 83886179'))

    def test_without_mantel_the_classic_programs_ask(self):
        pe = load(NIMBUS / 'pinentry-hearth', 'pinentry_hearth2')
        self.assertIn('/usr/bin/pinentry-curses', pe.CLASSIC)
        ha = load(NIMBUS / 'hearth_askpass.py', 'hearth_askpass')
        with mock.patch.dict(os.environ, {'WAYLAND_DISPLAY': '', 'DISPLAY': ''}):
            self.assertFalse(ha.mantel_here())

    def test_the_answer_comes_back_through_a_private_fifo(self):
        ha = load(NIMBUS / 'hearth_askpass.py', 'hearth_askpass2')
        seen = {}
        def ipc(args, timeout=5):
            if args[0] == 'ask':
                fifo = args[1]
                seen['fifo'] = fifo
                seen['mode'] = oct(os.stat(os.path.dirname(fifo)).st_mode & 0o777)
                self.assertNotIn('s3cret', ' '.join(args))
                pid = os.fork()
                if pid == 0:
                    with open(fifo, 'w') as f:
                        f.write('ok\ns3cret phrase')
                    os._exit(0)
                seen['pid'] = pid
            return 0, ''
        with mock.patch.object(ha, 'ipc', ipc):
            r = ha.ask('secret', 'SSH', 'Enter passphrase', 'Passphrase')
        os.waitpid(seen['pid'], 0)
        self.assertEqual(r, (True, 's3cret phrase'))
        self.assertEqual(seen['mode'], '0o700')
        self.assertFalse(os.path.exists(seen['fifo']))

    def test_askpass_qml_writes_only_into_its_fifo(self):
        t = (SHELL / 'AskPass.qml').read_text()
        self.assertIn('[ -p \\"$0\\" ] ||', t)
        self.assertIn('/^[0-9a-f]{24}$/', t)
        self.assertNotRegex(t, r'console\.(log|warn|info)\([^)]*field\.text')
        self.assertIn('source: "AskPass.qml"', (SHELL / 'shell.qml').read_text())


class SessionActions(unittest.TestCase):
    def test_switch_user_needs_a_display_manager_seat(self):
        sa = load(SHELL / 'hearth-session-actions', 'hearth_session_actions')
        with mock.patch.dict(os.environ, {'XDG_SEAT_PATH': '/org/freedesktop/DisplayManager/Seat0'}), \
                mock.patch.object(sa, 'run', lambda cmd, timeout=10: (0, 'interface org.freedesktop.DisplayManager.Seat {\n  methods:\n    SwitchToGreeter();\n  properties:\n    readonly b CanSwitch = true;\n}')):
            self.assertTrue(sa.can_switch())
        with mock.patch.dict(os.environ, {'XDG_SEAT_PATH': ''}):
            self.assertFalse(sa.can_switch())
        with mock.patch.dict(os.environ, {'XDG_SEAT_PATH': '/evil; reboot'}):
            self.assertEqual(sa.seat_path(), '')
        with mock.patch.object(sa, 'run', lambda cmd, timeout=10: (0, '("na",)')):
            self.assertFalse(sa.logind_can('CanHibernate'))
        with mock.patch.object(sa, 'run', lambda cmd, timeout=10: (0, 's "yes"')):
            self.assertTrue(sa.logind_can('CanRebootToFirmwareSetup'))


def handler(text, name):
    """The body of a QML handler (`name: { ... }` with its braces matched, or the rest of the line)."""
    i = text.find(name + ':')
    if i < 0:
        return ''
    j = i + len(name) + 1
    while text[j] == ' ':
        j += 1
    if text[j] != '{':
        return text[i:text.find('\n', i)]
    depth = 0
    for k in range(j, len(text)):
        depth += {'{': 1, '}': -1}.get(text[k], 0)
        if depth == 0:
            return text[i:k + 1]
    return text[i:]


class Pages(unittest.TestCase):
    NEW = ('SetFirewall.qml', 'SetVaults.qml', 'SetNetDetails.qml', 'SetVpn.qml')

    def test_registered_and_in_the_search(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        for f in ('SetFirewall.qml', 'SetVaults.qml'):
            self.assertIn('file: "%s"' % f, reg)
        idx = (SHELL / 'settings-index.js').read_text()
        self.assertIn('"p": "firewall"', idx)
        self.assertIn('"p": "vaults"', idx)

    def test_opening_a_page_writes_nothing(self):
        writes = r'"(vpn-add|enterprise-set|wifi-enterprise-add|wired-add|mobile-add|mobile-set|duplicate|firewall-on|firewall-off|firewall-allow|firewall-remove|firewall-incoming|create|unlock|lock|set|forget|fingerprint-places|delete)"'
        for f in self.NEW + ('SetSignin.qml', 'SetUsers.qml'):
            t = (SHELL / f).read_text()
            self.assertNotRegex(handler(t, 'Component.onCompleted'), writes, f)
            for line in t.splitlines():
                if 'onTriggered' in line:
                    self.assertNotRegex(line, writes, (f, line.strip()))
            if f in self.NEW:
                self.assertEqual(re.findall(r'\bSettings\.\w+\s*=(?!=)', t), [], f)

    def test_no_secret_on_a_command_line(self):
        for f in ('SetVpn.qml', 'SetNetDetails.qml', 'SetVaults.qml', 'SetFirewall.qml'):
            t = (SHELL / f).read_text()
            self.assertNotRegex(t, r'\[[^\]\n]*\b(vaddPass|vpsk|eapPw|keyPw|mobPw|mobPin|addMobPw|addPin|pw|npw)\.text[^\]\n]*\]', f)

    def test_power_menu_choices_are_offered_only_where_they_work(self):
        t = (SHELL / 'PowerMenu.qml').read_text()
        self.assertIn('can.switchUser && can.otherAccounts > 0', t)
        self.assertIn('can.hibernate', t)
        self.assertIn('can.firmware', t)
        self.assertIn('"Log out"', t)

    def test_net_speed_is_off_until_turned_on(self):
        s = (SHELL / 'Settings.qml').read_text()
        self.assertIn('property bool netSpeed: false', s)
        self.assertIn('"netSpeed"', s)
        self.assertIn('running: root.on', (SHELL / 'NetSpeed.qml').read_text())


if __name__ == '__main__':
    unittest.main()
