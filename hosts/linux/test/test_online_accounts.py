"""Online accounts (hearth_accounts.py, SetAccounts.qml, docs/ONLINE-ACCOUNTS.md), against fake servers on localhost
only: a CalDAV/CardDAV/WebDAV server, Nextcloud's Login Flow v2, an OAuth 2.0 server that checks PKCE, Microsoft
Graph's calendarView and an IMAP server. The keyring is a stand-in secret-tool that logs its command lines (a secret
must only ever arrive on its stdin). No real account is ever used.

Checked: adding each kind of account; wrong passwords; nothing secret in the accounts file, on a command line or in
`list`'s answer; https only (plain http just for this device); the switches; removing an account clears its keyring
item; no keyring = nothing saved; the account file kept safe (hearth_settings_io); events in the calendar widget's
shape (time zones, all-day, cancelled ones left out); fresh OAuth tokens; Files' mounts found under gvfs; the page."""
import sys
sys.dont_write_bytecode = True
import base64
import datetime as dt
import hashlib
import http.server
import json
import os
import pathlib
import re
import socketserver
import subprocess
import tempfile
import threading
import time
import unittest
import urllib.parse
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
import hearth_accounts as ha  # noqa: E402

USER, PASS = 'bob', 'pa$$-W0rd-never-in-files'
APP_PASS = 'nc-app-password-XYZ-123'
ACCESS, REFRESH = 'access-token-AAA', 'refresh-token-RRR'
NEW_ACCESS = 'access-token-BBB'

FAKE_SECRET_TOOL = r'''#!/usr/bin/env python3
import json, os, sys
db = os.environ['FAKE_KEYRING']
with open(db + '.argv', 'a') as f:
    f.write(json.dumps(sys.argv[1:]) + '\n')
try:
    items = json.load(open(db))
except Exception:
    items = {}
args = sys.argv[1:]
cmd = args[0]
attrs = [a for a in args[1:] if not a.startswith('--label=')]
key = json.dumps(attrs)
if os.environ.get('FAKE_KEYRING_LOCKED'):
    sys.exit(1)
if cmd == 'store':
    items[key] = sys.stdin.read()
elif cmd == 'lookup':
    if key not in items:
        sys.exit(1)
    sys.stdout.write(items[key]); sys.exit(0)
elif cmd == 'clear':
    items.pop(key, None)
json.dump(items, open(db, 'w'))
'''

FAKE_BROWSER = r'''#!/usr/bin/env python3
import sys, urllib.request
# A browser that "signs in": opens the page and follows its redirects (to Hearth's loopback for OAuth).
try:
    urllib.request.urlopen(sys.argv[1], timeout=20).read()
except Exception as e:
    open(__file__ + '.err', 'a').write(repr(e) + '\n')
'''


def ics(*events):
    return 'BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//fake//EN\r\n' + ''.join(events) + 'END:VCALENDAR\r\n'


def vevent(uid, summary, start, end=None, extra=''):
    return ('BEGIN:VEVENT\r\nUID:%s\r\nSUMMARY:%s\r\nDTSTART%s\r\n%s%sBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT10M\r\n'
            'END:VALARM\r\nEND:VEVENT\r\n') % (uid, summary, start, ('DTEND%s\r\n' % end) if end else '', extra)


def today(n=0):
    return dt.date.today() + dt.timedelta(days=n)


