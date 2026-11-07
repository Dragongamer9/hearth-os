"""Your devices' shared folders in Hearth Files (FileManager.qml's sidebar, and the Open / Save dialogs that use it).

Found by itself, read-only:
- your paired Hearth devices' shares: each device's status over Hearth Link (hearth-device-status devices: name, Home
  or whole drive, read only or read & write, home / Tailscale reach, its names at home and through Tailscale);
- other computers sharing on the home network (mDNS/DNS-SD `_smb._tcp`, avahi-browse when it's there);
- your Tailscale devices that answer SMB on port 445 (a plain connect, nothing sent).

What was found is kept in $XDG_STATE_HOME/nimbus/files-devices.json (state, not a setting: rebuilt any time; no
passwords). The sidebar reads it at once and asks for a fresh look in the background when it's a minute old.

Opening one mounts it with gvfs (`gio mount smb://host/share`), using the home name at home and the Tailscale name
away, chosen by itself. The first time it asks for the user name and sharing password; "Remember" keeps the password
in the system keyring (Secret Service, `secret-tool`, with gvfs's own network-password attributes, so gvfs finds it
too), never in Hearth's files. The password reaches gio and secret-tool on their input, never a command line.

Connect automatically (a shared folder's menu in Hearth Files; off for each until you turn it on): the shares you
picked are kept in Hearth Files' own settings (files.json "autoConnect", written through hearth_settings_io: atomic,
with .bak, merged), never a password. hearth-files-autoconnect.timer runs `hearth-files net-autoconnect` 30 seconds
after signing in (Desktop Mode and Steam Game Mode alike) and every 3 minutes after: each picked share that isn't
open yet is opened in the background with what you signed in with before (the keyring). A device that doesn't answer
within a second or two is skipped quietly and tried again next time; one that needs a password or no longer has that
share is left alone until you open it yourself (noted in $XDG_RUNTIME_DIR only). Nothing waits for it: it isn't
part of signing in, Game Mode or Steam starting.
"""
import json
import os
import re
import select
import shutil
import socket
import subprocess
import tempfile
import threading
import time
import urllib.parse

STALE_S = 60
SMB_PORT = 445
DOMAIN = 'WORKGROUP'


def home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def state_path():
    base = os.environ.get('XDG_STATE_HOME') or os.path.join(home(), '.local', 'state')
    return os.path.join(base, 'nimbus', 'files-devices.json')


def secret_tool():
    return os.environ.get('HEARTH_SECRET_TOOL') or 'secret-tool'


# ------------------------------------------------------------------ finding them

def parse_avahi(text):
    """`avahi-browse -rtp _smb._tcp` lines ("=;eth0;IPv4;Name;_smb._tcp;local;host.local;192.168.1.5;445;...") ->
    [{name, host, addr, port}], once per host."""
    out, seen = [], set()
    for line in (text or '').splitlines():
        p = line.split(';')
        if len(p) < 9 or p[0] != '=' or p[4] != '_smb._tcp':
            continue
        name = re.sub(r'\\(\d{3})', lambda m: chr(int(m.group(1))), p[3])
        host, addr = p[6].rstrip('.'), p[7]
        try:
            port = int(p[8])
        except ValueError:
            port = SMB_PORT
        key = host.lower()
        if not host or key in seen:
            continue
        seen.add(key)
        out.append({'name': name, 'host': host, 'addr': addr, 'port': port})
    return out


