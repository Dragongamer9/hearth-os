#!/usr/bin/python3
"""CI's test answers for Resource Monitor's pictures (HEARTH_RESMON_HELPER, never set on a PC): hearth-resmon's own
commands (serve, ports, drives, writing, services, libs, files, end, eco) answered with the approved mockups'
numbers (docs/mockups/next-update/resmon-*.png), so the pictures can be set beside them. `serve` gives a minute of
history at once (the graphs are full, as in the mockups), then one sample a second, and stands still when told
(interval 0), like the real one. `pc` goes to the real helper with a test machine. The x-… answers stand in for
tailscale, nmcli, hearth-hwset status and disk-health."""
import json
import math
import os
import select
import subprocess
import sys
import time

G, M, K = 1073741824, 1048576, 1024


def wave(i, base, amp, ph=0.0):
    return max(0.0, base + amp * math.sin(i / 4.0 + ph) + amp * 0.6 * math.sin(i / 1.7 + ph * 2))


def group(i, gid, name, pid, cpu, avg, mem, threads, **kw):
    g = {'id': gid, 'name': name, 'pid': pid, 'pids': [pid] + kw.pop('more', []), 'cpu': cpu, 'avg': avg, 'mem': mem,
         'threads': threads, 'nice': kw.pop('nice', 0), 'cores': 'All', 'kind': kw.pop('kind', 'app'), 'color': kw.pop('color', 'app'),
         'exe': '/usr/bin/' + name.split()[0].lower(), 'protected': kw.pop('protected', False), 'why': '', 'eco': False,
         'private': kw.pop('private', mem * 0.8), 'shared': kw.pop('shared', mem * 0.15), 'swap': kw.pop('swap', 0), 'waits': kw.pop('waits', 0),
         'read': kw.pop('read', 0), 'write': kw.pop('write', 0), 'disk': 0, 'conns': kw.pop('conns', 0), 'remote': '',
         'gpu': kw.get('g3d', 0), 'vram': kw.pop('vram', 0), 'g3d': kw.pop('g3d', None), 'gdec': kw.pop('gdec', None), 'genc': kw.pop('genc', None),
         'api': kw.pop('api', '')}
    g.update(kw)
    return g


