"""Sort my Downloads in Mantel's Hearth Files (FileManager.qml, FilesSortDownloads.qml), through hearth-files.

The same rules as the Rust Hearth Files (hosts/hearth-ui/apps/files/src/sortdl.rs; mockups
docs/mockups/1.17-sort-downloads, checklist docs/mockups/checklists/sort-downloads.md): what's loose at the top of
Downloads goes into five folders, Hearth, Installers, Media, Documents and _to_delete (copies of a download that are
the same size as it, "name (1).ext", for you to empty yourself). It only moves: never deletes, never overwrites (a name
that's already there is left where it is), and one Undo (the note's button, or Ctrl+Z) puts everything back.

What the rules don't know (Documents/Unsorted) is asked of Sort my Downloads' AI while the sheet is open
(`hearth-layout sort-ask`, names only, nothing in the files); an answer outside the places it may give is dropped and
the rules' place stays.

  hearth-files sort-state DIR      {downloads, here, loose, folders: {name: items inside}}
  hearth-files sort-plan DIR       {dir, moves: [{from, to, group}], unknown: [names]}
  hearth-files sort-ai PLAN        the plan with the AI's places for the unknown names (the plan as it was without one)
  hearth-files sort-apply PLAN     {done: [[from, to]], skipped, folders}
  hearth-files sort-undo MOVES     {moved} (put back; MOVES is done's list)  · sort-redo MOVES: again
"""
import json
import os
import shutil
import subprocess

TOP = ('Hearth', 'Installers', 'Media', 'Documents', '_to_delete')
TITLES = ('Hearth', 'Installers', 'Media', 'Documents', 'To delete')
# Each group's colour (the mockup's; sortdl.rs COLOURS).
COLOURS = ('#e9a43d', '#3d7ab8', '#7a5ab0', '#3f8f6a', '#b0563a')
# The places Sort my Downloads' AI may give (hearth-layout describe.rs SORT_PLACES) and their groups.
AI_PLACES = {'Installers': 1, 'Installers/Archives': 1, 'Media/Images': 2, 'Media/Videos': 2, 'Media/Sound': 2,
             'Documents': 3, 'Documents/Scripts': 3, 'Documents/Unsorted': 3}
UNSORTED = 'Documents/Unsorted'
COMMANDS = ('sort-state', 'sort-plan', 'sort-ai', 'sort-apply', 'sort-undo', 'sort-redo')


def _home():
    return os.environ.get('HOME') or '/'


def downloads(hf=None):
    """Your Downloads folder (user-dirs.dirs' XDG_DOWNLOAD_DIR, unless it's your home folder; else ~/Downloads)."""
    h = _home()
    d = ''
    if hf is not None:
        d = hf.user_dirs().get('DOWNLOAD', '')
    else:
        cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(h, '.config')
        try:
            with open(os.path.join(cfg, 'user-dirs.dirs'), encoding='utf-8') as f:
                for line in f:
                    line = line.strip()
                    if line.startswith('XDG_DOWNLOAD_DIR='):
                        d = line.split('=', 1)[1].strip().strip('"').replace('$HOME', h)
        except OSError:
            pass
    if d and d.startswith('/') and d.rstrip('/') != h.rstrip('/'):
        return d.rstrip('/')
    return os.path.join(h, 'Downloads')


def is_here(path, hf=None):
    return bool(path) and os.path.normpath(path) == os.path.normpath(downloads(hf))


def original_of(name):
    """"name (1).ext" / "name(2).ext" → "name.ext" (a browser's second copy of the same download); else None."""
    i = name.rfind('.')
    stem, ext = (name[:i], name[i:]) if i > 0 else (name, '')
    stem = stem.rstrip()
    if not stem.endswith(')'):
        return None
    o = stem.rfind('(')
    if o < 0:
        return None
    n = stem[o + 1:-1]
    if not n or not all('0' <= c <= '9' for c in n):
        return None
    b = stem[:o].rstrip()
    if not b:
        return None
    return b + ext


