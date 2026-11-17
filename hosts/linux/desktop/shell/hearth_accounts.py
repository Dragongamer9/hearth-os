#!/usr/bin/python3
"""hearth-accounts: Hearth's online accounts (Settings › Accounts › Online accounts, docs/ONLINE-ACCOUNTS.md).

Sign in once to Nextcloud, Google, Microsoft, or a plain CalDAV/CardDAV, WebDAV or IMAP server, and Hearth's apps
use the account: the calendar widget and Hearth Time read its calendars, Hearth Contacts its address books, Hearth
Files opens its files (gvfs, davs://), a mail app its IMAP.

Where things are kept:
  * The list of accounts and what each is used for: ~/.config/nimbus/online-accounts.json, written only through
    hearth_settings_io (atomic, FILE.bak, merge under the lock). It never holds a password or a token.
  * Passwords, app passwords and OAuth tokens: only the system keyring (the Secret Service, through `secret-tool`,
    given on its stdin, never on a command line), attributes application=hearth-accounts account=<id>. With no
    keyring, adding an account fails with a plain message: a secret is never written to a file instead.
  * Google and Microsoft sign in with OAuth 2.0 PKCE in your browser (a loopback redirect to 127.0.0.1 / localhost).
    Their client ids are not in this repo: the owner registers them and fills /etc/hearth/online-accounts.json
    (shipped empty as /usr/share/hearth/online-accounts/clients.json). Until then the buttons say what's needed.

The API for Hearth's apps (JSON on stdout, one object; "error: ..." on a failure; progress lines "progress: N text"):
  hearth-accounts list                         the accounts (no secrets): id, kind, name, user, server, can, use
  hearth-accounts providers                    which sign-ins can be used here (Google/Microsoft need a client id)
  hearth-accounts add-dav       < JSON         {"server", "user", "password", "name"?}: CalDAV and/or CardDAV
  hearth-accounts add-webdav    < JSON         {"server", "user", "password", "name"?}: files only
  hearth-accounts add-imap      < JSON         {"host", "port"?, "user", "password", "ssl"?}: mail
  hearth-accounts add-nextcloud SERVER         Nextcloud Login Flow v2 in the browser, an app password kept
  hearth-accounts add-oauth google|microsoft   OAuth PKCE in the browser, the refresh token kept
  hearth-accounts remove ID                    the account, its keyring items, its Files mount
  hearth-accounts use ID calendar|contacts|files|mail on|off
  hearth-accounts rename ID < NAME
  hearth-accounts token ID [mail|calendar|contacts]   a fresh access token (OAuth accounts), refreshed if needed
  hearth-accounts credentials ID              {"user", "password"} or {"user", "token"}: for an app's own DAV/IMAP
  hearth-accounts calendars [ID]              every calendar of the accounts with "calendar" on
  hearth-accounts events [--from YYYY-MM-DD] [--days N]   their events, the calendar widget's shape
  hearth-accounts addressbooks [ID]           every address book of the accounts with "contacts" on
  hearth-accounts cal-objects ID URL [--from YYYY-MM-DD] [--days N]   a calendar's event objects {href, etag, ics}
  hearth-accounts cal-put ID    < JSON        {"url", "ics", "etag"}: an event saved, only over that etag
  hearth-accounts cal-delete ID < JSON        {"url", "etag"}: an event deleted, only over that etag
  hearth-accounts cal-edit      < JSON        {"ics", "uid", "change", "scope": all|this, "occ", "op"}: the new ics
  hearth-accounts mount ID                    the account's files mounted with gvfs: {"path"}
  hearth-accounts unmount ID

Python standard library only. Tests: hosts/linux/test/test_online_accounts.py (fake CalDAV, WebDAV, OAuth,
Nextcloud and IMAP servers on localhost; a stand-in keyring; never a real account).
"""
import base64
import datetime as dt
import hashlib
import http.server as httpserver
import imaplib
import json
import os
import re
import secrets
import shutil
import socket
import ssl
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
import hearth_settings_io as sio  # noqa: E402
import hearth_ical  # noqa: E402

KINDS = ('nextcloud', 'dav', 'webdav', 'imap', 'google', 'microsoft')
USES = ('calendar', 'contacts', 'files', 'mail')
KIND_LABEL = {'nextcloud': 'Nextcloud', 'dav': 'CalDAV / CardDAV', 'webdav': 'WebDAV', 'imap': 'IMAP mail',
              'google': 'Google', 'microsoft': 'Microsoft'}
TIMEOUT = 20
FLOW_TIMEOUT = 20 * 60          # Nextcloud's own login flow token lasts 20 minutes
OAUTH_TIMEOUT = 10 * 60
NS = {'d': 'DAV:', 'c': 'urn:ietf:params:xml:ns:caldav', 'card': 'urn:ietf:params:xml:ns:carddav',
      'ical': 'http://apple.com/ns/ical/', 'cs': 'http://calendarserver.org/ns/'}


class Fail(Exception):
    pass


# ------------------------------------------------------------------ files

def home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def accounts_file():
    if os.environ.get('HEARTH_ACCOUNTS_FILE'):
        return os.environ['HEARTH_ACCOUNTS_FILE']
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(home(), '.config')
    return os.path.join(cfg, 'nimbus', 'online-accounts.json')


CLIENT_FILES = ('/usr/share/hearth/online-accounts/clients.json', '/etc/hearth/online-accounts.json')


def clients():
    """The OAuth providers: the image's defaults (endpoints, scopes, empty client ids), then the owner's
    /etc/hearth/online-accounts.json over them, key by key. HEARTH_ACCOUNTS_CLIENTS replaces both (tests)."""
    files = [os.environ['HEARTH_ACCOUNTS_CLIENTS']] if os.environ.get('HEARTH_ACCOUNTS_CLIENTS') else list(CLIENT_FILES)
    here = os.path.join(os.path.dirname(os.path.realpath(__file__)), '..', '..', '..', '..', 'os', 'system_files',
                        'usr', 'share', 'hearth', 'online-accounts', 'clients.json')
    if not os.environ.get('HEARTH_ACCOUNTS_CLIENTS') and not os.path.exists(CLIENT_FILES[0]) and os.path.exists(here):
        files[0] = here           # run from a checkout (CI's screenshots)
    out = {}
    for f in files:
        try:
            with open(f, encoding='utf-8') as fh:
                d = json.load(fh)
        except (OSError, ValueError):
            continue
        for k, v in (d.get('providers') or {}).items():
            if isinstance(v, dict):
                out.setdefault(k, {}).update({kk: vv for kk, vv in v.items() if vv not in (None, '')})
    return out


def load():
    d = sio.load_json(accounts_file(), {})
    lst = d.get('accounts')
    return [a for a in lst if isinstance(a, dict) and a.get('id')] if isinstance(lst, list) else []


def change_accounts(fn):
    """FN(list) under the file's lock, written atomically (hearth_settings_io): only the accounts key changes."""
    def apply(d):
        lst = d.get('accounts') if isinstance(d.get('accounts'), list) else []
        r = fn(lst)
        d['accounts'] = r if isinstance(r, list) else lst
        d.setdefault('version', 1)
    return sio.update_json(accounts_file(), apply).get('accounts', [])


def find(aid):
    a = next((x for x in load() if x.get('id') == aid), None)
    if not a:
        raise Fail("There's no such account (it may have been removed).")
    return a


def public(a):
    """An account as apps and the page see it: never a secret (there are none in the file anyway)."""
    keep = ('id', 'kind', 'name', 'user', 'server', 'email', 'can', 'use', 'added', 'calendars', 'addressbooks', 'imap')
    p = {k: a[k] for k in keep if k in a}
    p['kindLabel'] = KIND_LABEL.get(a.get('kind'), a.get('kind', ''))
    p['use'] = {u: bool((a.get('use') or {}).get(u)) and bool((a.get('can') or {}).get(u)) for u in USES}
    p['can'] = {u: bool((a.get('can') or {}).get(u)) for u in USES}
    return p


def new_id(kind):
    return '%s-%s' % (kind, secrets.token_hex(4))


