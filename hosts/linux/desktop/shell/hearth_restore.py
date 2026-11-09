#!/usr/bin/python3
"""hearth_restore: System Restore's restore points (docs/SYSTEM-RESTORE.md).

A restore point is two things:
  * the Hearth version that was running (the bootc deployment it can go back to), and
  * a copy of your Hearth settings files (the ones docs/SETTINGS-PERSISTENCE.md lists; SETTINGS below).
Apps you installed and your own files are never part of it and never touched.

Points live in ~/.local/share/hearth/restore/<id>/ (point.json and files/). About 10 are kept (plus the two newest
"Update" ones), so they never fill the drive. Each is a few hundred KB at most.

  hearth_restore.py overview                   JSON: the points, what each would change, which versions bootc keeps
  hearth_restore.py changes ID                 JSON: every setting that would go back
  hearth_restore.py snapshot --kind KIND [--title TEXT]
                                               make a point now (KIND: update, automatic, scheduled, manual)
  hearth_restore.py auto                       the user timer (hearth-restore-points.timer): a point for a newly
                                               installed Hearth version, and the weekly one
  hearth_restore.py apply ID [--settings-only] restore: a copy of today's settings first, then the settings, then
                                               (when the point's version is the one bootc keeps) Hearth's own
                                               `hearth-admin rollback`. If that fails, today's settings come back.

Going back to a version is never done here: it's Settings › Updates' own `pkexec hearth-admin rollback` (bootc
rollback: the version bootc keeps starts next, and the one you come from stays in the boot menu). Only when the
point's version is exactly the one bootc keeps, nothing waits for a restart and no rollback is queued already.

Settings are only ever written through hearth_settings_io (atomic, .bak), and only by `apply` (the person confirmed).
Opening System Restore (overview, changes) writes nothing. Python standard library only.
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import hearth_settings_io as sio  # noqa: E402

ADMIN = '/usr/libexec/nimbus/hearth-admin'
IMAGE_INFO = '/usr/share/nimbus/image-info.json'
KEEP = 10                 # restore points kept
KEEP_UPDATES = 2          # ...and the newest "Update" ones, even when older
MAX_FILE = 1024 * 1024    # a settings file bigger than this isn't a settings file
MIN_FREE = 200 * 1024 * 1024
WEEK = 7 * 86400
KINDS = {'update': 'Update', 'automatic': 'Automatic', 'scheduled': 'Scheduled', 'manual': 'Manual'}
ID_RE = re.compile(r'^[0-9]{8}-[0-9]{6}-(update|automatic|scheduled|manual)(-[0-9]{1,3})?$')

# ------------------------------------------------------------------ which files are settings
# (root, pattern) pairs, relative to the root. Only these are ever copied or put back.
#   config = $XDG_CONFIG_HOME (~/.config), data = $XDG_DATA_HOME (~/.local/share)
TOP = r'[A-Za-z0-9][A-Za-z0-9_.-]*\.(json|conf|sh)'
SETTINGS = [
    ('config', re.compile(r'^nimbus/' + TOP + r'$')),
    ('config', re.compile(r'^nimbus/input/[A-Za-z0-9][A-Za-z0-9_. -]*\.json$')),
    ('config', re.compile(r'^nimbus/theme/([A-Za-z0-9][A-Za-z0-9_.-]*\.json|themerc-override)$')),
    ('config', re.compile(r'^hearth/(drive-letters\.json|theme\.json|theme\.css|seats-audio\.json)$')),
    ('config', re.compile(r'^labwc/(environment|rc\.xml)$')),
    ('config', re.compile(r'^mimeapps\.list$')),
    ('data', re.compile(r'^nimbus/desktop\.json$')),
]
# Never kept: secrets, a display mode on trial, learned words (not settings), counters.
SKIP_RE = re.compile(r'(oauth|token|secret|credential|password|passwd|cookie)|'
                     r'^nimbus/(display-pending\.json|osk-words\.json|renderer[^/]*)$', re.I)
# Folders looked in (only these; never anything deeper).
SCAN = [('config', 'nimbus'), ('config', 'nimbus/input'), ('config', 'nimbus/theme'), ('config', 'hearth'),
        ('config', 'labwc'), ('config', ''), ('data', 'nimbus')]


class RestoreError(Exception):
    pass


def _home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def roots():
    return {'config': os.environ.get('XDG_CONFIG_HOME') or os.path.join(_home(), '.config'),
            'data': os.environ.get('XDG_DATA_HOME') or os.path.join(_home(), '.local', 'share')}


def store_dir():
    return os.path.join(roots()['data'], 'hearth', 'restore')


def allowed(key):
    """KEY is 'config/nimbus/desk.json'-style: one of SETTINGS, plainly written, not a secret."""
    if not isinstance(key, str) or '\0' in key or '\\' in key:
        return False
    root, _, rel = key.partition('/')
    if root not in ('config', 'data') or not rel or os.path.normpath(rel) != rel or rel.startswith(('/', '.')):
        return False
    if any(p in ('..', '.', '') for p in rel.split('/')) or SKIP_RE.search(rel):
        return False
    if rel.endswith(('.bak', '.tmp', '.lock')) or '.broken-' in rel:
        return False
    return any(root == r and rx.match(rel) for r, rx in SETTINGS)


def path_of(key):
    root, _, rel = key.partition('/')
    return os.path.join(roots()[root], rel)


def settings_files():
    """{key: path} of the settings files on this PC now (regular files only, never through a link)."""
    out = {}
    r = roots()
    for root, sub in SCAN:
        d = os.path.join(r[root], sub) if sub else r[root]
        try:
            names = sorted(os.listdir(d))
        except OSError:
            continue
        for n in names:
            key = '%s/%s' % (root, (sub + '/' + n) if sub else n)
            p = os.path.join(d, n)
            if allowed(key) and os.path.isfile(p) and not os.path.islink(p):
                out[key] = p
    return out


def _is_json(key):
    return key.endswith('.json')


def _valid(key, data):
    """A copy worth keeping or putting back: JSON files must be a JSON object, text must be text."""
    if _is_json(key):
        return sio._valid_json_object(data)
    try:
        data.decode('utf-8')
    except UnicodeDecodeError:
        return False
    return True


def _read(path, limit=MAX_FILE):
    try:
        with open(path, 'rb') as f:
            data = f.read(limit + 1)
    except OSError:
        return None
    return None if len(data) > limit else data


# ------------------------------------------------------------------ versions
def shown(v):
    """From 1.0 on Hearth OS is MAJOR.MINOR: 1.9.0 shows (and compares) as 1.9 (as Settings › Updates)."""
    v = str(v or '').strip()
    m = re.fullmatch(r'(\d+)\.(\d+)\.0', v)
    return '%s.%s' % (m.group(1), m.group(2)) if m and int(m.group(1)) >= 1 else v


def running_version(info=None):
    try:
        with open(info or os.environ.get('HEARTH_IMAGE_INFO') or IMAGE_INFO, encoding='utf-8') as f:
            v = json.load(f).get('version')
    except (OSError, ValueError, AttributeError):
        return ''
    v = shown(v)
    return '' if v in ('', '0.0.0') else v


# ------------------------------------------------------------------ points
def _fsync(path):
    try:
        fd = os.open(path, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except OSError:
        pass


def list_points():
    """Every readable point, newest first. A damaged one is skipped (and pruned later)."""
    out = []
    d = store_dir()
    try:
        names = os.listdir(d)
    except OSError:
        return out
    for n in names:
        if not ID_RE.match(n):
            continue
        try:
            with open(os.path.join(d, n, 'point.json'), encoding='utf-8') as f:
                p = json.load(f)
        except (OSError, ValueError):
            continue
        if isinstance(p, dict) and p.get('id') == n and isinstance(p.get('files'), dict):
            out.append(p)
    out.sort(key=lambda p: (p.get('created') or 0, p['id']), reverse=True)
    return out


def get_point(pid):
    if not isinstance(pid, str) or not ID_RE.match(pid):
        raise RestoreError('there is no restore point called that')
    for p in list_points():
        if p['id'] == pid:
            return p
    raise RestoreError('that restore point is gone')


def point_file(point, key):
    """The point's copy of KEY (bytes), or None."""
    if not allowed(key) or key not in point.get('files', {}):
        return None
    data = _read(os.path.join(store_dir(), point['id'], 'files', key))
    if data is None or hashlib.sha256(data).hexdigest() != point['files'][key]:
        return None
    return data


