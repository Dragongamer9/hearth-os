"""The Hearth Decky plugin's backend (Steam Game Mode, Experimental; docs/STEAM-GAME-MODE.md).

Runs as you (no root flag) inside Decky Loader and only talks to Hearth's own command line on this device:

  nimbus --list-games      your installed games from Hearth's library, as JSON (hosts/hearth-core desktop_games.rs)
  nimbus --play-now ID     plays one the way Hearth's Play button does, and waits until it has closed
  nimbus-mode game         back to Hearth's Game Mode
  nimbus --steam-mode-data WHAT [ARG]
                           what the takeover's pages show (Hearth's home, Store, Library, Downloads, Storage, a game's
                           facts, the power mode), as JSON (hosts/hearth-core steam_takeover.rs). Nothing listens on a
                           port or socket for this: Decky's own channel to this backend is the only way in.

Nothing is sent anywhere: game pictures from the web are fetched only to hand them to Steam as the shortcut's art.
"""
import base64
import json
import os
import pwd
import shlex
import shutil
import subprocess
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


def shortcut(program, args, flatpak):
    """What a Steam shortcut runs: {exe, startDir, options}. Steam adds the options after the program through a shell,
    so each argument is quoted. Flathub's Steam reaches the system's programs through flatpak-spawn --host."""
    if flatpak:
        exe, opts = '/usr/bin/flatpak-spawn', ['--host', program] + list(args)
    else:
        exe, opts = program, list(args)
    return {'exe': exe, 'startDir': home(), 'options': ' '.join(shlex.quote(a) for a in opts)}


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


def art_allowed(path, home_dir=None):
    """Only pictures in your home, the system's shared files or Flatpak's (never other files)."""
    if not isinstance(path, str) or not path.startswith('/'):
        return False
    real = os.path.realpath(path)
    roots = ALLOWED_ROOTS + (os.path.realpath(home_dir or home()).rstrip('/') + '/',)
    return real.lower().endswith(('.png', '.jpg', '.jpeg')) and any(real.startswith(r) for r in roots)


def kind_of(data):
    if data.startswith(b'\x89PNG'):
        return 'png'
    if data.startswith(b'\xff\xd8'):
        return 'jpg'
    return None


def read_art(ref, fetch=None):
    """A picture as Steam's SetCustomArtworkForApp wants it: {data (base64), type: png|jpg}, or None."""
    if not isinstance(ref, str) or not ref:
        return None
    data = None
    if ref.startswith('file://'):
        ref = urllib.request.url2pathname(ref[7:])
    if ref.startswith('https://'):
        try:
            data = (fetch or _fetch)(ref)
        except Exception as e:  # noqa: BLE001 - any failure just means no picture
            log('art: %s: %s' % (ref, e))
            return None
    elif art_allowed(ref):
        try:
            if os.path.getsize(ref) > ART_LIMIT:
                return None
            with open(ref, 'rb') as f:
                data = f.read()
        except OSError:
            return None
    if not data or len(data) > ART_LIMIT:
        return None
    k = kind_of(data)
    return {'data': base64.b64encode(data).decode(), 'type': k} if k else None


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


PICTURE_KINDS = {'.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp', '.gif': 'image/gif'}


def picture_url(ref):
    """A picture on this device as a data: address for the takeover's pages (Steam's window can't open files); only
    from the places art_allowed allows, and only pictures. Web pictures aren't read here (the page loads them)."""
    if not isinstance(ref, str) or not ref:
        return ''
    if ref.startswith('file://'):
        ref = urllib.request.url2pathname(ref[7:])
    if not ref.startswith('/'):
        return ''
    real = os.path.realpath(ref)
    ext = os.path.splitext(real)[1].lower()
    roots = ALLOWED_ROOTS + (os.path.realpath(home()).rstrip('/') + '/',)
    if ext not in PICTURE_KINDS or not any(real.startswith(r) for r in roots):
        return ''
    try:
        if os.path.getsize(real) > ART_LIMIT:
            return ''
        with open(real, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    ok = kind_of(data) or (data[:4] == b'RIFF' and data[8:12] == b'WEBP' and 'webp') or (data[:3] == b'GIF' and 'gif')
    return 'data:%s;base64,%s' % (PICTURE_KINDS[ext], base64.b64encode(data).decode()) if ok else ''


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

    async def switch_to_hearth(self):
        prog = find_program('nimbus-mode')
        if not prog:
            return {'ok': False, 'error': "Hearth's session isn't installed here"}
        log('switching to Hearth Game Mode')
        subprocess.Popen([prog, 'game'], env=clean_env(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, start_new_session=True)
        return {'ok': True}

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