# ------------------------------------------------------------------ the keyring (Secret Service, secret-tool)

def secret_tool():
    return os.environ.get('HEARTH_SECRET_TOOL') or 'secret-tool'


def _have_keyring():
    st = secret_tool()
    return bool(shutil.which(st) or (os.path.isabs(st) and os.access(st, os.X_OK)))


def keyring_set(aid, label, value):
    """VALUE (a dict) kept as one keyring item for the account, given to secret-tool on its stdin."""
    if not _have_keyring():
        raise Fail("This device's keyring (the Secret Service) isn't available, so the password can't be kept safely. "
                   'Nothing was saved.')
    try:
        r = subprocess.run([secret_tool(), 'store', '--label=' + label, 'application', 'hearth-accounts', 'account', aid],
                           input=json.dumps(value), capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise Fail("The keyring didn't answer (%s). Nothing was saved." % e.__class__.__name__)
    if r.returncode != 0:
        raise Fail('The keyring said no (it may be locked). Nothing was saved.')


def keyring_get(aid):
    if not _have_keyring():
        raise Fail("This device's keyring isn't available, so the account's password can't be read.")
    try:
        r = subprocess.run([secret_tool(), 'lookup', 'application', 'hearth-accounts', 'account', aid],
                           capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        raise Fail("The keyring didn't answer.")
    if r.returncode != 0 or not r.stdout.strip():
        raise Fail("This account's password isn't in the keyring any more: remove the account and add it again.")
    try:
        v = json.loads(r.stdout)
    except ValueError:
        raise Fail("This account's keyring item can't be read: remove the account and add it again.")
    return v if isinstance(v, dict) else {}


def keyring_clear(aid):
    if not _have_keyring():
        return
    try:
        subprocess.run([secret_tool(), 'clear', 'application', 'hearth-accounts', 'account', aid],
                       capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        pass


# ------------------------------------------------------------------ HTTP

LOOPBACK = ('127.0.0.1', 'localhost', '::1')


def check_url(u):
    """https only; plain http only for this device itself (tests, a server on this PC)."""
    p = urllib.parse.urlsplit(u)
    if p.scheme == 'https' and p.hostname:
        return u
    if p.scheme == 'http' and p.hostname in LOOPBACK:
        return u
    if p.scheme == 'http':
        raise Fail('Only secure (https://) addresses can be used, so your password is never sent in the clear.')
    raise Fail("That isn't a web address (it should start with https://).")


def normalize_server(s):
    s = (s or '').strip()
    if not s:
        raise Fail('Type the server address.')
    if not re.match(r'^[a-z]+://', s, re.I):
        s = 'https://' + s
    return check_url(s.rstrip('/') + '/' if urllib.parse.urlsplit(s).path in ('', '/') else s)


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


_opener = urllib.request.build_opener(_NoRedirect, urllib.request.HTTPSHandler(context=ssl.create_default_context()))


class Resp:
    def __init__(self, status, headers, body, url):
        self.status, self.headers, self.body, self.url = status, headers, body, url

    def text(self):
        return self.body.decode('utf-8', 'replace')

    def json(self):
        try:
            return json.loads(self.body.decode('utf-8'))
        except ValueError:
            raise Fail('The server answered something that isn\'t JSON.')


def auth_header(cred):
    if not cred:
        return {}
    if cred.get('token'):
        return {'Authorization': 'Bearer ' + cred['token']}
    raw = ('%s:%s' % (cred.get('user', ''), cred.get('password', ''))).encode()
    return {'Authorization': 'Basic ' + base64.b64encode(raw).decode()}


def http(method, url, body=None, headers=None, cred=None, timeout=TIMEOUT, ok=(200, 201, 204, 207)):
    """One request; redirects followed by hand (WebDAV's PROPFIND and REPORT too), the password only sent to the same
    host. The status must be in OK, else a plain error."""
    check_url(url)
    first = urllib.parse.urlsplit(url).netloc
    data = body.encode('utf-8') if isinstance(body, str) else body
    for _ in range(6):
        h = dict(headers or {})
        if urllib.parse.urlsplit(url).netloc == first:
            h.update(auth_header(cred))
        h.setdefault('User-Agent', 'Hearth-Accounts/1')
        req = urllib.request.Request(url, data=data, method=method, headers=h)
        try:
            r = _opener.open(req, timeout=timeout)
            res = Resp(r.status, r.headers, r.read(), url)
        except urllib.error.HTTPError as e:
            res = Resp(e.code, e.headers, e.read() if e.fp else b'', url)
        except (urllib.error.URLError, socket.timeout, OSError) as e:
            reason = getattr(e, 'reason', e)
            if isinstance(reason, ssl.SSLCertVerificationError):
                raise Fail("The server's security certificate isn't valid, so Hearth didn't send your password.")
            raise Fail("Couldn't reach %s (%s)." % (urllib.parse.urlsplit(url).hostname, reason))
        if res.status in (301, 302, 303, 307, 308) and res.headers.get('Location'):
            url = check_url(urllib.parse.urljoin(url, res.headers['Location']))
            if res.status == 303:
                method, data = 'GET', None
            continue
        if res.status == 401:
            raise Fail('Wrong user name or password (the server said no).')
        if ok and res.status not in ok:
            raise Fail('The server answered %d at %s.' % (res.status, urllib.parse.urlsplit(url).path or '/'))
        return res
    raise Fail('The server sent Hearth round in circles (too many redirects).')


def post_form(url, fields, timeout=TIMEOUT, ok=(200,)):
    return http('POST', url, urllib.parse.urlencode(fields), {'Content-Type': 'application/x-www-form-urlencoded',
                                                              'Accept': 'application/json'}, timeout=timeout, ok=ok)


# ------------------------------------------------------------------ WebDAV / CalDAV / CardDAV

def multistatus(xml_text):
    """[(href, {prop tag: Element})] for the 200 propstats of a 207 answer."""
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError:
        raise Fail("The server's answer couldn't be read (not WebDAV XML).")
    out = []
    for r in root.findall('d:response', NS):
        href = (r.findtext('d:href', '', NS) or '').strip()
        props = {}
        for ps in r.findall('d:propstat', NS):
            st = ps.findtext('d:status', '', NS) or ''
            if st and ' 200' not in st:
                continue
            for pr in ps.findall('d:prop', NS):
                for el in pr:
                    props[el.tag] = el
        out.append((href, props))
    return out


def _tag(ns, name):
    return '{%s}%s' % (NS[ns], name)


def propfind(url, cred, props, depth='0'):
    body = '<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="%s" xmlns:card="%s" xmlns:ical="%s"><d:prop>%s</d:prop></d:propfind>' % (
        NS['c'], NS['card'], NS['ical'], ''.join(props))
    r = http('PROPFIND', url, body, {'Depth': depth, 'Content-Type': 'application/xml; charset=utf-8'}, cred, ok=(207,))
    return r.url, multistatus(r.text())


def _href(el):
    h = el.find('d:href', NS) if el is not None else None
    return h.text.strip() if h is not None and h.text else None


def discover(server, cred, kind):
    """The CalDAV ('c') or CardDAV ('card') collections of the account at SERVER: [{url, name, color}].
    Tries the address itself, /.well-known/caldav|carddav and Nextcloud's /remote.php/dav, then follows
    current-user-principal and the home set."""
    home_prop = 'calendar-home-set' if kind == 'c' else 'addressbook-home-set'
    well = '/.well-known/caldav' if kind == 'c' else '/.well-known/carddav'
    tries = [server, urllib.parse.urljoin(server, well), urllib.parse.urljoin(server, '/remote.php/dav/')]
    seen, last = set(), None
    for start in tries:
        if start in seen:
            continue
        seen.add(start)
        try:
            base, res = propfind(start, cred, ['<d:current-user-principal/>', '<%s:%s/>' % (kind, home_prop)])
        except Fail as e:
            if 'Wrong user name' in str(e):
                raise
            last = e
            continue
        props = res[0][1] if res else {}
        homeset = _href(props.get(_tag(kind, home_prop)))
        if not homeset:
            principal = _href(props.get(_tag('d', 'current-user-principal')))
            if not principal:
                continue
            purl = urllib.parse.urljoin(base, principal)
            try:
                base2, res2 = propfind(purl, cred, ['<%s:%s/>' % (kind, home_prop)])
            except Fail as e:
                last = e
                continue
            homeset = _href((res2[0][1] if res2 else {}).get(_tag(kind, home_prop)))
            base = base2
        if not homeset:
            continue
        hurl = urllib.parse.urljoin(base, homeset)
        want = ['<d:resourcetype/>', '<d:displayname/>'] + (['<ical:calendar-color/>', '<c:supported-calendar-component-set/>',
                                                            '<d:current-user-privilege-set/>'] if kind == 'c' else [])
        hb, items = propfind(hurl, cred, want, '1')
        out = []
        coll = 'calendar' if kind == 'c' else 'addressbook'
        for href, p in items:
            rt = p.get(_tag('d', 'resourcetype'))
            if rt is None or rt.find('%s:%s' % (kind, coll), NS) is None:
                continue
            if kind == 'c':
                comps = p.get(_tag('c', 'supported-calendar-component-set'))
                if comps is not None and len(comps) and not any(x.get('name') == 'VEVENT' for x in comps):
                    continue          # a task list only
            name_el = p.get(_tag('d', 'displayname'))
            color_el = p.get(_tag('ical', 'calendar-color'))
            u = urllib.parse.urljoin(hb, href)
            item = {'url': u, 'name': (name_el.text or '').strip() if name_el is not None and name_el.text else
                    urllib.parse.unquote(u.rstrip('/').rsplit('/', 1)[-1]),
                    'color': (color_el.text or '').strip()[:7] if color_el is not None and color_el.text else ''}
            if kind == 'c':
                # A calendar that's shared with you to look at (or a holidays one): its events can't be changed.
                privs = p.get(_tag('d', 'current-user-privilege-set'))
                item['readOnly'] = privs is not None and not any(
                    x.tag in (_tag('d', 'write'), _tag('d', 'write-content'), _tag('d', 'all')) for x in privs.iter())
            out.append(item)
        return out
    if last and 'Wrong user name' in str(last):
        raise last
    return []


def webdav_check(url, cred):
    propfind(url, cred, ['<d:resourcetype/>'])
    return True


# ------------------------------------------------------------------ iCalendar (enough for events)

def _unfold(text):
    return re.sub(r'\r?\n[ \t]', '', text.replace('\r\n', '\n'))


def _unescape(v):
    return re.sub(r'\\([\\;,nN])', lambda m: '\n' if m.group(1) in 'nN' else m.group(1), v)


def _parse_prop(line):
    m = re.match(r'^([A-Za-z0-9-]+)((?:;[^:]*)?):(.*)$', line)
    if not m:
        return None, {}, ''
    params = {}
    for p in re.findall(r';([^=;]+)=("[^"]*"|[^;]*)', m.group(2)):
        params[p[0].upper()] = p[1].strip('"')
    return m.group(1).upper(), params, m.group(3)


def _when(value, params):
    """(datetime aware or date, all_day)."""
    v = value.strip()
    if params.get('VALUE') == 'DATE' or re.fullmatch(r'\d{8}', v):
        return dt.date(int(v[:4]), int(v[4:6]), int(v[6:8])), True
    m = re.fullmatch(r'(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})(Z?)', v)
    if not m:
        raise ValueError(v)
    t = dt.datetime(*[int(x) for x in m.groups()[:6]])
    if m.group(7):
        return t.replace(tzinfo=dt.timezone.utc), False
    tz = params.get('TZID')
    if tz:
        try:
            from zoneinfo import ZoneInfo
            return t.replace(tzinfo=ZoneInfo(tz)), False
        except Exception:                # noqa: BLE001 (an unknown zone name: taken as local time)
            pass
    return t.astimezone(), False         # floating: this device's time


def _zone_of(params, value):
    """The event's own zone as Hearth writes it: 'UTC', an IANA name, or '' (floating local time, or all-day)."""
    v = (value or '').strip()
    if params.get('VALUE') == 'DATE' or re.fullmatch(r'\d{8}', v):
        return ''
    if v.endswith('Z'):
        return 'UTC'
    return params.get('TZID', '')


def parse_events(ics, calendar='', color='', account='', window=None):
    """The events of a VCALENDAR in the widget's shape. A repeating event the server didn't expand (some ignore
    RFC 4791's expand) is expanded here for WINDOW (start, end: aware datetimes), its EXDATEs and changed times
    (RECURRENCE-ID) taken into account."""
    raw = []
    for block in re.findall(r'BEGIN:VEVENT\n(.*?)END:VEVENT', _unfold(ics), re.S):
        ev, depth = {'EXDATE': []}, 0
        for line in block.split('\n'):
            if line.startswith('BEGIN:'):
                depth += 1
            elif line.startswith('END:'):
                depth -= 1
            if depth or not line:
                continue
            name, params, val = _parse_prop(line)
            if name in ('DTSTART', 'DTEND', 'RECURRENCE-ID'):
                try:
                    ev[name] = _when(val, params)
                    if name == 'DTSTART':
                        ev['TZ'] = _zone_of(params, val)
                except ValueError:
                    pass
            elif name == 'EXDATE':
                for x in val.split(','):
                    try:
                        ev['EXDATE'].append(_when(x, params)[0])
                    except ValueError:
                        pass
            elif name in ('SUMMARY', 'LOCATION', 'UID', 'DURATION', 'STATUS', 'RRULE'):
                ev[name] = _unescape(val) if name != 'RRULE' else val.strip()
        if 'DTSTART' in ev:
            raw.append(ev)
    # Times of a repeating event that were changed on their own (they replace that time of the series).
    moved = {}
    for ev in raw:
        if 'RECURRENCE-ID' in ev:
            moved.setdefault(ev.get('UID', ''), []).append(ev['RECURRENCE-ID'][0])
    out = []
    for ev in raw:
        if ev.get('STATUS', '').upper() == 'CANCELLED':
            continue
        start, all_day = ev['DTSTART']
        end = ev.get('DTEND', (None, all_day))[0]
        if end is None:
            dur = _duration(ev.get('DURATION', ''))
            end = start + (dur if dur else (dt.timedelta(days=1) if all_day else dt.timedelta(0)))
        args = (ev.get('LOCATION', ''), calendar, color, account, ev.get('UID', ''))
        title = ev.get('SUMMARY') or '(no title)'
        recurring = bool(ev.get('RRULE')) or 'RECURRENCE-ID' in ev
        if ev.get('RRULE') and 'RECURRENCE-ID' not in ev and window:
            for s, e in _series(start, end, all_day, ev['RRULE'], ev['EXDATE'] + moved.get(ev.get('UID', ''), []), window):
                out.append(_event(title, s, e, all_day, *args, tz=ev.get('TZ', ''), recurring=True))
            continue
        out.append(_event(title, start, end, all_day, *args, tz=ev.get('TZ', ''), recurring=recurring))
    return out


def _series(start, end, all_day, rule, skip, window):
    """(start, end) of each time of a repeating event in WINDOW, in its own wall-clock time (so it stays at 9:00
    across a clock change)."""
    length = end - start
    if all_day:
        wall = lambda d: dt.datetime.combine(d if not isinstance(d, dt.datetime) else d.date(), dt.time())
        back = lambda w: w.date()
    else:
        tz = start.tzinfo
        wall = lambda d: (d.astimezone(tz) if isinstance(d, dt.datetime) else dt.datetime.combine(d, dt.time(), tz)).replace(tzinfo=None)
        back = lambda w: w.replace(tzinfo=tz) if tz is not None else w.astimezone()
    lo = wall(window[0]) - (length if isinstance(length, dt.timedelta) else dt.timedelta(0)) - dt.timedelta(days=2)
    hi = wall(window[1]) + dt.timedelta(days=2)
    ex = [wall(x) for x in skip]
    out = []
    for w in hearth_ical.expand(rule, wall(start), lo, hi, ex):
        s = back(w)
        out.append((s, s + length))
    return out


def _duration(s):
    m = re.fullmatch(r'([+-])?P(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?', s.strip()) if s else None
    if not m:
        return None
    w, d, h, mi, se = (int(x or 0) for x in m.groups()[1:])
    return dt.timedelta(weeks=w, days=d, hours=h, minutes=mi, seconds=se)


def _event(title, start, end, all_day, location, calendar, color, account, uid, tz='', recurring=False):
    """The calendar widget's / Hearth Time's shape: date and lastDate (this device's dates), start and end in ms,
    and the event's own zone (tz: 'UTC', an IANA name, or '' for floating time and all-day events)."""
    if all_day:
        first = start if isinstance(start, dt.date) and not isinstance(start, dt.datetime) else start.date()
        last_excl = end if isinstance(end, dt.date) and not isinstance(end, dt.datetime) else end.date()
        last = max(first, last_excl - dt.timedelta(days=1))
        s_ms = int(dt.datetime.combine(first, dt.time()).astimezone().timestamp() * 1000)
        e_ms = int(dt.datetime.combine(last_excl, dt.time()).astimezone().timestamp() * 1000)
    else:
        ls, le = start.astimezone(), end.astimezone()
        first, last = ls.date(), max(ls.date(), (le - dt.timedelta(seconds=1)).date() if le > ls else ls.date())
        s_ms, e_ms = int(ls.timestamp() * 1000), int(le.timestamp() * 1000)
    return {'title': title, 'date': first.isoformat(), 'lastDate': last.isoformat(), 'allDay': all_day,
            'start': s_ms, 'end': e_ms, 'location': location, 'color': color, 'calendar': calendar,
            'account': account, 'uid': uid, 'tz': '' if all_day else tz, 'recurring': bool(recurring)}


def caldav_events(cal_url, cred, start, end, calendar='', color='', account=''):
    """The events between START and END (UTC datetimes), recurring ones expanded by the server (RFC 4791 expand)."""
    f = lambda t: t.strftime('%Y%m%dT%H%M%SZ')
    body = ('<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">'
            '<d:prop><c:calendar-data><c:expand start="%s" end="%s"/></c:calendar-data></d:prop>'
            '<c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"><c:time-range start="%s" end="%s"/>'
            '</c:comp-filter></c:comp-filter></c:filter></c:calendar-query>') % (f(start), f(end), f(start), f(end))
    r = http('REPORT', cal_url, body, {'Depth': '1', 'Content-Type': 'application/xml; charset=utf-8'}, cred, ok=(207,))
    out = []
    for _href_, props in multistatus(r.text()):
        el = props.get(_tag('c', 'calendar-data'))
        if el is not None and el.text:
            out += parse_events(el.text, calendar, color, account, (start, end))
    s_ms, e_ms = start.timestamp() * 1000, end.timestamp() * 1000
    return [e for e in out if e['end'] > s_ms and e['start'] < e_ms or e['start'] == e['end'] and s_ms <= e['start'] < e_ms]


def graph_events(api, token, start, end, account=''):
    """Microsoft's calendar (Microsoft Graph calendarView: recurring events come expanded)."""
    q = urllib.parse.urlencode({'startDateTime': start.strftime('%Y-%m-%dT%H:%M:%SZ'),
                                'endDateTime': end.strftime('%Y-%m-%dT%H:%M:%SZ'), '$top': '200',
                                '$select': 'subject,start,end,isAllDay,location,iCalUId,isCancelled'})
    url, out = api.rstrip('/') + '/me/calendarView?' + q, []
    for _ in range(10):
        d = http('GET', url, None, {'Accept': 'application/json', 'Prefer': 'outlook.timezone="UTC"'},
                 {'token': token}, ok=(200,)).json()
        for e in d.get('value') or []:
            if e.get('isCancelled'):
                continue
            def t(x):
                s = (x or {}).get('dateTime', '')[:19]
                return dt.datetime.strptime(s, '%Y-%m-%dT%H:%M:%S').replace(tzinfo=dt.timezone.utc)
            try:
                s, en = t(e.get('start')), t(e.get('end'))
            except ValueError:
                continue
            allday = bool(e.get('isAllDay'))
            if allday:
                s, en = s.date(), en.date()
            out.append(_event(e.get('subject') or '(no title)', s, en, allday, ((e.get('location') or {}).get('displayName') or ''),
                              'Calendar', '', account, e.get('iCalUId') or '', tz='UTC'))
        url = d.get('@odata.nextLink')
        if not url:
            break
    return out


# ------------------------------------------------------------------ adding accounts

def read_input():
    raw = os.environ.get('HEARTH_INPUT')
    if raw is None or raw == '':
        raw = sys.stdin.read() if not sys.stdin.isatty() else ''
    try:
        v = json.loads(raw or '{}')
    except ValueError:
        raise Fail('The details were sent in a form Hearth can\'t read.')
    if not isinstance(v, dict):
        raise Fail('The details were sent in a form Hearth can\'t read.')
    return v


def progress(pct, text):
    print('progress: %d %s' % (pct, text), flush=True)


def _store(acct, secret):
    """Keyring first; only then the account goes in the list (a failed keyring leaves nothing behind)."""
    keyring_set(acct['id'], 'Hearth online account: %s' % acct['name'], secret)
    try:
        change_accounts(lambda lst: [a for a in lst if a.get('id') != acct['id']] + [acct])
    except Exception:
        keyring_clear(acct['id'])
        raise
    return public(acct)


def _same(kind, server, user):
    for a in load():
        if a.get('kind') == kind and a.get('server') == server and (a.get('user') or '').lower() == (user or '').lower():
            raise Fail('That account is already here.')


def add_dav(v):
    server, user, pw = normalize_server(v.get('server')), (v.get('user') or '').strip(), v.get('password') or ''
    if not user or not pw:
        raise Fail('Type the user name and password.')
    _same('dav', server, user)
    cred = {'user': user, 'password': pw}
    progress(20, 'Looking for calendars…')
    cals = discover(server, cred, 'c')
    progress(60, 'Looking for address books…')
    books = discover(server, cred, 'card')
    if not cals and not books:
        raise Fail('Signed in, but no calendars or address books were found there. Check the address (your provider '
                   'shows a CalDAV or CardDAV address).')
    host = urllib.parse.urlsplit(server).hostname
    acct = {'id': new_id('dav'), 'kind': 'dav', 'name': (v.get('name') or '').strip() or '%s (%s)' % (host, user),
            'server': server, 'user': user, 'added': int(time.time()),
            'can': {'calendar': bool(cals), 'contacts': bool(books), 'files': False, 'mail': False},
            'use': {'calendar': bool(cals), 'contacts': bool(books), 'files': False, 'mail': False},
            'calendars': cals, 'addressbooks': books}
    return _store(acct, {'password': pw})


def add_webdav(v):
    server, user, pw = normalize_server(v.get('server')), (v.get('user') or '').strip(), v.get('password') or ''
    if not user or not pw:
        raise Fail('Type the user name and password.')
    _same('webdav', server, user)
    progress(40, 'Signing in…')
    webdav_check(server, {'user': user, 'password': pw})
    host = urllib.parse.urlsplit(server).hostname
    acct = {'id': new_id('webdav'), 'kind': 'webdav', 'name': (v.get('name') or '').strip() or '%s (%s)' % (host, user),
            'server': server, 'user': user, 'added': int(time.time()), 'files': server,
            'can': {'calendar': False, 'contacts': False, 'files': True, 'mail': False},
            'use': {'calendar': False, 'contacts': False, 'files': True, 'mail': False}}
    return _store(acct, {'password': pw})


def imap_connect(host, port, ssl_on, timeout=TIMEOUT):
    if not ssl_on and host not in LOOPBACK:
        raise Fail('Only secure (SSL/TLS) mail servers can be used, so your password is never sent in the clear.')
    if ssl_on:
        return imaplib.IMAP4_SSL(host, port, ssl_context=ssl.create_default_context(), timeout=timeout)
    return imaplib.IMAP4(host, port, timeout=timeout)


def add_imap(v):
    host = (v.get('host') or '').strip().lower()
    user, pw = (v.get('user') or '').strip(), v.get('password') or ''
    ssl_on = v.get('ssl', True) is not False
    try:
        port = int(v.get('port') or (993 if ssl_on else 143))
    except (TypeError, ValueError):
        raise Fail('The port should be a number (usually 993).')
    if not re.fullmatch(r'[a-z0-9.-]+|\[?[0-9a-f:]+\]?', host or '-'):
        raise Fail("Type the mail server's name (like imap.example.com).")
    if not user or not pw:
        raise Fail('Type the user name and password.')
    _same('imap', host, user)
    progress(40, 'Signing in to the mail server…')
    try:
        m = imap_connect(host, port, ssl_on)
        try:
            m.login(user, pw)
        finally:
            try:
                m.logout()
            except Exception:     # noqa: BLE001
                pass
    except imaplib.IMAP4.error:
        raise Fail('Wrong user name or password (the mail server said no).')
    except ssl.SSLError:
        raise Fail("The mail server's security certificate isn't valid, so Hearth didn't send your password.")
    except OSError as e:
        raise Fail("Couldn't reach %s (%s)." % (host, e))
    acct = {'id': new_id('imap'), 'kind': 'imap', 'name': (v.get('name') or '').strip() or '%s (%s)' % (host, user),
            'server': host, 'user': user, 'added': int(time.time()), 'imap': {'host': host, 'port': port, 'ssl': ssl_on},
            'can': {'calendar': False, 'contacts': False, 'files': False, 'mail': True},
            'use': {'calendar': False, 'contacts': False, 'files': False, 'mail': True}}
    return _store(acct, {'password': pw})


def open_browser(url):
    """Your browser at URL (xdg-open). HEARTH_ACCOUNTS_OPEN replaces it (tests: a stand-in that follows the page)."""
    cmd = os.environ.get('HEARTH_ACCOUNTS_OPEN')
    argv = [cmd, url] if cmd else ['xdg-open', url]
    try:
        subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True)
    except OSError:
        print('log: open this address in a browser: ' + url, flush=True)


def add_nextcloud(server_text, deadline=None):
    """Nextcloud Login Flow v2: Nextcloud's own sign-in page in your browser (two-factor too); Nextcloud then hands
    Hearth an app password for this device only (you can revoke it in Nextcloud › Settings › Security)."""
    server = normalize_server(server_text)
    progress(5, 'Asking %s for a sign-in page…' % urllib.parse.urlsplit(server).hostname)
    try:
        d = http('POST', urllib.parse.urljoin(server, 'index.php/login/v2'), b'', {'Accept': 'application/json'},
                 ok=(200,)).json()
    except Fail as e:
        raise Fail("That doesn't look like a Nextcloud server (%s)" % str(e).rstrip('.'))
    poll, login = d.get('poll') or {}, d.get('login')
    if not (poll.get('token') and poll.get('endpoint') and login):
        raise Fail("That doesn't look like a Nextcloud server (its sign-in answer was incomplete).")
    check_url(login)
    endpoint = check_url(poll['endpoint'])
    open_browser(login)
    progress(30, 'Sign in to Nextcloud in your browser, then come back here.')
    print(json.dumps({'waiting': True, 'login': login}), flush=True)
    deadline = deadline or time.time() + FLOW_TIMEOUT
    got = None
    while time.time() < deadline:
        r = post_form(endpoint, {'token': poll['token']}, ok=(200, 404))
        if r.status == 200:
            got = r.json()
            break
        time.sleep(float(os.environ.get('HEARTH_ACCOUNTS_POLL', '2')))
    if not got:
        raise Fail('The sign-in took too long. Try again.')
    srv, user, pw = got.get('server'), got.get('loginName'), got.get('appPassword')
    if not (srv and user and pw):
        raise Fail('Nextcloud finished the sign-in but sent no app password.')
    srv = normalize_server(srv)
    _same('nextcloud', srv, user)
    progress(70, 'Signed in. Looking for calendars, contacts and files…')
    cred = {'user': user, 'password': pw}
    dav = urllib.parse.urljoin(srv, 'remote.php/dav/')
    files = urllib.parse.urljoin(dav, 'files/%s/' % urllib.parse.quote(user))
    try:
        cals = discover(dav, cred, 'c')
    except Fail:
        cals = []
    try:
        books = discover(dav, cred, 'card')
    except Fail:
        books = []
    try:
        has_files = webdav_check(files, cred)
    except Fail:
        has_files = False
    acct = {'id': new_id('nextcloud'), 'kind': 'nextcloud', 'name': 'Nextcloud (%s)' % user, 'server': srv, 'user': user,
            'added': int(time.time()), 'files': files, 'dav': dav, 'calendars': cals, 'addressbooks': books,
            'can': {'calendar': bool(cals), 'contacts': bool(books), 'files': has_files, 'mail': False},
            'use': {'calendar': bool(cals), 'contacts': bool(books), 'files': has_files, 'mail': False}}
    return _store(acct, {'password': pw, 'appPassword': True})


# ---- OAuth 2.0 with PKCE (RFC 7636) and a loopback redirect (RFC 8252)

def provider(name):
    p = clients().get(name)
    if not p or name not in ('google', 'microsoft'):
        raise Fail('Hearth doesn\'t know that sign-in.')
    return p


def provider_ready(p):
    return bool(p.get('clientId')) and bool(p.get('authUrl')) and bool(p.get('tokenUrl'))


def providers():
    out = {}
    for name in ('google', 'microsoft'):
        p = clients().get(name) or {}
        out[name] = {'ready': provider_ready(p), 'label': KIND_LABEL[name],
                     'why': '' if provider_ready(p) else
                     ('Signing in with %s needs Hearth\'s own %s client id, which isn\'t set up on this device yet '
                      '(/etc/hearth/online-accounts.json, docs/ONLINE-ACCOUNTS.md).' % (KIND_LABEL[name], KIND_LABEL[name]))}
    out['secretService'] = _have_keyring()
    return out


def pkce():
    verifier = base64.urlsafe_b64encode(secrets.token_bytes(48)).rstrip(b'=').decode()
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b'=').decode()
    return verifier, challenge


DONE_PAGE = ('<!doctype html><meta charset="utf-8"><title>Hearth</title><body style="font-family:sans-serif;'
             'background:#14161a;color:#eee;display:grid;place-items:center;height:90vh"><div><h2>%s</h2>'
             '<p>%s</p></div>')


def loopback_code(state, host='127.0.0.1', timeout=OAUTH_TIMEOUT, on_ready=None):
    """A one-shot web server on this device for the browser's redirect: (code or None, error, redirect_uri)."""
    got = {}

    class H(httpserver.BaseHTTPRequestHandler):
        def do_GET(self):        # noqa: N802
            q = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
            if urllib.parse.urlsplit(self.path).path != '/callback':
                self.send_response(404)
                self.end_headers()
                return
            if q.get('state', [''])[0] != state:
                got.setdefault('error', 'The answer from the sign-in page didn\'t match (state). Try again.')
                msg = ('Something went wrong', 'Go back to Hearth and try again.')
            elif q.get('error'):
                got['error'] = 'The sign-in was cancelled or refused (%s).' % q['error'][0][:80]
                msg = ('Not signed in', 'You can close this tab and go back to Hearth.')
            else:
                got['code'] = q.get('code', [''])[0]
                msg = ('Signed in', 'You can close this tab: Hearth is finishing up.')
            body = (DONE_PAGE % msg).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *a):     # nothing about the request (its code) goes anywhere
            pass

    bind = '127.0.0.1' if host in ('127.0.0.1', 'localhost') else host
    srv = httpserver.HTTPServer((bind, 0), H)
    srv.timeout = 1
    redirect = 'http://%s:%d/callback' % (host, srv.server_port)
    if on_ready:
        on_ready(redirect)
    end = time.time() + timeout
    try:
        while time.time() < end and not got.get('code') and not got.get('error'):
            srv.handle_request()
    finally:
        srv.server_close()
    return got.get('code'), got.get('error'), redirect


