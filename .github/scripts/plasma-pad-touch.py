#!/usr/bin/python3
"""Touch users keep their pop-up keyboard (plasma-pad.yml, KWin nested in labwc so input is real):
  plasma-pad-touch.py devices FIFO        a uinput absolute mouse and a uinput touchscreen, run before the session
                                          (labwc reads them); lines on FIFO: "click X Y", "tap X Y"
  plasma-pad-touch.py test OUT BIN FIFO   in the running session: a mouse click on a text field doesn't open Hearth's
                                          keyboard, a touch tap does, B closes it and it stays closed, nothing at rest
Hearth's typing (hearth-kwin-im --panel) is KWin's input method; KWin shows its panel only for touch, and hearth-pad
--plasma opens Hearth's keyboard then (pad.log: "a text field was touched")."""
import json
import os
import subprocess
import sys
import time


def devices(fifo):
    import evdev
    e = evdev.ecodes
    A = evdev.AbsInfo
    mouse = evdev.UInput({e.EV_KEY: [e.BTN_LEFT, e.BTN_RIGHT],
                          e.EV_ABS: [(e.ABS_X, A(0, 0, 1919, 0, 0, 0)), (e.ABS_Y, A(0, 0, 1079, 0, 0, 0))]},
                         name='Hearth check absolute mouse')
    touch = evdev.UInput({e.EV_KEY: [e.BTN_TOUCH],
                          e.EV_ABS: [(e.ABS_X, A(0, 0, 1919, 0, 0, 0)), (e.ABS_Y, A(0, 0, 1079, 0, 0, 0)),
                                     (e.ABS_MT_SLOT, A(0, 0, 9, 0, 0, 0)), (e.ABS_MT_TRACKING_ID, A(0, 0, 65535, 0, 0, 0)),
                                     (e.ABS_MT_POSITION_X, A(0, 0, 1919, 0, 0, 0)), (e.ABS_MT_POSITION_Y, A(0, 0, 1079, 0, 0, 0))]},
                         name='Hearth check touchscreen', input_props=[e.INPUT_PROP_DIRECT])
    if not os.path.exists(fifo):
        os.mkfifo(fifo)
    tid = 1
    while True:
        with open(fifo) as f:
            for line in f:
                p = line.split()
                if len(p) != 3:
                    continue
                x, y = int(p[1]), int(p[2])
                if p[0] == 'click':
                    mouse.write(e.EV_ABS, e.ABS_X, x)
                    mouse.write(e.EV_ABS, e.ABS_Y, y)
                    mouse.syn()
                    time.sleep(0.15)
                    for v in (1, 0):
                        mouse.write(e.EV_KEY, e.BTN_LEFT, v)
                        mouse.syn()
                        time.sleep(0.08)
                elif p[0] == 'tap':
                    tid += 1
                    for k, v in ((e.ABS_MT_SLOT, 0), (e.ABS_MT_TRACKING_ID, tid), (e.ABS_MT_POSITION_X, x), (e.ABS_MT_POSITION_Y, y)):
                        touch.write(e.EV_ABS, k, v)
                    touch.write(e.EV_KEY, e.BTN_TOUCH, 1)
                    touch.write(e.EV_ABS, e.ABS_X, x)
                    touch.write(e.EV_ABS, e.ABS_Y, y)
                    touch.syn()
                    time.sleep(0.08)
                    touch.write(e.EV_ABS, e.ABS_MT_SLOT, 0)
                    touch.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, -1)
                    touch.write(e.EV_KEY, e.BTN_TOUCH, 0)
                    touch.syn()


