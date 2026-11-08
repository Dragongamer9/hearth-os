"""Hearth Music's backend (docs/HEARTH-MUSIC.md): finds the theme song of the game you're looking at in Steam Game Mode.

Runs as you (no root flag) inside Decky Loader. The frontend (src/) knows which game is highlighted and plays the
song; this file finds it, in this order:

  1. a file you picked for that game, or for one of its collections (picks.json)
  2. the game's own soundtrack, when you have it: Steam soundtrack folders (steamapps/music/<name>,
     steamapps/common/<Game> Soundtrack) and, conservatively, an audio file in the game's folder whose name says it's
     the main/title theme
  3. Apple's official 30-second preview from the iTunes Search API (itunes.apple.com/search), the best match for
     "<title> main theme" / "<title> soundtrack" from the game's own soundtrack (score(): nothing rather than a wrong
     song), streamed from Apple's address and never stored (the frontend
     plays it directly; if Steam's browser can't, this backend passes it on from memory); the panel shows the
     track, artist and a link to it.
  4. When Apple has no good match: Deezer's 30-second preview (api.deezer.com/search, no account or key), by the same
     rules. Deezer's preview addresses expire, so only the track id is kept and the address is asked for again when it
     plays. Streamed, never stored ("Preview from Deezer").
  5. When nothing above has the game's song: a YouTube video in YouTube's own small visible player (the frontend's
     mini player: the official embed, www.youtube-nocookie.com/embed/<id>, at least 200x200, YouTube's controls and
     branding kept). Found only with the official YouTube Data API v3 (search.list, embeddable videos), which needs
     an API key the person pastes in (youtube-key.json, 0600, never logged, sent only to googleapis.com). Only the
     video's id is kept (lookups.json), never any media; nothing of YouTube is ever downloaded or extracted. Without a
     key, a YouTube link pasted into "Choose music..." still works as the person's pick. No match: silence.

The library scan (scan_library) goes through every game in Steam's library in the background the first time, then
new games as they appear and stale answers weekly; it pauses while a game runs and resumes where it left off.
Apple's API is asked at most once every few seconds and 20 times a minute, Deezer's once a second; a 429/403 (or
Deezer's quota answer) backs off for a minute, doubling up to an hour. YouTube's search costs 100 of the key's
10,000 daily units: automatic searches (the scan, a highlighted game) stop at 8,000 a day, leaving the rest for
"Choose music...", and a quota answer stops YouTube until YouTube's day ends (midnight Pacific time). The answers (which match belongs to which game,
and "no match") are kept in lookups.json, saved atomically with a .bak. Only those answers are kept, never any audio.
Nothing else is sent anywhere. The log gets counts and reasons only.
"""
import asyncio
import base64
import glob
import hashlib
import html
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

# Hearth OS only, and the load mark for plugin updates (hosts/decky-shared/hearth_only.py, docs/PLUGIN-UPDATES.md):
# next to main.py when packed, in hosts/decky-shared in the repo.
import sys as _sys  # noqa: E402
_here = os.path.dirname(os.path.abspath(__file__))
for _p in (os.path.join(_here, '..', 'decky-shared'), _here):
    if _p not in _sys.path:
        _sys.path.insert(0, _p)
import hearth_only  # noqa: E402

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
DEFAULTS = {'enabled': True, 'volume': 0.35, 'delay': 1.5, 'where': 'pages', 'online': True, 'toast': True, 'deezer': True,
            'youtube': True}
# YouTube (source 5): the official Data API v3 only (search.list / videos.list), with the person's own key.
YT_API = 'https://www.googleapis.com/youtube/v3'
YT_SEARCH_COST = 100
YT_VIDEOS_COST = 1
YT_DAILY = 10000          # a key's default daily quota
YT_AUTO_BUDGET = 8000     # the scan and highlighted games stop here; the rest is for "Choose music..."
YT_GAP = 1.0
YT_ID = re.compile(r'^[A-Za-z0-9_-]{11}$')
YT_KEY = re.compile(r'^[A-Za-z0-9_-]{20,80}$')
YT_MIN = 7                # a video counts from this score

_calls = []   # times of Apple calls in the last minute
_deezer_calls = []   # time of the last Deezer call
_yt_calls = []       # time of the last YouTube call
_backoff = {}        # api -> [until, step]
_rate_lock = threading.Lock()
_urlopen = urllib.request.urlopen   # the one way out to the network (tests replace it)


def log(msg):
    if decky:
        decky.logger.info(redact_key(msg))


def redact_key(msg):
    """Never a YouTube API key in the log: the kept one, anything shaped like a Google key, a key= in an address."""
    msg = re.sub(r'AIza[0-9A-Za-z_-]{20,}', '[removed]', str(msg))
    msg = re.sub(r'([?&]key=)[^&\s]+', r'\1[removed]', msg)
    try:
        k = youtube_key()
    except Exception:
        k = ''
    return msg.replace(k, '[removed]') if k else msg


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
    for k in ('enabled', 'online', 'toast', 'deezer', 'youtube'):
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


NO_MUSIC = {'source': 'none'}   # "No music for this game": a pick too


def is_no_music(p):
    return isinstance(p, dict) and p.get('source') == 'none'


def online_pick(p):
    """A pick that is an Apple or Deezer preview or a YouTube video ("Choose music…"): only its id and names, never
    audio or video."""
    if not isinstance(p, dict) or p.get('source') not in ('apple', 'deezer', 'youtube'):
        return None
    pid = str(p.get('id') or '')
    if not (YT_ID.match(pid) if p['source'] == 'youtube' else pid.isdigit()):
        return None
    out = {'source': p['source'], 'id': pid[:20]}
    for k in ('track', 'artist', 'album'):
        out[k] = str(p.get(k) or '')[:200]
    link = str(p.get('link') or '')
    out['link'] = link if link.startswith('https://') else ''
    if p['source'] == 'youtube':
        out['link'] = youtube_watch(pid)
    if p['source'] == 'apple' and allowed_preview(p.get('preview') or ''):
        out['preview'] = p['preview']
    return out


def picked(appid, collections):
    picks = load_json('picks.json', {})
    for k in [pick_key('app', appid)] + [pick_key('collection', c) for c in collections or []]:
        p = picks.get(k)
        via = 'collection' if k.startswith('collection:') else 'game'
        if is_no_music(p):
            return {'source': 'pick', 'none': True, 'via': via, 'name': k.split(':', 1)[1]}
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
    if is_no_music(path):
        picks[k] = dict(NO_MUSIC)
    elif isinstance(path, dict):
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
    whole = match_forms(title)
    libs = libraries(h)
    if not forms and not appid:
        return None
    for lib in libs:
        for d in sorted(glob.glob(os.path.join(lib, 'steamapps', 'music', '*'))):
            n = norm(os.path.basename(d))
            # the folder is this game's soundtrack: its whole title and soundtrack words only ("Hollow Knight
            # Soundtrack" isn't Silksong's, "DOOM Soundtrack" isn't DOOM Eternal's)
            if os.path.isdir(d) and _title_then_filler(n, whole):
                f = from_soundtrack_folder(d, forms)
                if f:
                    return f, 'soundtrack'
        for d in sorted(glob.glob(os.path.join(lib, 'steamapps', 'common', '*'))):
            n = norm(os.path.basename(d))
            if os.path.isdir(d) and _title_then_filler(n, whole) and re.search(r'\b(soundtrack|ost|original score)\b', n):
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

