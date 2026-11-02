"""Library art for Quad Mech (Hearth's "Show demo"): a golden-hour scene with the four-legged mech in
silhouette, in the game's own colours. Writes art/cover.png (600x900), art/wide.png (920x430) and
art/hero.png (1920x620). CI puts them next to the game in the release zips (.github/workflows/quadmech.yml).

Run:  python3 tools/make_art.py art/
Needs Pillow; uses DejaVu Sans Bold for the title when it's installed (falls back to Pillow's font).
"""
import math
import os
import random
import sys

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

OUT = sys.argv[1] if len(sys.argv) > 1 else "art"
FONTS = ["/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
         "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf"]


def font(size):
    for f in FONTS:
        if os.path.exists(f):
            return ImageFont.truetype(f, size)
    return ImageFont.load_default()


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def sky(w, h, horizon):
    img = Image.new("RGB", (w, h))
    px = img.load()
    top, mid, low = (34, 44, 72), (196, 112, 70), (255, 196, 120)
    for y in range(h):
        t = min(1.0, y / max(1, horizon))
        c = lerp(top, mid, t / 0.7) if t < 0.7 else lerp(mid, low, (t - 0.7) / 0.3)
        for x in range(w):
            px[x, y] = c
    return img


def hills(d, w, base, amp, color, seed, step=6):
    r = random.Random(seed)
    phase = [r.uniform(0, 6.28) for _ in range(3)]
    pts = [(0, d.im.size[1])]
    for x in range(0, w + step, step):
        y = base - amp * (0.55 * math.sin(x / w * 5.0 + phase[0]) + 0.3 * math.sin(x / w * 11.0 + phase[1])
                          + 0.15 * math.sin(x / w * 23.0 + phase[2]))
        pts.append((x, y))
    pts.append((w, d.im.size[1]))
    d.polygon(pts, fill=color)


def pines(d, w, base, height, color, seed, count, keep_clear=None):
    r = random.Random(seed)
    for _ in range(count):
        x = r.uniform(-20, w + 20)
        if keep_clear and keep_clear[0] < x < keep_clear[1]:
            continue
        hh = height * r.uniform(0.6, 1.2)
        y = base + r.uniform(-height * 0.1, height * 0.15)
        for k in range(4):
            ww = hh * (0.32 - k * 0.06)
            top = y - hh + k * hh * 0.18
            d.polygon([(x, top), (x - ww, top + hh * 0.42), (x + ww, top + hh * 0.42)], fill=color)
        d.rectangle([x - hh * 0.03, y - hh * 0.2, x + hh * 0.03, y + 2], fill=color)


def mech(d, cx, ground, s, color, glow):
    """The quad mech side-on: hull, four bent legs with three-toed feet, turret, twin cannons, headlights."""
    hull_w, hull_h = 2.2 * s, 0.62 * s
    hy = ground - 1.55 * s
    # legs: hip on the hull, knee up and out, foot on the ground
    for hip_dx, foot_dx, knee_up, back in ((-0.95, -1.55, 0.45, True), (-0.55, -0.85, 0.5, False),
                                           (0.55, 0.95, 0.5, False), (0.95, 1.6, 0.45, True)):
        hip = (cx + hip_dx * s, hy + hull_h * 0.6)
        foot = (cx + foot_dx * s, ground)
        knee = (hip[0] + (foot[0] - hip[0]) * 0.45, hy - knee_up * s)
        wdt = int(0.16 * s) if not back else int(0.13 * s)
        d.line([hip, knee], fill=color, width=wdt)
        d.line([knee, foot], fill=color, width=int(wdt * 0.85))
        d.ellipse([knee[0] - wdt * 0.7, knee[1] - wdt * 0.7, knee[0] + wdt * 0.7, knee[1] + wdt * 0.7], fill=color)
        for toe in (-0.22, 0, 0.22):
            d.polygon([(foot[0] - 0.08 * s, ground - 0.1 * s), (foot[0] + 0.08 * s, ground - 0.1 * s),
                       (foot[0] + toe * s + 0.06 * s, ground + 0.02 * s), (foot[0] + toe * s - 0.06 * s, ground + 0.02 * s)], fill=color)
    # hull (a low wedge) and the turret on top
    d.polygon([(cx - hull_w / 2, hy + hull_h * 0.25), (cx - hull_w / 2 + 0.25 * s, hy - hull_h * 0.15),
               (cx + hull_w / 2 - 0.2 * s, hy - hull_h * 0.15), (cx + hull_w / 2, hy + hull_h * 0.2),
               (cx + hull_w / 2 - 0.15 * s, hy + hull_h), (cx - hull_w / 2 + 0.15 * s, hy + hull_h)], fill=color)
    ty = hy - hull_h * 0.15
    d.polygon([(cx - 0.55 * s, ty), (cx - 0.42 * s, ty - 0.45 * s), (cx + 0.5 * s, ty - 0.45 * s), (cx + 0.7 * s, ty)], fill=color)
    for dy in (-0.32, -0.18):
        d.rectangle([cx + 0.5 * s, ty + dy * s - 0.035 * s, cx + 1.75 * s, ty + dy * s + 0.035 * s], fill=color)
    # exhaust stacks
    for dx in (-0.85, -0.7):
        d.rectangle([cx + dx * s, hy - hull_h * 0.6, cx + dx * s + 0.08 * s, hy], fill=color)
    # headlights and a glowing vent
    for dx, dy in ((0.98, 0.25), (0.98, 0.45)):
        x, y = cx + dx * s, hy + dy * hull_h
        d.ellipse([x - 0.05 * s, y - 0.05 * s, x + 0.05 * s, y + 0.05 * s], fill=glow)
    d.rectangle([cx - 0.3 * s, ty - 0.2 * s, cx + 0.1 * s, ty - 0.16 * s], fill=(255, 140, 60))


