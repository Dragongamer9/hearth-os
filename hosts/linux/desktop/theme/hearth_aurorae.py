"""Hearth's window frames for KWin: an Aurorae (SVG) decoration theme in Mantel's look, made by hearth-theme.

On labwc, Mantel's windows get their title bars from labwc/themerc-override, recoloured for the look and accent by
hearth-theme. KWin draws its frames with a decoration plugin instead; Aurorae takes a folder of SVG pictures, so the
same colours, corner radius, font and buttons are drawn here as Hearth's own theme:

  ~/.local/share/aurorae/themes/hearth/   metadata.desktop, hearthrc, decoration.svg, close/minimize/maximize/
                                          restore.svg (only this folder: other themes are never touched; the
                                          menu button is the app's own icon, as on labwc)

Hearth's KWin defaults (desktop/kwin/windows/xdg/kwinrc, [org.kde.kdecoration2] theme=__aurorae__svg__hearth) pick
it; a decoration someone chose in their own ~/.config/kwinrc wins. After a look change hearth-theme writes the theme
again and asks KWin to reconfigure (only under KWin), so the frames follow the look live.

The colours are labwc's (themerc-override keys), so the two window managers' frames match; build() gets them already
mapped to the chosen look and accent. So is the drop shadow (SHADOW: labwc's sizes and opacities, in front and behind),
drawn in Aurorae's padding round the frame; a maximized window has none and sits edge to edge.
"""
import hashlib
import math
import os
import re
import shutil

NAME = 'hearth'
TITLE_H = 30            # the title bar, as labwc's (padding.height 6 + a 10 pt font + the border) rounded up
BUTTON_W, BUTTON_H = 34, 24
GLYPH = 10              # the button pictures' glyph size
BORDER = 1              # labwc's border.width


def themerc_colors(text):
    """themerc-override's "key: #colour" lines as a dict."""
    out = {}
    for line in text.splitlines():
        if ':' in line and not line.lstrip().startswith('#'):
            k, _, v = line.partition(':')
            v = v.strip()
            if v.startswith('#') and len(v) in (7, 9):
                out[k.strip()] = v[:7]
    return out


def rgb_text(h):
    h = h.lstrip('#')
    return ','.join(str(int(h[i:i + 2], 16)) for i in (0, 2, 4))


# The drop shadow, as labwc's (labwc-theme(5) defaults; Hearth's themerc-override doesn't change them): 60 px for the
# window in front at #00000060 (38 %), 40 px behind it at #00000040 (25 %), straight behind the window (labwc has no
# offset), starting 30 % of its size under the window's edge (labwc's SSD_SHADOW_INSET) and fading out like a gaussian.
# Aurorae draws a shadow in the frame's padding (the PaddingLeft/Right/Top/Bottom space round the window, made the
# window's shadow by KWin and left out when it's maximized), one size for both, so the padding is the bigger one's
# visible part and the smaller shadow fades out inside it.
SHADOW = {'active': (60, 0x60 / 255.0), 'inactive': (40, 0x40 / 255.0)}
SHADOW_INSET = 0.3
PAD = int(round(SHADOW['active'][0] * (1 - SHADOW_INSET)))      # 42


def shadow_alpha(state, d):
    """The shadow's opacity D px outside the frame's edge (STATE: 'active' or 'inactive')."""
    size, a = SHADOW[state]
    t = (d + SHADOW_INSET * size) / float(size)
    return 0.0 if t >= 1 else a * math.exp(-4.5 * t * t)


def _stops(state, span, inner=0):
    """Gradient stops from the frame's edge (INNER px into the gradient) out to SPAN px."""
    out = []
    n = 12
    for i in range(n + 1):
        d = PAD * i / float(n)
        off = (inner + d) / float(span)
        out.append('<stop offset="%.4f" stop-color="#000000" stop-opacity="%.4f"/>' % (min(off, 1.0), shadow_alpha(state, d)))
    if inner:
        out.insert(0, '<stop offset="0" stop-color="#000000" stop-opacity="%.4f"/>' % shadow_alpha(state, 0))
    return ''.join(out)


