#!/usr/bin/python3
"""Made-up things for Hearth Forge's pictures (docs/mockups/hearth-forge, the boards' own words and numbers), so each
picture can be laid beside its board. Nothing here is a real account, key or model.

  forge-fixture.py make DIR     DIR/Forge: the models made (the old iron lantern as a real .glb, its preview made by
                                hearth-forge.py's own code, its materials; the others the board names; two waiting),
                                the four installed models (their lock and folders, a sparse file for the space they
                                take), the app's draft (the photo and the words), and DIR/bin: a pretend nvidia-smi
                                (an RTX 5070, 12 GB), secret-tool (a made-up key) and "gamescope" (so the service
                                waits, as it does while you play, and the queue stays as the board shows it)
  forge-fixture.py tripo PORT   a pretend Tripo3D: /user/balance says 2,275 credits
The service itself is hearth-forge.py's own (serve), started by forge.sh on this folder.
"""
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import time

HELPER = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'hosts', 'hearth-ui', 'apps', 'forge', 'helper', 'hearth-forge.py')


def wj(p, v):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        json.dump(v, f, indent=1)


# ------------------------------------------------------------------ the lantern, made of simple parts

class Mesh:
    def __init__(self):
        self.v, self.c, self.f = [], [], []

    def ring(self, y0, y1, r0, r1, col, n=40, spots=None):
        base = len(self.v)
        for i in range(n):
            a = 2 * math.pi * i / n
            for y, r in ((y0, r0), (y1, r1)):
                self.v.append((r * math.cos(a), y, r * math.sin(a)))
                k = 0.85 + 0.3 * (0.5 + 0.5 * math.cos(a * 3 + y * 40))
                cc = col
                if spots and (i * 7 + int(y * 1000)) % spots == 0:
                    cc = (140, 80, 40)
                self.c.append(tuple(min(1.0, x / 255.0 * k) for x in cc))
        for i in range(n):
            j = (i + 1) % n
            a, b, c2, d = base + 2 * i, base + 2 * i + 1, base + 2 * j, base + 2 * j + 1
            self.f += [(a, c2, b), (b, c2, d)]
        return base

    def disc(self, y, r, col, n=40, up=True):
        base = len(self.v)
        self.v.append((0.0, y, 0.0))
        self.c.append(tuple(x / 255.0 for x in col))
        for i in range(n):
            a = 2 * math.pi * i / n
            self.v.append((r * math.cos(a), y, r * math.sin(a)))
            self.c.append(tuple(x / 255.0 for x in col))
        for i in range(n):
            j = (i + 1) % n
            self.f.append((base, base + 1 + i, base + 1 + j) if up else (base, base + 1 + j, base + 1 + i))

    def box(self, cx, cz, w, y0, y1, col):
        base = len(self.v)
        for y in (y0, y1):
            for dx, dz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                self.v.append((cx + dx * w, y, cz + dz * w))
                self.c.append(tuple(x / 255.0 for x in col))
        for i in range(4):
            j = (i + 1) % 4
            self.f += [(base + i, base + j, base + 4 + i), (base + 4 + i, base + j, base + 4 + j)]
        self.f += [(base + 4, base + 5, base + 6), (base + 4, base + 6, base + 7)]

    def arc(self, y, rad, tube, col, n=24, m=8):
        base = len(self.v)
        for i in range(n + 1):
            a = math.pi * i / n
            cx, cy = rad * math.cos(a), y + rad * math.sin(a)
            for k in range(m):
                b = 2 * math.pi * k / m
                self.v.append((cx + tube * math.cos(b) * math.cos(a), cy + tube * math.cos(b) * math.sin(a), tube * math.sin(b)))
                self.c.append(tuple(x / 255.0 for x in col))
        for i in range(n):
            for k in range(m):
                a, b = base + i * m + k, base + i * m + (k + 1) % m
                c2, d = a + m, b + m
                self.f += [(a, c2, b), (b, c2, d)]


