"""Ithil (Hearth AI) in Server mode (1.13, mockups next-update/server-ithil-dash.png and server-ithil-detail.png).

Ithil is the owner's own AI app (Dragongamer9/hearth-ai), installed from its .deb. Hearth OS only READS her files
(~/HearthAI, or HEARTH_AI_HOME): her state, her settings, her sleeps and her event log (events.db, opened read-only).
It never writes there. Starting her app, restarting it and "Put her to sleep" (her own API on this PC) are the only
things Server mode does to her, each only when the owner presses the button (or, for starting, when Server mode starts).

view(now) -> None when she isn't on this PC, else everything the tile and the opened view show.
brain(hours) -> her real log entries mapped to the parts of the 3D brain (the same mapping as her app's Mind screen).
"""
import json
import os
import shutil
import socket
import sqlite3
import subprocess
import time
import urllib.request

HOME = os.environ.get('HEARTH_AI_HOME') or os.path.expanduser('~/HearthAI')
PORT = 47860
APP_PATHS = ('/opt/Hearth AI/hearth-ai', '/usr/bin/hearth-ai', '/usr/local/bin/hearth-ai')


def _json(name, default=None):
    try:
        with open(os.path.join(HOME, name)) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def _jsonl(name, limit=400):
    out = []
    try:
        with open(os.path.join(HOME, name)) as f:
            for line in f.readlines()[-limit:]:
                try:
                    out.append(json.loads(line))
                except ValueError:
                    pass
    except OSError:
        pass
    return out


def app_path():
    for p in APP_PATHS:
        if os.path.isfile(p) and os.access(p, os.X_OK):
            return p
    return shutil.which('hearth-ai')


def installed():
    return bool(app_path()) or os.path.isfile(os.path.join(HOME, 'state.json'))


def running():
    try:
        return subprocess.run(['pgrep', '-u', str(os.getuid()), '-f', 'hearth-ai'], capture_output=True, timeout=3).returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


def version():
    try:
        r = subprocess.run(['dpkg-query', '-W', '-f=${Version}', 'hearth-ai'], capture_output=True, text=True, timeout=3)
        v = r.stdout.strip()
        return v[:-2] if v.endswith('.0') else v
    except (OSError, subprocess.SubprocessError):
        return ''


def _db():
    path = os.path.join(HOME, 'events.db')
    if not os.path.isfile(path):
        return None
    try:
        db = sqlite3.connect('file:%s?mode=ro' % path, uri=True, timeout=1)
        db.row_factory = sqlite3.Row
        return db
    except sqlite3.Error:
        return None


def _iso_ms(ts):
    """Her log's times are ISO in UTC ('2026-11-15T07:04:05.123Z')."""
    import calendar
    try:
        base = calendar.timegm(time.strptime(ts[:19], '%Y-%m-%dT%H:%M:%S'))
        frac = ts[19:].lstrip('.').rstrip('Z')
        return base * 1000 + (int(frac[:3].ljust(3, '0')) if frac[:3].isdigit() else 0)
    except (ValueError, TypeError):
        return 0


def mood_words(n):
    """Her mood words from her needs (the same rules as her app: core/needs.js moodLabels)."""
    out = []
    if n.get('energy', 1) < 0.35:
        out.append('tired')
    elif n.get('energy', 0) > 0.75:
        out.append('rested')
    if n.get('curiosity', 0) > 0.65:
        out.append('curious')
    if n.get('stability', 1) < 0.5:
        out.append('unsettled')
    return out or ['steady']


def _cut(t, n=70):
    t = ' '.join(str(t or '').split())
    return t if len(t) <= n else t[:n - 1] + '…'


