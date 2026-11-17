#!/usr/bin/python3
"""hearth_kconfig: reads and changes KDE's INI files (kwinrc, kdeglobals, kaccessrc, kwinrulesrc, kcminputrc) the safe way
(docs/SETTINGS-PERSISTENCE.md, docs/KWIN-SETTINGS.md).

  * Only the keys the person changed are written. Every other line of the file stays exactly as it was: other keys,
    other groups, comments, blank lines, keys in other languages (Name[de]=), KDE's [$i] / [$e] markers.
  * The write goes through hearth_settings_io.update_text: under Hearth's lock, atomic, the last good copy kept as
    FILE.bak. KDE programs (KWin) lock these files with a QLockFile (FILE.lock); that lock is taken too while writing.
  * A file that can't be read as an INI file (not UTF-8, a NUL byte, a group header that isn't closed) is never
    written over: BadFile is raised and nothing changes.
  * Hearth's defaults live in system files (XDG_CONFIG_DIRS, e.g. the session's $SHARE/desktop/kwin/xdg/kwinrc), never
    in the person's file; `effective` reads them under the person's file the way KConfig does.

Group names are KConfig's: a nested group is written [Tiling][<screen id>] and named here "Tiling][<screen id>" (the
text between the first "[" and the last "]"). Python standard library only.
"""
import contextlib
import os
import re
import socket
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
import hearth_settings_io as hsio  # noqa: E402

DEFAULT_GROUP = '<default>'
_HEADER = re.compile(r'^[ \t]*\[(.*)\][ \t]*$')
_MARKERS = re.compile(r'\[\$[a-z]+\]$')
KEY_RE = re.compile(r'^[A-Za-z0-9_./ -]{1,120}$')


class BadFile(Exception):
    """The file couldn't be read as an INI file: it is left exactly as it is."""


def config_home():
    return os.environ.get('XDG_CONFIG_HOME') or os.path.join(os.path.expanduser('~'), '.config')


def config_dirs():
    """XDG_CONFIG_DIRS, most important first (KConfig reads them least important first, then the person's file)."""
    dirs = [d for d in (os.environ.get('XDG_CONFIG_DIRS') or '/etc/xdg').split(':') if d]
    return dirs


def user_path(name):
    return os.path.join(config_home(), name)


# ------------------------------------------------------------------ parsing
def check_text(text, name='file'):
    """Raises BadFile when TEXT isn't something KConfig wrote (or could read)."""
    if '\x00' in text:
        raise BadFile('%s has a NUL byte in it' % name)
    if any('\udc80' <= ch <= '\udcff' for ch in text):
        raise BadFile('%s is not UTF-8 text' % name)
    for n, line in enumerate(text.splitlines(), 1):
        s = line.strip()
        if s.startswith('[') and not _HEADER.match(line):
            raise BadFile('%s line %d: a group header that is not closed (%r)' % (name, n, s[:60]))


def _split(line):
    """(key, markers, value) of an entry line, or None for a comment / blank / header."""
    s = line.strip()
    if not s or s.startswith('#') or s.startswith(';') or _HEADER.match(line):
        return None
    if '=' not in line:
        return None
    k, v = line.split('=', 1)
    k = k.strip()
    markers = ''
    m = _MARKERS.search(k)
    while m:
        markers = m.group(0) + markers
        k = k[:m.start()].rstrip()
        m = _MARKERS.search(k)
    return k, markers, v.strip(' \t')


def unescape(v):
    out, i = [], 0
    while i < len(v):
        c = v[i]
        if c == '\\' and i + 1 < len(v):
            n = v[i + 1]
            out.append({'n': '\n', 't': '\t', 'r': '\r', 's': ' ', '\\': '\\', ';': ';', ',': ','}.get(n, '\\' + n))
            i += 2
            continue
        out.append(c)
        i += 1
    return ''.join(out)


