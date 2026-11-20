#!/usr/bin/python3
"""KWin's input method on the Fedora edition, in a real Plasma session that plasma-pad.yml has just started:
  plasma-pad-fallback.py OUT CASE
CASE hearth: Hearth's typing (hearth-kwin-im) is KWin's input method, KDE's keyboard isn't running.
CASE missing | crash | noreg: hearth-kwin-im isn't there / crashes at once / doesn't get the input-method protocol:
  KWin and Plasma came up anyway and KDE's keyboard (plasma-keyboard) is KWin's input method, by itself.
CASE user: the person picked KDE's keyboard in System Settings (their ~/.config/kwinrc): that's what runs, and their
  file is exactly as it was.
Prints ::error:: for what's wrong (exit 1), and the findings into OUT/results.txt; a picture OUT/pad-k4N-CASE.png."""
import os
import subprocess
import sys
import time

OUT, CASE = os.path.abspath(sys.argv[1]), sys.argv[2]
SHOTS = {'hearth': 'pad-k40-im-hearth', 'missing': 'pad-k41-fallback-missing', 'crash': 'pad-k42-fallback-crash',
         'noreg': 'pad-k43-fallback-no-protocol', 'user': 'pad-k44-user-choice-kept'}
bad = []


def note(t, m):
    with open(os.path.join(OUT, 'results.txt'), 'a') as f:
        f.write('::notice title=%s::%s\n' % (t, str(m).replace('\n', '|').replace('%', '%25')[:2500]))


def err(t, m):
    bad.append(t)
    print('::error title=%s::%s' % (t, str(m).replace('\n', '|')[:2500]), flush=True)


def procs(pat):
    r = subprocess.run(['pgrep', '-af', pat], capture_output=True, text=True)
    return [ln for ln in r.stdout.splitlines() if 'pgrep' not in ln and 'plasma-pad-fallback' not in ln]


import dbus  # noqa: E402
from PIL import Image  # noqa: E402

bus = dbus.SessionBus()
time.sleep(8)
try:
    p = dbus.Interface(bus.get_object('org.kde.KWin', '/VirtualKeyboard'), 'org.freedesktop.DBus.Properties')
    vk = {str(k): str(v) for k, v in p.GetAll('org.kde.kwin.VirtualKeyboard').items()}
except Exception as e:
    vk = {'error': repr(e)}
hearth = procs('libexec/hearth/hearth-kwin-im$')
kde = procs('plasma-keyboard')
plasma = procs('plasmashell')
log = ''
try:
    log = open(os.path.join(os.environ['HOME'], '.local/share/nimbus/pad.log')).read()[-800:]
except OSError:
    pass
note('input method (%s)' % CASE, 'KWin: %s · Hearth: %s · KDE keyboard: %s · plasmashell: %d · launcher: %s' % (
    vk, hearth or 'not running', kde or 'not running', len(plasma), ' | '.join(l for l in log.splitlines() if 'kwin-im-launch' in l)[-500:]))
if not plasma:
    err('Plasma (%s)' % CASE, "plasmashell isn't running")
if vk.get('available') != '1' and vk.get('available') != 'True':
    err('input method (%s)' % CASE, "KWin has no input method running: %s" % vk)
if CASE == 'hearth':
    if not hearth:
        err('input method (hearth)', "hearth-kwin-im isn't running as KWin's input method")
    if kde:
        err('input method (hearth)', 'KDE keyboard runs too: %s' % kde)
else:
    if not kde:
        err('input method (%s)' % CASE, "KDE's keyboard didn't take over: %s" % log[-600:])
    if hearth:
        err('input method (%s)' % CASE, 'hearth-kwin-im still runs: %s' % hearth)
if CASE == 'user':
    # KWin itself keeps other things in the same file (its desktops' ids, tiling): what must stay is the person's
    # virtual-keyboard choice, every line of it, and nothing of Hearth's may appear in it.
    text = open(os.path.join(os.environ['HOME'], '.config/kwinrc')).read()
    before = open('/tmp/user-kwinrc.txt').read()
    kept = all(ln in text.splitlines() for ln in before.splitlines() if ln.strip())
    note('your own setting (user)', ('kept: ' if kept else 'CHANGED: ') + text[:900])
    if not kept or 'hearth' in text.lower():
        err('your own setting', '~/.config/kwinrc lost the virtual-keyboard choice or got Hearth in it: %r' % text[:600])
# A window with a focused text field, so the picture shows the desktop working with that input method.
w = subprocess.Popen(['python3', '/tmp/padwin.py', 'id=org.hearth.PadFallback', 'title=Typing (%s)' % CASE, 'focus=1',
                      'text=KWin input method: %s' % ('Hearth typing' if hearth else "KDE's keyboard" if kde else 'none')],
                     env=dict(os.environ, GDK_BACKEND='wayland'), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(4)
try:
    i = dbus.Interface(bus.get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
    r, wfd = os.pipe()
    res = i.CaptureWorkspace({'native-resolution': True}, dbus.types.UnixFd(wfd), timeout=15)
    os.close(wfd)
    data = b''
    while True:
        c = os.read(r, 1 << 20)
        if not c:
            break
        data += c
    W, H, S = int(res['width']), int(res['height']), int(res['stride'])
    raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
    Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(os.path.join(OUT, SHOTS[CASE] + '.png'))
except Exception as e:
    err('picture %s' % CASE, repr(e))
w.terminate()
sys.exit(1 if bad else 0)
