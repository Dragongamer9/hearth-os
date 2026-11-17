#!/usr/bin/env python3
"""Mantel on KWin, the window list end to end (kwin-windows.yml): KWin on a virtual screen with Hearth's KWin defaults
(desktop/kwin/windows/xdg), Mantel (the real shell) started with HEARTH_WM=kwin, so its WinList starts
hearth-kwin-bridge, which loads KWin's hearthwindows script. Two test windows (foot) are opened and Mantel's taskbar
ways (WinList's Win objects and functions, over quickshell ipc) and the bridge's own commands move them about; what
KWin did comes back in the window list ($XDG_RUNTIME_DIR/hearth/windows.json), which the script makes from KWin's own
window state (the snap zone from the window's real place on screen).

  e2e-kwin-windows.py SHELL_DIR OUT_DIR SHOT_COMMAND     (SHOT_COMMAND gets the picture's name as $1)

Each check is a ::notice:: (passed) or ::error:: (failed) line; the exit code is the number that failed.
Pictures: OUT_DIR/kwin-*.png.
"""
import json
import os
import subprocess
import sys
import time

import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

SHELL, OUT, SHOT = sys.argv[1], sys.argv[2], sys.argv[3]
BRIDGE = os.path.join(SHELL, 'hearth-kwin-bridge')
LIST = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth', 'windows.json')
results = []


def log(text):
    print(text, flush=True)


def check(name, ok, detail=''):
    results.append((name, bool(ok)))
    clean = str(detail).replace('%', '%25').replace('\n', ' ')[:600]
    log('::%s title=KWin windows: %s::%s' % ('notice' if ok else 'error', name, ('passed. ' if ok else 'FAILED. ') + clean))


def shot(name):
    subprocess.run(['bash', '-c', SHOT, 'shot', 'kwin-' + name], timeout=40)


def ipc(*args):
    r = subprocess.run(['quickshell', 'ipc', '-p', SHELL, 'call'] + [str(a) for a in args], capture_output=True, text=True, timeout=20)
    lines = [l for l in r.stdout.strip().splitlines() if l.strip()]
    return lines[-1] if lines else ''


def state():
    try:
        return json.loads(ipc('winlist', 'state'))
    except ValueError:
        return {}


