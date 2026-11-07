"""Server apps catalog and Phone notifications (docs/SERVER-APPS.md, planned for Hearth 1.7): hearth-server-apps with
stand-ins for podman, systemctl and ntfy. Each app is off and not installed until asked; settings are written only
when you change something (one key, merged, never defaults); passwords never reach settings or a command line; the units
are your own user services with no [Install] and a start limit, never part of starting the PC or Game Mode; Tailscale
only really means 127.0.0.1 and the Tailscale address; data stays unless you confirm twice. Also the summary your other
devices see (hearth-device-status, Server mode's summary, the Hearth Link plugin) and the Settings pages' wiring."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import stat
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
HELPER = SHELL / 'hearth-server-apps'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        self.env = {'XDG_CONFIG_HOME': t + '/config', 'XDG_STATE_HOME': t + '/state', 'XDG_RUNTIME_DIR': t + '/run',
                    'HEARTH_SERVER_APPS_DATA': t + '/Server apps', 'HEARTH_HOME_SERVER_ALERT_STATE': t + '/alerts.json'}
        self.old = {k: os.environ.get(k) for k in self.env}
        os.environ.update(self.env)
        os.makedirs(t + '/run', exist_ok=True)
        self.m = load(HELPER, 'hearth_server_apps_t')
        self.calls = []
        self.images = set()
        self.active = {}
        self.posts = []

        def cmd(args, timeout=60, env=None):
            self.calls.append(list(args))
            if args[:3] == ['podman', 'image', 'exists']:
                return (0, '') if args[3] in self.images else (1, '')
            if args[:3] == ['systemctl', '--user', 'is-active']:
                s = self.active.get(args[3], 'inactive')
                return (0 if s == 'active' else 3), s + '\n'
            if args[:2] == ['podman', 'pull']:
                self.images.add(args[2])
                return 0, ''
            return 0, ''
        self.m.cmd = cmd
        self.m.have = lambda n: n in ('podman', 'tailscale')
        self.m.ts_ip4 = lambda: '100.84.12.7'
        self.m.lan_ip = lambda: '192.168.1.20'
        self.m.post = lambda url, text, title, priority='default', tags='': (self.posts.append((url, text, title, priority)), (True, ''))[1]

    def tearDown(self):
        for k, v in self.old.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        self.tmp.cleanup()

    def install(self, aid):
        for r in self.m.CATALOG[aid]['images']:
            self.images.add(r)

    def saved(self):
        p = self.m.SETTINGS
        return json.loads(pathlib.Path(p).read_text()) if os.path.exists(p) else None


class Catalog(Base):
    def test_the_catalog_has_what_the_owner_listed(self):
        for aid in ('minecraft', 'valheim', 'terraria', 'palworld', 'vaultwarden', 'immich', 'navidrome', 'ntfy'):
            self.assertIn(aid, self.m.CATALOG)
        for aid, a in self.m.CATALOG.items():
            for ref in a['images']:
                self.assertRegex(ref, r'^(docker\.io|ghcr\.io)/[a-z0-9._/-]+:[A-Za-z0-9._-]+$', aid + ': a full name with a registry and a tag')
            self.assertIn(a['reach'], ('home', 'tailscale'))
            for o in a['opts'].values():
                self.assertTrue(self.m.opt_ok(o, o['default']), aid + ': its default is one of its own choices')

    def test_everything_is_off_and_listing_saves_nothing(self):
        r = self.m.list_apps()
        self.assertTrue(all(not a['on'] and not a['installed'] for a in r['apps']))
        self.assertIsNone(self.saved(), 'listing never writes settings')
        self.assertFalse(any(c[:2] == ['systemctl', '--user'] and c[2] in ('enable', 'start', 'restart') for c in self.calls))
        self.assertEqual(self.m.quick(), {'apps': [], 'on': 0, 'running': 0, 'failed': []})


class Switching(Base):
    def test_on_needs_install_eula_and_password(self):
        with self.assertRaisesRegex(self.m.Fail, 'Install'):
            self.m.switch('terraria', True)
        self.install('minecraft')
        with self.assertRaisesRegex(self.m.Fail, 'EULA'):
            self.m.switch('minecraft', True)
        self.install('valheim')
        with self.assertRaisesRegex(self.m.Fail, 'password'):
            self.m.switch('valheim', True)
        self.assertIsNone(self.saved(), 'nothing saved when turning on was refused')

    def test_on_enables_only_its_timer_and_writes_one_key(self):
        self.install('terraria')
        self.m.switch('terraria', True)
        self.assertEqual(self.saved(), {'apps': {'terraria': {'on': True}}}, 'only the key you changed, no defaults')
        self.assertIn(['systemctl', '--user', 'enable', 'hearth-app@terraria.timer'], self.calls)
        self.assertIn(['systemctl', '--user', 'restart', 'hearth-app@terraria.service'], self.calls)
        self.m.switch('terraria', False)
        self.assertEqual(self.saved(), {'apps': {'terraria': {'on': False}}})
        self.assertIn(['systemctl', '--user', 'disable', 'hearth-app@terraria.timer'], self.calls)

    def test_units_are_guarded_user_services(self):
        self.m.ensure_units()
        d = pathlib.Path(self.m.UNIT_DIR)
        svc = (d / 'hearth-app@.service').read_text()
        self.assertNotIn('[Install]', svc, 'the service is never enabled on its own: only its timer, while the app is on')
        for want in ('ConditionPathExists=/usr/bin/podman', 'StartLimitBurst=4', 'StartLimitIntervalSec=900', 'Restart=on-failure'):
            self.assertIn(want, svc)
        self.assertIn('WantedBy=timers.target', (d / 'hearth-app@.timer').read_text())
        self.assertNotIn('graphical', svc + (d / 'hearth-app@.timer').read_text())
        n = len(self.calls)
        self.assertFalse(self.m.ensure_units(), 'unchanged units are not written again')
        self.assertEqual(len(self.calls), n)

    def test_two_apps_on_one_port_refused(self):
        self.install('navidrome')
        self.m.switch('navidrome', True)
        self.m.CATALOG['ntfy']['ports'] = [(4533, 'tcp')]
        self.install('ntfy')
        with self.assertRaisesRegex(self.m.Fail, 'Navidrome'):
            self.m.switch('ntfy', True)

    def test_options_are_checked(self):
        with self.assertRaises(self.m.Fail):
            self.m.set_opt('minecraft', 'memory', '64G')
        with self.assertRaises(self.m.Fail):
            self.m.set_opt('valheim', 'serverName', 'bad;name$(x)')
        with self.assertRaises(self.m.Fail):
            self.m.set_opt('navidrome', 'music', '/etc/../root')
        with self.assertRaises(self.m.Fail):
            self.m.set_key('minecraft', 'reach', 'internet')
        self.m.set_opt('minecraft', 'memory', '6G')
        self.m.set_key('minecraft', 'reach', 'tailscale')
        self.assertEqual(self.saved(), {'apps': {'minecraft': {'opts': {'memory': '6G'}, 'reach': 'tailscale'}}})

    def test_a_damaged_settings_file_is_never_overwritten_with_defaults(self):
        os.makedirs(os.path.dirname(self.m.SETTINGS), exist_ok=True)
        pathlib.Path(self.m.SETTINGS).write_text('{"apps": {"minecraft": {"on": tr')
        self.m.list_apps()
        now = pathlib.Path(self.m.SETTINGS).read_text() if os.path.exists(self.m.SETTINGS) else ''
        self.assertNotIn('"reach"', now, 'never filled with defaults')
        self.assertTrue(any('broken' in f for f in os.listdir(os.path.dirname(self.m.SETTINGS))), 'the damaged copy is kept')


class Running(Base):
    def test_home_binds_everywhere_tailscale_binds_only_tailscale(self):
        cfg = self.m.settings()['apps']['minecraft']
        argv = self.m.run_argv('minecraft', cfg, {}, '100.84.12.7')
        self.assertIn('25565:25565/tcp', argv)
        self.assertIn('--pull=never', argv)
        cfg['reach'] = 'tailscale'
        argv = self.m.run_argv('minecraft', cfg, {}, '100.84.12.7')
        ps = [argv[i + 1] for i, a in enumerate(argv) if a == '-p']
        self.assertEqual(ps, ['127.0.0.1:25565:25565/tcp', '100.84.12.7:25565:25565/tcp'])
        argv = self.m.run_argv('minecraft', cfg, {}, '')
        self.assertEqual([argv[i + 1] for i, a in enumerate(argv) if a == '-p'], ['127.0.0.1:25565:25565/tcp'])

    def test_passwords_never_in_settings_or_on_a_command_line(self):
        self.install('valheim')
        self.m.set_password('valheim', 'odinrules')
        self.assertIsNone(self.saved())
        mode = stat.S_IMODE(os.stat(self.m.SECRETS).st_mode)
        self.assertEqual(mode, 0o600)
        cfg = self.m.settings()['apps']['valheim']
        argv = self.m.run_argv('valheim', cfg, self.m.secrets_read()['valheim'], '')
        self.assertNotIn('odinrules', ' '.join(argv))
        p = self.m.env_file('valheim', self.m.env_for('valheim', cfg, self.m.secrets_read()['valheim']))
        self.assertEqual(stat.S_IMODE(os.stat(p).st_mode), 0o600)
        self.assertIn('SERVER_PASS=odinrules', pathlib.Path(p).read_text())
        with self.assertRaises(self.m.Fail):
            self.m.set_password('valheim', 'abc')
        with self.assertRaises(self.m.Fail):
            self.m.set_password('immich', 'whatever')

    def test_run_refuses_when_off(self):
        with self.assertRaisesRegex(self.m.Fail, 'off'):
            self.m.run('terraria')

    def test_terraria_world_and_immich_pod(self):
        argv = self.m.run_argv('terraria', self.m.settings()['apps']['terraria'], {}, '')
        self.assertEqual(argv[-6:], ['-world', '/root/.local/share/Terraria/Worlds/Hearth.wld', '-autocreate', '2', '-worldname', 'Hearth'])
        argv = self.m.run_argv('immich', self.m.settings()['apps']['immich'], {}, '')
        self.assertIn('--pod', argv)
        self.assertNotIn('-p', argv, 'the pod holds the port')

    def test_install_runs_in_its_own_unit_and_the_job_pulls(self):
        self.m.install('immich')
        self.assertTrue(any(c[0] == 'systemd-run' and '--user' in c and c[-2:] == ['job', 'immich'] for c in self.calls))
        self.assertTrue(self.m.secrets_read()['immich']['db'], 'its database password is made once, never shown')
        self.m.job('immich')
        self.assertTrue(self.m.installed('immich'))
        self.assertEqual(self.m.state()['immich']['progress'], 100)

    def test_remove_keeps_data_unless_confirmed_twice(self):
        self.install('terraria')
        d = self.m.data_dir('terraria', 'worlds')
        os.makedirs(d)
        r = self.m.remove('terraria', ['data', 'sure'])
        self.assertFalse(r['dataRemoved'])
        self.assertFalse(any(c[:3] == ['podman', 'unshare', 'rm'] for c in self.calls))
        self.m.remove('terraria', ['data', 'sure', 'really-sure'])
        self.assertTrue(any(c[:3] == ['podman', 'unshare', 'rm'] and c[-1] == self.m.data_dir('terraria') for c in self.calls))

    def test_list_and_quick(self):
        self.install('minecraft')
        self.m.set_key('minecraft', 'eula', 'on')
        self.m.switch('minecraft', True)
        self.active['hearth-app@minecraft.service'] = 'active'
        r = {a['id']: a for a in self.m.list_apps()['apps']}
        self.assertTrue(r['minecraft']['running'])
        self.assertEqual(r['minecraft']['addresses'][0], {'where': 'Home network', 'url': '192.168.1.20:25565'})
        self.assertFalse(r['valheim']['installed'])
        q = self.m.quick()
        self.assertEqual((q['on'], q['running']), (1, 1))
        self.active['hearth-app@minecraft.service'] = 'failed'
        self.assertEqual(self.m.quick()['failed'], ['Minecraft'])


class Ntfy(Base):
    def test_off_until_a_topic_and_on(self):
        with self.assertRaisesRegex(self.m.Fail, 'topic'):
            self.m.ntfy_set('on', 'on')
        self.assertEqual(self.m.ntfy_forward(), {'sent': 0, 'note': 'off'})
        self.assertEqual(self.posts, [])
        v = self.m.ntfy_set('topic', 'new')
        self.assertRegex(v['settings']['topic'], r'^hearth-[A-Za-z0-9]{16}$')
        with self.assertRaises(self.m.Fail):
            self.m.ntfy_set('server', 'file:///etc/passwd')
        with self.assertRaises(self.m.Fail):
            self.m.ntfy_set('topic', 'a b')
        self.m.ntfy_set('on', 'on')
        self.assertIn(['systemctl', '--user', 'enable', 'hearth-server-ntfy.timer'], self.calls)
        self.assertEqual(set(self.saved()['ntfy']), {'topic', 'on'}, 'only what you changed')

    def test_forward_sends_what_is_new_once_a_day(self):
        self.m.ntfy_set('topic', 'hearth-test')
        self.m.ntfy_set('on', 'on')
        pathlib.Path(self.env['HEARTH_HOME_SERVER_ALERT_STATE']).write_text(json.dumps(
            {'active': [{'key': 'full:/mnt/a', 'kind': 'full', 'text': 'The drive with /mnt/a is 92% full'},
                        {'key': 'smart:x', 'kind': 'smart', 'text': 'Disk X is failing'}]}))
        self.assertEqual(self.m.ntfy_forward(now=1000)['sent'], 2)
        self.assertEqual(self.posts[0][0], 'https://ntfy.sh/hearth-test')
        self.assertEqual(self.posts[1][3], 'high', 'a failing drive is urgent')
        self.assertEqual(self.m.ntfy_forward(now=2000)['sent'], 0, 'not again within a day')
        self.assertEqual(self.m.ntfy_forward(now=1000 + 86400)['sent'], 2)
        self.install('terraria')
        self.m.switch('terraria', True)
        self.active['hearth-app@terraria.service'] = 'failed'
        self.assertEqual(self.m.ntfy_forward(now=1000 + 86400 + 60)['sent'], 1)
        self.assertIn('Terraria', self.posts[-1][1])
        self.m.ntfy_set('alerts', 'off')
        self.m.ntfy_set('apps', 'off')
        n = len(self.posts)
        self.m.ntfy_forward(now=10 ** 7)
        self.assertEqual(len(self.posts), n)

    def test_test_message(self):
        with self.assertRaises(self.m.Fail):
            self.m.ntfy_send('x')
        self.m.ntfy_set('topic', 'hearth-test')
        self.assertTrue(self.m.ntfy_send('hello')['ok'])
        self.assertTrue(self.m.ntfy_view()['last']['ok'])


class Summary(unittest.TestCase):
    def test_device_status_trims_the_server_summary(self):
        ds = load(SHELL / 'hearth-device-status', 'hearth_device_status_t')
        v = {'status': {'level': 'ok', 'title': 'All good'}, 'playing': False, 'sinceText': '7:02 AM',
             'services': [{'id': 'storage', 'title': 'Storage', 'engine': 'Shared folders: Media', 'state': 'running', 'chip': 'Healthy',
                           'big': '1.2 TB', 'unit': 'of 4 TB used', 'rows': [['Media', '/mnt/secret/path']]},
                          {'id': 'who', 'title': 'Who\'s connected', 'state': 'running', 'chip': 'Connected'}],
             'who': [{'name': 'phone', 'what': ['Media']}], 'alerts': [{'level': 'warn', 'text': 'A drive is hot'}],
             'apps': [{'name': 'Minecraft', 'state': 'active', 'kind': 'game'}]}
        t = ds.trim_server(v)
        self.assertEqual([s['title'] for s in t['services']], ['Storage'])
        self.assertNotIn('/mnt/secret', json.dumps(t))
        self.assertNotIn('phone', json.dumps(t))
        self.assertEqual(t['alerts'], ['A drive is hot'])
        self.assertEqual(t['apps'], [{'name': 'Minecraft', 'state': 'active'}])
        with tempfile.TemporaryDirectory() as t:
            self.assertFalse(ds.server_on(os.path.join(t, 'server-mode.json')))
            pathlib.Path(t, 'server-mode.json').write_text('{"on": true}')
            self.assertTrue(ds.server_on(os.path.join(t, 'server-mode.json')))
            old = os.environ.get('XDG_CONFIG_HOME')
            os.environ['XDG_CONFIG_HOME'] = os.path.join(t, 'none')
            try:
                self.assertIsNone(ds.server_summary('desktop', run=lambda *a, **k: self.fail('not asked without Server mode')))
            finally:
                if old is None:
                    os.environ.pop('XDG_CONFIG_HOME', None)
                else:
                    os.environ['XDG_CONFIG_HOME'] = old

    def test_server_mode_summary_carries_the_apps(self):
        sm = (SHELL / 'hearth-server-mode').read_text()
        self.assertIn("'apps': sa", sm)
        self.assertIn("('Server apps', ''", sm)

    def test_hearth_link_plugin_shows_your_server(self):
        src = (ROOT / 'hosts/decky-hearth-link/main.py').read_text()
        self.assertIn('def servers(', src)
        self.assertIn('async def servers(self)', src)
        tsx = (ROOT / 'hosts/decky-hearth-link/src/index.tsx').read_text()
        self.assertIn('<Guard><YourServers /></Guard>', tsx)


class Pages(unittest.TestCase):
    def test_pages_are_registered_and_linked(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        self.assertIn('id: "hs-apps", file: "SetHomeApps.qml"', reg)
        self.assertIn('id: "hs-phone", file: "SetHomePhone.qml"', reg)
        landing = (SHELL / 'SetHomeServer.qml').read_text()
        self.assertIn('target: "hs-apps"', landing)
        self.assertIn('target: "hs-phone"', landing)
        apps = (SHELL / 'SetHomeApps.qml').read_text()
        self.assertIn('helper: "hearth-server-apps"', apps)
        self.assertIn('"data", "sure", "really-sure"', apps)
        self.assertIn('secret: true', apps)
        self.assertIn('Its server', (SHELL / 'SetDevices.qml').read_text())

    def test_nothing_in_the_image_or_the_session_changed_for_it(self):
        for f in (ROOT / 'hosts/linux/session/nimbus-session', ROOT / 'os-bases/peppermint/build.sh'):
            self.assertNotIn('hearth-server-apps', f.read_text())
        self.assertFalse((ROOT / 'os/system_files/usr/lib/systemd/user/hearth-app@.service').exists())


if __name__ == '__main__':
    unittest.main()
