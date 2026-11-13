#!/usr/bin/env python3
"""Draws the sub themes' wallpapers (Settings › Personalisation › Sub theme; theme/palettes.json "subThemes").

  python3 tools/make-sub-theme-wallpapers.py [out-dir]   (default: hosts/linux/desktop/shell/images/wallpapers/sub)

Writes <out>/<name>.jpg (2560x1440, as the other built-in wallpapers) and <out>/thumbs/<look>-<id>.jpg (320x180, the
Sub theme list's pictures; one for every look's every sub theme, its first one too). Needs numpy and Pillow.
- Hearth: Hearth's own scenes (os/branding/wallpapers/hearth-*.jpg) gradient-mapped into the sub theme's colours; the
  four mixed black ones are Hearth scenes with a many-coloured Hearth fire.
- Steam-style: waves with light streaks. Gaming: a neon grid to the horizon. The black set in both: light streaks
  on black (one set of pictures, shared).
The same run always gives the same pictures (fixed seeds).
"""
import math
import os
import random
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell', 'images', 'wallpapers', 'sub')
SRC = os.path.join(ROOT, 'os', 'branding', 'wallpapers', 'hearth-%s.jpg')
W, H = 2560, 1440
S = W / 1600          # the drawings were laid out at 1600x900
QUALITY = 85


def hx(h):
    h = h.lstrip('#')
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], float)


def t3(h):
    return tuple(int(v) for v in hx(h))


def mixc(a, b, t):
    return '#' + ''.join('%02x' % int(hx(a)[i] * t + hx(b)[i] * (1 - t)) for i in range(3))


def fin(arr):
    return Image.fromarray(np.clip(arr, 0, 255).astype('uint8'))


def seed_of(name):
    return sum(ord(c) * (i + 1) for i, c in enumerate(name)) % 1000


def save(name, arr):
    os.makedirs(OUT, exist_ok=True)
    im = fin(arr) if isinstance(arr, np.ndarray) else arr
    im.save(os.path.join(OUT, name + '.jpg'), quality=QUALITY, optimize=True, progressive=True)
    return im


def thumb(look, sid, im):
    os.makedirs(os.path.join(OUT, 'thumbs'), exist_ok=True)
    t = im.convert('RGB').copy()
    t = t.resize((320, int(320 * t.height / t.width)), Image.LANCZOS)
    if t.height != 180:
        y = (t.height - 180) // 2
        t = t.crop((0, max(0, y), 320, max(0, y) + 180))
    t.save(os.path.join(OUT, 'thumbs', '%s-%s.jpg' % (look, sid)), quality=82, optimize=True)


# ---------------------------------------------------------------- Hearth: gradient-mapped scenes
def scene(name):
    return Image.open(SRC % name).convert('RGB').resize((W, H), Image.LANCZOS)


def gmap(name, stops):
    a = np.asarray(scene(name), float) / 255
    L = np.clip(0.2126 * a[..., 0] + 0.7152 * a[..., 1] + 0.0722 * a[..., 2], 0, 1) ** 0.85
    xs = [s[0] for s in stops]
    cols = np.array([hx(s[1]) for s in stops])
    return np.stack([np.interp(L, xs, cols[:, c]) for c in range(3)], -1)


def stops(bg, panel3, ember, flame):
    top = mixc(flame, '#ffffff', 0.55)
    return [(0, mixc(bg, '#000000', 0.7)), (0.12, bg), (0.3, panel3), (0.5, ember), (0.72, flame), (1, top)]


# The Hearth fire: the mark's flame, in layers of colour.
def cubic(p0, p1, p2, p3, n=40):
    t = np.linspace(0, 1, n)[:, None]
    P = [np.array(p) for p in (p0, p1, p2, p3)]
    return list(map(tuple, ((1 - t) ** 3 * P[0] + 3 * (1 - t) ** 2 * t * P[1] + 3 * (1 - t) * t ** 2 * P[2] + t ** 3 * P[3])))


SEGS = [((256, 168), (298, 222), (340, 262), (340, 332)), ((340, 332), (340, 386), (302, 422), (256, 422)),
        ((256, 422), (210, 422), (172, 386), (172, 332)), ((172, 332), (172, 294), (190, 266), (212, 244)),
        ((212, 244), (210, 276), (222, 298), (242, 306)), ((242, 306), (232, 258), (236, 212), (256, 168))]
PTS = np.array(sum([cubic(*s) for s in SEGS], []))