def lantern():
    iron, rust, cream = (74, 62, 53), (138, 74, 34), (239, 225, 198)
    m = Mesh()
    for k in range(6):
        y = 0.004 * k
        m.ring(y, y + 0.004, 0.090 - 0.0015 * k, 0.090 - 0.0015 * (k + 1), iron, spots=11)
    m.disc(0.0, 0.09, iron, up=False)
    m.ring(0.024, 0.034, 0.081, 0.081, rust, spots=5)
    m.ring(0.034, 0.040, 0.081, 0.074, iron)
    m.disc(0.040, 0.074, (60, 36, 20))
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        m.box(0.071 * math.cos(a), 0.071 * math.sin(a), 0.005, 0.040, 0.215, iron)
    m.box(0.0, 0.071, 0.003, 0.040, 0.215, iron)
    m.ring(0.124, 0.131, 0.075, 0.075, iron)
    m.ring(0.124, 0.131, 0.071, 0.071, iron)
    m.ring(0.050, 0.118, 0.011, 0.011, cream, n=20)
    m.disc(0.118, 0.011, cream, n=20)
    m.ring(0.118, 0.126, 0.002, 0.0, (255, 241, 194), n=8)
    m.ring(0.215, 0.240, 0.082, 0.058, rust, spots=4)
    m.ring(0.240, 0.252, 0.058, 0.058, iron)
    m.disc(0.252, 0.058, iron)
    m.arc(0.252, 0.035, 0.004, iron)
    return m


def write_glb(path, m):
    pos = b''.join(struct.pack('<3f', *v) for v in m.v)
    col = b''.join(struct.pack('<3f', *c) for c in m.c)
    idx = b''.join(struct.pack('<3I', *f) for f in m.f)
    binb = pos + col + idx
    lo = [min(v[i] for v in m.v) for i in range(3)]
    hi = [max(v[i] for v in m.v) for i in range(3)]
    js = {'asset': {'version': '2.0', 'generator': 'forge-fixture'}, 'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
          'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'COLOR_0': 1}, 'indices': 2}]}],
          'buffers': [{'byteLength': len(binb)}],
          'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': len(pos)}, {'buffer': 0, 'byteOffset': len(pos), 'byteLength': len(col)},
                          {'buffer': 0, 'byteOffset': len(pos) + len(col), 'byteLength': len(idx)}],
          'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': len(m.v), 'type': 'VEC3', 'min': lo, 'max': hi},
                        {'bufferView': 1, 'componentType': 5126, 'count': len(m.c), 'type': 'VEC3'},
                        {'bufferView': 2, 'componentType': 5125, 'count': len(m.f) * 3, 'type': 'SCALAR'}]}
    j = json.dumps(js).encode()
    j += b' ' * ((4 - len(j) % 4) % 4)
    binb += b'\0' * ((4 - len(binb) % 4) % 4)
    out = struct.pack('<III', 0x46546C67, 2, 12 + 8 + len(j) + 8 + len(binb)) + struct.pack('<II', len(j), 0x4E4F534A) + j + struct.pack('<II', len(binb), 0x004E4942) + binb
    with open(path, 'wb') as f:
        f.write(out)


# ------------------------------------------------------------------ pictures

def blob(path, centre, edge, size=(256, 160), glow=None):
    from PIL import Image, ImageDraw, ImageFilter
    im = Image.new('RGB', size, edge)
    d = ImageDraw.Draw(im)
    w, h = size
    for k in range(30, 0, -1):
        t = k / 30
        c = tuple(int(edge[i] + (centre[i] - edge[i]) * (1 - t)) for i in range(3))
        d.ellipse((w / 2 - w * 0.3 * t, h * 0.55 - h * 0.35 * t, w / 2 + w * 0.3 * t, h * 0.55 + h * 0.35 * t), fill=c)
    if glow:
        d.ellipse((w * 0.44, h * 0.35, w * 0.56, h * 0.75), fill=glow)
    im.filter(ImageFilter.GaussianBlur(6)).save(path)


def lantern_photo(path):
    from PIL import Image, ImageDraw, ImageFilter
    im = Image.new('RGB', (400, 400), (36, 28, 20))
    d = ImageDraw.Draw(im)
    for k in range(40, 0, -1):
        t = k / 40
        d.ellipse((200 - 150 * t, 210 - 150 * t, 200 + 150 * t, 210 + 150 * t), fill=(int(36 + 90 * (1 - t)), int(28 + 60 * (1 - t)), int(20 + 20 * (1 - t))))
    d.rectangle((150, 120, 250, 320), outline=(40, 34, 30), width=10)
    d.rectangle((188, 190, 212, 290), fill=(239, 225, 198))
    d.ellipse((180, 150, 220, 200), fill=(255, 214, 140))
    d.polygon([(140, 120), (260, 120), (240, 95), (160, 95)], fill=(110, 60, 28))
    d.arc((165, 40, 235, 120), 180, 360, fill=(50, 44, 40), width=8)
    im.filter(ImageFilter.GaussianBlur(1.2)).save(path)


def gradient(path, a, b):
    from PIL import Image
    im = Image.new('RGB', (256, 128))
    px = im.load()
    for x in range(256):
        for y in range(128):
            t = (x + y) / 384
            px[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))
    im.save(path)