def _jwt_claims(tok):
    """The claims of the id_token the token endpoint itself just sent over TLS (only the e-mail is read)."""
    try:
        part = tok.split('.')[1]
        return json.loads(base64.urlsafe_b64decode(part + '=' * (-len(part) % 4)))
    except (IndexError, ValueError, TypeError):
        return {}


def token_request(p, fields):
    fields = dict(fields, client_id=p['clientId'])
    if p.get('clientSecret'):
        fields['client_secret'] = p['clientSecret']     # Google's desktop clients: see docs/ONLINE-ACCOUNTS.md
    try:
        r = post_form(p['tokenUrl'], fields, ok=None)
    except Fail as e:
        raise Fail('Couldn\'t finish signing in (%s)' % str(e).rstrip('.'))
    d = r.json() if r.body else {}
    if r.status != 200 or not d.get('access_token'):
        why = d.get('error_description') or d.get('error') or 'answer %d' % r.status
        raise Fail('The sign-in server said no (%s).' % str(why)[:160])
    return d


def add_oauth(name):
    p = provider(name)
    if not provider_ready(p):
        raise Fail(providers()[name]['why'])
    if not _have_keyring():
        raise Fail("This device's keyring (the Secret Service) isn't available, so the sign-in can't be kept safely.")
    verifier, challenge = pkce()
    state = secrets.token_urlsafe(24)
    scopes = p.get('scopes') or ''

    def go(redirect):
        q = dict(p.get('authParams') or {})
        q.update({'response_type': 'code', 'client_id': p['clientId'], 'redirect_uri': redirect, 'scope': scopes,
                  'state': state, 'code_challenge': challenge, 'code_challenge_method': 'S256'})
        url = p['authUrl'] + ('&' if '?' in p['authUrl'] else '?') + urllib.parse.urlencode(q)
        progress(20, 'Sign in to %s in your browser, then come back here.' % KIND_LABEL[name])
        threading.Thread(target=open_browser, args=(url,), daemon=True).start()

    code, err, redirect = loopback_code(state, p.get('redirectHost') or '127.0.0.1',
                                        float(os.environ.get('HEARTH_ACCOUNTS_OAUTH_TIMEOUT', OAUTH_TIMEOUT)), go)
    if err:
        raise Fail(err)
    if not code:
        raise Fail('The sign-in took too long. Try again.')
    progress(70, 'Signed in. Finishing up…')
    tok = token_request(p, {'grant_type': 'authorization_code', 'code': code, 'redirect_uri': redirect,
                            'code_verifier': verifier})
    if not tok.get('refresh_token'):
        raise Fail('%s didn\'t give Hearth a way to stay signed in (no refresh token). Try again.' % KIND_LABEL[name])
    claims = _jwt_claims(tok.get('id_token', ''))
    email = claims.get('email') or claims.get('preferred_username') or ''
    if not email:
        raise Fail('%s didn\'t say which account this is (no e-mail address in the answer).' % KIND_LABEL[name])
    _same(name, name, email)
    can = {'calendar': bool(p.get('caldav') or p.get('graph')), 'contacts': bool(p.get('carddav') or p.get('graph')),
           'files': False, 'mail': bool(p.get('imapHost'))}
    acct = {'id': new_id(name), 'kind': name, 'name': '%s (%s)' % (KIND_LABEL[name], email), 'server': name,
            'user': email, 'email': email, 'added': int(time.time()), 'can': can, 'use': dict(can)}
    if p.get('imapHost'):
        acct['imap'] = {'host': p['imapHost'], 'port': int(p.get('imapPort') or 993), 'ssl': True, 'oauth': True}
    secret = {'refresh': tok['refresh_token'], 'access': tok['access_token'],
              'expires': int(time.time()) + int(tok.get('expires_in') or 3600), 'scope': tok.get('scope') or scopes}
    return _store(acct, secret)