def place(d, name, is_dir):
    """Where one thing goes: (the folder under Downloads, its group 0…4)."""
    lo = name.lower()

    def ext(xs):
        return any(lo.endswith(x) for x in xs)

    orig = original_of(name)
    if orig is not None:
        try:
            a, b = os.stat(os.path.join(d, name)), os.stat(os.path.join(d, orig))
            import stat as _st
            if _st.S_ISREG(a.st_mode) and _st.S_ISREG(b.st_mode) and a.st_size == b.st_size:
                return '_to_delete/Old copies', 4
        except OSError:
            pass
    if lo.startswith('hearth-ai-command-center') or 'command-center' in lo:
        return 'Hearth/AI Command Center', 0
    if lo.startswith('hearth-console-remote-play') or lo.startswith('console-remote-play'):
        return 'Hearth/Console Remote Play', 0
    if lo.startswith('capture-card') or lo.startswith('hearth-capture-card'):
        return 'Hearth/Capture Card', 0
    if lo.startswith('hearth-os') or lo.startswith('hearth-recovery') or (lo.startswith('hearth') and lo.endswith('.iso')):
        return 'Hearth/Hearth OS', 0
    if lo.startswith('hearth') or lo.startswith('nimbus') or lo.startswith('ithil'):
        return 'Hearth/Other', 0
    if is_dir:
        return 'Documents/Folders', 3
    if ext(('.iso', '.img')):
        return 'Installers/ISOs', 1
    if ext(('.appimage', '.deb', '.rpm', '.flatpak', '.flatpakref', '.exe', '.msi', '.run')) or 'installer' in lo:
        return 'Installers', 1
    if ext(('.zip', '.7z', '.rar', '.tar', '.tgz', '.tar.gz', '.tar.xz', '.tar.zst', '.gz', '.xz')):
        return 'Installers/Archives', 1
    if ext(('.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg', '.heic', '.bmp', '.avif')):
        return 'Media/Images', 2
    if ext(('.mp4', '.mkv', '.mov', '.webm', '.avi')):
        return 'Media/Videos', 2
    if ext(('.mp3', '.wav', '.flac', '.ogg', '.m4a', '.opus')):
        return 'Media/Sound', 2
    if ext(('.sh', '.py')):
        return 'Documents/Scripts', 3
    if ext(('.pdf', '.txt', '.md', '.doc', '.docx', '.odt', '.csv', '.xlsx', '.ods', '.json', '.html', '.log')):
        return 'Documents', 3
    return UNSORTED, 3


def _loose_names(d):
    try:
        it = list(os.scandir(d))
    except OSError:
        return []
    out = []
    for e in it:
        if e.name.startswith('.') or e.name in TOP:
            continue
        try:
            isdir = e.is_dir(follow_symlinks=False)
        except OSError:
            isdir = False
        out.append((e.name, isdir))
    out.sort()
    return out


def _items_in(p, cap=100000):
    """How many things are inside one of the five folders (all the way down; hidden ones aren't counted)."""
    n = 0
    for root, dirs, files in os.walk(p):
        dirs[:] = [x for x in dirs if not x.startswith('.')]
        n += sum(1 for f in files if not f.startswith('.'))
        if n >= cap:
            break
    return n


def state(d, hf=None):
    """Downloads as Files shows it: is this it, what's loose, and the five folders with what's in each."""
    here = is_here(d, hf)
    out = {'downloads': downloads(hf), 'here': here, 'loose': 0, 'folders': {}}
    if not here:
        return out
    out['loose'] = len(_loose_names(d))
    for t in TOP:
        p = os.path.join(d, t)
        if os.path.isdir(p) and not os.path.islink(p):
            out['folders'][t] = _items_in(p)
    return out


def plan(d):
    """What would move, from what's in the folder now (read again, not from the window's list)."""
    d = os.path.normpath(d)
    moves, unknown = [], []
    for name, isdir in _loose_names(d):
        to, g = place(d, name, isdir)
        moves.append({'from': os.path.join(d, name), 'to': os.path.join(d, to), 'group': g})
        if to == UNSORTED:
            unknown.append(name)
    return {'dir': d, 'moves': moves, 'unknown': unknown, 'counts': [sum(1 for m in moves if m['group'] == g) for g in range(5)]}


def _ok_move(m):
    """A move this may make: something at the top of a folder into one of the five folders under that same folder."""
    try:
        f, to = os.path.normpath(str(m['from'])), os.path.normpath(str(m['to']))
    except (KeyError, TypeError):
        return None
    d = os.path.dirname(f)
    rel = os.path.relpath(to, d)
    if rel.startswith('..') or rel == '.' or rel.split(os.sep)[0] not in TOP or os.path.basename(f) in TOP:
        return None
    if not os.path.isabs(f) or os.path.basename(f).startswith('.'):
        return None
    return f, to