MADE = [
    ('old-iron-lantern', 'Old iron lantern', (0x5a, 0x46, 0x36), (0x1a, 0x15, 0x12)),
    ('plague-doctor-mask', 'Plague doctor mask', (0x6a, 0x6a, 0x72), (0x16, 0x16, 0x1a)),
    ('yellow-wall-lamp', 'Yellow wall lamp', (0x6a, 0x5a, 0x2a), (0x18, 0x16, 0x0e)),
    ('wooden-cart', 'Wooden cart', (0x5a, 0x3a, 0x2a), (0x18, 0x12, 0x10)),
    ('mech-foot', 'Mech foot', (0x4a, 0x5a, 0x6a), (0x12, 0x16, 0x18)),
    ('truck-console-knob', 'Truck console knob', (0x5a, 0x4a, 0x3a), (0x16, 0x12, 0x10)),
]
OLDER = ['Backrooms door', 'Almond water bottle', 'Knight sword', 'Rusty bucket', 'Wooden barrel', 'Torch holder', 'Church bell', 'Market stall',
         'Hay bale', 'Iron gate', 'Office chair', 'Fluorescent light', 'Vending machine', 'Ceiling tile', 'Wet floor sign', 'Mech arm', 'Mech cockpit',
         'Fuel can', 'Ammo crate', 'Radio', 'Old phone', 'Cassette player', 'Water cooler', 'Plant pot', 'Fire axe', 'Shield', 'Helmet stand',
         'Lantern hook', 'Rope coil', 'Anvil', 'Tool chest', 'Workbench']


