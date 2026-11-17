#!/usr/bin/python3
"""hearth_screencast: the screen through xdg-desktop-portal, for hearth-record and hearth-vnc.

The portal is how a program gets the screen on any Wayland desktop: KWin (the kde backend) and labwc (the wlr backend)
alike, with the person asked at the PC the first time. It hands over a PipeWire stream (a node id and a file
descriptor), which GStreamer's pipewiresrc reads.

  Session(conn).screencast(types, cursor, restore_token, persist)   ScreenCast only (recording; view-only remote desktop)
  Session(conn).remote_desktop(types, cursor, restore_token)        RemoteDesktop with a ScreenCast on the same session:
                                                                    the screen and the keyboard and pointer (KWin)
  session.fd()                                                      the PipeWire fd for pipewiresrc (OpenPipeWireRemote)
  session.key(keysym, down) / pointer_abs(x, y) / button(code, down) / scroll(dx, dy)
  session.close()

Calls are synchronous for the caller: the portal's answers (Request.Response signals) are waited for in a private GLib
main context, so this works from any thread and from a program that runs no main loop of its own. python3-gi only.
Nothing here writes a setting; a restore token (the portal's "don't ask again" for this screen) is handed back for the
caller to keep in its state folder, never in a settings file.
"""
import os
import re
import secrets
import threading

try:
    import gi
    gi.require_version('Gio', '2.0')
    from gi.repository import Gio, GLib
except Exception:  # pragma: no cover - no GLib for Python: hearth-record and hearth-vnc say so
    Gio = GLib = None

BUS = 'org.freedesktop.portal.Desktop'
PATH = '/org/freedesktop/portal/desktop'
SCREENCAST = 'org.freedesktop.portal.ScreenCast'
REMOTE = 'org.freedesktop.portal.RemoteDesktop'
REQUEST = 'org.freedesktop.portal.Request'
SESSION = 'org.freedesktop.portal.Session'

MONITOR, WINDOW = 1, 2              # ScreenCast source types
CURSOR_HIDDEN, CURSOR_EMBEDDED, CURSOR_METADATA = 1, 2, 4
KEYBOARD, POINTER = 1, 2            # RemoteDesktop device types


class PortalError(Exception):
    """Something the person can read: the portal isn't there, they said no, the session ended."""


class Cancelled(PortalError):
    pass


def available():
    """(ok, why): xdg-desktop-portal with ScreenCast on the session bus, and RemoteDesktop's version (0 = none)."""
    if Gio is None:
        return False, 'GLib for Python (python3-gi) is missing', 0
    try:
        conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    except Exception as e:
        return False, 'no session bus (%s)' % e, 0
    try:
        sc = _prop(conn, SCREENCAST, 'version')
    except Exception:
        return False, "the desktop's screen-sharing portal (xdg-desktop-portal) isn't running", 0
    try:
        rd = _prop(conn, REMOTE, 'version')
    except Exception:
        rd = 0
    return bool(sc), '' if sc else 'no ScreenCast portal', int(rd or 0)


def _prop(conn, iface, name):
    v = conn.call_sync(BUS, PATH, 'org.freedesktop.DBus.Properties', 'Get', GLib.Variant('(ss)', (iface, name)),
                       GLib.VariantType('(v)'), Gio.DBusCallFlags.NONE, 3000, None)
    return v.unpack()[0]


def _sender_path(conn):
    # The portal's object paths use the caller's unique name with ':' dropped and '.' as '_'.
    return conn.get_unique_name().lstrip(':').replace('.', '_')


def token():
    return 'hearth_' + secrets.token_hex(6)


