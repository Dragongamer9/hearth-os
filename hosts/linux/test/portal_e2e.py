#!/usr/bin/python3
"""End-to-end check of Hearth Files as the file chooser and the file manager (run by desktop-test.yml inside a real
Desktop Mode: labwc + the Hearth shell, a session bus, xdg-desktop-portal with nimbus-portals.conf naming hearth).

Calls org.freedesktop.portal.FileChooser (OpenFile, SaveFile) the way apps do, drives the picker (picker.qml) over
quickshell ipc like a person would with the keyboard or controller, and checks the URIs the app gets back; then a
GTK 3 app's own Open dialog (GtkFileChooserNative with GTK_USE_PORTAL=1), and org.freedesktop.FileManager1.ShowItems.

  portal_e2e.py HOME_DOCUMENTS_FOLDER SHOTS_FOLDER
Prints one line per check; exits 1 if any failed (each failure is also a GitHub ::error line).
"""
import json
import os
import subprocess
import sys
import threading
import time
from urllib.parse import quote

import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
SHELL = os.path.join(os.path.dirname(HERE), 'desktop', 'shell')
PICKER = os.path.join(SHELL, 'picker.qml')
DOCS, SHOTS = sys.argv[1], sys.argv[2]
FAILED = []


def uri(p):
    return 'file://' + quote(p)


def check(name, ok, detail=''):
    detail = detail if isinstance(detail, str) else json.dumps(detail, default=str)
    print(('ok    ' if ok else 'FAIL  ') + name + ((': ' + detail) if detail else ''), flush=True)
    if not ok:
        FAILED.append(name)
        print('::error title=file chooser::%s %s' % (name, str(detail)[:300].replace('\n', ' ')), flush=True)


def ipc(*args, path=PICKER):
    r = subprocess.run(['quickshell', 'ipc', '-p', path, 'call'] + list(args), capture_output=True, text=True, timeout=10)
    return r.returncode, (r.stdout or '').strip(), (r.stderr or '').strip()


def shot(name):
    subprocess.run(['grim', os.path.join(SHOTS, name)], capture_output=True)


def picker_state(timeout=25):
    """The picker's state once its window is up (it's a new Quickshell each time)."""
    end = time.time() + timeout
    while time.time() < end:
        code, out, _ = ipc('picker', 'state')
        if code == 0 and out.startswith('{'):
            try:
                st = json.loads(out.splitlines()[-1])
                if st.get('picking') and st.get('path'):
                    return st
            except ValueError:
                pass
        time.sleep(0.5)
    return None


bus = Gio.bus_get_sync(Gio.BusType.SESSION)
sender = bus.get_unique_name()[1:].replace('.', '_')
counter = [0]


def portal_call(method, options, drive, title='Test'):
    """Calls the portal and drives the picker from another thread; returns (response, results) or None."""
    counter[0] += 1
    token = 'hearthtest%d' % counter[0]
    handle = '/org/freedesktop/portal/desktop/request/%s/%s' % (sender, token)
    got = {}
    loop = GLib.MainLoop()

    def on_response(conn, snd, path, iface, sig, params, *a):
        got['r'] = params.unpack()
        loop.quit()
    sub = bus.signal_subscribe('org.freedesktop.portal.Desktop', 'org.freedesktop.portal.Request', 'Response', handle,
                               None, Gio.DBusSignalFlags.NO_MATCH_RULE, on_response)
    opts = {'handle_token': GLib.Variant('s', token)}
    opts.update(options)
    args = GLib.Variant('(ssa{sv})', ('', title, opts))

    def called(conn, res):
        try:
            conn.call_finish(res)
        except GLib.Error as e:
            got['error'] = str(e)
            loop.quit()
    bus.call('org.freedesktop.portal.Desktop', '/org/freedesktop/portal/desktop', 'org.freedesktop.portal.FileChooser',
             method, args, GLib.VariantType('(o)'), Gio.DBusCallFlags.NONE, 60000, None, called)
    threading.Thread(target=drive, daemon=True).start()
    GLib.timeout_add_seconds(75, loop.quit)
    loop.run()
    bus.signal_unsubscribe(sub)
    if 'error' in got:
        return ('error', got['error'])
    return got.get('r')


def wait_gone(timeout=10):
    end = time.time() + timeout
    while time.time() < end:
        if ipc('picker', 'state')[0] != 0:
            return
        time.sleep(0.4)


# ------------------------------------------------------------------ OpenFile: a file, with the app's filters
seen = {}


def drive_open():
    st = picker_state()
    seen['open'] = st
    if not st:
        return
    shot('picker-1-open.png')
    ipc('filemanager', 'activate', 'data.csv')      # Enter / A on the file picks it


r = portal_call('OpenFile', {
    'current_folder': GLib.Variant('ay', DOCS.encode() + b'\0'),
    'filters': GLib.Variant('a(sa(us))', [('Spreadsheets', [(0, '*.csv'), (1, 'text/csv')]), ('Everything', [(0, '*')])]),
}, drive_open, 'Open a spreadsheet')
st = seen.get('open') or {}
check('the picker opened for OpenFile, in the folder the app asked for', st.get('path') == DOCS, json.dumps(st))
check('the picker shows the app\'s title', st.get('title') == 'Open a spreadsheet', st.get('title'))
ents = st.get('entries') or []
check('the filter shows folders and .csv files only', bool(ents) and 'data.csv' in ents and 'Reports' in ents and 'backup.bin' not in ents, ents)
check('OpenFile returns the picked file', bool(r) and r[0] == 0 and r[1].get('uris') == [uri(os.path.join(DOCS, 'data.csv'))], r)
check('OpenFile says which filter was on', bool(r) and r[0] == 0 and (r[1].get('current_filter') or [''])[0] == 'Spreadsheets', r)
wait_gone()