def fresh_token(aid, purpose=''):
    """An access token for the account, refreshed with the kept refresh token when it's under 2 minutes from
    expiring (the new tokens go back into the keyring). PURPOSE 'mail' on Microsoft asks for the IMAP scope."""
    a = find(aid)
    if a.get('kind') not in ('google', 'microsoft'):
        raise Fail('This account signs in with a password, not a token.')
    p = provider(a['kind'])
    s = keyring_get(aid)
    want = (p.get('mailScopes') or '') if purpose == 'mail' and p.get('mailScopes') else ''
    if not want and s.get('access') and int(s.get('expires') or 0) - time.time() > 120:
        return {'token': s['access'], 'expires': int(s['expires'])}
    if not s.get('refresh'):
        raise Fail('This account needs signing in again: remove it and add it again.')
    if not provider_ready(p):
        raise Fail(providers()[a['kind']]['why'])
    fields = {'grant_type': 'refresh_token', 'refresh_token': s['refresh']}
    if want:
        fields['scope'] = want
    try:
        tok = token_request(p, fields)
    except Fail as e:
        if 'invalid_grant' in str(e):
            raise Fail('%s signed this device out (the password changed or access was removed). Remove the account '
                       'and add it again.' % KIND_LABEL[a['kind']])
        raise
    exp = int(time.time()) + int(tok.get('expires_in') or 3600)
    s2 = dict(s, refresh=tok.get('refresh_token') or s['refresh'])   # a provider may hand out a new refresh token
    if not want:                       # a mail-only token (another scope) isn't kept as the account's own
        s2.update(access=tok['access_token'], expires=exp)
    if s2 != s:
        keyring_set(aid, 'Hearth online account: %s' % a.get('name', aid), s2)
    return {'token': tok['access_token'], 'expires': exp}


