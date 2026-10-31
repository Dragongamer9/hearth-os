#!/usr/bin/env python3
"""Hearth Plasma with a controller, end to end (plasma-test.yml): a virtual Xbox 360 pad made with /dev/uinput drives the
real hearth-pad --plasma in a running Plasma session (KWin + plasmashell with Hearth's defaults). This script stands in
for desktop/shell/osk-plasma.qml (the host): it reads hearth-pad's events and does what the host does with them (Menu:
Plasma's launcher over D-Bus; the Hearth menu's picks), and asks KWin what happened through a one-line KWin script that
calls hearth-pad's own org.hearth.Pad Probe (the cursor position and the window in front).

  e2e-plasma-pad.py HEARTH_PAD OUT_DIR SHOT_COMMAND     (SHOT_COMMAND gets the picture's name as $1)

Checks: the right stick moves the pointer; View shows the controls; Guide + X asks for the keyboard; Menu opens
Plasma's launcher; the D-pad moves in it and A starts the app picked; the Hearth menu (Guide, D-pad, A) opens Hearth
Quick Access and B closes it; View + B closes a window; then each KDE-based Hearth app that's installed (Hearth Files
from Dolphin, Konsole, Kate, Okular, KCalc, Spectacle) comes to the front with the controller as keys (hearth-pad's
per-app mapping), Y does its thing (search / the keyboard), and View + B closes it. Each is a ::notice:: (passed) or ::error:: (failed) line;
the exit code is the number that failed. Pictures: OUT_DIR/pad-*.png.
"""
import json
import os
import shutil
import subprocess
import sys
import threading
import time

import evdev
from evdev import ecodes as e
import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

PAD, OUT, SHOT = sys.argv[1], sys.argv[2], sys.argv[3]
LAUNCHED = '/tmp/hearth-pad-launched'
results = []
events = []
lock = threading.Lock()


def log(text):
    print(text, flush=True)


def shot(name):
    subprocess.run(['bash', '-c', SHOT, 'shot', name], timeout=40)


def check(name, ok, detail=''):
    results.append((name, bool(ok), detail))
    clean = str(detail).replace('%', '%25').replace('\n', ' ')[:500]
    log('::%s title=Controller: %s::%s' % ('notice' if ok else 'error', name, ('passed. ' if ok else 'FAILED. ') + clean))


# ---------------------------------------------------------------- the virtual controller
ui = evdev.UInput({
    e.EV_KEY: [e.BTN_SOUTH, e.BTN_EAST, e.BTN_NORTH, e.BTN_WEST, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START,
               e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
    e.EV_ABS: [(e.ABS_X, evdev.AbsInfo(0, -32768, 32767, 16, 128, 0)), (e.ABS_Y, evdev.AbsInfo(0, -32768, 32767, 16, 128, 0)),
               (e.ABS_RX, evdev.AbsInfo(0, -32768, 32767, 16, 128, 0)), (e.ABS_RY, evdev.AbsInfo(0, -32768, 32767, 16, 128, 0)),
               (e.ABS_Z, evdev.AbsInfo(0, 0, 255, 0, 0, 0)), (e.ABS_RZ, evdev.AbsInfo(0, 0, 255, 0, 0, 0)),
               (e.ABS_HAT0X, evdev.AbsInfo(0, -1, 1, 0, 0, 0)), (e.ABS_HAT0Y, evdev.AbsInfo(0, -1, 1, 0, 0, 0))],
}, name='Microsoft X-Box 360 pad', vendor=0x045e, product=0x028e, version=0x110)
BTN = {'a': e.BTN_SOUTH, 'b': e.BTN_EAST, 'x': e.BTN_NORTH, 'y': e.BTN_WEST, 'lb': e.BTN_TL, 'rb': e.BTN_TR,
       'view': e.BTN_SELECT, 'menu': e.BTN_START, 'guide': e.BTN_MODE}
HAT = {'up': (e.ABS_HAT0Y, -1), 'down': (e.ABS_HAT0Y, 1), 'left': (e.ABS_HAT0X, -1), 'right': (e.ABS_HAT0X, 1)}


def hold(name, down):
    if name in HAT:
        code, v = HAT[name]
        ui.write(e.EV_ABS, code, v if down else 0)
    else:
        ui.write(e.EV_KEY, BTN[name], 1 if down else 0)
    ui.syn()


def press(name, wait=0.35):
    hold(name, True)
    time.sleep(0.08)
    hold(name, False)
    time.sleep(wait)


def chord(first, second, wait=0.6):
    hold(first, True)
    time.sleep(0.12)
    press(second, 0.1)
    hold(first, False)
    time.sleep(wait)


def stick(rx, ry, secs):
    ui.write(e.EV_ABS, e.ABS_RX, int(rx * 32767))
    ui.write(e.EV_ABS, e.ABS_RY, int(ry * 32767))
    ui.syn()
    time.sleep(secs)
    ui.write(e.EV_ABS, e.ABS_RX, 0)
    ui.write(e.EV_ABS, e.ABS_RY, 0)
    ui.syn()
    time.sleep(0.3)


# ---------------------------------------------------------------- hearth-pad, with this script as its host
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)


