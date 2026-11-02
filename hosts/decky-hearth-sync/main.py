"""Hearth Sync's backend (docs/HEARTH-SYNC.md): keeps Hearth's non-Steam games in Steam's library.

Runs as you (no root flag) inside Decky Loader and only talks to Hearth's own command line on this device:

  nimbus --list-games --with-cloud   your installed games from Hearth's library (and the Xbox Cloud Gaming tile), as
                                     JSON (hosts/hearth-core desktop_games.rs). Steam's own games are left out here.
  nimbus --play-now ID               what each game's Steam shortcut runs (through WRAPPER): plays it the way Hearth's
                                     Play button does and waits until it has closed, so Steam tracks the play time.
  nimbus --windowed --play ID        the same for a cloud game (Hearth's stream window).

The frontend (src/) does the Steam side: shortcuts, artwork, collections. This file hands it the games, the pictures
(as base64, from Hearth's art cache, nimbus-art:// references, files or https addresses) and keeps the plugin's state:
which Steam shortcut belongs to which Hearth game (shortcuts.json: id -> Steam app id; only shortcuts Hearth Sync made
or took over from the old Hearth plugin are ever changed or removed), what was last applied to each (applied.json),
the store switches (settings.json) and the panel's status (status.json).

Nothing is sent anywhere: web pictures are fetched (when Hearth's art cache hasn't got them) only to hand to Steam.
The log gets counts and reasons, never paths or addresses.
"""
import base64
import hashlib
import json
import os
import pwd
import shlex
import shutil
import subprocess
import time
import urllib.parse
import urllib.request

try:
    import decky  # provided by Decky Loader
except ImportError:  # tests
    decky = None

LIST_TIMEOUT = 60
ART_LIMIT = 10 * 1024 * 1024
# A full re-read of Hearth's library at least this often, even when none of its files changed (seconds).
RESCAN_EVERY = 30 * 60
# Hearth's library source -> (store switch, Steam collection name).
STORES = {'epic': ('epic', 'Epic'), 'gog': ('gog', 'GOG'), 'battlenet': ('battlenet', 'Battle.net'),
          'xbox': ('xbox', 'Xbox'), 'xodus': ('xbox', 'Xbox'), 'xcloud': ('xbox', 'Xbox'),
          'emu': ('emu', 'Emulators'), 'recomps': ('recomps', 'Recomps'), 'recomp': ('recomps', 'Recomps'),
          'beacon': ('beacon', 'Beacon'), 'manual': ('manual', 'Added by hand'), 'device': ('manual', 'Added by hand'),
          'ea': ('ea', 'EA'), 'ubisoft': ('ubisoft', 'Ubisoft'), 'amazon': ('amazon', 'Amazon'), 'itch': ('itch', 'itch.io'),
          'heroic': ('heroic', 'Heroic'), 'lutris': ('lutris', 'Lutris'), 'mods': ('mods', 'Modded games')}
OTHER = ('other', 'Other games')
# The old Hearth plugin's settings folder names (Decky names it after the plugin's folder or its name).
OLD_PLUGIN_SETTINGS = ('decky-hearth', 'Hearth')


def log(msg):
    if decky:
        decky.logger.info(msg)


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth-sync')


def clean_env():
    """The environment for Hearth's programs: Decky's own library path removed (it's a bundled program whose libraries
    break the system's), and the session's runtime folder, which nimbus-mode and Hearth need."""
    env = dict(os.environ)
    orig = env.pop('LD_LIBRARY_PATH_ORIG', None)
    env.pop('LD_LIBRARY_PATH', None)
    if orig:
        env['LD_LIBRARY_PATH'] = orig
    env['HOME'] = home()
    if not env.get('XDG_RUNTIME_DIR'):
        try:
            uid = pwd.getpwnam(os.path.basename(home())).pw_uid if os.getuid() == 0 else os.getuid()
        except KeyError:
            uid = os.getuid()
        env['XDG_RUNTIME_DIR'] = '/run/user/%d' % uid
    env.setdefault('PATH', '/usr/local/bin:/usr/bin:/bin')
    return env


def find_program(name):
    for c in ('/usr/bin/' + name, '/usr/local/bin/' + name, shutil.which(name, path=clean_env()['PATH'])):
        if c and os.path.isfile(c) and os.access(c, os.X_OK):
            return c
    return None


def valid_id(i):
    """The same check as hosts/hearth-core desktop_games::valid_id."""
    return (isinstance(i, str) and 0 < len(i) <= 200 and ':' in i
            and not any(c in i for c in '"\\') and all(ord(c) >= 32 and ord(c) != 127 for c in i))




# What every Hearth shortcut in Steam runs: a tiny /bin/sh script that cleans Steam's environment before Hearth starts.
# Steam starts a non-Steam shortcut with its runtime's libraries (LD_LIBRARY_PATH) and its overlay preloaded
# (LD_PRELOAD); Hearth's program links the system's GTK/WebKit and broke on Steam's older libraries, so 0.6.60's synced
# games didn't start. The overlay library is set aside (HEARTH_STEAM_LD_PRELOAD) and Hearth gives it back to the game
# only (desktop_games.rs steam_env_scrub). Which Steam is running is decided at every launch, not at sync time: inside
# Flathub's Steam sandbox (/.flatpak-info) the program runs on the system through flatpak-spawn --host, which starts it
# with the session's own environment.
WRAPPER = ('if [ -e /.flatpak-info ]; then exec /usr/bin/flatpak-spawn --host /usr/bin/env -u LD_PRELOAD -u LD_LIBRARY_PATH "$@"; fi; '
           'exec /usr/bin/env -u LD_PRELOAD -u LD_LIBRARY_PATH HEARTH_STEAM_LD_PRELOAD="${LD_PRELOAD:-}" "$@"')
