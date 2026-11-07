"""Hearth's Proton detector (docs/WINDOWS-APPS.md): which Proton a Windows program starts with when you haven't
picked one, and what to try next when it closes right away.

The order (candidates()):
  1. the Proton it last worked with on this device (remembered in ~/.local/share/nimbus/proton-worked.json);
  2. what Hearth's bundled table says (/usr/share/hearth/proton-rules.json; never fetched live);
  3. the GE-Proton built into Hearth OS (/usr/share/steam/compatibilitytools.d/GE-Proton*), which works offline;
  4. the newest Valve Proton Steam has installed;
  5. older GE-Protons: ones you downloaded before, then ones Hearth can download (only after you say yes).
A Proton that closed right away for that program goes to the end of the list. Hearth Games reads the same table and
the same file (hosts/hearth-core/src/proton_pick.rs auto_for).

Used by hearth-winapps (Windows apps). Settings-style files are written with hearth_settings_io (atomic, .bak).
"""
import fnmatch
import hashlib
import json
import os
import re
import shutil
import sys
import tarfile
import tempfile
import time
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
try:
    import hearth_settings_io as hsio
except ImportError:  # a copy run on its own elsewhere
    hsio = None

SYSTEM_DIR = '/usr/share/steam/compatibilitytools.d'
RULES = '/usr/share/hearth/proton-rules.json'
GE_URL = 'https://github.com/GloriousEggroll/proton-ge-custom/releases/download/{0}/{0}.tar.gz'
CRASH_SECONDS = 15


def home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def system_dir():
    return os.environ.get('HEARTH_BUILTIN_PROTON_DIR') or SYSTEM_DIR


def rules_path():
    return os.environ.get('HEARTH_PROTON_RULES') or RULES


def worked_path():
    return os.path.join(home(), '.local', 'share', 'nimbus', 'proton-worked.json')


def user_dir():
    """Where older GE-Protons Hearth downloads go (not Steam's folders: Steam's settings are never touched)."""
    return os.path.join(home(), '.local', 'share', 'hearth', 'proton')


def version_key(name):
    return [int(x) for x in re.findall(r'\d+', name)]


def usable(path):
    return os.path.isfile(os.path.join(path, 'proton'))


def load_rules():
    try:
        with open(rules_path(), encoding='utf-8') as f:
            d = json.load(f)
        return d if isinstance(d, dict) else {}
    except (OSError, ValueError):
        return {}


def _tools_in(d, prefix=''):
    try:
        names = os.listdir(d)
    except OSError:
        return []
    return [(n, os.path.join(d, n)) for n in names if n.startswith(prefix) and usable(os.path.join(d, n))]


def builtin():
    """(name, path) of the newest GE-Proton built into Hearth OS, or None."""
    found = sorted(_tools_in(system_dir(), 'GE-Proton'), key=lambda t: version_key(t[0]))
    return found[-1] if found else None


def steam_roots():
    h = home()
    out = []
    for r in (os.path.join(h, '.local/share/Steam'), os.path.join(h, '.steam/steam'), os.path.join(h, '.steam/root'),
              os.path.join(h, '.var/app/com.valvesoftware.Steam/.local/share/Steam')):
        try:
            real = os.path.realpath(r)
        except OSError:
            continue
        if os.path.isdir(real) and real not in out:
            out.append(real)
    return out


def valve():
    """(name, path) of the newest Valve Proton Steam has installed (a numbered one before Experimental), or None."""
    found = []
    for r in steam_roots():
        for n, p in _tools_in(os.path.join(r, 'steamapps', 'common'), 'Proton'):
            if 'hotfix' in n.lower() or 'easyanticheat' in n.lower().replace(' ', ''):
                continue
            k = version_key(n)
            found.append(((1, k) if k else (0, []), n, p))
    found.sort()
    return (found[-1][1], found[-1][2]) if found else None


def downloaded():
    """Older GE-Protons Hearth downloaded, and ones in Steam's own compatibility folder: {name: path}."""
    out = {}
    for d in [user_dir()] + [os.path.join(r, 'compatibilitytools.d') for r in steam_roots()]:
        for n, p in _tools_in(d, 'GE-Proton'):
            out.setdefault(n, p)
    return out


def find(name):
    """The folder of a Proton by its name, wherever it is on this device, or None."""
    if not name:
        return None
    b = builtin()
    if name == 'builtin':
        return b[1] if b else None
    if name == 'valve':
        v = valve()
        return v[1] if v else None
    for n, p in _tools_in(system_dir()):
        if n == name:
            return p
    d = downloaded()
    if name in d:
        return d[name]
    for r in steam_roots():
        p = os.path.join(r, 'steamapps', 'common', name)
        if usable(p):
            return p
    return None


def rule_for(exe=None, game=None, rules=None):
    """The first rule in the table for a program's file name or a Hearth Games id, or None."""
    rules = load_rules() if rules is None else rules
    base = os.path.basename(exe or '').lower()
    for r in rules.get('rules', []) if isinstance(rules.get('rules'), list) else []:
        m = r.get('match') if isinstance(r, dict) else None
        if not isinstance(m, dict):
            continue
        if game and m.get('game') == game:
            return r
        if base and isinstance(m.get('exe'), str) and fnmatch.fnmatch(base, m['exe'].lower()):
            return r
    return None


def _load_worked():
    try:
        if hsio:
            d = hsio.load_json(worked_path(), {})
        else:
            with open(worked_path(), encoding='utf-8') as f:
                d = json.load(f)
    except Exception:  # noqa: BLE001 - a damaged file means "nothing remembered", never a crash
        d = {}
    return d if isinstance(d, dict) else {}