def default_title(kind, to=''):
    if kind == 'update':
        return 'Before updating to Hearth %s' % shown(to) if to else 'Before updating Hearth'
    if kind == 'scheduled':
        return 'Weekly restore point'
    if kind == 'manual':
        return 'Restore point you made'
    return 'Before a change Hearth made'


def snapshot(kind, title='', now=None, protect=(), version=None):
    """A new point with a copy of today's settings files. Built in a hidden folder and renamed into place whole."""
    if kind not in KINDS:
        raise RestoreError('kind is one of ' + ', '.join(KINDS))
    store = store_dir()
    os.makedirs(store, mode=0o700, exist_ok=True)
    try:
        if shutil.disk_usage(store).free < MIN_FREE:
            raise RestoreError('the drive is almost full, so no restore point was made')
    except OSError:
        pass
    now = time.time() if now is None else now
    base = time.strftime('%Y%m%d-%H%M%S', time.localtime(now)) + '-' + kind
    pid, n = base, 1
    while os.path.exists(os.path.join(store, pid)):
        n += 1
        pid = '%s-%d' % (base, n)
    tmp = os.path.join(store, '.%s.tmp-%d' % (pid, os.getpid()))
    shutil.rmtree(tmp, ignore_errors=True)
    os.mkdir(tmp, 0o700)
    os.chmod(tmp, 0o700)
    files, skipped = {}, []
    try:
        for key, path in settings_files().items():
            data = _read(path)
            if data is None or not _valid(key, data):
                skipped.append(key)       # too big, unreadable or damaged: never kept (so never put back)
                continue
            dest = os.path.join(tmp, 'files', key)
            os.makedirs(os.path.dirname(dest), mode=0o700, exist_ok=True)
            fd = os.open(dest, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, 'wb') as f:
                f.write(data)
                f.flush()
                os.fsync(f.fileno())
            files[key] = hashlib.sha256(data).hexdigest()
        point = {'id': pid, 'created': int(now), 'kind': kind, 'title': (title or default_title(kind))[:120],
                 'version': running_version() if version is None else shown(version), 'files': files,
                 'skipped': skipped}
        fd = os.open(os.path.join(tmp, 'point.json'), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'w', encoding='utf-8') as f:
            json.dump(point, f, indent=1, ensure_ascii=False)
            f.flush()
            os.fsync(f.fileno())
        os.rename(tmp, os.path.join(store, pid))
        _fsync(store)
    except BaseException:
        shutil.rmtree(tmp, ignore_errors=True)
        raise
    prune(protect=set(protect) | {pid}, now=now)
    return point


