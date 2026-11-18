#!/usr/bin/env python3
"""Where the desktop shell's memory goes (1.15 "Hearth's own apps made faster"; apps-speed.yml).

  perf-breakdown.py parts SHELL_DIR                 the parts of shell.qml that can be left out one at a time
  perf-breakdown.py run SHELL_DIR OUT.json          starts the shell (quickshell -p) once with everything, then once
                                                    without each part, and notes its memory (RSS and PSS) and CPU
                                                    once it has settled; the whole shell again at the middle and the
                                                    end, so the noise between starts shows
  perf-breakdown.py report OUT.json                 the table (markdown), biggest saving first

Only for measuring in CI: the shell runs from a copy of SHELL_DIR (shell.qml rewritten in the copy), never the real
one, and nothing is saved anywhere but OUT.json. A part that's left out can make other parts log errors (they name
it); its memory is still what that part costs, give or take.
"""
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

PAGE = os.sysconf('SC_PAGE_SIZE')
HZ = os.sysconf('SC_CLK_TCK')
# Small glue that costs nothing measurable: not tried on its own.
SKIP = ('IpcHandler', 'Connections', 'Binding', 'Timer', 'Process', 'FileView', 'QtObject')


def braces(line):
    """The change in { } depth on one line, ignoring strings and // comments."""
    d, q, i = 0, '', 0
    while i < len(line):
        c = line[i]
        if q:
            if c == '\\':
                i += 1
            elif c == q:
                q = ''
        elif c in '"\'`':
            q = c
        elif c == '/' and line[i + 1:i + 2] == '/':
            break
        elif c == '{':
            d += 1
        elif c == '}':
            d -= 1
        i += 1
    return d


def parts(text):
    """[(name, first line, last line)] for ShellRoot's children (and the per-screen Scope's), in order."""
    lines = text.split('\n')
    out, depth, starts = [], 0, {}
    root = next(i for i, l in enumerate(lines) if re.match(r'\s*ShellRoot\s*\{', l))
    for i in range(root, len(lines)):
        l = lines[i]
        before = depth
        depth += braces(l)
        m = re.match(r'\s*([A-Z][A-Za-z0-9_.]*)\s*\{', l)
        # Direct children of ShellRoot (depth 1), and those of Variants › Scope (depth 4: the wallpaper, the desktop,
        # the bar), named by their type, the Loader's file or their id.
        if m and i > root and before in (1, 4) and m.group(1) not in SKIP:
            src = re.search(r'source:\s*"([^"]+)"', l)
            ident = re.search(r'\bid:\s*(\w+)', l)
            name = m.group(1)
            if name == 'Loader':
                if not src:
                    nxt = '\n'.join(lines[i:i + 4])
                    src = re.search(r'source:\s*"([^"]+)"', nxt)
                name = 'Loader ' + (src.group(1) if src else (ident.group(1) if ident else 'line %d' % (i + 1)))
            elif ident:
                name += ' (' + ident.group(1) + ')'
            starts[i] = (name, before)
        if depth <= before:
            # Blocks that ended on this line.
            for s in sorted(list(starts), reverse=True):
                name, d0 = starts[s]
                if depth <= d0:
                    out.append((name, s, i))
                    del starts[s]
    out.sort(key=lambda p: p[1])
    # A Variants or a Scope holding parts that are tried on their own is still tried as a whole.
    return out


def without(text, part):
    lines = text.split('\n')
    _, a, b = part
    return '\n'.join(lines[:a] + ['    // (left out for perf-breakdown)'] + lines[b + 1:])


def shell_procs(tag):
    out = []
    for pid in os.listdir('/proc'):
        if not pid.isdigit():
            continue
        try:
            cmd = open('/proc/%s/cmdline' % pid, 'rb').read().split(b'\0')
        except OSError:
            continue
        if cmd and os.path.basename(cmd[0]).decode('utf-8', 'replace') in ('quickshell', 'qs') and tag.encode() in b' '.join(cmd) and b'ipc' not in cmd:
            out.append(int(pid))
    return out


def mem(pids):
    rss = pss = cpu = 0
    for p in pids:
        try:
            st = open('/proc/%d/stat' % p).read()
            f = st[st.rindex(')') + 2:].split()
            rss += int(f[21]) * PAGE
            cpu += int(f[11]) + int(f[12])
            for l in open('/proc/%d/smaps_rollup' % p):
                if l.startswith('Pss:'):
                    pss += int(l.split()[1]) * 1024
        except (OSError, ValueError, IndexError):
            pass
    return rss, pss, cpu