def avahi_servers(timeout=4):
    if not shutil.which('avahi-browse'):
        return []
    try:
        r = subprocess.run(['avahi-browse', '-rtp', '_smb._tcp'], capture_output=True, text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        return []
    return parse_avahi(r.stdout)


def tailscale_state():
    """{running, self, peers: [{name, dns, online}]} from `tailscale status --json` ({} without Tailscale)."""
    if not shutil.which('tailscale'):
        return {}
    try:
        r = subprocess.run(['tailscale', 'status', '--json'], capture_output=True, text=True, timeout=5)
        j = json.loads(r.stdout or '{}')
    except (OSError, subprocess.TimeoutExpired, ValueError):
        return {}
    return parse_tailscale(j)


def parse_tailscale(j):
    me = j.get('Self') or {}
    peers = [{'name': p.get('HostName') or '', 'dns': (p.get('DNSName') or '').rstrip('.'), 'online': bool(p.get('Online'))}
             for p in (j.get('Peer') or {}).values() if isinstance(p, dict)]
    return {'running': j.get('BackendState') == 'Running', 'self': (me.get('DNSName') or '').rstrip('.'), 'peers': peers}


def answers_smb(host, port=SMB_PORT, timeout=0.8):
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def tailscale_servers(ts, probe=answers_smb, limit=24):
    """Online Tailscale devices that answer on 445, probed at once (nothing is sent but the connect)."""
    peers = [p for p in (ts or {}).get('peers', []) if p.get('online') and p.get('dns')][:limit]
    found, threads = [], []

    def one(p):
        if probe(p['dns']):
            found.append({'name': p['name'] or p['dns'].split('.')[0], 'host': p['dns'], 'away': True})
    for p in peers:
        t = threading.Thread(target=one, args=(p,), daemon=True)
        t.start()
        threads.append(t)
    for t in threads:
        t.join(2)
    return sorted(found, key=lambda s: s['name'].lower())


def hearth_devices(helper):
    """Paired Hearth devices with their shares (hearth-device-status devices)."""
    try:
        r = subprocess.run(['python3', helper, 'devices'], capture_output=True, text=True, timeout=30)
        v = json.loads((r.stdout or '').strip().splitlines()[-1])
    except (OSError, subprocess.TimeoutExpired, ValueError, IndexError):
        return []
    out = []
    for d in v.get('devices') or [] if isinstance(v, dict) else []:
        st = d.get('status') or {}
        smb = st.get('smb') or {}
        shares = [s for s in st.get('shares') or [] if isinstance(s, dict) and s.get('name')]
        out.append({'id': d.get('id'), 'name': d.get('name') or st.get('name') or 'Hearth device', 'online': bool(d.get('online')),
                    'kind': st.get('kind') or '', 'home': smb.get('home') or '', 'away': smb.get('away') or '',
                    'shares': [{'name': s['name'], 'write': bool(s.get('write')), 'badge': s.get('badge') or '', 'reach': s.get('reach') or 'home'}
                               for s in shares], 'wake': bool(d.get('wake'))})
    return [d for d in out if d['shares']]


def discover(helper, avahi=avahi_servers, ts_state=tailscale_state, probe=answers_smb, now=None):
    """Everything found, ready for the sidebar: {at, devices, servers, tailscale}."""
    ts = ts_state()
    devs = hearth_devices(helper)
    known = set()
    for d in devs:
        for h in (d['home'], d['away']):
            if h:
                known.add(h.lower())
                known.add(h.lower().split('.')[0])
    servers = []
    for s in avahi() + tailscale_servers(ts, probe):
        k = s['host'].lower()
        if k in known or k.split('.')[0] in known:
            continue
        known.add(k)
        known.add(k.split('.')[0])
        servers.append(s)
    return {'at': int(now or time.time()), 'devices': devs, 'servers': servers,
            'tailscale': {'running': bool(ts.get('running')), 'self': ts.get('self') or ''}}


# ------------------------------------------------------------------ state (not settings)

def read_state(path=None):
    try:
        with open(path or state_path()) as f:
            v = json.load(f)
        return v if isinstance(v, dict) else {}
    except (OSError, ValueError):
        return {}


def write_state(data, path=None):
    path = path or state_path()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=os.path.dirname(path), prefix='.files-devices-')
    with os.fdopen(fd, 'w') as f:
        json.dump(data, f)
    os.replace(tmp, path)


