"""Hearth Link's sync switches (docs/HEARTH-LINK-PLUGIN.md, "Sync between your devices"): Decky plugins and their
settings, and Hearth's portable settings, following you across your paired Hearth devices.

Each switch is this device's own and starts OFF: nothing changes by itself. With a switch on, this device shares the
keys it covers and takes what your other devices changed, one key at a time, newest change wins (each key carries the
time it changed). Never a whole file: a key that came from another device is merged into the latest file under its
lock, atomically, with FILE.bak (hearth_settings_io, docs/SETTINGS-PERSISTENCE.md). Keys that look like sign-ins
(token, password, secret, ...) are never shared. A value that's removed on one device is never removed on another.

How it travels: Hearth Link's own sync (hosts/hearth-link sync.rs) carries prefs.json's "local" keys as fields
g/x/<ns>/<key>, and puts what came from another device in prefs.json's "remote" (and "local", so it isn't taken for a
change made here). This file writes "local" only for switches that are on, and applies "remote" only then.

Decky plugins from the store aren't installed by this file: the panel asks Hearth Link for the plugin's files from a
device that has it (decky.fetch), they're checked again here and packed as a zip, and Decky Loader installs it after
its own "Install ...?" prompt on this device.
"""
import base64
import hashlib
import io
import json
import os
import re
import sys
import time
import zipfile

try:
    import hearth_settings_io as hio          # packed next to main.py (package.sh)
except ImportError:                           # Hearth OS's own copy
    sys.path.insert(0, '/usr/share/nimbus/desktop/shell')
    import hearth_settings_io as hio          # noqa: E402

# Plugins every Hearth OS device has or had from Hearth itself (hearth-decky's PLUGINS and ThemeDeck, the older Hearth
# plugin and the retired Hearth Music): never offered to install from another device, and only their settings.json
# can be shared.
HEARTH_PLUGINS = ('decky-hearth-sync', 'decky-hearth-link', 'decky-hearth-music', 'decky-hearth', 'ThemeDeck')
# Not shared with the plugins switch: Hearth Link's plugin keeps its switches and state per device; Hearth Music is
# retired (docs/THEMEDECK.md), so its settings no longer travel; ThemeDeck's files are songs on this device (paths to
# your music, its own downloads and previews), which mean nothing on another device, so nothing of it travels.
NOT_SHARED_SETTINGS = ('decky-hearth-link', 'decky-hearth-music', 'ThemeDeck')
SECRET = re.compile(r'token|passw|secret|api[_-]?key|apikey|auth|cookie|credential|session|private|bearer|cert', re.I)
MAX_VALUE = 32 * 1024
MAX_PLUGIN = 40 << 20
SWITCHES = ('plugins', 'settings')
NS_PLUGINS = 'decky:plugins'


def _wallpaper_ok(v):
    """Only Hearth's own wallpapers travel (a picture file's path means nothing on another device)."""
    return isinstance(v, str) and (v == '' or v.startswith('builtin:') or v.startswith('anim:'))


def _scalar(v):
    return isinstance(v, (str, bool, int, float)) and not (isinstance(v, str) and len(v) > 200)


def _any(v):
    return v is not None


def _bool(v):
    return isinstance(v, bool)


def _between(lo, hi):
    return lambda v: isinstance(v, (int, float)) and not isinstance(v, bool) and lo <= v <= hi


# What "Hearth settings" covers: (ns, file under home, {key: check}). Everything else stays on its device: display
# modes (session.conf, display.sh), audio devices, volume and louder-than-100%, HHD / TDP / power, network, pointer
# speed, power timers, lock on wake, the on-screen keyboard's position.
HEARTH = (
    ('hearth:desktop', '.local/share/nimbus/desktop.json',
     {'look': _scalar, 'mode': _scalar, 'accent': _scalar, 'wallpaper': _wallpaper_ok, 'reduceMotion': _scalar,
      'clock24': _scalar, 'nightLight': _scalar}),
    ('hearth:osk', '.config/nimbus/osk.json',
     {k: _any for k in ('layouts', 'suggestions', 'learn', 'autocap', 'doubleSpace', 'fast', 'shape', 'size', 'theme',
                        'haptics', 'trail')}),
    ('hearth:buttons', '.config/nimbus/buttons.json', {'press': _scalar, 'hold': _scalar}),
    ('hearth:gamemode', '.config/nimbus/game-mode-ui.json', {'nimbus.look': _scalar, 'nimbus.swapAB': _scalar, 'nimbus.swapXY': _scalar}),
    ('hearth:music', '.config/hearth-player/settings.json',
     {'shuffle': _scalar, 'repeat': _scalar, 'subsOff': _scalar, 'subLang': _scalar, 'cue': _scalar}),
)