# ------------------------------------------------------------------ SaveFile: a new name
def drive_save():
    st = picker_state()
    seen['save'] = st
    if not st:
        return
    shot('picker-2-save.png')
    ipc('picker', 'name', 'saved report.txt')
    ipc('picker', 'accept')


r = portal_call('SaveFile', {
    'current_name': GLib.Variant('s', 'Untitled.txt'),
    'current_folder': GLib.Variant('ay', DOCS.encode() + b'\0'),
}, drive_save, 'Save the report')
st = seen.get('save') or {}
check('the picker opened for SaveFile with the suggested name', st.get('name') == 'Untitled.txt', json.dumps(st))
check('SaveFile returns the new file', bool(r) and r[0] == 0 and r[1].get('uris') == [uri(os.path.join(DOCS, 'saved report.txt'))], r)
wait_gone()


# ------------------------------------------------------------------ SaveFile over a file that's there: asks first
def drive_overwrite():
    st = picker_state()
    if not st:
        return
    ipc('picker', 'name', 'data.csv')
    ipc('picker', 'accept')
    time.sleep(1.5)
    code, out, _ = ipc('picker', 'state')
    try:
        seen['overwrite'] = json.loads(out.splitlines()[-1])
    except (ValueError, IndexError):
        seen['overwrite'] = {}
    shot('picker-3-replace.png')
    ipc('picker', 'confirm')


r = portal_call('SaveFile', {'current_folder': GLib.Variant('ay', DOCS.encode() + b'\0')}, drive_overwrite)
check('saving over a file asks before replacing it', (seen.get('overwrite') or {}).get('overlay') == 'confirm', seen.get('overwrite'))
check('…and returns it once you say Replace', bool(r) and r[0] == 0 and r[1].get('uris') == [uri(os.path.join(DOCS, 'data.csv'))], r)
wait_gone()


# ------------------------------------------------------------------ a folder, and Cancel
def drive_folder():
    if picker_state():
        shot('picker-4-folder.png')
        ipc('picker', 'accept')                       # this folder


r = portal_call('OpenFile', {'directory': GLib.Variant('b', True), 'current_folder': GLib.Variant('ay', DOCS.encode() + b'\0')}, drive_folder)
check('OpenFile in folder mode returns the folder', bool(r) and r[0] == 0 and r[1].get('uris') == [uri(DOCS)], r)
wait_gone()


def drive_cancel():
    if picker_state():
        ipc('picker', 'cancel')


r = portal_call('OpenFile', {}, drive_cancel)
check('Cancel tells the app it was cancelled', bool(r) and r[0] == 1, r)
wait_gone()


# ------------------------------------------------------------------ a GTK 3 app's own Open dialog goes through the portal
def gtk_app():
    code = r'''
import os, sys, gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk
d = Gtk.FileChooserNative.new("GTK app", None, Gtk.FileChooserAction.OPEN, "_Open", "_Cancel")
d.set_current_folder(sys.argv[1])
print("RESULT", d.run() == Gtk.ResponseType.ACCEPT and d.get_filename() or "")
'''
    env = dict(os.environ, GTK_USE_PORTAL='1', GDK_BACKEND='wayland')
    return subprocess.Popen([sys.executable, '-c', code, DOCS], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


try:
    gi.require_version('Gtk', '3.0')
    have_gtk = True
except ValueError:
    have_gtk = False
if have_gtk:
    p = gtk_app()
    st = picker_state()
    if st:
        shot('picker-5-gtk-app.png')
        ipc('filemanager', 'activate', 'data.csv')
    try:
        out, err = p.communicate(timeout=40)
    except subprocess.TimeoutExpired:
        p.kill()
        out, err = p.communicate()
    check('a GTK 3 app (GTK_USE_PORTAL=1) gets Hearth Files as its Open dialog', bool(st), err[-300:])
    check('…and the file picked in it', ('RESULT ' + os.path.join(DOCS, 'data.csv')) in out, (out + err)[-300:])
    wait_gone()
else:
    print('skip  GTK 3 isn\'t installed here')


# ------------------------------------------------------------------ "Show in folder": org.freedesktop.FileManager1
ipc('files', 'close', path=SHELL)
time.sleep(1)
try:
    bus.call_sync('org.freedesktop.FileManager1', '/org/freedesktop/FileManager1', 'org.freedesktop.FileManager1', 'ShowItems',
                  GLib.Variant('(ass)', ([uri(os.path.join(DOCS, 'data.csv'))], '')), None, Gio.DBusCallFlags.NONE, 10000, None)
    err = ''
except GLib.Error as e:
    err = str(e)
where = ''
for _ in range(20):
    time.sleep(0.5)
    code, where, _e = ipc('filemanager', 'where', path=SHELL)
    if where.endswith('|data.csv'):
        break
shot('filemanager1-show-items.png')
check('"Show in folder" (FileManager1.ShowItems) opens Hearth Files at the file', where == DOCS + '|data.csv', err or where)
ipc('files', 'close', path=SHELL)

print('%d failed' % len(FAILED))
sys.exit(1 if FAILED else 0)
