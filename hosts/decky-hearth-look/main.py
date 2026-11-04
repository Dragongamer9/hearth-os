"""Hearth Classic's backend (docs/HEARTH-LOOK.md): the old Hearth Game Mode look on top of Steam's Game Mode.

Runs as you inside Decky Loader. It keeps the plugin's settings (settings.json in the plugin's Decky settings folder:
atomic writes, the last good copy as settings.json.bak, merge-on-write, a damaged file never replaced with defaults;
docs/SETTINGS-PERSISTENCE.md), hands the frontend Hearth's skin and pictures, Hearth's home layout and Hearth Sync's
shortcuts, and pictures from this device. Only on Hearth OS (hosts/decky-shared/hearth_only.py): anywhere else nothing
starts and every call answers "not Hearth OS".

  nimbus --steam-mode-data home   Hearth's home layout (hosts/hearth-core steam_takeover.rs), asked at most once a
                                  minute; nothing keeps running.

Nothing is sent anywhere and nothing listens on the network. The settings only ever change when the person changes them
in the panel; the plugin never writes them by itself.
"""
import asyncio
import base64
import json
import os
import shutil
import subprocess
import sys
import time
import re
import urllib.parse
import urllib.request

try:
    import decky
except ImportError:  # tests
    decky = None

# Hearth OS only, and the load mark for plugin updates (hosts/decky-shared/hearth_only.py): next to main.py when packed,
# in hosts/decky-shared in the repo.
_here = os.path.dirname(os.path.abspath(__file__))
for _p in (os.path.join(_here, '..', 'decky-shared'), _here):
    if _p not in sys.path:
        sys.path.insert(0, _p)
import hearth_only  # noqa: E402

NAME = 'Hearth Classic'
DEFAULTS = {'look': 'steam', 'home': True, 'sounds': False, 'wallpaper': True, 'menu': True, 'menuTop': True, 'autoSend': False}
HOME_TIMEOUT = 20
HOME_EVERY = 60
ART_LIMIT = 8 * 1024 * 1024
NOT_HEARTH = {'ok': False, 'error': 'not Hearth OS'}


def log(msg):
    if decky:
        try:
            decky.logger.info(msg)
        except Exception:
            pass


def home():
    if decky and getattr(decky, 'DECKY_USER_HOME', None):
        return decky.DECKY_USER_HOME
    return os.environ.get('HOME') or os.path.expanduser('~')


def settings_dir():
    d = getattr(decky, 'DECKY_PLUGIN_SETTINGS_DIR', None) if decky else None
    return d or os.path.join(home(), '.config', 'decky-hearth-look')


def plugin_dir():
    d = getattr(decky, 'DECKY_PLUGIN_DIR', None) if decky else None
    return d or _here


# ---- settings the safe way (the same rules and file names as Hearth OS's hearth_settings_io.py and Hearth Sync's)

