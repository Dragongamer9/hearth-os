"""Hearth 1.14's clipboard, recorder and capture extras (docs/KDE-FEATURES-1.14.md): clipboard actions (Open in… a
browser, QR code, Edit), selection (middle-click) history and keeping the clipboard and selection in step, the
recorder's format (MP4, WebM, MKV, GIF) and quality and its Recording pop-up, and Hearth Capture's scrolling capture
(stitching) and Share (copy, save, open with, send to the phone over Phone link's socket). Nothing here needs a
desktop: the stand-ins are files, fake apps and a fake Phone link service; patched globals are put back."""
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import random
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import zlib
from contextlib import redirect_stdout

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
sys.path.insert(0, str(SHELL))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def code(s):
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


class Env(unittest.TestCase):
    """A home of its own: data, config, runtime and state folders, and the apps in XDG_DATA_HOME."""
    KEYS = ('XDG_DATA_HOME', 'XDG_DATA_DIRS', 'XDG_RUNTIME_DIR', 'XDG_STATE_HOME', 'XDG_CONFIG_HOME', 'CLIPBOARD_STATE',
            'HEARTH_CLIP_NOW', 'HEARTH_CLIP_TYPES', 'HEARTH_CONNECT_SOCKET', 'HEARTH_CONNECT_BIN', 'PATH', 'HEARTH_WM')

    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-114-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.saved = {k: os.environ.get(k) for k in self.KEYS}
        self.addCleanup(self.restore)
        for k, d in (('XDG_DATA_HOME', 'data'), ('XDG_RUNTIME_DIR', 'run'), ('XDG_STATE_HOME', 'state'), ('XDG_CONFIG_HOME', 'config')):
            os.environ[k] = os.path.join(self.tmp, d)
            os.makedirs(os.environ[k], exist_ok=True)
        os.environ['XDG_DATA_DIRS'] = os.path.join(self.tmp, 'none')
        for k in ('CLIPBOARD_STATE', 'HEARTH_CLIP_NOW', 'HEARTH_CLIP_TYPES', 'HEARTH_WM'):
            os.environ.pop(k, None)

    def restore(self):
        for k, v in self.saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def app(self, name, exec_line, mime, hidden=False):
        d = os.path.join(os.environ['XDG_DATA_HOME'], 'applications')
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, name), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=%s\nExec=%s\nMimeType=%s\n%s' % (
                name[:-8].title(), exec_line, mime, 'NoDisplay=true\n' if hidden else ''))

    def fake_bin(self, name, body):
        d = os.path.join(self.tmp, 'bin')
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(p, 0o755)
        os.environ['PATH'] = d + os.pathsep + (self.saved.get('PATH') or '/usr/bin:/bin')
        return p

    def wait_for(self, path, timeout=10):
        end = time.time() + timeout
        while time.time() < end:
            if os.path.exists(path) and os.path.getsize(path) > 0:
                time.sleep(0.05)
                return True
            time.sleep(0.05)
        return False