def escape(v):
    v = v.replace('\\', '\\\\').replace('\n', '\\n').replace('\t', '\\t').replace('\r', '\\r')
    if v.startswith(' '):
        v = '\\s' + v[1:]
    if v.endswith(' '):
        v = v[:-1] + '\\s'
    return v


def to_text(value):
    """A JSON value as KConfig writes it: booleans true/false, lists comma-separated."""
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, float):
        return ('%.4f' % value).rstrip('0').rstrip('.') if value != int(value) else str(int(value))
    if isinstance(value, (list, tuple)):
        return ','.join(to_text(x).replace(',', '\\,') for x in value)
    return str(value)


def parse(text):
    """{group: {key: value}} (later lines win, as in KConfig), plus {group: {key: markers}} for [$i] locks."""
    data, marks = {}, {}
    group = DEFAULT_GROUP
    group_locked = False
    for line in text.splitlines():
        h = _HEADER.match(line)
        if h:
            group = h.group(1)
            group_locked = group.endswith('][$i')
            if group_locked:
                group = group[:-4]
            data.setdefault(group, {})
            if group_locked:
                marks.setdefault(group, {})['*'] = '[$i]'
            continue
        e = _split(line)
        if not e:
            continue
        k, mk, v = e
        if '[' in k:
            continue    # a key in another language (Name[de]): not one Hearth reads
        data.setdefault(group, {})[k] = unescape(v)
        if mk:
            marks.setdefault(group, {})[k] = mk
    return data, marks


def read_file(path):
    try:
        with open(path, 'rb') as f:
            raw = f.read()
    except OSError:
        return ''
    try:
        text = raw.decode('utf-8')
    except UnicodeDecodeError:
        raise BadFile('%s is not UTF-8 text' % path)
    check_text(text, os.path.basename(path))
    return text


def effective(name):
    """What KDE programs see for config NAME: the system files (least important first), then the person's file.
    Returns (values, mine, locked): mine = {group: [keys set in the person's file]}, locked = {group: [keys]} that a
    system file marks [$i] (KDE won't let the person change them)."""
    values, locked = {}, {}
    for d in reversed(config_dirs()):
        p = os.path.join(d, name)
        if not os.path.isfile(p):
            continue
        try:
            data, marks = parse(read_file(p))
        except BadFile:
            continue
        for g, kv in data.items():
            values.setdefault(g, {}).update(kv)
        for g, kv in marks.items():
            for k, mk in kv.items():
                if '[$i]' in mk:
                    locked.setdefault(g, []).append(k)
    data, _ = parse(read_file(user_path(name)))
    mine = {}
    for g, kv in data.items():
        for k, v in kv.items():
            if k in locked.get(g, []) or '*' in locked.get(g, []):
                continue
            values.setdefault(g, {})[k] = v
            mine.setdefault(g, []).append(k)
    return values, mine, locked


# ------------------------------------------------------------------ changing
def _valid_name(s, what):
    if not isinstance(s, str) or not s or '\n' in s or '\r' in s or '\x00' in s:
        raise ValueError('bad %s name %r' % (what, s))


