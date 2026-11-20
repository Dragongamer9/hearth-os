#!/usr/bin/python3
"""The controller's overlays (hearth-pad-host, docs/mockups/checklists/plasma-pad.md) in a real KDE Plasma session:
KDE's KWin on a virtual 1920x1080 screen with plasmashell and its panel, real windows (GTK test windows standing in
for an app with a text field, KDE's password prompt, a controller app and a fullscreen game), and hearth-pad --plasma
started by the host the way it is on a PC, with a simulated controller (HEARTH_PAD_SIM: buttons and sticks written to
a file, everything else real: KWin's hearthpad script, org.hearth.Pad, KWin's text-field signal). Touch and the mouse
are KWin's own fake input (hearth-kwin-cast input). Pictures are KWin's (ScreenShot2) in OUT; checks are ::error::.
  plasma-pad-e2e.py OUT BIN"""
import json
import os
import subprocess
import sys
import time

OUT, BIN = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
HOST = os.path.join(BIN, 'hearth-pad-host')
SIM = '/tmp/padsim'
GEO = '/tmp/padgeo.json'
os.makedirs(OUT, exist_ok=True)
FAILED = []


RESULTS = os.path.join(OUT, 'results.txt')


def notice(t, m):
    # GitHub keeps ten notices a step: they're kept in OUT/results.txt and posted by later steps, ten each.
    with open(RESULTS, 'a') as f:
        f.write('::notice title=%s::%s\n' % (t, str(m).replace('\n', '|').replace('%', '%25')[:3000]))


def error(t, m):
    FAILED.append(t)
    print('::error title=%s::%s' % (t, str(m).replace('\n', '|').replace('%', '%25')[:3500]), flush=True)


def tail(p, n=1500):
    try:
        with open(p, errors='replace') as f:
            return f.read()[-n:]
    except OSError:
        return ''


import dbus  # noqa: E402
from PIL import Image  # noqa: E402

BUS = dbus.SessionBus()


