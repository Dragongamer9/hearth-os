"""Hearth Link's Decky backend (docs/HEARTH-LINK-PLUGIN.md): your Hearth devices, game copies between them, Hearth
Storage and pairing, in Steam Game Mode's Quick Access.

Runs as you (no root flag) inside Decky Loader and only talks to Hearth Link on this device: the service's own socket,
<runtime dir>/hearth-link/link.sock (hosts/hearth-link ipc.rs; one JSON line each way, the folder 0700 and the socket
this user's, the other end checked to be this user too). Only the commands in CALLS are ever sent, each with only its
checked arguments (the rest dropped); nothing that unpairs, deletes or shows a password is in it (those stay in
Settings > Hearth Link). The one setting it changes is saves that follow you (storage.saves.set, sent as Hearth Link's
storage.set with only savesFollow).

The sync switches (Decky plugins, Hearth settings) are this device's own, start off and live in link_sync.py. The status the panel gets is cut short (short()): never the device's ticket,
its relay, its settings or addresses.

When Hearth Link isn't running, the panel gets {'running': False, ...} and the last status it saw (last.json, the same
short form), read only. 'Start Hearth Link' asks systemd for your own hearth-link.service, nothing else.
Nothing is sent anywhere else; the log gets command names and reasons, never codes, ids or paths.
"""
import asyncio
import json
import os
import pwd
import re
import shutil
import socket
import stat
import struct
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import link_sync  # noqa: E402

try:
    import decky  # provided by Decky Loader
except ImportError:  # tests
    decky = None

# Hearth OS only, and the load mark for plugin updates (hosts/decky-shared/hearth_only.py, docs/PLUGIN-UPDATES.md):
# next to main.py when packed, in hosts/decky-shared in the repo.
import sys as _sys  # noqa: E402
_here = os.path.dirname(os.path.abspath(__file__))
for _p in (os.path.join(_here, '..', 'decky-shared'), _here):
    if _p not in _sys.path:
        _sys.path.insert(0, _p)
import hearth_only  # noqa: E402

PROGRAM = '/usr/bin/hearth-link'
UNIT = 'hearth-link.service'
NOT_RUNNING = "Hearth Link isn't running on this device"
NOT_ALLOWED = "That isn't something the Hearth Link plugin asks"
MAX_ANSWER = 8 * 1024 * 1024
MAX_PLUGIN_ANSWER = 72 * 1024 * 1024                 # decky.fetch: a plugin's files as base64 (40 MiB at most)
SYNC_EVERY = 30                                      # seconds between sync rounds while a switch is on

PEER = re.compile(r'^[0-9a-f]{64}$')                 # a device id (hearth-link discovery.rs: 64 hex digits)
HEXID = re.compile(r'^[0-9a-f]{8,64}$')              # transfer and offer ids (crate::hex of random bytes)
LAN_CODE = re.compile(r'^[0-9]{6}$')                 # pair.rs: a 6-digit code
NET_CODE = re.compile(r'^[A-Za-z0-9 -]{4,24}$')      # shortcode.rs: letters and digits, typed with spaces or dashes


def log(msg):
    if decky:
        decky.logger.info(msg)


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth-link')


def runtime_dir():
    d = os.environ.get('XDG_RUNTIME_DIR')
    if d:
        return d
    try:
        uid = pwd.getpwnam(os.path.basename(home())).pw_uid if os.getuid() == 0 else os.getuid()
    except KeyError:
        uid = os.getuid()
    return '/run/user/%d' % uid


def socket_path():
    return os.environ.get('HEARTH_LINK_SOCKET') or os.path.join(runtime_dir(), 'hearth-link', 'link.sock')


def game_id(i):
    """A Hearth game id (hearth-core desktop_games::valid_id): `store:rest`, no quotes or control characters."""
    return (isinstance(i, str) and 0 < len(i) <= 200 and ':' in i
            and not any(c in i for c in '"\\') and all(ord(c) >= 32 and ord(c) != 127 for c in i))


def text(v, n=120):
    return ''.join(c for c in str(v or '') if c.isprintable())[:n]


# ---- the allow-list: command -> {argument: check}. A check returns the value to send, or raises ValueError.
def _peer(v):
    if isinstance(v, str) and PEER.match(v):
        return v
    raise ValueError("That isn't a Hearth device")


