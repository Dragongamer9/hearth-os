"""Hearth Classic's backend (docs/HEARTH-LOOK.md): the old Hearth Game Mode look on top of Steam's Game Mode.

Runs as you inside Decky Loader. It keeps the plugin's settings (settings.json in the plugin's Decky settings folder:
atomic writes, the last good copy as settings.json.bak, merge-on-write, a damaged file never replaced with defaults;
docs/SETTINGS-PERSISTENCE.md), hands the frontend Hearth's skin and pictures, Hearth's home layout and Hearth Sync's
shortcuts, and pictures from this device. Only on Hearth OS (hosts/decky-shared/hearth_only.py): anywhere else nothing
starts and every call answers "not Hearth OS".

  nimbus --steam-mode-data home   Hearth's home layout (hosts/hearth-core steam_takeover.rs), asked at most once a
                                  minute; nothing keeps running.

Nothing is sent anywhere and nothing listens on the network. The settings only ever change when the person changes them
in the panel; the plugin never writes them by itself.
"""
import asyncio
import base64
import json
import os
import shutil
import subprocess
import sys
import time

try:
    import decky
except ImportError:  # tests
    decky = None

# Hearth OS only, and the load mark for plugin updates (hosts/decky-shared/hearth_only.py): next to main.py when packed,
# in hosts/decky-shared in the repo.
_here = os.path.dirname(os.path.abspath(__file__))
for _p in (os.path.join(_here, '..', 'decky-shared'), _here):
    if _p not in sys.path:
        sys.path.insert(0, _p)
import hearth_only  # noqa: E402

NAME = 'Hearth Classic'
DEFAULTS = {'look': 'steam', 'home': True, 'sounds': False, 'wallpaper': True, 'menu': True, 'menuTop': True}
HOME_TIMEOUT = 20
HOME_EVERY = 60
ART_LIMIT = 8 * 1024 * 1024
NOT_HEARTH = {'ok': False, 'error': 'not Hearth OS'}


def log(msg):
    if decky:
        try:
            decky.logger.info(msg)
        except Exception:
            pass


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth-look')


def plugin_dir():
    d = getattr(decky, 'DECKY_PLUGIN_DIR', None) if decky else None
    return d or _here


# ---- settings the safe way (the same rules and file names as Hearth OS's hearth_settings_io.py and Hearth Sync's)