def test(out, bin_dir, fifo):
    import dbus
    from PIL import Image
    bus = dbus.SessionBus()
    bad = []
    host_log = os.path.join(out, 'host-touch.log')
    pad_log = os.path.join(os.environ['HOME'], '.local/share/nimbus/pad.log')
    sim = '/tmp/padsim-touch'
    os.makedirs(sim, exist_ok=True)
    open(os.path.join(sim, 'in'), 'w').close()

    def note(t, m):
        with open(os.path.join(out, 'results.txt'), 'a') as f:
            f.write('::notice title=%s::%s\n' % (t, str(m).replace('\n', '|').replace('%', '%25')[:2500]))

    def err(t, m):
        bad.append(t)
        print('::error title=%s::%s' % (t, str(m).replace('\n', '|')[:2500]), flush=True)

    def tail(p, n=600):
        try:
            return open(p, errors='replace').read()[-n:]
        except OSError:
            return ''

    def vk():
        try:
            p = dbus.Interface(bus.get_object('org.kde.KWin', '/VirtualKeyboard'), 'org.freedesktop.DBus.Properties')
            return {str(k): str(v) for k, v in p.GetAll('org.kde.kwin.VirtualKeyboard').items()}
        except Exception as e:
            return {'error': repr(e)[:200]}

    def kb_open():
        t = tail(host_log, 100000)
        return t.rfind('keyboard open') > t.rfind('keyboard closed')

    def dev(line):
        with open(fifo, 'w') as f:
            f.write(line + '\n')
        time.sleep(0.4)

    def press(b):
        with open(os.path.join(sim, 'in'), 'a') as f:
            f.write(json.dumps({'b': b, 'down': True}) + '\n')
        time.sleep(0.12)
        with open(os.path.join(sim, 'in'), 'a') as f:
            f.write(json.dumps({'b': b, 'down': False}) + '\n')
        time.sleep(0.4)

    def shot(name):
        # Nested, the picture is labwc's (grim), as kwin-input.yml takes them: KWin's own capture isn't for nested KWin.
        if os.environ.get('OUTER'):
            r = subprocess.run(['grim', os.path.join(out, name + '.png')], env=dict(os.environ, WAYLAND_DISPLAY=os.environ['OUTER']),
                               capture_output=True, text=True, timeout=20)
            if r.returncode:
                err('picture ' + name, r.stderr[-300:])
            return
        try:
            i = dbus.Interface(bus.get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
            r, w = os.pipe()
            res = i.CaptureWorkspace({'native-resolution': True}, dbus.types.UnixFd(w), timeout=15)
            os.close(w)
            data = b''
            while True:
                c = os.read(r, 1 << 20)
                if not c:
                    break
                data += c
            W, H, S = int(res['width']), int(res['height']), int(res['stride'])
            raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
            Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(os.path.join(out, name + '.png'))
        except Exception as e:
            err('picture ' + name, repr(e))

    def window(wid, title, text):
        p = subprocess.Popen(['python3', '/tmp/padwin.py', 'id=' + wid, 'title=' + title, 'text=' + text, 'fullscreen=1'],
                             env=dict(os.environ, GDK_BACKEND='wayland'), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(3.5)
        return p

    def close(p):
        p.terminate()
        p.wait(5)
        time.sleep(1.0)

    host = subprocess.Popen([os.path.join(bin_dir, 'hearth-pad-host')], env=dict(os.environ, HEARTH_PAD_SIM=sim),
                            stdout=subprocess.DEVNULL, stderr=open(host_log, 'w'))
    time.sleep(6)
    im = subprocess.run(['pgrep', '-af', 'libexec/hearth/hearth-kwin-im( |$)'], capture_output=True, text=True).stdout.strip()
    note('touch session', 'hearth-kwin-im: %s · KWin: %s · labwc input: %s' % (im or 'NOT running', vk(),
         ' | '.join(l for l in tail(os.path.join(out, 'labwc-touch.log'), 20000).splitlines() if 'Hearth check' in l)[-400:]))
    # The field in a fullscreen test window sits at y 88 (nothing above it). The mouse first, with no controller use.
    mw = window('org.hearth.PadMouse', 'Mouse', 'A text field clicked with a real mouse.')
    # The first click may only give KWin's window labwc's focus; the second is the one in the field.
    dev('click 600 88')
    time.sleep(1.0)
    dev('click 600 88')
    time.sleep(2.0)
    v = vk()
    note('mouse click on a text field (real mouse)', 'KWin %s · keyboard %s' % (v, 'OPEN' if kb_open() else 'not open'))
    shot('pad-k34-mouse-click-text-field')
    if v.get('active') not in ('1', 'True'):
        err('mouse click', "the click didn't reach the text field (KWin %s), so it proves nothing" % v)
    if kb_open():
        err('mouse click', 'a mouse click on a text field opened the keyboard (only touch and the controller may)')
    close(mw)
    tw = window('org.hearth.PadTouch', 'Touch', 'A text field touched with a finger (a real touchscreen).')
    dev('tap 600 88')
    time.sleep(2.5)
    v = vk()
    note('touch on a text field (real touchscreen)', 'KWin %s · keyboard %s · pad.log %s' % (v, 'open' if kb_open() else 'NOT open', tail(pad_log, 300)))
    shot('pad-k35-touch-text-field')
    if not kb_open():
        err('touch', "touching a text field didn't open Hearth's keyboard: KWin %s" % v)
    # B closes it, and it stays closed (KWin shows its panel again as the window gets its room back).
    press('b')
    time.sleep(4.0)
    note('closed after touch', 'keyboard %s · host: %s' % ('OPEN again' if kb_open() else 'closed', tail(host_log, 300)))
    shot('pad-k36-touch-keyboard-closed')
    if kb_open():
        err('close', 'the keyboard came back by itself after B closed it')
    close(tw)
    time.sleep(14)
    a = sum(int(x) for x in open('/proc/%d/stat' % host.pid).read().rsplit(')', 1)[1].split()[11:13])
    time.sleep(15)
    b = sum(int(x) for x in open('/proc/%d/stat' % host.pid).read().rsplit(')', 1)[1].split()[11:13])
    c = 100.0 * (b - a) / os.sysconf('SC_CLK_TCK') / 15
    note('pad at rest (touch session)', '%.2f%% of a core' % c)
    if c > 0.3:
        err('pad at rest (touch session)', '%.2f%% of a core at rest' % c)
    host.terminate()
    return 1 if bad else 0


if __name__ == '__main__':
    if sys.argv[1] == 'devices':
        devices(sys.argv[2])
    else:
        sys.exit(test(sys.argv[2], sys.argv[3], sys.argv[4]))
