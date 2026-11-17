#!/usr/bin/python3
"""hearth_screencast: screen sharing and screenshots for apps on KWin, Hearth's own (docs/KWIN-INPUT.md).

xdg-desktop-portal's ScreenCast (browsers' and Discord's "share your screen", OBS) and Screenshot backends,
served by hearth-portal (D-Bus name org.freedesktop.impl.portal.desktop.hearth, next to Hearth Files' FileChooser):

  * The app asks to share → Hearth's picker (cast-picker.qml, drawn like the rest of Mantel: your screens and your
    windows, each with a picture of what it shows now) → the chosen screen or window comes from KWin as a PipeWire
    stream (zkde_screencast_unstable_v1, through hearth-kwin-cast) → xdg-desktop-portal hands the app that stream.
    The stream ends when the app stops sharing (the portal closes the session) or the app goes away.
  * "Remember this choice" (apps that ask to restore it, persist_mode): the same screen next time without asking,
    while that screen is still there. Nothing is saved by Hearth: the app keeps the restore data.
  * Screenshot: without questions, the whole desktop (what KWin shows), saved in Pictures/Screenshots; with
    "interactive", the same picker (a screen, a window or the whole desktop).

KWin gives its screen-sharing protocol and screenshot interface only to a program whose .desktop file asks for
them, so the calls go through the small helper /usr/libexec/hearth/hearth-kwin-cast (HEARTH_KWIN_CAST overrides).
The window list comes from a KWin script (callDBus back to this service). Only on KWin: on labwc screen sharing
stays xdg-desktop-portal-wlr's (nimbus-portals.conf).

  python3 hearth_screencast.py outputs | shot FILE.png [screen NAME | window ID | workspace] | picker REQUEST.json
  (checks)
"""
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
PICKER_QML = os.path.join(HERE, 'cast-picker.qml')
HELPER_DEFAULT = '/usr/libexec/hearth/hearth-kwin-cast'
SC_IFACE = 'org.freedesktop.impl.portal.ScreenCast'
SS_IFACE = 'org.freedesktop.impl.portal.Screenshot'
SESSION_IFACE = 'org.freedesktop.impl.portal.Session'
REQ_IFACE = 'org.freedesktop.impl.portal.Request'
WIN_IFACE = 'org.hearth.KWinWindows'
WIN_PATH = '/org/hearth/KWinWindows'
SC_VERSION = 5
SS_VERSION = 2
MONITOR, WINDOW, VIRTUAL = 1, 2, 4
CURSOR_HIDDEN, CURSOR_EMBEDDED, CURSOR_METADATA = 1, 2, 4
STREAM_WAIT = 8.0          # s for KWin to make a stream
MAX_THUMBS = 16            # windows with a picture in the picker (the rest: their icon)
RESTORE_VENDOR = 'hearth'
RESTORE_VERSION = 1

_REQ_METHOD = '<arg type="o" name="handle" direction="in"/><arg type="o" name="session_handle" direction="in"/>' \
              '<arg type="s" name="app_id" direction="in"/>'
XML = '''<node>
<interface name="%s">
  <method name="CreateSession">%s<arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/><arg type="a{sv}" name="results" direction="out"/></method>
  <method name="SelectSources">%s<arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/><arg type="a{sv}" name="results" direction="out"/></method>
  <method name="Start">%s<arg type="s" name="parent_window" direction="in"/><arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/><arg type="a{sv}" name="results" direction="out"/></method>
  <property name="AvailableSourceTypes" type="u" access="read"/>
  <property name="AvailableCursorModes" type="u" access="read"/>
  <property name="version" type="u" access="read"/>
</interface>
<interface name="%s">
  <method name="Screenshot"><arg type="o" name="handle" direction="in"/><arg type="s" name="app_id" direction="in"/>
    <arg type="s" name="parent_window" direction="in"/><arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/><arg type="a{sv}" name="results" direction="out"/></method>
  <method name="PickColor"><arg type="o" name="handle" direction="in"/><arg type="s" name="app_id" direction="in"/>
    <arg type="s" name="parent_window" direction="in"/><arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/><arg type="a{sv}" name="results" direction="out"/></method>
  <property name="version" type="u" access="read"/>
</interface>
<interface name="%s"><method name="Close"/><signal name="Closed"/><property name="version" type="u" access="read"/></interface>
<interface name="%s"><method name="Close"/></interface>
<interface name="%s"><method name="Report"><arg type="s" name="token" direction="in"/><arg type="s" name="windows" direction="in"/></method></interface>
</node>''' % (SC_IFACE, _REQ_METHOD, _REQ_METHOD, _REQ_METHOD, SS_IFACE, SESSION_IFACE, REQ_IFACE, WIN_IFACE)

