"""Stand-in devices for Settings › Hearth Link › Your devices (desktop-test.yml screenshots, test_device_status.py):
the ROG Ally on battery, the Nvidia PC in Game Mode, a laptop that's offline (with its last numbers). Prints the
JSON `hearth-device-status devices` would."""
import json
import math
import time


def history(now, base, swing, temp, battery=None):
    out = []
    for i in range(60):
        t = now - (59 - i) * 60000
        p = {'t': t, 'cpu': round(base + swing * math.sin(i / 6)), 'gpu': round(base + 10 + swing * math.cos(i / 7)),
             'ram': round(40 + 10 * math.sin(i / 15)), 'cpuTemp': round(temp + 6 * math.sin(i / 9)), 'gpuTemp': round(temp + 4 + 5 * math.cos(i / 8))}
        if battery is not None:
            p['battery'] = max(5, battery + (59 - i) // 3)
        out.append(p)
    return out


def sample(now=None):
    now = now or int(time.time() * 1000)
    ally = {'protocol': 1, 'shared': True, 'name': 'rog-ally', 'model': 'ROG Ally RC71L', 'kind': 'handheld', 'version': '1.6',
            'mode': 'game', 'playing': {'title': 'Hades II', 'appid': 1145350},
            'battery': {'percent': 46, 'charging': False, 'pluggedIn': False},
            'cpu': {'use': 38, 'temp': 71}, 'gpu': {'vendor': 'amd', 'use': 74, 'temp': 68, 'vramUsed': 3100, 'vramTotal': 4096},
            'ram': {'used': 9.8, 'total': 15.4}, 'drives': [{'name': 'System', 'used': 301.2, 'total': 1000.2}],
            'uptime': 9240, 'history': history(now, 40, 15, 66, battery=46), 'updated': now}
    pc = {'protocol': 1, 'shared': True, 'name': 'chris-hearth', 'model': 'MS-7D75', 'kind': 'desktop', 'version': '1.6',
          'mode': 'game', 'playing': {'title': 'Cyberpunk 2077', 'appid': 1091500}, 'battery': None,
          'cpu': {'use': 41, 'temp': 63}, 'gpu': {'vendor': 'nvidia', 'use': 97, 'temp': 72, 'vramUsed': 14200, 'vramTotal': 24576},
          'ram': {'used': 21.4, 'total': 62.7},
          'drives': [{'name': 'System', 'used': 512.0, 'total': 2000.4}, {'name': 'Games', 'used': 3410.9, 'total': 4000.8}],
          'uptime': 3 * 86400 + 4 * 3600, 'history': history(now, 55, 20, 60), 'updated': now}
    laptop = {'protocol': 1, 'shared': True, 'name': 'laptop', 'model': 'ThinkPad T14', 'kind': 'laptop', 'version': '1.5',
              'mode': 'desktop', 'playing': None, 'battery': {'percent': 82, 'charging': True, 'pluggedIn': True},
              'cpu': {'use': 6, 'temp': 44}, 'gpu': {'vendor': 'intel', 'use': 2}, 'ram': {'used': 5.2, 'total': 15.5},
              'drives': [{'name': 'System', 'used': 120.4, 'total': 512.1}], 'uptime': 7200, 'history': [], 'updated': now - 5 * 3600000}
    return {'sharing': True, 'devices': [
        {'id': 'a1' * 16, 'name': 'chris-hearth', 'online': True, 'lastSeen': now, 'shared': True, 'status': pc,
         'shares': [{'id': '1', 'name': 'Media'}]},
        {'id': 'b2' * 16, 'name': 'Laptop', 'online': False, 'lastSeen': now - 5 * 3600000, 'stale': True, 'status': laptop, 'wake': True},
        {'id': 'c3' * 16, 'name': 'ROG Ally', 'online': True, 'lastSeen': now, 'shared': True, 'status': ally},
    ]}


if __name__ == '__main__':
    print(json.dumps(sample()))
