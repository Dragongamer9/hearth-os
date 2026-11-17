#!/usr/bin/python3
"""A stand-in game for Firehold's game tests (.github/scripts/firehold-game-test.sh, docs/FIREHOLD.md).

A tiny Wayland client in plain Python (the wire protocol by hand, no libraries): one xdg toplevel, fullscreen, drawing
into shared memory at whatever size the compositor configures, as fast as frame callbacks let it. It writes what it
saw to STATS (JSON, every half second): the configure sizes, the buffer size it draws, and the frame callbacks' rate
over the last two seconds (the frame limit's measure). Its picture: a dark background, a white square of exactly
200 x 200 buffer pixels at (100, 100), and a label with its render size (with Pillow, when it's there), so a screen
picture shows the upscaling: on a 1920 x 1080 screen at Quality the square is 300 x 300 at (150, 150).

  firehold-game-client.py STATS [--app-id ID] [--title TITLE] [--seconds N] [--windowed]
"""
import json
import mmap
import os
import select
import socket
import struct
import sys
import time

args = sys.argv[1:]
if not args:
    sys.exit(__doc__)
STATS = args[0]


def opt(name, default):
    return args[args.index(name) + 1] if name in args else default


APP_ID = opt('--app-id', 'firehold-test-game')
TITLE = opt('--title', 'Firehold test game')
SECONDS = float(opt('--seconds', '120'))
FULLSCREEN = '--windowed' not in args

path = os.path.join(os.environ['XDG_RUNTIME_DIR'], os.environ.get('WAYLAND_DISPLAY', 'wayland-0'))
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(path)

next_id = [2]
handlers = {}   # object id -> (interface name, handler)


def new_id(iface, handler=None):
    i = next_id[0]
    next_id[0] += 1
    handlers[i] = (iface, handler)
    return i


def pad(b):
    return b + b'\0' * (-len(b) % 4)


def s_arg(text):
    b = text.encode() + b'\0'
    return struct.pack('<I', len(b)) + pad(b)


def send(obj, op, payload=b'', fds=()):
    msg = struct.pack('<II', obj, ((8 + len(payload)) << 16) | op) + payload
    if fds:
        socket.send_fds(sock, [msg], list(fds))
    else:
        sock.sendall(msg)


def u32(*v):
    return struct.pack('<%dI' % len(v), *v)


def i32(*v):
    return struct.pack('<%di' % len(v), *v)


globals_ = {}
state = {
    'configured': False, 'pending_serial': None, 'size': (0, 0), 'configures': [], 'frames': [], 'done': 0,
    'closed': False, 'errors': [], 'buffers': [], 'free': [], 'buf_size': (0, 0), 'frame_pending': False,
}


def on_display(op, data):
    if op == 0:   # error
        oid, code = struct.unpack_from('<II', data)
        n = struct.unpack_from('<I', data, 8)[0]
        state['errors'].append('error %d on %d: %s' % (code, oid, data[12:12 + n - 1].decode(errors='replace')))
        print('wayland error:', state['errors'][-1], file=sys.stderr)
        state['closed'] = True


def on_registry(op, data):
    if op == 0:
        name, n = struct.unpack_from('<II', data)
        iface = data[8:8 + n - 1].decode()
        ver = struct.unpack_from('<I', data, 8 + n + (-n % 4))[0]
        globals_[iface] = (name, ver)


def bind(registry, iface, ver, handler=None):
    name, have = globals_[iface]
    v = min(ver, have)
    oid = new_id(iface, handler)
    send(registry, 0, u32(name) + s_arg(iface) + u32(v) + u32(oid))
    return oid


handlers[1] = ('wl_display', on_display)
buf = b''


def dispatch(block=True):
    global buf
    sock.setblocking(block)
    try:
        chunk = sock.recv(65536)
    except BlockingIOError:
        return
    if not chunk:
        state['closed'] = True
        return
    buf += chunk
    while len(buf) >= 8:
        oid, word = struct.unpack_from('<II', buf)
        size, op = word >> 16, word & 0xffff
        if len(buf) < size:
            break
        data, buf = buf[8:size], buf[size:]
        h = handlers.get(oid)
        if h and h[1]:
            h[1](op, data)


def roundtrip():
    done = [False]
    cb = new_id('wl_callback', lambda op, d: done.__setitem__(0, True))
    send(1, 0, u32(cb))
    while not done[0] and not state['closed']:
        dispatch()


registry = new_id('wl_registry', on_registry)
send(1, 1, u32(registry))
roundtrip()
for need in ('wl_compositor', 'wl_shm', 'xdg_wm_base'):
    if need not in globals_:
        sys.exit('no %s' % need)
compositor = bind(registry, 'wl_compositor', 4)
shm = bind(registry, 'wl_shm', 1)


def on_wm_base(op, data):
    if op == 0:   # ping
        send(wm_base, 3, data[:4])


wm_base = bind(registry, 'xdg_wm_base', 2, on_wm_base)
surface = new_id('wl_surface')
send(compositor, 0, u32(surface))


def on_xdg_surface(op, data):
    if op == 0:
        state['pending_serial'] = struct.unpack_from('<I', data)[0]


def on_toplevel(op, data):
    if op == 0:
        w, h = struct.unpack_from('<ii', data)
        state['next_size'] = (w, h)
        state['configures'].append([w, h])
    elif op == 1:
        state['closed'] = True


