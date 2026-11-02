#!/usr/bin/env python3
"""Hearth OS's startup animations for Steam Game Mode (1.1): four original ~5 second animations, made here from the
Hearth mark with code (no Valve/Steam material), each for 16:9 (1920x1080) and 16:10 (1920x1200) screens, as VP9
WebM, into os/system_files/usr/share/hearth/startup-movies/hearth-<id>-<16x9|16x10>.webm.

  flame   Flame ignition: a spark rises and lights the Hearth flame, then the arch and the name appear
  sweep   Slate light sweep: a blue light sweeps across the slate and leaves the mark and the name behind it
  reveal  Logo reveal: the arch draws itself, the flame grows inside it, the name fades in letter by letter
  embers  Rising embers: embers drift up, gather into the flame, the mark settles

hearth-boot-look puts the chosen one in Steam's override folder (Settings › Boot & startup).

  python3 tools/make-startup-movies.py [OUT_DIR] [--only ID] [--frames DIR]   (Pillow, numpy and ffmpeg with libvpx-vp9)
"""
import math
import os
import subprocess
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
LOGO = os.path.join(ROOT, 'os/system_files/usr/share/plymouth/themes/hearth/logo.png')
OUT = os.path.join(ROOT, 'os/system_files/usr/share/hearth/startup-movies')
FPS = 30
SECS = 5.0
SIZES = {'16x9': (1920, 1080), '16x10': (1920, 1200)}
SLATE_TOP = np.array([0x1b, 0x28, 0x38], float)
SLATE_BOTTOM = np.array([0x0b, 0x10, 0x16], float)
BLUE = np.array([0x1a, 0x9f, 0xff], float)
SKY = np.array([0x66, 0xc0, 0xf4], float)
WARM = np.array([0xff, 0x9a, 0x3d], float)
TEXT = (0xdc, 0xe5, 0xed)


def ease(t):
    t = min(1.0, max(0.0, t))
    return t * t * (3 - 2 * t)


def ramp(t, a, b):
    return ease((t - a) / (b - a)) if b > a else float(t >= a)


