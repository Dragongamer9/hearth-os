"""hearth_files_extras: Hearth Files' 1.14 extras (Dolphin's features), run through hearth-files (it hands itself over
as `hf`, so the same rules apply: what may change, plain-words errors, the JSON line or "error: ..."):

  tags PATH...                          {"tags": {path: [tag, ...]}}: the tags a file carries (the user.xdg.tags extended
                                        attribute, what Dolphin and KDE's file index use, so tags made there show here)
  tag-set [--add TAG]... [--remove TAG]... PATH...    add or take away tags (only where you may change files)
  tag-list                              every tag in your home folder with how many items carry it (looked up at most
                                        once a minute: ~/.cache/hearth/files-tags.json is only a cache)
  tag-find TAG                          "found: {entry}" lines for what carries TAG, then {"ok", "count"}
  git-status FOLDER                     {"repo", "branch", "ahead", "status": {name: modified|added|untracked|deleted|
                                        conflict|ignored|clean}} for what's in FOLDER, when it's in a git repository
  iso-mount FILE / iso-unmount FILE|FOLDER   open a disc image (.iso, .img) as a drive, read only (udisks), and close it
  portable                              phones (MTP) and cameras (PTP) plugged in, through gvfs: [{label, uri, kind, path}]
  portable-mount URI / portable-unmount PATH  open one (it shows up under This PC and the sidebar) / let it go
  service-menus PATH...                 right-click actions apps add (KDE service menus: kio/servicemenus/*.desktop, and
                                        the older kservices5/ServiceMenus): [{id, label, icon, submenu}]
  service-run ID PATH...                run one of them on these files
  sharing FOLDER                        is it shared on your network (Settings › Home server's shares, Samba)?
  share FOLDER on|off|write-on|write-off    share it, stop, or let others change files (through hearth-home-server)
  recent-prefs / recent-prefs-set JSON  Settings › Privacy › Recent files: {"remember": true, "keepDays": 0, "leaveOut": []}
                                        (~/.config/nimbus/recent.json through hearth_settings_io)
  recent-forget hour|day|all|PATH|activity:ID   forget recently opened files (the shared recently-used.xbel,
                                        written atomically with a .bak); activity:ID: the ones opened on that activity
  activity-mark ID / activity-tidy      Activities: you're on ID from now (kept so each activity has its own recent
                                        files) / forget what was opened on an activity set to "Don't remember"
  chmod-tree FOLDER MODE                the same permissions for everything inside (folders get MODE; files get MODE
                                        without "run as a program" unless they had it)
  groups / chgrp PATH GROUP             your groups / give a file of yours to one of them

Python standard library only.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import time

TAG_ATTR = 'user.xdg.tags'
TAG_MAX = 40


# ---------------------------------------------------------------------- tags
def read_tags(path):
    try:
        raw = os.getxattr(path, TAG_ATTR, follow_symlinks=False)
    except OSError:
        return []
    return split_tags(raw.decode('utf-8', 'replace'))


def split_tags(text):
    out = []
    for t in text.split(','):
        t = t.strip()
        if t and t not in out:
            out.append(t)
    return out


def clean_tag(t):
    t = (t or '').strip()
    if not t or ',' in t or '\0' in t or '\n' in t or len(t) > TAG_MAX:
        return None
    return t


def tags_of(hf, paths):
    return {'tags': {p: read_tags(p) for p in paths}}


def tag_set(hf, args):
    add, remove, paths, i = [], [], [], 0
    while i < len(args):
        if args[i] in ('--add', '--remove') and i + 1 < len(args):
            t = clean_tag(args[i + 1])
            if t is None:
                raise hf.Fail('A tag is a short name without commas')
            (add if args[i] == '--add' else remove).append(t)
            i += 2
        else:
            paths.append(args[i])
            i += 1
    if not paths:
        raise hf.Fail('Pick something to tag')
    done = {}
    for p in paths:
        p = hf.check_writable(p)
        cur = read_tags(p)
        new = [t for t in cur if t not in remove] + [t for t in add if t not in cur and t not in remove]
        try:
            if new:
                os.setxattr(p, TAG_ATTR, ','.join(new).encode('utf-8'), follow_symlinks=False)
            elif cur:
                os.removexattr(p, TAG_ATTR, follow_symlinks=False)
        except OSError as e:
            if e.errno in (95, 1):          # EOPNOTSUPP (FAT, exFAT, some network folders), EPERM (links)
                raise hf.Fail('Tags can\'t be kept on this drive or this kind of item')
            raise hf.Fail(hf.friendly(e, p))
        done[p] = new
    _cache_clear()
    return {'ok': True, 'tags': done}


def _cache_file():
    base = os.environ.get('XDG_CACHE_HOME') or os.path.join(os.path.expanduser('~'), '.cache')
    return os.path.join(base, 'hearth', 'files-tags.json')


def _cache_clear():
    try:
        os.unlink(_cache_file())
    except OSError:
        pass


def _walk(root, budget=4.0, limit=60000):
    """(path, name) for what's under ROOT: hidden folders and huge trees left out, stopping after a few seconds."""
    end, n, stack = time.time() + budget, 0, [root]
    while stack:
        d = stack.pop()
        try:
            it = os.scandir(d)
        except OSError:
            continue
        with it:
            for e in it:
                if e.name.startswith('.') or e.name in ('node_modules', '__pycache__', 'target'):
                    continue
                n += 1
                yield e.path, e.name
                try:
                    if e.is_dir(follow_symlinks=False):
                        stack.append(e.path)
                except OSError:
                    pass
                if n >= limit or time.time() > end:
                    return