def brain(hours=24, now=None):
    """Her log entries in the window (hours=0: her whole life), mapped to the 3D brain's parts."""
    now = now or time.time()
    to = int(now * 1000)
    frm = to - int(max(1, float(hours or 24)) * 3600000)
    out = {'from': frm, 'to': to, 'events': [], 'moods': [], 'sleeps': []}
    db = _db()
    if db is None:
        return out
    try:
        if not hours:
            r = db.execute('SELECT ts FROM events ORDER BY id LIMIT 1').fetchone()
            frm = min(to - 60000, _iso_ms(r['ts'])) if r else to - 3600000
            out['from'] = frm
        rows = db.execute('SELECT id, ts, actor, kind, body, ref_id FROM events ORDER BY id DESC LIMIT 6000').fetchall()
    except sqlite3.Error:
        rows = []
    finally:
        db.close()

    def E(i, ms, reg, text, **o):
        out['events'].append(dict({'id': i, 'ms': ms, 'reg': reg, 'text': text}, **o))
    for x in reversed(rows):
        ms = _iso_ms(x['ts'])
        if ms < frm:
            continue
        try:
            b = json.loads(x['body']) if x['body'] else {}
        except ValueError:
            b = {}
        k, i, ref = x['kind'], x['id'], x['ref_id']
        if k == 'owner.message':
            E(i, ms, 'senses', 'He said: ' + _cut(b.get('text')), turn=True)
        elif k == 'ithil.message':
            E(i, ms - 1200, 'workspace', 'Broadcast to every part', bc=True, quiet=True)
            E(i, ms - 800, 'confidence', 'Claims tagged: remembered / summary / guess')
            E(i, ms - 300, 'gate', 'Gate: reply allowed')
            E(i, ms, 'workspace', 'Replies: ' + _cut(b.get('text')))
            E(i, ms + 300, 'memory', 'Event #%d written (passed the gate)' % i)
        elif k == 'system.input':
            if b.get('observedKind'):
                E(i, ms, 'prediction', 'His reply was: ' + str(b['observedKind']).replace('_', ' '))
            elif b.get('allowed'):
                E(i, ms, 'gate', 'Gate: %s allowed' % b['allowed'])
            elif b.get('applied') == 'settings':
                E(i, ms, 'gate', 'Your signed setting applied: ' + _cut(b.get('key'), 40))
            elif b.get('from'):
                E(i, ms, 'senses', _cut(b['from'], 20) + ': ' + _cut(b.get('text')), turn=True)
        elif k == 'memory.pin':
            E(i, ms, 'knowledge', ('You pinned: ' if x['actor'] == 'owner' else 'Kept: ') + _cut(b.get('value'), 60))
        elif k == 'memory.correct':
            E(i, ms, 'knowledge', 'Correction to #%s (new entry)' % ref)
        elif k in ('memory.delete', 'owner.restore', 'owner.purge'):
            E(i, ms, 'memory', {'memory.delete': 'You marked #%s deleted (still in the log)', 'owner.restore': 'You restored #%s',
                                'owner.purge': 'You erased #%s (words wiped)'}[k] % ref)
        elif k == 'memory.forget':
            E(i, ms, 'sleep', 'Marked #%s "forget" (left out of search; nothing erased)' % ref)
        elif k == 'reflection':
            if b.get('sleep'):
                E(i, ms, 'knowledge', 'Sleep summary: ' + _cut(b.get('text')))
            elif b.get('field'):
                E(i, ms, 'selfmodel', 'Self-model: ' + _cut('%s: %s' % (b['field'], b.get('value'))))
            else:
                E(i, ms, 'selfmodel', 'Reflection: ' + _cut(b.get('text')))
        elif k == 'mood.snapshot':
            out['moods'].append(dict({'ms': ms}, **{n: b.get(n) for n in ('energy', 'social', 'curiosity', 'stability')}))
            E(i, ms, 'needs', 'Needs by fixed rules: energy %.2f, curiosity %.2f' % (b.get('energy') or 0, b.get('curiosity') or 0), quiet=True)
        elif k == 'gate.reject':
            E(i, ms, 'gate', 'Gate: BLOCKED (hard limit: %s) · %s' % (str(b.get('limit', '')).replace('_', ' '), b.get('effect', '')), block=True)
    for s in _jsonl('sleeps.jsonl'):
        a, z = int(s.get('at') or 0), int(s.get('end') or s.get('at') or 0)
        if z >= frm:
            out['sleeps'].append({'from': a, 'to': z})
            E('s%d' % a, a, 'sleep', 'Sleep cycle starts')
            if z > a:
                E('w%d' % z, z, 'sleep', 'Wakes up')
    out['events'].sort(key=lambda e: e['ms'])
    out['events'] = out['events'][-4000:]
    return out


def _ollama_loaded(model):
    try:
        with urllib.request.urlopen('http://127.0.0.1:11434/api/ps', timeout=1.5) as r:
            ps = json.loads(r.read() or b'{}')
        names = [m.get('name') or m.get('model') or '' for m in ps.get('models') or []]
        return any(n == model or n.split(':')[0] == str(model).split(':')[0] for n in names)
    except (OSError, ValueError):
        return False


