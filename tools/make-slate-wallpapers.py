#!/usr/bin/env python3
"""Draws Mantel's Steam-style ("slate") wallpapers from code: original abstract art, no outside artwork.

  python3 tools/make-slate-wallpapers.py [out-dir]   (default: hosts/linux/desktop/shell/images)

Writes the still wallpapers to <out>/wallpapers/slate-*.jpg (2560x1440), the layers the moving ones are made of
(blob-*.png, ridge-*.png) and their small previews (wall-tide/ridges/motes.jpg). Needs numpy and Pillow.
The same seed always gives the same pictures.
"""
import os
import sys

import numpy as np
from PIL import Image, ImageFilter

W, H = 2560, 1440
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), '..', 'hosts', 'linux', 'desktop', 'shell', 'images')
rng = np.random.default_rng(1792)


def hexrgb(h):
    h = h.lstrip('#')
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float32)


BG, BG2, SLATE, BLUE, SKY = (hexrgb(c) for c in ('#0d1117', '#1b2838', '#2a475e', '#1a9fff', '#66c0f4'))


def grid(w=W, h=H):
    y, x = np.mgrid[0:h, 0:w].astype(np.float32)
    return x / w, y / h


def vgrad(top, bottom, w=W, h=H, curve=1.0):
    _, y = grid(w, h)
    t = (y ** curve)[..., None]
    return top * (1 - t) + bottom * t


def glow(img, cx, cy, rx, ry, color, strength):
    x, y = grid(img.shape[1], img.shape[0])
    d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
    return img + np.exp(-d)[..., None] * color * strength


def value_noise(w, h, cell, seed):
    r = np.random.default_rng(seed)
    gw, gh = w // cell + 2, h // cell + 2
    g = r.random((gh, gw)).astype(np.float32)
    small = Image.fromarray((g * 255).astype(np.uint8))
    return np.asarray(small.resize((gw * cell, gh * cell), Image.BICUBIC), dtype=np.float32)[:h, :w] / 255


def fbm(w, h, seed, octaves=5, cell=320):
    acc = np.zeros((h, w), np.float32)
    amp, tot = 1.0, 0.0
    for o in range(octaves):
        acc += value_noise(w, h, max(4, cell >> o), seed + o) * amp
        tot += amp
        amp *= 0.5
    return acc / tot


def grain(img, k=3.0):
    return img + rng.normal(0, k, img.shape[:2])[..., None]


