"""Your devices' shared folders in Hearth Files (hearth_netshares.py, hearth-files net-open / devices-refresh /
places): finding them (avahi-browse, Tailscale, Hearth devices' status), the home name at home and the Tailscale name
away, the keyring (a stand-in secret-tool), signing in through gio's questions (a stand-in gio), the sidebar model,
and that no password is ever written to Hearth's files or a command line."""
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
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import hearth_netshares as ns  # noqa: E402
import devices_sample  # noqa: E402

AVAHI = '\n'.join([
    '+;eth0;IPv4;Living-room\\032NAS;_smb._tcp;local',
    '=;eth0;IPv4;Living-room\\032NAS;_smb._tcp;local;nas.local;192.168.1.20;445;"model=Xserve"',
    '=;eth0;IPv6;Living-room\\032NAS;_smb._tcp;local;nas.local;fe80::1;445;',
    '=;eth0;IPv4;CHRIS-HEARTH;_smb._tcp;local;chris-hearth.local;192.168.1.30;445;',
    'junk'])

FAKE_SECRET_TOOL = r'''#!/usr/bin/env python3
import json, os, sys
db = os.environ['FAKE_SECRETS']
try:
    items = json.load(open(db))
except Exception:
    items = []
a = sys.argv[1:]
cmd = a.pop(0)
if cmd == 'store':
    label = a.pop(0)
    attrs = dict(zip(a[::2], a[1::2]))
    secret = sys.stdin.read()
    open(os.environ['FAKE_ARGS'], 'a').write(' '.join(sys.argv) + '\n')
    items = [i for i in items if i['attrs'] != attrs] + [{'attrs': attrs, 'secret': secret, 'label': label}]
    json.dump(items, open(db, 'w'))
elif cmd == 'lookup':
    q = dict(zip(a[::2], a[1::2]))
    for i in items:
        if all(i['attrs'].get(k) == v for k, v in q.items()):
            sys.stdout.write(i['secret'])
            sys.exit(0)
    sys.exit(1)
'''

FAKE_GIO = r'''#!/usr/bin/env python3
import os, sys
open(os.environ['FAKE_ARGS'], 'a').write(' '.join(sys.argv) + '\n')
if sys.argv[1] == 'mount':
    sys.stdout.write('Authentication Required\nEnter user and password for share \u201cMedia\u201d on \u201chost\u201d:\n')
    sys.stdout.write('User [me]: '); sys.stdout.flush(); user = sys.stdin.readline().strip()
    sys.stdout.write('Domain [WORKGROUP]: '); sys.stdout.flush(); sys.stdin.readline()
    sys.stdout.write('Password: '); sys.stdout.flush(); pw = sys.stdin.readline().rstrip('\n')
    if (user, pw) == ('chris', 'hunter 2'):
        sys.exit(0)
    sys.stdout.write('\ngio: smb://host/Media/: Failed to mount Windows share: Permission denied\n')
    sys.exit(2)
'''