def prune(keep=KEEP, keep_updates=KEEP_UPDATES, protect=(), now=None):
    """Keeps the newest KEEP points and the newest KEEP_UPDATES "Update" ones; the rest go. Points in PROTECT stay
    (the one being restored). Half-made folders older than an hour go too. Returns the ids removed."""
    pts = list_points()
    keepers = {p['id'] for p in pts[:keep]} | {p['id'] for p in [q for q in pts if q.get('kind') == 'update'][:keep_updates]}
    keepers |= set(protect)
    removed = []
    store = store_dir()
    for p in pts:
        if p['id'] not in keepers and ID_RE.match(p['id']):
            shutil.rmtree(os.path.join(store, p['id']), ignore_errors=True)
            removed.append(p['id'])
    now = time.time() if now is None else now
    try:
        for n in os.listdir(store):
            full = os.path.join(store, n)
            if n.startswith('.') and '.tmp-' in n and os.path.isdir(full):
                try:
                    if now - os.stat(full).st_mtime > 3600:
                        shutil.rmtree(full, ignore_errors=True)
                except OSError:
                    pass
    except OSError:
        pass
    return removed


# ------------------------------------------------------------------ what would change
def _conf_pairs(text):
    out = {}
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith('#'):
            continue
        m = re.match(r'^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)=(.*)$', s)
        out[m.group(1) if m else s] = m.group(2) if m else ''
    return out


def _short(v):
    s = v if isinstance(v, str) else json.dumps(v, ensure_ascii=False)
    return s if len(s) <= 80 else s[:77] + '…'