def sample(i, last):
    j = 1.0 if last else 0.0
    w = (lambda b, a, ph=0.0: b) if last else (lambda b, a, ph=0.0: wave(i, b, a, ph))
    cores = [30, 12, 55, 8, 22, 40, 10, 18]
    groups = [
        group(i, 'app:aicc', 'Hearth AI Command Center', 320684, 8.4, 16, int(1.21 * G), 25, private=980 * M, shared=180 * M, swap=310 * M, more=[320701], conns=4),
        group(i, 'app:firefox', 'firefox-bin', 383590, 7.2, 14, 820 * M, 21, private=610 * M, shared=210 * M, swap=460 * M, waits=2,
              read=int(0.1 * M), write=int(0.4 * M), g3d=0.8, gdec=2.1, vram=420 * M, api='VA-API (video)', conns=12),
        group(i, 'sys:quickshell', 'quickshell (desktop)' if os.environ.get('RESMON_GPU_NAMES') else 'quickshell', 1806, 2.0, 4, 430 * M, 6,
              private=370 * M, shared=60 * M, swap=40 * M, g3d=2.4, vram=310 * M, api='OpenGL', kind='system', color='sys', protected=True, why='The desktop'),
        group(i, 'sys:labwc', 'labwc', 1664, 1.4, 2, 29 * M, 4, nice=-11, g3d=0.8, vram=40 * M, api='Vulkan', kind='system', color='sys', protected=True, why='The desktop'),
        group(i, 'app:steam', 'steam (updating)' if os.environ.get('RESMON_DISK_NAMES') else 'steam', 4410, 0.3, 0, 310 * M, 0, nice=19 if os.environ.get('RESMON_DISK_NAMES') else 0,
              private=250 * M, shared=60 * M, swap=210 * M, read=int(2.1 * M), write=18 * M, color='steam', conns=8),
        group(i, 'app:claude', 'claude', 325370, 5.9, 11, 380 * M, 17, private=350 * M, shared=30 * M, swap=140 * M),
        group(i, 'app:tailscaled', 'tailscaled', 1180, 0.1, 0, 40 * M, 0, conns=1),
    ]
    for g in groups:
        if not last:
            g['cpu'] = round(wave(i, g['cpu'], g['cpu'] * 0.3), 1)
            for k in ('g3d', 'gdec'):
                if g.get(k):
                    g[k] = round(wave(i, g[k], g[k] * 0.4, 1.3), 1)
    return {
        't': time.time(),
        'cpu': {'name': 'Intel Core i3-N305', 'logical': 8, 'coreCount': 8, 'base': 2800, 'total': round(w(21, 5), 1), 'mhz': 2800, 'temp': round(w(61, 2, 1), 1),
                'threads': 312, 'processes': 214, 'cores': [round(w(c, c * 0.25, k), 1) for k, c in enumerate(cores)]},
        'coreMhz': [2900, 1400, 3400, 1000, 2100, 3000, 1200, 1800],
        'uptime': 111600 + 1200,
        'mem': {'total': int(7.5 * G), 'available': int(2.6 * G), 'commitLimit': int(55.5 * G)},
        'memBar': {'installed': 8 * G, 'total': int(7.5 * G), 'reserved': int(0.5 * G), 'inUse': int(w(3.9, 0.08, 2) * G), 'saving': 0, 'cache': int(1.1 * G),
                   'free': int(1.5 * G), 'committed': int(9.8 * G), 'swapTotal': 48 * G, 'swapUsed': int(3.3 * G)},
        'zram': {'orig': int(w(3.1, 0.15, 3) * G), 'compr': int(1.0 * G), 'mem': int(1.0 * G), 'size': 12 * G},
        'swaps': [{'path': '/dev/zram0', 'size': 12 * G, 'used': int(3.1 * G)}, {'path': '/var/swap/swapfile', 'size': 36 * G, 'used': int(0.2 * G)}],
        'pressure': round(w(0.0, 0.4), 1) if not last else 0.0,
        'swapIO': {'in': int(w(0, 60 * K)), 'out': int(w(0, 40 * K, 1))} if not last else {'in': 0, 'out': 0},
        'disks': [
            {'name': 'nvme0n1', 'label': 'Disk 0 (C:)', 'letters': ['C'], 'system': True, 'mounts': ['/', '/var'], 'type': 'SSD (NVMe)', 'model': 'SK hynix BC711',
             'active': round(w(3, 2), 1), 'read': int(w(2.2, 0.8) * M), 'write': int(w(18.3, 6, 1) * M), 'response': round(w(0.2, 0.05), 2)},
            {'name': 'sda', 'label': 'Disk 1 (G:)', 'letters': ['G'], 'system': False, 'mounts': ['/run/media/chris/Games'], 'type': 'SSD', 'model': 'Samsung 870',
             'active': round(w(12, 5, 2), 1), 'read': 0, 'write': int(w(0.4, 0.2) * M), 'response': round(w(0.6, 0.1), 2)},
        ],
        'nets': [{'name': 'wlan0', 'kind': 'Wi-Fi', 'up': True, 'speed': 866, 'recv': int(w(2.4, 0.9) * M), 'send': int(w(60, 20) * K)},
                 {'name': 'tailscale0', 'kind': 'Tailscale', 'up': True, 'speed': None, 'recv': int(w(6, 2, 1) * K), 'send': int(w(2, 1) * K)}],
        'gpus': [{'card': 'card1', 'vendor': 'Intel', 'name': 'Intel UHD (Alder Lake-N)', 'busy': round(w(4, 1.2), 1), 'temp': round(w(48, 1.5, 2), 1),
                  'mhz': 450, 'mhzMax': 1250, 'gttUsed': int(w(770, 20) * M), 'gttTotal': int(3.8 * G), 'vramUsed': None, 'vramTotal': None, 'watts': 1.2, 'index': 0}],
        'card': {'driver': 'i915', 'mesa': '26.1', 'vulkan': '1.4', 'opengl': '4.6', 'screens': ['1920×1200 at 60 Hz · 125%']},
        'fans': {}, 'battery': None,
        'procs': groups,
    }


def serve():
    interval, sent = 1.0, 0
    buf = b''
    for i in range(59):   # a minute of history at once
        print(json.dumps(sample(i, False)), flush=True)
    due = time.monotonic()
    while True:
        timeout = None if due is None else max(0.0, due - time.monotonic())
        r, _, _ = select.select([0], [], [], timeout)
        if r:
            chunk = os.read(0, 65536)
            if not chunk:
                return
            buf += chunk
            while b'\n' in buf:
                line, buf = buf.split(b'\n', 1)
                try:
                    m = json.loads(line or b'{}')
                except ValueError:
                    continue
                if 'interval' in m:
                    interval = float(m['interval'])
                    due = time.monotonic() if interval > 0 else None
                if m.get('now') and interval > 0:
                    due = time.monotonic()
            continue
        print(json.dumps(sample(60 + sent, True)), flush=True)
        sent += 1
        due = time.monotonic() + interval if interval > 0 else None


