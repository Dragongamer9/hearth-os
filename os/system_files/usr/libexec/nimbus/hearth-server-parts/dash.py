#!/usr/bin/python3 -I
"""The server dashboard (boards docs/mockups/server-expansion/03-dashboard.png, 04-dashboard-tv.png; checklist
docs/mockups/checklists/server-dashboard.md): every service on this PC in one place, its real health, and the fixes.

    dash.py status [1h|24h|7d]   everything the Dashboard shows (desk, TV, phone and Ally): the PC (CPU, graphics card,
                                 its memory, memory, drives, temperatures), every service (running, answering, its CPU,
                                 graphics memory, memory, disk, last backup), what needs a look, earlier today, the
                                 one-tap fixes, network, power and tonight's jobs; also a point of history (light)
    dash.py summary              Server home's tile and the overall line (healthy services, PCs, your devices)
    dash.py fix ID|all           one fix (restart:<service>, shield-lists, gpu-free, restart-all, caches, logs)
    dash.py action SERVICE WHAT  a service's own button (restart, rescan, clear-queue)
    dash.py set KEY VALUE        phone on|off, quiet on|off, onlyIdle on|off, price DOLLARS (a kWh)
    dash.py alerts on|off        Alerts go to your phone: a user timer that looks every 10 minutes (off until you turn
                                 it on; never part of starting the PC)
    dash.py watch                that timer's run: anything new that needs a look is said (desktop, phone), quiet
                                 hours kept
    dash.py autoupdate on|off|run  Update server apps by themselves (off unless you turn it on): app parts update with
                                 no password; the system part waits for your password and says so

How health is found (never made up): each service's systemd unit (system or yours: ActiveState, NRestarts, its
memory and CPU) and its own local address answering (Ollama's /api/ps, ComfyUI's /queue, AdGuard Home's
/control/filtering/status…). A service that isn't installed isn't counted; one you turned off shows as off.
"""
import importlib.machinery
import importlib.util
import json
import os
import re
import sys
import time


