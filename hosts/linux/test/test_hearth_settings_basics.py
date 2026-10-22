"""Tests for Desktop Mode's Settings basics: hearth-admin's date/time, account and printer commands (argument
checks above all, with a fake command runner), and the parsing in hearth-system (time zones, keyboard layouts,
labwc keybinds, default apps, lpstat), hearth-apps (flatpak and Flathub's appstream) and hearth-clipboard."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import tempfile
import types
import unittest
from contextlib import redirect_stdout
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LABWC = ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


PASSWD = '\n'.join([
    'root:x:0:0:root:/root:/bin/bash',
    'daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin',
    'sam:x:1000:1000:Sam Smith,,,:/var/home/sam:/bin/bash',
    'kid:x:1001:1001:Kid:/var/home/kid:/bin/bash',
    'hearth-p2:x:1002:1002:Hearth Player 2:/var/home/hearth-p2:/bin/bash',
    'svc:x:1003:1003::/nonexistent:/usr/sbin/nologin',
    'nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin',
]) + '\n'
GROUP = 'root:x:0:\nwheel:x:10:sam\nsam:x:1000:\nkid:x:1001:\n'


class FakeRunner:
    """Records commands; answers from a {first words: (code, output)} table."""

    def __init__(self, answers=None):
        self.calls = []
        self.answers = answers or {}

    def __call__(self, cmd, input=None, timeout=120, env=None):
        self.calls.append((list(cmd), input))
        for key, ans in self.answers.items():
            if tuple(cmd[:len(key)]) == key:
                return ans(cmd) if callable(ans) else ans
        return 0, ''


class AdminBase(unittest.TestCase):
    def setUp(self):
        self.a = load(NIMBUS / 'hearth-admin', 'hearth_admin_basics')
        self.tmp = tempfile.mkdtemp()
        self.a.PASSWD_FILE = os.path.join(self.tmp, 'passwd')
        self.a.GROUP_FILE = os.path.join(self.tmp, 'group')
        self.a.ACCOUNTS_DIR = os.path.join(self.tmp, 'accounts')
        write(self.a.PASSWD_FILE, PASSWD)
        write(self.a.GROUP_FILE, GROUP)
        self.run = FakeRunner()
        self.a.admin_run = self.run
        self.env = mock.patch.dict(os.environ, {'PKEXEC_UID': str(os.getuid())})
        self.env.start()
        me = types.SimpleNamespace(pw_name='sam', pw_uid=1000, pw_dir='/var/home/sam')
        self.a.caller = lambda: me
        self.a.time.sleep = lambda s: None

    def tearDown(self):
        self.env.stop()

    def call(self, fn, *args, stdin=''):
        out = io.StringIO()
        fake_in = io.TextIOWrapper(io.BytesIO(stdin.encode() if isinstance(stdin, str) else stdin))
        with redirect_stdout(out), mock.patch.object(sys, 'stdin', fake_in):
            try:
                fn(*args)
                code = 0
            except SystemExit as e:
                code = e.code
        return code, out.getvalue()


class Accounts(AdminBase):
    def test_people_are_the_real_accounts(self):
        people = {p['name']: p for p in self.a.people()}
        self.assertEqual(sorted(people), ['hearth-p2', 'kid', 'sam'])
        self.assertTrue(people['sam']['admin'])
        self.assertFalse(people['kid']['admin'])
        self.assertTrue(people['hearth-p2']['player'])
        self.assertEqual(people['sam']['fullname'], 'Sam Smith')

    def test_add_request_checks_everything(self):
        ok = json.dumps({'password': 'secret1', 'fullname': 'Alex Doe'})
        self.assertEqual(self.a.user_add_request('alex', 'admin', ok), ('alex', True, 'Alex Doe', 'secret1'))
        self.assertEqual(self.a.user_add_request('_x-1', 'standard', json.dumps({'password': 'abcd'}))[:3], ('_x-1', False, ''))
        bad_names = ['', 'Alex', '1alex', 'al ex', 'alex;rm', '-alex', 'a' * 33, '../etc', 'alex\n', 'al:ex', 'hearth-p3']
        for n in bad_names:
            with self.assertRaises(ValueError, msg=n):
                self.a.user_add_request(n, 'standard', ok)
        for kind in ('root', 'wheel', '', 'Admin'):
            with self.assertRaises(ValueError, msg=kind):
                self.a.user_add_request('alex', kind, ok)
        for raw in ('', 'nope', '[]', '{"fullname": "x"}', '{"password": 5}', json.dumps({'password': 'abc'}),
                    json.dumps({'password': 'ab\ncd'}), json.dumps({'password': 'abcd', 'fullname': 'a:b'}),
                    json.dumps({'password': 'abcd', 'fullname': 'x' * 65}), json.dumps({'password': 'abcd', 'fullname': 'a,b'})):
            with self.assertRaises(ValueError, msg=raw):
                self.a.user_add_request('alex', 'standard', raw)

    def test_add_runs_useradd_and_chpasswd_with_single_arguments(self):
        code, out = self.call(self.a.user_add, 'alex', 'admin', stdin=json.dumps({'password': 'pw-1234', 'fullname': 'Alex'}) + '\n')
        self.assertEqual(code, 0, out)
        cmds = [c for c, _ in self.run.calls]
        useradd = cmds[0]
        self.assertEqual(useradd[0], 'useradd')
        self.assertEqual(useradd[-2:], ['--', 'alex'])
        self.assertIn('--groups', useradd)
        self.assertEqual(useradd[useradd.index('--groups') + 1], 'wheel')
        self.assertEqual(useradd[useradd.index('--comment') + 1], 'Alex')
        self.assertEqual(self.run.calls[1], (['chpasswd'], 'alex:pw-1234\n'))
        self.assertNotIn('pw-1234', out)                       # the password never shows
        self.assertEqual(json.loads(out.strip().splitlines()[-1]), {'ok': True, 'user': 'alex', 'admin': True})

    def test_standard_account_isnt_in_wheel(self):
        code, out = self.call(self.a.user_add, 'alex', 'standard', stdin=json.dumps({'password': 'pw-1234'}) + '\n')
        self.assertEqual(code, 0, out)
        self.assertNotIn('--groups', self.run.calls[0][0])
        self.assertNotIn('--comment', self.run.calls[0][0])

    def test_add_refuses_bad_input_before_running_anything(self):
        for name, kind, stdin in (('Bad Name', 'standard', '{"password": "abcd"}'), ('alex', 'root', '{"password": "abcd"}'),
                                  ('alex', 'standard', '{"password": "ab"}'), ('sam', 'standard', '{"password": "abcd"}'),
                                  ('kid', 'admin', '{"password": "abcd"}'), ('wheel', 'standard', '{"password": "abcd"}')):
            self.run.calls.clear()
            code, out = self.call(self.a.user_add, name, kind, stdin=stdin + '\n')
            self.assertEqual(code, 1, (name, out))
            self.assertTrue(out.startswith('error: '), out)
            self.assertEqual(self.run.calls, [], name)

    def test_failed_password_removes_the_new_account(self):
        self.run.answers[('chpasswd',)] = (1, 'chpasswd: bad')
        code, out = self.call(self.a.user_add, 'alex', 'standard', stdin='{"password": "abcd1"}\n')
        self.assertEqual(code, 1)
        self.assertEqual(self.run.calls[-1][0], ['userdel', '-r', '--', 'alex'])
        self.assertNotIn('abcd1', out)

    def test_remove_checks(self):
        people = self.a.people()
        check = self.a.user_remove_check
        self.assertIsNone(check('kid', 'keep-home', 'sam', people))
        self.assertIn('using', check('sam', 'keep-home', 'sam', people))
        self.assertIn('no such', check('daemon', 'keep-home', 'sam', people))
        self.assertIn('no such', check('root', 'delete-home', 'sam', people))
        self.assertIn('multi-seat', check('hearth-p2', 'keep-home', 'sam', people))
        self.assertIn('keep-home or delete-home', check('kid', 'everything', 'sam', people))
        self.assertIn('not a valid', check('../kid', 'keep-home', 'sam', people))
        # the last administrator stays
        self.assertIn('last administrator', check('sam', 'keep-home', 'kid', people))

    def test_remove_runs_userdel(self):
        def userdel(cmd):
            write(self.a.PASSWD_FILE, PASSWD.replace('kid:x:1001:1001:Kid:/var/home/kid:/bin/bash\n', ''))
            return 0, ''
        self.run.answers[('userdel',)] = userdel
        write(os.path.join(self.a.ACCOUNTS_DIR, 'icons', 'kid'), 'x')
        code, out = self.call(self.a.user_remove, 'kid', 'delete-home')
        self.assertEqual(code, 0, out)
        cmds = [c for c, _ in self.run.calls]
        self.assertEqual(cmds[0], ['loginctl', 'terminate-user', '--', 'kid'])
        self.assertEqual(cmds[1], ['userdel', '-r', '--', 'kid'])
        self.assertFalse(os.path.exists(os.path.join(self.a.ACCOUNTS_DIR, 'icons', 'kid')))

    def test_keep_home_doesnt_delete_files(self):
        def userdel(cmd):
            write(self.a.PASSWD_FILE, PASSWD.replace('kid:x:1001:1001:Kid:/var/home/kid:/bin/bash\n', ''))
            return 0, ''
        self.run.answers[('userdel',)] = userdel
        code, out = self.call(self.a.user_remove, 'kid', 'keep-home')
        self.assertEqual(code, 0, out)
        self.assertEqual(self.run.calls[1][0], ['userdel', '--', 'kid'])

    def test_remove_refusals_run_nothing(self):
        for name, mode in (('sam', 'keep-home'), ('root', 'keep-home'), ('kid', 'rm -rf'), ('hearth-p2', 'delete-home'), ('k id', 'keep-home')):
            self.run.calls.clear()
            code, out = self.call(self.a.user_remove, name, mode)
            self.assertEqual(code, 1, (name, out))
            self.assertEqual(self.run.calls, [])

    def test_picture_written_for_the_caller_only(self):
        png = b'\x89PNG\r\n\x1a\n' + b'0' * 100
        code, out = self.call(self.a.user_picture, stdin=png)
        self.assertEqual(code, 0, out)
        with open(os.path.join(self.a.ACCOUNTS_DIR, 'icons', 'sam'), 'rb') as f:
            self.assertEqual(f.read(), png)
        with open(os.path.join(self.a.ACCOUNTS_DIR, 'users', 'sam')) as f:
            self.assertIn('Icon=' + os.path.join(self.a.ACCOUNTS_DIR, 'icons', 'sam'), f.read())
        code, out = self.call(self.a.user_picture, stdin=b'GIF89a....')
        self.assertEqual(code, 1)
        self.assertIn('PNG or JPEG', out)
        code, out = self.call(self.a.user_picture, stdin=b'')
        self.assertEqual(code, 0)
        self.assertFalse(os.path.exists(os.path.join(self.a.ACCOUNTS_DIR, 'icons', 'sam')))

    def test_accounts_user_file_keeps_other_lines(self):
        text = '[User]\nLanguage=de_DE\nIcon=/old\nSystemAccount=false\n[Other]\nIcon=/keep\n'
        out = self.a.accounts_user_file(text, '/new')
        self.assertIn('Language=de_DE', out)
        self.assertIn('Icon=/new', out)
        self.assertNotIn('Icon=/old', out)
        self.assertIn('Icon=/keep', out)
        self.assertEqual(self.a.accounts_user_file('', '/p'), '[User]\nIcon=/p\n')
        self.assertNotIn('Icon=/old', self.a.accounts_user_file(text, None))


class DateTime(AdminBase):
    def test_timezone_must_be_listed(self):
        self.run.answers[('timedatectl', 'list-timezones')] = (0, 'Europe/Berlin\nAmerica/New_York\nUTC\n')
        self.assertTrue(self.a.valid_timezone('Europe/Berlin'))
        self.assertTrue(self.a.valid_timezone('UTC'))
        for z in ('Europe/Nowhere', '../etc/passwd', 'Europe/../../x', '', 'Europe/Berlin;reboot', '/Europe/Berlin', 5, None):
            self.assertFalse(self.a.valid_timezone(z), z)

    def test_timezone_sets_and_refuses(self):
        self.run.answers[('timedatectl', 'list-timezones')] = (0, 'Europe/Berlin\n')
        code, out = self.call(self.a.timezone, 'Europe/Berlin')
        self.assertEqual(code, 0, out)
        self.assertEqual(self.run.calls[-1][0], ['timedatectl', 'set-timezone', 'Europe/Berlin'])
        self.run.calls.clear()
        code, out = self.call(self.a.timezone, 'Mars/Base')
        self.assertEqual(code, 1)
        self.assertFalse(any(c[0][:2] == ['timedatectl', 'set-timezone'] for c in self.run.calls))

    def test_ntp_and_time(self):
        self.assertEqual(self.call(self.a.ntp, 'on')[0], 0)
        self.assertEqual(self.run.calls[-1][0], ['timedatectl', 'set-ntp', 'true'])
        self.assertEqual(self.call(self.a.ntp, 'yes')[0], 1)
        self.assertEqual(self.call(self.a.time_set, '2026-10-20 14:05')[0], 0)
        self.assertEqual(self.run.calls[-1][0], ['timedatectl', 'set-time', '2026-10-20 14:05:00'])
        for bad in ('2026-13-01 10:00', '2026-10-20 24:00', '2026-10-20', 'now', '2026-10-20 14:05; reboot', '1999-01-01 10:00'):
            self.run.calls.clear()
            self.assertEqual(self.call(self.a.time_set, bad)[0], 1, bad)
            self.assertEqual(self.run.calls, [])


LPINFO = '''Device: uri = dnssd://HP%20LaserJet%20M404._ipp._tcp.local/?uuid=1234
        class = network
        info = HP LaserJet M404
        make-and-model = HP LaserJet Pro M404-M405
        device-id = MFG:HP;MDL:LaserJet Pro M404;
        location = Office
Device: uri = socket
        class = network
        info = AppSocket/HP JetDirect
        make-and-model = Unknown
        device-id =
        location =
Device: uri = usb://Brother/HL-L2350DW%20series?serial=E7
        class = direct
        info = Brother HL-L2350DW series
        make-and-model = Brother HL-L2350DW series
        device-id = MFG:Brother;
        location =
'''


class Printers(AdminBase):
    def test_lpinfo_parsing_keeps_real_printers(self):
        d = self.a.parse_lpinfo_devices(LPINFO)
        self.assertEqual([x['uri'] for x in d], ['dnssd://HP%20LaserJet%20M404._ipp._tcp.local/?uuid=1234',
                                                'usb://Brother/HL-L2350DW%20series?serial=E7'])
        self.assertEqual(d[0]['makeModel'], 'HP LaserJet Pro M404-M405')
        self.assertEqual(d[0]['location'], 'Office')
        self.assertEqual(d[1]['cls'], 'direct')

    def test_printer_args(self):
        pa = self.a.printer_args
        self.assertEqual(pa(['HP_M404', 'ipp://192.168.1.20/ipp/print']), ('HP_M404', 'ipp://192.168.1.20/ipp/print', None))
        self.assertEqual(pa(['HP', 'ipps://[fe80::1]/ipp/print', '--info', 'HP in the office']), ('HP', 'ipps://[fe80::1]/ipp/print', 'HP in the office'))
        for bad in (['HP M404', 'ipp://x/ipp'], ['-p', 'ipp://x/ipp'], ['HP/1', 'ipp://x'], ['HP', 'file:///etc/shadow'],
                    ['HP', 'ipp://x/ipp print'], ['HP', 'ipp://x/"y'], ['HP', 'ipp://x', '--info'], ['HP', 'ipp://x', '--info', 'a\nb'],
                    ['HP', 'ipp://x', '--other', 'y'], ['HP'], [], ['HP', 'cups-pdf:/'], ['HP', 'ipp://x', '--info', '']):
            self.assertIsNone(pa(bad), bad)

    def test_add_tries_driverless_first(self):
        self.run.answers[('lpinfo', '-l', '-v')] = (0, LPINFO)
        code, out = self.call(self.a.printer_add, 'HP', 'dnssd://HP%20LaserJet%20M404._ipp._tcp.local/?uuid=1234', 'HP LaserJet')
        self.assertEqual(code, 0, out)
        lpadmin = next(c for c, _ in self.run.calls if c[0] == 'lpadmin')
        self.assertEqual(lpadmin[:7], ['lpadmin', '-p', 'HP', '-E', '-v', 'dnssd://HP%20LaserJet%20M404._ipp._tcp.local/?uuid=1234', '-m'])
        self.assertEqual(lpadmin[7], 'everywhere')
        self.assertEqual(lpadmin[lpadmin.index('-D') + 1], 'HP LaserJet')

    def test_usb_printer_gets_a_matching_driver(self):
        self.run.answers[('lpinfo', '-l', '-v')] = (0, LPINFO)
        self.run.answers[('lpinfo', '--make-and-model')] = (0, 'brlaser:0/cups/model/brother.ppd Brother HL-L2350DW, using brlaser\n')
        code, out = self.call(self.a.printer_add, 'Brother', 'usb://Brother/HL-L2350DW%20series?serial=E7', None)
        self.assertEqual(code, 0, out)
        lpadmin = next(c for c, _ in self.run.calls if c[0] == 'lpadmin')
        self.assertEqual(lpadmin[lpadmin.index('-m') + 1], 'brlaser:0/cups/model/brother.ppd')

    def test_add_reports_when_nothing_works(self):
        self.run.answers[('lpinfo',)] = (0, '')
        self.run.answers[('lpadmin',)] = (1, 'lpadmin: Unable to connect')
        code, out = self.call(self.a.printer_add, 'HP', 'ipp://10.0.0.5/ipp/print', None)
        self.assertEqual(code, 1)
        self.assertIn('Unable to connect', out)

    def test_remove_and_default_need_an_existing_printer(self):
        self.run.answers[('lpstat', '-v')] = (0, 'device for HP: ipp://x/ipp/print\n')
        self.assertEqual(self.call(self.a.printer_default, 'HP')[0], 0)
        self.assertEqual(self.run.calls[-1][0], ['lpadmin', '-d', 'HP'])
        self.assertEqual(self.call(self.a.printer_remove, 'HP')[0], 0)
        self.assertEqual(self.run.calls[-1][0], ['lpadmin', '-x', 'HP'])
        for name in ('Other', '-x', 'H P', ''):
            self.run.calls.clear()
            self.assertEqual(self.call(self.a.printer_remove, name)[0], 1, name)
            self.assertFalse(any(c[0][0] == 'lpadmin' for c in self.run.calls))

    def test_main_rejects_bad_usage(self):
        with mock.patch.object(self.a.os, 'geteuid', return_value=0):
            for argv in (['x', 'user-add', 'alex'], ['x', 'user-remove', 'kid'], ['x', 'printer-add', 'HP M', 'ipp://x'],
                         ['x', 'timezone'], ['x', 'user-picture', 'extra']):
                self.run.calls.clear()
                code, out = self.call(self.a.main, argv)
                self.assertEqual(code, 1, argv)
                self.assertIn('usage', out)
                self.assertEqual(self.run.calls, [])


# ---------------------------------------------------------------------- hearth-system
class SystemHelper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = mock.patch.dict(os.environ, {'HOME': self.tmp, 'XDG_CONFIG_HOME': os.path.join(self.tmp, '.config'),
                                                'XDG_RUNTIME_DIR': os.path.join(self.tmp, 'run')})
        self.env.start()
        self.s = load(SHELL / 'hearth-system', 'hearth_system')

    def tearDown(self):
        self.env.stop()

    def test_timedatectl(self):
        st = self.s.parse_timedatectl('Timezone=Europe/Berlin\nLocalRTC=no\nCanNTP=yes\nNTP=yes\nNTPSynchronized=no\n')
        self.assertEqual(st, {'timezone': 'Europe/Berlin', 'ntp': True, 'synced': False, 'canNtp': True})

    def test_timezones_grouped_by_region_and_city(self):
        import datetime
        when = datetime.datetime(2026, 1, 15, 12, tzinfo=datetime.timezone.utc)
        z = self.s.parse_timezones('America/Argentina/Buenos_Aires\nEurope/Berlin\nUTC\nAmerica/New_York\nbad zone\n../x\n', when)
        self.assertEqual([x['zone'] for x in z], ['America/Argentina/Buenos_Aires', 'America/New_York', 'Europe/Berlin', 'UTC'])
        self.assertEqual(z[0]['city'], 'Argentina / Buenos Aires')
        self.assertEqual(z[1]['region'], 'America')
        self.assertEqual(z[-1]['region'], 'Other')
        if z[2]['offset']:                       # needs the system's zone files
            self.assertEqual(z[2]['offset'], '+01:00')
            self.assertEqual(z[1]['offset'], '-05:00')

    def test_xkb_list(self):
        text = '! model\n  pc105  Generic 105-key PC\n\n! layout\n  us              English (US)\n  de              German\n\n' \
               '! variant\n  nodeadkeys      de: German (no dead keys)\n  intl            us: English (US, intl., with dead keys)\n\n' \
               '! option\n  grp             Switching to another layout\n'
        l = self.s.parse_xkb_lst(text)
        self.assertEqual([(x['layout'], x['variant']) for x in l], [('us', ''), ('us', 'intl'), ('de', ''), ('de', 'nodeadkeys')])
        self.assertEqual(l[3]['name'], 'German (no dead keys)')

    def test_keyboard_request_checks(self):
        known = {('us', ''), ('de', ''), ('de', 'nodeadkeys')}
        lays, sw, d, r = self.s.keyboard_request({'layouts': [{'layout': 'de', 'variant': 'nodeadkeys'}, {'layout': 'us'}, {'layout': 'us'}],
                                                  'switch': 'grp:alt_shift_toggle', 'delay': 400, 'rate': 30}, known)
        self.assertEqual(lays, [{'layout': 'de', 'variant': 'nodeadkeys'}, {'layout': 'us', 'variant': ''}])
        self.assertEqual((sw, d, r), ('grp:alt_shift_toggle', 400, 30))
        bad = [{'layouts': []}, {'layouts': [{'layout': 'xx'}]}, {'layouts': [{'layout': 'us;rm'}]}, {'layouts': [{'layout': 'us'}] * 5},
               {'layouts': [{'layout': 'us'}], 'switch': 'grp:evil'}, {'layouts': [{'layout': 'us'}], 'delay': 5},
               {'layouts': [{'layout': 'us'}], 'rate': 500}, {'layouts': [{'layout': 'us', 'variant': 'a b'}]}, [], 'x']
        for b in bad:
            with self.assertRaises(self.s.Fail, msg=b):
                self.s.keyboard_request(b, known | {('xx', '')} - {('xx', '')})

    def test_environment_file_lines(self):
        old = 'FOO=1\nXKB_DEFAULT_LAYOUT=us\n# note\nXKB_DEFAULT_LAYOUT=fr\n'
        vals = self.s.xkb_values([{'layout': 'de', 'variant': 'nodeadkeys'}, {'layout': 'us', 'variant': ''}],
                                 'grp:alt_shift_toggle', 'caps:escape,grp:ctrl_shift_toggle')
        self.assertEqual(vals, {'XKB_DEFAULT_LAYOUT': 'de,us', 'XKB_DEFAULT_VARIANT': 'nodeadkeys,',
                                'XKB_DEFAULT_OPTIONS': 'caps:escape,grp:alt_shift_toggle'})
        text = self.s.env_text(old, vals)
        self.assertEqual(text.count('XKB_DEFAULT_LAYOUT='), 1)
        self.assertIn('FOO=1', text)
        self.assertIn('XKB_DEFAULT_VARIANT=nodeadkeys,', text)
        env = self.s.parse_env(text)
        self.assertEqual(self.s.layouts_from_env(env), [{'layout': 'de', 'variant': 'nodeadkeys'}, {'layout': 'us', 'variant': ''}])
        # one layout: no switch shortcut, no variant
        one = self.s.xkb_values([{'layout': 'us', 'variant': ''}], 'grp:alt_shift_toggle', '')
        self.assertEqual(one, {'XKB_DEFAULT_LAYOUT': 'us', 'XKB_DEFAULT_VARIANT': '', 'XKB_DEFAULT_OPTIONS': ''})

    def test_repeat_in_rc_xml(self):
        rc = (LABWC / 'rc.xml').read_text()
        out = self.s.rc_with_repeat(rc, 400, 33)
        import xml.etree.ElementTree as ET
        kb = ET.fromstring(out).find('keyboard')
        self.assertEqual(kb.find('repeatRate').text, '33')
        self.assertEqual(kb.find('repeatDelay').text, '400')
        self.assertEqual(len(kb.findall('keybind')), len(ET.fromstring(rc).find('keyboard').findall('keybind')))
        again = self.s.rc_with_repeat(out, 500, 20)
        self.assertEqual(again.count('<repeatRate>'), 1)
        self.assertIn('<repeatDelay>500</repeatDelay>', again)
        bare = self.s.rc_with_repeat('<labwc_config>\n</labwc_config>\n', 600, 25)
        self.assertEqual(ET.fromstring(bare).find('keyboard/repeatRate').text, '25')

    def test_keyboard_set_writes_files_and_labwc_folder(self):
        self.s.keyboard_layouts = lambda: {'layouts': [{'layout': 'us', 'variant': '', 'name': 'English (US)'},
                                                       {'layout': 'de', 'variant': '', 'name': 'German'}]}
        os.makedirs(self.s.runtime_dir())
        src = os.path.join(self.tmp, 'share-labwc')
        write(os.path.join(src, 'rc.xml'), (LABWC / 'rc.xml').read_text())
        write(os.path.join(src, 'environment'), 'XKB_DEFAULT_LAYOUT=us\nMOZ_ENABLE_WAYLAND=1\n')
        write(os.path.join(src, 'autostart'), '#!/bin/sh\n')
        self.s.LABWC_DIR = src
        with mock.patch.dict(os.environ, {'LABWC_PID': ''}):
            r = self.s.keyboard_set(json.dumps({'layouts': [{'layout': 'de'}, {'layout': 'us'}], 'switch': 'grp:alt_shift_toggle', 'delay': 300, 'rate': 40}))
        self.assertTrue(r['ok'])
        env = self.s.parse_env(pathlib.Path(self.s.LABWC_ENV).read_text())
        self.assertEqual(env['XKB_DEFAULT_LAYOUT'], 'de,us')
        dest = self.s.runtime_dir()
        self.assertTrue(os.path.islink(os.path.join(dest, 'autostart')))
        denv = pathlib.Path(dest, 'environment').read_text()
        self.assertIn('MOZ_ENABLE_WAYLAND=1', denv)
        self.assertTrue(denv.rstrip().endswith('XKB_DEFAULT_VARIANT='), denv)   # yours come last, so they win
        self.assertIn('<repeatDelay>300</repeatDelay>', pathlib.Path(dest, 'rc.xml').read_text())
        st = self.s.keyboard_status()
        self.assertEqual([l['layout'] for l in st['layouts']], ['de', 'us'])
        self.assertEqual((st['delay'], st['rate'], st['switch']), (300, 40, 'grp:alt_shift_toggle'))

    def test_keybinds_in_plain_words(self):
        binds = self.s.parse_keybinds((LABWC / 'rc.xml').read_text())
        by = {b['key']: b for b in binds}
        self.assertEqual(by['W-e']['keys'], 'Super+E')
        self.assertEqual(by['W-e']['action'], 'Open Files')
        self.assertEqual(by['C-S-Escape']['keys'], 'Ctrl+Shift+Esc')
        self.assertEqual(by['W-Left']['action'], 'Snap the window to the left half')
        self.assertIn('A-Tab', by)                          # labwc's own, kept by <default />
        xml = '''<labwc_config><keyboard>
          <keybind key="W-1"><action name="GoToDesktop" to="1" /></keybind>
          <keybind key="C-A-t"><action name="Execute"><command>foot</command></action></keybind>
          <keybind key="A-Tab"><action name="NextWindow" /></keybind>
          <keybind key="W-v"><action name="Execute" command="sh -c 'quickshell ipc -p x call clipboard toggle'" /></keybind>
        </keyboard></labwc_config>'''
        b = {x['key']: x for x in self.s.parse_keybinds(xml)}
        self.assertEqual(b['W-1']['action'], 'Go to desktop 1')
        self.assertEqual(b['C-A-t']['action'], 'Run foot')
        self.assertEqual(b['C-A-t']['keys'], 'Ctrl+Alt+T')
        self.assertEqual(b['W-v']['action'], 'Clipboard history')
        self.assertEqual(self.s.parse_keybinds('not xml'), [])

    def test_mimeapps_defaults(self):
        old = '[Default Applications]\ntext/plain=gedit.desktop;\nimage/png=old.desktop\n\n[Added Associations]\nimage/png=old.desktop;eog.desktop;\n' \
              '[Removed Associations]\nimage/png=eog.desktop;x.desktop;\n'
        out = self.s.mimeapps_with(old, ['image/png', 'image/jpeg'], 'eog.desktop')
        self.assertIn('text/plain=gedit.desktop;', out)
        self.assertIn('image/png=eog.desktop;', out)
        self.assertIn('image/jpeg=eog.desktop;', out)
        self.assertNotIn('image/png=old.desktop\n', out)
        self.assertIn('image/png=eog.desktop;old.desktop;', out)       # added association first
        self.assertIn('image/png=x.desktop;', out)                    # no longer "removed"
        self.assertEqual(self.s.mimeapps_with('', ['text/plain'], 'a.desktop'),
                         '[Default Applications]\ntext/plain=a.desktop;\n\n[Added Associations]\ntext/plain=a.desktop;\n')

    def test_defaults_from_desktop_files(self):
        data = os.path.join(self.tmp, 'data')
        apps = os.path.join(data, 'applications')
        write(os.path.join(apps, 'eog.desktop'), '[Desktop Entry]\nType=Application\nName=Image Viewer\nExec=eog %U\nMimeType=image/png;image/jpeg;\n')
        write(os.path.join(apps, 'gimp.desktop'), '[Desktop Entry]\nType=Application\nName=GIMP\nExec=gimp %U\nMimeType=image/png;\n')
        write(os.path.join(apps, 'hidden.desktop'), '[Desktop Entry]\nType=Application\nName=Hidden\nExec=h\nNoDisplay=true\nMimeType=image/png;\n')
        write(os.path.join(apps, 'firefox.desktop'), '[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\nMimeType=text/html;x-scheme-handler/https;\n')
        with mock.patch.dict(os.environ, {'XDG_DATA_HOME': data, 'XDG_DATA_DIRS': os.path.join(self.tmp, 'none'),
                                          'XDG_CONFIG_DIRS': os.path.join(self.tmp, 'none'), 'XDG_CURRENT_DESKTOP': 'Nimbus:wlroots',
                                          'PATH': '/nonexistent'}):
            hf = self.s.load_files_helper()
            d = {k['kind']: k for k in self.s.defaults(hf)['kinds']}
            self.assertEqual([o['id'] for o in d['image']['options']], ['gimp.desktop', 'eog.desktop'])
            self.assertEqual([o['id'] for o in d['browser']['options']], ['firefox.desktop'])
            self.assertIsNone(d['image']['current'])
            r = self.s.default_set('image', 'eog.desktop', hf)
            self.assertTrue(r['ok'])
            d = {k['kind']: k for k in self.s.defaults(hf)['kinds']}
            self.assertEqual(d['image']['current'], 'eog.desktop')
            for kind, app in (('image', 'hidden.desktop.x'), ('image', '../x.desktop'), ('image', 'missing.desktop'), ('nope', 'eog.desktop')):
                with self.assertRaises(self.s.Fail, msg=(kind, app)):
                    self.s.default_set(kind, app, hf)

    def test_people(self):
        p = self.s.parse_people(PASSWD, GROUP)
        self.assertEqual([x['name'] for x in p], ['sam', 'kid', 'hearth-p2'])
        self.assertTrue(p[0]['admin'])

    def test_lpstat(self):
        v = 'device for HP_M404: dnssd://HP._ipp._tcp.local/\ndevice for Brother: usb://Brother/HL?serial=1\n'
        d = 'system default destination: Brother\n'
        p = ('printer Brother disabled since Mon 19 Oct 2026 10:00:00 -\n\tPaused\n\tForm mounted:\n\tDescription: Brother HL-L2350DW\n'
             '\tLocation: Hall\n\tAlerts: none\nprinter HP_M404 is idle.  enabled since Mon 19 Oct 2026\n\tForm mounted:\n'
             '\tDescription: HP LaserJet M404\n\tLocation:\n')
        out = self.s.parse_lpstat(v, d, p)
        by = {x['name']: x for x in out}
        self.assertEqual([x['name'] for x in out], ['HP_M404', 'Brother'])
        self.assertTrue(by['Brother']['default'])
        self.assertFalse(by['HP_M404']['default'])
        self.assertEqual(by['Brother']['state'], 'disabled')
        self.assertEqual(by['Brother']['reason'], 'Paused')
        self.assertEqual(by['Brother']['location'], 'Hall')
        self.assertEqual(by['HP_M404']['info'], 'HP LaserJet M404')
        self.assertEqual(by['HP_M404']['state'], 'idle')
        self.assertEqual(self.s.parse_lpstat('', 'no system default destination\n', ''), [])

    def test_printer_test_name_checked(self):
        for bad in ('', '-d', 'a b', 'x;y'):
            with self.assertRaises(self.s.Fail):
                self.s.printer_test(bad)

    def test_picture_must_be_an_image(self):
        bad = os.path.join(self.tmp, 'x.png')
        write(bad, 'not a picture')
        with self.assertRaises(self.s.Fail):
            self.s.picture_set(bad)
        good = os.path.join(self.tmp, 'y.png')
        with open(good, 'wb') as f:
            f.write(b'\x89PNG\r\n\x1a\n1234')
        self.s.ADMIN = os.path.join(self.tmp, 'no-admin')
        r = self.s.picture_set(good)
        self.assertEqual(r['picture'], os.path.join(self.tmp, '.face'))
        self.assertFalse(r['loginScreen'])


# ---------------------------------------------------------------------- hearth-apps
APPSTREAM = b'''<?xml version="1.0" encoding="UTF-8"?>
<components version="0.8" origin="flathub">
  <component type="desktop-application">
    <id>org.gimp.GIMP.desktop</id>
    <name>GNU Image Manipulation Program</name>
    <name xml:lang="de">GNU-Bildbearbeitungsprogramm</name>
    <summary>Create images and edit photographs</summary>
    <summary xml:lang="de">Bilder erstellen</summary>
    <description><p>GIMP is an image editor.</p><p xml:lang="de">Deutsch</p><ul><li>Layers</li><li>Filters</li></ul></description>
    <icon type="cached" width="128" height="128">org.gimp.GIMP.png</icon>
    <icon type="remote" width="128" height="128">https://dl.flathub.org/media/gimp.png</icon>
    <categories><category>Graphics</category><category>2DGraphics</category></categories>
    <keywords><keyword>photoshop</keyword></keywords>
    <developer_name>The GIMP team</developer_name>
    <project_license>GPL-3.0+</project_license>
    <url type="homepage">https://www.gimp.org/</url>
    <screenshots><screenshot type="default">
      <image type="source" width="1920" height="1080">https://dl.flathub.org/s/source.png</image>
      <image type="thumbnail" width="624" height="351">https://dl.flathub.org/s/624.png</image>
      <image type="thumbnail" width="112" height="63">https://dl.flathub.org/s/112.png</image>
    </screenshot></screenshots>
    <releases><release version="2.10.38" timestamp="1700000000"/></releases>
    <bundle type="flatpak" runtime="org.gnome.Platform/x86_64/46">app/org.gimp.GIMP/x86_64/stable</bundle>
    <custom><value key="flathub::verification::verified">true</value></custom>
  </component>
  <component type="runtime"><id>org.gnome.Platform</id><name>GNOME</name></component>
  <component type="desktop">
    <id>org.videolan.VLC</id>
    <name>VLC</name>
    <summary>VLC media player</summary>
    <icon type="remote">https://dl.flathub.org/media/vlc.png</icon>
    <categories><category>AudioVideo</category></categories>
  </component>
</components>
'''


class AppsHelper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = mock.patch.dict(os.environ, {'HOME': self.tmp, 'XDG_DATA_HOME': os.path.join(self.tmp, 'data'),
                                                'XDG_CACHE_HOME': os.path.join(self.tmp, 'cache')})
        self.env.start()
        self.h = load(SHELL / 'hearth-apps', 'hearth_apps')

    def tearDown(self):
        self.env.stop()

    def test_appstream(self):
        icon_dir = os.path.join(self.tmp, 'as')
        write(os.path.join(icon_dir, 'icons', '128x128', 'org.gimp.GIMP.png'), 'png')
        apps, details = self.h.parse_appstream(io.BytesIO(APPSTREAM), icon_dir)
        self.assertEqual(sorted(apps), ['org.gimp.GIMP', 'org.videolan.VLC'])
        g = apps['org.gimp.GIMP']
        self.assertEqual(g['name'], 'GNU Image Manipulation Program')
        self.assertEqual(g['summary'], 'Create images and edit photographs')
        self.assertEqual(g['icon'], os.path.join(icon_dir, 'icons', '128x128', 'org.gimp.GIMP.png'))
        self.assertEqual(g['categories'], ['Graphics', '2DGraphics'])
        self.assertEqual(g['developer'], 'The GIMP team')
        self.assertEqual(apps['org.videolan.VLC']['icon'], 'https://dl.flathub.org/media/vlc.png')
        d = details['org.gimp.GIMP']
        self.assertEqual(d['description'], 'GIMP is an image editor.\n\n• Layers\n• Filters')
        self.assertEqual(d['screenshots'], ['https://dl.flathub.org/s/624.png'])
        self.assertEqual((d['version'], d['license'], d['homepage'], d['verified']), ('2.10.38', 'GPL-3.0+', 'https://www.gimp.org/', True))

    def test_catalog_cache_from_users_appstream(self):
        d = os.path.join(self.tmp, 'data', 'flatpak', 'appstream', 'flathub', self.h.arch(), 'active')
        import gzip
        os.makedirs(d)
        with gzip.open(os.path.join(d, 'appstream.xml.gz'), 'wb') as f:
            f.write(APPSTREAM)
        apps, det = self.h.catalog()
        self.assertIn('org.videolan.VLC', apps)
        self.assertTrue(os.path.exists(det))
        apps2, _ = self.h.catalog()                  # from the cache now
        self.assertEqual(apps, apps2)
        r = self.h.search('photo edit', apps)
        self.assertEqual([a['id'] for a in r['apps']], ['org.gimp.GIMP'])
        self.assertEqual(self.h.search('photoshop', apps)['apps'][0]['id'], 'org.gimp.GIMP')
        self.assertEqual(self.h.search('', apps)['apps'], [])

    def test_columns(self):
        text = 'org.gimp.GIMP\tGIMP\t2.10.38\tuser\t320.1 MB\tflathub\ncom.valvesoftware.Steam\tSteam\t1.0.0.81\tsystem\t12 MB\tflathub\n' \
               'Application ID\tName\n\nnot-an-id\tx\n'
        rows = self.h.parse_columns(text, ['id', 'name', 'version', 'installation', 'size', 'origin'])
        self.assertEqual([r['id'] for r in rows], ['org.gimp.GIMP', 'com.valvesoftware.Steam'])
        self.assertEqual(rows[1]['installation'], 'system')
        self.assertEqual(rows[0]['size'], '320.1 MB')
        short = self.h.parse_columns('org.gimp.GIMP\t2.10\n', ['id', 'version', 'origin'])
        self.assertEqual(short[0]['origin'], '')

    def test_progress_lines(self):
        self.assertEqual(self.h.progress_of('Installing 2/3… ████▌     45%  1.2 MB/s  00:12'), (2, 3, 45))
        self.assertEqual(self.h.progress_of('Updating app/org.gimp.GIMP/x86_64/stable'), (None, None, None))
        self.assertEqual(self.h.progress_of('Installing runtime/org.freedesktop.Platform/x86_64/24.08 100%'), (None, None, 100))
        self.assertEqual(self.h.progress_of(' 1/1 Installing… 7%'), (1, 1, 7))

    def test_featured_from_api(self):
        self.assertEqual(self.h.featured_from_api({'hits': [{'app_id': 'org.a.B'}, {'app_id': 'bad'}, {'id': 'org.c.D'}, {'app_id': 'org.a.B'}]}),
                         ['org.a.B', 'org.c.D'])
        self.assertEqual(self.h.featured_from_api(['org.a.B', 7, 'x']), ['org.a.B'])
        self.assertEqual(self.h.featured_from_api(None), [])

    def test_ids_checked(self):
        for bad in ('', 'org', 'org.foo', 'org.foo.Bar;rm', '--user', '../org.a.b', 'org.a.b c'):
            with self.assertRaises(self.h.Fail, msg=bad):
                self.h.check_id(bad)
        self.assertEqual(self.h.check_id('com.valvesoftware.Steam'), 'com.valvesoftware.Steam')

    def test_main_usage(self):
        out = io.StringIO()
        with redirect_stdout(out), self.assertRaises(SystemExit):
            self.h.main(['x', 'remove', 'org.a.B', '--everything'])
        self.assertIn('usage', out.getvalue())


# ---------------------------------------------------------------------- hearth-clipboard
class ClipboardHelper(unittest.TestCase):
    def setUp(self):
        self.c = load(SHELL / 'hearth-clipboard', 'hearth_clipboard')

    def test_cliphist_list(self):
        text = '12\thello world\n11\t[[ binary data 12 KiB png 800x600 ]]\n10\t[[ binary data 3 KiB application/octet-stream ]]\nx\tbad\n9\tline\twith tab\n'
        items = self.c.parse_list(text)
        self.assertEqual([i['id'] for i in items], ['12', '11', '10', '9'])
        self.assertEqual(items[0], {'id': '12', 'kind': 'text', 'text': 'hello world'})
        self.assertEqual((items[1]['kind'], items[1]['format'], items[1]['dims'], items[1]['size']), ('image', 'png', '800x600', '12 KiB'))
        self.assertEqual(items[2]['kind'], 'binary')
        self.assertEqual(items[3]['text'], 'line\twith tab')
        self.assertEqual(len(self.c.parse_list('\n'.join('%d\tx' % i for i in range(100)))), self.c.LIMIT)

    def test_ids_checked(self):
        for bad in ('', 'abc', '1;2', '-1', '1 2'):
            with self.assertRaises(self.c.Fail):
                self.c.decode(bad)
            with self.assertRaises(self.c.Fail):
                self.c.delete(bad)

    def test_list_with_text_that_isnt_utf8(self):
        # One item copied from a Latin-1 file mustn't make the whole history fail to load.
        tmp = tempfile.mkdtemp(prefix='hearth-clip-test-')
        self.addCleanup(shutil.rmtree, tmp, True)
        for name, body in (('cliphist', "printf '2\\tcaf\\351\\n1\\tplain\\n'"), ('wl-copy', 'exit 0'), ('wl-paste', 'exit 0')):
            with open(os.path.join(tmp, name), 'w') as f:
                f.write('#!/bin/sh\n' + body + '\n')
            os.chmod(os.path.join(tmp, name), 0o755)
        self.c.CACHE = os.path.join(tmp, 'cache')
        old_path = os.environ.get('PATH', '')
        os.environ['PATH'] = tmp + os.pathsep + old_path
        self.addCleanup(os.environ.__setitem__, 'PATH', old_path)
        r = self.c.list_items()
        self.assertTrue(r['available'])
        self.assertEqual([i['id'] for i in r['items']], ['2', '1'])
        self.assertEqual(r['items'][0]['text'], 'caf\ufffd')

    def test_mime(self):
        self.assertEqual(self.c.mime_of(b'\x89PNG\r\n'), 'image/png')
        self.assertEqual(self.c.mime_of(b'RIFF1234WEBPxx'), 'image/webp')
        self.assertIsNone(self.c.mime_of(b'hello'))


if __name__ == '__main__':
    unittest.main()