def ai(p, ask=None):
    """The plan, with Sort my Downloads' AI's places for what the rules leave in Documents/Unsorted."""
    names = [n for n in (p.get('unknown') or []) if isinstance(n, str)][:40]
    if not names:
        return p
    if ask is None:
        def ask(ns):
            exe = shutil.which('hearth-layout')
            if not exe:
                return {}
            try:
                r = subprocess.run([exe, 'sort-ask', json.dumps({'names': ns})], capture_output=True, text=True, timeout=90)
                return json.loads((r.stdout or '').strip().splitlines()[-1] or '{}')
            except (OSError, ValueError, IndexError, subprocess.TimeoutExpired):
                return {}
    got = ask(names) or {}
    places = got.get('places') if isinstance(got, dict) else None
    if not isinstance(places, dict):
        p = dict(p)
        p['ai'] = {'ok': False, 'error': (got.get('error') if isinstance(got, dict) else '') or 'no answer'}
        return p
    out = dict(p)
    moves = []
    asked = 0
    for m in p.get('moves') or []:
        m = dict(m)
        if m.get('to', '').endswith('/' + UNSORTED):
            name = os.path.basename(m.get('from', ''))
            pl = places.get(name)
            if isinstance(pl, str) and pl in AI_PLACES:
                m['to'] = os.path.join(os.path.dirname(m['from']), pl)
                m['group'] = AI_PLACES[pl]
                m['ai'] = True
                asked += 1
        moves.append(m)
    out['moves'] = moves
    out['counts'] = [sum(1 for m in moves if m.get('group') == g) for g in range(5)]
    out['ai'] = {'ok': True, 'placed': asked}
    return out


def apply(p):
    """Do it: each thing into its folder, only when nothing of that name is there. (from, to) of what moved."""
    done, skipped = [], 0
    for m in p.get('moves') or []:
        ok = _ok_move(m)
        if not ok:
            skipped += 1
            continue
        f, d = ok
        to = os.path.join(d, os.path.basename(f))
        if not os.path.lexists(f):
            continue
        if os.path.lexists(to):
            skipped += 1
            continue
        try:
            os.makedirs(d, exist_ok=True)
        except OSError:
            skipped += 1
            continue
        if os.path.lexists(to):
            skipped += 1
            continue
        try:
            # rename(2) never replaces a folder that's there unless it's empty; a file only when nothing was there a
            # moment ago (checked just above). Same folder tree: never a copy.
            os.rename(f, to)
            done.append([f, to])
        except OSError:
            skipped += 1
    base = os.path.normpath(p.get('dir') or (os.path.dirname(done[0][0]) if done else ''))
    folders = sum(1 for t in TOP if base and os.path.isdir(os.path.join(base, t)))
    return {'done': done, 'skipped': skipped, 'folders': folders}


def swap(moves, back):
    """Back (or again, for Redo): only when nothing of that name is where it goes."""
    n = 0
    for mv in moves or []:
        if not isinstance(mv, (list, tuple)) or len(mv) != 2:
            continue
        a, b = (mv[1], mv[0]) if back else (mv[0], mv[1])
        if os.path.basename(str(mv[0])) != os.path.basename(str(mv[1])) or not _ok_move({'from': mv[0], 'to': os.path.dirname(str(mv[1]))}):
            continue
        if os.path.lexists(a) and not os.path.lexists(b):
            try:
                os.makedirs(os.path.dirname(b), exist_ok=True)
                os.rename(a, b)
                n += 1
            except OSError:
                pass
    return {'moved': n}


def _json_arg(s):
    try:
        return json.loads(s)
    except ValueError:
        raise ValueError("Sort my Downloads couldn't read what it was given")


def run(hf, c, rest):
    one = len(rest) == 1
    if c == 'sort-state' and one:
        return state(rest[0], hf)
    if c == 'sort-plan' and one:
        if not is_here(rest[0], hf):
            return {'dir': rest[0], 'moves': [], 'unknown': [], 'counts': [0] * 5, 'error': 'Sort my Downloads is for your Downloads folder'}
        return plan(rest[0])
    if c == 'sort-ai' and one:
        return ai(_json_arg(rest[0]))
    if c == 'sort-apply' and one:
        p = _json_arg(rest[0])
        if not is_here(p.get('dir', ''), hf):
            return {'done': [], 'skipped': 0, 'folders': 0, 'error': 'Sort my Downloads is for your Downloads folder'}
        return apply(p)
    if c == 'sort-undo' and one:
        return swap(_json_arg(rest[0]), True)
    if c == 'sort-redo' and one:
        return swap(_json_arg(rest[0]), False)
    raise ValueError('usage: hearth-files sort-state DIR | sort-plan DIR | sort-ai PLAN | sort-apply PLAN | sort-undo MOVES | sort-redo MOVES')
