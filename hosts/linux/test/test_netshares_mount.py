"""Why a listed shared folder wouldn't open, and that each reason is now fixed or said plainly (hearth_netshares.py,
hearth-files, hearth-device-status, the home server's helpers, build.sh):
- the image had no gvfs SMB backend, gvfs-fuse, gio or secret-tool, so nothing could mount;
- a share that's "Only through Tailscale" was opened by its home name (Samba's hosts allow / the firewall refused it);
- a Tailscale name that doesn't resolve here (MagicDNS off) is opened by its Tailscale IP;
- gio's errors are told apart (wrong password, share off, asleep, Samba refusing, Samba not running, Tailscale, no
  gvfs) with what to do, and a wrong password stops at once instead of being asked again and again;
- mounted without a folder to show (no gvfs-fuse) is an error, not a silent "Couldn't open it";
- a whole computer's shares can be picked; the sharing device says its account name, whether a sharing password is
  set and whether Samba runs; smbpasswd's own reason is shown; ufw / firewalld get port 445 for this network."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import stat
import tempfile
import time
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIBEXEC = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
sys.path.insert(0, str(SHELL))
import hearth_netshares as ns  # noqa: E402


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


# gvfs asks again and again when the password is wrong (it never prints "Permission denied" by itself).
REASKING_GIO = r'''#!/usr/bin/env python3
import os, sys
open(os.environ['FAKE_ARGS'], 'a').write(' '.join(sys.argv) + '\n')
while True:
    sys.stdout.write('Authentication Required\nEnter user and password for share \u201cMedia\u201d on \u201chost\u201d:\n')
    sys.stdout.write('User [me]: '); sys.stdout.flush()
    if not sys.stdin.readline(): sys.exit(1)
    sys.stdout.write('Domain [WORKGROUP]: '); sys.stdout.flush()
    if not sys.stdin.readline(): sys.exit(1)
    sys.stdout.write('Password: '); sys.stdout.flush()
    if not sys.stdin.readline(): sys.exit(1)
'''

ENV_GIO = r'''#!/usr/bin/env python3
import os, sys
open(os.environ['FAKE_ARGS'], 'a').write(os.environ.get('DBUS_SESSION_BUS_ADDRESS', '-') + ' ' + os.environ.get('LC_ALL', '-') + '\n')
sys.stdout.write('gio: smb://host/Media/: Failed to mount Windows share: Connection refused\n')
sys.exit(2)
'''


class Reach(unittest.TestCase):
    """A Tailscale-only share is opened through Tailscale, even at home."""
    dev = {'home': 'pc.local', 'away': 'pc.tail.ts.net', 'reach': 'tailscale'}

    def test_tailscale_only_never_uses_the_home_name(self):
        self.assertEqual(ns.choose_address(self.dev, lambda h: True, True), ('pc.tail.ts.net', 'away'))
        self.assertEqual(ns.choose_address(self.dev, lambda h: True, False), (None, 'tailscale-off'))
        self.assertEqual(ns.choose_address(dict(self.dev, away=''), lambda h: True, True), (None, 'tailscale-server-off'))
        # Shares for home and Tailscale stay as they were: the home name at home.
        self.assertEqual(ns.choose_address(dict(self.dev, reach='home'), lambda h: True, True), ('pc.local', 'home'))

    def test_saved_entries_and_old_links_get_the_reach_from_what_was_found(self):
        st = {'devices': [{'id': 'd1', 'name': 'nvidia-pc', 'home': 'nvidia-pc.local', 'away': 'nvidia-pc.tail.ts.net', 'user': 'dragon',
                           'shares': [{'name': 'Games', 'reach': 'tailscale'}, {'name': 'Media', 'reach': 'home'}]}]}
        full, dev = ns.with_device({'device': 'd1', 'share': 'Games', 'home': 'nvidia-pc.local'}, st)
        self.assertEqual((full['reach'], full['away'], full['user']), ('tailscale', 'nvidia-pc.tail.ts.net', 'dragon'))
        self.assertEqual(dev['name'], 'nvidia-pc')
        self.assertEqual(ns.with_device({'device': 'd1', 'share': 'Media'}, st)[0]['reach'], 'home')
        self.assertEqual(ns.with_device({'share': 'x', 'home': 'nas.local'}, st), ({'share': 'x', 'home': 'nas.local'}, {}))

    def test_opening_one_uses_tailscale_and_the_sidebar_says_so(self):
        st = {'tailscale': {'running': True}, 'devices': [
            {'id': 'd1', 'name': 'nvidia-pc', 'online': True, 'home': 'nvidia-pc.local', 'away': 'nvidia-pc.tail.ts.net',
             'shares': [{'name': 'Games', 'reach': 'tailscale'}]}]}
        net = [p for p in ns.sidebar(st) if p.get('net')][0]['net']
        self.assertEqual(net['reach'], 'tailscale')
        uris = []
        r = ns.net_open(net, at_home=lambda h: True, mount=lambda u, us, pw: uris.append(u) or (True, ''),
                        path_of=lambda u: '/run/user/1000/gvfs/x' if uris else None, state=st, lookup=lambda *a: None)
        self.assertTrue(r['ok'])
        self.assertEqual(uris, ['smb://nvidia-pc.tail.ts.net/Games'])

    def test_autoconnect_goes_through_tailscale_for_a_tailscale_only_share(self):
        st = {'tailscale': {'running': True}, 'users': {'nvidia-pc.tail.ts.net': 'dragon'}, 'devices': [
            {'id': 'd1', 'name': 'nvidia-pc', 'home': 'nvidia-pc.local', 'away': 'nvidia-pc.tail.ts.net',
             'shares': [{'name': 'Games', 'reach': 'tailscale'}]}]}
        probed, uris = [], []
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        r = ns.autoconnect({'k': {'device': 'd1', 'share': 'Games', 'home': 'nvidia-pc.local', 'away': 'nvidia-pc.tail.ts.net'}},
                           at_home=lambda h: True, probe=lambda h, p, t: probed.append(h) or True,
                           mount=lambda u, us, pw: uris.append(u) or (True, ''), lookup=lambda *a: 'pw',
                           path_of=lambda u: '/x' if uris else None, state=st, runtime=os.path.join(tmp, 'rt.json'), budget=5)
        self.assertEqual(r, {'k': 'opened'})
        self.assertEqual(probed, ['nvidia-pc.tail.ts.net'])
        self.assertEqual(uris, ['smb://nvidia-pc.tail.ts.net/Games'])


class TailscaleAddress(unittest.TestCase):
    def test_peers_keep_their_tailscale_ip(self):
        ts = ns.parse_tailscale({'BackendState': 'Running', 'Peer': {'a': {'HostName': 'pc', 'DNSName': 'PC.tail.ts.net.', 'Online': True,
                                                                           'TailscaleIPs': ['fd7a:115c:a1e0::1', '100.101.102.103']}}})
        self.assertEqual(ns.tailscale_ips(ts), {'pc.tail.ts.net': '100.101.102.103'})

    def test_a_name_magicdns_cant_find_here_is_opened_by_its_ip(self):
        ips = {'pc.tail.ts.net': '100.101.102.103'}
        self.assertEqual(ns.address_for('pc.tail.ts.net', 'away', lambda h: False, ips), '100.101.102.103')
        self.assertEqual(ns.address_for('pc.tail.ts.net', 'away', lambda h: True, ips), 'pc.tail.ts.net')
        self.assertEqual(ns.address_for('pc.local', 'home', lambda h: False, ips), 'pc.local')
        st = {'tailscale': {'running': True, 'ips': ips}, 'users': {'pc.tail.ts.net': 'chris'}}
        uris, looked = [], []
        r = ns.net_open({'name': 'pc', 'share': 'Media', 'home': '', 'away': 'pc.tail.ts.net'}, at_home=lambda h: False, state=st,
                        mount=lambda u, us, pw: uris.append((u, us, pw)) or (True, ''), lookup=lambda *a: looked.append(a) or 'pw',
                        path_of=lambda u: '/x' if uris else None)
        self.assertTrue(r['ok'])
        self.assertEqual(uris, [('smb://100.101.102.103/Media', 'chris', 'pw')])
        self.assertEqual(looked, [('pc.tail.ts.net', 'Media', 'chris')])       # the keyring keeps the name, not the IP


class Errors(unittest.TestCase):
    def need(self, text, where='home', dev=None):
        return ns.classify(text, where, dev, 'nvidia-pc')

    def test_each_reason_said_plainly(self):
        g = 'gio: smb://pc.local/Media/: Failed to mount Windows share: '
        self.assertEqual(self.need(g + 'Connection refused')['need'], 'sharing-off')
        self.assertEqual(self.need(g + 'Connection reset by peer')['need'], 'refused')
        self.assertIn('Tailscale', self.need(g + 'Connection reset by peer')['error'])
        self.assertEqual(self.need(g + 'No such file or directory')['need'], 'off')
        self.assertEqual(self.need(g + 'Permission denied')['need'], 'password')
        self.assertEqual(self.need(g + 'Connection timed out')['need'], 'asleep')
        self.assertEqual(self.need(g + 'Invalid argument')['need'], 'protocol')
        self.assertEqual(self.need('gio: smb://pc.local/Media/: The specified location is not supported')['need'], 'nogvfs')
        self.assertEqual(self.need('gio: smb://pc.local/Media/: Location is not mountable')['need'], 'nogvfs')
        self.assertEqual(self.need('gio is missing (gvfs)')['need'], 'nogvfs')
        self.assertEqual(self.need('gio: Cannot autolaunch D-Bus without X11 $DISPLAY')['need'], 'session')
        self.assertIn('Tailscale', self.need(g + 'Name or service not known', 'away')['error'])
        self.assertEqual(self.need('', 'tailscale-server-off')['need'], 'tailscale')
        self.assertIn('nvidia-pc', self.need('', 'tailscale-server-off')['error'])

    def test_the_sharing_devices_own_word_makes_it_plainer(self):
        g = 'gio: smb://pc.local/Media/: Failed to mount Windows share: '
        self.assertIn('no sharing password yet', self.need(g + 'Permission denied', dev={'pwSet': False})['error'])
        self.assertIn('(dragon)', self.need(g + 'Permission denied', dev={'pwSet': True, 'user': 'dragon'})['error'])
        r = self.need(g + 'Connection timed out', dev={'online': True, 'smbOn': False})
        self.assertEqual(r['need'], 'sharing-off')
        self.assertIn('online', self.need(g + 'Connection timed out', dev={'online': True})['error'])

    def test_gios_questions_never_count_as_the_error(self):
        text = ('Authentication Required\nEnter user and password for share “Media” on “pc”:\nUser [me]: \nDomain [WORKGROUP]: \n'
                'Password: \ngio: smb://pc/Media/: Failed to mount Windows share: Connection reset by peer\n')
        self.assertEqual(ns.gio_error(text), 'gio: smb://pc/Media/: Failed to mount Windows share: Connection reset by peer')
        self.assertEqual(ns.classify(text, 'home')['need'], 'refused')       # not "password" from the questions

    def test_what_each_prompt_is(self):
        self.assertEqual(ns._prompt('User [me]: '), 'user')
        self.assertEqual(ns._prompt('Domain [WORKGROUP]: '), 'domain')
        self.assertEqual(ns._prompt('Password: '), 'password')
        self.assertIsNone(ns._prompt('Enter user and password for share “Media” on “pc”:'))
        self.assertIsNone(ns._prompt('Password required for share Media on pc'))


class Mounting(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.bin = os.path.join(self.tmp, 'bin')
        os.makedirs(self.bin)
        self.args = os.path.join(self.tmp, 'args.log')

    def gio(self, text):
        p = os.path.join(self.bin, 'gio')
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, os.stat(p).st_mode | stat.S_IEXEC)

    def test_a_wrong_password_stops_at_once(self):
        self.gio(REASKING_GIO)
        with mock.patch.dict(os.environ, {'PATH': self.bin + os.pathsep + os.environ['PATH'], 'FAKE_ARGS': self.args}):
            t0 = time.time()
            ok, text = ns.gio_mount('smb://host/Media', 'chris', 'wrong pw', timeout=20)
        self.assertFalse(ok)
        self.assertLess(time.time() - t0, 10)
        self.assertEqual(ns.classify(text, 'home')['need'], 'password')
        self.assertNotIn('wrong pw', text)

    def test_no_gio_says_so(self):
        with mock.patch.object(ns.shutil, 'which', return_value=None):
            ok, text = ns.gio_mount('smb://host/Media')
        self.assertEqual((ok, ns.classify(text, 'home')['need']), (False, 'nogvfs'))

    def test_gio_gets_the_session_bus_when_started_without_it(self):
        self.gio(ENV_GIO)
        run = os.path.join(self.tmp, 'run')
        os.makedirs(run)
        open(os.path.join(run, 'bus'), 'w').close()
        env = {'PATH': self.bin + os.pathsep + os.environ['PATH'], 'FAKE_ARGS': self.args, 'XDG_RUNTIME_DIR': run}
        with mock.patch.dict(os.environ, env):
            os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)
            ok, text = ns.gio_mount('smb://host/Media', 'chris', 'pw', timeout=10)
        self.assertFalse(ok)
        with open(self.args) as f:
            self.assertEqual(f.read().split(), ['unix:path=' + os.path.join(run, 'bus'), 'C'])
        self.assertEqual(ns.classify(text, 'home')['need'], 'sharing-off')

    def test_mounted_but_no_folder_to_show_is_said(self):
        mounted = []
        r = ns.net_open({'name': 'pc', 'share': 'Media', 'home': 'pc.local'}, user='chris', password='pw', at_home=lambda h: True,
                        mount=lambda u, us, pw: mounted.append(u) or (True, ''), path_of=lambda u: None, state={})
        self.assertEqual((r['ok'], r['need']), (False, 'nogvfs'))
        self.assertIn('gvfs-fuse', r['error'])

    def test_a_whole_computer_asks_which_share(self):
        mounted = []
        common = dict(user='chris', password='pw', at_home=lambda h: True, state={}, store=lambda *a: None,
                      mount=lambda u, us, pw: mounted.append(u) or (True, ''), path_of=lambda u: '/x' if u in mounted[1:] else None)
        r = ns.net_open({'name': 'nas', 'share': '', 'home': 'nas.local'}, lister=lambda u: ['Media', 'Photos'], **common)
        self.assertEqual((r['ok'], r['need'], r['shares']), (False, 'pick', ['Media', 'Photos']))
        mounted.clear()
        r = ns.net_open({'name': 'nas', 'share': '', 'home': 'nas.local'}, lister=lambda u: ['Media'], **common)
        self.assertTrue(r['ok'])
        self.assertEqual(mounted, ['smb://nas.local/', 'smb://nas.local/Media'])

    def test_share_names_with_spaces(self):
        self.assertEqual(ns.smb_uri('pc.local', 'My Games'), 'smb://pc.local/My%20Games')
        self.assertEqual(ns.split_uri('smb://pc.local/My%20Games'), ('pc.local', 'My Games'))

    def test_failures_carry_gios_own_words_too(self):
        r = ns.net_open({'name': 'pc', 'share': 'Media', 'home': 'pc.local'}, user='chris', password='pw', at_home=lambda h: True, state={},
                        mount=lambda u, us, pw: (False, 'gio: smb://pc.local/Media/: Failed to mount Windows share: Connection refused'),
                        path_of=lambda u: None)
        self.assertEqual(r['need'], 'sharing-off')
        self.assertIn('Connection refused', r['detail'])


class Keyring(unittest.TestCase):
    def test_lookup_falls_back_to_any_share_on_that_computer(self):
        calls = []

        class R:
            def __init__(self, code, out):
                self.returncode, self.stdout = code, out

        def run(cmd, **kw):
            calls.append(cmd)
            return R(0, 'pw\n') if 'object' not in cmd else R(1, '')
        with mock.patch.object(ns.subprocess, 'run', side_effect=run), mock.patch.dict(os.environ, {'HEARTH_SECRET_TOOL': '/bin/true'}):
            self.assertEqual(ns.keyring_lookup('PC.local', 'Media', 'chris'), 'pw')
        self.assertEqual(calls[0][-4:], ['object', 'Media', 'user', 'chris'])
        self.assertEqual(calls[1][-2:], ['user', 'chris'])


class SharingDevice(unittest.TestCase):
    def test_it_says_its_account_password_set_and_samba_running(self):
        ds = load('hds_t', SHELL / 'hearth-device-status')
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        f = os.path.join(tmp, 'home-server.json')
        with open(f, 'w') as fh:
            json.dump({'smbPasswordSet': 1762300000000, 'shares': []}, fh)

        class R:
            stdout = 'active\n'
        with mock.patch.object(ds, '_load', side_effect=ImportError):
            out = ds.smb_names(path=f, run=lambda *a, **k: R())
        self.assertTrue(out['password'])
        self.assertTrue(out['on'])
        self.assertTrue(out['user'])
        self.assertNotIn('smbPasswordSet', json.dumps(out))

        class Off:
            stdout = 'inactive\n'
        with open(f, 'w') as fh:
            json.dump({'smbPasswordSet': 0}, fh)
        with mock.patch.object(ds, '_load', side_effect=ImportError):
            out = ds.smb_names(path=f, run=lambda *a, **k: Off())
        self.assertEqual((out['password'], out['on']), (False, False))

    def test_hearth_files_keeps_the_hints_from_its_status(self):
        sample = {'devices': [{'id': 'd', 'name': 'pc', 'online': True, 'status': {
            'shares': [{'name': 'Games', 'reach': 'tailscale'}],
            'smb': {'home': 'pc.local', 'away': 'pc.t.ts.net', 'user': 'dragon', 'password': False, 'on': True}}}]}
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        fake = os.path.join(tmp, 'helper')
        with open(fake, 'w') as f:
            f.write('import json\nprint(json.dumps(%r))\n' % sample)
        d = ns.hearth_devices(fake)[0]
        self.assertEqual((d['user'], d['pwSet'], d['smbOn']), ('dragon', False, True))
        self.assertEqual(d['shares'][0]['reach'], 'tailscale')

    def test_net_parse_drops_bad_hints(self):
        hf = load('hf_t', SHELL / 'hearth-files')
        n = hf.net_parse(json.dumps({'home': 'pc.local', 'share': 'G', 'reach': 'everywhere', 'user': 'a b;rm'}))
        self.assertNotIn('reach', n)
        self.assertNotIn('user', n)
        n = hf.net_parse(json.dumps({'home': 'pc.local', 'share': 'G', 'reach': 'tailscale', 'user': 'dragon'}))
        self.assertEqual((n['reach'], n['user']), ('tailscale', 'dragon'))


class Server(unittest.TestCase):
    def test_smbpasswds_own_reason_is_shown(self):
        for p in ('hearth-home-server-helper', 'hearth-storage-helper'):
            src = (LIBEXEC / p).read_text()
            self.assertIn('raise Fail(smbpasswd_failed(said))', src, p)
            self.assertNotIn('raise Fail("Samba didn\'t take the password")', src, p)
        hs = load('hshelper_t', LIBEXEC / 'hearth-storage-helper')
        self.assertEqual(hs.smbpasswd_failed('Failed to add entry for user chris.\n'),
                         "Samba didn't take the password: Failed to add entry for user chris.")

    def test_ufw_or_firewalld_get_port_445_while_home_shares_are_on(self):
        h = load('hhs_t', LIBEXEC / 'hearth-home-server-helper')
        calls = []

        def run(cmd, stdin=None, timeout=60):
            calls.append(cmd)
            return 0, '{"ok": true, "firewall": "ufw"}\n'
        real = os.path.exists
        with mock.patch.object(h, 'run', side_effect=run), \
             mock.patch.object(h.os.path, 'exists', side_effect=lambda p: True if p == h.LINK_FW else False if p == h.STORAGE_CONF else real(p)):
            self.assertEqual(h.lan_firewall(True), {'lanFirewall': 'ufw'})
            self.assertEqual(h.lan_firewall(False), {'lanFirewall': 'ufw'})
        self.assertEqual(calls, [[h.LINK_FW, 'open', 'smb'], [h.LINK_FW, 'close', 'smb']])
        # Hearth Storage's Windows sharing still on: its port stays open.
        calls.clear()
        with mock.patch.object(h, 'run', side_effect=run), mock.patch.object(h.os.path, 'exists', return_value=True):
            self.assertEqual(h.lan_firewall(False), {})
        self.assertEqual(calls, [])
        src = (LIBEXEC / 'hearth-home-server-helper').read_text()
        self.assertIn('res.update(lan_firewall(not blocked))', src)
        self.assertIn('out.update(lan_firewall(False))', src)


class Image(unittest.TestCase):
    def test_the_image_can_mount_shares(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        line = next(l for l in b.splitlines() if 'gvfs-backends' in l and l.startswith('"${APT[@]}"'))
        for pkg in ('libglib2.0-bin', 'gvfs', 'gvfs-backends', 'gvfs-fuse', 'libsecret-tools'):
            self.assertIn(' ' + pkg, line + ' ')
        self.assertIn('gvfsd-smb', b)
        self.assertIn('gvfsd-fuse', b)

    def test_files_and_settings_wiring(self):
        fm = (SHELL / 'FileManager.qml').read_text()
        self.assertIn('r.need === "pick"', fm)
        self.assertIn('netSignIn(net, r.user || user || "", !!password, password ? r.error : "")', fm)
        sd = (SHELL / 'SetDevices.qml').read_text()
        self.assertIn('reach: modelData.reach === "tailscale" ? "tailscale" : "home"', sd)


if __name__ == '__main__':
    unittest.main()
