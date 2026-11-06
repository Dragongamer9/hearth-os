#!/usr/bin/env python3
"""Hearth Friends: signing in on the server's own page (OAuth 2.0), end to end, against Synapse with the Matrix
Authentication Service (MAS). CI starts both on this machine, with a throwaway account made there
(.github/workflows/friends.yml); nothing here talks to a real server.

  HF_OAUTH_PASSWORD=... python3 tools/friends-oauth-e2e.py --bin PATH/hearth-friends --homeserver http://127.0.0.1:8018 --user dave

This script plays the phone and the browser with a small HTML form filler on MAS's own pages (/link, /login, the
consent pages):
 * what the server offers (authMethods): sign-in on its page, by phone (device grant), in the browser, sign-up there
 * phone (QR code): Hearth gets a device code; the "phone" opens the address, enters the code, signs in and allows
   Hearth; Hearth finishes by itself and syncs
 * the session comes back from the keyring after a restart; the access token is renewed (MAS gives 60 s tokens here)
   and the new one is kept
 * signing out revokes the session at the server
 * turning the sign-in down on the phone ends it with a message
 * "Create an account" in the browser opens MAS's sign-up page (prompt=create)
 * the browser on this device: authorization code + PKCE, the browser comes back to Hearth's loopback address and
   Hearth is signed in, no password typed into Hearth
 * nothing secret in the service's log

Needs a D-Bus session with an unlocked Secret Service (GNOME Keyring). Prints PASS lines; a failure prints FAIL and
exits 1.
"""
import argparse
import html.parser
import http.cookiejar
import json
import os
import re
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

ap = argparse.ArgumentParser()
ap.add_argument('--bin', required=True)
ap.add_argument('--homeserver', required=True)
ap.add_argument('--user', required=True)
A = ap.parse_args()
BIN = os.path.abspath(A.bin)
PW = os.environ.get('HF_OAUTH_PASSWORD') or ''
if not PW:
    print('FAIL HF_OAUTH_PASSWORD is not set')
    sys.exit(1)
SVC = None


def hide(s):
    """Addresses without their query (codes, states) for this script's own output."""
    return re.sub(r'(https?://[^\s?"\']+)\?[^\s"\']*', r'\1?[hidden]', str(s))


def fail(msg):
    print('FAIL ' + hide(msg), flush=True)
    if SVC:
        SVC.stop()
        try:
            print('--- service log (tail) ---')
            print(open(SVC.log_path).read()[-4000:])
            print('AUTHLOG ' + ' | '.join(l.split('hearth-friends: ', 1)[-1] for l in open(SVC.log_path).read().splitlines() if 'auth' in l or 'e2ee' in l or 'restored' in l)[-3000:])
        except OSError:
            pass
    sys.exit(1)


def ok(msg):
    print('PASS ' + msg, flush=True)


def wait(what, fn, secs=60, every=1.0):
    end = time.time() + secs
    last = None
    while time.time() < end:
        try:
            v = fn()
            if v:
                return v
            last = v
        except Exception as e:  # noqa: BLE001
            last = e
        time.sleep(every)
    fail('%s (waited %ds; last: %r)' % (what, secs, last))


