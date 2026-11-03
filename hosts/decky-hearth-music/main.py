"""Hearth Music's backend (docs/HEARTH-MUSIC.md): finds the theme song of the game you're looking at in Steam Game Mode.

Runs as you (no root flag) inside Decky Loader. The frontend (src/) knows which game is highlighted and plays the
song; this file finds it, in this order:

  1. a file you picked for that game, or for one of its collections (picks.json)
  2. the game's own soundtrack, when you have it: Steam soundtrack folders (steamapps/music/<name>,
     steamapps/common/<Game> Soundtrack) and, conservatively, an audio file in the game's folder whose name says it's
     the main/title theme
  3. Apple's official 30-second preview from the iTunes Search API (itunes.apple.com/search), the best match for
     "<title> main theme" / "<title> soundtrack", streamed from Apple's address and never stored (the frontend
     plays it directly; if Steam's browser can't, this backend passes it on from memory); the panel shows the
     track, artist and a link to it.
  4. When Apple has no good match: Deezer's 30-second preview (api.deezer.com/search, no account or key), by the same
     rules. Deezer's preview addresses expire, so only the track id is kept and the address is asked for again when it
     plays. Streamed, never stored ("Preview from Deezer"). No match: silence. Never YouTube or anything that
     downloads from it.

The library scan (scan_library) goes through every game in Steam's library in the background the first time, then
new games as they appear and stale answers weekly; it pauses while a game runs and resumes where it left off.
Apple's API is asked at most once every few seconds and 20 times a minute, Deezer's once a second; a 429/403 (or
Deezer's quota answer) backs off for a minute, doubling up to an hour. The answers (which match belongs to which game,
and "no match") are kept in lookups.json, saved atomically with a .bak. Only those answers are kept, never any audio.
Nothing else is sent anywhere. The log gets counts and reasons only.
"""
import asyncio
import base64
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

try:
    import decky  # provided by Decky Loader
except ImportError:  # tests
    decky = None

AUDIO_EXTS = ('.ogg', '.opus', '.mp3', '.flac', '.m4a', '.aac', '.wav')
MIMES = {'.ogg': 'audio/ogg', '.opus': 'audio/ogg', '.mp3': 'audio/mpeg', '.flac': 'audio/flac', '.m4a': 'audio/mp4',
         '.aac': 'audio/aac', '.wav': 'audio/wav'}
AUDIO_LIMIT = 25 * 1024 * 1024      # what's handed to the frontend at most
PREVIEW_LIMIT = 4 * 1024 * 1024     # a 30-second preview is well under 1 MB
ITUNES = 'https://itunes.apple.com/search'
# Apple asks for no more than about 20 calls a minute.
ITUNES_GAP = 3.5
ITUNES_PER_MINUTE = 20
KEEP_MATCH = 30 * 86400
KEEP_NO_MATCH = 7 * 86400
KEEP_ERROR = 3600
DEEZER = 'https://api.deezer.com'
DEEZER_GAP = 1.0          # Deezer allows 50 calls in 5 s; one a second is polite
BACKOFF_FIRST = 60        # a 429/403: wait a minute, doubling up to an hour
BACKOFF_MAX = 3600
SCAN_REFRESH = 7 * 86400  # the library scan refreshes answers older than a week
SCAN_FORCE_REFRESH = 86400  # "Scan my library": answers older than a day
DEFAULTS = {'enabled': True, 'volume': 0.35, 'delay': 1.5, 'where': 'pages', 'online': True, 'toast': True, 'deezer': True}

_calls = []   # times of Apple calls in the last minute
_deezer_calls = []   # time of the last Deezer call
_backoff = {}        # api -> [until, step]
_rate_lock = threading.Lock()
_urlopen = urllib.request.urlopen   # the one way out to the network (tests replace it)


def log(msg):
    if decky:
        decky.logger.info(msg)


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth-music')


def cache_dir():
    return os.path.join(home(), '.cache', 'hearth-music')