def _safe_write(path, data):
    """Atomic: a temporary file, fsync, rename; the previous good file kept as PATH.bak. Under a lock."""
    import fcntl
    import tempfile
    d = os.path.dirname(path)
    os.makedirs(d, exist_ok=True)
    lock = os.open(os.path.join(d, '.' + os.path.basename(path) + '.lock'), os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            with open(path, 'rb') as f:
                cur = f.read()
        except OSError:
            cur = None
        if cur == data:
            return
        good_cur = None
        if cur is not None:
            try:
                json.loads(cur)
                good_cur = cur
            except ValueError:
                good_cur = None   # a damaged file never becomes the .bak (the last good copy stays there)
        for target, content in ((path + '.bak', good_cur), (path, data)):
            if content is None or not content.strip():
                continue
            fd, tmp = tempfile.mkstemp(prefix='.' + os.path.basename(target) + '.', suffix='.tmp', dir=d)
            try:
                with os.fdopen(fd, 'wb') as f:
                    f.write(content)
                    f.flush()
                    os.fsync(f.fileno())
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
    finally:
        os.close(lock)


def clean_settings(raw):
    """Only known keys with the right types; anything else is the default (look: "steam", i.e. off)."""
    r = raw if isinstance(raw, dict) else {}
    out = dict(DEFAULTS)
    if r.get('look') in ('steam', 'classic'):
        out['look'] = r['look']
    for k in ('home', 'sounds', 'wallpaper', 'menu', 'menuTop', 'autoSend'):
        if isinstance(r.get(k), bool):
            out[k] = r[k]
    return out


def read_settings():
    """(settings, state): state is 'ok', 'new' (no file yet: the defaults, nothing written), 'restored' (the file was
    damaged and settings.json.bak was put back) or 'damaged' (neither could be read: the defaults are used but the
    damaged file is left alone, never overwritten with defaults)."""
    p = os.path.join(settings_dir(), 'settings.json')
    exists = os.path.exists(p)
    for i, path in enumerate((p, p + '.bak')):
        try:
            with open(path) as f:
                v = json.load(f)
        except (OSError, ValueError):
            continue
        if not isinstance(v, dict):
            continue
        if i == 1:
            try:
                _safe_write(p, json.dumps(v).encode())
                log('settings.json was damaged; put back the last good copy')
            except OSError:
                pass
            return clean_settings(v), 'restored'
        return clean_settings(v), 'ok'
    return dict(DEFAULTS), ('damaged' if exists else 'new')


def write_settings(change):
    """Merges the person's change into what's saved (never into defaults when the file is damaged: the damaged file is
    kept as settings.json.damaged first) and saves it. Returns the saved settings."""
    if not isinstance(change, dict):
        raise ValueError('not a settings change')
    cur, state = read_settings()
    p = os.path.join(settings_dir(), 'settings.json')
    if state == 'damaged':
        try:
            shutil.copyfile(p, p + '.damaged')
        except OSError:
            pass
    # The file may hold keys a newer version wrote: they're kept (merge-on-write).
    try:
        with open(p) as f:
            raw = json.load(f)
    except (OSError, ValueError):
        raw = None
    # Only what's saved plus the person's change: the defaults are never written into the file.
    merged = dict(raw) if isinstance(raw, dict) else {}
    for k, v in change.items():
        if k not in DEFAULTS:
            continue
        if clean_settings({k: v})[k] != v or type(v) is not type(DEFAULTS[k]):
            raise ValueError('not a valid value for %s' % k)
        merged[k] = v
    _safe_write(p, json.dumps(merged, indent=1, sort_keys=True).encode())
    return clean_settings(merged)


# ---- Hearth's skin and pictures

def data_url(path, limit=ART_LIMIT):
    try:
        if os.path.getsize(path) > limit:
            return ''
        with open(path, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    mime = kind_of(data) or ({'.svg': 'image/svg+xml'}.get(os.path.splitext(path)[1].lower()) if b'<svg' in data[:2048] else None)
    return 'data:%s;base64,%s' % (mime, base64.b64encode(data).decode()) if mime else ''


def kind_of(data):
    if data[:8] == b'\x89PNG\r\n\x1a\n':
        return 'image/png'
    if data[:3] == b'\xff\xd8\xff':
        return 'image/jpeg'
    if data[:4] == b'RIFF' and data[8:12] == b'WEBP':
        return 'image/webp'
    if data[:6] in (b'GIF87a', b'GIF89a'):
        return 'image/gif'
    return None


def brand(pdir=None):
    """The skin's CSS and Hearth's logo and wallpaper (data: addresses), from the plugin's own folder."""
    pdir = pdir or plugin_dir()
    css = ''
    for p in (os.path.join(pdir, 'hearth-classic.css'), os.path.join(pdir, 'defaults', 'hearth-classic.css')):
        try:
            with open(p) as f:
                css = f.read()
            break
        except OSError:
            continue
    wall = os.path.join(pdir, 'assets', 'wallpaper.jpg')
    if not os.path.isfile(wall):
        wall = '/usr/share/backgrounds/hearth/hearth-embers.jpg'
    return {'css': css, 'logo': data_url(os.path.join(pdir, 'assets', 'logo.svg')), 'wallpaper': data_url(wall)}


def art_roots(home_dir=None):
    """Where pictures may be read from: Hearth's library and art cache, Steam's library cache and its custom pictures."""
    h = home_dir or home()
    out = [os.path.join(h, '.local', 'share', 'nimbus')]
    for steam in (os.path.join(h, '.local', 'share', 'Steam'), os.path.join(h, '.steam', 'steam'),
                  os.path.join(h, '.var', 'app', 'com.valvesoftware.Steam', '.local', 'share', 'Steam')):
        out += [os.path.join(steam, 'appcache', 'librarycache'), os.path.join(steam, 'userdata')]
    return [os.path.realpath(r) for r in out]


def read_art(ref, home_dir=None):
    """A picture on this device as a data: address ('' when it isn't one, is too big or is outside art_roots)."""
    if not isinstance(ref, str) or not ref or len(ref) > 4096:
        return ''
    path = ref[7:] if ref.startswith('file://') else ref
    if not path.startswith('/'):
        return ''
    real = os.path.realpath(path)
    if not any(real == r or real.startswith(r + os.sep) for r in art_roots(home_dir)):
        return ''
    try:
        if not os.path.isfile(real) or os.path.getsize(real) > ART_LIMIT:
            return ''
        with open(real, 'rb') as f:
            data = f.read()
    except OSError:
        return ''
    mime = kind_of(data)
    return 'data:%s;base64,%s' % (mime, base64.b64encode(data).decode()) if mime else ''


# ---- Hearth's home layout and Hearth Sync's shortcuts

def find_program(name):
    for c in ('/usr/bin/' + name, '/usr/local/bin/' + name, shutil.which(name)):
        if c and os.path.isfile(c) and os.access(c, os.X_OK):
            return c
    return None


def clean_env():
    env = dict(os.environ)
    orig = env.pop('LD_LIBRARY_PATH_ORIG', None)
    env.pop('LD_LIBRARY_PATH', None)
    if orig:
        env['LD_LIBRARY_PATH'] = orig
    env['HOME'] = home()
    env.setdefault('PATH', '/usr/local/bin:/usr/bin:/bin')
    return env


def sync_shortcuts():
    """Hearth Sync's shortcuts (Hearth id -> Steam app id), read only, from its Decky settings folder."""
    root = os.path.dirname(settings_dir().rstrip('/'))
    for name in ('decky-hearth-sync', 'Hearth Sync'):
        try:
            with open(os.path.join(root, name, 'shortcuts.json')) as f:
                m = json.load(f)
        except (OSError, ValueError):
            continue
        if isinstance(m, dict):
            return {k: v for k, v in m.items() if isinstance(k, str) and isinstance(v, int) and not isinstance(v, bool) and 0 < v < 2 ** 32}
    return {}


HOME_CACHE = {'at': 0.0, 'result': None}


def _nice():
    try:
        os.nice(10)
    except OSError:
        pass


def home_layout(run=subprocess.run, now=None, finder=None):
    """{ok, home, shortcuts} or {ok: False, error}. Asked at most every HOME_EVERY seconds."""
    now = time.time() if now is None else now
    c = HOME_CACHE
    if c['result'] is not None and now - c['at'] < HOME_EVERY:
        return dict(c['result'], shortcuts=sync_shortcuts())
    app = (finder or find_program)('nimbus')
    if not app:
        res = {'ok': False, 'error': "Hearth's program isn't installed"}
    else:
        try:
            r = run([app, '--steam-mode-data', 'home'], capture_output=True, text=True, timeout=HOME_TIMEOUT,
                    env=clean_env(), preexec_fn=_nice)
            lines = (r.stdout or '').strip().splitlines()
            v = json.loads(lines[-1]) if lines else None
            res = {'ok': True, 'home': v} if isinstance(v, dict) and 'error' not in v else \
                  {'ok': False, 'error': (v or {}).get('error', 'no answer') if isinstance(v, dict) else 'no answer'}
        except (OSError, subprocess.TimeoutExpired, ValueError) as e:
            res = {'ok': False, 'error': "Hearth didn't answer (%s)" % type(e).__name__}
    if not res['ok']:
        log('home: %s; the home uses Steam\'s library' % res['error'])
    c.update(at=now, result=res)
    return dict(res, shortcuts=sync_shortcuts())


DIAG_LIMIT = 256 * 1024
DIAG_TEXT = 60000   # the deep diagnostics (DOM outlines) are long


def diag_log_path(home_dir=None):
    """Hearth Classic's log in Hearth's own log folder, which Hearth's Logs app lists (hearth-tools logs.rs)."""
    return os.path.join(home_dir or home(), '.local', 'share', 'nimbus', 'hearth-classic.log')


def write_diag(text, home_dir=None, now=None):
    """Adds the diagnostics (or one line of them) to hearth-classic.log, with the time; the file is kept small (the
    newer half stays when it grows past DIAG_LIMIT). Returns the path, or '' when it couldn't be written."""
    if not isinstance(text, str) or not text.strip():
        return ''
    p = diag_log_path(home_dir)
    stamp = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(now if now is not None else time.time()))
    entry = ''.join('%s %s\n' % (stamp, line) for line in text[:DIAG_TEXT].splitlines() if line.strip())
    try:
        os.makedirs(os.path.dirname(p), exist_ok=True)
        try:
            if os.path.getsize(p) > DIAG_LIMIT:
                with open(p, 'rb') as f:
                    f.seek(-DIAG_LIMIT // 2, os.SEEK_END)
                    keep = f.read()
                keep = keep[keep.find(b'\n') + 1:]
                tmp = p + '.tmp'
                with open(tmp, 'wb') as f:
                    f.write(keep)
                os.replace(tmp, p)
        except OSError:
            pass
        with open(p, 'a', encoding='utf-8') as f:
            f.write(entry)
        return p
    except OSError:
        return ''


# ---- Sending Hearth Classic's diagnostics through Hearth's own "Send logs" service (hearth-core logs.rs send_report:
# POST text to SERVICE/v1/logs; the counter worker, counter/src/index.js, keeps it for 14 days under a code and files it
# as an issue with the full text in the private repo Dragongamer9/hearth-logs). Only Hearth Classic's own diagnostics
# and its log go: personal details are taken out first (home folder, user and Steam account names, web addresses'
# queries, anything that looks like a token).
SEND_SERVICE = 'https://hearth-counter.game305-cl.workers.dev'
SEND_MAX = 400 * 1024
AUTO_PER_DAY = 10
MANUAL_PER_DAY = 20
SENT_THIS_START = set()


def steam_names(home_dir=None):
    """Steam's account and persona names on this device (only to take them out of what's sent)."""
    h = home_dir or home()
    names = set()
    for p in (os.path.join(h, '.local', 'share', 'Steam', 'config', 'loginusers.vdf'), os.path.join(h, '.steam', 'steam', 'config', 'loginusers.vdf')):
        try:
            with open(p, encoding='utf-8', errors='replace') as f:
                for m in re.finditer(r'"(?:AccountName|PersonaName)"\s+"([^"]{2,64})"', f.read(200000)):
                    names.add(m.group(1))
        except OSError:
            continue
    return names


def redact(text, home_dir=None, names=None):
    h = home_dir or home()
    user = os.path.basename(h.rstrip('/'))
    t = text.replace(h, '~')
    t = re.sub(r'(https?://[^\s?#"\']+)\?[^\s"\']*', r'\1?[query left out]', t)
    t = re.sub(r'(?i)\b(token|access_token|refresh_token|key|secret|password|passwd|auth|session|cookie)\b(\s*[=:]\s*)\S+', r'\1\2[left out]', t)
    t = re.sub(r'(?i)\bBearer\s+\S+', 'Bearer [left out]', t)
    t = re.sub(r'\b7656119\d{10}\b', '[steam id]', t)
    t = re.sub(r'[\w.+-]+@[\w-]+\.[\w.]+', '[email]', t)
    for n in sorted((names if names is not None else steam_names(h)) | ({user} if len(user) > 2 else set()), key=len, reverse=True):
        t = re.sub(r'(?<![\w])' + re.escape(n) + r'(?![\w])', '[user]', t)
    return t


def device_name():
    try:
        parts = []
        for p in ('/sys/class/dmi/id/sys_vendor', '/sys/class/dmi/id/product_name'):
            with open(p) as f:
                parts.append(f.read().strip())
        return ' '.join(x for x in parts if x)[:80] or 'unknown device'
    except OSError:
        return 'unknown device'


def send_state_path():
    d = getattr(decky, 'DECKY_PLUGIN_RUNTIME_DIR', None) if decky else None
    return os.path.join(d or settings_dir(), 'sent.json')


def read_send_state():
    try:
        with open(send_state_path()) as f:
            v = json.load(f)
        return v if isinstance(v, dict) else {}
    except (OSError, ValueError):
        return {}


def write_send_state(v):
    """Atomic; only the plugin's own bookkeeping (when it last sent, how many today), not a setting."""
    p = send_state_path()
    try:
        os.makedirs(os.path.dirname(p), exist_ok=True)
        tmp = p + '.tmp'
        with open(tmp, 'w') as f:
            json.dump(v, f)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, p)
    except OSError:
        pass


def send_body(reason, diag_text, version, home_dir=None, names=None, now=None):
    """What's sent: Hearth's log format (the service only takes text starting "Hearth logs"), Hearth Classic only."""
    try:
        with open(diag_log_path(home_dir), encoding='utf-8', errors='replace') as f:
            tail = f.read()[-200000:]
    except OSError:
        tail = '(no hearth-classic.log yet)'
    when = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(now if now is not None else time.time()))
    body = '\n'.join(['Hearth logs (Hearth Classic diagnostics only)', 'Device: ' + device_name(), 'Hearth Classic: ' + version,
                      'Reason: ' + reason, 'When: ' + when, '', '== What Hearth Classic found ==', diag_text or '(none)', '',
                      '== hearth-classic.log (newest last) ==', tail])
    return redact(body, home_dir, names)[-SEND_MAX:] if len(body) > SEND_MAX else redact(body, home_dir, names)