def dbus(dest, path, iface, method, args=None, reply=None, timeout=5000):
    return bus.call_sync(dest, path, iface, method, args, GLib.VariantType(reply) if reply else None,
                         Gio.DBusCallFlags.NONE, timeout, None)


def open_launcher():
    try:
        dbus('org.kde.plasmashell', '/PlasmaShell', 'org.kde.PlasmaShell', 'activateLauncherMenu')
    except GLib.Error as err:
        log('activateLauncherMenu: %s' % err.message)


RADIAL = ['apps', 'keyboard', 'quick', 'close', 'game', 'screenshot', 'mouse', 'help']
proc = subprocess.Popen([sys.executable, PAD, '--plasma'], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        stderr=open(os.path.join(OUT, 'hearth-pad.stderr'), 'w'), text=True, bufsize=1)


def send(obj):
    proc.stdin.write(json.dumps(obj) + '\n')
    proc.stdin.flush()


def reader():
    for line in proc.stdout:
        try:
            m = json.loads(line)
        except ValueError:
            continue
        with lock:
            events.append(m)
        # What osk-plasma.qml does with them.
        if m.get('t') == 'launcher':
            open_launcher()
        elif m.get('t') == 'radial' and not m.get('open') and 0 <= m.get('pick', -1) < len(RADIAL):
            act = RADIAL[m['pick']]
            if act == 'apps':
                open_launcher()
            elif act in ('quick', 'close'):
                send({'cmd': 'plasma', 'do': act})


threading.Thread(target=reader, daemon=True).start()


def wait_event(pred, secs=5.0, start=0):
    end = time.time() + secs
    while time.time() < end:
        with lock:
            for m in events[start:]:
                if pred(m):
                    return m
        time.sleep(0.05)
    return None


def mark():
    with lock:
        return len(events)


PROBE_JS = '/tmp/hearth-probe.js'
with open(PROBE_JS, 'w') as f:
    f.write('(function(){var w=workspace.activeWindow;var c=workspace.cursorPos;'
            'callDBus("org.hearth.Pad","/org/hearth/Pad","org.hearth.Pad","Probe",JSON.stringify({x:c.x,y:c.y,'
            'cls:w?String(w.resourceClass):"",caption:w?String(w.caption):"",'
            'kind:!w?"none":w.desktopWindow?"desktop":w.dock?"dock":w.appletPopup?"popup":w.normalWindow?"normal":"other"}));})();\n')
probes = [0]


def probe():
    probes[0] += 1
    name = 'hearth-probe-%d' % probes[0]
    start = mark()
    try:
        sid = dbus('org.kde.KWin', '/Scripting', 'org.kde.kwin.Scripting', 'loadScript',
                   GLib.Variant('(ss)', (PROBE_JS, name)), '(i)').unpack()[0]
        dbus('org.kde.KWin', '/Scripting/Script%d' % sid, 'org.kde.kwin.Script', 'run')
    except GLib.Error as err:
        log('probe: %s' % err.message)
        return None
    m = wait_event(lambda m: m.get('t') == 'probe', 4, start)
    try:
        dbus('org.kde.KWin', '/Scripting', 'org.kde.kwin.Scripting', 'unloadScript', GLib.Variant('(s)', (name,)))
    except GLib.Error:
        pass
    try:
        return json.loads(m['text']) if m else None
    except ValueError:
        return None


