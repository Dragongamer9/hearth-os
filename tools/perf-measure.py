#!/usr/bin/env python3
"""Hearth's own apps, measured (1.13 "faster apps": measure first, then fix the slowest; the numbers go in the release
notes). Run by desktop-test.yml in its virtual Desktop Mode (the same commands work on a device):

  perf-measure.py idle SECONDS OUT.json             CPU (% of one core) and memory (MB) of every process Hearth runs
                                                    (the shell, labwc, Hearth's helpers), averaged over SECONDS
  perf-measure.py open NAME OUT.json -- CMD...      runs CMD (e.g. quickshell ipc call settings open) and times it:
                                                    "first" = the screen first changes, "ready" = it stops changing
                                                    (0.6 s still), and the shell's memory before and after
  perf-measure.py report OUT.json                   a table of everything in OUT.json (markdown, for the release notes)
  perf-measure.py compare BEFORE.json AFTER.json    the two side by side (1.15: the same run measures bughunt's shell
                                                    and the branch's, so the numbers compare), with a notice per row

  perf-measure.py ipc NAME OUT.json -- CMD...       how long an IPC call that changes nothing takes (the part of every
                                                    "on screen" time that is only `quickshell ipc` starting)

Screen changes are read with grim at about 20 a second, or on KWin (HEARTH_PERF_KWIN=1, 1.15) through KWin's
screenshot API (a software-drawn virtual screen in CI is slower than a real device: compare runs with runs, never with
a device). Each result is merged into OUT.json under its name.
"""
import hashlib
import json
import os
import subprocess
import sys
import time

HZ = os.sysconf('SC_CLK_TCK')
PAGE = os.sysconf('SC_PAGE_SIZE')
OURS = ('quickshell', 'qs', 'labwc', 'python3', 'hearth', 'nimbus', 'foot', 'swaybg', 'wlsunset')


def procs():
    out = {}
    for pid in os.listdir('/proc'):
        if not pid.isdigit():
            continue
        try:
            stat = open('/proc/%s/stat' % pid).read()
            cmd = open('/proc/%s/cmdline' % pid, 'rb').read().replace(b'\0', b' ').decode('utf-8', 'replace').strip()
        except OSError:
            continue
        comm = stat[stat.index('(') + 1:stat.rindex(')')]
        if os.stat('/proc/%s' % pid).st_uid != os.getuid():
            continue
        if not any(o in comm or o in cmd.split(' ')[0] for o in OURS):
            continue
        f = stat[stat.rindex(')') + 2:].split()
        name = label(comm, cmd)
        out[int(pid)] = {'name': name, 'cpu': int(f[11]) + int(f[12]), 'rss': int(f[21]) * PAGE}
    return out


def label(comm, cmd):
    """A readable name: the helper's file for python, the QML file for quickshell."""
    w = cmd.split()
    if comm.startswith('python') and len(w) > 1:
        return os.path.basename(w[1])
    if comm in ('quickshell', 'qs'):
        q = next((x for x in w if x.endswith('.qml')), '')
        return 'quickshell ' + (os.path.basename(q) if q else 'shell')
    return comm


def idle(secs):
    a = procs()
    time.sleep(secs)
    b = procs()
    rows = {}
    for pid, p in b.items():
        if pid not in a:
            continue
        r = rows.setdefault(p['name'], {'cpu': 0.0, 'mb': 0.0, 'n': 0})
        r['cpu'] += 100.0 * (p['cpu'] - a[pid]['cpu']) / HZ / secs
        r['mb'] += p['rss'] / 1e6
        r['n'] += 1
    return {k: {'cpu': round(v['cpu'], 2), 'mb': round(v['mb'], 1), 'n': v['n']} for k, v in sorted(rows.items(), key=lambda x: -x[1]['cpu'])}


_kwin = None


def kwin_frame():
    """A hash of KWin's screen (org.kde.KWin.ScreenShot2; KWin needs KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 in CI).
    The pipe is read while KWin writes it (a full screen doesn't fit in a pipe: waiting for the reply first could
    stall until the call timed out, so nothing seemed to change)."""
    global _kwin
    import dbus
    import threading
    if _kwin is None:
        _kwin = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
    r, w = os.pipe()
    h = hashlib.sha1()

    def drain():
        with os.fdopen(r, 'rb') as f:
            while True:
                c = f.read(1 << 20)
                if not c:
                    break
                h.update(c)
    t = threading.Thread(target=drain, daemon=True)
    t.start()
    try:
        _kwin.CaptureActiveScreen({'native-resolution': True}, dbus.types.UnixFd(w), timeout=5)
    except Exception:
        _kwin = None
        raise
    finally:
        os.close(w)
        t.join(5)
    return h.hexdigest()


def frame():
    if os.environ.get('HEARTH_PERF_KWIN') == '1':
        try:
            return kwin_frame()
        except Exception:
            return ''
    try:
        r = subprocess.run(['grim', '-t', 'ppm', '-s', '0.25', '-'], capture_output=True, timeout=5)
        return hashlib.sha1(r.stdout).hexdigest() if r.returncode == 0 else ''
    except (OSError, subprocess.TimeoutExpired):
        return ''


def shell_mb():
    return round(sum(p['rss'] for p in procs().values() if p['name'].startswith('quickshell')) / 1e6, 1)


