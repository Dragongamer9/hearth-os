#!/usr/bin/env python3
"""CI's pretend home server for the Home server helper's pictures (home-server.sh): a pretend PC (the dashboard's
world: units, answers on each service's address, the graphics card, Tailscale's peers, a day of history), a vault
drive with the models' MODEL.json files and a week of checks, a pretend restic with four devices' backups, and the
Store's last list. The REAL engines (os/system_files/usr/libexec/nimbus/hearth-server-parts) are run against it and
their answers are what the app shows (HEARTH_SERVER_FIXTURES: <engine>-<command>.json). Nothing is sent anywhere.

    home-server-fixture.py OUT_DIR
"""
import json
import os
import shutil
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
PARTS = os.path.join(ROOT, 'os', 'system_files', 'usr', 'libexec', 'nimbus', 'hearth-server-parts')
OUT = os.path.abspath(sys.argv[1])
W = os.path.join(OUT, 'world')
HOME = os.path.join(W, 'home')
NOW = time.time()
DAY = 86400


def wj(p, d):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        json.dump(d, f, indent=1)


def at(days_ago, hh, mm=0):
    lt = time.localtime(NOW - days_ago * DAY)
    return int(time.mktime((lt.tm_year, lt.tm_mon, lt.tm_mday, hh, mm, 0, 0, 0, -1)))


