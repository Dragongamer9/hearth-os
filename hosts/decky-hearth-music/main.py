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
     track, artist and a link to it. No match: silence. Never YouTube or anything that downloads from it.

Apple's API is asked at most once every few seconds and 20 times a minute; answers (and "no match") are kept for weeks
(lookups.json), so a game is looked up once. Only those answers are kept, never Apple's audio. Nothing else is sent anywhere. The log gets counts and reasons only.
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
import time
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
DEFAULTS = {'enabled': True, 'volume': 0.35, 'delay': 1.5, 'where': 'pages', 'online': True, 'toast': True}

_calls = []   # times of Apple calls in the last minute


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
    """A cache file (Apple's answers): only replaced whole and atomically; it's no setting (losing it means asking again)."""
    d = cache_dir()
    os.makedirs(d, exist_ok=True)
    tmp = os.path.join(d, name + '.tmp')
    with open(tmp, 'w') as f:
        json.dump(value, f)
    os.replace(tmp, os.path.join(d, name))


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
    for k in ('enabled', 'online', 'toast'):
        if isinstance(s.get(k), bool):
            out[k] = s[k]
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


def picked(appid, collections):
    picks = load_json('picks.json', {})
    for k in [pick_key('app', appid)] + [pick_key('collection', c) for c in collections or []]:
        p = picks.get(k)
        if isinstance(p, str) and os.path.isfile(p) and p.lower().endswith(AUDIO_EXTS):
            return {'source': 'pick', 'path': p, 'title': os.path.splitext(os.path.basename(p))[0],
                    'via': 'collection' if k.startswith('collection:') else 'game', 'name': k.split(':', 1)[1]}
    return None


def set_pick(kind, name, path):
    if kind not in ('app', 'collection') or not str(name):
        raise ValueError('bad pick')
    picks = load_json('picks.json', {})
    k = pick_key(kind, str(name)[:200])
    if path:
        if not (os.path.isfile(path) and path.lower().endswith(AUDIO_EXTS)):
            raise ValueError('not an audio file')
        picks[k] = path
    else:
        picks.pop(k, None)
    save_json('picks.json', picks)
    return picks


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


# ---- 3. Apple's preview

BAD = ('cover', 'tribute', 'karaoke', 'lofi', 'lo fi', '8 bit', 'remix', 'parody', 'rap', 'piano version', 'ringtone',
       'medley', 'reaction', 'in the style of')
OST = ('soundtrack', 'ost', 'original score', 'original game', 'music from', 'game music', 'score')


def score(result, forms):
    """How well one iTunes result fits the game's theme song (0: not it)."""
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


def allowed_preview(url):
    try:
        u = urllib.parse.urlparse(url)
    except ValueError:
        return False
    return u.scheme == 'https' and (u.hostname or '').endswith(('.apple.com', '.mzstatic.com'))


def best_match(results, forms):
    ranked = sorted(((score(r, forms), r.get('trackNumber') or 99, i, r) for i, r in enumerate(results or [])
                     if isinstance(r, dict) and allowed_preview(r.get('previewUrl') or '')), key=lambda x: (-x[0], x[1], x[2]))
    if not ranked or ranked[0][0] < 5:   # the title and a soundtrack album or a theme track at least
        return None
    r = ranked[0][3]
    return {'track': str(r.get('trackName') or '')[:200], 'artist': str(r.get('artistName') or '')[:200],
            'album': str(r.get('collectionName') or '')[:200], 'preview': r['previewUrl'],
            'link': r.get('trackViewUrl') if str(r.get('trackViewUrl') or '').startswith('https://') else '', 'score': ranked[0][0]}


def rate_ok(now, sleep=time.sleep):
    """Apple's limit: waits out the few seconds between calls; False when this minute's calls are used up."""
    while _calls and _calls[0] < now - 60:
        _calls.pop(0)
    if len(_calls) >= ITUNES_PER_MINUTE:
        return False
    if _calls and now - _calls[-1] < ITUNES_GAP:
        sleep(ITUNES_GAP - (now - _calls[-1]))
        now = _calls[-1] + ITUNES_GAP
    _calls.append(now)
    return True


def _itunes_get(term, now=None):
    """One call to Apple's search (rate limited). None when it's too soon (try later)."""
    if not rate_ok(time.time() if now is None else now):
        return None
    q = urllib.parse.urlencode({'term': term, 'media': 'music', 'entity': 'song', 'limit': 25})
    req = urllib.request.Request(ITUNES + '?' + q, headers={'User-Agent': 'HearthMusic/1 (Hearth OS)'})
    with urllib.request.urlopen(req, timeout=15) as r:
        data = json.loads(r.read(2 * 1024 * 1024).decode('utf-8', 'replace'))
    return data.get('results') if isinstance(data, dict) else []


