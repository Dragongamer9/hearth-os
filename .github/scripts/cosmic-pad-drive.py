#!/usr/bin/env python3
"""Hearth COSMIC's controller check (cosmic-test.yml): a virtual Xbox 360 pad (uinput) drives Hearth COSMIC the way a
real one would. hearth-pad --cosmic (started by the companion, desk-cosmic.qml) reads the pad and types / points through
its own uinput keyboard and mouse. In CI nothing reads those (COSMIC runs nested in a headless labwc), so this script
reads them back, checks each press gave the keys it should, and passes the keys on to the outer labwc with wtype, so
COSMIC's apps really get them and the screenshots show what happened.

  cosmic-pad-drive.py SCENARIO OUTDIR      SCENARIO: files | term | edit | desk
Env: SHOT (a command taking a file name: grim on the outer display), APPID (a command printing the app id the
companion sees), OUTER_WAYLAND (the outer labwc's socket, for wtype).
Writes OUTDIR/pad-<scenario>.txt (one line per step: ok / FAIL, what was expected, what came) and exits 1 on a FAIL."""
import os
import subprocess
import sys
import threading
import time

import evdev
from evdev import ecodes as e

MODS = {'KEY_LEFTCTRL': 'ctrl', 'KEY_LEFTALT': 'alt', 'KEY_LEFTSHIFT': 'shift', 'KEY_LEFTMETA': 'logo'}
WTYPE_KEYS = {'KEY_ENTER': 'Return', 'KEY_ESC': 'Escape', 'KEY_TAB': 'Tab', 'KEY_LEFT': 'Left', 'KEY_RIGHT': 'Right',
              'KEY_UP': 'Up', 'KEY_DOWN': 'Down', 'KEY_BACKSPACE': 'BackSpace', 'KEY_SPACE': 'space',
              'KEY_PAGEUP': 'Prior', 'KEY_PAGEDOWN': 'Next', 'KEY_HOME': 'Home', 'KEY_END': 'End', 'KEY_F10': 'F10'}
BUTTONS = {'a': e.BTN_SOUTH, 'b': e.BTN_EAST, 'x': e.BTN_NORTH, 'y': e.BTN_WEST, 'lb': e.BTN_TL, 'rb': e.BTN_TR,
           'view': e.BTN_SELECT, 'menu': e.BTN_START, 'guide': e.BTN_MODE, 'l3': e.BTN_THUMBL, 'r3': e.BTN_THUMBR}


def wtype_name(key):
    if key in WTYPE_KEYS:
        return WTYPE_KEYS[key]
    k = key[4:]
    return k.lower() if len(k) == 1 else None


class Reader:
    """Reads hearth-pad's virtual keyboard and pointer, records combos, forwards keys with wtype."""

    def __init__(self):
        self.combos = []
        self.rel = {'move': 0, 'wheel': 0}
        self.buttons = []
        self.lock = threading.Lock()
        self.devs = []
        for p in evdev.list_devices():
            d = evdev.InputDevice(p)
            if d.name in ('Hearth controller keyboard', 'Hearth controller pointer'):
                self.devs.append(d)
        for d in self.devs:
            threading.Thread(target=self.loop, args=(d,), daemon=True).start()

    def loop(self, d):
        held = []
        for ev in d.read_loop():
            if ev.type == e.EV_REL:
                with self.lock:
                    if ev.code in (e.REL_X, e.REL_Y):
                        self.rel['move'] += abs(ev.value)
                    elif ev.code in (e.REL_WHEEL,):
                        self.rel['wheel'] += ev.value
            elif ev.type == e.EV_KEY and ev.value in (0, 1):
                name = e.KEY.get(ev.code) if ev.code in e.KEY else e.BTN.get(ev.code)
                if isinstance(name, (list, tuple)):
                    name = name[0]
                name = str(name)
                if name.startswith('BTN_'):
                    if ev.value:
                        with self.lock:
                            self.buttons.append(name)
                    continue
                if ev.value == 1:
                    held.append(name)
                    if name not in MODS:
                        combo = '+'.join(held)
                        with self.lock:
                            self.combos.append(combo)
                        self.forward([MODS[m] for m in held if m in MODS], name)
                else:
                    held = [k for k in held if k != name]

    def forward(self, mods, key):
        w = wtype_name(key)
        if not w:
            return
        argv = ['wtype']
        for m in mods:
            argv += ['-M', m]
        argv += ['-k', w]
        for m in reversed(mods):
            argv += ['-m', m]
        env = dict(os.environ, WAYLAND_DISPLAY=os.environ.get('OUTER_WAYLAND', ''))
        subprocess.run(argv, env=env, capture_output=True, timeout=10)

    def take(self):
        with self.lock:
            c, r, b = self.combos, dict(self.rel), self.buttons
            self.combos, self.rel, self.buttons = [], {'move': 0, 'wheel': 0}, []
        return c, r, b