def merge_text(text, changes, remove_groups=()):
    """TEXT with CHANGES ({group: {key: value or None}}) made. A key that's there gets its value on its last line (the
    one KConfig uses); None removes the key's lines in that group (so the system default counts again); a new key goes
    at the end of the group's last block; a new group at the end of the file. REMOVE_GROUPS: whole groups taken out
    (a window rule that's deleted). Everything else is kept byte for byte."""
    check_text(text)
    for g, kv in changes.items():
        _valid_name(g, 'group')
        if not isinstance(kv, dict):
            raise ValueError('changes for [%s] must be an object' % g)
        for k in kv:
            if not isinstance(k, str) or not KEY_RE.match(k):
                raise ValueError('bad key name %r' % (k,))
    lines = text.splitlines()
    # Where each group's blocks and keys are.
    group = DEFAULT_GROUP
    blocks = {}          # group -> [(start, end)] of its line ranges (start = header line or 0)
    start = 0
    for i, line in enumerate(lines):
        h = _HEADER.match(line)
        if h:
            blocks.setdefault(group, []).append((start, i))
            group, start = h.group(1), i
    blocks.setdefault(group, []).append((start, len(lines)))
    drop = set()
    for g in remove_groups:
        for a, b in blocks.get(g, []):
            drop.update(range(a, b))
    replace, add_after, new_groups = {}, {}, []
    for g, kv in changes.items():
        if g in remove_groups:
            continue
        found = {}
        for a, b in blocks.get(g, []):
            for i in range(a, b):
                e = _split(lines[i])
                if e and e[0] in kv:
                    found.setdefault(e[0], []).append(i)
        new_keys = []
        for k, v in kv.items():
            if v is None:
                drop.update(found.get(k, []))
                continue
            if k in found:
                i = found[k][-1]
                e = _split(lines[i])
                replace[i] = '%s%s=%s' % (k, e[1], escape(to_text(v)))
            else:
                new_keys.append('%s=%s' % (k, escape(to_text(v))))
        if not new_keys:
            continue
        if g in blocks and not (g == DEFAULT_GROUP and blocks[g] == [(0, 0)]):
            a, b = blocks[g][-1]
            # after the block's last line that isn't blank
            j = b - 1
            while j > a and not lines[j].strip():
                j -= 1
            add_after.setdefault(j, []).extend(new_keys)
        else:
            new_groups.append((g, new_keys))
    out = []
    if -1 in add_after:
        out.extend(add_after[-1])
    for i, line in enumerate(lines):
        if i not in drop:
            out.append(replace.get(i, line))
        if i in add_after:
            out.extend(add_after[i])
    for g, keys in new_groups:
        if g == DEFAULT_GROUP:
            out = keys + out
            continue
        while out and not out[-1].strip():
            out.pop()
        if out:
            out.append('')
        out.append('[%s]' % g)
        out.extend(keys)
    while out and not out[-1].strip() and lines and lines[-1].strip():
        out.pop()
    return '\n'.join(out) + ('\n' if out else '')


@contextlib.contextmanager
def kde_lock(path, wait=3.0):
    """The QLockFile KConfig takes while it writes PATH (PATH.lock: pid, app name, host). A lock left by a program
    that's gone is taken over; one that's held longer than WAIT is waited out and then left alone (we write anyway:
    our write is atomic and only changes our keys)."""
    lock = path + '.lock'
    got = False
    end = time.time() + wait
    while True:
        try:
            fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
            with os.fdopen(fd, 'w') as f:
                f.write('%d\nhearth-kwin-settings\n%s\n' % (os.getpid(), socket.gethostname()))
            got = True
            break
        except FileExistsError:
            try:
                with open(lock) as f:
                    pid = int((f.readline() or '0').strip() or 0)
            except (OSError, ValueError):
                pid = 0
            if pid and not _alive(pid):
                with contextlib.suppress(OSError):
                    os.unlink(lock)
                continue
            if time.time() > end:
                break
            time.sleep(0.05)
        except OSError:
            break
    try:
        yield
    finally:
        if got:
            with contextlib.suppress(OSError):
                os.unlink(lock)


def _alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def update(path, changes, remove_groups=()):
    """Makes CHANGES in the file at PATH (see merge_text) under Hearth's lock and KDE's, atomically, with PATH.bak.
    Returns True when the file changed. A file that can't be read is left as it is (BadFile)."""
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    changed = []

    def change(text):
        if text and any('\udc80' <= ch <= '\udcff' for ch in text):
            raise BadFile('%s is not UTF-8 text' % os.path.basename(path))
        new = merge_text(text, changes, remove_groups)
        changed.append(new != text)
        return new

    with kde_lock(path):
        hsio.update_text(path, change)
    return bool(changed and changed[0])
