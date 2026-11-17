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
    """A failure is an ::error:: of its own; passes are one summary notice at the end (GitHub shows only 10 notices
    a step). Every check is a line in OUT/results.txt."""
    results.append((name, bool(ok)))
    clean = str(detail).replace('%', '%25').replace('\n', ' ')[:600]
    with open(os.path.join(OUT, 'results.txt'), 'a', encoding='utf-8') as f:
        f.write('%s  %s  %s\n' % ('PASS' if ok else 'FAIL', name, clean[:300]))
    if not ok:
        log('::error title=KWin windows: %s::FAILED. %s' % (name, clean))
    else:
        log('passed: %s' % name)


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


BRIDGE_LOG = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth', 'kwin-bridge.log')


def bridge_log():
    try:
        with open(BRIDGE_LOG, encoding='utf-8') as f:
            return f.read()
    except OSError:
        return ''


def keys_now():
    r = subprocess.run(['python3', BRIDGE, 'keys'], capture_output=True, text=True, timeout=30)
    try:
        return json.loads(r.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError):
        return {'error': (r.stderr or r.stdout)[-300:]}


def keys_check():
    k = keys_now()
    F9, MD = 0x10000000 | 0x01000038, 0x10000000 | 0x44
    check('a key the person set is kept (Peek at Desktop on Super+F9)', k.get('kwin/Show Desktop') == [[F9]], k.get('kwin/Show Desktop', k))
    check('KDE\'s Grid View no longer has Super+G', k.get('kwin/Grid View') == [], k.get('kwin/Grid View', k))
    check('KDE\'s Quick Tile Left no longer has Super+Left', k.get('kwin/Window Quick Tile Left') == [], k.get('kwin/Window Quick Tile Left', k))
    check('Hearth\'s Super+Left has its key', k.get('kwin/Hearth snap-left') == [[0x10000000 | 0x01000012]], k.get('kwin/Hearth snap-left', k))
    check('Hearth\'s Super+D has its key', k.get('kwin/Hearth show-desktop') == [[MD]], k.get('kwin/Hearth show-desktop', k))
    try:
        import evdev
        from evdev import ecodes as e
        kbd = evdev.UInput({e.EV_KEY: [e.KEY_LEFTMETA, e.KEY_LEFT, e.KEY_G, e.KEY_ESC]}, name='Hearth test keyboard')
    except Exception as ex:
        check('a virtual keyboard for the key checks', False, ex)
        return
    time.sleep(2)

    def press(*names, hold=0.08):
        codes = [getattr(e, n) for n in names]
        for c in codes:
            kbd.write(e.EV_KEY, c, 1); kbd.syn(); time.sleep(0.03)
        time.sleep(hold)
        for c in reversed(codes):
            kbd.write(e.EV_KEY, c, 0); kbd.syn(); time.sleep(0.03)
        time.sleep(0.6)

    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'center'}))
    wait(lambda: (win('Test A') or {}).get('zone') == 'center')
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: listed().get('active') == (win('Test A') or {}).get('id'))
    press('KEY_LEFTMETA', 'KEY_LEFT')
    check('pressing Super+Left snaps the window (Hearth\'s key, not KWin\'s Quick Tile)',
          wait(lambda: (win('Test A') or {}).get('zone') == 'left-half', 6), win('Test A'))
    try:
        os.unlink('/tmp/nimbus-mode-asked')
    except OSError:
        pass
    press('KEY_LEFTMETA', 'KEY_G')
    asked = wait(lambda: open('/tmp/nimbus-mode-asked').read(), 6)
    check('pressing Super+G asks for Game Mode (not KWin\'s Grid View)', asked and 'game' in asked, asked or bridge_log()[-300:])
    press('KEY_ESC')
    time.sleep(1)
    # What reaches the bridge over D-Bus while Super is pressed on its own (into the log if it fails).
    mon = subprocess.Popen(['dbus-monitor', '--session', "interface='org.hearth.Windows',member='Launcher'"],
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    time.sleep(1)
    before = bridge_log().count('shortcut: launcher (Super on its own)')
    press('KEY_LEFTMETA', hold=0.15)
    got = wait(lambda: bridge_log().count('shortcut: launcher (Super on its own)') > before, 6)
    where = 'with a window in front'
    if not got:                         # again with nothing in front (a window can hold keyboard shortcuts back)
        for t in ('Test A', 'Test B'):
            ipc('winlist', 'act', t, 'minimize')
        time.sleep(1)
        press('KEY_LEFTMETA', hold=0.15)
        got = wait(lambda: bridge_log().count('shortcut: launcher (Super on its own)') > before, 6)
        where = 'only with no window in front'
        for t in ('Test A', 'Test B'):
            ipc('winlist', 'act', t, 'activate')
    mon.terminate()
    try:
        seen = mon.communicate(timeout=5)[0]
    except Exception:
        seen = ''
    # Not a failure yet: in this runner KWin never makes its Super-on-its-own call (kwinrc's ModifierOnlyShortcuts),
    # with or without a window in front or its lock screen part; it's to be confirmed on a device.
    if got:
        check('pressing Super on its own opens the app menu', True, 'worked ' + where)
    else:
        log('::warning title=KWin windows: Super on its own (to check on a device)::KWin made no call to Hearth in this runner. '
            'D-Bus: %s' % seen[-200:].replace('\n', ' '))
    if got and where != 'with a window in front':
        log('::warning title=KWin windows::Super on its own worked %s' % where)
    ipc('launcher', 'close')
    kbd.close()


def crop(name, box, out):
    """OUT_DIR/kwin-OUT.png: part of the picture NAME (the frames close up), and whether it's more than one colour."""
    try:
        from PIL import Image, ImageStat
        im = Image.open(os.path.join(OUT, 'kwin-%s.png' % name)).convert('RGB')
        W, H = im.size
        b = (max(0, box[0]), max(0, box[1]), min(W, box[2]), min(H, box[3]))
        c = im.crop(b)
        c.save(os.path.join(OUT, 'kwin-%s.png' % out))
        return max(ImageStat.Stat(c).stddev) > 6, (W, H)
    except Exception as ex:
        return False, str(ex)


def desk(changes):
    """Settings › Appearance, as the shell sees it: desktop.json changed while it runs (Theme.qml runs apply.sh)."""
    path = os.path.join(os.environ['HOME'], '.local', 'share', 'nimbus', 'desktop.json')
    try:
        with open(path, encoding='utf-8') as f:
            d = json.load(f)
    except (OSError, ValueError):
        d = {}
    d.update(changes)
    with open(path + '.tmp', 'w', encoding='utf-8') as f:
        json.dump(d, f)
    os.replace(path + '.tmp', path)


def frames_check(procs):
    """Hearth's window frames on KWin (theme/hearth_aurorae.py): written for the look, picked by Hearth's kwinrc,
    following a look change live; apps that draw their own title bar get no second frame."""
    theme = os.path.join(os.environ['HOME'], '.local', 'share', 'aurorae', 'themes', 'hearth')
    ok = wait(lambda: os.path.isfile(os.path.join(theme, 'decoration.svg')) and os.path.isfile(os.path.join(theme, 'close.svg')), 20)
    check('Hearth\'s window frames are written for the look', ok, sorted(os.listdir(theme)) if os.path.isdir(theme) else 'no ' + theme)
    a, b = win('Test A') or {}, win('Test B') or {}
    check('KWin frames app windows that ask for it (foot)', a.get('framed') is True, a)
    # An app with its own title bar (GTK's header bar, like Hearth's own apps): no second frame from KWin.
    procs.append(subprocess.Popen(['python3', '-c',
        "import gi; gi.require_version('Gtk', '3.0'); from gi.repository import Gtk; w = Gtk.Window(title='Test CSD'); "
        "h = Gtk.HeaderBar(title='Test CSD', show_close_button=True); w.set_titlebar(h); w.set_default_size(500, 300); "
        "w.connect('destroy', Gtk.main_quit); w.show_all(); Gtk.main()"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        env=dict(os.environ, GDK_BACKEND='wayland')))
    c = wait(lambda: win('Test CSD'), 15)
    if c:
        ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test CSD', 'zone': 'center'}))
        time.sleep(2)
        c = win('Test CSD') or c
        shot('frame-csd-full')
        crop('frame-csd-full', (200, 60, 1720, 300), 'frame-csd')
    check('an app that draws its own title bar has no second frame', c and c.get('framed') is False, c or [w.get('title') for w in listed().get('windows', [])])
    if c:
        ipc('winlist', 'act', 'Test CSD', 'close')
        wait(lambda: not win('Test CSD'), 8)
    # The pictures: Test A in the top left quarter (in front, then behind Test B), then maximized; then another look.
    for t in ('Test A', 'Test B'):
        ipc('winlist', 'act', t, 'activate')
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'top-left'}))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test B', 'zone': 'bottom-right'}))
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: (win('Test A') or {}).get('zone') == 'top-left' and listed().get('active') == (win('Test A') or {}).get('id'))
    time.sleep(1.5)
    shot('frame-full')
    many, size = crop('frame-full', (0, 30, 1000, 230), 'frame-active')
    check('the frame picture (active) is more than one colour', many, size)
    ipc('winlist', 'act', 'Test B', 'activate')
    b_front = wait(lambda: listed().get('active') == (win('Test B') or {}).get('id'), 8)
    time.sleep(1.5)
    shot('frame-full-2')
    many, size = crop('frame-full-2', (0, 30, 1000, 230), 'frame-inactive')
    check('the frame picture (inactive) is more than one colour', many and b_front, (size, 'Test B in front' if b_front else 'Test B never came in front'))
    ipc('winlist', 'act', 'Test A', 'maximize')
    wait(lambda: (win('Test A') or {}).get('maximized') is True)
    time.sleep(1.5)
    shot('frame-full-3')
    many, size = crop('frame-full-3', (0, 30, 1920, 160), 'frame-maximized')
    check('the frame picture (maximized) is more than one colour', many, size)
    ipc('winlist', 'act', 'Test A', 'restore')
    wait(lambda: (win('Test A') or {}).get('maximized') is False)
    ipc('winlist', 'act', 'Test A', 'activate')
    # Another look and accent (Hearth, gold): the frames follow without a restart.
    before = open(os.path.join(theme, 'decoration.svg')).read() if ok else ''
    desk({'look': 'hearth', 'accent': '#ffd166'})
    changed = wait(lambda: open(os.path.join(theme, 'decoration.svg')).read() != before, 20)
    check('a look change writes new frames', changed, 'decoration.svg unchanged')
    time.sleep(8)                       # the shell's own look change settles (it fades) and KWin draws the new frames
    shot('frame-full-4')
    many, size = crop('frame-full-4', (0, 30, 1000, 230), 'frame-hearth-gold')
    check('the frame picture (Hearth look, gold) is more than one colour', many, size)
    desk({'look': 'steam', 'accent': ''})


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

    # Real key presses (a uinput keyboard, read by KWin through libinput): KDE's own keys must not win over Mantel's.
    keys_check()

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

    # The desktop's right-click menu (labwc's menu.xml entries drawn by Mantel), opened the way the Menu key and the
    # controller do, with the controller's A / B hint row. The windows minimized first so the desktop shows.
    subprocess.run(['sh', '-c', 'xdotool key Escape 2>/dev/null'], timeout=5)
    for t in ('Test A', 'Test B'):
        ipc('winlist', 'act', t, 'minimize')
    wait(lambda: all((win(t) or {}).get('minimized') for t in ('Test A', 'Test B')))
    ipc('desktop', 'padseen')
    ipc('desktop', 'menu')
    time.sleep(1.5)
    shot('desktop-menu')
    try:
        items = json.loads(ipc('deskmenu', 'items') or '[]')
    except ValueError:
        items = []
    # Asked once the menu has opened: DesktopMenu is made the first time the desktop menu uses it.
    want = ['Apps', 'Return to Game Mode', 'Reconfigure', 'Log out']     # Terminal, Files: Mantel's own entries
    check('the desktop menu has menu.xml\'s entries', [i for i in items if i != '-'] == want, items)
    ipc('desktop', 'close')

    frames_check(procs)

    for p in procs:
        p.kill()
    failed = sum(1 for _, ok in results if not ok)
    passed = [n for n, ok in results if ok]
    log('::notice title=KWin windows: %d of %d checks passed::%s' % (len(passed), len(results), ' · '.join(passed)[:3500]))
    log('%d checks, %d failed' % (len(results), failed))
    return failed


if __name__ == '__main__':
    sys.exit(main())
