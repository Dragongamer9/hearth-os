#!/usr/bin/python3
"""End-to-end checks of Mantel's typing, lock screen and screen sharing on KWin (docs/KWIN-INPUT.md), run by
.github/workflows/kwin-input.yml inside KWin (nested in a headless labwc, OUTER, which takes the pictures), with
Hearth's input method started by KWin, PipeWire, and xdg-desktop-portal reading hearth-portals.conf.

  kwin_e2e.py SHOTS_FOLDER
One line per check; each failure is also a ::error line, findings ::notice lines. Exit 1 if a check failed.
"""
import json
import os
import shutil
import socket
import subprocess
import sys
import threading
import time

import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
SHELL = os.path.join(os.path.dirname(HERE), 'desktop', 'shell')
INSTALLED = '/usr/share/nimbus/desktop/shell'
HT = os.path.join(INSTALLED, 'hearth-type')
CAST_PICKER = os.path.join(SHELL, 'cast-picker.qml')
SHOTS = sys.argv[1]
ENTRY = '/tmp/hearth-entry.txt'
FAILED = []
sys.path.insert(0, SHELL)
import hearth_screencast as hs  # noqa: E402


def esc(s):
    return str(s).replace('%', '%25').replace('\r', '').replace('\n', '%0A')[:1500]


def check(name, ok, detail=''):
    detail = detail if isinstance(detail, str) else json.dumps(detail, default=str)
    print(('ok    ' if ok else 'FAIL  ') + name + ((': ' + detail) if detail else ''), flush=True)
    if ok:
        print('::notice title=KWin: %s::%s' % (name, esc(detail or 'ok')), flush=True)
    else:
        FAILED.append(name)
        print('::error title=KWin: %s::%s' % (name, esc(detail)), flush=True)


def note(name, detail):
    print('::notice title=%s::%s' % (name, esc(detail)), flush=True)


def grim(name):
    env = dict(os.environ, WAYLAND_DISPLAY=os.environ.get('OUTER', ''))
    r = subprocess.run(['grim', os.path.join(SHOTS, name + '.png')], env=env, capture_output=True, text=True)
    if r.returncode != 0:
        print('no picture %s: %s' % (name, r.stderr.strip()), flush=True)


def run(args, timeout=20, env=None, **kw):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=timeout, env=env, **kw)
    except subprocess.TimeoutExpired as e:
        return subprocess.CompletedProcess(args, 124, e.stdout or '', 'timed out')


def entry_text():
    try:
        with open(ENTRY, encoding='utf-8') as f:
            return f.read()
    except OSError:
        return None


def wait_for(fn, timeout=10, step=0.3):
    end = time.time() + timeout
    while time.time() < end:
        v = fn()
        if v:
            return v
        time.sleep(step)
    return fn()


def im_state():
    p = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth', 'im.sock')
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect(p)
        s.sendall(b'S\n')
        a = s.recv(200).decode().strip()
        s.close()
        return a
    except OSError as e:
        return 'unreachable (%s)' % e


def qs_ipc(path, *args):
    r = run(['quickshell', 'ipc', '-p', path, 'call'] + list(args), timeout=10)
    return r.returncode, (r.stdout or '').strip()


# ------------------------------------------------------------------ 1. what KWin offers
r = run(['wayland-info'])
names = set()
for line in r.stdout.splitlines():
    if "interface: '" in line:
        names.add(line.split("interface: '")[1].split("'")[0])
have = {k: k in names for k in ('ext_session_lock_manager_v1', 'zwp_virtual_keyboard_manager_v1', 'zwlr_layer_shell_v1',
                                'zwp_text_input_manager_v3', 'zwp_text_input_manager_v2', 'zwp_input_method_v1',
                                'zkde_screencast_unstable_v1', 'org_kde_kwin_fake_input', 'zwlr_screencopy_manager_v1',
                                'ext_data_control_manager_v1', 'zwlr_data_control_manager_v1')}
note('KWin protocols (an ordinary client)', ', '.join('%s %s' % (k, 'yes' if v else 'no') for k, v in sorted(have.items())))
check('no ext-session-lock on KWin (so Lock.qml hands locking to KWin\'s locker)', not have['ext_session_lock_manager_v1'],
      'ext_session_lock_manager_v1 is %s' % ('offered' if have['ext_session_lock_manager_v1'] else 'not offered'))
