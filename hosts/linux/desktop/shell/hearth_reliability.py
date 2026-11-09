#!/usr/bin/python3
"""hearth_reliability: what Reliability history shows (docs/SYSTEM-RESTORE.md, "Reliability history").

Reads, never changes anything:
  App stopped working   crashes from the system journal (systemd-coredump's entries)
  Hearth problems       the session's fallbacks and watchdog lines (~/.local/share/nimbus/session.log), the reasons
                        it leaves ($XDG_RUNTIME_DIR/hearth-*-mode-failed, ~/.local/state/nimbus/*-off), and Hearth's
                        own services that failed (hearth-*, nimbus-*)
  Warnings              network drops (NetworkManager), kept modest: drops close together are one warning
  Updates and changes   restore points (hearth_restore.py): a new Hearth version, "Before updating…", restores

  hearth_reliability.py summary [--days 14] [--weeks]   JSON: per day (or week) a stability score 1-10, counts, events

Light: the journal is read only when Reliability history opens, with journalctl's own filters (coredumps by their
message id, NetworkManager by its name, failed units by their message id), and only for days not seen before. Each
finished day's summary is kept in ~/.cache/hearth/reliability.json (small; days older than 120 are dropped), so the
next opening reads just today. Python standard library only.
"""
import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

COREDUMP_ID = 'fc2e22bc6ee647b6b90729ab34a250b1'     # systemd-coredump: "Process N (x) dumped core"
UNIT_FAILED_ID = 'd9b373ed55a64feb8242e02dbe79a49c'  # systemd: "Unit x entered failed state"
CACHE_VERSION = 2
KEEP_DAYS = 120
MAX_EVENTS = 40          # per day in the summary
CATS = ('app', 'hearth', 'warn', 'info')

# Process names people know by another name.
NAMES = {'steamwebhelper': 'Steam', 'steam': 'Steam', 'gamescope': 'gamescope (the Game Mode screen)',
         'gamescope-wl': 'gamescope (the Game Mode screen)', 'quickshell': 'Hearth desktop (Mantel)',
         'qs': 'Hearth desktop (Mantel)', 'nimbus': 'Hearth', 'labwc': 'the desktop\'s window manager (labwc)',
         'firefox': 'Hearth Browser', 'firefox-bin': 'Hearth Browser', 'wine64-preloader': 'a Windows app',
         'wine-preloader': 'a Windows app', 'pressure-vessel': 'a Steam game'}
# The session's own words for "it didn't start, so this instead" and its watchdog.
SESSION_PROBLEM = re.compile(r"instead|couldn't start|could not start|didn't start|back to the screen's own mode|"
                             r'rolled? back|crash|failed (twice|\d+ times)|paused|gave up|ended right away|stopped \d+ times',
                             re.I)
SESSION_SKIP = re.compile(r'^(desktop: |defaults-tidy)', re.I)
MODE_FALLBACK = re.compile(r'^(?:start: )?(steam game mode|hearth game mode|server mode|game mode): (.+?); (.+?) instead$', re.I)
SESSION_LINE = re.compile(r'^(\d{4}-\d{2}-\d{2}) (\d{2}):(\d{2}):(\d{2}) (.*)$')
NM_CHANGE = re.compile(r"device \(([^)]+)\): state change: (\w+) -> (\w+)(?: \(reason '([^']*)')?")
NM_QUIET_REASONS = ('sleeping', 'user-requested', 'unmanaged', 'removed', 'now-unmanaged', 'none', 'quitting')


def _env_dir(var, *default):
    return os.environ.get(var) or os.path.join(os.environ.get('HOME') or os.path.expanduser('~'), *default)


def cache_path():
    return os.path.join(_env_dir('XDG_CACHE_HOME', '.cache'), 'hearth', 'reliability.json')


def day_of(ts):
    return datetime.date.fromtimestamp(ts).isoformat()


def day_start(day):
    return time.mktime(datetime.date.fromisoformat(day).timetuple())


def nice_name(comm, exe=''):
    c = (comm or os.path.basename(exe or '') or 'An app').strip()
    return NAMES.get(c.lower(), NAMES.get(os.path.basename(exe or '').lower(), c))


def _usec(entry):
    try:
        return int(entry.get('__REALTIME_TIMESTAMP')) / 1e6
    except (TypeError, ValueError):
        return None


def _field(entry, k):
    v = entry.get(k)
    if isinstance(v, list):          # journalctl gives bytes as a list of numbers
        try:
            v = bytes(v).decode('utf-8', 'replace')
        except (TypeError, ValueError):
            v = ''
    return v if isinstance(v, str) else ('' if v is None else str(v))


