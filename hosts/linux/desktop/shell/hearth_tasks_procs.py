"""Task Manager's helper (hearth-tasks), reading processes and grouping them into apps. Split out of hearth-tasks in the 1.12 tidy-up;
hearth-tasks re-exports everything here, so `hearth-tasks` and what loads it (Resource Monitor, the tests) are unchanged."""
from hearth_tasks_base import *  # noqa: F401,F403

# ---------------------------------------------------------------------- /proc parsing
def parse_stat(text):
    """/proc/PID/stat: "pid (comm) S ppid ...". The name can hold spaces and ')': split at the last ')'."""
    a, b = text.find('('), text.rfind(')')
    if a < 0 or b < a:
        raise ValueError('bad stat line')
    rest = text[b + 2:].split()
    return {'pid': int(text[:a]), 'comm': text[a + 1:b], 'state': rest[0], 'ppid': int(rest[1]),
            'jiffies': int(rest[11]) + int(rest[12]), 'start': int(rest[19]),
            'nice': int(rest[16]), 'threads': int(rest[17]),
            'rt': len(rest) > 38 and rest[38] in ('1', '2')}       # SCHED_FIFO / SCHED_RR: Realtime (1.12)


def parse_status(text):
    """/proc/PID/status: the real user id and memory (bytes). Memory is what the process uses itself
    (anonymous + shared memory), so shared libraries aren't counted again for every process."""
    out = {'uid': None, 'mem': 0, 'swap': 0, 'shared': 0, 'kthread': False, 'cpus': ''}
    for line in text.splitlines():
        k, _, v = line.partition(':')
        v = v.split()
        if not v:
            continue
        if k == 'Uid':
            out['uid'] = int(v[0])
        elif k in ('RssAnon', 'RssShmem'):
            out['mem'] += int(v[0]) * 1024
            if k == 'RssShmem':
                out['shared'] += int(v[0]) * 1024
        elif k == 'RssFile':
            out['shared'] += int(v[0]) * 1024       # libraries and files mapped, shared with other programs (1.12)
        elif k == 'VmSwap':
            out['swap'] = int(v[0]) * 1024          # moved to compressed memory (zram) or the swap file (1.11)
        elif k == 'Kthread':
            out['kthread'] = v[0] == '1'
        elif k == 'Cpus_allowed_list':
            out['cpus'] = v[0]                      # Details' Cores column (1.12, mockup tm-2)
    return out


def parse_cmdline(raw):
    return [s for s in raw.split('\0') if s]


def parse_cpu_stat(text):
    """/proc/stat: [(name, total, idle)] for "cpu" then "cpu0", "cpu1", ... (idle includes iowait)."""
    out = []
    for line in text.splitlines():
        if not line.startswith('cpu'):
            continue
        f = line.split()
        nums = [int(x) for x in f[1:]]
        # guest time is already counted in user/nice
        total = sum(nums[:8]) if len(nums) >= 8 else sum(nums)
        idle = nums[3] + (nums[4] if len(nums) > 4 else 0)
        out.append((f[0], total, idle))
    return out


def busy_percent(prev, cur):
    """CPU use between two (total, idle) samples, 0-100."""
    dt = cur[0] - prev[0]
    if dt <= 0:
        return 0.0
    return max(0.0, min(100.0, 100.0 * (dt - (cur[1] - prev[1])) / dt))


def parse_meminfo(text):
    out = {}
    for line in text.splitlines():
        k, _, v = line.partition(':')
        v = v.split()
        if v and v[0].isdigit():
            out[k] = int(v[0]) * 1024
    return out

def fdinfo_vram(text):
    """Video memory a DRM client holds (bytes): drm-memory-vram / drm-total-vram (KiB or MiB)."""
    m = re.search(r'^drm-(?:total|memory)-(?:vram|local)\d*:\s*(\d+)\s*(KiB|MiB|GiB)?', text or '', re.M)
    if not m:
        return 0
    return int(m.group(1)) * {'KiB': 1024, 'MiB': 1048576, 'GiB': 1073741824, None: 1}[m.group(2)]