WRAPPER_NAME = 'hearth-steam'



ALLOWED_ROOTS = ('/usr/share/', '/var/lib/flatpak/', '/opt/')
# What a picture is, from its first bytes (never from its name): Steam's SetCustomArtworkForApp takes png and jpg only,
# the others are converted to PNG first (to_png).
MIMES = {'png': 'image/png', 'jpg': 'image/jpeg', 'webp': 'image/webp', 'gif': 'image/gif', 'avif': 'image/avif'}
CACHE_EXTS = ('jpg', 'png', 'webp', 'jpeg', 'gif', 'avif')   # hearth-core artcache.rs EXTS
CONVERT_TIMEOUT = 20
# Art that couldn't be shown, counted by reason (the plugin log gets the reasons and counts, never paths or addresses).
ART_FAILS = {}


def art_failed(reason):
    """Counts a picture that couldn't be used and logs it: the first few of each reason, then every 25th."""
    reason = str(reason)[:80]
    n = ART_FAILS.get(reason, 0) + 1
    ART_FAILS[reason] = n
    if n <= 3 or n % 25 == 0:
        log('art: not shown (%s), %d so far; all: %s' % (reason, n, ', '.join('%s=%d' % kv for kv in sorted(ART_FAILS.items()))))
    return None


def art_cache_dir(home_dir=None):
    """Hearth's art cache (hearth-core artcache.rs: ~/.local/share/nimbus/art-cache/<name>.<ext>)."""
    return os.path.join(home_dir or home(), '.local', 'share', 'nimbus', 'art-cache')


def cache_name(url):
    """The cache's name for a web picture (hearth-core artcache::name_of: the address's SHA-1, first 24 hex digits)."""
    return hashlib.sha1(url.encode('utf-8')).hexdigest()[:24]


def cached_file(name, home_dir=None):
    """The file behind nimbus-art://cache/<name> (or a web address's cached copy), or None."""
    if not isinstance(name, str) or len(name) != 24 or any(c not in '0123456789abcdefABCDEF' for c in name):
        return None
    d = art_cache_dir(home_dir)
    for ext in CACHE_EXTS + ('',):
        f = os.path.join(d, name + ('.' + ext if ext else ''))
        if os.path.isfile(f):
            return f
    return None


def roots(home_dir=None):
    h = home_dir or home()
    out = list(ALLOWED_ROOTS) + [os.path.realpath(h).rstrip('/') + '/']
    cache = os.path.realpath(art_cache_dir(h)).rstrip('/') + '/'
    if cache not in out:
        out.append(cache)
    return tuple(out)


def path_allowed(path, home_dir=None):
    """A file in your home (Hearth's art cache is there), the system's shared files or Flatpak's; links are followed
    first, so one pointing out of these places is refused."""
    if not isinstance(path, str) or not path.startswith('/'):
        return False
    real = os.path.realpath(path)
    return any(real.startswith(r) for r in roots(home_dir))


def art_allowed(path, home_dir=None):
    """Only pictures (png, jpg, webp, gif, avif by name) in the allowed places. The bytes are checked when read."""
    return (isinstance(path, str) and os.path.realpath(path).lower().endswith(tuple('.' + e for e in CACHE_EXTS))
            and path_allowed(path, home_dir))


def kind_of(data):
    """png, jpg, webp, gif or avif from a picture's first bytes, else None."""
    if not data:
        return None
    if data.startswith(b'\x89PNG'):
        return 'png'
    if data.startswith(b'\xff\xd8'):
        return 'jpg'
    if data[:4] == b'RIFF' and data[8:12] == b'WEBP':
        return 'webp'
    if data[:4] in (b'GIF8',):
        return 'gif'
    if data[4:8] == b'ftyp' and data[8:12] in (b'avif', b'avis'):
        return 'avif'
    return None


def local_path(ref, home_dir=None):
    """Where a picture reference Hearth gives points on this device: (path, None), or (None, reason).
    Takes absolute paths, ~/ paths, file:// addresses, nimbus-art://file/<path> and nimbus-art://cache/<name>."""
    h = home_dir or home()
    if ref.startswith('nimbus-art://cache/'):
        name = ref[len('nimbus-art://cache/'):].split('?')[0].split('#')[0].strip('/')
        f = cached_file(name, h)
        if not f:
            return None, 'not in the art cache'
        return (f, None) if path_allowed(f, h) else (None, 'outside the allowed folders')
    if ref.startswith('nimbus-art://file/'):
        ref = urllib.parse.unquote(ref[len('nimbus-art://file/'):].split('?')[0].split('#')[0])
        if not ref.startswith('/'):
            ref = '/' + ref
    elif ref.startswith('file://'):
        rest = ref[7:]
        if rest.startswith('localhost/'):
            rest = rest[9:]
        ref = urllib.request.url2pathname(rest.split('?')[0].split('#')[0])
    if ref == '~' or ref.startswith('~/'):
        ref = os.path.join(h, ref[2:])
    if not ref.startswith('/'):
        return None, 'unknown kind of address'
    if not path_allowed(ref, h):
        return None, 'outside the allowed folders'
    return ref, None