def merge_state(old, new):
    """The fresh look, keeping the user names you signed in with (never a password) and devices seen before (offline)."""
    out = dict(new)
    out['users'] = dict(old.get('users') or {})
    have = {d.get('id') for d in new.get('devices') or []}
    for d in old.get('devices') or []:
        if d.get('id') and d['id'] not in have:
            out.setdefault('devices', []).append(dict(d, online=False))
    return out


def remember_user(host, user, path=None):
    st = read_state(path)
    users = dict(st.get('users') or {})
    if users.get(host.lower()) == user:
        return
    users[host.lower()] = user
    st['users'] = users
    write_state(st, path)


# ------------------------------------------------------------------ where to reach it

def resolves(host, timeout=1.5):
    """Does this name answer at home (mDNS .local or the home network's DNS)? Asked with a short timeout."""
    res = []
    t = threading.Thread(target=lambda: res.append(_resolve(host)), daemon=True)
    t.start()
    t.join(timeout)
    return bool(res and res[0])


def _resolve(host):
    try:
        return bool(socket.getaddrinfo(host, SMB_PORT, proto=socket.IPPROTO_TCP))
    except OSError:
        return False


def choose_address(dev, at_home, ts_running):
    """(host, where) for a device: its home name when it answers here, else its Tailscale name when Tailscale runs.
    where: "home" | "away" | "tailscale-off" (only reachable through Tailscale, which isn't signed in) | "nowhere"."""
    if dev.get('home') and at_home(dev['home']):
        return dev['home'], 'home'
    if dev.get('away'):
        return (dev['away'], 'away') if ts_running else (None, 'tailscale-off')
    if dev.get('home'):
        return dev['home'], 'home'     # not answering now: try anyway (it may be asleep; mounting says so)
    return None, 'nowhere'


def smb_uri(host, share=''):
    return 'smb://%s/%s' % (host, urllib.parse.quote(share) if share else '')


def split_uri(uri):
    m = re.fullmatch(r'smb://([A-Za-z0-9._-]{1,253})/?([^/]*)/?', uri or '')
    if not m:
        return None, None
    return m.group(1), urllib.parse.unquote(m.group(2))


# ------------------------------------------------------------------ the sidebar

def share_key(net):
    """The name a shared folder is kept under in "autoConnect": dev:<device id>:<share> for a paired Hearth device's,
    else smb:<computer>:<share>."""
    share = (net or {}).get('share') or ''
    if net.get('device'):
        return 'dev:%s:%s' % (net['device'], share)
    return 'smb:%s:%s' % ((net.get('home') or net.get('away') or '').lower(), share)


AUTO_FIELDS = ('device', 'name', 'share', 'home', 'away', 'wake')


def auto_entry(net):
    """What's kept for a shared folder that connects by itself: where to find it, never a password."""
    return {k: net[k] for k in AUTO_FIELDS if k in net and isinstance(net[k], (str, bool))}


