#!/usr/bin/env python3
"""End-to-end test for Hearth Input (hosts/linux/desktop/shell/hearth-input), with real devices.

Starts the real daemon (`hearth-input serve`), makes fake controllers with /dev/uinput (an Xbox 360 pad as xpad
shows it, an Xbox controller over Bluetooth as hid-generic shows it, a DualSense as hid-playstation shows it),
talks to the control socket the way Hearth does (input_map.rs: a "state" line every 0.5 s) and reads what comes
out of the daemon's virtual keyboard and mouse. Nothing is faked: the grab, the key presses and the mouse
movement all go through the kernel.

  sudo python3 tools/e2e-hearth-input.py          (needs python3-evdev, /dev/uinput and the /dev/input nodes)

Prints PASS/FAIL per check and the daemon's log when something failed. Exit 1 on any failure, 2 when it can't
run at all (no evdev, no uinput).
"""
import errno
import json
import os
import select
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time

try:
    import evdev
    from evdev import ecodes as E, AbsInfo, InputDevice, UInput
except ImportError:
    print('python3-evdev is missing')
    sys.exit(2)

REPO = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
SHELL = os.path.join(REPO, 'hosts', 'linux', 'desktop', 'shell')
DAEMON = os.path.join(SHELL, 'hearth-input')
with open(os.path.join(SHELL, 'input-layouts.json'), encoding='utf-8') as f:
    TEMPLATES = {t['id']: t['layout'] for t in json.load(f)['templates']}

BUTTON_CODES = {'a': E.BTN_SOUTH, 'b': E.BTN_EAST, 'x': E.BTN_NORTH, 'y': E.BTN_WEST, 'lb': E.BTN_TL,
                'rb': E.BTN_TR, 'view': E.BTN_SELECT, 'menu': E.BTN_START, 'guide': E.BTN_MODE,
                'l3': E.BTN_THUMBL, 'r3': E.BTN_THUMBR}
HAT = {'up': (E.ABS_HAT0Y, -1), 'down': (E.ABS_HAT0Y, 1), 'left': (E.ABS_HAT0X, -1), 'right': (E.ABS_HAT0X, 1)}
MOUSE = {'left': E.BTN_LEFT, 'right': E.BTN_RIGHT, 'middle': E.BTN_MIDDLE, 'back': E.BTN_SIDE, 'forward': E.BTN_EXTRA}

RESULTS = []


def check(name, ok, detail=''):
    RESULTS.append((name, bool(ok), detail))
    print('%s  %s%s' % ('PASS' if ok else 'FAIL', name, ('  (%s)' % detail) if detail else ''), flush=True)
    return bool(ok)


def wait_for(fn, timeout, step=0.02):
    end = time.monotonic() + timeout
    while True:
        v = fn()
        if v or time.monotonic() >= end:
            return v
        time.sleep(step)


# ---------------------------------------------------------------------- fake controllers

