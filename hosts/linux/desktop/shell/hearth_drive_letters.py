#!/usr/bin/python3
"""hearth_drive_letters: drive letters that stay the same, for Hearth Files' This PC and for Windows apps
(docs/HEARTH-FILES.md "Drive letters").

  * C: is always the drive Hearth runs from. It is never stored and can't be given to anything else.
  * Every other drive (by its file system UUID, or where it's mounted when it has none) and every remembered network
    location (by its address) gets the next free letter, D: to Y:, the first time it's seen. Z: is left alone: it's
    how Windows apps see the whole Linux system.
  * Once a drive has a letter it keeps it: unplugged drives keep theirs, and a letter never moves to another drive by
    itself. Only "Change drive letter…" (set_letter) changes one, and that choice is kept.
  * The letters are state, kept in ~/.config/hearth/drive-letters.json through hearth_settings_io (atomic, with a
    .bak, under its lock). The file is only written when a drive is seen for the first time, when a drive's mount
    place changes, or when you change a letter: never just because Files was opened, and never with defaults. A file
    that can't be read is put back from its .bak, or kept aside as .broken-<time> (never written over).

Windows apps (hearth-winapps): map_prefix(PREFIX) points each lettered drive's dosdevices link (d:, e: …) in a Wine
prefix at the same drive and the prefix's Documents, Desktop, Downloads, Pictures, Music and Videos at your own
folders, touching only links Hearth made (remembered in PREFIX/.hearth-drives.json) and empty folders.

Hearth (C:) with Windows folder names (docs/HEARTH-FILES.md "Hearth (C:) the Windows way"): a view, nothing moves on
disk. from_c / to_c are the same mapping as drive-paths.js's (C:\\Program Files\\<app>, C:\\ProgramData = /var/lib,
C:\\Windows = /usr, C:\\Users = the folder home folders are in, …\\AppData\\Roaming = ~/.config, \\Local = ~/.local/share,
\\Local\\Temp = ~/.cache), for hearth-files and the tests. view_refusal says why nothing can be put into the view's own
folders (hearth:c paths) or into Windows and ProgramData (look only). Windows apps never see it: inside a Wine prefix
C: stays the prefix's own drive (map_prefix never touches c:).

  hearth_drive_letters.py show                 the letters, as JSON
  hearth_drive_letters.py map-prefix PREFIX    what map_prefix would change, then changes it (JSON)
  hearth_drive_letters.py c-path TEXT          what a C:\\ form stands for with the view on (JSON: {path} or {error})
"""
import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
try:
    import hearth_settings_io as hsio
except ImportError:
    hsio = None

SYSTEM = 'C'
LETTERS = 'DEFGHIJKLMNOPQRSTUVWXY'
SYSTEM_KEY = 'system'
USER_FOLDERS = (('DESKTOP', 'Desktop'), ('DOCUMENTS', 'Documents'), ('DOWNLOAD', 'Downloads'),
                ('PICTURES', 'Pictures'), ('MUSIC', 'Music'), ('VIDEOS', 'Videos'))


class Fail(Exception):
    pass


def home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def state_file():
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(home(), '.config')
    return os.path.join(cfg, 'hearth', 'drive-letters.json')


def _valid(raw):
    try:
        return isinstance(json.loads(raw.decode('utf-8')), dict)
    except (ValueError, UnicodeDecodeError):
        return False


def _drives(data):
    d = data.get('drives') if isinstance(data, dict) else None
    out = {}
    if isinstance(d, dict):
        for k, v in d.items():
            if isinstance(k, str) and isinstance(v, dict) and isinstance(v.get('letter'), str) \
                    and len(v['letter']) == 1 and v['letter'].upper() in LETTERS:
                out[k] = dict(v, letter=v['letter'].upper())
    return out


