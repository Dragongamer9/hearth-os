#!/usr/bin/env python3
"""Makes Hearth OS's boot art (1.1), all original and generated here from the Hearth mark:

  os-bases/peppermint/system_files/usr/share/hearth/grub-theme/   GRUB's look: slate background, the Hearth flame,
                                                                   blue highlight (theme.txt and its pictures; the
                                                                   fonts are made from Inter at image build time)
  os/system_files/usr/share/plymouth/themes/hearth-steam/          boot splash preset: slate blue with a light sweep
  os/system_files/usr/share/plymouth/themes/hearth-minimal/        boot splash preset: plain, a small mark, no motion

The default splash (themes/hearth) keeps its own script and pictures; only its background colours moved to slate.

  python3 tools/make-boot-art.py          (needs Pillow and numpy; writes the files above)
"""
import os
import shutil

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
HEARTH = os.path.join(ROOT, 'os/system_files/usr/share/plymouth/themes/hearth')
GRUB = os.path.join(ROOT, 'os-bases/peppermint/system_files/usr/share/hearth/grub-theme')
STEAM = os.path.join(ROOT, 'os/system_files/usr/share/plymouth/themes/hearth-steam')
MINIMAL = os.path.join(ROOT, 'os/system_files/usr/share/plymouth/themes/hearth-minimal')

SLATE_TOP = np.array([0x1b, 0x28, 0x38], float)
SLATE_BOTTOM = np.array([0x0e, 0x13, 0x1a], float)
BLUE = np.array([0x1a, 0x9f, 0xff], float)
SKY = np.array([0x66, 0xc0, 0xf4], float)
TEXT = (0xdc, 0xe5, 0xed)


def slate(w, h, glow=0.16, band=0.05):
    """The slate background: a vertical gradient, a soft blue glow above the middle and a faint diagonal band."""
    y, x = np.mgrid[0:h, 0:w].astype(float)
    t = (y / (h - 1))[..., None]
    img = SLATE_TOP * (1 - t) + SLATE_BOTTOM * t
    d = np.sqrt(((x - w * 0.5) / (w * 0.55)) ** 2 + ((y - h * 0.32) / (h * 0.55)) ** 2)
    g = np.clip(1 - d, 0, 1) ** 2.2 * glow
    img = img * (1 - g[..., None]) + BLUE * g[..., None]
    s = (x / w - y / h * 0.55)
    b = np.exp(-((s - 0.62) / 0.06) ** 2) * band
    img = img * (1 - b[..., None]) + SKY * b[..., None]
    # A light ordered dither so the gradient doesn't band on 8-bit screens (and the file stays small).
    bayer = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], float) / 16 - 0.5
    img += np.tile(bayer, (h // 4 + 1, w // 4 + 1))[:h, :w, None]
    return Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), 'RGB')


def recolor(im, rgb):
    """The same shape (its alpha) in one colour."""
    a = im.convert('RGBA').split()[3]
    out = Image.new('RGBA', im.size, rgb + (255,))
    out.putalpha(a)
    return out


def glow_dot(size, rgb, soft=0.42):
    y, x = np.mgrid[0:size, 0:size].astype(float)
    d = np.sqrt((x - (size - 1) / 2) ** 2 + (y - (size - 1) / 2) ** 2) / (size / 2)
    a = np.clip(1 - d, 0, 1) ** (1 / soft)
    arr = np.zeros((size, size, 4), np.uint8)
    arr[..., :3] = rgb
    arr[..., 3] = (a * 255).astype(np.uint8)
    return Image.fromarray(arr, 'RGBA')


def nine_slice(dirpath, prefix, fill, radius=6, size=32, border=None):
    """GRUB's styled box: a rounded rectangle cut into its nine pieces (prefix_nw.png ... prefix_c.png)."""
    big = Image.new('RGBA', (size * 4, size * 4), (0, 0, 0, 0))
    dr = ImageDraw.Draw(big)
    dr.rounded_rectangle((0, 0, size * 4 - 1, size * 4 - 1), radius=radius * 4, fill=fill,
                         outline=border, width=4 if border else 0)
    box = big.resize((size, size), Image.LANCZOS)
    r = radius
    cuts = {'nw': (0, 0, r, r), 'n': (r, 0, size - r, r), 'ne': (size - r, 0, size, r),
            'w': (0, r, r, size - r), 'c': (r, r, size - r, size - r), 'e': (size - r, r, size, size - r),
            'sw': (0, size - r, r, size), 's': (r, size - r, size - r, size), 'se': (size - r, size - r, size, size)}
    for k, c in cuts.items():
        box.crop(c).save(os.path.join(dirpath, f'{prefix}_{k}.png'), optimize=True)