class FakePad:
    """A controller made with uinput. profile: xpad (Xbox 360 pad over USB), xbox-bt (Xbox Wireless Controller
    over Bluetooth with hid-generic/hid-microsoft: right stick on ABS_Z/RZ, triggers on BRAKE/GAS) or dualsense
    (hid-playstation: Triangle is BTN_NORTH, Square BTN_WEST, 0..255 axes, digital L2/R2 too)."""

    def __init__(self, profile='xpad', name=None):
        keys = [E.BTN_SOUTH, E.BTN_EAST, E.BTN_NORTH, E.BTN_WEST, E.BTN_TL, E.BTN_TR, E.BTN_SELECT, E.BTN_START,
                E.BTN_MODE, E.BTN_THUMBL, E.BTN_THUMBR]
        hat = AbsInfo(0, -1, 1, 0, 0, 0)
        if profile == 'xpad':
            stick, trig = AbsInfo(0, -32768, 32767, 16, 128, 0), AbsInfo(0, 0, 1023, 0, 0, 0)
            self.axes = {'lx': E.ABS_X, 'ly': E.ABS_Y, 'rx': E.ABS_RX, 'ry': E.ABS_RY, 'lt': E.ABS_Z, 'rt': E.ABS_RZ}
            ids = dict(name=name or 'Microsoft X-Box 360 pad', vendor=0x045e, product=0x028e, bustype=E.BUS_USB)
        elif profile == 'xbox-bt':
            stick, trig = AbsInfo(32768, 0, 65535, 255, 4095, 0), AbsInfo(0, 0, 1023, 3, 63, 0)
            self.axes = {'lx': E.ABS_X, 'ly': E.ABS_Y, 'rx': E.ABS_Z, 'ry': E.ABS_RZ, 'lt': E.ABS_BRAKE, 'rt': E.ABS_GAS}
            ids = dict(name=name or 'Xbox Wireless Controller', vendor=0x045e, product=0x0b13, bustype=E.BUS_BLUETOOTH)
        elif profile == 'dualsense':
            stick, trig = AbsInfo(128, 0, 255, 0, 0, 0), AbsInfo(0, 0, 255, 0, 0, 0)
            keys += [E.BTN_TL2, E.BTN_TR2]
            self.axes = {'lx': E.ABS_X, 'ly': E.ABS_Y, 'rx': E.ABS_RX, 'ry': E.ABS_RY, 'lt': E.ABS_Z, 'rt': E.ABS_RZ}
            ids = dict(name=name or 'Sony Interactive Entertainment DualSense Wireless Controller', vendor=0x054c,
                       product=0x0ce6, bustype=E.BUS_USB)
        else:
            raise ValueError(profile)
        self.info = {}
        absl = []
        for k in ('lx', 'ly', 'rx', 'ry'):
            absl.append((self.axes[k], stick))
            self.info[k] = stick
        for k in ('lt', 'rt'):
            absl.append((self.axes[k], trig))
            self.info[k] = trig
        absl += [(E.ABS_HAT0X, hat), (E.ABS_HAT0Y, hat)]
        self.name = ids['name']
        self.ui = UInput({E.EV_KEY: keys, E.EV_ABS: absl}, version=0x110, **ids)
        self.path = wait_for(lambda: find_device(self.name, exclude=TAKEN), 5)
        if not self.path:
            raise RuntimeError('the fake controller %r never showed up in /dev/input' % self.name)
        TAKEN.add(self.path)

    def button(self, ctl, down):
        if ctl in HAT:
            code, v = HAT[ctl]
            self.ui.write(E.EV_ABS, code, v if down else 0)
        else:
            self.ui.write(E.EV_KEY, BUTTON_CODES[ctl], 1 if down else 0)
        self.ui.syn()

    def key(self, code, down):
        self.ui.write(E.EV_KEY, code, 1 if down else 0)
        self.ui.syn()

    def axis(self, k, v):
        """Sticks -1..1 (y down), triggers 0..1."""
        i = self.info[k]
        if k in ('lt', 'rt'):
            raw = i.min + v * (i.max - i.min)
        else:
            raw = (i.max + i.min) / 2 + v * (i.max - i.min) / 2
        self.ui.write(E.EV_ABS, self.axes[k], int(round(max(i.min, min(i.max, raw)))))
        self.ui.syn()

    def center(self):
        for k in ('lx', 'ly', 'rx', 'ry'):
            self.axis(k, 0.0)
        for k in ('lt', 'rt'):
            self.axis(k, 0.0)
        for d in ('up', 'left'):
            self.button(d, False)

    def close(self):
        TAKEN.discard(self.path)
        self.ui.close()


TAKEN = set()


def find_device(name, exclude=()):
    for path in evdev.list_devices():
        if path in exclude:
            continue
        try:
            d = InputDevice(path)
        except OSError:
            continue
        n = d.name
        d.close()
        if n == name:
            return path
    return None


def grab_state(path):
    """'busy' when someone else holds the device (EVIOCGRAB fails with EBUSY), 'free' when a grab works (let go
    straight away). Only asked when the daemon isn't about to grab it itself."""
    try:
        d = InputDevice(path)
    except OSError as err:
        return 'error: %s' % err
    try:
        d.grab()
    except OSError as err:
        d.close()
        return 'busy' if err.errno == errno.EBUSY else 'error: %s' % err
    d.ungrab()
    d.close()
    return 'free'


# ---------------------------------------------------------------------- reading devices

class Recorder:
    """Every key/button/relative event of one input device, with the time it was read."""

    def __init__(self, path, types=(E.EV_KEY, E.EV_REL)):
        self.dev = InputDevice(path)
        self.types = types
        self.events = []
        self.lock = threading.Lock()
        self.stop = False
        self.gone = False
        threading.Thread(target=self.loop, daemon=True).start()

    def loop(self):
        while not self.stop:
            try:
                r, _, _ = select.select([self.dev.fd], [], [], 0.1)
                if r:
                    for ev in self.dev.read():
                        if ev.type in self.types:
                            with self.lock:
                                self.events.append((time.monotonic(), ev.type, ev.code, ev.value))
            except (BlockingIOError, InterruptedError):
                pass
            except (OSError, ValueError):
                self.gone = True
                return

    def mark(self):
        with self.lock:
            return len(self.events)

    def since(self, m):
        with self.lock:
            return list(self.events[m:])

    def wait(self, m, etype, code, value, timeout=1.0):
        """The first event since mark m matching (value None: any), or None."""
        def find():
            for ev in self.since(m):
                if ev[1] == etype and ev[2] == code and (value is None or ev[3] == value):
                    return ev
            return None
        return wait_for(find, timeout)

    def rel(self, m, code):
        return sum(v for _, t, c, v in self.since(m) if t == E.EV_REL and c == code)

    def close(self):
        self.stop = True
        time.sleep(0.15)
        try:
            self.dev.close()
        except OSError:
            pass