def fpoly(cx, base, sc, sx=1.0):
    p = PTS.copy()
    p[:, 0] = (p[:, 0] - 256) * sc * sx + cx
    p[:, 1] = (p[:, 1] - 422) * sc + base
    return [tuple(q) for q in p]


def fire(a, cx, base, sc, cols, logs=True, light=1.0, seed=1):
    sc = sc * S
    n = len(cols)
    yy, xx = np.mgrid[0:H, 0:W]
    d = np.sqrt(((xx - cx) / (sc * 520)) ** 2 + ((yy - base + sc * 60) / (sc * 360)) ** 2)
    m = np.clip(1 - d, 0, 1) ** 1.6 * 0.85 * light
    a = a * (1 + m[..., None] * 0.9) + hx(cols[0])[None, None, :] * m[..., None] * 0.55
    lay = Image.new('RGB', (W, H))
    dl = ImageDraw.Draw(lay)
    polys = [(fpoly(cx, base - i * 6 * sc, sc * (1 - i * 0.24), 1 - i * 0.05), c) for i, c in enumerate(cols)]
    for p, c in polys:
        dl.polygon(p, fill=t3(c))
    a = a + np.asarray(lay.filter(ImageFilter.GaussianBlur(40 * sc + 8 * S)), float) * 0.9
    a = a + np.asarray(lay.filter(ImageFilter.GaussianBlur(10 * sc + 3 * S)), float) * 0.5
    im = fin(a)
    d2 = ImageDraw.Draw(im)
    if logs:
        lw, lh = 150 * sc, 26 * sc
        for ang in (-0.32, 0.32):
            ca, sa = np.cos(ang), np.sin(ang)
            pts = [(cx + x * ca - y * sa, base + 12 * sc + x * sa + y * ca) for (x, y) in [(-lw, -lh / 2), (lw, -lh / 2), (lw, lh / 2), (-lw, lh / 2)]]
            d2.polygon(pts, fill=(38, 24, 16))
    for p, c in polys:
        d2.polygon(p, fill=t3(c))
    d2.polygon(fpoly(cx, base - n * 6 * sc, sc * (1 - n * 0.24) * 0.85, 0.8), fill=tuple(int(v * 0.45 + 255 * 0.55) for v in hx(cols[-1])))
    im = im.convert('RGBA')
    sp = Image.new('RGBA', (W, H))
    ds = ImageDraw.Draw(sp)
    r = random.Random(seed)
    for k in range(int(160 * light)):
        x = r.gauss(cx, 90 * sc + 30 * S)
        y = base - r.uniform(40 * sc, 700 * sc + 100 * S)
        s = r.uniform(1, 3) * S
        al = int(255 * r.uniform(.3, 1) * max(0, 1 - (base - y) / (800 * sc + 150 * S)))
        ds.ellipse([x - s, y - s, x + s, y + s], fill=t3(cols[k % n]) + (al,))
    im = Image.alpha_composite(im, sp.filter(ImageFilter.GaussianBlur(1.5 * S)))
    im = Image.alpha_composite(im, sp)
    return np.asarray(im.convert('RGB'), float)


def dark(cols):
    return [(0, '#000000'), (0.16, '#0a0a0a'), (0.4, mixc(cols[0], '#000000', 0.22)), (0.62, mixc(cols[1], '#000000', 0.4)),
            (0.85, mixc(cols[-1], '#000000', 0.6)), (1, mixc(cols[-1], '#ffffff', 0.5))]