def _hexid(v):
    if isinstance(v, str) and HEXID.match(v):
        return v
    raise ValueError("That isn't a copy Hearth Link knows")


def _game(v):
    if game_id(v):
        return v
    raise ValueError("That isn't a game Hearth knows")


def _bool(v):
    return v is True


def _strict_bool(v):
    if isinstance(v, bool):
        return v
    raise ValueError('That has to be on or off')


def _title(v):
    return text(v)


def _code(v):
    v = text(v, 24).strip()
    if LAN_CODE.match(v) or NET_CODE.match(v):
        return v
    raise ValueError('That code doesn\'t look right: it\'s the 6 numbers the other device shows, or its internet code')


# Optional arguments end with '?'.
CALLS = {
    'status': {},
    'local.games': {},
    'games': {'peer': _peer, 'refresh?': _bool},
    'send': {'peer': _peer, 'game': _game},
    'get': {'peer': _peer, 'game': _game, 'title?': _title},
    'offer.accept': {'id': _hexid},
    'offer.decline': {'id': _hexid},
    'transfer.pause': {'id': _hexid},
    'transfer.resume': {'id': _hexid},
    'transfer.cancel': {'id': _hexid},
    'transfer.clear': {},
    'sync.now': {'peer?': _peer},
    'pair.open': {'internet?': _bool},
    'pair.close': {},
    'pair.join': {'code': _code, 'id?': _peer},
    'storage.wake': {'server': _peer},
    'storage.cache.status': {},
    'storage.cache.get': {'game': _game},
    'storage.saves.status': {},
    'storage.updates.status': {},
    'storage.devices': {},
    'storage.saves.set': {'follow': _strict_bool},
}
# Sent to Hearth Link as another command (only these arguments).
SENT_AS = {'storage.saves.set': lambda a: ('storage.set', {'savesFollow': a['follow']})}
TIMEOUTS = {'status': 8, 'games': 45, 'send': 30, 'get': 30, 'sync.now': 70, 'pair.join': 40, 'pair.open': 20,
            'storage.cache.status': 20, 'storage.cache.get': 30, 'storage.devices': 15}


def request(cmd, args):
    """The checked request for one of CALLS, or raises ValueError."""
    spec = CALLS.get(cmd)
    if spec is None:
        raise ValueError(NOT_ALLOWED)
    args = args if isinstance(args, dict) else {}
    out = {}
    for key, check in spec.items():
        name, optional = key.rstrip('?'), key.endswith('?')
        if name not in args or args[name] is None:
            if optional:
                continue
            raise ValueError("Missing '%s'" % name)
        out[name] = check(args[name])
    if cmd == 'pair.join' and 'id' in out and not LAN_CODE.match(out['code']):
        raise ValueError('The code is 6 digits')
    return out


def link_safe(path, uid=None):
    """The socket and its folder are this user's own (nobody else could have put a socket there)."""
    uid = os.getuid() if uid is None else uid
    try:
        d = os.lstat(os.path.dirname(path))
        s = os.lstat(path)
    except OSError:
        return False
    return (stat.S_ISDIR(d.st_mode) and d.st_uid == uid and not (d.st_mode & 0o077)
            and stat.S_ISSOCK(s.st_mode) and s.st_uid == uid)


def peer_uid(sock):
    try:
        _pid, uid, _gid = struct.unpack('3i', sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize('3i')))
        return uid
    except (OSError, AttributeError, struct.error):
        return None


IDS = [0]


def link_call(cmd, args, path=None, timeout=None, limit=MAX_ANSWER):
    """One request to Hearth Link. None when it isn't running; else {'ok': True, 'data': ...} or {'ok': False, 'error'}."""
    path = path or socket_path()
    if not link_safe(path):
        return None
    IDS[0] += 1
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        s.settimeout(timeout or TIMEOUTS.get(cmd, 10))
        s.connect(path)
        if peer_uid(s) != os.getuid():
            log('hearth link: the socket belongs to someone else; not used')
            return None
        s.sendall((json.dumps({'id': IDS[0], 'cmd': cmd, 'args': args}) + '\n').encode())
        buf = b''
        while b'\n' not in buf:
            chunk = s.recv(65536)
            if not chunk:
                break
            buf += chunk
            if len(buf) > limit:
                return {'ok': False, 'error': 'Hearth Link gave too long an answer'}
    except (OSError, socket.timeout) as e:
        if isinstance(e, (ConnectionRefusedError, FileNotFoundError)):
            return None
        return {'ok': False, 'error': 'Hearth Link took too long to answer' if isinstance(e, socket.timeout) else "Hearth Link didn't answer"}
    finally:
        s.close()
    try:
        v = json.loads(buf.split(b'\n', 1)[0].decode('utf-8', 'replace'))
    except ValueError:
        return {'ok': False, 'error': "Hearth Link didn't answer"}
    return v if isinstance(v, dict) else {'ok': False, 'error': "Hearth Link didn't answer"}