def shot(name):
    try:
        i = dbus.Interface(BUS.get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
        r, w = os.pipe()
        res = i.CaptureWorkspace({'native-resolution': True}, dbus.types.UnixFd(w), timeout=15)
        os.close(w)
        data = b''
        while True:
            c = os.read(r, 1 << 20)
            if not c:
                break
            data += c
        os.close(r)
        W, H, S = int(res['width']), int(res['height']), int(res['stride'])
        raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
        Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(os.path.join(OUT, name + '.png'))
    except Exception as e:  # a missing picture is said, the run goes on
        error('picture ' + name, repr(e))


def tablet():
    """KWin's touch (tablet) mode: in it KDE shows its keyboard on every text field, as KWin does Hearth's panel."""
    try:
        p = dbus.Interface(BUS.get_object('org.kde.KWin', '/org/kde/KWin'), 'org.freedesktop.DBus.Properties')
        return {str(k): str(v) for k, v in p.GetAll('org.kde.KWin.TabletModeManager').items()}
    except Exception as e:
        return {'error': repr(e)[:120]}


def vk():
    """KWin's virtual keyboard state (what tells hearth-pad a text field has the focus)."""
    try:
        p = dbus.Interface(BUS.get_object('org.kde.KWin', '/VirtualKeyboard'), 'org.freedesktop.DBus.Properties')
        return {str(k): bool(v) if isinstance(v, dbus.Boolean) else str(v) for k, v in p.GetAll('org.kde.kwin.VirtualKeyboard').items()}
    except Exception as e:
        return {'error': repr(e)}


# ---------------------------------------------------------------------- the controller (HEARTH_PAD_SIM) and touch

def sim(obj):
    with open(os.path.join(SIM, 'in'), 'a') as f:
        f.write(json.dumps(obj) + '\n')


def press(b, hold=0.12, after=0.35):
    sim({'b': b, 'down': True})
    time.sleep(hold)
    sim({'b': b, 'down': False})
    time.sleep(after)


def axes(lx=0.0, ly=0.0, rx=0.0, ry=0.0):
    sim({'axes': [lx, ly, rx, ry]})


FAKE = None


def fake(*lines):
    """KWin's fake input: 'a X Y' (pointer to), 'b 272 1' (left button), 'td 0 X Y' / 'tm' / 'tu 0' / 'tf' (touch)."""
    global FAKE
    if FAKE is None or FAKE.poll() is not None:
        FAKE = subprocess.Popen(['python3', '/usr/local/lib/padtest/hearth-portal'], stdin=subprocess.PIPE, stdout=open(os.path.join(OUT, 'fake-input.log'), 'a'),
                                stderr=subprocess.STDOUT, text=True)
        time.sleep(1.0)
    for ln in lines:
        FAKE.stdin.write(ln + '\n')
    FAKE.stdin.flush()


def click(x, y):
    fake('a %d %d' % (x, y))
    time.sleep(0.15)
    fake('b 272 1')
    time.sleep(0.08)
    fake('b 272 0')


def kwin_windows():
    """KWin's windows now (layer-shell overlays too): [class, caption, x, y, w, h], read back from KWin's log."""
    tag = 'PADWIN%d' % int(time.time() * 1000)
    with open('/tmp/padwins.js', 'w') as f:
        f.write('var ws = workspace.stackingOrder; for (var i = 0; i < ws.length; i++) { var w = ws[i], g = w.frameGeometry; '
                'print("%s " + JSON.stringify([String(w.resourceClass), String(w.caption), g.x, g.y, g.width, g.height])); }' % tag)
    try:
        sc = dbus.Interface(BUS.get_object('org.kde.KWin', '/Scripting'), 'org.kde.kwin.Scripting')
        sid = int(sc.loadScript('/tmp/padwins.js', tag, signature='ss'))
        dbus.Interface(BUS.get_object('org.kde.KWin', '/Scripting/Script%d' % sid), 'org.kde.kwin.Script').run()
        time.sleep(1.0)
        sc.unloadScript(tag)
    except Exception as e:
        return [['error', repr(e), 0, 0, 0, 0]]
    out = []
    for ln in tail(os.path.join(OUT, 'kwin.log'), 200000).splitlines():
        if tag + ' ' in ln:
            try:
                out.append(json.loads(ln.split(tag + ' ', 1)[1].strip().rstrip('"')))
            except ValueError:
                pass
    return out


# ---------------------------------------------------------------------- windows

WIN = r'''
import sys, gi
gi.require_version('Gtk', '3.0')
from gi.repository import GLib
GLib.set_prgname(dict(x.split('=', 1) for x in sys.argv[1:])['id'])
from gi.repository import Gtk, Gio, GLib
a = dict(x.split('=', 1) for x in sys.argv[1:])
GLib.set_prgname(a['id'])   # the Wayland app id KWin reports as the window's class
app = Gtk.Application(application_id=a['id'], flags=Gio.ApplicationFlags.NON_UNIQUE)
def up(app):
    w = Gtk.ApplicationWindow(application=app, title=a.get('title', 'Test'))
    w.set_default_size(900, 600)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14, margin=40)
    lab = Gtk.Label(label=a.get('text', ''))
    lab.set_line_wrap(True)
    box.pack_start(lab, False, False, 0)
    e = Gtk.Entry()
    e.set_placeholder_text(a.get('hint', 'Type here'))
    if a.get('password') == '1':
        e.set_visibility(False)
        e.set_input_purpose(Gtk.InputPurpose.PASSWORD)
    box.pack_start(e, False, False, 0)
    # What's in the field, for the checks (what typing reached the app).
    e.connect('changed', lambda x: open('/tmp/entry-%s.txt' % a['id'], 'w').write(x.get_text()))
    w.add(box)
    if a.get('fullscreen') == '1':
        w.fullscreen()
    elif a.get('max') == '1':
        w.maximize()
    w.show_all()
    if a.get('focus') != '1':
        w.set_focus(None)
app.connect('activate', up)
app.run([])
'''
with open('/tmp/padwin.py', 'w') as f:
    f.write(WIN)
WINS = []


def window(**kw):
    env = dict(os.environ, GDK_BACKEND='wayland')
    p = subprocess.Popen(['python3', '/tmp/padwin.py'] + ['%s=%s' % kv for kv in kw.items()], env=env,
                         stdout=open(os.path.join(OUT, 'windows.log'), 'a'), stderr=subprocess.STDOUT)
    WINS.append(p)
    time.sleep(3.0)
    return p


def close(p):
    p.terminate()
    try:
        p.wait(5)
    except subprocess.TimeoutExpired:
        p.kill()
    time.sleep(1.0)


def ticks(pid):
    try:
        with open('/proc/%d/stat' % pid) as f:
            v = f.read().rsplit(')', 1)[1].split()
        return int(v[11]) + int(v[12])
    except (OSError, IndexError, ValueError):
        return 0


def cpu(pid, secs):
    a = ticks(pid)
    time.sleep(secs)
    b = ticks(pid)
    return 100.0 * (b - a) / os.sysconf('SC_CLK_TCK') / secs


# ---------------------------------------------------------------------- 1. the motion, held still (--show --at)

desk = window(id='org.hearth.PadTest', title='Notes', text='An app in KDE Plasma, maximized: the keyboard takes its own space at the bottom.', max='1')


def show(name, *args, wait=2.2):
    log = os.path.join(OUT, name + '.log')
    p = subprocess.Popen([HOST] + list(args), stdout=subprocess.DEVNULL, stderr=open(log, 'w'))
    time.sleep(wait)
    if p.poll() is not None:
        error(name, 'it ended by itself: ' + tail(log))
    shot(name)
    p.terminate()
    try:
        p.wait(5)
    except subprocess.TimeoutExpired:
        p.kill()
    time.sleep(0.5)


show('pad-k01-menu-opening-40ms', '--show', 'menu', '--at', '40')
show('pad-k02-menu-opening-120ms', '--show', 'menu', '--at', '120')
show('pad-k03-menu-open', '--show', 'menu', '--at', '400')
show('pad-k04-menu-closing-90ms', '--show', 'menu-closing', '--at', '90')
show('pad-k05-menu-closing-200ms', '--show', 'menu-closing', '--at', '200')
show('pad-k06-ember-glide-60ms', '--show', 'glide', '--at', '60')
show('pad-k07-ember-glide-110ms', '--show', 'glide', '--at', '110')
show('pad-k08-ember-warmth-low', '--show', 'keyboard', '--at', '2400')
show('pad-k09-ember-warmth-high', '--show', 'keyboard', '--at', '1100')
show('pad-k10-password-mode', '--show', 'password')
show('pad-k11-long-touch', '--show', 'longtouch', wait=2.6)

# ---------------------------------------------------------------------- 2. the real thing: hearth-pad, KWin, windows

os.makedirs(SIM, exist_ok=True)
open(os.path.join(SIM, 'in'), 'w').close()
host_log = os.path.join(OUT, 'host.log')
env = dict(os.environ, HEARTH_PAD_SIM=SIM, HEARTH_PAD_GEOMETRY=GEO)
host = subprocess.Popen([HOST], env=env, stdout=subprocess.DEVNULL, stderr=open(host_log, 'w'))
time.sleep(6)
pad_log = os.path.join(os.environ['HOME'], '.local/share/nimbus/pad.log')
if host.poll() is not None:
    error('host', 'hearth-pad-host ended: ' + tail(host_log))
notice('hearth-pad', tail(pad_log, 900))
out_log = os.path.join(SIM, 'out.log')


def typed_since(n):
    try:
        with open(out_log) as f:
            return f.read()[n:]
    except OSError:
        return ''


def out_len():
    try:
        return os.path.getsize(out_log)
    except OSError:
        return 0


# The Hearth menu with Guide, over a real Plasma desktop and app; the stick lights Keyboard; A opens it.
press('guide', after=0.9)
shot('pad-k20-live-menu')
axes(0.7, -0.7)
time.sleep(0.5)
shot('pad-k21-live-menu-keyboard-lit')
press('a', after=0.2)
axes()
time.sleep(1.5)
shot('pad-k22-live-keyboard')
for _ in range(3):
    press('right', hold=0.08, after=0.12)
time.sleep(0.05)
shot('pad-k23-live-ember-moving')
time.sleep(1.5)
# Never lag: what the keyboard costs while it's open with the ember's warmth pulsing (and at rest further down).
c = cpu(host.pid, 10)
rss = 0
try:
    rss = int([ln.split()[1] for ln in open('/proc/%d/status' % host.pid) if ln.startswith('VmRSS')][0]) / 1024
except (OSError, IndexError, ValueError):
    pass
notice('pad keyboard open', 'ember pulsing: %.2f%% of a core, %.1f MB resident' % (c, rss))
if c > 8.0:
    error('pad keyboard open', '%.2f%% of a core with the keyboard open (more than 8%%)' % c)

# Left open and untouched for 30 s: the warmth holds still and nothing is drawn (never lag).
time.sleep(31)
c2 = cpu(host.pid, 10)
notice('pad keyboard open, left alone', '%.2f%% of a core' % c2)
if c2 > 0.5:
    error('pad keyboard open, left alone', '%.2f%% of a core with the keyboard open and nobody using it (limit 0.5%%)' % c2)

# A long touch on "e" (KWin's touch): its accents open after 480 ms; sliding onto the second and lifting types it.
try:
    geo = json.load(open(GEO))
except (OSError, ValueError) as e:
    geo = {}
    error('geometry', repr(e))
# Hearth's typing is KWin's input method (the image's /etc/xdg/kwinrc): what the keyboard types reaches the app whole.
im = subprocess.run(['pgrep', '-f', 'libexec/hearth/hearth-kwin-im( |$)'], capture_output=True, text=True).stdout.split()
notice('Hearth typing', 'hearth-kwin-im %s, socket %s' % (im or 'NOT running', os.path.exists(os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth', 'im.sock'))))
if not im:
    error('Hearth typing', "hearth-kwin-im isn't KWin's input method in this session")
ENTRY = '/tmp/entry-org.hearth.PadTest.txt'
click(500, 117)     # the Notes window's text field has the keyboard
time.sleep(1.0)


def entry():
    try:
        return open(ENTRY, encoding='utf-8').read()
    except OSError:
        return ''


if geo.get('keys', {}).get('e'):
    ex, ey = geo['keys']['e']
    top = 1080 - geo['h']
    ws = kwin_windows()
    notice('KWin windows', ws)
    for w in ws:
        if len(w) == 6 and abs(w[5] - geo['h']) <= 1 and w[4] >= 1000 and w[3] > 300:
            top = w[3]
    n0 = out_len()
    fake('td 0 %d %d' % (ex, top + ey), 'tf')
    time.sleep(0.9)
    shot('pad-k24-live-long-touch')
    try:
        geo = json.load(open(GEO))
    except (OSError, ValueError):
        pass
    alts = geo.get('alts') or []
    notice('long touch', 'accents: %s at %s' % (geo.get('popup'), alts[:4]))
    if len(alts) >= 2:
        ax, ay = alts[1]
        for k in range(1, 6):
            fake('tm 0 %d %d' % (ex + (ax - ex) * k / 5, top + ey + (ay - ey) * k / 5), 'tf')
            time.sleep(0.05)
        time.sleep(0.4)
        shot('pad-k25-live-long-touch-slide')
        fake('tu 0', 'tf')
        time.sleep(0.8)
        got = entry()
        notice('long touch typed', 'the app has %r' % got)
        if 'è' not in got:
            error('accent typed', 'the accent picked with a long touch (è) didn\'t reach the app: %r' % got)
        # A plain tap on "x" and on "ñ"-less keys still types (through Hearth's typing too).
        gx, gy = geo['keys'].get('x', (0, 0))
        if gx:
            fake('td 0 %d %d' % (gx, top + gy), 'tf')
            time.sleep(0.12)
            fake('tu 0', 'tf')
            time.sleep(0.8)
        notice('tap typed', 'the app has %r' % entry())
        if not entry().endswith('x'):
            error('tap typed', 'a tap on x didn\'t reach the app: %r' % entry())
        shot('pad-k25b-live-accent-in-app')
    else:
        error('long touch', 'no accents opened after a long touch on e (%s)' % tail(host_log, 400))
        fake('tu 0', 'tf')
else:
    error('long touch', 'no key positions (HEARTH_PAD_GEOMETRY): %s' % str(geo)[:300])
time.sleep(0.6)
shot('pad-k26-live-after-touch')
press('b', after=1.0)

# A text field gets the focus after the controller was used: the keyboard opens by itself (KWin's text-field signal).
notice('KWin virtual keyboard (before)', vk())
press('a', after=0.3)
click(500, 117)
time.sleep(2.0)
notice('KWin virtual keyboard (text field focused)', vk())
shot('pad-k27-live-text-field')
# And a window that comes up with its text field focused (a search window, a chat), right after the controller was used.
press('b', after=0.6)
press('a', after=0.2)
tf = window(id='org.hearth.PadSearch', title='Search', text='A window that opens with its text field focused.', focus='1')
time.sleep(1.5)
notice('KWin virtual keyboard (new window)', vk())
shot('pad-k27b-live-text-field-new-window')
close(tf)
notice('hearth-pad after the text field', tail(pad_log, 600))
press('b', after=0.8)

# KDE's password prompt in front after the controller was used: the keyboard opens in its password mode.
press('a', after=0.2)
pw = window(id='org.kde.polkit-kde-authentication-agent-1', title='Authentication Required', password='1', focus='1',
            text='Authentication is needed to change the system settings.', hint='Password')
time.sleep(1.5)
shot('pad-k28-live-password-prompt')
notice('hearth-pad with the password prompt', tail(pad_log, 500))
press('b', after=0.6)
close(pw)

# A controller app in front (it reads the controller itself): Guide still opens the Hearth menu.
ca = window(id='hearth-photos', title='Hearth Photos', text='A controller app: it reads the controller itself.', max='1')
time.sleep(1.0)
press('guide', after=1.0)
shot('pad-k29-live-over-controller-app')
press('b', after=0.8)
close(ca)

# A fullscreen game: the controller pauses (no pointer, no keys); Guide still opens the Hearth menu over it.
game = window(id='padtest-game', title='A Game', text='A fullscreen game: the controller pauses here.', fullscreen='1')
time.sleep(1.0)
for _ in range(20):          # KWin says it's fullscreen a moment after it's mapped
    if 'fullscreen: paused' in tail(pad_log, 400):
        break
    time.sleep(0.25)
ht = tail(host_log, 100000)
if ht.rfind('keyboard open') > ht.rfind('keyboard closed'):
    notice('fullscreen game', 'the keyboard was still open from before: closed with B first (%s)' % tail(pad_log, 200))
    press('b', after=0.8)
n0 = out_len()
press('a', after=0.2)
axes(0, 0, 1.0, 0)
time.sleep(0.6)
axes()
moved = typed_since(n0)
notice('fullscreen game', ('paused: nothing sent' if 'move' not in moved and 'BTN' not in moved else 'NOT paused: ' + moved[-300:]))
if 'move' in moved:
    error('fullscreen game', "the controller wasn't paused over a fullscreen game: " + moved[-300:])
press('guide', after=1.0)
shot('pad-k30-live-over-fullscreen-game')
press('b', after=0.8)
close(game)

# Quick Access from the Hearth menu: with KDE's own panel (no Hearth bar) it says where Quick Access comes from.
press('guide', after=0.6)
axes(1.0, 0)
time.sleep(0.4)
press('a', after=0.2)
axes()
time.sleep(1.2)
shot('pad-k31-live-quick-access-kde-panel')
notice('host after Quick Access (KDE panel)', tail(host_log, 400))

# Hearth's bar in (Settings › Bar and dock › Hearth's panel): the Hearth menu's Quick Access opens Hearth's Quick settings.
r = subprocess.run(['timeout', '150', 'hearth-plasma-panel', 'apply'], capture_output=True, text=True)
notice('Hearth panel apply', (r.stdout + r.stderr)[-600:])
time.sleep(12)
press('guide', after=0.6)
axes(1.0, 0)
time.sleep(0.4)
press('a', after=0.2)
axes()
time.sleep(3.0)
shot('pad-k32-live-quick-access')
notice('host after Quick Access (Hearth bar)', tail(host_log, 400))
# Closed again (Quick settings toggles; B hides the keyboard if a text field had opened it).
subprocess.run(['timeout', '20', 'hearth-plasma-panel', 'open', 'quick'], capture_output=True)
press('b', after=0.5)
time.sleep(1.0)
ws = kwin_windows()
open_now = [w for w in ws if len(w) == 6 and w[0] == 'hearth-pad-host']
notice('overlays open before the rest measurement', open_now or 'none')
if open_now:
    press('b', after=1.0)
shot('pad-k33-live-all-closed')


# At rest again (nothing open): nothing may run.
time.sleep(14)
c = cpu(host.pid, 15)
notice('pad at rest (live)', '%.2f%% of a core' % c)
for pid in im:
    ci = cpu(int(pid), 10)
    notice('hearth-kwin-im at rest', '%.2f%% of a core' % ci)
    if ci > 0.3:
        error('hearth-kwin-im at rest', '%.2f%% of a core at rest (limit 0.3%%)' % ci)
if c > 0.3:
    error('pad at rest (live)', '%.2f%% of a core at rest (limit 0.3%%)' % c)

notice('host log', tail(host_log, 1500))
notice('pad.log', tail(pad_log, 1500))
host.terminate()
for p in WINS:
    if p.poll() is None:
        p.terminate()
if FAKE is not None:
    FAKE.terminate()
sys.exit(1 if FAILED else 0)
