"""What the server's parts share (docs/HOME-SERVER-PARTS.md): where their choices and state go, the safe writers,
"is a game running?", notifications to the desktop and the phone, the keyring, small HTTP, and the answer line.

The engines live next to the Home server helper (os/system_files/usr/libexec/nimbus/hearth-server-parts), so they come
and update with its Store system part ("All this has to be updated thru the store"): root-owned files, run as you.
Each engine is started with `python3 -I`, so nothing from the folder you're in or your environment's Python paths is
ever imported; this file is loaded by its full path (load_common in each engine).

Choices (red flag 9): one file per part, ~/.config/nimbus/server-<part>.json, through Hearth's own hearth_settings_io
(atomic, .bak, merged under a lock, never defaults over a file that couldn't be read); only a key you changed is
written, and opening anything writes nothing. What a part did last is state, not a setting:
~/.local/state/hearth/server/<part>.json. Nothing secret is ever in either: keys are in the system's password store
(secret-tool), read only when they're needed, never printed or logged.
"""
import fcntl
import importlib.machinery
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.realpath(__file__))
HOME = os.path.expanduser('~')
CONFIG = os.environ.get('XDG_CONFIG_HOME') or os.path.join(HOME, '.config')
STATE_HOME = os.environ.get('XDG_STATE_HOME') or os.path.join(HOME, '.local', 'state')
DATA_HOME = os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local', 'share')
CACHE_HOME = os.environ.get('XDG_CACHE_HOME') or os.path.join(HOME, '.cache')
RUN = os.environ.get('XDG_RUNTIME_DIR') or os.path.join('/tmp', 'hearth-%d' % os.getuid())


class Fail(Exception):
    pass


def out(obj):
    print(json.dumps(obj, ensure_ascii=False), flush=True)


def main_guard(fn):
    """Runs fn(argv) and prints its answer as one JSON line; a Fail is {"ok": false, "error": ...} (exit 1)."""
    try:
        r = fn(sys.argv[1:])
        if r is not None:
            out(r)
        return 0
    except Fail as e:
        out({'ok': False, 'error': str(e)})
        return 1


# ------------------------------------------------------------------ the safe writers (red flag 9)
def _settings_io():
    """Hearth's own hearth_settings_io (Mantel's folder, or the repository for the tests), else None."""
    for d in (os.environ.get('HEARTH_SHELL_DIR'), '/usr/share/nimbus/desktop/shell',
              os.path.join(HERE, '..', '..', '..', '..', '..', 'hosts', 'linux', 'desktop', 'shell')):
        if not d:
            continue
        p = os.path.join(d, 'hearth_settings_io.py')
        if os.path.isfile(p):
            loader = importlib.machinery.SourceFileLoader('hearth_settings_io', p)
            spec = importlib.util.spec_from_loader('hearth_settings_io', loader)
            m = importlib.util.module_from_spec(spec)
            loader.exec_module(m)
            return m
    return None


SIO = _settings_io()


def _atomic(path, text, mode=0o644):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=os.path.dirname(path), prefix='.' + os.path.basename(path) + '.', suffix='.tmp')
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as f:
            f.write(text)
            f.flush()
            os.fsync(f.fileno())
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _read_obj(path):
    try:
        with open(path, encoding='utf-8') as f:
            d = json.load(f)
        return d if isinstance(d, dict) else None
    except FileNotFoundError:
        return {}
    except (OSError, ValueError):
        return None


def settings_path(part):
    return os.path.join(CONFIG, 'nimbus', 'server-%s.json' % part)


def settings(part, defaults):
    """Your choices with the defaults filled in where a key is missing (never saved). A file that can't be read is put
    back from its .bak by hearth_settings_io; without it the defaults are used and nothing is written."""
    p = settings_path(part)
    if SIO is not None:
        try:
            d = SIO.load_json(p, {})
        except Exception:  # noqa: BLE001 - a broken file never stops the page
            d = {}
    else:
        d = _read_obj(p)
        if d is None:
            bak = _read_obj(p + '.bak')
            d = bak if bak else {}
    d = d if isinstance(d, dict) else {}
    outd = json.loads(json.dumps(defaults))
    for k, v in d.items():
        if isinstance(v, dict) and isinstance(outd.get(k), dict):
            outd[k].update(v)
        else:
            outd[k] = v
    return outd