def credentials(aid, purpose=''):
    a = find(aid)
    if a.get('kind') in ('google', 'microsoft'):
        return dict(fresh_token(aid, purpose), user=a.get('user', ''))
    s = keyring_get(aid)
    return {'user': a.get('user', ''), 'password': s.get('password', '')}


# ------------------------------------------------------------------ calendars, address books, events

def _dav_cred(a):
    if a['kind'] in ('google', 'microsoft'):
        return {'token': fresh_token(a['id'], 'calendar')['token']}
    return {'user': a.get('user'), 'password': keyring_get(a['id']).get('password', '')}


def _cal_root(a):
    if a['kind'] == 'google':
        return provider('google').get('caldav')
    return a.get('dav') or a.get('server')


def _uses(kind, aid=None):
    out = []
    for a in load():
        if aid and a['id'] != aid:
            continue
        pa = public(a)
        if pa['use'][kind] or (aid and pa['can'][kind]):
            out.append(a)
    return out


def calendars(aid=None):
    out, errors = [], []
    for a in _uses('calendar', aid):
        try:
            if a['kind'] == 'microsoft':
                out.append({'account': a['id'], 'name': 'Calendar', 'url': '', 'color': '', 'readOnly': True,
                            'accountName': a.get('name', ''), 'kind': a['kind']})
                continue
            cals = discover(_cal_root(a), _dav_cred(a), 'c')
            # Google: changing events needs the full calendar scope (accounts signed in with calendar.readonly
            # can only look until they sign in again).
            ro = a['kind'] == 'google' and not re.search(r'auth/calendar(\s|$)', keyring_get(a['id']).get('scope') or '')
            for c in cals:
                out.append(dict(c, account=a['id'], accountName=a.get('name', ''), kind=a['kind'],
                                readOnly=bool(c.get('readOnly')) or ro))
            if a['kind'] in ('nextcloud', 'dav'):
                change_accounts(lambda lst: [dict(x, calendars=cals) if x.get('id') == a['id'] else x for x in lst]
                                if any(x.get('id') == a['id'] and x.get('calendars') != cals for x in lst) else lst)
        except Fail as e:
            errors.append({'account': a['id'], 'name': a.get('name'), 'error': str(e)})
    return {'calendars': out, 'errors': errors}