def sidebar(state, show=True, auto=()):
    """Places for Hearth Files' "Your devices" section: each device (online or not), its shares under it, other
    computers sharing, and "How to open from Windows or a phone". Nothing when it's turned off or nothing's found."""
    if not show:
        return []
    out = []
    ts_on = bool((state.get('tailscale') or {}).get('running'))
    for d in sorted(state.get('devices') or [], key=lambda x: (not x.get('online'), (x.get('name') or '').lower())):
        out.append({'id': 'dev:%s' % d.get('id'), 'label': d.get('name') or 'Hearth device', 'kind': 'device', 'section': 'devices',
                    'sub': 'Online' if d.get('online') else 'Offline or asleep', 'offline': not d.get('online'),
                    'device': d.get('id'), 'wake': bool(d.get('wake'))})
        for s in d.get('shares') or []:
            badge = {'home': 'Home', 'drive': 'Whole drive', 'system': 'System drive'}.get(s.get('badge'), '')
            on = share_key({'device': d.get('id'), 'share': s['name']}) in auto
            out.append({'id': 'share:%s:%s' % (d.get('id'), s['name']), 'label': s['name'], 'kind': 'network', 'section': 'devices',
                        'child': True, 'offline': not d.get('online'), 'auto': on,
                        'sub': ' · '.join(x for x in (badge, 'Read & write' if s.get('write') else 'Read only',
                                                      'Through Tailscale' if s.get('reach') == 'tailscale' else '',
                                                      'Connects by itself' if on else '') if x),
                        'net': {'device': d.get('id'), 'name': d.get('name'), 'share': s['name'], 'home': d.get('home') or '',
                                'away': d.get('away') or '', 'wake': bool(d.get('wake'))}})
    for s in state.get('servers') or []:
        out.append({'id': 'smb:%s' % s['host'], 'label': s.get('name') or s['host'], 'kind': 'network', 'section': 'devices',
                    'sub': 'Through Tailscale' if s.get('away') else 'On this network',
                    'net': {'name': s.get('name') or s['host'], 'share': '', 'home': '' if s.get('away') else s['host'],
                            'away': s['host'] if s.get('away') else ''}})
    if out:
        out.append({'id': 'devices-howto', 'label': 'How to open from Windows or a phone', 'kind': 'howto', 'section': 'devices',
                    'howto': howto(state), 'child': True})
    return out


def howto(state):
    """The addresses for Windows (\\\\host\\share) and phones or Macs (smb://host/share), at home and away."""
    lines = []
    for d in state.get('devices') or []:
        for s in d.get('shares') or []:
            if d.get('home'):
                lines.append({'device': d.get('name'), 'share': s['name'], 'windows': '\\\\%s\\%s' % (d['home'], s['name']),
                              'phone': smb_uri(d['home'], s['name']),
                              'away': '\\\\%s\\%s' % (d['away'], s['name']) if d.get('away') else ''})
    return lines


# ------------------------------------------------------------------ the keyring