def now_ms():
    return int(time.time() * 1000)


def h(v):
    return hashlib.sha1(json.dumps(v, sort_keys=True).encode()).hexdigest()


def has_secret(key, value):
    """A key that looks like a sign-in, or a value holding one anywhere inside."""
    if isinstance(key, str) and SECRET.search(key):
        return True
    if isinstance(value, dict):
        return any(has_secret(k, v) for k, v in value.items())
    if isinstance(value, list):
        return any(has_secret(None, v) for v in value)
    return False


def small(v):
    try:
        return len(json.dumps(v)) <= MAX_VALUE
    except (TypeError, ValueError):
        return False


class Paths:
    def __init__(self, home, settings_dir, prefs=None):
        self.home = home
        self.own = settings_dir
        self.prefs = prefs or os.environ.get('HEARTH_LINK_PREFS') or os.path.join(home, '.local/share/nimbus/link/prefs.json')

    def plugins(self):
        return os.path.join(self.home, 'homebrew', 'plugins')

    def decky_settings(self, folder=''):
        return os.path.join(self.home, 'homebrew', 'settings', folder) if folder else os.path.join(self.home, 'homebrew', 'settings')

    def switches(self):
        return os.path.join(self.own, 'link-sync.json')

    def state(self):
        return os.path.join(self.own, 'link-sync-state.json')

    def incoming(self):
        return os.path.join(self.own, 'incoming')


# ---------------------------------------------------------------------------------------------------- switches
def switches(p):
    s = hio.load_json(p.switches(), {})
    return {k: s.get(k) is True for k in SWITCHES}


def set_switch(p, name, on):
    """One switch, merged into the file (the others stay as they are)."""
    if name not in SWITCHES or not isinstance(on, bool):
        raise ValueError("That isn't one of Hearth Link's sync switches")
    hio.merge_json(p.switches(), {name: on})
    return switches(p)


# ---------------------------------------------------------------------------------------------------- what's here
def valid_folder(s):
    return (isinstance(s, str) and 0 < len(s) <= 100 and '/' not in s and not s.startswith('.')
            and all(c.isprintable() for c in s) and s not in ('.', '..'))


def installed(p):
    """{folder: {'name', 'version'}} for every Decky plugin here (Hearth's own too)."""
    out = {}
    try:
        names = sorted(os.listdir(p.plugins()))
    except OSError:
        return out
    for n in names:
        d = os.path.join(p.plugins(), n)
        if not valid_folder(n) or os.path.islink(d) or not os.path.isdir(d):
            continue
        try:
            with open(os.path.join(d, 'plugin.json')) as f:
                meta = json.load(f)
        except (OSError, ValueError):
            continue
        version = ''
        try:
            with open(os.path.join(d, 'package.json')) as f:
                version = str(json.load(f).get('version') or '')[:40]
        except (OSError, ValueError, AttributeError):
            pass
        name = meta.get('name') if isinstance(meta, dict) else None
        out[n] = {'name': str(name or n)[:80], 'version': version}
    return out


def _mtime_ms(path):
    try:
        return int(os.stat(path).st_mtime * 1000)
    except OSError:
        return 0


def sources(p, sw, me=''):
    """Every shared group this device takes part in now: (ns, file or None, {key: check}, current {key: value}, its
    time). A file of None is never written (the list of plugins)."""
    out = []
    if sw.get('settings'):
        for ns, rel, keys in HEARTH:
            path = os.path.join(p.home, rel)
            out.append((ns, path, keys, hio.load_json(path, {}), _mtime_ms(path)))
    if sw.get('plugins'):
        here = installed(p)
        mine = {f: {'name': v['name'], 'version': v['version'], 'from': me} for f, v in here.items() if f not in HEARTH_PLUGINS}
        out.append((NS_PLUGINS, None, {f: _any for f in mine}, mine, max([_mtime_ms(os.path.join(p.plugins(), f, 'plugin.json')) for f in mine] or [0])))
        for folder in sorted(here):
            if folder in NOT_SHARED_SETTINGS:
                continue
            d = p.decky_settings(folder)
            try:
                files = sorted(os.listdir(d))
            except OSError:
                files = []
            names = ['settings.json'] if folder in HEARTH_PLUGINS else [n for n in files if re.match(r'^(settings[\w-]*|config)\.json$', n)]
            for n in names:
                path = os.path.join(d, n)
                cur = hio.load_json(path, {}) if os.path.exists(path) else {}
                keys = {k: _any for k, v in cur.items() if not has_secret(k, v)}
                out.append(('decky:%s:%s' % (folder, n), path, keys, cur, _mtime_ms(path)))
        # Settings that came for a plugin that's here but has no settings file yet.
        for ns in remote_ns(p):
            parts = ns.split(':', 2)
            if len(parts) == 3 and parts[0] == 'decky' and parts[1] in here and parts[1] not in NOT_SHARED_SETTINGS \
                    and not any(o[0] == ns for o in out) and re.match(r'^(settings[\w-]*|config)\.json$', parts[2]) \
                    and (parts[1] not in HEARTH_PLUGINS or parts[2] == 'settings.json'):
                out.append((ns, os.path.join(p.decky_settings(parts[1]), parts[2]), None, {}, 0))
    return out