def load_common():
    p = os.path.join(os.path.dirname(os.path.realpath(__file__)), 'common.py')
    loader = importlib.machinery.SourceFileLoader('hearth_server_common', p)
    spec = importlib.util.spec_from_loader('hearth_server_common', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


C = load_common()
Fail = C.Fail
PART = 'dash'
DEFAULTS = {'phone': False, 'quiet': False, 'quietFrom': 22, 'quietTo': 6, 'onlyIdle': True, 'auto': False, 'price': 0.14}
FIXTURE = os.environ.get('HEARTH_DASH_WORLD')  # the tests' pretend PC: a JSON of what each probe answers

# The services: (id, name, what it is, mark, colour, group, unit, user unit?, its address, data folders, small card)
SERVICES = [
    ('ollama', 'Ollama', 'AI chat · small model while you play', 'OL', 'pu', 'AI', 'hearth-ollama.service', False, 'http://127.0.0.1:11434', ['/var/lib/hearth-ollama/models'], False),
    ('colibri', 'Colibri · Big AI', 'the big models', 'CO', 'pu', 'AI', 'hearth-colibri.service', True, 'http://127.0.0.1:11440', ['~/colibri-models'], False),
    ('jellyfin', 'Jellyfin', 'TV and movies', 'JF', 'bl', 'Media', 'hearth-jellyfin.service', False, 'http://127.0.0.1:8096', ['/var/lib/jellyfin'], False),
    ('comfyui', 'Imagine · ComfyUI', 'AI pictures', 'IM', 'pk', 'AI', 'hearth-comfyui.service', True, 'http://127.0.0.1:8188', ['~/.local/share/hearth/comfyui'], False),
    ('kokoro', 'Kokoro voices', 'Ithil\'s voice · Listen', 'KO', 'tl', 'AI', 'hearth-kokoro.service', True, 'http://127.0.0.1:8880', [], False),
    ('vaultwarden', 'Passwords · Vaultwarden', 'your passwords', 'PW', 'gy', 'Home', 'hearth-vaultwarden.service', True, 'http://127.0.0.1:8222', ['~/.local/share/hearth/oss/vaultwarden'], False),
    ('nextcloud', 'Cloud · Nextcloud', 'files, calendar, contacts', 'CL', 'bl', 'Home', 'hearth-nextcloud.service', True, 'http://127.0.0.1:8090', ['~/.local/share/hearth/oss/nextcloud'], False),
    ('homeassistant', 'Home · Home Assistant', 'your home', 'HM', 'gr', 'Home', 'hearth-home-assistant.service', True, 'http://127.0.0.1:8123', ['~/.local/share/hearth/oss/homeassistant'], False),
    ('shield', 'Hearth Shield · AdGuard Home', 'ad blocking for the whole network', 'SH', 'rd', 'Network', 'hearth-server-dns.service', False, 'http://127.0.0.1:3053', [], False),
    ('games', 'Game servers', 'your game servers', 'GS', 'gr', 'Games', '', True, '', [], True),
    ('cache', 'Game cache', 'downloads once for every device', 'GC', 'yl', 'Games', '', False, '', [], True),
    ('sunshine', 'Remote play · Sunshine', 'any game on any screen', 'RP', 'or', 'Games', 'sunshine.service', True, 'https://127.0.0.1:47990', [], True),
    ('tailscale', 'Tailscale · Hearth Link', 'your devices, privately', 'TL', 'tl', 'Network', 'tailscaled.service', False, '', [], True),
]
STALE_LISTS = 7 * 86400


def cfg():
    return C.settings(PART, DEFAULTS)


# ------------------------------------------------------------------ the pretend PC (tests) or the real one
def world():
    if FIXTURE:
        try:
            with open(FIXTURE, encoding='utf-8') as f:
                return json.load(f)
        except (OSError, ValueError):
            return {}
    return None


def systemd(unit, user):
    """{active, restarts, mem, cpu_ns, pid, exists} of a unit."""
    w = world()
    if w is not None:
        return (w.get('units') or {}).get(unit) or {'exists': False}
    if not unit or not C.which('systemctl'):
        return {'exists': False}
    code, out = C.run(['systemctl'] + (['--user'] if user else []) + ['show', unit, '-p', 'ActiveState', '-p', 'LoadState', '-p', 'NRestarts',
                                                                     '-p', 'MemoryCurrent', '-p', 'CPUUsageNSec', '-p', 'MainPID', '-p', 'UnitFileState'], timeout=10)
    d = dict(l.split('=', 1) for l in out.splitlines() if '=' in l)
    if d.get('LoadState') in ('not-found', '', None):
        return {'exists': False}

    def num(k):
        try:
            v = int(d.get(k) or 0)
            return v if v < (1 << 62) else 0
        except ValueError:
            return 0
    return {'exists': True, 'active': d.get('ActiveState'), 'restarts': num('NRestarts'), 'mem': num('MemoryCurrent'), 'cpu_ns': num('CPUUsageNSec'),
            'pid': num('MainPID'), 'enabled': d.get('UnitFileState')}


def probe(url, path, timeout=2):
    w = world()
    if w is not None:
        v = (w.get('http') or {}).get(url + path)
        return (200, v) if v is not None else (0, None)
    if not url:
        return 0, None
    return C.http_json('GET', url + path, timeout=timeout)


def nvidia():
    w = world()
    if w is not None:
        return w.get('gpu')
    if not C.which('nvidia-smi'):
        return amd_gpu()
    code, out = C.run(['nvidia-smi', '--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw', '--format=csv,noheader,nounits'], timeout=10)
    if code != 0 or not out.strip():
        return amd_gpu()
    p = [x.strip() for x in out.splitlines()[0].split(',')]

    def fl(x):
        try:
            return float(x)
        except ValueError:
            return 0.0
    g = {'name': p[0].replace('NVIDIA GeForce ', '').replace('NVIDIA ', ''), 'pct': fl(p[1]), 'used': fl(p[2]) * 1048576, 'total': fl(p[3]) * 1048576, 'temp': fl(p[4]), 'watts': fl(p[5]) if len(p) > 5 else 0}
    code, out = C.run(['nvidia-smi', '--query-compute-apps=pid,used_memory', '--format=csv,noheader,nounits'], timeout=10)
    g['procs'] = {}
    for l in out.splitlines():
        a = [x.strip() for x in l.split(',')]
        if len(a) == 2 and a[0].isdigit():
            try:
                g['procs'][int(a[0])] = float(a[1]) * 1048576
            except ValueError:
                pass
    return g


def amd_gpu():
    for card in sorted(os.listdir('/sys/class/drm')) if os.path.isdir('/sys/class/drm') else []:
        d = '/sys/class/drm/%s/device' % card
        try:
            with open(d + '/gpu_busy_percent') as f:
                pct = float(f.read())
            used = float(open(d + '/mem_info_vram_used').read())
            total = float(open(d + '/mem_info_vram_total').read())
            return {'name': 'Radeon', 'pct': pct, 'used': used, 'total': total, 'temp': 0, 'watts': 0, 'procs': {}}
        except (OSError, ValueError):
            continue
    return None


def meminfo():
    w = world()
    if w is not None:
        return w.get('mem') or {'total': 0, 'avail': 0}
    m = {}
    try:
        with open('/proc/meminfo') as f:
            for l in f:
                k, v = l.split(':', 1)
                m[k] = int(v.split()[0]) * 1024
    except OSError:
        pass
    return {'total': m.get('MemTotal', 0), 'avail': m.get('MemAvailable', 0)}


def cpu_sample():
    w = world()
    if w is not None:
        return w.get('cpu') or {}
    try:
        with open('/proc/stat') as f:
            a = [int(x) for x in f.readline().split()[1:]]
    except (OSError, ValueError):
        return {}
    name = ''
    try:
        with open('/proc/cpuinfo') as f:
            for l in f:
                if l.startswith('model name'):
                    name = l.split(':', 1)[1].strip()
                    break
    except OSError:
        pass
    return {'busy': sum(a) - a[3] - (a[4] if len(a) > 4 else 0), 'all': sum(a), 'name': name, 'cores': os.cpu_count() or 1}


def cpu_short(name):
    m = re.search(r'(Ryzen \d+|Core\(TM\) i\d|Core i\d|Core Ultra \d|Xeon|EPYC|Threadripper)', name or '')
    return m.group(1).replace('(TM)', '') if m else (name.split()[0] if name else 'CPU')


def temps():
    w = world()
    if w is not None:
        return w.get('cpuTemp')
    best = None
    base = '/sys/class/hwmon'
    for h in sorted(os.listdir(base)) if os.path.isdir(base) else []:
        try:
            name = open(os.path.join(base, h, 'name')).read().strip()
        except OSError:
            continue
        if name in ('k10temp', 'coretemp', 'zenpower'):
            for f in sorted(os.listdir(os.path.join(base, h))):
                if f.startswith('temp') and f.endswith('_input'):
                    try:
                        v = int(open(os.path.join(base, h, f)).read()) / 1000.0
                        best = max(best or 0, v)
                    except (OSError, ValueError):
                        pass
    return best


def drives():
    w = world()
    if w is not None:
        return w.get('drives') or []
    outl = []
    seen = set()
    try:
        with open('/proc/mounts') as f:
            mounts = [l.split() for l in f]
    except OSError:
        mounts = []
    for m in mounts:
        dev, mnt, fs = m[0], m[1].replace('\\040', ' '), m[2]
        if not dev.startswith('/dev/') or fs in ('squashfs', 'overlay', 'tmpfs', 'vfat') or mnt.startswith(('/boot', '/sysroot/ostree', '/proc', '/run/credentials')):
            continue
        disk = re.sub(r'(p?\d+)$', '', os.path.basename(dev)) if not dev.startswith('/dev/mapper') else dev
        if disk in seen:
            continue
        try:
            s = os.statvfs(mnt)
        except OSError:
            continue
        seen.add(disk)
        outl.append({'disk': disk, 'mount': mnt, 'total': s.f_blocks * s.f_frsize, 'used': (s.f_blocks - s.f_bfree) * s.f_frsize})
    health = {}
    for p in ('/var/lib/hearth/home-server-alerts.json',):
        try:
            with open(p) as f:
                a = json.load(f)
            for k, v in (a.get('drives') or {}).items():
                health[k] = v
        except (OSError, ValueError):
            pass
    for d in outl:
        h = health.get(d['disk']) or {}
        d['healthy'] = h.get('ok', True) is not False
    return outl


def net_bytes():
    w = world()
    if w is not None:
        return w.get('net') or [0, 0]
    rx = tx = 0
    try:
        with open('/proc/net/dev') as f:
            for l in f.readlines()[2:]:
                name, rest = l.split(':', 1)
                if name.strip() in ('lo',) or name.strip().startswith(('veth', 'docker', 'podman', 'virbr')):
                    continue
                v = rest.split()
                rx += int(v[0])
                tx += int(v[8])
    except (OSError, ValueError, IndexError):
        pass
    return [rx, tx]


def power_points():
    w = world()
    if w is not None:
        return w.get('power') or []
    try:
        with open('/var/lib/hearth/power-history.json') as f:
            return json.load(f).get('points') or []
    except (OSError, ValueError):
        return []


def tailscale():
    w = world()
    if w is not None:
        return w.get('tailscale')
    if not C.which('tailscale'):
        return None
    code, out = C.run(['tailscale', 'status', '--json'], timeout=10)
    try:
        return json.loads(out) if code == 0 else None
    except ValueError:
        return None


def pid_unit(pid):
    try:
        with open('/proc/%d/cgroup' % pid) as f:
            t = f.read()
        m = re.findall(r'([\w@.\-]+\.service)', t)
        return m[-1] if m else ''
    except OSError:
        return ''


def du(paths):
    """A folder's size, cached for 6 hours (du at the lowest priority, 10 seconds at most)."""
    w = world()
    if w is not None:
        return None
    for p in paths:
        p = os.path.expanduser(p)
        if os.path.isdir(p):
            s = C.state(PART).get('du') or {}
            hit = s.get(p)
            if hit and hit[0] > time.time() - 6 * 3600:
                return hit[1]
            code, out = C.run(['nice', '-n', '19', 'du', '-sb', p], timeout=10)
            try:
                n = int(out.split()[0]) if code == 0 else None
            except (ValueError, IndexError):
                n = None
            if n is not None:
                C.state_update(PART, lambda d: dict(d, du=dict(d.get('du') or {}, **{p: [int(time.time()), n]})))
            return n
    return None


def other_part(part):
    """A part's own summary file (the parts write what tonight brings and how they're doing in their state)."""
    return C.state(part)


# ------------------------------------------------------------------ each service
def service_state(sv, prev, now):
    sid, name, what, mark, grad, group, unit, user, url, dirs, small = sv
    u = systemd(unit, user) if unit else {'exists': False}
    info = {'id': sid, 'name': name, 'what': what, 'mark': mark, 'grad': grad, 'group': group, 'small': small}
    extra = None
    if sid in ('games', 'cache'):
        s = other_part(sid)
        if not s.get('summary'):
            return None
        sm = s['summary']
        info.update(state=sm.get('state', 'ok'), what=sm.get('what', what), foot=sm.get('foot', ''), on=True)
        return info
    if sid == 'tailscale':
        ts = tailscale()
        if not ts:
            return None
        peers = list((ts.get('Peer') or {}).values())
        online = [p for p in peers if p.get('Online')]
        relayed = [p for p in online if not p.get('CurAddr') and p.get('Relay')]
        backend = ts.get('BackendState')
        info.update(state='ok' if backend == 'Running' else 'warn', on=True, what='%d devices%s' % (len(online), ''),
                    foot=('direct to all' if not relayed else 'direct to %d · %s via relay' % (len(online) - len(relayed), ', '.join(pretty(p.get('HostName', '')) for p in relayed[:2]))) if backend == 'Running' else 'Tailscale is %s' % (backend or 'off'))
        links = C.state('link').get('aiUsers') if C.state('link') else None
        if links:
            info['what'] += ' · %d using its AI' % links
        return info
    if not u.get('exists') and not url:
        return None
    code, body = (0, None)
    if url:
        path = {'ollama': '/api/ps', 'colibri': '/v1/models', 'jellyfin': '/System/Info/Public', 'comfyui': '/queue', 'kokoro': '/v1/models',
                'vaultwarden': '/alive', 'nextcloud': '/status.php', 'homeassistant': '/api/', 'shield': '/control/filtering/status', 'sunshine': '/'}.get(sid, '/')
        code, body = probe(url, path)
    if not u.get('exists') and not code:
        return None
    active = u.get('active') if u.get('exists') else ('active' if code else 'inactive')
    if active in ('inactive', 'deactivating') and not code:
        info.update(state='off', on=False, foot='off · it starts when you turn it on')
        return info
    healthy = active == 'active' and code in (200, 204, 401, 403)
    state = 'ok' if healthy else ('bad' if active == 'failed' or not code else 'warn')
    info.update(state=state, on=True)
    # its CPU since the last look, memory, graphics memory, disk
    cpu = None
    last = (prev.get('cpu') or {}).get(sid)
    if u.get('cpu_ns') and last and now > last[0]:
        cpu = max(0.0, (u['cpu_ns'] - last[1]) / 1e9 / (now - last[0]) / max(1, os.cpu_count() or 1) * 100)
    info['_cpu'] = [now, u.get('cpu_ns', 0)]
    info['cpu'] = '%d%%' % round(cpu) if cpu is not None else ('%d%%' % u['cpuPct'] if 'cpuPct' in u else '—')
    info['ram'] = size_gb(u.get('mem')) if u.get('mem') else '—'
    info['_ram'] = u.get('mem') or 0
    info['restarts'] = u.get('restarts', 0)
    info['gpuB'] = u.get('gpu', 0)
    d = du(dirs)
    info['disk'] = C.size_words(d).replace('.0 ', ' ') if d else u.get('disk', '—')
    # what it's doing, from its own answer
    if sid == 'ollama' and isinstance(body, dict):
        ms = body.get('models') or []
        vram = sum(int(m.get('size_vram') or 0) for m in ms)
        info['gpuB'] = info['gpuB'] or vram
        info['what'] = 'AI chat · ' + (', '.join(m.get('name', '') for m in ms[:2]) + ' loaded' if ms else 'idle')
        extra = 'idle' if not ms else '%d loaded' % len(ms)
    elif sid == 'colibri' and isinstance(body, dict):
        ms = [m.get('id') for m in body.get('data') or [] if isinstance(m, dict)]
        cs = C.state('colibri') or {}
        info['what'] = (ms[0] if ms else 'no model loaded') + (' · %s words/s' % cs['wps'] if cs.get('wps') else '')
        extra = 'Models in the vault · stops while you play' if C.state('vault').get('online') else 'stops while you play'
    elif sid == 'comfyui' and isinstance(body, dict):
        q = len(body.get('queue_running') or []) + len(body.get('queue_pending') or [])
        info['queue'] = q
        info['what'] = 'making %d picture%s' % (q, '' if q == 1 else 's') if q else 'waiting for a picture to make'
        extra = 'queue %d' % q
    elif sid == 'homeassistant':
        n_dev, lights = ha_counts()
        if n_dev is not None:
            info['what'] = '%d devices · %d lights on' % (n_dev, lights)
            info['tvLine'] = '%d lights on' % lights
    elif sid == 'shield' and isinstance(body, dict):
        lists = [f for f in body.get('filters') or [] if isinstance(f, dict) and f.get('enabled')]
        old = [f for f in lists if f.get('last_updated') and age_of(f.get('last_updated')) > STALE_LISTS]
        never = [f for f in lists if not f.get('last_updated')]
        days = int(max([age_of(f.get('last_updated')) for f in lists if f.get('last_updated')] or [0]) // 86400)
        if old or never:
            info['state'] = 'warn'
            info['foot'] = ('Block lists %d days old' % days if old else 'Block lists up to date') + (' · couldn\'t reach one' if never else '')
            info['fix'] = 'shield-lists'
            info['need'] = {'id': 'shield-lists', 'title': 'Hearth Shield\'s block lists are old', 'tvTitle': 'Hearth Shield\'s block lists are %d days old' % days,
                            'text': 'One list\'s address stopped answering%s. Fix it swaps it for its new address and updates all %d.' % ((' on ' + C.day(time.time() - days * 86400)) if days else '', len(lists)),
                            'tvText': 'One list moved to a new address. Press A and Hearth swaps it and updates all %s. Ads stay blocked meanwhile.' % words_n(len(lists)),
                            'show': 'shield'}
    elif sid == 'jellyfin' and isinstance(body, dict):
        watching = jellyfin_watching(url)
        if watching is None:
            info['what'] = 'TV and movies · %s' % (body.get('ServerName') or 'ready')
        else:
            info['what'] = 'TV and movies · %d watching%s' % (len(watching), (' (%s)' % ', '.join(watching[:2])) if watching else '')
            info['tvLine'] = '%d watching' % len(watching)
    # the foot: when its data was last backed up (Backups), and its button
    b = (C.state('backups').get('services') or {}).get(sid)
    if 'foot' not in info:
        parts_ = []
        if b:
            parts_.append('Backed up %s' % C.clock(b))
        if extra:
            parts_.append(extra)
        info['foot'] = ' · '.join(parts_) if parts_ else ('Backed up every night' if b else '')
    info['action'] = {'comfyui': ['Clear queue', 'clear-queue'], 'jellyfin': ['Rescan', 'rescan']}.get(sid, ['Restart', 'restart'])
    if info['state'] == 'bad':
        info['fix'] = 'restart:%s' % sid
        info['need'] = {'id': 'restart:%s' % sid, 'title': '%s isn\'t answering' % name, 'text': ('Its service %s. Fix it starts it again (its data stays as it is).' % ('stopped with an error' if active == 'failed' else 'is running but doesn\'t answer')), 'show': sid}
    return info


def jellyfin_watching(url):
    """Who's watching now (Jellyfin's /Sessions, with the API key you gave Hearth: hearth-app home-server key
    jellyfin, in the password store), or None when Hearth has no key."""
    w = world()
    if w is not None:
        v = (w.get('http') or {}).get(url + '/Sessions')
        if v is None:
            return None
    else:
        key = C.secret('jellyfin')
        if not key:
            return None
        st_, v = C.http_json('GET', url + '/Sessions?activeWithinSeconds=120', headers={'X-Emby-Token': key}, timeout=3)
        del key
        if st_ != 200:
            return None
    return [x.get('DeviceName') or x.get('Client') or 'a device' for x in v or [] if isinstance(x, dict) and x.get('NowPlayingItem')]


def words_n(n):
    return ['none', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten'][n] if 0 <= n <= 10 else str(n)


def age_of(iso):
    if not iso:
        return 10 ** 9
    try:
        import datetime
        t = datetime.datetime.fromisoformat(str(iso).replace('Z', '+00:00'))
        return time.time() - t.timestamp()
    except ValueError:
        return 10 ** 9


def ha_counts():
    """Home Assistant's devices and lights on, with the token Hearth Home keeps in your keyring (read only here)."""
    w = world()
    if w is not None:
        h = w.get('homeassistant') or {}
        return h.get('devices'), h.get('lights', 0)
    if not C.which('secret-tool'):
        return None, 0
    code, tok = C.run(['secret-tool', 'lookup', 'hearth-app', 'hearth-home', 'key', 'token'], timeout=10)
    tok = tok.strip()
    if code != 0 or not tok:
        return None, 0
    st_, j = C.http_json('GET', 'http://127.0.0.1:8123/api/states', headers={'Authorization': 'Bearer ' + tok}, timeout=5)
    del tok
    if st_ != 200 or not isinstance(j, list):
        return None, 0
    ents = [e for e in j if isinstance(e, dict)]
    devs = len({e.get('entity_id', '').split('.', 1)[-1].rsplit('_', 1)[0] for e in ents if e.get('entity_id', '').split('.')[0] in ('light', 'switch', 'sensor', 'climate', 'camera', 'media_player', 'lock', 'cover', 'fan')})
    lights = sum(1 for e in ents if e.get('entity_id', '').startswith('light.') and e.get('state') == 'on')
    return devs, lights


def size_gb(b):
    if not b:
        return '—'
    if b >= 1e9:
        return ('%.1f GB' % (b / 1e9)).replace('.0 GB', ' GB') if b < 1e11 else '%d GB' % (b / 1e9)
    return '%d MB' % (b / 1e6)


PRETTY = {'rog': 'ROG', 'tcl': 'TCL', 'pc': 'PC', 'tv': 'TV', 'ally': 'Ally', 'xbox': 'Xbox', 'nvidia': 'Nvidia', 'dads': 'Dad\'s', 'dad': 'Dad\'s'}


def pretty(host):
    words = [w for w in re.split(r'[-_ ]+', host or '') if w]
    return ' '.join(PRETTY.get(w.lower(), w.capitalize() if i == 0 else w.lower()) for i, w in enumerate(words))


def devices(cfg_, playing_on=''):
    ts = tailscale()
    if not ts:
        return []
    names = cfg_.get('names') or {}
    outl = []
    for p in (ts.get('Peer') or {}).values():
        host = p.get('HostName') or p.get('DNSName', '').split('.')[0]
        name = names.get(host) or pretty(host)
        note = ''
        stt = 'ok'
        if not p.get('Online'):
            note, stt = 'off', 'off'
        elif host == playing_on:
            note = 'playing'
        elif p.get('CurAddr') and not re.match(r'(10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.|\[?fd|\[?fe80)', p['CurAddr']):
            note = 'away'
        elif not p.get('CurAddr') and p.get('Relay'):
            note = 'away'
        outl.append({'name': name, 'note': note, 'state': stt, 'host': host})
    return outl


# ------------------------------------------------------------------ history (sampled while the dashboard is open)
def sample(prev, now, cpu, gpu, mem, ctemp, net):
    """One point every minute at most, kept for 7 days (light: only while the Dashboard or its tile is looked at)."""
    h = prev.get('hist') or []
    if h and now - h[-1][0] < 55:
        return h
    pct = None
    pc = prev.get('cpuRaw')
    if cpu.get('all') and pc and cpu['all'] > pc[1]:
        pct = (cpu['busy'] - pc[0]) / float(cpu['all'] - pc[1]) * 100
    if cpu.get('pct') is not None:
        pct = cpu['pct']
    point = [int(now), round(pct or 0, 1), round((gpu or {}).get('pct') or 0, 1), round((gpu or {}).get('temp') or 0), round(ctemp or 0), net[0], net[1],
             round(((mem['total'] - mem['avail']) / mem['total'] * 100) if mem.get('total') else 0, 1), game_now()]
    return (h + [point])[-10080:]


def game_now():
    """The game running on this PC now, by name (a Steam game's launch line and its appmanifest), or ""."""
    w = world()
    if w is not None:
        return w.get('game', '')
    try:
        pids = [p for p in os.listdir('/proc') if p.isdigit()]
    except OSError:
        return ''
    for pid in pids:
        try:
            with open('/proc/%s/cmdline' % pid, 'rb') as f:
                cmd = f.read(4096).decode('utf-8', 'replace')
        except OSError:
            continue
        m = re.search(r'SteamLaunch\x00AppId=(\d+)', cmd)
        if m:
            for lib in (os.path.join(C.HOME, '.local/share/Steam/steamapps'), os.path.join(C.HOME, '.steam/steam/steamapps')):
                try:
                    with open(os.path.join(lib, 'appmanifest_%s.acf' % m.group(1)), encoding='utf-8', errors='replace') as f:
                        n = re.search(r'"name"\s+"([^"]+)"', f.read())
                    if n:
                        return n.group(1)
                except OSError:
                    continue
            return ''
    return ''


def window(h, secs, now, idx, scale=100.0, n=48):
    pts = [p for p in h if p[0] >= now - secs]
    if not pts:
        return []
    step = max(1, len(pts) // n)
    return [min(1.0, max(0.0, p[idx] / scale)) for p in pts[::step]][-n:]


# ------------------------------------------------------------------ everything
def status(rng='24h', write_history=True):
    c = cfg()
    now = time.time()
    prev = C.state(PART)
    cpu = cpu_sample()
    gpu = nvidia()
    mem = meminfo()
    ctemp = temps()
    net = net_bytes()
    svcs = []
    cpu_marks = {}
    for sv in SERVICES:
        x = service_state(sv, prev, now)
        if x is None:
            continue
        if '_cpu' in x:
            cpu_marks[x['id']] = x.pop('_cpu')
        svcs.append(x)
    # graphics memory by service (nvidia-smi's processes → their units)
    if gpu and gpu.get('procs'):
        by_unit = {}
        for pid, b in gpu['procs'].items():
            by_unit[pid_unit(pid)] = by_unit.get(pid_unit(pid), 0) + b
        for x in svcs:
            unit = next((s[6] for s in SERVICES if s[0] == x['id']), '')
            if unit in by_unit:
                x['gpuB'] = by_unit[unit]
    for x in svcs:
        if not x.get('small'):
            x['gpu'] = size_gb(x.get('gpuB')) if x.get('gpuB') else ('0 GB' if x['id'] == 'ollama' else '—')
    hist = sample(prev, now, cpu, gpu, mem, ctemp, net) if write_history else (prev.get('hist') or [])
    if write_history:
        C.state_update(PART, lambda d: dict(d, hist=hist, cpu=dict(d.get('cpu') or {}, **cpu_marks), cpuRaw=[cpu.get('busy', 0), cpu.get('all', 0)] if cpu.get('all') else d.get('cpuRaw')))
    secs = {'1h': 3600, '24h': 86400, '7d': 7 * 86400}.get(rng, 86400)
    cpu_pct = hist[-1][1] if hist else 0
    on = [x for x in svcs if x.get('on')]
    healthy = [x for x in on if x['state'] == 'ok']
    needs = [x['need'] for x in on if x.get('need')]
    ds = drives()
    d_used = sum(d['used'] for d in ds)
    d_total = sum(d['total'] for d in ds)
    bad_drives = [d for d in ds if not d.get('healthy', True)]
    if bad_drives:
        needs.append({'id': 'drives', 'title': 'A drive\'s health check failed', 'text': 'Drives & undo shows which one. Copy what\'s on it to another drive soon.', 'show': 'drives', 'nofix': True})
    if gpu and gpu.get('temp', 0) >= 85:
        needs.append({'id': 'gpu-hot', 'title': 'The graphics card is hot (%d°)' % gpu['temp'], 'text': 'Free up GPU memory stops what\'s using it most.', 'show': 'gpu', 'fix': 'gpu-free'})
    bk = C.state('backups')
    for f in bk.get('failed') or []:
        needs.append({'id': 'backup:%s' % f.get('id', ''), 'title': '%s didn\'t back up last night' % f.get('name', 'A device'), 'text': f.get('why', 'It tries again tonight.'), 'show': 'backup', 'nofix': True})
    # the big numbers
    holders = sorted([x for x in on if x.get('_ram')], key=lambda x: -x['_ram'])
    gpu_users = [x['name'].split(' · ')[0] for x in sorted([x for x in on if x.get('gpuB')], key=lambda x: -x['gpuB'])]
    kpis = [
        {'k': 'CPU · %s' % cpu_short(cpu.get('name', '')), 'v': '%d%%' % round(cpu_pct), 'spark': window(hist, secs, now, 1), 'color': '#ff8a3d'},
        {'k': 'GPU · %s' % ((gpu or {}).get('name') or 'none'), 'v': '%d%%' % round((gpu or {}).get('pct') or 0) if gpu else '—', 'spark': window(hist, secs, now, 2), 'color': '#5ec8ff'},
        {'k': 'GPU memory', 'v': '%.1f' % ((gpu or {}).get('used', 0) / 1e9) if gpu else '—', 'of': '/ %d GB' % round(gpu['total'] / 1e9) if gpu and gpu.get('total') else '',
         'bar': gpu['used'] / gpu['total'] if gpu and gpu.get('total') else 0, 's': ' · '.join(gpu_users[:3]) or 'nothing on it'},
        {'k': 'Memory', 'v': '%.1f' % ((mem['total'] - mem['avail']) / 1e9), 'of': '/ %d GB' % round(mem['total'] / 1e9) if mem.get('total') else '',
         'bar': (mem['total'] - mem['avail']) / mem['total'] if mem.get('total') else 0, 's': ('%s holds %s of it' % (holders[0]['name'].split(' · ')[0], size_gb(holders[0]['_ram']))) if holders else ''},
        {'k': 'Drives', 'v': ('%.1f' % (d_used / 1e12)) if d_total >= 1e12 else '%d' % (d_used / 1e9), 'of': ('/ %d TB' % round(d_total / 1e12)) if d_total >= 1e12 else '/ %d GB' % (d_total / 1e9),
         'bar': d_used / d_total if d_total else 0, 'green': True, 's': '%d drives · %s' % (len(ds), 'all healthy' if not bad_drives else '%d need a look' % len(bad_drives))},
        {'k': 'Temperatures', 'v': '%d°' % round(gpu['temp']) if gpu and gpu.get('temp') else ('%d°' % round(ctemp) if ctemp else '—'),
         'of': ('GPU · %d° CPU' % round(ctemp) if ctemp else 'GPU') if gpu and gpu.get('temp') else 'CPU', 'spark': window(hist, secs, now, 3), 'color': '#ffd36b', 'nofill': True},
    ]
    # earlier today: what the parts said, and services that restarted themselves
    midnight = time.mktime(tuple(time.localtime(now)[:3]) + (0, 0, 0, 0, 0, -1))
    ev = C.events(since=int(now - 86400 * 1.5))
    earlier = []
    for e in sorted(ev, key=lambda e: -e['at'])[:6]:
        earlier.append({'text': e['text'], 'when': C.clock(e['at']).replace(' AM', '').replace(' PM', '') if e['at'] >= midnight else 'Yest.', 'tone': e.get('tone', 'ok')})
    for x in on:
        if x.get('restarts') and not any(x['name'].split(' · ')[0] in e['text'] for e in earlier):
            earlier.append({'text': '%s restarted itself %d time%s' % (x['name'].split(' · ')[0], x['restarts'], '' if x['restarts'] == 1 else 's'), 'when': 'today', 'tone': 'ok'})
    # one-tap fixes
    q = next((x.get('queue', 0) for x in svcs if x['id'] == 'comfyui'), 0)
    fixes = [
        {'id': 'gpu-free', 'label': 'Free up GPU memory', 'note': 'stops Imagine\'s queue' if q else 'unloads the AI models'},
        {'id': 'restart-all', 'label': 'Restart everything safely', 'note': 'one at a time'},
        {'id': 'caches', 'label': 'Clear old caches', 'note': 'frees %s' % C.size_words(cache_size()).replace('.0 ', ' ')},
        {'id': 'logs', 'label': 'Send logs', 'note': 'to your hearth-logs'},
    ]
    # network, power, tonight
    pts = [p for p in hist if p[0] >= now - secs]
    down = (pts[-1][5] - pts[0][5]) if len(pts) > 1 and pts[-1][5] >= pts[0][5] else 0
    up = (pts[-1][6] - pts[0][6]) if len(pts) > 1 and pts[-1][6] >= pts[0][6] else 0
    rates = []
    for a, b in zip(pts, pts[1:]):
        dt = max(1, b[0] - a[0])
        rates.append(max(0, (b[5] - a[5]) + (b[6] - a[6])) / dt)
    top = max(rates or [1]) or 1
    net_spark = [r / top for r in rates[:: max(1, len(rates) // 48)]][-48:] if rates else []
    pw = [p for p in power_points() if p[0] >= now - 86400]
    watts = [(p[1] or 0) + (p[2] or 0) for p in pw]
    kwh = sum(w * 300 for w in watts) / 3.6e6 if watts else 0
    now_w = watts[-1] if watts else ((gpu or {}).get('watts') or 0)
    buckets = []
    if watts:
        n = 36
        step = max(1, len(watts) // n)
        buckets = [sum(watts[i:i + step]) / step for i in range(0, len(watts), step)][-n:]
        mx = max(buckets) or 1
        buckets = [b / mx for b in buckets]
    tonight = []
    froms = []
    for part, label in (('backups', 'Backups'), ('games', 'Game servers'), ('vault', 'the Model vault'), ('media', 'Media')):
        for t in C.state(part).get('tonight') or []:
            tonight.append(t)
            if label not in froms:
                froms.append(label)
    tonight.sort(key=lambda t: t.get('order', 0))
    temps24 = window(hist, 86400, now, 3)
    peak = max((p for p in hist if p[0] >= now - 86400), key=lambda p: p[3], default=None)
    tv = tv_parts(svcs, kpis, gpu, ctemp, now)
    return {
        'ok': True, 'healthy': len(healthy), 'total': len(on), 'needsCount': len(needs), 'updated': 'updated every 2 seconds', 'tv': tv,
        'pc': os.environ.get('HEARTH_SERVER_NAME') or server_name(gpu), 'kpis': kpis, 'services': svcs, 'needs': needs, 'earlier': earlier[:4], 'fixes': fixes,
        'temps24': temps24, 'tempNote': ('GPU peak %d° at %s%s' % (peak[3], C.clock(peak[0]), (' (%s)' % peak[8]) if len(peak) > 8 and peak[8] else '')) if peak and peak[3] else '',
        'network': {'line': 'down %s · up %s' % (gb_short(down), gb_short(up)), 'spark': net_spark},
        'power': {'line': 'now %d W · %.1f kWh · about %s' % (now_w, kwh, C.money(kwh * float(c.get('price') or 0.14))), 'bars': buckets, 'now': '%d W' % now_w, 'kwh': '%.1f kWh · about %s' % (kwh, C.money(kwh * float(c.get('price') or 0.14)))},
        'tonight': tonight[:4], 'tonightFrom': 'from %s' % ' and '.join(froms) if froms else 'nothing planned tonight',
        'alerts': {'phone': bool(c.get('phone')), 'quiet': bool(c.get('quiet')), 'quietLine': 'Quiet while you\'re at work, %s–%s' % (hour_words(c.get('quietFrom', 22)), hour_words(c.get('quietTo', 6)))},
        'range': rng, 'playing': playing_now(), 'devices': devices(c, (playing_now() or {}).get('device', '')),
    }


TV_LINE = {'ollama': lambda x: x.get('tvLine') or ('idle' if 'idle' in x.get('what', '') else 'loaded'),
           'colibri': lambda x: x.get('what', '').split(' · ')[-1] if 'words/s' in x.get('what', '') else 'ready',
           'comfyui': lambda x: '%d in queue' % x.get('queue', 0),
           'shield': lambda x: 'fix ready' if x.get('state') == 'warn' else 'blocking'}


def tv_parts(svcs, kpis, gpu, ctemp, now):
    """What the TV view and the phone show beyond the desk's: each service's few words, the PC's bars, right now,
    the game servers and last night's backups (from those parts' own state), so every view is the same blocks."""
    on = [x for x in svcs if x.get('on') and not x.get('small')]
    first = ['jellyfin', 'colibri', 'comfyui', 'homeassistant', 'shield', 'vaultwarden', 'ollama']
    order = sorted(on, key=lambda x: first.index(x['id']) if x['id'] in first else 9)
    top = []
    for x in order[:5]:
        line = x.get('tvLine') or (TV_LINE[x['id']](x) if x['id'] in TV_LINE else ('ready' if x['state'] == 'ok' else 'needs a look'))
        top.append({'name': x['name'].replace('Imagine · ComfyUI', 'Imagine').replace('Home · Home Assistant', 'Home Assistant').replace('Hearth Shield · AdGuard Home', 'Hearth Shield'), 'short': x['name'].split(' · ')[0].replace('Home', 'Home Assistant') if x['id'] == 'homeassistant' else x['name'].split(' · ')[0], 'line': line, 'state': x['state']})
    rest = [x for x in svcs if x.get('on')][len(top):]
    k = {x['k'].split(' · ')[0]: x for x in kpis}
    bars = [['CPU', k.get('CPU', {}).get('v', '—'), (float(k.get('CPU', {}).get('v', '0%').rstrip('%') or 0) / 100) if k.get('CPU', {}).get('v', '—') != '—' else 0],
            ['GPU', k.get('GPU', {}).get('v', '—'), (float(k.get('GPU', {}).get('v', '0%').rstrip('%') or 0) / 100) if k.get('GPU', {}).get('v', '—') not in ('—', '') else 0],
            ['GPU memory', '%s GB' % k.get('GPU memory', {}).get('v', '—'), k.get('GPU memory', {}).get('bar', 0)],
            ['Memory', '%d GB' % round(float(k.get('Memory', {}).get('v', '0') or 0)), k.get('Memory', {}).get('bar', 0)],
            ['Drives', '%d%%' % round((k.get('Drives', {}).get('bar') or 0) * 100), k.get('Drives', {}).get('bar', 0)]]
    nowl = []
    play = C.state('play').get('now')
    if isinstance(play, dict):
        nowl.append({'mark': 'RP', 'grad': 'or', 'title': play.get('title', 'Streaming'), 'line': play.get('line', '')})
    for sv in (C.state('games').get('servers') or [])[:1]:
        if sv.get('on'):
            nowl.append({'mark': 'GS', 'grad': 'gr', 'title': '%s server' % sv.get('name', ''), 'line': '%s online' % sv.get('players', '')})
    cache = C.state('cache').get('now')
    if isinstance(cache, dict):
        nowl.append({'mark': 'GC', 'grad': 'yl', 'title': 'Game cache', 'line': cache.get('line', '')})
    bk = C.state('backups')
    nb = next(iter(bk.get('tonight') or []), None)
    if nb:
        nowl.append({'mark': 'BK', 'grad': 'bl', 'title': 'Next backup', 'line': nb['text'].split(' · ')[0] + ' · ' + nb['text'].split(' · ')[-1].replace('back up ', '').replace(' and this server\'s services', '')})
    games = [{'name': g.get('name', ''), 'line': g.get('line', ''), 'on': bool(g.get('on')), 'id': g.get('id', '')} for g in (C.state('games').get('servers') or [])[:3]]
    steps = bk.get('steps') or []
    rows = []
    devs = [x for x in steps if 'server' not in x.get('what', '') and 'R2' not in x.get('what', '')]
    for a, b_ in zip(devs[0::2], devs[1::2] + [None]):
        rows.append({'what': a['what'].split(' · ')[0] + (' · ' + b_['what'].split(' · ')[0] if b_ else ''), 'when': C.clock(a['at']).replace(' AM', '').replace(' PM', '') + (' · ' + C.clock(b_['at']) if b_ else '')})
    srv = [x for x in steps if 'server' in x.get('what', '')]
    if srv:
        rows.append({'what': srv[0]['what'].replace('server\'s ', ''), 'when': C.clock(srv[0]['at'])})
    keep = max([int(dict(d).get('nights', 30)) for d in (C.settings('backups', {'devices': {}}).get('devices') or {}).values()] or [30])
    total = sum((bk.get('sizes') or {}).values())
    return {'services': top, 'more': len(rest), 'moreOk': all(x['state'] == 'ok' for x in rest), 'bars': bars, 'now': nowl, 'games': games,
            'backups': rows[:3], 'backupsLine': 'Kept %d nights · %s in all' % (keep, C.tb_words(total / 1e9)) if total else 'Kept %d nights' % keep,
            'temp': {'big': '%d°' % round(gpu['temp']) if gpu and gpu.get('temp') else '—', 'line': ('%d° CPU · fans quiet' % round(ctemp)) if ctemp else ''},
            'clock': C.clock(now)}


def hour_words(h):
    h = int(h) % 24
    return '%d %s' % (h % 12 or 12, 'AM' if h < 12 else 'PM')


def gb_short(b):
    return '%d GB' % round(b / 1e9) if b >= 1e9 else '%d MB' % round(b / 1e6)


def server_name(gpu):
    import socket
    host = socket.gethostname()
    return host


def playing_now():
    """What's being played or streamed from this PC right now (Remote play writes its state; Game Mode's mark)."""
    rp = C.state('play') or {}
    if rp.get('now'):
        return rp['now']
    return None


CACHES = ['~/.cache/pip', '~/.cache/thumbnails', '~/.cache/huggingface/xet', '~/.cache/mesa_shader_cache_db']


def cache_size():
    w = world()
    if w is not None:
        return w.get('caches', 0)
    s = C.state(PART).get('cacheSize')
    if s and s[0] > time.time() - 6 * 3600:
        return s[1]
    total = 0
    for p in CACHES:
        p = os.path.expanduser(p)
        if os.path.isdir(p):
            code, out = C.run(['nice', '-n', '19', 'du', '-sb', p], timeout=10)
            try:
                total += int(out.split()[0]) if code == 0 else 0
            except (ValueError, IndexError):
                pass
    C.state_update(PART, lambda d: dict(d, cacheSize=[int(time.time()), total]))
    return total


# ------------------------------------------------------------------ the fixes
def svc_of(sid):
    return next((s for s in SERVICES if s[0] == sid), None)


def restart(sid):
    sv = svc_of(sid)
    if not sv or not sv[6]:
        raise Fail('That service can\'t be restarted from here')
    if FIXTURE:
        return {'ok': True, 'said': '%s restarted' % sv[1]}
    if sv[7]:
        code, _ = C.run(['systemctl', '--user', 'restart', sv[6]], timeout=90)
    elif sid in ('ollama', 'jellyfin'):
        code, _ = C.run(['pkexec', '/usr/libexec/nimbus/hearth-home-server-helper', 'svc', 'restart', sid], timeout=120)
    elif sid == 'shield':
        code, _ = C.run(['pkexec', '/usr/libexec/nimbus/hearth-server-dns', 'restart'], timeout=120)
    else:
        raise Fail('That one restarts from its own page')
    if code != 0:
        raise Fail('%s didn\'t start again: its own page says why' % sv[1])
    C.event(PART, '%s restarted from the dashboard' % sv[1].split(' · ')[0])
    return {'ok': True, 'said': '%s restarted' % sv[1].split(' · ')[0]}


# Block lists that moved: their old address → their new one (Hearth's own list of known moves; it updates with this part).
MOVED_LISTS = {
    'https://adaway.org/hosts.txt': 'https://raw.githubusercontent.com/AdAway/adaway.github.io/master/hosts.txt',
    'https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts': 'https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts',
    'https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=1&mimetype=plaintext': 'https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext',
}


def shield_lists():
    """Each list that doesn't answer is swapped for its new address (when Hearth knows it), then all of them are
    fetched again by AdGuard Home itself. Ads stay blocked meanwhile (the old lists stay loaded until the new ones are in)."""
    base = 'http://127.0.0.1:3053'
    st_, j = probe(base, '/control/filtering/status')
    if st_ != 200 or not isinstance(j, dict):
        raise Fail('Hearth Shield isn\'t answering: restart it first')
    swapped = 0
    for f in j.get('filters') or []:
        url = f.get('url', '')
        if url in MOVED_LISTS and MOVED_LISTS[url] != url:
            if not FIXTURE:
                C.http_json('POST', base + '/control/filtering/set_url', {'url': url, 'whitelist': False, 'data': {'name': f.get('name', ''), 'url': MOVED_LISTS[url], 'enabled': True}})
            swapped += 1
    if not FIXTURE:
        st2, _ = C.http_json('POST', base + '/control/filtering/refresh', {'whitelist': False}, timeout=120)
        if st2 not in (200, 204):
            raise Fail('AdGuard Home didn\'t fetch its lists again (%s)' % (st2 or 'no answer'))
    C.event(PART, 'Hearth Shield\'s block lists updated%s' % (' (%d moved list swapped)' % swapped if swapped else ''))
    return {'ok': True, 'said': 'Hearth Shield\'s lists are fresh again%s' % (': %d moved list swapped for its new address' % swapped if swapped else '')}


def gpu_free():
    """The AI models unloaded (Ollama: keep_alive 0; ComfyUI: /free), ComfyUI's queue stopped. Nothing is deleted."""
    if FIXTURE:
        return {'ok': True, 'said': 'GPU memory freed'}
    st_, j = C.http_json('GET', 'http://127.0.0.1:11434/api/ps', timeout=3)
    for m in (j or {}).get('models') or []:
        C.http_json('POST', 'http://127.0.0.1:11434/api/generate', {'model': m.get('name'), 'keep_alive': 0}, timeout=20)
    C.http_json('POST', 'http://127.0.0.1:8188/queue', {'clear': True}, timeout=5)
    C.http_json('POST', 'http://127.0.0.1:8188/interrupt', {}, timeout=5)
    C.http_json('POST', 'http://127.0.0.1:8188/free', {'unload_models': True, 'free_memory': True}, timeout=10)
    C.event(PART, 'GPU memory freed from the dashboard')
    return {'ok': True, 'said': 'GPU memory freed: the models load again when they\'re next asked for'}


def restart_all():
    done = []
    for sv in SERVICES:
        if not sv[6] or not sv[7]:
            continue
        u = systemd(sv[6], sv[7])
        if u.get('active') == 'active':
            try:
                restart(sv[0])
                done.append(sv[1].split(' · ')[0])
                time.sleep(2)
            except Fail:
                pass
    return {'ok': True, 'said': 'Restarted one at a time: %s' % (', '.join(done) or 'nothing was running')}


def caches():
    freed = 0
    for p in CACHES:
        p = os.path.expanduser(p)
        if not os.path.isdir(p) or FIXTURE:
            continue
        for root, dirs, files in os.walk(p, topdown=False):
            for f in files:
                fp = os.path.join(root, f)
                try:
                    if os.path.islink(fp):
                        continue
                    if time.time() - os.path.getatime(fp) > 7 * 86400:
                        freed += os.path.getsize(fp)
                        os.unlink(fp)
                except OSError:
                    pass
    C.state_update(PART, lambda d: dict(d, cacheSize=None))
    return {'ok': True, 'said': 'Old caches cleared: %s freed (only caches unused for a week)' % C.size_words(freed)}


def send_logs():
    if FIXTURE:
        return {'ok': True, 'said': 'Logs sent'}
    code, _ = C.run(['nimbus', '--send-logs', 'Sent from the Home server dashboard'], timeout=120)
    if code != 0:
        raise Fail('The logs couldn\'t be sent (Settings › Updates & recovery › Send logs says why)')
    return {'ok': True, 'said': 'Logs sent to your hearth-logs'}


def fix(fid):
    if fid == 'all':
        st_ = status(write_history=False)
        said = []
        for n in st_['needs']:
            if n.get('nofix'):
                continue
            try:
                said.append(fix(n.get('fix') or n['id'])['said'])
            except Fail as e:
                said.append(str(e))
        return {'ok': True, 'said': ' · '.join(said) if said else 'Nothing needs fixing'}
    if fid.startswith('restart:'):
        return restart(fid.split(':', 1)[1])
    f = {'shield-lists': shield_lists, 'gpu-free': gpu_free, 'restart-all': restart_all, 'caches': caches, 'logs': send_logs}.get(fid)
    if not f:
        raise Fail('Not one of the dashboard\'s fixes')
    return f()


def action(sid, what):
    if what == 'restart':
        return restart(sid)
    if sid == 'comfyui' and what == 'clear-queue':
        if not FIXTURE:
            C.http_json('POST', 'http://127.0.0.1:8188/queue', {'clear': True}, timeout=5)
        return {'ok': True, 'said': 'Imagine\'s queue is empty'}
    if sid == 'jellyfin' and what == 'rescan':
        if not FIXTURE:
            code, _ = C.run(['pkexec', '/usr/libexec/nimbus/hearth-home-server-helper', 'svc', 'restart', 'jellyfin'], timeout=120)
        return {'ok': True, 'said': 'Jellyfin looks through your library again as it starts'}
    raise Fail('Not one of that service\'s buttons')


# ------------------------------------------------------------------ alerts (a user timer, off until you turn it on)
WATCH_UNITS = {
    'hearth-server-watch.service': '[Unit]\nDescription=Hearth home server: anything that needs a look, to your phone\n\n[Service]\nType=oneshot\nNice=10\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part dash watch\n',
    'hearth-server-watch.timer': '[Unit]\nDescription=Hearth home server: look every 10 minutes\n\n[Timer]\nOnBootSec=10min\nOnUnitActiveSec=10min\n\n[Install]\nWantedBy=timers.target\n',
}
UPDATE_UNITS = {
    'hearth-server-updates.service': '[Unit]\nDescription=Hearth home server: server apps updated from the Hearth Store\n\n[Service]\nType=oneshot\nNice=10\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part dash autoupdate run\n',
    'hearth-server-updates.timer': '[Unit]\nDescription=Hearth home server: look for server app updates every 6 hours\n\n[Timer]\nOnBootSec=30min\nOnUnitActiveSec=6h\nRandomizedDelaySec=20min\n\n[Install]\nWantedBy=timers.target\n',
}


def user_timer(units, timer, on):
    d = os.path.join(C.CONFIG, 'systemd', 'user')
    if on:
        for name, text in units.items():
            C._atomic(os.path.join(d, name), text)
    if C.which('systemctl') and not os.environ.get('HEARTH_TEST_NO_SYSTEMCTL'):
        C.run(['systemctl', '--user', 'daemon-reload'])
        C.run(['systemctl', '--user', 'enable' if on else 'disable', '--now', timer])
    if not on:
        for name in units:
            try:
                os.unlink(os.path.join(d, name))
            except OSError:
                pass


def quiet_now(c, now=None):
    if not c.get('quiet'):
        return False
    h = time.localtime(now or time.time()).tm_hour
    a, b = int(c.get('quietFrom', 22)), int(c.get('quietTo', 6))
    return (a <= h or h < b) if a > b else (a <= h < b)


def watch(now=None):
    c = cfg()
    s = status(write_history=False)
    sent = C.state(PART).get('said') or {}
    now = now or time.time()
    new = [n for n in s['needs'] if now - int(sent.get(n['id'], 0)) > 86400]
    for n in new:
        C.notify('%s. %s' % (n['title'], n['text']), 'Home server')
        if c.get('phone') and not quiet_now(c, now):
            C.phone('%s. %s' % (n['title'], n['text']), 'Home server')
        sent[n['id']] = int(now)
    C.state_update(PART, lambda d: dict(d, said=sent))
    return {'ok': True, 'new': len(new)}


def store_ready():
    """Store parts with an update ready (the Store's own last signed list, read as the Store reads it)."""
    p = os.path.join(C.CACHE_HOME, 'hearth', 'hearth-store', 'catalog.json')
    try:
        with open(p, encoding='utf-8') as f:
            cat = json.load(f)
    except (OSError, ValueError):
        return []
    outl = []
    for e in cat.get('apps') or []:
        part = e.get('part') if isinstance(e.get('part'), dict) else None
        if not part or part.get('id') not in ('hearth-home-server-helper', 'hearth-home-server-app'):
            continue
        outl.append(part)
    return outl


def autoupdate_run():
    """Only when nobody's playing or watching (when you keep that on): the window's own part through hearth-part (no
    password: it's yours); the system part needs your password, so it's said instead and waits for you."""
    c = cfg()
    if not c.get('auto'):
        return {'ok': True, 'said': 'Off'}
    if c.get('onlyIdle', True) and (C.playing() or (C.state('play') or {}).get('now')):
        return {'ok': True, 'said': 'Waiting: someone is playing or watching'}
    said = []
    for part in store_ready():
        if part.get('system'):
            C.notify('An update for the server\'s parts is ready in the Hearth Store: it asks for your password once', 'Home server')
            said.append('the system part waits for your password')
            continue
        hp = '/usr/libexec/hearth/hearth-part'
        if os.path.exists(hp) and not FIXTURE:
            code, _ = C.run([hp, 'install', part['id'], part['version'], part['url'], part['sha256'], part['os'], part['path']], timeout=900)
            said.append('%s %s' % (part['id'], 'updated' if code == 0 else 'didn\'t update'))
    return {'ok': True, 'said': ', '.join(said) or 'Nothing to update'}


def set_choice(key, value):
    if key in ('phone', 'quiet', 'onlyIdle'):
        C.set_key(PART, key, value == 'on')
        if key == 'phone':
            user_timer(WATCH_UNITS, 'hearth-server-watch.timer', value == 'on')
    elif key == 'price':
        try:
            v = float(value)
        except ValueError:
            raise Fail('A price like 0.14')
        if not 0 <= v < 5:
            raise Fail('A price like 0.14')
        C.set_key(PART, 'price', v)
    else:
        raise Fail('phone, quiet, onlyIdle or price')
    return {'ok': True}


# ------------------------------------------------------------------ Server home's tile and the overall line
def summary():
    c = cfg()
    s = status(write_history=True)
    gpu = next((k for k in s['kpis'] if k['k'].startswith('GPU ·')), {})
    temp = next((k for k in s['kpis'] if k['k'] == 'Temperatures'), {})
    drv = next((k for k in s['kpis'] if k['k'] == 'Drives'), {})
    pcs = C.state('pcs').get('count') or 1
    devs = s['devices']
    need = s['needs'][0] if s['needs'] else None
    bits = []
    if need:
        bits.append(need['title'].replace(' are old', ' need a fix').replace('isn\'t answering', 'needs a look'))
    bk = C.state('backups')
    if bk.get('lastNight'):
        bits.append('last backups %s' % C.clock(bk['lastNight']))
    playing = [d for d in devs if d['note'] == 'playing']
    if playing:
        bits.append('%d playing on the %s' % (len(playing), playing[0]['name'].replace('TCL 55" ', '')))
    ts = tailscale()
    if ts and ts.get('BackendState') == 'Running':
        bits.append('all reached privately through Tailscale')
    power = s['power']['now']
    overall = {
        'headline': '%d of %d services healthy · %d PC%s · %d device%s' % (s['healthy'], s['total'], pcs, '' if pcs == 1 else 's', len(devs), '' if len(devs) == 1 else 's'),
        'line': ' · '.join(bits), 'state': 'ok' if s['healthy'] >= s['total'] - 1 else ('warn' if s['healthy'] >= s['total'] - 3 else 'bad'),  # board 11: green with one thing to fix
        'healthy': s['healthy'], 'total': s['total'], 'pcs': pcs, 'needs': s['needs'], 'devices': devs,
        'devicesLine': 'all on Tailscale · Hearth Link shares the AI' if devs else '',
        'server': {'name': '%s · %s' % (s['pc'], gpu.get('k', 'GPU').split(' · ', 1)[-1]) if gpu else s['pc'], 'state': 'ok',
                   'lines': ['Hearth OS · up %s' % uptime_words(), 'Reached over Tailscale only' if ts else 'At home only']},
        'updates': {'auto': bool(c.get('auto')), 'idle': bool(c.get('onlyIdle', True))},
    }
    hist = [k for k in s['kpis'] if k['k'].startswith('CPU')][0].get('spark') or []
    return {'ok': True, 'big': str(s['healthy']), 'of': 'of %d healthy' % s['total'],
            'lines': ['GPU %s · %s · drives %d%% · power %s' % (gpu.get('v', '—'), temp.get('v', '—'), round((drv.get('bar') or 0) * 100), power), 'Desk, TV, phone and Ally views'],
            'alert': '', 'spark': hist[-30:] or [0.3, 0.35, 0.3], 'count': '%d ok' % s['healthy'], 'state': overall['state'], 'overall': overall}


def uptime_words():
    w = world()
    if w is not None:
        secs = w.get('uptime', 0)
    else:
        try:
            secs = float(open('/proc/uptime').read().split()[0])
        except (OSError, ValueError):
            secs = 0
    d = int(secs // 86400)
    return '%d day%s' % (d, '' if d == 1 else 's') if d else '%d hours' % int(secs // 3600)


def main(argv):
    if not argv:
        raise Fail('status, summary, fix, action, set, alerts, watch or autoupdate')
    cmd, rest = argv[0], argv[1:]
    if cmd == 'status':
        return status(rest[0] if rest else '24h')
    if cmd == 'summary':
        return summary()
    if cmd == 'fix' and len(rest) == 1:
        return fix(rest[0])
    if cmd == 'action' and len(rest) == 2:
        return action(rest[0], rest[1])
    if cmd == 'set' and len(rest) == 2:
        return set_choice(rest[0], rest[1])
    if cmd == 'alerts' and rest and rest[0] in ('on', 'off'):
        return set_choice('phone', rest[0])
    if cmd == 'watch':
        return watch()
    if cmd == 'autoupdate' and rest and rest[0] in ('on', 'off'):
        C.set_key(PART, 'auto', rest[0] == 'on')
        user_timer(UPDATE_UNITS, 'hearth-server-updates.timer', rest[0] == 'on')
        return {'ok': True}
    if cmd == 'autoupdate' and rest == ['run']:
        return autoupdate_run()
    raise Fail('Not one of the dashboard\'s commands')


if __name__ == '__main__':
    sys.exit(C.main_guard(main))