def load(path=None):
    """{key: {letter, name, path, kind, user?}} from the state file. Missing: {}. Damaged: put back from .bak; with no
    good .bak the damaged file is kept aside (.broken-<time>, by hearth_settings_io) and moved out of the way, so it's
    kept once, never written over, and the letters start again from what's seen next."""
    path = path or state_file()
    try:
        with open(path, 'rb') as f:
            raw = f.read()
    except FileNotFoundError:
        return {}
    except OSError:
        return {}
    if _valid(raw):
        return _drives(json.loads(raw.decode('utf-8')))
    data = hsio.load_json(path, {}) if hsio else {}
    try:
        with open(path, 'rb') as f:
            now = f.read()
    except OSError:
        now = None
    if now is not None and _valid(now):          # put back from .bak
        return _drives(json.loads(now.decode('utf-8')))
    if now is not None:
        # hearth_settings_io kept a copy (.broken-<time>); without it, keep one here. Then out of the way.
        kept = [n for n in os.listdir(os.path.dirname(path) or '.') if n.startswith(os.path.basename(path) + '.broken-')]
        try:
            if not kept:
                os.replace(path, '%s.broken-%s' % (path, time.strftime('%Y%m%d-%H%M%S')))
            else:
                os.remove(path)
        except OSError:
            pass
        if hsio:
            hsio.log_event('drive letters: %s could not be read; kept aside, letters start again' % path)
    return _drives(data)


def _write(change, path):
    """CHANGE(drives dict) under the lock, written only when it changed."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if hsio:
        def apply(d):
            drives = _drives(d)
            change(drives)
            d['drives'] = drives
            d.setdefault('version', 1)
        return _drives(hsio.update_json(path, apply))
    cur = load(path)
    change(cur)
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        json.dump({'version': 1, 'drives': cur}, f, indent=2)
    os.replace(tmp, path)
    return cur


def used(drives, but=None):
    return {v['letter'] for k, v in drives.items() if k != but}


def next_free(drives):
    taken = used(drives)
    return next((l for l in LETTERS if l not in taken), None)


def assign(seen, path=None):
    """SEEN: [{key, name, path, kind}] in the order to hand out letters. Returns {key: letter} for them (None when
    every letter is taken). Writes only for a drive seen the first time or one whose mount place changed."""
    path = path or state_file()
    drives = load(path)
    seen = [s for s in seen if s.get('key') and s['key'] != SYSTEM_KEY]
    todo = [s for s in seen if s['key'] not in drives
            or (s.get('path') and drives[s['key']].get('path') != s.get('path'))]
    if todo and (next_free(drives) or any(s['key'] in drives for s in todo)):
        def change(d):
            for s in todo:
                if s['key'] in d:
                    if s.get('path'):
                        d[s['key']]['path'] = s['path']
                    continue
                l = next_free(d)
                if not l:
                    continue
                d[s['key']] = {'letter': l, 'name': s.get('name') or '', 'path': s.get('path') or '',
                               'kind': s.get('kind') or '', 'since': int(time.time())}
        try:
            drives = _write(change, path)
        except OSError:
            pass                                   # a read-only home: the letters just aren't kept this time
    return {s['key']: (drives.get(s['key']) or {}).get('letter') for s in seen}


def set_letter(key, letter, path=None):
    """Your choice ("Change drive letter…"): KEY gets LETTER, if it's free. Kept with user: true."""
    path = path or state_file()
    letter = (letter or '').strip().rstrip(':').upper()
    if key == SYSTEM_KEY or letter == SYSTEM:
        raise Fail('C: is always Hearth, the drive Hearth runs from')
    if letter not in LETTERS:
        raise Fail('Pick a letter from D: to Y: (Z: is how Windows apps see the whole system)')
    if not isinstance(key, str) or not key or len(key) > 600:
        raise Fail('Not a drive')
    drives = load(path)
    for k, v in drives.items():
        if k != key and v['letter'] == letter:
            raise Fail('%s: is already used by %s' % (letter, v.get('name') or 'another drive'))

    def change(d):
        for k, v in d.items():
            if k != key and v['letter'] == letter:
                raise Fail('%s: is already used by %s' % (letter, v.get('name') or 'another drive'))
        cur = d.get(key) or {'name': '', 'path': '', 'kind': '', 'since': int(time.time())}
        cur.update({'letter': letter, 'user': True})
        d[key] = cur
    return _write(change, path)[key]