def remote_ns(p):
    r = hio.load_json(p.prefs, {}).get('remote')
    return list(r) if isinstance(r, dict) else []


# ---------------------------------------------------------------------------------------------------- one round
def tick(p, me='', now=None):
    """Applies what came from your other devices and shares what changed here, for the switches that are on.
    Returns {'applied': n, 'shared': n, 'errors': [...]}."""
    now = now or now_ms()
    sw = switches(p)
    res = {'applied': 0, 'shared': 0, 'errors': []}
    if not any(sw.values()):
        return res
    prefs = hio.load_json(p.prefs, {})
    remote = prefs.get('remote') if isinstance(prefs.get('remote'), dict) else {}
    state = hio.load_json(p.state(), {})
    st_all = state.get('keys') if isinstance(state.get('keys'), dict) else {}
    publish = {}
    for ns, path, keys, cur, mtime in sources(p, sw, me):
        rem = remote.get(ns) if isinstance(remote.get(ns), dict) else {}
        st = st_all.setdefault(ns, {})
        if not isinstance(st, dict):
            st = st_all[ns] = {}
        apply = {}
        hearth = ns.startswith('hearth:')
        if hearth:
            names = set(keys)                                     # only the keys Hearth shares
        elif path is None:
            names = set(cur)                                      # the list of plugins: shared, never written
        else:
            names = set(cur) | set(rem)
        for k in sorted(names):
            if not hearth and (SECRET.search(k) or (k in cur and has_secret(k, cur[k]))):
                continue                                          # a sign-in, say: never shared, never replaced
            check = keys.get(k, _any) if hearth else _any
            r = rem.get(k) if isinstance(rem.get(k), dict) else None
            rv, rt = (r.get('v'), r.get('t') if isinstance(r.get('t'), int) else 0) if r else (None, 0)
            r_ok = r is not None and path is not None and rv is not None and check(rv) and small(rv) and not has_secret(k, rv)
            c_present = k in cur and cur[k] is not None
            c_has = c_present and check(cur[k]) and small(cur[k]) and not has_secret(k, cur[k])   # shareable
            s = st.get(k) if isinstance(st.get(k), dict) else None
            if s is None:
                # First time with this switch on: whichever changed last, the file here or the other device.
                if r_ok and (not c_present or rt >= mtime):
                    if not c_present or h(cur[k]) != h(rv):
                        apply[k] = rv
                    st[k] = {'h': h(rv), 'rt': rt}
                elif c_present:
                    if c_has:
                        publish.setdefault(ns, {})[k] = {'v': cur[k], 't': mtime or now}
                    st[k] = {'h': h(cur[k]), 'rt': max(rt, mtime or now)}   # only a newer change replaces it
                continue
            if c_present and h(cur[k]) != s.get('h'):
                if c_has:                                         # changed here since last time
                    publish.setdefault(ns, {})[k] = {'v': cur[k], 't': now}
                st[k] = {'h': h(cur[k]), 'rt': max(rt, now)}
            elif r_ok and rt > (s.get('rt') or 0):
                if not c_present or h(cur[k]) != h(rv):
                    apply[k] = rv
                st[k] = {'h': h(rv), 'rt': rt}
        if apply and path is not None:
            try:
                hio.merge_json(path, apply)                       # these keys only, into the latest file, under its lock
                res['applied'] += len(apply)
            except OSError as e:
                res['errors'].append('%s: %s' % (os.path.basename(path), e.strerror or e))
                for k in apply:                                   # tried again next round
                    st.pop(k, None)
    if publish:
        def put(d):
            loc = d.setdefault('local', {})
            if not isinstance(loc, dict):
                loc = d['local'] = {}
            for ns, kv in publish.items():
                n = loc.setdefault(ns, {})
                if not isinstance(n, dict):
                    n = loc[ns] = {}
                n.update(kv)
        hio.update_json(p.prefs, put)
        res['shared'] = sum(len(v) for v in publish.values())
    state['keys'] = st_all
    hio.update_json(p.state(), lambda d: d.update(state))
    return res


