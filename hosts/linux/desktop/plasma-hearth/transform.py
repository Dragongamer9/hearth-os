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


# Names a Quickshell window may use that an Item already has (it's an Item here): renamed in that file.
RENAME = {'ClockPanel.qml': [('function left(sec)', 'function timeLeft(sec)'), ('root.left((', 'root.timeLeft((')],
          # Firehold's style of the shared notifications (FireholdCards.qml draws the cards): Mantel's own pop-ups
          # step aside; Mantel's rules (what pops up, the history) stay the ones deciding.
          'Notifications.qml': [('readonly property var keys: DeskState.popups.slice(0, 4)',
                                 'readonly property var keys: Hearth.style === "firehold" ? [] : DeskState.popups.slice(0, 4)'),
                                # its Reply box takes the keyboard while you type (LayerPanel.qml's wantsKeys)
                                ('WlrLayershell.keyboardFocus: popups.replying ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None',
                                 'wantsKeys: popups.replying')]}
# Small windows of their own (a bar at the top of the screen): a real layer window on Plasma (LayerPanel.qml), their
# edges and margins kept.
OWN_WINDOW = {'VolumeOsd.qml', 'SystemOsd.qml'}


def fix(text, name=''):
    for a, b in RENAME.get(name, []):
        text = text.replace(a, b)
    out = []
    own = name in OWN_WINDOW
    if own:
        text = re.sub(r'(?m)^PanelWindow \{', 'LayerPanel {', text, count=1)
    for line in text.split('\n'):
        if own and re.match(r'^    anchors \{', line):
            out.append(line)
            continue
        if any(p.match(line) for p in DROP):
            out.append('// (plasma-hearth: ' + line.strip()[:60].replace('*/', '') + ')')
            continue
        # A small window inside a file (Notifications.qml's pop-ups): a layer window of its own (LayerPanel.qml).
        line = re.sub(r'^(\s+)PanelWindow \{', r'\1LayerPanel {', line)
        out.append(line.replace('Qt.resolvedUrl("', 'Hearth.shellUrl("'))
    return '\n'.join(out)


def main(d):
    names = []
    for fn in sorted(os.listdir(d)):
        p = os.path.join(d, fn)
        if fn.endswith('.qml'):
            t = open(p, encoding='utf-8').read()
            t2 = fix(t, fn)
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