# ------------------------------------------------------------------ Windows apps: the same letters in a prefix

def user_dirs():
    h = home()
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(h, '.config')
    out = {}
    try:
        with open(os.path.join(cfg, 'user-dirs.dirs'), encoding='utf-8') as f:
            for line in f:
                m = re.match(r'\s*XDG_(\w+)_DIR\s*=\s*"(.*)"\s*$', line)
                if m:
                    v = m.group(2).replace('$HOME', h)
                    out[m.group(1)] = os.path.normpath(v if v.startswith('/') else os.path.join(h, v))
    except OSError:
        pass
    return out


def _empty_dir(p):
    try:
        return os.path.isdir(p) and not os.path.islink(p) and not os.listdir(p)
    except OSError:
        return False


def map_prefix(prefix, letters_path=None, dry=False):
    """Points PREFIX's dosdevices d: … y: at the lettered drives and its users' Documents, Desktop… at your own
    folders. Only links Hearth made before (PREFIX/.hearth-drives.json) are changed or removed; a letter Wine or you
    set up is left as it is; a user folder is only linked when it's missing, an empty folder, or already Hearth's.
    C: and Z: are never touched. Returns {"drives": {letter: target}, "folders": {...}, "kept": [...]}."""
    prefix = os.path.abspath(prefix)
    dos = os.path.join(prefix, 'dosdevices')
    marker = os.path.join(prefix, '.hearth-drives.json')
    try:
        with open(marker, encoding='utf-8') as f:
            made = json.load(f)
        if not isinstance(made, dict):
            made = {}
    except (OSError, ValueError):
        made = {}
    made_links = made.get('links') if isinstance(made.get('links'), dict) else {}
    made_folders = made.get('folders') if isinstance(made.get('folders'), dict) else {}
    want = {}
    for key, v in load(letters_path).items():
        if v.get('path') and v['path'].startswith('/') and v['letter'] in LETTERS:
            want[v['letter'].lower() + ':'] = v['path']
    out = {'drives': {}, 'folders': {}, 'kept': []}
    links = dict(made_links)
    # Only a prefix Proton has already made (it makes dosdevices, c: and z: on the first start): a new app gets its
    # letters at its next start.
    if os.path.isdir(dos):
        # Letters Hearth made that no drive has now (you changed the letter): taken away.
        for name, target in list(made_links.items()):
            p = os.path.join(dos, name)
            if name in want:
                continue
            if os.path.islink(p) and os.readlink(p) == target and not dry:
                os.remove(p)
            links.pop(name, None)
            out['drives'][name] = None
        for name, target in sorted(want.items()):
            p = os.path.join(dos, name)
            if os.path.lexists(p):
                mine = os.path.islink(p) and made_links.get(name) == os.readlink(p)
                if not mine:
                    if not (os.path.islink(p) and os.readlink(p) == target):
                        out['kept'].append(name)        # Wine's or yours: left alone
                    continue
                if os.readlink(p) == target:
                    links[name] = target
                    out['drives'][name] = target
                    continue
                if not dry:
                    os.remove(p)
            if not dry:
                os.symlink(target, p)
            links[name] = target
            out['drives'][name] = target
    # Documents, Desktop, Downloads, Pictures, Music, Videos of each user in the prefix (Proton's steamuser too).
    xdg = user_dirs()
    folders = dict(made_folders)
    users = os.path.join(prefix, 'drive_c', 'users')
    try:
        names = [n for n in os.listdir(users) if n.lower() != 'public' and os.path.isdir(os.path.join(users, n))]
    except OSError:
        names = []
    for u in names:
        for key, win in USER_FOLDERS:
            real = xdg.get(key) or os.path.join(home(), win)
            if not os.path.isdir(real) or os.path.realpath(real) == os.path.realpath(home()):
                continue
            p = os.path.join(users, u, win)
            rel = u + '/' + win
            if os.path.islink(p):
                if os.readlink(p) == real:
                    folders[rel] = real
                    out['folders'][rel] = real
                elif made_folders.get(rel) == os.readlink(p):
                    if not dry:
                        os.remove(p)
                        os.symlink(real, p)
                    folders[rel] = real
                    out['folders'][rel] = real
                continue                                # someone else's link (Wine's own to ~): left alone
            if os.path.lexists(p) and not _empty_dir(p):
                out['kept'].append(rel)                 # has files: nothing is moved
                continue
            if not dry:
                if os.path.lexists(p):
                    os.rmdir(p)
                os.makedirs(os.path.dirname(p), exist_ok=True)
                os.symlink(real, p)
            folders[rel] = real
            out['folders'][rel] = real
    if not dry and (links != made_links or folders != made_folders):
        data = {'links': links, 'folders': folders}
        if hsio:
            hsio.write_json(marker, data)
        else:
            with open(marker + '.tmp', 'w', encoding='utf-8') as f:
                json.dump(data, f)
            os.replace(marker + '.tmp', marker)
    return out