class Finding(unittest.TestCase):
    def test_avahi_lines(self):
        s = ns.parse_avahi(AVAHI)
        self.assertEqual([x['host'] for x in s], ['nas.local', 'chris-hearth.local'])
        self.assertEqual(s[0]['name'], 'Living-room NAS')
        self.assertEqual(s[0]['port'], 445)

    def test_tailscale_and_probing_only_online_peers(self):
        ts = ns.parse_tailscale({'BackendState': 'Running', 'Self': {'DNSName': 'laptop.tail.ts.net.'},
                                 'Peer': {'a': {'HostName': 'nvidia-pc', 'DNSName': 'nvidia-pc.tail.ts.net.', 'Online': True},
                                          'b': {'HostName': 'phone', 'DNSName': 'phone.tail.ts.net.', 'Online': True},
                                          'c': {'HostName': 'old', 'DNSName': 'old.tail.ts.net.', 'Online': False}}})
        self.assertTrue(ts['running'])
        self.assertEqual(ts['self'], 'laptop.tail.ts.net')
        asked = []
        found = ns.tailscale_servers(ts, probe=lambda h: asked.append(h) or h.startswith('nvidia'))
        self.assertEqual(sorted(asked), ['nvidia-pc.tail.ts.net', 'phone.tail.ts.net'])   # never the offline one
        self.assertEqual(found, [{'name': 'nvidia-pc', 'host': 'nvidia-pc.tail.ts.net', 'away': True}])

    def test_discover_leaves_out_hearth_devices_found_twice(self):
        devs = [{'id': 'a', 'name': 'chris-hearth', 'online': True, 'home': 'chris-hearth.local', 'away': 'chris-hearth.tail.ts.net',
                 'shares': [{'name': 'Media'}]}]
        with mock.patch.object(ns, 'hearth_devices', return_value=devs):
            d = ns.discover('helper', avahi=lambda: ns.parse_avahi(AVAHI), now=5,
                            ts_state=lambda: {'running': True, 'self': 'me', 'peers': [{'name': 'chris-hearth', 'dns': 'chris-hearth.tail.ts.net', 'online': True}]},
                            probe=lambda h: True)
        self.assertEqual([s['host'] for s in d['servers']], ['nas.local'])
        self.assertEqual(d['devices'], devs)
        self.assertEqual(d['at'], 5)

    def test_hearth_devices_from_their_status(self):
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        fake = os.path.join(tmp, 'helper')
        with open(fake, 'w') as f:
            f.write('import json\nprint(json.dumps(%r))\n' % devices_sample.sample())
        devs = ns.hearth_devices(fake)
        self.assertEqual([d['name'] for d in devs], ['chris-hearth'])          # only devices sharing something
        self.assertEqual(devs[0]['home'], 'chris-hearth.local')
        self.assertEqual(devs[0]['shares'], [{'name': 'Media', 'write': False, 'badge': 'drive', 'reach': 'home'}])


class Address(unittest.TestCase):
    dev = {'home': 'pc.local', 'away': 'pc.tail.ts.net'}

    def test_home_when_it_answers_here_else_tailscale(self):
        self.assertEqual(ns.choose_address(self.dev, lambda h: True, True), ('pc.local', 'home'))
        self.assertEqual(ns.choose_address(self.dev, lambda h: False, True), ('pc.tail.ts.net', 'away'))
        self.assertEqual(ns.choose_address(self.dev, lambda h: False, False), (None, 'tailscale-off'))
        self.assertEqual(ns.choose_address({'home': 'pc.local'}, lambda h: False, False), ('pc.local', 'home'))
        self.assertEqual(ns.choose_address({}, lambda h: True, True), (None, 'nowhere'))

    def test_errors_say_what_happened(self):
        self.assertEqual(ns.classify('Failed to mount Windows share: Permission denied', 'home')['need'], 'password')
        self.assertEqual(ns.classify('NT_STATUS_BAD_NETWORK_NAME', 'home')['need'], 'off')
        self.assertEqual(ns.classify('Failed to connect: Connection timed out', 'home')['need'], 'asleep')
        self.assertEqual(ns.classify('', 'tailscale-off')['need'], 'tailscale')


class Sidebar(unittest.TestCase):
    def test_model(self):
        st = devices_sample.files_state(now=1000)
        side = ns.sidebar(st)
        self.assertEqual([p['label'] for p in side], ['chris-hearth', 'Media', 'chris', 'ROG Ally', 'Screenshots', 'Living-room NAS',
                                                      'How to open from Windows or a phone'])
        self.assertTrue(all(p['section'] == 'devices' for p in side))
        media = side[1]
        self.assertTrue(media['child'])
        self.assertEqual(media['sub'], 'Whole drive · Read only')
        self.assertEqual(side[2]['sub'], 'Home · Read & write')
        self.assertEqual(media['net'], {'device': 'a1' * 16, 'name': 'chris-hearth', 'share': 'Media', 'home': 'chris-hearth.local',
                                        'away': 'chris-hearth.tail4f2a.ts.net', 'wake': True})
        self.assertTrue(side[3]['offline'])
        self.assertEqual(side[3]['sub'], 'Offline or asleep')
        how = side[-1]['howto']
        self.assertEqual(how[0]['windows'], '\\\\chris-hearth.local\\Media')
        self.assertEqual(how[0]['phone'], 'smb://chris-hearth.local/Media')
        self.assertEqual(how[0]['away'], '\\\\chris-hearth.tail4f2a.ts.net\\Media')
        self.assertEqual(ns.sidebar(st, show=False), [])
        self.assertEqual(ns.sidebar({}), [])                                   # nothing found: no section

    def test_state_keeps_user_names_and_offline_devices(self):
        old = {'users': {'pc.local': 'chris'}, 'devices': [{'id': 'x', 'name': 'Old', 'online': True, 'shares': []}]}
        new = {'at': 9, 'devices': [], 'servers': []}
        m = ns.merge_state(old, new)
        self.assertEqual(m['users'], {'pc.local': 'chris'})
        self.assertEqual(m['devices'][0]['online'], False)