def fireplace():
    """Volt's stone fireplace: the hearth itself."""
    im = Image.new('RGB', (W, H), (10, 10, 9))
    d = ImageDraw.Draw(im)
    r = random.Random(5)
    row_h, stone = int(46 * S), int(120 * S)
    for row in range(0, H, row_h):
        off = (row // row_h) % 2 * stone // 2
        for x in range(-stone + off, W + stone, stone):
            w = r.uniform(100, 118) * S
            g = int(r.uniform(16, 26))
            d.rounded_rectangle([x, row + 3 * S, x + w, row + 43 * S], radius=int(8 * S), fill=(g, g, g - 2))
    cx = W * 0.5
    d.rectangle([cx - 420 * S, H * 0.32, cx + 420 * S, H * 0.38], fill=(34, 28, 22))            # mantel
    d.rectangle([cx - 440 * S, H * 0.30, cx + 440 * S, H * 0.33], fill=(48, 38, 30))
    d.rectangle([cx - 360 * S, H * 0.38, cx + 360 * S, H], fill=(26, 24, 22))                    # chimney breast
    d.rounded_rectangle([cx - 250 * S, H * 0.47, cx + 250 * S, H * 0.98], radius=int(40 * S), fill=(5, 5, 5))  # firebox
    d.rectangle([0, H * 0.94, W, H], fill=(20, 18, 16))                                           # floor
    for xx in (cx - 330 * S, cx + 260 * S):                                                       # things on the mantel
        d.rectangle([xx, H * 0.255, xx + 70 * S, H * 0.30], fill=(40, 36, 30))
    return np.asarray(im, float)


def hearth():
    made = {}
    for sid, src, st in [
        ('rose-quartz', 'peaks', stops('#110a0c', '#3a2330', '#ff6f91', '#ffc2d1')),
        ('evergreen', 'cabin', stops('#0a0e0a', '#25351f', '#8fd16a', '#f2d56b')),
        ('midnight', 'fox', stops('#090b12', '#232c48', '#7c9cff', '#ffd166')),
        ('copper-sea', 'lanterns', stops('#08100f', '#1f3836', '#2ec4b6', '#ff9f5a')),
        ('black-red', 'forge', stops('#050505', '#1a0808', '#ff2a2a', '#ff8080')),
        ('black-green', 'topo', stops('#050505', '#081a0c', '#2bd94a', '#a6ff8f')),
        ('black-yellow', 'lanterns', stops('#050505', '#1a1600', '#ffd400', '#fff27a')),
    ]:
        made[sid] = save('hearth-' + sid, gmap(src, st))
    # Hazard: a campfire under the peaks
    c = ['#ff2a2a', '#ff8a1a', '#ffd400']
    made['hazard'] = save('hearth-hazard', fire(gmap('peaks', dark(c)) * 0.8, W * 0.5, H * 0.9, 0.75, c, seed=2))
    # Volt: a stone fireplace
    c = ['#2bd94a', '#a6e81a', '#ffd400']
    made['volt'] = save('hearth-volt', fire(fireplace(), W * 0.5, H * 0.93, 0.9, c, seed=6, light=1.1))
    # Holly: the snowy cabin with a campfire outside
    c = ['#ff2a2a', '#2bd94a', '#c8ffb0']
    made['holly'] = save('hearth-holly', fire(gmap('cabin', dark(['#ff2a2a', '#ff2a2a', '#2bd94a'])) * 0.85, W * 0.3, H * 0.86, 0.6, c, seed=3, light=0.9))
    # Traffic Light: the fox by a campfire under the moon
    c = ['#ff2a2a', '#ffd400', '#2bd94a']
    made['traffic-light'] = save('hearth-traffic-light', fire(gmap('fox', dark(c)) * 0.8, W * 0.72, H * 0.82, 0.6, c, seed=4))
    for sid, im in made.items():
        thumb('hearth', sid, im)
    # Firelight keeps the look's own (animated) fireside: its still for the list.
    thumb('hearth', 'firelight', scene('fireside'))


# ---------------------------------------------------------------- Steam-style and Gaming: drawn
def grad(top, bot):
    t = np.linspace(0, 1, H)[:, None, None]
    return np.broadcast_to(hx(top) * (1 - t) + hx(bot) * t, (H, W, 3)).copy()


def glow(img, cx, cy, r, col, a):
    yy, xx = np.mgrid[0:H, 0:W]
    m = np.clip(1 - np.sqrt(((xx - cx) / r) ** 2 + ((yy - cy) / r) ** 2), 0, 1) ** 2 * a
    return img + hx(col) * m[..., None]


def blurglow(img, y, width, col, blur):
    lay = Image.new('RGB', (W, H))
    ImageDraw.Draw(lay).line([(0, y), (W, y)], fill=t3(col), width=int(width * S))
    return img + np.array(lay.filter(ImageFilter.GaussianBlur(blur * S)), float)


def waves(img, cols, seed):
    im = fin(img)
    d = ImageDraw.Draw(im)
    rnd = random.Random(seed)
    for k, c in enumerate(cols):
        y0 = H * (0.35 + k * 0.13)
        p = rnd.random() * 6
        pts = [(0, H)] + [(x, y0 + 60 * S * math.sin(x / S * 0.0035 + p + k) + 25 * S * math.sin(x / S * 0.009 + p * 2)) for x in range(0, W + 16, 16)] + [(W, H)]
        d.polygon(pts, fill=t3(c))
    return np.array(im.filter(ImageFilter.GaussianBlur(1 * S)), float)


def grid(img, col, hz=0.6):
    lay = Image.new('RGB', (W, H))
    d = ImageDraw.Draw(lay)
    c = t3(col)
    y0 = H * hz
    lw = int(2 * S)
    for i in range(1, 26):
        y = y0 + (H - y0) * (i / 25) ** 2.1
        d.line([(0, y), (W, y)], fill=c, width=lw)
    for x in range(-30, 31):
        d.line([(W / 2 + x * 20 * S, y0), (W / 2 + x * 190 * S, H)], fill=c, width=lw)
    g = np.array(lay.filter(ImageFilter.GaussianBlur(3 * S)), float) * 1.4 + np.array(lay, float) * 0.8
    return img + g * 0.6


def streaks(img, cols, seed):
    lay = Image.new('RGB', (W, H))
    d = ImageDraw.Draw(lay)
    rnd = random.Random(seed)
    for c in cols:
        for _ in range(3):
            x0 = rnd.uniform(-400, W / S * 0.6) * S
            y0 = H * rnd.uniform(0.2, 1.0)
            L = rnd.uniform(700, 1500) * S
            a = -0.42 + rnd.uniform(-.06, .06)
            d.line([(x0, y0), (x0 + L * math.cos(a), y0 + L * math.sin(a))], fill=t3(c), width=int(rnd.uniform(3, 9) * S))
    g1 = np.array(lay.filter(ImageFilter.GaussianBlur(26 * S)), float)
    g2 = np.array(lay.filter(ImageFilter.GaussianBlur(4 * S)), float)
    return img + g1 * 1.6 + g2 * 0.9


def steam_gaming():
    for nm, top, bot, cols, acc in [('crimson', '#140f11', '#2a1a1f', ['#22151a', '#2a181e', '#331b23'], '#e5484d'),
                                    ('emerald', '#0f1714', '#16302a', ['#132620', '#162e26', '#19372c'], '#1fbf75'),
                                    ('royal', '#13111c', '#231d3a', ['#1d1834', '#221c3e', '#282048'], '#9b6bff'),
                                    ('graphite', '#0f0f10', '#1f1f22', ['#1b1b1e', '#202023', '#252528'], '#e4e6ea')]:
        a = glow(grad(top, bot), W * 0.75, H * 0.2, H * 0.9, acc, 0.22)
        a = waves(a, cols, seed_of(nm) % 100)
        thumb('steam', nm, save('steam-' + nm, streaks(a, [acc], seed_of(nm) % 50) * 0.85 + a * 0.15))
    for nm, top, bot, c1, c2 in [('toxic', '#050805', '#0e1a0b', '#a6ff2e', '#00d5ff'), ('inferno', '#0a0503', '#2a0c05', '#ff4d1a', '#ffcc00'),
                                 ('ultraviolet', '#07050d', '#1d0f33', '#b14dff', '#2effd5'), ('frostbite', '#04070c', '#0f2236', '#8ad8ff', '#ffffff')]:
        a = glow(grad(top, bot), W * 0.5, H * 0.6, H * 0.7, c2, 0.35)
        thumb('gaming', nm, save('gaming-' + nm, blurglow(grid(a, c1), H * 0.6, 6, c2, 10)))
    for nm, cols in [('black-red', ['#ff2a2a']), ('black-green', ['#2bd94a']), ('black-yellow', ['#ffd400']), ('hazard', ['#ff2a2a', '#ffd400']),
                     ('volt', ['#2bd94a', '#ffd400']), ('holly', ['#ff2a2a', '#2bd94a']), ('traffic-light', ['#ff2a2a', '#ffd400', '#2bd94a'])]:
        im = save('streaks-' + nm, streaks(grad('#030303', '#0b0b0b'), cols, len(nm) * 7))
        thumb('steam', nm, im)
        thumb('gaming', nm, im)
    # The first sub themes keep the looks' own moving wallpapers (Slate Tide, Neon HUD): their stills for the list.
    shell_images = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell', 'images')
    thumb('steam', 'steam-blue', Image.open(os.path.join(shell_images, 'wall-tide.jpg')))
    a = glow(grad('#05070b', '#0b1626'), W * 0.5, H * 0.6, H * 0.7, '#ff3dc8', 0.35)
    thumb('gaming', 'cyber', fin(blurglow(grid(a, '#22e4ff'), H * 0.6, 6, '#ff3dc8', 10)))


if __name__ == '__main__':
    hearth()
    steam_gaming()
    total = sum(os.path.getsize(os.path.join(dp, f)) for dp, _, fs in os.walk(OUT) for f in fs)
    print('wrote %s (%.1f MB)' % (OUT, total / 1e6))
