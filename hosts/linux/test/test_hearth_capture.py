"""Hearth Capture (1.14, Mantel's own screenshot tool in place of KDE's Spectacle): hearth-capture-helper (taking the
picture through one swappable grab(), the names, Save as never over a file, the recent list as state, text and QR
codes), capture-draw.js (the area maths, pixelate, blur, the shapes; with node), and how it's wired in (shell.qml,
open.sh, Super+Shift+S, the menu entry, nothing of Spectacle left in Mantel's paths)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
HELPER = SHELL / 'hearth-capture-helper'
NODE = shutil.which('node')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_capture_helper', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_capture_helper', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_js(lib, body):
    src = (SHELL / lib).read_text().replace('.pragma library', '')
    r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


def code(s):
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


def decode_png(data):
    """(width, height, rows of RGBA bytes) of a PNG made by png_bytes (filter 0, RGBA 8-bit)."""
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    pos, idat, w, h = 8, b'', 0, 0
    while pos < len(data):
        n = struct.unpack('>I', data[pos:pos + 4])[0]
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + n]
        if kind == b'IHDR':
            w, h = struct.unpack('>II', body[:8])
        elif kind == b'IDAT':
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    rows = [raw[y * (w * 4 + 1) + 1:(y + 1) * (w * 4 + 1)] for y in range(h)]
    return w, h, rows


STAND_INS = ('grim', 'hearth-screens', 'tesseract', 'zbarimg', 'wl-copy', 'hearth-record', 'hearth-recorder', 'xdg-user-dir')
_SYS = []


def sysbin():
    """/usr/bin and /bin without the tools the tests stand in for (links to everything else), made once."""
    if not _SYS:
        d = tempfile.mkdtemp(prefix='capture-sys-')
        for src in ('/usr/bin', '/bin'):
            for f in os.listdir(src):
                p = os.path.join(d, f)
                if f not in STAND_INS and not os.path.lexists(p):
                    os.symlink(os.path.join(src, f), p)
        _SYS.append(d)
    return _SYS[0]


class Sandbox(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp()
        self.bin = os.path.join(self.t, 'bin')
        os.makedirs(self.bin)
        self.env = dict(os.environ, HOME=os.path.join(self.t, 'home'), XDG_RUNTIME_DIR=os.path.join(self.t, 'run'),
                        XDG_STATE_HOME=os.path.join(self.t, 'state'), XDG_CONFIG_HOME=os.path.join(self.t, 'config'),
                        PATH=self.bin + ':/usr/bin:/bin')
        self.env.pop('HEARTH_WM', None)
        os.makedirs(self.env['HOME'])
        self.env['PATH'] = self.bin + ':' + sysbin()

    def tearDown(self):
        shutil.rmtree(self.t, ignore_errors=True)

    def fake(self, name, body):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(p, 0o755)
        return p

    def helper(self, *args, input=None):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), env=self.env, capture_output=True, text=True,
                           input=input, timeout=60)
        last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else '{}'
        return r.returncode, json.loads(last)

    def png(self, path, w=4, h=3):
        m = load()
        with open(path, 'wb') as f:
            f.write(m.png_bytes(w, h, [bytes([255, 0, 0, 255]) * w for _ in range(h)]))


class Grab(Sandbox):
    def grim(self):
        # A stand-in grim: says what it was asked and writes a small PNG to the last argument.
        png = os.path.join(self.t, 'one.png')
        self.png(png, 8, 6)
        return self.fake('grim', 'echo "$@" >> "%s/grim.log"; for a; do last="$a"; done; cp "%s" "$last"' % (self.t, png))

    def log(self, name='grim.log'):
        p = os.path.join(self.t, name)
        return open(p).read().splitlines() if os.path.exists(p) else []

    def test_grim_modes(self):
        self.grim()
        code_, j = self.helper('grab', 'all', '--pointer')
        self.assertEqual(code_, 0, j)
        self.assertTrue(j['ok'] and os.path.isfile(j['path']))
        self.assertEqual((j['width'], j['height']), (8, 6))
        self.assertTrue(j['path'].startswith(self.env['XDG_RUNTIME_DIR'] + '/hearth-capture/'))
        self.helper('grab', 'screen', '--screen', 'HDMI-A-1')
        self.helper('grab', 'area', '--geometry', '10,20,300,200')
        log = self.log()
        self.assertTrue(log[0].startswith('-c '))
        self.assertIn('-o HDMI-A-1', log[1])
        self.assertIn('-g 10,20 300x200', log[2])

    def test_window_on_labwc_asks_for_the_area_picker(self):
        self.grim()
        code_, j = self.helper('grab', 'window')
        self.assertEqual(code_, 0)
        self.assertEqual(j, {'ok': False, 'error': '', 'pick': True})
        self.assertEqual(self.log(), [])

    def test_hearth_screens_comes_first(self):
        self.grim()
        png = os.path.join(self.t, 'two.png')
        self.png(png, 5, 5)
        self.fake('hearth-screens', 'echo "$@" >> "%s/screens.log"; while [ "$1" ]; do [ "$1" = --out ] && cp "%s" "$2"; shift; done' % (self.t, png))
        code_, j = self.helper('grab', 'window', '--pointer')
        self.assertTrue(j['ok'], j)
        self.assertEqual(j['width'], 5)
        self.assertEqual(self.log(), [])
        self.assertTrue(self.log('screens.log')[0].startswith('grab window --pointer --out '))
        # Exit 3: "can't find the window here": the picker.
        self.fake('hearth-screens', 'exit 3')
        _, j = self.helper('grab', 'window')
        self.assertTrue(j['pick'])

    def test_kwin_failure_is_said_not_a_crash(self):
        self.grim()
        self.env['HEARTH_WM'] = 'kwin'
        self.env['DBUS_SESSION_BUS_ADDRESS'] = 'unix:path=' + os.path.join(self.t, 'nobus')
        code_, j = self.helper('grab', 'all')
        self.assertEqual(code_, 1)
        self.assertFalse(j['ok'])
        self.assertIn('KWin', j['error'])
        self.assertEqual(self.log(), [])          # not grim under KWin

    def test_no_grim(self):
        code_, j = self.helper('grab', 'all')
        self.assertEqual(code_, 1)
        self.assertIn('grim', j['error'])

    def test_grab_each(self):
        self.grim()
        code_, j = self.helper('grab-each', 'DP-1', 'HDMI-A-1')
        self.assertTrue(j['ok'], j)
        self.assertEqual(sorted(j['shots']), ['DP-1', 'HDMI-A-1'])
        self.assertEqual(j['shots']['DP-1']['width'], 8)

    def test_area_needs_geometry(self):
        self.assertEqual(self.helper('grab', 'area')[0], 1)
        self.assertEqual(self.helper('grab', 'nope')[0], 1)


class Kwin(unittest.TestCase):
    """KWin's raw picture → PNG (QImage's formats)."""
    def test_formats(self):
        m = load()
        bgra = bytes([10, 20, 30, 40, 1, 2, 3, 4]) + b'\x00' * 8      # stride 16 for 2 pixels
        rows = m.raw_to_rows(bgra + bgra, 2, 2, 16, 6)
        self.assertEqual(rows[0], bytes([30, 20, 10, 40, 3, 2, 1, 4]))
        rows = m.raw_to_rows(bgra, 2, 1, 16, 4)                        # RGB32: opaque
        self.assertEqual(rows[0], bytes([30, 20, 10, 255, 3, 2, 1, 255]))
        rows = m.raw_to_rows(bgra, 2, 1, 16, 17)                       # RGBA8888 as it is
        self.assertEqual(rows[0], bgra[:8])
        with self.assertRaises(ValueError):
            m.raw_to_rows(bgra, 2, 1, 16, 99)
        with self.assertRaises(ValueError):
            m.raw_to_rows(bgra[:4], 2, 1, 16, 6)
        w, h, back = decode_png(m.png_bytes(2, 2, m.raw_to_rows(bgra + bgra, 2, 2, 16, 5)))
        self.assertEqual((w, h), (2, 2))
        self.assertEqual(back[1], bytes([30, 20, 10, 40, 3, 2, 1, 4]))

    def test_one_grab_function(self):
        src = (HELPER).read_text()
        # Everything that takes a picture goes through grab(): the commands never call grim or KWin themselves.
        body = src.split('def main(argv):')[1]
        self.assertNotIn("'grim'", body)
        self.assertNotIn('grab_kwin(', body)
        self.assertIn('hearth-screens', src.split('def grab(')[1].split('def parse_geometry')[0])


class SaveAndRecent(Sandbox):
    def test_save_names_and_history(self):
        src = os.path.join(self.t, 'edit.png')
        self.png(src, 7, 3)
        self.fake('xdg-user-dir', 'echo "%s/Pics"' % self.env['HOME'])
        _, a = self.helper('save', src)
        _, b = self.helper('save', src)
        self.assertTrue(a['ok'] and b['ok'])
        self.assertTrue(a['path'].startswith(self.env['HOME'] + '/Pics/Screenshots/Screenshot '))
        self.assertRegex(a['name'], r'^Screenshot \d{4}-\d\d-\d\d \d\d-\d\d-\d\d( \(\d+\))?\.png$')
        self.assertNotEqual(a['path'], b['path'])
        _, h = self.helper('history')
        self.assertEqual([i['path'] for i in h['items']], [b['path'], a['path']])
        self.assertEqual(h['items'][0]['width'], 7)
        hist = os.path.join(self.env['XDG_STATE_HOME'], 'nimbus', 'capture-history.json')
        self.assertTrue(os.path.isfile(hist))
        self.assertTrue(os.path.isfile(hist + '.bak'))      # written by hearth_settings_io (a .bak before a change)
        # A file that's gone isn't listed; forget takes one off (the file stays).
        os.unlink(a['path'])
        _, h = self.helper('history')
        self.assertEqual(len(h['items']), 1)
        self.helper('forget', b['path'])
        self.assertEqual(self.helper('history')[1]['items'], [])
        self.assertTrue(os.path.isfile(b['path']))

    def test_put_writes_a_new_file_only_for_a_png(self):
        import base64
        src = os.path.join(self.t, 'a.png')
        self.png(src, 6, 2)
        code_, j = self.helper('put', input=base64.b64encode(open(src, 'rb').read()).decode())
        self.assertTrue(j['ok'], j)
        self.assertEqual((j['width'], j['height']), (6, 2))
        self.assertTrue(j['path'].startswith(self.env['XDG_RUNTIME_DIR'] + '/hearth-capture/edit-'))
        self.assertEqual(self.helper('put', input='bm90IGEgcG5n')[0], 1)

    def test_save_as_never_over_a_file(self):
        src = os.path.join(self.t, 'edit.png')
        self.png(src)
        dest = os.path.join(self.t, 'out', 'mine')
        code_, j = self.helper('save', src, '--to', dest)
        self.assertEqual(code_, 0, j)
        self.assertEqual(j['path'], dest + '.png')
        before = open(dest + '.png', 'rb').read()
        self.png(src, 9, 9)
        code_, j = self.helper('save', src, '--to', dest + '.png')
        self.assertEqual(code_, 1)
        self.assertTrue(j['exists'])
        self.assertEqual(open(dest + '.png', 'rb').read(), before)
        code_, j = self.helper('save', src, '--to', dest + '.png', '--replace')
        self.assertTrue(j['ok'])
        self.assertNotEqual(open(dest + '.png', 'rb').read(), before)

    def test_opening_writes_nothing(self):
        self.helper('tools')
        self.helper('history')
        self.assertFalse(os.path.exists(os.path.join(self.env['XDG_STATE_HOME'], 'nimbus')))
        self.assertFalse(os.path.exists(os.path.join(self.env['XDG_CONFIG_HOME'], 'nimbus')))

    def test_history_is_kept_short(self):
        m = load()
        os.environ_backup = dict(os.environ)
        try:
            os.environ['XDG_STATE_HOME'] = self.env['XDG_STATE_HOME']
            for i in range(m.HISTORY_MAX + 5):
                p = os.path.join(self.t, 'p%d.png' % i)
                self.png(p)
                m.history_add(p, 4, 3)
            self.assertEqual(len(m.history_read()), m.HISTORY_MAX)
            self.assertTrue(m.history_read()[0]['path'].endswith('p%d.png' % (m.HISTORY_MAX + 4)))
        finally:
            os.environ.clear()
            os.environ.update(os.environ_backup)

    def test_tools_and_record(self):
        _, t = self.helper('tools')
        self.assertEqual((t['tesseract'], t['zbar'], t['record'], t['windowPick']), (False, False, '', True))
        self.assertEqual(self.helper('record')[0], 1)
        self.fake('hearth-recorder', 'echo "$@" > "%s/rec.log"' % self.t)
        self.assertEqual(self.helper('tools')[1]['record'], 'hearth-recorder')
        self.fake('hearth-record', 'echo hr > "%s/rec.log"' % self.t)
        self.assertEqual(self.helper('tools')[1]['record'], 'hearth-record')   # the other branch's recorder first
        self.env['HEARTH_WM'] = 'kwin'
        self.assertFalse(self.helper('tools')[1]['windowPick'])


class TextAndQr(Sandbox):
    def test_missing(self):
        p = os.path.join(self.t, 'a.png')
        self.png(p)
        self.assertEqual(self.helper('ocr', p)[1], {'ok': False, 'missing': 'tesseract'})
        self.assertEqual(self.helper('qr', p)[1], {'ok': False, 'missing': 'zbarimg'})

    def test_found(self):
        p = os.path.join(self.t, 'a.png')
        self.png(p)
        self.fake('tesseract', 'printf "Hello  Hearth\\n\\n\\n\\nsecond\\f"')
        self.fake('zbarimg', 'printf "https://hearthos.fun\\nWIFI:S:home;;\\n"')
        self.assertEqual(self.helper('ocr', p, '--lang', 'eng')[1], {'ok': True, 'text': 'Hello  Hearth\n\nsecond'})
        self.assertEqual(self.helper('qr', p)[1], {'ok': True, 'codes': ['https://hearthos.fun', 'WIFI:S:home;;']})
        self.fake('zbarimg', 'exit 4')
        self.assertEqual(self.helper('qr', p)[1], {'ok': True, 'codes': []})

    def test_copy_text_goes_on_stdin(self):
        self.fake('wl-copy', 'cat > "%s/clip.txt"' % self.t)
        code_, j = self.helper('copy-text', input='line one\nline two')
        self.assertTrue(j['ok'])
        clip = pathlib.Path(self.t, 'clip.txt')
        for _ in range(50):
            if clip.exists() and clip.read_text():
                break
            time.sleep(0.05)
        self.assertEqual(clip.read_text(), 'line one\nline two')


@unittest.skipUnless(NODE, 'node is needed for capture-draw.js')
class Draw(unittest.TestCase):
    def test_area_maths(self):
        r = run_js('capture-draw.js', '''console.log(JSON.stringify([
            pickerToCrop({x: 100, y: 50, w: 400, h: 300}, 1280, 800, 2560, 1600),
            pickerToCrop({x: 1200, y: 700, w: 400, h: 300}, 1280, 800, 1280, 800),
            moveSel({x: 10, y: 10, w: 100, h: 100}, -50, 5000, 1920, 1080),
            resizeSel({x: 1800, y: 10, w: 100, h: 100}, 500, -500, 1920, 1080),
            startSel(1920, 1080), fit(1000, 500, 2000, 500), fit(1000, 1000, 100, 100), normRect(30, 40, 10, 5)]))''')
        self.assertEqual(r[0], {'x': 200, 'y': 100, 'w': 800, 'h': 600})        # a 2× screen: the picture's pixels
        self.assertEqual(r[1], {'x': 1200, 'y': 700, 'w': 80, 'h': 100})        # kept on the picture
        self.assertEqual(r[2], {'x': 0, 'y': 980, 'w': 100, 'h': 100})
        self.assertEqual(r[3], {'x': 1800, 'y': 10, 'w': 120, 'h': 24})
        self.assertEqual(r[4], {'x': 480, 'y': 270, 'w': 960, 'h': 540})
        self.assertEqual(r[5], 0.5)
        self.assertEqual(r[6], 2)
        self.assertEqual(r[7], {'x': 10, 'y': 5, 'w': 20, 'h': 35})

    def test_pixelate_and_blur(self):
        r = run_js('capture-draw.js', '''
            var d = []; for (var i = 0; i < 4 * 4 * 4; i++) d.push(i % 4 === 3 ? 255 : ((i >> 2) % 2 ? 200 : 0));
            var p = pixelate(d.slice(), 4, 4, 2);
            var b = blur(d.slice(), 4, 4, 1);
            console.log(JSON.stringify({p: p.slice(0, 8), len: b.length, spread: Math.max.apply(null, b) - Math.min.apply(null, b.filter(function (v, i) { return i % 4 !== 3; }))}));''')
        self.assertEqual(r['p'], [100, 100, 100, 255, 100, 100, 100, 255])
        self.assertEqual(r['len'], 64)
        self.assertLess(r['spread'], 255)

    def test_shapes(self):
        r = run_js('capture-draw.js', '''
            var calls = [];
            var ctx = new Proxy({}, { get: function (t, k) { if (k in t) return t[k]; return function () { calls.push(k); if (k === "getImageData") return { data: [0, 0, 0, 255, 9, 9, 9, 255] }; }; },
                                       set: function (t, k, v) { t[k] = v; return true; } });
            var drew = [];
            var shapes = [{t: "arrow", c: "#ff0000", w: 6, x1: 0, y1: 0, x2: 100, y2: 0}, {t: "pix", x1: 10, y1: 10, x2: 60, y2: 40},
                          {t: "step", c: "#4aa8ff", x1: 5, y1: 5, n: 3}, {t: "text", x1: 1, y1: 1, text: "a\\nb", size: 20}];
            paint(ctx, shapes, 0.5, 0, 0, function () { drew.push([].slice.call(arguments)); }, 100, 100);
            console.log(JSON.stringify({calls: calls, drew: drew, next: nextStep(shapes),
              keep: [worthKeeping({t: "box", x1: 1, y1: 1, x2: 2, y2: 2}), worthKeeping({t: "text", text: " "}), worthKeeping({t: "pen", pts: [1, 2]}), worthKeeping({t: "arrow", x1: 0, y1: 0, x2: 10, y2: 0})],
              name: defaultName(new Date(2026, 10, 16, 21, 4, 5)), head: arrowHead(0, 0, 10, 0, 5).map(Math.round)}));''')
        self.assertIn('fill', r['calls'])                       # the arrow's head
        self.assertIn('putImageData', r['calls'])               # pixelate wrote its pixels back
        self.assertEqual(r['calls'].count('fillText'), 3)       # two lines of text, the step's number
        self.assertEqual(r['drew'], [[10, 10, 50, 30, 5, 5, 25, 15]])   # the picture's own pixels for that area
        self.assertEqual(r['next'], 4)
        self.assertEqual(r['keep'], [False, False, True, True])
        self.assertEqual(r['name'], 'Screenshot 2026-11-16 21-04-05.png')
        self.assertEqual(sorted(r['head'][1::2]), [-2, 2])
        self.assertEqual(r['head'][0::2], [5, 5])

    def test_helper_names_like_the_editor(self):
        m = load()
        d = tempfile.mkdtemp()
        try:
            p = m.free_name(d, 1700000000)
            q = m.free_name(d, 1700000000)
            self.assertTrue(os.path.basename(q).endswith(' (2).png'))
            js = run_js('capture-draw.js', 'console.log(JSON.stringify(defaultName(new Date(1700000000 * 1000))))')
            self.assertEqual(os.path.basename(p), js)
        finally:
            shutil.rmtree(d)


class Wiring(unittest.TestCase):
    def test_shell_loads_capture_by_file_name(self):
        s = code((SHELL / 'shell.qml').read_text())
        self.assertIn('Loader { source: "Capture.qml" }', s)
        cap = code((SHELL / 'Capture.qml').read_text())
        self.assertIn('target: "capture"', cap)
        self.assertIn('SettingsWriter { id: writer; path: cap.prefsPath }', cap)
        self.assertNotIn('setText(', cap)                     # choices only through SettingsWriter, never a whole file
        self.assertNotIn('writer.merge', cap.split('function setPref')[0])   # nothing saved on open
        self.assertIn('source: "CaptureEditor.qml"', cap)
        ed = code((SHELL / 'CaptureEditor.qml').read_text())
        self.assertNotIn('save(ed.src', ed)                   # the picture taken is never written over
        self.assertIn('cap.run(["save", p', ed)
        self.assertIn('cap.run(["put"]', ed)

    def test_every_tool_is_there(self):
        ed = (SHELL / 'CaptureEditor.qml').read_text()
        for tool in ('"arrow"', '"box"', '"pen"', '"hl"', '"text"', '"blur"', '"pix"', '"step"', '"crop"'):
            self.assertIn('id: ' + tool, ed)
        for act in ('saveAndCopy()', 'copyOnly()', 'openSaveAs()', 'showFolder()', 'readText()', 'readQr()', 'undo()', 'redo()'):
            self.assertIn(act, ed)
        cap = (SHELL / 'Capture.qml').read_text()
        for m in ('"area"', '"window"', '"screen"', '"all"'):
            self.assertIn('id: ' + m, cap)
        self.assertIn('[0, 3, 5, 10]', cap)                   # the delays
        self.assertIn('"pointer"', cap)
        self.assertIn('["record"]', cap)

    def test_controller_and_health(self):
        pad = (SHELL / 'Pad.qml').read_text()
        self.assertIn('SettingsNav.captureShown', pad)
        self.assertIn('SettingsNav.captureOpen', (SHELL / 'ShellHealth.qml').read_text())
        nav = (SHELL / 'SettingsNav.qml').read_text()
        for p in ('captureShown', 'captureOpen', 'polkitShown'):
            self.assertIn('property bool ' + p, nav)

    def test_no_spectacle_in_mantels_paths(self):
        open_sh = (ROOT / 'hosts/linux/desktop/open.sh').read_text()
        self.assertNotIn('spectacle', open_sh)
        self.assertIn('capture-region)', open_sh)
        rc = (ROOT / 'hosts/linux/desktop/labwc/rc.xml').read_text()
        self.assertIn('<keybind key="W-S-s"><action name="Execute" command="sh -c \'sh $NIMBUS_SHARE/desktop/open.sh capture-region\'" /></keybind>', rc)
        self.assertIn("open.sh screenshot'", rc)               # Print Screen still saves one right away
        entry = (ROOT / 'os/system_files/usr/share/applications/hearth-screenshot.desktop').read_text()
        self.assertIn('Exec=/usr/bin/hearth-capture\n', entry)
        self.assertIn('OnlyShowIn=Nimbus;', entry)
        kinds = json.loads((ROOT / 'os/system_files/usr/share/hearth/app-kinds.json').read_text())
        self.assertEqual(kinds['kinds']['screenshot']['apps']['Nimbus'], ['hearth-screenshot.desktop'])
        launcher = (ROOT / 'os/system_files/usr/bin/hearth-capture').read_text()
        self.assertIn('call capture "$@"', launcher)
        self.assertTrue(os.access(ROOT / 'os/system_files/usr/bin/hearth-capture', os.X_OK))
        self.assertTrue(os.access(HELPER, os.X_OK))


if __name__ == '__main__':
    unittest.main()