def addressbooks(aid=None):
    out, errors = [], []
    for a in _uses('contacts', aid):
        try:
            if a['kind'] == 'microsoft':
                continue          # Microsoft's contacts: Graph (/me/contacts) with `token`, no CardDAV
            root = provider('google').get('carddav') if a['kind'] == 'google' else (a.get('dav') or a.get('server'))
            for b in discover(root, _dav_cred(a), 'card'):
                out.append(dict(b, account=a['id'], accountName=a.get('name', ''), kind=a['kind'], user=a.get('user', '')))
        except Fail as e:
            errors.append({'account': a['id'], 'name': a.get('name'), 'error': str(e)})
    return {'addressbooks': out, 'errors': errors}


def events(start_day=None, days=31, aid=None):
    first = start_day or dt.date.today()
    start = dt.datetime.combine(first, dt.time()).astimezone().astimezone(dt.timezone.utc)
    end = start + dt.timedelta(days=max(1, min(int(days), 400)))
    out, errors = [], []
    for a in _uses('calendar', aid):
        try:
            if a['kind'] == 'microsoft':
                out += graph_events(provider('microsoft').get('graph', 'https://graph.microsoft.com/v1.0'),
                                    fresh_token(a['id'], 'calendar')['token'], start, end, a['id'])
                continue
            cred = _dav_cred(a)
            cals = a.get('calendars') if a['kind'] != 'google' and a.get('calendars') else discover(_cal_root(a), cred, 'c')
            for c in cals:
                out += caldav_events(c['url'], cred, start, end, c.get('name', ''), c.get('color', ''), a['id'])
        except Fail as e:
            errors.append({'account': a['id'], 'name': a.get('name'), 'error': str(e)})
    out.sort(key=lambda e: (e['start'], e['title']))
    return {'events': out, 'errors': errors}