# ---------------------------------------------------------------------- Hearth's side of the socket

def runtime_dir():
    """The same choice the daemon (and hearth-core overlay_link.rs) makes."""
    d = '/run/user/%d' % os.getuid()
    if os.path.isdir(d):
        return d
    return os.environ.get('XDG_RUNTIME_DIR') or d


class Hearth:
    """One connection held open, a state line every 0.5 s (input_map.rs start_link), events read back."""

    def __init__(self, path, **state):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(path)
        self.state = {'cmd': 'state', 'game': None, 'front': False, 'overlay': False, 'layout': None,
                      'prefs': {'sensitivity': 1, 'gyro': False}}
        self.state.update(state)
        self.sending = True
        self.lock = threading.Lock()
        self.events = []
        self.closed = False
        threading.Thread(target=self.send_loop, daemon=True).start()
        threading.Thread(target=self.read_loop, daemon=True).start()

    def send_now(self):
        with self.lock:
            line = json.dumps(self.state) + '\n'
        try:
            self.sock.sendall(line.encode())
        except OSError:
            self.closed = True

    def set(self, **kw):
        with self.lock:
            self.state.update(kw)
        if self.sending:
            self.send_now()

    def send_loop(self):
        while not self.closed:
            if self.sending:
                self.send_now()
            time.sleep(0.5)

    def read_loop(self):
        buf = b''
        while not self.closed:
            try:
                chunk = self.sock.recv(65536)
            except OSError:
                break
            if not chunk:
                break
            buf += chunk
            while b'\n' in buf:
                line, buf = buf.split(b'\n', 1)
                try:
                    obj = json.loads(line)
                except ValueError:
                    obj = {'unparsed': line.decode(errors='replace')}
                with self.lock:
                    self.events.append((time.monotonic(), obj))
        self.closed = True

    def mark(self):
        with self.lock:
            return len(self.events)

    def wait_event(self, m, pred, timeout):
        def find():
            with self.lock:
                return next((o for _, o in self.events[m:] if pred(o)), None)
        return wait_for(find, timeout)

    def close(self):
        self.closed = True
        try:
            self.sock.close()
        except OSError:
            pass


def request(path, cmd, timeout=2.0):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(timeout)
    s.connect(path)
    s.sendall((json.dumps(cmd) + '\n').encode())
    buf = b''
    while b'\n' not in buf:
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
    s.close()
    return json.loads(buf.split(b'\n', 1)[0] or b'{}')


# ---------------------------------------------------------------------- what a layout should do

def press_of(layout, ctl, shifted=False):
    b = None
    if shifted:
        b = layout.get('shift', {}).get('buttons', {}).get(ctl)
    if b is None:
        b = layout['buttons'].get(ctl, 'none')
    return (b, None) if isinstance(b, str) else (b['press'], b.get('hold'))


def codes_of(action):
    """An action → [(device, code)] it holds down, device 'kbd' or 'ptr'. Scrolling → [('wheel', +1/-1)]."""
    if action.startswith('key:'):
        return [('kbd', E.ecodes['KEY_' + n]) for n in action[4:].split('+')]
    if action.startswith('mouse:'):
        return [('ptr', MOUSE[action[6:]])]
    if action.startswith('scroll:'):
        return [('wheel', {'up': 1, 'down': -1}.get(action[7:], 0))]
    return []


class Rig:
    def __init__(self, kbd, ptr):
        self.dev = {'kbd': kbd, 'ptr': ptr}

    def marks(self):
        return {k: d.mark() for k, d in self.dev.items()}

    def got(self, m, dev, code, value, timeout=1.0):
        return self.dev[dev].wait(m[dev], E.EV_KEY, code, value, timeout)

    def keylog(self, m):
        out = []
        for k, d in self.dev.items():
            out += ['%s=%d' % (code_name(c), v) for _, t, c, v in d.since(m[k]) if t == E.EV_KEY]
        return ' '.join(out) or 'nothing'