WINDOW_SCRIPT = r'''
var out = [];
var ws = workspace.windowList ? workspace.windowList() : workspace.clientList();
for (var i = 0; i < ws.length; i++) {
    var w = ws[i];
    if (!w || !w.normalWindow || w.skipTaskbar || w.deleted) continue;
    var g = w.frameGeometry;
    out.push({id: String(w.internalId), title: String(w.caption || ""), app: String(w.resourceClass || ""),
              desktop: String(w.desktopFileName || ""), minimized: !!w.minimized, active: w === workspace.activeWindow,
              x: g.x, y: g.y, w: g.width, h: g.height});
}
callDBus("%(name)s", "%(path)s", "%(iface)s", "Report", "%(token)s", JSON.stringify(out));
'''


def log(msg):
    sys.stderr.write('hearth-screencast: %s\n' % msg)
    sys.stderr.flush()


def helper():
    return os.environ.get('HEARTH_KWIN_CAST') or HELPER_DEFAULT


# ---------------------------------------------------------------------- what KWin has

def parse_outputs(text):
    """hearth-kwin-cast outputs → [{name, desc, x, y, w, h, scale}], and the screencast protocol's version."""
    outs, version = [], 0
    for line in text.splitlines():
        if line.startswith('output '):
            f = line[7:].split('\t')
            if len(f) >= 7:
                try:
                    outs.append({'name': f[0], 'desc': f[1], 'x': int(f[2]), 'y': int(f[3]), 'w': int(f[4]),
                                 'h': int(f[5]), 'scale': int(f[6])})
                except ValueError:
                    pass
        elif line.startswith('screencast '):
            try:
                version = int(line.split()[1])
            except (IndexError, ValueError):
                pass
    outs.sort(key=lambda o: (o['x'], o['y'], o['name']))
    return outs, version


def list_outputs():
    try:
        r = subprocess.run([helper(), 'outputs'], capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.SubprocessError) as e:
        log('no screens: %s' % e)
        return [], 0
    return parse_outputs(r.stdout)


def to_png(raw_header, data):
    """KWin's raw picture (hearth-kwin-cast shot) → PNG bytes."""
    w, h, stride, fmt = (int(x) for x in raw_header.split()[:4])
    if len(data) < stride * h:
        raise ValueError('picture cut short')
    # Qt's formats: 4 RGB32, 5 ARGB32, 6 ARGB32 premultiplied (B G R A in memory), 16-18 RGBX/RGBA8888 (R G B A).
    bgra = fmt in (4, 5, 6)
    if not bgra and fmt not in (16, 17, 18):
        raise ValueError('unknown picture format %d' % fmt)
    row = w * 4
    src = data if stride == row else b''.join(data[y * stride:y * stride + row] for y in range(h))
    src = bytes(src[:row * h])
    px = bytearray(src)
    if bgra:
        px[0::4], px[2::4] = src[2::4], src[0::4]
    if fmt in (4, 16):
        px[3::4] = b'\xff' * (w * h)
    raw = b''.join(b'\x00' + bytes(px[y * row:(y + 1) * row]) for y in range(h))

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) +
            chunk(b'IDAT', zlib.compress(raw, 3)) + chunk(b'IEND', b''))