# Decky's Python is a bundled (PyInstaller) one whose OpenSSL looks for certificates where it was built, not where Hearth
# OS keeps them: a plain urlopen failed on the owner's device with an HTTPS error. So the direct send uses the system's
# CA bundle (verification stays on), and if it fails anyway, the report goes through the system's curl with the same
# request Hearth's own sender makes (hearth-core logs.rs send_report runs curl too), with the system's libraries.
CA_FILES = ('/etc/ssl/certs/ca-certificates.crt', '/etc/pki/tls/certs/ca-bundle.crt')


def ssl_context(files=CA_FILES):
    """(context, where its certificates come from): the first system CA bundle that loads, else Python's defaults.
    Verification always stays on."""
    import ssl
    for f in files:
        if os.path.isfile(f):
            try:
                return ssl.create_default_context(cafile=f), f
            except (OSError, ssl.SSLError):
                continue
    p = ssl.get_default_verify_paths()
    return ssl.create_default_context(), 'Python default (%s)' % (p.cafile or p.capath or 'none')


def _headers(note, version, agent=True):
    """The request Hearth's own sender makes. The direct send says who it is (Python's default User-Agent is one the
    service's front door may refuse); curl keeps its own, as Hearth's sender does."""
    h = {'Content-Type': 'text/plain; charset=utf-8', 'X-Hearth-Version': version, 'X-Hearth-Kind': 'peppermint',
         'X-Hearth-Note': urllib.parse.quote(note[:300], safe=' ')}
    if agent:
        h['User-Agent'] = 'hearth-classic/%s (Hearth OS)' % version
    return h


