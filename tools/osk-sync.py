#!/usr/bin/env python3
"""osk-sync: copies Hearth's on-screen keyboard (ui/osk: hearth-osk.js, hearth-osk.css, words/) to everything that
ships it on its own, so there is one keyboard (docs/OSK.md):
  - Desktop Mode (hosts/linux/desktop/shell/osk: the engine and word lists; OskBoard.qml draws it),
  - Hearth's apps that embed their own page (hosts/hearth-*/ui/osk),
  - Hearth Browser (os/system_files/usr/share/hearth-browser/firefox/hearth/osk).
  python3 tools/osk-sync.py          copy
  python3 tools/osk-sync.py --check  only say which copies differ (exit 1); hosts/linux/test/test_hearth_osk.py runs it
Edit ui/osk, then run this."""
import filecmp
import os
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, 'ui', 'osk')
# where → which parts (js: hearth-osk.js, css: hearth-osk.css, words: words/*.js)
TARGETS = {
    'hosts/linux/desktop/shell/osk': ('js', 'words'),
    'hosts/hearth-time/ui/osk': ('js', 'css', 'words'),
    'hosts/hearth-maps/ui/osk': ('js', 'css', 'words'),
    'hosts/hearth-contacts/ui/osk': ('js', 'css', 'words'),
    'hosts/hearth-gaming/ui/osk': ('js', 'css', 'words'),
    'os/system_files/usr/share/hearth-browser/firefox/hearth/osk': ('js', 'css', 'words'),
}


def files(parts):
    out = []
    if 'js' in parts:
        out.append('hearth-osk.js')
    if 'css' in parts:
        out.append('hearth-osk.css')
    if 'words' in parts:
        out += ['words/' + f for f in sorted(os.listdir(os.path.join(SRC, 'words'))) if f.endswith('.js')]
    return out


def main():
    check = '--check' in sys.argv[1:]
    bad = []
    for dest, parts in TARGETS.items():
        want = files(parts)
        for f in want:
            a, b = os.path.join(SRC, f), os.path.join(ROOT, dest, f)
            if os.path.isfile(b) and filecmp.cmp(a, b, shallow=False):
                continue
            if check:
                bad.append(os.path.join(dest, f))
                continue
            os.makedirs(os.path.dirname(b), exist_ok=True)
            shutil.copyfile(a, b)
            print('copied', os.path.join(dest, f))
        # Word lists that aren't in ui/osk any more
        wd = os.path.join(ROOT, dest, 'words')
        if os.path.isdir(wd):
            for f in os.listdir(wd):
                if 'words/' + f not in want:
                    if check:
                        bad.append(os.path.join(dest, 'words', f) + ' (not in ui/osk)')
                    else:
                        os.remove(os.path.join(wd, f))
    if check and bad:
        print('These copies of the on-screen keyboard differ from ui/osk (run python3 tools/osk-sync.py):')
        for b in bad:
            print('  ' + b)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
