#!/usr/bin/env python3
"""Hearth's own apps, measured (1.13 "faster apps": measure first, then fix the slowest; the numbers go in the release
notes). Run by desktop-test.yml in its virtual Desktop Mode (the same commands work on a device):

  perf-measure.py idle SECONDS OUT.json             CPU (% of one core) and memory (MB) of every process Hearth runs
                                                    (the shell, labwc, Hearth's helpers), averaged over SECONDS
  perf-measure.py open NAME OUT.json -- CMD...      runs CMD (e.g. quickshell ipc call settings open) and times it:
                                                    "first" = the screen first changes, "ready" = it stops changing
                                                    (0.6 s still), and the shell's memory before and after
  perf-measure.py report OUT.json                   a table of everything in OUT.json (markdown, for the release notes)

Screen changes are read with grim at about 20 a second (a software-drawn virtual screen in CI is slower than a real
device: compare runs with runs, never with a device). Each result is merged into OUT.json under its name.
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


def frame():
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
        if isinstance(v, dict) and 'first' in v:
            lines.append('| %s | %s s | %s s | %s → %s MB |' % (k, v['first'], v['ready'], v['mbBefore'], v['mbAfter']))
    if 'idle' in d:
        lines += ['', '| Running (nothing open) | CPU | Memory |', '|---|---|---|']
        for k, v in d['idle'].items():
            lines.append('| %s%s | %s %% | %s MB |' % (k, ' (×%d)' % v['n'] if v['n'] > 1 else '', v['cpu'], v['mb']))
    return '\n'.join(lines)


def main(a):
    if len(a) == 3 and a[0] == 'idle':
        merge(a[2], 'idle', idle(float(a[1])))
    elif len(a) >= 5 and a[0] == 'open' and a[3] == '--':
        r = open_(a[4:])
        merge(a[2], a[1], r)
        print('::notice title=speed::%s: on screen %s s, ready %s s, shell %s → %s MB' % (a[1], r['first'], r['ready'], r['mbBefore'], r['mbAfter']))
    elif len(a) == 2 and a[0] == 'report':
        print(report(a[1]))
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
