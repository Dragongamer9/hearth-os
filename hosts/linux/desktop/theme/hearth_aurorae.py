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
mapped to the chosen look and accent.
"""
import hashlib
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


# 1.14: Settings › Personalization › Window frame colour (KDE's "frame colour intensity"): the title bar and border
# tinted with the accent, 0 (the look's own, as before) to 100 %. The active frame takes up to 55 % of the accent, the
# inactive one half of that, so the window in front still stands out.
TINT_KEYS = {'window.active.title.bg.color': 0.55, 'window.active.border.color': 0.7,
             'window.inactive.title.bg.color': 0.27, 'window.inactive.border.color': 0.35}


def mix(a, b, t):
    a, b = a.lstrip('#'), b.lstrip('#')
    ca = [int(a[i:i + 2], 16) for i in (0, 2, 4)]
    cb = [int(b[i:i + 2], 16) for i in (0, 2, 4)]
    return '#%02x%02x%02x' % tuple(round(x * (1 - t) + y * t) for x, y in zip(ca, cb))


def tint_amount(pct):
    try:
        p = int(pct)
    except (TypeError, ValueError):
        return 0
    return max(0, min(100, p))


def tint_colors(colors, accent, pct):
    """The themerc keys' colours with the accent mixed in (a copy); pct 0 gives them back unchanged."""
    p = tint_amount(pct)
    if not p or not re.match(r'^#[0-9a-fA-F]{6}$', accent or ''):
        return dict(colors)
    out = dict(colors)
    for k, w in TINT_KEYS.items():
        if re.match(r'^#[0-9a-fA-F]{6}$', out.get(k, '')):
            out[k] = mix(out[k], accent, w * p / 100.0)
    return out


def tint_themerc(text, accent, pct):
    """labwc's themerc-override text with the same tint (only those keys' lines change)."""
    p = tint_amount(pct)
    if not p or not re.match(r'^#[0-9a-fA-F]{6}$', accent or ''):
        return text

    def one(m):
        return m.group(1) + mix(m.group(2), accent, TINT_KEYS[m.group(1).split(':')[0].strip()] * p / 100.0)
    keys = '|'.join(re.escape(k) for k in TINT_KEYS)
    return re.sub(r'(?m)^((?:%s):\s*)(#[0-9a-fA-F]{6})' % keys, one, text)


def rgb_text(h):
    h = h.lstrip('#')
    return ','.join(str(int(h[i:i + 2], 16)) for i in (0, 2, 4))


def frame_svg(c, radius):
    """decoration.svg: the frame in nine parts, active and inactive. Top corners rounded (labwc rounds the title bar's
    top corners only); the window's own content stays square below."""
    r = max(0, min(int(radius), 16))
    parts = []
    for prefix, bg, line in (('', c['active.bg'], c['active.border']), ('inactive-', c['inactive.bg'], c['inactive.border'])):
        R = max(r, 1)
        # Corners are R x TITLE_H at the top, BORDER x BORDER at the bottom; edges stretch.
        def rect(eid, x, y, w, h, fill):
            parts.append('<rect id="decoration-%s%s" x="%d" y="%d" width="%d" height="%d" fill="%s"/>' % (prefix, eid, x, y, w, h, fill))
        oy = 0 if prefix == '' else 200        # the inactive set below the active one
        ox = 0
        # top-left: a rounded corner filled with the title colour, the border on its outside
        tl = ('<g id="decoration-%stopleft"><rect x="%d" y="%d" width="%d" height="%d" fill="none"/>'
              '<path d="M %d %d L %d %d A %d %d 0 0 1 %d %d L %d %d L %d %d Z" fill="%s"/>'
              '<path d="M %.1f %d L %.1f %d A %.1f %.1f 0 0 1 %d %.1f" fill="none" stroke="%s" stroke-width="%d"/></g>') % (
            prefix, ox, oy, R, TITLE_H,
            ox, oy + TITLE_H, ox, oy + R, R, R, ox + R, oy, ox + R, oy + TITLE_H, ox, oy + TITLE_H, bg,
            ox + 0.5, oy + TITLE_H, ox + 0.5, oy + R, R - 0.5, R - 0.5, ox + R, oy + 0.5, line, BORDER)
        parts.append(tl)
        ox2 = ox + R + 20
        parts.append(('<g id="decoration-%stop"><rect x="%d" y="%d" width="10" height="%d" fill="%s"/>'
                      '<rect x="%d" y="%d" width="10" height="%d" fill="%s"/></g>') % (prefix, ox2, oy, TITLE_H, bg, ox2, oy, BORDER, line))
        ox3 = ox2 + 30
        tr = ('<g id="decoration-%stopright"><rect x="%d" y="%d" width="%d" height="%d" fill="none"/>'
              '<path d="M %d %d L %d %d A %d %d 0 0 1 %d %d L %d %d Z" fill="%s"/>'
              '<path d="M %d %.1f A %.1f %.1f 0 0 1 %.1f %d L %.1f %d" fill="none" stroke="%s" stroke-width="%d"/></g>') % (
            prefix, ox3, oy, R, TITLE_H,
            ox3, oy + TITLE_H, ox3, oy, R, R, ox3 + R, oy + R, ox3 + R, oy + TITLE_H, bg,
            ox3, oy + 0.5, R - 0.5, R - 0.5, ox3 + R - 0.5, oy + R, ox3 + R - 0.5, oy + TITLE_H, line, BORDER)
        parts.append(tr)
        by = oy + TITLE_H + 40
        rect('left', ox, by, BORDER, 10, line)
        rect('right', ox + 20, by, BORDER, 10, line)
        rect('center', ox + 40, by, 10, 10, bg)
        rect('bottomleft', ox, by + 30, BORDER, BORDER, line)
        rect('bottom', ox + 20, by + 30, 10, BORDER, line)
        rect('bottomright', ox + 40, by + 30, BORDER, BORDER, line)
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="400" viewBox="0 0 200 400">\n'
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
        'Shadow=false',
        'DecorationPosition=0',
        '',
        '[Layout]',
        'BorderLeft=%d' % BORDER, 'BorderRight=%d' % BORDER, 'BorderBottom=%d' % BORDER, 'BorderTop=0',
        'TitleHeight=%d' % (TITLE_H - 4), 'TitleEdgeTop=2', 'TitleEdgeBottom=2', 'TitleEdgeLeft=6', 'TitleEdgeRight=4',
        'TitleBorderLeft=8', 'TitleBorderRight=8',
        'ButtonWidth=%d' % BUTTON_W, 'ButtonHeight=%d' % BUTTON_H, 'ButtonSpacing=2', 'ButtonMarginTop=0',
        'ExplicitButtonSpacer=10',
        'PaddingTop=0', 'PaddingBottom=0', 'PaddingLeft=0', 'PaddingRight=0',
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