class SignIn(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        bin_ = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_)
        for name, text in (('secret-tool', FAKE_SECRET_TOOL), ('gio', FAKE_GIO)):
            p = os.path.join(bin_, name)
            with open(p, 'w') as f:
                f.write(text)
            os.chmod(p, os.stat(p).st_mode | stat.S_IEXEC)
        self.args = os.path.join(self.tmp, 'args.log')
        self.env = mock.patch.dict(os.environ, {'PATH': bin_ + os.pathsep + os.environ.get('PATH', ''), 'HOME': self.tmp,
                                                'XDG_STATE_HOME': os.path.join(self.tmp, 'state'), 'FAKE_SECRETS': os.path.join(self.tmp, 'secrets.json'),
                                                'FAKE_ARGS': self.args, 'HEARTH_SECRET_TOOL': os.path.join(bin_, 'secret-tool')})
        self.env.start()
        self.net = {'device': 'a', 'name': 'chris-hearth', 'share': 'Media', 'home': 'host', 'away': ''}

    def tearDown(self):
        self.env.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def open(self, **kw):
        return ns.net_open(self.net, at_home=lambda h: True, ts_running=False, path_of=lambda uri: None, state={}, **kw)

    def test_keyring_store_and_lookup(self):
        ns.keyring_store('PC.local', 'Media', 'chris', 'hunter 2')
        self.assertEqual(ns.keyring_lookup('pc.local', 'Media', 'chris'), 'hunter 2')
        self.assertIsNone(ns.keyring_lookup('pc.local', 'Media', 'someone'))
        log = open(self.args).read()
        self.assertNotIn('hunter', log)                                       # never on a command line
        self.assertIn('protocol smb server pc.local object Media user chris domain WORKGROUP', log)

    def test_first_time_asks_then_remembers_in_the_keyring_only(self):
        r = self.open()
        self.assertEqual((r['ok'], r['need']), (False, 'password'))
        r = self.open(user='chris', password='wrong')
        self.assertEqual((r['ok'], r['need']), (False, 'password'))
        r = self.open(user='chris', password='hunter 2', remember=True)
        self.assertTrue(r['ok'])
        self.assertTrue(r['remembered'])
        self.assertEqual(r['where'], 'home')
        # Next time: one click (the user name from Hearth's state, the password from the keyring).
        st = ns.read_state()
        self.assertEqual(st['users'], {'host': 'chris'})
        r = ns.net_open(self.net, at_home=lambda h: True, ts_running=False, path_of=lambda uri: None)
        self.assertTrue(r['ok'])
        # No password anywhere in Hearth's files, nor on any command line.
        for root, _, files in os.walk(os.path.join(self.tmp, 'state')):
            for f in files:
                self.assertNotIn('hunter', open(os.path.join(root, f)).read())
        self.assertNotIn('hunter', open(self.args).read())

    def test_just_this_time_keeps_nothing(self):
        r = self.open(user='chris', password='hunter 2', remember=False)
        self.assertTrue(r['ok'])
        self.assertIsNone(r['remembered'])
        self.assertFalse(os.path.exists(os.environ['FAKE_SECRETS']))

    def test_tailscale_off_says_so(self):
        r = ns.net_open({'name': 'pc', 'share': 'Media', 'home': '', 'away': 'pc.tail.ts.net'}, at_home=lambda h: False,
                        ts_running=False, path_of=lambda uri: None, state={})
        self.assertEqual((r['ok'], r['need']), (False, 'tailscale'))