THEME_TXT = '''# Hearth OS's GRUB menu (Steam-style look). Made by tools/make-boot-art.py; fonts (hearth-*.pf2) are made from
# Inter when the image is built. hearth-bls-grub copies this folder to /boot/grub2/themes/hearth and switches to it
# only when GRUB has everything it needs; otherwise the menu is GRUB's plain text one.
title-text: ""
desktop-image: "background.png"
desktop-image-scale-method: "stretch"
desktop-color: "#121820"
terminal-font: "Hearth Regular 18"
terminal-left: "10%"
terminal-top: "10%"
terminal-width: "80%"
terminal-height: "80%"
terminal-border: "0"

+ image {
  left = 50%-48
  top = 12%
  width = 96
  height = 96
  file = "logo.png"
}

+ label {
  left = 0
  top = 12%+108
  width = 100%
  height = 40
  align = "center"
  text = "Hearth OS"
  font = "Hearth Bold 28"
  color = "#dce5ed"
}

+ boot_menu {
  left = 50%-300
  top = 12%+176
  width = 600
  height = 50%
  item_font = "Hearth Regular 22"
  item_color = "#b8c4cf"
  selected_item_font = "Hearth Bold 22"
  selected_item_color = "#ffffff"
  item_height = 50
  item_padding = 18
  item_spacing = 8
  icon_width = 0
  icon_height = 0
  item_icon_space = 0
  selected_item_pixmap_style = "select_*.png"
  menu_pixmap_style = "menu_*.png"
  scrollbar = false
}

+ label {
  id = "__timeout__"
  left = 0
  top = 100%-84
  width = 100%
  height = 24
  align = "center"
  text = "Starting in %d s"
  font = "Hearth Regular 18"
  color = "#8f98a0"
}

+ label {
  left = 0
  top = 100%-52
  width = 100%
  height = 24
  align = "center"
  text = "Up and Down to choose    Enter to start    E to edit"
  font = "Hearth Regular 18"
  color = "#66727e"
}
'''


def grub_theme():
    os.makedirs(GRUB, exist_ok=True)
    for f in os.listdir(GRUB):
        if f.endswith('.png') or f == 'theme.txt':
            os.remove(os.path.join(GRUB, f))
    slate(1920, 1080).save(os.path.join(GRUB, 'background.png'), optimize=True)
    Image.open(os.path.join(HEARTH, 'logo.png')).convert('RGBA').resize((96, 96), Image.LANCZOS) \
        .save(os.path.join(GRUB, 'logo.png'), optimize=True)
    nine_slice(GRUB, 'select', (0x1a, 0x9f, 0xff, 255), radius=6)
    nine_slice(GRUB, 'menu', (0x0e, 0x14, 0x1c, 150), radius=8, border=(0x32, 0x40, 0x4f, 200))
    with open(os.path.join(GRUB, 'theme.txt'), 'w') as f:
        f.write(THEME_TXT)


def copy_common(dst):
    for f in ('entry.png', 'bullet.png'):
        shutil.copy(os.path.join(HEARTH, f), os.path.join(dst, f))


def steam_splash():
    os.makedirs(STEAM, exist_ok=True)
    copy_common(STEAM)
    Image.open(os.path.join(HEARTH, 'logo.png')).convert('RGBA').save(os.path.join(STEAM, 'logo.png'), optimize=True)
    recolor(Image.open(os.path.join(HEARTH, 'wordmark.png')), TEXT).save(os.path.join(STEAM, 'wordmark.png'), optimize=True)
    # The light sweep: a soft blue bar, brightest in the middle.
    w, h = 1024, 48
    y, x = np.mgrid[0:h, 0:w].astype(float)
    a = np.exp(-((x - w / 2) / (w * 0.22)) ** 2) * np.exp(-((y - h / 2) / (h * 0.18)) ** 2)
    arr = np.zeros((h, w, 4), np.uint8)
    arr[..., :3] = SKY.astype(np.uint8)
    arr[..., 3] = (np.clip(a, 0, 1) * 255).astype(np.uint8)
    Image.fromarray(arr, 'RGBA').save(os.path.join(STEAM, 'sweep.png'), optimize=True)
    # The track under it (a thin line) and the halo behind the mark.
    track = Image.new('RGBA', (1024, 4), (0x32, 0x40, 0x4f, 255))
    track.save(os.path.join(STEAM, 'track.png'), optimize=True)
    glow_dot(512, tuple(BLUE.astype(int)), soft=0.5).save(os.path.join(STEAM, 'halo.png'), optimize=True)


def minimal_splash():
    os.makedirs(MINIMAL, exist_ok=True)
    copy_common(MINIMAL)
    Image.open(os.path.join(HEARTH, 'logo.png')).convert('RGBA').resize((256, 256), Image.LANCZOS) \
        .save(os.path.join(MINIMAL, 'logo.png'), optimize=True)


if __name__ == '__main__':
    grub_theme()
    steam_splash()
    minimal_splash()
    for d in (GRUB, STEAM, MINIMAL):
        print(d, sorted(os.listdir(d)))