def shot(target, path, cursor=False):
    """A screenshot as a PNG file. target: ('screen', name) | ('window', id) | ('workspace',). True if it worked."""
    try:
        r = subprocess.run([helper(), 'shot'] + list(target) + ['1' if cursor else '0'], capture_output=True,
                           timeout=20)
    except (OSError, subprocess.SubprocessError) as e:
        log('screenshot failed: %s' % e)
        return False
    if r.returncode != 0:
        log('screenshot of %s failed: %s' % (' '.join(target), r.stderr.decode('utf-8', 'replace').strip()))
        return False
    head, _, data = r.stdout.partition(b'\n')
    try:
        png = to_png(head.decode(), data)
    except ValueError as e:
        log('screenshot of %s unreadable: %s' % (' '.join(target), e))
        return False
    tmp = path + '.part'
    with open(tmp, 'wb') as f:
        f.write(png)
    os.replace(tmp, path)
    return True


class WindowLister:
    """KWin's window list: a short KWin script reports it back over D-Bus (Report)."""

    def __init__(self, bus_name):
        self.bus_name = bus_name
        self.waiting = {}

    def report(self, token, text):
        ev = self.waiting.get(token)
        if ev is not None:
            try:
                ev[1] = json.loads(text)
            except ValueError:
                ev[1] = []
            ev[0].set()

    def list(self, timeout=4.0):
        token = 'w%d%d' % (os.getpid(), int(time.monotonic() * 1000))
        ev = [threading.Event(), None]
        self.waiting[token] = ev
        fd, path = tempfile.mkstemp(prefix='hearth-windows-', suffix='.js')
        name = 'hearth-windows-' + token
        try:
            with os.fdopen(fd, 'w') as f:
                f.write(WINDOW_SCRIPT % {'name': self.bus_name, 'path': WIN_PATH, 'iface': WIN_IFACE, 'token': token})
            sid = dbus_kwin(['org.kde.kwin.Scripting.loadScript', path, name], '/Scripting')
            if sid is None:
                return []
            # KWin 6: /Scripting/Script<id>; KWin 5: /<id>.
            if dbus_kwin(['org.kde.kwin.Script.run'], '/Scripting/Script%s' % sid) is None:
                dbus_kwin(['org.kde.kwin.Script.run'], '/%s' % sid)
            ev[0].wait(timeout)
            return ev[1] or []
        finally:
            self.waiting.pop(token, None)
            dbus_kwin(['org.kde.kwin.Scripting.unloadScript', name], '/Scripting')
            try:
                os.unlink(path)
            except OSError:
                pass


def dbus_kwin(method_and_args, path):
    """gdbus call to KWin; the answer's first number (a script id) or '' on success, None on failure."""
    try:
        r = subprocess.run(['gdbus', 'call', '--session', '--dest', 'org.kde.KWin', '--object-path', path,
                            '--method'] + method_and_args, capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.SubprocessError):
        return None
    if r.returncode != 0:
        return None
    import re
    m = re.search(r'-?\d+', r.stdout)
    return m.group(0) if m else ''


# ---------------------------------------------------------------------- the picker

def quickshell():
    return shutil.which('quickshell') or shutil.which('qs')


def build_request(kind, app_id, types, multiple, outputs, windows, thumbs_dir, with_thumbs=True):
    """What the picker shows. kind: 'cast' (ScreenCast) or 'shot' (an interactive screenshot)."""
    req = {'kind': kind, 'app': app_id or '', 'multiple': bool(multiple), 'screens': [], 'windows': [],
           'desktop': kind == 'shot'}
    if types & MONITOR or kind == 'shot':
        for i, o in enumerate(outputs):
            t = os.path.join(thumbs_dir, 'screen-%d.png' % i)
            ok = with_thumbs and shot(('screen', o['name']), t)
            req['screens'].append({'name': o['name'], 'label': o['desc'] or o['name'], 'w': o['w'], 'h': o['h'],
                                   'thumb': t if ok else ''})
    if types & WINDOW or kind == 'shot':
        n = 0
        for w in windows:
            t = ''
            if with_thumbs and not w.get('minimized') and n < MAX_THUMBS:
                p = os.path.join(thumbs_dir, 'window-%d.png' % n)
                if shot(('window', w['id']), p):
                    t = p
                n += 1
            req['windows'].append({'id': w['id'], 'title': w.get('title') or w.get('app') or 'Window',
                                   'app': w.get('desktop') or w.get('app') or '', 'thumb': t,
                                   'minimized': bool(w.get('minimized'))})
    return req