def utc_stamp(d, h, m=0):
    return dt.datetime(d.year, d.month, d.day, h, m, tzinfo=dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')


def xml_escape(s):
    return s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')


class Fake:
    """The state the fake servers share: who may sign in, what the browser did, what the token server saw."""
    def __init__(self):
        self.passwords = {USER: PASS}
        self.tokens = {ACCESS, NEW_ACCESS}
        self.flow_done = False
        self.polls = 0
        self.challenge = None
        self.codes = {}
        self.token_calls = []
        self.reports = []


def handler(fake):
    class H(http.server.BaseHTTPRequestHandler):
        protocol_version = 'HTTP/1.1'

        def log_message(self, *a):
            pass

        def body(self):
            n = int(self.headers.get('Content-Length') or 0)
            return self.rfile.read(n).decode() if n else ''

        def send(self, code, text='', ctype='application/xml; charset=utf-8', headers=None):
            data = text.encode()
            self.send_response(code)
            self.send_header('Content-Type', ctype)
            self.send_header('Content-Length', str(len(data)))
            for k, v in (headers or {}).items():
                self.send_header(k, v)
            self.end_headers()
            self.wfile.write(data)

        def authed(self):
            a = self.headers.get('Authorization') or ''
            if a.startswith('Bearer '):
                return a[7:] in fake.tokens
            if a.startswith('Basic '):
                u, _, p = base64.b64decode(a[6:]).decode().partition(':')
                return fake.passwords.get(u) == p
            return False

        def ms(self, responses):
            return ('<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" '
                    'xmlns:card="urn:ietf:params:xml:ns:carddav" xmlns:ical="http://apple.com/ns/ical/">%s</d:multistatus>') % ''.join(
                '<d:response><d:href>%s</d:href><d:propstat><d:prop>%s</d:prop><d:status>HTTP/1.1 200 OK</d:status>'
                '</d:propstat></d:response>' % r for r in responses)

        # ---- WebDAV / CalDAV / CardDAV (also under Nextcloud's /remote.php/dav)
        def do_PROPFIND(self):   # noqa: N802
            self.body()
            p = urllib.parse.urlsplit(self.path).path
            if p in ('/.well-known/caldav', '/.well-known/carddav'):
                return self.send(301, '', headers={'Location': '/remote.php/dav/'})
            if not self.authed():
                return self.send(401, '', headers={'WWW-Authenticate': 'Basic realm="fake"'})
            if p in ('/remote.php/dav/', '/caldav/v2/'):
                return self.send(207, self.ms([(p, '<d:current-user-principal><d:href>/remote.php/dav/principals/users/bob/</d:href></d:current-user-principal>')]))
            if p == '/remote.php/dav/principals/users/bob/':
                return self.send(207, self.ms([(p, '<c:calendar-home-set><d:href>/remote.php/dav/calendars/bob/</d:href></c:calendar-home-set>'
                                                   '<card:addressbook-home-set><d:href>/remote.php/dav/addressbooks/users/bob/</d:href></card:addressbook-home-set>')]))
            if p == '/remote.php/dav/calendars/bob/':
                return self.send(207, self.ms([
                    (p, '<d:resourcetype><d:collection/></d:resourcetype>'),
                    (p + 'personal/', '<d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>Personal</d:displayname>'
                                      '<ical:calendar-color>#0082C9FF</ical:calendar-color><c:supported-calendar-component-set><c:comp name="VEVENT"/></c:supported-calendar-component-set>'),
                    (p + 'tasks/', '<d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>Tasks</d:displayname>'
                                   '<c:supported-calendar-component-set><c:comp name="VTODO"/></c:supported-calendar-component-set>')]))
            if p == '/remote.php/dav/addressbooks/users/bob/':
                return self.send(207, self.ms([
                    (p, '<d:resourcetype><d:collection/></d:resourcetype>'),
                    (p + 'contacts/', '<d:resourcetype><d:collection/><card:addressbook/></d:resourcetype><d:displayname>Contacts</d:displayname>')]))
            if p.startswith('/remote.php/dav/files/bob/') or p == '/webdav/':
                return self.send(207, self.ms([(p, '<d:resourcetype><d:collection/></d:resourcetype>')]))
            self.send(404, '')

        def do_REPORT(self):     # noqa: N802
            q = self.body()
            if not self.authed():
                return self.send(401, '')
            fake.reports.append(q)
            d = today(1)
            cal = ics(
                vevent('e1', 'Dentist\\, 2nd floor', ':' + utc_stamp(d, 14), ':' + utc_stamp(d, 15), 'LOCATION:Smile Clinic\r\n'),
                vevent('e2', 'Holiday', ';VALUE=DATE:' + today(3).strftime('%Y%m%d'), ';VALUE=DATE:' + today(5).strftime('%Y%m%d')),
                vevent('e3', 'Berlin call', ';TZID=Europe/Berlin:' + today(2).strftime('%Y%m%d') + 'T090000', None, 'DURATION:PT30M\r\n'),
                vevent('e4', 'Cancelled thing', ':' + utc_stamp(d, 9), ':' + utc_stamp(d, 10), 'STATUS:CANCELLED\r\n'))
            # folded line, as servers send long ones
            cal = cal.replace('SUMMARY:Holiday', 'SUMMARY:Holi\r\n day')
            self.send(207, self.ms([('/remote.php/dav/calendars/bob/personal/x.ics',
                                     '<c:calendar-data>%s</c:calendar-data>' % xml_escape(cal))]))

        def do_GET(self):        # noqa: N802
            u = urllib.parse.urlsplit(self.path)
            q = urllib.parse.parse_qs(u.query)
            if u.path == '/login/v2/flow/abc':          # Nextcloud's sign-in page: the person signs in
                fake.flow_done = True
                return self.send(200, 'signed in', 'text/html')
            if u.path == '/oauth/authorize':
                assert q['code_challenge_method'] == ['S256'] and q['response_type'] == ['code'], q
                assert q['client_id'] == ['test-client'], q
                fake.challenge = q['code_challenge'][0]
                code = 'code-%d' % len(fake.codes)
                fake.codes[code] = q['redirect_uri'][0]
                loc = q['redirect_uri'][0] + '?' + urllib.parse.urlencode({'code': code, 'state': q['state'][0]})
                return self.send(302, '', 'text/html', {'Location': loc})
            if u.path == '/graph/me/calendarView':
                if not self.authed():
                    return self.send(401, '', 'application/json')
                d = today(1).isoformat()
                return self.send(200, json.dumps({'value': [
                    {'subject': 'Team sync', 'start': {'dateTime': d + 'T16:00:00.0000000'}, 'end': {'dateTime': d + 'T16:30:00.0000000'},
                     'isAllDay': False, 'location': {'displayName': 'Room 4'}, 'iCalUId': 'g1'},
                    {'subject': 'Gone', 'start': {'dateTime': d + 'T10:00:00'}, 'end': {'dateTime': d + 'T11:00:00'}, 'isCancelled': True}]}),
                    'application/json')
            self.send(404, '')

        def do_POST(self):       # noqa: N802
            b = self.body()
            u = urllib.parse.urlsplit(self.path)
            base = 'http://127.0.0.1:%d' % self.server.server_port
            if u.path == '/index.php/login/v2':
                return self.send(200, json.dumps({'poll': {'token': 'poll-token', 'endpoint': base + '/login/v2/poll'},
                                                  'login': base + '/login/v2/flow/abc'}), 'application/json')
            if u.path == '/login/v2/poll':
                fake.polls += 1
                assert urllib.parse.parse_qs(b)['token'] == ['poll-token']
                if not fake.flow_done:
                    return self.send(404, '', 'application/json')
                fake.passwords[USER] = APP_PASS
                return self.send(200, json.dumps({'server': base, 'loginName': USER, 'appPassword': APP_PASS}), 'application/json')
            if u.path == '/oauth/token':
                f = {k: v[0] for k, v in urllib.parse.parse_qs(b).items()}
                fake.token_calls.append(f)
                if f.get('client_id') != 'test-client':
                    return self.send(401, json.dumps({'error': 'invalid_client'}), 'application/json')
                if f['grant_type'] == 'authorization_code':
                    want = base64.urlsafe_b64encode(hashlib.sha256(f.get('code_verifier', '').encode()).digest()).rstrip(b'=').decode()
                    if f.get('code') not in fake.codes or want != fake.challenge or fake.codes[f['code']] != f.get('redirect_uri'):
                        return self.send(400, json.dumps({'error': 'invalid_grant'}), 'application/json')
                    claims = base64.urlsafe_b64encode(json.dumps({'email': 'bob@example.com'}).encode()).rstrip(b'=').decode()
                    return self.send(200, json.dumps({'access_token': ACCESS, 'refresh_token': REFRESH, 'expires_in': 3600,
                                                      'id_token': 'h.' + claims + '.s', 'scope': 'x'}), 'application/json')
                if f['grant_type'] == 'refresh_token':
                    if f.get('refresh_token') != REFRESH:
                        return self.send(400, json.dumps({'error': 'invalid_grant'}), 'application/json')
                    return self.send(200, json.dumps({'access_token': NEW_ACCESS, 'expires_in': 3600}), 'application/json')
            self.send(404, '')
    return H


class FakeImap(socketserver.StreamRequestHandler):
    def handle(self):
        self.wfile.write(b'* OK fake IMAP4rev1 ready\r\n')
        while True:
            line = self.rfile.readline()
            if not line:
                return
            tag, _, rest = line.decode().strip().partition(' ')
            cmd = rest.split(' ')[0].upper()
            if cmd == 'CAPABILITY':
                self.wfile.write(b'* CAPABILITY IMAP4rev1 AUTH=PLAIN\r\n' + tag.encode() + b' OK done\r\n')
            elif cmd == 'LOGIN':
                parts = re.findall(r'"((?:[^"\\]|\\.)*)"|(\S+)', rest[len('LOGIN '):])
                vals = [a.replace('\\"', '"').replace('\\\\', '\\') if a else b for a, b in parts]
                ok = vals[:2] == [USER, PASS]
                self.wfile.write(tag.encode() + (b' OK logged in\r\n' if ok else b' NO wrong\r\n'))
            elif cmd == 'LOGOUT':
                self.wfile.write(b'* BYE\r\n' + tag.encode() + b' OK bye\r\n')
                return
            else:
                self.wfile.write(tag.encode() + b' BAD\r\n')


class Base(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fake = Fake()
        cls.srv = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler(cls.fake))
        threading.Thread(target=cls.srv.serve_forever, daemon=True).start()
        cls.base = 'http://127.0.0.1:%d' % cls.srv.server_port
        socketserver.ThreadingTCPServer.allow_reuse_address = True
        cls.imap = socketserver.ThreadingTCPServer(('127.0.0.1', 0), FakeImap)
        threading.Thread(target=cls.imap.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.srv.shutdown()
        cls.imap.shutdown()

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        bin_ = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_)
        st = os.path.join(bin_, 'secret-tool')
        with open(st, 'w') as f:
            f.write(FAKE_SECRET_TOOL)
        os.chmod(st, 0o755)
        br = os.path.join(bin_, 'browser')
        with open(br, 'w') as f:
            f.write(FAKE_BROWSER)
        os.chmod(br, 0o755)
        self.keyring = os.path.join(self.tmp, 'keyring.json')
        self.file = os.path.join(self.tmp, 'config', 'nimbus', 'online-accounts.json')
        clients = os.path.join(self.tmp, 'clients.json')
        with open(clients, 'w') as f:
            json.dump({'providers': {
                'google': {'clientId': 'test-client', 'authUrl': self.base + '/oauth/authorize', 'tokenUrl': self.base + '/oauth/token',
                           'scopes': 'openid email cal', 'redirectHost': '127.0.0.1', 'caldav': self.base + '/caldav/v2/',
                           'carddav': self.base + '/remote.php/dav/', 'imapHost': 'imap.gmail.com'},
                'microsoft': {'clientId': 'test-client', 'authUrl': self.base + '/oauth/authorize', 'tokenUrl': self.base + '/oauth/token',
                              'scopes': 'openid email offline_access', 'mailScopes': 'imap-scope', 'redirectHost': 'localhost',
                              'graph': self.base + '/graph', 'imapHost': 'outlook.office365.com'}}}, f)
        self.env = {'HEARTH_SECRET_TOOL': st, 'FAKE_KEYRING': self.keyring, 'HEARTH_ACCOUNTS_FILE': self.file,
                    'HEARTH_ACCOUNTS_CLIENTS': clients, 'HEARTH_ACCOUNTS_OPEN': br, 'HEARTH_ACCOUNTS_POLL': '0.05',
                    'HEARTH_ACCOUNTS_OAUTH_TIMEOUT': '20', 'XDG_RUNTIME_DIR': os.path.join(self.tmp, 'run'),
                    'XDG_STATE_HOME': os.path.join(self.tmp, 'state'), 'HEARTH_INPUT': ''}
        os.makedirs(self.env['XDG_RUNTIME_DIR'])
        self.patch = mock.patch.dict(os.environ, self.env)
        self.patch.start()
        self.fake.passwords = {USER: PASS}
        self.fake.flow_done = False
        self.fake.token_calls.clear()

    def tearDown(self):
        self.patch.stop()

    def cli(self, *args, input_text=''):
        env = dict(os.environ, HEARTH_INPUT=input_text)
        r = subprocess.run([sys.executable, str(SHELL / 'hearth_accounts.py')] + list(args), input=input_text,
                           capture_output=True, text=True, env=env, timeout=60)
        lines = [l for l in r.stdout.splitlines() if l.startswith('{')]
        err = next((l[7:] for l in r.stdout.splitlines() if l.startswith('error: ')), '')
        return (json.loads(lines[-1]) if lines else None), err, r

    def secret_argv(self):
        try:
            return open(self.keyring + '.argv').read()
        except OSError:
            return ''

    def keyring_items(self):
        try:
            return json.load(open(self.keyring))
        except OSError:
            return {}

    def assert_no_secrets_in(self, text, where):
        for s in (PASS, APP_PASS, ACCESS, REFRESH, NEW_ACCESS, 'poll-token'):
            self.assertNotIn(s, text, '%s holds a secret' % where)

    def add_dav(self, pw=PASS):
        return self.cli('add-dav', input_text=json.dumps({'server': self.base + '/', 'user': USER, 'password': pw}))


class Dav(Base):
    def test_empty_list_and_providers(self):
        out, err, _ = self.cli('list')
        self.assertEqual(out, {'accounts': []})
        self.assertFalse(os.path.exists(self.file), 'listing wrote a settings file')
        with mock.patch.dict(os.environ, {'HEARTH_ACCOUNTS_CLIENTS': str(ROOT / 'os/system_files/usr/share/hearth/online-accounts/clients.json')}):
            p = ha.providers()
        self.assertFalse(p['google']['ready'])
        self.assertFalse(p['microsoft']['ready'])
        self.assertIn('/etc/hearth/online-accounts.json', p['google']['why'])
        with self.assertRaises(ha.Fail) as c, mock.patch.dict(os.environ, {'HEARTH_ACCOUNTS_CLIENTS': str(ROOT / 'os/system_files/usr/share/hearth/online-accounts/clients.json')}):
            ha.add_oauth('google')
        self.assertIn('client id', str(c.exception))

    def test_the_shipped_client_slot_is_empty(self):
        d = json.load(open(ROOT / 'os/system_files/usr/share/hearth/online-accounts/clients.json'))
        for name, p in d['providers'].items():
            self.assertEqual(p.get('clientId'), '', name)
            self.assertEqual(p.get('clientSecret', ''), '', name)
            for k in ('authUrl', 'tokenUrl'):
                self.assertTrue(p[k].startswith('https://'), (name, k))

    def test_add_caldav_carddav(self):
        out, err, r = self.add_dav()
        self.assertEqual(err, '', r.stdout)
        a = out['account']
        self.assertEqual(a['kind'], 'dav')
        self.assertEqual([c['name'] for c in a['calendars']], ['Personal'], 'the task list is left out')
        self.assertEqual(a['calendars'][0]['color'], '#0082C9')
        self.assertEqual([b['name'] for b in a['addressbooks']], ['Contacts'])
        self.assertEqual(a['use'], {'calendar': True, 'contacts': True, 'files': False, 'mail': False})
        # The password: only in the keyring, given on stdin, never in the file, on a command line or in an answer.
        self.assert_no_secrets_in(open(self.file).read(), 'the accounts file')
        self.assert_no_secrets_in(self.secret_argv(), "secret-tool's command line")
        self.assert_no_secrets_in(r.stdout + r.stderr, "add-dav's answer")
        listed, _, r2 = self.cli('list')
        self.assert_no_secrets_in(r2.stdout, "list's answer")
        self.assertEqual(len(listed['accounts']), 1)
        self.assertIn(PASS, ''.join(self.keyring_items().values()))
        # The same account again is refused.
        _, err2, _ = self.add_dav()
        self.assertIn('already here', err2)

    def test_wrong_password_saves_nothing(self):
        out, err, _ = self.add_dav('wrong')
        self.assertIsNone(out)
        self.assertIn('Wrong user name or password', err)
        self.assertFalse(os.path.exists(self.file))
        self.assertEqual(self.keyring_items(), {})

    def test_no_keyring_no_account(self):
        with mock.patch.dict(os.environ, {'HEARTH_SECRET_TOOL': os.path.join(self.tmp, 'missing-secret-tool')}):
            out, err, _ = self.add_dav()
        self.assertIsNone(out)
        self.assertIn('keyring', err)
        self.assertFalse(os.path.exists(self.file), 'an account was saved without its password in the keyring')
        with mock.patch.dict(os.environ, {'FAKE_KEYRING_LOCKED': '1'}):
            out, err, _ = self.add_dav()
        self.assertIn('keyring', err)
        self.assertEqual(ha.load(), [])

    def test_https_only(self):
        for bad in ('http://dav.example.com/', 'ftp://x/'):
            with self.assertRaises(ha.Fail):
                ha.check_url(bad)
        ha.check_url('https://dav.example.com/')
        ha.check_url('http://127.0.0.1:8080/')
        self.assertEqual(ha.normalize_server('cloud.example.com'), 'https://cloud.example.com/')
        with self.assertRaises(ha.Fail):
            ha.imap_connect('mail.example.com', 143, False)

    def test_events_in_the_widgets_shape(self):
        self.add_dav()
        out, err, r = self.cli('events', '--days', '10')
        self.assertEqual(err, '', r.stdout)
        self.assertEqual(out['errors'], [])
        ev = {e['uid']: e for e in out['events']}
        self.assertNotIn('e4', ev, 'a cancelled event is shown')
        d = ev['e1']
        self.assertEqual(d['title'], 'Dentist, 2nd floor')
        self.assertEqual(d['location'], 'Smile Clinic')
        start = dt.datetime.combine(today(1), dt.time(14), dt.timezone.utc)
        self.assertEqual(d['start'], int(start.timestamp() * 1000))
        self.assertEqual(d['end'] - d['start'], 3600 * 1000)
        self.assertEqual(d['date'], start.astimezone().date().isoformat())
        self.assertFalse(d['allDay'])
        self.assertEqual(d['color'], '#0082C9')
        self.assertEqual(d['calendar'], 'Personal')
        h = ev['e2']
        self.assertEqual(h['title'], 'Holiday', 'a folded line was not unfolded')
        self.assertTrue(h['allDay'])
        self.assertEqual((h['date'], h['lastDate']), (today(3).isoformat(), today(4).isoformat()), 'all-day end is exclusive')
        b = ev['e3']
        from zoneinfo import ZoneInfo
        bs = dt.datetime.combine(today(2), dt.time(9), ZoneInfo('Europe/Berlin'))
        self.assertEqual(b['start'], int(bs.timestamp() * 1000))
        self.assertEqual(b['end'] - b['start'], 30 * 60 * 1000, 'DURATION is used when there is no DTEND')
        for k in ('title', 'date', 'lastDate', 'allDay', 'start', 'end', 'location', 'color'):
            self.assertIn(k, d)          # what WidgetCalendar.qml and the clock panel read
        self.assertEqual(out['events'], sorted(out['events'], key=lambda e: (e['start'], e['title'])))
        self.assertIn('<c:expand', self.fake.reports[-1], 'recurring events are not asked for expanded')
        cals, _, _ = self.cli('calendars')
        self.assertEqual([c['name'] for c in cals['calendars']], ['Personal'])
        books, _, _ = self.cli('addressbooks')
        self.assertEqual([b['name'] for b in books['addressbooks']], ['Contacts'])

    def test_switches_and_remove(self):
        a = self.add_dav()[0]['account']
        out, err, _ = self.cli('use', a['id'], 'calendar', 'off')
        self.assertFalse(out['account']['use']['calendar'])
        self.assertFalse(ha.public(ha.find(a['id']))['use']['calendar'])
        ev, _, _ = self.cli('events')
        self.assertEqual(ev['events'], [], 'calendar is off but its events were read')
        _, err, _ = self.cli('use', a['id'], 'files', 'on')
        self.assertIn("doesn't offer", err)
        _, err, _ = self.cli('use', a['id'], 'nonsense', 'on')
        self.assertTrue(err)
        # Other keys in the file stay as they were (merge on write).
        sys_path_d = json.load(open(self.file))
        sys_path_d['somethingElse'] = 7
        json.dump(sys_path_d, open(self.file, 'w'))
        self.cli('use', a['id'], 'calendar', 'on')
        self.assertEqual(json.load(open(self.file))['somethingElse'], 7)
        self.assertTrue(os.path.exists(self.file + '.bak'))
        out, err, _ = self.cli('remove', a['id'])
        self.assertEqual(err, '')
        self.assertEqual(ha.load(), [])
        self.assertEqual(self.keyring_items(), {}, "the account's keyring item was left behind")

    def test_a_damaged_file_is_put_back_not_replaced(self):
        a = self.add_dav()[0]['account']
        self.cli('use', a['id'], 'contacts', 'off')        # a second write: the .bak has the account
        with open(self.file, 'w') as f:
            f.write('{"accounts": [')
        self.assertEqual([x['id'] for x in ha.load()], [a['id']], 'the damaged file was not put back from .bak')

    def test_webdav_and_imap(self):
        out, err, r = self.cli('add-webdav', input_text=json.dumps({'server': self.base + '/webdav/', 'user': USER, 'password': PASS}))
        self.assertEqual(err, '', r.stdout)
        self.assertTrue(out['account']['use']['files'])
        port = self.imap.server_address[1]
        out, err, r = self.cli('add-imap', input_text=json.dumps({'host': '127.0.0.1', 'port': port, 'ssl': False, 'user': USER, 'password': PASS}))
        self.assertEqual(err, '', r.stdout)
        self.assertEqual(out['account']['imap'], {'host': '127.0.0.1', 'port': port, 'ssl': False})
        self.assertTrue(out['account']['use']['mail'])
        _, err, _ = self.cli('add-imap', input_text=json.dumps({'host': '127.0.0.1', 'port': port, 'ssl': False, 'user': 'eve', 'password': 'x'}))
        self.assertIn('Wrong user name or password', err)
        cred, _, r = self.cli('credentials', out['account']['id'])
        self.assertEqual(cred, {'user': USER, 'password': PASS})
        self.assert_no_secrets_in(open(self.file).read(), 'the accounts file')


class Nextcloud(Base):
    def test_login_flow_v2_and_files(self):
        out, err, r = self.cli('add-nextcloud', self.base)
        self.assertEqual(err, '', r.stdout + r.stderr)
        a = out['account']
        self.assertEqual(a['kind'], 'nextcloud')
        self.assertEqual(a['user'], USER)
        self.assertEqual(a['use'], {'calendar': True, 'contacts': True, 'files': True, 'mail': False})
        self.assertTrue(self.fake.flow_done, 'the browser was not sent to the sign-in page')
        self.assertIn('progress: 30 Sign in to Nextcloud in your browser', r.stdout)
        self.assertIn(APP_PASS, ''.join(self.keyring_items().values()))
        self.assert_no_secrets_in(open(self.file).read() + self.secret_argv() + r.stdout, 'a file, command line or answer')
        full = ha.find(a['id'])
        self.assertEqual(full['files'], self.base + '/remote.php/dav/files/bob/')
        self.assertEqual(ha.dav_uri(full), 'dav://bob@127.0.0.1:%d/remote.php/dav/files/bob/' % self.srv.server_port)
        https = dict(full, files='https://cloud.example.com/remote.php/dav/files/bob/')
        self.assertEqual(ha.dav_uri(https), 'davs://bob@cloud.example.com/remote.php/dav/files/bob/')
        # Hearth Files' left menu: not connected yet, then found where gvfs mounts it.
        places = ha.file_places()
        self.assertEqual(len(places), 1)
        self.assertEqual((places[0]['kind'], places[0]['mounted'], places[0]['path']), ('cloud', False, ''))
        g = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'gvfs', 'dav:host=127.0.0.1,ssl=false,port=%d,user=bob,prefix=%%2Fremote.php%%2Fdav' % self.srv.server_port)
        os.makedirs(os.path.join(g, 'files', 'bob'))
        self.assertEqual(ha.mounted_path(full), os.path.join(g, 'files', 'bob'))
        self.assertEqual(ha.mount(a['id'])['path'], os.path.join(g, 'files', 'bob'))
        self.assertTrue(ha.file_places()[0]['mounted'])
        # Files off: gone from Files.
        self.cli('use', a['id'], 'files', 'off')
        self.assertEqual(ha.file_places(), [])
        with self.assertRaises(ha.Fail):
            ha.mount(a['id'])

    def test_mount_answers_gios_questions_from_the_keyring(self):
        a = self.cli('add-nextcloud', self.base)[0]['account']
        seen = {}

        def fake_gio(uri, user=None, password=None, timeout=45):
            seen.update(uri=uri, user=user, password=password)
            g = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'gvfs', 'dav:host=127.0.0.1,ssl=false,user=bob,prefix=%2Fremote.php%2Fdav%2Ffiles%2Fbob')
            os.makedirs(g)
            return True, ''
        import hearth_netshares
        with mock.patch.object(hearth_netshares, 'gio_mount', fake_gio):
            r = ha.mount(a['id'])
        self.assertEqual(seen['password'], APP_PASS)
        self.assertEqual(seen['user'], USER)
        self.assertTrue(seen['uri'].startswith('dav://bob@127.0.0.1'))
        self.assertTrue(r['path'].endswith('prefix=%2Fremote.php%2Fdav%2Ffiles%2Fbob'))

    def test_not_a_nextcloud(self):
        _, err, _ = self.cli('add-nextcloud', self.base + '/nothing-here/')
        self.assertIn("doesn't look like a Nextcloud", err)

    def test_hearth_files_lists_the_account(self):
        self.cli('add-nextcloud', self.base)
        src = (SHELL / 'hearth-files').read_text()
        self.assertIn('account_places()', src)
        self.assertIn("'account-open'", src)
        fm = (SHELL / 'FileManager.qml').read_text()
        self.assertIn('if (p.account && !p.path) return accountOpen(p);', fm)


