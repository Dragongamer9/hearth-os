#!/usr/bin/env python3
"""Hearth GNOME end to end (gnome-test.yml, run by .github/scripts/gnome-check.sh inside each base's container, with
a headless GNOME Shell already running in Hearth's session mode plus the CI helper extension hearth-ci@hearthos.fun).

  python3 tools/e2e-hearth-gnome.py OUT_DIR

1. Hearth's extensions: all loaded and active, without errors (Main.extensionManager); the bar at the bottom, Super+G
   taken, the launcher's lists, hearth-pad running.
2. Screenshots (org.gnome.Shell.Screenshot): the desktop, Quick Settings, Hearth Link, the launcher, Hearth Storage,
   the overview, the app grid, the Hearth menu and the keyboard.
3. The controller under a virtual uinput pad (an Xbox 360 pad made here): hearth-pad --gnome, started by
   hearth-pad@hearthos.fun, reads it; its uinput keyboard and pointer are read here and replayed into the headless
   shell (hearth-ci). Checked: the right stick moves the pointer, RT clicks, Menu opens the launcher and the D-pad / B
   work in it, Guide opens the Hearth menu, View the overview, Guide + X the keyboard (A types, B hides it), View +
   Menu the bar; then GNOME Files (open a folder, back, open a file in Text Editor) and Console (type through the
   keyboard) with the pad.
Prints "ok: ..." / "FAIL: ..." lines and ::error:: / ::warning:: annotations; exits 1 when something failed.
"""
import ast
import json
import os
import re
import subprocess
import sys
import threading
import time

OUT = sys.argv[1] if len(sys.argv) > 1 else 'shots'
HOME = os.environ.get('HOME', '/tmp/home')
UUIDS = ['hearth-%s@hearthos.fun' % n for n in ('theme', 'bar', 'launcher', 'quick', 'link', 'storage', 'gamemode', 'pad')]
failed = []


def ok(msg):
    print('ok: ' + msg, flush=True)


def fail(msg, title='Hearth GNOME'):
    failed.append(msg)
    print('FAIL: ' + msg, flush=True)
    print('::error title=%s::%s' % (title, msg.replace('\n', ' ')[:900]), flush=True)


def warn(msg, title='Hearth GNOME'):
    print('::warning title=%s::%s' % (title, msg.replace('\n', ' ')[:900]), flush=True)