# ---------------------------------------------------------------- Hearth (C:) with Windows folder names
C_ROOT = 'hearth:c'


def c_context(apps=None, home_dir=None, user_name=None):
    """{home, user, users, apps: [{name, path, x86}]} for from_c / to_c."""
    h = (home_dir or home()).rstrip('/') or '/'
    u = user_name or os.path.basename(h) or 'you'
    return {'home': h, 'user': u, 'users': os.path.dirname(h) or '/', 'apps': list(apps or [])}


def _join(root, segs):
    root = root.rstrip('/')
    rest = '/'.join(s for s in segs if s)
    return (root + '/' + rest if rest else root) or '/'


def is_c_path(p):
    return isinstance(p, str) and p.startswith(C_ROOT)


def c_page(p, ctx):
    """'root', 'pf', 'x86', 'appdata', or '' (the view's own pages, like drive-paths.js cPage)."""
    if not is_c_path(p):
        return ''
    segs = [s.lower() for s in p[len(C_ROOT):].split('/') if s]
    if not segs:
        return 'root'
    if segs == ['program files']:
        return 'pf'
    if segs == ['program files (x86)']:
        return 'x86'
    if len(segs) == 3 and segs[0] == 'users' and segs[2] == 'appdata' and segs[1] == ctx['user'].lower():
        return 'appdata'
    return ''


def _c_segs(segs, ctx):
    if not segs:
        return {'path': C_ROOT}
    low = [s.lower() for s in segs]
    h = ctx['home']
    if low[0] in ('program files', 'program files (x86)'):
        x86 = low[0] == 'program files (x86)'
        name = 'Program Files (x86)' if x86 else 'Program Files'
        if len(segs) == 1:
            return {'path': C_ROOT + '/' + name}
        for a in ctx.get('apps') or []:
            if bool(a.get('x86')) == x86 and str(a.get('name', '')).lower() == low[1] and a.get('path'):
                return {'path': _join(a['path'], segs[2:])}
        return {'error': 'There\'s no “%s” in %s' % (segs[1], name)}
    if low[0] == 'programdata':
        return {'path': _join('/var/lib', segs[1:])}
    if low[0] == 'windows':
        return {'path': _join('/usr', segs[1:])}
    if low[0] == 'users':
        if len(segs) == 1:
            return {'path': ctx['users']}
        if low[1] == ctx['user'].lower():
            r, rl = segs[2:], low[2:]
            if rl[:1] == ['appdata']:
                if len(r) == 1:
                    return {'path': C_ROOT + '/Users/' + ctx['user'] + '/AppData'}
                if rl[1] == 'roaming':
                    return {'path': _join(h + '/.config', r[2:])}
                if rl[1] == 'local' and rl[2:3] == ['temp']:
                    return {'path': _join(h + '/.cache', r[3:])}
                if rl[1] == 'local':
                    return {'path': _join(h + '/.local/share', r[2:])}
                return {'error': 'AppData has Roaming and Local (Local\\Temp too)'}
            return {'path': _join(h, r)}
        return {'path': _join(ctx['users'], segs[1:])}
    return {'path': '/' + '/'.join(segs)}