def call(cmd, args=None, link=None):
    """Always a dict: {'ok': True, 'data': ...}, {'ok': False, 'error': ...} or {'ok': False, 'running': False, ...}."""
    try:
        req = request(cmd, args)
    except ValueError as e:
        return {'ok': False, 'error': str(e)}
    sent, sargs = SENT_AS[cmd](req) if cmd in SENT_AS else (cmd, req)
    r = (link or link_call)(sent, sargs)
    if r is None:
        return {'ok': False, 'running': False, 'error': NOT_RUNNING}
    if r.get('ok') is True:
        data = r.get('data')
        if cmd == 'status':
            data = short(data)
        elif cmd in ('local.games', 'games'):
            data = {'games': clean_games((data or {}).get('games') if isinstance(data, dict) else None)}
        elif cmd == 'storage.devices':
            data = clean_devices(data)
        return {'ok': True, 'data': data}
    log('hearth link: %s: %s' % (cmd, text(r.get('error'), 160)))
    return {'ok': False, 'error': text(r.get('error') or "That didn't work", 300)}


# ---- what the panel gets back: short, and nothing secret
def _num(v):
    return v if isinstance(v, (int, float)) and not isinstance(v, bool) else None


def clean_games(raw):
    out = []
    for g in raw if isinstance(raw, list) else []:
        if isinstance(g, dict) and game_id(g.get('id')):
            out.append({'id': g['id'], 'title': text(g.get('title') or g['id']), 'store': text(g.get('store'), 30),
                        'size': _num(g.get('size')) or 0})
    out.sort(key=lambda g: g['title'].lower())
    return out


def clean_job(j):
    if not isinstance(j, dict):
        return None
    return {k: (_num(j.get(k)) if k in ('total', 'done', 'speed', 'eta', 'created') else text(j.get(k), 200))
            for k in ('id', 'peer', 'game', 'title', 'store', 'state', 'total', 'done', 'speed', 'eta', 'error', 'from', 'created')} \
        | {'relay': j.get('relay') is True}