# The matcher (matcher version MATCHER, kept with each answer in lookups.json: answers from an older matcher are
# looked up again once, and an old answer that fails today's rules is never played). It plays nothing rather than a
# wrong song: a result counts only when
#   - its album is the game's own soundtrack: the album's name is the game's title (™/®, years and edition words
#     left out) followed only by soundtrack words ("Original Soundtrack", "OST", "Original Score", "Music from",
#     "Vol. 2"...), never another word ("Inside Out", "Hades II", "Celeste: Farewell" aren't "Inside", "Hades",
#     "Celeste"), and not a film's or series' ("Motion Picture", "Netflix Series"...); or
#   - (titles of two words or more only) its track is the game's title plus a theme word ("Hades Main Theme") on an
#     album that names the game or is a soundtrack;
# and no cover, tribute, remix, piano/lo-fi/8-bit version or compilation, by title, album or artist.
MATCHER = 2

BAD = ('cover', 'covers', 'tribute', 'tributes', 'karaoke', 'lofi', 'lo fi', '8 bit', '16 bit', 'chiptune', 'remix', 'remixes',
       'remixed', 'parody', 'rap', 'piano version', 'piano covers', 'ringtone', 'medley', 'reaction', 'in the style of',
       'nightcore', 'lullaby', 'lullabies', 'music box', 'epic version', 'metal version', 'orchestral version',
       'acoustic version', 'guitar version', 'arrangement', 'arranged', 'made famous', 'originally performed',
       'sped up', 'slowed', 'instrumental version')
# Compilations and cover artists (album and artist only)
COMPILATION = ('greatest', 'best of', 'hits', 'video game themes', 'game themes', 'gaming', 'gamer', 'gamers', 'workout',
               'study', 'sleep', 'relaxing', 'compilation', 'playlist', 'players', 'piano guy', 'piano', 'covers',
               'various artists', 'orchestral covers', 'game music collective')
OST = ('soundtrack', 'ost', 'original score', 'score', 'music from', 'original game soundtrack', 'game soundtrack')
FILM = ('motion picture', 'film', 'movie', 'television', 'tv series', 'series', 'netflix', 'amazon original',
        'apple tv', 'musical', 'broadway', 'cast recording', 'season', 'episode', 'anime', 'documentary')
# Words an album or track may hold besides the title: soundtrack words and volume/edition words, never a word that
# could name something else.
FILLER = set('''a an the of and from to for by music musics soundtrack soundtracks ost osts original official game games
video videogame score scores selections selected select songs song tracks track themes theme main title opening menu
vol volume disc cd part pt deluxe edition complete expanded extended collection collectors special digital bonus
version remastered definitive anniversary goty year limited directors cut hd sound sounds sountrack o s t'''.split())
STRONG_THEME = ('main theme', 'title theme', 'main title', 'theme song', 'main menu', 'menu theme', 'title screen', 'theme')
EDITIONS = re.compile(r"\b(?:(?:game of the year|goty|definitive|complete|enhanced|anniversary|deluxe|digital deluxe|"
                      r"ultimate|gold|special|collector'?s|standard|premium|legendary|platinum|launch|day one|"
                      r"director'?s cut)\s+)?edition\b|\bgoty\b|\bremaster(?:ed)?\b|\bdirector'?s\s+cut\b|\bHD\b|"
                      r"\bgame of the year\b|\bthe final cut$", re.I)
BRANDS = ('tom clancys ', 'sid meiers ', 'marvels ', 'disneys ', 'disney pixar ')
ROMAN = {'2': 'ii', '3': 'iii', '4': 'iv', '5': 'v', '6': 'vi', '7': 'vii', '8': 'viii', '9': 'ix'}


def clean_title(title):
    """A Steam name made searchable: "DARK SOULS™ III: Deluxe Edition" -> "DARK SOULS III", "Tomb Raider (2013)" ->
    "Tomb Raider", "The Witcher® 3: Wild Hunt - Game of the Year Edition" -> "The Witcher 3: Wild Hunt"."""
    raw = str(title or '')
    t = re.sub(r'[™®©]', '', raw).replace('’', "'")
    t = re.sub(r'\s*[\(\[](?:19|20)\d\d[\)\]]', '', t)            # (2013)
    # a store's name some shortcuts carry: "Epic: Alan Wake 2", "Alan Wake 2 (GOG)"
    t = re.sub(r'^\s*(?:epic|epic games|gog|gog galaxy|amazon|amazon games|ea|ea app|origin|ubisoft|ubisoft connect|uplay|'
               r'battle\.net|itch|itch\.io|heroic|lutris|xbox|game pass)\s*[:\-–|]\s*', '', t, flags=re.I)
    t = re.sub(r'\s*[\(\[](?:epic|epic games|gog|amazon|ea|ea app|origin|ubisoft|uplay|battle\.net|itch\.io|heroic|'
               r'lutris|xbox|game pass|non-steam)[\)\]]\s*$', '', t, flags=re.I)
    t = EDITIONS.sub(' ', t)
    t = re.sub(r'[\(\[]\s*[\)\]]', ' ', t)                          # "()" left behind
    t = ' '.join(t.split())
    t = re.sub(r'(\s*[:–—-]\s*)+$', '', t).strip()
    return t if norm(t) else raw.strip()


def match_forms(title):
    """The forms of a game's title a result must hold, in full: "the witcher 3 wild hunt"; numbers both ways
    ("dark souls iii" / "dark souls 3"); a publisher's name in front left out when the rest is distinctive
    ("tom clancys the division 2" / "the division 2"). Never the part before a subtitle alone: "Hollow Knight:
    Silksong" isn't "Hollow Knight"."""
    base = norm(clean_title(title))
    if not base:
        return []
    forms = [base]
    for b in BRANDS:
        if base.startswith(b) and len(base[len(b):].replace(' ', '')) >= 6:
            forms.append(base[len(b):])
    for f in list(forms):
        w = f.split()
        if w[-1] in ROMAN:
            forms.append(' '.join(w[:-1] + [ROMAN[w[-1]]]))
        inv = {v: k for k, v in ROMAN.items()}
        if w[-1] in inv and len(w) > 1:
            forms.append(' '.join(w[:-1] + [inv[w[-1]]]))
        if w[0] == 'the' and len(w) > 2:
            forms.append(' '.join(w[1:]))
    out = []
    for f in forms:
        if f and f not in out:
            out.append(f)
    return out


def _rest_is_filler(rest):
    """Whether what's left of an album or track name besides the title is only soundtrack/volume words (anything after
    "by" or "feat" is the composer)."""
    rest = re.split(r'\b(by|feat|featuring|ft|composed)\b', rest)[0]
    rest = re.sub(r'\b(vol|volume|disc|cd|part|pt|chapter|act)\s*(\d+|[ivx]+|one|two|three)\b', ' ', rest)
    return all(w in FILLER for w in rest.split())