def code_name(c):
    n = E.KEY.get(c) or E.BTN.get(c) or str(c)
    return n[0] if isinstance(n, (list, tuple)) else n


def check_button(rig, pad, layout, ctl, label, shifted=False):
    """Press and let go a button; the layout's keys/clicks go down (while held, for a plain button) and up."""
    press, hold = press_of(layout, ctl, shifted)
    codes = codes_of(press)
    if not codes:
        return
    m = rig.marks()
    if codes[0][0] == 'wheel':
        pad.button(ctl, True)
        time.sleep(0.15)
        pad.button(ctl, False)
        want = codes[0][1]
        ok = wait_for(lambda: rig.dev['ptr'].rel(m['ptr'], E.REL_WHEEL) * want > 0, 1.0)
        check('%s: %s → %s' % (label, ctl, press), ok, 'wheel %d' % rig.dev['ptr'].rel(m['ptr'], E.REL_WHEEL))
        return
    pad.button(ctl, True)
    if hold is None:
        downs = all(rig.got(m, d, c, 1) for d, c in codes)
        time.sleep(0.05)
        pad.button(ctl, False)
        ups = all(rig.got(m, d, c, 0) for d, c in codes)
    else:   # tap: goes down when let go, up a moment later
        time.sleep(0.08)
        pad.button(ctl, False)
        downs = all(rig.got(m, d, c, 1) for d, c in codes)
        ups = all(rig.got(m, d, c, 0) for d, c in codes)
    check('%s: %s → %s' % (label, ctl, press), downs and ups, rig.keylog(m))


# ---------------------------------------------------------------------- the run

def main():
    if not os.access('/dev/uinput', os.W_OK):
        print('no access to /dev/uinput (modprobe uinput; run as root or give this user the device)')
        return 2
    tmp = tempfile.mkdtemp(prefix='hearth-input-e2e-')
    os.chmod(tmp, 0o700)
    os.environ['XDG_RUNTIME_DIR'] = tmp
    sock_path = os.path.join(runtime_dir(), 'hearth', 'input.sock')
    log_path = os.path.join(tmp, 'daemon.log')
    print('runtime dir %s, socket %s' % (runtime_dir(), sock_path), flush=True)

    pad = FakePad('xpad')
    time.sleep(0.3)
    logf = open(log_path, 'w')
    env = dict(os.environ, PYTHONUNBUFFERED='1', PYTHONDONTWRITEBYTECODE='1')
    proc = subprocess.Popen([sys.executable, DAEMON, 'serve'], stdout=logf, stderr=subprocess.STDOUT, env=env)
    extra = []
    recs = []
    try:
        run(proc, pad, sock_path, log_path, extra, recs)
    except Exception as err:     # a crash in the test itself is a failure too
        import traceback
        traceback.print_exc()
        check('the test ran to the end', False, '%s: %s' % (type(err).__name__, err))
    finally:
        for r in recs:
            r.close()
        if proc.poll() is None:
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(3)
            except subprocess.TimeoutExpired:
                proc.kill()
        for p in extra + [pad]:
            try:
                p.close()
            except Exception:
                pass
        logf.close()

    failed = [r for r in RESULTS if not r[1]]
    print('\n%d checks, %d passed, %d failed' % (len(RESULTS), len(RESULTS) - len(failed), len(failed)))
    if failed:
        print('\nFailed:')
        for name, _, detail in failed:
            print('  - %s%s' % (name, ('  (%s)' % detail) if detail else ''))
    with open(log_path, encoding='utf-8', errors='replace') as f:
        daemon_log = f.read()
    if failed or os.environ.get('E2E_SHOW_LOG'):
        print('\n---- hearth-input log ----\n' + daemon_log + '---- end of log ----')
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if failed else 0