# ------------------------------------------------------------------ changing events (Hearth Time's Calendar)
#
# Hearth Time keeps a copy of each account calendar's event objects ({href, etag, ics}) and its own changes until the
# server has them. A change goes up with If-Match: the etag of the copy it was made on, so a change someone made on
# another device in the meantime is never overwritten: the server says 412 and Hearth gets the server's version
# back ({"conflict": true, "server": {...}}) to show the person both. Nothing here keeps anything: Hearth Time does.

def _hosts(a):
    """The hosts this account's password or token may be sent to."""
    roots = [a.get('server'), a.get('dav')] + [c.get('url') for c in (a.get('calendars') or []) if isinstance(c, dict)]
    if a.get('kind') == 'google':
        roots += [provider('google').get('caldav'), provider('google').get('carddav')]
    return {urllib.parse.urlsplit(r).netloc.lower() for r in roots if r}


def _own(a, url):
    """URL must be on the account's own server: an app can't have the password sent anywhere else."""
    check_url(url or '')
    if urllib.parse.urlsplit(url).netloc.lower() not in _hosts(a):
        raise Fail("That address isn't on this account's server.")
    if a.get('kind') == 'microsoft':
        raise Fail("Microsoft calendars can only be read here.")
    return url


def cal_objects(aid, url, start, end):
    """The event objects of the calendar at URL with a time between START and END (aware datetimes), as the server
    keeps them (not expanded): [{href, etag, ics}]."""
    a = find(aid)
    _own(a, url)
    f = lambda t: t.astimezone(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    body = ('<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">'
            '<d:prop><d:getetag/><c:calendar-data/></d:prop><c:filter><c:comp-filter name="VCALENDAR">'
            '<c:comp-filter name="VEVENT"><c:time-range start="%s" end="%s"/></c:comp-filter></c:comp-filter></c:filter>'
            '</c:calendar-query>') % (f(start), f(end))
    r = http('REPORT', url, body, {'Depth': '1', 'Content-Type': 'application/xml; charset=utf-8'}, _dav_cred(a), ok=(207,))
    out = []
    for href, props in multistatus(r.text()):
        data, etag = props.get(_tag('c', 'calendar-data')), props.get(_tag('d', 'getetag'))
        if data is None or not data.text or 'BEGIN:VEVENT' not in data.text:
            continue
        out.append({'href': urllib.parse.urljoin(r.url, href), 'etag': (etag.text or '').strip() if etag is not None else '',
                    'ics': data.text})
    return {'objects': out}


def _server_copy(url, cred):
    r = http('GET', url, None, {'Accept': 'text/calendar'}, cred, ok=None)
    if r.status in (404, 410):
        return None
    if r.status != 200:
        raise Fail('The server answered %d when Hearth looked at the event again.' % r.status)
    return {'ics': r.text(), 'etag': r.headers.get('ETag') or ''}


def _answer(r, url, cred, what):
    if r.status == 412:
        theirs = _server_copy(url, cred)
        return {'conflict': True, 'gone': theirs is None, 'server': theirs}
    if 400 <= r.status < 500 and r.status not in (401, 408, 429):
        return {'refused': True, 'status': r.status,
                'message': "The server didn't take %s (it answered %d)%s." % (
                    what, r.status, ': you may only be allowed to look at this calendar' if r.status in (403, 405) else '')}
    raise Fail('The server answered %d when %s was sent.' % (r.status, what))


def cal_put(aid, v):
    """{url, ics, etag}: the object at URL becomes ICS, only if the server's copy still has ETAG (with no etag:
    only if there's none yet). {"ok", "etag", "ics"?} or {"conflict", "gone", "server"} or {"refused", "message"}."""
    a = find(aid)
    url, ics, etag = _own(a, v.get('url') or ''), v.get('ics') or '', (v.get('etag') or '').strip()
    if 'BEGIN:VCALENDAR' not in ics:
        raise Fail('That is not an event.')
    cred = _dav_cred(a)
    h = {'Content-Type': 'text/calendar; charset=utf-8'}
    h['If-Match' if etag else 'If-None-Match'] = etag or '*'
    r = http('PUT', url, ics, h, cred, ok=None)
    if 200 <= r.status < 300:
        new = r.headers.get('ETag') or ''
        if new:
            return {'ok': True, 'etag': new}
        # The server changed the event as it saved it (and so sent no etag): Hearth keeps the server's version.
        theirs = _server_copy(url, cred) or {}
        return {'ok': True, 'etag': theirs.get('etag', ''), 'ics': theirs.get('ics', '')}
    if r.status in (404, 410) and etag:
        return {'conflict': True, 'gone': True, 'server': None}
    return _answer(r, url, cred, 'the change')


def cal_delete(aid, v):
    """{url, etag}: deletes the object, only if the server's copy still has ETAG."""
    a = find(aid)
    url, etag = _own(a, v.get('url') or ''), (v.get('etag') or '').strip()
    cred = _dav_cred(a)
    r = http('DELETE', url, None, {'If-Match': etag} if etag else {}, cred, ok=None)
    if 200 <= r.status < 300 or r.status in (404, 410):
        return {'ok': True}
    return _answer(r, url, cred, 'the deletion')


def cal_edit(v):
    """{ics, uid, change, scope, occ, op}: the object with the change made (hearth_ical.edit). Nothing is sent."""
    try:
        return hearth_ical.edit(v.get('ics') or '', v.get('uid') or '', v.get('change') or {}, v.get('scope') or 'all',
                                v.get('occ') or '', v.get('op') or 'save', v.get('defaultTz'))
    except ValueError as e:
        raise Fail(str(e))


# ------------------------------------------------------------------ files (gvfs, davs://)

def gvfs_root():
    return os.path.join(os.environ.get('XDG_RUNTIME_DIR') or '/run/user/%d' % os.getuid(), 'gvfs')


def dav_uri(a):
    u = urllib.parse.urlsplit(a.get('files') or '')
    if not u.hostname:
        raise Fail("This account has no files to open.")
    scheme = 'davs' if u.scheme == 'https' else 'dav'
    netloc = '%s@%s' % (urllib.parse.quote(a.get('user', ''), safe=''), u.hostname + (':%d' % u.port if u.port else ''))
    return urllib.parse.urlunsplit((scheme, netloc, u.path or '/', '', ''))


def mounted_path(a):
    """Where gvfs shows the account's files ($XDG_RUNTIME_DIR/gvfs/dav:host=…,ssl=true,user=…,prefix=…), or None."""
    u = urllib.parse.urlsplit(a.get('files') or '')
    if not u.hostname:
        return None
    try:
        names = os.listdir(gvfs_root())
    except OSError:
        return None
    want_prefix = (u.path or '/').rstrip('/')
    best = None
    for n in names:
        m = re.match(r'^davs?:(.*)$', n)
        if not m:
            continue
        kv = dict(x.split('=', 1) for x in m.group(1).split(',') if '=' in x)
        if urllib.parse.unquote(kv.get('host', '')).lower() != u.hostname.lower():
            continue
        if kv.get('user') and urllib.parse.unquote(kv['user']) != a.get('user'):
            continue
        prefix = urllib.parse.unquote(kv.get('prefix', '')).rstrip('/')
        if prefix and want_prefix.startswith(prefix):
            rest = want_prefix[len(prefix):].lstrip('/')
            best = os.path.join(gvfs_root(), n, urllib.parse.unquote(rest)) if rest else os.path.join(gvfs_root(), n)
        elif not prefix:
            best = os.path.join(gvfs_root(), n, urllib.parse.unquote(want_prefix.lstrip('/')))
    return best


def mount(aid):
    a = find(aid)
    if not public(a)['use']['files']:
        raise Fail('Files are off for this account (Settings › Accounts › Online accounts).')
    have = mounted_path(a)
    if have and os.path.isdir(have):
        return {'path': have, 'already': True}
    import hearth_netshares as ns
    pw = keyring_get(aid).get('password', '')
    ok, text = ns.gio_mount(dav_uri(a), user=a.get('user'), password=pw)
    if not ok and 'already mounted' not in (text or '').lower():
        t = (text or '').lower()
        if 'authentication' in t or 'wrong user' in t:
            raise Fail('The server didn\'t take the saved password. Remove the account and add it again.')
        if 'gio is missing' in t:
            raise Fail('Opening online files needs gvfs, which isn\'t on this device.')
        raise Fail('Couldn\'t open the files (%s).' % ns.gio_error(text)[:200])
    for _ in range(20):
        have = mounted_path(a)
        if have:
            return {'path': have}
        time.sleep(0.25)
    raise Fail('The files were opened, but gvfs didn\'t show them as a folder (is gvfs-fuse running?).')


def unmount(aid):
    a = find(aid)
    if a.get('files') and shutil.which('gio'):
        try:
            subprocess.run(['gio', 'mount', '-u', dav_uri(a)], capture_output=True, timeout=20)
        except (OSError, subprocess.TimeoutExpired):
            pass
    return {'ok': True}


def file_places():
    """The accounts whose files are on, for Hearth Files' left menu (hearth-files places)."""
    out = []
    for a in load():
        pa = public(a)
        if not (pa['use']['files'] and a.get('files')):
            continue
        p = mounted_path(a)
        out.append({'id': 'account:' + a['id'], 'label': a.get('name') or 'Online files', 'path': p or '',
                    'kind': 'cloud', 'section': 'bookmarks', 'account': a['id'], 'mounted': bool(p),
                    'sub': 'Connected' if p else 'Online account'})
    return out


# ------------------------------------------------------------------ changing accounts

def remove(aid):
    a = find(aid)
    if a.get('files'):
        unmount(aid)
    change_accounts(lambda lst: [x for x in lst if x.get('id') != aid])
    keyring_clear(aid)
    return {'ok': True, 'removed': aid}


def set_use(aid, what, on):
    if what not in USES:
        raise Fail('Not something an account is used for.')
    a = find(aid)
    if on and not public(a)['can'][what]:
        raise Fail('%s doesn\'t offer that.' % (a.get('name') or 'This account'))

    def ch(lst):
        for x in lst:
            if x.get('id') == aid:
                x['use'] = dict(x.get('use') or {}, **{what: bool(on)})
        return lst
    change_accounts(ch)
    if what == 'files' and not on:
        unmount(aid)
    return public(find(aid))


def rename(aid, name):
    name = (name or '').strip()[:80]
    if not name:
        raise Fail('Type a name.')
    find(aid)
    change_accounts(lambda lst: [dict(x, name=name) if x.get('id') == aid else x for x in lst])
    return public(find(aid))


# ------------------------------------------------------------------ the command line

def main(argv):
    if not argv:
        print(__doc__)
        return 2
    c, rest = argv[0], argv[1:]
    try:
        if c == 'list':
            out = {'accounts': [public(a) for a in load()]}
        elif c == 'providers':
            out = providers()
        elif c == 'add-dav':
            out = {'account': add_dav(read_input())}
        elif c == 'add-webdav':
            out = {'account': add_webdav(read_input())}
        elif c == 'add-imap':
            out = {'account': add_imap(read_input())}
        elif c == 'add-nextcloud' and len(rest) == 1:
            out = {'account': add_nextcloud(rest[0])}
        elif c == 'add-oauth' and len(rest) == 1:
            out = {'account': add_oauth(rest[0])}
        elif c == 'remove' and len(rest) == 1:
            out = remove(rest[0])
        elif c == 'use' and len(rest) == 3 and rest[2] in ('on', 'off'):
            out = {'account': set_use(rest[0], rest[1], rest[2] == 'on')}
        elif c == 'rename' and len(rest) == 1:
            raw = os.environ.get('HEARTH_INPUT') or (sys.stdin.read() if not sys.stdin.isatty() else '')
            out = {'account': rename(rest[0], raw)}
        elif c == 'token' and 1 <= len(rest) <= 2:
            out = fresh_token(rest[0], rest[1] if len(rest) > 1 else '')
        elif c == 'credentials' and 1 <= len(rest) <= 2:
            out = credentials(rest[0], rest[1] if len(rest) > 1 else '')
        elif c == 'calendars':
            out = calendars(rest[0] if rest else None)
        elif c == 'addressbooks':
            out = addressbooks(rest[0] if rest else None)
        elif c == 'events':
            start, days, aid = None, 31, None
            i = 0
            while i < len(rest):
                if rest[i] == '--from' and i + 1 < len(rest):
                    start = dt.date.fromisoformat(rest[i + 1]); i += 2
                elif rest[i] == '--days' and i + 1 < len(rest):
                    days = int(rest[i + 1]); i += 2
                elif rest[i] == '--account' and i + 1 < len(rest):
                    aid = rest[i + 1]; i += 2
                else:
                    raise Fail('events [--from YYYY-MM-DD] [--days N] [--account ID]')
            out = events(start, days, aid)
        elif c == 'cal-objects' and len(rest) >= 2:
            start, days = dt.date.today() - dt.timedelta(days=62), 500
            i = 2
            while i < len(rest):
                if rest[i] == '--from' and i + 1 < len(rest):
                    start = dt.date.fromisoformat(rest[i + 1]); i += 2
                elif rest[i] == '--days' and i + 1 < len(rest):
                    days = max(1, min(int(rest[i + 1]), 1500)); i += 2
                else:
                    raise Fail('cal-objects ID URL [--from YYYY-MM-DD] [--days N]')
            s0 = dt.datetime.combine(start, dt.time()).astimezone()
            out = cal_objects(rest[0], rest[1], s0, s0 + dt.timedelta(days=days))
        elif c == 'cal-put' and len(rest) == 1:
            out = cal_put(rest[0], read_input())
        elif c == 'cal-delete' and len(rest) == 1:
            out = cal_delete(rest[0], read_input())
        elif c == 'cal-edit' and not rest:
            out = cal_edit(read_input())
        elif c == 'mount' and len(rest) == 1:
            out = mount(rest[0])
        elif c == 'unmount' and len(rest) == 1:
            out = unmount(rest[0])
        elif c == 'file-places':
            out = {'places': file_places()}
        else:
            print('error: unknown command (hearth-accounts with no arguments lists them)')
            return 2
    except Fail as e:
        print('error: ' + str(e), flush=True)
        return 1
    except ValueError as e:
        print('error: %s' % e, flush=True)
        return 1
    print(json.dumps(out), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