class Service:
    def __init__(self, name):
        self.dir = tempfile.mkdtemp(prefix='friends-%s-' % name, dir=os.environ.get('RUNNER_TEMP') or None)
        self.home = os.path.join(self.dir, 'home')
        self.run = os.path.join(self.dir, 'run')
        os.makedirs(self.home)
        os.makedirs(self.run, mode=0o700)
        self.sock = os.path.join(self.run, 'hearth-friends.sock')
        self.log_path = os.path.join(self.dir, 'service.log')
        self.proc = None

    def env(self):
        e = dict(os.environ)
        e.pop('HF_OAUTH_PASSWORD', None)
        e.update(HOME=self.home, XDG_RUNTIME_DIR=self.run, XDG_CONFIG_HOME=os.path.join(self.home, '.config'),
                 XDG_DATA_HOME=os.path.join(self.home, '.local/share'), XDG_CACHE_HOME=os.path.join(self.home, '.cache'),
                 HEARTH_FRIENDS_SOCKET=self.sock, HEARTH_FRIENDS_BIN=BIN, BROWSER='true')
        return e

    def start(self):
        self.proc = subprocess.Popen([BIN, 'serve'], env=self.env(), stdout=open(self.log_path, 'a'), stderr=subprocess.STDOUT)
        wait('service listening', lambda: os.path.exists(self.sock), 20, 0.2)

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
            try:
                os.unlink(self.sock)
            except OSError:
                pass

    def call(self, method, args=None, timeout=90):
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(timeout)
        s.connect(self.sock)
        s.sendall((json.dumps({'id': 1, 'method': method, 'args': args or {}}) + '\n').encode())
        buf = b''
        while not buf.endswith(b'\n'):
            chunk = s.recv(1 << 20)
            if not chunk:
                break
            buf += chunk
        s.close()
        r = json.loads(buf)
        if not r.get('ok'):
            raise RuntimeError(r.get('error'))
        return r.get('value')

    def log(self):
        return open(self.log_path).read() if os.path.exists(self.log_path) else ''


class Forms(html.parser.HTMLParser):
    """The POST forms on a page: their fields, checkboxes and named buttons."""

    def __init__(self):
        super().__init__()
        self.forms = []
        self.cur = None

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == 'form':
            self.cur = {'method': (a.get('method') or 'get').lower(), 'action': a.get('action') or '', 'fields': {}, 'buttons': [], 'checkboxes': []}
            self.forms.append(self.cur)
        elif self.cur is not None and tag == 'input' and a.get('name'):
            t = (a.get('type') or 'text').lower()
            if t == 'checkbox':
                self.cur['checkboxes'].append(a['name'])
            elif t == 'submit':
                self.cur['buttons'].append((a['name'], a.get('value') or ''))
            else:
                self.cur['fields'][a['name']] = a.get('value') or ''
        elif self.cur is not None and tag == 'button' and a.get('name'):
            self.cur['buttons'].append((a['name'], a.get('value') or ''))

    def handle_endtag(self, tag):
        if tag == 'form':
            self.cur = None


class Browser:
    """A phone's or this computer's browser, enough for MAS's pages: cookies, redirects, forms."""

    def __init__(self, who):
        self.jar = http.cookiejar.CookieJar()
        self.op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.jar), urllib.request.ProxyHandler({}))
        self.who = who
        self.url, self.body, self.status = None, '', 0

    def open(self, url, data=None):
        body = urllib.parse.urlencode(data).encode() if data is not None else None
        req = urllib.request.Request(url, data=body, headers={'User-Agent': 'Mozilla/5.0 (HearthCI %s)' % self.who, 'Accept': 'text/html'})
        try:
            r = self.op.open(req, timeout=60)
        except urllib.error.HTTPError as e:
            r = e
        self.url = r.geturl()
        self.status = getattr(r, 'status', None) or getattr(r, 'code', 0)
        self.body = r.read().decode('utf-8', 'replace')
        return self

    def forms(self):
        p = Forms()
        p.feed(self.body)
        return [f for f in p.forms if f['method'] == 'post' and 'logout' not in f['action']]

    def has(self, pick):
        return any(pick(f) for f in self.forms())

    def submit(self, pick, fill):
        fs = [f for f in self.forms() if pick(f)]
        if not fs:
            raise RuntimeError('no such form on %s (status %s); forms: %r; text: %s' % (
                hide(self.url), self.status, [(sorted(f['fields']), f['buttons']) for f in self.forms()], re.sub(r'<[^>]+>|\s+', ' ', self.body)[:600]))
        f = fs[0]
        data = dict(f['fields'])
        for c in f['checkboxes']:
            data[c] = 'on'
        data.update(fill)
        return self.open(urllib.parse.urljoin(self.url, f['action'] or self.url), data)

    def login_if_asked(self):
        if self.has(lambda f: 'password' in f['fields']):
            self.submit(lambda f: 'password' in f['fields'], {'username': A.user, 'password': PW})
            if self.has(lambda f: 'password' in f['fields']):
                raise RuntimeError('MAS didn\'t take the CI account\'s password')