def gdbus(*args, timeout=20):
    try:
        r = subprocess.run(['gdbus', 'call', '--session'] + list(args), capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout.strip(), r.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, '', 'timeout'


def ev(js, timeout=20):
    """org.gnome.Shell.Eval: the result (JSON-decoded when it can be), or raises."""
    rc, out, err = gdbus('--dest', 'org.gnome.Shell', '--object-path', '/org/gnome/Shell', '--method',
                         'org.gnome.Shell.Eval', js, timeout=timeout)
    if rc != 0:
        raise RuntimeError(err or out or 'Eval failed')
    m = re.match(r'^\((true|false), (.*)\)$', out, re.S)
    if not m:
        raise RuntimeError('odd answer: ' + out[:200])
    val = ast.literal_eval(m.group(2))
    if m.group(1) != 'true':
        raise RuntimeError(val)
    for _ in range(2):
        try:
            val = json.loads(val)
        except (TypeError, ValueError):
            break
        if not isinstance(val, str):
            break
    return val


def ext(uuid, call):
    return ev("(() => { const e = Main.extensionManager.lookup('%s'); return e && e.stateObj ? e.stateObj.%s : null; })()" % (uuid, call))


def pad_state():
    try:
        return ext('hearth-pad@hearthos.fun', 'debugState()') or {}
    except Exception as e:  # noqa: BLE001
        return {'error': str(e)}


def wait(fn, secs=8, step=0.25):
    end = time.time() + secs
    while time.time() < end:
        try:
            v = fn()
            if v:
                return v
        except Exception:  # noqa: BLE001
            pass
        time.sleep(step)
    return None


def shot(name):
    path = os.path.join(OUT, name + '.png')
    time.sleep(0.8)
    rc, out, err = gdbus('--dest', 'org.gnome.Shell.Screenshot', '--object-path', '/org/gnome/Shell/Screenshot', '--method',
                         'org.gnome.Shell.Screenshot.Screenshot', 'false', 'false', path)
    if rc != 0 or not os.path.exists(path):
        warn('no screenshot %s: %s' % (name, (err or out)[:300]), 'screenshot')
    return path


def close_all():
    try:
        ev("Main.panel.statusArea.quickSettings.menu.close(); Main.panel.statusArea.dateMenu.menu.close(); "
           "Main.overview.hide(); (() => { const l = Main.extensionManager.lookup('hearth-launcher@hearthos.fun'); "
           "if (l && l.stateObj && l.stateObj.isOpen()) l.stateObj.toggle(); "
           "const p = Main.extensionManager.lookup('hearth-pad@hearthos.fun'); if (p && p.stateObj) { p.stateObj.keyboard.close(); p.stateObj.radial.hide(); } })(); true")
    except Exception:  # noqa: BLE001
        pass
    time.sleep(0.6)


# ---------------------------------------------------------------- 1. the extensions
def check_extensions():
    states = wait(lambda: ev("JSON.stringify(Main.extensionManager.getUuids().filter(u => u.endsWith('@hearthos.fun')).map(u => "
                             "{ const e = Main.extensionManager.lookup(u); return [u, e.state, String(e.error || '')]; }))"), 30)
    if not states:
        fail("couldn't ask GNOME Shell for its extensions")
        return
    got = {u: (s, e) for u, s, e in states}
    for u in UUIDS:
        if u not in got:
            fail('%s was not found by GNOME Shell' % u, 'extensions')
        elif got[u][0] != 1 or got[u][1]:
            fail('%s is not active (state %s): %s' % (u, got[u][0], got[u][1]), 'extensions')
        else:
            ok(u + ' active')
    try:
        bar = ext('hearth-bar@hearthos.fun', 'debugState()')
        if bar and abs(bar['y'] + bar['height'] - bar['monitorBottom']) <= 1:
            ok('the bar is at the bottom (%s)' % bar)
        else:
            fail('the bar is not at the bottom: %s' % bar, 'bar')
        gm = ext('hearth-gamemode@hearthos.fun', 'debugState()')
        if gm and gm.get('superG'):
            ok('Super+G is Game Mode')
        else:
            fail('Super+G was not taken: %s' % gm, 'Game Mode button')
        ln = wait(lambda: (lambda s: s if s and s.get('apps') else None)(ext('hearth-launcher@hearthos.fun', 'debugState()')), 20)
        if ln:
            ok('the launcher lists %s apps and %s games' % (ln['apps'], ln['games']))
        else:
            fail('the launcher has no apps (hearth-gnome apps)', 'launcher')
        th = ev("(() => { const s = new imports.gi.Gio.Settings({schema_id: 'org.gnome.desktop.background'}); return s.get_string('picture-uri'); })()")
        if 'hearth-fireside' in str(th):
            ok("Hearth's wallpaper is GNOME's background")
        else:
            warn('the background is %s, not Fireside' % th, 'look')
    except Exception as e:  # noqa: BLE001
        fail('checking the extensions: %s' % e)


def screenshots():
    close_all()
    shot('gnome-1-desktop')
    ev('Main.panel.statusArea.quickSettings.menu.open(); true')
    shot('gnome-2-quicksettings')
    close_all()
    ext('hearth-link@hearthos.fun', 'openMenu()')
    shot('gnome-3-link')
    close_all()
    ext('hearth-launcher@hearthos.fun', 'toggle()')
    shot('gnome-4-launcher')
    close_all()
    ext('hearth-storage@hearthos.fun', 'openMenu()')
    shot('gnome-5-storage')
    close_all()
    ev('Main.overview.show(); true')
    time.sleep(1)
    shot('gnome-6-overview')
    ev('Main.overview.showApps(); true')
    time.sleep(1)
    shot('gnome-9-appgrid')
    close_all()
    ev('Main.panel.statusArea.dateMenu.menu.open(); true')
    shot('gnome-10-calendar')
    close_all()


# ---------------------------------------------------------------- 3. the controller
class Pad:
    def __init__(self, evdev):
        e = evdev.ecodes
        self.e = e
        stick = evdev.AbsInfo(0, -32768, 32767, 16, 128, 0)
        trig = evdev.AbsInfo(0, 0, 255, 0, 0, 0)
        hat = evdev.AbsInfo(0, -1, 1, 0, 0, 0)
        caps = {e.EV_KEY: [e.BTN_SOUTH, e.BTN_EAST, e.BTN_NORTH, e.BTN_WEST, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START,
                           e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
                e.EV_ABS: [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick), (e.ABS_Z, trig),
                           (e.ABS_RZ, trig), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat)]}
        self.dev = evdev.UInput(caps, name='Microsoft X-Box 360 pad', vendor=0x045e, product=0x028e, version=0x110)
        self.names = {'a': e.BTN_SOUTH, 'b': e.BTN_EAST, 'x': e.BTN_NORTH, 'y': e.BTN_WEST, 'lb': e.BTN_TL, 'rb': e.BTN_TR,
                      'view': e.BTN_SELECT, 'menu': e.BTN_START, 'guide': e.BTN_MODE}

    def key(self, name, down):
        self.dev.write(self.e.EV_KEY, self.names[name], 1 if down else 0)
        self.dev.syn()

    def press(self, *names, hold=0.12):
        for n in names:
            self.key(n, True)
            time.sleep(0.05)
        time.sleep(hold)
        for n in reversed(names):
            self.key(n, False)
            time.sleep(0.03)
        time.sleep(0.35)

    def axis(self, code, value):
        self.dev.write(self.e.EV_ABS, code, value)
        self.dev.syn()

    def dpad(self, direction):
        code = self.e.ABS_HAT0X if direction in ('left', 'right') else self.e.ABS_HAT0Y
        self.axis(code, -1 if direction in ('left', 'up') else 1)
        time.sleep(0.12)
        self.axis(code, 0)
        time.sleep(0.35)


