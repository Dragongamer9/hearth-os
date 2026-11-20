#!/usr/bin/python3
"""Touch users keep their pop-up keyboard (plasma-pad.yml): in a fresh Plasma session (nothing touched yet, so KWin's
last input is no leftover touch), with KWin's own pointer and touch (hearth-kwin-cast input, KWin tells a pointer
event from a touch event by the kind of event, as for a real mouse and touchscreen):
  plasma-pad-touch.py test OUT BIN        a mouse click on a text field doesn't open Hearth's keyboard, a touch tap
                                          does, B closes it and it stays closed, nothing at rest
Hearth's typing (hearth-kwin-im --panel) is KWin's input method; KWin shows its panel only for touch, and hearth-pad
--plasma opens Hearth's keyboard then (pad.log: "a text field was touched")."""
import json
import os
import subprocess
import sys
import time


def test(out, bin_dir):
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

    fake = subprocess.Popen(['python3', '/usr/local/lib/padtest/hearth-portal'], stdin=subprocess.PIPE, text=True,
                            stdout=open(os.path.join(out, 'fake-input-touch.log'), 'w'), stderr=subprocess.STDOUT)
    time.sleep(1.0)

    def dev(line):
        x, y = line.split()[1:]
        if line.startswith('click'):
            seq = ['a %s %s' % (x, y), 'b 272 1', 'b 272 0']
        elif line.startswith('move'):
            seq = ['a %s %s' % (x, y)]
        else:
            seq = ['td 3 %s %s' % (x, y), 'tf', 'tu 3', 'tf']
        for ln in seq:
            fake.stdin.write(ln + '\n')
            fake.stdin.flush()
            time.sleep(0.1)
        time.sleep(0.4)

    def press(b):
        with open(os.path.join(sim, 'in'), 'a') as f:
            f.write(json.dumps({'b': b, 'down': True}) + '\n')
        time.sleep(0.12)
        with open(os.path.join(sim, 'in'), 'a') as f:
            f.write(json.dumps({'b': b, 'down': False}) + '\n')
        time.sleep(0.4)

    def shot(name):
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

    def window(wid, title, text, mode='max'):
        p = subprocess.Popen(['python3', '/tmp/padwin.py', 'id=' + wid, 'title=' + title, 'text=' + text, ('fullscreen=1' if mode == 'full' else 'max=1')],
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
    note('touch session', 'hearth-kwin-im: %s · KWin: %s' % (im or 'NOT running', vk()))
    # The field in a maximized test window sits at y 117 (KDE's own panel at the bottom: a fresh Plasma). The mouse first, with no controller use.
    mw = window('org.hearth.PadMouse', 'Mouse', 'A text field clicked with the pointer (KWin\'s own pointer events).')
    dev('click 600 117')
    time.sleep(2.0)
    v = vk()
    note('mouse click on a text field (pointer)', 'KWin %s · keyboard %s' % (v, 'OPEN' if kb_open() else 'not open'))
    shot('pad-k34-mouse-click-text-field')
    if v.get('active') not in ('1', 'True'):
        err('mouse click', "the click didn't reach the text field (KWin %s), so it proves nothing" % v)
    if kb_open():
        err('mouse click', 'a mouse click on a text field opened the keyboard (only touch and the controller may)')
    close(mw)
    tw = window('org.hearth.PadTouch', 'Touch', 'A text field touched (KWin\'s own touch events).')
    dev('tap 600 117')
    time.sleep(2.5)
    v = vk()
    note('touch on a text field (touch)', 'KWin %s · keyboard %s · pad.log %s' % (v, 'open' if kb_open() else 'NOT open', tail(pad_log, 300)))
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
    # Over a fullscreen window (a fullscreen browser): no touch, no keyboard; a touch tap opens it. The field of a
    # fullscreen test window sits at y 88.
    time.sleep(4)
    # The pointer moved last (the touch above is behind us), as when you use the mouse after touching.
    dev('move 900 600')
    fw = window('org.hearth.PadFullMouse', 'Fullscreen', 'A fullscreen window: its field clicked with the pointer.', 'full')
    dev('click 600 88')
    time.sleep(2.0)
    v = vk()
    note('fullscreen, pointer click', 'KWin %s · keyboard %s' % (v, 'OPEN' if kb_open() else 'not open'))
    shot('pad-k37-fullscreen-click-no-keyboard')
    if v.get('active') not in ('1', 'True'):
        err('fullscreen click', "the click didn't reach the fullscreen window's field (KWin %s)" % v)
    if kb_open():
        err('fullscreen click', 'the keyboard opened over a fullscreen window without a touch')
    close(fw)
    fw = window('org.hearth.PadFullTouch', 'Fullscreen', 'A fullscreen window: its field touched.', 'full')
    dev('tap 600 88')
    time.sleep(2.5)
    v = vk()
    note('fullscreen, touch', 'KWin %s · keyboard %s' % (v, 'open' if kb_open() else 'NOT open'))
    shot('pad-k38-fullscreen-touch-keyboard')
    if not kb_open():
        err('fullscreen touch', "touching a fullscreen window's text field didn't open Hearth's keyboard: KWin %s" % v)
    press('b')
    close(fw)
    time.sleep(14)
    a = sum(int(x) for x in open('/proc/%d/stat' % host.pid).read().rsplit(')', 1)[1].split()[11:13])
    time.sleep(15)
    b = sum(int(x) for x in open('/proc/%d/stat' % host.pid).read().rsplit(')', 1)[1].split()[11:13])
    c = 100.0 * (b - a) / os.sysconf('SC_CLK_TCK') / 15
    note('pad at rest (touch session)', '%.2f%% of a core' % c)
    if c > 0.3:
        err('pad at rest (touch session)', '%.2f%% of a core at rest' % c)
    host.terminate()
    fake.terminate()
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(test(sys.argv[2], sys.argv[3]))
