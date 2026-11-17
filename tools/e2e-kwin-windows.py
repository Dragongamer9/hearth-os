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


def title_matches(name, theme, label):
    """The title bar's colour in picture NAME (Test A in the top left quarter) is the theme's (decoration-top's fill)."""
    import re as _re
    try:
        from PIL import Image
        svg = open(os.path.join(theme, 'decoration.svg')).read()
        # decoration-center's colour (its first rect is only its size, see hearth_aurorae.frame_svg)
        center = _re.search(r'<g id="decoration-center">(.*?)</g>', svg).group(1)
        want = [f for f, o in _re.findall(r'<rect [^>]*?fill="(#[0-9a-f]{6})"( fill-opacity="0")?', center) if not o][0]
        w = tuple(int(want[i:i + 2], 16) for i in (1, 3, 5))
        im = Image.open(os.path.join(OUT, 'kwin-%s.png' % name)).convert('RGB')
        # The title bar is somewhere down the window's column (the bar's height differs between looks): look for it.
        col = [im.getpixel((300, y)) for y in range(30, 220)]
        near = [c for c in col if max(abs(a - b) for a, b in zip(w, c)) <= 8]
        got = near[0] if near else max(col, key=lambda c: -max(abs(a - b) for a, b in zip(w, c)))
        ok = len(near) >= 12             # a band at least 12 px tall in the theme's colour
        check('the title bar has the look\'s colour (%s)' % label, ok, 'theme %s, picture %s' % (want, '#%02x%02x%02x' % got))
    except Exception as ex:
        check('the title bar has the look\'s colour (%s)' % label, False, ex)


def set_look(theme, look, accent):
    """A look and accent as Settings › Appearance saves them; waits for the frames to be written again."""
    path = os.path.join(theme, 'decoration.svg')
    before = open(path).read() if os.path.isfile(path) else ''
    desk({'look': look, 'accent': accent})
    changed = wait(lambda: os.path.isfile(path) and open(path).read() != before, 20)
    ipc('browserpick', 'close')          # a first-run prompt the shell may show when its settings reload
    time.sleep(6)                        # the shell's own look change settles and KWin draws the new frames
    return changed


def bridge_info():
    """KWin's own account of its decoration (supportInformation's Decoration section) and the screen size."""
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    text = bus.call_sync('org.kde.KWin', '/KWin', 'org.kde.KWin', 'supportInformation', None, GLib.VariantType.new('(s)'),
                         Gio.DBusCallFlags.NONE, 10000, None).unpack()[0]
    keep, on = [], False
    for line in text.splitlines():
        if line.strip().lower().startswith('decoration'):
            on = True
        elif on and line and not line.startswith(' ') and ':' not in line:
            on = False
        if on or 'Geometry:' in line or 'theme' in line.lower() or 'plugin' in line.lower():
            keep.append(line.strip())
    return ' | '.join(keep)[:3000]


def frames_check(procs):
    """Hearth's window frames on KWin (theme/hearth_aurorae.py): written for the look, picked by Hearth's kwinrc,
    following a look change live; apps that draw their own title bar get no second frame."""
    theme = os.path.join(os.environ['HOME'], '.local', 'share', 'aurorae', 'themes', 'hearth')
    try:
        info = bridge_info()
        log('::notice title=KWin decoration::%s' % info)
    except Exception as ex:
        log('::warning title=KWin decoration::%s' % ex)
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
    set_look(theme, 'steam', '')
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: (win('Test A') or {}).get('zone') == 'top-left' and listed().get('active') == (win('Test A') or {}).get('id'))
    time.sleep(1.5)
    shot('frame-full')
    title_matches('frame-full', theme, 'Steam-style, the default')
    crop('frame-full', (0, 0, 640, 140), 'frame-zoom')
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
    fr = (win('Test A') or {}).get('frame') or {}
    sw = size[0] if isinstance(size, tuple) else 1920
    check('a maximized window has no gap (edge to edge, no shadow room)', fr.get('x', 99) <= 0 and fr.get('w', 0) >= sw, (fr, sw))
    ipc('winlist', 'act', 'Test A', 'restore')
    wait(lambda: (win('Test A') or {}).get('maximized') is False)
    ipc('winlist', 'act', 'Test A', 'activate')
    # Another look and accent (Hearth, gold): the frames follow without a restart.
    changed = set_look(theme, 'hearth', '#ffd166')
    check('a look change writes new frames', changed, 'decoration.svg unchanged')
    ipc('winlist', 'act', 'Test A', 'activate')
    time.sleep(1.5)
    shot('frame-full-4')
    title_matches('frame-full-4', theme, 'Hearth, gold')
    many, size = crop('frame-full-4', (0, 30, 1000, 230), 'frame-hearth-gold')
    check('the frame picture (Hearth look, gold) is more than one colour', many, size)
    desk({'look': 'steam', 'accent': ''})
    shadow_check(theme)