class OAuth(Base):
    def test_google_pkce_loopback_and_fresh_tokens(self):
        out, err, r = self.cli('add-oauth', 'google')
        self.assertEqual(err, '', r.stdout + r.stderr)
        a = out['account']
        self.assertEqual((a['kind'], a['user']), ('google', 'bob@example.com'))
        self.assertEqual(a['use']['files'], False)
        first = self.fake.token_calls[0]
        self.assertEqual(first['grant_type'], 'authorization_code')
        self.assertTrue(first['redirect_uri'].startswith('http://127.0.0.1:'))
        self.assertNotIn('client_secret', first, 'a client secret was sent though none is set')
        self.assert_no_secrets_in(open(self.file).read() + self.secret_argv() + r.stdout, 'a file, command line or answer')
        tok, _, _ = self.cli('token', a['id'])
        self.assertEqual(tok['token'], ACCESS, 'a valid token was refreshed for nothing')
        # Nearly expired: refreshed with the kept refresh token, the new one kept.
        items = self.keyring_items()
        k = next(iter(items))
        s = json.loads(items[k]); s['expires'] = int(time.time()) + 30
        items[k] = json.dumps(s); json.dump(items, open(self.keyring, 'w'))
        tok, err, _ = self.cli('token', a['id'])
        self.assertEqual(err, '')
        self.assertEqual(tok['token'], NEW_ACCESS)
        self.assertEqual(json.loads(self.keyring_items()[k])['access'], NEW_ACCESS)
        # Its calendar through CalDAV with the token.
        ev, err, r = self.cli('events', '--days', '10')
        self.assertEqual(err, '', r.stdout)
        self.assertEqual(ev['errors'], [])
        self.assertIn('e1', [e['uid'] for e in ev['events']])

    def test_microsoft_graph_calendar_and_mail_scope(self):
        out, err, r = self.cli('add-oauth', 'microsoft')
        self.assertEqual(err, '', r.stdout + r.stderr)
        a = out['account']
        self.assertTrue(self.fake.token_calls[0]['redirect_uri'].startswith('http://localhost:'))
        self.assertEqual(a['imap']['host'], 'outlook.office365.com')
        ev, err, _ = self.cli('events', '--days', '5')
        self.assertEqual(err, '')
        self.assertEqual([e['title'] for e in ev['events']], ['Team sync'])
        self.assertEqual(ev['events'][0]['location'], 'Room 4')
        tok, _, _ = self.cli('token', a['id'], 'mail')
        self.assertEqual(self.fake.token_calls[-1].get('scope'), 'imap-scope')
        self.assertEqual(tok['token'], NEW_ACCESS)

    def test_a_wrong_state_is_refused(self):
        code, err, _ = ha.loopback_code('right-state', timeout=10, on_ready=lambda u: threading.Thread(
            target=lambda: __import__('urllib.request').request.urlopen(u + '?code=x&state=wrong', timeout=5).read(), daemon=True).start())
        self.assertIsNone(code)
        self.assertIn("didn't match", err)


class Page(unittest.TestCase):
    def test_settings_entry_and_page(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        self.assertRegex(reg, r'\{ id: "online-accounts", file: "SetAccounts.qml", cat: "accounts"')
        qml = (SHELL / 'SetAccounts.qml').read_text()
        self.assertIn('hearth_accounts.py', qml)
        # Passwords go to the helper on stdin (SetJob's input), never on a command line or into a settings file.
        self.assertNotRegex(qml, r'run\(\[[^\]]*(pw|password|Pass)[^\]]*\]')
        self.assertNotIn('writer.merge', qml)
        self.assertNotRegex(qml, r'\bSettings\.\w+\s*=(?!=)')
        for words in ('Nextcloud', 'Google', 'Microsoft', 'CalDAV', 'IMAP', 'Calendar', 'Contacts', 'Files', 'Mail'):
            self.assertIn(words, qml)
        self.assertIn('online-accounts:add-nextcloud', (ROOT / '.github' / 'workflows' / 'online-accounts.yml').read_text())

    def test_build_installs_the_helper(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('/usr/bin/hearth-accounts', b)


if __name__ == '__main__':
    unittest.main()