def phone(start, approve=True):
    """The phone: opens the address from the QR code, types the code if asked, signs in, allows (or turns down)."""
    b = Browser('phone')
    b.open(start['url'])
    if b.has(lambda f: 'code' in f['fields']):
        b.submit(lambda f: 'code' in f['fields'], {'code': start['userCode']})
    b.login_if_asked()
    action = 'consent' if approve else 'reject'
    b.submit(lambda f: ('action', action) in f['buttons'], {'action': action})
    return b


HS = A.homeserver
try:
    SVC = Service('oauth')
    SVC.start()

    # ---- what the server offers
    m = SVC.call('authMethods', {'homeserver': HS}, 60)
    if not (m.get('oauth') and m.get('qr') and m.get('browser') and m.get('create')):
        fail('the server\'s ways to sign in weren\'t found: %r' % m)
    if m.get('register') != 'oauth':
        fail('sign-up from Hearth should be on the server\'s page: %r' % m)
    ok('authMethods: sign-in on the server\'s page (%s): phone (device grant), browser, make an account there; password %s' % (m.get('issuer'), m.get('password')))

    # ---- phone: device authorization grant
    st = SVC.call('authStart', {'homeserver': HS, 'mode': 'qr'}, 90)
    if st.get('mode') != 'qr' or not st.get('userCode') or not st.get('shortUrl') or '<svg' not in (st.get('qr') or '') or not st.get('qrRows'):
        fail('no QR code / code / short address: %r' % {k: (v if k not in ('qr', 'qrRows') else bool(v)) for k, v in st.items()})
    if (SVC.call('status').get('auth') or {}).get('mode') != 'qr':
        fail('status doesn\'t show the waiting sign-in')
    ok('phone: QR code, short address %s and a %d-character code, valid %d s' % (st['shortUrl'], len(st['userCode']), st.get('expiresIn', 0)))
    phone(st)
    s = wait('Hearth is signed in after the phone said yes', lambda: SVC.call('status').get('signedIn') and SVC.call('status'), 60)
    if s.get('signIn') != 'oauth' or not s['userId'].startswith('@%s:' % A.user):
        fail('signed in, but not as expected: %r' % s)
    ok('phone: signed in as %s (device %s) without typing anything into Hearth' % (s['userId'], s.get('deviceId')))
    wait('first sync', lambda: SVC.call('status').get('synced'), 60)
    if not s.get('keyringOk'):
        fail('the OAuth session wasn\'t kept in the keyring')
    if not s.get('accountUrl'):
        fail('no account page from the server: %r' % s)
    ok('synced; session in the Secret Service; the server\'s account page is offered (%s)' % hide(s['accountUrl']))
    rec = SVC.call('recovery')
    if rec.get('state') not in ('disabled', 'incomplete', 'enabled', 'unknown'):
        fail('recovery state: %r' % rec)
    key = SVC.call('recoveryEnable', {}, 120)
    if len((key.get('key') or '').replace(' ', '')) < 40:
        fail('no recovery key made')
    if SVC.call('status').get('askRecovery'):
        fail('the recovery question is still asked after setting it up')
    ok('recovery key made after signing in (shown once; state now %s)' % key.get('state'))

    # ---- restart: the session comes back from the keyring; the access token is renewed
    SVC.stop()
    SVC.start()
    s = wait('signed in again after a restart', lambda: SVC.call('status').get('signedIn') and SVC.call('status'), 60)
    if s.get('signIn') != 'oauth':
        fail('the restored session isn\'t the OAuth one: %r' % s)
    wait('syncs after the restart', lambda: SVC.call('status').get('synced'), 60)
    ok('restart: the OAuth session came back from the keyring')
    def renewed():
        # Something that needs the server, now and then: an expired token is only noticed on a request.
        SVC.call('setName', {'name': 'Dave %d' % int(time.time())})
        return 'renewed the access and refresh tokens; the new ones are in the keyring' in SVC.log()
    wait('the access token is renewed (60 s tokens) and kept', renewed, 330, 20)
    SVC.call('setName', {'name': 'Dave again'})
    if not SVC.call('status').get('signedIn'):
        fail('signed out after the token was renewed')
    SVC.stop()
    SVC.start()
    wait('signed in again with the renewed token', lambda: SVC.call('status').get('signedIn') and SVC.call('status').get('synced'), 60)
    ok('the access token was renewed with the refresh token, kept in the keyring, and works after another restart')

    # ---- sign out: revoked at the server
    r = SVC.call('logout', {}, 60)
    if r.get('signedIn'):
        fail('still signed in after signing out')
    if 'the server revoked the session\'s tokens' not in SVC.log():
        fail('signing out didn\'t revoke the session at the server')
    ok('signed out; the server revoked the session')

    # ---- turned down on the phone
    st = SVC.call('authStart', {'homeserver': HS, 'mode': 'qr'}, 90)
    phone(st, approve=False)
    wait('Hearth hears the sign-in was turned down', lambda: 'turned down' in (SVC.call('status').get('error') or ''), 40)
    if SVC.call('status').get('signedIn') or SVC.call('status').get('auth'):
        fail('still signed in / waiting after the sign-in was turned down')
    ok('turned down on the phone: Hearth stops waiting and says so')

    # ---- "Create an account" in the browser: the server's sign-up page (prompt=create)
    st = SVC.call('authStart', {'homeserver': HS, 'mode': 'browser', 'create': True}, 90)
    if not st.get('create'):
        fail('prompt=create wasn\'t used: %r' % {k: v for k, v in st.items() if k != 'url'})
    b = Browser('desktop').open(st['url'])
    if '/register' not in urllib.parse.urlparse(b.url).path:
        fail('"Create an account" didn\'t open the sign-up page (landed on %s)' % hide(b.url))
    SVC.call('authCancel')
    ok('"Create an account": the browser opens the server\'s sign-up page (prompt=create)')

    # ---- the browser on this device: authorization code with PKCE, back to Hearth's loopback address
    st = SVC.call('authStart', {'homeserver': HS, 'mode': 'browser'}, 90)
    if not st.get('url', '').startswith('http'):
        fail('no sign-in page address')
    b = Browser('desktop').open(st['url'])
    b.login_if_asked()
    b.submit(lambda f: sorted(f['fields']) == ['csrf'] and not f['buttons'], {})
    if not b.url.startswith('http://127.0.0.1:') or 'signed in' not in b.body:
        fail('the browser didn\'t come back to Hearth (at %s: %s)' % (hide(b.url), re.sub(r'<[^>]+>|\s+', ' ', b.body)[:300]))
    s = wait('signed in from the browser', lambda: SVC.call('status').get('signedIn') and SVC.call('status'), 40)
    if s.get('signIn') != 'oauth':
        fail('browser sign-in: %r' % s)
    ok('browser on this device: signed in as %s, the page said "You\'re signed in", nothing typed into Hearth' % s['userId'])
    wait('syncs', lambda: SVC.call('status').get('synced'), 60)
    SVC.call('logout', {}, 60)

    # ---- the log: steps, never secrets
    log = SVC.log()
    for step in ('dynamic client registration', 'device code issued', 'approved on the phone', 'the browser came back to Hearth'):
        if step not in log:
            fail('the log has no step "%s"' % step)
    bad = [p for p in (r'mat_[A-Za-z0-9]', r'mar_[A-Za-z0-9]', r'mct_[A-Za-z0-9]', r'syt_[A-Za-z0-9]', r'code=[A-Za-z0-9]', r'state=[A-Za-z0-9]', re.escape(PW)) if re.search(p, log)]
    if bad:
        fail('secrets in the service log: %r' % bad)
    ok('the log has each sign-in step and no tokens, codes or passwords')
    SVC.stop()
except SystemExit:
    raise
except Exception as e:  # noqa: BLE001
    import traceback
    traceback.print_exc()
    fail(repr(e))
finally:
    if SVC:
        SVC.stop()
