#!/usr/bin/env python3
"""A pretend OpenRGB SDK server (protocol 3) for Hearth Lights' pictures (lights.sh): the board's PC (four memory
sticks, three front fans, a back fan, the CPU cooler's pump ring, the graphics card's strip, two motherboard headers,
a per-key keyboard, a mouse's logo and wheel, a USB desk strip), answered exactly as OpenRGB answers, so the app's own
protocol code (hosts/hearth-ui/apps/lights/src/orgb.rs) is what draws them. What the app sends is written to LOG, one
line per packet (its id and device), so the pictures job can say the lights were really set.

  lights-fixture.py PORT LOG
"""
import socket
import struct
import sys
import threading

PORT, LOG = int(sys.argv[1]), sys.argv[2]

F_SPEED, F_LR, F_BRIGHT, F_PERLED, F_MODECOL, F_MANUAL = 1, 2, 16, 32, 64, 256


def s(x):
    b = x.encode() + b'\0'
    return struct.pack('<H', len(b)) + b


def mode(name, value, flags, colours=0, color_mode=0):
    return (s(name) + struct.pack('<iIII', value, flags, 0, 100) + struct.pack('<II', 0, 100) + struct.pack('<II', colours, colours)
            + struct.pack('<III', 50, 100, 1) + struct.pack('<I', color_mode) + struct.pack('<H', colours) + b'\xff\x8a\x3d\x00' * colours)


MODES = [mode('Direct', 0, F_PERLED, 0, 1), mode('Static', 1, F_MODECOL | F_MANUAL, 1, 2), mode('Breathing', 2, F_SPEED | F_MODECOL | F_BRIGHT, 1, 2),
         mode('Rainbow Wave', 3, F_SPEED | F_LR | F_BRIGHT)]


def zone(name, leds, kind=1, matrix=None):
    z = s(name) + struct.pack('<iIII', kind, 0, leds, leds)
    if matrix:
        rows, cols = matrix
        cells = struct.pack('<%dI' % (rows * cols), *[(i if i < leds else 0xFFFFFFFF) for i in range(rows * cols)])
        z += struct.pack('<H', 8 + len(cells)) + struct.pack('<II', rows, cols) + cells
    else:
        z += struct.pack('<H', 0)
    return z


def device(kind, name, location, zones):
    n = sum(z[1] for z in zones)
    body = struct.pack('<i', kind) + s(name) + s('Hearth CI') + s('pretend') + s('1.0') + s('0') + s(location)
    body += struct.pack('<H', len(MODES)) + struct.pack('<i', 0) + b''.join(MODES)
    body += struct.pack('<H', len(zones)) + b''.join(zone(*z) for z in zones)
    body += struct.pack('<H', n) + b''.join(s('LED %d' % i) + struct.pack('<I', i) for i in range(n))
    body += struct.pack('<H', n) + b'\xff\x8a\x3d\x00' * n
    return struct.pack('<I', len(body) + 4) + body


DEVICES = [device(1, 'DRAM %d' % (i + 1), 'I2C: /dev/i2c-1, address 0x5%d' % i, [('Stick', 16)]) for i in range(4)] + [
    device(3, 'Front fans', 'HID: /dev/hidraw3', [('Fan 1', 24), ('Fan 2', 24), ('Fan 3', 24)]),
    device(3, 'Back fan', 'I2C: /dev/i2c-2, address 0x30', [('Back fan', 24)]),
    device(3, 'Pump', 'HID: /dev/hidraw4', [('Pump ring', 16)]),
    device(2, 'Graphics card', 'I2C: /dev/i2c-5, address 0x29', [('Strip', 12)]),
    device(0, 'Motherboard', 'I2C: /dev/i2c-0, address 0x4E', [('Header 1', 1), ('Header 2', 1)]),
    device(5, 'Keyboard', 'HID: /dev/hidraw5', [('Keyboard', 104, 5, (6, 18))]),
    device(6, 'Mouse', 'HID: /dev/hidraw6', [('Logo', 1, 0), ('Wheel', 1, 0)]),
    device(4, 'Desk light strip', 'HID: /dev/hidraw7', [('Strip', 60)]),
]


def answer(c, dev, pid, data):
    c.sendall(b'ORGB' + struct.pack('<III', dev, pid, len(data)) + data)


def serve(c):
    with c, open(LOG, 'a') as log:
        buf = b''
        while True:
            try:
                d = c.recv(65536)
            except OSError:
                return
            if not d:
                return
            buf += d
            while len(buf) >= 16:
                if buf[:4] != b'ORGB':
                    return
                dev, pid, n = struct.unpack('<III', buf[4:16])
                if len(buf) < 16 + n:
                    break
                buf = buf[16 + n:]
                if pid == 40:
                    answer(c, 0, 40, struct.pack('<I', 3))
                elif pid == 0:
                    answer(c, 0, 0, struct.pack('<I', len(DEVICES)))
                elif pid == 1 and dev < len(DEVICES):
                    answer(c, dev, 1, DEVICES[dev])
                else:
                    log.write('%d %d %d\n' % (pid, dev, n))
                    log.flush()


srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(('127.0.0.1', PORT))
srv.listen(8)
while True:
    conn, _ = srv.accept()
    threading.Thread(target=serve, args=(conn,), daemon=True).start()