class Session:
    """One portal session. Every method blocks until the portal answers (or `timeout` seconds pass)."""

    def __init__(self, conn=None, timeout=180):
        if Gio is None:
            raise PortalError('GLib for Python (python3-gi) is missing')
        self.ctx = GLib.MainContext.new()
        self.timeout = timeout
        self.lock = threading.RLock()
        self.handle = None          # the session's object path
        self.streams = []           # [(node_id, {size, position, source_type})]
        self.restore_token = ''
        self.remote = False         # a RemoteDesktop session (keyboard and pointer go through it)
        self.devices = 0
        self.closed = threading.Event()
        self.on_closed = None       # called (from the portal thread) when the desktop ends the session
        self._closed_sub = None
        self.ctx.push_thread_default()
        try:
            self.conn = conn or Gio.DBusConnection.new_for_address_sync(
                Gio.dbus_address_get_for_bus_sync(Gio.BusType.SESSION, None),
                Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION, None, None)
        finally:
            self.ctx.pop_thread_default()
        self.sender = _sender_path(self.conn)

    # ---- the Request / Response dance
    def _request(self, iface, method, args_fmt, args, options):
        """Calls iface.method(args..., options) and waits for its Response: (code, results). Signals are matched on
        their object path (unique to this request's token), not the sender: they come from the portal's unique name."""
        tok = token()
        options = dict(options)
        options['handle_token'] = GLib.Variant('s', tok)
        req_path = '/org/freedesktop/portal/desktop/request/%s/%s' % (self.sender, tok)
        got = {}
        loop = GLib.MainLoop.new(self.ctx, False)

        def on_response(_c, _s, _p, _i, _sig, params):
            code, results = params.unpack()
            got['code'], got['results'] = code, results
            loop.quit()

        self.ctx.push_thread_default()
        try:
            sub = self.conn.signal_subscribe(None, REQUEST, 'Response', req_path, None,
                                             Gio.DBusSignalFlags.NONE, on_response)
            try:
                self.conn.call_sync(BUS, PATH, iface, method, GLib.Variant(args_fmt, tuple(args) + (options,)),
                                    GLib.VariantType('(o)'), Gio.DBusCallFlags.NONE, 10000, None)
            except GLib.Error as e:
                self.conn.signal_unsubscribe(sub)
                raise PortalError(_dbus_reason(e))
            src = GLib.timeout_source_new_seconds(self.timeout)
            src.set_callback(lambda *_: (got.setdefault('code', 'timeout'), loop.quit(), False)[-1])
            src.attach(self.ctx)
            loop.run()
            src.destroy()
            self.conn.signal_unsubscribe(sub)
        finally:
            self.ctx.pop_thread_default()
        code = got.get('code')
        if code == 'timeout':
            raise PortalError("nobody answered the screen-sharing question at the PC in time")
        if code == 1:
            raise Cancelled('the screen sharing was cancelled at the PC')
        if code != 0:
            raise PortalError("the desktop couldn't share the screen (portal answer %s)" % code)
        return got.get('results') or {}

    def _watch_closed(self):
        def on_closed(*_a):
            self.closed.set()
            cb = self.on_closed
            if cb:
                try:
                    cb()
                except Exception:
                    pass
        self.ctx.push_thread_default()
        try:
            self._closed_sub = self.conn.signal_subscribe(None, SESSION, 'Closed', self.handle, None,
                                                          Gio.DBusSignalFlags.NONE, on_closed)
        finally:
            self.ctx.pop_thread_default()
        # The Closed signal is delivered in our context: a small thread keeps iterating it.
        def pump():
            while not self.closed.is_set():
                if not (self.ctx.pending() and self.ctx.iteration(False)):
                    self.closed.wait(0.25)
        threading.Thread(target=pump, daemon=True, name='portal-closed').start()

    # ---- sessions
    def _sources_options(self, types, cursor, restore_token, persist, multiple=False):
        o = {'types': GLib.Variant('u', types), 'multiple': GLib.Variant('b', multiple)}
        try:
            modes = int(_prop(self.conn, SCREENCAST, 'AvailableCursorModes'))
        except Exception:
            modes = 0
        if cursor and modes & cursor:
            o['cursor_mode'] = GLib.Variant('u', cursor)
        if persist:
            o['persist_mode'] = GLib.Variant('u', persist)
            if restore_token:
                o['restore_token'] = GLib.Variant('s', restore_token)
        return o

    def screencast(self, types=MONITOR, cursor=CURSOR_EMBEDDED, restore_token='', persist=0):
        with self.lock:
            r = self._request(SCREENCAST, 'CreateSession', '(a{sv})', (), {'session_handle_token': GLib.Variant('s', token())})
            self.handle = r.get('session_handle')
            if not self.handle:
                raise PortalError('the portal made no session')
            self._request(SCREENCAST, 'SelectSources', '(oa{sv})', (self.handle,),
                          self._sources_options(types, cursor, restore_token, persist))
            self._start(SCREENCAST)
            return self

    def remote_desktop(self, types=MONITOR, cursor=CURSOR_EMBEDDED, restore_token='', persist=2):
        with self.lock:
            r = self._request(REMOTE, 'CreateSession', '(a{sv})', (), {'session_handle_token': GLib.Variant('s', token())})
            self.handle = r.get('session_handle')
            if not self.handle:
                raise PortalError('the portal made no session')
            dev = {'types': GLib.Variant('u', KEYBOARD | POINTER)}
            if persist:
                dev['persist_mode'] = GLib.Variant('u', persist)
                if restore_token:
                    dev['restore_token'] = GLib.Variant('s', restore_token)
            self._request(REMOTE, 'SelectDevices', '(oa{sv})', (self.handle,), dev)
            # The screen on the same session (no persist here: RemoteDesktop's token covers both).
            self._request(SCREENCAST, 'SelectSources', '(oa{sv})', (self.handle,), self._sources_options(types, cursor, '', 0))
            self.remote = True
            self._start(REMOTE)
            return self

    def _start(self, iface):
        r = self._request(iface, 'Start', '(osa{sv})', (self.handle, ''), {})
        self.streams = list(r.get('streams') or [])
        self.restore_token = r.get('restore_token') or ''
        self.devices = int(r.get('devices') or 0)
        if not self.streams:
            raise PortalError('nothing was picked to share')
        self._watch_closed()

    def node(self):
        return int(self.streams[0][0]) if self.streams else 0

    def size(self):
        try:
            w, h = self.streams[0][1].get('size') or (0, 0)
            return int(w), int(h)
        except Exception:
            return 0, 0

    def fd(self):
        """A new PipeWire remote fd for this session's streams (for pipewiresrc fd=N; pass it to the child)."""
        with self.lock:
            ret, fdlist = self.conn.call_with_unix_fd_list_sync(
                BUS, PATH, SCREENCAST, 'OpenPipeWireRemote', GLib.Variant('(oa{sv})', (self.handle, {})),
                GLib.VariantType('(h)'), Gio.DBusCallFlags.NONE, 10000, None, None)
            idx = ret.unpack()[0]
            fd = fdlist.get(idx)
            os.set_inheritable(fd, True)
            return fd

    # ---- input (RemoteDesktop sessions only)
    def _notify(self, method, fmt, args):
        if not self.remote or self.closed.is_set():
            return
        try:
            self.conn.call_sync(BUS, PATH, REMOTE, method, GLib.Variant(fmt, (self.handle, {}) + tuple(args)),
                                None, Gio.DBusCallFlags.NONE, 2000, None)
        except GLib.Error:
            pass

    def key(self, keysym, down):
        self._notify('NotifyKeyboardKeysym', '(oa{sv}iu)', (int(keysym), 1 if down else 0))

    def pointer_abs(self, x, y):
        self._notify('NotifyPointerMotionAbsolute', '(oa{sv}udd)', (self.node(), float(x), float(y)))

    def button(self, code, down):
        self._notify('NotifyPointerButton', '(oa{sv}iu)', (int(code), 1 if down else 0))

    def scroll(self, axis, steps):
        self._notify('NotifyPointerAxisDiscrete', '(oa{sv}ui)', (int(axis), int(steps)))

    def close(self):
        with self.lock:
            if self.handle and not self.closed.is_set():
                try:
                    self.conn.call_sync(BUS, self.handle, SESSION, 'Close', None, None, Gio.DBusCallFlags.NONE, 3000, None)
                except Exception:
                    pass
            self.closed.set()
            if self._closed_sub is not None:
                try:
                    self.conn.signal_unsubscribe(self._closed_sub)
                except Exception:
                    pass
                self._closed_sub = None


def _dbus_reason(e):
    m = str(getattr(e, 'message', e) or e)
    if 'ServiceUnknown' in m or 'UnknownMethod' in m or 'No such interface' in m:
        return "this desktop's portal can't do that (%s)" % re.sub(r'^GDBus\.Error:[\w.]+:\s*', '', m)
    return re.sub(r'^GDBus\.Error:[\w.]+:\s*', '', m)


def caps_size(line):
    """(width, height) from a gst-launch -v caps line, or None."""
    m = re.search(r'width=\(int\)(\d+).*?height=\(int\)(\d+)', line)
    return (int(m.group(1)), int(m.group(2))) if m else None
