#!/usr/bin/env python3
"""The board's Hearth Shield day (board 15) as AdGuard Home's API answers it (/control/stats, /control/filtering/status,
/control/querylog, /control/clients) and as hearth-server-dns keeps Hearth's side (its status, each device's day,
each day's totals), so Hearth Shield's own reading code (hosts/hearth-ui/apps/shield/src/data.rs) draws it. Plus the
names given to the devices in Hearth Shield (shield.json).

  shield-fixture.py FIXTURE.json CONFIG_DIR
"""
import json
import math
import os
import sys

out, cfg = sys.argv[1], sys.argv[2]
DEVS = [('192.168.1.20', 'TCL 55" TV', 'Living room · Google TV', 9312, 4870), ('192.168.1.21', 'Phone', 'on Wi-Fi', 8840, 2104),
        ('192.168.1.5', 'Nvidia PC', 'the server itself', 12410, 1380), ('192.168.1.22', 'Laptop', 'Hearth OS', 7022, 1290),
        ('192.168.1.23', 'ROG Ally', 'Hearth OS', 3180, 740), ('192.168.1.24', 'Xbox Series X', 'Living room', 4210, 702),
        ('192.168.1.25', 'Tablet', 'on Wi-Fi', 1210, 340), ('192.168.1.26', "Dad's PC", 'Hearth OS', 880, 210), ('192.168.1.27', 'Printer', 'on Wi-Fi', 520, 120),
        ('192.168.1.28', 'Doorbell', 'on Wi-Fi', 410, 98), ('192.168.1.29', 'Speaker', 'Kitchen', 219, 50)]
ASKED, BLOCKED = sum(d[3] for d in DEVS), sum(d[4] for d in DEVS)
assert (ASKED, BLOCKED) == (48213, 11904), (ASKED, BLOCKED)


def spread(total, seed):
    w = [0.35 + 0.65 * abs(math.sin(i * 0.9 + seed)) * (0.4 if i < 7 else 1.0) for i in range(24)]
    v = [int(total * x / sum(w)) for x in w]
    v[-2] += total - sum(v)
    return v


q, b = spread(ASKED, 0.3), spread(BLOCKED, 1.1)
b = [min(x, y) for x, y in zip(b, q)]
b[-3] += BLOCKED - sum(b)
TOP = [('acr0.samsungcloudsolution.com', 3402, ['192.168.1.20']), ('imasdk.googleapis.com', 2210, ['192.168.1.20', '192.168.1.21']),
       ('crashlytics.com', 1488, ['192.168.1.21']), ('googleads.g.doubleclick.net', 1104, ['192.168.1.22', '192.168.1.21']),
       ('ads.samsungads.com', 986, ['192.168.1.20']), ('pixel.facebook.com', 640, ['192.168.1.22']), ('launcher-ads.epicgames.com', 312, ['192.168.1.5'])]
log = []
for i, (dom, _, who) in enumerate(TOP):
    for w in who:
        log.append({'time': '2026-11-26T01:%02d:%02dZ' % (20 - i, 30), 'question': {'name': dom, 'type': 'A'}, 'client': w, 'reason': 'FilteredBlackList'})
for i, (ip, *_rest) in enumerate(DEVS):
    log.append({'time': '2026-11-26T01:%02d:10Z' % (5 + i), 'question': {'name': 'www.example%d.org' % i, 'type': 'A'}, 'client': ip, 'reason': 'NotFilteredNotFound'})
log.sort(key=lambda e: e['time'], reverse=True)
LISTS = {'main': ('Ads and trackers (main list)', 1, True, 612000), 'tv': ('Smart TV spying', 2, True, 241000), 'phone': ('Phone app trackers', 3, True, 168000),
         'scam': ('Scam and malware sites', 4, True, 151000), 'games': ('Game launcher ads', 5, True, 38000), 'adult': ('Adult sites', 6, False, 92000)}
fx = {
    '/control/stats': {'time_units': 'hours', 'num_dns_queries': ASKED + 900, 'num_blocked_filtering': BLOCKED + 200, 'dns_queries': q, 'blocked_filtering': b,
                       'top_clients': [{d[0]: d[3]} for d in DEVS], 'top_blocked_domains': [{t[0]: t[1]} for t in TOP]},
    '/control/filtering/status': {'enabled': True, 'filters': [{'id': v[1], 'enabled': v[2], 'url': 'https://lists.example/%s.txt' % k, 'name': v[0], 'rules_count': v[3],
                                                                'last_updated': '2026-11-26T03:00:00Z'} for k, v in LISTS.items()]},
    '/control/querylog?limit=500': {'data': log},
    '/control/clients': {'clients': [], 'auto_clients': [{'ip': d[0], 'name': d[1].lower().replace(' ', '-'), 'source': 'rdns'} for d in DEVS]},
    'status': {'ads': {'on': True, 'running': True, 'pausedUntil': 0, 'safeSearch': False,
                       'allow': ['site%d.example.com' % i for i in range(14)], 'block': ['tracker%d.example.net' % i for i in range(9)],
                       'lists': {k: v[2] for k, v in LISTS.items()}, 'listNames': {k: {'label': v[0], 'id': v[1], 'on': v[2]} for k, v in LISTS.items()},
                       'view': {'devices': []}}, 'lan': '192.168.1.5'},
    'clients-today': {'day': '2026-11-26', 'last': '2026-11-26T23:59:00Z', 'clients': {d[0]: {'asked': d[3], 'blocked': d[4]} for d in DEVS}},
    'days': {'2026-11-%02d' % (d + 1): {'asked': 40000 + 900 * (d % 7), 'blocked': 9000 + 300 * (d % 5)} for d in range(25)} | {'2026-11-26': {'asked': ASKED, 'blocked': BLOCKED}},
}
json.dump(fx, open(out, 'w'))
os.makedirs(os.path.join(cfg, 'hearth'), exist_ok=True)
json.dump({'names': {d[0]: {'name': d[1], 'about': d[2]} for d in DEVS}}, open(os.path.join(cfg, 'hearth', 'shield.json'), 'w'))
