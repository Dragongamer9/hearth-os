#!/usr/bin/python3
"""One screenshot of KWin's whole screen through org.kde.KWin.ScreenShot2 (KWin started with
KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 and without its file capabilities, as firehold-plasma-shots.sh does), written as
a PNG with the standard library only. D-Bus through dbus-python or, without it, PyGObject (Gio).
  anatase-look-shot.py OUT.png"""
import os
import struct
import sys
import zlib


def capture_dbus_python():
    import dbus
    i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
    r, w = os.pipe()
    try:
        res = i.CaptureWorkspace({'native-resolution': True}, dbus.types.UnixFd(w), timeout=20)
    except dbus.DBusException:
        res = i.CaptureActiveScreen({}, dbus.types.UnixFd(w), timeout=20)
    os.close(w)
    return r, {str(k): v for k, v in res.items()}


def capture_gio():
    import gi
    gi.require_version('Gio', '2.0')
    from gi.repository import Gio, GLib
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    last = None
    for method, opts in (('CaptureWorkspace', {'native-resolution': GLib.Variant('b', True)}), ('CaptureActiveScreen', {})):
        r, w = os.pipe()
        fds = Gio.UnixFDList.new()
        idx = fds.append(w)
        try:
            res, _ = bus.call_with_unix_fd_list_sync('org.kde.KWin', '/org/kde/KWin/ScreenShot2', 'org.kde.KWin.ScreenShot2',
                                                    method, GLib.Variant('(a{sv}h)', (opts, idx)), GLib.VariantType('(a{sv})'),
                                                    Gio.DBusCallFlags.NONE, 20000, fds, None)
        except GLib.Error as e:
            os.close(w)
            os.close(r)
            last = e
            continue
        os.close(w)
        return r, res.unpack()[0]
    raise last


def main(out):
    try:
        r, meta = capture_dbus_python()
        how = 'dbus-python'
    except ImportError:
        r, meta = capture_gio()
        how = 'gio'
    data = bytearray()
    while True:
        c = os.read(r, 1 << 20)
        if not c:
            break
        data += c
    W, H, S = int(meta['width']), int(meta['height']), int(meta['stride'])
    fmt = int(meta.get('format', 5))
    rgba = fmt in (16, 17, 18)        # QImage RGBX/RGBA8888: R, G, B, A in memory; 4-6 (ARGB32): B, G, R, A
    rows = []
    for y in range(H):
        row = data[y * S:y * S + W * 4]
        o = bytearray(W * 3)
        o[0::3] = row[0::4] if rgba else row[2::4]
        o[1::3] = row[1::4]
        o[2::3] = row[2::4] if rgba else row[0::4]
        rows.append(b'\x00' + bytes(o))

    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0)) \
        + chunk(b'IDAT', zlib.compress(b''.join(rows), 6)) + chunk(b'IEND', b'')
    with open(out, 'wb') as f:
        f.write(png)
    print('%s: %dx%d format %d (%s)' % (os.path.basename(out), W, H, fmt, how), file=sys.stderr)


if __name__ == '__main__':
    main(sys.argv[1])
