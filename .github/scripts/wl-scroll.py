#!/usr/bin/python3
"""wl-scroll.py: scrolls whatever is under the middle of the screen, as a mouse wheel or a touchpad would, through
wlroots' virtual pointer (zwlr_virtual_pointer_v1; sway has it). For hearth-apps-rust-test.sh's scroll measurement:
headless sway has no input devices. Plain Wayland wire protocol, no libraries.

  wl-scroll.py SECONDS touchpad|wheel [PX_PER_STEP] [STEPS_PER_S]

touchpad: continuous scrolling (axis source finger) of PX_PER_STEP px per step (default 8 px, 60 steps a second);
wheel: one notch (discrete) per step (default 12 steps a second). Down for the first half, up for the second.
Prints one line: steps sent."""
import os
import socket
import struct
import sys
import time


def fixed(v):
    return int(round(v * 256)) & 0xffffffff


class Wl:
    def __init__(self):
        path = os.path.join(os.environ['XDG_RUNTIME_DIR'], os.environ.get('WAYLAND_DISPLAY', 'wayland-0'))
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(path)
        self.next = 2
        self.buf = b''

    def new(self):
        i = self.next
        self.next += 1
        return i

    def send(self, obj, op, payload=b''):
        self.s.sendall(struct.pack('<II', obj, ((8 + len(payload)) << 16) | op) + payload)

    @staticmethod
    def string(t):
        b = t.encode() + b'\0'
        return struct.pack('<I', len(b)) + b + b'\0' * ((4 - len(b) % 4) % 4)

    def events(self, until_cb):
        """Reads events until wl_callback `until_cb` is done; yields (obj, opcode, payload)."""
        while True:
            while len(self.buf) >= 8:
                obj, so = struct.unpack('<II', self.buf[:8])
                size, op = so >> 16, so & 0xffff
                if len(self.buf) < size:
                    break
                payload, self.buf = self.buf[8:size], self.buf[size:]
                if obj == until_cb and op == 0:
                    return
                if obj == 1 and op == 0:      # wl_display.error
                    raise SystemExit('wayland error: %r' % payload)
                yield obj, op, payload
            d = self.s.recv(65536)
            if not d:
                raise SystemExit('the compositor closed the connection')
            self.buf += d


def main():
    secs = float(sys.argv[1]) if len(sys.argv) > 1 else 3.0
    kind = sys.argv[2] if len(sys.argv) > 2 else 'touchpad'
    px = float(sys.argv[3]) if len(sys.argv) > 3 else 8.0
    rate = float(sys.argv[4]) if len(sys.argv) > 4 else (60.0 if kind == 'touchpad' else 12.0)
    w = Wl()
    reg = w.new()
    w.send(1, 1, struct.pack('<I', reg))             # wl_display.get_registry
    cb = w.new()
    w.send(1, 0, struct.pack('<I', cb))              # wl_display.sync
    globs = {}
    for obj, op, p in w.events(cb):
        if obj == reg and op == 0:
            name, ln = struct.unpack('<II', p[:8])
            iface = p[8:8 + ln - 1].decode()
            ver = struct.unpack('<I', p[8 + ((ln + 3) & ~3):][:4])[0]
            globs[iface] = (name, ver)
    if 'zwlr_virtual_pointer_manager_v1' not in globs or 'wl_seat' not in globs:
        raise SystemExit('no virtual pointer here (%s)' % ', '.join(sorted(globs)))
    seat = w.new()
    n, v = globs['wl_seat']
    w.send(reg, 0, struct.pack('<I', n) + w.string('wl_seat') + struct.pack('<II', min(v, 1), seat))
    mgr = w.new()
    n, v = globs['zwlr_virtual_pointer_manager_v1']
    w.send(reg, 0, struct.pack('<I', n) + w.string('zwlr_virtual_pointer_manager_v1') + struct.pack('<II', min(v, 1), mgr))
    vp = w.new()
    w.send(mgr, 0, struct.pack('<II', seat, vp))     # create_virtual_pointer(seat, id)
    t0 = time.monotonic()
    ms = lambda: int((time.monotonic() - t0) * 1000) & 0xffffffff
    # To the middle of the screen (extent 1000 x 1000: any output size).
    w.send(vp, 1, struct.pack('<IIIII', ms(), 500, 500, 1000, 1000))
    w.send(vp, 4)
    cb = w.new()
    w.send(1, 0, struct.pack('<I', cb))
    for _ in w.events(cb):
        pass
    time.sleep(0.3)
    steps = 0
    start = time.monotonic()
    while True:
        el = time.monotonic() - start
        if el >= secs:
            break
        sign = 1.0 if el < secs / 2 else -1.0
        if kind == 'wheel':
            w.send(vp, 5, struct.pack('<I', 0))                                      # axis_source wheel
            w.send(vp, 7, struct.pack('<IIIi', ms(), 0, fixed(15.0 * sign), int(sign)))  # axis_discrete
        else:
            w.send(vp, 5, struct.pack('<I', 1))                                      # axis_source finger
            w.send(vp, 3, struct.pack('<III', ms(), 0, fixed(px * sign)))            # axis
        w.send(vp, 4)                                                                # frame
        steps += 1
        nxt = start + steps / rate
        time.sleep(max(0.0, nxt - time.monotonic()))
    w.send(vp, 8)                                                                    # destroy
    cb = w.new()
    w.send(1, 0, struct.pack('<I', cb))
    for _ in w.events(cb):
        pass
    print('steps=%d' % steps)


if __name__ == '__main__':
    main()