def run(proc, pad, sock_path, log_path, extra, recs):
    def daemon_log():
        with open(log_path, encoding='utf-8', errors='replace') as f:
            return f.read()

    def status():
        return request(sock_path, {'cmd': 'status'})

    # ------------------------------------------------------------------ start
    up = wait_for(lambda: os.path.exists(sock_path) and 'ready at' in daemon_log(), 6)
    if not check('daemon starts and listens on its socket', up and proc.poll() is None,
                 'exit code %s' % proc.poll() if proc.poll() is not None else ''):
        return
    st = oct(os.stat(sock_path).st_mode & 0o777)
    check('socket is 0600 in a 0700 folder', st == '0o600' and (os.stat(os.path.dirname(sock_path)).st_mode & 0o777) == 0o700, st)
    seen = wait_for(lambda: any(d['name'] == pad.name for d in status()['devices']), 4)
    check('daemon finds the controller', seen, json.dumps(status().get('devices')))
    check('nothing grabbed before Hearth says so', grab_state(pad.path) == 'free' and not status()['active'])

    h = Hearth(sock_path)
    rts, shooter = TEMPLATES['rts'], TEMPLATES['shooter']
    h.set(game='test', front=True, overlay=False, layout=rts)

    kbd_path = wait_for(lambda: find_device('Hearth Input keyboard'), 5)
    ptr_path = wait_for(lambda: find_device('Hearth Input mouse'), 5)
    if not check('virtual keyboard and mouse appear', kbd_path and ptr_path, 'keyboard %s, mouse %s' % (kbd_path, ptr_path)):
        return
    kbd, ptr = Recorder(kbd_path), Recorder(ptr_path)
    recs += [kbd, ptr]
    rig = Rig(kbd, ptr)
    active = wait_for(lambda: status()['active'], 3)
    s = status()
    check('layout on (status)', active and s['layout'] == rts['name'] and s['error'] is None, json.dumps(s))
    check('controller grabbed (a second EVIOCGRAB gets EBUSY)', grab_state(pad.path) == 'busy')
    check('status says grabbed', any(d['name'] == pad.name and d['grabbed'] for d in s['devices']))
    time.sleep(0.2)   # udev/libinput settle; nothing should depend on it, but the readers just opened

    # The game doesn't see the controller while the layout is on: a plain reader of the pad gets nothing.
    spy = Recorder(pad.path, types=(E.EV_KEY,))
    recs.append(spy)
    m = spy.mark()
    check_button(rig, pad, rts, 'a', 'RTS')
    check('the game sees none of it (grab really holds)', not spy.since(m), '%d events leaked' % len(spy.since(m)))

    # ------------------------------------------------------------------ RTS buttons
    for ctl in ('b', 'x', 'y', 'rb', 'menu', 'l3', 'r3', 'up', 'down', 'left', 'right'):
        check_button(rig, pad, rts, ctl, 'RTS')

    # Triggers → clicks (threshold 0.3).
    for t, code in (('rt', E.BTN_LEFT), ('lt', E.BTN_RIGHT)):
        m = rig.marks()
        pad.axis(t, 0.2)
        time.sleep(0.15)
        below = not rig.got(m, 'ptr', code, 1, 0.01)
        pad.axis(t, 0.8)
        down = rig.got(m, 'ptr', code, 1)
        pad.axis(t, 0.0)
        upv = rig.got(m, 'ptr', code, 0)
        check('RTS: %s → %s (nothing below the threshold)' % (t, press_of(rts, t)[0]), below and down and upv, rig.keylog(m))

    # Right stick → pointer.
    m = rig.marks()
    pad.axis('rx', 1.0)
    time.sleep(0.5)
    pad.axis('rx', 0.0)
    time.sleep(0.1)
    dx, dy = ptr.rel(m['ptr'], E.REL_X), ptr.rel(m['ptr'], E.REL_Y)
    check('RTS: right stick right → pointer moves right', dx > 200 and abs(dy) < 20, 'dx %d dy %d in 0.5 s' % (dx, dy))
    m = rig.marks()
    pad.axis('ry', 1.0)
    time.sleep(0.5)
    pad.axis('ry', 0.0)
    time.sleep(0.1)
    dx, dy = ptr.rel(m['ptr'], E.REL_X), ptr.rel(m['ptr'], E.REL_Y)
    check('RTS: right stick down → pointer moves down', dy > 200 and abs(dx) < 20, 'dx %d dy %d' % (dx, dy))
    m = rig.marks()
    time.sleep(0.3)
    check('RTS: centred stick → pointer stays still', ptr.rel(m['ptr'], E.REL_X) == 0 and ptr.rel(m['ptr'], E.REL_Y) == 0)

    # Left stick → arrow keys.
    m = rig.marks()
    pad.axis('ly', -1.0)
    down = rig.got(m, 'kbd', E.KEY_UP, 1)
    pad.axis('ly', 0.0)
    upv = rig.got(m, 'kbd', E.KEY_UP, 0)
    check('RTS: left stick up → UP held, let go when centred', down and upv, rig.keylog(m))
    m = rig.marks()
    pad.axis('lx', 0.8)
    pad.axis('ly', 0.8)
    both = rig.got(m, 'kbd', E.KEY_RIGHT, 1) and rig.got(m, 'kbd', E.KEY_DOWN, 1)
    pad.axis('lx', 0.0)
    pad.axis('ly', 0.0)
    gone = rig.got(m, 'kbd', E.KEY_RIGHT, 0) and rig.got(m, 'kbd', E.KEY_DOWN, 0)
    check('RTS: left stick diagonal → two arrows', both and gone, rig.keylog(m))

    # Shift layer (LB held).
    pad.button('lb', True)
    time.sleep(0.05)
    check_button(rig, pad, rts, 'a', 'RTS shift layer', shifted=True)
    check_button(rig, pad, rts, 'up', 'RTS shift layer', shifted=True)
    pad.button('lb', False)
    time.sleep(0.05)
    check_button(rig, pad, rts, 'a', 'RTS after shift let go')

    # Radial menu (View held, right stick up → slot 0, let go).
    m = rig.marks()
    hm = h.mark()
    pad.button('view', True)
    time.sleep(0.1)
    pad.axis('ry', -1.0)
    time.sleep(0.2)
    pad.button('view', False)
    pad.axis('ry', 0.0)
    first = codes_of(rts['radial'][0])[0]
    got = rig.got(m, first[0], first[1], 1) and rig.got(m, first[0], first[1], 0)
    osd = h.wait_event(hm, lambda o: o.get('t') == 'osd', 1.0)
    check('RTS: radial menu picks %s' % rts['radial'][0], got and ptr.rel(m['ptr'], E.REL_Y) == 0,
          rig.keylog(m) + '; osd %s' % (osd and osd.get('text')))

    # ------------------------------------------------------------------ other controllers
    # A DualSense (hid-playstation): Triangle is the top button (Y on Xbox), Square the left one (X).
    ds = FakePad('dualsense')
    extra.append(ds)
    grabbed = wait_for(lambda: grab_state(ds.path) == 'busy', 5, step=0.25)
    check('DualSense plugged in while the layout is on gets grabbed', grabbed)
    if grabbed:
        m = rig.marks()
        ds.key(E.BTN_NORTH, True)
        ok1 = rig.got(m, *codes_of(press_of(rts, 'y')[0])[0], 1)
        ds.key(E.BTN_NORTH, False)
        ds.key(E.BTN_WEST, True)
        ok2 = rig.got(m, *codes_of(press_of(rts, 'x')[0])[0], 1)
        ds.key(E.BTN_WEST, False)
        check('DualSense: Triangle → Y action, Square → X action', ok1 and ok2, rig.keylog(m))
        m = rig.marks()
        ds.axis('lt', 1.0)
        ds.key(E.BTN_TL2, True)
        down = rig.got(m, 'ptr', E.BTN_RIGHT, 1)
        ds.axis('lt', 0.0)
        ds.key(E.BTN_TL2, False)
        check('DualSense: L2 → right click', down and rig.got(m, 'ptr', E.BTN_RIGHT, 0), rig.keylog(m))
        m = rig.marks()
        ds.axis('rx', 1.0)
        time.sleep(0.4)
        ds.axis('rx', 0.0)
        time.sleep(0.1)
        check('DualSense: right stick → pointer', ptr.rel(m['ptr'], E.REL_X) > 150, 'dx %d' % ptr.rel(m['ptr'], E.REL_X))
    ds.close()
    extra.remove(ds)

    # An Xbox controller over Bluetooth without xpadneo: right stick on ABS_Z/ABS_RZ, triggers on BRAKE/GAS.
    bt = FakePad('xbox-bt')
    extra.append(bt)
    grabbed = wait_for(lambda: grab_state(bt.path) == 'busy', 5, step=0.25)
    check('Bluetooth Xbox controller gets grabbed', grabbed)
    if grabbed:
        bt.center()
        time.sleep(0.15)
        m = rig.marks()
        bt.axis('rx', 1.0)
        time.sleep(0.4)
        bt.axis('rx', 0.0)
        time.sleep(0.15)
        dx = ptr.rel(m['ptr'], E.REL_X)
        clicks = [k for k in rig.keylog(m).split() if k.startswith('BTN_')]
        check('Bluetooth Xbox: right stick (ABS_Z) → pointer, not a trigger', dx > 150 and not clicks,
              'dx %d, %s' % (dx, rig.keylog(m)))
        m = rig.marks()
        bt.axis('lt', 1.0)
        down = rig.got(m, 'ptr', E.BTN_RIGHT, 1)
        bt.axis('lt', 0.0)
        check('Bluetooth Xbox: LT (ABS_BRAKE) → right click', down and rig.got(m, 'ptr', E.BTN_RIGHT, 0), rig.keylog(m))
        m = rig.marks()
        bt.axis('ly', -1.0)
        down = rig.got(m, 'kbd', E.KEY_UP, 1)
        bt.axis('ly', 0.0)
        check('Bluetooth Xbox: left stick (0..65535) → arrows', down and rig.got(m, 'kbd', E.KEY_UP, 0), rig.keylog(m))
    m = rig.marks()
    bt.close()
    extra.remove(bt)
    time.sleep(0.3)
    check('unplugging a controller lets go of everything it held', 'BTN_RIGHT=1' not in rig.keylog(m) or 'BTN_RIGHT=0' in rig.keylog(m))

    # A new device taking the event node number an ignored (non-controller) device just left. Handheld Daemon
    # and InputPlumber remove and re-make their devices when they start or switch modes.
    dummy = UInput({E.EV_KEY: [E.KEY_A, E.KEY_B]}, name='E2E Not A Controller')
    dpath = wait_for(lambda: find_device('E2E Not A Controller'), 5)
    time.sleep(3.6)    # the daemon looks at it (and ignores it) on its next scan
    dummy.close()
    reuse = FakePad('xpad', name='Microsoft X-Box 360 pad (reused node)')
    extra.append(reuse)
    grabbed = wait_for(lambda: grab_state(reuse.path) == 'busy', 7, step=0.25)
    check('controller on a reused event node gets grabbed', grabbed,
          'node %s, the ignored device had %s%s' % (reuse.path, dpath, '' if reuse.path == dpath else ' (not reused: inconclusive)'))
    reuse.close()
    extra.remove(reuse)

    # ------------------------------------------------------------------ switching layouts
    m = rig.marks()
    pad.button('a', True)
    rig.got(m, 'kbd', E.KEY_SPACE, 1)
    h.set(layout=shooter)
    ok = wait_for(lambda: status()['layout'] == shooter['name'], 2)
    released = rig.got(m, 'kbd', E.KEY_SPACE, 0)
    m2 = rig.marks()
    pad.button('a', False)
    time.sleep(0.2)
    check('layout change: held keys let go, the held button does nothing until pressed again',
          ok and released and rig.keylog(m2) == 'nothing', rig.keylog(m))

    for ctl in ('a', 'lb', 'rb', 'menu', 'l3', 'r3', 'up', 'right', 'x'):
        check_button(rig, pad, shooter, ctl, 'Shooter')
    # Tap B → C; hold B → left Ctrl while held, no C.
    m = rig.marks()
    pad.button('b', True)
    time.sleep(0.6)
    held = rig.got(m, 'kbd', E.KEY_LEFTCTRL, 1, 0.1)
    pad.button('b', False)
    let = rig.got(m, 'kbd', E.KEY_LEFTCTRL, 0)
    time.sleep(0.15)
    check('Shooter: hold B → %s while held, no tap' % shooter['buttons']['b']['hold'],
          held and let and not rig.got(m, 'kbd', E.KEY_C, 1, 0.01), rig.keylog(m))
    m = rig.marks()
    pad.axis('ly', -1.0)
    pad.axis('lx', -1.0)
    wa = rig.got(m, 'kbd', E.KEY_W, 1) and rig.got(m, 'kbd', E.KEY_A, 1)
    pad.axis('ly', 0.0)
    pad.axis('lx', 0.0)
    check('Shooter: left stick up-left → W+A', wa and rig.got(m, 'kbd', E.KEY_W, 0) and rig.got(m, 'kbd', E.KEY_A, 0), rig.keylog(m))
    m = rig.marks()
    pad.axis('rt', 0.5)
    check('Shooter: RT half way → left click', rig.got(m, 'ptr', E.BTN_LEFT, 1), rig.keylog(m))
    pad.axis('rt', 0.0)
    rig.got(m, 'ptr', E.BTN_LEFT, 0)

    # ------------------------------------------------------------------ letting go
    # Game leaves the front while W and the left button are held.
    m = rig.marks()
    pad.axis('ly', -1.0)
    pad.axis('rt', 1.0)
    rig.got(m, 'kbd', E.KEY_W, 1)
    rig.got(m, 'ptr', E.BTN_LEFT, 1)
    h.set(front=False)
    released = rig.got(m, 'kbd', E.KEY_W, 0) and rig.got(m, 'ptr', E.BTN_LEFT, 0)
    off = wait_for(lambda: not status()['active'], 2)
    check('front=false: every key and button let go', released and off, rig.keylog(m))
    check('front=false: controller let go (grab works for others)', grab_state(pad.path) == 'free')
    pad.axis('ly', 0.0)
    pad.axis('rt', 0.0)
    time.sleep(0.1)
    m = rig.marks()
    sm = spy.mark()
    pad.button('a', True)
    pad.button('a', False)
    time.sleep(0.3)
    check('front=false: the game gets the controller, no keys come out',
          rig.keylog(m) == 'nothing' and len(spy.since(sm)) >= 2, '%s; game saw %d events' % (rig.keylog(m), len(spy.since(sm))))
    h.set(front=True)
    check('front=true again: grabbed again', wait_for(lambda: status()['active'], 2) and grab_state(pad.path) == 'busy')

    # Quick Access (overlay): Hearth asks and waits for the answer.
    h.set(front=False, overlay=True)
    r = request(sock_path, {'cmd': 'overlay', 'on': True})
    check('overlay on: answered after letting go', r.get('ok') and r.get('grabbed') is False and grab_state(pad.path) == 'free', json.dumps(r))
    h.set(front=True, overlay=False)
    r = request(sock_path, {'cmd': 'overlay', 'on': False})
    check('overlay off: grabbed again', r.get('ok') and r.get('grabbed') is True and grab_state(pad.path) == 'busy', json.dumps(r))

    # Guide: passed to Hearth, controller let go for a moment, keys let go.
    time.sleep(0.2)
    m = rig.marks()
    pad.button('a', True)
    rig.got(m, 'kbd', E.KEY_SPACE, 1)
    hm = h.mark()
    t0 = time.monotonic()
    pad.button('guide', True)
    ev = h.wait_event(hm, lambda o: o.get('t') == 'guide', 1.5)
    free = grab_state(pad.path) == 'free'
    check('Guide → {"t": "guide"} to Hearth', ev, 'after %.2f s' % (time.monotonic() - t0))
    check('Guide → controller let go and keys up', free and rig.got(m, 'kbd', E.KEY_SPACE, 0), rig.keylog(m))
    pad.button('guide', False)
    pad.button('a', False)
    back = wait_for(lambda: status()['active'], 4)
    check('after Guide: grabbed again once Quick Access isn\'t opening', back and grab_state(pad.path) == 'busy',
          'after %.2f s' % (time.monotonic() - t0))

    # Heartbeat: Hearth stops talking (crashed, stuck) → let go after 5 s.
    time.sleep(0.2)
    m = rig.marks()
    pad.button('a', True)
    rig.got(m, 'kbd', E.KEY_SPACE, 1)
    h.sending = False
    t0 = time.monotonic()
    up_ev = rig.got(m, 'kbd', E.KEY_SPACE, 0, 9)
    took = (up_ev[0] - t0) if up_ev else -1
    check('Hearth silent → let go after about 5 s', up_ev and 4.0 <= took <= 7.5 and grab_state(pad.path) == 'free',
          'after %.1f s' % took)
    h.sending = True
    h.send_now()
    back = wait_for(lambda: status()['active'], 2)
    m = rig.marks()
    pad.button('a', False)
    time.sleep(0.2)
    quiet = rig.keylog(m) == 'nothing'
    check('Hearth back → grabbed again, the held A does nothing', back and quiet and grab_state(pad.path) == 'busy', rig.keylog(m))
    check_button(rig, pad, shooter, 'a', 'Shooter after the heartbeat')

    # Hearth's connection goes away and comes back (Hearth restarted).
    h.close()
    lines = daemon_log().count('\n')
    h2 = Hearth(sock_path, game='test', front=True, overlay=False, layout=shooter)
    time.sleep(0.5)
    flaps = [l for l in daemon_log().split('\n')[lines:] if l.endswith('layout off')]
    check('a new connection (Hearth restarted) carries on', status()['active'], '%d layout off on the way' % len(flaps))
    check_button(rig, pad, shooter, 'a', 'Shooter after reconnecting')

    # ------------------------------------------------------------------ the daemon ends
    m = rig.marks()
    pad.button('a', True)
    rig.got(m, 'kbd', E.KEY_SPACE, 1)
    proc.send_signal(signal.SIGTERM)
    try:
        code = proc.wait(4)
    except subprocess.TimeoutExpired:
        code = None
    pad.button('a', False)
    check('SIGTERM: daemon exits cleanly', code == 0, 'exit code %s' % code)
    check('SIGTERM: controller let go, socket removed', grab_state(pad.path) == 'free' and not os.path.exists(sock_path))
    gone = wait_for(lambda: not find_device('Hearth Input keyboard') and not find_device('Hearth Input mouse'), 3)
    check('SIGTERM: virtual keyboard and mouse removed', gone)
    log = daemon_log()
    check('daemon log has no Python traceback', 'Traceback' not in log)
    h2.close()


if __name__ == '__main__':
    sys.exit(main())