def luma(im, x, y, r=2):
    """The mean brightness of a (2r+1)-square at (x, y)."""
    px = [im.getpixel((min(max(i, 0), im.width - 1), min(max(j, 0), im.height - 1)))
          for i in range(x - r, x + r + 1) for j in range(y - r, y + r + 1)]
    return sum(0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2] for p in px) / len(px)


def shadow_check(theme):
    """The frames' drop shadow (hearth_aurorae SHADOW, as labwc's): Test A on its own in the middle on a light and a
    dark wallpaper; just outside the frame (left side and below) is darker than the wallpaper further out, and the
    window behind (inactive) has a lighter shadow than the one in front."""
    from PIL import Image
    walls = {}
    for name, colour in (('light', (216, 220, 224)), ('dark', (30, 42, 58))):
        walls[name] = os.path.join('/tmp', 'hearth-wall-%s.png' % name)
        Image.new('RGB', (1920, 1080), colour).save(walls[name])
    ipc('winlist', 'act', 'Test B', 'minimize')
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'center'}))
    wait(lambda: (win('Test A') or {}).get('zone') == 'center')
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: listed().get('active') == (win('Test A') or {}).get('id'))
    darkness = {}
    for name in ('light', 'dark'):
        desk({'wallpaper': walls[name]})
        time.sleep(4)                    # the shell loads the picture
        ipc('winlist', 'act', 'Test A', 'activate')
        time.sleep(1.5)
        fr = (win('Test A') or {}).get('frame') or {}
        shot('frame-shadow-%s-full' % name)
        try:
            im = Image.open(os.path.join(OUT, 'kwin-frame-shadow-%s-full.png' % name)).convert('RGB')
            x, y, w, h = fr['x'], fr['y'], fr['w'], fr['h']
            crop('frame-shadow-%s-full' % name, (x - 90, y - 70, x + 560, y + 230), 'frame-shadow-%s' % name)
            crop('frame-shadow-%s-full' % name, (x + w - 300, y + h - 160, x + w + 90, y + h + 90), 'frame-shadow-%s-corner' % name)
            # Left of the frame at half height, and just below it a quarter across (clear of the dock), each 4 px out,
            # against the wallpaper 80 px left of the frame at the same height (below the frame there's no room: the
            # middle zone ends near the work area's bottom).
            left_in, left_out = luma(im, x - 5, y + h // 2), luma(im, x - 80, y + h // 2)
            below_in, below_out = luma(im, x + w // 4, y + h + 4), luma(im, x - 80, y + h + 4)
            need = 12 if name == 'light' else 3
            ok = left_out - left_in >= need and below_out - below_in >= need
            darkness[name] = (left_out - left_in, below_out - below_in)
            check('the frame has a shadow on a %s wallpaper (just outside darker than the wallpaper)' % name, ok,
                  'left %.0f vs %.0f, below %.0f vs %.0f (frame %s)' % (left_in, left_out, below_in, below_out, fr))
        except Exception as ex:
            check('the frame has a shadow on a %s wallpaper (just outside darker than the wallpaper)' % name, False, '%s (frame %s)' % (ex, fr))
    # Behind another window: the lighter shadow (labwc's 40 px at 25 %), on the light wallpaper.
    desk({'wallpaper': walls['light']})
    time.sleep(4)
    ipc('winlist', 'act', 'Test B', 'activate')
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test B', 'zone': 'bottom-right'}))
    wait(lambda: listed().get('active') == (win('Test B') or {}).get('id'))
    time.sleep(1.5)
    fr = (win('Test A') or {}).get('frame') or {}
    shot('frame-shadow-inactive-full')
    try:
        im = Image.open(os.path.join(OUT, 'kwin-frame-shadow-inactive-full.png')).convert('RGB')
        x, y, h = fr['x'], fr['y'], fr['h']
        crop('frame-shadow-inactive-full', (x - 90, y - 70, x + 560, y + 230), 'frame-shadow-inactive')
        d = luma(im, x - 80, y + h // 2) - luma(im, x - 5, y + h // 2)
        act = (darkness.get('light') or (0, 0))[0]
        check('a window behind has a lighter shadow than the one in front', 3 <= d < act, 'behind %.0f, in front %.0f' % (d, act))
    except Exception as ex:
        check('a window behind has a lighter shadow than the one in front', False, '%s (frame %s)' % (ex, fr))
    desk({'wallpaper': ''})


def differs(a, b):
    """How different two of the pictures are (0..255 on average); -1 if either is missing."""
    try:
        from PIL import Image, ImageChops, ImageStat
        x = Image.open(os.path.join(OUT, 'kwin-%s.png' % a)).convert('L')
        y = Image.open(os.path.join(OUT, 'kwin-%s.png' % b)).convert('L').resize(x.size)
        return ImageStat.Stat(ImageChops.difference(x, y)).mean[0]
    except Exception:
        return -1


def extras_check(procs):
    """The KDE extras on KWin (feature/kwin-window-extras): live previews, Hearth's Alt+Tab, Snap layouts from resting on
    Maximize, force closing a frozen window, hiding a window from screen sharing, KWin's Overview / Present windows /
    Desktop grid opened from Mantel, and the keys in Settings › Keyboard. A picture of each."""
    try:
        import evdev
        from evdev import ecodes as e
        kbd = evdev.UInput({e.EV_KEY: [e.KEY_LEFTALT, e.KEY_TAB, e.KEY_ESC, e.KEY_LEFTMETA, e.KEY_W, e.KEY_DOWN, e.KEY_RIGHT, e.KEY_UP]}, name='Hearth test keyboard 2')
        time.sleep(2)
    except Exception as ex:
        kbd = None
        log('::warning title=KWin extras::no virtual keyboard: %s' % ex)

    def key(*names, hold=0.08):
        if not kbd:
            return
        for n in names:
            kbd.write(e.EV_KEY, getattr(e, n), 1); kbd.syn(); time.sleep(0.03)
        time.sleep(hold)
        for n in reversed(names):
            kbd.write(e.EV_KEY, getattr(e, n), 0); kbd.syn(); time.sleep(0.03)
        time.sleep(0.5)

    ipc('browserpick', 'close')          # the first-run "Pick your browser" card would cover the pictures
    for t in ('Test A', 'Test B'):
        ipc('winlist', 'act', t, 'activate')
        wait(lambda: (win(t) or {}).get('minimized') is False, 5)
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test A', 'zone': 'left-half'}))
    ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test B', 'zone': 'right-half'}))
    wait(lambda: (win('Test A') or {}).get('zone') == 'left-half' and (win('Test B') or {}).get('zone') == 'right-half')
    time.sleep(1)
    shot('extras-before')
    blog = bridge_log()
    check('the script follows the pointer (resting on Maximize can open Snap layouts)', 'no cursorPosChanged' not in blog, blog[-300:])

    # ---- live previews (KWin's previews script, above the dock as the dock asks)
    ready = wait(lambda: state().get('previews'), 20)
    check('KWin\'s previews script is loaded and asking the bridge', ready, [l for l in bridge_log().splitlines() if 'preview' in l.lower()][-3:])
    ipc('winlist', 'previewAt', '*', 960, 960)
    time.sleep(2)
    log('::notice title=KWin previews::%s' % ' | '.join(l for l in bridge_log().splitlines() if 'preview' in l.lower())[-600:])
    shot('preview')
    crop('preview', (240, 420, 1680, 1080), 'preview-zoom')
    d = differs('preview', 'extras-before')
    check('the previews card shows over the desktop', d > 0.15, 'picture difference %.2f' % d)
    ipc('winlist', 'previewHide')

    # ---- Hearth's Alt+Tab (hold Alt, Tab once: the switcher stays up while Alt is held)
    a_id = (win('Test A') or {}).get('id')
    ipc('winlist', 'act', 'Test A', 'activate')
    wait(lambda: listed().get('active') == a_id)
    if kbd:
        kbd.write(e.EV_KEY, e.KEY_LEFTALT, 1); kbd.syn(); time.sleep(0.1)
        kbd.write(e.EV_KEY, e.KEY_TAB, 1); kbd.syn(); time.sleep(0.05)
        kbd.write(e.EV_KEY, e.KEY_TAB, 0); kbd.syn()
        time.sleep(1.2)
        shot('alt-tab')
        kbd.write(e.EV_KEY, e.KEY_LEFTALT, 0); kbd.syn()
        switched = wait(lambda: listed().get('active') == (win('Test B') or {}).get('id'), 6)
        check('Alt+Tab switches to the other window', switched, listed().get('active'))
        d = differs('alt-tab', 'extras-before')
        check('the Alt+Tab switcher shows (Hearth\'s, org.hearth.switcher)', d > 0.4, 'picture difference %.2f' % d)

    # ---- Snap layouts from resting on Maximize: what the script sends when the pointer is on the button
    a = win('Test A') or {}
    f = a.get('frame') or {}
    if f:
        btn = {'id': a['id'], 'x': f['x'] + f['w'] - 5 - 34 - 2 - 34, 'y': f['y'] + 2, 'w': 34, 'h': 24, 'output': a.get('output', '')}
        bridge('HoverMax', GLib.Variant('(s)', (json.dumps(btn),)))
        st = wait(lambda: json.loads(ipc('snap', 'state') or '{}').get('hover') and json.loads(ipc('snap', 'state')), 6)
        check('resting on Maximize opens Snap layouts under the button', st, ipc('snap', 'state'))
        time.sleep(1)
        shot('snap-maximize')
        crop('snap-maximize', (max(0, btn['x'] - 420), max(0, btn['y'] - 20), btn['x'] + 300, btn['y'] + 330), 'snap-maximize-zoom')
        bridge('HoverMax', GLib.Variant('(s)', (json.dumps({'id': ''}),)))
        gone = wait(lambda: not json.loads(ipc('snap', 'state') or '{}').get('shown'), 5)
        check('Snap layouts goes away when the pointer leaves the button', gone, ipc('snap', 'state'))
        ipc('snap', 'close')

    # ---- hidden from screen sharing: the title bar menu's entry, then the portal's picker leaves it out
    ipc('winlist', 'run', json.dumps({'op': 'noShare', 'title': 'Test B', 'on': True}))
    hid = wait(lambda: (win('Test B') or {}).get('noShare') is True, 6)
    check('Hide from screen sharing marks the window', hid, win('Test B'))
    try:
        sys.path.insert(0, SHELL)
        import hearth_screencast as hs
        wins = [{'id': w['id'], 'title': w['title'], 'app': w.get('appId', '')} for w in listed().get('windows', [])]
        outs = [{'name': 'Virtual-1', 'desc': 'Screen', 'x': 0, 'y': 0, 'w': 1920, 'h': 1080}]
        req = hs.build_request('cast', 'com.discordapp.Discord', hs.MONITOR | hs.WINDOW, False, outs, wins, '/nonexistent', False)
        check('the screen-sharing picker leaves the hidden window out', [w['title'] for w in req['windows']] == ['Test A'] and req['hiddenCount'] == 1,
              [w['title'] for w in req['windows']])
        rq = os.path.join(OUT, 'cast-request.json')
        with open(rq, 'w') as fh:
            json.dump(req, fh)
        picker = subprocess.Popen(['python3', os.path.join(SHELL, 'hearth_screencast.py'), 'picker', rq], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(5)
        subprocess.run(['quickshell', 'ipc', '-p', os.path.join(SHELL, 'cast-picker.qml'), 'call', 'cast', 'tab', 'windows'], timeout=10, capture_output=True)
        time.sleep(1)
        shot('cast-hidden')
        subprocess.run(['quickshell', 'ipc', '-p', os.path.join(SHELL, 'cast-picker.qml'), 'call', 'cast', 'cancel'], timeout=10, capture_output=True)
        try:
            picker.wait(10)
        except subprocess.TimeoutExpired:
            picker.kill()
    except Exception as ex:
        check('the screen-sharing picker leaves the hidden window out', False, ex)
    ipc('winlist', 'run', json.dumps({'op': 'noShare', 'title': 'Test B', 'on': False}))
    ipc('winlist', 'act', 'Test A', 'activate')
    time.sleep(0.5)
    do({'op': 'menu'})
    time.sleep(1.5)
    # KWin puts scripts' entries under its menu's last group ("Extensions" here): the keys walk to it and open it.
    # (Up from nothing picked: the last entry, Close; once more: Extensions, just above it.)
    for _ in range(2):
        key('KEY_UP', hold=0.03)
    key('KEY_RIGHT')
    time.sleep(1)
    shot('window-menu-extras')
    key('KEY_ESC'); key('KEY_ESC')

    # ---- force closing a frozen window: KWin's click-a-window first (then Esc), then the prompt for a stopped app
    ipc('forceclose', 'start')
    picking = wait(lambda: json.loads(ipc('forceclose', 'state') or '{}').get('phase') == 'picking', 5)
    check('Ctrl+Alt+Esc\'s pick starts (KWin\'s click-a-window)', picking, ipc('forceclose', 'state'))
    time.sleep(1)
    shot('force-close-pick')
    key('KEY_ESC')
    cancelled = wait(lambda: json.loads(ipc('forceclose', 'state') or '{}').get('phase') == '', 6)
    check('Esc cancels the pick', cancelled, ipc('forceclose', 'state'))
    ipc('forceclose', 'cancel')
    k = subprocess.Popen(['foot', '--app-id=hearth.test.k', '--title=Test K', 'sleep', '600'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    procs.append(k)
    if wait(lambda: win('Test K'), 15):
        ipc('winlist', 'run', json.dumps({'op': 'snap', 'title': 'Test K', 'zone': 'center'}))
        time.sleep(1.5)
        os.kill(k.pid, 19)                      # SIGSTOP: it really stops answering
        ipc('forceclose', 'ask', 'Test K')
        time.sleep(1.5)
        shot('force-close')
        ipc('forceclose', 'confirm')
        closed = wait(lambda: not win('Test K') and k.poll() is not None, 12)
        check('Force close ends a frozen window (asked, then made to)', closed, [w.get('title') for w in listed().get('windows', [])])
    else:
        check('a window to force close', False, 'Test K never came')

    # ---- KWin's own views, opened the way Mantel's Task view button and keys do
    for name, pic in (('overview', 'overview'), ('windows', 'present-windows'), ('grid', 'desktop-grid')):
        ipc('winlist', 'effect', name)
        time.sleep(2.5)
        shot(pic)
        d = differs(pic, 'extras-before')
        check('KWin\'s %s opens from Mantel' % pic.replace('-', ' '), d > 1.0, 'picture difference %.2f' % d)
        key('KEY_ESC')
        time.sleep(1.5)
    # Super+W: Hearth's key for the Overview (KDE's own freed for it).
    if kbd:
        key('KEY_LEFTMETA', 'KEY_W')
        time.sleep(2.5)
        shot('overview-key')
        d = differs('overview-key', 'extras-before')
        check('Super+W opens the Overview', d > 1.0, 'picture difference %.2f' % d)
        key('KEY_ESC')
        time.sleep(1)
    kk = keys_now()
    check('Hearth\'s Super+W has its key (KDE\'s Overview freed)', kk.get('kwin/Hearth overview') == [[0x10000000 | 0x57]], kk.get('kwin/Hearth overview', kk))
    check('Hearth\'s Ctrl+Alt+Esc has its key (KDE\'s Kill Window freed)', kk.get('kwin/Hearth kill-window') == [[0x04000000 | 0x08000000 | 0x01000000]],
          kk.get('kwin/Hearth kill-window', kk))

    # ---- the keys in Settings › Keyboard › Keyboard shortcuts
    ipc('browserpick', 'close')
    ipc('settings', 'page', 'keyboard:shortcuts')
    time.sleep(4)
    shot('shortcuts')
    ipc('settings', 'close')
    if kbd:
        kbd.close()


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
    for t in ('Test A', 'Test B'):
        ipc('winlist', 'act', t, 'activate')

    extras_check(procs)
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