def save(img, path, q=90):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)).save(path, quality=q, optimize=True, progressive=True)
    print(path, os.path.getsize(path) // 1024, 'KB')


# ---------------------------------------------------------------- stills
def dusk():
    img = vgrad(BG, BG2, curve=1.3)
    img = glow(img, 0.78, 0.12, 0.55, 0.45, SLATE, 0.9)
    img = glow(img, 0.82, 0.08, 0.18, 0.14, SKY, 0.22)
    img = glow(img, 0.10, 1.05, 0.6, 0.35, BLUE, 0.10)
    # a soft diagonal sheen
    x, y = grid()
    band = np.exp(-((x * 0.8 - y + 0.15) / 0.05) ** 2)
    img += band[..., None] * SKY * 0.05
    return grain(img)


def contours():
    img = vgrad(BG, hexrgb('#142030'))
    f = fbm(W, H, 11, 4, 640)
    lines = np.abs(np.sin(f * np.pi * 26))
    edge = np.clip(1 - lines / 0.06, 0, 1)
    x, y = grid()
    light = np.exp(-(((x - 0.62) / 0.5) ** 2 + ((y - 0.45) / 0.45) ** 2))
    img += edge[..., None] * (SLATE * 0.55 + SKY * 0.25 * light[..., None])
    img = glow(img, 0.62, 0.45, 0.4, 0.35, BLUE, 0.07)
    return grain(img, 2.0)


def horizon():
    img = vgrad(hexrgb('#0a0e14'), hexrgb('#111a26'))
    x, y = grid()
    hy = 0.58
    img = glow(img, 0.5, hy, 0.9, 0.08, SKY, 0.35)
    img = glow(img, 0.5, hy, 0.35, 0.03, SKY, 0.35)
    floor = y > hy
    d = np.where(floor, (y - hy), 1e-3)
    zx = (x - 0.5) / d                       # perspective lines
    zz = 1.0 / d                             # depth rows
    gx = np.abs(((zx * 2.4) % 1.0) - 0.5)
    gz = np.abs(((zz * 0.35) % 1.0) - 0.5)
    lw = 0.012 / np.maximum(d, 0.02) * 0.02
    line = np.clip(1 - np.minimum(gx / (0.012 + 0.05 * (1 - d)), gz / (0.02 + lw)), 0, 1) ** 2
    fade = np.clip((y - hy) / 0.42, 0, 1) ** 0.6 * np.exp(-((x - 0.5) / 0.9) ** 2)
    img += (line * fade * floor)[..., None] * SLATE * 1.3
    # distant stars above
    sky = np.zeros((H, W), np.float32)
    n = 420
    xs, ys = rng.integers(0, W, n), rng.integers(0, int(H * hy * 0.95), n)
    sky[ys, xs] = rng.random(n) * 0.9 + 0.1
    sky = np.asarray(Image.fromarray((sky * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2)), np.float32) / 255 * 3
    img += sky[..., None] * hexrgb('#c7d5e0') * 0.6
    return grain(img, 2.0)


def facets():
    cols, rows = 22, 13
    pts = np.zeros((rows + 1, cols + 1, 2), np.float32)
    for j in range(rows + 1):
        for i in range(cols + 1):
            jx = 0 if i in (0, cols) else rng.uniform(-0.38, 0.38)
            jy = 0 if j in (0, rows) else rng.uniform(-0.38, 0.38)
            pts[j, i] = ((i + jx) * W / cols, (j + jy) * H / rows)
    img = Image.new('RGB', (W, H))
    from PIL import ImageDraw
    dr = ImageDraw.Draw(img)
    def shade(cx, cy):
        u, v = cx / W, cy / H
        base = BG * (1 - v) + BG2 * v
        l = np.exp(-(((u - 0.3) / 0.45) ** 2 + ((v - 0.2) / 0.5) ** 2))
        c = base + SLATE * 0.55 * l + rng.uniform(-6, 6)
        return tuple(int(min(255, max(0, k))) for k in c)
    for j in range(rows):
        for i in range(cols):
            a, b, c, d = pts[j, i], pts[j, i + 1], pts[j + 1, i], pts[j + 1, i + 1]
            for tri in ((a, b, d), (a, d, c)) if (i + j) % 2 else ((a, b, c), (b, d, c)):
                cx = sum(p[0] for p in tri) / 3; cy = sum(p[1] for p in tri) / 3
                dr.polygon([tuple(p) for p in tri], fill=shade(cx, cy))
    arr = np.asarray(img, np.float32)
    arr = glow(arr, 0.3, 0.2, 0.25, 0.25, SKY, 0.06)
    return grain(arr, 2.0)


def nebula():
    img = vgrad(hexrgb('#090c12'), hexrgb('#101826'))
    f = fbm(W, H, 33, 6, 900)
    g = fbm(W, H, 77, 5, 500)
    cloud = np.clip((f - 0.45) * 2.6, 0, 1) ** 1.6
    img += cloud[..., None] * (SLATE * 0.9 + BLUE * 0.25 * g[..., None])
    img += (np.clip((g - 0.6) * 3, 0, 1) ** 2 * cloud)[..., None] * SKY * 0.35
    stars = np.zeros((H, W), np.float32)
    n = 900
    xs, ys = rng.integers(0, W, n), rng.integers(0, H, n)
    stars[ys, xs] = rng.random(n) ** 3
    stars = np.asarray(Image.fromarray((np.clip(stars, 0, 1) * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.9)), np.float32) / 255 * 4
    img += stars[..., None] * hexrgb('#dce5ed')
    return grain(img, 2.0)


def ridge_profile(w, seed, base, amp, rough):
    r = np.random.default_rng(seed)
    x = np.linspace(0, 1, w)
    y = np.zeros(w)
    for k in range(1, 7):
        y += np.sin(x * np.pi * (k * 1.7 + r.random()) + r.random() * 6.28) * amp / (k ** rough)
    return base + y


def ridges_layers(w, h):
    """[(alpha mask HxW, colour)] back to front."""
    out = []
    specs = [(0.52, 0.06, 1.2, '#22374d'), (0.62, 0.07, 1.1, '#1a2a3c'), (0.72, 0.06, 1.0, '#13202e'), (0.84, 0.05, 0.9, '#0d1620')]
    _, yy = grid(w, h)
    for n, (base, amp, rough, col) in enumerate(specs):
        prof = ridge_profile(w, 500 + n, base, amp, rough)
        m = (yy > prof[None, :]).astype(np.float32)
        out.append((m, hexrgb(col)))
    return out


def ridges():
    img = vgrad(hexrgb('#0b1018'), hexrgb('#2a475e'), curve=0.8)
    img = glow(img, 0.7, 0.42, 0.4, 0.16, SKY, 0.25)
    _, y = grid()
    for n, (m, col) in enumerate(ridges_layers(W, H)):
        fog = np.exp(-((y - 0.55 - n * 0.08) / 0.12) ** 2)[..., None] * SKY * 0.06
        img = img * (1 - m[..., None]) + (col + fog) * m[..., None]
    return grain(img, 2.0)


# ---------------------------------------------------------------- the moving ones' layers
def blob(path, color, size=384):
    x, y = grid(size, size)
    d = ((x - 0.5) ** 2 + (y - 0.5) ** 2) / 0.25
    a = np.clip(np.exp(-d * 3.2) - 0.02, 0, 1)
    rgba = np.zeros((size, size, 4), np.uint8)
    rgba[..., :3] = color.astype(np.uint8)
    rgba[..., 3] = (a * 255).astype(np.uint8)
    Image.fromarray(rgba, 'RGBA').save(path, optimize=True)
    print(path)


def ridge_png(path, n, w=1920, h=540):
    r = np.random.default_rng(900 + n)
    x = np.linspace(0, 1, w)
    prof = 0.35 + n * 0.12
    for k in range(1, 7):
        prof = prof + np.sin(x * np.pi * 2 * k + r.random() * 6.28) * 0.12 / (k ** 1.1)   # whole periods: it tiles
    _, yy = grid(w, h)
    m = np.clip((yy - prof[None, :]) * h / 2.0, 0, 1)
    col = hexrgb(['#1f3348', '#152435', '#0d1620'][n])
    rgba = np.zeros((h, w, 4), np.uint8)
    rgba[..., :3] = (col + np.clip(1 - (yy - prof[None, :]) * 4, 0, 1)[..., None] * SKY * 0.05).clip(0, 255).astype(np.uint8)
    rgba[..., 3] = (m * 255).astype(np.uint8)
    Image.fromarray(rgba, 'RGBA').save(path, optimize=True)
    print(path)


def main():
    walls = os.path.join(OUT, 'wallpapers')
    stills = {'slate-dusk': dusk, 'slate-contours': contours, 'slate-horizon': horizon, 'slate-facets': facets,
              'slate-nebula': nebula, 'slate-ridges': ridges}
    for name, fn in stills.items():
        save(fn(), os.path.join(walls, name + '.jpg'))
    blob(os.path.join(OUT, 'blob-blue.png'), BLUE)
    blob(os.path.join(OUT, 'blob-sky.png'), SKY)
    blob(os.path.join(OUT, 'blob-slate.png'), hexrgb('#3d6a8c'))
    for n in range(3):
        ridge_png(os.path.join(OUT, 'ridge-%s.png' % 'abc'[n]), n)
    # Previews of the moving ones (Settings › Appearance tiles): a still frame of each, small.
    pw, ph = 640, 360
    t = vgrad(hexrgb('#0e131b'), BG2, pw, ph)
    for cx, cy, s, c in ((0.22, 0.62, 0.35, BLUE), (0.55, 0.32, 0.3, SKY), (0.75, 0.66, 0.32, hexrgb('#3d6a8c')), (0.82, 0.22, 0.22, BLUE)):
        t = glow(t, cx, cy, s * 0.6, s * 1.0, c, 0.35)
    save(t, os.path.join(OUT, 'wall-tide.jpg'), 85)
    save(np.asarray(Image.fromarray(np.clip(ridges(), 0, 255).astype(np.uint8)).resize((pw, ph), Image.LANCZOS), np.float32), os.path.join(OUT, 'wall-ridges.jpg'), 85)
    m = glow(vgrad(hexrgb('#0b0f15'), BG2, pw, ph), 0.5, 1.1, 0.7, 0.5, SLATE, 0.8)
    for _ in range(70):
        m = glow(m, rng.random(), rng.random(), 0.004, 0.007, SKY, rng.uniform(0.3, 1.0))
    save(m, os.path.join(OUT, 'wall-motes.jpg'), 85)


if __name__ == '__main__':
    main()