check('no virtual-keyboard protocol on KWin (so wtype can\'t type there)', not have['zwp_virtual_keyboard_manager_v1'])
w = run(['wtype', 'x']) if shutil.which('wtype') else None
if w is not None:
    note('wtype on KWin', 'exit %d: %s' % (w.returncode, (w.stderr or '').strip()))

# ------------------------------------------------------------------ 2. typing: the input method and the uinput keyboard
sock = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth', 'im.sock')
check('KWin started Hearth\'s input method (hearth-kwin-im)', bool(wait_for(lambda: os.path.exists(sock), 15)),
      sock if os.path.exists(sock) else 'no %s' % sock)

app = '''
import gi, sys
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk
css = Gtk.CssProvider(); css.load_from_data(b"entry { font-size: 34px; min-height: 70px; } label { font-size: 20px; }")
Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), css, 800)
w = Gtk.Window(title="Hearth typing test"); w.set_default_size(1400, 500)
box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=20); box.set_border_width(40)
box.pack_start(Gtk.Label(label="A GTK text field on KWin: typed by hearth-type (Hearth's input method and uinput keyboard)"), False, False, 0)
e = Gtk.Entry(); box.pack_start(e, False, False, 0)
def changed(x):
    open(sys.argv[1], "w").write(x.get_text())
e.connect("changed", changed)
w.add(box); w.connect("destroy", Gtk.main_quit); w.show_all(); w.maximize(); e.grab_focus()
Gtk.main()
'''
with open('/tmp/hearth-entry-app.py', 'w') as f:
    f.write(app)
with open(ENTRY, 'w') as f:
    f.write('')
gtk = subprocess.Popen([sys.executable, '/tmp/hearth-entry-app.py', ENTRY], env=dict(os.environ, GTK_IM_MODULE='wayland'),
                       stdout=subprocess.DEVNULL, stderr=open(os.path.join(SHOTS, 'gtk-app.log'), 'w'))
time.sleep(5)
st = wait_for(lambda: im_state().startswith('active 1'), 10) and im_state()
check('a focused GTK text field gives Hearth\'s input method a context', bool(st), im_state())
ht_status = run([sys.executable, HT, '--status']).stdout.strip()
note('hearth-type --status', ht_status)

text = 'Grüße 🔥 café ñ'
r = run([sys.executable, HT, '--', text], timeout=20)
got = wait_for(lambda: entry_text() == text, 6) and entry_text()
check('hearth-type types any character into a GTK field on KWin (input method)', entry_text() == text,
      'typed %r, the field has %r (exit %d %s)' % (text, entry_text(), r.returncode, r.stderr.strip()))
r = run([sys.executable, HT, '-k', 'BackSpace', '-k', 'BackSpace'], timeout=20)
want = text[:-2]
wait_for(lambda: entry_text() == want, 6)
check('hearth-type presses keys on KWin (uinput keyboard: BackSpace twice)', entry_text() == want,
      'the field has %r (exit %d %s)' % (entry_text(), r.returncode, r.stderr.strip()))
r = run([sys.executable, HT, '-M', 'ctrl', '-k', 'a', '-m', 'ctrl', '-k', 'Delete'], timeout=20)
wait_for(lambda: entry_text() == '', 6)
check('hearth-type shortcuts on KWin (Ctrl+A, Delete)', entry_text() == '', 'the field has %r' % entry_text())
r = run([sys.executable, HT, '--', 'Hearth on KWin: ü ß é ✓ 😀'], timeout=20)
time.sleep(1.5)
grim('kwin-typed-unicode')