def run_picker(req, holder=None, env=None):
    """cast-picker.qml in its own Quickshell until it answers: {kind: screen|window|desktop, name|id} or None."""
    qs = quickshell()
    if not qs:
        log('Quickshell is missing: no picker')
        return None
    tmp = tempfile.mkdtemp(prefix='hearth-cast-', dir=os.environ.get('XDG_RUNTIME_DIR') or None)
    rq, rs = os.path.join(tmp, 'request.json'), os.path.join(tmp, 'result.json')
    try:
        with open(rq, 'w') as f:
            json.dump(req, f)
        e = dict(env or os.environ)
        e.update({'HEARTH_CAST_REQUEST': rq, 'HEARTH_CAST_RESULT': rs})
        proc = subprocess.Popen([qs, '-p', PICKER_QML], env=e, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, start_new_session=True)
        if holder is not None:
            holder['proc'] = proc
        try:
            proc.wait(timeout=600)
        except subprocess.TimeoutExpired:
            proc.terminate()
        try:
            with open(rs) as f:
                res = json.load(f)
        except (OSError, ValueError):
            return None
        return None if not isinstance(res, dict) or res.get('cancel') else res
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


# ---------------------------------------------------------------------- streams

class Stream:
    """One hearth-kwin-cast stream: its PipeWire node while it runs."""

    def __init__(self, target, cursor):
        self.target, self.cursor, self.node, self.error = target, cursor, None, ''
        args = [helper(), 'stream'] + list(target) + [str(cursor)]
        self.proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                     start_new_session=True)

    def wait_node(self, timeout=STREAM_WAIT):
        import select
        end = time.monotonic() + timeout
        buf = b''
        while time.monotonic() < end:
            r, _, _ = select.select([self.proc.stdout], [], [], max(0.05, end - time.monotonic()))
            if not r:
                continue
            c = os.read(self.proc.stdout.fileno(), 4096)
            if not c:
                break
            buf += c
            for line in buf.decode('utf-8', 'replace').splitlines():
                if line.startswith('node '):
                    try:
                        self.node = int(line.split()[1])
                        return self.node
                    except (IndexError, ValueError):
                        pass
                elif line.startswith('failed'):
                    self.error = line[7:]
                    return None
        self.error = self.error or 'KWin made no stream'
        return None

    def stop(self):
        if self.proc.poll() is None:
            try:
                self.proc.stdin.close()
            except OSError:
                pass
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.terminate()


def cursor_mode(asked):
    """The portal's cursor mode → KWin's (the same numbers); embedded when nothing sensible was asked."""
    for m in (CURSOR_METADATA, CURSOR_EMBEDDED, CURSOR_HIDDEN):
        if asked == m:
            return m
    return CURSOR_EMBEDDED


def restore_target(restore, outputs):
    """Restore data from an earlier "remember this" → the same screen, if it's still there."""
    try:
        vendor, version, data = restore
        if vendor != RESTORE_VENDOR or version != RESTORE_VERSION:
            return None
        name = data.get('screen') if isinstance(data, dict) else None
    except (TypeError, ValueError):
        return None
    return {'kind': 'screen', 'name': name} if name and any(o['name'] == name for o in outputs) else None


