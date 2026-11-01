"""The Hearth Decky plugin's backend (Steam Game Mode, Experimental; docs/STEAM-GAME-MODE.md).

Runs as you (no root flag) inside Decky Loader and only talks to Hearth's own command line on this device:

  nimbus --list-games      your installed games from Hearth's library, as JSON (hosts/hearth-core desktop_games.rs)
  nimbus --play-now ID     plays one the way Hearth's Play button does, and waits until it has closed
  nimbus-mode game         back to Hearth's Game Mode
  nimbus --steam-mode-data WHAT [ARG]
                           what the takeover's pages show (Hearth's home, Store, Library, Downloads, Storage, a game's
                           facts, the power mode), as JSON (hosts/hearth-core steam_takeover.rs). Nothing listens on a
                           port or socket for this: Decky's own channel to this backend is the only way in.

Nothing is sent anywhere: game pictures from the web are fetched (when Hearth's art cache hasn't got them) only to hand
them to Steam as a shortcut's art or to a page that couldn't load them itself.
"""
import base64
import hashlib
import json
import os
import pwd
import shlex
import shutil
import subprocess
import threading
import urllib.parse
import urllib.request

try:
    import decky  # provided by Decky Loader
except ImportError:  # tests
    decky = None

LIST_TIMEOUT = 60
ART_LIMIT = 10 * 1024 * 1024
# Store names for the panel (Hearth's library sources, desktop_games.rs entry "source").
SOURCES = {'epic': 'Epic Games', 'gog': 'GOG', 'battlenet': 'Battle.net', 'xbox': 'Xbox', 'xodus': 'Xbox PC',
           'beacon': 'Beacon', 'manual': 'Added by you', 'emu': 'Emulators', 'ea': 'EA', 'ubisoft': 'Ubisoft',
           'amazon': 'Amazon', 'itch': 'itch.io', 'heroic': 'Heroic', 'lutris': 'Lutris'}
# Hearth's own apps as Steam shortcuts (the panel's quick links), and the programs they run.
LINKS = [
    {'id': 'hearth-app:mods', 'title': 'Hearth Mods', 'program': 'hearth-mods', 'args': []},
    {'id': 'hearth-app:theater', 'title': 'Hearth Theater', 'program': 'hearth-theater', 'args': ['--fullscreen']},
    {'id': 'hearth-app:store', 'title': 'Hearth Store', 'program': 'nimbus', 'args': ['--windowed', '--page', 'shop']},
    {'id': 'hearth-app:hearth', 'title': 'Hearth', 'program': 'nimbus', 'args': ['--windowed']},
    {'id': 'hearth-app:settings', 'title': 'Hearth Settings', 'program': 'nimbus', 'args': ['--windowed', '--settings']},
]
# The takeover's questions for Hearth (steam_takeover.rs), how long each may take, and which argument each takes.
DATA = {'home': 60, 'library': 60, 'store': 90, 'downloads': 30, 'storage': 30, 'info': 40, 'power': 20, 'power-mode': 40}
STORE_PAGES = ('steam', 'gog', 'epic', 'xbox', 'free', 'recomps', 'battlenet')
POWER_MODES = ('efficiency', 'balanced', 'performance')
ERRORS_KEPT = 50


def log(msg):
    if decky:
        decky.logger.info(msg)


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth')


def plugin_dir():
    d = getattr(decky, 'DECKY_PLUGIN_DIR', None) if decky else None
    return d or os.path.dirname(os.path.abspath(__file__))


# "Start in Steam Game Mode" (Hearth's Settings › System; docs/STEAM-GAME-MODE.md): HEARTH_STEAM_DEFAULT in Hearth's
# session.conf, read by nimbus-session at every sign-in. Quick Access › Hearth can turn it off (a way back to Hearth's
# Game Mode with only a controller) or on again; the session still checks everything at each start and turns it off
# by itself when Steam fails. The same rules as hearth-core steam_mode.rs read_default / write_default.
STEAM_DEFAULT_KEY = 'HEARTH_STEAM_DEFAULT'


def session_conf(home_dir=None):
    return os.path.join(home_dir or home(), '.config', 'nimbus', 'session.conf')