def scene(w, h, title_size, mech_scale, mech_x, title_pos, subtitle=True):
    horizon = int(h * 0.62)
    img = sky(w, h, horizon)
    glow = Image.new("RGB", (w, h), (0, 0, 0))
    gd = ImageDraw.Draw(glow)
    sx, sy, sr = int(w * 0.72), int(horizon - h * 0.08), int(min(w, h) * 0.09)
    for k in range(10, 0, -1):
        rr = sr * (1 + k * 0.5)
        c = int(18 * (11 - k))
        gd.ellipse([sx - rr, sy - rr, sx + rr, sy + rr], fill=(c, int(c * 0.7), int(c * 0.35)))
    img = ImageChops.add(img, glow.filter(ImageFilter.GaussianBlur(sr * 0.6)))   # the sun's haze, added on top
    d = ImageDraw.Draw(img)
    d.ellipse([sx - sr, sy - sr, sx + sr, sy + sr], fill=(255, 232, 180))
    hills(d, w, horizon - h * 0.02, h * 0.06, (122, 84, 86), 3)
    hills(d, w, horizon + h * 0.03, h * 0.05, (84, 62, 72), 7)
    # lake with the sun's glint
    d.rectangle([0, horizon + h * 0.08, w, horizon + h * 0.14], fill=(214, 150, 110))
    for i in range(14):
        y = horizon + h * 0.085 + i * h * 0.004
        d.line([(sx - sr * (1.6 - i * 0.08), y), (sx + sr * (1.6 - i * 0.08), y)], fill=(255, 220, 160), width=1)
    pines(d, w, horizon + h * 0.08, h * 0.16, (52, 40, 50), 11, max(8, w // 40), (mech_x - mech_scale * 1.2, mech_x + mech_scale * 1.4))
    hills(d, w, horizon + h * 0.2, h * 0.04, (38, 30, 38), 13)
    clear = (mech_x - mech_scale * 2.2, mech_x + mech_scale * 2.4)
    pines(d, w, horizon + h * 0.22, h * 0.24, (30, 24, 32), 17, max(6, w // 70), clear)
    ground = horizon + h * 0.28
    hills(d, w, ground, h * 0.015, (22, 18, 26), 19, step=10)
    mech(d, mech_x, ground + h * 0.01, mech_scale, (16, 14, 20), (255, 240, 200))
    # low mist over the ground, then a soft vignette
    mist = Image.new("L", (w, h), 0)
    md = ImageDraw.Draw(mist)
    for i in range(6):
        y = ground - h * 0.02 + i * h * 0.015
        md.rectangle([0, y, w, y + h * 0.02], fill=26)
    mist = mist.filter(ImageFilter.GaussianBlur(h * 0.02))
    img = Image.composite(Image.new("RGB", (w, h), (230, 180, 150)), img, mist)
    vig = Image.new("L", (w, h), 0)
    vd = ImageDraw.Draw(vig)
    vd.ellipse([-w * 0.25, -h * 0.25, w * 1.25, h * 1.25], fill=255)
    vig = vig.filter(ImageFilter.GaussianBlur(min(w, h) * 0.12))
    img = Image.composite(img, Image.new("RGB", (w, h), (10, 8, 14)), vig)
    if title_size:
        d = ImageDraw.Draw(img)
        size = title_size
        while size > 16 and ImageDraw.Draw(img).textlength("QUAD MECH", font=font(size)) > w - 2 * title_pos[0]:
            size -= 2
        f = font(size)
        title_size = size
        tx, ty = title_pos
        for dx, dy in ((3, 3), (2, 2)):
            d.text((tx + dx, ty + dy), "QUAD MECH", font=f, fill=(20, 12, 16))
        d.text((tx, ty), "QUAD MECH", font=f, fill=(255, 238, 214))
        if subtitle:
            fs = font(max(12, title_size // 4))
            d.text((tx + 4, ty + int(title_size * 1.12)), "TWO SEATS  ·  ONE SCREEN", font=fs, fill=(255, 196, 140))
    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    scene(600, 900, 92, 82, 360, (36, 70)).save(os.path.join(OUT, "cover.png"), optimize=True)
    scene(920, 430, 64, 50, 650, (40, 40)).save(os.path.join(OUT, "wide.png"), optimize=True)
    scene(1920, 620, 0, 70, 1330, (0, 0)).save(os.path.join(OUT, "hero.png"), optimize=True)
    print("wrote cover.png, wide.png, hero.png")


if __name__ == "__main__":
    main()