def _code_from(text):
    """The service's answer: its code, or an error with the service's own words."""
    try:
        v = json.loads(text or '{}')
    except ValueError:
        raise ValueError('the service answered %r' % (text or '')[:120])
    code = v.get('code') if isinstance(v, dict) else None
    if not isinstance(code, str) or not code:
        raise ValueError('the service said: %s' % ((v or {}).get('error', 'no code') if isinstance(v, dict) else 'no code'))
    return code


def why_failed(e):
    """A send's failure in plain words: HTTP status and the service's error, the network's or TLS's reason."""
    import urllib.error
    if isinstance(e, urllib.error.HTTPError):
        body = ''
        try:
            body = e.read(2000).decode('utf-8', 'replace')
            err = json.loads(body).get('error', '')
        except Exception:  # noqa: BLE001
            err = body[:120]
        return 'HTTP %s%s' % (e.code, (': ' + str(err)) if err else '')
    if isinstance(e, urllib.error.URLError):
        r = e.reason
        return str(getattr(r, 'reason', None) or r)[:200]
    if isinstance(e, (RuntimeError, ValueError)):
        return str(e)[:200]
    return ('%s: %s' % (type(e).__name__, e))[:200]


def post_log(body, note, version, opener=urllib.request.urlopen, ctx=None):
    """The direct send (Python, the system's CA bundle). Returns the code."""
    req = urllib.request.Request(SEND_SERVICE + '/v1/logs', data=body.encode('utf-8'), method='POST', headers=_headers(note, version))
    if ctx is None:
        ctx = ssl_context()[0]
    with opener(req, timeout=60, context=ctx) as r:
        return _code_from(r.read(4096).decode('utf-8', 'replace'))