def keyring_lookup(server, share, user):
    """The remembered password, or None (no keyring, nothing kept)."""
    if not user or not shutil.which(secret_tool()) and not os.path.isabs(secret_tool()):
        return None
    try:
        r = subprocess.run([secret_tool(), 'lookup', 'protocol', 'smb', 'server', server.lower(), 'object', share, 'user', user],
                           capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return r.stdout.rstrip('\n') if r.returncode == 0 and r.stdout else None


def keyring_store(server, share, user, password):
    """Keeps the password in the system keyring (the same attributes gvfs uses), on secret-tool's input."""
    if not shutil.which(secret_tool()) and not os.path.isabs(secret_tool()):
        raise OSError('no keyring')
    label = 'Sharing password for %s on %s' % (share or server, server)
    r = subprocess.run([secret_tool(), 'store', '--label=' + label, 'protocol', 'smb', 'server', server.lower(), 'object', share,
                        'user', user, 'domain', DOMAIN], input=password, capture_output=True, text=True, timeout=15)
    if r.returncode != 0:
        raise OSError((r.stderr or 'the keyring said no').strip())


# ------------------------------------------------------------------ mounting

def classify(text, where):
    """gio's words -> {error, need}: password (wrong or missing), off (the share isn't there), asleep, tailscale."""
    t = (text or '').lower()
    if where == 'tailscale-off':
        return {'need': 'tailscale', 'error': "It's only reachable through Tailscale, and Tailscale isn't signed in on this device"}
    if any(w in t for w in ('logon_failure', 'permission denied', 'access denied', 'wrong password', 'password', 'authentication')):
        return {'need': 'password', 'error': 'Wrong user name or password'}
    if any(w in t for w in ('bad_network_name', 'no such file', 'not found', 'share not')):
        return {'need': 'off', 'error': "That shared folder is off or isn't there anymore"}
    if any(w in t for w in ('timed out', 'no route', 'host is down', 'connection refused', 'failed to connect', 'unreachable', 'resolve', 'name or service')):
        return {'need': 'asleep', 'error': "It isn't answering: it may be off or asleep"}
    return {'need': '', 'error': (text or "Couldn't open it").strip().splitlines()[-1][:300]}


def gio_mount(uri, user=None, password=None, timeout=45):
    """`gio mount URI`, answering its questions: never anonymous, the user, the domain, the password (on its input).
    (ok, text)."""
    if not shutil.which('gio'):
        return False, 'gio is missing (gvfs)'
    p = subprocess.Popen(['gio', 'mount', uri], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    buf, asked, end = b'', 0, time.time() + timeout
    try:
        while True:
            if time.time() > end:
                p.kill()
                return False, 'timed out'
            r, _, _ = select.select([p.stdout], [], [], 0.3)
            if r:
                ch = os.read(p.stdout.fileno(), 1024)
                if not ch:
                    break
                buf += ch
                low = buf.decode('utf-8', 'replace').lower()
                tail = low[-120:]
                answer = None
                if re.search(r'(^|\n)[ \t]*(login )?anonymous[^\n]*[?:][ \t]*$', tail):
                    answer = 'n'
                elif re.search(r'(^|\n)[ \t]*(login )?user[^\n]*:[ \t]*$', tail):
                    answer = user or ''
                elif re.search(r'(^|\n)[ \t]*(login )?domain[^\n]*:[ \t]*$', tail):
                    answer = DOMAIN
                elif re.search(r'(^|\n)[ \t]*(login )?password[^\n]*:[ \t]*$', tail):
                    if password is None:
                        p.kill()
                        return False, 'password needed'
                    answer = password
                if answer is not None:
                    asked += 1
                    if asked > 6:
                        p.kill()
                        return False, 'Wrong user name or password (authentication failed)'
                    p.stdin.write((answer + '\n').encode())
                    p.stdin.flush()
                    buf += b'\n'
            elif p.poll() is not None:
                break
        p.wait(5)
    finally:
        for f in (p.stdin, p.stdout):
            try:
                f.close()
            except OSError:
                pass
    text = buf.decode('utf-8', 'replace')
    text = text.replace(password, '***') if password else text
    return p.returncode == 0, text


def local_path(uri):
    """Where gvfs shows a mounted share (/run/user/UID/gvfs/smb-share:server=…,share=…)."""
    try:
        r = subprocess.run(['gio', 'info', '-a', 'standard::name', uri], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    m = re.search(r'^local path:\s*(.+)$', r.stdout or '', re.M)
    return m.group(1).strip() if m else None


def net_open(net, user=None, password=None, remember=False, at_home=resolves, ts_running=None, mount=gio_mount,
             lookup=keyring_lookup, store=keyring_store, path_of=local_path, state=None):
    """Mounts one share (or a computer's list of shares) and answers {path} or {ok: false, need, error, user, wake}."""
    st = state if state is not None else read_state()
    if ts_running is None:
        ts_running = bool((st.get('tailscale') or {}).get('running'))
    host, where = choose_address(net, at_home, ts_running)
    if not host:
        return dict(classify('', where), ok=False, wake=bool(net.get('wake')))
    share = net.get('share') or ''
    uri = smb_uri(host, share)
    already = path_of(uri)
    if already and os.path.isdir(already):
        return {'ok': True, 'path': already, 'where': where, 'uri': uri}
    user = user or (st.get('users') or {}).get(host.lower()) or ''
    pw = password if password is not None else (lookup(host, share, user) if user else None)
    if pw is None and not user:
        # First time: try without signing in (an open share), then ask.
        ok, text = mount(uri, None, None)
    else:
        ok, text = mount(uri, user, pw)
    if not ok:
        c = classify(text, where)
        if text == 'password needed':
            c = {'need': 'password', 'error': 'Sign in to open it' if password is None else 'Wrong user name or password'}
        return dict(c, ok=False, user=user, wake=bool(net.get('wake')), where=where)
    kept = None
    if user:
        try:
            remember_user(host, user)
        except OSError:
            pass
    if remember and password:
        try:
            store(host, share, user, password)
            kept = True
        except OSError as e:
            kept = False
            text = str(e)
    return {'ok': True, 'path': path_of(uri) or '', 'where': where, 'uri': uri, 'remembered': kept}


# ------------------------------------------------------------------ connect automatically

def runtime_path():
    base = os.environ.get('XDG_RUNTIME_DIR') or tempfile.gettempdir()
    return os.path.join(base, 'hearth-files-autoconnect.json')


def _read_runtime(path):
    try:
        with open(path) as f:
            v = json.load(f)
        return v if isinstance(v, dict) else {}
    except (OSError, ValueError):
        return {}


def autoconnect(entries, at_home=resolves, probe=answers_smb, mount=None, lookup=keyring_lookup, path_of=local_path,
                state=None, runtime=None, budget=60):
    """Opens the shared folders picked to connect by themselves ({key: net}), in the background, never asking:
    {key: "open" | "opened" | "asleep" | "password" | "off" | "skipped" | "error"}. A device that doesn't answer on
    445 within 1.5 s is "asleep" (tried again next run); "password" and "off" are left alone until the owner opens
    the share himself (kept in the runtime file, so they're tried again after the next sign-in)."""
    st = state if state is not None else read_state()
    rt_path = runtime or runtime_path()
    rt = _read_runtime(rt_path)
    gave_up = dict(rt.get('gaveUp') or {})
    ts_running = bool((st.get('tailscale') or {}).get('running'))
    mount = mount or (lambda uri, user, pw: gio_mount(uri, user, pw, timeout=20))
    results, lock = {}, threading.Lock()

    def one(key, net):
        try:
            if not isinstance(net, dict) or not net.get('share') or not (net.get('home') or net.get('away')):
                res = 'skipped'
            elif key in gave_up:
                res = 'skipped'
            else:
                host, where = choose_address(net, at_home, ts_running)
                if not host:
                    res = 'asleep'
                elif path_of(smb_uri(host, net['share'])):
                    res = 'open'
                elif not probe(host, SMB_PORT, 1.5):
                    res = 'asleep'
                else:
                    r = net_open(net, at_home=lambda h: h == host and where == 'home', ts_running=ts_running, mount=mount,
                                 lookup=lookup, store=lambda *a: None, path_of=path_of, state=st)
                    if r.get('ok'):
                        res = 'opened'
                    else:
                        res = {'password': 'password', 'off': 'off'}.get(r.get('need'), 'asleep' if r.get('need') == 'asleep' else 'error')
        except Exception:      # one share's trouble never stops the others
            res = 'error'
        with lock:
            results[key] = res
            if res in ('password', 'off'):
                gave_up[key] = res

    threads = []
    for key, net in (entries or {}).items():
        t = threading.Thread(target=one, args=(key, net), daemon=True)
        t.start()
        threads.append(t)
    end = time.time() + budget
    for t in threads:
        t.join(max(0.0, end - time.time()))
    for key in entries or {}:
        results.setdefault(key, 'error')
    try:
        os.makedirs(os.path.dirname(rt_path), exist_ok=True)
        fd, tmp = tempfile.mkstemp(dir=os.path.dirname(rt_path), prefix='.hearth-files-autoconnect-')
        with os.fdopen(fd, 'w') as f:
            json.dump({'gaveUp': gave_up, 'last': results, 'at': int(time.time())}, f)
        os.replace(tmp, rt_path)
    except OSError:
        pass
    return results


def forget_gave_up(key, runtime=None):
    """The owner opened it himself (or turned the switch on again): try it again from the next run."""
    path = runtime or runtime_path()
    rt = _read_runtime(path)
    if key in (rt.get('gaveUp') or {}):
        rt['gaveUp'].pop(key, None)
        try:
            with open(path + '.tmp', 'w') as f:
                json.dump(rt, f)
            os.replace(path + '.tmp', path)
        except OSError:
            pass