def _title_then_filler(name, forms):
    """NAME (normalised) is one of the game's title forms followed only by filler; leading "music from (the)",
    "original soundtrack (from|of)" and "the" are allowed."""
    n = re.sub(r'^(the )?(original )?(game )?(music|songs|soundtrack|score|ost)( from| of| to)?( the)? ', '', name)
    for cand in {name, n, re.sub(r'^the ', '', n)}:
        for f in forms:
            if cand == f or cand.startswith(f + ' '):
                if _rest_is_filler(cand[len(f):]):
                    return f
    return None


def score(result, forms):
    """How surely one result (iTunes' shape; Deezer's is turned into it) is the game's theme song: 0 when it isn't
    clearly the game's own music, else higher for the game's own soundtrack album and a theme-named track."""
    if not isinstance(result, dict) or not result.get('previewUrl') or not forms:
        return 0
    album, track, artist = norm(result.get('collectionName')), norm(result.get('trackName')), norm(result.get('artistName'))
    # (a word of the game's own title never counts against it: "The Walking Dead: Season Two", "Sleep Tight")
    if any(has_words(x, b) for b in BAD for x in (album, track, artist) if not has_words(forms[0], b)):
        return 0
    if any(has_words(x, b) for b in COMPILATION for x in (album, artist) if not has_words(forms[0], b)):
        return 0
    filmish = any(has_words(album, f) for f in FILM if not has_words(forms[0], f)) and not has_words(album, 'game')
    if filmish:
        return 0
    has_ost = any(has_words(album, o) for o in OST)
    tw = max([sc for w, sc in THEME_WORDS if has_words(track, w)] or [0])
    s = 0
    by_album = _title_then_filler(album, forms) if has_ost else None
    if by_album:
        s = 10 if by_album == forms[0] else 9
        s += min(tw, 6) / 2.0
        if _title_then_filler(track, forms):
            s += 2   # the title track
        if str(result.get('primaryGenreName') or '').lower() in ('video game', 'video games', 'videogame'):
            s += 0.5
        return s
    # No soundtrack album of the game: only a track that is clearly the game's theme ("Hades Main Theme"), never for a
    # one-word title ("Control", "Prey", "Inside" name too many other things).
    by_track = _title_then_filler(track, forms)
    strong = any(has_words(track, w) for w in STRONG_THEME)
    if by_track and strong and len(by_track.split()) >= 2 and (has_ost or any(has_words(album, f) for f in forms)):
        return 6 + min(tw, 6) / 2.0
    return 0


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
    if not ranked or ranked[0][0] < 6:   # the game's own soundtrack album, or its clearly named theme
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
    title = str(d.get('title') or d.get('title_short') or '')
    ver = str(d.get('title_version') or '').strip()
    if ver and norm(ver) not in norm(title):
        title = '%s %s' % (title, ver)   # "(Remix)", "(Piano Version)": the same rules see it
    return {'trackId': d.get('id'), 'trackName': title,
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


# ---- 5. YouTube: the official embed (the frontend's mini player), found with the official YouTube Data API v3
#
# Only a video's id and names are kept, never media. Nothing of YouTube is fetched here except the Data API's JSON
# answers from googleapis.com; the video plays in YouTube's own visible player in Steam's page.

def youtube_watch(vid):
    """The video's page on YouTube (the panel's "Open on YouTube")."""
    return 'https://www.youtube.com/watch?v=%s' % vid if YT_ID.match(str(vid or '')) else ''


def youtube_id(link):
    """The video id of a pasted YouTube link (youtube.com/watch?v=, youtu.be/, /shorts/, /embed/, /live/), or None."""
    s = str(link or '').strip()[:500]
    if not s:
        return None
    if '://' not in s:
        s = 'https://' + s
    try:
        u = urllib.parse.urlparse(s)
    except ValueError:
        return None
    host = (u.hostname or '').lower()
    if host.startswith('www.') or host.startswith('m.'):
        host = host.split('.', 1)[1]
    if host == 'music.youtube.com':
        host = 'youtube.com'
    vid = None
    if host == 'youtu.be':
        vid = u.path.strip('/').split('/')[0]
    elif host in ('youtube.com', 'youtube-nocookie.com'):
        parts = [x for x in u.path.split('/') if x]
        if parts[:1] == ['watch']:
            vid = (urllib.parse.parse_qs(u.query).get('v') or [''])[0]
        elif len(parts) >= 2 and parts[0] in ('shorts', 'embed', 'live', 'v'):
            vid = parts[1]
    return vid if vid and YT_ID.match(vid) else None


def youtube_key_file():
    return os.path.join(settings_dir(), 'youtube-key.json')


def youtube_key():
    """The API key the person pasted in, or ''. Read only here; never logged, never handed to the frontend."""
    v = _read_json(youtube_key_file(), repair=False)
    k = v.get('key') if isinstance(v, dict) else None
    return k if isinstance(k, str) and YT_KEY.match(k) else ''


def set_youtube_key(key):
    """Keeps the person's key in youtube-key.json (0600, its .bak too), or forgets it ('' or None). True when kept or
    forgotten, False when it doesn't look like a key (nothing changes then)."""
    key = str(key or '').strip()
    path = youtube_key_file()
    if not key:
        for p in (path, path + '.bak'):
            try:
                os.remove(p)
            except OSError:
                pass
        _yt_state_update(lambda st: st.pop('bad_key', None))
        return True
    if not YT_KEY.match(key):
        return False
    os.makedirs(settings_dir(), exist_ok=True)
    _safe_write(path, json.dumps({'key': key}).encode())
    for p in (path, path + '.bak'):
        try:
            os.chmod(p, 0o600)
        except OSError:
            pass
    _yt_state_update(lambda st: st.pop('bad_key', None))   # a new key: asked again
    return True


def _pacific_day(now):
    """YouTube's quota day (it starts at midnight Pacific time): (day, the time it ends)."""
    try:
        from zoneinfo import ZoneInfo
        import datetime
        tz = ZoneInfo('America/Los_Angeles')
        d = datetime.datetime.fromtimestamp(now, tz)
        end = (d + datetime.timedelta(days=1)).replace(hour=0, minute=0, second=0, microsecond=0)
        return d.strftime('%Y-%m-%d'), end.timestamp()
    except Exception:   # no time zone data: Pacific standard time
        t = now - 8 * 3600
        return time.strftime('%Y-%m-%d', time.gmtime(t)), (t // 86400 + 1) * 86400 + 8 * 3600


def _yt_state():
    v = load_json('youtube.json', {}, cache_dir())
    return v if isinstance(v, dict) else {}


def _yt_state_update(fn):
    try:
        return update_cache('youtube.json', fn)
    except OSError:
        return {}


def _yt_units(st, now):
    day, _ = _pacific_day(now)
    return int(st.get('units') or 0) if st.get('day') == day else 0


def youtube_ready(budget=YT_AUTO_BUDGET, cost=YT_SEARCH_COST, now=None, key=None):
    """'' when YouTube can be asked now, else why not: 'key' (none), 'bad_key', 'quota' (used up for today), 'slow'."""
    now = time.time() if now is None else now
    if not (youtube_key() if key is None else key):
        return 'key'
    st = _yt_state()
    if st.get('bad_key'):
        return 'bad_key'
    if now < float(st.get('until') or 0) or _yt_units(st, now) + cost > budget:
        return 'quota'
    if backed_off('youtube', now):
        return 'slow'
    return ''


def youtube_status(now=None):
    """For the panel: whether a key is kept (never the key), whether YouTube can be asked, and today's units."""
    now = time.time() if now is None else now
    st = _yt_state()
    why = youtube_ready(YT_DAILY, YT_SEARCH_COST, now)
    return {'key': why != 'key', 'state': why or 'ok', 'used': _yt_units(st, now), 'daily': YT_DAILY,
            'auto': YT_AUTO_BUDGET}


def _yt_spend(now, cost):
    day, _ = _pacific_day(now)

    def put(st):
        if st.get('day') != day:
            st['day'], st['units'] = day, 0
        st['units'] = int(st.get('units') or 0) + cost
    _yt_state_update(put)


def _yt_request(path, params, now, cost, budget):
    """One call to the YouTube Data API (googleapis.com only). None when it can't be asked now (no key, a key YouTube
    refused, today's quota used up, or YouTube asked to slow down). The key never reaches the log."""
    key = youtube_key()
    if youtube_ready(budget, cost, now, key):
        return None
    with _rate_lock:
        if _yt_calls and now - _yt_calls[-1] < YT_GAP:
            time.sleep(YT_GAP - (now - _yt_calls[-1]))
        _yt_calls[:] = [max(now, (_yt_calls or [0])[-1] + YT_GAP)]
    url = '%s/%s?%s' % (YT_API, path, urllib.parse.urlencode(dict(params, key=key)))
    req = urllib.request.Request(url, headers={'User-Agent': 'HearthMusic/1 (Hearth OS)', 'Accept': 'application/json'})
    try:
        with _urlopen(req, timeout=15) as r:
            data = json.loads(r.read(2 * 1024 * 1024).decode('utf-8', 'replace'))
    except urllib.error.HTTPError as e:
        reason = ''
        try:
            body = json.loads(e.read(65536).decode('utf-8', 'replace'))
            errs = (body.get('error') or {}).get('errors') or []
            reason = str((errs[0] if errs else {}).get('reason') or '')
            if not reason and 'api key not valid' in str((body.get('error') or {}).get('message') or '').lower():
                reason = 'keyInvalid'
        except Exception:
            pass
        if reason in ('quotaExceeded', 'dailyLimitExceeded'):
            _, end = _pacific_day(now)
            _yt_state_update(lambda st: st.update(until=end))
            log('youtube: today\'s quota is used up; more after midnight Pacific time')
            return None
        if reason in ('keyInvalid', 'keyExpired', 'accessNotConfigured', 'forbidden', 'ipRefererBlocked') or e.code == 400 and reason.startswith('key'):
            _yt_state_update(lambda st: st.update(bad_key=True))
            log('youtube: the API key was refused (%s)' % (reason or e.code))
            return None
        if e.code in (429, 403) or reason in ('rateLimitExceeded', 'userRateLimitExceeded'):
            back_off('youtube', now)
            return None
        raise ValueError('youtube error %s' % e.code)
    calmed('youtube')
    _yt_spend(now, cost)
    return data if isinstance(data, dict) else {}


def _yt_item(it):
    """One search or videos result as {'id', 'title', 'channel', 'description'}, or None."""
    if not isinstance(it, dict):
        return None
    vid = it.get('id')
    if isinstance(vid, dict):
        vid = vid.get('videoId')
    sn = it.get('snippet') if isinstance(it.get('snippet'), dict) else {}
    if not isinstance(vid, str) or not YT_ID.match(vid):
        return None
    return {'id': vid, 'title': html.unescape(str(sn.get('title') or ''))[:300],
            'channel': html.unescape(str(sn.get('channelTitle') or ''))[:200],
            'description': html.unescape(str(sn.get('description') or ''))[:500]}


def _youtube_get(term, now=None, budget=YT_AUTO_BUDGET, limit=10):
    """One search.list (100 units): embeddable videos only. [items] or None (can't be asked now)."""
    now = time.time() if now is None else now
    data = _yt_request('search', {'part': 'snippet', 'type': 'video', 'videoEmbeddable': 'true', 'maxResults': limit,
                                  'q': term, 'safeSearch': 'moderate'}, now, YT_SEARCH_COST, budget)
    if data is None:
        return None
    return [x for x in (_yt_item(i) for i in data.get('items') or []) if x]


def _youtube_video(vid, now=None):
    """videos.list (1 unit) for a pasted link: its names, and whether it may be embedded. None: can't be asked now;
    {} when YouTube doesn't know it."""
    now = time.time() if now is None else now
    data = _yt_request('videos', {'part': 'snippet,status', 'id': vid}, now, YT_VIDEOS_COST, YT_DAILY)
    if data is None:
        return None
    for it in data.get('items') or []:
        x = _yt_item(it)
        if x and x['id'] == vid:
            x['embeddable'] = bool((it.get('status') or {}).get('embeddable', True))
            return x
    return {}


# Words in a video's title that say it isn't the game's music as released (besides BAD's covers and remixes).
YT_BAD = ('trailer', 'teaser', 'gameplay', 'walkthrough', 'playthrough', 'review', 'reaction', 'speedrun', 'lets play',
          'let s play', 'cutscene', 'cutscenes', 'explained', 'tutorial', 'guide', 'boss fight', 'all bosses', 'fan made',
          'fanmade', 'unofficial', 'mashup', 'mix', 'live', 'concert', 'asmr', 'reverb', 'bass boosted', 'hour', 'hours',
          'loop', 'looped', 'orchestra', 'symphony', 'ai', 'meme', 'shorts', 'animation', 'animated', 'amv', 'gmv')
_YT_SEP = re.compile(r'\s+[-~]\s+|[|–—:()\[\]/•·「」【】"]')


def _yt_tokens(raw):
    """A video title as words, with '|' where it had a separator (" - ", "|", ":", brackets...)."""
    out = []
    for piece in _YT_SEP.split(str(raw or '')):
        w = norm(piece).split()
        if w:
            out += w + ['|']
    return out


def _yt_find(tokens, form):
    """Where FORM's words stand in TOKENS (separators inside it skipped, "The Witcher 3: Wild Hunt"): (start, end)."""
    fw = form.split()
    for i in range(len(tokens)):
        j, k = i, 0
        while j < len(tokens) and k < len(fw):
            if tokens[j] == '|' and k > 0:
                j += 1
                continue
            if tokens[j] != fw[k]:
                break
            j, k = j + 1, k + 1
        if k == len(fw):
            return i, j
    return None


def _names_soundtrack(text, forms):
    """TEXT names one of the game's soundtrack albums: a title form followed by soundtrack words only up to an OST
    word ("Hades (Original Soundtrack)", not "Hades II Original Soundtrack")."""
    t = _yt_tokens(text)
    for f in forms:
        for i in range(len(t)):
            at = _yt_find(t[i:], f)
            if not at or at[0] != 0:
                continue
            nxt = [w for w in t[i + at[1]:i + at[1] + 6] if w != '|']
            for w in nxt:
                if any(w == o or o.startswith(w + ' ') for o in OST) or w == 'ost':
                    return True
                if w not in FILLER:
                    break
    return False


def youtube_score(v, forms):
    """How surely a video is the game's theme song, by the same rules as Apple's and Deezer's (BAD, COMPILATION, FILM;
    the game's whole title followed by soundtrack words, never another word: "Hades II", "Hollow Knight Silksong");
    official channels ("- Topic", "Official", the game's own) count for it. 0 when it isn't clearly the game's music."""
    if not isinstance(v, dict) or not YT_ID.match(str(v.get('id') or '')) or not forms:
        return 0
    raw_t, raw_c = str(v.get('title') or ''), str(v.get('channel') or '')
    t, c, d = norm(raw_t), norm(raw_c), norm(v.get('description'))
    own = forms[0]
    if any(has_words(x, b) for b in BAD for x in (t, c) if not has_words(own, b)):
        return 0
    if any(has_words(t, b) for b in YT_BAD if not has_words(own, b)):
        return 0
    if any(has_words(x, b) for b in COMPILATION for x in (t, c) if not has_words(own, b)):
        return 0
    if any(has_words(t, f) for f in FILM if not has_words(own, f)) and not has_words(t, 'game'):
        return 0
    topic = raw_c.strip().lower().endswith(' - topic')
    bonus = 2 if topic else 0
    if has_words(c, 'official') or c.endswith('vevo') or any(c == f or c.startswith(f + ' ') for f in forms):
        bonus = max(bonus, 1.5)
    tokens = _yt_tokens(raw_t)
    for f in sorted(forms, key=len, reverse=True):
        at = _yt_find(tokens, f)
        if not at:
            continue
        i, j = at
        before = tokens[i - 1] if i else '|'
        after = tokens[j] if j < len(tokens) else '|'
        if before not in ('|', 'the', 'official', 'original', 'ost', 'soundtrack') or (after != '|' and after not in FILLER):
            continue   # "Hades II", "Hollow Knight Silksong", "Halsey Control"
        rest = [w for w in tokens[:i] + tokens[j:] if w != '|']
        seg_end = tokens.index('|', j) if '|' in tokens[j:] else len(tokens)
        after_words = ' '.join(w for w in tokens[j:] if w != '|')
        ost = any(has_words(after_words, o) for o in OST) or _names_soundtrack(raw_t, forms)
        tw = max([sc for w, sc in THEME_WORDS if has_words(t, w)] or [0])
        strong = any(has_words(t, w) for w in STRONG_THEME)
        if all(w in FILLER for w in rest):
            # the game's title and soundtrack words only: "Hades - Main Theme", "Celeste Original Soundtrack"
            if len(f.split()) < 2 and not ost and not bonus:
                return 0   # "Control - Main Theme" could be anything called Control (unless its channel is official)
            if not (ost or strong):
                return 0   # just the title: a trailer, a let's play...
            s = 7 + min(tw, 6) / 2.0
        elif ost and all(w in FILLER for w in tokens[j:seg_end]):
            # "<title> OST - <track>": a track of the game's soundtrack
            s = 7 + min(tw, 6) / 4.0
        else:
            return 0
        if any(has_words(t, w) for w in ('full', 'complete', 'all songs', 'all tracks')):
            s -= 1   # a whole soundtrack in one video: right, but not its theme
        return s + bonus
    # A "- Topic" channel's video is a track of an album; its description names the album ("Provided to YouTube by
    # ... Hades (Original Soundtrack)").
    if topic and _names_soundtrack(v.get('description') or '', forms):
        if any(has_words(d, b) for b in BAD if not has_words(own, b)):
            return 0
        tw = max([sc for w, sc in THEME_WORDS if has_words(t, w)] or [0])
        return 7 + min(tw, 6) / 2.0 + bonus
    return 0


def youtube_best(items, forms):
    ranked = sorted(((youtube_score(v, forms), i, v) for i, v in enumerate(items or []) if isinstance(v, dict)),
                    key=lambda x: (-x[0], x[1]))
    if not ranked or ranked[0][0] < YT_MIN:
        return None
    return youtube_match(ranked[0][2], ranked[0][0])


def youtube_match(v, sc=0):
    return {'source': 'youtube', 'id': v['id'], 'track': str(v.get('title') or '')[:200],
            'artist': str(v.get('channel') or '')[:200], 'album': '', 'desc': str(v.get('description') or '')[:300],
            'link': youtube_watch(v['id']), 'score': sc}


def _current(hit):
    """An online answer from today's matcher (an older matcher's answers are looked up again once)."""
    return isinstance(hit, dict) and hit.get('v') == MATCHER


def still_fits(m, title):
    """Whether a kept match (from any matcher) passes today's rules for TITLE; one that doesn't is never played."""
    if not isinstance(m, dict):
        return False
    if m.get('source') == 'youtube':
        v = {'id': m.get('id'), 'title': m.get('track'), 'channel': m.get('artist'), 'description': m.get('desc')}
        return youtube_score(v, match_forms(title)) >= YT_MIN
    r = {'trackName': m.get('track'), 'collectionName': m.get('album'), 'artistName': m.get('artist'),
         'previewUrl': m.get('preview') or 'https://x.dzcdn.net/'}
    return score(r, match_forms(title)) >= 6


def _fresh(hit, now, max_age=None):
    if not isinstance(hit, dict) or hit.get('local') or not _current(hit):
        return False
    keep = KEEP_ERROR if hit.get('error') else (KEEP_MATCH if hit.get('match') else KEEP_NO_MATCH)
    if max_age is not None:
        keep = min(keep, max_age)
    return now - float(hit.get('at') or 0) < keep


def _usable(hit, apple, deezer, youtube=False, yt_search=False):
    """Whether a kept answer still fits the switches: its source is on, and a "no match" asked every source now on
    (YouTube only counts when it can be searched: a key is kept and today's quota isn't used up)."""
    m = hit.get('match')
    if m:
        return {'apple': apple, 'deezer': deezer, 'youtube': youtube}.get(m.get('source') or 'apple', False)
    if hit.get('error'):
        return True   # a failed call: asked again after an hour (KEEP_ERROR)
    tried = hit.get('tried') or ['apple']
    return (not apple or 'apple' in tried) and (not deezer or 'deezer' in tried) and (not yt_search or 'youtube' in tried)


def lookup(title, get=None, now=None, deezer=None, apple=True, use_deezer=True, max_age=None, youtube=None, youtube_on=False):
    """The best online preview for a title: Apple's, else Deezer's (only when Apple has no good match), else (with
    youtube_on and YOUTUBE, the search, given because it may be asked now) a YouTube video for the mini player. From
    lookups.json when it was asked recently (a kept video plays while youtube_on, the mini player's switch, even when
    YouTube can't be searched now); a recent "no match" asks only the sources it didn't ask yet.
    {'match': ...|None} or {'later': True} when it can't be asked now. A failed call never changes a kept answer.
    YouTube that can't be asked now (quota, key) is left out of this answer and asked another time."""
    get = get or _itunes_get
    deezer = deezer or _deezer_get
    yt = youtube if youtube_on else None
    now = time.time() if now is None else now
    keys = title_forms(title)
    forms = match_forms(title)
    if not keys or not forms or not (apple or use_deezer or youtube_on):
        return {'match': None}
    key = keys[0]
    query = clean_title(title)
    hit = load_json('lookups.json', {}, cache_dir()).get(key)
    if isinstance(hit, dict) and hit.get('match') and not still_fits(hit['match'], title):
        hit = dict(hit, match=None, at=0)   # an older matcher's wrong song: never played, looked up again
    if _fresh(hit, now, max_age) and _usable(hit, apple, use_deezer, youtube_on, bool(yt)):
        return {'match': hit.get('match')}
    # A recent "no match" (YouTube came later, or had no quota then): only the sources it didn't ask.
    prior = hit if (_fresh(hit, now, max_age) and not hit.get('match') and not hit.get('error')) else None
    done = list((prior or {}).get('tried') or ([] if not prior else ['apple']))
    match, tried = None, list(done)
    try:
        for src, ask in (('apple', get if apple else None), ('deezer', deezer if use_deezer else None), ('youtube', yt)):
            if not ask or src in done:
                continue
            terms = ('%s main theme OST' % query, '%s soundtrack' % query) if src == 'youtube' else \
                ('%s main theme' % query, '%s soundtrack' % query)
            asked = False
            kept = hit.get('match') if isinstance(hit, dict) else None
            if src == 'youtube' and isinstance(kept, dict) and kept.get('source') == 'youtube':
                match = kept   # the video found before still fits today's rules: no search spent on it again
                tried.append(src)
                break
            for term in terms:
                res = ask(term)
                if res is None and src == 'youtube':
                    break   # quota used up, key refused, asked to slow down: YouTube another time
                if res is None:
                    if isinstance(hit, dict) and hit.get('match'):
                        return {'match': hit['match'], 'later': True}   # the kept answer (it fits today's rules) meanwhile
                    return {'later': True}
                asked = True
                m = youtube_best(res, forms) if src == 'youtube' else best_match(res, forms, src)
                if m and (match is None or m['score'] > match['score']):
                    match = m
                if match and (match['score'] >= 12 or src == 'youtube'):   # the soundtrack album's theme, a video
                    break
            if asked and src not in tried:
                tried.append(src)
            if match:
                break   # Deezer only when Apple has no good match, YouTube only when neither has
        entry = {'at': now, 'match': match, 'tried': tried, 'v': MATCHER}
        if prior and not match and tried == done:
            return {'match': None}   # nothing could be asked: the kept answer stays as it was
    except (OSError, ValueError) as e:
        log('lookup: %s' % type(e).__name__)
        if isinstance(hit, dict) and (hit.get('match') or hit.get('local')):
            return {'match': hit.get('match'), 'error': True}   # the kept answer stays as it is
        entry = {'at': now, 'match': None, 'error': True, 'tried': tried, 'v': MATCHER}

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

CREDIT = {'apple': 'Apple Music', 'deezer': 'Deezer', 'youtube': 'YouTube'}


def online_song(m, apple=True, deezer=True, deezer_get=None, youtube=True):
    """A kept match (or an online pick) made playable: Apple's address as kept, Deezer's asked for now, a YouTube
    video's id for the mini player (YouTube's own embed plays it; nothing of it passes through here). A dict with
    url (or youtube)/key/names, or a silent answer."""
    src = m.get('source') or 'apple'
    if src == 'youtube':
        if not youtube:
            return {'source': None, 'why': 'the YouTube mini player is off'}
        vid = str(m.get('id') or '')
        if not YT_ID.match(vid):
            return {'source': None, 'why': 'no theme song found'}
        return {'youtube': vid, 'key': 'yt:' + vid, 'title': m.get('track') or '', 'artist': m.get('artist') or '',
                'album': '', 'link': youtube_watch(vid), 'from': 'youtube'}
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


def playable(o):
    return bool(o.get('url') or o.get('youtube'))


def youtube_search_now(budget=YT_AUTO_BUDGET):
    """The YouTube search to use now, or None when it can't be asked (no key, refused key, today's quota used up)."""
    if youtube_ready(budget):
        return None
    return lambda term: _youtube_get(term, budget=budget)


def find_song(appid, title, collections=None, online=True, get=None, h=None, deezer=True, deezer_get=None, deezer_track=None,
              youtube=False, yt_get=None):
    """The song for a game: your pick, the game's soundtrack, Apple's preview, Deezer's, then (youtube: the mini player's
    switch) a YouTube video, searched with yt_get (default: the Data API while a key is kept and quota is left)."""
    appid = str(appid or '')
    title = str(title or '')[:200]
    p = picked(appid, collections)
    if p:
        if p.get('none'):
            return {'source': None, 'pick': 'none', 'why': 'you chose no music for this game'}
        if p.get('online'):
            o = online_song(p.pop('online'), online, deezer, deezer_track, youtube)
            if not playable(o):
                return dict(o, why='your pick: %s' % o.get('why'))
            return dict(p, **o)
        return dict(p, key=keyed(p.pop('path')))
    st = soundtrack(appid, title, h)
    if st:
        path, where = st
        return {'source': where, 'key': keyed(path), 'title': os.path.splitext(os.path.basename(path))[0]}
    if not online and not deezer and not youtube:
        return {'source': None, 'why': 'no song of its own (online previews are off)'}
    if youtube and yt_get is None:
        yt_get = youtube_search_now()
    lk = lookup(title, get=get, deezer=deezer_get, apple=online, use_deezer=deezer, youtube=yt_get, youtube_on=youtube)
    if lk.get('later') and not lk.get('match'):
        return {'source': None, 'later': True, 'why': 'looked up in a moment'}
    m = lk.get('match')
    if not m:
        return {'source': None, 'why': 'no theme song found'}
    o = online_song(m, online, deezer, deezer_track, youtube)
    if not playable(o):
        return o
    return dict(o, source=o['from'])


# ---- the library scan

_scan = {'running': False, 'paused': False, 'stop': False, 'total': 0, 'done': 0, 'matched': 0, 'waiting': '',
         'current': '', 'recheck': False, 'all': False, 'cancelled': False,
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
        r['installed'] = r.pop('rank') != 1   # installed Steam games and non-Steam shortcuts
    return rows


def all_games(games, h=None):
    """What "Scan all games" and the review list go through: installed Steam games and non-Steam shortcuts."""
    return [r for r in scan_order(games, h) if r['installed']]


def scan_state_file():
    return os.path.join(cache_dir(), 'scan.json')


def scan_library(games, settings=None, get=None, deezer=None, now=time.time, sleep=time.sleep, paused=lambda: False,
                 stopped=lambda: False, progress=None, force=False, h=None, recheck=False, yt_get=None):
    """Goes through GAMES (scan_order) and keeps each one's best match in lookups.json: your pick wins (left alone),
    then a soundtrack you have, then Apple's preview, then Deezer's, then (the mini player's switch on, a key kept and
    today's automatic YouTube quota left; checked for every game, so it stops for the day when the quota runs out and
    goes on the next day) a YouTube video, only for games nothing else matched. Games answered recently are skipped, so a scan
    that stopped carries on where it was. Waits while a game runs (paused()) and while an API asks to slow down.
    recheck ("Scan all games"): every installed game and shortcut is looked at again with today's matcher, the kept
    automatic answers ignored (your picks still never). Returns {'total', 'done', 'matched'}."""
    s = clean_settings(settings if settings is not None else load_json('settings.json', {}))
    rows = all_games(games, h) if recheck else scan_order(games, h)
    st = {'total': len(rows), 'done': 0, 'matched': 0, 'waiting': '', 'current': ''}
    max_age = 0 if recheck else (SCAN_FORCE_REFRESH if force else SCAN_REFRESH)
    wait = 5

    def tell():
        if progress:
            progress(dict(st))
    tell()
    picks = load_json('picks.json', {})
    for r in rows:
        st['current'] = r['title']
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
            if not recheck and isinstance(hit, dict) and hit.get('local') and now() - float(hit.get('at') or 0) < max_age:
                st['matched'] += 1
                break
            yt = None
            if s['youtube']:
                yt = yt_get if yt_get is not None else youtube_search_now()
            if _fresh(hit, now(), max_age) and _usable(hit, s['online'], s['deezer'], s['youtube'], bool(yt)) and not hit.get('error'):
                st['matched'] += 1 if hit.get('match') else 0
                break
            local = soundtrack(r['appid'], r['title'], h)
            if local:
                path, where = local
                entry = {'at': now(), 'local': where, 'track': os.path.splitext(os.path.basename(path))[0]}
                update_cache('lookups.json', lambda c: c.__setitem__(key, entry))
                st['matched'] += 1
                break
            if not (s['online'] or s['deezer'] or yt):
                if recheck and isinstance(hit, dict) and hit.get('local'):
                    update_cache('lookups.json', lambda c: c.pop(key, None))   # its soundtrack is gone
                break
            lk = lookup(r['title'], get=get, deezer=deezer, now=now(), apple=s['online'], use_deezer=s['deezer'],
                        max_age=max_age, youtube=yt, youtube_on=s['youtube'])
            if lk.get('later') or lk.get('error'):   # (also when a kept answer was handed back meanwhile)
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
    out = {k: _scan[k] for k in ('running', 'paused', 'total', 'done', 'matched', 'waiting', 'current', 'cancelled')}
    out['all'] = bool(_scan['running'] and _scan['all'])   # "Scan all games" (n of N, cancellable)
    if _scan['cancelled'] and not _scan['running']:
        return dict(out, last=saved.get('last') or 0)
    if not _scan['running'] and saved.get('total'):
        out.update(total=saved.get('total', 0), done=saved.get('total', 0), matched=saved.get('matched', 0))
    out['last'] = saved.get('last') or 0
    return out


def _scan_worker():
    try:
        while True:
            with _scan_lock:
                games, force, recheck = list(_scan['games']), _scan['force'], _scan['recheck']
                _scan['force'] = _scan['recheck'] = False
                _scan['again'] = False
                _scan['all'] = recheck
            st = scan_library(games, paused=lambda: _scan['paused'], stopped=lambda: _scan['stop'],
                              progress=lambda p: _scan.update(p), force=force, recheck=recheck)
            if _scan['stop']:
                return
            with _scan_lock:
                if _scan.get('again'):
                    continue   # new games (or "Scan my library") came in meanwhile
                save_cache('scan.json', {'last': time.time(), 'total': st['total'], 'matched': st['matched'],
                                         'all': time.time() if recheck else load_json('scan.json', {}, cache_dir()).get('all', 0)})
                log('library scan: %d of %d matched' % (st['matched'], st['total']))
                return
    except Exception as e:  # never takes the plugin down
        log('library scan stopped: %s' % type(e).__name__)
    finally:
        _scan['running'] = False
        _scan['all'] = False
        _scan['current'] = ''


def start_scan(games, force=False, recheck=False):
    """Starts (or feeds) the background scan with Steam's library. Games answered recently are skipped, so this is
    cheap to call often: the first time it goes through everything, then only new games and week-old answers."""
    with _scan_lock:
        changed = bool(games) and [str(g.get('appid')) for g in games] != [str(g.get('appid')) for g in _scan['games']]
        if games:
            _scan['games'] = list(games)
        _scan['force'] = _scan['force'] or bool(force)
        _scan['recheck'] = _scan['recheck'] or bool(recheck)
        _scan['stop'] = False
        _scan['cancelled'] = False
        if _scan['running']:
            if changed or force or recheck:
                _scan['again'] = True   # new games or "Scan my library": one more pass when this one ends
            return True
        _scan['running'] = True
        t = threading.Thread(target=_scan_worker, name='hearth-music-scan', daemon=True)
        _scan['thread'] = t
    t.start()
    return True


def cancel_scan():
    """Stops the scan where it is ("Cancel" in the panel); every answer found so far is kept, nothing else changes."""
    with _scan_lock:
        if _scan['running']:
            _scan['stop'] = True
            _scan['cancelled'] = True
            _scan['recheck'] = _scan['again'] = False
    return True


# ---- the review list: every installed game and shortcut, grouped

def _song_of_pick(p):
    if isinstance(p, str):
        return {'title': os.path.splitext(os.path.basename(p))[0], 'artist': '', 'album': '', 'from': 'file'}
    if is_no_music(p):
        return {'title': '', 'artist': '', 'album': '', 'from': 'none'}
    o = online_pick(p)
    if o:
        return {'title': o['track'], 'artist': o['artist'], 'album': o['album'], 'from': o['source']}
    return None


def review(games, h=None):
    """Each installed game and non-Steam shortcut with what it plays, for the review list: 'pick' (yours, also "No
    music"), 'auto' (found automatically: a soundtrack you have, Apple's or Deezer's preview) or 'none' (nothing found
    yet, 'checked' False when it was never looked up). Read only: nothing is looked up or written here."""
    picks = load_json('picks.json', {})
    cache = load_json('lookups.json', {}, cache_dir())
    out = []
    for r in all_games(games, h):
        row = {'appid': r['appid'], 'title': r['title'], 'kind': r['kind'], 'query': clean_title(r['title'])}
        song = _song_of_pick(picks.get(pick_key('app', r['appid'])))
        if song:
            row.update(group='pick', song=song)
            out.append(row)
            continue
        hit = cache.get(title_forms(r['title'])[0])
        hit = hit if isinstance(hit, dict) else None
        m = hit.get('match') if hit else None
        if hit and hit.get('local'):
            row.update(group='auto', song={'title': hit.get('track') or '', 'artist': '', 'album': '',
                                           'from': 'game' if hit['local'] == 'game' else 'soundtrack'})
        elif m and (_current(hit) or still_fits(m, r['title'])):
            row.update(group='auto', song={'title': m.get('track') or '', 'artist': m.get('artist') or '',
                                           'album': m.get('album') or '', 'from': m.get('source') or 'apple'})
        else:
            row.update(group='none', song=None, checked=bool(hit and _current(hit) and not hit.get('error')))
        out.append(row)
    return out


# ---- "Choose music…"

_choices = {}


def youtube_choice(v):
    k = 'youtube:%s' % v['id']
    m = youtube_match(v)
    m.pop('score', None)
    _choices[k] = m
    return {'choice': k, 'from': 'youtube', 'title': m['track'] or 'YouTube video', 'artist': m['artist'], 'album': ''}


def search_music(query, apple=True, deezer=True, get=None, deezer_get=None, h=None, youtube=False, yt_get=None, yt_video=None):
    """What you can pick for a game: tracks of soundtracks you have whose folder matches, and Apple's and Deezer's
    previews (searched only when their switch is on), and YouTube videos (the mini player's switch on and a key kept).
    A pasted YouTube link is that video (with or without a key). [{'choice', 'from', 'title', 'artist', 'album'}],
    plus 'later' when a source couldn't be asked now, and 'note' for a link that can't be used."""
    raw = str(query or '').strip()[:500]
    vid = youtube_id(raw) if re.search(r'youtu\.?be', raw, re.I) else None
    if vid:
        if not youtube:
            return {'results': [], 'later': [], 'note': 'Turn on the YouTube mini player to use a YouTube video.'}
        v = None
        if yt_video is not None or not youtube_ready(YT_DAILY, YT_VIDEOS_COST):
            try:
                v = (yt_video or _youtube_video)(vid)   # 1 unit: its names, and whether it may be embedded
            except (OSError, ValueError):
                v = None
            if v == {}:
                return {'results': [], 'later': [], 'note': "YouTube doesn't know that video."}
            if v and not v.get('embeddable', True):
                return {'results': [], 'later': [], 'note': "That video's owner doesn't allow it to play outside YouTube."}
        return {'results': [youtube_choice(v or {'id': vid, 'title': 'YouTube video (%s)' % vid, 'channel': ''})], 'later': []}
    q = raw[:120]
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
    if youtube:
        ask = yt_get or (None if youtube_ready(YT_DAILY) else (lambda term: _youtube_get(term, budget=YT_DAILY)))
        if ask:
            try:
                res = ask(q)
            except (OSError, ValueError):
                res = None
            if res is None:
                later.append('youtube')
            else:
                fm = match_forms(q)
                ranked = sorted(enumerate(r for r in res if isinstance(r, dict) and YT_ID.match(str(r.get('id') or ''))),
                                key=lambda x: (-youtube_score(x[1], fm), x[0]))
                for _, v in ranked[:6]:
                    out.append(youtube_choice(v))
    return {'results': out, 'later': later}


def choice_preview(choice, deezer_get=None):
    """A "Choose music…" result made playable to hear it before choosing: {'key', 'url'} (Apple's address as found,
    Deezer's asked for now, your soundtrack's file through its key) or None. Streamed, never stored."""
    v = _choices.get(str(choice))
    if isinstance(v, str):
        return {'key': keyed(v)} if os.path.isfile(v) else None
    if not isinstance(v, dict):
        return None
    if v.get('source') == 'youtube':   # YouTube's own player plays it (the mini player)
        return {'key': 'yt:' + v['id'], 'youtube': v['id']} if YT_ID.match(str(v.get('id') or '')) else None
    url = v.get('preview') if v.get('source') == 'apple' else deezer_preview(v.get('id'), get=deezer_get)
    if not url or preview_source(url) != v.get('source'):
        return None
    return {'key': keyed(url), 'url': url}


def no_music(appid):
    """"No music for this game": kept as your pick (picks.json), so it stays silent and no scan changes it."""
    if not str(appid).isdigit():
        return False
    set_pick('app', str(appid), dict(NO_MUSIC))
    return True


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
    async def hearth_os(self):
        """Whether this is Hearth OS (the frontend shows the Hearth OS panel instead of the plugin when not)."""
        return hearth_only.status(log, 'Hearth Music')

    async def get_settings(self):
        return clean_settings(load_json('settings.json', {}))

    async def set_settings(self, settings):
        """The keys the panel changed (only those are written); returns all of them as they are now."""
        return merge_settings(settings if isinstance(settings, dict) else {})

    async def find_song(self, appid, title, collections=None):
        s = clean_settings(load_json('settings.json', {}))
        cols = [str(c)[:200] for c in collections or [] if isinstance(c, str)][:30]
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: find_song(appid, title, cols, online=s['online'], deezer=s['deezer'],
                                                                     youtube=s['youtube']))

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
        if not hearth_only.status(log, 'Hearth Music')['ok']:
            return False   # not Hearth OS: the scan never starts
        rows = [g for g in (games or []) if isinstance(g, dict)][:20000]
        return start_scan(rows, bool(force))

    async def scan_all(self, games=None):
        """"Scan all games": every installed game and shortcut looked at again with today's matcher (picks kept)."""
        if not hearth_only.status(log, 'Hearth Music')['ok']:
            return False
        rows = [g for g in (games or []) if isinstance(g, dict)][:20000]
        return start_scan(rows, recheck=True)

    async def scan_cancel(self):
        return cancel_scan()

    async def review(self, games=None):
        rows = [g for g in (games or []) if isinstance(g, dict)][:20000]
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: review(rows))

    async def choice_preview(self, choice):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: choice_preview(choice))

    async def no_music(self, appid):
        try:
            return no_music(appid)
        except (ValueError, OSError):
            return False

    async def scan_pause(self, paused):
        """A game runs (or Play was pressed): the scan waits."""
        _scan['paused'] = bool(paused)
        return True

    async def scan_status(self):
        return scan_status()

    async def search_music(self, query):
        s = clean_settings(load_json('settings.json', {}))
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: search_music(query, s['online'], s['deezer'], youtube=s['youtube']))

    async def youtube_status(self):
        """Whether a YouTube API key is kept (never the key itself), whether YouTube can be asked, today's units."""
        return youtube_status()

    async def set_youtube_key(self, key):
        """The key the person pasted ('' forgets it). Kept only in youtube-key.json (0600); never logged."""
        try:
            ok = set_youtube_key(key if isinstance(key, str) else '')
        except OSError:
            ok = False
        return dict(youtube_status(), saved=bool(ok))

    async def choose_music(self, appid, choice):
        return choose_music(appid, choice)

    async def clean_title(self, title):
        """The game's name as "Choose music…" searches it (™/®, years and edition words left out)."""
        return clean_title(str(title or '')[:200])

    async def get_home(self):
        return home()

    async def get_picks(self):
        return load_json('picks.json', {})

    async def report(self, line):
        log(redact_key(str(line)[:300]))

    async def _main(self):
        if not hearth_only.status(log, 'Hearth Music')['ok']:
            return   # not Hearth OS: nothing starts (the panel says why)
        log('Hearth Music ready')
        self._loaded = asyncio.get_running_loop().create_task(hearth_only.mark_loaded_later(decky))

    async def _unload(self):
        _scan['stop'] = True

    async def _uninstall(self):
        pass