def post_with_curl(body, note, version, run=subprocess.run, finder=None):
    """The same request through the system's curl (the way hearth-core's send_report sends Hearth's own logs)."""
    curl = (finder or find_program)('curl')
    if not curl:
        raise RuntimeError("curl isn't installed")
    import tempfile
    fd, tmp = tempfile.mkstemp(prefix='hearth-classic-send-', suffix='.txt')
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as f:
            f.write(body)
        args = [curl, '-sS', '-m', '60', '-X', 'POST', '-w', '\n%{http_code}']
        for k, v in _headers(note, version, agent=False).items():
            args += ['-H', '%s: %s' % (k, v)]
        args += ['--data-binary', '@' + tmp, SEND_SERVICE + '/v1/logs']
        r = run(args, capture_output=True, text=True, timeout=75, env=clean_env())
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass
    out = (r.stdout or '').rstrip('\n')
    text, _, status = out.rpartition('\n')
    if r.returncode != 0:
        raise RuntimeError((r.stderr or '').strip()[:200] or 'curl stopped (exit %s)' % r.returncode)
    if status and status != '200':
        try:
            err = json.loads(text).get('error', '')
        except (ValueError, AttributeError):
            err = text[:120]
        raise RuntimeError('HTTP %s%s' % (status, (': ' + str(err)) if err else ''))
    return _code_from(text)


