#!/usr/bin/python3
"""A stand-in flatpak for the App store's update tests and CI pictures (test_store_extras.py, activities-store.yml):
updates with the installed and the new metadata, so "new permissions" can be shown without a real Flathub.
State: the JSON file in $FAKE_STATE
  {"updates": {"user": [[id, name, version, branch, origin, size]], "system": [...]},
   "meta": {id: {"old": "<keyfile>", "new": "<keyfile>"}}, "remotes": {"user": [[name, title, url, options]], "system": []}}
Each call is added to $FAKE_LOG when it's set."""
import json
import os
import sys

a = sys.argv[1:]
if os.environ.get('FAKE_LOG'):
    with open(os.environ['FAKE_LOG'], 'a') as f:
        f.write(' '.join(a) + '\n')
with open(os.environ['FAKE_STATE']) as f:
    st = json.load(f)
inst = next((x[2:] for x in a if x in ('--user', '--system')), 'system')
pos = [x for x in a[1:] if not x.startswith('-')]


def save():
    with open(os.environ['FAKE_STATE'], 'w') as f:
        json.dump(st, f)


def app_id(r):
    return r.split('//')[0]


cmd = a[0] if a else ''
if cmd == 'remotes':
    for row in st.get('remotes', {}).get(inst, []):
        print('\t'.join(row))
elif cmd == 'remote-ls':
    for row in st.get('updates', {}).get(inst, []):
        print('\t'.join(row))
elif cmd == 'info':
    ids = [r[0] for r in st.get('updates', {}).get(inst, [])] + st.get('installed', {}).get(inst, [])
    if not pos or app_id(pos[-1]) not in ids:
        print('error: %s not installed' % (pos[-1] if pos else '?'), file=sys.stderr)
        sys.exit(1)
    if '--show-metadata' in a:
        print(st.get('meta', {}).get(app_id(pos[-1]), {}).get('old', '[Application]\nname=%s\n' % app_id(pos[-1])))
elif cmd == 'remote-info':
    m = st.get('meta', {}).get(app_id(pos[-1]), {})
    print(m.get('new', m.get('old', '[Application]\nname=%s\n' % app_id(pos[-1]))))
elif cmd == 'update':
    refs = [app_id(r) for r in pos]
    rows = st.get('updates', {}).get(inst, [])
    have = [r[0] for r in rows]
    for r in refs:
        if r not in have:
            print('error: %s not installed' % r, file=sys.stderr)
            sys.exit(1)
    print('Updating… 50%')
    st.setdefault('updates', {})[inst] = [r for r in rows if refs and r[0] not in refs]
    save()
    print('Updating… 100%')
else:
    print('error: unknown command ' + cmd, file=sys.stderr)
    sys.exit(1)