def from_c(text, ctx):
    """'C:\\Program Files\\Firefox' → {'path': real path or a hearth:c page}, {'error': …}; None if it isn't a C: path."""
    m = re.match(r'^\s*[Cc]:([\\/].*)?\s*$', text or '')
    if not m:
        return None
    segs = []
    for s in re.split(r'[\\/]+', m.group(1) or ''):
        if not s or s == '.':
            continue
        if s == '..':
            if segs:
                segs.pop()
            continue
        segs.append(s)
    return _c_segs(segs, ctx)


def to_c(path, ctx):
    """The C:\\ form of a real path (or a hearth:c page) with the view on."""
    if is_c_path(path):
        return 'C:\\' + '\\'.join(s for s in path[len(C_ROOT):].split('/') if s)
    h, u = ctx['home'], ctx['user']
    maps = [(a['path'].rstrip('/'), 'C:\\%s\\%s' % ('Program Files (x86)' if a.get('x86') else 'Program Files', a['name']))
            for a in ctx.get('apps') or [] if a.get('path', '').startswith('/') and a.get('name') and not a.get('noMap')]
    maps += [(h + '/.config', 'C:\\Users\\%s\\AppData\\Roaming' % u), (h + '/.local/share', 'C:\\Users\\%s\\AppData\\Local' % u),
             (h + '/.cache', 'C:\\Users\\%s\\AppData\\Local\\Temp' % u), (h, 'C:\\Users\\' + u),
             (ctx['users'], 'C:\\Users'), ('/var/lib', 'C:\\ProgramData'), ('/usr', 'C:\\Windows')]
    maps.sort(key=lambda x: -len(x[0]))
    for root, win in maps:
        if path == root or path.startswith(root + '/'):
            rest = '\\'.join(s for s in path[len(root):].split('/') if s)
            return win + ('\\' + rest if rest else '')
    return 'C:\\' + '\\'.join(s for s in path.split('/') if s)


def view_refusal(path, ctx=None):
    """Why nothing can be dropped or pasted here, or '' (real folders reached through the view work as always)."""
    if is_c_path(path):
        pg = c_page(path, ctx or c_context())
        if pg in ('pf', 'x86'):
            return ('You can\'t put files straight into %s. To add an app, install it, and it shows up here by itself.'
                    % ('Program Files (x86)' if pg == 'x86' else 'Program Files'))
        if pg == 'appdata':
            return 'You can\'t put files straight into AppData: open Roaming or Local first.'
        if pg == 'root':
            return 'You can\'t put files straight into Hearth (C:). Open a folder in it first (your own folder is in Users).'
        return 'That\'s a name for another folder: open it first.'
    p = os.path.abspath(path) if isinstance(path, str) and path else ''
    if p == '/usr' or p.startswith('/usr/'):
        return 'Windows (C:\\Windows) is Hearth itself: look only. Each update replaces it all at once.'
    if p == '/var/lib' or p.startswith('/var/lib/'):
        return 'ProgramData (C:\\ProgramData) is look only: what apps keep there is changed by the apps themselves.'
    return ''


def main(argv):
    a = argv[1:]
    try:
        if a == ['show']:
            r = {'letters': load(), 'file': state_file()}
        elif a[:1] == ['map-prefix'] and len(a) == 2:
            r = map_prefix(a[1])
        elif a[:1] == ['c-path'] and len(a) == 2:
            r = from_c(a[1], c_context()) or {'error': 'That isn\'t a C:\\ path'}
        else:
            print(__doc__.strip(), file=sys.stderr)
            return 2
    except (Fail, OSError, ValueError) as e:
        print('error: %s' % e, flush=True)
        return 1
    print(json.dumps(r), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