def cores_label(cpus, n=None):
    """Cpus_allowed_list as Details shows it: "All" when every core is allowed, else "4–7" / "0,2"."""
    n = n or os.cpu_count() or 1
    if not cpus or cpus == ('0-%d' % (n - 1) if n > 1 else '0'):
        return 'All'
    return cpus.replace('-', '–')


def ai_job(cmd):
    """An AI agent the Command Center (or you) started: its name for the AI tag (1.12, mockup tm-1)."""
    base = os.path.basename((cmd or [''])[0]) if cmd else ''
    if base in ('node', 'python3', 'python', 'bun') and len(cmd) > 1:
        base = os.path.basename(cmd[1])
    return base in AI_AGENTS


AI_AGENTS = ('claude', 'codex', 'gemini', 'aider', 'freebuff', 'opencode', 'goose', 'ollama')


def parse_swaps(text):
    """/proc/swaps split into zram (in memory) and the drive (swap files and partitions), bytes."""
    out = {'zramSize': 0, 'zramUsed': 0, 'driveSize': 0, 'driveUsed': 0}
    for line in (text or '').splitlines()[1:]:
        f = line.split()
        if len(f) < 4 or not f[2].isdigit() or not f[3].isdigit():
            continue
        k = 'zram' if '/zram' in f[0] else 'drive'
        out[k + 'Size'] += int(f[2]) * 1024
        out[k + 'Used'] += int(f[3]) * 1024
    return out


def parse_psi(text):
    """/proc/pressure/memory "some" avg10 and avg60: how often programs waited for memory, percent."""
    m = re.search(r'^some\s+avg10=([\d.]+)\s+avg60=([\d.]+)', text or '', re.M)
    return {'psi10': float(m.group(1)), 'psi60': float(m.group(2))} if m else {'psi10': None, 'psi60': None}


def parse_mm_stat(text):
    """/sys/block/zram0/mm_stat: what was put in, its compressed size, and the memory zram really uses."""
    f = (text or '').split()
    n = lambda i: int(f[i]) if len(f) > i and f[i].isdigit() else 0
    return {'zramOrig': n(0), 'zramCompr': n(1), 'zramMem': n(2)}


def mem_extras(root, sys_root):
    """Task Manager › Memory's page (1.12, the approved mockup): zram, swap on the drive, memory pressure."""
    out = parse_swaps(read(os.path.join(root, 'swaps'), '') or '')
    out.update(parse_psi(read(os.path.join(root, 'pressure', 'memory'), '') or ''))
    out.update(parse_mm_stat(read(os.path.join(sys_root, 'block', 'zram0', 'mm_stat'), '') or ''))
    return out


def mem_used(mi):
    """Memory in use as Windows' Task Manager counts it: total − free − cache (page cache, buffers, reclaimable slab;
    shared memory counts as used). Not total − MemAvailable: MemAvailable also subtracts the kernel's reserve, so with
    Keep memory free set high it said "100% in use" while gigabytes were free (1.11)."""
    cache = mi.get('Cached', 0) + mi.get('Buffers', 0) + mi.get('SReclaimable', 0) - mi.get('Shmem', 0)
    return max(0, mi.get('MemTotal', 0) - mi.get('MemFree', 0) - max(0, cache))


def parse_cgroup_app(text):
    """The app a process belongs to, from its systemd unit: Flatpak app id or the app-…scope name."""
    for line in (text or '').splitlines():
        m = re.search(r'app-flatpak-([A-Za-z0-9._-]+?)-\d+\.scope', line)
        if m:
            return 'flatpak', m.group(1)
        m = re.search(r'/app-(?:[a-z]+-)?([A-Za-z0-9._\\-]+?)(?:@[^/]*)?(?:-\d+)?\.(?:scope|service)$', line)
        if m:
            return 'unit', m.group(1).replace('\\x2d', '-')
    return None, None


