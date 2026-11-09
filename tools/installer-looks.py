#!/usr/bin/env python3
"""The USB installer's looks (ui/installer.html's look picker): the same eight looks as Mantel and hearthos.fun, with
their colours taken from Mantel's own palettes (hosts/linux/desktop/theme/palettes.json). Writes ui/installer-looks.js,
which the installer page loads; without it (or with a broken one) the page stays in its built-in Hearth dark look.

    python3 tools/installer-looks.py            # write ui/installer-looks.js
    python3 tools/installer-looks.py --check    # fail if it's out of date (tools/installer-check.py runs this)
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAL = os.path.join(ROOT, 'hosts/linux/desktop/theme/palettes.json')
OUT = os.path.join(ROOT, 'ui/installer-looks.js')
COLOURS = ('bg', 'bg2', 'panel', 'panel2', 'panel3', 'line', 'text', 'muted', 'danger', 'ember', 'flame', 'ink')
STYLE = ('label', 'desc', 'order', 'font', 'frame', 'islandStyle', 'scanlines', 'hints')


def looks(pal):
    out = {}
    items = [(k, v) for k, v in pal.items() if not k.startswith('_') and isinstance(v, dict) and v.get('dark')]
    for k, v in sorted(items, key=lambda kv: kv[1].get('order', 99)):
        o = {s: v[s] for s in STYLE if s in v}
        for mode in ('dark', 'light'):
            src = v.get(mode) or v['dark']
            o[mode] = {c: src[c] for c in COLOURS if c in src}
        out[k] = o
    return out


def text():
    pal = json.load(open(PAL, encoding='utf-8'))
    body = json.dumps(looks(pal), indent=1, ensure_ascii=False)
    return ('// Made by tools/installer-looks.py from hosts/linux/desktop/theme/palettes.json (Mantel\'s looks): don\'t edit.\n'
            'window.HEARTH_LOOKS = ' + body + ';\n')


def main(argv):
    want = text()
    if '--check' in argv:
        try:
            have = open(OUT, encoding='utf-8').read()
        except OSError:
            have = ''
        if have != want:
            print('ui/installer-looks.js is out of date with palettes.json: run python3 tools/installer-looks.py')
            return 1
        print('ui/installer-looks.js matches palettes.json')
        return 0
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(want)
    print('wrote ui/installer-looks.js')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