# ====================================================================== clipboard actions
class ClipboardActions(Env):
    def setUp(self):
        super().setUp()
        self.c = load(SHELL / 'hearth-clipboard', 'hearth_clipboard_114')
        self.n = 1000.0

    def put(self, text, primary=False, types=('text/plain;charset=utf-8',), state=''):
        self.n += 1
        return self.c.store(text.encode() if isinstance(text, str) else text, types=list(types), app='Firefox', now=self.n,
                            state=state, primary=primary)

    def items(self):
        return self.c.list_items()['items']

    def test_a_link_is_marked_and_opens_in_the_browser_you_pick(self):
        out = os.path.join(self.tmp, 'opened.txt')
        self.app('firefox.desktop', 'sh -c \'echo "firefox $1" > %s\' sh %%u' % out, 'text/html;x-scheme-handler/http;x-scheme-handler/https;')
        self.app('chrome.desktop', 'sh -c \'echo "chrome $1" > %s\' sh %%U' % out, 'x-scheme-handler/https;')
        self.app('kate.desktop', 'kate %F', 'text/html;text/plain;')          # opens HTML files, not links
        self.app('hidden.desktop', 'hidden %u', 'x-scheme-handler/https;', hidden=True)
        iid = self.put('https://hearthos.fun/download')['id']
        self.put('just words')
        its = {i['text']: i for i in self.items()}
        self.assertEqual(its['https://hearthos.fun/download']['link'], 'https://hearthos.fun/download')
        self.assertEqual(its['just words']['link'], '')
        apps = load(SHELL / 'hearth_desktop_apps.py', 'hda_114')
        names = [a['id'] for a in apps.apps_for('x-scheme-handler/https', default='chrome.desktop')]
        self.assertEqual(names, ['chrome.desktop', 'firefox.desktop'])      # the default first; no editor, nothing hidden
        self.c.open_in(iid, 'firefox.desktop')
        self.assertTrue(self.wait_for(out))
        with open(out) as f:
            self.assertEqual(f.read().strip(), 'firefox https://hearthos.fun/download')
        with self.assertRaises(self.c.Fail):
            self.c.open_in(its['just words']['id'], 'firefox.desktop')
        with self.assertRaises(self.c.Fail):
            self.c.open_in(iid, '../../bin/sh.desktop')

    def test_a_bare_www_address_gets_https(self):
        self.assertEqual(self.c.link_of('www.debian.org'), 'https://www.debian.org')
        self.assertEqual(self.c.link_of('see https://x.y'), '')
        self.assertEqual(self.c.link_of('  http://a.b/c?d=1  '), 'http://a.b/c?d=1')

    def test_exec_field_codes(self):
        apps = load(SHELL / 'hearth_desktop_apps.py', 'hda_114b')
        self.assertEqual(apps.command('firefox --new-window %u', 'https://a'), ['firefox', '--new-window', 'https://a'])
        self.assertEqual(apps.command('gimp-2.10 %U %i', '/p/a b.png'), ['gimp-2.10', '/p/a b.png'])
        self.assertEqual(apps.command('eog', '/p/x.png'), ['eog', '/p/x.png'])
        self.assertEqual(apps.command('app --file=%f', '/x'), ['app', '--file=/x'])

    def test_edit_changes_the_item_not_the_clipboard(self):
        iid = self.put('sudo apt install hearth')['id']
        self.c.set_pin(iid, True)
        self.put('another')
        r = self.c.edit(iid, b'sudo apt install hearth-os')
        self.assertTrue(r['ok'])
        it = next(i for i in self.items() if i['id'] == iid)
        self.assertEqual((it['text'], it['pinned']), ('sudo apt install hearth-os', True))
        with open(it['path'], 'rb') as f:
            self.assertEqual(f.read(), b'sudo apt install hearth-os')
        self.assertEqual(self.c.item_text(iid)[1], 'sudo apt install hearth-os')
        # Copying the edited text again finds it (no second copy).
        self.put('sudo apt install hearth-os')
        self.assertEqual(len([i for i in self.items() if i['text'] == 'sudo apt install hearth-os']), 1)
        with self.assertRaises(self.c.Fail):
            self.c.edit(iid, b'   ')
        # Editing into the same text as another item: that other copy goes.
        other = self.put('typo')['id']
        self.c.edit(other, b'another')
        self.assertEqual(len([i for i in self.items() if i['text'] == 'another']), 1)

    def test_a_picture_has_no_text_actions(self):
        png = b'\x89PNG\r\n\x1a\n' + b'\0' * 40
        iid = self.put(png, types=('image/png',))['id']
        with self.assertRaises(self.c.Fail):
            self.c.qr(iid)
        with self.assertRaises(self.c.Fail):
            self.c.edit(iid, b'x')

    def test_qr_code(self):
        iid = self.put('https://hearthos.fun')['id']
        if not shutil.which('qrencode'):
            # The message when it's missing (the image has it: os-bases/peppermint/build.sh).
            with self.assertRaises(self.c.Fail) as e:
                self.c.qr(iid)
            self.assertIn('qrencode', str(e.exception))
            return
        r = self.c.qr(iid)
        with open(r['path'], 'rb') as f:
            self.assertEqual(f.read(8), b'\x89PNG\r\n\x1a\n')
        self.assertEqual(os.stat(os.path.dirname(r['path'])).st_mode & 0o777, 0o700)
        if shutil.which('zbarimg'):
            z = subprocess.run(['zbarimg', '-q', '--raw', r['path']], capture_output=True, text=True)
            self.assertEqual(z.stdout.strip(), 'https://hearthos.fun')
        long_id = self.put('x' * 3000)['id']
        with self.assertRaises(self.c.Fail):
            self.c.qr(long_id)

    # ------------------------------------------------------------------ the selection
    def test_selection_is_kept_only_with_keep_and_grows_into_one_item(self):
        h = str(SHELL / 'hearth-clipboard')
        env = dict(os.environ, HEARTH_CLIP_TYPES='text/plain')
        r = subprocess.run([sys.executable, h, 'store', '--primary'], input=b'highlighted', capture_output=True, env=env, timeout=30)
        self.assertEqual(json.loads(r.stdout)['stored'], False)       # Selection history off: nothing kept
        self.assertEqual(self.items(), [])
        # Dragging over words: each step is a new selection; they become one item.
        for t in ('Hea', 'Hearth', 'Hearth OS', 'Hearth OS 1.14'):
            self.n += 2
            self.c.store(t.encode(), types=['text/plain'], app='Kate', now=self.n, state='', primary=True)
        its = self.items()
        self.assertEqual([i['text'] for i in its], ['Hearth OS 1.14'])
        self.assertTrue(its[0]['sel'])
        # Something else highlighted later is its own item; a picture never comes from the selection.
        self.n += 60
        self.c.store(b'Another line', types=['text/plain'], app='Kate', now=self.n, state='', primary=True)
        self.assertEqual(len(self.items()), 2)
        self.assertFalse(self.c.store(b'\x89PNG\r\n\x1a\n' + b'\0' * 30, types=['image/png'], now=self.n + 1, primary=True)['stored'])
        # Copying (the clipboard) never joins a selection item.
        self.put('Another line, copied')
        self.assertEqual(len(self.items()), 3)
        self.assertFalse(next(i for i in self.items() if i['text'] == 'Another line, copied')['sel'])
        # A secret is never kept from the selection either.
        self.assertFalse(self.c.store(b'hunter2', types=['text/plain', 'x-kde-passwordManagerHint'], now=self.n + 5, primary=True)['stored'])

    def test_sync_puts_text_on_the_other_and_never_bounces(self):
        now = os.path.join(self.tmp, 'now')
        os.environ['HEARTH_CLIP_NOW'] = now
        # Copied: onto the selection too.
        self.assertTrue(self.c.sync(b'copied text', from_primary=False, types=['text/plain'], state=''))
        with open(now + '.primary', 'rb') as f:
            self.assertEqual(f.read(), b'copied text')
        # The selection's watcher then sees it: it's on the clipboard already, so nothing goes back (no loop).
        with open(now, 'wb') as f:
            f.write(b'copied text')
        self.assertFalse(self.c.sync(b'copied text', from_primary=True, types=['text/plain'], state=''))
        # Highlighted: onto the clipboard.
        self.assertTrue(self.c.sync(b'highlighted', from_primary=True, types=['text/plain'], state=''))
        with open(now, 'rb') as f:
            self.assertEqual(f.read(), b'highlighted')
        # Secrets and pictures stay where they are.
        self.assertFalse(self.c.sync(b'hunter2', from_primary=False, types=['text/plain'], state='sensitive'))
        self.assertFalse(self.c.sync(b'pw', from_primary=False, types=['x-kde-passwordManagerHint'], state=''))
        self.assertFalse(self.c.sync(b'\x89PNG\r\n\x1a\n' + b'\0' * 30, from_primary=False, types=['image/png'], state=''))
        with open(now + '.primary', 'rb') as f:
            self.assertEqual(f.read(), b'copied text')

    def test_store_command_line(self):
        h = str(SHELL / 'hearth-clipboard')
        env = dict(os.environ, HEARTH_CLIP_TYPES='text/plain', HEARTH_CLIP_NOW=os.path.join(self.tmp, 'now2'))
        r = subprocess.run([sys.executable, h, 'store', '--primary', '--keep', '--sync'], input=b'middle click me', capture_output=True, env=env, timeout=30)
        j = json.loads(r.stdout)
        self.assertTrue(j['stored'] and j['synced'], j)
        with open(env['HEARTH_CLIP_NOW'], 'rb') as f:
            self.assertEqual(f.read(), b'middle click me')
        r = subprocess.run([sys.executable, h, 'store', '--loud'], input=b'x', capture_output=True, env=env, timeout=30)
        self.assertEqual(r.returncode, 2)

    def test_the_popup_and_settings_page(self):
        q = code((SHELL / 'Clipboard.qml').read_text())
        # Two watchers, the selection's only when you asked, and with your choices.
        self.assertIn('"wl-paste", "--primary", "--watch", "python3", cb.helper, "store", "--primary"', q)
        self.assertIn('readonly property bool selWanted: wanted && prefsRead && (selectionOn || syncOn)', q)
        self.assertIn('cb.selectionOn ? ["--keep"] : []', q)
        # Its choices in their own file, merged a key at a time; read before anything runs; never written by opening.
        self.assertIn('/nimbus/clipboard.json', q)
        self.assertIn('SettingsWriter { id: prefWriter; path: cb.prefsPath }', q)
        self.assertEqual(len(re.findall(r'prefWriter\.merge\(', q)), 1)
        self.assertIn('function setPref(k, v) {', q)
        open_fn = q.split('function open() {')[1].split('\n')[0]
        self.assertNotIn('setPref', open_fn)
        self.assertNotIn('merge', open_fn)
        # More, QR code and Edit.
        for s in ('function more(i)', 'function showQr(it)', 'function startEdit(it)', 'function saveEdit(t)', '["open", id, a.app]',
                  'job(["qr", it.id]', 'job(["edit", id]', 'function more(i: int): void'):
            self.assertIn(s, q)
        page = code((SHELL / 'SetClipboard.qml').read_text())
        self.assertIn('page.cbx.setPref("selection", v)', page)
        self.assertIn('page.cbx.setPref("sync", v)', page)
        self.assertIn('checked: !!page.cbx && page.cbx.selectionOn', page)