class Proc:
    __slots__ = ('pid', 'ppid', 'comm', 'uid', 'mem', 'jiffies', 'start', 'cmd', 'exe', 'kthread', 'root',
                 'state', 'nice', 'threads', 'path', 'swap', 'shared', 'cpus', 'rt')

    def __init__(self, **kw):
        for k in self.__slots__:
            setattr(self, k, kw.get(k))

    @property
    def key(self):
        return '%d:%d' % (self.pid, self.start)


def scan(root=PROC):
    """Every process: {pid: Proc}. Processes that end while being read are skipped."""
    procs = {}
    try:
        names = os.listdir(root)
    except OSError:
        raise Fail("can't read the process list")
    for n in names:
        if not n.isdigit():
            continue
        d = os.path.join(root, n)
        st, status = read(os.path.join(d, 'stat')), read(os.path.join(d, 'status'))
        if st is None or status is None:
            continue
        try:
            s, u = parse_stat(st), parse_status(status)
        except (ValueError, IndexError):
            continue
        try:
            exe = os.path.basename(os.readlink(os.path.join(d, 'exe'))).replace(' (deleted)', '')
        except OSError:
            exe = ''
        kthread = u['kthread'] or s['pid'] == 2 or s['ppid'] == 2
        procs[s['pid']] = Proc(pid=s['pid'], ppid=s['ppid'], comm=s['comm'], uid=u['uid'], mem=u['mem'],
                               jiffies=s['jiffies'], start=s['start'], kthread=kthread, root=root,
                               cmd=parse_cmdline(read(os.path.join(d, 'cmdline'), '')), exe=exe)
    return procs


def basename(arg):
    return re.split(r'[\\/]', arg)[-1] if arg else ''


def is_interpreter(p):
    return bool(p.cmd) and (basename(p.cmd[0]) in INTERPRETERS or p.comm in INTERPRETERS
                            or (p.exe or '').rstrip('0123456789.') in INTERPRETERS)


def script_name(p):
    """The script an interpreter (sh, python…) runs, or ''."""
    if not is_interpreter(p):
        return ''
    args = p.cmd[1:]
    if '-c' in args and args.index('-c') + 1 < len(args):          # sh -c "program args…"
        words = [w for w in args[args.index('-c') + 1].split() if w not in ('exec', 'env') and '=' not in w]
        return basename(words[0]) if words else ''
    return basename(next((a for a in args if not a.startswith('-')), ''))


def names_of(p):
    """The names a process goes by: its program, its short name, how it was started, and for scripts
    (sh, python…) the script itself."""
    out = [p.exe, p.comm]
    if p.cmd:
        out.append(basename(p.cmd[0]))
        out.append(script_name(p))
    return [n for n in out if n]


def protected_reason(p, guarded=()):
    """Why a process can't be ended here, or None."""
    if p.pid == 1:
        return 'System services (systemd)'
    if p.pid in guarded:
        return 'Mantel taskbar and menus'
    joined = ' '.join(p.cmd)
    for n in names_of(p):
        if n in PROTECTED:
            if n in ('nimbus', 'hearth') and '--browser' in p.cmd:
                return None   # Hearth Browser is a normal app
            return PROTECTED[n]
        if n.startswith('hearth-') and SHARE_MARK in joined:
            return 'Hearth controller and buttons'
    if 'nimbus-session' in joined or 'labwc/autostart' in joined:
        return 'Hearth session'
    return None


def ancestors(procs, pid):
    out, seen = [], set()
    while pid in procs and pid not in seen:
        seen.add(pid)
        out.append(pid)
        pid = procs[pid].ppid
    return out


def steam_app_id(p):
    """A game started by Steam: its launcher process ("reaper SteamLaunch AppId=N -- …")."""
    if 'SteamLaunch' not in p.cmd:
        return None
    for a in p.cmd:
        if a.startswith('AppId='):
            return a[6:]
    return None