def deliver(body, note, version, opener=urllib.request.urlopen, run=subprocess.run, finder=None, ctx=None):
    """Direct first, then curl. {ok, code, via} or {ok: False, error} with each way's reason; every try is logged."""
    tries = []
    try:
        if ctx is None:
            ctx, ca = ssl_context()
        else:
            ca = 'given'
        code = post_log(body, note, version, opener, ctx)
        write_diag('send: direct (Python, certificates from %s): sent, code %s' % (ca, code))
        return {'ok': True, 'code': code, 'via': 'direct'}
    except Exception as e:  # noqa: BLE001
        tries.append('direct: ' + why_failed(e))
    try:
        code = post_with_curl(body, note, version, run, finder)
        write_diag('send: %s; through curl: sent, code %s' % (tries[0], code))
        return {'ok': True, 'code': code, 'via': 'curl'}
    except Exception as e:  # noqa: BLE001
        tries.append('through curl: ' + why_failed(e))
    why = '; '.join(tries)
    write_diag('send: not sent (%s)' % why)
    log('send: not sent (%s)' % why)
    return {'ok': False, 'error': "Couldn't send it (%s)" % why}


def send_now(key, reason, diag_text, auto, settings=None, opener=urllib.request.urlopen, now=None, run=subprocess.run, finder=None, ctx=None):
    """Sends (or says why not): {ok, code, at, via, skipped, error}. Automatic sends need the switch, at most one per
    reason per plugin start and AUTO_PER_DAY a day; manual ones MANUAL_PER_DAY a day."""
    now = time.time() if now is None else now
    if auto:
        s = settings if settings is not None else read_settings()[0]
        if not s.get('autoSend'):
            return {'ok': False, 'skipped': 'automatic sending is off'}
        if key in SENT_THIS_START:
            return {'ok': False, 'skipped': 'already sent for this since the plugin started'}
    st = read_send_state()
    day = time.strftime('%Y-%m-%d', time.localtime(now))
    counts = st.get('days', {}) if isinstance(st.get('days'), dict) else {}
    n = counts.get(day + (':auto' if auto else ':manual'), 0)
    if n >= (AUTO_PER_DAY if auto else MANUAL_PER_DAY):
        return {'ok': False, 'skipped': 'the limit for today is reached'}
    if auto:
        SENT_THIS_START.add(key)
    version = hearth_only.plugin_version(plugin_dir()) or '?'
    r = deliver(send_body(reason, diag_text, version, now=now), 'Hearth Classic: ' + reason, version, opener, run, finder, ctx)
    if not r['ok']:
        return r
    code = r['code']
    counts = {k: v for k, v in counts.items() if k.startswith(day)}
    counts[day + (':auto' if auto else ':manual')] = n + 1
    write_send_state({'days': counts, 'last': {'at': int(now), 'code': code, 'reason': reason[:200], 'auto': auto, 'via': r['via']}})
    write_diag('sent to Hearth (%s, %s): code %s, %s' % ('automatically' if auto else 'by hand', r['via'], code, reason))
    return {'ok': True, 'code': code, 'at': int(now), 'via': r['via']}