def set_key(part, key, value):
    """You changed one: only that key is written (merged under the lock, atomic, .bak). Nothing when it's already so."""
    p = settings_path(part)
    if SIO is not None:
        def change(d):
            if d.get(key) == value:
                return d
            d[key] = value
            return d
        SIO.update_json(p, change)
        return
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(os.path.join(os.path.dirname(p), '.' + os.path.basename(p) + '.lock'), 'w') as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        cur = _read_obj(p)
        if cur is None:
            raise Fail('Your server settings file couldn\'t be read, so nothing was changed (it\'s kept as it is)')
        if cur.get(key) == value:
            return
        if os.path.exists(p):
            shutil.copy2(p, p + '.bak')
        cur[key] = value
        _atomic(p, json.dumps(cur, indent=2, ensure_ascii=False) + '\n')


def state_path(part):
    return os.path.join(STATE_HOME, 'hearth', 'server', '%s.json' % part)


def state(part):
    d = _read_obj(state_path(part))
    return d if isinstance(d, dict) else {}


def state_write(part, data):
    _atomic(state_path(part), json.dumps(data, indent=1, ensure_ascii=False) + '\n', 0o600)


def state_update(part, change):
    p = state_path(part)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(os.path.join(os.path.dirname(p), '.%s.lock' % part), 'w') as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        d = state(part)
        d = change(d) or d
        state_write(part, d)
        return d


# ------------------------------------------------------------------ what happened (Earlier today, alerts)
def event(part, text, tone='ok', at=None):
    """One line in the server's log (~/.local/state/hearth/server/events.jsonl), kept to the last 400."""
    p = os.path.join(STATE_HOME, 'hearth', 'server', 'events.jsonl')
    os.makedirs(os.path.dirname(p), exist_ok=True)
    line = json.dumps({'at': int(at or time.time()), 'part': part, 'text': text, 'tone': tone}, ensure_ascii=False)
    with open(p + '.lock', 'w') as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        try:
            with open(p, encoding='utf-8') as f:
                lines = f.read().splitlines()[-399:]
        except OSError:
            lines = []
        lines.append(line)
        _atomic(p, '\n'.join(lines) + '\n', 0o600)


def events(since=0):
    p = os.path.join(STATE_HOME, 'hearth', 'server', 'events.jsonl')
    outl = []
    try:
        with open(p, encoding='utf-8') as f:
            for l in f:
                try:
                    e = json.loads(l)
                except ValueError:
                    continue
                if isinstance(e, dict) and int(e.get('at', 0)) >= since:
                    outl.append(e)
    except OSError:
        pass
    return outl