def make_pad():
    absinfo = lambda lo, hi: evdev.AbsInfo(0, lo, hi, 16, 128, 0)
    caps = {e.EV_KEY: list(BUTTONS.values()),
            e.EV_ABS: [(e.ABS_X, absinfo(-32768, 32767)), (e.ABS_Y, absinfo(-32768, 32767)),
                       (e.ABS_RX, absinfo(-32768, 32767)), (e.ABS_RY, absinfo(-32768, 32767)),
                       (e.ABS_Z, evdev.AbsInfo(0, 0, 255, 0, 0, 0)), (e.ABS_RZ, evdev.AbsInfo(0, 0, 255, 0, 0, 0)),
                       (e.ABS_HAT0X, evdev.AbsInfo(0, -1, 1, 0, 0, 0)), (e.ABS_HAT0Y, evdev.AbsInfo(0, -1, 1, 0, 0, 0))]}
    return evdev.UInput(caps, name='Microsoft X-Box 360 pad', vendor=0x045e, product=0x028e, version=0x110)


class Pad:
    def __init__(self, ui):
        self.ui = ui

    def btn(self, name, down):
        if name in ('up', 'down', 'left', 'right'):
            code = e.ABS_HAT0Y if name in ('up', 'down') else e.ABS_HAT0X
            val = (-1 if name in ('up', 'left') else 1) if down else 0
            self.ui.write(e.EV_ABS, code, val)
        elif name in ('lt', 'rt'):
            self.ui.write(e.EV_ABS, e.ABS_Z if name == 'lt' else e.ABS_RZ, 255 if down else 0)
        else:
            self.ui.write(e.EV_KEY, BUTTONS[name], 1 if down else 0)
        self.ui.syn()

    def press(self, *names, hold=0.08):
        for n in names:
            self.btn(n, True)
            time.sleep(0.05)
        time.sleep(hold)
        for n in reversed(names):
            self.btn(n, False)
            time.sleep(0.05)

    def stick(self, code, value, secs):
        self.ui.write(e.EV_ABS, code, value)
        self.ui.syn()
        time.sleep(secs)
        self.ui.write(e.EV_ABS, code, 0)
        self.ui.syn()


def run(cmd, *args):
    if not cmd:
        return ''
    r = subprocess.run(['sh', '-c', cmd + ' "$@"', 'x'] + list(args), capture_output=True, text=True, timeout=30)
    return r.stdout.strip()