def make(root):
    forge = os.path.join(root, 'Forge')
    shutil.rmtree(root, ignore_errors=True)
    os.makedirs(forge)
    now = int(time.time())
    # the older ones first (so the six the board names are the newest)
    for i, title in enumerate(OLDER):
        d = os.path.join(forge, title.lower().replace(' ', '-'))
        os.makedirs(d)
        wj(os.path.join(d, 'ask.json'), {'title': title, 'mode': 'photo', 'model': 'hunyuan3d-2', 'realSize': '50 cm tall', 'at': now - 86400 * (3 + i)})
        wj(os.path.join(d, 'done.json'), {'ok': True, 'at': now - 86400 * (3 + i), 'secs': 150, 'model': 'Hunyuan3D 2.0', 'timing': {'shape': 40, 'textures': 110}})
    for i, (mid, title, c, e) in enumerate(MADE):
        d = os.path.join(forge, mid)
        os.makedirs(d)
        blob(os.path.join(d, 'photo-cut.png'), c, e, glow=(255, 214, 140) if i == 0 else None)
        wj(os.path.join(d, 'ask.json'), {'title': title, 'mode': 'photo', 'words': '', 'model': 'hunyuan3d-2', 'detail': 'high', 'textures': 'pbr', 'textureSize': '2k',
                                         'realSize': '30 cm tall', 'seed': 71204, 'photos': ['photo.png'], 'at': now - 600 * (i + 1)})
        wj(os.path.join(d, 'done.json'), {'ok': True, 'at': now - 600 * (i + 1) + 192, 'secs': 192, 'model': 'Hunyuan3D 2.0', 'timing': {'shape': 48, 'textures': 144}})
    d = os.path.join(forge, 'old-iron-lantern')
    a = json.load(open(os.path.join(d, 'ask.json')))
    a['words'] = 'A worn iron lantern, rusty edges, old glass, a candle inside, real size about 30 cm tall'
    a['photoName'] = 'old-lantern.jpg'
    wj(os.path.join(d, 'ask.json'), a)
    lantern_photo(os.path.join(d, 'photo.png'))
    write_glb(os.path.join(d, 'model.glb'), lantern())
    # its preview and stats by hearth-forge.py's own code (what the service does after each model)
    r = subprocess.run([sys.executable, HELPER, 'preview', d], capture_output=True, text=True)
    if r.returncode != 0 or not os.path.isfile(os.path.join(d, 'preview.json')):
        print('::error title=forge fixture::hearth-forge.py preview failed: %s' % (r.stdout + r.stderr)[-600:].replace('\n', '|'))
    st = json.load(open(os.path.join(d, 'stats.json')))
    print('::notice title=forge fixture::the lantern\'s own preview: %d faces, %d points, closed=%s' % (st['faces'], st['points'], st['closed']))
    # the board's numbers for a real Hunyuan3D 2.0 model of it (the made-up one is smaller)
    wj(os.path.join(d, 'stats.json'), {'faces': 48210, 'points': 24120, 'size': [0.174, 0.30, 0.174], 'closed': True, 'maps': ['bumps', 'colour', 'metal', 'roughness'], 'mapsWorkedOut': True})
    for name, ga, gb in (('colour', (0x6b, 0x4a, 0x2a), (0x8a, 0x4a, 0x22)), ('roughness', (0x88, 0x88, 0x88), (0xaa, 0xaa, 0xaa)), ('metal', (0x22, 0x22, 0x22), (0xbb, 0xbb, 0xbb)), ('bumps', (0x80, 0x80, 0xff), (0x60, 0x70, 0xf0))):
        gradient(os.path.join(d, 'map-%s.png' % name), ga, gb)
    # waiting: from words, and from 3 photos
    for mid, ask, photos in (("knights-helmet-dented", {'title': "Knight's helmet, dented", 'mode': 'words', 'words': "Knight's helmet, dented", 'model': 'hunyuan3d-2'}, 0),
                             ('backrooms-chair', {'title': 'Backrooms chair', 'mode': 'photos', 'model': 'hunyuan3d-2'}, 3)):
        dd = os.path.join(forge, mid)
        os.makedirs(dd)
        names = []
        for k in range(photos):
            fn = 'photo.png' if k == 0 else 'photo-%d.png' % (k + 1)
            blob(os.path.join(dd, fn), (0x6a, 0x5a, 0x2a), (0x18, 0x16, 0x0e))
            names.append(fn)
        wj(os.path.join(dd, 'ask.json'), dict(ask, photos=names, at=now - 30))
    wj(os.path.join(root, 'state', 'hearth', 'forge.json'), {'queue': ["knights-helmet-dented", 'backrooms-chair']})
    # the four installed models: their lock, their folders, the space they take (a sparse file: nothing written)
    hub = os.path.join(forge, 'models', 'hub')
    lock = {}
    for k, (mid, repo) in enumerate((('hunyuan3d-2', 'tencent/Hunyuan3D-2'), ('hunyuan3d-2-mini', 'tencent/Hunyuan3D-2mini'), ('stable-fast-3d', 'stabilityai/stable-fast-3d'), ('triposr', 'stabilityai/TripoSR'))):
        commit = ('%x' % (k + 10)) * 40
        os.makedirs(os.path.join(hub, 'models--' + repo.replace('/', '--'), 'snapshots', commit), exist_ok=True)
        lock[mid] = {'done': True, 'at': now - 86400, 'bytes': 7e9, 'sources': [{'repo': repo, 'commit': commit, 'files': 3}]}
    lock['hunyuan3d-2-mini']['sources'].append({'repo': 'tencent/Hunyuan3D-2', 'commit': 'a' * 40, 'files': 2})
    wj(os.path.join(forge, 'models', 'forge-lock.json'), lock)
    with open(os.path.join(hub, 'weights.sparse'), 'wb') as f:
        f.truncate(31 * 10 ** 9)
    # the app's draft: the photo and the words of the one on show
    data = os.path.join(root, 'data', 'hearth', 'forge')
    os.makedirs(data, exist_ok=True)
    shutil.copyfile(os.path.join(d, 'photo.png'), os.path.join(data, 'draft-photo'))
    wj(os.path.join(data, 'draft.json'), {'words': a['words'], 'photo': 'old-lantern.jpg', 'open': 'old-iron-lantern'})
    # pretend programs
    b = os.path.join(root, 'bin')
    os.makedirs(b, exist_ok=True)
    with open(os.path.join(b, 'nvidia-smi'), 'w') as f:
        f.write('#!/bin/sh\necho "NVIDIA GeForce RTX 5070, 12227, 812"\n')
    with open(os.path.join(b, 'secret-tool'), 'w') as f:
        f.write('#!/bin/sh\n[ "$1" = lookup ] && { echo made-up-key-for-pictures; exit 0; }\ncat >/dev/null; exit 0\n')
    for n in ('nvidia-smi', 'secret-tool'):
        os.chmod(os.path.join(b, n), 0o755)
    shutil.copyfile(shutil.which('sleep'), os.path.join(b, 'gamescope'))
    os.chmod(os.path.join(b, 'gamescope'), 0o755)


def tripo(port):
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_GET(self):
            body = json.dumps({'code': 0, 'data': {'balance': 2275, 'frozen': 0}} if self.path.endswith('/user/balance') else {'code': 404}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
    ThreadingHTTPServer(('127.0.0.1', port), H).serve_forever()


if __name__ == '__main__':
    if sys.argv[1] == 'make':
        make(sys.argv[2])
    elif sys.argv[1] == 'tripo':
        tripo(int(sys.argv[2]))
