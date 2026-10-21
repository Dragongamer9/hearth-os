# Original cover art for the browser preview's invented games (ui/index.html MOCK library), used ONLY for the
# website's screenshots (tools/site-shots/shots.py). Every picture is drawn here from vector shapes: no real game's
# art, logo or name. Hearth itself never loads these.
#
#   python3 tools/site-shots/art.py            # writes tools/site-shots/art/<id>-{cover,hero,wide,logo}.webp
#
# Needs Python Playwright with Chromium, Pillow, and the fonts used below (Inter, Poppins, Lora, TeX Gyre,
# GFS Baskerville, DejaVu): they're on most Linux systems with texlive-fonts / fonts-* packages.
import asyncio, math, os, random, sys
from io import BytesIO

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'art')

# ---------------------------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------------------------
def f(x): return f'{x:.1f}'

def lin(id, stops, x1=0, y1=0, x2=0, y2=1):
    s = ''.join(f'<stop offset="{o}" stop-color="{c}"' + (f' stop-opacity="{a}"' if a is not None else '') + '/>'
                for o, c, *r in stops for a in [r[0] if r else None])
    return f'<linearGradient id="{id}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">{s}</linearGradient>'

def rad(id, stops, cx=.5, cy=.5, r=.5):
    s = ''.join(f'<stop offset="{o}" stop-color="{c}"' + (f' stop-opacity="{a}"' if a is not None else '') + '/>'
                for o, c, *r2 in stops for a in [r2[0] if r2 else None])
    return f'<radialGradient id="{id}" cx="{cx}" cy="{cy}" r="{r}">{s}</radialGradient>'