class Wiring(unittest.TestCase):
    def test_hearth_files_and_the_sidebar(self):
        hf = (SHELL / 'hearth-files').read_text()
        self.assertIn('out += device_places()', hf)
        self.assertIn("if prefs_get().get('showDevices', True) is False:", hf)
        self.assertIn("os.environ.get('HEARTH_INPUT', '')", hf)                # the password: never argv
        self.assertIn("['/tmp', gvfs_root()]", hf)
        fm = (SHELL / 'FileManager.qml').read_text()
        self.assertIn('devices: "Your devices"', fm)
        self.assertIn('if (p.net) return netOpen(p.net);', fm)
        self.assertIn('environment: ({ HEARTH_INPUT: procIn.input })', fm)
        self.assertIn('echoMode: app.dlgSecret ? TextInput.Password : TextInput.Normal', fm)
        self.assertIn('"How to open from Windows or a phone"', fm)
        self.assertIn('hearthnet://', (SHELL / 'SetDevices.qml').read_text())

    def test_places_without_a_session_never_start_looking(self):
        loader = importlib.machinery.SourceFileLoader('hearth_files_t', str(SHELL / 'hearth-files'))
        spec = importlib.util.spec_from_loader('hearth_files_t', loader)
        hf = importlib.util.module_from_spec(spec)
        loader.exec_module(hf)
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        env = {'HOME': tmp, 'XDG_STATE_HOME': os.path.join(tmp, 'state'), 'XDG_CONFIG_HOME': os.path.join(tmp, 'cfg')}
        with mock.patch.dict(os.environ, env), mock.patch.object(hf, 'start_refresh') as sr:
            os.environ.pop('WAYLAND_DISPLAY', None)
            self.assertEqual(hf.device_places(), [])
            sr.assert_not_called()
            os.makedirs(os.path.join(tmp, 'state', 'nimbus'))
            with open(os.path.join(tmp, 'state', 'nimbus', 'files-devices.json'), 'w') as f:
                json.dump(devices_sample.files_state(), f)
            self.assertEqual(len(hf.device_places()), 7)
            os.makedirs(os.path.join(tmp, 'cfg', 'nimbus'))
            with open(os.path.join(tmp, 'cfg', 'nimbus', 'files.json'), 'w') as f:
                json.dump({'showDevices': False}, f)
            self.assertEqual(hf.device_places(), [])


def load_files():
    loader = importlib.machinery.SourceFileLoader('hearth_files_auto', str(SHELL / 'hearth-files'))
    spec = importlib.util.spec_from_loader('hearth_files_auto', loader)
    hf = importlib.util.module_from_spec(spec)
    loader.exec_module(hf)
    return hf