class Bridge:
    """Reads hearth-pad's uinput keyboard and pointer and replays them into the headless shell (hearth-ci)."""

    def __init__(self, evdev):
        self.evdev = evdev
        self.e = evdev.ecodes
        self.keys = []          # (code, value)
        self.buttons = []
        self.moved = 0
        self.forward = True
        self.devs = []
        self.stop = False

    def find(self, secs=15):
        end = time.time() + secs
        while time.time() < end:
            found = []
            for p in self.evdev.list_devices():
                try:
                    d = self.evdev.InputDevice(p)
                except OSError:
                    continue
                if d.name in ('Hearth controller pointer', 'Hearth controller keyboard'):
                    found.append(d)
                else:
                    d.close()
            if len(found) >= 2:
                self.devs = found
                return True
            for d in found:
                d.close()
            time.sleep(0.5)
        return False

    def run(self):
        import select
        e = self.e
        acc = [0, 0]
        last = time.time()
        while not self.stop:
            r, _, _ = select.select([d.fd for d in self.devs], [], [], 0.03)
            for d in self.devs:
                if d.fd not in r:
                    continue
                try:
                    events = list(d.read())
                except (BlockingIOError, OSError):
                    continue
                for x in events:
                    if x.type == e.EV_KEY and x.value in (0, 1):
                        if x.code >= 0x110 and x.code < 0x120:
                            self.buttons.append((x.code, x.value))
                            self.call('hearthCi.button(%d, %s)' % (x.code, 'true' if x.value else 'false'))
                        else:
                            self.keys.append((x.code, x.value))
                            self.call('hearthCi.key(%d, %s)' % (x.code, 'true' if x.value else 'false'))
                    elif x.type == e.EV_REL and x.code in (e.REL_X, e.REL_Y):
                        acc[0 if x.code == e.REL_X else 1] += x.value
                        self.moved += abs(x.value)
                    elif x.type == e.EV_REL and x.code == e.REL_WHEEL:
                        self.call('hearthCi.wheel(%d)' % x.value)
            if (acc[0] or acc[1]) and time.time() - last > 0.05:
                self.call('hearthCi.move(%d, %d)' % (acc[0], acc[1]))
                acc = [0, 0]
                last = time.time()

    def call(self, js):
        if self.forward:
            try:
                ev(js + '; true', timeout=5)
            except Exception as err:  # noqa: BLE001
                print('bridge: %s' % err, flush=True)

    def saw_key(self, name, since):
        code = getattr(self.e, name)
        return any(c == code and v == 1 for c, v in self.keys[since:])


