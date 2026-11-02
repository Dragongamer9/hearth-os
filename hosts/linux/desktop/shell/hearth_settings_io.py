#!/usr/bin/python3
"""hearth_settings_io: the one safe way Hearth OS reads and writes a settings file (docs/SETTINGS-PERSISTENCE.md).

The rule: settings are always remembered and never change by themselves.
  * Writes are atomic: a temporary file in the same folder, flushed to the drive (fsync), then renamed over the old
    one. A crash or power cut leaves the old file or the new one, never half a file.
  * The last good version is kept as FILE.bak.
  * A file that can't be read (empty, cut short, not JSON) is restored from FILE.bak; with no good .bak it is kept
    as FILE.broken-<time>. Defaults are never saved over it. The person is told once (a notification and
    ~/.local/state/nimbus/settings-events.log, which Send logs includes).
  * Merge on write: the latest file is read under a lock, only the changed keys are changed, then it's written.
    Two programs changing different keys at the same moment both keep their change.
  * Defaults fill in only keys that are missing; a value that's there is never replaced.

Used from Python (import it: the shell's helpers sit next to it, /usr/share/nimbus/desktop/shell), from QML
(SettingsWriter.qml runs this file) and from shell scripts:

  hearth_settings_io.py get FILE                  print the JSON object (restored from .bak if needed; {} if none)
  hearth_settings_io.py merge FILE JSON           set these top-level keys (null removes a key), keep the rest
  hearth_settings_io.py repair FILE               restore a damaged FILE from FILE.bak (prints what happened)
  hearth_settings_io.py conf-set FILE KEY VALUE   KEY=VALUE in a shell-style file, keeping every other line
  hearth_settings_io.py conf-strip FILE KEY REGEX remove REGEX matches from KEY's line only (session.conf's mode)
  hearth_settings_io.py write FILE < TEXT         replace FILE atomically (with .bak)
  hearth_settings_io.py notice ID TEXT            tell the person once (deduplicated by ID)

Python standard library only.
"""
import contextlib
import fcntl
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

_MISSING = object()


def state_dir():
    return os.path.join(os.environ.get('XDG_STATE_HOME') or os.path.join(os.path.expanduser('~'), '.local', 'state'), 'nimbus')


def log_event(text):
    """One line in ~/.local/state/nimbus/settings-events.log (and stderr, so the journal has it)."""
    line = '%s %s' % (time.strftime('%Y-%m-%d %H:%M:%S'), text)
    try:
        print('hearth-settings: ' + text, file=sys.stderr)
    except Exception:
        pass
    try:
        os.makedirs(state_dir(), exist_ok=True)
        with open(os.path.join(state_dir(), 'settings-events.log'), 'a', encoding='utf-8') as f:
            f.write(line + '\n')
    except OSError:
        pass