def changes(point):
    """Every setting that would go back: [{file, key, then, now}]. A file made since the point isn't in it (it stays);
    those are listed as 'added' (file names only)."""
    out, added = [], []
    cur = settings_files()
    for key in sorted(point.get('files', {})):
        then = point_file(point, key)
        if then is None:
            continue
        now = _read(path_of(key))
        if now == then:
            continue
        name = key.split('/', 1)[1]
        if now is None:
            out.append({'file': name, 'key': '', 'then': 'saved', 'now': 'missing'})
            continue
        if _is_json(key) and sio._valid_json_object(now):
            a, b = json.loads(then.decode('utf-8')), json.loads(now.decode('utf-8'))
        else:
            a = _conf_pairs(then.decode('utf-8', 'replace'))
            b = _conf_pairs(now.decode('utf-8', 'replace'))
        diff = [k for k in sorted(set(a) | set(b)) if a.get(k, None) != b.get(k, None)]
        if not diff:
            out.append({'file': name, 'key': '', 'then': 'saved', 'now': 'written differently'})
        for k in diff:
            out.append({'file': name, 'key': k, 'then': _short(a[k]) if k in a else 'not set',
                        'now': _short(b[k]) if k in b else 'not set'})
    for key in sorted(cur):
        if key not in point.get('files', {}):
            added.append(key.split('/', 1)[1])
    return out, added


# ------------------------------------------------------------------ the version part
def status_command():
    return ['pkexec', ADMIN, 'update-status']


def rollback_command():
    """Settings › Updates' own "Go back": hearth-admin rollback (bootc rollback). An argument list, never a shell."""
    return ['pkexec', ADMIN, 'rollback']


def _last_json(text):
    for line in reversed((text or '').splitlines()):
        line = line.strip()
        if line.startswith('{'):
            try:
                v = json.loads(line)
            except ValueError:
                continue
            if isinstance(v, dict):
                return v
    return None