def view(now=None):
    """Everything the tile and the opened view show, or None when she isn't on this PC."""
    if not installed():
        return None
    now = now or time.time()
    st = _json('state.json', {}) or {}
    se = _json('settings.json', {}) or {}
    up = running()
    needs, today = {}, {'talks': 0, 'memories': 0, 'blocks': 0}
    day0 = time.mktime(time.localtime(now)[:3] + (0, 0, 0, 0, 0, -1)) * 1000
    db = _db()
    if db is not None:
        try:
            r = db.execute("SELECT body FROM events WHERE kind = 'mood.snapshot' AND purged = 0 ORDER BY id DESC LIMIT 1").fetchone()
            if r and r['body']:
                needs = {k: float(v) for k, v in json.loads(r['body']).items() if isinstance(v, (int, float))}
            for x in db.execute("SELECT ts, kind FROM events WHERE kind IN ('owner.message', 'memory.pin', 'gate.reject') ORDER BY id DESC LIMIT 3000"):
                if _iso_ms(x['ts']) < day0:
                    break
                today[{'owner.message': 'talks', 'memory.pin': 'memories', 'gate.reject': 'blocks'}[x['kind']]] += 1
        except (sqlite3.Error, ValueError):
            pass
        finally:
            db.close()
    status = st.get('status') or 'awake'
    asleep = status == 'sleeping'
    cur = st.get('current') or {}
    focus = (st.get('spotlight') or {}).get('label') if isinstance(st.get('spotlight'), dict) else None
    focus = focus or cur.get('wants') or ''
    model = se.get('chatModel') or ''
    loaded = _ollama_loaded(model) if model and up else False
    last_reply = next((m for m in reversed(_jsonl('chat.jsonl', 200)) if m.get('role') == 'ithil' and not m.get('error')), None)
    sleeps = _jsonl('sleeps.jsonl', 5)
    last_sleep = sleeps[-1] if sleeps else None
    seen = _json('link-clients.json', {}) or {}
    reach = []
    for name, e in sorted(seen.items(), key=lambda x: -int((x[1] or {}).get('at') or 0)):
        if not isinstance(e, dict) or name == 'Hearth Server mode':
            continue
        fresh = now * 1000 - int(e.get('at') or 0) < 10 * 60000
        reach.append({'name': name, 'via': e.get('via') or '', 'chip': ('Connected' if fresh else 'Seen ' + _ago(int(e.get('at') or 0) / 1000, now)) +
                      (' · over Tailscale' if e.get('via') == 'Tailscale' else ''), 'ok': True})
    if not reach:
        reach.append({'name': 'AI Command Center', 'via': '', 'chip': 'Allowed' if se.get('allowCommandCenter') is not False else 'Turned off in her settings',
                      'ok': se.get('allowCommandCenter') is not False})
    reach.append({'name': 'Anything not yours', 'via': '', 'chip': 'Never', 'ok': False, 'never': True})
    host = socket.gethostname().split('.')[0]
    sl_text = ''
    if last_sleep and now * 1000 - int(last_sleep.get('end') or last_sleep.get('at') or 0) < 18 * 3600000:
        a, z = int(last_sleep.get('at') or 0) / 1000, int(last_sleep.get('end') or last_sleep.get('at') or 0) / 1000
        sl_text = '%s slept %s–%s' % (st.get('name') or 'Ithil', time.strftime('%H:%M', time.localtime(a)), time.strftime('%H:%M', time.localtime(z)))
        bits = []
        if last_sleep.get('replayed') is not None:
            bits.append('%d moments replayed' % int(last_sleep['replayed']))
        if last_sleep.get('energyBefore') is not None and last_sleep.get('energyAfter') is not None:
            bits.append('energy %s → %s' % (('%.2f' % last_sleep['energyBefore']).lstrip('0'), ('%.2f' % last_sleep['energyAfter']).lstrip('0')))
        sl_text = (sl_text, ', '.join(bits))
    return {
        'name': st.get('name') or 'Ithil', 'version': version(), 'running': up,
        'state': 'off' if not up else 'asleep' if asleep else 'awake',
        'chip': 'Not running' if not up else 'Asleep' if asleep else 'Awake',
        'mood': ', '.join(mood_words(needs)) if needs else (cur.get('mood') or ''),
        'focus': _cut(focus, 60), 'needs': {k: round(needs.get(k, 0), 2) for k in ('energy', 'social', 'curiosity', 'stability')} if needs else None,
        'brainModel': model, 'brainLoaded': loaded,
        'answered': _ago(int(last_reply['at']) / 1000, now) if last_reply and last_reply.get('at') else '',
        'today': today, 'sleepTime': se.get('sleepTime') or '',
        'address': 'http://%s:%d' % (host, PORT), 'reach': reach,
        'sleepLine': sl_text if isinstance(sl_text, tuple) else None,
    }


def _ago(t, now):
    m = int(max(0, now - t) // 60)
    if m < 1:
        return 'just now'
    if m < 60:
        return '%d min ago' % m
    h = m // 60
    return '%d h ago' % h if h < 24 else '%d days ago' % (h // 24)


def start(log=None):
    """Server mode started: start her app too if it's installed and not running (never blocks; never fails Server mode)."""
    p = app_path()
    if not p or running():
        return False
    try:
        subprocess.Popen([p], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        return True
    except OSError:
        return False


def show(screen='talk'):
    """Talk to her / Her settings: her app's window on that screen (a second start just brings the running one forward)."""
    p = app_path()
    if not p:
        return False
    try:
        subprocess.Popen([p, '--screen=' + screen], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        return True
    except OSError:
        return False


def restart():
    subprocess.run(['pkill', '-TERM', '-u', str(os.getuid()), '-f', 'hearth-ai'], capture_output=True, timeout=5)
    for _ in range(30):
        if not running():
            break
        time.sleep(0.2)
    return start()


def put_to_sleep():
    """Her own API on this PC (POST /v1/sleep, only from this PC, with her key from ~/HearthAI/connect.json)."""
    key = _json('connect.json', {}) or {}
    tok, port = key.get('token'), int(key.get('port') or PORT)
    if not tok:
        return False
    req = urllib.request.Request('http://127.0.0.1:%d/v1/sleep' % port, data=b'{}', method='POST',
                                 headers={'authorization': 'Bearer ' + tok, 'content-type': 'application/json', 'x-client': 'Hearth Server mode'})
    try:
        with urllib.request.urlopen(req, timeout=4) as r:
            return r.status == 200
    except OSError:
        return False
