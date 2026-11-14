"""Task Manager's helper (hearth-tasks), the performance numbers: graphics cards, temperatures, battery, the CPU. Split out of hearth-tasks in the 1.12 tidy-up;
hearth-tasks re-exports everything here, so `hearth-tasks` and what loads it (Resource Monitor, the tests) are unchanged."""
from hearth_tasks_procs import *  # noqa: F401,F403

# ---------------------------------------------------------------------- performance
def gpu_vendor(v):
    return {'0x1002': 'AMD', '0x8086': 'Intel', '0x10de': 'NVIDIA'}.get((v or '').strip().lower(), '')


def amd_clock(text):
    for line in (text or '').splitlines():
        if line.strip().endswith('*'):
            m = re.search(r'(\d+)\s*Mhz', line, re.I)
            if m:
                return int(m.group(1))
    return None


def sysfs_gpus(sys_root=SYS):
    out = []
    for card in sorted(glob.glob(os.path.join(sys_root, 'class/drm/card[0-9]*'))):
        if '-' in os.path.basename(card):
            continue
        dev = os.path.join(card, 'device')
        vendor = gpu_vendor(read(os.path.join(dev, 'vendor')))
        if not vendor:
            continue
        g = {'card': os.path.basename(card), 'vendor': vendor, 'name': vendor + ' graphics',
             'busy': read_int(os.path.join(dev, 'gpu_busy_percent')),
             'vramUsed': read_int(os.path.join(dev, 'mem_info_vram_used')),
             'vramTotal': read_int(os.path.join(dev, 'mem_info_vram_total')),
             'gttUsed': read_int(os.path.join(dev, 'mem_info_gtt_used')),
             'gttTotal': read_int(os.path.join(dev, 'mem_info_gtt_total')),
             'mhz': amd_clock(read(os.path.join(dev, 'pp_dpm_sclk'))) or read_int(os.path.join(card, 'gt_act_freq_mhz')),
             'temp': None}
        product = (read(os.path.join(dev, 'product_name')) or '').strip()
        if product:
            g['name'] = product
        out.append(g)
    return out


def parse_nvidia_smi(text):
    out = []
    for line in (text or '').splitlines():
        f = [x.strip() for x in line.split(',')]
        if len(f) < 5:
            continue

        def num(x):
            try:
                return float(x)
            except ValueError:
                return None
        used, total = num(f[2]), num(f[3])
        out.append({'vendor': 'NVIDIA', 'name': f[0], 'busy': num(f[1]),
                    'vramUsed': int(used * 1048576) if used is not None else None,
                    'vramTotal': int(total * 1048576) if total is not None else None,
                    'temp': num(f[4]), 'mhz': None, 'gttUsed': None, 'gttTotal': None})
    return out


def gpus(sys_root=SYS):
    out = sysfs_gpus(sys_root)
    if shutil.which('nvidia-smi') and (not out or any(g['vendor'] == 'NVIDIA' for g in out)):
        try:
            r = subprocess.run(['nvidia-smi', '--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu',
                                '--format=csv,noheader,nounits'], capture_output=True, text=True, timeout=3)
            smi = parse_nvidia_smi(r.stdout) if r.returncode == 0 else []
            if smi:
                out = [g for g in out if g['vendor'] != 'NVIDIA'] + smi
        except (OSError, subprocess.TimeoutExpired):
            pass
    return out


CPU_SENSORS = ('k10temp', 'zenpower', 'coretemp', 'cpu_thermal', 'soc_thermal')
GPU_SENSORS = ('amdgpu', 'radeon', 'nouveau', 'i915', 'xe', 'nvidia')


def sensors(sys_root=SYS):
    """Temperatures and fans from hwmon. Temperatures: [{kind: cpu|gpu|nvme|other, label, c}]."""
    temps, fans = [], []
    for h in sorted(glob.glob(os.path.join(sys_root, 'class/hwmon/hwmon*')), key=lambda p: int(re.sub(r'\D', '', os.path.basename(p)) or 0)):
        name = (read(os.path.join(h, 'name')) or '').strip()
        kind = 'cpu' if name in CPU_SENSORS else 'gpu' if name in GPU_SENSORS else 'nvme' if name == 'nvme' else 'other'
        for t in sorted(glob.glob(os.path.join(h, 'temp*_input'))):
            v = read_int(t)
            if v is None or v <= -40000 or v >= 150000 or v == 0:
                continue
            label = (read(t.replace('_input', '_label')) or '').strip()
            temps.append({'kind': kind, 'sensor': name, 'label': label or name, 'c': round(v / 1000, 1)})
        for f in sorted(glob.glob(os.path.join(h, 'fan*_input'))):
            v = read_int(f)
            if v is None:
                continue
            label = (read(f.replace('_input', '_label')) or '').strip()
            fans.append({'label': label or (name + ' fan ' + re.sub(r'\D', '', os.path.basename(f))).strip(), 'rpm': v})
    return temps, fans


