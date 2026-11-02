#!/usr/bin/env python3
"""Packs Mantel's one-colour icons (hosts/linux/desktop/shell/icons/*-light.svg) into shell/icon-data.js, so Icon.qml
can draw any of them in any colour (the accent you picked, light mode's dark text) instead of only the four tones
that have files. Run it again after adding or changing an icon (hosts/linux/test/test_hearth_theme.py checks).

  python3 tools/make-icon-data.py
"""
import json
import os
import sys

SHELL = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'hosts', 'linux', 'desktop', 'shell')


def build():
    icons = {}
    folder = os.path.join(SHELL, 'icons')
    for f in sorted(os.listdir(folder)):
        if f.endswith('-light.svg'):
            with open(os.path.join(folder, f)) as fh:
                icons[f[:-len('-light.svg')]] = fh.read().strip().replace('#f4ece3', '{c}')
    return ('.pragma library\n// Made by tools/make-icon-data.py from icons/*-light.svg: each icon with {c} where its colour goes.\n'
            'var icons = ' + json.dumps(icons, indent=0, sort_keys=True) + ';\n')


if __name__ == '__main__':
    out = os.path.join(SHELL, 'icon-data.js')
    if '--check' in sys.argv:
        with open(out) as f:
            sys.exit(0 if f.read() == build() else 1)
    with open(out, 'w') as f:
        f.write(build())
    print(out)