# ====================================================================== the recorder
class Recorder(Env):
    def setUp(self):
        super().setUp()
        self.rec = load(SHELL / 'hearth-record', 'hearth_record_114')
        self.rec.SETTINGS = os.path.join(self.tmp, 'config', 'nimbus', 'record.json')
        self.rec.STATE = os.path.join(self.tmp, 'run', 'hearth-record', 'state.json')
        self.rec.RUNTIME = os.path.dirname(self.rec.STATE)
        real = self.rec.gst_has
        self.addCleanup(setattr, self.rec, 'gst_has', real)

    def run_cmd(self, argv):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = self.rec.main(argv)
        return rc, (buf.getvalue().strip().splitlines() or [''])[-1]

    def test_formats_and_quality_are_choices_saved_only_when_changed(self):
        self.assertEqual(self.rec.DEFAULTS['format'], 'mp4')                 # as it always recorded
        self.assertEqual(self.rec.DEFAULTS['quality'], 'high')
        self.assertIs(self.rec.DEFAULTS['popup'], True)
        rc, line = self.run_cmd(['status'])
        self.assertEqual(rc, 0, line)
        self.assertFalse(os.path.exists(self.rec.SETTINGS))                   # status writes nothing
        self.assertEqual(json.loads(line)['choices']['format'], 'mp4')
        self.assertIn('formats', json.loads(line)['can'])
        for k, v in (('format', 'gif'), ('quality', 'low'), ('popup', 'off')):
            self.assertEqual(self.run_cmd(['set', k, v])[0], 0)
        with open(self.rec.SETTINGS) as f:
            self.assertEqual(json.load(f), {'format': 'gif', 'quality': 'low', 'popup': False})
        self.assertEqual(self.run_cmd(['set', 'format', 'avi'])[0], 1)
        self.assertEqual(self.run_cmd(['set', 'quality', 'ultra'])[0], 1)
        self.assertEqual(self.run_cmd(['set', 'popup', 'maybe'])[0], 1)
        o = self.rec.parse_opts(['--format', 'webm', '--quality', 'medium'])
        self.assertEqual((o['format'], o['quality']), ('webm', 'medium'))

    def test_pipelines_per_format(self):
        self.rec.gst_has = lambda n: n in ('avenc_aac', 'mp4mux', 'opusenc', 'webmmux', 'matroskamux', 'vp8enc', 'x264enc')
        x264 = self.rec.ENCODERS[3]
        o = {'fps': 30, 'sound': 'both', 'format': 'mp4', 'quality': 'high'}
        p = self.rec.pipeline(7, 42, o, x264, '/v/a.mp4', True, 'mp4')
        self.assertIn('mp4mux name=mux fragment-duration=1000', p)
        self.assertIn('avenc_aac', p)
        self.assertIn('bitrate=24000', p)                                    # high: the bitrate it always had
        p = self.rec.pipeline(7, 42, dict(o, quality='low'), x264, '/v/a.mp4', True, 'mp4')
        self.assertIn('bitrate=7200', p)
        p = self.rec.pipeline(7, 42, dict(o, format='mkv'), x264, '/v/a.mkv', True, 'mkv')
        self.assertIn('matroskamux name=mux', p)
        self.assertIn('h264parse', p)
        vp8 = self.rec.WEBM_ENCODERS[1]
        p = self.rec.pipeline(7, 42, dict(o, format='webm', quality='medium'), vp8, '/v/a.webm', False, 'webm')
        self.assertIn('webmmux name=mux', p)
        self.assertIn('vp8enc', p)
        self.assertIn('target-bitrate=13200000', p)
        self.assertIn('opusenc', p)
        self.assertNotIn('h264parse', p)
        p = self.rec.pipeline(7, 42, dict(o, format='gif', quality='low'), x264, '/v/.a.gif.part.mkv', False, 'gif')
        self.assertNotIn('pulsesrc', p)                                      # a GIF has no sound
        self.assertIn('matroskamux', p)
        self.assertIn('bitrate=24000', p)                                    # the H.264 in between stays good
        # The old call (no format) is still MP4 / MKV as before.
        self.assertIn('mp4mux', self.rec.pipeline(7, 42, {'fps': 60, 'sound': 'none'}, x264, '/v/a.mp4', True))
        self.assertIn('matroskamux', self.rec.pipeline(7, 42, {'fps': 60, 'sound': 'none'}, x264, '/v/a.mkv', False))

    def test_what_this_pc_can_record(self):
        self.rec.gst_has = lambda n: n in ('mp4mux', 'x264enc', 'matroskamux')
        real_which = self.rec.shutil.which
        self.addCleanup(setattr, self.rec.shutil, 'which', real_which)
        self.rec.shutil.which = lambda n: None if n == 'ffmpeg' else real_which(n)
        self.assertEqual(self.rec.can_formats(), {'mp4': True, 'mkv': True, 'webm': False, 'gif': False})
        self.rec.shutil.which = lambda n: '/usr/bin/ffmpeg' if n == 'ffmpeg' else real_which(n)
        self.assertTrue(self.rec.can_formats()['gif'])

    def test_gif_from_a_recording(self):
        cmd = self.rec.gif_command('/v/.x.part.mkv', '/v/x.gif', 'medium')
        self.assertEqual(cmd[0], 'ffmpeg')
        vf = cmd[cmd.index('-vf') + 1]
        self.assertIn('fps=15', vf)
        self.assertIn('palettegen', vf)
        self.assertIn("min(1280,iw)", vf)
        if not shutil.which('ffmpeg'):
            return
        d = os.path.join(self.tmp, 'v')
        os.makedirs(d)
        src = os.path.join(d, '.clip.part.mkv')
        r = subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i', 'testsrc=size=320x180:rate=15',
                            '-t', '1', '-c:v', 'libx264' if b'libx264' in subprocess.run(['ffmpeg', '-hide_banner', '-encoders'], capture_output=True).stdout else 'mpeg4', src],
                           capture_output=True)
        if r.returncode != 0:
            self.skipTest('ffmpeg could not make a test video')
        ok, why = self.rec.make_gif(src, os.path.join(d, 'clip.gif'), 'low')
        self.assertTrue(ok, why)
        with open(os.path.join(d, 'clip.gif'), 'rb') as f:
            self.assertEqual(f.read(6), b'GIF89a')
        self.assertEqual(sorted(x for x in os.listdir(d) if not x.startswith('.clip.part')), ['clip.gif'])   # no .part left

    def test_converting_counts_as_busy_and_gifs_are_recent(self):
        self.assertIn('converting', self.rec.BUSY)
        os.makedirs(self.rec.RUNTIME, exist_ok=True)
        with open(self.rec.STATE, 'w') as f:
            json.dump({'state': 'converting', 'pid': 0}, f)
        self.assertEqual(self.rec.read_state()['state'], 'idle')             # not ours (pid 0): idle
        vids = os.path.join(self.tmp, 'Videos', 'Recordings')
        os.makedirs(vids)
        for n in ('a.gif', 'b.webm', 'c.mkv', '.d.gif.part.mkv', 'e.txt'):
            open(os.path.join(vids, n), 'w').close()
        real = self.rec.videos_dir
        self.addCleanup(setattr, self.rec, 'videos_dir', real)
        self.rec.videos_dir = lambda: vids
        self.assertEqual(sorted(r['name'] for r in self.rec.recent()), ['a.gif', 'b.webm', 'c.mkv'])

    def test_panel_and_popup(self):
        q = code((SHELL / 'Recorder.qml').read_text())
        self.assertIn('{ key: "format", label: "Format", opts: [["mp4", "MP4"], ["webm", "WebM"], ["mkv", "MKV"], ["gif", "GIF"]] }', q)
        self.assertIn('{ key: "quality", label: "Quality"', q)
        self.assertIn('WlrLayershell.namespace: "hearth-recording-popup"', q)
        self.assertIn('readonly property bool popupShown: recording && live.state !== "starting" && !popupHidden && choices.popup !== false && !shown', q)
        self.assertIn('onClicked: rp.stop()', q)
        self.assertIn('rp.choose("popup"', q)
        # The pop-up never takes the keyboard (the app being recorded keeps it).
        pop = q.split('namespace: "hearth-recording-popup"')[1].split('\n')[1]
        self.assertIn('WlrKeyboardFocus.None', pop)


