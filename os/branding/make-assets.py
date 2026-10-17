#!/usr/bin/env python3
"""Renders the Hearth boot splash images from hearth-logo.svg (needs Playwright with Chromium, and Pillow).
Run from anywhere: python3 os/branding/make-assets.py  → os/system_files/usr/share/plymouth/themes/hearth/"""
import pathlib, math
from PIL import Image, ImageDraw, ImageFilter
from playwright.sync_api import sync_playwright

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE.parent / 'system_files/usr/share/plymouth/themes/hearth'
OUT.mkdir(parents=True, exist_ok=True)
CREAM = (239, 228, 214)

def chromium_png(html, w, h, path):
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_page(viewport={'width': w, 'height': h})
        pg.set_content(html)
        pg.wait_for_timeout(200)
        pg.screenshot(path=str(path), omit_background=True)
        b.close()

svg = (HERE / 'hearth-logo.svg').read_text()
# Logo, 512 px, transparent.
chromium_png(f"<body style='margin:0;background:transparent'>{svg}</body>", 512, 512, OUT / 'logo.png')
# Wordmark.
chromium_png("""<body style='margin:0;background:transparent;display:flex;align-items:center;justify-content:center;height:160px'>
  <div style="font:500 112px 'Inter Display';letter-spacing:.34em;margin-right:-.34em;color:rgb(239,228,214)">HEARTH</div></body>""",
  1024, 160, OUT / 'wordmark.png')
wm = Image.open(OUT / 'wordmark.png'); wm.crop(wm.getbbox()).save(OUT / 'wordmark.png')

# Ember glow behind the flame: warm radial falloff.
S = 512
g = Image.new('RGBA', (S, S))
px = g.load()
for y in range(S):
    for x in range(S):
        d = math.hypot(x - S / 2, y - S / 2) / (S / 2)
        a = max(0.0, 1 - d) ** 2.2
        px[x, y] = (255, int(120 + 60 * a), int(50 + 20 * a), int(200 * a))
g.save(OUT / 'glow.png')

# Throbber dot (ember).
D = 64
dot = Image.new('RGBA', (D, D))
dd = ImageDraw.Draw(dot)
dd.ellipse((8, 8, D - 8, D - 8), fill=(255, 150, 70, 255))
dot = dot.filter(ImageFilter.GaussianBlur(1.2))
dot.save(OUT / 'dot.png')

# Password field and bullet (encrypted disks).
E = Image.new('RGBA', (720, 88))
ImageDraw.Draw(E).rounded_rectangle((2, 2, 717, 85), radius=24, fill=(29, 22, 19, 235), outline=(255, 138, 61, 255), width=3)
E.save(OUT / 'entry.png')
B = Image.new('RGBA', (32, 32))
ImageDraw.Draw(B).ellipse((6, 6, 26, 26), fill=CREAM + (255,))
B.save(OUT / 'bullet.png')
print('wrote', *sorted(p.name for p in OUT.glob('*.png')))
