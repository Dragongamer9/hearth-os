#!/usr/bin/python3
"""Stand-ins for hearth_restore.py and hearth_reliability.py in CI (hearth-apps-rust.yml): a container has no restore
points and no journal, so the pictures of System Restore and Reliability history get the same JSON shapes those
helpers print, with believable content. Never used outside CI.
  hearth-apps-rust-fixtures.py restore overview|changes ID|snapshot ...|apply ID [--settings-only]
  hearth-apps-rust-fixtures.py reliability summary [--weeks]"""
import datetime
import json
import sys
import time

now = int(time.time())
day = 86400


def restore(args):
    cmd = args[0] if args else ''
    if cmd == 'overview':
        pts = [
            {'id': 'p6', 'created': now - 3 * 3600, 'kind': 'update', 'type': 'Before an update', 'title': 'Before updating to 1.15', 'version': '1.14', 'files': 14, 'changed': 3, 'added': 1,
             'plan': {'action': 'none', 'to': '1.14'}},
            {'id': 'p5', 'created': now - day - 2 * 3600, 'kind': 'manual', 'type': 'Made by you', 'title': 'Before trying the Gaming look', 'version': '1.14', 'files': 14, 'changed': 7, 'added': 0,
             'plan': {'action': 'none', 'to': '1.14'}},
            {'id': 'p4', 'created': now - 4 * day, 'kind': 'update', 'type': 'Before an update', 'title': 'Before updating to 1.14', 'version': '1.13', 'files': 13, 'changed': 12, 'added': 2,
             'plan': {'action': 'rollback', 'from': '1.14', 'to': '1.13'}},
            {'id': 'p3', 'created': now - 8 * day, 'kind': 'scheduled', 'type': 'Weekly', 'title': 'Weekly restore point', 'version': '1.13', 'files': 13, 'changed': 15, 'added': 2,
             'plan': {'action': 'blocked', 'to': '1.13', 'why': 'Hearth 1.13 is no longer kept on this PC, so only the settings can go back.'}},
            {'id': 'p2', 'created': now - 15 * day, 'kind': 'scheduled', 'type': 'Weekly', 'title': 'Weekly restore point', 'version': '1.12', 'files': 12, 'changed': 19, 'added': 3,
             'plan': {'action': 'blocked', 'to': '1.12', 'why': 'Hearth 1.12 is no longer kept on this PC, so only the settings can go back.'}},
            {'id': 'p1', 'created': now - 22 * day, 'kind': 'install', 'type': 'First start', 'title': 'Hearth was installed', 'version': '1.12', 'files': 9, 'changed': 24, 'added': 5,
             'plan': {'action': 'blocked', 'to': '1.12', 'why': 'Hearth 1.12 is no longer kept on this PC, so only the settings can go back.'}},
            {'id': 'p0', 'created': now - 29 * day, 'kind': 'scheduled', 'type': 'Weekly', 'title': 'Weekly restore point', 'version': '1.11', 'files': 9, 'changed': 25, 'added': 5,
             'plan': {'action': 'blocked', 'to': '1.11'}},
        ]
        out = {'running': '1.14', 'status': {'booted': {'version': '1.14'}}, 'points': pts, 'keep': 10}
    elif cmd == 'changes':
        out = {'changes': [
            {'file': 'desktop.json', 'key': 'look', 'now': '"gaming"', 'then': '"steam"'},
            {'file': 'desktop.json', 'key': 'accent', 'now': '"#ff2a2a"', 'then': '""'},
            {'file': 'desktop.json', 'key': 'wallpaper', 'now': '"neon-grid.jpg"', 'then': '""'},
            {'file': 'sound.json', 'key': 'volume', 'now': '35', 'then': '60'},
            {'file': 'taskmgr.json', 'key': 'speed', 'now': '"high"', 'then': '"normal"'},
            {'file': 'controller.json', 'key': 'deadzone', 'now': '0.12', 'then': '0.08'},
            {'file': 'power.json', 'key': 'mode', 'now': '"performance"', 'then': '"balanced"'},
        ], 'added': ['firehold.json']}
    elif cmd == 'snapshot':
        out = {'ok': True, 'id': 'p7'}
    elif cmd == 'apply':
        out = {'ok': True, 'restart': False, 'changed': 3, 'plan': {'action': 'none'}}
    else:
        print('error: unknown'); sys.exit(1)
    print(json.dumps(out))


def reliability(args):
    weeks = '--weeks' in args
    n = 14
    rows = []
    today = datetime.date.today()
    for i in range(n):
        d = today - datetime.timedelta(days=(n - 1 - i) * (7 if weeks else 1))
        ts = int(datetime.datetime(d.year, d.month, d.day, 12).timestamp())
        ev, counts = [], {'app': 0, 'hearth': 0, 'warn': 0, 'info': 0}
        if i in (3, 9):
            ev.append({'cat': 'app', 't': 'Steam stopped working', 's': 'steamwebhelper ended with a crash (SIGSEGV)', 'time': ts + 3600 * 2,
                       'detail': 'Process 48211 (steamwebhelper) of user 1000 dumped core.\n#0 0x00007f3a libcef.so + 0x4a21f3\n#1 0x00007f3a libcef.so + 0x4a1e10'})
            counts['app'] += 1
        if i == 9:
            ev.append({'cat': 'hearth', 't': 'Desktop Mode restarted', 's': 'KDE Plasma ended on its own; Hearth started it again', 'time': ts + 3600 * 3, 'detail': 'nimbus-session: desktop mode: KDE Plasma ended (code 1)'})
            counts['hearth'] += 1
        if i in (5, 11, 12):
            ev.append({'cat': 'warn', 't': 'Wi-Fi dropped for a moment', 's': 'Home-5G · back after 8 s', 'time': ts - 3600 * 4})
            counts['warn'] += 1
        if i in (2, 10, 13):
            ev.append({'cat': 'info', 't': 'Restore point made', 's': 'Before updating to 1.15', 'time': ts - 3600})
            counts['info'] += 1
        score = max(1.0, 10.0 - 2.5 * counts['app'] - 2.0 * counts['hearth'] - 0.5 * counts['warn'])
        rows.append({'date': d.isoformat(), 'score': round(score, 1), 'counts': counts, 'events': ev})
    print(json.dumps({'rows': rows, 'today': today.isoformat()}))


if __name__ == '__main__':
    import os
    me = os.path.basename(sys.argv[0])
    if me.startswith('restore'):            # as HEARTH_RESTORE_HELPER (a link named restore-fixture.py)
        restore(sys.argv[1:])
    elif me.startswith('reliability'):      # as HEARTH_RELIABILITY_HELPER
        reliability(sys.argv[1:])
    elif sys.argv[1] == 'restore':
        restore(sys.argv[2:])
    else:
        reliability(sys.argv[2:])