# ---------------------------------------------------------------------------------------------------- plugins
def available(p, peers=()):
    """Plugins on your other devices that aren't here: [{'folder', 'name', 'version', 'from', 'fromName', 'online'}]."""
    if not switches(p).get('plugins'):
        return []
    prefs = hio.load_json(p.prefs, {})
    seen = {}
    for side in ('local', 'remote'):
        n = (prefs.get(side) or {}).get(NS_PLUGINS) if isinstance(prefs.get(side), dict) else None
        for f, e in (n or {}).items() if isinstance(n, dict) else []:
            if isinstance(e, dict) and isinstance(e.get('v'), dict) and valid_folder(f) and f not in HEARTH_PLUGINS:
                seen[f] = e['v']
    here = installed(p)
    names = {x.get('id'): x for x in peers if isinstance(x, dict)}
    out = []
    for f in sorted(seen, key=lambda x: str(seen[x].get('name') or x).lower()):
        if f in here:
            continue
        v = seen[f]
        src = v.get('from') if isinstance(v.get('from'), str) else ''
        peer = names.get(src) or {}
        out.append({'folder': f, 'name': str(v.get('name') or f)[:80], 'version': str(v.get('version') or '')[:40],
                    'from': src, 'fromName': str(peer.get('name') or 'another device')[:60], 'online': peer.get('online') is True})
    return out


def _rel_ok(rel):
    parts = rel.split('/') if isinstance(rel, str) else []
    return (0 < len(rel) <= 400 and not rel.startswith('/') and all(x and x not in ('.', '..') and all(c.isprintable() for c in x) for x in parts))


def pack_zip(p, folder, data):
    """The files another device sent (decky.fetch), checked again, as a zip Decky Loader can install:
    {'ok', 'artifact': 'file://...', 'name', 'version', 'hash'} or {'ok': False, 'error'}."""
    if not valid_folder(folder) or folder in HEARTH_PLUGINS:
        return {'ok': False, 'error': "That isn't a plugin Hearth Link copies"}
    if not isinstance(data, dict) or data.get('folder') != folder or not isinstance(data.get('files'), list):
        return {'ok': False, 'error': 'The other device sent something else'}
    buf, total, names = io.BytesIO(), 0, set()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        for f in data['files']:
            rel = f.get('p') if isinstance(f, dict) else None
            if not _rel_ok(rel) or not isinstance(f.get('b64'), str) or rel in names:
                return {'ok': False, 'error': "The other device sent a path that isn't allowed"}
            try:
                b = base64.b64decode(f['b64'], validate=True)
            except ValueError:
                return {'ok': False, 'error': 'The other device sent a damaged file'}
            total += len(b)
            if total > MAX_PLUGIN:
                return {'ok': False, 'error': 'That plugin is too big to copy this way'}
            names.add(rel)
            info = zipfile.ZipInfo(folder + '/' + rel, date_time=(2020, 1, 1, 0, 0, 0))
            info.external_attr = (0o755 if f.get('x') is True else 0o644) << 16
            z.writestr(info, b)
    if 'plugin.json' not in names:
        return {'ok': False, 'error': "That isn't a Decky plugin (no plugin.json)"}
    raw = buf.getvalue()
    os.makedirs(p.incoming(), mode=0o755, exist_ok=True)
    dst = os.path.join(p.incoming(), folder + '.zip')
    tmp = dst + '.%d.tmp' % os.getpid()
    with open(tmp, 'wb') as fh:
        fh.write(raw)
        fh.flush()
        os.fsync(fh.fileno())
    os.chmod(tmp, 0o644)
    os.replace(tmp, dst)
    return {'ok': True, 'artifact': 'file://' + dst, 'name': str(data.get('name') or folder)[:80],
            'version': str(data.get('version') or '')[:40], 'hash': hashlib.sha256(raw).hexdigest()}