def windows_exe(p):
    for a in p.cmd[:3]:
        if a.lower().endswith('.exe'):
            n = basename(a)[:-4]
            if n.lower() not in ('wine', 'wine64', 'wineserver', 'services', 'explorer', 'winedevice', 'plugplay',
                                 'svchost', 'rpcss', 'tabtip', 'conhost', 'start', 'steam', 'winedbg'):
                return n
    return None


# ---------------------------------------------------------------------- apps (.desktop files)
def data_dirs():
    home = os.path.expanduser('~')
    user = os.environ.get('XDG_DATA_HOME') or os.path.join(home, '.local', 'share')
    system = (os.environ.get('XDG_DATA_DIRS') or '/usr/local/share:/usr/share').split(':')
    extra = [os.path.join(user, 'flatpak', 'exports', 'share'), '/var/lib/flatpak/exports/share']
    out = []
    for d in [user] + system + extra:
        if d and d not in out:
            out.append(d)
    return out


def parse_desktop(text):
    """The [Desktop Entry] group of a .desktop file as a dict (unlocalized keys only)."""
    out, group = {}, None
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('['):
            group = line
            continue
        if group == '[Desktop Entry]' and '=' in line:
            k, v = line.split('=', 1)
            k = k.strip()
            if '[' not in k:
                out.setdefault(k, v.strip())
    return out


def exec_names(exec_line):
    """What an Exec= line starts: program names (and the Flatpak app id) to recognize it by."""
    try:
        args = shlex.split(exec_line)
    except ValueError:
        args = exec_line.split()
    args = [a for a in args if not a.startswith('%')]
    while args and (args[0] == 'env' or re.match(r'^[A-Za-z_][A-Za-z0-9_]*=', args[0])):
        args.pop(0)
    if not args:
        return [], None
    if basename(args[0]) == 'flatpak' and 'run' in args:
        rest = [a for a in args[args.index('run') + 1:] if not a.startswith('-')]
        return [], rest[0] if rest else None
    first = basename(args[0])
    names = [first]
    if first in INTERPRETERS and len(args) > 1:
        arg = next((a for a in args[1:] if not a.startswith('-')), '')
        if arg:
            names.append(basename(arg))
    return names, None


def desktop_index(dirs=None):
    """{key: {id, name, icon}} where key is a program name, desktop id or window class (lower case)."""
    dirs = dirs or [os.path.join(d, 'applications') for d in data_dirs()]
    by = {}

    def put(key, entry, weak=False):
        key = key.lower()
        if key and (key not in by or (by[key].get('hidden') and not entry.get('hidden'))):
            if weak and key in by:
                return
            by[key] = entry
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, '**', '*.desktop'), recursive=True)):
            de = parse_desktop(read(path, ''))
            if de.get('Type', 'Application') != 'Application' or not de.get('Name'):
                continue
            did = os.path.relpath(path, d)[:-8].replace('/', '-')
            entry = {'id': did, 'name': de['Name'], 'icon': de.get('Icon', ''),
                     'hidden': de.get('NoDisplay') == 'true' or de.get('Hidden') == 'true'}
            put(did, entry)
            names, flatpak = exec_names(de.get('Exec', ''))
            if flatpak:
                put(flatpak, entry)
            for n in names:
                if n not in INTERPRETERS:
                    put(n, entry, weak=True)
            if de.get('StartupWMClass'):
                put(de['StartupWMClass'], entry)
            last = did.split('.')[-1]
            if last != did:
                put(last, entry, weak=True)
    return by


def cached_desktop_index():
    """The index, kept in $XDG_RUNTIME_DIR until an applications folder changes."""
    dirs = [os.path.join(d, 'applications') for d in data_dirs()]
    sig = []
    for d in dirs:
        try:
            sig.append([d, os.stat(d).st_mtime])
        except OSError:
            pass
    st = load_state('apps')
    if st and st.get('sig') == sig:
        return st['index']
    idx = desktop_index(dirs)
    save_state('apps', {'sig': sig, 'index': idx})
    return idx