def glow(cx, cy, r, color, op=1):
    gid = f'gl{abs(hash((cx, cy, r, color))) % 10**8}'
    return (f'<defs>{rad(gid, [(0, color, op), (.45, color, op * .35), (1, color, 0)])}</defs>'
            f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}" fill="url(#{gid})"/>')

def ridge(W, H, y0, amp, seed, color, step=None, octaves=4, op=1, rough=1.0):
    """A mountain/hill silhouette across the width, filled down to the bottom."""
    rnd = random.Random(seed)
    ph = [rnd.uniform(0, 6.3) for _ in range(octaves)]
    step = step or max(4, W / 160)
    pts = []
    x = -10
    while x <= W + 10:
        y = y0
        for o in range(octaves):
            fr = (o + 1) ** 1.7 / W * 6.0
            y -= amp * (1 / (o + 1) ** rough) * math.sin(x * fr * math.pi + ph[o])
        pts.append(f'{f(x)},{f(y)}')
        x += step
    return f'<path d="M-10,{H + 10} L{" L".join(pts)} L{W + 10},{H + 10} Z" fill="{color}" opacity="{op}"/>'

def peaks(W, H, y0, seed, color, n=7, hmin=.3, hmax=1, amp=200, op=1):
    """Sharp jagged peaks."""
    rnd = random.Random(seed)
    xs = sorted(rnd.uniform(-.1, 1.1) * W for _ in range(n * 2))
    pts = [f'-20,{H + 10}', f'-20,{y0}']
    for i, x in enumerate(xs):
        y = y0 - (rnd.uniform(hmin, hmax) * amp if i % 2 == 0 else rnd.uniform(0, .25) * amp)
        pts.append(f'{f(x)},{f(y)}')
    pts += [f'{W + 20},{y0}', f'{W + 20},{H + 10}']
    return f'<polygon points="{" ".join(pts)}" fill="{color}" opacity="{op}"/>'

def pine(x, base, h, color, w=None):
    w = w or h * .42
    t = []
    for i in range(4):
        yb = base - h * (i * .22)
        ww = w * (1 - i * .2)
        yt = yb - h * .42
        t.append(f'M{f(x - ww / 2)},{f(yb)} L{f(x)},{f(yt)} L{f(x + ww / 2)},{f(yb)} Z')
    t.append(f'M{f(x - h * .025)},{f(base)} h{f(h * .05)} v{f(h * .06)} h{f(-h * .05)} Z')
    return f'<path d="{" ".join(t)}" fill="{color}"/>'

def forest(W, base, hmin, hmax, seed, color, gap=None):
    rnd = random.Random(seed)
    out, x = [], -20
    while x < W + 40:
        h = rnd.uniform(hmin, hmax)
        out.append(pine(x, base + rnd.uniform(-h * .05, h * .08), h, color))
        x += (gap or hmax * .32) * rnd.uniform(.5, 1.2)
    return ''.join(out)

def stars(W, H, n, seed, ymax=1.0, color='#fff', rmax=1.6, op=1):
    rnd = random.Random(seed)
    return ''.join(f'<circle cx="{f(rnd.uniform(0, W))}" cy="{f(rnd.uniform(0, H * ymax) ** 1.0)}" r="{f(rnd.uniform(.3, rmax))}" '
                   f'fill="{color}" opacity="{f(rnd.uniform(.25, 1) * op)}"/>' for _ in range(n))

def grain(W, H, op=.06):
    return (f'<filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".85" numOctaves="2" stitchTiles="stitch"/>'
            f'<feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 1.4 -.4"/></filter>'
            f'<rect width="{W}" height="{H}" filter="url(#grain)" opacity="{op}" style="mix-blend-mode:overlay"/>')

def vignette(W, H, op=.55, color='#000'):
    return (f'<defs>{rad("vig", [(.55, color, 0), (1, color, op)], r=.75)}</defs>'
            f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')

def bottom_fade(W, H, frac=.45, color='#000', op=.85):
    return (f'<defs>{lin("bf", [(0, color, 0), (1, color, op)])}</defs>'
            f'<rect y="{f(H * (1 - frac))}" width="{W}" height="{f(H * frac + 1)}" fill="url(#bf)"/>')

def birds(x, y, n, s, color, seed):
    rnd = random.Random(seed)
    out = []
    for _ in range(n):
        bx, by, k = x + rnd.uniform(-s * 4, s * 4), y + rnd.uniform(-s * 2, s * 2), s * rnd.uniform(.6, 1.2)
        out.append(f'<path d="M{f(bx - k)},{f(by)} q{f(k * .5)},{f(-k * .55)} {f(k)},0 q{f(k * .5)},{f(-k * .55)} {f(k)},0" fill="none" stroke="{color}" stroke-width="{f(k * .14)}" stroke-linecap="round"/>')
    return ''.join(out)


class C:
    """Canvas description handed to each scene: size, portrait or wide, where the subject goes."""
    def __init__(self, W, H, kind):
        self.W, self.H, self.kind = W, H, kind
        self.u = min(W, H)
        self.portrait = H > W
        # The subject sits in the middle of a cover, right of centre on wide art (Hearth puts the logo bottom-left).
        self.cx = W * (.5 if self.portrait else (.66 if kind == 'hero' else .5))
        self.hz = H * (.6 if self.portrait else .68)


# ---------------------------------------------------------------------------------------------------------------
# scenes: each returns SVG for a W x H canvas
# ---------------------------------------------------------------------------------------------------------------
def s_starfall(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.H * (.58 if c.portrait else .62)
    o = [f'<defs>{lin("sky", [(0, "#0b0624"), (.45, "#3a0f57"), (.8, "#c2316b"), (1, "#ff8a4c")])}'
         f'{lin("sun", [(0, "#ffe27a"), (.5, "#ff8a3d"), (1, "#ff2e7a")])}'
         f'{lin("road", [(0, "#1a0730"), (1, "#05010d")])}'
         f'<clipPath id="sunc"><circle cx="{f(cx)}" cy="{f(hz - u * .02)}" r="{f(u * .3)}"/></clipPath></defs>',
         f'<rect width="{W}" height="{hz + 2}" fill="url(#sky)"/>',
         stars(W, H, 160, 1, .45, rmax=1.5)]
    # streaking stars
    rnd = random.Random(7)
    for i in range(9):
        x, y, l = rnd.uniform(0, W), rnd.uniform(0, hz * .5), rnd.uniform(u * .12, u * .35)
        o.append(f'<line x1="{f(x)}" y1="{f(y)}" x2="{f(x - l * .8)}" y2="{f(y + l * .45)}" stroke="#fff" stroke-width="{f(rnd.uniform(1, 2.6))}" stroke-linecap="round" opacity="{f(rnd.uniform(.3, .85))}"/>')
        o.append(glow(x, y, u * .025, '#ffe9b0', .9))
    o.append(glow(cx, hz, u * .7, '#ff3d7f', .45))
    # striped sun
    bands = ''.join(f'<rect x="0" y="{f(hz - u * .02 - u * .3 + u * .6 * (.45 + i * .085))}" width="{W}" height="{f(u * (.012 + i * .007))}" fill="#3a0f57"/>' for i in range(7))
    o.append(f'<g clip-path="url(#sunc)"><rect width="{W}" height="{H}" fill="url(#sun)" y="{f(hz - u * .34)}" height="{f(u * .62)}"/>{bands}</g>')
    o.append(peaks(W, H, hz, 3, '#24093d', n=9, amp=u * .14))
    o.append(peaks(W, H, hz, 5, '#140525', n=6, amp=u * .07))
    # road + perspective grid
    o.append(f'<rect y="{f(hz)}" width="{W}" height="{f(H - hz)}" fill="url(#road)"/>')
    g = []
    for i in range(-16, 17):
        x2 = cx + i * W * .16
        g.append(f'<line x1="{f(cx + i * u * .01)}" y1="{f(hz)}" x2="{f(cx + (x2 - cx) * 3)}" y2="{f(H + (H - hz) * 2)}"/>')
    for i in range(1, 14):
        t = (i / 13) ** 2.2
        y = hz + (H - hz) * t
        g.append(f'<line x1="0" y1="{f(y)}" x2="{W}" y2="{f(y)}" opacity="{f(.25 + t * .75)}"/>')
    o.append(f'<g stroke="#ff4fa3" stroke-width="{f(u * .004)}" opacity=".7">{"".join(g)}</g>')
    # road edges
    rw = u * .9
    o.append(f'<path d="M{f(cx - u * .03)},{f(hz)} L{f(cx - rw)},{f(H)} L{f(cx + rw)},{f(H)} L{f(cx + u * .03)},{f(hz)} Z" fill="#0d0418" opacity=".85"/>')
    o.append(f'<path d="M{f(cx - u * .03)},{f(hz)} L{f(cx - rw)},{f(H)} M{f(cx + u * .03)},{f(hz)} L{f(cx + rw)},{f(H)}" stroke="#4ff0ff" stroke-width="{f(u * .008)}" fill="none"/>')
    for i in range(8):
        t0, t1 = (i / 8) ** 2, ((i + .45) / 8) ** 2
        o.append(f'<path d="M{f(cx)},{f(hz + (H - hz) * t0)} L{f(cx)},{f(hz + (H - hz) * t1)}" stroke="#ffd166" stroke-width="{f(u * (.003 + .02 * t1))}"/>')
    # car (rear view)
    cw, chh = u * .5, u * .16
    cy = H - (H - hz) * (.38 if c.portrait else .3)
    x0 = cx - cw / 2
    o.append(glow(cx, cy + chh * .55, cw * .7, '#ff2e55', .35))
    o.append(f'<ellipse cx="{f(cx)}" cy="{f(cy + chh * .62)}" rx="{f(cw * .56)}" ry="{f(chh * .12)}" fill="#000" opacity=".6"/>')
    o.append(f'<path d="M{f(x0)},{f(cy + chh * .55)} L{f(x0 + cw * .04)},{f(cy + chh * .12)} Q{f(x0 + cw * .2)},{f(cy - chh * .55)} {f(cx)},{f(cy - chh * .58)} '
             f'Q{f(x0 + cw * .8)},{f(cy - chh * .55)} {f(x0 + cw * .96)},{f(cy + chh * .12)} L{f(x0 + cw)},{f(cy + chh * .55)} Z" fill="#12061f" stroke="#ff7ad0" stroke-width="{f(u * .004)}"/>')
    o.append(f'<path d="M{f(x0 + cw * .22)},{f(cy - chh * .05)} Q{f(cx)},{f(cy - chh * .5)} {f(x0 + cw * .78)},{f(cy - chh * .05)} Z" fill="#2c1550"/>')
    o.append(f'<rect x="{f(x0 + cw * .06)}" y="{f(cy + chh * .14)}" width="{f(cw * .88)}" height="{f(chh * .12)}" rx="{f(chh * .05)}" fill="#ff2e55"/>')
    o.append(glow(x0 + cw * .14, cy + chh * .2, u * .07, '#ff4466', .9) + glow(x0 + cw * .86, cy + chh * .2, u * .07, '#ff4466', .9))
    o.append(f'<rect x="{f(cx - cw * .1)}" y="{f(cy + chh * .32)}" width="{f(cw * .2)}" height="{f(chh * .12)}" fill="#ffd166" opacity=".9"/>')
    o.append(f'<rect x="{f(x0 + cw * .04)}" y="{f(cy + chh * .5)}" width="{f(cw * .12)}" height="{f(chh * .28)}" rx="4" fill="#05010a"/><rect x="{f(x0 + cw * .84)}" y="{f(cy + chh * .5)}" width="{f(cw * .12)}" height="{f(chh * .28)}" rx="4" fill="#05010a"/>')
    return ''.join(o)


def s_hollow(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#05080a"), (.55, "#1b2a2c"), (1, "#3d4d47")])}{lin("fog", [(0, "#a9bdb4", 0), (1, "#a9bdb4", .55)])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         stars(W, H, 50, 2, .4, rmax=1.1, op=.6)]
    mx, my = cx + (u * .22 if c.portrait else -u * .55), H * (.33 if c.portrait else .26)
    o.append(glow(mx, my, u * .5, '#dfe9d6', .35))
    o.append(f'<circle cx="{f(mx)}" cy="{f(my)}" r="{f(u * .085)}" fill="#eef3e4"/><circle cx="{f(mx + u * .025)}" cy="{f(my - u * .015)}" r="{f(u * .07)}" fill="#cfd8c7" opacity=".35"/>')
    o.append(forest(W, hz - u * .05, u * .2, u * .32, 11, '#22322f'))
    o.append(f'<rect y="{f(hz - u * .25)}" width="{W}" height="{f(u * .3)}" fill="url(#fog)"/>')
    o.append(forest(W, hz + u * .08, u * .35, u * .5, 12, '#141f1d'))
    o.append(f'<rect y="{f(hz - u * .05)}" width="{W}" height="{f(u * .2)}" fill="url(#fog)" opacity=".7"/>')
    # cabin with a lit window
    bx, by, bw = cx - u * .1, hz + u * .12, u * .2
    o.append(glow(bx + bw * .7, by - bw * .25, u * .25, '#ff9a3d', .5))
    o.append(f'<path d="M{f(bx)},{f(by)} v{f(-bw * .45)} l{f(bw * .5)},{f(-bw * .35)} l{f(bw * .5)},{f(bw * .35)} v{f(bw * .45)} Z" fill="#0a100f"/>')
    o.append(f'<rect x="{f(bx + bw * .6)}" y="{f(by - bw * .35)}" width="{f(bw * .2)}" height="{f(bw * .17)}" fill="#ffb25c"/>')
    o.append(f'<rect x="{f(bx + bw * .15)}" y="{f(by - bw * .7)}" width="{f(bw * .1)}" height="{f(bw * .25)}" fill="#0a100f"/>')
    o.append(ridge(W, H, hz + u * .14, u * .02, 4, '#070b0a'))
    o.append(forest(W, H + u * .05, u * .6, u * .85, 13, '#030505', gap=u * .5))
    # a red mark in the trees: something is out there
    o.append(f'<circle cx="{f(cx + u * .26)}" cy="{f(hz + u * .02)}" r="{f(u * .006)}" fill="#ff3b30"/><circle cx="{f(cx + u * .277)}" cy="{f(hz + u * .02)}" r="{f(u * .006)}" fill="#ff3b30"/>')
    return ''.join(o)


def s_ironvale(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#1d2c45"), (.6, "#5a6f8c"), (1, "#d8b67a")])}{lin("hill", [(0, "#3a4a3a"), (1, "#1a231c")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         glow(cx, hz - u * .1, u * .8, '#ffd79a', .5)]
    # clouds
    rnd = random.Random(5)
    for i in range(7):
        x, y, s = rnd.uniform(0, W), rnd.uniform(H * .08, hz * .55), rnd.uniform(.6, 1.4) * u * .18
        o.append(f'<g fill="#e9e2d2" opacity="{f(rnd.uniform(.12, .3))}"><ellipse cx="{f(x)}" cy="{f(y)}" rx="{f(s)}" ry="{f(s * .18)}"/><ellipse cx="{f(x + s * .3)}" cy="{f(y - s * .1)}" rx="{f(s * .5)}" ry="{f(s * .16)}"/></g>')
    o.append(peaks(W, H, hz - u * .05, 8, '#46566c', n=8, amp=u * .25))
    o.append(ridge(W, H, hz, u * .05, 9, '#33402f'))
    # castle on the hill
    k = u * .0042
    base = hz - u * .02
    def tower(x, w, h, roof=True):
        s = f'<rect x="{f(x - w / 2)}" y="{f(base - h)}" width="{f(w)}" height="{f(h)}"/>'
        for i in range(3):
            s += f'<rect x="{f(x - w / 2 + i * w / 2.5 - w * .02)}" y="{f(base - h - w * .18)}" width="{f(w / 5)}" height="{f(w * .2)}"/>'
        if roof:
            s += f'<path d="M{f(x - w * .62)},{f(base - h)} L{f(x)},{f(base - h - w * 1.3)} L{f(x + w * .62)},{f(base - h)} Z" fill="#2c3a55"/>'
            s += f'<path d="M{f(x)},{f(base - h - w * 1.3)} v{f(-w * .7)}" stroke="#121820" stroke-width="{f(k * 2)}"/><path d="M{f(x)},{f(base - h - w * 2)} l{f(w * .6)},{f(w * .15)} l{f(-w * .6)},{f(w * .15)} Z" fill="#c8402e"/>'
        return s
    o.append(f'<g fill="#121820">'
             f'<rect x="{f(cx - u * .3)}" y="{f(base - u * .12)}" width="{f(u * .6)}" height="{f(u * .12)}"/>'
             + ''.join(f'<rect x="{f(cx - u * .3 + i * u * .04)}" y="{f(base - u * .14)}" width="{f(u * .022)}" height="{f(u * .025)}"/>' for i in range(15))
             + tower(cx - u * .3, u * .07, u * .22) + tower(cx + u * .3, u * .07, u * .22) + tower(cx, u * .1, u * .34)
             + tower(cx - u * .14, u * .06, u * .18, False) + tower(cx + u * .14, u * .06, u * .18, False)
             + f'</g>')
    o.append(f'<g fill="#ffcf7a">' + ''.join(f'<rect x="{f(cx + dx * u)}" y="{f(base - dy * u)}" width="{f(u * .012)}" height="{f(u * .022)}"/>' for dx, dy in [(-.01, .25), (-.31, .16), (.29, .16), (.01, .16), (-.15, .1), (.13, .1)]) + '</g>')
    o.append(ridge(W, H, hz + u * .12, u * .06, 10, 'url(#hill)'))
    # banners/spears of an army in the foreground
    rnd = random.Random(3)
    sp = []
    for i in range(int(W / (u * .035))):
        x = i * u * .035 + rnd.uniform(-5, 5)
        y = H - u * (.08 + rnd.uniform(0, .05))
        sp.append(f'<line x1="{f(x)}" y1="{f(H)}" x2="{f(x + u * .02)}" y2="{f(y - u * .18)}"/>')
    o.append(f'<g stroke="#0b0f0c" stroke-width="{f(k * 1.6)}">{"".join(sp)}</g>')
    for x, col in [(.15, '#a8322a'), (.82, '#2a4f8a'), (.5, '#a8322a')]:
        bx = W * x
        o.append(f'<path d="M{f(bx)},{f(H)} V{f(H - u * .42)}" stroke="#0b0f0c" stroke-width="{f(k * 3)}"/><path d="M{f(bx)},{f(H - u * .41)} h{f(u * .1)} l{f(-u * .025)},{f(u * .06)} l{f(u * .025)},{f(u * .06)} h{f(-u * .1)} Z" fill="{col}"/>')
    o.append(f'<rect y="{f(H - u * .1)}" width="{W}" height="{f(u * .1)}" fill="#0b0f0c"/>')
    # faint hex grid
    hx = []
    r = u * .07
    for row in range(int(H / (r * 1.5)) + 2):
        for col in range(int(W / (r * 1.732)) + 2):
            x = col * r * 1.732 + (r * .866 if row % 2 else 0)
            y = row * r * 1.5
            hx.append('M' + ' L'.join(f'{f(x + r * math.cos(math.pi / 3 * i + math.pi / 6))},{f(y + r * math.sin(math.pi / 3 * i + math.pi / 6))}' for i in range(6)) + 'Z')
    o.append(f'<path d="{" ".join(hx)}" fill="none" stroke="#fff" stroke-width="1" opacity=".05"/>')
    return ''.join(o)


def s_neon(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#07021a"), (.6, "#24064a"), (1, "#5a0b6e")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         glow(cx, H * .55, u * .9, '#ff2fd0', .25)]
    rnd = random.Random(21)
    neon = ['#ff2fd0', '#22e6ff', '#ffe14d', '#7c5cff']
    for layer, (col, hmin, hmax, wmul, op) in enumerate([('#1b0b3a', .3, .7, 1, 1), ('#120626', .2, .55, 1.4, 1), ('#07020f', .1, .35, 1.9, 1)]):
        x = -20
        while x < W + 20:
            bw = rnd.uniform(.08, .16) * u * wmul
            bh = rnd.uniform(hmin, hmax) * H
            top = H - bh - (2 - layer) * u * .06
            o.append(f'<rect x="{f(x)}" y="{f(top)}" width="{f(bw)}" height="{f(H)}" fill="{col}"/>')
            # windows
            if layer < 2:
                for wy in range(int(top + 10), int(H), int(u * .03)):
                    for wx in range(int(x + 6), int(x + bw - 6), int(u * .025)):
                        if rnd.random() < .22:
                            o.append(f'<rect x="{wx}" y="{wy}" width="{f(u * .01)}" height="{f(u * .013)}" fill="{rnd.choice(neon)}" opacity="{f(rnd.uniform(.25, .7))}"/>')
            # signs
            if rnd.random() < .45 and layer > 0:
                col2 = rnd.choice(neon)
                sy = top + rnd.uniform(.05, .3) * bh
                o.append(f'<rect x="{f(x + bw * .2)}" y="{f(sy)}" width="{f(bw * .6)}" height="{f(u * .045)}" rx="4" fill="none" stroke="{col2}" stroke-width="{f(u * .006)}" style="filter:drop-shadow(0 0 {f(u * .012)}px {col2})"/>')
            x += bw + rnd.uniform(0, u * .02)
    # rain
    o.append(f'<g stroke="#9fe8ff" stroke-width="1.2" opacity=".28">' + ''.join(
        f'<line x1="{f(x)}" y1="{f(y)}" x2="{f(x - u * .02)}" y2="{f(y + u * .07)}"/>' for x, y in [(rnd.uniform(0, W), rnd.uniform(0, H)) for _ in range(140)]) + '</g>')
    # rooftop + runner leaping with a parcel
    ry = H * (.8 if c.portrait else .82)
    o.append(f'<path d="M-10,{f(ry)} H{f(cx - u * .12)} V{H + 10} H-10 Z M{f(cx + u * .2)},{f(ry + u * .05)} H{W + 10} V{H + 10} H{f(cx + u * .2)} Z" fill="#030108"/>')
    o.append(f'<path d="M-10,{f(ry)} H{f(cx - u * .12)} M{f(cx + u * .2)},{f(ry + u * .05)} H{W + 10}" stroke="#22e6ff" stroke-width="{f(u * .006)}" style="filter:drop-shadow(0 0 6px #22e6ff)"/>')
    s = u * .0019
    px, py = cx + u * .02, ry - u * .26
    o.append(glow(px, py, u * .2, '#22e6ff', .3))
    o.append(f'<g transform="translate({f(px)},{f(py)}) scale({f(s)}) rotate(-8)" fill="#05010c" stroke="#ff2fd0" stroke-width="4" stroke-linejoin="round">'
             '<circle cx="18" cy="-70" r="16"/>'
             '<path d="M0,-50 C20,-52 35,-40 30,-10 L22,25 C40,30 60,45 72,60 L60,70 C45,55 30,45 10,40 L-10,70 L-40,85 L-44,72 L-22,58 L-8,20 L-2,-20 C-20,-20 -40,-10 -55,0 L-62,-10 C-45,-28 -20,-45 0,-50 Z"/>'
             '<rect x="-60" y="-58" width="42" height="32" rx="4" fill="#ffe14d" stroke="#05010c"/>'
             '<path d="M30,-35 L70,-55 L76,-46 L36,-22 Z"/></g>')
    # speed trails
    o.append(''.join(f'<line x1="{f(px - u * (.14 + i * .05))}" y1="{f(py + u * (-.03 + i * .03))}" x2="{f(px - u * (.4 + i * .08))}" y2="{f(py + u * (.02 + i * .04))}" stroke="{neon[i % 2]}" stroke-width="{f(u * .006)}" stroke-linecap="round" opacity=".7"/>' for i in range(3)))
    return ''.join(o)


def s_tidebound(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    hz = H * (.55 if c.portrait else .6)
    o = [f'<defs>{lin("sky", [(0, "#14284b"), (.5, "#e0745a"), (.85, "#ffc27a"), (1, "#ffe0a3")])}{lin("sea", [(0, "#f1a36d"), (.12, "#3c5d78"), (1, "#0b1b2b")])}</defs>',
         f'<rect width="{W}" height="{hz + 2}" fill="url(#sky)"/>']
    sx = cx + (u * .18 if c.portrait else -u * .7)
    o.append(glow(sx, hz - u * .05, u * .7, '#ffd08a', .7))
    o.append(f'<circle cx="{f(sx)}" cy="{f(hz - u * .06)}" r="{f(u * .1)}" fill="#fff1cf"/>')
    rnd = random.Random(4)
    for i in range(6):
        x, y, s = rnd.uniform(0, W), rnd.uniform(hz * .15, hz * .8), rnd.uniform(.12, .32) * u
        o.append(f'<ellipse cx="{f(x)}" cy="{f(y)}" rx="{f(s)}" ry="{f(s * .07)}" fill="#ffd9b0" opacity="{f(rnd.uniform(.2, .45))}"/>')
    o.append(ridge(W, H, hz, u * .02, 2, '#5b4a6e', op=.8))
    o.append(f'<rect y="{f(hz)}" width="{W}" height="{f(H - hz)}" fill="url(#sea)"/>')
    o.append(f'<rect x="{f(sx - u * .05)}" y="{f(hz)}" width="{f(u * .1)}" height="{f(H - hz)}" fill="#ffe2a8" opacity=".35"/>')
    for i in range(16):
        y = hz + (H - hz) * (i / 16) ** 1.6
        o.append(f'<path d="M-10,{f(y)} ' + ' '.join(f'q{f(u * .04)},{f(-u * .012 * (1 + i * .15))} {f(u * .08)},0' for _ in range(int(W / (u * .08)) + 2)) + f'" stroke="#ffd2a0" stroke-width="{f(1 + i * .25)}" fill="none" opacity="{f(.08 + i * .015)}"/>')
    # ship
    k = u * .0026
    bx, by = cx, hz + (H - hz) * .38
    o.append(f'<g transform="translate({f(bx)},{f(by)}) scale({f(k)})" fill="#0b1420">'
             '<path d="M-120,0 L120,0 L95,38 L-90,38 Z"/><path d="M-135,-8 L-120,0 L120,0 L150,-14 L120,-6 Z"/>'
             '<path d="M-4,0 V-230 M-70,0 V-160 M64,0 V-170" stroke="#0b1420" stroke-width="6"/>'
             '<path d="M0,-222 C40,-190 46,-90 0,-40 Z" fill="#f2e2c4"/><path d="M-8,-222 C-60,-190 -66,-90 -8,-40 Z" fill="#e9d3ae"/>'
             '<path d="M-66,-152 C-34,-120 -32,-70 -66,-30 Z" fill="#f2e2c4"/><path d="M-74,-152 C-110,-120 -112,-70 -74,-30 Z" fill="#e2c9a0"/>'
             '<path d="M68,-162 C100,-130 102,-70 68,-30 Z" fill="#f2e2c4"/><path d="M60,-162 C20,-130 20,-70 60,-30 Z" fill="#e2c9a0"/>'
             '<path d="M-4,-232 l40,10 l-40,10 Z" fill="#d6463a"/><path d="M120,-6 L190,-40" stroke="#0b1420" stroke-width="5"/></g>')
    o.append(f'<ellipse cx="{f(bx)}" cy="{f(by + 40 * k)}" rx="{f(150 * k)}" ry="{f(8 * k)}" fill="#ffe2a8" opacity=".25"/>')
    o.append(birds(cx - u * .2, hz - u * .3, 5, u * .02, '#2a2440', 9))
    # foreground wave
    o.append(f'<path d="M-10,{f(H - u * .08)} ' + ' '.join(f'q{f(u * .09)},{f(-u * .06)} {f(u * .18)},0' for _ in range(int(W / (u * .18)) + 2)) + f' V{H + 10} H-10 Z" fill="#081624"/>')
    return ''.join(o)


def s_papermoon(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.H * (.7 if c.portrait else .74)
    o = [f'<defs>{lin("sky", [(0, "#2b2a5c"), (.6, "#6b5b9a"), (1, "#f2a6a0")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         stars(W, H, 60, 6, .6, '#fff6d8', 1.8)]
    # paper-cut layered hills
    for i, col in enumerate(['#8a76b8', '#a690c9', '#c7a9d6']):
        o.append(ridge(W, H, hz - u * (.25 - i * .09), u * .04, 30 + i, col, octaves=3))
    # crescent moon on a string
    mx, my = cx + (u * .26 if c.portrait else -u * .5), H * (.29 if c.portrait else .3)
    o.append(f'<line x1="{f(mx)}" y1="0" x2="{f(mx)}" y2="{f(my - u * .1)}" stroke="#fff6d8" stroke-width="2" opacity=".7"/>')
    o.append(glow(mx, my, u * .3, '#ffe9a8', .45))
    o.append(f'<defs><mask id="cres"><rect width="{W}" height="{H}" fill="#fff"/><circle cx="{f(mx + u * .05)}" cy="{f(my - u * .03)}" r="{f(u * .09)}" fill="#000"/></mask></defs>')
    o.append(f'<circle cx="{f(mx)}" cy="{f(my)}" r="{f(u * .1)}" fill="#ffe7a0" mask="url(#cres)" style="filter:drop-shadow(4px 6px 0 rgba(60,40,90,.35))"/>')
    # cafe
    k = u * (.0018 if c.portrait else .0021)
    fx, fy = cx - (u * .04 if c.portrait else 0), hz + u * .04
    o.append(glow(fx, fy - 90 * k, u * .4, '#ffb36b', .5))
    o.append(f'<g transform="translate({f(fx)},{f(fy)}) scale({f(k)})" style="filter:drop-shadow(6px 8px 0 rgba(50,30,80,.3))">'
             '<rect x="-150" y="-230" width="300" height="230" fill="#f6e2c8"/>'
             '<path d="M-170,-230 L0,-320 L170,-230 Z" fill="#c9576b"/>'
             '<rect x="-120" y="-150" width="110" height="100" rx="6" fill="#ffcf7e"/><rect x="30" y="-170" width="70" height="170" rx="6" fill="#7a4a5a"/>'
             '<circle cx="85" cy="-85" r="5" fill="#ffd166"/>'
             + ''.join(f'<path d="M{-160 + i * 40},-200 h40 v24 a20,20 0 0 1 -40,0 Z" fill="{["#e8eef4", "#c9576b"][i % 2]}"/>' for i in range(8))
             + '<rect x="-80" y="-280" width="160" height="38" rx="8" fill="#3b2f5e"/>'
             '<path d="M-120,-100 h110 M-65,-150 v100" stroke="#e6a95a" stroke-width="4"/>'
             '<g fill="#5a3b4a"><rect x="-108" y="-80" width="20" height="26" rx="3"/><rect x="-46" y="-86" width="22" height="32" rx="3"/></g>'
             '</g>')
    o.append(f'<text x="{f(fx)}" y="{f(fy - 252 * k)}" font-family="TeX Gyre Chorus" font-size="{f(30 * k)}" fill="#ffe7a0" text-anchor="middle">open late</text>')
    # steaming cup in foreground
    o.append(f'<rect y="{f(hz + u * .04)}" width="{W}" height="{H}" fill="#3b2f5e"/>')
    o.append(ridge(W, H, hz + u * .06, u * .015, 33, '#2c2349', octaves=2))
    k2 = u * .0016
    qx, qy = cx + (-u * .26 if c.portrait else u * .52), H - u * .06
    o.append(f'<g transform="translate({f(qx)},{f(qy)}) scale({f(k2)})"><ellipse cx="0" cy="0" rx="90" ry="14" fill="#efe1ee"/>'
             '<path d="M-55,-90 h110 v50 a55,40 0 0 1 -110,0 Z" fill="#fff3e8"/><path d="M55,-75 a22,22 0 0 1 0,40" stroke="#fff3e8" stroke-width="12" fill="none"/>'
             '<path d="M-20,-110 c-15,-25 15,-35 0,-60 M15,-110 c-15,-25 15,-35 0,-60" stroke="#fff3e8" stroke-width="7" fill="none" opacity=".7" stroke-linecap="round"/></g>')
    return ''.join(o)


def s_grimforge(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#0a0505"), (.6, "#3a0d07"), (1, "#a8320f")])}{lin("anv", [(0, "#565a63"), (1, "#16171b")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    o.append(peaks(W, H, hz - u * .05, 41, '#2a0c08', n=7, amp=u * .5, hmin=.5))
    o.append(glow(cx, hz, u * .9, '#ff5a1f', .55))
    o.append(peaks(W, H, hz + u * .05, 42, '#150605', n=9, amp=u * .3))
    # lava rivers
    o.append(f'<path d="M{f(cx - u * .9)},{H} C{f(cx - u * .4)},{f(hz + u * .25)} {f(cx + u * .1)},{f(hz + u * .4)} {f(cx + u * .2)},{f(hz + u * .15)}" stroke="#ff7a1f" stroke-width="{f(u * .02)}" fill="none" style="filter:drop-shadow(0 0 {f(u * .02)}px #ff5a1f)"/>')
    # anvil + hammer
    k = u * (.0032 if c.portrait else .0025)
    ax, ay = cx, hz + u * (.12 if c.portrait else .08)
    o.append(glow(ax, ay - 60 * k, u * .45, '#ffb347', .7))
    o.append(f'<g transform="translate({f(ax)},{f(ay)}) scale({f(k)})">'
             '<path d="M-110,-70 H90 C120,-70 150,-60 170,-40 C130,-40 110,-35 95,-25 C70,-20 50,-15 40,0 L45,40 H75 V60 H-75 V40 H-45 L-40,0 C-60,-15 -90,-25 -110,-30 Z" fill="url(#anv)" stroke="#ff9a4a" stroke-width="2"/>'
             '<rect x="-60" y="-86" width="90" height="16" rx="3" fill="#ff8a2a"/><rect x="-60" y="-86" width="90" height="6" fill="#ffd27a"/>'
             '<g transform="rotate(-38 40 -120)"><rect x="34" y="-260" width="12" height="170" rx="5" fill="#3a2418"/><rect x="0" y="-280" width="80" height="44" rx="4" fill="#2a2c33" stroke="#ff9a4a" stroke-width="2"/></g>'
             '</g>')
    # sparks
    rnd = random.Random(8)
    for _ in range(70):
        a = rnd.uniform(-math.pi * .95, -math.pi * .05)
        d = rnd.uniform(.05, .55) * u
        x, y = ax + math.cos(a) * d, ay - 80 * k + math.sin(a) * d
        l = rnd.uniform(.01, .04) * u
        o.append(f'<line x1="{f(x)}" y1="{f(y)}" x2="{f(x + math.cos(a) * l)}" y2="{f(y + math.sin(a) * l)}" stroke="{rnd.choice(["#ffd27a", "#ff9a3a", "#fff1c1"])}" stroke-width="{f(rnd.uniform(1.5, 3.5))}" stroke-linecap="round"/>')
    o.append(stars(W, H, 80, 9, 1, '#ff8a3a', 2.2, .7))
    o.append(f'<rect y="{f(ay + 60 * k)}" width="{W}" height="{H}" fill="#0b0403"/>')
    return ''.join(o)


def s_orbit(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    cy = H * (.45 if c.portrait else .5)
    o = [f'<defs>{rad("bg", [(0, "#123a4a"), (1, "#050d18")], .5, .45, .8)}{rad("pl", [(0, "#8de0a8"), (.7, "#2f9b78"), (1, "#145a52")], .35, .3, .75)}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>',
         stars(W, H, 200, 10, 1, '#d8fff0', 1.7)]
    o.append(glow(cx, cy, u * .6, '#5df2c2', .3))
    # orbits with moons
    for i, (rx, ry, rot, col) in enumerate([(.42, .14, -18, '#ffcf6b'), (.34, .1, 12, '#ff8aa8'), (.5, .2, 30, '#9db8ff')]):
        o.append(f'<ellipse cx="{f(cx)}" cy="{f(cy)}" rx="{f(rx * u)}" ry="{f(ry * u)}" transform="rotate({rot} {f(cx)} {f(cy)})" fill="none" stroke="#bff6e2" stroke-width="2" stroke-dasharray="{f(u * .01)} {f(u * .012)}" opacity=".5"/>')
        a = 1.1 + i * 2.1
        mx = cx + math.cos(a) * rx * u * math.cos(math.radians(rot)) - math.sin(a) * ry * u * math.sin(math.radians(rot))
        my = cy + math.cos(a) * rx * u * math.sin(math.radians(rot)) + math.sin(a) * ry * u * math.cos(math.radians(rot))
        o.append(f'<circle cx="{f(mx)}" cy="{f(my)}" r="{f(u * (.03 + i * .008))}" fill="{col}"/>')
    # planet with a garden on top
    r = u * .17
    o.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}" fill="url(#pl)"/>')
    o.append(f'<path d="M{f(cx - r * .9)},{f(cy + r * .1)} q{f(r * .4)},{f(-r * .3)} {f(r * .9)},{f(-r * .05)} t{f(r * .9)},{f(-r * .1)}" stroke="#1d6f5d" stroke-width="{f(r * .08)}" fill="none" opacity=".6"/>')
    rnd = random.Random(12)
    for i in range(11):
        a = -math.pi / 2 + (i - 5) * .22
        bx, by = cx + math.cos(a) * r, cy + math.sin(a) * r
        h = r * rnd.uniform(.25, .55)
        tx, ty = cx + math.cos(a) * (r + h), cy + math.sin(a) * (r + h)
        col = rnd.choice(['#ff8aa8', '#ffcf6b', '#ffffff', '#b48cff'])
        o.append(f'<line x1="{f(bx)}" y1="{f(by)}" x2="{f(tx)}" y2="{f(ty)}" stroke="#1f7d5f" stroke-width="{f(u * .006)}" stroke-linecap="round"/>')
        o.append(''.join(f'<circle cx="{f(tx + math.cos(j * 1.256) * u * .014)}" cy="{f(ty + math.sin(j * 1.256) * u * .014)}" r="{f(u * .011)}" fill="{col}"/>' for j in range(5)))
        o.append(f'<circle cx="{f(tx)}" cy="{f(ty)}" r="{f(u * .008)}" fill="#ffe27a"/>')
    # little gardener
    o.append(f'<g transform="translate({f(cx + r * .45)},{f(cy - r * .9)}) rotate(28) scale({f(u * .0016)})"><rect x="-12" y="-40" width="24" height="34" rx="12" fill="#fff"/><circle cx="0" cy="-52" r="14" fill="#fff"/><rect x="-8" y="-58" width="16" height="8" rx="4" fill="#2a7fff"/></g>')
    return ''.join(o)


def s_rustline(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#3b2a4a"), (.5, "#d26a3c"), (1, "#f7c071")])}{lin("dust", [(0, "#e8a86a", 0), (1, "#e8a86a", .9)])}{lin("ground", [(0, "#b3592b"), (1, "#5a2410")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         glow(cx - u * .3, hz, u * .7, '#ffd38a', .6)]
    # mesas
    rnd = random.Random(14)
    for i in range(4):
        mx, mw, mh = rnd.uniform(0, W), rnd.uniform(.25, .5) * u, rnd.uniform(.12, .28) * u
        o.append(f'<path d="M{f(mx - mw / 2 - mh * .4)},{f(hz)} L{f(mx - mw / 2)},{f(hz - mh)} H{f(mx + mw / 2)} L{f(mx + mw / 2 + mh * .5)},{f(hz)} Z" fill="#7a3b2e" opacity="{f(.6 + i * .1)}"/>')
    o.append(f'<rect y="{f(hz)}" width="{W}" height="{f(H - hz)}" fill="url(#ground)"/>')
    # dirt road curving in
    o.append(f'<path d="M{f(cx - u * .02)},{f(hz)} C{f(cx - u * .3)},{f(hz + (H - hz) * .3)} {f(cx + u * .6)},{f(hz + (H - hz) * .6)} {f(cx - u * .2)},{f(H + 10)} H{f(cx + u * .7)} C{f(cx + u * 1.1)},{f(hz + (H - hz) * .5)} {f(cx + u * .1)},{f(hz + (H - hz) * .3)} {f(cx + u * .02)},{f(hz)} Z" fill="#d89058" opacity=".55"/>')
    # dust cloud
    for i in range(14):
        x, y, r = cx - u * (.15 + i * .045) + rnd.uniform(-20, 20), hz + (H - hz) * .55 + rnd.uniform(-u * .08, u * .04), u * rnd.uniform(.06, .14)
        o.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(r)}" fill="#f0c08a" opacity="{f(.5 - i * .025)}"/>')
    # rally car side view, flying a little
    k = u * .0028
    px, py = cx + u * .06, hz + (H - hz) * .5
    o.append(f'<ellipse cx="{f(px)}" cy="{f(py + 52 * k)}" rx="{f(120 * k)}" ry="{f(10 * k)}" fill="#3a1408" opacity=".45"/>')
    o.append(f'<g transform="translate({f(px)},{f(py)}) rotate(-5) scale({f(k)})">'
             '<path d="M-130,10 L-120,-20 L-60,-30 L-30,-62 L50,-62 L85,-30 L130,-22 L135,12 Z" fill="#f2efe9"/>'
             '<path d="M-130,-4 H135 V8 H-130 Z" fill="#d83a1f"/><path d="M-60,-30 L-30,-62 H-10 L-35,-30 Z M50,-62 L85,-30 H60 L35,-62 Z" fill="#d83a1f"/>'
             '<path d="M-24,-56 H44 L68,-32 H-48 Z" fill="#1a2430"/>'
             '<circle cx="-5" cy="-14" r="17" fill="#fff" stroke="#1a2430" stroke-width="4"/><text x="-5" y="-6" font-family="Inter" font-weight="900" font-size="22" text-anchor="middle" fill="#1a2430">7</text>'
             '<rect x="-128" y="-34" width="16" height="10" fill="#ffe27a"/><path d="M110,-32 L135,-50 L140,-40 L120,-24 Z" fill="#1a2430"/>'
             '<circle cx="-78" cy="14" r="30" fill="#14181d"/><circle cx="-78" cy="14" r="13" fill="#9aa3ad"/><circle cx="82" cy="14" r="30" fill="#14181d"/><circle cx="82" cy="14" r="13" fill="#9aa3ad"/></g>')
    o.append(f'<rect y="{f(H - u * .15)}" width="{W}" height="{f(u * .15)}" fill="url(#dust)" opacity=".6"/>')
    # stones
    for _ in range(40):
        x, y = rnd.uniform(0, W), rnd.uniform(hz + (H - hz) * .6, H)
        o.append(f'<ellipse cx="{f(x)}" cy="{f(y)}" rx="{f(rnd.uniform(2, 7) * u / 600)}" ry="{f(rnd.uniform(1, 3) * u / 600)}" fill="#4a1d0c" opacity=".7"/>')
    return ''.join(o)


def s_echo(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    cy = H * (.44 if c.portrait else .5)
    o = [f'<defs>{rad("bg", [(0, "#0f2f45"), (1, "#03070d")], .5, .45, .8)}{lin("visor", [(0, "#7ff6ff"), (.5, "#1aa6d6"), (1, "#062a45")])}{lin("hel", [(0, "#3a4656"), (1, "#0d1219")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    # echo rings
    for i in range(12):
        o.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(u * (.2 + i * .07))}" fill="none" stroke="#4fe3ff" stroke-width="{f(2.5 - i * .15)}" opacity="{f(.5 - i * .04)}"/>')
    # HUD ticks
    t = []
    for i in range(72):
        a = i / 72 * math.pi * 2
        r1, r2 = u * .36, u * (.38 if i % 6 else .41)
        t.append(f'<line x1="{f(cx + math.cos(a) * r1)}" y1="{f(cy + math.sin(a) * r1)}" x2="{f(cx + math.cos(a) * r2)}" y2="{f(cy + math.sin(a) * r2)}"/>')
    o.append(f'<g stroke="#7ff6ff" stroke-width="2" opacity=".6">{"".join(t)}</g>')
    o.append(f'<path d="M{f(cx - u * .44)},{f(cy)} A{f(u * .44)},{f(u * .44)} 0 0 1 {f(cx)},{f(cy - u * .44)}" stroke="#ff4d6d" stroke-width="{f(u * .008)}" fill="none"/>')
    o.append(glow(cx, cy, u * .45, '#1ad0ff', .35))
    # helmet
    k = u * .0026
    o.append(f'<g transform="translate({f(cx)},{f(cy + 10 * k)}) scale({f(k)})">'
             '<path d="M-95,40 C-110,-40 -80,-115 0,-120 C80,-115 110,-40 95,40 L70,95 C40,110 -40,110 -70,95 Z" fill="url(#hel)" stroke="#5c6b7e" stroke-width="2"/>'
             '<path d="M-78,-20 C-70,-60 -40,-75 0,-75 C40,-75 70,-60 78,-20 C75,10 55,25 0,25 C-55,25 -75,10 -78,-20 Z" fill="url(#visor)"/>'
             '<path d="M-60,-40 C-40,-62 20,-66 50,-50" stroke="#e8feff" stroke-width="6" fill="none" opacity=".7" stroke-linecap="round"/>'
             '<path d="M-60,60 h120 M-50,75 h100" stroke="#5c6b7e" stroke-width="4"/>'
             '<rect x="-112" y="-10" width="18" height="50" rx="5" fill="#222b36"/><rect x="94" y="-10" width="18" height="50" rx="5" fill="#222b36"/>'
             '<circle cx="104" cy="2" r="4" fill="#ff4d6d"/></g>')
    # data lines
    rnd = random.Random(15)
    o.append(f'<g fill="#7ff6ff" opacity=".35">' + ''.join(f'<rect x="{f(rnd.uniform(0, W))}" y="{f(rnd.uniform(0, H))}" width="{f(rnd.uniform(.02, .12) * u)}" height="2"/>' for _ in range(40)) + '</g>')
    o.append(f'<g stroke="#4fe3ff" stroke-width="1" opacity=".08">' + ''.join(f'<line x1="0" y1="{y}" x2="{W}" y2="{y}"/>' for y in range(0, H, 4)) + '</g>')
    return ''.join(o)


def s_lantern(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("sky", [(0, "#0b0a24"), (.6, "#24204f"), (1, "#3b2a5c")])}{lin("lan", [(0, "#ffe9a3"), (1, "#ff8a3d")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         stars(W, H, 120, 16, .7, '#fff', 1.4)]
    o.append(peaks(W, H, hz, 17, '#1c1840', n=8, amp=u * .3))
    # keep: a tall tower on a crag
    k = u * .0028
    tx, ty = cx, hz + u * .05
    o.append(glow(tx, ty - 150 * k, u * .5, '#ffb35c', .3))
    o.append(f'<g transform="translate({f(tx)},{f(ty)}) scale({f(k)})" fill="#0c0a1c">'
             '<path d="M-160,40 L-110,-20 L-60,-10 L-40,-40 L50,-40 L80,-15 L130,-10 L170,40 Z"/>'
             '<rect x="-38" y="-260" width="76" height="230"/><path d="M-50,-260 L0,-340 L50,-260 Z"/>'
             '<rect x="-100" y="-150" width="50" height="120"/><path d="M-108,-150 L-75,-200 L-42,-150 Z"/>'
             '<rect x="45" y="-120" width="48" height="90"/><path d="M38,-120 L69,-165 L100,-120 Z"/>'
             '<g fill="#ffcf6b"><rect x="-8" y="-230" width="16" height="26" rx="8"/><rect x="-8" y="-170" width="16" height="22" rx="8"/><rect x="-82" y="-120" width="12" height="18" rx="6"/><rect x="62" y="-95" width="12" height="18" rx="6"/></g>'
             '</g>')
    # floating lanterns
    rnd = random.Random(18)
    for i in range(26):
        x, y, s = rnd.uniform(0, W), rnd.uniform(H * .05, hz + u * .1), rnd.uniform(.012, .035) * u
        o.append(glow(x, y, s * 4, '#ffb35c', .45))
        o.append(f'<path d="M{f(x - s * .6)},{f(y - s)} h{f(s * 1.2)} l{f(s * .2)},{f(s * 1.6)} q{f(-s * .8)},{f(s * .4)} {f(-s * 1.6)},0 Z" fill="url(#lan)"/>')
    o.append(ridge(W, H, H - u * .12, u * .03, 19, '#06050f'))
    # keeper with a lantern
    o.append(f'<g transform="translate({f(cx - u * .25)},{f(H - u * .12)}) scale({f(u * .0022)})" fill="#06050f"><path d="M-20,0 L-14,-70 C-14,-95 14,-95 14,-70 L20,0 Z"/><circle cx="0" cy="-100" r="16"/><path d="M14,-60 L40,-80" stroke="#06050f" stroke-width="6"/></g>')
    o.append(glow(cx - u * .25 + 42 * u * .0022, H - u * .12 - 80 * u * .0022, u * .08, '#ffcf6b', 1))
    return ''.join(o)


def s_moss(c):
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    o = [f'<defs>{lin("bg", [(0, "#16301f"), (.6, "#3d6b3a"), (1, "#a3c46b")])}{lin("cap", [(0, "#d9544a"), (1, "#8e2a2a")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    # light shafts
    for i in range(5):
        x = cx - u * .6 + i * u * .3
        o.append(f'<path d="M{f(x)},0 L{f(x + u * .12)},0 L{f(x + u * .5)},{H} L{f(x + u * .2)},{H} Z" fill="#fff6c8" opacity=".07"/>')
    # tree trunks
    rnd = random.Random(20)
    for i in range(7):
        x, w = rnd.uniform(0, W), rnd.uniform(.05, .14) * u
        o.append(f'<rect x="{f(x)}" y="0" width="{f(w)}" height="{H}" fill="#1d3a26" opacity="{f(rnd.uniform(.5, .9))}"/>')
    o.append(ridge(W, H, hz + u * .04, u * .05, 21, '#2f5a2e', octaves=3))
    # giant mushrooms
    def mush(x, base, s, col='url(#cap)'):
        return (f'<g transform="translate({f(x)},{f(base)}) scale({f(s)})">'
                '<path d="M-14,0 C-10,-60 -18,-120 -8,-160 H8 C18,-120 10,-60 14,0 Z" fill="#efe3c8"/>'
                f'<path d="M-90,-150 C-80,-215 80,-215 90,-150 C60,-140 -60,-140 -90,-150 Z" fill="{col}"/>'
                '<g fill="#fff3e0"><circle cx="-40" cy="-175" r="9"/><circle cx="10" cy="-190" r="7"/><circle cx="45" cy="-168" r="10"/><circle cx="-10" cy="-162" r="5"/></g></g>')
    o.append(mush(cx + u * .25, hz + u * .1, u * .0035))
    o.append(mush(cx - u * .3, hz + u * .14, u * .0022))
    o.append(mush(cx + u * .02, hz + u * .16, u * .0012, '#e0a04a'))
    o.append(ridge(W, H, hz + u * .18, u * .03, 22, '#1d3f22', octaves=3))
    # wanderer with a big hat
    o.append(f'<g transform="translate({f(cx - u * .05)},{f(hz + u * .18)}) scale({f(u * .0018)})"><path d="M-22,0 L-16,-60 H16 L22,0 Z" fill="#c6a46a"/><circle cx="0" cy="-74" r="15" fill="#f1dcc0"/><path d="M-40,-80 L0,-130 L40,-80 Z" fill="#6a4a8a"/><path d="M16,-40 L40,-10" stroke="#5a3d22" stroke-width="6"/></g>')
    # spores
    o.append(stars(W, H, 60, 23, 1, '#fff6c8', 2.4, .6))
    # foreground moss + bones
    o.append(ridge(W, H, H - u * .05, u * .03, 24, '#0f2414', octaves=5, rough=.6))
    o.append(f'<g fill="#efe6d2" transform="translate({f(cx + u * .3)},{f(H - u * .06)}) scale({f(u * .0016)})"><rect x="-40" y="-6" width="80" height="12" rx="6"/><circle cx="-42" cy="-6" r="9"/><circle cx="-42" cy="6" r="9"/><circle cx="42" cy="-6" r="9"/><circle cx="42" cy="6" r="9"/></g>')
    return ''.join(o)


def s_skyline(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    o = [f'<defs>{lin("sky", [(0, "#5fb4ff"), (1, "#d7f0ff")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    rnd = random.Random(25)
    for i in range(6):
        x, y, s = rnd.uniform(0, W), rnd.uniform(H * .05, H * .35), rnd.uniform(.08, .16) * u
        o.append(f'<g fill="#fff" opacity=".9"><ellipse cx="{f(x)}" cy="{f(y)}" rx="{f(s)}" ry="{f(s * .35)}"/><circle cx="{f(x - s * .3)}" cy="{f(y - s * .2)}" r="{f(s * .35)}"/><circle cx="{f(x + s * .2)}" cy="{f(y - s * .3)}" r="{f(s * .45)}"/></g>')
    # isometric blocks
    def block(x, y, w, d, h, top, left, right, win=True):
        s = (f'<path d="M{f(x)},{f(y)} l{f(w)},{f(-w * .5)} l{f(d)},{f(d * .5)} l{f(-w)},{f(w * .5)} Z" fill="{top}"/>'
             f'<path d="M{f(x)},{f(y)} l{f(d)},{f(d * .5)} v{f(h)} l{f(-d)},{f(-d * .5)} Z" fill="{left}"/>'
             f'<path d="M{f(x + d)},{f(y + d * .5)} l{f(w)},{f(-w * .5)} v{f(h)} l{f(-w)},{f(w * .5)} Z" fill="{right}"/>')
        if win:
            for j in range(1, int(h / (u * .04))):
                yy = y + d * .5 + j * u * .04
                s += f'<path d="M{f(x + d + w * .12)},{f(yy - w * .06)} l{f(w * .76)},{f(-w * .38)}" stroke="#bfe4ff" stroke-width="{f(u * .012)}" opacity=".8"/>'
        return s
    base = H * (.95 if c.portrait else 1.02)
    if not c.portrait: u = u * .8
    pal = [('#ffd27a', '#e3a54a', '#f6c062'), ('#9fd6ff', '#5a8fd0', '#78aee6'), ('#ff9a8a', '#d7645a', '#ec7f72'), ('#c8f08a', '#7cb34a', '#9fd06a'), ('#e9e4ff', '#9b92d6', '#bab2ee')]
    o.append(f'<path d="M{f(cx - u * 1.6)},{f(base)} L{f(cx)},{f(base - u * .8)} L{f(cx + u * 1.6)},{f(base)} L{f(cx)},{f(base + u * .8)} Z" fill="#7cc46a"/>')
    items = []
    for gx in range(-5, 6):
        for gy in range(-5, 6):
            x = cx + (gx - gy) * u * .09
            y = base - u * .1 + (gx + gy) * u * .045 - u * .2
            if abs(gx) + abs(gy) > 7: continue
            dist = math.hypot(gx, gy)
            h = u * max(.05, .55 - dist * .07) * rnd.uniform(.6, 1.2)
            items.append((y, x, h, rnd.choice(pal)))
    for y, x, h, p in sorted(items):
        o.append(block(x - u * .07, y - h, u * .07, u * .07, h, *p))
    # crane
    k = u * .002
    o.append(f'<g transform="translate({f(cx + u * .05)},{f(base - u * .95)}) scale({f(k)})" stroke="#ff8a3d" stroke-width="8" fill="none"><path d="M0,0 V300 M-140,0 H160 M0,0 L-140,0 M0,-30 L-140,0 M0,-30 L160,0 M120,0 V80"/><rect x="105" y="80" width="30" height="24" fill="#ff8a3d"/></g>')
    return ''.join(o)


def s_velvet(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    cy = H * (.46 if c.portrait else .5)
    o = [f'<defs>{lin("cur", [(0, "#5a0a1e"), (.5, "#a3162f"), (1, "#5a0a1e")], 0, 0, 1, 0)}'
         f'<pattern id="fold" width="{f(u * .08)}" height="10" patternUnits="userSpaceOnUse"><rect width="{f(u * .08)}" height="10" fill="url(#cur)"/></pattern>'
         f'<filter id="static"><feTurbulence type="fractalNoise" baseFrequency="1.6" numOctaves="1" seed="4"/><feColorMatrix type="saturate" values="0"/></filter></defs>',
         f'<rect width="{W}" height="{H}" fill="#16030a"/>',
         f'<rect width="{W}" height="{H}" fill="url(#fold)" opacity=".85"/>']
    o.append(glow(cx, cy, u * .7, '#ffcfa3', .3))
    # spotlight floor
    o.append(f'<ellipse cx="{f(cx)}" cy="{f(cy + u * .35)}" rx="{f(u * .5)}" ry="{f(u * .08)}" fill="#ffd9b0" opacity=".25"/>')
    # retro TV
    k = u * .0032
    o.append(f'<g transform="translate({f(cx)},{f(cy)}) scale({f(k)})">'
             '<path d="M-40,-110 L-80,-170 M40,-110 L80,-170" stroke="#1a0a0f" stroke-width="5"/>'
             '<rect x="-120" y="-110" width="240" height="190" rx="28" fill="#2a1418" stroke="#d4a373" stroke-width="3"/>'
             '<rect x="-100" y="-92" width="160" height="150" rx="22" fill="#000"/>'
             '<circle cx="90" cy="-50" r="12" fill="#d4a373"/><circle cx="90" cy="-12" r="12" fill="#d4a373"/>'
             '<path d="M-90,80 L-100,110 M90,80 L100,110" stroke="#1a0a0f" stroke-width="8"/></g>')
    o.append(f'<g><clipPath id="scr"><rect x="{f(cx - 100 * k)}" y="{f(cy - 92 * k)}" width="{f(160 * k)}" height="{f(150 * k)}" rx="{f(22 * k)}"/></clipPath>'
             f'<g clip-path="url(#scr)"><rect x="{f(cx - 100 * k)}" y="{f(cy - 92 * k)}" width="{f(160 * k)}" height="{f(150 * k)}" filter="url(#static)" opacity=".85"/>'
             f'<rect x="{f(cx - 100 * k)}" y="{f(cy - 92 * k)}" width="{f(160 * k)}" height="{f(150 * k)}" fill="#7fd6ff" opacity=".18"/>'
             f'<path d="M{f(cx - 60 * k)},{f(cy + 30 * k)} C{f(cx - 40 * k)},{f(cy - 40 * k)} {f(cx + 0 * k)},{f(cy - 40 * k)} {f(cx + 20 * k)},{f(cy + 30 * k)}" fill="#000" opacity=".7"/><circle cx="{f(cx - 20 * k)}" cy="{f(cy - 30 * k)}" r="{f(18 * k)}" fill="#000" opacity=".7"/></g></g>')
    # curtains framing
    for side in (-1, 1):
        x0 = 0 if side < 0 else W
        o.append(f'<path d="M{f(x0)},0 H{f(x0 - side * -1 * W * .28)} C{f(x0 + side * -1 * W * .2)},{f(H * .4)} {f(x0 + side * -1 * W * .08)},{f(H * .6)} {f(x0 + side * -1 * W * .14)},{H} H{f(x0)} Z" fill="url(#fold)" style="filter:drop-shadow({-side * 10}px 0 20px #000)"/>')
    o.append(f'<path d="M0,0 H{W} V{f(u * .08)} ' + ' '.join(f'q{f(u * .05)},{f(u * .06)} {f(u * .1)},0' for _ in range(int(W / (u * .1)) + 1)) + '" fill="#7a0f24" transform="scale(-1,1) translate(-' + str(W) + ',0)"/>')
    return ''.join(o)


def s_frostbyte(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    cy = H * (.5 if c.portrait else .52)
    o = [f'<defs>{lin("bg", [(0, "#04122a"), (.6, "#0b3a6b"), (1, "#56c8ff")])}{lin("ice", [(0, "#ffffff"), (.4, "#bff0ff"), (1, "#3aa8ff")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    # arena lights
    for i in range(7):
        x = W * (i + .5) / 7
        o.append(f'<path d="M{f(x)},0 L{f(x - u * .15)},{H} L{f(x + u * .15)},{H} Z" fill="#bff0ff" opacity=".05"/>')
    o.append(glow(cx, cy, u * .7, '#7fe0ff', .45))
    # crystal shards
    rnd = random.Random(27)
    for i in range(22):
        a = rnd.uniform(0, math.pi * 2)
        d = rnd.uniform(.25, .9) * u
        x, y = cx + math.cos(a) * d, cy + math.sin(a) * d * .8
        s = rnd.uniform(.03, .09) * u
        rot = math.degrees(a) + 90
        o.append(f'<path d="M0,{f(-s * 1.8)} L{f(s * .5)},0 L0,{f(s * .8)} L{f(-s * .5)},0 Z" fill="url(#ice)" opacity="{f(rnd.uniform(.5, .95))}" transform="translate({f(x)},{f(y)}) rotate({f(rot)})"/>')
    # big diagonal slashes
    o.append(f'<path d="M{f(cx - u * .7)},{f(cy + u * .35)} L{f(cx + u * .7)},{f(cy - u * .45)} L{f(cx + u * .72)},{f(cy - u * .4)} L{f(cx - u * .66)},{f(cy + u * .4)} Z" fill="#fff" opacity=".9"/>')
    o.append(f'<path d="M{f(cx - u * .6)},{f(cy + u * .5)} L{f(cx + u * .8)},{f(cy - u * .25)} L{f(cx + u * .8)},{f(cy - u * .22)} L{f(cx - u * .58)},{f(cy + u * .53)} Z" fill="#7fe0ff" opacity=".8"/>')
    # fighter silhouette mid-punch
    k = u * .0028
    o.append(f'<g transform="translate({f(cx)},{f(cy + u * .05)}) scale({f(k)})" fill="#03101f" stroke="#bff0ff" stroke-width="3" stroke-linejoin="round">'
             '<circle cx="-10" cy="-120" r="24"/>'
             '<path d="M-40,-92 C-10,-100 20,-98 30,-85 L120,-110 L128,-92 L40,-60 L30,-10 L70,60 L90,120 L68,126 L40,70 L0,30 L-30,80 L-60,128 L-80,118 L-58,60 L-46,0 L-60,-50 L-110,-20 L-122,-34 L-70,-80 Z"/>'
             '<circle cx="130" cy="-102" r="16" fill="#bff0ff" stroke="none"/></g>')
    o.append(glow(cx + 130 * k, cy + u * .05 - 102 * k, u * .14, '#ffffff', .9))
    o.append(f'<path d="M0,{f(H - u * .12)} L{W},{f(H - u * .2)} V{H} H0 Z" fill="#cdefff" opacity=".9"/>')
    o.append(f'<path d="M0,{f(H - u * .09)} L{W},{f(H - u * .16)} V{H} H0 Z" fill="#8fd2f5"/>')
    return ''.join(o)


def s_deepwell(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    o = [f'<defs>{lin("bg", [(0, "#0c3b3a"), (.4, "#06191f"), (1, "#020507")])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    # light from far above
    o.append(f'<path d="M{f(cx - u * .12)},0 H{f(cx + u * .12)} L{f(cx + u * .45)},{H} H{f(cx - u * .45)} Z" fill="#9ffff0" opacity=".08"/>')
    o.append(glow(cx, 0, u * .5, '#bfffee', .5))
    # cavern walls with ledges
    rnd = random.Random(28)
    for side in (-1, 1):
        pts = []
        y = -10
        while y < H + 20:
            x = cx + side * (u * .42 + rnd.uniform(-u * .06, u * .1) + (u * .25 if not c.portrait else 0))
            pts.append(f'{f(x)},{f(y)}')
            y += u * .06
        edge = W + 10 if side > 0 else -10
        o.append(f'<polygon points="{edge},-10 {" ".join(pts)} {edge},{H + 10}" fill="#04100f"/>')
    # ledges/ruins
    for i in range(6):
        y = H * (.18 + i * .14)
        side = -1 if i % 2 else 1
        x = cx + side * u * .3
        o.append(f'<rect x="{f(x - u * .12)}" y="{f(y)}" width="{f(u * .24)}" height="{f(u * .025)}" fill="#0b2624"/>')
        o.append(f'<rect x="{f(x - u * .1)}" y="{f(y + u * .025)}" width="{f(u * .02)}" height="{f(u * .08)}" fill="#0b2624"/><rect x="{f(x + u * .08)}" y="{f(y + u * .025)}" width="{f(u * .02)}" height="{f(u * .08)}" fill="#0b2624"/>')
    # bioluminescent plants
    for _ in range(50):
        side = rnd.choice((-1, 1))
        x = cx + side * rnd.uniform(.3, .55) * u * (1.6 if not c.portrait else 1)
        y = rnd.uniform(0, H)
        col = rnd.choice(['#3dffd0', '#7a8cff', '#c46bff'])
        o.append(glow(x, y, u * .03, col, .7))
        o.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(u * .005)}" fill="#fff"/>')
    # explorer descending on a rope with a lamp
    py = H * .55
    o.append(f'<line x1="{f(cx)}" y1="0" x2="{f(cx)}" y2="{f(py - u * .04)}" stroke="#bfe" stroke-width="1.5" opacity=".6"/>')
    o.append(glow(cx + u * .02, py + u * .01, u * .2, '#ffd27a', .7))
    o.append(f'<g transform="translate({f(cx)},{f(py)}) scale({f(u * .0016)})" fill="#020507"><circle cx="0" cy="-30" r="12"/><path d="M-12,-16 H12 L16,30 L6,60 H-6 L-16,30 Z"/><path d="M12,-6 L34,8" stroke="#020507" stroke-width="6"/></g>')
    o.append(f'<circle cx="{f(cx + u * .055)}" cy="{f(py + u * .015)}" r="{f(u * .01)}" fill="#ffe7a8"/>')
    # depth fog bottom
    o.append(bottom_fade(W, H, .35, '#000', .9))
    return ''.join(o)


def s_comet(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    cy = H * (.44 if c.portrait else .5)
    o = [f'<defs>{rad("bg", [(0, "#ff6fae"), (.5, "#a03bd6"), (1, "#2a1060")], .5, .45, .8)}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    # sunburst
    o.append(f'<g opacity=".12" fill="#fff">' + ''.join(f'<path d="M{f(cx)},{f(cy)} L{f(cx + math.cos(a) * u * 2)},{f(cy + math.sin(a) * u * 2)} L{f(cx + math.cos(a + .14) * u * 2)},{f(cy + math.sin(a + .14) * u * 2)} Z"/>' for a in [i * math.pi / 12 for i in range(24)]) + '</g>')
    o.append(stars(W, H, 60, 29, 1, '#fff', 2.5))
    # comet trail
    o.append(f'<path d="M{f(cx - u * .9)},{f(cy - u * .55)} Q{f(cx - u * .3)},{f(cy - u * .2)} {f(cx + u * .02)},{f(cy - u * .02)} L{f(cx - u * .08)},{f(cy + u * .12)} Q{f(cx - u * .4)},{f(cy - u * .1)} {f(cx - u * .95)},{f(cy - u * .4)} Z" fill="#ffe14d" opacity=".85"/>')
    o.append(f'<path d="M{f(cx - u * .8)},{f(cy - u * .48)} Q{f(cx - u * .3)},{f(cy - u * .15)} {f(cx)},{f(cy + u * .02)}" stroke="#fff" stroke-width="{f(u * .02)}" fill="none" opacity=".8"/>')
    # frying pan with a comet egg
    k = u * .003
    o.append(f'<g transform="translate({f(cx + u * .06)},{f(cy + u * .06)}) rotate(-14) scale({f(k)})">'
             '<rect x="70" y="-12" width="150" height="24" rx="12" fill="#3a2a5a"/>'
             '<circle cx="0" cy="0" r="90" fill="#2a1f40"/><circle cx="0" cy="0" r="76" fill="#3d2f5c"/>'
             '<path d="M-50,-20 C-60,-60 0,-70 30,-50 C70,-40 60,10 40,30 C20,60 -40,50 -50,-20 Z" fill="#fff"/>'
             '<circle cx="0" cy="-8" r="24" fill="#ffb21f"/><circle cx="-7" cy="-15" r="7" fill="#ffe08a"/></g>')
    # confetti and veggies
    rnd = random.Random(30)
    for _ in range(40):
        x, y = rnd.uniform(0, W), rnd.uniform(0, H)
        col = rnd.choice(['#ffe14d', '#4dffd2', '#ffffff', '#ff4d8a', '#7cff6b'])
        o.append(f'<rect x="{f(x)}" y="{f(y)}" width="{f(u * .018)}" height="{f(u * .008)}" fill="{col}" transform="rotate({rnd.randint(0, 180)} {f(x)} {f(y)})"/>')
    for i, (dx, dy, col) in enumerate([(-.35, .3, '#ff4d4d'), (.38, -.3, '#7cff6b'), (.32, .32, '#ff9a3d')]):
        x, y = cx + dx * u, cy + dy * u
        o.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(u * .05)}" fill="{col}" stroke="#2a1060" stroke-width="{f(u * .006)}"/><path d="M{f(x)},{f(y - u * .05)} l{f(u * .02)},{f(-u * .03)}" stroke="#2a8a2a" stroke-width="{f(u * .01)}" stroke-linecap="round"/>')
    o.append(f'<path d="M0,{f(H - u * .1)} ' + ' '.join(f'l{f(u * .05)},{f(u * .05)} l{f(u * .05)},{f(-u * .05)}' for _ in range(int(W / (u * .1)) + 2)) + f' V{H} H0 Z" fill="#ffe14d"/>')
    o.append(f'<path d="M0,{f(H - u * .06)} ' + ' '.join(f'l{f(u * .05)},{f(u * .05)} l{f(u * .05)},{f(-u * .05)}' for _ in range(int(W / (u * .1)) + 2)) + f' V{H} H0 Z" fill="#2a1060"/>')
    return ''.join(o)


def s_atlas(c):
    W, H, u, cx = c.W, c.H, c.u, c.cx
    o = [f'<defs>{lin("bg", [(0, "#020814"), (.5, "#0b2a3a"), (1, "#14544f")])}{lin("water", [(0, "#1fa39a", .6), (1, "#03141a", 1)])}{rad("orb", [(0, "#fff8d8"), (.4, "#ffd27a"), (1, "#c46b1f")], .4, .35, .7)}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']
    wl = H * (.78 if c.portrait else .8)
    # cavern ceiling stalactites
    rnd = random.Random(31)
    for _ in range(int(W / (u * .05))):
        x, h = rnd.uniform(0, W), rnd.uniform(.05, .3) * u
        o.append(f'<path d="M{f(x - h * .18)},0 L{f(x)},{f(h)} L{f(x + h * .18)},0 Z" fill="#01040a"/>')
    # colossal statue holding a glowing sphere
    k = u * (.0034 if c.portrait else .0026)
    sx, sy = cx, wl
    o.append(glow(sx, sy - 205 * k, u * .55, '#ffcf7a', .5))
    o.append(f'<g transform="translate({f(sx)},{f(sy)}) scale({f(k)})" fill="#11303a" stroke="#3d8a8a" stroke-width="1.5">'
             '<path d="M-60,0 L-50,-80 L-62,-120 L-48,-140 C-60,-150 -66,-170 -60,-180 L-95,-200 L-110,-230 L-92,-236 L-70,-210 L-35,-196 L-20,-205 C-25,-220 -18,-232 0,-232 C18,-232 25,-220 20,-205 L35,-196 L70,-210 L92,-236 L110,-230 L95,-200 L60,-180 C66,-170 60,-150 48,-140 L62,-120 L50,-80 L60,0 Z"/>'
             '<path d="M-30,-160 C-10,-150 10,-150 30,-160" stroke="#163847" stroke-width="3" fill="none"/></g>')
    o.append(f'<circle cx="{f(sx)}" cy="{f(sy - 258 * k)}" r="{f(30 * k)}" fill="url(#orb)"/>')
    o.append(f'<g stroke="#fff3c8" stroke-width="1" fill="none" opacity=".6"><ellipse cx="{f(sx)}" cy="{f(sy - 258 * k)}" rx="{f(30 * k)}" ry="{f(10 * k)}"/><ellipse cx="{f(sx)}" cy="{f(sy - 258 * k)}" rx="{f(12 * k)}" ry="{f(30 * k)}"/></g>')
    # pillars
    for side in (-1, 1):
        for j in range(3):
            x = cx + side * u * (.45 + j * .28) * (1 if c.portrait else 1.3)
            o.append(f'<rect x="{f(x - u * .03)}" y="{f(wl - u * (.5 - j * .1))}" width="{f(u * .06)}" height="{f(u * (.5 - j * .1))}" fill="#06141c"/><rect x="{f(x - u * .045)}" y="{f(wl - u * (.5 - j * .1))}" width="{f(u * .09)}" height="{f(u * .02)}" fill="#06141c"/>')
    # water + reflection
    o.append(f'<rect y="{f(wl)}" width="{W}" height="{f(H - wl)}" fill="url(#water)"/>')
    o.append(f'<rect x="{f(sx - u * .04)}" y="{f(wl)}" width="{f(u * .08)}" height="{f(H - wl)}" fill="#ffd27a" opacity=".2"/>')
    for i in range(10):
        y = wl + (H - wl) * (i / 10) ** 1.5
        o.append(f'<line x1="{f(sx - u * (.1 + i * .03))}" y1="{f(y)}" x2="{f(sx + u * (.1 + i * .03))}" y2="{f(y)}" stroke="#ffe7a8" stroke-width="1.5" opacity="{f(.4 - i * .03)}"/>')
    # small boat with explorer and lamp
    bx, by = cx - u * .22, wl + (H - wl) * .45
    o.append(f'<path d="M{f(bx - u * .07)},{f(by)} h{f(u * .14)} l{f(-u * .02)},{f(u * .025)} h{f(-u * .1)} Z" fill="#01060a"/><path d="M{f(bx)},{f(by)} v{f(-u * .04)}" stroke="#01060a" stroke-width="{f(u * .012)}" stroke-linecap="round"/><circle cx="{f(bx)}" cy="{f(by - u * .05)}" r="{f(u * .009)}" fill="#01060a"/>')
    o.append(glow(bx + u * .05, by - u * .04, u * .07, '#ffe08a', 1))
    o.append(stars(W, H, 50, 32, .7, '#7ff6e6', 1.6, .6))
    return ''.join(o)


def s_fantasy(c, seed=0, hue='#c8873a'):
    """Generic misty-mountain landscape for Beacon's games (no titles, no emblems: those games' real art isn't ours)."""
    W, H, u, cx, hz = c.W, c.H, c.u, c.cx, c.hz
    tones = {0: ('#2b1a12', '#a8642c', '#f2c27a'), 1: ('#14161f', '#5a4a6a', '#d8a06a'), 2: ('#1b1410', '#7a3a1c', '#f0a24a'),
             3: ('#0f1418', '#3a5a5a', '#c8d6b0'), 4: ('#1a1210', '#6a4a2a', '#e8c890'), 5: ('#0a1020', '#2a3a6a', '#9ab8ff')}[seed % 6]
    o = [f'<defs>{lin("sky", [(0, tones[0]), (.65, tones[1]), (1, tones[2])])}</defs>',
         f'<rect width="{W}" height="{H}" fill="url(#sky)"/>',
         glow(cx, hz - u * .15, u * .8, tones[2], .45)]
    o.append(peaks(W, H, hz - u * .05, 50 + seed, tones[0], n=8, amp=u * .45, hmin=.4, op=.55))
    o.append(peaks(W, H, hz + u * .05, 60 + seed, tones[0], n=9, amp=u * .28, op=.8))
    o.append(ridge(W, H, hz + u * .16, u * .04, 70 + seed, '#0a0706'))
    o.append(birds(cx + u * .1, hz - u * .35, 4, u * .02, '#1a1210', seed))
    return ''.join(o)


# ---------------------------------------------------------------------------------------------------------------
# title lettering (logos): drawn in a 1000 x 400 box centred on (500, 200)
# ---------------------------------------------------------------------------------------------------------------
def L(defs, body): return defs, body

def l_starfall():
    return L(lin('lsf', [(0, '#ffffff'), (.45, '#ffe6f6'), (.5, '#ff6fc3'), (1, '#ffd166')]) + lin('lsf2', [(0, '#4ff0ff'), (1, '#7c5cff')]),
             '<g transform="skewX(-12) translate(42,0)" text-anchor="middle" style="filter:drop-shadow(0 6px 0 #2a0b45) drop-shadow(0 0 22px rgba(255,79,163,.6))">'
             '<text x="500" y="205" font-family="Poppins" font-weight="700" font-style="italic" font-size="150" letter-spacing="-4" fill="url(#lsf)" stroke="#2a0b45" stroke-width="4" paint-order="stroke">STARFALL</text>'
             '<text x="500" y="300" font-family="Poppins" font-weight="700" font-style="italic" font-size="84" letter-spacing="34" fill="url(#lsf2)">DRIFT</text></g>')

def l_hollow():
    return L('', '<g text-anchor="middle" fill="#e8eadf" style="filter:drop-shadow(0 4px 18px rgba(0,0,0,.8))">'
             '<text x="500" y="190" font-family="GFS Baskerville" font-size="132" letter-spacing="26">HOLLOW</text>'
             '<line x1="250" y1="225" x2="750" y2="225" stroke="#b33a2e" stroke-width="3"/>'
             '<text x="500" y="300" font-family="GFS Baskerville" font-size="74" letter-spacing="40" fill="#c9cfbf">PINES</text></g>')

def l_ironvale():
    return L(lin('liv', [(0, '#fff2c4'), (.5, '#e2b354'), (.52, '#a8702a'), (1, '#f6d27a')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 5px 0 #1a1208) drop-shadow(0 8px 20px rgba(0,0,0,.7))">'
             '<text x="500" y="205" font-family="TeX Gyre Bonum" font-weight="700" font-size="140" letter-spacing="8" fill="url(#liv)" stroke="#3a2610" stroke-width="3" paint-order="stroke">IRONVALE</text>'
             '<path d="M300,250 H440 M560,250 H700" stroke="#e2b354" stroke-width="3"/><path d="M480,232 l20,18 l-20,18 l-20,-18 Z M520,232 l20,18 l-20,18 l-20,-18 Z" fill="#e2b354"/>'
             '<text x="500" y="320" font-family="TeX Gyre Bonum" font-weight="700" font-size="58" letter-spacing="30" fill="#f0e2c0">TACTICS</text></g>')

def l_neon():
    return L('', '<g text-anchor="middle" font-family="Inter" font-weight="900" font-style="italic">'
             '<text x="500" y="200" font-size="150" letter-spacing="-6" fill="none" stroke="#ff2fd0" stroke-width="6" style="filter:drop-shadow(0 0 14px #ff2fd0)">NEON</text>'
             '<text x="500" y="200" font-size="150" letter-spacing="-6" fill="#ffe6fa" opacity=".95">NEON</text>'
             '<text x="500" y="300" font-size="92" letter-spacing="4" fill="#22e6ff" style="filter:drop-shadow(0 0 12px #22e6ff)">COURIER</text></g>')

def l_tidebound():
    return L(lin('ltb', [(0, '#ffffff'), (1, '#ffe0b0')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 4px 14px rgba(10,20,40,.7))">'
             '<text x="500" y="250" font-family="Lora" font-style="italic" font-weight="700" font-size="170" fill="url(#ltb)">Tidebound</text>'
             '<path d="M240,290 q30,-18 60,0 t60,0 t60,0 t60,0 t60,0 t60,0 t60,0 t60,0" stroke="#ffe0b0" stroke-width="5" fill="none"/></g>')

def l_papermoon():
    return L('', '<g text-anchor="middle" style="filter:drop-shadow(4px 5px 0 rgba(59,47,94,.6))">'
             '<text x="500" y="200" font-family="TeX Gyre Chorus" font-size="190" fill="#fff4dc">Paper Moon</text>'
             '<rect x="380" y="235" width="240" height="56" rx="28" fill="#c9576b"/>'
             '<text x="500" y="276" font-family="Poppins" font-weight="700" font-size="38" letter-spacing="16" fill="#fff4dc">CAFE</text></g>')

def l_grimforge():
    return L(lin('lgf', [(0, '#fff1c1'), (.4, '#ffb347'), (.6, '#d2451a'), (1, '#5a1206')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 6px 0 #1a0503) drop-shadow(0 0 30px rgba(255,90,31,.55))">'
             '<text x="500" y="250" font-family="TeX Gyre Pagella" font-weight="700" font-size="168" letter-spacing="6" fill="url(#lgf)" stroke="#1a0503" stroke-width="5" paint-order="stroke">GRIMFORGE</text></g>')

def l_orbit():
    return L('', '<g text-anchor="middle" style="filter:drop-shadow(0 4px 12px rgba(0,0,0,.6))">'
             '<text x="500" y="200" font-family="TeX Gyre Adventor" font-weight="700" font-size="140" letter-spacing="-2" fill="#e9fff4">orbit</text>'
             '<ellipse cx="500" cy="160" rx="250" ry="48" fill="none" stroke="#ffcf6b" stroke-width="6" transform="rotate(-8 500 160)"/>'
             '<text x="500" y="300" font-family="TeX Gyre Adventor" font-size="80" letter-spacing="18" fill="#8de0a8">garden</text></g>')

def l_rustline():
    return L(lin('lrr', [(0, '#ffd27a'), (1, '#d83a1f')]),
             '<g transform="skewX(-14) translate(50,0)" text-anchor="middle" style="filter:drop-shadow(5px 5px 0 #2a0e05)">'
             '<text x="500" y="200" font-family="Inter Display" font-weight="900" font-size="150" letter-spacing="-5" fill="#fff5e8">RUSTLINE</text>'
             '<rect x="250" y="226" width="500" height="78" fill="url(#lrr)"/>'
             '<text x="500" y="290" font-family="Inter Display" font-weight="900" font-size="72" letter-spacing="24" fill="#1a0a05">RALLY</text></g>')

def l_echo():
    return L('', '<g text-anchor="middle" style="filter:drop-shadow(0 0 16px rgba(79,227,255,.6))">'
             '<text x="500" y="200" font-family="Inter Display" font-weight="800" font-size="150" letter-spacing="30" fill="#e8feff">ECHO</text>'
             '<text x="500" y="290" font-family="DejaVu Sans Mono" font-weight="700" font-size="56" letter-spacing="22" fill="#4fe3ff">PROTOCOL</text>'
             '<path d="M200,230 h80 M720,230 h80" stroke="#ff4d6d" stroke-width="5"/></g>')

def l_lantern():
    return L(lin('llk', [(0, '#fff3c8'), (1, '#ffb35c')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 0 24px rgba(255,179,92,.55)) drop-shadow(0 4px 0 #1c1840)">'
             '<text x="500" y="190" font-family="Lora" font-weight="700" font-size="128" letter-spacing="12" fill="url(#llk)">LANTERN</text>'
             '<text x="500" y="300" font-family="Lora" font-weight="700" font-size="100" letter-spacing="40" fill="#ffe1a0">KEEP</text></g>')

def l_moss():
    return L('', '<g text-anchor="middle" style="filter:drop-shadow(0 4px 10px rgba(0,0,0,.6))">'
             '<text x="500" y="170" font-family="Lora" font-style="italic" font-weight="700" font-size="150" fill="#f2f0d8">Moss</text>'
             '<text x="500" y="320" font-family="Lora" font-style="italic" font-weight="700" font-size="150" fill="#f2f0d8"><tspan fill="#d9544a" font-family="TeX Gyre Chorus" font-style="normal">&amp; </tspan>Marrow</text></g>')

def l_skyline():
    return L('', '<g text-anchor="middle">'
             '<rect x="190" y="90" width="620" height="140" rx="22" fill="#1f3d7a"/>'
             '<text x="500" y="200" font-family="Poppins" font-weight="700" font-size="118" letter-spacing="-2" fill="#fff">SKYLINE</text>'
             '<rect x="300" y="240" width="400" height="72" rx="18" fill="#ff8a3d"/>'
             '<text x="500" y="295" font-family="Poppins" font-weight="700" font-size="50" letter-spacing="14" fill="#fff">BUILDERS</text></g>')

def l_velvet():
    return L(lin('lvs', [(0, '#ffe6c8'), (1, '#d4a373')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 4px 14px rgba(0,0,0,.8))">'
             '<text x="500" y="200" font-family="TeX Gyre Chorus" font-size="190" fill="url(#lvs)">Velvet</text>'
             '<text x="500" y="290" font-family="DejaVu Sans Mono" font-size="64" letter-spacing="34" fill="#e8f6ff" opacity=".9">STATIC</text></g>')

def l_frostbyte():
    return L(lin('lfb', [(0, '#ffffff'), (.5, '#bff0ff'), (1, '#3aa8ff')]),
             '<g transform="skewX(-10) translate(36,0)" text-anchor="middle" style="filter:drop-shadow(0 6px 0 #03101f) drop-shadow(0 0 20px rgba(127,224,255,.6))">'
             '<text x="500" y="200" font-family="Inter" font-weight="900" font-size="146" letter-spacing="-4" fill="url(#lfb)" stroke="#03101f" stroke-width="4" paint-order="stroke">FROSTBYTE</text>'
             '<text x="500" y="290" font-family="Inter" font-weight="800" font-size="62" letter-spacing="40" fill="#e8fbff">ARENA</text></g>')

def l_deepwell():
    return L(lin('ldw', [(0, '#e8fff8'), (1, '#3dffd0')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 0 20px rgba(61,255,208,.45))">'
             '<text x="500" y="250" font-family="TeX Gyre Pagella" font-size="150" letter-spacing="30" fill="url(#ldw)">DEEPWELL</text>'
             '<path d="M500,280 v60" stroke="#3dffd0" stroke-width="3"/><circle cx="500" cy="350" r="7" fill="#3dffd0"/></g>')

def l_comet():
    return L('', '<g text-anchor="middle" font-family="Poppins" font-weight="700" style="filter:drop-shadow(6px 7px 0 #2a1060)">'
             '<text x="500" y="200" font-size="150" letter-spacing="-3" fill="#ffe14d" stroke="#2a1060" stroke-width="8" paint-order="stroke" transform="rotate(-4 500 160)">Comet</text>'
             '<text x="520" y="310" font-size="110" letter-spacing="-2" fill="#4dffd2" stroke="#2a1060" stroke-width="8" paint-order="stroke" transform="rotate(-4 500 280)">Kitchen!</text></g>')

def l_atlas():
    return L(lin('lab', [(0, '#fff6dc'), (1, '#ffcf7a')]),
             '<g text-anchor="middle" style="filter:drop-shadow(0 4px 18px rgba(0,0,0,.8))">'
             '<text x="500" y="200" font-family="TeX Gyre Bonum" font-size="136" letter-spacing="34" fill="url(#lab)">ATLAS</text>'
             '<path d="M260,236 H420 M580,236 H740" stroke="#ffcf7a" stroke-width="2"/><circle cx="500" cy="236" r="10" fill="none" stroke="#ffcf7a" stroke-width="3"/>'
             '<text x="500" y="310" font-family="TeX Gyre Bonum" font-size="64" letter-spacing="44" fill="#bfe8e0">BELOW</text></g>')

def l_plain(title):
    """Beacon's games: just the name in plain type, as Hearth itself shows it."""
    return L('', f'<text x="500" y="230" text-anchor="middle" font-family="Inter" font-weight="700" font-size="60" fill="#fff">{title}</text>')


GAMES = {
    'g01': ('Starfall Drift', s_starfall, l_starfall, 'top'),
    'g02': ('Hollow Pines', s_hollow, l_hollow, 'top'),
    'g03': ('Ironvale Tactics', s_ironvale, l_ironvale, 'top'),
    'g04': ('Neon Courier', s_neon, l_neon, 'top'),
    'g05': ('Tidebound', s_tidebound, l_tidebound, 'top'),
    'g06': ('Paper Moon Cafe', s_papermoon, l_papermoon, 'top'),
    'g07': ('Grimforge', s_grimforge, l_grimforge, 'top'),
    'g08': ('Orbit Garden', s_orbit, l_orbit, 'bottom'),
    'g09': ('Rustline Rally', s_rustline, l_rustline, 'top'),
    'g10': ('Echo Protocol', s_echo, l_echo, 'bottom'),
    'g11': ('Lantern Keep', s_lantern, l_lantern, 'bottom'),
    'g12': ('Moss & Marrow', s_moss, l_moss, 'top'),
    'g13': ('Skyline Builders', s_skyline, l_skyline, 'top'),
    'g14': ('Velvet Static', s_velvet, l_velvet, 'bottom'),
    'g15': ('Frostbyte Arena', s_frostbyte, l_frostbyte, 'top'),
    'g16': ('Deepwell', s_deepwell, l_deepwell, 'bottom'),
    'g17': ('Comet Kitchen', s_comet, l_comet, 'bottom'),
    'g18': ('Atlas Below', s_atlas, l_atlas, 'bottom'),
}
BEACON = ['beacon:bfme2', 'beacon:rotwk', 'beacon:aotr', 'beacon:conquest', 'beacon:bfme1', 'beacon:swgb']


def doc(W, H, body, transparent=False):
    return (f'<!doctype html><html><head><meta charset="utf-8"><style>html,body{{margin:0;background:{"transparent" if transparent else "#000"}}}svg{{display:block}}</style></head>'
            f'<body><svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">{body}</svg></body></html>')


BEACON_TITLES = {'beacon:bfme2': 'The Battle for Middle-earth II', 'beacon:rotwk': 'The Rise of the Witch-king',
                 'beacon:aotr': 'Age of the Ring', 'beacon:conquest': 'The Lord of the Rings: Conquest',
                 'beacon:bfme1': 'The Battle for Middle-earth', 'beacon:swgb': 'Star Wars: Galactic Battlegrounds Saga'}


def wrap(t, n):
    lines, cur = [], ''
    for w in t.split():
        if cur and len(cur) + 1 + len(w) > n: lines.append(cur); cur = w
        else: cur = (cur + ' ' + w).strip()
    return lines + [cur]


def logo_image(logo, box_w, box_h, cx, y, anchor):
    """Place a rendered logo (png bytes, w, h) to fit box_w x box_h, centred on cx; y is its top or bottom edge."""
    import base64
    png, w, h = logo
    s = min(box_w / w, box_h / h)
    lw, lh = w * s, h * s
    top = y if anchor == 'top' else y - lh
    return f'<image href="data:image/png;base64,{base64.b64encode(png).decode()}" x="{f(cx - lw / 2)}" y="{f(top)}" width="{f(lw)}" height="{f(lh)}"/>'


def compose(gid, kind, logo=None):
    """kind: cover (600x900, with title), hero (1920x620, no title), wide (920x430, with title), logo (transparent)."""
    if gid in GAMES:
        title, scene, logofn, where = GAMES[gid]
    else:
        i = BEACON.index(gid)
        title, scene, logofn, where = BEACON_TITLES[gid], (lambda c, i=i: s_fantasy(c, i)), None, 'bottom'
    W, H = {'cover': (600, 900), 'hero': (1920, 620), 'wide': (920, 430), 'logo': (1100, 420)}[kind]
    if kind == 'logo':
        d, b = logofn()
        return doc(W, H, f'<defs>{d}</defs><g transform="translate(50,10)">{b}</g>', True), W, H
    c = C(W, H, kind)
    body = scene(c)
    if kind == 'cover':
        body += vignette(W, H, .45)
        if logo:
            if where == 'top':
                body += f'<defs>{lin("tf", [(0, "#000", .55), (1, "#000", 0)])}</defs><rect width="{W}" height="380" fill="url(#tf)"/>'
                body += logo_image(logo, W * .8, H * .21, W / 2, 96, 'top')
            else:
                body += bottom_fade(W, H, .42, '#000', .75) + logo_image(logo, W * .84, H * .23, W / 2, H - 50, 'bottom')
        else:
            # Beacon's games: the name in plain type at the bottom, the way Hearth draws a game without art.
            lines = wrap(title, 16)
            body += bottom_fade(W, H, .5, '#000', .8)
            y0 = H - 54 - (len(lines) - 1) * 62
            body += ''.join(f'<text x="44" y="{y0 + k * 62}" font-family="Inter" font-weight="800" font-size="54" letter-spacing="-1" fill="#fff">{t.replace("&", "&amp;")}</text>' for k, t in enumerate(lines))
    elif kind == 'hero':
        body += vignette(W, H, .35)
    elif kind == 'wide':
        body += vignette(W, H, .4)
        if logo:
            body += bottom_fade(W, H, .55, '#000', .6) + logo_image(logo, W * .62, H * .4, W / 2, H - 22, 'bottom')
        else:
            lines = wrap(title, 26)
            body += bottom_fade(W, H, .55, '#000', .7)
            y0 = H - 34 - (len(lines) - 1) * 50
            body += ''.join(f'<text x="34" y="{y0 + k * 50}" font-family="Inter" font-weight="800" font-size="44" fill="#fff">{t}</text>' for k, t in enumerate(lines))
    body += grain(W, H, .05)
    return doc(W, H, body), W, H


async def main(only=None):
    from playwright.async_api import async_playwright
    from PIL import Image
    os.makedirs(OUT, exist_ok=True)
    async with async_playwright() as p:
        b = await p.chromium.launch()
        pg = await b.new_page()

        async def shot(html, W, H, transparent=False):
            await pg.set_viewport_size({'width': W, 'height': H})
            await pg.set_content(html)
            await pg.evaluate('document.fonts.ready')
            return Image.open(BytesIO(await pg.screenshot(omit_background=transparent)))

        for gid in list(GAMES) + BEACON:
            if only and gid not in only: continue
            name = gid.replace(':', '-')
            logo = None
            if gid in GAMES:
                im = await shot(*compose(gid, 'logo'), True)
                im = im.crop(im.getbbox())
                buf = BytesIO(); im.save(buf, 'PNG', optimize=True)
                im.save(os.path.join(OUT, f'{name}-logo.png'), optimize=True)
                logo = (buf.getvalue(), im.width, im.height)
            for kind in ['cover', 'hero', 'wide']:
                im = await shot(*compose(gid, kind, logo))
                im.convert('RGB').save(os.path.join(OUT, f'{name}-{kind}.webp'), quality=84, method=6)
            print('drew', gid, flush=True)
        await b.close()


if __name__ == '__main__':
    asyncio.run(main(sys.argv[1:] or None))
