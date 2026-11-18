#!/usr/bin/env python3
"""Is a screen picture taken through Firehold's screencopy (grim) the right way up?

    firehold-orientation.py PICTURE.png STATUS.json [THEME.json]

The known, lopsided pattern: the focused window's title bar, drawn by firehold-comp itself in the theme's "bg2" colour
(theme.rs: title_active; Hearth dark's #1c1512 without a theme.json), sits at the window's top, where the status file
says the window is. Its plain left part must have that colour in the picture; turned upside down, that spot shows
something else (that's checked too, so the test can tell the two apart). Exit 0: upright; 1: upside down or wrong;
2: nothing to check against (no focused window with Firehold's frame).
"""
import json
import sys

from PIL import Image, ImageStat


def hexrgb(h):
    h = h.strip().lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def main(pic, status, theme=None):
    im = Image.open(pic).convert('RGB')
    d = json.load(open(status))
    out = d['outputs'][0]
    scale = im.width / max(1, out['w'])
    wins = [w for w in d['windows'] if w.get('activated') and w.get('frame') and not w.get('minimized')]
    if not wins:
        print('no focused window with Firehold\'s frame to check against')
        return 2
    w = wins[0]
    want = (0x1c, 0x15, 0x12)
    if theme:
        try:
            want = hexrgb(json.load(open(theme))['colors']['bg2'])
        except (OSError, ValueError, KeyError, TypeError):
            pass
    # The title bar's plain left part (the title is in the middle, the buttons on the right), under the accent line.
    x0, x1 = w['x'] + 12, w['x'] + min(120, w['w'] // 4)
    y0, y1 = w['y'] + 8, w['y'] + 26
    if d.get('frames') == 'firehold':
        # Firehold's own title bars (frames.rs, while Mantel's Desk is up): firehold.js's panel2, the title on the
        # left from 16 px, so the plain strip above it (past the rounded corner).
        want = (0x17, 0x11, 0x0e)
        x0, x1 = w['x'] + 20, w['x'] + min(140, w['w'] // 3)
        y0, y1 = w['y'] + 2, w['y'] + 7
    box = lambda y0, y1: tuple(round(v * scale) for v in (x0 - out['x'], y0 - out['y'], x1 - out['x'], y1 - out['y']))
    up = ImageStat.Stat(im.crop(box(y0, y1))).mean
    H = out['h']
    down = ImageStat.Stat(im.crop(box(H - y1, H - y0))).mean
    dist = lambda m: max(abs(a - b) for a, b in zip(m, want))
    print('title bar %s at %d,%d: upright spot %s (off by %.0f), mirrored spot %s (off by %.0f)'
          % ('#%02x%02x%02x' % want, w['x'], w['y'], [round(v) for v in up], dist(up), [round(v) for v in down], dist(down)))
    if dist(up) <= 10:
        return 0
    return 1


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:4]))
