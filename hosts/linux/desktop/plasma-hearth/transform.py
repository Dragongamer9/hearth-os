#!/usr/bin/python3
"""Mantel's files for Plasma (plasma-hearth/install.sh): takes out only what's Quickshell's own (its imports and the
layer-shell lines of its windows), points Qt.resolvedUrl("x") at Mantel's folder (Hearth.shellUrl), and writes the
qmldir that makes the shims and Mantel's singletons work as they do in Mantel. Everything else stays as it is.
    transform.py DIR
"""
import os
import re
import sys

DROP = [
    re.compile(r'^\s*import Quickshell(\.[A-Za-z.]+)?\s*$'),
    re.compile(r'^\s*WlrLayershell\.[A-Za-z]+\s*:'),
    re.compile(r'^\s*exclusionMode\s*:'),
    re.compile(r'^\s*mask\s*:\s*Region\b.*\}\s*$'),
    re.compile(r'^\s*focusable\s*:'),
    # A PanelWindow's own edges (it's an Item on Plasma: the panel or the layer window places it).
    re.compile(r'^    anchors \{ *(top|bottom|left|right): *(true|false|edge[^;]*)(; *(top|bottom|left|right): *(true|false|edge[^;]*))* *\}\s*$'),
]


def fix(text):
    out = []
    for line in text.split('\n'):
        if any(p.match(line) for p in DROP):
            out.append('// (plasma-hearth: ' + line.strip()[:60].replace('*/', '') + ')')
            continue
        out.append(line.replace('Qt.resolvedUrl("', 'Hearth.shellUrl("'))
    return '\n'.join(out)


def main(d):
    names = []
    for fn in sorted(os.listdir(d)):
        p = os.path.join(d, fn)
        if fn.endswith('.qml'):
            t = open(p, encoding='utf-8').read()
            t2 = fix(t)
            if t2 != t:
                open(p, 'w', encoding='utf-8').write(t2)
            if not fn[0].isupper():
                continue
            single = re.search(r'^\s*pragma Singleton\s*$', t2, re.M) is not None
            names.append(('singleton ' if single else '') + fn[:-4] + ' 1.0 ' + fn)
    with open(os.path.join(d, 'qmldir'), 'w') as f:
        f.write('\n'.join(names) + '\n')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))
