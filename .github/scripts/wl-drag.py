#!/usr/bin/python3
"""wl-drag.py: a mouse drag (or a click) on the headless screen through wlroots' virtual pointer
(zwlr_virtual_pointer_v1, as wl-scroll.py), for CI's pictures of things you drag: Hearth 3D's box select, sculpting
and painting strokes, Hearth Overlay's placing.

  wl-drag.py W H X0,Y0 X1,Y1 [X2,Y2 ...] [--steps N] [--hold-ms MS] [--button left|right]

W H: the screen's size in pixels (the points are pixels on it). Goes to the first point, presses, moves through the
others in N steps each (default 20, 60 a second), lets go. One point: a click."""
import os
import socket
import struct
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import importlib.util
_spec = importlib.util.spec_from_file_location('wl_scroll', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'wl-scroll.py'))
wls = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wls)


def main():
    a = sys.argv[1:]
    steps, hold, button = 20, 60, 0x110
    pts = []
    W, H = int(a[0]), int(a[1])
    i = 2
    while i < len(a):
        if a[i] == '--steps':
            steps = int(a[i + 1]); i += 2; continue
        if a[i] == '--hold-ms':
            hold = int(a[i + 1]); i += 2; continue
        if a[i] == '--button':
            button = 0x111 if a[i + 1] == 'right' else 0x110; i += 2; continue
        x, y = a[i].split(',')
        pts.append((float(x), float(y)))
        i += 1
    w = wls.Wl()
    reg = w.new()
    w.send(1, 1, struct.pack('<I', reg))
    cb = w.new()
    w.send(1, 0, struct.pack('<I', cb))
    globs = {}
    for obj, op, p in w.events(cb):
        if obj == reg and op == 0:
            name, ln = struct.unpack('<II', p[:8])
            iface = p[8:8 + ln - 1].decode()
            ver = struct.unpack('<I', p[8 + ((ln + 3) & ~3):][:4])[0]
            globs[iface] = (name, ver)
    if 'zwlr_virtual_pointer_manager_v1' not in globs:
        raise SystemExit('no virtual pointer here')
    seat = w.new()
    n, v = globs['wl_seat']
    w.send(reg, 0, struct.pack('<I', n) + w.string('wl_seat') + struct.pack('<II', min(v, 1), seat))
    mgr = w.new()
    n, v = globs['zwlr_virtual_pointer_manager_v1']
    w.send(reg, 0, struct.pack('<I', n) + w.string('zwlr_virtual_pointer_manager_v1') + struct.pack('<II', min(v, 1), mgr))
    vp = w.new()
    w.send(mgr, 0, struct.pack('<II', seat, vp))
    t0 = time.monotonic()
    ms = lambda: int((time.monotonic() - t0) * 1000) & 0xffffffff

    def at(x, y):
        w.send(vp, 1, struct.pack('<IIIII', ms(), int(max(0, min(W - 1, x))), int(max(0, min(H - 1, y))), W, H))
        w.send(vp, 4)

    def flush():
        c = w.new()
        w.send(1, 0, struct.pack('<I', c))
        for _ in w.events(c):
            pass

    at(*pts[0]); flush(); time.sleep(0.15)
    at(pts[0][0] + 1, pts[0][1]); at(*pts[0]); flush(); time.sleep(0.1)
    w.send(vp, 2, struct.pack('<III', ms(), button, 1)); w.send(vp, 4); flush()
    time.sleep(hold / 1000)
    sent = 0
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        for s in range(1, steps + 1):
            f = s / steps
            at(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f)
            sent += 1
            time.sleep(1 / 60)
    flush()
    time.sleep(hold / 1000)
    w.send(vp, 2, struct.pack('<III', ms(), button, 0)); w.send(vp, 4)
    w.send(vp, 8)
    flush()
    print('moves=%d' % sent)


if __name__ == '__main__':
    main()