def main():
    scenario, outdir = sys.argv[1], sys.argv[2]
    ui = make_pad()
    pad = Pad(ui)
    time.sleep(4.5)                      # hearth-pad looks for controllers every 3 s
    reader = Reader()
    if not reader.devs:
        print('FAIL: hearth-pad\'s virtual keyboard and pointer aren\'t there (is hearth-pad --cosmic running?)')
        with open(os.path.join(outdir, 'pad-%s.txt' % scenario), 'w') as f:
            f.write('FAIL no Hearth controller keyboard / pointer\n')
        return 1
    app = run(os.environ.get('APPID', '')).strip().strip('"')
    lines = ['app the companion sees: %r' % app]
    fails = 0
    known = app.removesuffix('.desktop') in ('com.system76.CosmicFiles', 'com.system76.CosmicTerm', 'com.system76.CosmicEdit')
    tabs = known
    files = app.removesuffix('.desktop') == 'com.system76.CosmicFiles'
    reader.take()

    def step(label, action, expect=None, shot=None, check=None, wait=1.2):
        nonlocal fails
        action()
        time.sleep(wait)
        combos, rel, buttons = reader.take()
        ok = True
        if expect is not None:
            ok = expect in combos
        if check is not None:
            ok = ok and check(combos, rel, buttons)
        if shot:
            run(os.environ.get('SHOT', ''), os.path.join(outdir, shot + '.png'))
        lines.append('%s %s: expected %s, got keys %s, pointer %s, buttons %s' % ('ok' if ok else 'FAIL', label, expect, combos, rel, buttons))
        fails += 0 if ok else 1

    if scenario == 'files':
        step('D-pad down moves the selection', lambda: pad.press('down'), 'KEY_DOWN')
        step('D-pad down again', lambda: pad.press('down'), 'KEY_DOWN', shot='cosmic-files-2-pad-select')
        step('RB: next tab' if tabs else 'RB: next window', lambda: pad.press('rb'),
             'KEY_LEFTCTRL+KEY_TAB' if tabs else 'KEY_LEFTALT+KEY_TAB')
        step('Y: search and the keyboard', lambda: pad.press('y'), 'KEY_LEFTCTRL+KEY_F' if files else None,
             shot='cosmic-files-3-pad-search-keyboard', wait=2.0)
        step('Guide + X closes the keyboard', lambda: pad.press('guide', 'x'), None, wait=1.5)
        step('B: back', lambda: pad.press('b'), 'KEY_LEFTALT+KEY_LEFT' if files else 'KEY_ESC')
        step('A: open', lambda: pad.press('a'), 'KEY_ENTER', shot='cosmic-files-4-pad-open')
        step('RT scrolls', lambda: pad.press('rt', hold=0.8), None, check=lambda c, r, b: r['wheel'] < 0)
        step('Right stick moves the pointer', lambda: pad.stick(e.ABS_RX, 30000, 0.6), None, check=lambda c, r, b: r['move'] > 20)
        step('A right after moving clicks', lambda: (pad.stick(e.ABS_RY, 20000, 0.3), pad.press('a')), None,
             check=lambda c, r, b: 'BTN_LEFT' in b)
        step('X: right-click menu', lambda: pad.press('x'), None, shot='cosmic-files-5-pad-menu', check=lambda c, r, b: 'BTN_RIGHT' in b)
        step('B closes it', lambda: pad.press('b'), None)
    elif scenario == 'term':
        step('Y: the on-screen keyboard', lambda: pad.press('y'), None, shot='cosmic-term-2-pad-keyboard', wait=2.0)
        step('A on the keyboard types a key', lambda: pad.press('a'), None, check=lambda c, r, b: len(c) >= 1, wait=1.5)
        step('Guide + X closes it', lambda: pad.press('guide', 'x'), None, wait=1.5)
        step('A: Enter', lambda: pad.press('a'), 'KEY_ENTER', shot='cosmic-term-3-pad-typed')
        step('LB: tab' if tabs else 'LB: window', lambda: pad.press('lb'),
             'KEY_LEFTCTRL+KEY_LEFTSHIFT+KEY_TAB' if tabs else 'KEY_LEFTALT+KEY_LEFTSHIFT+KEY_TAB')
    elif scenario == 'edit':
        step('Y: find and the keyboard', lambda: pad.press('y'), 'KEY_LEFTCTRL+KEY_F' if known else None,
             shot='cosmic-edit-2-pad-find', wait=2.0)
        step('Guide + X closes the keyboard', lambda: pad.press('guide', 'x'), None, wait=1.5)
        step('B: Esc', lambda: pad.press('b'), 'KEY_ESC')
        step('D-pad right moves the caret', lambda: pad.press('right'), 'KEY_RIGHT')
    elif scenario == 'desk':
        step('Guide: the Hearth menu', lambda: pad.press('guide'), None, shot='cosmic-pad-1-hearth-menu', wait=1.5)
        step('B closes it', lambda: pad.press('b'), None)
        step('Menu: Hearth launcher', lambda: pad.press('menu'), None, shot='cosmic-pad-2-launcher', wait=2.0)
        step('D-pad in the launcher', lambda: pad.press('right'), 'KEY_RIGHT')
        step('B (Esc) closes it', lambda: pad.press('b'), 'KEY_ESC')
        step('View: the controls', lambda: pad.press('view', hold=0.1), None, shot='cosmic-pad-3-controls', wait=1.5)
        # The companion may still be coming back from the last menu closing (docs/COSMIC.md): give it a moment, so the
        # controller is on the plain desktop again (not in a menu's keys).
        time.sleep(3)
        step('View + B closes the window (Super+Q)', lambda: pad.press('view', 'b'), 'KEY_LEFTMETA+KEY_Q')
    with open(os.path.join(outdir, 'pad-%s.txt' % scenario), 'w') as f:
        f.write('\n'.join(lines) + '\n')
    print('\n'.join(lines))
    ui.close()
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
