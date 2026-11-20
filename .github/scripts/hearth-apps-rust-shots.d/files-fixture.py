#!/usr/bin/env python3
"""CI's stand-in in front of hearth-files (HEARTH_FILES_HELPER, files.sh) for the pictures of what CI's container has
no hardware or network for: This PC with a nearly full drive and the network locations (the home server's drive with
its letter, a Hearth Link phone), the left menu's shared folders, signing in to one, and a folder's sharing. Everything
else goes to the real hearth-files, untouched. Only the pictures that name it use it."""
import json
import os
import subprocess
import sys

REAL = os.path.join(os.environ.get('HEARTH_SHELL_DIR', '/usr/share/nimbus/desktop/shell'), 'hearth-files')
args = sys.argv[1:]
cmd = args[0] if args else ''
NET = {'name': 'hearth-server', 'share': 'Media', 'home': 'hearth-server.local', 'device': 'hearth-server', 'user': 'chris'}


def real():
    r = subprocess.run(['python3', REAL] + args, capture_output=True, text=True, timeout=60)
    lines = [l for l in r.stdout.strip().splitlines() if l.strip()]
    return json.loads(lines[-1]) if lines and lines[-1].startswith('{') else {}


if cmd == 'net-open':
    pw = ''
    try:
        pw = json.loads(os.environ.get('HEARTH_INPUT') or '{}').get('password') or ''
    except ValueError:
        pass
    out = {'ok': False, 'need': 'password', 'user': 'chris'}
    if pw:
        out['error'] = 'Wrong user name or password'
    print(json.dumps(out))
elif cmd == 'sharing' and len(args) == 2:
    print(json.dumps({'path': os.path.realpath(args[1]), 'shared': True, 'samba': True,
                      'share': {'id': 'desktop', 'name': 'Desktop', 'write': False,
                                'how': {'windows': '\\\\HEARTH-PC\\Desktop', 'mac': 'smb://hearth-pc.local/Desktop',
                                        'away': 'smb://hearth-pc.tail0a1b2.ts.net/Desktop'}}}))
elif cmd == 'this-pc':
    v = real()
    v.setdefault('drives', []).append({'id': 'games', 'label': 'Games (D:)', 'letter': 'D', 'path': '/run/media/ci/Games',
                                       'total': 512110190592, 'free': 61203283968, 'kind': 'ssd', 'mounted': True})
    v.setdefault('network', []).extend([
        {'id': 'net:hearth-server/Media', 'label': 'Media (\\\\hearth-server) (Z:)', 'letter': 'Z', 'kind': 'share',
         'path': '/run/user/1000/gvfs/smb-share:server=hearth-server.local,share=media', 'host': 'hearth-server',
         'total': 3999688294400, 'free': 1846835937280, 'net': NET},
        {'id': 'dev:phone', 'label': 'Pixel 8 (Chris)', 'kind': 'device', 'sub': 'Hearth Link · online'},
    ])
    print(json.dumps(v))
elif cmd == 'places':
    v = real()
    v.setdefault('places', []).extend([
        {'id': 'net:hearth-server/Media', 'label': 'Media', 'section': 'network', 'kind': 'network', 'sub': 'hearth-server', 'net': NET},
        {'id': 'dev:phone', 'label': 'Pixel 8 (Chris)', 'section': 'devices', 'kind': 'device', 'sub': 'Hearth Link · online'},
    ])
    print(json.dumps(v))
else:
    os.execvp('python3', ['python3', REAL] + args)