def steam_libraries():
    """Steam's library folders (where appmanifest_N.acf files are)."""
    home = os.path.expanduser('~')
    roots = [os.path.join(home, '.local/share/Steam'), os.path.join(home, '.steam/steam'),
             os.path.join(home, '.var/app/com.valvesoftware.Steam/.local/share/Steam')]
    libs = []
    for r in roots:
        lf = read(os.path.join(r, 'steamapps', 'libraryfolders.vdf'))
        libs.append(os.path.join(r, 'steamapps'))
        for m in re.finditer(r'"path"\s+"([^"]+)"', lf or ''):
            libs.append(os.path.join(m.group(1).replace('\\\\', '\\'), 'steamapps'))
    return libs


def steam_name(app_id, libs=None):
    for lib in libs if libs is not None else steam_libraries():
        m = re.search(r'"name"\s+"([^"]+)"', read(os.path.join(lib, 'appmanifest_%s.acf' % app_id), '') or '')
        if m:
            return m.group(1)
    return None


def lookup_app(p, index, cache=None):
    if cache is not None:
        if p.key not in cache:
            cache[p.key] = lookup_app(p, index)
        return cache[p.key]
    kind, app = parse_cgroup_app(read(os.path.join(p.root, str(p.pid), 'cgroup')))
    if kind == 'flatpak' and app.lower() in index:
        return index[app.lower()]
    for n in names_of(p):
        n = n.lower()
        if n in INTERPRETERS:
            continue
        if n in index:
            return index[n]
    if app and app.lower() in index:
        return index[app.lower()]
    return None


# ---------------------------------------------------------------------- grouping
def build_groups(procs, uid, self_pid=None, index=None, game_name=None, cache=None):
    """Your processes grouped by the app that started them. An app is a process whose parent is one of the
    session's own parts (labwc, Quickshell, systemd…) or isn't yours; everything it starts belongs to it.
    Steam games get their own group. The session's own parts are listed one by one, protected."""
    index = index or {}
    own = {pid: p for pid, p in procs.items() if p.uid == uid and not p.kthread}
    guarded = set(ancestors(procs, self_pid)) if self_pid else set()
    reasons = {pid: protected_reason(p, guarded) for pid, p in own.items()}
    infra = {pid for pid, r in reasons.items() if r}
    roots = {}

    def root_of(pid):
        path, cur = [], pid
        while True:
            if cur in roots:
                r = roots[cur]
                break
            path.append(cur)
            par = own[cur].ppid
            if steam_app_id(own[cur]) or par not in own or par in infra or par in path:
                r = cur
                break
            cur = par
        for x in path:
            roots[x] = r
        return r

    members = {}
    for pid in own:
        if pid in infra:
            members[pid] = [pid]
        else:
            members.setdefault(root_of(pid), []).append(pid)

    groups = []
    for root, pids in members.items():
        rp = own[root]
        pids.sort()
        g = {'id': rp.key, 'pid': root, 'pids': pids, 'count': len(pids), 'mem': sum(own[x].mem for x in pids),
             'name': '', 'icon': '', 'desktop': '', 'appIds': [], 'kind': 'background', 'protected': False, 'why': '', 'tag': ''}
        if root in infra:
            g.update(kind='system', protected=True, why=reasons[root], name=script_name(rp) or rp.exe or rp.comm, tag='Part of Hearth')
            groups.append(g)
            continue
        appid = steam_app_id(rp)
        # The app: the first process in the group with a .desktop file (the group's own first).
        order = [root] + [x for x in pids if x != root]
        entry = next((e for e in (lookup_app(own[x], index, cache) for x in order[:40]) if e), None)
        exe = next((n for n in (windows_exe(own[x]) for x in order) if n), None)
        if appid:
            g.update(kind='game', name=(game_name(appid) if game_name else None) or exe or 'Steam game ' + appid,
                     icon='steam_icon_' + appid, appIds=['steam_app_' + appid], appid=appid,
                     tag='Game · Proton' if exe else 'Steam')          # the tag in mockup tm-1 (1.12)
        elif entry:
            g.update(kind='app', name=entry['name'], icon=entry['icon'], desktop=entry['id'])
            if exe:
                g['tag'] = 'Windows app'
        elif exe:
            g.update(kind='game', name=exe, icon='wine', tag='Game · Proton' if any('proton' in x.lower() for x in (own[root].cmd or [])[:3]) else 'Windows app')
        else:
            g['name'] = rp.exe or rp.comm or str(root)
        ids = set(g['appIds'])
        for x in order[:40]:
            ids.update(n.lower() for n in names_of(own[x])[:3] if n not in INTERPRETERS)
        if entry:
            ids.add(entry['id'].lower())
        g['appIds'] = sorted(ids)
        groups.append(g)
    return groups