def measure(shell_dir, settle=12.0, sample=6.0, up_timeout=45.0):
    run = os.environ.get('XDG_RUNTIME_DIR', '/tmp')
    up = os.path.join(run, 'hearth-shell-up')
    try:
        os.unlink(up)
    except OSError:
        pass
    log = open(os.path.join(tempfile.gettempdir(), 'perf-breakdown-shell.log'), 'a')
    p = subprocess.Popen(['quickshell', '-p', shell_dir], stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    t0 = time.monotonic()
    while not os.path.exists(up) and time.monotonic() - t0 < up_timeout and p.poll() is None:
        time.sleep(0.2)
    upt = time.monotonic() - t0 if os.path.exists(up) else None
    time.sleep(settle)
    pids = shell_procs(shell_dir) or [p.pid]
    _, _, c0 = mem(pids)
    time.sleep(sample)
    rss, pss, c1 = mem(pids)
    alive = p.poll() is None
    try:
        os.killpg(p.pid, signal.SIGTERM)
        p.wait(10)
    except (OSError, subprocess.TimeoutExpired):
        try:
            os.killpg(p.pid, signal.SIGKILL)
        except OSError:
            pass
    time.sleep(1.5)
    return {'rss': round(rss / 1e6, 1), 'pss': round(pss / 1e6, 1), 'cpu': round(100.0 * (c1 - c0) / HZ / sample, 1),
            'up': round(upt, 1) if upt is not None else None, 'alive': alive}


def run_all(shell_dir, out_path, only=None):
    src = open(os.path.join(shell_dir, 'shell.qml')).read()
    ps = parts(src)
    if only:
        ps = [p for p in ps if any(o in p[0] for o in only)]
    work = tempfile.mkdtemp(prefix='hearth-breakdown-')
    copy = os.path.join(work, 'shell')
    shutil.copytree(shell_dir, copy, symlinks=True)
    res = {'full': [], 'parts': {}}

    def save():
        with open(out_path, 'w') as f:
            json.dump(res, f, indent=1)

    def full():
        open(os.path.join(copy, 'shell.qml'), 'w').write(src)
        r = measure(copy)
        res['full'].append(r)
        print('full shell: %s' % r, flush=True)
        save()

    full()
    for n, part in enumerate(ps):
        if n == len(ps) // 2:
            full()
        open(os.path.join(copy, 'shell.qml'), 'w').write(without(src, part))
        r = measure(copy)
        res['parts'][part[0]] = r
        print('without %s: %s' % (part[0], r), flush=True)
        save()
    full()
    shutil.rmtree(work, ignore_errors=True)
    return res


def report(path):
    d = json.load(open(path))
    fulls = [f for f in d['full'] if f['alive']]
    if not fulls:
        return 'The whole shell never stayed up.'
    base = sum(f['pss'] for f in fulls) / len(fulls)
    rbase = sum(f['rss'] for f in fulls) / len(fulls)
    spread = max(f['pss'] for f in fulls) - min(f['pss'] for f in fulls)
    lines = ['Whole shell: PSS %.0f MB, RSS %.0f MB (starts: %s; spread %.0f MB)' % (base, rbase, ', '.join('%.0f' % f['pss'] for f in fulls), spread),
             '', '| Left out | Saves (PSS) | PSS | RSS | CPU |', '|---|---|---|---|---|']
    rows = sorted(d['parts'].items(), key=lambda kv: kv[1]['pss'] if kv[1]['alive'] else 1e9)
    for k, v in rows:
        if not v['alive']:
            lines.append('| %s | (shell didn\'t stay up) | | | |' % k)
            continue
        lines.append('| %s | %.0f MB | %.0f | %.0f | %s %% |' % (k, base - v['pss'], v['pss'], v['rss'], v['cpu']))
    return '\n'.join(lines)


def main(a):
    if len(a) == 2 and a[0] == 'parts':
        for n, s, e in parts(open(os.path.join(a[1], 'shell.qml')).read()):
            print('%4d-%-4d %s' % (s + 1, e + 1, n))
    elif len(a) >= 3 and a[0] == 'run':
        run_all(a[1], a[2], a[3:] or None)
    elif len(a) == 2 and a[0] == 'report':
        print(report(a[1]))
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