def art_bytes(ref, fetch=None, home_dir=None):
    """A picture's bytes and kind: (data, kind, None), or (None, None, reason). Web pictures come from Hearth's art
    cache when it has them (no network), else https ones are fetched; plain http only from the cache."""
    if not isinstance(ref, str) or not ref.strip():
        return None, None, 'no picture'
    ref = ref.strip()
    low = ref.lower()
    data = None
    if low.startswith(('http://', 'https://')):
        f = cached_file(cache_name(ref), home_dir)
        if f and path_allowed(f, home_dir):
            ref, low = f, f.lower()
        elif not low.startswith('https://'):
            return None, None, 'plain http, not cached'
        else:
            try:
                data = (fetch or _fetch)(ref)
            except Exception as e:  # noqa: BLE001 - any failure just means no picture
                return None, None, 'download failed (%s)' % type(e).__name__
    if data is None:
        path, why = local_path(ref, home_dir)
        if not path:
            return None, None, why
        try:
            if os.path.getsize(path) > ART_LIMIT:
                return None, None, 'too big'
            with open(path, 'rb') as f:
                data = f.read()
        except OSError:
            return None, None, 'unreadable file'
    if not data:
        return None, None, 'empty'
    if len(data) > ART_LIMIT:
        return None, None, 'too big'
    k = kind_of(data)
    if not k:
        return None, None, 'not a picture'
    return data, k, None


def _pillow_png(data):
    from PIL import Image  # noqa: WPS433 - optional, only when Decky's Python has it
    import io
    im = Image.open(io.BytesIO(data))
    im.seek(0)
    out = io.BytesIO()
    im.convert('RGBA').save(out, 'PNG')
    return out.getvalue()


def _tool_png(data, finder=None):
    finder = finder or find_program
    for name, args in (('magick', ['-[0]', 'png:-']), ('convert', ['-[0]', 'png:-']),
                       ('ffmpeg', ['-loglevel', 'error', '-i', 'pipe:0', '-frames:v', '1', '-f', 'image2pipe', '-vcodec', 'png', 'pipe:1'])):
        prog = finder(name)
        if not prog:
            continue
        try:
            r = subprocess.run([prog] + args, input=data, capture_output=True, timeout=CONVERT_TIMEOUT, env=clean_env())
        except (OSError, subprocess.TimeoutExpired):
            continue
        if r.returncode == 0 and r.stdout.startswith(b'\x89PNG'):
            return r.stdout
    return None


def to_png(data, converters=None):
    """A webp/gif/avif picture as PNG for Steam: Pillow if Decky's Python has it, else ImageMagick or ffmpeg if the
    system has them. None when nothing here can convert it."""
    for conv in converters if converters is not None else (_pillow_png, _tool_png):
        try:
            out = conv(data)
        except Exception:  # noqa: BLE001 - try the next one
            out = None
        if out and out.startswith(b'\x89PNG'):
            return out
    return None


def read_art(ref, fetch=None, converters=None):
    """A picture as Steam's SetCustomArtworkForApp wants it: {data (base64), type: png|jpg}, or None (logged)."""
    data, k, why = art_bytes(ref, fetch)
    if not data:
        return art_failed('shortcut art: ' + why)
    if k not in ('png', 'jpg'):
        png = to_png(data, converters)
        if not png:
            return art_failed('shortcut art: %s and nothing here converts it' % k)
        data, k = png, 'png'
    return {'data': base64.b64encode(data).decode(), 'type': k}


def _fetch(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'Hearth'})
    with urllib.request.urlopen(req, timeout=20) as r:
        return r.read(ART_LIMIT + 1)