def frame_svg(c, radius):
    """decoration.svg: the frame in nine parts, active and inactive, each with its shadow on the outside (PAD px, see
    SHADOW), plus flat ones for a maximized window (no shadow, no border). Top corners rounded (labwc rounds the title
    bar's top corners only); the window's own content stays square below.

    Each part is a group whose first rect is its size (KSvg takes a part's size from its bounds). The sides are PAD + R
    wide (R the corner radius) so the rounded corner fits whichever way KSvg sizes the corners; the client covers all
    but the outer PAD + 1 px of them (Aurorae places it by the hearthrc borders and the padding)."""
    r = max(0, min(int(radius), 16))
    R = max(r, BORDER)
    P = PAD
    W = P + R                      # the sides' and corners' width
    TH = P + TITLE_H               # the top parts' height
    BH = P + BORDER                # the bottom parts' height
    defs, parts = [], []

    def group(eid, x, y, w, h, body):
        parts.append('<g id="%s"><rect x="%d" y="%d" width="%d" height="%d" fill="#000000" fill-opacity="0"/>%s</g>'
                     % (eid, x, y, w, h, body))

    def box(x, y, w, h, fill):
        return '<rect x="%s" y="%s" width="%s" height="%s" fill="%s"/>' % (x, y, w, h, fill)

    def corner(x, y, rr, h, line, fill, right):
        """A rounded title bar corner RR wide and H tall at (X, Y): the border as the outer shape in LINE, the title
        colour as the inner one, both filled (a stroke's bounds reach half its width past the part, and KSvg would
        shrink the part to fit: a light seam where it meets the next one)."""
        def shape(inset):
            r2 = rr - inset
            if right:
                x0, x1 = x, x + rr - inset          # the arc on the right
                arc = ('L %s %s A %s %s 0 0 1 %s %s' % (x1 - r2, y + inset, r2, r2, x1, y + rr)) if r2 > 0 else 'L %s %s' % (x1, y + inset)
                return 'M %s %s L %s %s %s L %s %s Z' % (x0, y + h, x0, y + inset, arc, x1, y + h)
            x0, x1 = x + inset, x + rr               # the arc on the left
            arc = ('L %s %s A %s %s 0 0 1 %s %s' % (x0, y + rr, r2, r2, x0 + r2, y + inset)) if r2 > 0 else 'L %s %s' % (x0, y + inset)
            return 'M %s %s %s L %s %s L %s %s Z' % (x0, y + h, arc, x1, y + inset, x1, y + h)
        return ('<path d="%s" fill="%s"/><path d="%s" fill="%s"/>' % (shape(0), line, shape(BORDER), fill))

    def lin(gid, state, x1, y1, x2, y2):
        defs.append('<linearGradient id="%s" gradientUnits="userSpaceOnUse" x1="%s" y1="%s" x2="%s" y2="%s">%s</linearGradient>'
                    % (gid, x1, y1, x2, y2, _stops(state, P)))
        return 'url(#%s)' % gid

    def rad(gid, state, cx, cy, inner):
        defs.append('<radialGradient id="%s" gradientUnits="userSpaceOnUse" cx="%s" cy="%s" fx="%s" fy="%s" r="%s">%s</radialGradient>'
                    % (gid, cx, cy, cx, cy, inner + P, _stops(state, inner + P, inner)))
        return 'url(#%s)' % gid

    sets = (('', 'active', c['active.bg'], c['active.border']), ('inactive-', 'inactive', c['inactive.bg'], c['inactive.border']))
    for k, (prefix, state, bg, line) in enumerate(sets):
        oy = k * 400                       # the inactive set below the active one
        g = 's%d' % k
        e = 'decoration-' + prefix
        # top-left: the shadow round the rounded corner (centred on the arc's centre), the corner in the title colour,
        # the border on its outside, then the left side's shadow and border down to the title bar's bottom.
        x0, y0 = 0, oy
        cx, cy = x0 + P + R, y0 + P + R
        body = box(x0, y0, W, P + R, rad(g + 'tl', state, cx, cy, R))
        body += box(x0, y0 + P + R, P, TITLE_H - R, lin(g + 'tll', state, x0 + P, 0, x0, 0)) if TITLE_H > R else ''
        body += corner(x0 + P, y0 + P, R, TH - P, line, bg, False)
        group(e + 'topleft', x0, y0, W, TH, body)
        # top: the shadow above, the border, the title colour
        x0 = W + 20
        body = box(x0, y0, 10, P, lin(g + 't', state, 0, y0 + P, 0, y0)) + box(x0, y0 + P, 10, TITLE_H, bg) + box(x0, y0 + P, 10, BORDER, line)
        group(e + 'top', x0, y0, 10, TH, body)
        # top-right: top-left mirrored
        x0 = W + 50
        cx = x0                            # the arc's centre (the window's right edge is at x0 + R)
        body = box(x0, y0, W, P + R, rad(g + 'tr', state, cx, cy, R))
        body += box(x0 + R, y0 + P + R, P, TITLE_H - R, lin(g + 'trr', state, x0 + R, 0, x0 + R + P, 0)) if TITLE_H > R else ''
        body += corner(x0, y0 + P, R, TH - P, line, bg, True)
        group(e + 'topright', x0, y0, W, TH, body)
        # the sides, the middle (under the window), the bottom
        by = y0 + TH + 40
        x0 = 0
        group(e + 'left', x0, by, W, 10, box(x0, by, P, 10, lin(g + 'l', state, x0 + P, 0, x0, 0)) + box(x0 + P, by, BORDER, 10, line)
              + box(x0 + P + BORDER, by, R - BORDER, 10, bg) if R > BORDER else
              box(x0, by, P, 10, lin(g + 'l', state, x0 + P, 0, x0, 0)) + box(x0 + P, by, BORDER, 10, line))
        x0 = W + 20
        group(e + 'center', x0, by, 10, 10, box(x0, by, 10, 10, bg))
        x0 = W + 50
        group(e + 'right', x0, by, W, 10, (box(x0, by, R - BORDER, 10, bg) if R > BORDER else '')
              + box(x0 + R - BORDER, by, BORDER, 10, line) + box(x0 + R, by, P, 10, lin(g + 'r', state, x0 + R, 0, x0 + R + P, 0)))
        by += 30
        # bottom corners: square (labwc's are); the shadow round them centred on the frame's outer corner
        x0 = 0
        group(e + 'bottomleft', x0, by, W, BH,
              box(x0, by, P, BORDER, lin(g + 'bll', state, x0 + P, 0, x0, 0))
              + box(x0, by + BORDER, P, P, rad(g + 'bl', state, x0 + P, by + BORDER, 0))
              + box(x0 + P, by + BORDER, R, P, lin(g + 'blb', state, 0, by + BORDER, 0, by + BORDER + P))
              + box(x0 + P, by, R, BORDER, line))
        x0 = W + 20
        group(e + 'bottom', x0, by, 10, BH, box(x0, by, 10, BORDER, line) + box(x0, by + BORDER, 10, P, lin(g + 'b', state, 0, by + BORDER, 0, by + BORDER + P)))
        x0 = W + 50
        group(e + 'bottomright', x0, by, W, BH,
              box(x0, by, R, BORDER, line)
              + box(x0, by + BORDER, R, P, lin(g + 'brb', state, 0, by + BORDER, 0, by + BORDER + P))
              + box(x0 + R, by + BORDER, P, P, rad(g + 'br', state, x0 + R, by + BORDER, 0))
              + box(x0 + R, by, P, BORDER, lin(g + 'brr', state, x0 + R, 0, x0 + R + P, 0)))
        # Maximized: flat title colour, no shadow, no border (KWin leaves the padding out then; Fitts' law at the edges).
        my = by + BH + 40
        for j, part in enumerate(('topleft', 'top', 'topright', 'left', 'center', 'right', 'bottomleft', 'bottom', 'bottomright')):
            mx = (j % 3) * 20
            yy = my + (j // 3) * 20
            mid = 'decoration-maximized' + ('-inactive' if prefix else '') + '-' + part
            group(mid, mx, yy, 10 if j % 3 == 1 else 1, 10 if j // 3 == 1 else 1, box(mx, yy, 10 if j % 3 == 1 else 1, 10 if j // 3 == 1 else 1, bg))
    width = 2 * W + 60
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="800" viewBox="0 0 %d 800">\n<defs>%s</defs>\n' % (width, width, ''.join(defs))
            + '\n'.join(parts) + '\n</svg>\n')


GLYPHS = {
    # In a GLYPH x GLYPH box at (0, 0): labwc's shapes (iconify, max, close), drawn a little rounder.
    'minimize': '<path d="M 0.5 {m} H {e}" />',
    'maximize': '<rect x="0.75" y="0.75" width="{s1}" height="{s1}" rx="2" />',
    'restore': '<rect x="0.75" y="2.75" width="{s3}" height="{s3}" rx="1.5" /><path d="M 3 0.75 H {e1} a 1.5 1.5 0 0 1 1.5 1.5 V {s3b}" />',
    'close': '<path d="M 0.75 0.75 L {e1} {e1} M {e1} 0.75 L 0.75 {e1}" />',
}


def button_svg(kind, c):
    """One button: Aurorae's active / inactive / hover / pressed / deactivated pictures. Hover and press are a rounded
    pill (labwc's window.button.hover.bg, corner radius 6); Close's is the danger colour."""
    g = GLYPHS[kind].format(m=GLYPH / 2, e=GLYPH - 0.5, e1=GLYPH - 0.75, s1=GLYPH - 1.5, s3=GLYPH - 3.5,
                            s3b=GLYPH - 3.5, r=GLYPH / 2 - 1)
    hover = c['danger'] if kind == 'close' else c['hover']
    hover_ink = '#ffffff' if kind == 'close' else c['active.text']
    states = [('active', None, c['active.text']), ('inactive', None, c['inactive.text']),
              ('hover', hover, hover_ink), ('pressed', hover, hover_ink), ('deactivated', None, c['inactive.text'])]
    out = []
    for i, (st, pill, ink) in enumerate(states):
        x = i * (BUTTON_W + 4)
        fill = 'fill="none"'
        body = ('<rect x="%d" y="0" width="%d" height="%d" rx="6" fill="%s"%s/>' % (
            x, BUTTON_W, BUTTON_H, pill or '#000000', '' if pill else ' fill-opacity="0"'))
        body += '<g transform="translate(%.1f %.1f)" stroke="%s" stroke-width="1.5" stroke-linecap="round" %s>%s</g>' % (
            x + (BUTTON_W - GLYPH) / 2, (BUTTON_H - GLYPH) / 2, ink, fill, g)
        out.append('<g id="%s-center">%s</g>' % (st, body))
    w = len(states) * (BUTTON_W + 4)
    return ('<?xml version="1.0" encoding="UTF-8"?>\n<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
            'viewBox="0 0 %d %d">\n%s\n</svg>\n' % (w, BUTTON_H, w, BUTTON_H, '\n'.join(out)))


def rc(c):
    """hearthrc: sizes, title alignment, text colours."""
    return '\n'.join([
        '[General]',
        'ActiveTextColor=%s' % rgb_text(c['active.text']),
        'InactiveTextColor=%s' % rgb_text(c['inactive.text']),
        'TitleAlignment=Center',
        'TitleVerticalAlignment=Center',
        'Animation=120',
        'Shadow=true',
        'DecorationPosition=0',
        '',
        '[Layout]',
        'BorderLeft=%d' % BORDER, 'BorderRight=%d' % BORDER, 'BorderBottom=%d' % BORDER, 'BorderTop=0',
        'TitleHeight=%d' % (TITLE_H - 4), 'TitleEdgeTop=2', 'TitleEdgeBottom=2', 'TitleEdgeLeft=6', 'TitleEdgeRight=4',
        'TitleBorderLeft=8', 'TitleBorderRight=8',
        'ButtonWidth=%d' % BUTTON_W, 'ButtonHeight=%d' % BUTTON_H, 'ButtonSpacing=2', 'ButtonMarginTop=0',
        'ExplicitButtonSpacer=10',
        # The shadow's room (see SHADOW); KWin leaves it out for a maximized window, whose title bar sits flush.
        'PaddingTop=%d' % PAD, 'PaddingBottom=%d' % PAD, 'PaddingLeft=%d' % PAD, 'PaddingRight=%d' % PAD,
        'TitleEdgeTopMaximized=0', 'TitleEdgeBottomMaximized=0', 'TitleEdgeLeftMaximized=0', 'TitleEdgeRightMaximized=0',
        '',
    ])


METADATA = """[Desktop Entry]
Name=Hearth
Comment=Hearth Desktop Mode's window frames (written by Hearth for the look you chose; changes with it)
X-KDE-PluginInfo-Name=%(name)s
X-Hearth-Frames=1
X-KDE-PluginInfo-Author=Dragongamer9
X-KDE-PluginInfo-License=GPL-3.0-or-later
X-KDE-PluginInfo-Website=https://hearthos.fun
X-Plasma-API=javascript
"""


def build(colors, radius, name=NAME):
    """{file name: text} for the theme folder NAME. colors: the mapped themerc keys plus 'danger'."""
    c = {
        'active.bg': colors.get('window.active.title.bg.color', '#1c1512'),
        'inactive.bg': colors.get('window.inactive.title.bg.color', '#140f0c'),
        'active.border': colors.get('window.active.border.color', '#3b2e26'),
        'inactive.border': colors.get('window.inactive.border.color', '#261d18'),
        'active.text': colors.get('window.active.label.text.color', '#f4ece3'),
        'inactive.text': colors.get('window.inactive.label.text.color', '#a69484'),
        'hover': colors.get('window.active.button.hover.bg.color', '#33271f'),
        'danger': colors.get('danger', '#e5484d'),
    }
    files = {'metadata.desktop': METADATA % {'name': name}, name + 'rc': rc(c), 'decoration.svg': frame_svg(c, radius)}
    for kind in ('close', 'minimize', 'maximize', 'restore'):
        files[kind + '.svg'] = button_svg(kind, c)
    return files


def theme_dir(data, name=NAME):
    return os.path.join(data, 'aurorae', 'themes', name)


def version_name(colors, radius):
    """KWin keeps a theme it has loaded (Aurorae caches it by name), so each look gets a folder of its own,
    hearth-<8 hex of its pictures>, and the new name is what KWin is told to use; "hearth" itself is the same frames
    under the name Hearth's KWin defaults pick before any of that."""
    blob = ''.join(v for k, v in sorted(build(colors, radius).items()) if k != 'metadata.desktop' and not k.endswith('rc'))
    blob += build(colors, radius)[NAME + 'rc']
    return NAME + '-' + hashlib.sha1(blob.encode()).hexdigest()[:8]


def ours(folder):
    try:
        with open(os.path.join(folder, 'metadata.desktop'), encoding='utf-8') as f:
            return 'X-Hearth-Frames=1' in f.read()
    except OSError:
        return False


def write_theme(data, colors, radius, write):
    """Writes the frames as "hearth" and as this look's own hearth-<hex> (see version_name) with WRITE(path, text) ->
    changed, and removes Hearth's older hearth-<hex> folders (only folders Hearth made: X-Hearth-Frames in their
    metadata; nobody else's theme is touched). Returns (changed, the look's theme name)."""
    changed = False
    current = version_name(colors, radius)
    for name in (NAME, current):
        d = theme_dir(data, name)
        for fname, text in build(colors, radius, name).items():
            changed |= bool(write(os.path.join(d, fname), text))
    root = os.path.join(data, 'aurorae', 'themes')
    for old in os.listdir(root) if os.path.isdir(root) else []:
        if re.fullmatch(NAME + r'-[0-9a-f]{8}', old) and old != current and ours(os.path.join(root, old)):
            shutil.rmtree(os.path.join(root, old), ignore_errors=True)
    return changed, current


DECO_GROUP = 'org.kde.kdecoration2'


def kwinrc_pick(text, theme):
    """The person's kwinrc with Hearth's frames picked: [org.kde.kdecoration2] theme=__aurorae__svg__THEME, but only
    when the decoration there is Hearth's or none was chosen (a library or theme someone picked stays). Returns the
    new text, or None when it's not Hearth's to change."""
    lines = text.splitlines(True)
    start = end = None
    for i, l in enumerate(lines):
        if l.strip() == '[%s]' % DECO_GROUP:
            start = i
        elif start is not None and end is None and l.startswith('['):
            end = i
    if start is not None and end is None:
        end = len(lines)
    want = 'theme=__aurorae__svg__%s\n' % theme
    if start is None:
        sep = '' if not text or text.endswith('\n\n') else ('\n' if text.endswith('\n') else '\n\n')
        return text + sep + '[%s]\n' % DECO_GROUP + want
    body = lines[start + 1:end]
    for l in body:
        k, _, v = l.partition('=')
        k, v = k.strip(), v.strip()
        if k == 'library' and v not in ('org.kde.kwin.aurorae', 'org.kde.kwin.aurorae.v2'):
            return None
        if k == 'theme' and not v.startswith('__aurorae__svg__' + NAME):
            return None
    out = [l for l in body if l.partition('=')[0].strip() != 'theme']
    out.insert(0, want)
    return ''.join(lines[:start + 1] + out + lines[end:])