def notice(nid, text, title='Hearth settings'):
    """Tells the person once per id: a desktop notification when one can be shown, always the events log."""
    d = os.path.join(state_dir(), 'settings-notified')
    mark = os.path.join(d, hashlib.sha1(nid.encode('utf-8', 'surrogateescape')).hexdigest()[:16])
    if os.path.exists(mark):
        return False
    log_event('notice: ' + text)
    try:
        os.makedirs(d, exist_ok=True)
        with open(mark, 'w') as f:
            f.write(text + '\n')
    except OSError:
        pass
    if shutil.which('notify-send') and (os.environ.get('DBUS_SESSION_BUS_ADDRESS') or os.environ.get('XDG_RUNTIME_DIR')):
        try:
            subprocess.Popen(['notify-send', '-a', 'Hearth', title, text], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except OSError:
            pass
    return True


@contextlib.contextmanager
def locked(path):
    """An exclusive lock for read-change-write of PATH (a .NAME.lock file beside it), shared with Rust and QML."""
    d = os.path.dirname(os.path.abspath(path))
    os.makedirs(d, exist_ok=True)
    lf = os.path.join(d, '.' + os.path.basename(path) + '.lock')
    fd = os.open(lf, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        yield
    finally:
        try:
            fcntl.flock(fd, fcntl.LOCK_UN)
        finally:
            os.close(fd)


def _fsync_dir(d):
    try:
        fd = os.open(d, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    except OSError:
        pass


def _atomic_bytes(path, data, mode=None):
    d = os.path.dirname(os.path.abspath(path))
    os.makedirs(d, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix='.' + os.path.basename(path) + '.', suffix='.tmp', dir=d)
    try:
        with os.fdopen(fd, 'wb') as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        if mode is not None:
            os.chmod(tmp, mode)
        else:
            try:
                os.chmod(tmp, os.stat(path).st_mode & 0o7777)
            except OSError:
                os.chmod(tmp, 0o644)
        os.replace(tmp, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp)
        raise
    _fsync_dir(d)


def _valid_json_object(data):
    try:
        return isinstance(json.loads(data.decode('utf-8')), dict)
    except (ValueError, UnicodeDecodeError):
        return False


def _valid_text(data):
    try:
        data.decode('utf-8')
    except UnicodeDecodeError:
        return False
    return len(data.strip()) > 0


def write_bytes(path, data, valid=None, mode=None):
    """Atomic write with FILE.bak: the current file goes to .bak first when it is good (VALID says so)."""
    valid = valid or (lambda b: len(b.strip()) > 0)
    try:
        with open(path, 'rb') as f:
            cur = f.read()
    except OSError:
        cur = None
    if cur is not None and cur == data:
        return False
    if cur is not None and valid(cur):
        _atomic_bytes(path + '.bak', cur, mode=0o600)
    _atomic_bytes(path, data, mode=mode)
    return True


def write_text(path, text, mode=None):
    return write_bytes(path, text.encode('utf-8', 'surrogateescape'), _valid_text, mode)


def write_json(path, obj, indent=2):
    text = json.dumps(obj, indent=indent, ensure_ascii=False) + '\n'
    return write_bytes(path, text.encode('utf-8'), _valid_json_object)


# Kept for helpers that had their own write_atomic(path, text): same call, now with fsync, .bak and a unique temp file.
def write_atomic(path, text):
    if isinstance(text, bytes):
        return write_bytes(path, text)
    return write_text(path, text)


def _keep_broken(path, raw):
    dest = '%s.broken-%s' % (path, time.strftime('%Y%m%d-%H%M%S'))
    with contextlib.suppress(OSError):
        _atomic_bytes(dest, raw, mode=0o600)
    return dest


def load_json(path, default=_MISSING, repair=True):
    """The JSON object in PATH. Missing: DEFAULT (or {}). Damaged: restored from PATH.bak (and written back, with the
    damaged copy kept), or, with no good .bak, the damaged file is kept as PATH.broken-<time>, the person is told,
    and DEFAULT is returned. Never writes defaults into PATH."""
    fallback = {} if default is _MISSING else default
    try:
        with open(path, 'rb') as f:
            raw = f.read()
    except FileNotFoundError:
        return _copy(fallback)
    except OSError as e:
        log_event('could not read %s: %s (nothing changed)' % (path, e))
        return _copy(fallback)
    if _valid_json_object(raw):
        return json.loads(raw.decode('utf-8'))
    if not repair:
        return _copy(fallback)
    with locked(path):
        return _repair_locked(path, raw, fallback)


def _repair_locked(path, raw, fallback):
    # Someone may have fixed it meanwhile.
    with contextlib.suppress(OSError):
        with open(path, 'rb') as f:
            raw = f.read()
    if _valid_json_object(raw):
        return json.loads(raw.decode('utf-8'))
    name = os.path.basename(path)
    try:
        with open(path + '.bak', 'rb') as f:
            bak = f.read()
    except OSError:
        bak = None
    if bak is not None and _valid_json_object(bak):
        broken = _keep_broken(path, raw) if raw.strip() else None
        _atomic_bytes(path, bak)
        notice('restored:%s:%s' % (path, hashlib.sha1(raw).hexdigest()),
               'Your %s settings were damaged (probably a power cut) and have been put back from the last good copy.' % name)
        log_event('restored %s from %s.bak%s' % (path, name, ' (damaged copy kept as %s)' % broken if broken else ''))
        return json.loads(bak.decode('utf-8'))
    broken = _keep_broken(path, raw)
    notice('broken:%s:%s' % (path, hashlib.sha1(raw).hexdigest()),
           "Your %s settings couldn't be read and there was no good copy to put back. The file is kept as %s; "
           'Hearth uses its defaults until you change something.' % (name, os.path.basename(broken)))
    return _copy(fallback)


def _copy(v):
    return json.loads(json.dumps(v))


def with_defaults(data, defaults):
    """DATA with DEFAULTS filled in for keys that are missing only (nested dicts too). Present values stay."""
    out = dict(data) if isinstance(data, dict) else {}
    for k, v in defaults.items():
        if k not in out:
            out[k] = _copy(v)
        elif isinstance(v, dict) and isinstance(out[k], dict):
            out[k] = with_defaults(out[k], v)
    return out


def update_json(path, change, indent=2):
    """Read the latest PATH under the lock, CHANGE(dict) in place (or return a new dict), write it if it changed."""
    with locked(path):
        try:
            with open(path, 'rb') as f:
                raw = f.read()
        except FileNotFoundError:
            raw = None
        if raw is None:
            data = {}
        elif _valid_json_object(raw):
            data = json.loads(raw.decode('utf-8'))
        else:
            data = _repair_locked(path, raw, {})
        before = json.dumps(data, sort_keys=True)
        r = change(data)
        if isinstance(r, dict):
            data = r
        if raw is None or json.dumps(data, sort_keys=True) != before:
            write_json(path, data, indent)
        return data


def merge_json(path, changes):
    """Set CHANGES' top-level keys (None removes one); every other key stays as it is on disk."""
    def apply(d):
        for k, v in changes.items():
            if v is None:
                d.pop(k, None)
            else:
                d[k] = v
    return update_json(path, apply)


# ------------------------------------------------------------------ shell-style KEY=VALUE files (session.conf)

def _conf_key_re(key):
    return re.compile(r'^[ \t]*(export[ \t]+)?' + re.escape(key) + r'=')


def conf_set_text(text, key, value):
    """TEXT with KEY=VALUE: the last KEY= line replaced (the shell takes the last), or added at the end."""
    rx = _conf_key_re(key)
    line = '%s=%s' % (key, value)
    lines = text.splitlines()
    idx = [i for i, l in enumerate(lines) if rx.match(l)]
    if idx:
        lines[idx[-1]] = line
    else:
        lines.append(line)
    return '\n'.join(lines) + '\n'


def conf_strip_text(text, key, pattern):
    """TEXT with PATTERN's matches removed from KEY's lines only (group 1 is kept, as in the session's sed)."""
    rx = _conf_key_re(key)
    prx = re.compile(pattern)
    out = []
    for l in text.splitlines(True):
        out.append(prx.sub(lambda m: m.group(1) or '', l) if rx.match(l) else l)
    return ''.join(out)


def update_text(path, change):
    with locked(path):
        try:
            with open(path, encoding='utf-8', errors='surrogateescape') as f:
                text = f.read()
        except FileNotFoundError:
            text = ''
        new = change(text)
        if new != text:
            write_text(path, new)
        return new


def main(argv):
    if len(argv) >= 3 and argv[1] == 'get':
        print(json.dumps(load_json(argv[2], {}), ensure_ascii=False))
        return 0
    if len(argv) == 4 and argv[1] == 'merge':
        changes = json.loads(argv[3])
        if not isinstance(changes, dict):
            print('error: merge takes a JSON object', file=sys.stderr)
            return 2
        merge_json(argv[2], changes)
        return 0
    if len(argv) == 3 and argv[1] == 'repair':
        if not os.path.exists(argv[2]):
            print('missing')
            return 0
        load_json(argv[2], {})
        print('ok')
        return 0
    if len(argv) == 5 and argv[1] == 'conf-set':
        update_text(argv[2], lambda t: conf_set_text(t, argv[3], argv[4]))
        return 0
    if len(argv) == 5 and argv[1] == 'conf-strip':
        if os.path.exists(argv[2]):
            update_text(argv[2], lambda t: conf_strip_text(t, argv[3], argv[4]))
        return 0
    if len(argv) == 3 and argv[1] == 'write':
        data = sys.stdin.buffer.read()
        with locked(argv[2]):
            write_bytes(argv[2], data)
        return 0
    if len(argv) == 4 and argv[1] == 'notice':
        notice(argv[2], argv[3])
        return 0
    print(__doc__.strip(), file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv))