# Mantel's on-screen keyboard (Keyboard.qml), typing through hearth-type on KWin.
osk_qml = os.path.join(INSTALLED, 'osk-kwin-check.qml')
with open(osk_qml, 'w') as f:
    f.write('''import QtQuick
import Quickshell
import Quickshell.Io
ShellRoot {
    Keyboard { id: kb; onCloseRequested: kb.closeKb() }
    IpcHandler {
        target: "osk"
        function open(): void { kb.openKb(); }
        function tap(k: string): void { kb.tapKey(k, false); }
        function state(): string { return kb.stateJson(); }
    }
}
''')
run([sys.executable, HT, '-M', 'ctrl', '-k', 'a', '-m', 'ctrl', '-k', 'Delete'])
osk = subprocess.Popen(['quickshell', '-p', osk_qml], stdout=open(os.path.join(SHOTS, 'osk.log'), 'w'), stderr=subprocess.STDOUT)
time.sleep(6)
qs_ipc(osk_qml, 'osk', 'open')
time.sleep(3)
code, out = qs_ipc(osk_qml, 'osk', 'state')
note('on-screen keyboard state', out[-600:])
for k in 'hello':
    qs_ipc(osk_qml, 'osk', 'tap', k)
    time.sleep(0.6)
wait_for(lambda: (entry_text() or '').endswith('hello'), 8)
check('Mantel\'s on-screen keyboard types on KWin (through hearth-type)', (entry_text() or '').endswith('hello'),
      'the field has %r; keyboard: %s' % (entry_text(), out[-300:]))
grim('kwin-osk-typing')
osk.terminate()
time.sleep(1)

# ------------------------------------------------------------------ 3. KWin's screen sharing through hearth-kwin-cast
outs, version = hs.list_outputs()
check('KWin gives hearth-kwin-cast its screen-sharing protocol (its .desktop file)', version > 0,
      'zkde_screencast_unstable_v1 version %d; screens %s' % (version, [o['name'] for o in outs]))
if outs and version:
    st = hs.Stream(('output', outs[0]['name']), hs.CURSOR_EMBEDDED)
    node = st.wait_node()
    check('KWin makes a PipeWire stream of a screen', node is not None, 'node %s %s' % (node, st.error))
    if node is not None:
        frame = os.path.join(SHOTS, 'cast-frame.png')
        for prop in ('target-object', 'path'):
            r = run(['gst-launch-1.0', '-q', 'pipewiresrc', '%s=%d' % (prop, node), 'num-buffers=4', 'always-copy=true', '!',
                     'videoconvert', '!', 'pngenc', 'snapshot=true', '!', 'filesink', 'location=' + frame], timeout=30)
            if os.path.exists(frame) and os.path.getsize(frame) > 1000:
                break
        check('frames of the shared screen arrive over PipeWire', os.path.exists(frame) and os.path.getsize(frame) > 1000,
              '%s bytes (%s)' % (os.path.getsize(frame) if os.path.exists(frame) else 0, (r.stderr or '')[-300:]))
    st.stop()
ws = '/tmp/hearth-workspace.png'
check('KWin gives hearth-kwin-cast screenshots (ScreenShot2, its .desktop file)', hs.shot(('workspace',), ws),
      '%s bytes' % (os.path.getsize(ws) if os.path.exists(ws) else 0))

