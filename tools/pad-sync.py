#!/usr/bin/env python3
"""pad-sync: copies Hearth's controller reader (ui/pad/hearth-gamepad.js, docs/CONTROLLER-FOCUS.md) next to every app page
that reads the controller (pages in PAGES load it from ui/pad itself), so there is one way the apps read it:
  python3 tools/pad-sync.py          copy
  python3 tools/pad-sync.py --check  only say which copies differ or which pages don't load it (exit 1);
                                     hosts/linux/test/test_hearth_pad_focus.py runs it
Edit ui/pad/hearth-gamepad.js, then run this. An app in TARGETS loads it with <script src="hearth-gamepad.js"> before its
own scripts, reads the controller with HearthGamepad.pads() and runs its controller loop with HearthGamepad.loop(fn)."""
import filecmp
import os
import re
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, 'ui', 'pad', 'hearth-gamepad.js')
# The app's page folder → the file with its controller loop (relative to that folder).
TARGETS = {
    'hosts/hearth-calculator/ui': 'app.js',
    'hosts/hearth-camera/ui': 'nav.js',
    'hosts/hearth-connect/ui': 'index.html',
    'hosts/hearth-contacts/ui': 'app.js',
    'hosts/hearth-disks/ui': 'index.html',
    'hosts/hearth-documents/ui': 'app.js',
    'hosts/hearth-emoji/ui': 'index.html',
    'hosts/hearth-fonts/ui': 'index.html',
    'hosts/hearth-gaming/ui': 'kit.js',
    'hosts/hearth-maps/ui': 'app.js',
    'hosts/hearth-nearby/ui': 'index.html',
    'hosts/hearth-office/ui': 'core.js',
    'hosts/hearth-paint/ui': 'index.html',
    'hosts/hearth-photos/ui': 'app.js',
    'hosts/hearth-player/ui': 'app.js',
    'hosts/hearth-recorder/ui': 'nav.js',
    'hosts/hearth-share/ui': 'nav.js',
    'hosts/hearth-time/ui': 'hearth.js',
    'hosts/hearth-tools/ui': 'common.js',
}
TAG = '<script src="hearth-gamepad.js"></script>'
# Pages next to ui/pad that load the reader from there (no copy): the page → its src. The USB installer's screens
# (ui/installer.html, shown by the Hearth app, io.github.dragongamer9.hearth in desktop/shell/controller-apps.json).
PAGES = {
    'ui/installer.html': 'pad/hearth-gamepad.js',
}


def pages(folder):
    """The folder's pages that run scripts (they all load the reader before their own)."""
    out = []
    for f in sorted(os.listdir(folder)):
        if f.endswith('.html'):
            with open(os.path.join(folder, f), encoding='utf-8') as fh:
                if '<script' in fh.read():
                    out.append(f)
    return out


def problems():
    bad = []
    for dest, loop in TARGETS.items():
        folder = os.path.join(ROOT, dest)
        copy = os.path.join(folder, 'hearth-gamepad.js')
        if not (os.path.isfile(copy) and filecmp.cmp(SRC, copy, shallow=False)):
            bad.append(dest + '/hearth-gamepad.js differs from ui/pad (run python3 tools/pad-sync.py)')
        for p in pages(folder):
            with open(os.path.join(folder, p), encoding='utf-8') as fh:
                html = fh.read()
            first = html.find('<script')
            if html.find(TAG) != first:
                bad.append('%s/%s: %s must be its first script' % (dest, p, TAG))
        with open(os.path.join(folder, loop), encoding='utf-8') as fh:
            code = fh.read()
        if 'navigator.getGamepads' in code:
            bad.append('%s/%s reads navigator.getGamepads() itself: use HearthGamepad.pads()' % (dest, loop))
        if 'HearthGamepad.loop(' not in code:
            bad.append('%s/%s: its controller loop should run with HearthGamepad.loop(fn)' % (dest, loop))
    for page, src in PAGES.items():
        with open(os.path.join(ROOT, page), encoding='utf-8') as fh:
            html = fh.read()
        tag = '<script src="%s"></script>' % src
        if html.find(tag) == -1 or html.find(tag) != html.find('<script'):
            bad.append('%s: %s must be its first script' % (page, tag))
        if 'navigator.getGamepads' in html:
            bad.append('%s reads navigator.getGamepads() itself: use HearthGamepad.pads()' % page)
        if 'HearthGamepad.loop(' not in html:
            bad.append('%s: its controller loop should run with HearthGamepad.loop(fn)' % page)
    return bad


def main():
    if '--check' in sys.argv[1:]:
        bad = problems()
        for b in bad:
            print(b)
        return 1 if bad else 0
    for dest in TARGETS:
        folder = os.path.join(ROOT, dest)
        copy = os.path.join(folder, 'hearth-gamepad.js')
        if not (os.path.isfile(copy) and filecmp.cmp(SRC, copy, shallow=False)):
            shutil.copyfile(SRC, copy)
            print('copied', os.path.relpath(copy, ROOT))
        for p in pages(folder):
            path = os.path.join(folder, p)
            with open(path, encoding='utf-8') as fh:
                html = fh.read()
            if TAG in html:
                continue
            i = html.find('<script')
            line_start = html.rfind('\n', 0, i) + 1
            indent = re.match(r'[ \t]*', html[line_start:i]).group(0) if html[line_start:i].strip() == '' else ''
            html = html[:i] + TAG + ('\n' + indent if indent or html[line_start:i].strip() == '' else '') + html[i:]
            with open(path, 'w', encoding='utf-8') as fh:
                fh.write(html)
            print('loads it:', os.path.relpath(path, ROOT))
    return 0


if __name__ == '__main__':
    sys.exit(main())