# ====================================================================== scrolling capture
def page(w, total, seed=7):
    """A tall made-up page: rows of coloured blocks, so every row is different."""
    rnd = random.Random(seed)
    rows = []
    for y in range(total):
        r = bytearray(w * 3)
        base = (y * 37) % 251
        for x in range(0, w, 8):
            v = (base + x * 3 + rnd.randrange(40)) & 255
            r[x * 3:(x + 8) * 3] = bytes((v, (v * 7) & 255, (255 - v)) * 8)[:len(r[x * 3:(x + 8) * 3])]
        rows.append(bytes(r))
    return rows


class Scrolling(Env):
    def setUp(self):
        super().setUp()
        self.h = load(SHELL / 'hearth-capture-helper', 'hearth_capture_114')

    def view(self, rows, top, h, w, header=None, footer=None, bar=True):
        """The window as it'd be scrolled to TOP: a toolbar and a footer that stay, and a scroll bar on the right."""
        body = rows[top:top + h]
        if header:
            body = header + body[len(header):]
        if footer:
            body = body[:h - len(footer)] + footer
        out = []
        for i, r in enumerate(body):
            r = bytearray(r)
            if bar:
                thumb = top * h // max(1, len(rows)) <= i < top * h // max(1, len(rows)) + 30
                for x in range(w - 10, w):
                    r[x * 3:x * 3 + 3] = b'\x90\x90\x90' if thumb else b'\x20\x20\x20'
            out.append(bytes(r))
        return self.h.Frame(w, h, b''.join(out))

    def test_ppm_and_png_both_read(self):
        w, h = 40, 12
        rgb = bytes((x * 5 + y * 11) & 255 for y in range(h) for x in range(w * 3))
        ppm = b'P6\n# grim\n%d %d\n255\n' % (w, h) + rgb
        self.assertEqual(self.h.read_ppm(ppm), (w, h, rgb))
        self.assertEqual(self.h.read_png(self.h.rgb_png(w, h, rgb)), (w, h, rgb))
        try:
            from PIL import Image
        except ImportError:
            return
        # A PNG with every filter (as grim's libpng writes them), RGB and RGBA.
        im = Image.frombytes('RGB', (w, h), rgb)
        for mode in ('RGB', 'RGBA'):
            b = io.BytesIO()
            im.convert(mode).save(b, 'PNG', optimize=True)
            self.assertEqual(self.h.read_png(b.getvalue()), (w, h, rgb), mode)

    def test_stitching_with_a_toolbar_a_footer_and_a_scroll_bar(self):
        W, H, TOTAL = 120, 200, 900
        rows = page(W, TOTAL)
        header = [b'\x11\x22\x33' * W] * 6 + [bytes((i * 3) & 255 for i in range(W * 3))] * 4
        footer = [bytes((i * 5 + 1) & 255 for i in range(W * 3))] * 8
        f0 = self.view(rows, 0, H, W, header, footer)
        st = self.h.Stitcher(f0)
        tops = [0]
        for top in (37, 80, 81, 150, 260, 400, 520, 640):
            self.assertEqual(st.add(self.view(rows, top, H, W, header, footer)), 'added', top)
            tops.append(top)
        self.assertEqual(st.add(self.view(rows, 640, H, W, header, footer)), "same")
        # Too far at once (no overlap): not joined.
        self.assertEqual(st.add(self.view(rows, 640 + H, H, W, header, footer)), 'lost')
        # The long picture: the toolbar once, the page from where it shows under the toolbar to the bottom of the
        # last view, the footer once at the very end.
        expect = header + rows[len(header):640 + H - len(footer)] + footer
        self.assertEqual(len(st.rows), len(expect))
        cut = W * 3 - 3 * min(24, W // 10)
        self.assertEqual([r[:cut] for r in st.rows], [r[:cut] for r in expect])
        w, h, rgb = self.h.read_png(st.png())
        self.assertEqual((w, h), (W, len(expect)))

    def test_capture_ends_by_itself_when_it_scrolls_for_you(self):
        W, H = 100, 160
        rows = page(W, 700, seed=3)
        pos = {'top': 0}

        class FakeWheel:
            def scroll(self_inner, clicks=3):
                pos['top'] = min(700 - H, pos['top'] + 50)
        grab = lambda: self.view(rows, pos['top'], H, W)
        t = {'now': 0.0}
        st, why = self.h.scroll_capture((0, 0, W, H), grab=grab, wheel=FakeWheel(), pause=0, now=lambda: t['now'], sleep=lambda s: None)
        self.assertEqual(why, 'end')
        self.assertEqual(len(st.rows), 700)
        with open(os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth-capture', 'scroll.json')) as f:
            self.assertTrue(json.load(f)['done'])

    def test_capture_as_you_scroll_stops_when_asked(self):
        W, H = 100, 160
        rows = page(W, 700, seed=5)
        seq = iter([0, 0, 30, 30, 90, 120, 200])
        state = {'top': 0, 'n': 0}

        def grab():
            state['n'] += 1
            state['top'] = next(seq, state['top'])
            return self.view(rows, state['top'], H, W)
        stop = lambda: 'stopped' if state['n'] >= 7 else ''
        t = {'now': 0.0}
        st, why = self.h.scroll_capture((0, 0, W, H), grab=grab, wheel=None, stop=stop, pause=0, now=lambda: t['now'], sleep=lambda s: None)
        self.assertEqual(why, 'stopped')
        self.assertEqual(len(st.rows), 200 + H)
        self.assertEqual(st.frames, 5)

    def test_scroll_command_and_stop_file(self):
        # The command end to end with a stand-in grim (a PPM of a still window): no movement for 20 s would end it,
        # so scroll-stop finishes it; --cancel throws it away.
        W, H = 64, 48
        ppm = os.path.join(self.tmp, 'shot.ppm')
        rgb = b''.join(page(W, H, seed=9))
        with open(ppm, 'wb') as f:
            f.write(b'P6\n%d %d\n255\n' % (W, H) + rgb)
        self.fake_bin('grim', 'cat "%s"' % ppm)
        hp = str(SHELL / 'hearth-capture-helper')
        for cancel in (False, True):
            p = subprocess.Popen([sys.executable, hp, 'scroll', '--geometry', '10,20,%d,%d' % (W, H)], stdout=subprocess.PIPE, text=True, env=dict(os.environ))
            time.sleep(1.5)
            subprocess.run([sys.executable, hp, 'scroll-stop'] + (['--cancel'] if cancel else []), capture_output=True, env=dict(os.environ), timeout=20)
            outp, _ = p.communicate(timeout=30)
            j = json.loads(outp.strip().splitlines()[-1])
            if cancel:
                self.assertEqual(j, {'ok': False, 'cancelled': True})
            else:
                self.assertTrue(j['ok'], j)
                self.assertEqual((j['width'], j['height']), (W, H))
                with open(j['path'], 'rb') as f:
                    self.assertEqual(self.h.read_png(f.read())[2], rgb)
        r = subprocess.run([sys.executable, hp, 'scroll'], capture_output=True, text=True, timeout=20)
        self.assertEqual(r.returncode, 1)

    def test_wiring(self):
        q = code((SHELL / 'Capture.qml').read_text())
        self.assertIn('{ id: "scroll", g: "scroll", t: "Scrolling"', q)
        self.assertIn('function startScroll(', q)
        self.assertIn('["scroll", "--geometry"', q)
        self.assertIn('run(["scroll-stop"]', q)
        ed = code((SHELL / 'CaptureEditor.qml').read_text())
        for s in ('function share(', '"open-with"', '"send-phone"', 'run(["apps"]', 'run(["phones"]'):
            self.assertIn(s, ed)


# ====================================================================== Share
class FakePhoneLink(threading.Thread):
    """Phone link's service socket, as hearth-phone talks to it: state (one phone connected, one not) and share."""

    def __init__(self, path):
        super().__init__(daemon=True)
        self.path, self.shared = path, []
        self.srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.srv.bind(path)
        self.srv.listen(5)

    def run(self):
        while True:
            try:
                c, _ = self.srv.accept()
            except OSError:
                return
            with c:
                buf = b''
                while b'\n' not in buf:
                    b = c.recv(65536)
                    if not b:
                        break
                    buf += b
                try:
                    req = json.loads(buf.split(b'\n')[0])
                except ValueError:
                    continue
                if req['cmd'] == 'state':
                    data = {'devices': [{'id': 'p1', 'name': 'Pixel 8', 'paired': True, 'reachable': True},
                                        {'id': 'p2', 'name': 'Old phone', 'paired': True, 'reachable': False}]}
                    c.sendall((json.dumps({'id': req['id'], 'ok': True, 'data': data}) + '\n').encode())
                elif req['cmd'] == 'share':
                    self.shared.append(req['args'])
                    c.sendall((json.dumps({'id': req['id'], 'ok': True, 'data': {}}) + '\n').encode())


class Share(Env):
    def setUp(self):
        super().setUp()
        self.h = load(SHELL / 'hearth-capture-helper', 'hearth_capture_114s')

    def test_send_to_the_phone_through_phone_link(self):
        sock = os.path.join(self.tmp, 'connect.sock')
        os.environ['HEARTH_CONNECT_SOCKET'] = sock
        os.environ['HEARTH_CONNECT_BIN'] = self.fake_bin('hearth-connect', 'echo \'{"enabled": true}\'')
        srv = FakePhoneLink(sock)
        srv.start()
        self.addCleanup(srv.srv.close)
        ph = self.h.phones()
        self.assertEqual(ph, {'on': True, 'phones': [{'id': 'p1', 'name': 'Pixel 8'}]})    # only the connected one
        pic = os.path.join(self.tmp, 'shot.png')
        with open(pic, 'wb') as f:
            f.write(self.h.rgb_png(2, 2, b'\0' * 12))
        self.assertEqual(self.h.send_phone(pic, 'p1'), {'ok': True})
        self.assertEqual(srv.shared, [{'id': 'p1', 'paths': [pic]}])
        self.assertFalse(self.h.send_phone(os.path.join(self.tmp, 'gone.png'), 'p1')['ok'])

    def test_phone_link_off(self):
        os.environ['HEARTH_CONNECT_SOCKET'] = os.path.join(self.tmp, 'nobody.sock')
        os.environ['HEARTH_CONNECT_BIN'] = os.path.join(self.tmp, 'not-installed')
        self.assertEqual(self.h.phones(), {'on': False, 'phones': []})

    def test_open_with_an_app_for_pictures(self):
        out = os.path.join(self.tmp, 'viewer.txt')
        self.app('viewer.desktop', 'sh -c \'echo "$1" > %s\' sh %%f' % out, 'image/png;image/jpeg;')
        self.app('gimp.desktop', 'gimp %U', 'image/*;')
        self.app('player.desktop', 'mpv %U', 'video/mp4;')
        names = sorted(a['id'] for a in self.h.apps.apps_for('image/png', default=''))
        self.assertEqual(names, ['gimp.desktop', 'viewer.desktop'])
        pic = os.path.join(self.tmp, 'a b.png')
        with open(pic, 'wb') as f:
            f.write(self.h.rgb_png(1, 1, b'\0\0\0'))
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = self.h.main(['open-with', 'viewer.desktop', pic])
        self.assertEqual(rc, 0, buf.getvalue())
        self.assertTrue(self.wait_for(out))
        with open(out) as f:
            self.assertEqual(f.read().strip(), pic)


if __name__ == '__main__':
    unittest.main()