def controller():
    try:
        import evdev
    except ImportError:
        fail('python3-evdev is missing in the test container', 'controller')
        return
    st = wait(lambda: (lambda s: s if s.get('ready') else None)(pad_state()), 30)
    if not st:
        fail('hearth-pad --gnome did not start from hearth-pad@hearthos.fun: %s' % pad_state(), 'controller')
        return
    ok('hearth-pad --gnome is running (%s)' % st.get('problem', ''))
    try:
        pad = Pad(evdev)
    except Exception as e:  # noqa: BLE001
        fail('could not make a virtual pad: %s' % e, 'controller')
        return
    br = Bridge(evdev)
    if not br.find():
        fail("hearth-pad's uinput keyboard and pointer did not appear", 'controller')
        return
    time.sleep(4)   # hearth-pad looks for controllers every 3 s
    e = evdev.ecodes
    # Does the headless shell read input devices itself? Then nothing is replayed (it would come twice).
    p0 = ev('JSON.stringify(global.get_pointer().slice(0, 2))')
    threading.Thread(target=br.run, daemon=True).start()
    br.forward = False
    pad.axis(e.ABS_RX, 30000)
    time.sleep(0.6)
    pad.axis(e.ABS_RX, 0)
    time.sleep(0.4)
    p1 = ev('JSON.stringify(global.get_pointer().slice(0, 2))')
    br.forward = p0 == p1
    print('pointer before/after (not replayed): %s %s; replaying: %s' % (p0, p1, br.forward), flush=True)
    if br.moved > 0:
        ok('the right stick moves the pointer (%d px)' % br.moved)
    else:
        fail('the right stick did not move the pointer', 'controller')
    n = len(br.buttons)
    pad.axis(e.ABS_RZ, 255)
    time.sleep(0.2)
    pad.axis(e.ABS_RZ, 0)
    time.sleep(0.4)
    if any(c == e.BTN_LEFT for c, v in br.buttons[n:]):
        ok('RT clicks')
    else:
        fail("RT didn't click", 'controller')

    close_all()
    # Menu: the Hearth launcher; GNOME Shell has the keyboard, so the D-pad walks it and B closes it.
    pad.press('menu')
    if wait(lambda: ext('hearth-launcher@hearthos.fun', 'isOpen()'), 4):
        ok('Menu opens the Hearth launcher')
        if wait(lambda: pad_state().get('sent', {}).get('keys'), 3):
            ok('hearth-pad knows GNOME Shell has the keyboard')
        else:
            fail('hearth-pad was not told GNOME Shell has the keyboard: %s' % pad_state(), 'controller')
        k = len(br.keys)
        pad.dpad('down')
        pad.dpad('right')
        shot('gnome-pad-launcher')
        if br.saw_key('KEY_DOWN', k):
            ok('the D-pad moves in the launcher (arrow keys)')
        else:
            fail("the D-pad didn't send arrow keys in the launcher", 'controller')
        pad.press('b')
        if br.forward and wait(lambda: not ext('hearth-launcher@hearthos.fun', 'isOpen()'), 4):
            ok('B closes the launcher')
        elif br.forward:
            fail("B didn't close the launcher", 'controller')
    else:
        fail("Menu didn't open the Hearth launcher: %s" % pad_state(), 'controller')
    close_all()

    # Guide: the Hearth menu; the D-pad picks; B closes.
    pad.press('guide')
    if wait(lambda: (pad_state().get('radial') or {}).get('open'), 4):
        ok('Guide opens the Hearth menu')
        pad.dpad('right')
        if wait(lambda: (pad_state().get('radial') or {}).get('sel') == 2, 3):
            ok('the D-pad picks in the Hearth menu')
        else:
            fail("the D-pad didn't pick in the Hearth menu: %s" % pad_state().get('radial'), 'controller')
        shot('gnome-7-radial')
        pad.press('b')
        if wait(lambda: not (pad_state().get('radial') or {}).get('open'), 3):
            ok('B closes the Hearth menu')
        else:
            fail("B didn't close the Hearth menu", 'controller')
    else:
        fail("Guide didn't open the Hearth menu: %s" % pad_state(), 'controller')
    close_all()

    # View: the Activities overview.
    pad.press('view', hold=0.08)
    if wait(lambda: ev('Main.overview.visible'), 4):
        ok('View opens the overview')
        k = len(br.keys)
        pad.dpad('down')
        if br.saw_key('KEY_DOWN', k):
            ok('the D-pad moves in the overview')
        else:
            fail("the D-pad didn't send arrow keys in the overview", 'controller')
    else:
        fail("View didn't open the overview (last action %s)" % pad_state().get('lastAction'), 'controller')
    close_all()

    # Guide + X: Hearth's keyboard; A types the picked key; Y a space; B hides it.
    pad.press('guide', 'x')
    if wait(lambda: (pad_state().get('osk') or {}).get('open'), 4):
        ok('Guide + X opens the keyboard')
        shot('gnome-8-keyboard')
        k = len(br.keys)
        pad.press('a')
        pad.press('y')
        typed = (pad_state().get('osk') or {}).get('typed', '')
        if typed and len(br.keys) > k:
            ok('the keyboard types through uinput (%r)' % typed)
        else:
            fail("the keyboard didn't type (typed %r, keys %d)" % (typed, len(br.keys) - k), 'keyboard')
        pad.press('b')
        if wait(lambda: not (pad_state().get('osk') or {}).get('open'), 3):
            ok('B hides the keyboard')
        else:
            fail("B didn't hide the keyboard", 'keyboard')
    else:
        fail("Guide + X didn't open the keyboard: %s" % pad_state(), 'keyboard')
    close_all()

    # View + Menu: the focus on the bar.
    pad.press('view', 'menu')
    if wait(lambda: ev('(() => { const f = global.stage.get_key_focus(); return !!f && Main.panel.contains(f); })()'), 3):
        ok('View + Menu puts the focus on the bar')
        shot('gnome-pad-bar')
        pad.press('b')
    else:
        fail("View + Menu didn't focus the bar (last action %s)" % pad_state().get('lastAction'), 'controller')
    close_all()

    if br.forward:
        apps(pad, br)
    else:
        warn('the headless shell reads input devices itself here: the apps part was skipped', 'apps')
    br.stop = True