def _safe_write(path, data):
    """Atomic: a temporary file, fsync, rename; the previous good file kept as PATH.bak. Under a lock."""
    import fcntl
    import tempfile
    d = os.path.dirname(path)
    os.makedirs(d, exist_ok=True)
    lock = os.open(os.path.join(d, '.' + os.path.basename(path) + '.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            with open(path, 'rb') as f:
                cur = f.read()
        except OSError:
            cur = None
        if cur == data:
            return
        good_cur = None
        if cur is not None:
            try:
                json.loads(cur)
                good_cur = cur
            except ValueError:
                good_cur = None   # a damaged file never becomes the .bak (the last good copy stays there)
        for target, content in ((path + '.bak', good_cur), (path, data)):
            if content is None or not content.strip():
                continue
            fd, tmp = tempfile.mkstemp(prefix='.' + os.path.basename(target) + '.', suffix='.tmp', dir=d)
            try:
                with os.fdopen(fd, 'wb') as f:
                    f.write(content)
                    f.flush()
                    os.fsync(f.fileno())
                os.replace(tmp, target)
            except BaseException:
                try:
                    os.unlink(tmp)
                except OSError:
                    pass
                raise
        try:
            dfd = os.open(d, os.O_RDONLY)
            os.fsync(dfd)
            os.close(dfd)
        except OSError:
            pass
    finally:
        os.close(lock)


def clean_settings(raw):
    """Only known keys with the right types; anything else is the default (look: "steam", i.e. off)."""
    r = raw if isinstance(raw, dict) else {}
    out = dict(DEFAULTS)
    if r.get('look') in ('steam', 'classic'):
        out['look'] = r['look']
    for k in ('home', 'sounds', 'wallpaper', 'menu', 'menuTop'):
        if isinstance(r.get(k), bool):
            out[k] = r[k]
    return out


def read_settings():
    """(settings, state): state is 'ok', 'new' (no file yet: the defaults, nothing written), 'restored' (the file was
    damaged and settings.json.bak was put back) or 'damaged' (neither could be read: the defaults are used but the
    damaged file is left alone, never overwritten with defaults)."""
    p = os.path.join(settings_dir(), 'settings.json')
    exists = os.path.exists(p)
    for i, path in enumerate((p, p + '.bak')):
        try:
            with open(path) as f:
                v = json.load(f)
        except (OSError, ValueError):
            continue
        if not isinstance(v, dict):
            continue
        if i == 1:
            try:
                _safe_write(p, json.dumps(v).encode())
                log('settings.json was damaged; put back the last good copy')
            except OSError:
                pass
            return clean_settings(v), 'restored'
        return clean_settings(v), 'ok'
    return dict(DEFAULTS), ('damaged' if exists else 'new')


def write_settings(change):
    """Merges the person's change into what's saved (never into defaults when the file is damaged: the damaged file is
    kept as settings.json.damaged first) and saves it. Returns the saved settings."""
    if not isinstance(change, dict):
        raise ValueError('not a settings change')
    cur, state = read_settings()
    p = os.path.join(settings_dir(), 'settings.json')
    if state == 'damaged':
        try:
            shutil.copyfile(p, p + '.damaged')
        except OSError:
            pass
    # The file may hold keys a newer version wrote: they're kept (merge-on-write).
    try:
        with open(p) as f:
            raw = json.load(f)
    except (OSError, ValueError):
        raw = None
    # Only what's saved plus the person's change: the defaults are never written into the file.
    merged = dict(raw) if isinstance(raw, dict) else {}
    for k, v in change.items():
        if k not in DEFAULTS:
            continue
        if clean_settings({k: v})[k] != v or type(v) is not type(DEFAULTS[k]):
            raise ValueError('not a valid value for %s' % k)
        merged[k] = v
    _safe_write(p, json.dumps(merged, indent=1, sort_keys=True).encode())
    return clean_settings(merged)


# ---- Hearth's skin and pictures

def data_url(path, limit=ART_LIMIT):
    try:
        if os.path.getsize(path) > limit:
            return ''
        with open(path, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    mime = kind_of(data) or ({'.svg': 'image/svg+xml'}.get(os.path.splitext(path)[1].lower()) if b'<svg' in data[:2048] else None)
    return 'data:%s;base64,%s' % (mime, base64.b64encode(data).decode()) if mime else ''


def kind_of(data):
    if data[:8] == b'\x89PNG\r\n\x1a\n':
        return 'image/png'
    if data[:3] == b'\xff\xd8\xff':
        return 'image/jpeg'
    if data[:4] == b'RIFF' and data[8:12] == b'WEBP':
        return 'image/webp'
    if data[:6] in (b'GIF87a', b'GIF89a'):
        return 'image/gif'
    return None


def brand(pdir=None):
    """The skin's CSS and Hearth's logo and wallpaper (data: addresses), from the plugin's own folder."""
    pdir = pdir or plugin_dir()
    css = ''
    for p in (os.path.join(pdir, 'hearth-classic.css'), os.path.join(pdir, 'defaults', 'hearth-classic.css')):
        try:
            with open(p) as f:
                css = f.read()
            break
        except OSError:
            continue
    wall = os.path.join(pdir, 'assets', 'wallpaper.jpg')
    if not os.path.isfile(wall):
        wall = '/usr/share/backgrounds/hearth/hearth-embers.jpg'
    return {'css': css, 'logo': data_url(os.path.join(pdir, 'assets', 'logo.svg')), 'wallpaper': data_url(wall)}


def art_roots(home_dir=None):
    """Where pictures may be read from: Hearth's library and art cache, Steam's library cache and its custom pictures."""
    h = home_dir or home()
    out = [os.path.join(h, '.local', 'share', 'nimbus')]
    for steam in (os.path.join(h, '.local', 'share', 'Steam'), os.path.join(h, '.steam', 'steam'),
                  os.path.join(h, '.var', 'app', 'com.valvesoftware.Steam', '.local', 'share', 'Steam')):
        out += [os.path.join(steam, 'appcache', 'librarycache'), os.path.join(steam, 'userdata')]
    return [os.path.realpath(r) for r in out]


def read_art(ref, home_dir=None):
    """A picture on this device as a data: address ('' when it isn't one, is too big or is outside art_roots)."""
    if not isinstance(ref, str) or not ref or len(ref) > 4096:
        return ''
    path = ref[7:] if ref.startswith('file://') else ref
    if not path.startswith('/'):
        return ''
    real = os.path.realpath(path)
    if not any(real == r or real.startswith(r + os.sep) for r in art_roots(home_dir)):
        return ''
    try:
        if not os.path.isfile(real) or os.path.getsize(real) > ART_LIMIT:
            return ''
        with open(real, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    mime = kind_of(data)
    return 'data:%s;base64,%s' % (mime, base64.b64encode(data).decode()) if mime else ''


# ---- Hearth's home layout and Hearth Sync's shortcuts

def find_program(name):
    for c in ('/usr/bin/' + name, '/usr/local/bin/' + name, shutil.which(name)):
        if c and os.path.isfile(c) and os.access(c, os.X_OK):
            return c
    return None


def clean_env():
    env = dict(os.environ)
    orig = env.pop('LD_LIBRARY_PATH_ORIG', None)
    env.pop('LD_LIBRARY_PATH', None)
    if orig:
        env['LD_LIBRARY_PATH'] = orig
    env['HOME'] = home()
    env.setdefault('PATH', '/usr/local/bin:/usr/bin:/bin')
    return env


def sync_shortcuts():
    """Hearth Sync's shortcuts (Hearth id -> Steam app id), read only, from its Decky settings folder."""
    root = os.path.dirname(settings_dir().rstrip('/'))
    for name in ('decky-hearth-sync', 'Hearth Sync'):
        try:
            with open(os.path.join(root, name, 'shortcuts.json')) as f:
                m = json.load(f)
        except (OSError, ValueError):
            continue
        if isinstance(m, dict):
            return {k: v for k, v in m.items() if isinstance(k, str) and isinstance(v, int) and not isinstance(v, bool) and 0 < v < 2 ** 32}
    return {}


HOME_CACHE = {'at': 0.0, 'result': None}


def _nice():
    try:
        os.nice(10)
    except OSError:
        pass


def home_layout(run=subprocess.run, now=None, finder=None):
    """{ok, home, shortcuts} or {ok: False, error}. Asked at most every HOME_EVERY seconds."""
    now = time.time() if now is None else now
    c = HOME_CACHE
    if c['result'] is not None and now - c['at'] < HOME_EVERY:
        return dict(c['result'], shortcuts=sync_shortcuts())
    app = (finder or find_program)('nimbus')
    if not app:
        res = {'ok': False, 'error': "Hearth's program isn't installed"}
    else:
        try:
            r = run([app, '--steam-mode-data', 'home'], capture_output=True, text=True, timeout=HOME_TIMEOUT,
                    env=clean_env(), preexec_fn=_nice)
            lines = (r.stdout or '').strip().splitlines()
            v = json.loads(lines[-1]) if lines else None
            res = {'ok': True, 'home': v} if isinstance(v, dict) and 'error' not in v else \
                  {'ok': False, 'error': (v or {}).get('error', 'no answer') if isinstance(v, dict) else 'no answer'}
        except (OSError, subprocess.TimeoutExpired, ValueError) as e:
            res = {'ok': False, 'error': "Hearth didn't answer (%s)" % type(e).__name__}
    if not res['ok']:
        log('home: %s; the home uses Steam\'s library' % res['error'])
    c.update(at=now, result=res)
    return dict(res, shortcuts=sync_shortcuts())


class Plugin:
    async def hearth_os(self):
        """Whether this is Hearth OS (the frontend shows the Hearth OS panel instead of the plugin when not)."""
        return hearth_only.status(log, NAME)

    async def get_settings(self):
        if not hearth_only.check()[0]:
            return dict(DEFAULTS)
        return read_settings()[0]

    async def set_settings(self, change):
        """Only from the panel, when the person changes something."""
        if not hearth_only.check()[0]:
            return NOT_HEARTH
        try:
            return {'ok': True, 'settings': write_settings(change)}
        except (OSError, ValueError) as e:
            log('settings not saved: %s' % e)
            return {'ok': False, 'error': "Couldn't save (%s)" % type(e).__name__}

    async def brand(self):
        if not hearth_only.check()[0]:
            return None
        return brand()

    async def home(self):
        if not hearth_only.check()[0]:
            return NOT_HEARTH
        return home_layout()

    async def art(self, ref):
        if not hearth_only.check()[0]:
            return ''
        return read_art(ref)

    async def report(self, line):
        if isinstance(line, str):
            log('frontend: ' + line[:400])

    async def _main(self):
        if not hearth_only.status(log, NAME)['ok']:
            return   # not Hearth OS: nothing starts (the panel says why)
        s, state = read_settings()
        log('%s ready (look: %s, settings %s)' % (NAME, s['look'], state))
        self._loaded = asyncio.get_running_loop().create_task(hearth_only.mark_loaded_later(decky))

    async def _unload(self):
        t = getattr(self, '_loaded', None)
        if t:
            t.cancel()