def open_(cmd, timeout=20.0):
    base, before = frame(), shell_mb()
    t0 = time.monotonic()
    subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    first = ready = None
    last, still = base, t0
    while time.monotonic() - t0 < timeout:
        f = frame()
        now = time.monotonic()
        if f and f != last:
            if first is None and f != base:
                first = now - t0
            last, still = f, now
        elif first is not None and now - still >= 0.6:
            ready = still - t0
            break
        time.sleep(0.03)
    return {'first': round(first, 2) if first is not None else None, 'ready': round(ready, 2) if ready is not None else None,
            'mbBefore': before, 'mbAfter': shell_mb()}


def ipc_time(cmd, n=5):
    ts = []
    for _ in range(n):
        t0 = time.monotonic()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        ts.append(time.monotonic() - t0)
        time.sleep(0.2)
    ts.sort()
    return {'ipc': round(ts[len(ts) // 2], 2)}


def merge(path, name, value):
    try:
        d = json.load(open(path))
    except (OSError, ValueError):
        d = {}
    d[name] = value
    with open(path, 'w') as f:
        json.dump(d, f, indent=1)


def report(path):
    d = json.load(open(path))
    lines = ['| Opening | On screen | Ready | Shell memory |', '|---|---|---|---|']
    for k, v in d.items():
        if isinstance(v, dict) and 'ipc' in v:
            lines.append('| %s (an IPC call alone) | %s s | | |' % (k, v['ipc']))
        if isinstance(v, dict) and 'first' in v:
            lines.append('| %s | %s s | %s s | %s → %s MB |' % (k, v['first'], v['ready'], v['mbBefore'], v['mbAfter']))
    if 'idle' in d:
        lines += ['', '| Running (nothing open) | CPU | Memory |', '|---|---|---|']
        for k, v in d['idle'].items():
            lines.append('| %s%s | %s %% | %s MB |' % (k, ' (×%d)' % v['n'] if v['n'] > 1 else '', v['cpu'], v['mb']))
    return '\n'.join(lines)


def total(rows, prefix='quickshell'):
    return round(sum(v['mb'] for k, v in rows.items() if k.startswith(prefix)), 1)


def compare(pa, pb):
    with open(pa) as fa, open(pb) as fb:
        a, b = json.load(fa), json.load(fb)
    lines = ['| | Before | After |', '|---|---|---|']
    notes = []
    for key, what in (('idle', 'Shell memory, nothing open'), ('idle-after', 'Shell memory after opening everything once and closing it')):
        if key in a or key in b:
            x, y = total(a.get(key, {})), total(b.get(key, {}))
            lines.append('| %s | %s MB | %s MB |' % (what, x, y))
            notes.append('%s: %s MB → %s MB' % (what, x, y))
            cx = round(sum(v['cpu'] for k, v in a.get(key, {}).items() if k.startswith('quickshell')), 2)
            cy = round(sum(v['cpu'] for k, v in b.get(key, {}).items() if k.startswith('quickshell')), 2)
            lines.append('| %s: shell CPU | %s %% | %s %% |' % (what, cx, cy))
    for k in a:
        if isinstance(a[k], dict) and 'ipc' in a[k] and isinstance(b.get(k), dict):
            lines.append('| An IPC call alone (in every "on screen") | %s s | %s s |' % (a[k]['ipc'], b[k].get('ipc')))
    for k in list(a) + [k for k in b if k not in a]:
        x, y = a.get(k), b.get(k)
        if not (isinstance(x, dict) and 'first' in x) and not (isinstance(y, dict) and 'first' in y):
            continue
        f = lambda v: '%s s on screen, %s s ready, %s → %s MB' % (v['first'], v['ready'], v['mbBefore'], v['mbAfter']) if v else '–'
        lines.append('| %s | %s | %s |' % (k, f(x), f(y)))
        notes.append('%s: before %s | after %s' % (k, f(x), f(y)))
    return '\n'.join(lines), notes


def main(a):
    if len(a) in (3, 4) and a[0] == 'idle':
        merge(a[2], a[3] if len(a) == 4 else 'idle', idle(float(a[1])))
    elif len(a) >= 5 and a[0] == 'open' and a[3] == '--':
        r = open_(a[4:])
        merge(a[2], a[1], r)
        line = '%s: on screen %s s, ready %s s, shell %s → %s MB' % (a[1], r['first'], r['ready'], r['mbBefore'], r['mbAfter'])
        # HEARTH_PERF_QUIET=1: a plain line (apps-speed.yml posts the comparison instead; a step keeps 10 notices).
        print(line if os.environ.get('HEARTH_PERF_QUIET') == '1' else '::notice title=speed::' + line)
    elif len(a) >= 5 and a[0] == 'ipc' and a[3] == '--':
        r = ipc_time(a[4:])
        merge(a[2], a[1], r)
        print('%s: an IPC call alone takes %s s (part of every "on screen" time)' % (a[1], r['ipc']))
    elif len(a) == 3 and a[0] == 'compare':
        table, notes = compare(a[1], a[2])
        print(table)
        # Four rows a notice (a step keeps 10 notices).
        for i in range(0, len(notes), 4):
            print('::notice title=speed before → after (%d)::%s' % (i // 4 + 1, ' ;; '.join(notes[i:i + 4])))
    elif len(a) == 2 and a[0] == 'report':
        print(report(a[1]))
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