def tag_list(hf):
    f = _cache_file()
    try:
        if time.time() - os.path.getmtime(f) < 60:
            with open(f) as fh:
                return json.load(fh)
    except (OSError, ValueError):
        pass
    counts = {}
    for p, _ in _walk(hf.home()):
        for t in read_tags(p):
            counts[t] = counts.get(t, 0) + 1
    out = {'tags': [{'name': k, 'count': v} for k, v in sorted(counts.items(), key=lambda kv: kv[0].lower())]}
    try:
        os.makedirs(os.path.dirname(f), exist_ok=True)
        tmp = f + '.tmp'
        with open(tmp, 'w') as fh:
            json.dump(out, fh)
        os.replace(tmp, f)
    except OSError:
        pass
    return out


def tag_find(hf, tag):
    t = clean_tag(tag)
    if t is None:
        raise hf.Fail('That isn\'t a tag')
    root, n = hf.home(), 0
    for p, name in _walk(root, budget=8.0, limit=200000):
        if t in read_tags(p):
            e = hf.entry(p, name)
            if e:
                e['where'] = os.path.relpath(os.path.dirname(p), root)
                print('found: ' + json.dumps(e), flush=True)
                n += 1
    return {'ok': True, 'count': n, 'tag': t}


# ---------------------------------------------------------------------- git
GIT_CODES = {'M': 'modified', 'T': 'modified', 'A': 'added', 'D': 'deleted', 'R': 'modified', 'C': 'added', 'U': 'conflict'}
RANK = ['clean', 'ignored', 'added', 'untracked', 'deleted', 'modified', 'conflict']


def repo_of(folder):
    d = os.path.realpath(folder)
    while True:
        if os.path.exists(os.path.join(d, '.git')):
            return d
        up = os.path.dirname(d)
        if up == d:
            return None
        d = up


def parse_porcelain(raw):
    """git status --porcelain=v1 -z: [(code, path)] (a rename's old name is skipped)."""
    out, parts, i = [], raw.split('\0'), 0
    while i < len(parts):
        rec = parts[i]
        i += 1
        if len(rec) < 4:
            continue
        xy, path = rec[:2], rec[3:]
        if xy == '??':
            code = 'untracked'
        elif xy == '!!':
            code = 'ignored'
        elif 'U' in xy or xy in ('AA', 'DD'):
            code = 'conflict'
        else:
            code = GIT_CODES.get(xy[1] if xy[1] != ' ' else xy[0], 'modified')
        if xy[0] in 'RC':
            i += 1
        out.append((code, path))
    return out


def worse(a, b):
    return a if RANK.index(a) >= RANK.index(b) else b


def git_status(hf, folder):
    p = os.path.realpath(folder)
    repo = repo_of(p) if os.path.isdir(p) else None
    if not repo or not shutil.which('git'):
        return {'repo': None, 'status': {}}
    env = dict(os.environ, GIT_OPTIONAL_LOCKS='0', LC_ALL='C')
    run = lambda *a: subprocess.run(['git', '-C', p] + list(a), capture_output=True, text=True, timeout=4, env=env)
    try:
        st = run('status', '--porcelain=v1', '-z', '--branch', '--untracked-files=normal', '--ignored=matching', '--', '.')
        tracked = run('ls-files', '-z', '--', '.')
    except (OSError, subprocess.TimeoutExpired):
        return {'repo': repo, 'status': {}, 'slow': True}
    if st.returncode != 0:
        return {'repo': None, 'status': {}}
    prefix = os.path.relpath(p, repo)
    prefix = '' if prefix == '.' else prefix + '/'
    out, branch, ahead = {}, '', 0
    recs = st.stdout.split('\0')
    if recs and recs[0].startswith('## '):
        head = recs[0][3:]
        branch = head.split('...')[0].replace('No commits yet on ', '')
        m = re.search(r'ahead (\d+)', head)
        ahead = int(m.group(1)) if m else 0
        recs = recs[1:]
    for name in (tracked.stdout.split('\0') if tracked.returncode == 0 else []):
        if name:
            out.setdefault(name.split('/')[0], 'clean')
    for code, path in parse_porcelain('\0'.join(recs)):
        rel = path[len(prefix):] if prefix and path.startswith(prefix) else path
        top = rel.rstrip('/').split('/')[0]
        if top:
            out[top] = worse(code, out.get(top, 'clean'))
    return {'repo': repo, 'branch': branch, 'ahead': ahead, 'status': out}


# ---------------------------------------------------------------------- disc images (udisks)
def udisks(args, timeout=30):
    if not shutil.which('udisksctl'):
        raise FileNotFoundError('udisksctl')
    return subprocess.run(['udisksctl'] + args + ['--no-user-interaction'], capture_output=True, text=True, timeout=timeout)