def window_titles():
    return ev("JSON.stringify(global.get_window_actors().map(a => [a.meta_window.get_wm_class() || '', a.meta_window.get_title() || '']))") or []


def focus():
    return ev("(() => { const w = global.display.get_focus_window(); return JSON.stringify(w ? [w.get_wm_class() || '', w.get_title() || ''] : ['', '']); })()")


def apps(pad, br):
    # GNOME Files: open a folder (A), back (B), open a file (A: Text Editor).
    base = os.path.join(HOME, 'Hearth-test')
    os.makedirs(os.path.join(base, 'Alpha'), exist_ok=True)
    with open(os.path.join(base, 'Alpha', 'note.txt'), 'w') as f:
        f.write('Hello from Hearth GNOME\n')
    subprocess.Popen(['nautilus', '--new-window', base], stdout=subprocess.DEVNULL, stderr=open(os.path.join(OUT, 'nautilus.log'), 'w'))
    if not wait(lambda: 'autilus' in focus()[0], 25):
        warn('GNOME Files did not come up (windows: %s)' % window_titles(), 'apps')
    else:
        ok('GNOME Files is in front: %s' % focus())
        if wait(lambda: pad_state().get('sent', {}).get('app') == 'files', 3):
            ok("hearth-pad uses GNOME Files' keys")
        else:
            fail('hearth-pad was not told GNOME Files is in front: %s' % pad_state().get('sent'), 'apps')
        time.sleep(1.5)
        pad.dpad('down')
        pad.press('a')
        if wait(lambda: 'Alpha' in focus()[1], 6):
            ok('A opens a folder in GNOME Files')
            shot('gnome-files')
            pad.press('b')
            if wait(lambda: 'Hearth-test' in focus()[1], 6):
                ok('B goes back in GNOME Files')
            else:
                fail("B didn't go back in GNOME Files: %s" % focus(), 'apps')
            pad.press('a')
            wait(lambda: 'Alpha' in focus()[1], 6)
            time.sleep(1)
            pad.dpad('down')
            pad.press('a')
            if wait(lambda: any('extEditor' in c or 'note.txt' in t for c, t in window_titles() if 'autilus' not in c), 15):
                ok('A opens a file (Text Editor)')
                time.sleep(2)
                shot('gnome-texteditor')
            else:
                fail("A didn't open the file in Text Editor: %s" % window_titles(), 'apps')
        else:
            fail("A didn't open the folder in GNOME Files: %s" % focus(), 'apps')
            shot('gnome-files')
    close_windows()

    # Console: type through Hearth's keyboard.
    out = os.path.join(HOME, 'console-typed.txt')
    if os.path.exists(out):
        os.remove(out)
    subprocess.Popen(['kgx', '--', 'sh', '-c', 'cat > %s' % out], stdout=subprocess.DEVNULL, stderr=open(os.path.join(OUT, 'console.log'), 'w'))
    if not wait(lambda: re.search('console|kgx', focus()[0], re.I), 25):
        warn('Console did not come up (windows: %s)' % window_titles(), 'apps')
        return
    ok('Console is in front')
    time.sleep(2)
    pad.press('y')      # Console: the keyboard
    if not wait(lambda: (pad_state().get('osk') or {}).get('open'), 4):
        fail("Y didn't open the keyboard in Console", 'apps')
        return
    pad.press('a')
    pad.press('a')
    pad.press('menu')   # the keyboard's Enter
    pad.press('b')
    time.sleep(1.5)
    shot('gnome-console')
    text = open(out).read() if os.path.exists(out) else ''
    if text.strip():
        ok('typed into Console through the keyboard: %r' % text)
    else:
        fail("nothing typed into Console arrived (keyboard typed %r)" % (pad_state().get('osk') or {}).get('typed'), 'apps')
    close_windows()


def close_windows():
    try:
        ev("global.get_window_actors().forEach(a => a.meta_window.delete(global.get_current_time())); true")
    except Exception:  # noqa: BLE001
        pass
    time.sleep(1.5)


def main():
    os.makedirs(OUT, exist_ok=True)
    if not wait(lambda: ev('1') == 1, 90, 1):
        fail('GNOME Shell did not answer on D-Bus')
        return 1
    time.sleep(6)
    check_extensions()
    try:
        screenshots()
    except Exception as e:  # noqa: BLE001
        fail('screenshots: %s' % e)
    try:
        controller()
    except Exception as e:  # noqa: BLE001
        fail('controller: %s' % e, 'controller')
    with open(os.path.join(OUT, 'pad-state.json'), 'w') as f:
        json.dump(pad_state(), f, indent=1)
    print('%d failed' % len(failed), flush=True)
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