def summary_temp(temps, kind, prefer):
    mine = [t for t in temps if t['kind'] == kind]
    for p in prefer:
        for t in mine:
            if t['label'].lower() == p:
                return t['c']
    return max((t['c'] for t in mine), default=None)


def battery(sys_root=SYS):
    """The main battery (not a controller's), or None on desktops."""
    plugged, best = False, None
    for d in sorted(glob.glob(os.path.join(sys_root, 'class/power_supply/*'))):
        typ = (read(os.path.join(d, 'type')) or '').strip()
        if typ in ('Mains', 'USB') and read_int(os.path.join(d, 'online')) == 1:
            plugged = True
        if typ != 'Battery' or (read(os.path.join(d, 'scope')) or '').strip() == 'Device':
            continue
        pct = read_int(os.path.join(d, 'capacity'))
        if pct is None:
            continue
        status = (read(os.path.join(d, 'status')) or 'Unknown').strip()
        uw = read_int(os.path.join(d, 'power_now'))
        if uw is None:
            ua, uv = read_int(os.path.join(d, 'current_now')), read_int(os.path.join(d, 'voltage_now'))
            uw = abs(ua) * uv / 1e6 if ua is not None and uv is not None else None
        watts = round(abs(uw) / 1e6, 1) if uw is not None else None
        now, full = read_int(os.path.join(d, 'energy_now')), read_int(os.path.join(d, 'energy_full'))
        if now is None:
            cn, cf, uv = read_int(os.path.join(d, 'charge_now')), read_int(os.path.join(d, 'charge_full')), read_int(os.path.join(d, 'voltage_now'))
            if cn is not None and uv:
                now, full = cn * uv / 1e6, (cf * uv / 1e6 if cf else None)
        hours = None
        if watts and watts > 0.3 and now is not None:
            if status == 'Discharging':
                hours = round(now / 1e6 / watts, 2)
            elif status == 'Charging' and full:
                hours = round(max(0, full - now) / 1e6 / watts, 2)
        best = best or {'name': os.path.basename(d), 'percent': pct, 'status': status, 'watts': watts, 'hours': hours}
    if best:
        best['plugged'] = plugged or best['status'] in ('Charging', 'Full', 'Not charging')
    return best


def cpu_info(root=PROC, sys_root=SYS):
    name, mhz = '', []
    for line in (read(os.path.join(root, 'cpuinfo'), '') or '').splitlines():
        k, _, v = line.partition(':')
        k = k.strip()
        if k == 'model name' and not name:
            name = v.strip()
        elif k == 'cpu MHz':
            try:
                mhz.append(float(v))
            except ValueError:
                pass
    freqs = [read_int(p) for p in glob.glob(os.path.join(sys_root, 'devices/system/cpu/cpu[0-9]*/cpufreq/scaling_cur_freq'))]
    freqs = [f / 1000 for f in freqs if f]
    vals = freqs or mhz
    return name, round(sum(vals) / len(vals)) if vals else None


def perf(root=PROC, sys_root=SYS):
    now = time.monotonic()
    st = load_state('perf')
    cur = cpu_total(root)
    if not st or not (0.15 <= now - st.get('t', 0) <= 15) or len(st.get('cpus', [])) != len(cur):
        prev = cur
        time.sleep(0.2)
        cur = cpu_total(root)
        now = time.monotonic()
    else:
        prev = [tuple(x) for x in st['cpus']]
    save_state('perf', {'t': now, 'cpus': cur})
    pct = [round(busy_percent(p[1:], c[1:]), 1) for p, c in zip(prev, cur)]
    name, mhz = cpu_info(root, sys_root)
    mi = parse_meminfo(read(os.path.join(root, 'meminfo'), '') or '')
    temps, fans = sensors(sys_root)
    gl = gpus(sys_root)
    for g in gl:
        if g.get('temp') is None and g['vendor'] in ('AMD', 'Intel'):
            g['temp'] = summary_temp(temps, 'gpu', ('edge', 'junction'))
    uptime = read(os.path.join(root, 'uptime'), '0') or '0'
    return {
        'cpu': {'total': pct[0] if pct else 0, 'cores': pct[1:], 'name': name, 'mhz': mhz,
                'temp': summary_temp(temps, 'cpu', ('tctl', 'tdie', 'package id 0', 'cpu'))},
        'mem': {'total': mi.get('MemTotal', 0), 'used': mem_used(mi),
                'cached': mi.get('Cached', 0)},
        'swap': {'total': mi.get('SwapTotal', 0), 'used': max(0, mi.get('SwapTotal', 0) - mi.get('SwapFree', 0))},
        'gpus': gl,
        'temps': {'cpu': summary_temp(temps, 'cpu', ('tctl', 'tdie', 'package id 0', 'cpu')),
                  'gpu': next((g['temp'] for g in gl if g.get('temp') is not None), None) or summary_temp(temps, 'gpu', ('edge',)),
                  'nvme': summary_temp(temps, 'nvme', ('composite',)), 'list': temps},
        'fans': fans,
        'battery': battery(sys_root),
        'uptime': int(float(uptime.split()[0])) if uptime.split() else 0,
    }


