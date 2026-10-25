#!/usr/bin/env python3
"""Makes qml/Icons.js from the web UI's icon set (the `const I = {...}` block in ui/index.html), so the Qt renderer
draws the same icons. Each icon becomes a list of [svgPathData, filled] drawn by Icon.qml with QtQuick.Shapes
(circles and rectangles are turned into paths). Run from anywhere:  python3 hosts/hearth-qml/tools/gen-icons.py"""
import json, os, re, sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(here, '..', '..', '..'))
html = open(os.path.join(root, 'ui', 'index.html'), encoding='utf-8').read()
start = html.index('const I = {')
end = html.index('\n};', start)
block = html[start:end]

def num(a, k, d=0.0):
    m = re.search(r'\b' + k + r'="([-\d.]+)"', a)
    return float(m.group(1)) if m else d

def f(x):
    s = ('%.3f' % x).rstrip('0').rstrip('.')
    return s if s not in ('-0', '') else '0'

def circle(a):
    cx, cy, r = num(a, 'cx'), num(a, 'cy'), num(a, 'r')
    return 'M%s %sA%s %s 0 1 0 %s %sA%s %s 0 1 0 %s %sZ' % (f(cx - r), f(cy), f(r), f(r), f(cx + r), f(cy), f(r), f(r), f(cx - r), f(cy))

def rect(a):
    x, y, w, h = num(a, 'x'), num(a, 'y'), num(a, 'width'), num(a, 'height')
    rx = num(a, 'rx', num(a, 'ry', 0)); rx = min(rx, w / 2, h / 2)
    if rx <= 0:
        return 'M%s %sH%sV%sH%sZ' % (f(x), f(y), f(x + w), f(y + h), f(x))
    return ('M%s %sH%sA%s %s 0 0 1 %s %sV%sA%s %s 0 0 1 %s %sH%sA%s %s 0 0 1 %s %sV%sA%s %s 0 0 1 %s %sZ' %
            (f(x + rx), f(y), f(x + w - rx), f(rx), f(rx), f(x + w), f(y + rx), f(y + h - rx), f(rx), f(rx), f(x + w - rx), f(y + h),
             f(x + rx), f(rx), f(rx), f(x), f(y + h - rx), f(y + rx), f(rx), f(rx), f(x + rx), f(y)))

icons = {}
for m in re.finditer(r"^\s*([A-Za-z0-9_]+)\s*:\s*'(<svg.*?</svg>)'", block, re.M):
    name, svg = m.group(1), m.group(2)
    parts = []
    for el in re.finditer(r'<(path|circle|rect)\b([^>]*)/?>', svg):
        kind, a = el.group(1), el.group(2)
        filled = 'fill="currentColor"' in a
        d = re.search(r'\bd="([^"]+)"', a).group(1) if kind == 'path' else circle(a) if kind == 'circle' else rect(a)
        parts.append([d, filled])
    if parts:
        icons[name] = parts

out = os.path.join(here, '..', 'qml', 'Icons.js')
with open(out, 'w', encoding='utf-8') as o:
    o.write('.pragma library\n')
    o.write('// Made by hosts/hearth-qml/tools/gen-icons.py from the web UI\'s icons (ui/index.html `const I`): do not edit.\n')
    o.write('// name → [[svg path data, filled], ...] on a 24×24 grid, stroked 1.8 wide with round ends (Icon.qml).\n')
    o.write('var I = ' + json.dumps(icons, separators=(',', ':')) + ';\n')
print('%d icons → %s' % (len(icons), os.path.relpath(out, root)))