def loop_of(image):
    """The loop device showing IMAGE, if one does (losetup -j)."""
    try:
        r = subprocess.run(['losetup', '-j', os.path.realpath(image)], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    m = re.match(r'(/dev/loop\d+):', r.stdout.strip())
    return m.group(1) if m else None


def mountpoint_of(dev):
    try:
        with open('/proc/mounts') as f:
            for line in f:
                parts = line.split()
                if len(parts) >= 2 and parts[0] == dev:
                    return re.sub(r'\\([0-7]{3})', lambda m: chr(int(m.group(1), 8)), parts[1])
    except OSError:
        pass
    return None


IMAGE = re.compile(r'\.(iso|img)$', re.I)


def iso_mount(hf, image):
    p = os.path.realpath(image)
    if not os.path.isfile(p) or not IMAGE.search(p):
        raise hf.Fail('Only disc images (.iso, .img) open this way')
    dev = loop_of(p)
    try:
        if not dev:
            r = udisks(['loop-setup', '-r', '-f', p])
            m = re.search(r'as (/dev/loop\d+)', r.stdout)
            if r.returncode != 0 or not m:
                raise hf.Fail('Couldn\'t open it: ' + ((r.stderr or r.stdout).strip().splitlines() or ['the system said no'])[-1])
            dev = m.group(1)
        for _ in range(20):                   # udisks may mount it by itself a moment later
            mp = mountpoint_of(dev)
            if mp:
                return {'ok': True, 'path': mp, 'device': dev}
            time.sleep(0.1)
        r = udisks(['mount', '-b', dev])
        m = re.search(r' at (.+?)\.?$', r.stdout.strip())
        mp = mountpoint_of(dev) or (m.group(1) if m else None)
        if not mp:
            raise hf.Fail('Couldn\'t open it: ' + ((r.stderr or r.stdout).strip().splitlines() or ['nothing to read on it'])[-1])
        return {'ok': True, 'path': mp, 'device': dev}
    except FileNotFoundError:
        raise hf.Fail('Opening disc images needs udisks (it comes with Hearth OS)')
    except subprocess.TimeoutExpired:
        raise hf.Fail('The system took too long to open it')


def iso_unmount(hf, what):
    p = os.path.realpath(what)
    dev = None
    if os.path.isfile(p):
        dev = loop_of(p)
    else:
        try:
            with open('/proc/mounts') as f:
                for line in f:
                    parts = line.split()
                    if len(parts) >= 2 and parts[0].startswith('/dev/loop') and re.sub(r'\\([0-7]{3})', lambda m: chr(int(m.group(1), 8)), parts[1]) == p:
                        dev = parts[0]
        except OSError:
            pass
    if not dev:
        return {'ok': True, 'closed': False}
    try:
        if mountpoint_of(dev):
            r = udisks(['unmount', '-b', dev])
            if r.returncode != 0:
                raise hf.Fail('It\'s still in use: close what has files open on it, then try again')
        udisks(['loop-delete', '-b', dev])
    except FileNotFoundError:
        raise hf.Fail('Closing disc images needs udisks')
    return {'ok': True, 'closed': True, 'device': dev}


# ---------------------------------------------------------------------- phones (MTP) and cameras (PTP), through gvfs
def parse_gio_mounts(text):
    """`gio mount -li`: the volumes and mounts whose activation root or root is mtp:// or gphoto2://."""
    out, cur = [], None
    for line in text.splitlines():
        s = line.strip()
        m = re.match(r'^(Volume|Mount)\(\d+\):\s*(.*?)(?:\s*->\s*(\S+))?$', s)
        if m:
            cur = {'what': m.group(1).lower(), 'label': m.group(2).strip(), 'uri': m.group(3) or ''}
            out.append(cur)
            continue
        if cur is None:
            continue
        m = re.match(r'^activation_root=(\S+)', s)
        if m:
            cur['uri'] = m.group(1)
    seen, devs = set(), []
    for v in out:
        uri = v.get('uri', '')
        if not re.match(r'^(mtp|gphoto2)://', uri):
            continue
        key = uri.rstrip('/')
        if key in seen:
            if v['what'] == 'mount':
                next(d for d in devs if d['uri'] == key)['mounted'] = True
            continue
        seen.add(key)
        devs.append({'label': v['label'] or ('Phone' if uri.startswith('mtp') else 'Camera'), 'uri': key,
                     'kind': 'phone' if uri.startswith('mtp') else 'camera', 'mounted': v['what'] == 'mount'})
    return devs


def gvfs_path(hf, uri):
    """Where gvfs shows a mounted mtp:// or gphoto2:// device (mtp:host=…, gphoto2:host=…)."""
    m = re.match(r'^(mtp|gphoto2)://([^/]+)', uri)
    if not m:
        return None
    import urllib.parse
    host = urllib.parse.unquote(m.group(2))
    p = os.path.join(hf.gvfs_root(), '%s:host=%s' % (m.group(1), host))
    if os.path.isdir(p):
        return p
    root = hf.gvfs_root()
    try:
        for n in os.listdir(root):
            if n.startswith(m.group(1) + ':host=') and host.replace('_', '').lower() in n.replace('_', '').lower():
                return os.path.join(root, n)
    except OSError:
        pass
    return None


def portable(hf):
    if not shutil.which('gio'):
        return {'devices': [], 'gio': False}
    try:
        r = subprocess.run(['gio', 'mount', '-li'], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return {'devices': []}
    devs = parse_gio_mounts(r.stdout)
    for d in devs:
        d['path'] = gvfs_path(hf, d['uri']) if d['mounted'] else None
    return {'devices': devs}


def portable_mount(hf, uri):
    if not re.match(r'^(mtp|gphoto2)://[^/\s]+/?$', uri or ''):
        raise hf.Fail('That isn\'t a phone or camera')
    try:
        r = subprocess.run(['gio', 'mount', uri], capture_output=True, text=True, timeout=60)
    except FileNotFoundError:
        raise hf.Fail('Phones and cameras need gvfs (it comes with Hearth OS)')
    except subprocess.TimeoutExpired:
        raise hf.Fail('It took too long: unlock the phone and choose "File transfer" on it, then try again')
    err = (r.stderr or '').strip()
    if r.returncode != 0 and 'already mounted' not in err.lower():
        raise hf.Fail('Couldn\'t open it. On a phone: unlock it and choose "File transfer" when it asks' + (' (' + err.splitlines()[-1][:160] + ')' if err else ''))
    for _ in range(30):
        p = gvfs_path(hf, uri)
        if p:
            return {'ok': True, 'path': p}
        time.sleep(0.2)
    raise hf.Fail('It opened, but its files can\'t be shown here (gvfs-fuse isn\'t running)')


def portable_unmount(hf, path):
    p = os.path.realpath(path)
    if not hf.under(p, os.path.realpath(hf.gvfs_root())):
        raise hf.Fail('That isn\'t a phone or camera')
    name = os.path.relpath(p, os.path.realpath(hf.gvfs_root())).split('/')[0]
    m = re.match(r'^(mtp|gphoto2):host=(.+)$', name)
    if not m:
        raise hf.Fail('That isn\'t a phone or camera')
    import urllib.parse
    uri = '%s://%s/' % (m.group(1), urllib.parse.quote(m.group(2), safe=''))
    try:
        subprocess.run(['gio', 'mount', '-u', uri], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        raise hf.Fail('Couldn\'t let it go: close what has its files open')
    return {'ok': True}


# ---------------------------------------------------------------------- service menus (right-click actions apps add)
def menu_dirs(hf):
    out = []
    for d in hf.data_dirs():
        for sub in ('kio/servicemenus', 'kservices5/ServiceMenus', 'kf6/servicemenus'):
            p = os.path.join(d, sub)
            if os.path.isdir(p) and p not in out:
                out.append(p)
    return out


def parse_service_menu(path):
    """{'entry': {...}, 'actions': {id: {...}}} from a service menu .desktop file (localized keys left out)."""
    entry, actions, section = {}, {}, None
    try:
        with open(path, encoding='utf-8', errors='replace') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                if line.startswith('['):
                    section = line[1:-1]
                    continue
                if '=' not in line:
                    continue
                k, v = line.split('=', 1)
                k = k.strip()
                if '[' in k:
                    continue
                if section == 'Desktop Entry':
                    entry[k] = v.strip()
                elif section and section.startswith('Desktop Action '):
                    actions.setdefault(section[len('Desktop Action '):].strip(), {})[k] = v.strip()
    except OSError:
        return None
    return {'entry': entry, 'actions': actions}


def mime_matches(pattern, mime, isdir):
    pattern = pattern.strip()
    if not pattern:
        return False
    if pattern in ('all/all', '*/*', '*'):
        return True
    if pattern == 'all/allfiles':
        return not isdir
    if pattern == 'inode/directory':
        return isdir
    if pattern.endswith('/*'):
        return not isdir and mime.split('/')[0] == pattern[:-2]
    return mime == pattern


def is_service_menu(entry):
    kinds = (entry.get('ServiceTypes', '') + ';' + entry.get('X-KDE-ServiceTypes', '') + ';' + entry.get('MimeType', ''))
    return entry.get('Type') == 'Service' or 'KonqPopupMenu/Plugin' in kinds


def service_menus(hf, paths):
    if not paths:
        return {'actions': []}
    kinds = []
    for p in paths:
        isdir = os.path.isdir(p)
        kinds.append((hf.sniff_mime(p) if not isdir else 'inode/directory', isdir))
    user_dir = os.path.realpath(os.path.join(os.environ.get('XDG_DATA_HOME') or os.path.join(hf.home(), '.local', 'share')))
    out, seen = [], set()
    for d in menu_dirs(hf):
        try:
            names = sorted(os.listdir(d))
        except OSError:
            continue
        for name in names:
            f = os.path.join(d, name)
            # Hearth's own Send to TV for Dolphin: Hearth Files has its own entry (one per TV).
            if not name.endswith('.desktop') or name in seen or name == 'hearth-send-to-tv.desktop':
                continue
            # Like KDE (5.85 and later): a service menu you added yourself only counts once it's marked as a program.
            if hf.under(os.path.realpath(f), user_dir) and not os.access(f, os.X_OK):
                continue
            sm = parse_service_menu(f)
            if not sm or not is_service_menu(sm['entry']):
                continue
            seen.add(name)
            e = sm['entry']
            if e.get('Hidden', '').lower() == 'true':
                continue
            mimes = [m for m in e.get('MimeType', '').split(';') if m]
            if not mimes or not all(any(mime_matches(m, k, isdir) for m in mimes) for k, isdir in kinds):
                continue
            n = len(paths)
            mn, mx = e.get('X-KDE-MinNumberOfUrls'), e.get('X-KDE-MaxNumberOfUrls')
            req = [int(x) for x in re.findall(r'\d+', e.get('X-KDE-RequiredNumberOfUrls', ''))]
            if (mn and mn.isdigit() and n < int(mn)) or (mx and mx.isdigit() and n > int(mx)) or (req and n not in req):
                continue
            want = [a for a in e.get('Actions', '').split(';') if a]
            sub = e.get('X-KDE-Submenu', '')
            for a in want:
                act = sm['actions'].get(a)
                if not act or not act.get('Exec') or not act.get('Name'):
                    continue
                out.append({'id': name + '#' + a, 'label': act['Name'], 'icon': act.get('Icon', ''), 'submenu': sub})
    return {'actions': out[:40]}


def find_service(hf, sid):
    if '#' not in sid:
        raise hf.Fail('That action isn\'t there any more')
    name, act = sid.split('#', 1)
    if '/' in name or not name.endswith('.desktop'):
        raise hf.Fail('That action isn\'t there any more')
    user_dir = os.path.realpath(os.path.join(os.environ.get('XDG_DATA_HOME') or os.path.join(hf.home(), '.local', 'share')))
    for d in menu_dirs(hf):
        f = os.path.join(d, name)
        if os.path.isfile(f):
            if hf.under(os.path.realpath(f), user_dir) and not os.access(f, os.X_OK):
                raise hf.Fail('That action isn\'t marked as a program, so it doesn\'t run')
            sm = parse_service_menu(f)
            a = (sm or {}).get('actions', {}).get(act)
            if a and a.get('Exec'):
                return f, a
    raise hf.Fail('That action isn\'t there any more')


def service_argv(hf, execline, paths, fields, desktop_file):
    """argv lists for an action's Exec with these files: %F / %U take them all in one run; %f / %u (or no code) run
    once per file. Field codes as the Desktop Entry spec has them (%% a plain %, %c the name, %k the file, %i the icon)."""
    import shlex
    try:
        words = shlex.split(execline)
    except ValueError:
        words = execline.split()
    many = any(w in ('%F', '%U') for w in words)

    def one(files):
        argv, used = [], False
        for w in words:
            if w in ('%F', '%f'):
                argv += files if w == '%F' else files[:1]
                used = True
            elif w in ('%U', '%u'):
                argv += [hf.file_uri(x) for x in (files if w == '%U' else files[:1])]
                used = True
            elif w == '%i':
                if fields.get('Icon'):
                    argv += ['--icon', fields['Icon']]
            else:
                def code(m):
                    c = m.group(1)
                    return {'%': '%', 'c': fields.get('Name', ''), 'k': desktop_file}.get(c, '')
                argv.append(re.sub(r'%(.)', code, w))
        if not used:
            argv += files
        return argv
    return [one(paths)] if many else [one([p]) for p in paths]


def service_run(hf, sid, paths):
    if not paths:
        raise hf.Fail('Pick something first')
    f, act = find_service(hf, sid)
    runs = service_argv(hf, act['Exec'], [os.path.abspath(p) for p in paths], {'Name': act.get('Name', ''), 'Icon': act.get('Icon', '')}, f)
    cwd = os.path.dirname(os.path.abspath(paths[0]))
    for argv in runs[:50]:
        hf.spawn(argv, cwd=cwd)
    return {'ok': True, 'ran': len(runs[:50]), 'label': act.get('Name', '')}


# ---------------------------------------------------------------------- sharing a folder (Settings › Home server)
def home_server(hf):
    """Settings › Home server's helper, next to this file (HEARTH_HOME_SERVER_BIN overrides it: tests)."""
    return os.environ.get('HEARTH_HOME_SERVER_BIN') or os.path.join(os.path.dirname(os.path.realpath(__file__)), 'hearth-home-server')


def hs(hf, args, timeout=120):
    h = home_server(hf)
    if not os.path.exists(h):
        raise hf.Fail('Sharing comes with Settings › Home server')
    try:
        r = subprocess.run([sys.executable, h] + args, capture_output=True, text=True, timeout=timeout, stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        raise hf.Fail('That took too long')
    lines = [l for l in r.stdout.strip().splitlines() if l.strip()]
    last = lines[-1] if lines else ''
    if last.startswith('error: '):
        raise hf.Fail(last[7:])
    try:
        return json.loads(last)
    except ValueError:
        raise hf.Fail('Sharing didn\'t answer')


def sharing(hf, folder):
    p = os.path.realpath(folder)
    st = hs(hf, ['status'], timeout=60)
    share = next((s for s in st.get('shares') or [] if os.path.realpath(s.get('path', '')) == p), None)
    tools = st.get('tools') or {}
    return {'path': p, 'shared': bool(share), 'share': share, 'samba': tools.get('samba', True)}


def share(hf, folder, what):
    p = os.path.realpath(folder)
    if not os.path.isdir(p):
        raise hf.Fail('Only folders can be shared')
    cur = sharing(hf, p)
    if what == 'on':
        if cur['shared']:
            return cur
        hs(hf, ['share-add', p])
    elif what == 'off':
        if cur['shared']:
            hs(hf, ['share-remove', cur['share']['id']])
    elif what in ('write-on', 'write-off'):
        if not cur['shared']:
            raise hf.Fail('Share it first')
        hs(hf, ['share-set', cur['share']['id'], 'write', 'on' if what == 'write-on' else 'off'])
    else:
        raise hf.Fail('on, off, write-on or write-off')
    return sharing(hf, p)


# ---------------------------------------------------------------------- recent files and their privacy
RECENT_DEFAULTS = {'remember': True, 'keepDays': 0, 'leaveOut': []}


def recent_prefs_file():
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(os.path.expanduser('~'), '.config')
    return os.path.join(cfg, 'nimbus', 'recent.json')


def recent_prefs(hsio=None):
    hsio = hsio or _hsio()
    d = dict(RECENT_DEFAULTS)
    if hsio and os.path.exists(recent_prefs_file()):
        saved = hsio.load_json(recent_prefs_file(), {})
        if isinstance(saved, dict):
            for k in RECENT_DEFAULTS:
                if k in saved:
                    d[k] = saved[k]
    return d


def _hsio():
    sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
    try:
        import hearth_settings_io
        return hearth_settings_io
    except ImportError:
        return None


def xbel_file():
    base = os.environ.get('XDG_DATA_HOME') or os.path.join(os.path.expanduser('~'), '.local', 'share')
    return os.path.join(base, 'recently-used.xbel')


def _when(attrs):
    """Seconds since 1970 for a bookmark's visited/modified/added time ("2026-11-16T10:00:00.123Z")."""
    import calendar
    w = attrs.get('visited') or attrs.get('modified') or attrs.get('added') or ''
    m = re.match(r'(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)', w)
    if not m:
        return 0
    return calendar.timegm(tuple(int(x) for x in m.groups()) + (0, 0, 0))


def recent_allowed(prefs, path, apps, when, now=None):
    """Is a recent file shown (and kept)? Off: none. keepDays: only newer ones. leaveOut: folders and apps."""
    if not prefs.get('remember', True):
        return False
    days = prefs.get('keepDays') or 0
    if days and when and (now or time.time()) - when > days * 86400:
        return False
    for x in prefs.get('leaveOut') or []:
        x = str(x)
        if x.startswith('/') and (path == x or path.startswith(x.rstrip('/') + '/')):
            return False
        if not x.startswith('/') and any(a.lower() == x.lower() for a in apps):
            return False
    return True


BOOKMARK = re.compile(r'<bookmark\b([^>]*?)/>|<bookmark\b([^>]*)>(.*?)</bookmark>', re.S)


def xbel_items(text):
    """[(whole text, attrs, path, apps)] for each bookmark."""
    import urllib.parse
    out = []
    for m in BOOKMARK.finditer(text):
        head = m.group(1) if m.group(1) is not None else m.group(2)
        attrs = dict(re.findall(r'(\w+)="([^"]*)"', head))
        href = attrs.get('href', '')
        path = urllib.parse.unquote(href[7:], errors='surrogateescape') if href.startswith('file://') else ''
        apps = re.findall(r'<bookmark:application\b[^>]*\bname="([^"]*)"', m.group(3) or '')
        out.append((m.group(0), attrs, path, apps))
    return out


def recent_rewrite(keep):
    """Rewrites recently-used.xbel keeping the bookmarks `keep(attrs, path, apps)` says to (atomic, with .bak)."""
    f = xbel_file()
    try:
        with open(f, encoding='utf-8', errors='surrogateescape') as fh:
            text = fh.read()
    except FileNotFoundError:
        return 0
    gone = 0
    new = text
    for whole, attrs, path, apps in xbel_items(text):
        if not keep(attrs, path, apps):
            new = new.replace(whole, '', 1)
            gone += 1
    if gone:
        new = re.sub(r'\n\s*\n+', '\n', new)
        try:
            shutil.copy2(f, f + '.bak')
        except OSError:
            pass
        tmp = f + '.hearth-tmp'
        with open(tmp, 'w', encoding='utf-8', errors='surrogateescape') as fh:
            fh.write(new)
            fh.flush()
            os.fsync(fh.fileno())
        os.replace(tmp, f)
    return gone


def recent_forget(hf, what):
    now = time.time()
    if what == 'all':
        gone = recent_rewrite(lambda a, p, apps: False)
    elif what in ('hour', 'day'):
        span = 3600 if what == 'hour' else 86400
        gone = recent_rewrite(lambda a, p, apps: _when(a) < now - span)
    else:
        p = os.path.abspath(what)
        gone = recent_rewrite(lambda a, path, apps: path != p)
    return {'ok': True, 'forgot': gone}


def recent_prefs_set(hf, raw):
    hsio = _hsio()
    if not hsio:
        raise hf.Fail('Hearth\'s settings helper is missing')
    try:
        ch = json.loads(raw)
    except ValueError:
        raise hf.Fail('That isn\'t a setting')
    if not isinstance(ch, dict):
        raise hf.Fail('That isn\'t a setting')
    clean = {}
    if isinstance(ch.get('remember'), bool):
        clean['remember'] = ch['remember']
    if isinstance(ch.get('keepDays'), int) and ch['keepDays'] in (0, 1, 7, 30, 90, 365):
        clean['keepDays'] = ch['keepDays']
    if isinstance(ch.get('leaveOut'), list) and all(isinstance(x, str) and 0 < len(x) < 400 for x in ch['leaveOut']) and len(ch['leaveOut']) <= 100:
        clean['leaveOut'] = ch['leaveOut']
    if not clean:
        raise hf.Fail('Nothing to change')
    hsio.merge_json(recent_prefs_file(), clean, who='Settings › Recent files')
    prefs = recent_prefs(hsio)
    # What the new choice says isn't kept goes now; GTK apps are told too (they keep their own list otherwise).
    gone = recent_rewrite(lambda a, p, apps: recent_allowed(prefs, p, apps, _when(a)))
    if 'remember' in clean and shutil.which('gsettings'):
        try:
            subprocess.run(['gsettings', 'set', 'org.gnome.desktop.privacy', 'remember-recent-files', 'true' if clean['remember'] else 'false'],
                           capture_output=True, timeout=5)
        except (OSError, subprocess.TimeoutExpired):
            pass
    return {'ok': True, 'prefs': prefs, 'forgot': gone}


def recent_apps(hf):
    """Apps that opened recent files (for "Leave out an app")."""
    try:
        with open(xbel_file(), encoding='utf-8', errors='replace') as fh:
            text = fh.read()
    except OSError:
        return []
    seen = []
    for _, _, _, apps in xbel_items(text):
        for a in apps:
            if a not in seen:
                seen.append(a)
    return seen[:40]


# ---------------------------------------------------------------------- recent files per activity (1.14)
# Activities.qml says which activity you're on as you switch (activity-mark): $XDG_DATA_HOME/hearth/activity-history.json
# keeps {"spans": [[seconds since 1970, activity id], ...]}, oldest first (a record of what happened, not a setting:
# written atomically with a .bak, the last ACT_SPANS kept). A recent file belongs to the activity that was on when it
# was last opened (before the first record: Main). Each activity's choice is in activities.json ("act:<id>".recent,
# activities.js): "own" (the default) shows only its files, "all" every activity's, and "off" forgets what's opened
# on it (activity-tidy, run at every switch and each minute while you're on it).
ACT_SPANS = 3000
ACT_MODES = ('own', 'all', 'off')
ACT_ID = re.compile(r'^[A-Za-z0-9_-]{1,40}$')


def activity_history_file():
    base = os.environ.get('XDG_DATA_HOME') or os.path.join(os.path.expanduser('~'), '.local', 'share')
    return os.path.join(base, 'hearth', 'activity-history.json')


def activities_file():
    cfg = os.environ.get('XDG_CONFIG_HOME') or os.path.join(os.path.expanduser('~'), '.config')
    return os.path.join(cfg, 'nimbus', 'activities.json')


def activity_spans():
    for f in (activity_history_file(), activity_history_file() + '.bak'):
        try:
            with open(f) as fh:
                d = json.load(fh)
            spans = [s for s in (d.get('spans') or []) if isinstance(s, list) and len(s) == 2
                     and isinstance(s[0], (int, float)) and isinstance(s[1], str)]
            return sorted(spans, key=lambda s: s[0])
        except (OSError, ValueError, AttributeError):
            continue
    return []


def activities_conf():
    """{"current", "modes": {id: own|all|off}, "names": {id: name}, "many"} from activities.json (read only; a file
    that can't be read means one activity, Main: everything shows as before)."""
    try:
        with open(activities_file()) as fh:
            d = json.load(fh)
        if not isinstance(d, dict):
            d = {}
    except (OSError, ValueError):
        d = {}
    modes, names = {'default': 'own'}, {'default': 'Main'}
    for k, v in d.items():
        if k.startswith('act:') and isinstance(v, dict) and k[4:]:
            modes[k[4:]] = v.get('recent') if v.get('recent') in ACT_MODES else 'own'
            names[k[4:]] = str(v.get('name') or '').strip() or ('Main' if k[4:] == 'default' else 'Activity')
    cur = d.get('current') if isinstance(d.get('current'), str) and d.get('current') in modes else 'default'
    return {'current': cur, 'modes': modes, 'names': names, 'many': len(modes) > 1}


def activity_at(spans, when):
    """The activity that was on at WHEN (seconds): the last switch before it; Main before the first one."""
    act = 'default'
    for t, a in spans:
        if t > when:
            break
        act = a
    return act


def recent_visible(prefs, path, apps, when, conf=None, spans=None):
    """recent_allowed, and on an activity with its own list ("own"), only what was opened on it. Lists only: never
    used to decide what's forgotten."""
    if not recent_allowed(prefs, path, apps, when):
        return False
    conf = conf if conf is not None else activities_conf()
    if not conf['many'] or conf['modes'].get(conf['current'], 'own') != 'own':
        return True
    spans = spans if spans is not None else activity_spans()
    return activity_at(spans, when or 0) == conf['current']


def _save_spans(spans):
    f = activity_history_file()
    os.makedirs(os.path.dirname(f), mode=0o700, exist_ok=True)
    if os.path.exists(f):
        try:
            with open(f) as fh:
                json.load(fh)
            shutil.copy2(f, f + '.bak')          # only a good copy becomes the .bak
        except (OSError, ValueError):
            pass
    tmp = f + '.tmp'
    with open(tmp, 'w') as fh:
        json.dump({'spans': spans[-ACT_SPANS:]}, fh)
        fh.flush()
        os.fsync(fh.fileno())
    os.replace(tmp, f)


def activity_mark(hf, act, now=None):
    """You're on ACT from now on (Activities.qml at every switch and when the shell starts); then activity_tidy."""
    if not ACT_ID.match(act or ''):
        raise hf.Fail('That isn\'t an activity')
    spans = activity_spans()
    if not spans or spans[-1][1] != act:
        spans.append([int(now or time.time()), act])
        _save_spans(spans)
    return dict(activity_tidy(hf, now), current=act)


def activity_tidy(hf, now=None):
    """Forgets recent files opened while an activity set to "off" (Don't remember) was on."""
    conf = activities_conf()
    off = {a for a, m in conf['modes'].items() if m == 'off'}
    if not off:
        return {'ok': True, 'forgot': 0}
    spans = activity_spans()
    return {'ok': True, 'forgot': recent_rewrite(lambda a, p, apps: activity_at(spans, _when(a)) not in off)}


def activity_forget(hf, act):
    """Forget the recent files that belong to one activity (Settings › Recent files, "Forget its recent files")."""
    if not ACT_ID.match(act or ''):
        raise hf.Fail('That isn\'t an activity')
    spans = activity_spans()
    return {'ok': True, 'forgot': recent_rewrite(lambda a, p, apps: activity_at(spans, _when(a)) != act)}


def activity_counts():
    """[{id, name, mode, files}] for Settings › Recent files (how many recent files each activity has)."""
    conf = activities_conf()
    if not conf['many']:
        return []
    try:
        with open(xbel_file(), encoding='utf-8', errors='replace') as fh:
            text = fh.read()
    except OSError:
        text = ''
    spans, n = activity_spans(), {}
    for _, attrs, p, _ in xbel_items(text):
        if p:
            a = activity_at(spans, _when(attrs))
            n[a] = n.get(a, 0) + 1
    return [{'id': a, 'name': conf['names'].get(a, a), 'mode': m, 'files': n.get(a, 0)} for a, m in conf['modes'].items()]


# ---------------------------------------------------------------------- permissions: a whole folder, groups
def chmod_tree(hf, folder, mode):
    p = hf.check_writable(folder, allow_root=True)
    if not os.path.isdir(p) or os.path.islink(p):
        raise hf.Fail('Only a folder\'s contents can be changed this way')
    try:
        m = int(str(mode), 8)
    except ValueError:
        raise hf.Fail('That isn\'t a permission number')
    if not 0 <= m <= 0o777:
        raise hf.Fail('That isn\'t a permission number')
    me, n, skipped = os.getuid(), 0, 0
    for root, dirs, files in os.walk(p, followlinks=False):
        for name in [None] + files:
            q = root if name is None else os.path.join(root, name)
            try:
                st = os.lstat(q)
            except OSError:
                continue
            if os.path.islink(q) or st.st_uid != me:
                skipped += 1
                continue
            if name is None:
                new = m
            else:
                new = (m & ~0o111) | (m & 0o111 if st.st_mode & 0o111 else 0)
            try:
                os.chmod(q, new)
                n += 1
            except OSError:
                skipped += 1
    return {'ok': True, 'changed': n, 'skipped': skipped}


def groups(hf):
    import grp
    out = []
    for g in os.getgroups():
        try:
            out.append(grp.getgrgid(g).gr_name)
        except KeyError:
            pass
    return {'groups': sorted(set(out))}


def chgrp(hf, path, group):
    import grp
    p = hf.check_writable(path, allow_root=True)
    try:
        gid = grp.getgrnam(group).gr_gid
    except KeyError:
        raise hf.Fail('There\'s no group called ' + group)
    if gid not in os.getgroups():
        raise hf.Fail('You\'re not in ' + group + ' (an administrator can change it: Act as administrator)')
    try:
        os.chown(p, -1, gid, follow_symlinks=False)
    except OSError as e:
        raise hf.Fail(hf.friendly(e, p))
    return {'ok': True, 'group': group}


# ---------------------------------------------------------------------- Send to TV
# Right-click a video or a song › Send to TV: it plays on a Hearth TV paired with this PC (docs/HEARTH-TV.md), through
# Hearth Link (hearth-send-to-tv). The file stays here; the TV plays it from this PC.
def send_tool():
    return shutil.which('hearth-send-to-tv') or '/usr/bin/hearth-send-to-tv'


def tv_list(hf):
    try:
        r = subprocess.run([sys.executable, send_tool(), '--list'], capture_output=True, text=True, timeout=15)
        return {'tvs': [t for t in json.loads(r.stdout or '{}').get('tvs', []) if t.get('id') and t.get('name')]}
    except (OSError, ValueError, subprocess.SubprocessError):
        return {'tvs': []}


def tv_send(hf, path, tv=''):
    if not os.path.isfile(path):
        raise hf.Fail('Only a file can be sent to the TV')
    args = [sys.executable, send_tool(), '--json'] + (['--tv', tv] if tv else []) + [path]
    try:
        r = subprocess.run(args, capture_output=True, text=True, timeout=60)
        v = json.loads((r.stdout or '').strip().splitlines()[-1])
    except (OSError, ValueError, IndexError, subprocess.SubprocessError):
        raise hf.Fail("Couldn't send it to the TV (is Hearth Link on?)")
    if v.get('error'):
        raise hf.Fail(v['error'])
    return v


# ---------------------------------------------------------------------- the commands
COMMANDS = ('tags', 'tag-set', 'tag-list', 'tag-find', 'git-status', 'iso-mount', 'iso-unmount', 'portable', 'portable-mount',
            'portable-unmount', 'service-menus', 'service-run', 'sharing', 'share', 'recent-prefs', 'recent-prefs-set',
            'recent-forget', 'activity-mark', 'activity-tidy', 'chmod-tree', 'groups', 'chgrp', 'tv-list', 'tv-send')


def run(hf, c, rest):
    one = len(rest) == 1
    if c == 'tags' and rest:
        return tags_of(hf, rest)
    if c == 'tag-set' and rest:
        return tag_set(hf, rest)
    if c == 'tag-list' and not rest:
        return tag_list(hf)
    if c == 'tag-find' and one:
        return tag_find(hf, rest[0])
    if c == 'git-status' and one:
        return git_status(hf, rest[0])
    if c == 'iso-mount' and one:
        return iso_mount(hf, rest[0])
    if c == 'iso-unmount' and one:
        return iso_unmount(hf, rest[0])
    if c == 'portable' and not rest:
        return portable(hf)
    if c == 'portable-mount' and one:
        return portable_mount(hf, rest[0])
    if c == 'portable-unmount' and one:
        return portable_unmount(hf, rest[0])
    if c == 'service-menus':
        return service_menus(hf, rest)
    if c == 'service-run' and len(rest) >= 2:
        return service_run(hf, rest[0], rest[1:])
    if c == 'sharing' and one:
        return sharing(hf, rest[0])
    if c == 'share' and len(rest) == 2:
        return share(hf, rest[0], rest[1])
    if c == 'recent-prefs' and not rest:
        return {'prefs': recent_prefs(), 'apps': recent_apps(hf), 'activities': activity_counts()}
    if c == 'recent-prefs-set' and one:
        return recent_prefs_set(hf, rest[0])
    if c == 'recent-forget' and one:
        if rest[0].startswith('activity:'):
            return activity_forget(hf, rest[0][9:])
        return recent_forget(hf, rest[0])
    if c == 'activity-mark' and one:
        return activity_mark(hf, rest[0])
    if c == 'activity-tidy' and not rest:
        return activity_tidy(hf)
    if c == 'chmod-tree' and len(rest) == 2:
        return chmod_tree(hf, rest[0], rest[1])
    if c == 'groups' and not rest:
        return groups(hf)
    if c == 'chgrp' and len(rest) == 2:
        return chgrp(hf, rest[0], rest[1])
    if c == 'tv-list' and not rest:
        return tv_list(hf)
    if c == 'tv-send' and rest and len(rest) <= 2:
        return tv_send(hf, rest[0], rest[1] if len(rest) == 2 else '')
    raise hf.Fail('usage: hearth-files ' + c + ' … (see hearth_files_extras.py)')