class ConnectAutomatically(unittest.TestCase):
    """Hearth OS 1.7: a shared folder's "Connect automatically" (files.json autoConnect, the user timer)."""
    NET = {'device': 'abc123', 'name': 'chris-hearth', 'share': 'Games', 'home': 'chris-hearth.local',
           'away': 'chris-hearth.tail0e2082.ts.net', 'wake': True}

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.rt = os.path.join(self.tmp, 'run', 'auto.json')

    def test_the_switch_is_kept_merged_with_the_other_settings_and_never_a_password(self):
        hf = load_files()
        env = {'HOME': self.tmp, 'XDG_CONFIG_HOME': os.path.join(self.tmp, 'cfg'), 'XDG_RUNTIME_DIR': os.path.join(self.tmp, 'run')}
        with mock.patch.dict(os.environ, env):
            hf.prefs_set(json.dumps({'view': 'list', 'showDevices': True}))
            r = hf.net_auto(json.dumps(dict(self.NET, password='hunter2')), True)
            self.assertTrue(r['on'])
            p = hf.prefs_get()
            self.assertEqual(p['view'], 'list')                              # the rest is untouched
            self.assertEqual(p['autoConnect']['dev:abc123:Games']['share'], 'Games')
            raw = open(hf.prefs_file()).read()
            self.assertNotIn('hunter2', raw)
            hf.prefs_set(json.dumps({'sort': 'date'}))                       # another save keeps it
            self.assertIn('dev:abc123:Games', hf.prefs_get()['autoConnect'])
            hf.net_auto(json.dumps(self.NET), False)
            self.assertEqual(hf.prefs_get()['autoConnect'], {})
            self.assertEqual(hf.prefs_get()['sort'], 'date')
            with self.assertRaises(hf.Fail):
                hf.net_auto(json.dumps(dict(self.NET, share='')), True)    # a whole computer isn't a share

    def test_nothing_to_do_without_any(self):
        hf = load_files()
        env = {'HOME': self.tmp, 'XDG_CONFIG_HOME': os.path.join(self.tmp, 'cfg')}
        with mock.patch.dict(os.environ, env):
            self.assertEqual(hf.net_autoconnect(), {'skipped': 'none'})
            with mock.patch.dict(os.environ, {'HEARTH_NO_AUTOCONNECT': '1'}):
                self.assertEqual(hf.net_autoconnect(), {'skipped': 'off'})

    def test_the_sidebar_marks_the_ones_that_connect_by_themselves(self):
        st = {'devices': [{'id': 'abc123', 'name': 'chris-hearth', 'online': True, 'home': 'chris-hearth.local',
                           'shares': [{'name': 'Games'}, {'name': 'Work', 'write': True}]}]}
        items = {i['label']: i for i in ns.sidebar(st, True, {'dev:abc123:Games'}) if i.get('child') and i.get('net')}
        self.assertTrue(items['Games']['auto'])
        self.assertIn('Connects by itself', items['Games']['sub'])
        self.assertFalse(items['Work']['auto'])

    def run_auto(self, entries, probe=True, mounted=None, mount_result=(True, ''), password='pw', users=None):
        calls = []

        def mount(uri, user, pw):
            calls.append((uri, user, pw))
            return mount_result
        st = {'users': users if users is not None else {'chris-hearth.local': 'chris'}, 'tailscale': {'running': True}}
        r = ns.autoconnect(entries, at_home=lambda h: h.endswith('.local'), probe=lambda h, port, t: probe,
                           mount=mount, lookup=lambda s, sh, u: password, path_of=lambda uri: mounted,
                           state=st, runtime=self.rt, budget=5)
        return r, calls

    def test_opens_with_what_the_keyring_remembers_never_asking(self):
        r, calls = self.run_auto({'k': self.NET})
        self.assertEqual(r, {'k': 'opened'})
        self.assertEqual(calls, [('smb://chris-hearth.local/Games', 'chris', 'pw')])

    def test_already_open_is_left_alone(self):
        r, calls = self.run_auto({'k': self.NET}, mounted='/run/user/1000/gvfs/x')
        self.assertEqual(r, {'k': 'open'})
        self.assertEqual(calls, [])

    def test_asleep_is_skipped_quietly_and_tried_again_next_time(self):
        r, calls = self.run_auto({'k': self.NET}, probe=False)
        self.assertEqual(r, {'k': 'asleep'})
        self.assertEqual(calls, [])
        r, calls = self.run_auto({'k': self.NET})
        self.assertEqual(r, {'k': 'opened'})

    def test_needing_a_password_is_left_until_opened_by_hand(self):
        r, _ = self.run_auto({'k': self.NET}, password=None, mount_result=(False, 'password needed'))
        self.assertEqual(r, {'k': 'password'})
        r, calls = self.run_auto({'k': self.NET})
        self.assertEqual(r, {'k': 'skipped'})
        self.assertEqual(calls, [])
        ns.forget_gave_up('k', runtime=self.rt)
        r, _ = self.run_auto({'k': self.NET})
        self.assertEqual(r, {'k': 'opened'})

    def test_one_shares_trouble_never_stops_the_others(self):
        bad = dict(self.NET, share='Broken')

        def path_of(uri):
            if 'Broken' in uri:
                raise RuntimeError('boom')
            return None
        st = {'users': {'chris-hearth.local': 'chris'}, 'tailscale': {'running': True}}
        r = ns.autoconnect({'a': bad, 'b': self.NET}, at_home=lambda h: True, probe=lambda h, p, t: True,
                           mount=lambda u, us, pw: (True, ''), lookup=lambda *a: 'pw', path_of=path_of,
                           state=st, runtime=self.rt, budget=5)
        self.assertEqual(r, {'a': 'error', 'b': 'opened'})

    def test_wiring_menu_timer_and_nothing_waits_for_it(self):
        fm = (SHELL / 'FileManager.qml').read_text()
        self.assertIn('id: "p-auto", label: (p.auto ? "✓  " : "     ") + "Connect automatically"', fm)
        self.assertIn('call(["net-auto", JSON.stringify(p.net), on ? "on" : "off"]', fm)
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('ExecStart=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-files net-autoconnect', b)
        self.assertIn('ln -sf ../hearth-files-autoconnect.timer /usr/lib/systemd/user/timers.target.wants/hearth-files-autoconnect.timer', b)
        unit = b.split('hearth-files-autoconnect.service <<')[1].split('UNIT\n', 2)[1]
        for word in ('Before=', 'RequiredBy=', 'WantedBy=graphical', 'gamescope', 'steam'):
            self.assertNotIn(word, unit)          # it isn't part of signing in, Game Mode or Steam starting


if __name__ == '__main__':
    unittest.main()