def notify(text, title='Home server', urgent=False):
    """On this desktop (notify-send), quietly nothing where there's none."""
    if not shutil.which('notify-send'):
        return False
    try:
        r = subprocess.run(['notify-send', '-a', 'Hearth', '-u', 'critical' if urgent else 'normal', title, text],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
        return r.returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


def phone(text, title='Hearth home server', priority='default'):
    """To your phone through ntfy, when you turned Phone notifications on (Settings › Home server: server-apps.json's
    "ntfy", read only here). Nothing is sent otherwise."""
    cfg = _read_obj(os.path.join(CONFIG, 'nimbus', 'server-apps.json')) or {}
    n = cfg.get('ntfy') if isinstance(cfg.get('ntfy'), dict) else {}
    if not n.get('on') or not n.get('topic'):
        return False
    server = str(n.get('server') or 'https://ntfy.sh').rstrip('/')
    if not re.fullmatch(r'https?://[A-Za-z0-9.\-:\[\]]+', server) or not re.fullmatch(r'[A-Za-z0-9_\-]{1,64}', str(n['topic'])):
        return False
    try:
        req = urllib.request.Request('%s/%s' % (server, n['topic']), data=text.encode('utf-8'), method='POST',
                                     headers={'Title': title.encode('ascii', 'ignore').decode(), 'Priority': priority, 'Tags': 'house'})
        urllib.request.urlopen(req, timeout=15).read()
        return True
    except (OSError, urllib.error.URLError, ValueError):
        return False


# ------------------------------------------------------------------ playing, streaming, at night
def playing():
    """Game Mode or a game is running (gamescope, or Hearth's hearth-playing mark), or someone's streaming a game from
    this PC (Sunshine's session mark)."""
    if os.environ.get('HEARTH_TEST_PLAYING') is not None:
        return os.environ['HEARTH_TEST_PLAYING'] == '1'
    for mark in ('hearth-playing', 'hearth-streaming'):
        if os.path.exists(os.path.join(RUN, mark)):
            return True
    try:
        for pid in os.listdir('/proc'):
            if pid.isdigit():
                try:
                    with open('/proc/%s/comm' % pid) as f:
                        if f.read().strip() in ('gamescope', 'gamescope-wl'):
                            return True
                except OSError:
                    pass
    except OSError:
        pass
    return False


def at_night(now=None, start=1, end=7):
    h = time.localtime(now or time.time()).tm_hour
    return start <= h < end


def low_priority():
    """This process at the lowest CPU and disk priority (the slow background work)."""
    try:
        os.nice(19)
    except OSError:
        pass
    if shutil.which('ionice'):
        subprocess.run(['ionice', '-c', '3', '-p', str(os.getpid())], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


# ------------------------------------------------------------------ the keyring
def secret(key):
    """A key from the system's password store (secret-tool, as Hearth's apps keep theirs: hearth-app home-server key
    <key>), or "". Never printed, never logged, never on a command line."""
    if os.environ.get('HEARTH_TEST_SECRETS'):
        return (json.loads(os.environ['HEARTH_TEST_SECRETS']) or {}).get(key, '')
    if not shutil.which('secret-tool'):
        return ''
    try:
        r = subprocess.run(['secret-tool', 'lookup', 'hearth-app', 'home-server', 'key', key], capture_output=True, text=True, timeout=15)
        return r.stdout.rstrip('\n') if r.returncode == 0 else ''
    except (OSError, subprocess.TimeoutExpired):
        return ''


def has_secret(key):
    return bool(secret(key))


# ------------------------------------------------------------------ small HTTP
def http(method, url, body=None, headers=None, timeout=10):
    """(status, bytes) — status 0 when it couldn't be reached."""
    try:
        req = urllib.request.Request(url, data=body, method=method, headers=dict({'User-Agent': 'Hearth-home-server'}, **(headers or {})))
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        try:
            return e.code, e.read()
        except OSError:
            return e.code, b''
    except (OSError, urllib.error.URLError, ValueError):
        return 0, b''


def http_json(method, url, body=None, headers=None, timeout=10):
    st, b = http(method, url, json.dumps(body).encode() if body is not None else None, dict({'Content-Type': 'application/json'} if body is not None else {}, **(headers or {})), timeout)
    try:
        return st, json.loads(b.decode('utf-8')) if b else None
    except ValueError:
        return st, None


# ------------------------------------------------------------------ words
def size_words(b):
    b = float(b or 0)
    for unit, n in (('TB', 1e12), ('GB', 1e9), ('MB', 1e6), ('KB', 1e3)):
        if b >= n:
            v = b / n
            return ('%.2f %s' % (v, unit)) if unit == 'TB' and v < 10 else (('%.1f %s' % (v, unit)) if v < 100 and unit != 'KB' else '{:,.0f} {}'.format(v, unit))
    return '%d bytes' % int(b)


def gb_words(gb):
    """1,560 GB; 21.7 GB; 0.3 GB (the vault's table)."""
    if gb >= 1000:
        return '{:,.0f} GB'.format(gb)
    return '%.1f GB' % gb


def tb_words(gb):
    """4.75 TB, 1.25 TB, ≈ 200 GB."""
    if gb >= 1000:
        return '%.2f TB' % (gb / 1000.0)
    return '%d GB' % round(gb)


def money(x):
    return '$%.2f' % x


def clock(t):
    lt = time.localtime(t)
    h = lt.tm_hour % 12 or 12
    return '%d:%02d %s' % (h, lt.tm_min, 'AM' if lt.tm_hour < 12 else 'PM')


MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']


def day(t):
    lt = time.localtime(t)
    return '%s %d' % (MONTHS[lt.tm_mon - 1], lt.tm_mday)


def day_long(t):
    lt = time.localtime(t)
    return '%s %s %d' % (DAYS[lt.tm_wday], MONTHS[lt.tm_mon - 1], lt.tm_mday)


def duration_words(secs):
    secs = int(max(0, secs))
    d, rem = divmod(secs, 86400)
    h, rem = divmod(rem, 3600)
    m = rem // 60
    if d:
        return '%d day%s %d hour%s' % (d, '' if d == 1 else 's', h, '' if h == 1 else 's')
    if h:
        return '%d h %d min' % (h, m)
    return '%d min' % max(1, m)


def which(name):
    return shutil.which(name)


def run(cmd, timeout=60, input_text=None, env=None):
    """(code, stdout) — never through a shell."""
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, input=input_text, env=env)
        return r.returncode, r.stdout
    except (OSError, subprocess.TimeoutExpired):
        return 1, ''