def font(size, weight='Regular'):
    for p in (f'/usr/share/fonts/opentype/inter/Inter-{weight}.otf', f'/usr/share/fonts/truetype/inter/Inter-{weight}.ttf',
              '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'):
        if os.path.isfile(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def wordmark(height):
    """'HEARTH OS' in Inter with wide letter spacing, as an RGBA image (and each letter's box, for letter fades)."""
    f = font(height, 'Medium')
    letters, x, boxes = 'HEARTH OS', 0, []
    track = int(height * 0.42)
    for ch in letters:
        w = f.getbbox(ch)[2] if ch != ' ' else int(height * 0.35)
        boxes.append((ch, x, w))
        x += w + track
    img = Image.new('RGBA', (x - track + 4, int(height * 1.3)), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for ch, bx, _ in boxes:
        if ch != ' ':
            d.text((bx, 0), ch, font=f, fill=TEXT + (255,))
    return img, boxes


def split_logo():
    """The Hearth mark split into the arch (with the hearth stone) and the flame inside it (the flame sits inside the
    arch's opening: 33-67% across, 32-83% down the 512 px mark, well clear of the arch's stroke)."""
    im = Image.open(LOGO).convert('RGBA')
    a = np.asarray(im).astype(float)
    n = a.shape[0]
    yy, xx = np.mgrid[0:n, 0:n] / n
    rgb = a[..., :3]
    sat = (rgb.max(-1) - rgb.min(-1)) / np.maximum(1, rgb.max(-1))
    # The arch's curve dips into that box near its corners: there only the flame's own (saturated) colours count.
    flame_mask = ((xx > 0.30) & (xx < 0.70) & (yy > 0.30) & (yy < 0.85) & ((sat > 0.25) | (yy > 0.45))).astype(float)
    arch = a.copy(); arch[..., 3] *= 1 - flame_mask
    flame = a.copy(); flame[..., 3] *= flame_mask
    return im, Image.fromarray(arch.astype(np.uint8), 'RGBA'), Image.fromarray(flame.astype(np.uint8), 'RGBA')


def background(w, h):
    y, x = np.mgrid[0:h, 0:w].astype(float)
    t = (y / (h - 1))[..., None]
    img = SLATE_TOP * (1 - t) + SLATE_BOTTOM * t
    d = np.sqrt(((x - w / 2) / (w * 0.6)) ** 2 + ((y - h * 0.45) / (h * 0.6)) ** 2)
    v = np.clip(1 - d, 0, 1) ** 2 * 0.10
    return img * (1 - v[..., None]) + BLUE * v[..., None]


def radial(w, h, cx, cy, r, color, strength):
    """An additive soft glow (float array) centred on cx, cy."""
    y, x = np.mgrid[0:h, 0:w].astype(float)
    d2 = ((x - cx) ** 2 + (y - cy) ** 2) / max(1.0, r * r)
    return np.exp(-d2 * 2.2)[..., None] * color * strength


class Scene:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.bg = background(w, h)
        self.logo, self.arch, self.flame = split_logo()
        self.mark = int(min(w, h) * 0.30)
        self.cx, self.cy = w // 2, int(h * 0.44)
        self.word, self.boxes = wordmark(int(min(w, h) * 0.05))
        self.wy = self.cy + self.mark // 2 + int(min(w, h) * 0.06)

    def frame(self, base):
        return Image.fromarray(np.clip(base, 0, 255).astype(np.uint8), 'RGB').convert('RGBA')

    def paste(self, canvas, sprite, cx, cy, size, alpha):
        if alpha <= 0.003 or size < 2:
            return
        s = sprite.resize((size, size), Image.LANCZOS) if sprite.size != (size, size) else sprite
        if alpha < 1:
            s = s.copy(); s.putalpha(s.split()[3].point(lambda v: int(v * alpha)))
        canvas.alpha_composite(s, (int(cx - size / 2), int(cy - size / 2)))

    def put_word(self, canvas, alpha, per_letter=None):
        img = self.word
        if per_letter is not None:
            a = np.asarray(img).astype(float)
            for i, (ch, bx, bw) in enumerate(self.boxes):
                a[:, bx:bx + bw + 2, 3] *= per_letter(i)
            img = Image.fromarray(a.astype(np.uint8), 'RGBA')
        if alpha < 1:
            img = img.copy(); img.putalpha(img.split()[3].point(lambda v: int(v * alpha)))
        canvas.alpha_composite(img, (self.cx - img.width // 2, self.wy))

    def fade_out(self, img, t):
        k = 1 - ramp(t, SECS - 0.45, SECS)
        if k >= 1:
            return img
        return Image.fromarray((np.asarray(img.convert('RGB')).astype(float) * k).astype(np.uint8), 'RGB')


def flame_ignition(sc, t):
    base = sc.bg.copy()
    spark_t = ramp(t, 0.2, 1.3)
    ign = ramp(t, 1.2, 2.0)
    sx, sy = sc.cx, sc.h * 0.95 - (sc.h * 0.95 - (sc.cy + sc.mark * 0.12)) * spark_t
    if t < 1.5:
        base += radial(sc.w, sc.h, sx, sy, sc.mark * 0.05 * (1 + 0.3 * math.sin(t * 40)), WARM, 1.2 * (1 - ign))
    glow = ign * (0.75 + 0.12 * math.sin(t * 7) + 0.05 * math.sin(t * 23))
    base += radial(sc.w, sc.h, sc.cx, sc.cy + sc.mark * 0.1, sc.mark * (0.45 + 0.35 * ign), WARM, 0.55 * glow)
    img = sc.frame(base)
    fs = int(sc.mark * (0.15 + 0.85 * ease(ign)))
    sc.paste(img, sc.flame, sc.cx, sc.cy + (sc.mark - fs) * 0.1, fs, ign)
    sc.paste(img, sc.arch, sc.cx, sc.cy, sc.mark, ramp(t, 2.0, 2.8))
    sc.put_word(img, ramp(t, 2.6, 3.4))
    return sc.fade_out(img, t)


def light_sweep(sc, t):
    base = sc.bg.copy()
    p = ramp(t, 0.3, 2.6)
    y, x = np.mgrid[0:sc.h, 0:sc.w].astype(float)
    pos = -0.25 + 1.5 * p                       # the band's place across the screen (diagonal)
    s = x / sc.w + (y / sc.h - 0.5) * 0.35
    band = np.exp(-((s - pos) / 0.07) ** 2)
    base += band[..., None] * SKY * 0.35 * (1 - ramp(t, 2.6, 3.2))
    base += radial(sc.w, sc.h, sc.cx, sc.cy, sc.mark * 0.9, BLUE, 0.22 * ramp(t, 1.4, 3.0))
    img = sc.frame(base)
    # Revealed where the band has passed: the mark's left-to-right wipe follows the band.
    reveal_x = (pos - (sc.cy / sc.h - 0.5) * 0.35) * sc.w
    m = sc.logo.resize((sc.mark, sc.mark), Image.LANCZOS)
    a = np.asarray(m).astype(float)
    xs = np.arange(sc.mark)[None, :] + sc.cx - sc.mark / 2
    a[..., 3] *= np.clip((reveal_x - xs) / (sc.w * 0.05), 0, 1)
    img.alpha_composite(Image.fromarray(a.astype(np.uint8), 'RGBA'), (sc.cx - sc.mark // 2, sc.cy - sc.mark // 2))
    wx0 = sc.cx - sc.word.width / 2
    sc.put_word(img, ramp(t, 1.6, 2.8), per_letter=lambda i: float(np.clip((reveal_x - (wx0 + sc.boxes[i][1])) / (sc.w * 0.05), 0, 1)))
    return sc.fade_out(img, t)


def logo_reveal(sc, t):
    base = sc.bg.copy()
    draw = ramp(t, 0.2, 2.0)
    base += radial(sc.w, sc.h, sc.cx, sc.cy + sc.mark * 0.1, sc.mark * 0.7, WARM, 0.35 * ramp(t, 1.8, 2.8))
    img = sc.frame(base)
    # The arch drawn along its own path: an angle sweep around the arch's centre, starting at the bottom left.
    m = sc.arch.resize((sc.mark, sc.mark), Image.LANCZOS)
    a = np.asarray(m).astype(float)
    yy, xx = np.mgrid[0:sc.mark, 0:sc.mark].astype(float)
    m_, yc, yb = sc.mark, sc.mark * 0.49, sc.mark * 0.86
    theta = np.arctan2(np.maximum(0, yc - yy), xx - m_ / 2)              # 0 (right) .. pi (left), over the top
    prog = np.where(yy < yc, 0.3 + 0.4 * (np.pi - theta) / np.pi,
                    np.where(xx < m_ / 2, 0.3 * (1 - (yy - yc) / (yb - yc)), 0.7 + 0.2 * (yy - yc) / (yb - yc)))
    prog = np.where(yy > yb, 0.9 + 0.1 * np.abs(xx - m_ / 2) / (m_ / 2), prog)
    a[..., 3] *= np.clip((draw * 1.1 - prog) / 0.04, 0, 1)
    img.alpha_composite(Image.fromarray(a.astype(np.uint8), 'RGBA'), (sc.cx - sc.mark // 2, sc.cy - sc.mark // 2))
    g = ramp(t, 1.8, 2.7)
    fs = int(sc.mark * (0.4 + 0.6 * g))
    sc.paste(img, sc.flame, sc.cx, sc.cy + (sc.mark - fs) * 0.12, fs, g)
    sc.put_word(img, 1.0, per_letter=lambda i: ramp(t, 2.6 + i * 0.09, 2.9 + i * 0.09))
    return sc.fade_out(img, t)


def rising_embers(sc, t):
    base = sc.bg.copy()
    rng = np.random.default_rng(7)
    n = 70
    x0 = rng.uniform(0.1, 0.9, n) * sc.w
    sp = rng.uniform(0.16, 0.32, n) * sc.h
    ph = rng.uniform(0, 6.28, n)
    rad = rng.uniform(0.004, 0.009, n) * min(sc.w, sc.h)
    gather = ramp(t, 1.4, 2.8)
    img = None
    glow = np.zeros_like(base)
    for i in range(n):
        y = sc.h * 1.05 - sp[i] * t
        x = x0[i] + math.sin(t * 2 + ph[i]) * sc.w * 0.01
        tx, ty = sc.cx, sc.cy + sc.mark * 0.12
        x = x + (tx - x) * gather
        y = y + (ty - y) * gather
        fl = 0.7 + 0.3 * math.sin(t * 9 + ph[i])
        # Only the area near each ember (fast enough at full size).
        r = rad[i] * 6
        xa, xb = int(max(0, x - r)), int(min(sc.w, x + r))
        ya, yb = int(max(0, y - r)), int(min(sc.h, y + r))
        if xb <= xa or yb <= ya:
            continue
        yy, xx = np.mgrid[ya:yb, xa:xb].astype(float)
        d2 = ((xx - x) ** 2 + (yy - y) ** 2) / (rad[i] ** 2)
        glow[ya:yb, xa:xb] += np.exp(-d2 * 0.5)[..., None] * WARM * 0.9 * fl * (1 - 0.8 * gather)
    base += glow
    base += radial(sc.w, sc.h, sc.cx, sc.cy + sc.mark * 0.1, sc.mark * 0.6, WARM, 0.5 * gather * (0.85 + 0.1 * math.sin(t * 8)))
    img = sc.frame(base)
    sc.paste(img, sc.flame, sc.cx, sc.cy, sc.mark, ramp(t, 2.2, 2.9))
    sc.paste(img, sc.arch, sc.cx, sc.cy, sc.mark, ramp(t, 2.6, 3.3))
    sc.put_word(img, ramp(t, 3.0, 3.7))
    return sc.fade_out(img, t)


ANIMS = {'flame': flame_ignition, 'sweep': light_sweep, 'reveal': logo_reveal, 'embers': rising_embers}


def render(aid, aspect, out_dir, frames_dir=None):
    w, h = SIZES[aspect]
    sc = Scene(w, h)
    out = os.path.join(out_dir, f'hearth-{aid}-{aspect}.webm')
    cmd = ['ffmpeg', '-v', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{w}x{h}', '-r', str(FPS), '-i', '-',
           '-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '38', '-row-mt', '1', '-deadline', 'good', '-cpu-used', '3',
           '-pix_fmt', 'yuv420p', '-an', '-metadata', f'title=Hearth OS startup ({aid})', out]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    n = int(FPS * SECS)
    for i in range(n):
        t = i / FPS
        img = ANIMS[aid](sc, t).convert('RGB')
        p.stdin.write(img.tobytes())
        if frames_dir and i in (int(n * 0.12), int(n * 0.35), int(n * 0.6), int(n * 0.8)):
            img.resize((w // 2, h // 2), Image.LANCZOS).save(os.path.join(frames_dir, f'movie-{aid}-{aspect}-{i:03d}.png'))
    p.stdin.close()
    if p.wait() != 0:
        raise SystemExit(f'ffmpeg failed for {out}')
    print(out, os.path.getsize(out) // 1024, 'KB', flush=True)


if __name__ == '__main__':
    args = sys.argv[1:]
    only = args[args.index('--only') + 1] if '--only' in args else None
    frames = args[args.index('--frames') + 1] if '--frames' in args else None
    rest = [a for i, a in enumerate(args) if not a.startswith('--') and (i == 0 or not args[i - 1].startswith('--'))]
    out_dir = rest[0] if rest else OUT
    os.makedirs(out_dir, exist_ok=True)
    if frames:
        os.makedirs(frames, exist_ok=True)
    for aid in ANIMS:
        if only and aid != only:
            continue
        for aspect in SIZES:
            render(aid, aspect, out_dir, frames)