class Plugin:
    async def hearth_os(self):
        """Whether this is Hearth OS (the frontend shows the Hearth OS panel instead of the plugin when not)."""
        return hearth_only.status(log, NAME)

    async def get_settings(self):
        if not hearth_only.check()[0]:
            return dict(DEFAULTS)
        return read_settings()[0]

    async def set_settings(self, change):
        """Only from the panel, when the person changes something."""
        if not hearth_only.check()[0]:
            return NOT_HEARTH
        try:
            return {'ok': True, 'settings': write_settings(change)}
        except (OSError, ValueError) as e:
            log('settings not saved: %s' % e)
            return {'ok': False, 'error': "Couldn't save (%s)" % type(e).__name__}

    async def brand(self):
        if not hearth_only.check()[0]:
            return None
        return brand()

    async def home(self):
        if not hearth_only.check()[0]:
            return NOT_HEARTH
        return home_layout()

    async def art(self, ref):
        if not hearth_only.check()[0]:
            return ''
        return read_art(ref)

    async def diagnostics(self, text):
        """The panel's "Write a Hearth Classic log", and each change of the menu bar's state: Decky's log and
        ~/.local/share/nimbus/hearth-classic.log (Hearth's Logs app)."""
        if not hearth_only.check()[0] or not isinstance(text, str):
            return {'ok': False}
        for line in text[:DIAG_TEXT].splitlines()[:60]:
            if line.strip():
                log('diagnostics: ' + line[:400])
        p = write_diag(text)
        return {'ok': bool(p), 'path': p.replace(home(), '~', 1) if p else ''}

    async def send_log(self, key, reason, text, auto):
        """Hearth Classic's diagnostics through Hearth's Send logs service: automatically (the switch, rate-limited)
        or by hand ("Send Hearth Classic log now")."""
        if not hearth_only.check()[0]:
            return NOT_HEARTH
        key = key if isinstance(key, str) else 'other'
        reason = reason if isinstance(reason, str) and reason.strip() else 'by hand'
        text = text if isinstance(text, str) else ''
        return await asyncio.get_running_loop().run_in_executor(None, lambda: send_now(key[:80], reason[:300], text[:DIAG_TEXT], bool(auto)))

    async def last_sent(self):
        if not hearth_only.check()[0]:
            return None
        v = read_send_state().get('last')
        return v if isinstance(v, dict) else None

    async def report(self, line):
        if isinstance(line, str):
            log('frontend: ' + line[:400])

    async def _main(self):
        if not hearth_only.status(log, NAME)['ok']:
            return   # not Hearth OS: nothing starts (the panel says why)
        s, state = read_settings()
        log('%s ready (look: %s, settings %s)' % (NAME, s['look'], state))
        self._loaded = asyncio.get_running_loop().create_task(hearth_only.mark_loaded_later(decky))

    async def _unload(self):
        t = getattr(self, '_loaded', None)
        if t:
            t.cancel()
