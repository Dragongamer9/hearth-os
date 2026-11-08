#!/usr/bin/env python3
"""Hearth's Decky plugins for the plugins release (docs/PLUGIN-UPDATES.md, .github/workflows/decky-plugins-release.yml).

  decky_plugins.py list [ROOT]                     every plugin folder: hosts/decky-hearth-<name>/ with package.json and
                                                   package.sh (never archive/, never a retired one), one id per line
  decky_plugins.py retired                         the retired plugins (RETIRED), one id per line
  decky_plugins.py check [ROOT]                    fails (::error) when a plugin lacks the shared Hearth OS check
                                                   (hosts/decky-shared) or the load-check mark plugin updates rely on
  decky_plugins.py manifest DIR VERSION MIN_HEARTH MIN_DECKY [ROOT]
                                                   plugins.json in DIR from DIR/<id>.zip for every plugin found

A new plugin needs nothing here: make hosts/decky-hearth-<name> from hosts/decky-shared/template and it is found.
"""
import hashlib
import json
import os
import pathlib
import re
import sys
import zipfile

PLUGIN_ID = re.compile(r'decky-hearth-[a-z0-9][a-z0-9-]{0,40}')   # the same rule as hearth-decky's PLUGIN_ID

# Plugins the owner retired (docs/PLUGIN-UPDATES.md, "Retiring a plugin"): their code stays in the repository, but they
# are never built into a plugins release, and every release's plugins.json lists them under "retired", so devices
# remove their downloaded copies and no longer offer them. Taking an id out of here offers the plugin again.
#   decky-hearth-look: Hearth Classic, retired by the owner (November 2026); its home and skin live on as the
#                      "Hearth Classic" era of Hearth Eras (hosts/decky-hearth-eras), which reuses its code.
#   decky-hearth-music: Hearth Music, retired by the owner (November 2026) for ThemeDeck (docs/THEMEDECK.md); its code
#                      is in archive/hosts/decky-hearth-music. It was built into the image: devices drop the built-in
#                      copy with the Hearth OS update that ships ThemeDeck (hearth-decky's RETIRED_BUILTIN), and a
#                      downloaded copy with the next plugins release.
RETIRED = ('decky-hearth-look', 'decky-hearth-music')


def _json(p):
    try:
        v = json.loads(pathlib.Path(p).read_text())
        return v if isinstance(v, dict) else {}
    except (OSError, ValueError):
        return {}


def discover(root):
    """[{id, dir, name, description}] sorted by id; retired plugins (RETIRED) are left out."""
    hosts = pathlib.Path(root) / 'hosts'
    out = []
    for d in sorted(hosts.iterdir()) if hosts.is_dir() else []:
        if not d.is_dir() or d.is_symlink() or not PLUGIN_ID.fullmatch(d.name) or d.name in RETIRED:
            continue
        if not ((d / 'package.json').is_file() and (d / 'package.sh').is_file()):
            continue
        meta = _json(d / 'plugin.json')
        name = meta.get('name') if isinstance(meta.get('name'), str) else ''
        pub = meta.get('publish') if isinstance(meta.get('publish'), dict) else {}
        desc = pub.get('description') if isinstance(pub.get('description'), str) else ''
        out.append({'id': d.name, 'dir': str(d), 'name': (name or d.name)[:60], 'description': desc[:300]})
    return out


def problems(root):
    """What each plugin lacks of the shared Hearth OS check and the load mark. [] when all is well."""
    root = pathlib.Path(root)
    canon_tsx = root / 'hosts/decky-shared/hearth-only.tsx'
    bad = []
    for p in discover(root):
        d = pathlib.Path(p['dir'])
        n = p['id']
        sh = (d / 'package.sh').read_text() if (d / 'package.sh').is_file() else ''
        main = (d / 'main.py').read_text() if (d / 'main.py').is_file() else ''
        index = (d / 'src/index.tsx').read_text() if (d / 'src/index.tsx').is_file() else ''
        if 'hosts/decky-shared/hearth_only.py' not in sh:
            bad.append('%s: package.sh doesn\'t copy hosts/decky-shared/hearth_only.py' % n)
        if 'import hearth_only' not in main:
            bad.append('%s: main.py doesn\'t import hearth_only (the Hearth OS check)' % n)
        if "hearth_only.status(" not in main or '_main' not in main:
            bad.append('%s: main.py\'s _main doesn\'t check hearth_only.status first' % n)
        if 'hearth_only.mark_loaded_later(' not in main:
            bad.append('%s: main.py never writes the load mark (hearth_only.mark_loaded_later)' % n)
        if not canon_tsx.is_file() or not (d / 'src/hearth-only.tsx').is_file() or \
                (d / 'src/hearth-only.tsx').read_text() != canon_tsx.read_text():
            bad.append('%s: src/hearth-only.tsx isn\'t the copy of hosts/decky-shared/hearth-only.tsx' % n)
        if '<HearthOnly' not in index:
            bad.append('%s: src/index.tsx doesn\'t wrap its panel in <HearthOnly>' % n)
    return bad


def manifest(d, version, min_hearth, min_decky, root):
    d = pathlib.Path(d)
    plugins = []
    for p in discover(root):
        z = d / (p['id'] + '.zip')
        data = z.read_bytes()
        with zipfile.ZipFile(z) as zf:
            pv = json.loads(zf.read(p['id'] + '/package.json'))['version']
        plugins.append({'id': p['id'], 'name': p['name'], 'description': p['description'], 'version': pv,
                        'archive': z.name, 'sha256': hashlib.sha256(data).hexdigest(), 'size': len(data),
                        'min_hearth_os': min_hearth, 'min_decky': min_decky})
    # "retired": plugins taken away from devices (RETIRED); never one that this release also ships.
    shipped = {p['id'] for p in plugins}
    m = {'format': 1, 'version': version, 'plugins': plugins,
         'retired': sorted(r for r in RETIRED if PLUGIN_ID.fullmatch(r) and r not in shipped)}
    (d / 'plugins.json').write_text(json.dumps(m, indent=2) + '\n')
    return m


def main(argv):
    cmd = argv[1] if len(argv) > 1 else ''
    if cmd == 'list':
        for p in discover(argv[2] if len(argv) > 2 else '.'):
            print(p['id'])
    elif cmd == 'retired':
        for r in RETIRED:
            print(r)
    elif cmd == 'check':
        bad = problems(argv[2] if len(argv) > 2 else '.')
        for b in bad:
            print('::error title=Decky plugin::%s' % b)
        if bad:
            return 1
        print('::notice title=Decky plugins::%s' % ', '.join(p['id'] for p in discover(argv[2] if len(argv) > 2 else '.')))
    elif cmd == 'manifest' and len(argv) in (6, 7):
        m = manifest(argv[2], argv[3], argv[4], argv[5], argv[6] if len(argv) > 6 else '.')
        print(json.dumps(m, indent=2))
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