def lookup(title, get=None, now=None):
    """Apple's best preview for a title, from lookups.json when it was asked recently. {'match': ...|None} or
    {'later': True} when Apple can't be asked right now."""
    get = get or _itunes_get
    now = time.time() if now is None else now
    forms = title_forms(title)
    if not forms:
        return {'match': None}
    key = forms[0]
    cache = load_json('lookups.json', {}, cache_dir())
    hit = cache.get(key)
    if isinstance(hit, dict):
        keep = KEEP_ERROR if hit.get('error') else (KEEP_MATCH if hit.get('match') else KEEP_NO_MATCH)
        if now - float(hit.get('at') or 0) < keep:
            return {'match': hit.get('match')}
    match = None
    try:
        for term in ('%s main theme' % title, '%s soundtrack' % title):
            res = get(term)
            if res is None:
                return {'later': True}
            m = best_match(res, forms)
            if m and (match is None or m['score'] > match['score']):
                match = m
            if match and match['score'] >= 9:
                break
        entry = {'at': now, 'match': match}
    except (OSError, ValueError) as e:
        log('apple: %s' % type(e).__name__)
        entry = {'at': now, 'match': None, 'error': True}
    cache[key] = entry
    # Kept small: the newest 2000 lookups.
    if len(cache) > 2000:
        for k in sorted(cache, key=lambda k: cache[k].get('at', 0) if isinstance(cache[k], dict) else 0)[:len(cache) - 2000]:
            cache.pop(k)
    save_cache('lookups.json', cache)
    log('apple lookup: %s' % ('match' if match else 'no match'))
    return {'match': match}


def _fetch(url):
    req = urllib.request.Request(url, headers={'User-Agent': 'HearthMusic/1 (Hearth OS)'})
    with urllib.request.urlopen(req, timeout=20) as r:
        return r.read(PREVIEW_LIMIT + 1)


# ---- the song, and its sound

def keyed(path):
    """Files (and Apple preview addresses) the frontend may ask for are only those this backend chose: keys, never
    paths, cross the bridge."""
    k = hashlib.sha1(path.encode('utf-8', 'surrogateescape')).hexdigest()[:24]
    _keys[k] = path
    return k


def is_url(v):
    return isinstance(v, str) and v.startswith('https://')


_keys = {}


def find_song(appid, title, collections=None, online=True, get=None, h=None):
    appid = str(appid or '')
    title = str(title or '')[:200]
    p = picked(appid, collections)
    if p:
        return dict(p, key=keyed(p.pop('path')))
    st = soundtrack(appid, title, h)
    if st:
        path, where = st
        return {'source': where, 'key': keyed(path), 'title': os.path.splitext(os.path.basename(path))[0]}
    if not online:
        return {'source': None, 'why': 'no song of its own (Apple Music previews are off)'}
    lk = lookup(title, get=get)
    if lk.get('later'):
        return {'source': None, 'later': True, 'why': 'looked up in a moment'}
    m = lk.get('match')
    if not m:
        return {'source': None, 'why': 'no theme song found'}
    if not allowed_preview(m.get('preview') or ''):
        return {'source': None, 'why': 'no theme song found'}
    # Streamed: the frontend plays Apple's address itself; the key is for the fallback through this backend.
    return {'source': 'apple', 'url': m['preview'], 'key': keyed(m['preview']), 'title': m['track'], 'artist': m['artist'],
            'album': m['album'], 'link': m['link']}


def ffmpeg():
    return shutil.which('ffmpeg') or ('/usr/bin/ffmpeg' if os.path.isfile('/usr/bin/ffmpeg') else None)


def stream_preview(url, transcode=False, run=subprocess.run, fetch=None):
    """Apple's preview passed on when Steam's browser can't play its address itself: fetched into memory and handed
    over (with transcode, piped through ffmpeg into Ogg Opus). Nothing of it is written to disk."""
    if not allowed_preview(url):
        return None
    data = (fetch or _fetch)(url)
    if not data or len(data) > PREVIEW_LIMIT:
        return None
    mime = 'audio/mp4'
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
        return await loop.run_in_executor(None, lambda: find_song(appid, title, cols, online=s['online']))

    async def audio(self, key, transcode=False):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: audio(key, bool(transcode)))

    async def set_pick(self, kind, name, path):
        try:
            set_pick(kind, name, path)
            return True
        except ValueError:
            return False

    async def get_home(self):
        return home()

    async def get_picks(self):
        return load_json('picks.json', {})

    async def report(self, line):
        log(str(line)[:300])

    async def _main(self):
        log('Hearth Music ready')

    async def _unload(self):
        pass

    async def _uninstall(self):
        pass