def stream_results(streams, choice, outputs, persist):
    """Start's results: the streams (node, position, size, source type) and restore data when it's to be kept."""
    out = []
    for s in streams:
        props = {'source_type': ('u', WINDOW if choice['kind'] == 'window' else MONITOR), 'id': ('s', str(s.node))}
        if choice['kind'] == 'screen':
            o = next((o for o in outputs if o['name'] == choice['name']), None)
            if o:
                props['position'] = ('(ii)', (o['x'], o['y']))
                props['size'] = ('(ii)', (o['w'], o['h']))
        out.append((s.node, props))
    res = {'streams': out}
    if persist and choice['kind'] == 'screen':
        res['persist_mode'] = ('u', persist)
        res['restore_data'] = ('(suv)', (RESTORE_VENDOR, RESTORE_VERSION, ('a{sv}', {'screen': ('s', choice['name'])})))
    return res


def screenshot_path(now=None):
    pics = ''
    try:
        pics = subprocess.run(['xdg-user-dir', 'PICTURES'], capture_output=True, text=True, timeout=2).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        pass
    home = os.path.expanduser('~')
    if not pics or pics == home:
        pics = os.path.join(home, 'Pictures')
    d = os.path.join(pics, 'Screenshots')
    os.makedirs(d, exist_ok=True)
    stamp = time.strftime('%Y-%m-%d %H-%M-%S', time.localtime(now))
    p = os.path.join(d, 'Screenshot from %s.png' % stamp)
    n = 2
    while os.path.exists(p):
        p = os.path.join(d, 'Screenshot from %s (%d).png' % (stamp, n))
        n += 1
    return p


# ---------------------------------------------------------------------- D-Bus

def variant(GLib, sig, val):
    if sig == 'a{sv}':
        return GLib.Variant('a{sv}', {k: variant(GLib, s, v) for k, (s, v) in val.items()})
    if sig == '(suv)':
        vendor, version, (isig, ival) = val
        return GLib.Variant('(suv)', (vendor, version, variant(GLib, isig, ival)))
    return GLib.Variant(sig, val)


def results_variant(GLib, res):
    out = {}
    for k, v in res.items():
        if k == 'streams':
            out[k] = GLib.Variant('a(ua{sv})', [(node, {pk: variant(GLib, ps, pv) for pk, (ps, pv) in props.items()})
                                                for node, props in v])
        else:
            out[k] = variant(GLib, v[0], v[1])
    return out


def unpack_restore(v):
    """restore_data as GLib gives it (vendor, version, inner) → plain Python."""
    try:
        vendor, version, inner = v
        if hasattr(inner, 'unpack'):
            inner = inner.unpack()
        return vendor, version, inner
    except (TypeError, ValueError):
        return None