def _is_default_line(line):
    t = line.lstrip()
    if t.startswith('export'):
        t = t[len('export'):].lstrip()
    return t.startswith(STEAM_DEFAULT_KEY + '=')


def read_steam_default(text):
    on = False
    for line in text.splitlines():
        if _is_default_line(line):
            on = line.split('=', 1)[1].strip().strip('"\'') == '1'
    return on


def write_steam_default(text, on):
    line = '%s=%d' % (STEAM_DEFAULT_KEY, 1 if on else 0)
    lines = text.splitlines()
    if any(_is_default_line(l) for l in lines):
        return '\n'.join(line if _is_default_line(l) else l for l in lines) + '\n'
    return text + ('\n' if text and not text.endswith('\n') else '') + line + '\n'


def steam_default(home_dir=None):
    try:
        with open(session_conf(home_dir)) as f:
            return read_steam_default(f.read())
    except OSError:
        return False


def set_steam_default(on, home_dir=None):
    """Writes the setting (all at once, keeping the rest of session.conf). Turning it on clears the session's note and
    count of failed starts (~/.local/state/nimbus), like Hearth's Settings does."""
    h = home_dir or home()
    path = session_conf(h)
    try:
        with open(path) as f:
            text = f.read()
    except OSError:
        text = ''
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.decky-hearth.tmp'
    with open(tmp, 'w') as f:
        f.write(write_steam_default(text, on))
    os.replace(tmp, path)
    if on:
        for n in ('steam-default-off', 'steam-default-fails'):
            try:
                os.remove(os.path.join(h, '.local', 'state', 'nimbus', n))
            except OSError:
                pass
    return on


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


def steam_is_flatpak(proc='/proc'):
    """Whether the Steam that's running is Flathub's (its shortcuts then run inside its sandbox)."""
    try:
        pids = [p for p in os.listdir(proc) if p.isdigit()]
    except OSError:
        return False
    for p in pids:
        try:
            with open(os.path.join(proc, p, 'comm')) as f:
                if f.read().strip() not in ('steam', 'steamwebhelper'):
                    continue
            with open(os.path.join(proc, p, 'environ'), 'rb') as f:
                if b'FLATPAK_ID=com.valvesoftware.Steam' in f.read():
                    return True
        except OSError:
            continue
    return False


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


def shortcut(program, args, flatpak=None):
    """What a Steam shortcut runs: {exe, startDir, options}. The exe is /bin/sh with WRAPPER (it exists in the system
    and in Flathub's Steam sandbox); Steam adds the options after the program through a shell, so each argument is
    quoted. `flatpak` is no longer needed (the wrapper decides at launch) and is ignored."""
    opts = ['-c', WRAPPER, WRAPPER_NAME, program] + list(args)
    return {'exe': '/bin/sh', 'startDir': home(), 'options': ' '.join(shlex.quote(a) for a in opts)}


def clean_games(raw):
    """Hearth's list (nimbus --list-games) without Steam's own games (they're in Steam already), A to Z."""
    out = []
    if not isinstance(raw, list):
        return out
    for g in raw:
        if not isinstance(g, dict) or not valid_id(g.get('id')) or g.get('source') == 'steam' or g['id'].startswith('steam:'):
            continue
        title = g.get('title') if isinstance(g.get('title'), str) and g.get('title').strip() else g['id']
        src = g.get('source') if isinstance(g.get('source'), str) else ''
        out.append({'id': g['id'], 'title': title.strip(), 'source': src, 'store': SOURCES.get(src, src.capitalize() or 'Hearth'),
                    'cover': g.get('cover') or '', 'wide': g.get('wide') or '', 'icon': g.get('icon') or ''})
    out.sort(key=lambda g: g['title'].lower())
    return out