def main():
    ready = wait_event(lambda m: m.get('t') in ('ready', 'error'), 10)
    if not ready or ready.get('t') != 'ready':
        check('hearth-pad starts', False, 'no ready line: %r' % ready)
        return
    time.sleep(4)          # it looks for controllers every 3 s
    p0 = probe()
    check('KWin answers through hearth-pad (org.hearth.Pad)', p0 is not None, 'first probe: %r' % p0)

    # The pointer: right stick down and right (labwc's cursor starts at the top left, over KWin's title bar).
    stick(0.9, 0.9, 0.7)
    p1 = probe()
    moved = p0 and p1 and (abs(p1['x'] - p0['x']) + abs(p1['y'] - p0['y'])) > 40
    shot('pad-1-pointer')
    check('right stick moves the pointer', moved, 'cursor %r -> %r' % (p0 and (p0['x'], p0['y']), p1 and (p1['x'], p1['y'])))

    s = mark()
    press('view', 0.5)
    check('View shows the controls', wait_event(lambda m: m.get('t') == 'help', 2, s), '')

    s = mark()
    chord('guide', 'x')
    osk = wait_event(lambda m: m.get('t') == 'osk' and m.get('open'), 2, s)
    radial_closed = wait_event(lambda m: m.get('t') == 'radial' and m.get('open') is False, 2, s)
    check('Guide + X asks for the on-screen keyboard', osk and radial_closed, 'events: %r' % events[s:][:6])

    # Menu: Plasma's launcher (Hearth's), then the D-pad and A start the first app ("Aa Pad Test").
    try:
        os.unlink(LAUNCHED)
    except OSError:
        pass
    s = mark()
    press('menu', 2.5)
    pl = probe()
    shot('pad-2-launcher')
    ok = wait_event(lambda m: m.get('t') == 'launcher', 1, s) and pl and pl.get('cls', '').lower() in ('plasmashell', 'org.kde.plasmashell') and pl.get('kind') != 'desktop'
    check('Menu opens Plasma\'s launcher', ok, 'in front: %r' % pl)
    press('down', 0.6)
    shot('pad-3-launcher-dpad')
    press('a', 0.2)
    for _ in range(40):
        if os.path.exists(LAUNCHED):
            break
        time.sleep(0.25)
    check('D-pad and A start an app from the launcher', os.path.exists(LAUNCHED),
          'the test app %s; pad.log says what was in front' % ('started' if os.path.exists(LAUNCHED) else 'did not start'))
    time.sleep(1.5)

    # The Hearth menu: Guide, right (Quick Access), A → Hearth Quick Access (Meta+Alt+A).
    s = mark()
    press('guide', 0.5)
    press('right', 0.3)
    shot('pad-4-hearth-menu')
    press('a', 2.5)
    pq = probe()
    shot('pad-5-quick-access')
    picked = wait_event(lambda m: m.get('t') == 'radial' and m.get('pick') == 2, 1, s)
    check('the Hearth menu opens Hearth Quick Access', picked and pq and pq.get('cls', '').lower() in ('plasmashell', 'org.kde.plasmashell') and pq.get('kind') != 'desktop',
          'picked: %r, in front: %r' % (bool(picked), pq))
    press('down', 0.4)
    shot('pad-6-quick-access-dpad')
    press('b', 1.0)
    pb = probe()
    check('B closes it', pb and pb.get('kind') != 'popup', 'in front: %r' % pb)

    # View + B closes the window in front (Alt+F4 in KWin).
    win = subprocess.Popen(['plasmawindowed', 'org.hearth.storage'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(7)
    pw = probe()
    shot('pad-7-window')
    chord('view', 'b', 0.5)
    try:
        win.wait(8)
        closed = True
    except subprocess.TimeoutExpired:
        closed = False
        win.kill()
    check('View + B closes the window', closed, 'in front before: %r' % pw)

    # Hearth Plasma's own apps, like Mantel's: the controller as keys in each, Y, then View + B closes it.
    apps = [('Hearth Files (Dolphin)', 'dolphin', ['dolphin', os.environ.get('HOME', '/tmp')]), ('Terminal (Konsole)', 'konsole', ['konsole']),
            ('Text (Kate)', 'kate', ['kate', '-n']), ('Reader (Okular)', 'okular', ['okular']),
            ('Calculator (KCalc)', 'kcalc', ['kcalc']), ('Capture (Spectacle)', 'spectacle', ['spectacle', '--launchonly'])]
    pad_log = os.path.join(os.environ.get('HOME', '/tmp'), '.local', 'share', 'nimbus', 'pad.log')
    for label, name, argv in apps:
        if not shutil.which(argv[0]):
            log('::notice title=Controller: %s::not installed here, skipped' % label)
            continue
        press('down', 0.2)                 # the controller was just used (Konsole opens the keyboard then)
        s = mark()
        app = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(8)
        pa = probe()
        front = bool(pa) and name in pa.get('cls', '').lower()
        try:
            with open(pad_log, errors='replace') as f:
                tail = f.read()[-4000:]
        except OSError:
            tail = ''
        keys = any(name in l.lower() and l.rstrip().endswith('keys') for l in tail.splitlines() if 'in front:' in l)
        shot('app-%s-1' % name)
        press('down', 0.3)
        press('down', 0.3)
        press('y', 1.2)
        shot('app-%s-2' % name)
        osk_ok = True
        if name in ('konsole', 'kcalc'):
            osk_ok = bool(wait_event(lambda m: m.get('t') == 'osk' and m.get('open'), 1, s))
        press('b', 0.5)
        chord('view', 'b', 0.5)
        try:
            app.wait(8)
            closed = True
        except subprocess.TimeoutExpired:
            chord('view', 'b', 0.5)
            try:
                app.wait(4)
                closed = True
            except subprocess.TimeoutExpired:
                closed = False
                app.kill()
        check('%s with the controller' % label, front and keys and osk_ok and closed,
              'in front: %r, keys: %s, keyboard: %s, closed with View + B: %s' % (pa and pa.get('cls'), keys, osk_ok, closed))
        time.sleep(1)


try:
    main()
finally:
    proc.terminate()
    ui.close()
    with open(os.path.join(OUT, 'pad-events.json'), 'w') as f:
        json.dump(events, f, indent=1)
    failed = [r for r in results if not r[1]]
    log('controller checks: %d passed, %d failed' % (len(results) - len(failed), len(failed)))
    sys.exit(min(len(failed), 100) if results else 1)