def cpu_total(root=PROC):
    st = parse_cpu_stat(read(os.path.join(root, 'stat'), '') or '')
    if not st:
        raise Fail("can't read CPU use")
    return st


def add_cpu(groups, procs, prev, cur_total, prev_total):
    """CPU % of the whole machine (like Windows' Task Manager) for each group, from two samples."""
    dt = cur_total - prev_total
    for g in groups:
        used = 0
        for pid in g['pids']:
            p = procs[pid]
            if p.key in prev:
                used += max(0, p.jiffies - prev[p.key])
        g['cpu'] = round(100.0 * used / dt, 1) if dt > 0 else 0.0


def list_apps(root=PROC):
    uid, now = os.getuid(), time.monotonic()
    st = load_state('list')
    procs, total = scan(root), cpu_total(root)
    if not st or not (0.2 <= now - st.get('t', 0) <= 15):
        st = {'t': now, 'total': total[0][1], 'idle': total[0][2], 'procs': {p.key: p.jiffies for p in procs.values() if p.uid == uid}}
        time.sleep(0.25)
        procs, total = scan(root), cpu_total(root)
        now = time.monotonic()
    libs = steam_libraries()
    groups = [g for g in build_groups(procs, uid, os.getpid(), cached_desktop_index(), lambda a: steam_name(a, libs))
              if g['pid'] != os.getpid()]          # not this helper itself
    add_cpu(groups, procs, st['procs'], total[0][1], st['total'])
    save_state('list', {'t': now, 'total': total[0][1], 'idle': total[0][2],
                        'procs': {p.key: p.jiffies for p in procs.values() if p.uid == uid}})
    groups.sort(key=lambda g: (-g['mem'], g['name'].lower()))
    mi = parse_meminfo(read(os.path.join(root, 'meminfo'), '') or '')
    return {'apps': groups, 'cpu': round(busy_percent((st['total'], st['idle']), total[0][1:]), 1),
            'mem': {'total': mi.get('MemTotal', 0), 'used': mem_used(mi)}}


def signal_app(pid, sig, root=PROC):
    """Sends SIGTERM/SIGKILL to an app: the process and everything in its group (what it started)."""
    procs = scan(root)
    p = procs.get(pid)
    if not p:
        raise Fail('that app has already closed')
    if p.uid != os.getuid():
        raise Fail("that belongs to another user, so it can't be ended here")
    groups = build_groups(procs, os.getuid(), os.getpid())
    g = next((g for g in groups if pid in g['pids']), None)
    if not g or g['protected']:
        raise Fail("%s keeps Hearth running, so it can't be ended here" % ((g or {}).get('why') or 'this'))
    done = []
    for x in g['pids']:
        try:
            os.kill(x, sig)
            done.append(x)
        except ProcessLookupError:
            pass
        except PermissionError:
            raise Fail("not allowed to end process %d" % x)
    return {'pid': pid, 'pids': done, 'signal': 'kill' if sig == signal.SIGKILL else 'term'}