def listed():
    try:
        with open(LIST, encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


def win(title, snap=None):
    for w in (snap or listed()).get('windows', []):
        if w.get('title') == title:
            return w
    return None


def wait(pred, secs=8.0):
    end = time.time() + secs
    while time.time() < end:
        try:
            v = pred()
            if v:
                return v
        except Exception:
            pass
        time.sleep(0.25)
    return None


def bridge(method, args=None, reply=None):
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    return bus.call_sync('org.hearth.Windows', '/org/hearth/Windows', 'org.hearth.Windows', method, args,
                         GLib.VariantType.new(reply) if reply else None, Gio.DBusCallFlags.NONE, 5000, None)


def do(cmd):
    try:
        return bridge('Command', GLib.Variant('(s)', (json.dumps(cmd),)), '(b)').unpack()[0]
    except GLib.Error as e:
        log('::warning title=KWin windows::the bridge did not take %s: %s' % (cmd.get('op'), e.message))
        return False


def main():
    s = wait(lambda: state().get('connected') and state(), 40)
    check('Mantel started the bridge and KWin\'s script is talking', s, json.dumps(s or state())[:400])
    check('Mantel knows it is on KWin', (s or {}).get('wm') == 'kwin', (s or {}).get('wm'))
    d = wait(lambda: len(listed().get('desktops', [])) >= 4 and listed(), 10)
    check('four desktops (Hearth\'s KWin defaults)', d, json.dumps(listed().get('desktops'))[:300])

    procs = [subprocess.Popen(['foot', '--app-id=hearth.test.' + n, '--title=Test ' + n.upper(), 'sleep', '600'],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) for n in ('a', 'b')]
    both = wait(lambda: win('Test A') and win('Test B') and listed(), 20)
    titles = sorted(w.get('title') for w in listed().get('windows', []))
    check('both test windows are in the list', both, titles)
    check('only app windows: none of Mantel\'s bar, dock or flyouts', titles == ['Test A', 'Test B'], titles)
    a = win('Test A') or {}
    check('app id comes through', a.get('appId') == 'hearth.test.a', a.get('appId'))
    sh = wait(lambda: sorted(w['title'] for w in state().get('windows', [])) == ['Test A', 'Test B'], 8)
    check('Mantel\'s WinList has them (what the taskbar shows)', sh, json.dumps(state().get('windows'))[:400])

    # The taskbar's ways: Win's setters and functions.
    ipc('winlist', 'act', 'Test A', 'minimize')
    check('minimize from Mantel', wait(lambda: (win('Test A') or {}).get('minimized') is True), win('Test A'))
    ipc('winlist', 'act', 'Test A', 'activate')
    ok = wait(lambda: (win('Test A') or {}).get('minimized') is False and listed().get('active') == (win('Test A') or {}).get('id'))
    check('activate brings it back in front', ok, json.dumps(listed())[:400])
    check('Mantel sees which window is in front', wait(lambda: state().get('active') == 'Test A'), state().get('active'))
    ipc('winlist', 'act', 'Test A', 'maximize')
    check('maximize from Mantel', wait(lambda: (win('Test A') or {}).get('maximized') is True), win('Test A'))
    do({'op': 'probe', 'id': (win('Test A') or {}).get('id', '')})
    ipc('winlist', 'act', 'Test A', 'restore')
    check('restore size', wait(lambda: (win('Test A') or {}).get('maximized') is False), win('Test A'))

    # Snap layouts (WinList.snap), then Super+Left/Up/Down's steps (the script's own keys). First what this KWin offers
    # (into kwin-bridge.log, shown after the run).
    do({'op': 'probe', 'id': (win('Test A') or {}).get('id', '')})
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'left-half'}))
    check('snap to the left half', wait(lambda: (win('Test A') or {}).get('zone') == 'left-half'), win('Test A'))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test B', 'zone': 'right-half'}))
    check('snap to the right half', wait(lambda: (win('Test B') or {}).get('zone') == 'right-half'), win('Test B'))
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: listed().get('active') == (win('Test A') or {}).get('id'))
    do({'op': 'key', 'key': 'up'})
    check('Super+Up on a half: the top quarter', wait(lambda: (win('Test A') or {}).get('zone') == 'top-left'), win('Test A'))
    do({'op': 'key', 'key': 'down'})
    check('Super+Down on a top quarter: the half again', wait(lambda: (win('Test A') or {}).get('zone') == 'left-half'), win('Test A'))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'left-third'}))
    check('a snap layouts zone (left third)', wait(lambda: (win('Test A') or {}).get('zone') == 'left-third'), win('Test A'))

    # An X11 app (through KWin's Xwayland): KWin places it at once, without waiting for the app to draw.
    if os.environ.get('DISPLAY'):
        procs.append(subprocess.Popen(['python3', '-c', "import tkinter as tk; r = tk.Tk(className='hearthx'); r.title('Test X'); "
                                       "r.geometry('500x400+300+300'); r.mainloop()"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        x = wait(lambda: win('Test X'), 15)
        check('an X11 window is in the list', x, [w.get('title') for w in listed().get('windows', [])])
        if x:
            ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test X', 'zone': 'right-half'}))
            check('snap an X11 window to the right half', wait(lambda: (win('Test X') or {}).get('zone') == 'right-half'), win('Test X'))
            ipc('winlist', 'act', 'Test X', 'maximize')
            check('maximize an X11 window', wait(lambda: (win('Test X') or {}).get('maximized') is True), win('Test X'))
            do({'op': 'probe', 'id': (win('Test X') or {}).get('id', '')})
            ipc('winlist', 'act', 'Test X', 'close')

    # Snap layouts' flyout from the shortcut (the bridge runs its command), then a zone picked the way the flyout does.
    try:
        bridge('Shortcut', GLib.Variant('(s)', ('snap-layouts',)))
    except GLib.Error as e:
        log('::warning title=KWin windows::Shortcut: %s' % e.message)
    fly = wait(lambda: json.loads(ipc('snap', 'state') or '{}').get('shown'), 8)
    check('Super+Z (Shortcut) opens snap layouts', fly, ipc('snap', 'state'))
    ipc('snap', 'close')

    # Desktops: the taskbar's "Move to desktop" and the desktop buttons.
    ipc('winlist', 'run', json.dumps({'op': 'desktop', 'title': 'Test B', 'n': 2}))
    check('move a window to desktop 2', wait(lambda: (win('Test B') or {}).get('desktops') == [2]), win('Test B'))
    ipc('winlist', 'run', json.dumps({'op': 'goDesktop', 'n': 2}))
    cur = wait(lambda: [i for i, x in enumerate(listed().get('desktops', [])) if x.get('active')] == [1])
    check('switch to desktop 2', cur, listed().get('desktops'))
    check('Mantel\'s desktop buttons follow', wait(lambda: [x.get('active') for x in state().get('desktops', [])][:2] == [False, True]),
          state().get('desktops'))
    ipc('winlist', 'run', json.dumps({'op': 'goDesktop', 'n': 1}))
    wait(lambda: (listed().get('desktops') or [{}])[0].get('active'))

    # The bridge goes away: Mantel starts it again and the list comes back.
    subprocess.run(['pkill', '-f', 'hearth-kwin-bridge serve'])
    time.sleep(1)
    back = wait(lambda: state().get('connected') and win('Test A'), 25)
    check('the bridge is started again if it stops', back, json.dumps(state())[:300])

    # Closing (a third window, so the pictures below still have both).
    procs.append(subprocess.Popen(['foot', '--app-id=hearth.test.c', '--title=Test C', 'sleep', '600'],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
    wait(lambda: win('Test C'), 15)
    ipc('winlist', 'act', 'Test C', 'close')
    check('close from Mantel', wait(lambda: listed().get('windows') is not None and not win('Test C'), 10),
          [w.get('title') for w in listed().get('windows', [])])

    # The pictures, last (taking one goes through KWin and mustn't spoil the checks): the taskbar with the two windows
    # side by side, Snap layouts' flyout, KWin's window menu with Hearth's Snap.
    ipc('winlist', 'run', json.dumps({'op': 'desktop', 'title': 'Test B', 'n': 1}))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'left-half'}))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test B', 'zone': 'right-half'}))
    wait(lambda: (win('Test A') or {}).get('zone') == 'left-half' and (win('Test B') or {}).get('zone') == 'right-half')
    time.sleep(1.5)
    shot('halves')
    ipc('snap', 'open')
    time.sleep(1.5)
    shot('snap-layouts')
    ipc('snap', 'close')
    ipc('winlist', 'act', 'Test A', 'activate')
    time.sleep(0.5)
    do({'op': 'menu'})
    time.sleep(1.5)
    shot('window-menu')

    for p in procs:
        p.kill()
    failed = sum(1 for _, ok in results if not ok)
    log('%d checks, %d failed' % (len(results), failed))
    return failed


if __name__ == '__main__':
    sys.exit(main())