def remembered(key):
    e = _load_worked().get(key)
    return e if isinstance(e, dict) else {}


def _save_worked(key, change):
    def apply(d):
        d = d if isinstance(d, dict) else {}
        e = d.get(key) if isinstance(d.get(key), dict) else {}
        change(e)
        e['at'] = int(time.time())
        d[key] = e
        return d
    if hsio:
        hsio.update_json(worked_path(), apply)
    else:
        os.makedirs(os.path.dirname(worked_path()), exist_ok=True)
        d = apply(_load_worked())
        tmp = worked_path() + '.tmp'
        with open(tmp, 'w', encoding='utf-8') as f:
            json.dump(d, f, indent=2)
        os.replace(tmp, worked_path())


def note_worked(key, name):
    """It ran: start with this Proton next time; it's no longer on the closed-right-away list."""
    def ch(e):
        e['worked'] = name
        e['failed'] = [n for n in e.get('failed', []) if n != name]
    _save_worked(key, ch)


def note_failed(key, name):
    """It closed right away with this Proton: try the next one first next time."""
    def ch(e):
        f = [n for n in e.get('failed', []) if isinstance(n, str) and n != name]
        e['failed'] = (f + [name])[-12:]
        if e.get('worked') == name:
            e.pop('worked', None)
    _save_worked(key, ch)


def candidates(key, exe=None, game=None):
    """Every Proton to try, best first: [{name, path (None: needs a download), why}]. Ones that closed right away
    for this program come last."""
    rules = load_rules()
    mem = remembered(key)
    out, seen = [], set()

    def add(name, path, why):
        if name and name not in seen:
            seen.add(name)
            out.append({'name': name, 'path': path, 'why': why})

    w = mem.get('worked')
    if isinstance(w, str) and find(w):
        add(w, find(w), 'It worked with this last time')
    r = rule_for(exe, game, rules)
    if r:
        want = r.get('proton')
        if want == 'builtin' and builtin():
            add(builtin()[0], builtin()[1], r.get('why') or "Hearth's table")
        elif want == 'valve' and valve():
            add(valve()[0], valve()[1], r.get('why') or "Hearth's table")
        elif isinstance(want, str) and want not in ('builtin', 'valve'):
            add(want, find(want), r.get('why') or "Hearth's table")
    b = builtin()
    if b:
        add(b[0], b[1], 'Built into Hearth OS (works offline)')
    v = valve()
    if v:
        add(v[0], v[1], "Valve's newest Proton, from Steam")
    for n, p in sorted(downloaded().items(), key=lambda t: version_key(t[0]), reverse=True):
        add(n, p, 'Downloaded before')
    for o in rules.get('older_ge', []) if isinstance(rules.get('older_ge'), list) else []:
        if isinstance(o, dict) and isinstance(o.get('name'), str):
            add(o['name'], find(o['name']), 'An older GE-Proton (a download)')
    failed = [n for n in mem.get('failed', []) if isinstance(n, str)]
    return [c for c in out if c['name'] not in failed] + [c for c in out if c['name'] in failed]


def pick(key, exe=None, game=None):
    """The Proton to start with: the first candidate that's on this device, or None."""
    for c in candidates(key, exe, game):
        if c['path']:
            return c
    return None


def next_after(key, current, exe=None, game=None):
    """The candidate to offer after `current` closed right away (it may need a download), or None."""
    for c in candidates(key, exe, game):
        if c['name'] != current:
            return c
    return None


def downloadable(name):
    for o in load_rules().get('older_ge', []) or []:
        if isinstance(o, dict) and o.get('name') == name and re.fullmatch(r'[0-9a-f]{128}', str(o.get('sha512', ''))):
            return o
    return None


def fetch(name, progress=None, opener=None):
    """Download an older GE-Proton from the table (checked against its SHA-512) into user_dir(). -> its folder."""
    o = downloadable(name)
    if not o:
        raise ValueError("%s isn't one Hearth can download" % name)
    if find(name):
        return find(name)
    os.makedirs(user_dir(), exist_ok=True)
    work = tempfile.mkdtemp(prefix='.fetch-', dir=user_dir())
    try:
        tgz = os.path.join(work, name + '.tar.gz')
        h = hashlib.sha512()
        with (opener or urllib.request.urlopen)(GE_URL.format(name)) as r, open(tgz, 'wb') as f:
            total = int(r.headers.get('Content-Length') or 0) if hasattr(r, 'headers') else 0
            got = 0
            while True:
                b = r.read(1 << 20)
                if not b:
                    break
                f.write(b)
                h.update(b)
                got += len(b)
                if progress and total:
                    progress(min(99, got * 100 // total), 'Downloading %s' % name)
        if h.hexdigest() != o['sha512']:
            raise ValueError("The download of %s didn't match its checksum, so it wasn't used" % name)
        if progress:
            progress(99, 'Unpacking %s' % name)
        with tarfile.open(tgz) as t:
            for m in t.getmembers():
                p = os.path.normpath(m.name)
                if p.startswith(('/', '..')) or not (p == name or p.startswith(name + '/')) or m.isdev():
                    raise ValueError("%s's download has files outside its folder" % name)
            t.extractall(work)
        dest = os.path.join(user_dir(), name)
        if not usable(os.path.join(work, name)):
            raise ValueError("%s's download has no proton script" % name)
        os.replace(os.path.join(work, name), dest)
        if progress:
            progress(100, '%s is ready' % name)
        return dest
    finally:
        shutil.rmtree(work, ignore_errors=True)