xdg_surface = new_id('xdg_surface', on_xdg_surface)
send(wm_base, 2, u32(xdg_surface, surface))
toplevel = new_id('xdg_toplevel', on_toplevel)
send(xdg_surface, 1, u32(toplevel))
send(toplevel, 2, s_arg(TITLE))
send(toplevel, 3, s_arg(APP_ID))
if FULLSCREEN:
    send(toplevel, 11, u32(0))   # set_fullscreen(no output: the compositor picks)
send(surface, 6)   # commit (no buffer): asks for the first configure

LABEL_FONT = None
try:
    from PIL import Image, ImageDraw, ImageFont
    try:
        LABEL_FONT = ImageFont.truetype('DejaVuSans-Bold.ttf', 34)
    except OSError:
        try:
            LABEL_FONT = ImageFont.load_default(size=34)
        except TypeError:
            LABEL_FONT = ImageFont.load_default()
except ImportError:
    Image = None


def on_buffer_release(idx):
    def h(op, data):
        if op == 0 and idx < len(state['buffers']):
            state['free'].append(idx)
    return h


def picture(w, h):
    """The frame's pixels (XRGB8888, little endian: B, G, R, X)."""
    row_bg = b'\x30\x24\x20\xff' * w
    rows = [row_bg] * h
    sq = b'\xff\xff\xff\xff' * 200
    for y in range(100, min(300, h)):
        r = bytearray(row_bg)
        r[100 * 4:min(300, w) * 4] = sq[:(min(300, w) - 100) * 4]
        rows[y] = bytes(r)
    data = bytearray(b''.join(rows))
    if Image is not None and w >= 700 and h >= 420:
        label = 'Firehold test game - drawn at %d x %d' % (w, h)
        im = Image.new('RGB', (min(w - 100, 1100), 60), (0x20, 0x24, 0x30))
        ImageDraw.Draw(im).text((10, 8), label, fill=(240, 200, 120), font=LABEL_FONT)
        raw = im.tobytes('raw', 'BGRX')
        lw = im.width * 4
        for y in range(im.height):
            off = ((340 + y) * w + 100) * 4
            data[off:off + lw] = raw[y * lw:(y + 1) * lw]
    return bytes(data)


def make_buffers(w, h):
    for (b, pool, mm, fd) in state['buffers']:
        send(b, 0)    # wl_buffer.destroy
        send(pool, 1)  # wl_shm_pool.destroy
        mm.close()
        os.close(fd)
    state['buffers'], state['free'] = [], []
    stride = w * 4
    size = stride * h
    pix = picture(w, h)
    for i in range(2):
        fd = os.memfd_create('firehold-game-%d' % i)
        os.ftruncate(fd, size)
        mm = mmap.mmap(fd, size)
        mm[:] = pix
        pool = new_id('wl_shm_pool')
        send(shm, 0, u32(pool) + i32(size), fds=[fd])   # create_pool: the fd goes alongside
        b = new_id('wl_buffer', on_buffer_release(i))
        send(pool, 0, u32(b) + i32(0, w, h, stride) + u32(1))   # XRGB8888
        state['buffers'].append((b, pool, mm, fd))
        state['free'].append(i)
    state['buf_size'] = (w, h)


def on_frame(op, data):
    if op == 0:
        state['frames'].append(time.monotonic())
        state['frame_pending'] = False


def draw():
    if not state['free']:
        return False
    i = state['free'].pop(0)
    b = state['buffers'][i][0]
    w, h = state['buf_size']
    send(surface, 1, u32(b) + i32(0, 0))            # attach
    send(surface, 9, i32(0, 0, w, h))                # damage_buffer
    cb = new_id('wl_callback', on_frame)
    send(surface, 3, u32(cb))                        # frame
    send(surface, 6)                                 # commit
    state['frame_pending'] = True
    return True


def write_stats(final=False):
    now = time.monotonic()
    recent = [t for t in state['frames'] if t >= now - 2.0]
    fps = 0.0
    if len(recent) >= 2:
        fps = (len(recent) - 1) / (recent[-1] - recent[0])
    out = {
        'appId': APP_ID, 'configures': state['configures'][-20:], 'bufferSize': list(state['buf_size']),
        'frames': len(state['frames']), 'fps': round(fps, 2), 'errors': state['errors'], 'final': final,
        'time': time.time(),
    }
    tmp = STATS + '.tmp'
    with open(tmp, 'w') as f:
        json.dump(out, f)
    os.replace(tmp, STATS)


end = time.monotonic() + SECONDS
last_stats = 0.0
while not state['closed'] and time.monotonic() < end:
    # Wait for something from the compositor (configure, frame callback, release), at most 50 ms.
    r, _, _ = select.select([sock], [], [], 0.05)
    if r:
        dispatch(block=False)
    if state['pending_serial'] is not None:
        send(xdg_surface, 4, u32(state['pending_serial']))   # ack_configure
        state['pending_serial'] = None
        w, h = state.get('next_size', (0, 0))
        if w <= 0 or h <= 0:
            w, h = 800, 600
        if (w, h) != state['buf_size']:
            make_buffers(w, h)
        state['configured'] = True
        state['frame_pending'] = False
        draw()
    elif state['configured'] and not state['frame_pending']:
        draw()
    if time.monotonic() - last_stats >= 0.5:
        last_stats = time.monotonic()
        write_stats()
write_stats(final=True)