# ---- Settings files the safe way (docs/SETTINGS-PERSISTENCE.md; the same rules as Hearth OS's hearth_settings_io.py and
# Hearth Sync's copy, kept here because a Decky plugin ships on its own): atomic (temporary file, fsync, rename) under
# one lock, the last good version kept as FILE.bak, a damaged file put back from it, never replaced with the defaults.
def _safe_write(path, data):
    import fcntl
    import tempfile
    d = os.path.dirname(path)
    os.makedirs(d, exist_ok=True)
    lock = os.open(os.path.join(d, '.' + os.path.basename(path) + '.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _write_locked(path, data, tempfile)
    finally:
        os.close(lock)


def _write_locked(path, data, tempfile):
    d = os.path.dirname(path)
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


def _read_json(path, repair=True):
    """The JSON in PATH; damaged: the copy in PATH.bak (put back unless repair is False: the caller holds the lock and
    writes the file itself). None when neither can be read."""
    for p in (path, path + '.bak'):
        try:
            with open(p) as f:
                v = json.load(f)
        except (OSError, ValueError):
            continue
        if p != path and repair:
            try:
                _safe_write(path, json.dumps(v).encode())
            except OSError:
                pass
        return v
    return None


def load_json(name, default, folder=None):
    v = _read_json(os.path.join(folder or settings_dir(), name))
    return v if isinstance(v, type(default)) else default


def save_json(name, value, folder=None):
    _safe_write(os.path.join(folder or settings_dir(), name), json.dumps(value).encode())


def save_cache(name, value):
    """A cache file (the lookup answers): written atomically with a .bak, like the settings files."""
    _safe_write(os.path.join(cache_dir(), name), json.dumps(value).encode())


def update_cache(name, fn):
    """Changes a cache file under its lock: the latest file is read, fn(dict) changes it, and it's written back
    atomically with a .bak (the scan and the panel's lookups both write lookups.json; neither loses the other's)."""
    import fcntl
    import tempfile
    d = cache_dir()
    os.makedirs(d, exist_ok=True)
    path = os.path.join(d, name)
    lock = os.open(os.path.join(d, '.' + name + '.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        cur = _read_json(path, repair=False)
        cur = cur if isinstance(cur, dict) else {}
        fn(cur)
        _write_locked(path, json.dumps(cur).encode(), tempfile)
        return cur
    finally:
        os.close(lock)


def merge_settings(changes):
    """Only the keys you changed, merged into the latest settings.json under its lock (Hearth Link's "Sync Hearth
    settings" writes single keys into it too, hosts/decky-hearth-link/link_sync.py); unknown keys are kept."""
    import fcntl
    path = os.path.join(settings_dir(), 'settings.json')
    os.makedirs(settings_dir(), exist_ok=True)
    lock = os.open(os.path.join(settings_dir(), '.settings.json.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        cur = _read_json(path, repair=False)
        cur = cur if isinstance(cur, dict) else {}
        picked = clean_settings(dict(cur, **{k: v for k, v in (changes or {}).items() if k in DEFAULTS}))
        new = dict(cur)
        for k in (changes or {}):
            if k in DEFAULTS:
                new[k] = picked[k]
        import tempfile
        _write_locked(path, json.dumps(new).encode(), tempfile)
        return clean_settings(new)
    finally:
        os.close(lock)


def clean_settings(s):
    s = s if isinstance(s, dict) else {}
    out = dict(DEFAULTS)
    for k in ('enabled', 'online', 'toast', 'deezer'):
        if isinstance(s.get(k), bool):
            out[k] = s[k]
    # Deezer's previews came later: someone who had turned Apple's previews off (nothing sent anywhere) doesn't get
    # Deezer turned on for him; nothing is written, it's only what an unset switch means.
    if 'deezer' not in s and s.get('online') is False:
        out['deezer'] = False
    try:
        out['volume'] = min(1.0, max(0.0, float(s.get('volume', out['volume']))))
        out['delay'] = min(10.0, max(0.0, float(s.get('delay', out['delay']))))
    except (TypeError, ValueError):
        pass
    if s.get('where') in ('pages', 'library'):
        out['where'] = s['where']
    return out


# ---- names

def norm(s):
    """Lower case words only: "THE WITCHER® 3: Wild Hunt" -> "the witcher 3 wild hunt"; "MainMenu_Theme" -> "main menu theme"."""
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', str(s or ''))
    s = s.lower().replace('&', ' and ')
    s = re.sub(r'[™®©]', '', s)
    s = re.sub(r"['’]", '', s)
    s = re.sub(r'[^a-z0-9]+', ' ', s)
    return ' '.join(s.split())


EDITION = re.compile(r'\b(game of the year|goty|definitive|complete|enhanced|remastered|anniversary|deluxe|ultimate|'
                     r'gold|special|directors cut|edition|hd|classic)\b')


def title_forms(title):
    """The title to look for, and a shorter form (no edition words, the part before ':' or ' - ') when it's still
    distinctive (4+ letters, not just a common word)."""
    full = norm(title)
    forms = [full] if full else []
    short = re.split(r'\s*[:–—]\s*|\s+-\s+', str(title or ''))[0]
    short = ' '.join(EDITION.sub(' ', norm(short)).split())
    if short and short != full and len(short.replace(' ', '')) >= 4:
        forms.append(short)
    return forms


def has_words(hay, needle):
    return bool(needle) and (' %s ' % needle) in (' %s ' % hay)


# ---- 1. your picks

def pick_key(kind, name):
    return '%s:%s' % (kind, name)


def online_pick(p):
    """A pick that is an Apple or Deezer preview ("Choose music…"): only its id and names, never audio."""
    if not isinstance(p, dict) or p.get('source') not in ('apple', 'deezer') or not str(p.get('id') or '').isdigit():
        return None
    out = {'source': p['source'], 'id': str(p['id'])[:20]}
    for k in ('track', 'artist', 'album'):
        out[k] = str(p.get(k) or '')[:200]
    link = str(p.get('link') or '')
    out['link'] = link if link.startswith('https://') else ''
    if p['source'] == 'apple' and allowed_preview(p.get('preview') or ''):
        out['preview'] = p['preview']
    return out


def picked(appid, collections):
    picks = load_json('picks.json', {})
    for k in [pick_key('app', appid)] + [pick_key('collection', c) for c in collections or []]:
        p = picks.get(k)
        via = 'collection' if k.startswith('collection:') else 'game'
        if isinstance(p, str) and os.path.isfile(p) and p.lower().endswith(AUDIO_EXTS):
            return {'source': 'pick', 'path': p, 'title': os.path.splitext(os.path.basename(p))[0],
                    'via': via, 'name': k.split(':', 1)[1]}
        o = online_pick(p)
        if o:
            return {'source': 'pick', 'online': o, 'title': o['track'], 'artist': o['artist'], 'album': o['album'],
                    'link': o['link'], 'from': o['source'], 'via': via, 'name': k.split(':', 1)[1]}
    return None


def set_pick(kind, name, path):
    """PATH: an audio file of yours, an Apple/Deezer preview chosen in "Choose music…" (a dict: id and names), or ''
    to forget the pick. Picks are only ever changed here, by you; the library scan never touches picks.json."""
    if kind not in ('app', 'collection') or not str(name):
        raise ValueError('bad pick')
    picks = load_json('picks.json', {})
    k = pick_key(kind, str(name)[:200])
    if isinstance(path, dict):
        o = online_pick(path)
        if not o:
            raise ValueError('not a preview')
        picks[k] = o
    elif path:
        if not (os.path.isfile(path) and path.lower().endswith(AUDIO_EXTS)):
            raise ValueError('not an audio file')
        picks[k] = path
    else:
        picks.pop(k, None)
    save_json('picks.json', picks)
    return picks


def has_app_pick(appid, picks=None):
    picks = load_json('picks.json', {}) if picks is None else picks
    return pick_key('app', str(appid)) in picks


# ---- 2. the game's own soundtrack

def steam_roots(h=None):
    h = h or home()
    out = []
    for c in (os.path.join(h, '.local/share/Steam'), os.path.join(h, '.steam/steam'),
              os.path.join(h, '.var/app/com.valvesoftware.Steam/.local/share/Steam')):
        if os.path.isdir(os.path.join(c, 'steamapps')) and os.path.realpath(c) not in out:
            out.append(os.path.realpath(c))
    return out


def libraries(h=None):
    out = []
    for root in steam_roots(h):
        for p in [root] + re.findall(r'"path"\s+"((?:[^"\\]|\\.)*)"', _read(os.path.join(root, 'steamapps/libraryfolders.vdf'))):
            p = os.path.realpath(p.replace('\\\\', '\\'))
            if os.path.isdir(os.path.join(p, 'steamapps')) and p not in out:
                out.append(p)
    return out


def _read(p, limit=1 << 20):
    try:
        with open(p, encoding='utf-8', errors='replace') as f:
            return f.read(limit)
    except OSError:
        return ''


def game_folder(appid, libs):
    for lib in libs:
        m = re.search(r'"installdir"\s+"([^"]+)"', _read(os.path.join(lib, 'steamapps', 'appmanifest_%s.acf' % appid)))
        if m and '/' not in m.group(1):
            d = os.path.join(lib, 'steamapps', 'common', m.group(1))
            if os.path.isdir(d):
                return d
    return None


def audio_files(folder, depth=3, limit=3000):
    out, seen = [], 0
    base = folder.rstrip('/').count('/')
    for d, dirs, files in os.walk(folder):
        if d.count('/') - base >= depth:
            dirs[:] = []
        dirs.sort()
        for f in sorted(files):
            seen += 1
            if seen > limit:
                return out
            if f.lower().endswith(AUDIO_EXTS):
                out.append(os.path.join(d, f))
    return out


THEME_WORDS = (('main theme', 9), ('title theme', 8), ('main title', 8), ('theme song', 7), ('main menu', 6),
               ('title screen', 6), ('menu theme', 6), ('opening', 4), ('overture', 4), ('theme', 3), ('title', 2), ('menu', 1))


def theme_score(name, forms=()):
    n = norm(os.path.splitext(os.path.basename(name))[0])
    best = 0
    for w, sc in THEME_WORDS:
        if has_words(n, w):
            best = max(best, sc)
    if best and any(has_words(n, f) for f in forms):
        best += 2
    return best


def track_number(path):
    m = re.match(r'\s*(?:disc\s*\d+\s*)?(\d{1,3})\b', norm(os.path.basename(path)))
    return int(m.group(1)) if m else 999


def from_soundtrack_folder(folder, forms):
    files = [f for f in audio_files(folder) if not f.lower().endswith('.wav') or os.path.getsize(f) < 80 << 20]
    if not files:
        return None
    files.sort(key=lambda f: (-theme_score(f, forms), track_number(f), f))
    return files[0]


def soundtrack(appid, title, h=None):
    """The song from a Steam soundtrack you have, or from the game's own files (only names that clearly say it's the
    main/title theme). (path, where) or None."""
    forms = title_forms(title)
    libs = libraries(h)
    if not forms and not appid:
        return None
    for lib in libs:
        for d in sorted(glob.glob(os.path.join(lib, 'steamapps', 'music', '*'))):
            n = norm(os.path.basename(d))
            if os.path.isdir(d) and any(has_words(n, f) for f in forms):
                f = from_soundtrack_folder(d, forms)
                if f:
                    return f, 'soundtrack'
        for d in sorted(glob.glob(os.path.join(lib, 'steamapps', 'common', '*'))):
            n = norm(os.path.basename(d))
            if os.path.isdir(d) and any(has_words(n, f) for f in forms) and re.search(r'\b(soundtrack|ost|original score)\b', n):
                f = from_soundtrack_folder(d, forms)
                if f:
                    return f, 'soundtrack'
    gd = game_folder(appid, libs) if str(appid).isdigit() else None
    if gd:
        best = None
        for f in audio_files(gd, depth=5, limit=6000):
            try:
                size = os.path.getsize(f)
            except OSError:
                continue
            sc = theme_score(f, forms)
            # Conservative: a strong theme word (score 6+, "main theme", "title theme", "main menu"...) and a real song's size.
            if sc >= 6 and 300 * 1024 <= size <= 60 * 1024 * 1024 and (best is None or sc > best[0]):
                best = (sc, f)
        if best:
            return best[1], 'game'
    return None


# ---- 3. Apple's preview, 4. Deezer's

BAD = ('cover', 'tribute', 'karaoke', 'lofi', 'lo fi', '8 bit', 'remix', 'parody', 'rap', 'piano version', 'ringtone',
       'medley', 'reaction', 'in the style of')
OST = ('soundtrack', 'ost', 'original score', 'original game', 'music from', 'game music', 'score')


def score(result, forms):
    """How well one result (iTunes' shape; Deezer's is turned into it) fits the game's theme song (0: not it)."""
    if not isinstance(result, dict) or not result.get('previewUrl'):
        return 0
    album, track, artist = norm(result.get('collectionName')), norm(result.get('trackName')), norm(result.get('artistName'))
    if not any(has_words(album, f) or has_words(track, f) for f in forms):
        return 0
    if any(has_words(x, b) for b in BAD for x in (album, track, artist)):
        return 0
    s = 2
    if any(has_words(album, o) for o in OST):
        s += 3
    tw = max([sc for w, sc in THEME_WORDS if has_words(track, w)] or [0])
    s += min(tw, 6)
    if any(has_words(album, f) for f in forms[:1]):
        s += 1
    return s


def preview_source(url):
    """'apple' or 'deezer' for an https preview address from their own servers, else None."""
    try:
        u = urllib.parse.urlparse(url)
    except ValueError:
        return None
    host = u.hostname or ''
    if u.scheme != 'https':
        return None
    if host.endswith(('.apple.com', '.mzstatic.com')):
        return 'apple'
    if host.endswith('.dzcdn.net'):
        return 'deezer'
    return None


def allowed_preview(url):
    return preview_source(url) is not None


def best_match(results, forms, source='apple'):
    ranked = sorted(((score(r, forms), r.get('trackNumber') or 99, i, r) for i, r in enumerate(results or [])
                     if isinstance(r, dict) and preview_source(r.get('previewUrl') or '') == source),
                    key=lambda x: (-x[0], x[1], x[2]))
    if not ranked or ranked[0][0] < 5:   # the title and a soundtrack album or a theme track at least
        return None
    return as_match(ranked[0][3], source, ranked[0][0])


def as_match(r, source, sc=0):
    m = {'source': source, 'id': str(r.get('trackId') or ''), 'track': str(r.get('trackName') or '')[:200],
         'artist': str(r.get('artistName') or '')[:200], 'album': str(r.get('collectionName') or '')[:200],
         'link': r.get('trackViewUrl') if str(r.get('trackViewUrl') or '').startswith('https://') else '', 'score': sc}
    if source == 'apple':
        m['preview'] = r['previewUrl']
    # Deezer's preview addresses expire: only the id is kept, the address is asked for when it plays.
    return m


def deezer_result(d):
    """One Deezer search result in iTunes' shape (so the same rules score it)."""
    if not isinstance(d, dict):
        return None
    return {'trackId': d.get('id'), 'trackName': d.get('title') or d.get('title_short'),
            'artistName': (d.get('artist') or {}).get('name') if isinstance(d.get('artist'), dict) else '',
            'collectionName': (d.get('album') or {}).get('title') if isinstance(d.get('album'), dict) else '',
            'previewUrl': d.get('preview') or '', 'trackViewUrl': d.get('link') or ''}


def backed_off(api, now):
    b = _backoff.get(api)
    return bool(b) and now < b[0]


def back_off(api, now):
    """A 429/403 (or Deezer's quota answer): no calls to API for a minute, doubling each time, up to an hour."""
    b = _backoff.get(api)
    step = min(BACKOFF_MAX, b[1] * 2) if b else BACKOFF_FIRST
    _backoff[api] = [now + step, step]
    log('%s asked to slow down: waiting %d s' % (api, step))


def calmed(api):
    _backoff.pop(api, None)   # an answer came: the next 429 starts at a minute again


def rate_ok(now, sleep=time.sleep):
    """Apple's limit: waits out the few seconds between calls; False when this minute's calls are used up."""
    with _rate_lock:
        while _calls and _calls[0] < now - 60:
            _calls.pop(0)
        if len(_calls) >= ITUNES_PER_MINUTE:
            return False
        if _calls and now - _calls[-1] < ITUNES_GAP:
            sleep(ITUNES_GAP - (now - _calls[-1]))
            now = _calls[-1] + ITUNES_GAP
        _calls.append(now)
        return True


def deezer_rate_ok(now, sleep=time.sleep):
    """Deezer: one call a second."""
    with _rate_lock:
        if _deezer_calls and now - _deezer_calls[-1] < DEEZER_GAP:
            sleep(DEEZER_GAP - (now - _deezer_calls[-1]))
            now = _deezer_calls[-1] + DEEZER_GAP
        _deezer_calls[:] = [now]
        return True


def _get_json(url, api, now):
    """One GET through _urlopen. None (try later) on a 429/403, which starts the back-off."""
    req = urllib.request.Request(url, headers={'User-Agent': 'HearthMusic/1 (Hearth OS)'})
    try:
        with _urlopen(req, timeout=15) as r:
            data = json.loads(r.read(2 * 1024 * 1024).decode('utf-8', 'replace'))
    except urllib.error.HTTPError as e:
        if e.code in (429, 403):
            back_off(api, now)
            return None
        raise
    if api == 'deezer' and isinstance(data, dict) and isinstance(data.get('error'), dict):
        if data['error'].get('code') == 4:   # "Quota limit exceeded"
            back_off(api, now)
            return None
        raise ValueError('deezer error %s' % data['error'].get('code'))
    calmed(api)
    return data


def _itunes_get(term, now=None, limit=25):
    """One call to Apple's search (rate limited). None when it's too soon or Apple asked to slow down (try later)."""
    now = time.time() if now is None else now
    if backed_off('apple', now) or not rate_ok(now):
        return None
    q = urllib.parse.urlencode({'term': term, 'media': 'music', 'entity': 'song', 'limit': limit})
    data = _get_json(ITUNES + '?' + q, 'apple', now)
    if data is None:
        return None
    return data.get('results') if isinstance(data, dict) else []


def _deezer_get(term, now=None, limit=25):
    """One call to Deezer's search (rate limited), its results in iTunes' shape. None: try later."""
    now = time.time() if now is None else now
    if backed_off('deezer', now) or not deezer_rate_ok(now):
        return None
    data = _get_json(DEEZER + '/search?' + urllib.parse.urlencode({'q': term, 'limit': limit}), 'deezer', now)
    if data is None:
        return None
    rows = data.get('data') if isinstance(data, dict) else None
    return [r for r in (deezer_result(d) for d in rows or []) if r]


_deezer_urls = {}   # track id -> (preview address, expires): in memory only


def _deezer_track(tid, now=None):
    now = time.time() if now is None else now
    if backed_off('deezer', now) or not deezer_rate_ok(now):
        return None
    return _get_json('%s/track/%s' % (DEEZER, int(tid)), 'deezer', now)


def deezer_preview(tid, get=None, now=None):
    """The current preview address of a Deezer track (they expire), asked for when it plays and kept in memory until
    shortly before it expires. None when it can't be had now."""
    now = time.time() if now is None else now
    tid = str(tid or '')
    if not tid.isdigit():
        return None
    hit = _deezer_urls.get(tid)
    if hit and now < hit[1] - 60:
        return hit[0]
    try:
        d = (get or _deezer_track)(tid)
    except (OSError, ValueError):
        return None
    url = d.get('preview') if isinstance(d, dict) else None
    if not url or preview_source(url) != 'deezer':
        return None
    m = re.search(r'exp=(\d+)', url)
    _deezer_urls[tid] = (url, int(m.group(1)) if m else now + 600)
    return url


def _fresh(hit, now, max_age=None):
    if not isinstance(hit, dict) or hit.get('local'):
        return False
    keep = KEEP_ERROR if hit.get('error') else (KEEP_MATCH if hit.get('match') else KEEP_NO_MATCH)
    if max_age is not None:
        keep = min(keep, max_age)
    return now - float(hit.get('at') or 0) < keep


def _usable(hit, apple, deezer):
    """Whether a kept answer still fits the switches: its source is on, and a "no match" asked every source now on."""
    m = hit.get('match')
    if m:
        return {'apple': apple, 'deezer': deezer}.get(m.get('source') or 'apple', False)
    if hit.get('error'):
        return True   # a failed call: asked again after an hour (KEEP_ERROR)
    tried = hit.get('tried') or ['apple']
    return (not apple or 'apple' in tried) and (not deezer or 'deezer' in tried)


def lookup(title, get=None, now=None, deezer=None, apple=True, use_deezer=True, max_age=None):
    """The best online preview for a title: Apple's, else Deezer's (only when Apple has no good match). From
    lookups.json when it was asked recently. {'match': ...|None} or {'later': True} when it can't be asked now.
    A failed call never changes a kept answer."""
    get = get or _itunes_get
    deezer = deezer or _deezer_get
    now = time.time() if now is None else now
    forms = title_forms(title)
    if not forms or not (apple or use_deezer):
        return {'match': None}
    key = forms[0]
    hit = load_json('lookups.json', {}, cache_dir()).get(key)
    if _fresh(hit, now, max_age) and _usable(hit, apple, use_deezer):
        return {'match': hit.get('match')}
    match, tried = None, []
    try:
        for src, ask in (('apple', get if apple else None), ('deezer', deezer if use_deezer else None)):
            if not ask:
                continue
            tried.append(src)
            for term in ('%s main theme' % title, '%s soundtrack' % title):
                res = ask(term)
                if res is None:
                    return {'later': True}
                m = best_match(res, forms, src)
                if m and (match is None or m['score'] > match['score']):
                    match = m
                if match and match['score'] >= 9:
                    break
            if match:
                break   # Deezer only when Apple has no good match
        entry = {'at': now, 'match': match, 'tried': tried}
    except (OSError, ValueError) as e:
        log('lookup: %s' % type(e).__name__)
        if isinstance(hit, dict) and (hit.get('match') or hit.get('local')):
            return {'match': hit.get('match'), 'error': True}   # the kept answer stays as it is
        entry = {'at': now, 'match': None, 'error': True, 'tried': tried}

    def put(cache):
        cache[key] = entry
        # Kept small: the newest 3000 lookups.
        if len(cache) > 3000:
            for k in sorted(cache, key=lambda k: cache[k].get('at', 0) if isinstance(cache[k], dict) else 0)[:len(cache) - 3000]:
                cache.pop(k)
    update_cache('lookups.json', put)
    log('lookup: %s' % ((match or {}).get('source') or 'no match'))
    return {'match': match, 'error': True} if entry.get('error') else {'match': match}


def _fetch(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'HearthMusic/1 (Hearth OS)'})
    with _urlopen(req, timeout=20) as r:
        return r.read(PREVIEW_LIMIT + 1)


# ---- the song, and its sound

def keyed(path):
    """Files (and preview addresses) the frontend may ask for are only those this backend chose: keys, never paths,
    cross the bridge."""
    k = hashlib.sha1(path.encode('utf-8', 'surrogateescape')).hexdigest()[:24]
    _keys[k] = path
    return k


def is_url(v):
    return isinstance(v, str) and v.startswith('https://')


_keys = {}

CREDIT = {'apple': 'Apple Music', 'deezer': 'Deezer'}


def online_song(m, apple=True, deezer=True, deezer_get=None):
    """A kept match (or an online pick) made playable: Apple's address as kept, Deezer's asked for now. A dict with
    url/key/names, or a silent answer."""
    src = m.get('source') or 'apple'
    if not {'apple': apple, 'deezer': deezer}.get(src):
        return {'source': None, 'why': '%s previews are off' % CREDIT.get(src, src)}
    url = m.get('preview') if src == 'apple' else deezer_preview(m.get('id'), get=deezer_get)
    if src == 'deezer' and not url:
        return {'source': None, 'later': True, 'why': 'looked up in a moment'}
    if preview_source(url or '') != src:
        return {'source': None, 'why': 'no theme song found'}
    # Streamed: the frontend plays the address itself; the key is for the fallback through this backend.
    return {'url': url, 'key': keyed(url), 'title': m.get('track') or '', 'artist': m.get('artist') or '',
            'album': m.get('album') or '', 'link': m.get('link') or '', 'from': src}


def find_song(appid, title, collections=None, online=True, get=None, h=None, deezer=True, deezer_get=None, deezer_track=None):
    appid = str(appid or '')
    title = str(title or '')[:200]
    p = picked(appid, collections)
    if p:
        if p.get('online'):
            o = online_song(p.pop('online'), online, deezer, deezer_track)
            if not o.get('url'):
                return dict(o, why='your pick: %s' % o.get('why'))
            return dict(p, **o)
        return dict(p, key=keyed(p.pop('path')))
    st = soundtrack(appid, title, h)
    if st:
        path, where = st
        return {'source': where, 'key': keyed(path), 'title': os.path.splitext(os.path.basename(path))[0]}
    if not online and not deezer:
        return {'source': None, 'why': 'no song of its own (online previews are off)'}
    lk = lookup(title, get=get, deezer=deezer_get, apple=online, use_deezer=deezer)
    if lk.get('later'):
        return {'source': None, 'later': True, 'why': 'looked up in a moment'}
    m = lk.get('match')
    if not m:
        return {'source': None, 'why': 'no theme song found'}
    o = online_song(m, online, deezer, deezer_track)
    if not o.get('url'):
        return o
    return dict(o, source=o['from'])


# ---- the library scan

_scan = {'running': False, 'paused': False, 'stop': False, 'total': 0, 'done': 0, 'matched': 0, 'waiting': '',
         'games': [], 'force': False, 'thread': None}
_scan_lock = threading.Lock()


def installed_games(h=None):
    """Steam games installed in any library (appmanifests): [{'appid', 'title'}], for when Steam's list isn't given."""
    out = []
    for lib in libraries(h):
        for f in sorted(glob.glob(os.path.join(lib, 'steamapps', 'appmanifest_*.acf'))):
            t = _read(f, 1 << 16)
            a, n = re.search(r'"appid"\s+"(\d+)"', t), re.search(r'"name"\s+"([^"]+)"', t)
            if a and n:
                out.append({'appid': a.group(1), 'title': n.group(1)})
    return out


def scan_order(games, h=None):
    """The scan's order: installed Steam games, then the rest of the Steam library, then non-Steam shortcuts (Hearth
    Sync's and others), each alphabetical; one per title. [{'appid', 'title', 'kind'}]."""
    inst = {g['appid'] for g in installed_games(h)}
    seen, rows = set(), []
    for g in list(games or []) + installed_games(h):
        if not isinstance(g, dict):
            continue
        appid, title = str(g.get('appid') or '')[:20], str(g.get('title') or '')[:200]
        forms = title_forms(title)
        if not appid.isdigit() or not forms or forms[0] in seen:
            continue
        seen.add(forms[0])
        shortcut = bool(g.get('shortcut')) or int(appid) >= 1 << 31
        rows.append({'appid': appid, 'title': title, 'kind': 'shortcut' if shortcut else 'steam',
                     'rank': 2 if shortcut else (0 if appid in inst else 1)})
    rows.sort(key=lambda r: (r['rank'], norm(r['title'])))
    for r in rows:
        r.pop('rank')
    return rows


def scan_state_file():
    return os.path.join(cache_dir(), 'scan.json')


def scan_library(games, settings=None, get=None, deezer=None, now=time.time, sleep=time.sleep, paused=lambda: False,
                 stopped=lambda: False, progress=None, force=False, h=None):
    """Goes through GAMES (scan_order) and keeps each one's best match in lookups.json: your pick wins (left alone),
    then a soundtrack you have, then Apple's preview, then Deezer's. Games answered recently are skipped, so a scan
    that stopped carries on where it was. Waits while a game runs (paused()) and while an API asks to slow down.
    Returns {'total', 'done', 'matched'}."""
    s = clean_settings(settings if settings is not None else load_json('settings.json', {}))
    rows = scan_order(games, h)
    st = {'total': len(rows), 'done': 0, 'matched': 0, 'waiting': ''}
    max_age = SCAN_FORCE_REFRESH if force else SCAN_REFRESH
    wait = 5

    def tell():
        if progress:
            progress(dict(st))
    tell()
    picks = load_json('picks.json', {})
    for r in rows:
        while True:
            if stopped():
                return st
            if paused():
                st['waiting'] = 'game'
                tell()
                sleep(1)
                continue
            key = title_forms(r['title'])[0]
            hit = load_json('lookups.json', {}, cache_dir()).get(key)
            if has_app_pick(r['appid'], picks):
                st['matched'] += 1   # your pick: never looked up, never changed
                break
            if isinstance(hit, dict) and hit.get('local') and now() - float(hit.get('at') or 0) < max_age:
                st['matched'] += 1
                break
            if _fresh(hit, now(), max_age) and _usable(hit, s['online'], s['deezer']) and not hit.get('error'):
                st['matched'] += 1 if hit.get('match') else 0
                break
            local = soundtrack(r['appid'], r['title'], h)
            if local:
                path, where = local
                entry = {'at': now(), 'local': where, 'track': os.path.splitext(os.path.basename(path))[0]}
                update_cache('lookups.json', lambda c: c.__setitem__(key, entry))
                st['matched'] += 1
                break
            if not (s['online'] or s['deezer']):
                break
            lk = lookup(r['title'], get=get, deezer=deezer, now=now(), apple=s['online'], use_deezer=s['deezer'],
                        max_age=max_age)
            if lk.get('later') or lk.get('error'):
                # Rate limit, an API asking to slow down, or the network gone: wait and try this game again.
                st['waiting'] = 'network' if lk.get('error') else 'slow'
                tell()
                sleep(wait)
                wait = min(BACKOFF_MAX, wait * 2) if lk.get('error') else 5
                continue
            wait = 5
            st['matched'] += 1 if lk.get('match') else 0
            break
        st['done'] += 1
        st['waiting'] = ''
        tell()
    return st


def scan_status():
    saved = load_json('scan.json', {}, cache_dir())
    out = {k: _scan[k] for k in ('running', 'paused', 'total', 'done', 'matched', 'waiting')}
    if not _scan['running'] and saved.get('total'):
        out.update(total=saved.get('total', 0), done=saved.get('total', 0), matched=saved.get('matched', 0))
    out['last'] = saved.get('last') or 0
    return out


def _scan_worker():
    try:
        while True:
            with _scan_lock:
                games, force = list(_scan['games']), _scan['force']
                _scan['force'] = False
                _scan['again'] = False
            st = scan_library(games, paused=lambda: _scan['paused'], stopped=lambda: _scan['stop'],
                              progress=lambda p: _scan.update(p), force=force)
            if _scan['stop']:
                return
            with _scan_lock:
                if _scan.get('again'):
                    continue   # new games (or "Scan my library") came in meanwhile
                save_cache('scan.json', {'last': time.time(), 'total': st['total'], 'matched': st['matched']})
                log('library scan: %d of %d matched' % (st['matched'], st['total']))
                return
    except Exception as e:  # never takes the plugin down
        log('library scan stopped: %s' % type(e).__name__)
    finally:
        _scan['running'] = False


def start_scan(games, force=False):
    """Starts (or feeds) the background scan with Steam's library. Games answered recently are skipped, so this is
    cheap to call often: the first time it goes through everything, then only new games and week-old answers."""
    with _scan_lock:
        changed = bool(games) and [str(g.get('appid')) for g in games] != [str(g.get('appid')) for g in _scan['games']]
        if games:
            _scan['games'] = list(games)
        _scan['force'] = _scan['force'] or bool(force)
        _scan['stop'] = False
        if _scan['running']:
            if changed or force:
                _scan['again'] = True   # new games or "Scan my library": one more pass when this one ends
            return True
        _scan['running'] = True
        t = threading.Thread(target=_scan_worker, name='hearth-music-scan', daemon=True)
        _scan['thread'] = t
    t.start()
    return True


# ---- "Choose music…"

_choices = {}


def search_music(query, apple=True, deezer=True, get=None, deezer_get=None, h=None):
    """What you can pick for a game: tracks of soundtracks you have whose folder matches, and Apple's and Deezer's
    previews (searched only when their switch is on). [{'choice', 'from', 'title', 'artist', 'album'}], plus 'later'
    when a source couldn't be asked now."""
    q = str(query or '').strip()[:120]
    forms = title_forms(q)
    out, later = [], []
    if not forms:
        return {'results': out, 'later': later}
    for lib in libraries(h):
        dirs = sorted(glob.glob(os.path.join(lib, 'steamapps', 'music', '*'))) + \
            [d for d in sorted(glob.glob(os.path.join(lib, 'steamapps', 'common', '*')))
             if re.search(r'\b(soundtrack|ost|original score)\b', norm(os.path.basename(d)))]
        for d in dirs:
            n = norm(os.path.basename(d))
            if os.path.isdir(d) and any(has_words(n, f) or all(has_words(n, w) for w in f.split()) for f in forms):
                files = audio_files(d)
                files.sort(key=lambda f: (-theme_score(f, forms), track_number(f), f))
                for f in files[:12]:
                    k = hashlib.sha1(f.encode('utf-8', 'surrogateescape')).hexdigest()[:16]
                    _choices[k] = f
                    out.append({'choice': k, 'from': 'soundtrack', 'title': os.path.splitext(os.path.basename(f))[0],
                                'artist': '', 'album': os.path.basename(d)})
    for src, on, ask in (('apple', apple, get or _itunes_get), ('deezer', deezer, deezer_get or _deezer_get)):
        if not on:
            continue
        try:
            res = ask(q)
        except (OSError, ValueError):
            res = None
        if res is None:
            later.append(src)
            continue
        n = 0
        for r in res:
            if n >= 8 or not isinstance(r, dict) or preview_source(r.get('previewUrl') or '') != src or not str(r.get('trackId') or '').isdigit():
                continue
            m = as_match(r, src)
            m.pop('score', None)
            k = '%s:%s' % (src, m['id'])
            _choices[k] = m
            out.append({'choice': k, 'from': src, 'title': m['track'], 'artist': m['artist'], 'album': m['album']})
            n += 1
    return {'results': out, 'later': later}


def choose_music(appid, choice):
    """Keeps what you chose in "Choose music…" as this game's pick (picks.json); it wins over every automatic match and
    no scan changes it."""
    v = _choices.get(str(choice))
    if v is None or not str(appid).isdigit():
        return False
    try:
        set_pick('app', str(appid), v)
        return True
    except ValueError:
        return False


def ffmpeg():
    return shutil.which('ffmpeg') or ('/usr/bin/ffmpeg' if os.path.isfile('/usr/bin/ffmpeg') else None)


def stream_preview(url, transcode=False, run=subprocess.run, fetch=None):
    """Apple's or Deezer's preview passed on when Steam's browser can't play its address itself: fetched into memory and handed
    over (with transcode, piped through ffmpeg into Ogg Opus). Nothing of it is written to disk."""
    src = preview_source(url)
    if not src:
        return None
    data = (fetch or _fetch)(url)
    if not data or len(data) > PREVIEW_LIMIT:
        return None
    mime = 'audio/mpeg' if src == 'deezer' else 'audio/mp4'
    if transcode:
        ff = ffmpeg()
        if not ff:
            return None
        r = run([ff, '-nostdin', '-v', 'error', '-i', 'pipe:0', '-vn', '-c:a', 'libopus', '-b:a', '128k', '-f', 'ogg', 'pipe:1'],
                input=data, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=60)
        if getattr(r, 'returncode', 1) != 0 or not r.stdout:
            return None
        data, mime = r.stdout, 'audio/ogg'
    return {'data': base64.b64encode(data).decode('ascii'), 'mime': mime}


def audio(key, transcode=False, run=subprocess.run, fetch=None):
    """The sound of a song this backend chose: {'data': base64, 'mime': ...}. With transcode, made into Ogg Opus first
    with ffmpeg (for a format Steam's browser can't play; your own files' conversions are kept in the cache)."""
    path = _keys.get(str(key))
    if is_url(path):
        try:
            return stream_preview(path, transcode, run=run, fetch=fetch)
        except (OSError, ValueError, subprocess.SubprocessError):
            return None
    if not path or not os.path.isfile(path):
        return None
    if transcode:
        ff = ffmpeg()
        if not ff:
            return None
        out = os.path.join(cache_dir(), 'ogg', hashlib.sha1(path.encode('utf-8', 'surrogateescape')).hexdigest()[:24] + '.ogg')
        if not os.path.isfile(out):
            os.makedirs(os.path.dirname(out), exist_ok=True)
            r = run([ff, '-nostdin', '-v', 'error', '-y', '-i', path, '-t', '600', '-vn', '-c:a', 'libopus', '-b:a', '128k', out + '.tmp.ogg'],
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120)
            if getattr(r, 'returncode', 1) != 0 or not os.path.isfile(out + '.tmp.ogg'):
                return None
            os.replace(out + '.tmp.ogg', out)
        path = out
    if os.path.getsize(path) > AUDIO_LIMIT:
        return None
    with open(path, 'rb') as f:
        data = f.read()
    return {'data': base64.b64encode(data).decode('ascii'), 'mime': MIMES.get(os.path.splitext(path)[1].lower(), 'audio/mpeg')}


class Plugin:
    async def get_settings(self):
        return clean_settings(load_json('settings.json', {}))

    async def set_settings(self, settings):
        """The keys the panel changed (only those are written); returns all of them as they are now."""
        return merge_settings(settings if isinstance(settings, dict) else {})

    async def find_song(self, appid, title, collections=None):
        s = clean_settings(load_json('settings.json', {}))
        cols = [str(c)[:200] for c in collections or [] if isinstance(c, str)][:30]
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: find_song(appid, title, cols, online=s['online'], deezer=s['deezer']))

    async def audio(self, key, transcode=False):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: audio(key, bool(transcode)))

    async def set_pick(self, kind, name, path):
        try:
            set_pick(kind, name, path)
            return True
        except ValueError:
            return False

    async def scan_library(self, games=None, force=False):
        """Steam's library from the frontend ([{appid, title, shortcut}]); starts or feeds the background scan."""
        rows = [g for g in (games or []) if isinstance(g, dict)][:20000]
        return start_scan(rows, bool(force))

    async def scan_pause(self, paused):
        """A game runs (or Play was pressed): the scan waits."""
        _scan['paused'] = bool(paused)
        return True

    async def scan_status(self):
        return scan_status()

    async def search_music(self, query):
        s = clean_settings(load_json('settings.json', {}))
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: search_music(query, s['online'], s['deezer']))

    async def choose_music(self, appid, choice):
        return choose_music(appid, choice)

    async def get_home(self):
        return home()

    async def get_picks(self):
        return load_json('picks.json', {})

    async def report(self, line):
        log(str(line)[:300])

    async def _main(self):
        log('Hearth Music ready')

    async def _unload(self):
        _scan['stop'] = True

    async def _uninstall(self):
        pass