def read_status(run=subprocess.run, admin=ADMIN):
    """bootc's view through hearth-admin update-status (read-only): {booted, staged, rollback, rollbackQueued}."""
    if not os.access(admin, os.X_OK):
        return None
    try:
        r = run(status_command(), capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return None
    if r.returncode != 0 or 'error: ' in (r.stdout or ''):
        return None
    return _last_json(r.stdout)


def _ver(entry):
    return shown((entry or {}).get('version') or '') if isinstance(entry, dict) else ''


def plan(point, status):
    """What going back to POINT does to the Hearth version:
         none     it's the version running now (only settings go back)
         rollback it's the version bootc keeps: hearth-admin rollback, then restart
         queued   that version already starts at the next restart: just restart
         blocked  an update waits for a restart (bootc refuses a rollback then)
         missing  that version isn't kept any more: only the settings can go back
         unknown  the system couldn't say: only the settings"""
    target = shown(point.get('version') or '')
    if not status:
        return {'action': 'unknown', 'to': target,
                'why': "Hearth couldn't ask the system which versions it keeps, so only the settings can go back."}
    booted, rb = _ver(status.get('booted')), _ver(status.get('rollback'))
    base = {'to': target, 'from': booted}
    if not target:
        return dict(base, action='missing', why="This restore point doesn't say which Hearth version it was made on, so only the settings can go back.")
    if target == booted:
        return dict(base, action='none')
    if rb and target == rb:
        if status.get('staged'):
            return dict(base, action='blocked', why='An update is waiting for a restart. Restart first, then come back here; or restore only the settings.')
        if status.get('rollbackQueued'):
            return dict(base, action='queued')
        return dict(base, action='rollback')
    return dict(base, action='missing',
                why='Hearth %s is no longer kept on this PC (it keeps the version you run and the one before it), so only the settings can go back.' % target)


# ------------------------------------------------------------------ restoring
def restore_settings(point):
    """Puts the point's settings files back, each atomically with a .bak of today's, under its own lock. Only files on
    the SETTINGS list whose copy is intact; files made since then are left as they are. Returns the keys written."""
    written = []
    for key in sorted(point.get('files', {})):
        if not allowed(key):
            continue
        data = point_file(point, key)
        if data is None or not _valid(key, data):
            continue
        dest = path_of(key)
        if os.path.islink(dest):
            continue
        with sio.locked(dest):
            if _read(dest) == data:
                continue
            sio.write_bytes(dest, data, valid=lambda b, k=key: _valid(k, b) and len(b.strip()) > 0)
        written.append(key)
    if written:
        sio.log_event('System Restore: put back %d settings files from restore point %s' % (len(written), point['id']))
    return written


def apply(pid, settings_only=False, run=subprocess.run, admin=ADMIN, now=None):
    point = get_point(pid)
    pl = {'action': 'none'} if settings_only else plan(point, read_status(run, admin))
    if pl['action'] == 'blocked':
        raise RestoreError(pl['why'])
    when = time.strftime('%b %-d, %H:%M', time.localtime(point.get('created') or 0))
    safety = snapshot('automatic', 'Before restoring to %s' % when, now=now, protect={pid})
    written = restore_settings(point)
    restart = pl['action'] == 'queued'
    if pl['action'] == 'rollback':
        ok, why = False, ''
        try:
            r = run(rollback_command(), capture_output=True, text=True, timeout=900)
            errs = [l[7:] for l in (r.stdout or '').splitlines() if l.startswith('error: ')]
            st = _last_json(r.stdout)
            ok = r.returncode == 0 and not errs and bool(st and st.get('rollbackQueued'))
            if not ok and r.returncode == 0 and not errs:
                st = read_status(run, admin)       # said nothing useful: ask again
                ok = bool(st and st.get('rollbackQueued'))
            why = (errs[-1] if errs else '') or ('not allowed (Hearth\'s polkit rule is missing)' if r.returncode in (126, 127) else '')
        except (OSError, subprocess.SubprocessError) as e:
            why = str(e)
        if not ok:
            restore_settings(safety)                # today's settings come back: nothing changed after all
            raise RestoreError("Hearth couldn't go back to %s%s. Nothing was changed: your settings are as they were."
                               % (pl.get('to') or 'that version', ': ' + why if why else ''))
        restart = True
    sio.log_event('System Restore: restored %s (%s), version %s' % (pid, len(written), pl['action']))
    return {'ok': True, 'restart': restart, 'changed': len(written), 'safety': safety['id'], 'plan': pl}


# ------------------------------------------------------------------ the timer
def auto(now=None):
    """hearth-restore-points.timer: a point the first time a Hearth version runs (an update installed by Settings or
    in the background), and the weekly one. Light: a few small file copies, nothing else."""
    now = time.time() if now is None else now
    made = []
    pts = list_points()
    v = running_version()
    if v and not any(shown(p.get('version')) == v for p in pts):
        made.append(snapshot('update', 'Hearth %s' % v, now=now))
        pts = list_points()
    last = max([p.get('created') or 0 for p in pts if p.get('kind') == 'scheduled'] or [0])
    newest = max([p.get('created') or 0 for p in pts] or [0])
    if now - last >= WEEK and now - newest >= 3600:
        made.append(snapshot('scheduled', now=now))
    return made


def overview():
    st = read_status()
    pts = []
    for p in list_points():
        ch, added = changes(p)
        pts.append({'id': p['id'], 'created': p.get('created'), 'kind': p.get('kind'), 'type': KINDS.get(p.get('kind'), ''),
                    'title': p.get('title') or '', 'version': shown(p.get('version')), 'files': len(p.get('files', {})),
                    'changed': len(ch), 'added': len(added), 'plan': plan(p, st)})
    return {'running': running_version(), 'status': st, 'points': pts, 'keep': KEEP}


def main(argv):
    ap = argparse.ArgumentParser(prog='hearth_restore.py')
    sub = ap.add_subparsers(dest='cmd')
    sub.add_parser('overview')
    c = sub.add_parser('changes')
    c.add_argument('id')
    s = sub.add_parser('snapshot')
    s.add_argument('--kind', required=True, choices=sorted(KINDS))
    s.add_argument('--title', default='')
    s.add_argument('--to', default='')
    sub.add_parser('auto')
    a = sub.add_parser('apply')
    a.add_argument('id')
    a.add_argument('--settings-only', action='store_true')
    args = ap.parse_args(argv[1:])
    try:
        if args.cmd == 'overview':
            out = overview()
        elif args.cmd == 'changes':
            ch, added = changes(get_point(args.id))
            out = {'changes': ch, 'added': added}
        elif args.cmd == 'snapshot':
            out = snapshot(args.kind, args.title or default_title(args.kind, args.to))
        elif args.cmd == 'auto':
            out = {'made': [p['id'] for p in auto()]}
        elif args.cmd == 'apply':
            out = apply(args.id, args.settings_only)
        else:
            ap.print_help(sys.stderr)
            return 2
    except RestoreError as e:
        print(json.dumps({'ok': False, 'error': str(e)}))
        return 1
    print(json.dumps(out, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
