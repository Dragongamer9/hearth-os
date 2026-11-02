"""Generates the tileable PBR texture sets used by the prototype.

Run:  python3 tools/gen_textures.py textures/
Every map tiles seamlessly (spectral noise and periodic Voronoi).

Packing:
  *_alb.png  RGB = albedo, A = height (terrain/concrete) or wear mask (metal), stored as 32..255
  *_nrm.png  R,G = normal X/Y (X along U, Y along V), B = roughness
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy.spatial import cKDTree

OUT = sys.argv[1] if len(sys.argv) > 1 else "textures"
os.makedirs(OUT, exist_ok=True)


# ------------------------------------------------------------------ noise

def spectral(n, beta=2.0, seed=0, kmin=1.0, kmax=None, aniso=(1.0, 1.0)):
    """Periodic fractal noise via FFT filtering. Zero mean, unit std."""
    r = np.random.default_rng(seed)
    w = r.standard_normal((n, n))
    f = np.fft.fft2(w)
    k1 = np.fft.fftfreq(n) * n
    kx, ky = np.meshgrid(k1, k1)
    k = np.sqrt((kx * aniso[0]) ** 2 + (ky * aniso[1]) ** 2)
    with np.errstate(divide="ignore"):
        amp = np.where(k >= kmin, k ** (-beta / 2.0), 0.0)
    if kmax:
        amp *= np.exp(-((k / kmax) ** 2))
    out = np.real(np.fft.ifft2(f * amp))
    return (out - out.mean()) / (out.std() + 1e-9)


def n01(a):
    return (a - a.min()) / (a.max() - a.min() + 1e-9)


def smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def voronoi(n, count, seed, stretch=1.0):
    """Periodic Voronoi. stretch < 1 elongates cells vertically."""
    r = np.random.default_rng(seed)
    pts = r.random((count, 2)) * np.array([1.0, stretch])
    tree = cKDTree(pts, boxsize=[1.0, stretch])
    ys, xs = np.mgrid[0:n, 0:n] / n
    q = np.stack([xs.ravel(), (ys * stretch).ravel()], 1)
    d, i = tree.query(q, k=2)
    return d[:, 0].reshape(n, n), d[:, 1].reshape(n, n), i[:, 0].reshape(n, n)


def warp(a, wx, wy, amount):
    """Periodic domain warp by integer pixel offsets."""
    n = a.shape[0]
    ys, xs = np.mgrid[0:n, 0:n]
    x2 = (xs + wx * amount).astype(int) % n
    y2 = (ys + wy * amount).astype(int) % n
    return a[y2, x2]


def normals(h, strength):
    gx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * 0.5 * strength
    gy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * 0.5 * strength
    nz = np.ones_like(h)
    l = np.sqrt(gx * gx + gy * gy + nz)
    return -gx / l, -gy / l


def save_set(name, albedo, height, height_strength, rough, alpha=None):
    nx, ny = normals(height, height_strength)
    a = n01(height) if alpha is None else np.clip(alpha, 0, 1)
    alb = np.dstack([np.clip(albedo, 0, 1), 32.0 / 255.0 + a * (223.0 / 255.0)])
    Image.fromarray((alb * 255).round().astype(np.uint8), "RGBA").save(os.path.join(OUT, name + "_alb.png"))
    nrm = np.dstack([nx * 0.5 + 0.5, ny * 0.5 + 0.5, np.clip(rough, 0, 1)])
    Image.fromarray((nrm * 255).round().astype(np.uint8), "RGB").save(os.path.join(OUT, name + "_nrm.png"))
    print("wrote", name)


def mix(a, b, t):
    t = t[..., None]
    return a * (1 - t) + b * t


def col(c, n):
    return np.broadcast_to(np.array(c, dtype=float), (n, n, 3))


# ------------------------------------------------------------------ ground: dry meadow

def grass_ground(n=1024):
    big = spectral(n, 2.4, 1, kmin=1)
    mid = spectral(n, 2.0, 2, kmin=6)
    fine = spectral(n, 1.2, 3, kmin=60, kmax=400)
    f1, f2, ids = voronoi(n, 2600, 4)
    tuft = 1.0 - smooth(0.0, 0.012, f1)  # little clumps
    rnd = np.random.default_rng(5).random(2600)[ids]
    blades = spectral(n, 0.8, 6, kmin=90, kmax=500, aniso=(1.0, 0.3))
    dry = smooth(-0.3, 0.9, big * 0.7 + mid * 0.4)
    green = col([0.23, 0.27, 0.11], n)
    olive = col([0.38, 0.37, 0.18], n)
    straw = col([0.58, 0.5, 0.3], n)
    dead = col([0.2, 0.16, 0.1], n)
    c = mix(green, olive, n01(mid))
    c = mix(c, straw, dry * 0.45)
    c = mix(c, straw, smooth(0.6, 1.4, spectral(n, 1.4, 7, kmin=25)) * 0.35)
    c = mix(c, dead, smooth(0.55, 1.2, -fine) * 0.6)  # soil showing through
    c *= (0.82 + 0.3 * n01(blades) + 0.12 * rnd * tuft)[..., None]
    h = n01(mid) * 0.3 + n01(blades) * 0.4 + tuft * 0.3
    rough = 0.86 + 0.08 * n01(fine)
    save_set("grass", c, h, n * 0.9, rough)


# ------------------------------------------------------------------ ground: soil and pebbles

def dirt_ground(n=1024):
    big = spectral(n, 2.2, 11, kmin=1)
    mid = spectral(n, 1.8, 12, kmin=8)
    fine = spectral(n, 1.0, 13, kmin=80, kmax=450)
    r = np.random.default_rng(14)
    count = 1300
    f1, f2, ids = voronoi(n, count, 15)
    radius = r.uniform(0.004, 0.014, count)[ids]
    keep = (r.random(count) < 0.3)[ids]
    tone = r.uniform(0.75, 1.25, count)[ids]
    peb = np.where(keep, np.clip(1.0 - (f1 / radius) ** 2, 0, 1), 0.0)
    pebble_mask = smooth(0.0, 0.15, peb)
    # dry mud cracks
    c1, c2, _ = voronoi(n, 70, 16)
    crack = 1.0 - smooth(0.0, 0.004, c2 - c1)
    soil = mix(col([0.36, 0.27, 0.19], n), col([0.23, 0.17, 0.12], n), smooth(-1, 1, mid))
    soil *= (0.85 + 0.25 * n01(fine))[..., None]
    soil = mix(soil, col([0.5, 0.42, 0.32], n), smooth(0.3, 1.5, big) * 0.5)
    stone = col([0.37, 0.34, 0.3], n) * tone[..., None] * (0.85 + 0.3 * n01(fine))[..., None]
    c = mix(soil, stone, pebble_mask)
    c = mix(c, col([0.12, 0.09, 0.07], n), crack * 0.6 * (1 - pebble_mask))
    h = n01(mid) * 0.25 + n01(fine) * 0.15 + np.sqrt(peb) * 0.6 - crack * 0.2
    rough = 0.93 - pebble_mask * 0.22
    save_set("dirt", c, h, n * 1.1, rough)


# ------------------------------------------------------------------ rock / cliff

def rock(n=1024):
    big = spectral(n, 2.3, 21, kmin=1)
    mid = spectral(n, 2.0, 22, kmin=5)
    fine = spectral(n, 1.3, 23, kmin=50, kmax=500)
    wx = spectral(n, 2.5, 24, kmin=2)
    wy = spectral(n, 2.5, 25, kmin=2)
    c1, c2, ids = voronoi(n, 45, 26)
    c1 = warp(c1, wx, wy, 6)
    c2 = warp(c2, wx, wy, 6)
    crack = 1.0 - smooth(0.0, 0.0045, c2 - c1)
    d1, d2, _ = voronoi(n, 260, 27)
    small = 1.0 - smooth(0.0, 0.0025, d2 - d1)
    face = np.random.default_rng(28).uniform(-1, 1, 45)[ids]
    ys = np.mgrid[0:n, 0:n][0] / n
    strata = np.sin(ys * 2 * np.pi * 9 + wx * 1.4) * 0.5 + 0.5
    light = col([0.47, 0.44, 0.4], n)
    dark = col([0.24, 0.22, 0.2], n)
    c = mix(dark, light, smooth(-1.2, 1.2, mid * 0.6 + big * 0.4 + face * 0.4))
    c *= (0.8 + 0.25 * strata + 0.2 * n01(fine))[..., None]
    lichen = smooth(0.9, 1.4, spectral(n, 1.6, 29, kmin=20))
    lichen2 = smooth(1.1, 1.6, spectral(n, 1.6, 30, kmin=30))
    c = mix(c, col([0.56, 0.53, 0.33], n), lichen * 0.8)
    c = mix(c, col([0.14, 0.15, 0.11], n), lichen2 * 0.7)
    c = mix(c, col([0.06, 0.055, 0.05], n), np.maximum(crack, small * 0.6))
    h = n01(mid) * 0.45 + n01(fine) * 0.2 + face * 0.1 + strata * 0.08 - crack * 0.45 - small * 0.15
    rough = 0.78 + 0.12 * lichen - 0.08 * n01(fine)
    save_set("rock", c, h, n * 1.6, rough)


# ------------------------------------------------------------------ concrete

def concrete(n=1024):
    big = spectral(n, 2.4, 41, kmin=1)
    stain = spectral(n, 1.8, 42, kmin=3)
    streak = spectral(n, 1.6, 43, kmin=4, aniso=(1.0, 0.08))
    speck = spectral(n, 0.3, 44, kmin=150)
    r = np.random.default_rng(45)
    pores = (r.random((n, n)) < 0.004).astype(float)
    pores = np.maximum(pores, np.roll(pores, 1, 0) * 0.6)
    c1, c2, _ = voronoi(n, 18, 46)
    crack = 1.0 - smooth(0.0, 0.0022, c2 - c1)
    crack *= smooth(0.2, 0.9, spectral(n, 2.2, 47, kmin=2))  # cracks only in places
    g = 0.5 + 0.06 * big + 0.05 * speck
    g -= smooth(0.4, 1.6, stain) * 0.1
    g -= smooth(0.5, 1.8, streak) * 0.1
    g -= pores * 0.25 + crack * 0.3
    c = np.dstack([g * 1.0, g * 0.98, g * 0.95])
    h = 0.5 + 0.05 * speck - pores * 0.4 - crack * 0.5
    rough = 0.9 + 0.05 * n01(speck) - smooth(0.6, 1.8, streak) * 0.1
    save_set("concrete", c, h, n * 0.7, rough)


# ------------------------------------------------------------------ painted metal (tinted in shader)

def painted_metal(n=1024):
    mott = spectral(n, 2.0, 51, kmin=2)
    grime = spectral(n, 1.9, 52, kmin=3)
    peel = spectral(n, 1.2, 53, kmin=40, kmax=600)  # orange peel
    chips_n = spectral(n, 1.5, 54, kmin=12)
    chips = smooth(1.75, 2.0, chips_n + 0.3 * spectral(n, 1.0, 55, kmin=60))
    # scratches: random thin line segments
    img = Image.new("L", (n * 2, n * 2), 0)
    d = ImageDraw.Draw(img)
    r = np.random.default_rng(56)
    for _ in range(900):
        x, y = r.random(2) * n * 2
        ang = r.normal(0.3, 0.6)
        ln = r.uniform(20, 220)
        w = 1 if r.random() < 0.8 else 2
        v = int(r.uniform(90, 255))
        for ox in (-n * 2, 0, n * 2):
            for oy in (-n * 2, 0, n * 2):
                d.line([(x + ox, y + oy), (x + ox + np.cos(ang) * ln, y + oy + np.sin(ang) * ln)], fill=v, width=w)
    scratch = np.asarray(img.resize((n, n), Image.LANCZOS), dtype=float) / 255.0
    dents = spectral(n, 2.6, 57, kmin=3)
    g = 0.5 + 0.035 * mott - smooth(0.5, 2.0, grime) * 0.12
    c = np.dstack([g, g, g])
    wear = np.clip(np.maximum(chips, scratch * 0.9), 0, 1)
    h = 0.5 + 0.012 * peel + 0.05 * dents - chips * 0.08 - scratch * 0.05
    rough = 0.55 + 0.08 * n01(grime) - wear * 0.25
    save_set("metal", c, h, n * 1.4, rough, alpha=wear)


# ------------------------------------------------------------------ bark

def bark(n=512):
    f1, f2, ids = voronoi(n, 160, 61, stretch=0.18)
    wx = spectral(n, 2.4, 62, kmin=2)
    ridge = warp(f2 - f1, wx, wx * 0.2, 4)
    fiss = 1.0 - smooth(0.0, 0.03, ridge)
    fine = spectral(n, 1.2, 63, kmin=40, kmax=250, aniso=(1.0, 0.25))
    c = mix(col([0.42, 0.33, 0.25], n), col([0.24, 0.18, 0.13], n), smooth(-1, 1, fine))
    c = mix(c, col([0.06, 0.04, 0.03], n), fiss * 0.8)
    c = mix(c, col([0.45, 0.42, 0.36], n), smooth(1.2, 2.2, spectral(n, 1.5, 64, kmin=8)) * 0.6)
    h = smooth(0.0, 0.08, ridge) * 0.7 + n01(fine) * 0.3
    save_set("bark", c, h, n * 2.0, 0.9 + 0.05 * n01(fine))


# ------------------------------------------------------------------ alpha cards

def grass_card(n=512):
    ss = 2
    img = Image.new("RGBA", (n * ss, n * ss), (60, 58, 30, 0))
    d = ImageDraw.Draw(img)
    r = np.random.default_rng(71)
    W = n * ss
    for _ in range(170):
        x0 = r.uniform(0.04, 0.96) * W
        h = r.uniform(0.35, 0.98) * W
        lean = r.normal(0, 0.12) * h
        w0 = r.uniform(0.006, 0.014) * W
        dry = r.random()
        base = np.array([0.2, 0.22, 0.09]) * (1 - dry) + np.array([0.32, 0.28, 0.14]) * dry
        tip = np.array([0.42, 0.44, 0.2]) * (1 - dry) + np.array([0.66, 0.58, 0.34]) * dry
        segs = 10
        pts_l, pts_r = [], []
        for s in range(segs + 1):
            t = s / segs
            x = x0 + lean * t * t
            y = W - h * t
            w = w0 * (1 - t) ** 0.8
            pts_l.append((x - w, y))
            pts_r.append((x + w, y))
        poly = pts_l + pts_r[::-1]
        # draw segment-wise for a gradient
        for s in range(segs):
            t = (s + 0.5) / segs
            cc = base * (1 - t) + tip * t
            q = [pts_l[s], pts_l[s + 1], pts_r[s + 1], pts_r[s]]
            d.polygon(q, fill=tuple(int(v * 255) for v in cc) + (255,))
    img = img.resize((n, n), Image.LANCZOS)
    img.save(os.path.join(OUT, "grass_card.png"))
    print("wrote grass_card")


def needle_card(n=512):
    """A drooping pine branch frond seen from above: stem along +X."""
    ss = 2
    W = n * ss
    img = Image.new("RGBA", (W, W), (30, 40, 22, 0))
    d = ImageDraw.Draw(img)
    r = np.random.default_rng(81)

    def twig(x0, y0, ang, length, width, depth):
        steps = 14
        x, y = x0, y0
        for s in range(steps):
            t = s / steps
            nx = x + np.cos(ang) * length / steps
            ny = y + np.sin(ang) * length / steps
            d.line([(x, y), (nx, ny)], fill=(58, 44, 30, 255), width=max(1, int(width * (1 - t))))
            # needles on both sides
            for side in (-1, 1):
                for k in range(7):
                    na = ang + side * r.uniform(0.5, 1.25)
                    nl = r.uniform(0.04, 0.075) * W * (1 - 0.45 * t)
                    g = r.uniform(0.7, 1.15)
                    cc = (int(34 * g), int(52 * g), int(28 * g), 255)
                    px = x + (nx - x) * k / 7
                    py = y + (ny - y) * k / 7
                    d.line([(px, py), (px + np.cos(na) * nl, py + np.sin(na) * nl)], fill=cc, width=ss + 1)
            if depth > 0 and s % 2 == 1 and s < steps - 2:
                for side in (-1, 1):
                    twig(nx, ny, ang + side * r.uniform(0.45, 0.8), length * r.uniform(0.32, 0.5) * (1 - 0.4 * t), width * 0.5, depth - 1)
            x, y = nx, ny
            ang += r.normal(0, 0.04)

    twig(0.03 * W, 0.5 * W, 0.0, 0.92 * W, 6 * ss, 2)
    img = img.resize((n, n), Image.LANCZOS)
    img.save(os.path.join(OUT, "needles_card.png"))
    print("wrote needles_card")


def clouds(n=1024):
    base = n01(spectral(n, 2.3, 91, kmin=2))
    f1, _, _ = voronoi(n, 220, 92)
    billow = 1.0 - n01(f1)
    detail = n01(spectral(n, 1.6, 93, kmin=12))
    c = base * 0.6 + billow * 0.25 + detail * 0.15
    g = (n01(c) * 255).astype(np.uint8)
    Image.fromarray(np.dstack([g, g, g]), "RGB").save(os.path.join(OUT, "clouds.png"))
    print("wrote clouds")


if __name__ == "__main__":
    grass_ground()
    dirt_ground()
    rock()
    concrete()
    painted_metal()
    bark()
    grass_card()
    needle_card()
    clouds()