def short(v):
    v = v if isinstance(v, dict) else {}
    me = v.get('me') if isinstance(v.get('me'), dict) else {}
    peers = []
    for p in v.get('peers') or []:
        if isinstance(p, dict) and isinstance(p.get('id'), str):
            peers.append({'id': p['id'], 'name': text(p.get('name') or 'Hearth device', 60), 'online': p.get('online') is True,
                          'lan': p.get('lan') if isinstance(p.get('lan'), bool) else None, 'lastSeen': _num(p.get('lastSeen')) or 0})
    pairing = v.get('pairing') if isinstance(v.get('pairing'), dict) else None
    if pairing:
        pairing = {'code': text(pairing.get('code'), 12) or None, 'until': _num(pairing.get('until')) or 0,
                   'netCode': text(pairing.get('netCode'), 24) or None, 'netUntil': _num(pairing.get('netUntil')) or 0}
    nearby = [{'id': n['id'], 'name': text(n.get('name') or 'Hearth device', 60), 'pairing': n.get('pairing') is True}
              for n in v.get('nearby') or [] if isinstance(n, dict) and isinstance(n.get('id'), str)]
    offers = [{'id': o['id'], 'from': text(o.get('from'), 60), 'title': text((o.get('game') or {}).get('title') if isinstance(o.get('game'), dict) else ''),
               'store': text((o.get('game') or {}).get('store') if isinstance(o.get('game'), dict) else '', 30),
               'size': _num((o.get('game') or {}).get('size') if isinstance(o.get('game'), dict) else 0) or 0}
              for o in v.get('offers') or [] if isinstance(o, dict) and isinstance(o.get('id'), str)]
    sending = [{'to': text(s.get('to'), 60), 'title': text(s.get('title')), 'bytes': _num(s.get('bytes')) or 0}
               for s in v.get('sending') or [] if isinstance(s, dict)]
    st = v.get('storage') if isinstance(v.get('storage'), dict) else {}
    cfg = st.get('config') if isinstance(st.get('config'), dict) else {}
    servers = [{'id': s['id'], 'name': text(s.get('name') or 'Hearth Storage', 60), 'free': _num(s.get('free')),
                'usable': _num(s.get('usable')), 'seen': _num(s.get('at')), 'canWake': s.get('canWake') is True}
               for s in st.get('servers') or [] if isinstance(s, dict) and isinstance(s.get('id'), str)]
    joining = v.get('joining') if isinstance(v.get('joining'), dict) else None
    event = v.get('pairEvent') if isinstance(v.get('pairEvent'), dict) else None
    sync = v.get('sync') if isinstance(v.get('sync'), dict) else {}
    return {
        'running': True,
        'me': {'id': text(me.get('id'), 64), 'name': text(me.get('name') or 'This device', 60), 'kind': text(me.get('kind'), 20)},
        'peers': peers, 'nearby': nearby, 'pairing': pairing, 'shortCodes': v.get('shortCodes') is True,
        'joining': {'state': text(joining.get('state'), 20), 'name': text(joining.get('name'), 60)} if joining else None,
        'pairEvent': {'name': text(event.get('name'), 60), 'ok': event.get('ok') is not False, 'error': text(event.get('error'), 200)} if event else None,
        'transfers': [j for j in (clean_job(x) for x in v.get('transfers') or []) if j],
        'offers': offers, 'sending': sending, 'playing': v.get('playing') is True,
        'lastChange': _num(sync.get('lastChange')) or 0,
        'storage': {'server': cfg.get('server') is True, 'savesFollow': cfg.get('savesFollow') is True, 'servers': servers,
                    'jobs': [j for j in (clean_job(x) for x in st.get('jobs') or []) if j]},
    }


def clean_devices(v):
    v = v if isinstance(v, dict) else {}
    rows = []
    for d in v.get('devices') or []:
        if not isinstance(d, dict):
            continue
        b = d.get('battery') if isinstance(d.get('battery'), dict) else None
        rows.append({'name': text(d.get('name') or 'Hearth device', 60), 'me': d.get('me') is True, 'online': d.get('online') is True,
                     'lastSeen': _num(d.get('lastSeen')) or 0, 'free': _num(d.get('free')), 'total': _num(d.get('total')),
                     'battery': _num(b.get('percent')) if b else None, 'version': text(d.get('version'), 30),
                     'upToDate': d.get('upToDate') if isinstance(d.get('upToDate'), bool) else None,
                     'lastBackup': _num(d.get('lastBackup')), 'server': d.get('server') is True})
    return {'devices': rows, 'serverName': text(v.get('serverName'), 60), 'serverOffline': v.get('serverOffline') is True}