def main():
    shutil.rmtree(W, ignore_errors=True)
    os.makedirs(HOME)
    env = dict(os.environ, HOME=HOME, XDG_CONFIG_HOME=os.path.join(HOME, '.config'), XDG_STATE_HOME=os.path.join(HOME, '.local/state'),
               XDG_DATA_HOME=os.path.join(HOME, '.local/share'), XDG_CACHE_HOME=os.path.join(HOME, '.cache'), XDG_RUNTIME_DIR=os.path.join(W, 'run'),
               HEARTH_SHELL_DIR=os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell'), HEARTH_TEST_PLAYING='0', HEARTH_TEST_NO_SYSTEMCTL='1',
               HEARTH_AI_MODELS=os.path.join(ROOT, 'os', 'system_files', 'usr', 'share', 'hearth', 'ai-models.json'),
               HEARTH_TEST_SECRETS=json.dumps({'r2-access': 'x', 'r2-secret': 'x', 'backup-rog-ally-3f9a': 'k', 'backup-laptop-71c2': 'k', 'backup-truck-laptop-0b44': 'k', 'backup-dads-pc-9e10': 'k'}),
               HEARTH_VAULT_RAM_GB='32', HEARTH_VAULT_PC='RTX 5070 · 32 GB', HEARTH_VAULT_WHERE='in the Nvidia PC', HEARTH_SERVER_NAME='Nvidia PC',
               HEARTH_VAULT_HF='http://127.0.0.1:9', HEARTH_DASH_WORLD=os.path.join(W, 'world.json'), HEARTH_RESTIC=os.path.join(W, 'restic'),
               HEARTH_BACKUP_RECEIVE=os.path.join(W, 'backups'), HEARTH_TEST_TSIP='100.101.4.7', TZ=os.environ.get('TZ', 'America/Chicago'))
    os.makedirs(env['XDG_RUNTIME_DIR'], exist_ok=True)
    cfg = os.path.join(HOME, '.config', 'nimbus')
    st = os.path.join(HOME, '.local', 'state', 'hearth', 'server')
    os.makedirs(st, exist_ok=True)

    # ---------------------------------------------------------------- the vault
    drive = os.path.join(W, 'vault-drive')
    os.makedirs(drive)
    forge = [('hunyuan3d-21', 'Hunyuan3D 2.1', 24.6, 'Tencent Hunyuan'), ('hunyuan3d-2mini', 'Hunyuan3D 2mini', 5.1, 'Hunyuan'), ('trellis-text-large', 'TRELLIS text large', 9.8, 'MIT'),
             ('trellis-base', 'TRELLIS base', 4.6, 'MIT'), ('sf3d', 'Stable Fast 3D', 2.1, 'Stability Community'), ('spar3d', 'SPAR3D', 4.0, 'Stability Community'),
             ('instantmesh', 'InstantMesh', 4.3, 'Apache-2.0'), ('crm', 'CRM', 3.2, 'MIT'), ('wonder3d', 'Wonder3D', 6.5, 'MIT'), ('lgm', 'LGM', 1.9, 'MIT'),
             ('zero123pp', 'Zero123++', 3.3, 'Apache-2.0'), ('triposg', 'TripoSG', 6.3, 'MIT'), ('step1x-3d', 'Step1X-3D', 13.6, 'Apache-2.0'), ('direct3d-s2', 'Direct3D-S2', 8.4, 'MIT'),
             ('partcrafter', 'PartCrafter', 5.2, 'MIT'), ('unirig', 'UniRig', 1.4, 'MIT'), ('hi3dgen', 'Hi3DGen', 7.7, 'MIT'), ('mv-adapter', 'MV-Adapter', 9.9, 'Apache-2.0'), ('sam-3d', 'SAM 3D Objects', 3.6, 'SAM')]
    wj(os.path.join(HOME, '.local/share/hearth/forge/models.json'), {'models': [{'id': i, 'name': n, 'repo': 'forge/%s' % i, 'gb': g, 'licence': l} for i, n, g, l in forge]})
    wj(os.path.join(cfg, 'server-vault.json'), {'keep': 'no-giants', 'drive': drive, 'on': True, 'r2': {'account': '0' * 32, 'bucket': 'hearth-models', 'class': 'infrequent'}})
    # what the catalog says, kept by "no-giants" (all but the five giants and the newest Forge one, still asked about)
    cat = json.loads(subprocess.run([sys.executable, '-I', os.path.join(PARTS, 'vault.py'), 'status'], env=env, capture_output=True, text=True).stdout or '{}')
    models = cat.get('models') or []
    revs = {}
    online = {}
    upload = {}
    n = 0
    for m in models:
        if m['giant'] or m['id'] in ('hunyuan3d-21',):
            online[m['id']] = {'state': 'ok', 'checked': at(4, 3), 'rev': 'b71c04e9a1' if 'mimo' in m['id'] else '6e3a0b4c11', 'licence': m['licence']}
            continue
        n += 1
        rev = ('%07x' % (0x8f2c1a9 + n * 7919))[:7] + 'f00d'
        revs[m['id']] = rev
        gb = float(m['size'].replace(' GB', '').replace(',', ''))
        files = [{'name': 'model.safetensors' if m['app'] != 'Uncensored' else m['name'].split(' · ')[0].replace(' ', '-') + '.gguf', 'size': int(gb * 1e9), 'sha256': ('%064x' % (0x3b9e0000c41d + n * 104729))}]
        if m['app'] == 'Hearth Forge':
            files.append({'name': 'config.json', 'size': 4000, 'sha256': '%064x' % (n + 7)})
        d = os.path.join(drive, 'Hearth Model Vault', m['app'], m['id'], rev[:7])
        wj(os.path.join(d, 'MODEL.json'), {'id': m['id'], 'name': m['name'], 'app': m['app'], 'source': {'repo': 'org/%s' % m['id']}, 'revision': rev, 'short': rev[:7], 'licence': m['licence'],
                                           'files': files, 'keptAt': at(30 + n, 4), 'where': 'drive+ia', 'quant': 'Q4_K' if m['app'] == 'Uncensored' else '', 'version': 'v1.0' if m['id'] == 'kokoro-82m' else ''})
        online[m['id']] = {'state': 'ok', 'checked': at(4, 3), 'rev': rev, 'licence': m['licence']}
        # TRELLIS large is on its way (64%); the newest uncensored ones and most of Forge's haven't gone up yet
        frac = 0.64 if m['id'] == 'trellis-large' else (0.0 if (m['app'] == 'Uncensored' and n % 3 == 0) or (m['app'] == 'Hearth Forge' and n % 2 == 0) else 1.0)
        upload[m['id']] = {'files': {f['name']: ({'done': True, 'sent': f['size'], 'size': f['size']} if frac >= 1 else {'sent': int(f['size'] * frac), 'size': f['size'], 'uploadId': 'u1', 'parts': []}) for f in files}}
    online['huihui-qwen3.6-35b-a3b-abliterated'] = {'state': 'removed', 'since': at(4, 3, 4), 'checked': at(1, 3)}
    online['trellis-large'] = dict(online.get('trellis-large', {}), state='changed', since=at(1, 3, 1), rev='e40a6d1c0', changedFiles=['model.safetensors'], addedFiles=['pipeline.json'],
                                   files={'model.safetensors': {'size': 11.9e9}, 'config.json': {'size': 4000}, 'pipeline.json': {'size': 1000}}, licence='MIT')
    checks = []
    for i in range(12, 0, -1):
        t = at(4 + (i - 1) * 7, 3)
        checks.append({'at': t, 'ok': len(models) - (2 if i == 1 else 0), 'removed': ['huihui-qwen3.6-35b-a3b-abliterated'] if i == 1 else [], 'changed': ['trellis-large'] if i == 1 else [], 'renamed': ['kokoro-82m'] if i == 7 else [], 'gated': [], 'licence': [], 'new': []})
    wj(os.path.join(st, 'vault.json'), {'online': online, 'checks': checks, 'upload': upload,
                                        'asks': {'hunyuan3d-21': {'at': at(0, 3), 'rev': '1c7e0a2d9', 'gb': 24.6, 'licence': 'Tencent Hunyuan licence'}},
                                        'queue': [{'id': 'huihui-qwen3-coder-30b-a3b-instruct-abliterated', 'where': 'drive+ia'}, {'id': 'huihui-qwen3.8-flash-next-abliterated', 'where': 'drive+ia'}],
                                        'pauses': [at(0, 1)], 'offline': {'at': at(2, 22), 'total': n, 'loaded': n, 'bad': []}, 'loads': {'vault': 9, 'online': 1},
                                        'r2days': {time.strftime('%Y-%m-', time.localtime(NOW)) + '%02d' % dd: int(412e9) for dd in range(1, time.localtime(NOW).tm_mday + 1)},
                                        'tonight': [{'text': '3:00 AM · check every model (Sundays)', 'order': 3}] if time.localtime(NOW).tm_wday == 5 else []})
    env['HEARTH_TEST_DRIVE'] = json.dumps({'total': 8e12, 'free': 6.75e12})

    # ---------------------------------------------------------------- backups: four devices, a pretend restic
    devices = {'rog-ally': ('ROG Ally', 'handheld', 'rog-ally-3f9a', 212e9, 1.8e9, (2, 4)), 'laptop': ('Laptop', 'laptop', 'laptop-71c2', 148e9, 640e6, (2, 11)),
               'truck-laptop': ('Truck laptop', 'laptop', 'truck-laptop-0b44', 64e9, 92e6, (2, 19)), 'dads-pc': ('Dad\'s PC', 'pc', 'dads-pc-9e10', 386e9, 3.1e9, (2, 36))}
    devcfg = {}
    snaps = {}
    for did, (name, kind, user, whole, sent, (hh, mm)) in devices.items():
        devcfg[did] = {'name': name, 'kind': kind, 'user': user, 'home': True, 'saves': True, 'settings': True, 'system': True, 'games': False, 'at': '02:00', 'away': True, 'nights': 30, 'weeks': '1y', 'lock': True,
                       'where': {'rog-ally': 'at home', 'laptop': 'at home', 'truck-laptop': 'at work · over 5G hotspot', 'dads-pc': 'at Dad\'s'}[did]}
        os.makedirs(os.path.join(env['HEARTH_BACKUP_RECEIVE'], user))
        ss = []
        count = {'rog-ally': 41, 'laptop': 38, 'truck-laptop': 30, 'dads-pc': 30}[did]
        for k in range(count):
            day = k if k < 30 else 30 + (k - 30) * 7
            late = (did == 'truck-laptop' and k == 11) or (did == 'dads-pc' and k == 6)
            t = at(day, 9 if late else hh, mm)
            ss.append({'short_id': '%08x' % (abs(hash((did, k))) & 0xffffffff), 'id': '%064x' % k, 'time': time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(t)) + time.strftime('%z', time.localtime(t))[:3] + ':' + time.strftime('%z', time.localtime(t))[3:],
                       'hostname': did, 'tags': ['hearth', 'home', 'saves', 'settings', 'system', 'size-home=%d' % {'rog-ally': 38e9}.get(did, whole * 0.2), 'size-saves=%d' % {'rog-ally': 4.2e9}.get(did, 2e9),
                                                 'size-settings=%d' % 0.3e9, 'size-system=%d' % whole], 'summary': {'data_added_packed': int(sent), 'total_bytes_processed': int(whole)}})
        ss.sort(key=lambda x: x['time'])
        snaps[user] = ss
    wj(os.path.join(cfg, 'server-backups.json'), {'on': True, 'devices': devcfg, 'r2': True, 'restoreTest': True})
    ls = [{'path': '/var/home/you/Saves/Gravesleep', 'type': 'dir', 'mtime': time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(at(1, 23, 52))) + '-06:00'},
          {'path': '/var/home/you/Saves/Gravesleep/slot1.save', 'type': 'file', 'size': 12e6, 'mtime': time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(at(1, 23, 52))) + '-06:00'},
          {'path': '/var/home/you/Saves/Gravesleep/slot2.save', 'type': 'file', 'size': 9e6, 'mtime': time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(at(5, 20))) + '-06:00'},
          {'path': '/var/home/you/Saves/Gravesleep/settings.cfg', 'type': 'file', 'size': 2e3, 'mtime': time.strftime('%Y-%m-%dT%H:%M:%S', time.localtime(at(23, 18))) + '-06:00'}]
    newest = [dict(x, mtime='2026-11-26T09:00:00-06:00') if x['path'].endswith('slot1.save') else x for x in ls]
    with open(os.path.join(OUT, 'restore-night'), 'w') as f:
        f.write(snaps['rog-ally-3f9a'][-2]['short_id'])
    wj(os.path.join(W, 'restic.json'), {'snapshots': snaps, 'ls': ls, 'lsNewest': newest, 'newestIds': [s_[-1]['short_id'] for s_ in snaps.values()]})
    with open(env['HEARTH_RESTIC'], 'w') as f:
        f.write('''#!/usr/bin/env python3
import json, os, sys
d = json.load(open(%r))
a = sys.argv[1:]
repo = a[a.index('-r') + 1] if '-r' in a else ''
user = os.path.basename(repo.rstrip('/'))
if 'snapshots' in a:
    print(json.dumps(d['snapshots'].get(user, [])))
elif 'ls' in a:
    snap = a[a.index('ls') + 1]
    print('\\n'.join(json.dumps(x) for x in (d['lsNewest'] if snap in d['newestIds'] else d['ls'])))
elif 'stats' in a:
    print(json.dumps({'total_size': 0}))
''' % os.path.join(W, 'restic.json'))
    os.chmod(env['HEARTH_RESTIC'], 0o755)
    steps = [{'at': at(0, 2, 0), 'what': 'server\'s 9 services', 'bytes': int(0.4e9)}, {'at': at(0, 2, 4), 'what': 'ROG Ally · 1.8 GB', 'bytes': int(1.8e9)}, {'at': at(0, 2, 11), 'what': 'Laptop · 640 MB', 'bytes': int(640e6)},
             {'at': at(0, 2, 19), 'what': 'Truck laptop · 92 MB', 'bytes': int(92e6)}, {'at': at(0, 2, 36), 'what': 'Dad\'s PC · 3.1 GB', 'bytes': int(3.1e9)}, {'at': at(0, 3, 10), 'what': 'copy to R2 · Sunday', 'bytes': 0}]
    wj(os.path.join(st, 'backups.json'), {'steps': steps, 'services': {k: at(0, 2) for k in ('ollama', 'jellyfin', 'comfyui', 'kokoro', 'nextcloud', 'homeassistant', 'colibri', 'shield')} | {'vaultwarden': at(0, 1)},
                                          'sizes': {'dads-pc-9e10': int(710e9), 'rog-ally-3f9a': int(421e9), 'laptop-71c2': int(292e9), 'truck-laptop-0b44': int(118e9), 'server': int(359e9)},
                                          'lastNight': at(0, 2, 36), 'restoreTest': {'at': at(25, 4), 'n': 50, 'ok': True, 'bad': 0}})

    # ---------------------------------------------------------------- the dashboard's pretend PC
    def u(active='active', mem=0, gpu=0, cpu=0, disk='—', restarts=0):
        return {'exists': True, 'active': active, 'mem': int(mem), 'gpu': int(gpu), 'cpuPct': cpu, 'disk': disk, 'restarts': restarts}
    lists = [{'enabled': True, 'url': 'https://adaway.org/hosts.txt', 'last_updated': ''}] + [{'enabled': True, 'url': 'https://list%d.example/hosts' % i, 'last_updated': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(NOW - 9 * DAY))} for i in range(5)]
    hist = []
    for i in range(1440, 0, -2):
        t = NOW - i * 60
        hh = time.localtime(t).tm_hour
        busy = 0.25 + 0.2 * ((i * 37) % 11) / 10 + (0.25 if 19 <= hh <= 23 else 0)
        hist.append([int(t), round(18 * busy * 1.6, 1), round(41 * busy * 1.7, 1), round(54 + 18 * busy + (6 if 21 <= hh <= 21 else 0)), 54, int(212e9 * (1440 - i) / 1440), int(41e9 * (1440 - i) / 1440), 67])
    hist[-1][1], hist[-1][2], hist[-1][3] = 18, 41, 62
    peak = max(range(len(hist)), key=lambda k: hist[k][3])
    hist[peak][3] = 74
    for k in range(max(0, peak - 60), min(len(hist), peak + 30)):
        hist[k].append('Gravesleep')   # what was being played then (dash.py game_now)
    wj(os.path.join(st, 'dash.json'), {'hist': hist})
    world = {
        'units': {'hearth-ollama.service': u(mem=0.4e9, cpu=1, disk='9 GB'), 'hearth-colibri.service': u(mem=11e9, gpu=6.1e9, cpu=9, disk='1.25 TB'), 'hearth-jellyfin.service': u(mem=1.2e9, gpu=0.3e9, cpu=4, disk='2.6 TB'),
                  'hearth-comfyui.service': u(mem=3.8e9, gpu=2.9e9, cpu=3, disk='148 GB', restarts=1), 'hearth-kokoro.service': u(mem=0.6e9, gpu=0.8e9, cpu=0, disk='0.3 GB'),
                  'hearth-vaultwarden.service': u(mem=0.1e9, cpu=0, disk='42 MB'), 'hearth-nextcloud.service': u(mem=0.9e9, cpu=2, disk='1.24 TB'), 'hearth-home-assistant.service': u(mem=0.7e9, cpu=1, disk='3.1 GB'),
                  'hearth-server-dns.service': u(mem=0.2e9, cpu=1, disk='0.4 GB'), 'sunshine.service': u(mem=0.3e9, cpu=2)},
        'http': {'http://127.0.0.1:11434/api/ps': {'models': []}, 'http://127.0.0.1:11440/v1/models': {'data': [{'id': 'Qwen3.6 35B'}]}, 'http://127.0.0.1:8096/System/Info/Public': {'ServerName': 'Nvidia PC'}, 'http://127.0.0.1:8096/Sessions': [{'DeviceName': 'Dad\'s PC', 'NowPlayingItem': {'Name': 'a film'}}],
                 'http://127.0.0.1:8188/queue': {'queue_running': [1], 'queue_pending': [2]}, 'http://127.0.0.1:8880/v1/models': {'data': []}, 'http://127.0.0.1:8222/alive': '', 'http://127.0.0.1:8090/status.php': {'installed': True},
                 'http://127.0.0.1:8123/api/': {'message': 'API running.'}, 'http://127.0.0.1:3053/control/filtering/status': {'filters': lists}, 'https://127.0.0.1:47990/': ''},
        'gpu': {'name': 'RTX 5070', 'pct': 41, 'used': 9.8e9, 'total': 12e9, 'temp': 62, 'watts': 140, 'procs': {}},
        'mem': {'total': 32e9, 'avail': 10.6e9}, 'cpu': {'busy': 100, 'all': 1000, 'pct': 18, 'name': 'AMD Ryzen 7 7700X 8-Core Processor'}, 'cpuTemp': 54, 'net': [int(212e9), int(41e9)],
        'drives': [{'disk': 'nvme0n1', 'total': 2e12, 'used': 0.9e12, 'healthy': True}, {'disk': 'nvme1n1', 'total': 4e12, 'used': 1.6e12, 'healthy': True}, {'disk': 'sda', 'total': 8e12, 'used': 1.3e12, 'healthy': True}, {'disk': 'sdb', 'total': 8e12, 'used': 4.3e12, 'healthy': True}],
        'homeassistant': {'devices': 23, 'lights': 5}, 'caches': int(38e9), 'uptime': 9 * DAY + 3600,
        'power': [[int(NOW - k * 300), 60 + (k * 13) % 50, 60 + (k * 29) % 90] for k in range(287, -1, -1)],
        'tailscale': {'BackendState': 'Running', 'Peer': {
            'a': {'HostName': 'rog-ally', 'Online': True, 'CurAddr': '192.168.1.20:41641'}, 'b': {'HostName': 'laptop', 'Online': True, 'CurAddr': '192.168.1.21:41641'},
            'c': {'HostName': 'tcl-tv', 'Online': True, 'CurAddr': '192.168.1.30:41641'}, 'd': {'HostName': 'phone', 'Online': True, 'CurAddr': '192.168.1.40:41641'},
            'e': {'HostName': 'xbox', 'Online': False}, 'f': {'HostName': 'truck-laptop', 'Online': True, 'CurAddr': '', 'Relay': 'dfw'},
            'g': {'HostName': 'dads-pc', 'Online': True, 'CurAddr': '72.14.1.9:41641'}, 'h': {'HostName': 'old-laptop', 'Online': True, 'CurAddr': '192.168.1.50:41641'}}},
    }
    world['power'][-1] = [int(NOW), 86, 100]
    wj(env['HEARTH_DASH_WORLD'], world)
    wj(os.path.join(cfg, 'server-dash.json'), {'phone': True, 'names': {'tcl-tv': 'TCL 55" TV', 'dads-pc': 'Dad\'s PC'}})
    wj(os.path.join(st, 'colibri.json'), {'wps': 22})
    # the other parts' own state (Game servers, Game cache: feature/server-2's parts write these)
    wj(os.path.join(st, 'games.json'), {'summary': {'state': 'ok', 'what': '3 on · 7 players', 'foot': 'CPU 14% · RAM 6.2 GB · worlds saved 4 AM'},
                                        'servers': [{'id': 'valheim', 'name': 'Valheim', 'line': '3 of 10 · since 11:40 PM', 'on': True, 'players': '3 friends'}, {'id': 'minecraft', 'name': 'Minecraft', 'line': '2 of 8 · Dad and you', 'on': True},
                                                    {'id': 'gravesleep', 'name': 'Gravesleep test', 'line': 'off · starts Sat 7 PM', 'on': False}],
                                        'tonight': [{'text': '4:00 AM · back up 3 game worlds', 'order': 4}, {'text': '6:30 AM · start the Valheim server', 'order': 6}]})
    wj(os.path.join(st, 'cache.json'), {'summary': {'state': 'ok', 'what': '612 GB · saved 1.9 TB', 'foot': 'serving the Ally · 940 Mb/s'}, 'now': {'line': 'sending to the Ally · 940 Mb/s'}})
    wj(os.path.join(st, 'play.json'), {'now': {'title': 'Streaming to the TV', 'line': 'Gravesleep · 4K 60', 'device': 'tcl-tv'}})
    ev = [{'at': at(0, 0, 41) + 12 * 3600 if time.localtime(NOW).tm_hour >= 13 else at(0, 0, 41), 'part': 'dash', 'text': 'Imagine ran out of GPU memory, restarted itself', 'tone': 'ok'},
          {'at': at(0, 2), 'part': 'backups', 'text': 'Nightly backups: 4 devices, 9 services', 'tone': 'ok'},
          {'at': at(1, 21), 'part': 'dash', 'text': 'Colibri stopped for Gravesleep, back after', 'tone': 'ok'}]
    with open(os.path.join(st, 'events.jsonl'), 'w') as f:
        f.write('\n'.join(json.dumps(e) for e in sorted(ev, key=lambda e: e['at'])) + '\n')

    # ---------------------------------------------------------------- the Store's last list
    wj(os.path.join(OUT, 'store.json'), {'hearth-home-server-helper': {'state': 'current', 'version': '1.18.0', 'have': '1.18.0', 'test': True}, 'hearth-home-server-app': {'state': 'current'}})

    # ---------------------------------------------------------------- the engines, for real
    for eng, cmds in (('dash', [['status'], ['summary']]), ('vault', [['status'], ['summary']]), ('backups', [['status'], ['summary'], ['files', 'rog-ally', open(os.path.join(OUT, 'restore-night')).read(), 'Gravesleep']])):
        for c in cmds:
            r = subprocess.run([sys.executable, '-I', os.path.join(PARTS, eng + '.py')] + c, env=env, capture_output=True, text=True)
            out = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ''
            try:
                v = json.loads(out)
            except ValueError:
                v = {'ok': False, 'error': (r.stderr or 'no answer')[-600:]}
            if not v.get('ok'):
                print('::error title=home server fixture::%s %s: %s' % (eng, ' '.join(c), (v.get('error') or r.stderr)[-600:]))
            wj(os.path.join(OUT, '%s-%s.json' % (eng, c[0])), v)
    print('::notice title=home server fixture::the engines answered: ' + ', '.join(sorted(os.listdir(OUT))))


if __name__ == '__main__':
    main()