# ---- Settings files the safe way (docs/SETTINGS-PERSISTENCE.md; the same rules and file names as Hearth OS's
# hearth_settings_io.py, kept here because a Decky plugin ships on its own): atomic (temporary file, fsync, rename),
# the last good version kept as FILE.bak, a damaged file put back from it, never replaced with the defaults.
def _safe_write(path, data):
    import fcntl
    import tempfile
    d = os.path.dirname(path)
    os.makedirs(d, exist_ok=True)
    lock = os.open(os.path.join(d, '.' + os.path.basename(path) + '.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            st = os.stat(path)
            with open(path, 'rb') as f:
                cur = f.read()
        except OSError:
            st, cur = None, None
        if cur == data:
            return
        for target, content in (((path + '.bak'), cur), (path, data)):
            if content is None or not content.strip():
                continue
            fd, tmp = tempfile.mkstemp(prefix='.' + os.path.basename(target) + '.', suffix='.tmp', dir=d)
            try:
                with os.fdopen(fd, 'wb') as f:
                    f.write(content)
                    f.flush()
                    os.fsync(f.fileno())
                if st is not None:
                    os.chmod(tmp, st.st_mode & 0o7777)
                    try:
                        os.chown(tmp, st.st_uid, st.st_gid)
                    except OSError:
                        pass
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


def _read_settings(path):
    """The JSON in PATH; damaged: the copy in PATH.bak (and put back). None when neither can be read."""
    for p in (path, path + '.bak'):
        try:
            with open(p) as f:
                v = json.load(f)
        except FileNotFoundError:
            continue
        except (OSError, ValueError):
            continue
        if p != path:
            try:
                _safe_write(path, json.dumps(v).encode())
            except OSError:
                pass
        return v
    return None


def load_json(name, default):
    v = _read_settings(os.path.join(settings_dir(), name))
    return v if isinstance(v, type(default)) else default


def save_json(name, value):
    _safe_write(os.path.join(settings_dir(), name), json.dumps(value).encode())




def shortcut(program, args):
    """What a Steam shortcut runs: {exe, startDir, options}. The exe is /bin/sh with WRAPPER (it exists in the system
    and in Flathub's Steam sandbox); Steam adds the options after the program through a shell, so each is quoted."""
    opts = ['-c', WRAPPER, WRAPPER_NAME, program] + list(args)
    return {'exe': '/bin/sh', 'startDir': home(), 'options': ' '.join(shlex.quote(a) for a in opts)}


def store_of(source):
    """(store switch, collection name) for a Hearth library source."""
    return STORES.get(source if isinstance(source, str) else '', OTHER)


def _ref(v):
    return v.strip() if isinstance(v, str) and v.strip() and len(v) < 4096 else ''


def clean_games(raw, app='/usr/bin/nimbus'):
    """Hearth's list without Steam's own games (they're in Steam already), each with its shortcut's target, A to Z."""
    out = []
    if not isinstance(raw, list):
        return out
    seen = set()
    for g in raw:
        if not isinstance(g, dict) or not valid_id(g.get('id')) or g['id'] in seen:
            continue
        if g.get('source') == 'steam' or g['id'].startswith('steam:'):
            continue
        seen.add(g['id'])
        title = g.get('title') if isinstance(g.get('title'), str) and g.get('title').strip() else g['id']
        src = g.get('source') if isinstance(g.get('source'), str) else ''
        key, name = store_of(src)
        cloud = g.get('cloud') is True
        wide, hero = _ref(g.get('wide')), _ref(g.get('hero'))
        item = {'id': g['id'], 'title': title.strip()[:200], 'source': src, 'store': key, 'storeName': name, 'cloud': cloud,
                'cover': _ref(g.get('cover')), 'wide': wide or hero, 'hero': hero or wide, 'logo': _ref(g.get('logo')),
                'icon': _ref(g.get('icon'))}
        item.update(shortcut(app, ['--windowed', '--play', g['id']] if cloud else ['--play-now', g['id']]))
        out.append(item)
    out.sort(key=lambda g: (g['title'].lower(), g['id']))
    return out


def signature(games):
    """Changes whenever anything a shortcut gets from Hearth changes (titles, targets, pictures, the set of games)."""
    return hashlib.sha1(json.dumps(games, sort_keys=True).encode()).hexdigest()


def library_stamp(home_dir=None):
    """A cheap look at whether Hearth's library may have changed: its files' times (no scan)."""
    d = os.path.join(home_dir or home(), '.local', 'share', 'nimbus')
    out = []
    h = home_dir or home()
    others = [os.path.join(h, '.config', 'minigalaxy', 'config.json'), os.path.join(h, 'GOG Games'), os.path.join(h, '.cache', 'minigalaxy', 'covers')]
    others += [os.path.join(h, r, f[1]) for r in HEROIC_ROOTS for f in HEROIC] + [os.path.join(h, r) for r in EGL_ROOTS]
    for p in [d, os.path.join(d, 'library.json'), os.path.join(d, 'art-cache'), os.path.join(d, 'legendary', 'installed.json'), mods_file(h)] + others:
        try:
            out.append(int(os.stat(p).st_mtime_ns))
        except OSError:
            out.append(0)
    return out


CACHE = {'stamp': None, 'at': 0.0, 'result': None}


def _nice():
    try:
        os.nice(10)
    except OSError:
        pass


def list_games(run=subprocess.run, force=False, now=None, finder=None):
    """Hearth's games for Steam: {ok, error, games, signature}. Re-read only when Hearth's library files changed, every
    RESCAN_EVERY seconds, or when asked (Sync now): the timer's checks are cheap."""
    now = time.time() if now is None else now
    stamp = library_stamp()
    c = CACHE
    if not force and c['result'] and c['result']['ok'] and c['stamp'] == stamp and now - c['at'] < RESCAN_EVERY:
        return c['result']
    app = (finder or find_program)('nimbus')
    if not app:
        return {'ok': False, 'error': "Hearth isn't installed here", 'games': [], 'signature': ''}
    try:
        r = run([app, '--list-games', '--with-cloud'], capture_output=True, text=True, timeout=LIST_TIMEOUT,
                env=clean_env(), preexec_fn=_nice)
    except (OSError, subprocess.TimeoutExpired) as e:
        return {'ok': False, 'error': "Hearth didn't answer (%s)" % type(e).__name__, 'games': [], 'signature': ''}
    try:
        raw = json.loads((r.stdout or '').strip().splitlines()[-1])
    except (ValueError, IndexError):
        return {'ok': False, 'error': "Hearth gave no list", 'games': [], 'signature': ''}
    if not isinstance(raw, list):
        return {'ok': False, 'error': "Hearth gave no list", 'games': [], 'signature': ''}
    SKIPPED.clear()
    games = merge(clean_games(raw, app), other_launchers())
    if SKIPPED:
        log('other launchers: not added: ' + ', '.join('%s=%d' % kv for kv in sorted(SKIPPED.items())))
    vias = {}
    for g in games:
        vias[g['via']] = vias.get(g['via'], 0) + 1
    res = {'ok': True, 'error': None, 'games': games, 'signature': signature(games),
           'via': vias}
    c.update(stamp=stamp, at=now, result=res)
    return res


# ---- Other launchers' games (not in Hearth's library): Minigalaxy (GOG), Heroic (Epic, GOG, Amazon) and Epic's own
# Epic Games Launcher for Linux. Read from their files only; each game gets the same id Hearth uses (gog:<id>,
# epic:<AppName>, amazon:<id>), so a game installed through two launchers is one shortcut: the newest install wins.

# What a shortcut runs for another launcher's game: like WRAPPER, but only Steam's runtime libraries are taken out
# (Steam's overlay stays preloaded for the game). Inside Flathub's Steam it runs on the system, in the game's folder.
DIRECT = ('if [ -e /.flatpak-info ]; then exec /usr/bin/flatpak-spawn --host --directory="$PWD" /usr/bin/env -u LD_LIBRARY_PATH "$@"; fi; '
          'exec /usr/bin/env -u LD_LIBRARY_PATH "$@"')
DIRECT_NAME = 'hearth-sync'
SKIPPED = {}


def skipped(reason):
    """A game another launcher has that can't be added (counted by reason for the log; never paths)."""
    SKIPPED[reason] = SKIPPED.get(reason, 0) + 1


def direct(cmd, cwd):
    """A shortcut that runs `cmd` (a list) in `cwd` through DIRECT."""
    opts = ['-c', DIRECT, DIRECT_NAME] + list(cmd)
    return {'exe': '/bin/sh', 'startDir': cwd or home(), 'options': ' '.join(shlex.quote(a) for a in opts)}


def _json(path):
    try:
        if os.path.getsize(path) > 32 * 1024 * 1024:
            return None
        with open(path, 'rb') as f:
            return json.loads(f.read().decode('utf-8-sig'))
    except (OSError, ValueError):
        return None


def _mtime(*paths):
    for p in paths:
        try:
            return int(os.stat(p).st_mtime)
        except OSError:
            continue
    return 0


def _game(gid, title, source, launch, cwd, installed_at, via, art=None):
    art = art or {}
    return {'id': gid, 'title': title, 'source': source, 'cover': _ref(art.get('cover')), 'wide': _ref(art.get('wide')),
            'hero': _ref(art.get('hero')), 'logo': _ref(art.get('logo')), 'icon': _ref(art.get('icon')),
            'launch': launch, 'cwd': cwd, 'installedAt': installed_at, 'via': via}


def minigalaxy_games(home_dir=None, which=None):
    """Games Minigalaxy installed: its install folder (config.json install_dir, default ~/GOG Games), one folder per
    game with a `gameinfo` (Linux: name, version, dev version, language, id) or a goggame-<id>.info (Windows, run in the
    game's own Wine prefix the way Minigalaxy does: wine start /b /wait /d c:\\game\\<dir> c:\\game\\<exe>)."""
    h = home_dir or home()
    which = which or shutil.which
    out = []
    cfgdir = os.path.join(h, '.config', 'minigalaxy')
    flat = os.path.join(h, '.var', 'app', 'io.github.sharkwouter.Minigalaxy', 'config', 'minigalaxy')
    for cfg, cache in ((cfgdir, os.path.join(h, '.cache', 'minigalaxy')),
                       (flat, os.path.join(h, '.var', 'app', 'io.github.sharkwouter.Minigalaxy', 'cache', 'minigalaxy'))):
        conf = _json(os.path.join(cfg, 'config.json'))
        if conf is None and not os.path.isdir(cfg):
            continue
        root = (conf or {}).get('install_dir') if isinstance((conf or {}).get('install_dir'), str) else os.path.join(h, 'GOG Games')
        try:
            dirs = sorted(os.listdir(root))
        except OSError:
            continue
        for d in dirs:
            g = os.path.join(root, d)
            if not os.path.isdir(g):
                continue
            try:
                files = os.listdir(g)
            except OSError:
                continue
            name, gid, info = '', '', None
            if 'gameinfo' in files:
                try:
                    with open(os.path.join(g, 'gameinfo')) as f:
                        lines = [f.readline().strip() for _ in range(5)]
                    name, gid = lines[0], lines[4]
                except OSError:
                    continue
            for fn in files:
                if fn.startswith('goggame-') and fn.endswith('.info') and fn[8:-5].isdigit():
                    info = _json(os.path.join(g, fn))
                    if isinstance(info, dict):
                        gid = gid or str(info.get('gameId') or fn[8:-5])
                        name = name or str(info.get('name') or '')
                        break
            if not gid.isdigit() or not name:
                continue
            windows = 'unins000.exe' in files or ('prefix' in files and 'start.sh' not in files)
            if not windows and 'start.sh' in files:
                launch = [os.path.join(g, 'start.sh')]
            elif windows and isinstance(info, dict):
                task = next((t for t in info.get('playTasks', []) if isinstance(t, dict) and t.get('isPrimary') and t.get('path')), None)
                status = _json(os.path.join(cfg, 'games', d + '.json')) or {}
                wine = status.get('custom_wine') if isinstance(status.get('custom_wine'), str) and status.get('custom_wine') else 'wine'
                if not task or not (wine.startswith('/') or which(wine)):
                    skipped('Minigalaxy: Windows game without a primary task or Wine')
                    continue
                launch = ['env', 'WINEPREFIX=' + os.path.join(g, 'prefix'), wine, 'start', '/b', '/wait',
                          '/d', 'c:\\game\\' + str(task.get('workingDir') or '.'), 'c:\\game\\' + str(task['path'])]
                launch += shlex.split(task['arguments']) if isinstance(task.get('arguments'), str) else []
            else:
                skipped('Minigalaxy: no start.sh (DOSBox/ScummVM games start in Minigalaxy)')
                continue
            art = {'cover': next((p for p in (os.path.join(cache, 'covers', gid + '.jpg'),) if os.path.isfile(p)), ''),
                   'wide': next((p for p in (os.path.join(cache, 'thumbnails', gid + '.jpg'), os.path.join(g, 'thumbnail.jpg')) if os.path.isfile(p)), '')}
            out.append(_game('gog:' + gid, name, 'gog', launch, g, _mtime(os.path.join(g, 'gameinfo'), g), 'minigalaxy', art))
    return out


HEROIC_ROOTS = ('.config/heroic', '.var/app/com.heroicgameslauncher.hgl/config/heroic')
HEROIC = (('legendary', 'legendaryConfig/legendary/installed.json', 'store_cache/legendary_library.json', 'epic', 'epic:'),
          ('gog', 'gog_store/installed.json', 'store_cache/gog_library.json', 'gog', 'gog:'),
          ('nile', 'nile_config/nile/installed.json', 'store_cache/nile_library.json', 'amazon', 'amazon:'))


def _heroic_art(lib):
    """app name -> art from Heroic's library cache (its art_cover / art_square / art_logo addresses)."""
    out = {}
    games = (lib.get('library') or lib.get('games')) if isinstance(lib, dict) else lib
    for g in games if isinstance(games, list) else []:
        if isinstance(g, dict) and isinstance(g.get('app_name'), str):
            out[g['app_name']] = {'cover': g.get('art_square'), 'wide': g.get('art_cover'), 'hero': g.get('art_background') or g.get('art_cover'),
                                  'logo': g.get('art_logo'), 'title': g.get('title')}
    return out


def heroic_games(home_dir=None):
    """Games Heroic installed (Epic through legendary, GOG, Amazon through nile), launched through Heroic's own
    heroic://launch link (Heroic then starts them with its settings)."""
    h = home_dir or home()
    out = []
    for root in HEROIC_ROOTS:
        base = os.path.join(h, root)
        if not os.path.isdir(base):
            continue
        for runner, inst, libf, source, prefix in HEROIC:
            data = _json(os.path.join(base, inst))
            if data is None:
                continue
            art = _heroic_art(_json(os.path.join(base, libf)))
            entries = data.get('installed') if isinstance(data, dict) and isinstance(data.get('installed'), list) else (
                list(data.values()) if isinstance(data, dict) else data if isinstance(data, list) else [])
            for e in entries:
                if not isinstance(e, dict):
                    continue
                app = e.get('app_name') or e.get('appName') or e.get('id')
                if not isinstance(app, str) or not app or any(c in app for c in '"\\/ ') or len(app) > 120:
                    continue
                a = art.get(app, {})
                title = e.get('title') or a.get('title') or app
                path = next((e[k] for k in ('install_path', 'path') if isinstance(e.get(k), str)), '')
                url = 'heroic://launch?appName=%s&runner=%s' % (urllib.parse.quote(app), runner)
                out.append(_game(prefix + app, str(title), source, ['xdg-open', url], path or h, _mtime(path, os.path.join(base, inst)), 'heroic', a))
    return out


EGL_ROOTS = ('.local/share/com.epicgames.eglv2', '.var/app/com.epicgames.eglv2/data/com.epicgames.eglv2')
EGL_URL = 'com.epicgames.launcher://apps/%s?action=launch&silent=true'


def egl_handler(run=subprocess.run):
    """Whether something on this device opens com.epicgames.launcher:// links (the Epic Games Launcher)."""
    prog = shutil.which('xdg-mime', path=clean_env()['PATH'])
    if not prog:
        return False
    try:
        r = run([prog, 'query', 'default', 'x-scheme-handler/com.epicgames.launcher'], capture_output=True, text=True, timeout=5, env=clean_env())
    except (OSError, subprocess.TimeoutExpired):
        return False
    return bool((r.stdout or '').strip())


def egl_games(home_dir=None, handler=None):
    """Games Epic's own Epic Games Launcher for Linux (preview, Oct 2026) installed. UNVERIFIED: its files aren't
    documented, so this looks (defensively, a few folders deep) under its data folder for install manifests shaped like
    the Windows launcher's (Manifests/*.item: AppName, DisplayName, InstallLocation, bIsIncompleteInstall) and launches
    them through its com.epicgames.launcher:// link, only when something on this device opens those links."""
    h = home_dir or home()
    found = {}
    for root in EGL_ROOTS:
        base = os.path.join(h, root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            depth = dirpath[len(base):].count(os.sep)
            dirnames[:] = [] if depth >= 4 else [d for d in dirnames if d not in ('compat', 'runtime', 'cache', 'Cache', 'webcache', 'logs')]
            for fn in filenames:
                if not fn.endswith(('.item', '.json')):
                    continue
                m = _json(os.path.join(dirpath, fn))
                if not isinstance(m, dict) or not isinstance(m.get('AppName'), str) or not isinstance(m.get('InstallLocation'), str):
                    continue
                app = m['AppName']
                if m.get('bIsIncompleteInstall') is True or not app or any(c in app for c in '"\\/ ') or len(app) > 120:
                    continue
                if not os.path.isdir(m['InstallLocation']):
                    continue
                found[app] = _game('epic:' + app, str(m.get('DisplayName') or app), 'epic', ['xdg-open', EGL_URL % urllib.parse.quote(app)],
                                   m['InstallLocation'], _mtime(os.path.join(dirpath, fn)), 'egl')
    if found and not (handler or egl_handler)():
        for _ in found:
            skipped('Epic Games Launcher: no com.epicgames.launcher link handler')
        return []
    return list(found.values())


def hearth_installed_at(home_dir=None):
    """When Hearth's own Epic installs (legendary) were made: app name -> time (Hearth's list has no install times)."""
    p = os.path.join(home_dir or home(), '.local', 'share', 'nimbus', 'legendary', 'installed.json')
    d = _json(p)
    out = {}
    for app, e in (d.items() if isinstance(d, dict) else []):
        if isinstance(e, dict):
            out['epic:' + app] = _mtime(e.get('install_path') or '', p)
    return out


def merge(items, extras, app='/usr/bin/nimbus', home_dir=None):
    """Hearth's items plus other launchers' games: one per id, the newest install winning; pictures the winner lacks
    are taken from the others. Returns the full item list, A to Z."""
    times = hearth_installed_at(home_dir)
    by = {}
    for it in items:
        it = dict(it)
        it['installedAt'] = times.get(it['id'], 0)
        it['via'] = 'hearth'
        by[it['id']] = it
    for e in extras:
        if not valid_id(e['id']) or not e['title'].strip():
            continue
        key, name = store_of(e['source'])
        wide, hero = e['wide'], e['hero']
        it = {'id': e['id'], 'title': e['title'].strip()[:200], 'source': e['source'], 'store': key, 'storeName': name, 'cloud': False,
              'cover': e['cover'], 'wide': wide or hero, 'hero': hero or wide, 'logo': e['logo'], 'icon': e['icon'],
              'installedAt': e['installedAt'], 'via': e['via']}
        it.update(direct(e['launch'], e['cwd']))
        old = by.get(it['id'])
        if old is None:
            by[it['id']] = it
            continue
        win, lose = (it, old) if it['installedAt'] > old['installedAt'] else (old, it)
        for k in ('cover', 'wide', 'hero', 'logo', 'icon'):
            win[k] = win[k] or lose[k]
        by[it['id']] = win
    out = list(by.values())
    out.sort(key=lambda g: (g['title'].lower(), g['id']))
    return out


# ---- Hearth Mods' "Add to Steam" (hosts/hearth-gaming launch.rs): a modded game's ways in, as its own shortcuts.
# Hearth Mods writes ~/.local/share/nimbus/mods/steam-shortcuts.json ([{id, title, game, kind, launch, cwd, cover, wide,
# hero, logo, icon}]); each entry is a shortcut with a stable id (hearth-mods:<kind>:<game id>), so adding it again
# updates it and taking it out of the file removes it. Only entries that run Hearth Mods' own program are taken.
MODS_KINDS = ('modded', 'mo2', 'hearth')


def mods_file(home_dir=None):
    return os.path.join(home_dir or home(), '.local', 'share', 'nimbus', 'mods', 'steam-shortcuts.json')


def mods_entries(home_dir=None, which=None):
    h = home_dir or home()
    raw = _json(mods_file(h))
    out = []
    for e in raw if isinstance(raw, list) else []:
        if not isinstance(e, dict) or not valid_id(e.get('id')) or not str(e.get('id')).startswith('hearth-mods:') or e.get('kind') not in MODS_KINDS:
            skipped('hearth mods: odd entry')
            continue
        launch = e.get('launch')
        if not isinstance(launch, list) or not launch or not all(isinstance(a, str) and a and len(a) < 4096 for a in launch):
            skipped('hearth mods: no program')
            continue
        prog = launch[0]
        if os.path.basename(prog) != 'hearth-mods' or not os.path.isabs(prog) or not (which or os.path.isfile)(prog):
            skipped('hearth mods: not hearth-mods')
            continue
        title = e.get('title') if isinstance(e.get('title'), str) else ''
        if not title.strip():
            continue
        cwd = e.get('cwd') if isinstance(e.get('cwd'), str) and e.get('cwd').startswith('/') else h
        art = {k: e.get(k) for k in ('cover', 'wide', 'hero', 'logo', 'icon') if isinstance(e.get(k), str)}
        added = e.get('added') if isinstance(e.get('added'), (int, float)) and not isinstance(e.get('added'), bool) else 0
        out.append(_game(e['id'], title.strip()[:200], 'mods', launch, cwd, int(added // 1000), 'hearth-mods', art))
    return out


def other_launchers(home_dir=None):
    """Every other launcher's games; a launcher whose files can't be read is skipped (counted)."""
    out = []
    for f in (minigalaxy_games, heroic_games, egl_games, mods_entries):
        try:
            out.extend(f(home_dir))
        except Exception as e:  # noqa: BLE001 - one launcher's odd files never stop the others
            skipped('%s: %s' % (f.__name__, type(e).__name__))
    return out


def clean_map(mapping):
    """id -> Steam app id, only sane entries."""
    if not isinstance(mapping, dict):
        return {}
    return {k: int(v) for k, v in mapping.items()
            if valid_id(k) and isinstance(v, (int, float)) and not isinstance(v, bool) and 0 < int(v) < 2 ** 32}


def clean_applied(applied):
    """id -> {'t': target key, 'art': {kind: picture reference}}: what was last given to Steam for each shortcut."""
    out = {}
    if not isinstance(applied, dict):
        return out
    for k, v in applied.items():
        if not valid_id(k) or not isinstance(v, dict):
            continue
        art = v.get('art') if isinstance(v.get('art'), dict) else {}
        out[k] = {'t': v.get('t') if isinstance(v.get('t'), str) else '',
                  'art': {a: r for a, r in art.items() if a in ('cover', 'wide', 'hero', 'logo') and isinstance(r, str)}}
    return out


def clean_status(s):
    s = s if isinstance(s, dict) else {}
    num = lambda k: int(s[k]) if isinstance(s.get(k), (int, float)) and not isinstance(s.get(k), bool) and s[k] >= 0 else 0
    probs = [str(p)[:200] for p in s.get('problems', []) if isinstance(p, str)][:10] if isinstance(s.get('problems'), list) else []
    return {'lastSync': num('lastSync'), 'lastArtTry': num('lastArtTry'), 'synced': num('synced'), 'artMissing': num('artMissing'),
            'signature': str(s.get('signature') or '')[:64], 'problems': probs,
            'off': str(s.get('off') or '')[:300], 'adopted': s.get('adopted') is True}


def clean_settings(s):
    """{'stores': {switch: bool}}: every store on unless turned off."""
    st = s.get('stores') if isinstance(s, dict) and isinstance(s.get('stores'), dict) else {}
    return {'stores': {k: bool(v) for k, v in st.items() if isinstance(k, str) and 0 < len(k) <= 32 and isinstance(v, bool)}}


def old_shortcuts(settings_root):
    """The old Hearth plugin's game shortcuts (id -> app id), to take over instead of adding them twice. Its quick links
    (hearth-app:) and split-screen shortcuts (coop:) are left to it."""
    for name in OLD_PLUGIN_SETTINGS:
        try:
            with open(os.path.join(settings_root, name, 'shortcuts.json')) as f:
                m = clean_map(json.load(f))
        except (OSError, ValueError):
            continue
        m = {k: v for k, v in m.items() if not k.startswith(('hearth-app:', 'coop:', 'steam:'))}
        if m:
            return m
    return {}


def shortcuts_map():
    """Hearth Sync's shortcuts. The first time, the old Hearth plugin's game shortcuts are taken over (once)."""
    st = clean_status(load_json('status.json', {}))
    mine = clean_map(load_json('shortcuts.json', {}))
    if not st['adopted']:
        old = old_shortcuts(os.path.dirname(settings_dir().rstrip('/')))
        for k, v in old.items():
            mine.setdefault(k, v)
        st['adopted'] = True
        save_json('shortcuts.json', mine)
        save_json('status.json', st)
        if old:
            log('took over %d game shortcuts the old Hearth plugin made' % len(old))
    return mine


class Plugin:
    async def get_games(self, force=False):
        r = list_games(force=bool(force))
        return r

    async def get_art(self, ref):
        return read_art(ref)

    async def art_error(self, reason):
        """A picture Steam refused (SetCustomArtworkForApp threw): counted and logged by reason."""
        art_failed('steam: ' + str(reason)[:60])

    async def get_shortcuts(self):
        return shortcuts_map()

    async def set_shortcuts(self, mapping):
        m = clean_map(mapping)
        save_json('shortcuts.json', m)
        return m

    async def get_applied(self):
        return clean_applied(load_json('applied.json', {}))

    async def set_applied(self, applied):
        a = clean_applied(applied)
        save_json('applied.json', a)
        return a

    async def get_settings(self):
        return clean_settings(load_json('settings.json', {}))

    async def set_settings(self, settings):
        s = clean_settings(settings)
        save_json('settings.json', s)
        return s

    async def get_status(self):
        return clean_status(load_json('status.json', {}))

    async def set_status(self, status):
        old = clean_status(load_json('status.json', {}))
        s = clean_status(status)
        s['adopted'] = old['adopted'] or s['adopted']
        save_json('status.json', s)
        return s

    async def report(self, line):
        """One line from the frontend for the log (counts and reasons only)."""
        log(str(line)[:300])

    async def _main(self):
        log('Hearth Sync ready')

    async def _unload(self):
        pass

    async def _uninstall(self):
        pass