def storage(link=None):
    """Hearth Storage's sections in one go: download once (the game cache), saves, update cache, device health. Each
    is its own answer, so one that fails doesn't hide the others."""
    out = {}
    for key, cmd in (('cache', 'storage.cache.status'), ('saves', 'storage.saves.status'), ('updates', 'storage.updates.status'),
                     ('health', 'storage.devices')):
        r = call(cmd, {}, link)
        if r.get('running') is False:
            return {'running': False, 'error': NOT_RUNNING}
        out[key] = r['data'] if r.get('ok') else {'error': r.get('error')}
    c = out['cache'] if isinstance(out['cache'], dict) else {}
    out['cache'] = {'error': c.get('error'), 'enabled': c.get('enabled') is True,
                    'using': text((c.get('using') or {}).get('name'), 60) if isinstance(c.get('using'), dict) else '',
                    'jobs': [j for j in (clean_job(x) for x in c.get('jobs') or []) if j],
                    'available': [{'game': x['game'], 'title': text(x.get('title') or x['game']), 'size': _num(x.get('size')) or 0}
                                  for x in c.get('available') or [] if isinstance(x, dict) and game_id(x.get('game'))][:50],
                    'updates': [{'game': x['game'], 'title': text(x.get('title') or x['game'])}
                                for x in c.get('updates') or [] if isinstance(x, dict) and game_id(x.get('game'))][:50]}
    s = out['saves'] if isinstance(out['saves'], dict) else {}
    out['saves'] = {'error': s.get('error'), 'follow': s.get('follow') is True, 'server': bool(s.get('server')),
                    'games': sorted(({'title': text(g.get('title') or g.get('id')), 'at': _num(g.get('at')) or 0,
                                      'conflict': bool(g.get('conflict')), 'pending': g.get('pending') is True}
                                     for g in s.get('games') or [] if isinstance(g, dict)), key=lambda g: -g['at'])[:30]}
    u = out['updates'] if isinstance(out['updates'], dict) else {}
    out['updates'] = {'error': u.get('error'), 'enabled': u.get('enabled') is True, 'fetching': u.get('fetching') is True,
                      'using': text((u.get('using') or {}).get('name'), 60) if isinstance(u.get('using'), dict) else '',
                      'cached': [{'version': text(x.get('version'), 40), 'at': _num(x.get('at')) or 0, 'verified': len(x.get('verifiedBy') or [])}
                                 for x in u.get('cached') or [] if isinstance(x, dict)][:10]}
    out['running'] = True
    return out


# ---- the last status seen (the read-only view when Hearth Link isn't running)
def load_last():
    try:
        with open(os.path.join(settings_dir(), 'last.json')) as f:
            v = json.load(f)
        return v if isinstance(v, dict) and isinstance(v.get('status'), dict) else None
    except (OSError, ValueError):
        return None


def save_last(status):
    d = settings_dir()
    try:
        os.makedirs(d, exist_ok=True)
        keep = {k: status.get(k) for k in ('me', 'peers', 'storage')}
        tmp = os.path.join(d, 'last.json.tmp')
        with open(tmp, 'w') as f:
            json.dump({'at': int(time.time() * 1000), 'status': keep}, f)
        os.replace(tmp, os.path.join(d, 'last.json'))
    except OSError:
        pass


def status(link=None):
    r = call('status', {}, link)
    if r.get('ok'):
        save_last(r['data'])
        return r['data']
    # Not running, or not answering: the read-only view.
    return {'running': False, 'installed': os.path.exists(PROGRAM), 'error': r.get('error') or NOT_RUNNING, 'last': load_last()}


def clean_env():
    env = dict(os.environ)
    orig = env.pop('LD_LIBRARY_PATH_ORIG', None)
    env.pop('LD_LIBRARY_PATH', None)
    if orig:
        env['LD_LIBRARY_PATH'] = orig
    env['HOME'] = home()
    env['XDG_RUNTIME_DIR'] = runtime_dir()
    env.setdefault('PATH', '/usr/local/bin:/usr/bin:/bin')
    return env


def start_link(run=subprocess.run):
    """Starts your own hearth-link.service (the unit the image installs). Nothing else."""
    if not os.path.exists(PROGRAM):
        return {'ok': False, 'error': "Hearth Link isn't part of this Hearth OS"}
    prog = shutil.which('systemctl', path=clean_env()['PATH']) or '/usr/bin/systemctl'
    try:
        r = run([prog, '--user', 'start', UNIT], capture_output=True, text=True, timeout=20, env=clean_env())
    except (OSError, subprocess.TimeoutExpired):
        return {'ok': False, 'error': "Couldn't start Hearth Link"}
    if r.returncode != 0:
        log('hearth link: start: %s' % text(r.stderr, 160))
        return {'ok': False, 'error': "Couldn't start Hearth Link: see Settings > Hearth Link"}
    return {'ok': True}


# ---- sync between your devices (link_sync.py)
def sync_paths():
    return link_sync.Paths(home(), settings_dir())


def sync_info(link=None):
    """The switches, and plugins on your other devices that aren't here (with the Plugins switch on)."""
    p = sync_paths()
    st = call('status', {}, link)
    peers = st['data']['peers'] if st.get('ok') else []
    return {'switches': link_sync.switches(p), 'available': link_sync.available(p, peers)}