# A second window, so the picker's Windows tab has two.
notes = subprocess.Popen([sys.executable, '-c', '''
import gi; gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
w = Gtk.Window(title="Notes – Hearth"); w.set_default_size(900, 600)
t = Gtk.TextView(); t.get_buffer().set_text("Shopping: bread, milk, ember logs\\nCall Dad about Trains 2010")
w.add(t); w.connect("destroy", Gtk.main_quit); w.show_all(); Gtk.main()
'''], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(3)

# ------------------------------------------------------------------ 4. the portals, as an app uses them
bus = Gio.bus_get_sync(Gio.BusType.SESSION)
sender = bus.get_unique_name()[1:].replace('.', '_')
counter = [0]


def portal(iface, method, args_fmt, args, drive=None, timeout=90):
    """A portal call that answers with a Request's Response; returns (response, results) or ('error', text)."""
    counter[0] += 1
    token = 'hearthkwin%d' % counter[0]
    handle = '/org/freedesktop/portal/desktop/request/%s/%s' % (sender, token)
    got = {}
    loop = GLib.MainLoop()

    def on_response(conn, snd, path, i, sig, params, *a):
        got['r'] = params.unpack()
        loop.quit()
    sub = bus.signal_subscribe('org.freedesktop.portal.Desktop', 'org.freedesktop.portal.Request', 'Response', handle,
                               None, Gio.DBusSignalFlags.NO_MATCH_RULE, on_response)
    args = list(args)
    args[-1] = dict(args[-1], handle_token=GLib.Variant('s', token))

    def called(conn, res):
        try:
            conn.call_finish(res)
        except GLib.Error as e:
            got['error'] = str(e)
            loop.quit()
    bus.call('org.freedesktop.portal.Desktop', '/org/freedesktop/portal/desktop', iface, method,
             GLib.Variant(args_fmt, tuple(args)), GLib.VariantType('(o)'), Gio.DBusCallFlags.NONE, timeout * 1000, None, called)
    if drive:
        threading.Thread(target=drive, daemon=True).start()
    GLib.timeout_add_seconds(timeout, loop.quit)
    loop.run()
    bus.signal_unsubscribe(sub)
    if 'error' in got:
        return ('error', got['error'])
    return got.get('r')


def picker_up(timeout=30):
    end = time.time() + timeout
    while time.time() < end:
        code, out = qs_ipc(CAST_PICKER, 'cast', 'state')
        if code == 0 and out.startswith('{'):
            try:
                return json.loads(out.splitlines()[-1])
            except ValueError:
                pass
        time.sleep(0.5)
    return None


SC = 'org.freedesktop.portal.ScreenCast'
r = portal(SC, 'CreateSession', '(a{sv})', [{'session_handle_token': GLib.Variant('s', 'hearthsess')}])
session = r[1].get('session_handle') if r and r[0] == 0 else None
check('ScreenCast portal: a session (Hearth\'s backend)', bool(session), r)
seen = {}
if session:
    r = portal(SC, 'SelectSources', '(oa{sv})', [session, {'types': GLib.Variant('u', 3), 'cursor_mode': GLib.Variant('u', 2),
                                                          'persist_mode': GLib.Variant('u', 2)}])
    check('ScreenCast portal: choosing screens and windows', bool(r) and r[0] == 0, r)

    def drive_cast():
        st = picker_up()
        seen['first'] = st
        if not st:
            return
        time.sleep(3)
        grim('cast-picker-screens')
        qs_ipc(CAST_PICKER, 'cast', 'tab', 'windows')
        time.sleep(2)
        seen['windows'] = qs_ipc(CAST_PICKER, 'cast', 'state')[1]
        grim('cast-picker-windows')
        qs_ipc(CAST_PICKER, 'cast', 'tab', 'screens')
        time.sleep(0.5)
        qs_ipc(CAST_PICKER, 'cast', 'share')
    r = portal(SC, 'Start', '(osa{sv})', [session, '', {}], drive_cast)
    check('the picker came up for the app (cast-picker.qml)', bool(seen.get('first')), seen)
    streams = r[1].get('streams') if r and r[0] == 0 else None
    check('ScreenCast portal: the app gets a PipeWire stream of the screen it was shown', bool(streams), r)
    if streams:
        note('ScreenCast streams', streams)
        check('ScreenCast portal: "remember this" restore data for next time', 'restore_data' in r[1], list(r[1].keys()))
        try:
            res, fds = bus.call_with_unix_fd_list_sync('org.freedesktop.portal.Desktop', '/org/freedesktop/portal/desktop', SC,
                                                       'OpenPipeWireRemote', GLib.Variant('(oa{sv})', (session, {})),
                                                       GLib.VariantType('(h)'), Gio.DBusCallFlags.NONE, 10000, None, None)
            check('ScreenCast portal: the app can open PipeWire for it', fds is not None and fds.get_length() == 1)
        except GLib.Error as e:
            check('ScreenCast portal: the app can open PipeWire for it', False, str(e))
    bus.call_sync('org.freedesktop.portal.Desktop', session, 'org.freedesktop.portal.Session', 'Close', None, None,
                  Gio.DBusCallFlags.NONE, 10000, None)
    time.sleep(2)
    left = run(['pgrep', '-f', 'hearth-kwin-cast stream']).stdout.strip()
    check('closing the session stops the stream', left == '', left)

SS = 'org.freedesktop.portal.Screenshot'
r = portal(SS, 'Screenshot', '(sa{sv})', ['', {'interactive': GLib.Variant('b', False)}])
uri = r[1].get('uri', '') if r and r[0] == 0 else ''
path = uri[7:] if uri.startswith('file://') else ''
from urllib.parse import unquote  # noqa: E402
check('Screenshot portal: the whole desktop, saved in Pictures/Screenshots', bool(path) and os.path.exists(unquote(path)), r)
shot_seen = {}


def drive_shot():
    st = picker_up()
    shot_seen['st'] = st
    if st:
        time.sleep(3)
        grim('shot-picker')
        qs_ipc(CAST_PICKER, 'cast', 'select', '0')
        qs_ipc(CAST_PICKER, 'cast', 'share')
r = portal(SS, 'Screenshot', '(sa{sv})', ['', {'interactive': GLib.Variant('b', True)}], drive_shot)
check('Screenshot portal: "interactive" shows Hearth\'s picker and takes what was picked',
      bool(shot_seen.get('st')) and bool(r) and r[0] == 0, {'picker': shot_seen.get('st'), 'answer': r})
notes.terminate()
gtk.terminate()
time.sleep(1)

# ------------------------------------------------------------------ 5. the lock screen
greeter = next((g for g in ('/usr/libexec/kscreenlocker_greet', '/usr/lib64/libexec/kscreenlocker_greet',
                            '/usr/lib/x86_64-linux-gnu/libexec/kscreenlocker_greet') if os.path.exists(g)), None)
check('KWin\'s screen locker is installed (kscreenlocker)', bool(greeter))
if greeter:
    glog = os.path.join(SHOTS, 'greeter.log')
    g = subprocess.Popen([greeter, '--testing', '--theme', '/usr/share/plasma/look-and-feel/org.hearth.mantel-lock'],
                         stdout=open(glog, 'w'), stderr=subprocess.STDOUT)
    time.sleep(8)
    grim('lock-greeter')
    run([sys.executable, HT, '--', 'not-the-password'])
    run([sys.executable, HT, '-k', 'Return'])
    time.sleep(5)
    grim('lock-wrong-password')
    alive = g.poll() is None
    g.terminate()
    time.sleep(1)
    with open(glog, errors='replace') as f:
        gl = f.read()
    bad = [l for l in gl.splitlines() if 'LockScreen.qml' in l and ('rror' in l or 'is not' in l or 'Cannot' in l)]
    check('Hearth\'s lock screen loads in KWin\'s locker without QML errors', alive and not bad,
          ('still running; ' if alive else 'it ended; ') + ' | '.join(bad[:5]) + ' | ' + gl[-400:])

lock_qml = os.path.join(INSTALLED, 'lock-kwin-check.qml')
with open(lock_qml, 'w') as f:
    f.write('''import QtQuick
import Quickshell
import Quickshell.Io
ShellRoot {
    Lock { id: l }
    IpcHandler {
        target: "lock"
        function lock(): void { l.lock(); }
        function state(): string { return JSON.stringify({ locked: l.locked, onKwin: l.onKwin }); }
    }
}
''')
lk = subprocess.Popen(['quickshell', '-p', lock_qml], stdout=open(os.path.join(SHOTS, 'lock.log'), 'w'), stderr=subprocess.STDOUT)
time.sleep(5)
qs_ipc(lock_qml, 'lock', 'lock')


def active():
    r = run(['gdbus', 'call', '--session', '--dest', 'org.freedesktop.ScreenSaver', '--object-path', '/ScreenSaver',
             '--method', 'org.freedesktop.ScreenSaver.GetActive'])
    return 'true' in r.stdout


ok = wait_for(active, 12)
time.sleep(4)
st = qs_ipc(lock_qml, 'lock', 'state')[1]
grim('lock-real')
check('Lock.qml on KWin locks with KWin\'s locker (and knows it)', ok and '"locked":true' in st.replace(' ', ''),
      'ScreenSaver active %s; Lock.qml %s' % (ok, st))
lk.terminate()

print('%d failed: %s' % (len(FAILED), ', '.join(FAILED)), flush=True)
sys.exit(1 if FAILED else 0)