def main():
    a = sys.argv[1:]
    c = a[0] if a else ''
    if c == 'serve':
        return serve()
    if c == 'pc':
        real = os.path.join(os.environ.get('HEARTH_SHELL_DIR', '/usr/share/nimbus/desktop/shell'), 'hearth-resmon')
        os.execvp('python3', ['python3', real] + a)
    out = {}
    if c == 'ports':
        out = {'listening': [{'pid': 1222, 'name': 'hearth-link', 'port': 47990, 'openTo': 'Tailscale only', 'what': 'your other Hearth devices'},
                             {'pid': 902, 'name': 'sshd', 'port': 22, 'openTo': 'Tailscale only', 'what': 'remote terminal'}],
               'connections': [{'pid': 4410, 'name': 'steam', 'remote': '23.62.99.12', 'host': 'cache8-dfw1.steamcontent.com', 'port': 443, 'ping': 22, 'state': 'Open'},
                               {'pid': 320684, 'name': 'node', 'remote': '160.79.104.10', 'host': 'api.anthropic.com', 'port': 443, 'ping': 31, 'state': 'Open'},
                               {'pid': 1180, 'name': 'tailscaled', 'remote': '100.84.2.9', 'host': 'gaming-pc (100.84.2.9)', 'port': 41641, 'ping': 4, 'state': 'Open'}],
               'apps': {'4410': {'down': int(2.4 * M), 'up': 40 * K}, '383590': {'down': 90 * K, 'up': 12 * K}, '320684': {'down': 20 * K, 'up': 8 * K}}}
        # a little ping history for the graph: the real helper's every 3 s
        out['connections'][0]['ping'] = 18 + int(time.time()) % 5
    elif c == 'drives':
        out = {'drives': [{'name': 'nvme0n1', 'label': 'Disk 0 (C:)', 'letters': ['C'], 'type': 'SSD (NVMe)', 'model': 'SK hynix BC711', 'fs': 'btrfs',
                           'free': 96 * G, 'total': 238 * G, 'pct': 60, 'temp': 41, 'system': True, 'mounts': ['/', '/var']},
                          {'name': 'sda', 'label': 'Disk 1 (G:)', 'letters': ['G'], 'type': 'SSD', 'model': 'Samsung 870', 'fs': 'ext4',
                           'free': 212 * G, 'total': 476 * G, 'pct': 55, 'temp': 36, 'system': False, 'mounts': ['/run/media/chris/Games']}]}
    elif c == 'writing':
        out = {'files': [{'pid': 4410, 'path': '/run/media/chris/Games/SteamLibrary/steamapps/downloading/19840/depot_19840.chunk',
                          'win': 'G:\\SteamLibrary\\steamapps\\downloading\\19840\\depot_19840.chunk', 'writing': True, 'read': 0, 'write': 18 * M},
                         {'pid': 383590, 'path': '/home/chris/.mozilla/firefox/x.default/places.sqlite',
                          'win': 'C:\\Users\\chris\\AppData\\Mozilla\\Firefox\\x.default\\places.sqlite', 'writing': True, 'read': int(0.1 * M), 'write': int(0.4 * M)}]}
    elif c == 'services':
        out = {'services': [{'name': 'hearth-ai-updater', 'pid': 320701, 'status': 'Running', 'what': 'checks for app updates', 'pids': [320701]},
                            {'name': 'hearth-link', 'pid': 1222, 'status': 'Running', 'what': 'your other Hearth devices', 'pids': [1222]}]}
    elif c == 'libs':
        out = {'libs': [{'name': n, 'path': '/usr/lib64/' + n} for n in ('libEGL.so.1', 'libGLX.so.0', 'libX11.so.6', 'libasound.so.2', 'libc.so.6', 'libdbus-1.so.3',
                                                                          'libffmpeg.so', 'libgtk-3.so.0', 'libnss3.so', 'libvulkan.so.1')]}
    elif c == 'files':
        out = {'files': [{'pid': 320684, 'path': '/home/chris/.local/share/hearth-ai/chats/c0000000412.json', 'win': 'C:\\Users\\chris\\AppData\\hearth-ai\\chats\\c0000000412.json'},
                         {'pid': 320684, 'path': '/home/chris/Projects/hearth-os/docs/ROADMAP.md', 'win': 'C:\\Users\\chris\\Projects\\hearth-os\\docs\\ROADMAP.md'}]}
    elif c in ('end', 'eco', 'kill'):
        out = {'ok': True}
    elif c == 'x-tailscale':
        out = {'Peer': {'a': {}, 'b': {}, 'c': {}}}
    elif c == 'x-ssid':
        print('Home-5G')
        return
    elif c == 'x-hwset':
        out = {'tempLimit': 100}
    elif c == 'x-health':
        out = {'drives': {'nvme0n1': {'ok': True, 'lifeLeft': 97, 'temp': 41}, 'sda': {'ok': True, 'lifeLeft': 99, 'temp': 36}}}
    print(json.dumps(out))


if __name__ == '__main__':
    main()