class Portal:
    """ScreenCast and Screenshot on the hearth-portal connection. busy(n): sessions and requests in flight (the
    service doesn't end while one lasts)."""

    def __init__(self, Gio, GLib, bus_name, busy):
        self.Gio, self.GLib, self.busy = Gio, GLib, busy
        node = Gio.DBusNodeInfo.new_for_xml(XML)
        self.sc_info = node.lookup_interface(SC_IFACE)
        self.ss_info = node.lookup_interface(SS_IFACE)
        self.session_info = node.lookup_interface(SESSION_IFACE)
        self.req_info = node.lookup_interface(REQ_IFACE)
        self.win_info = node.lookup_interface(WIN_IFACE)
        self.windows = WindowLister(bus_name)
        self.sessions = {}

    def register(self, conn, path):
        conn.register_object(path, self.sc_info, self.on_call, self.get_prop, None)
        conn.register_object(path, self.ss_info, self.on_call, self.get_prop, None)
        conn.register_object(WIN_PATH, self.win_info, self.on_windows, None, None)

    def get_prop(self, conn, sender, path, iface, prop):
        GLib = self.GLib
        if iface == SC_IFACE:
            return GLib.Variant('u', {'AvailableSourceTypes': MONITOR | WINDOW, 'AvailableCursorModes': 7,
                                      'version': SC_VERSION}.get(prop, 0))
        if iface == SS_IFACE and prop == 'version':
            return GLib.Variant('u', SS_VERSION)
        if iface == SESSION_IFACE and prop == 'version':
            return GLib.Variant('u', 1)
        return None

    def on_windows(self, conn, sender, path, iface, method, params, inv):
        if method == 'Report':
            token, text = params.unpack()
            self.windows.report(token, text)
        inv.return_value(None)

    def on_call(self, conn, sender, path, iface, method, params, inv):
        GLib = self.GLib
        try:
            if iface == SC_IFACE and method == 'CreateSession':
                handle, session_handle, app_id, options = params.unpack()
                self.create_session(conn, session_handle, app_id)
                inv.return_value(GLib.Variant('(ua{sv})', (0, {'session_id': GLib.Variant('s', session_handle)})))
            elif iface == SC_IFACE and method == 'SelectSources':
                handle, session_handle, app_id, options = params.unpack()
                s = self.sessions.get(session_handle)
                if not s:
                    inv.return_value(GLib.Variant('(ua{sv})', (2, {})))
                    return
                s['types'] = int(options.get('types', MONITOR)) or MONITOR
                s['multiple'] = bool(options.get('multiple', False))
                s['cursor'] = cursor_mode(int(options.get('cursor_mode', CURSOR_EMBEDDED)))
                s['persist'] = int(options.get('persist_mode', 0))
                s['restore'] = unpack_restore(options['restore_data']) if 'restore_data' in options else None
                inv.return_value(GLib.Variant('(ua{sv})', (0, {})))
            elif iface == SC_IFACE and method == 'Start':
                handle, session_handle, app_id, parent, options = params.unpack()
                self.start(conn, handle, session_handle, app_id, inv)
            elif iface == SS_IFACE and method == 'Screenshot':
                handle, app_id, parent, options = params.unpack()
                self.screenshot(conn, handle, app_id, bool(options.get('interactive', False)), inv)
            elif iface == SS_IFACE and method == 'PickColor':
                inv.return_value(GLib.Variant('(ua{sv})', (2, {})))
            else:
                inv.return_dbus_error('org.freedesktop.DBus.Error.UnknownMethod', method)
        except Exception as e:      # never leave an app waiting
            log('%s failed: %r' % (method, e))
            inv.return_value(GLib.Variant('(ua{sv})', (2, {})))

    # ---- sessions
    def create_session(self, conn, session_handle, app_id):
        s = {'app': app_id, 'streams': [], 'types': MONITOR, 'cursor': CURSOR_EMBEDDED, 'persist': 0,
             'restore': None, 'multiple': False, 'conn': conn}

        def on_session(c, sender, path, iface, method, params, inv):
            if method == 'Close':
                self.close_session(session_handle, signal=False)
                inv.return_value(None)
            else:
                inv.return_dbus_error('org.freedesktop.DBus.Error.UnknownMethod', method)
        s['reg'] = conn.register_object(session_handle, self.session_info, on_session, self.get_prop, None)
        self.sessions[session_handle] = s
        self.busy(1)

    def close_session(self, session_handle, signal=True):
        s = self.sessions.pop(session_handle, None)
        if not s:
            return
        for st in s['streams']:
            st.stop()
        if signal:
            try:
                s['conn'].emit_signal(None, session_handle, SESSION_IFACE, 'Closed', None)
            except Exception:
                pass
        try:
            s['conn'].unregister_object(s['reg'])
        except Exception:
            pass
        self.busy(-1)

    def watch_streams(self, session_handle):
        """A stream KWin ended by itself (the window closed, the screen was unplugged) ends the session."""
        s = self.sessions.get(session_handle)
        if not s:
            return False
        if s['streams'] and all(st.proc.poll() is not None for st in s['streams']):
            self.close_session(session_handle)
            return False
        return True

    def request_object(self, conn, handle, holder):
        if self.req_info is None:
            return 0

        def on_close(c, sender, path, iface, m, p, i):
            holder['closed'] = True
            pr = holder.get('proc')
            if pr and pr.poll() is None:
                pr.terminate()
            i.return_value(None)
        try:
            return conn.register_object(handle, self.req_info, on_close, None, None)
        except Exception:
            return 0

    def start(self, conn, handle, session_handle, app_id, inv):
        s = self.sessions.get(session_handle)
        if not s:
            inv.return_value(self.GLib.Variant('(ua{sv})', (2, {})))
            return
        holder = {'closed': False}
        reg = self.request_object(conn, handle, holder)
        self.busy(1)

        def work():
            resp, res = 2, {}
            try:
                outputs, version = list_outputs()
                choice = restore_target(s['restore'], outputs) if s.get('restore') else None
                if choice is None:
                    wins = self.windows.list() if s['types'] & WINDOW else []
                    thumbs = tempfile.mkdtemp(prefix='hearth-cast-thumbs-', dir=os.environ.get('XDG_RUNTIME_DIR') or None)
                    try:
                        req = build_request('cast', app_id, s['types'], s['multiple'], outputs, wins, thumbs)
                        choice = run_picker(req, holder)
                    finally:
                        shutil.rmtree(thumbs, ignore_errors=True)
                if holder['closed'] or choice is None:
                    resp = 1
                else:
                    target = ('output', choice['name']) if choice['kind'] == 'screen' else ('window', choice['id'])
                    st = Stream(target, s['cursor'])
                    if st.wait_node() is None:
                        log('KWin didn\'t share %s: %s' % (' '.join(target), st.error))
                        st.stop()
                    else:
                        s['streams'].append(st)
                        resp, res = 0, stream_results([st], choice, outputs, s['persist'])
                        self.GLib.timeout_add_seconds(2, self.watch_streams, session_handle)
            except Exception as e:
                log('sharing failed: %r' % e)
            finally:
                def done():
                    if reg:
                        conn.unregister_object(reg)
                    self.busy(-1)
                    inv.return_value(self.GLib.Variant('(ua{sv})', (resp, results_variant(self.GLib, res))))
                    return False
                self.GLib.idle_add(done)
        threading.Thread(target=work, daemon=True).start()

    # ---- screenshots
    def screenshot(self, conn, handle, app_id, interactive, inv):
        holder = {'closed': False}
        reg = self.request_object(conn, handle, holder)
        self.busy(1)

        def work():
            resp, res = 2, {}
            try:
                target = ('workspace',)
                if interactive:
                    outputs, _ = list_outputs()
                    wins = self.windows.list()
                    thumbs = tempfile.mkdtemp(prefix='hearth-shot-thumbs-', dir=os.environ.get('XDG_RUNTIME_DIR') or None)
                    try:
                        choice = run_picker(build_request('shot', app_id, MONITOR | WINDOW, False, outputs, wins, thumbs), holder)
                    finally:
                        shutil.rmtree(thumbs, ignore_errors=True)
                    if choice is None or holder['closed']:
                        target = None
                        resp = 1
                    elif choice['kind'] == 'screen':
                        target = ('screen', choice['name'])
                    elif choice['kind'] == 'window':
                        target = ('window', choice['id'])
                if target:
                    p = screenshot_path()
                    if shot(target, p):
                        import urllib.parse
                        resp, res = 0, {'uri': ('s', 'file://' + urllib.parse.quote(p))}
            except Exception as e:
                log('screenshot failed: %r' % e)
            finally:
                def done():
                    if reg:
                        conn.unregister_object(reg)
                    self.busy(-1)
                    inv.return_value(self.GLib.Variant('(ua{sv})', (resp, results_variant(self.GLib, res))))
                    return False
                self.GLib.idle_add(done)
        threading.Thread(target=work, daemon=True).start()


def main(argv):
    if argv[:1] == ['outputs']:
        outs, v = list_outputs()
        print(json.dumps({'screens': outs, 'screencast': v}))
        return 0 if outs else 1
    if argv[:1] == ['shot'] and len(argv) >= 2:
        target = tuple(argv[2:]) or ('workspace',)
        return 0 if shot(target, argv[1]) else 1
    if argv[:1] == ['picker'] and len(argv) == 2:
        with open(argv[1]) as f:
            print(json.dumps(run_picker(json.load(f))))
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