def list_games(run=subprocess.run):
    app = find_program('nimbus')
    if not app:
        return {'ok': False, 'error': "Hearth isn't installed here", 'games': []}
    try:
        r = run([app, '--list-games'], capture_output=True, text=True, timeout=LIST_TIMEOUT, env=clean_env())
    except (OSError, subprocess.TimeoutExpired) as e:
        return {'ok': False, 'error': 'Hearth didn\'t answer (%s)' % e, 'games': []}
    try:
        raw = json.loads((r.stdout or '').strip().splitlines()[-1])
    except (ValueError, IndexError):
        return {'ok': False, 'error': (r.stderr or 'Hearth gave no list').strip()[-300:], 'games': []}
    return {'ok': True, 'games': clean_games(raw)}


def targets(games, flatpak, app=None, finder=None):
    """Each game's and quick link's Steam shortcut (exe, start folder, launch options)."""
    finder = finder or find_program
    app = app or finder('nimbus') or '/usr/bin/nimbus'
    out = []
    for g in games:
        t = dict(g)
        t.update(shortcut(app, ['--play-now', g['id']], flatpak))
        out.append(t)
    links = []
    for l in LINKS:
        prog = app if l['program'] == 'nimbus' else finder(l['program'])
        if not prog:
            continue
        t = {'id': l['id'], 'title': l['title'], 'source': 'hearth', 'store': 'Hearth', 'cover': '', 'wide': '', 'icon': ''}
        t.update(shortcut(prog, l['args'], flatpak))
        links.append(t)
    return out, links


ALLOWED_ROOTS = ('/usr/share/', '/var/lib/flatpak/', '/opt/')
# What a picture is, from its first bytes (never from its name): Steam's SetCustomArtworkForApp takes png and jpg only,
# the takeover's pages (Steam's window) show all of these.
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