# ------------------------------------------------------------------ the journal
def journal(match, since, run=subprocess.run, extra=()):
    """Entries since SINCE (epoch) matching MATCH (journalctl's own filters), as dicts. Nothing on any problem."""
    cmd = ['journalctl', '--no-pager', '-q', '-o', 'json', '--since=@%d' % int(since)] + list(extra) + list(match)
    try:
        r = run(cmd, capture_output=True, text=True, timeout=30)
        if r.returncode != 0 and any(x.startswith('--grep') for x in extra):
            # journalctl built without PCRE2 has no --grep: the same without it (filtered here instead).
            r = run([x for x in cmd if not x.startswith('--grep')], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return []
    out = []
    for line in (r.stdout or '').splitlines():
        line = line.strip()
        if not line.startswith('{'):
            continue
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if isinstance(e, dict):
            out.append(e)
    return out


def crash_events(entries):
    out = []
    for e in entries:
        ts = _usec(e)
        if ts is None:
            continue
        comm, exe = _field(e, 'COREDUMP_COMM'), _field(e, 'COREDUMP_EXE')
        sig = _field(e, 'COREDUMP_SIGNAL_NAME') or (('signal ' + _field(e, 'COREDUMP_SIGNAL')) if _field(e, 'COREDUMP_SIGNAL') else '')
        name = nice_name(comm, exe)
        sub = '%s crashed%s' % (comm or os.path.basename(exe) or 'it', ' (%s)' % sig if sig else '')
        unit = _field(e, 'COREDUMP_USER_UNIT') or _field(e, 'COREDUMP_UNIT')
        detail = '\n'.join(x for x in ['Program: ' + exe if exe else '', 'Signal: ' + sig if sig else '',
                                        'Part of: ' + unit if unit else '', (_field(e, 'MESSAGE').splitlines() or [''])[0][:300]] if x)
        out.append({'cat': 'app', 't': '%s closed unexpectedly' % name, 's': sub, 'time': int(ts), 'detail': detail})
    return out


def unit_events(entries):
    out = []
    for e in entries:
        ts = _usec(e)
        unit = _field(e, 'UNIT') or _field(e, 'USER_UNIT')
        if ts is None or not re.match(r'^(hearth|nimbus)[-@.]', unit):
            continue
        result = _field(e, 'UNIT_RESULT')
        out.append({'cat': 'hearth', 't': 'A part of Hearth stopped with a problem', 's': '%s %s' % (unit, ('(' + result + ')') if result else 'failed'),
                    'time': int(ts), 'detail': (_field(e, 'MESSAGE') or unit)[:400]})
    return out


def network_events(entries, gap=600):
    """Drops (activated -> something else, not because of sleep or you) per device; one that comes back within GAP
    seconds is "dropped for N seconds"; drops within the same hour on one device are one warning."""
    out = []
    down = {}
    last = {}
    for e in sorted(entries, key=lambda x: _usec(x) or 0):
        ts = _usec(e)
        m = NM_CHANGE.search(_field(e, 'MESSAGE'))
        if ts is None or not m:
            continue
        dev, old, new, reason = m.group(1), m.group(2), m.group(3), (m.group(4) or '')
        if old == 'activated' and new != 'activated':
            if reason in NM_QUIET_REASONS:
                continue
            down[dev] = (ts, reason)
        elif new == 'activated' and dev in down:
            t0, why = down.pop(dev)
            secs = int(ts - t0)
            kind = 'Wi-Fi' if dev.startswith('wl') else 'The wired network'
            if secs > gap:
                title, sub = '%s was off for %d minutes' % (kind, secs // 60), 'Reconnected by itself'
            else:
                title, sub = '%s dropped for %d seconds' % (kind, max(1, secs)), 'Reconnected by itself'
            prev = last.get(dev)
            if prev is not None and ts - prev['time'] < 3600:
                prev['n'] = prev.get('n', 1) + 1
                prev['t'] = '%s dropped %d times' % (kind, prev['n'])
                continue
            ev = {'cat': 'warn', 't': title, 's': sub + (' · ' + dev), 'time': int(t0),
                  'detail': 'Device: %s\nReason: %s' % (dev, why or 'not given')}
            last[dev] = ev
            out.append(ev)
    for dev, (t0, why) in down.items():
        kind = 'Wi-Fi' if dev.startswith('wl') else 'The wired network'
        out.append({'cat': 'warn', 't': '%s disconnected' % kind, 's': dev, 'time': int(t0),
                    'detail': 'Device: %s\nReason: %s' % (dev, why or 'not given')})
    for ev in out:
        ev.pop('n', None)
    return out


# ------------------------------------------------------------------ Hearth's own files
def session_events(text, since=0):
    out = []
    for line in (text or '').splitlines():
        m = SESSION_LINE.match(line.strip())
        if not m:
            continue
        msg = m.group(5)
        if SESSION_SKIP.match(msg) or not SESSION_PROBLEM.search(msg):
            continue
        try:
            ts = time.mktime(time.strptime('%s %s:%s:%s' % m.group(1, 2, 3, 4), '%Y-%m-%d %H:%M:%S'))
        except (ValueError, OverflowError):
            continue
        if ts < since:
            continue
        f = MODE_FALLBACK.match(msg)
        if f:
            mode = f.group(1).strip()
            title = '%s didn\'t start, %s opened instead' % (mode[0].upper() + mode[1:], f.group(3).strip())
            sub = f.group(2).strip()
        else:
            title, sub = 'Hearth had a problem starting something', msg
        out.append({'cat': 'hearth', 't': title, 's': sub[:200], 'time': int(ts), 'detail': msg[:500]})
    return out


def reason_events(paths, near=()):
    """Reason files the session leaves (one line, the reason). Skipped when the session log already has an event
    within two minutes of the file's time."""
    out = []
    titles = {'hearth-steam-mode-failed': "Steam Game Mode didn't start", 'hearth-game-mode-failed': "Hearth Game Mode didn't start",
              'hearth-server-mode-failed': "Server mode didn't start", 'steam-default-off': 'Steam Game Mode was paused after failed starts',
              'hearth-gm-off': 'Hearth Game Mode was paused after failed starts'}
    for p in paths:
        try:
            ts = os.stat(p).st_mtime
            with open(p, encoding='utf-8', errors='replace') as f:
                why = f.read(400).strip()
        except OSError:
            continue
        if any(abs(ts - t) < 120 for t in near):
            continue
        out.append({'cat': 'hearth', 't': titles.get(os.path.basename(p), 'Hearth had a problem'), 's': why.splitlines()[0][:200] if why else '',
                    'time': int(ts), 'detail': why})
    return out


def reason_files():
    run = os.environ.get('XDG_RUNTIME_DIR') or '/run/user/%d' % os.getuid()
    state = os.path.join(_env_dir('XDG_STATE_HOME', '.local', 'state'), 'nimbus')
    return [os.path.join(run, n) for n in ('hearth-steam-mode-failed', 'hearth-game-mode-failed', 'hearth-server-mode-failed')] \
        + [os.path.join(state, n) for n in ('steam-default-off', 'hearth-gm-off')]


def read_session_logs():
    d = os.path.join(_env_dir('XDG_DATA_HOME', '.local', 'share'), 'nimbus')
    text = ''
    for n in ('session.log.old', 'session.log'):
        try:
            with open(os.path.join(d, n), encoding='utf-8', errors='replace') as f:
                text += f.read(4 * 1024 * 1024) + '\n'
        except OSError:
            pass
    return text


def restore_events(points):
    out = []
    for p in points:
        ts = p.get('created')
        if not ts:
            continue
        title, kind = p.get('title') or '', p.get('kind')
        if kind == 'update' and re.match(r'^Hearth \S+$', title):
            out.append({'cat': 'info', 't': title.replace('Hearth ', 'Hearth updated to ', 1), 's': 'A restore point was made: "%s"' % title,
                        'time': int(ts), 'detail': 'Restore point %s' % p.get('id')})
        elif title.startswith('Before restoring'):
            out.append({'cat': 'info', 't': 'System Restore was used', 's': 'Today\'s settings were kept first: "%s"' % title,
                        'time': int(ts), 'detail': 'Restore point %s' % p.get('id')})
        else:
            out.append({'cat': 'info', 't': 'Restore point made', 's': '"%s"' % title, 'time': int(ts),
                        'detail': 'Restore point %s (%s)' % (p.get('id'), kind)})
    return out


# ------------------------------------------------------------------ putting it together
def score_days(counts):
    """A stability line like Windows': 10 is steady; problems pull it down, quiet days bring it back slowly."""
    s, out = 10.0, []
    for c in counts:
        s = min(10.0, s + 0.5)
        s -= min(4.5, c.get('app', 0) * 1.2 + c.get('hearth', 0) * 2.0 + c.get('warn', 0) * 0.4)
        s = max(1.0, s)
        out.append(round(s, 1))
    return out


def collect(since, run=subprocess.run, points=None, session_text=None, reasons=None):
    """Every event since SINCE (epoch), from all the sources."""
    ev = crash_events(journal(['MESSAGE_ID=' + COREDUMP_ID], since, run))
    ev += unit_events(journal(['MESSAGE_ID=' + UNIT_FAILED_ID], since, run))
    ev += network_events(journal(['SYSLOG_IDENTIFIER=NetworkManager'], since, run, extra=['--grep=state change: ']))
    sess = session_events(read_session_logs() if session_text is None else session_text, since)
    ev += sess
    ev += reason_events(reason_files() if reasons is None else reasons, near=[e['time'] for e in sess])
    if points is None:
        try:
            import hearth_restore
            points = hearth_restore.list_points()
        except Exception:
            points = []
    ev += restore_events(points)
    return [e for e in ev if e['time'] >= since]


def load_cache():
    try:
        with open(cache_path(), encoding='utf-8') as f:
            c = json.load(f)
        if isinstance(c, dict) and c.get('v') == CACHE_VERSION and isinstance(c.get('days'), dict):
            return c
    except (OSError, ValueError):
        pass
    return {'v': CACHE_VERSION, 'days': {}}


def save_cache(cache):
    """A cache, not a setting: written whole and atomically; losing it only means reading the journal again."""
    p = cache_path()
    try:
        os.makedirs(os.path.dirname(p), exist_ok=True)
        tmp = '%s.%d.tmp' % (p, os.getpid())
        with open(tmp, 'w', encoding='utf-8') as f:
            json.dump(cache, f, separators=(',', ':'))
        os.replace(tmp, p)
    except OSError:
        pass


def summary(days=14, weeks=False, now=None, run=subprocess.run, use_cache=True, **sources):
    now = time.time() if now is None else now
    today = day_of(now)
    span = days * 7 if weeks else days
    first = datetime.date.fromisoformat(today) - datetime.timedelta(days=span - 1)
    all_days = [(first + datetime.timedelta(days=i)).isoformat() for i in range(span)]
    cache = load_cache() if use_cache else {'v': CACHE_VERSION, 'days': {}}
    missing = [d for d in all_days if d == today or d not in cache['days']]
    if missing:
        events = collect(day_start(missing[0]), run, **sources)
        fresh = {d: [] for d in missing}
        for e in events:
            d = day_of(e['time'])
            if d in fresh:
                fresh[d].append(e)
        for d, evs in fresh.items():
            evs.sort(key=lambda e: e['time'], reverse=True)
            counts = {c: sum(1 for e in evs if e['cat'] == c) for c in CATS}
            entry = {'counts': counts, 'events': evs[:MAX_EVENTS]}
            if d != today:
                cache['days'][d] = entry
            else:
                today_entry = entry
        if use_cache:
            cutoff = (datetime.date.fromisoformat(today) - datetime.timedelta(days=KEEP_DAYS)).isoformat()
            cache['days'] = {d: v for d, v in cache['days'].items() if d >= cutoff}
            save_cache(cache)
    rows = []
    for d in all_days:
        e = today_entry if d == today else cache['days'].get(d, {'counts': {c: 0 for c in CATS}, 'events': []})
        rows.append({'date': d, 'counts': e['counts'], 'events': e['events']})
    scores = score_days([r['counts'] for r in rows])
    for r, s in zip(rows, scores):
        r['score'] = s
    if not weeks:
        return {'unit': 'day', 'today': today, 'rows': rows}
    out = []
    for i in range(0, len(rows), 7):
        wk = rows[i:i + 7]
        evs = sorted([e for r in wk for e in r['events']], key=lambda e: e['time'], reverse=True)
        out.append({'date': wk[0]['date'], 'end': wk[-1]['date'], 'score': round(sum(r['score'] for r in wk) / len(wk), 1),
                    'counts': {c: sum(r['counts'][c] for r in wk) for c in CATS}, 'events': evs[:MAX_EVENTS]})
    return {'unit': 'week', 'today': today, 'rows': out}


def main(argv):
    ap = argparse.ArgumentParser(prog='hearth_reliability.py')
    sub = ap.add_subparsers(dest='cmd')
    s = sub.add_parser('summary')
    s.add_argument('--days', type=int, default=14)
    s.add_argument('--weeks', action='store_true')
    args = ap.parse_args(argv[1:])
    if args.cmd != 'summary':
        ap.print_help(sys.stderr)
        return 2
    print(json.dumps(summary(max(1, min(args.days, 31)), args.weeks), ensure_ascii=False))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