def set_sync(name, on):
    try:
        return {'ok': True, 'switches': link_sync.set_switch(sync_paths(), name, on)}
    except ValueError as e:
        return {'ok': False, 'error': str(e)}
    except OSError as e:
        return {'ok': False, 'error': "Couldn't save that: %s" % (e.strerror or e)}


def sync_round(link=None):
    """One round, when a switch is on and Hearth Link is running (it carries the changes)."""
    p = sync_paths()
    if not any(link_sync.switches(p).values()):
        return {'ok': True, 'skipped': True}
    st = call('status', {}, link)
    if not st.get('ok'):
        return {'ok': False, 'error': st.get('error') or NOT_RUNNING}
    r = link_sync.tick(p, st['data']['me']['id'])
    for e in r['errors']:
        log('hearth link: sync: %s' % text(e, 200))
    return dict(r, ok=True)


def fetch_plugin(folder, link=None):
    """A Decky plugin from one of your devices that has it, packed for Decky's own install (with its prompt):
    {'ok', 'artifact', 'name', 'version', 'hash'}. Only with the Plugins switch on."""
    p = sync_paths()
    if not link_sync.switches(p).get('plugins'):
        return {'ok': False, 'error': 'Turn on Sync Decky plugins first'}
    if not link_sync.valid_folder(folder) or folder in link_sync.HEARTH_PLUGINS:
        return {'ok': False, 'error': "That isn't a plugin Hearth Link copies"}
    st = call('status', {}, link)
    if not st.get('ok'):
        return {'ok': False, 'error': st.get('error') or NOT_RUNNING}
    peers = st['data']['peers']
    want = next((a for a in link_sync.available(p, peers) if a['folder'] == folder), None)
    if want is None:
        return {'ok': False, 'error': "None of your devices has that plugin (or it's here already)"}
    order = [x['id'] for x in peers if x['id'] == want['from'] and x['online']] + [x['id'] for x in peers if x['online'] and x['id'] != want['from']]
    if not order:
        return {'ok': False, 'error': '%s is offline' % want['fromName']}
    err = "Couldn't get that plugin"
    for peer in order[:4]:
        r = (link or (lambda c, a: link_call(c, a, timeout=120, limit=MAX_PLUGIN_ANSWER)))('decky.fetch', {'peer': _peer(peer), 'folder': folder})
        if r is None:
            return {'ok': False, 'error': NOT_RUNNING}
        if r.get('ok') is True:
            try:
                return link_sync.pack_zip(p, folder, r.get('data'))
            except OSError as e:
                return {'ok': False, 'error': "Couldn't keep it here: %s" % (e.strerror or e)}
        err = text(r.get('error') or err, 200)
    log('hearth link: plugin from another device: %s' % err)
    return {'ok': False, 'error': err}


async def _bg(fn, *a):
    return await asyncio.get_running_loop().run_in_executor(None, fn, *a)


class Plugin:
    async def hearth_os(self):
        """Whether this is Hearth OS (the frontend shows the Hearth OS panel instead of the plugin when not)."""
        return hearth_only.status(log, 'Hearth Link')

    async def status(self):
        return await _bg(status)

    async def call(self, cmd, args=None):
        return await _bg(call, cmd, args)

    async def storage(self):
        return await _bg(storage)

    async def start_link(self):
        return await _bg(start_link)

    async def sync_info(self):
        return await _bg(sync_info)

    async def set_sync(self, name, on):
        r = await _bg(set_sync, name, on)
        if r.get('ok') and on:
            await _bg(sync_round)
        return r

    async def fetch_plugin(self, folder):
        return await _bg(fetch_plugin, folder)

    async def _main(self):
        if not hearth_only.status(log, 'Hearth Link')['ok']:
            return   # not Hearth OS: nothing starts (the panel says why)
        log('Hearth Link plugin loaded')
        self._sync = asyncio.get_running_loop().create_task(self._sync_loop())
        self._loaded = asyncio.get_running_loop().create_task(hearth_only.mark_loaded_later(decky))

    async def _sync_loop(self):
        while True:
            try:
                await _bg(sync_round)
            except Exception as e:  # never stops the plugin
                log('hearth link: sync round: %s' % text(e, 200))
            await asyncio.sleep(SYNC_EVERY)

    async def _unload(self):
        t = getattr(self, '_sync', None)
        if t:
            t.cancel()