def data_url(path):
    try:
        with open(path, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    mime = {'.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg'}.get(os.path.splitext(path)[1].lower())
    return 'data:%s;base64,%s' % (mime, base64.b64encode(data).decode()) if mime else ''


def brand(pdir=None):
    """Hearth's look for Steam: the theme's CSS and Hearth's logo and wallpaper (as data: addresses for the CSS)."""
    pdir = pdir or plugin_dir()
    try:
        with open(os.path.join(pdir, 'hearth-theme.css')) as f:
            css = f.read()
    except OSError:
        css = ''
    wall = os.path.join(pdir, 'assets', 'wallpaper.jpg')
    if not os.path.isfile(wall):
        wall = '/usr/share/backgrounds/hearth/hearth-embers.jpg'
    return {'css': css, 'logo': data_url(os.path.join(pdir, 'assets', 'logo.svg')), 'wallpaper': data_url(wall)}


def load_json(name, default):
    try:
        with open(os.path.join(settings_dir(), name)) as f:
            v = json.load(f)
        return v if isinstance(v, type(default)) else default
    except (OSError, ValueError):
        return default


def save_json(name, value):
    d = settings_dir()
    os.makedirs(d, exist_ok=True)
    tmp = os.path.join(d, name + '.tmp')
    with open(tmp, 'w') as f:
        json.dump(value, f)
    os.replace(tmp, os.path.join(d, name))


# The same keys and defaults as src/model.ts DEFAULTS: the takeover (master switch and its parts) is on by default.
DEFAULT_SETTINGS = {'theme': True, 'homeRow': True, 'takeover': True, 'tkHome': True, 'tkMenu': True, 'tkLibraryTabs': True,
                    'tkGamePage': True, 'tkSounds': False}


def data_args(what, arg):
    """The command line for one of the takeover's questions, or None when it isn't one this asks."""
    if what not in DATA:
        return None
    arg = arg if isinstance(arg, str) else ''
    if what == 'store':
        if arg not in STORE_PAGES:
            return None
        return ['--steam-mode-data', what, arg]
    if what == 'info':
        return ['--steam-mode-data', what, arg] if valid_id(arg) else None
    if what == 'power-mode':
        return ['--steam-mode-data', what, arg] if arg in POWER_MODES else None
    return ['--steam-mode-data', what]


def hearth_data(what, arg='', run=subprocess.run):
    """Asks Hearth one of the takeover's questions. Always a dict: Hearth's answer, or {'error': ...}."""
    args = data_args(what, arg)
    if not args:
        return {'error': "That isn't something the Hearth plugin asks"}
    app = find_program('nimbus')
    if not app:
        return {'error': "Hearth isn't installed here"}
    try:
        r = run([app] + args, capture_output=True, text=True, timeout=DATA[what], env=clean_env())
    except (OSError, subprocess.TimeoutExpired) as e:
        return {'error': 'Hearth didn\'t answer (%s)' % e}
    lines = (r.stdout or '').strip().splitlines()
    try:
        v = json.loads(lines[-1]) if lines else None
    except ValueError:
        v = None
    if not isinstance(v, dict):
        return {'error': (r.stderr or 'Hearth gave no answer').strip()[-300:] or 'Hearth gave no answer'}
    return v


STEAM_ROOTS = ('.steam/steam', '.steam/root', '.local/share/Steam', '.var/app/com.valvesoftware.Steam/.local/share/Steam',
               '.var/app/com.valvesoftware.Steam/data/Steam')
# Steam's own pictures of its games in its library cache (appcache/librarycache): older Steam names them
# <appid>_<file>, newer Steam keeps them in <appid>/ (sometimes one folder deeper).
STEAM_ART = {'portrait': ('library_600x900.jpg', 'library_600x900_2x.jpg', 'library_capsule.jpg'),
             'wide': ('header.jpg', 'library_header.jpg', 'library_hero.jpg'),
             'hero': ('library_hero.jpg', 'header.jpg'), 'logo': ('logo.png',)}


def steam_art_file(appid, kind, home_dir=None):
    """A Steam game's own picture from Steam's library cache on this device, or None."""
    appid = str(appid)
    if not appid.isdigit() or kind not in STEAM_ART:
        return None
    h = home_dir or home()
    for root in STEAM_ROOTS:
        cache = os.path.join(h, root, 'appcache', 'librarycache')
        if not os.path.isdir(cache):
            continue
        for name in STEAM_ART[kind]:
            for f in (os.path.join(cache, '%s_%s' % (appid, name)), os.path.join(cache, appid, name)):
                if os.path.isfile(f):
                    return f
            sub = os.path.join(cache, appid)
            try:
                for d in sorted(os.listdir(sub)):
                    f = os.path.join(sub, d, name)
                    if os.path.isfile(f):
                        return f
            except OSError:
                pass
    return None


def picture_url(ref, fetch=None):
    """A picture as a data: address for the takeover's pages and the Hearth row (Steam's window can't open files on
    this device): every kind of reference Hearth gives (see local_path, art_bytes), and steam-lib:<appid>:<kind> for a
    Steam game's own picture from Steam's library cache. Only pictures, only from the allowed places; '' otherwise."""
    if isinstance(ref, str) and ref.startswith('steam-lib:'):
        parts = ref.split(':')
        f = steam_art_file(parts[1], parts[2]) if len(parts) == 3 else None
        if not f:
            return ''   # Steam hasn't got this picture: the card shows the title (not counted as a failure)
        ref = f
    data, k, why = art_bytes(ref, fetch)
    if not data:
        art_failed('page art: ' + why)
        return ''
    return 'data:%s;base64,%s' % (MIMES[k], base64.b64encode(data).decode())


def note_error(part, message, name='errors.log'):
    """A part of the takeover that turned itself off (Steam changed a screen): kept in a short log for Quick Access."""
    line = '%s: %s' % (str(part)[:40], str(message).replace('\n', ' ')[:400])
    log('takeover part off: ' + line)
    path = os.path.join(settings_dir(), name)
    try:
        with open(path) as f:
            lines = f.read().splitlines()
    except OSError:
        lines = []
    lines = (lines + [line])[-ERRORS_KEPT:]
    os.makedirs(settings_dir(), exist_ok=True)
    with open(path, 'w') as f:
        f.write('\n'.join(lines) + '\n')
    return lines


def startup_movie(pdir=None, home_dir=None):
    """Hearth's startup movie for Steam (its custom boot video folder), when the plugin ships one. Returns where it went."""
    src = os.path.join(pdir or plugin_dir(), 'assets', 'hearth-startup.webm')
    if not os.path.isfile(src):
        return None
    dst_dir = os.path.join(home_dir or home(), '.steam', 'root', 'config', 'uioverrides', 'movies')
    dst = os.path.join(dst_dir, 'deck_startup.webm')
    if os.path.exists(dst):
        return None   # yours, or one from before: kept
    os.makedirs(dst_dir, exist_ok=True)
    shutil.copyfile(src, dst)
    return dst


# Quick Access › Hearth › Update Hearth OS: the same updater Steam's Settings › System › Check for updates runs in Steam
# Game Mode (hearth-steamos-update: Hearth's own update through hearth-admin; it never restarts the device).
STEAMOS_UPDATE = '/usr/libexec/nimbus/hearth-steamos-update'
UPDATE_CHECK_TIMEOUT = 300
UPDATE_TIMEOUT = 3 * 3600
UPDATE = {'state': 'idle', 'pct': None, 'message': '', 'version': ''}
_update_lock = threading.Lock()


def update_result(mode, code, out, err):
    """What the updater's answer means for the panel: {state, pct, message, version}.
    check: 0 available, 7 up to date; apply: 0 ready for a restart, 7 nothing new; anything else failed."""
    lines = [l.strip() for l in (err or '').splitlines() if l.strip()]
    msg = lines[-1] if lines else ''
    version = ''
    for l in (out or '').splitlines():
        if l.startswith('Update available: '):
            version = l[len('Update available: '):].strip()
    if code == 7:
        return {'state': 'none', 'pct': None, 'message': 'Hearth OS is up to date', 'version': ''}
    if code == 0 and mode == 'check':
        return {'state': 'available', 'pct': None, 'message': 'An update is available' + (' (%s)' % version if version else ''), 'version': version}
    if code == 0:
        return {'state': 'ready', 'pct': 100, 'message': 'Ready: restart to finish updating (the version before stays installed)', 'version': version}
    return {'state': 'error', 'pct': None, 'message': msg.replace('Hearth OS: ', '') or 'The update failed', 'version': ''}


def progress_of(line):
    """A "42%" line from the updater, as 42; else None."""
    line = (line or '').strip()
    if line.endswith('%') and line[:-1].isdigit():
        return max(0, min(100, int(line[:-1])))
    return None


def updater():
    return STEAMOS_UPDATE if os.access(STEAMOS_UPDATE, os.X_OK) else None


def update_check(run=subprocess.run, prog=None):
    prog = prog or updater()
    if not prog:
        return {'state': 'error', 'pct': None, 'message': "Hearth's updater isn't installed here", 'version': ''}
    try:
        r = run([prog, 'check'], capture_output=True, text=True, timeout=UPDATE_CHECK_TIMEOUT, env=clean_env())
    except (OSError, subprocess.TimeoutExpired) as e:
        return {'state': 'error', 'pct': None, 'message': 'The update check didn\'t answer (%s)' % type(e).__name__, 'version': ''}
    res = update_result('check', r.returncode, r.stdout, r.stderr)
    log('update check: %s' % res['state'])
    return res


def update_apply(prog=None, popen=subprocess.Popen):
    """Runs the update in the background; UPDATE says how far it is. False when one is already running."""
    prog = prog or updater()
    if not prog:
        UPDATE.update({'state': 'error', 'pct': None, 'message': "Hearth's updater isn't installed here"})
        return False
    with _update_lock:
        if UPDATE['state'] == 'running':
            return False
        UPDATE.update({'state': 'running', 'pct': 0, 'message': 'Updating Hearth OS…', 'version': ''})

    def work():
        try:
            p = popen([prog], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=clean_env())
            err = []
            t = threading.Thread(target=lambda: err.extend(p.stderr.readlines()), daemon=True)
            t.start()
            for line in p.stdout:
                pct = progress_of(line)
                if pct is not None:
                    UPDATE['pct'] = pct
            code = p.wait(timeout=UPDATE_TIMEOUT)
            t.join(5)
            UPDATE.update(update_result('apply', code, '', ''.join(err)))
        except Exception as e:  # noqa: BLE001 - the panel says it failed
            UPDATE.update({'state': 'error', 'pct': None, 'message': 'The update failed (%s)' % type(e).__name__})
        log('update: %s' % UPDATE['state'])

    threading.Thread(target=work, daemon=True).start()
    return True


class Plugin:
    async def get_games(self):
        r = list_games()
        games, links = targets(r['games'], steam_is_flatpak())
        return {'ok': r['ok'], 'error': r.get('error'), 'games': games, 'links': links}

    async def get_art(self, ref):
        return read_art(ref)

    async def get_brand(self):
        return brand()

    async def get_settings(self):
        s = dict(DEFAULT_SETTINGS)
        s.update(load_json('settings.json', {}))
        return s

    async def set_settings(self, settings):
        s = dict(DEFAULT_SETTINGS)
        if isinstance(settings, dict):
            s.update({k: bool(v) for k, v in settings.items() if k in DEFAULT_SETTINGS})
        save_json('settings.json', s)
        return s

    async def get_shortcuts(self):
        return load_json('shortcuts.json', {})

    async def set_shortcuts(self, mapping):
        clean = {k: int(v) for k, v in (mapping or {}).items() if isinstance(k, str) and isinstance(v, (int, float)) and int(v) > 0}
        save_json('shortcuts.json', clean)
        return clean

    async def hearth_data(self, what, arg=''):
        return hearth_data(what, arg)

    async def get_picture(self, ref):
        return picture_url(ref)

    async def art_error(self, where, reason):
        """A picture that didn't load in Steam's window (the page's <img> said so): counted and logged by reason."""
        art_failed('%s: %s' % (str(where)[:30], str(reason)[:40]))

    async def get_target(self, game_id):
        if not valid_id(game_id) or game_id.startswith('steam:'):
            return {'ok': False, 'error': "That isn't a game Hearth plays from here"}
        app = find_program('nimbus')
        if not app:
            return {'ok': False, 'error': "Hearth isn't installed here"}
        t = shortcut(app, ['--play-now', game_id], steam_is_flatpak())
        t['ok'] = True
        return t

    async def get_link(self, link_id):
        _, links = targets([], steam_is_flatpak())
        return next((l for l in links if l['id'] == link_id), None)

    async def log_error(self, part, message):
        try:
            note_error(part, message)
        except OSError as e:
            log('takeover log: %s' % e)

    async def get_errors(self):
        try:
            with open(os.path.join(settings_dir(), 'errors.log')) as f:
                return f.read().splitlines()[-ERRORS_KEPT:]
        except OSError:
            return []

    async def update_check(self):
        if UPDATE['state'] == 'running':
            return dict(UPDATE)
        import asyncio
        UPDATE.update(await asyncio.get_event_loop().run_in_executor(None, update_check))
        return dict(UPDATE)

    async def update_start(self):
        update_apply()
        return dict(UPDATE)

    async def update_status(self):
        return dict(UPDATE)

    async def switch_to_hearth(self):
        prog = find_program('nimbus-mode')
        if not prog:
            return {'ok': False, 'error': "Hearth's session isn't installed here"}
        log('switching to Hearth Game Mode')
        subprocess.Popen([prog, 'game'], env=clean_env(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, start_new_session=True)
        return {'ok': True}

    async def get_steam_default(self):
        return steam_default()

    async def set_steam_default(self, on):
        try:
            set_steam_default(bool(on))
        except OSError as e:
            log('start in Steam Game Mode: %s' % e)
            return {'ok': False, 'error': str(e), 'on': steam_default()}
        log('start in Steam Game Mode: %s (from Quick Access)' % ('on' if on else 'off'))
        return {'ok': True, 'on': steam_default()}

    async def _main(self):
        # Parts that turned themselves off last time get their chance again (Steam may have been fixed): a fresh log.
        try:
            os.remove(os.path.join(settings_dir(), 'errors.log'))
        except OSError:
            pass
        try:
            movie = startup_movie()
            if movie:
                log('Hearth startup movie put in ' + movie)
        except OSError as e:
            log('startup movie: %s' % e)
        log('Hearth plugin ready')

    async def _unload(self):
        pass

    async def _uninstall(self):
        pass
