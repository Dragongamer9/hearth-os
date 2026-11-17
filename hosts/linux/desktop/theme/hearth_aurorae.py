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
import os

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
X-KDE-PluginInfo-Name=hearth
X-KDE-PluginInfo-Author=Dragongamer9
X-KDE-PluginInfo-License=GPL-3.0-or-later
X-KDE-PluginInfo-Website=https://hearthos.fun
X-Plasma-API=javascript
"""


def build(colors, radius):
    """{file name: text} for the theme folder. colors: the mapped themerc keys plus 'danger'."""
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
    files = {'metadata.desktop': METADATA, NAME + 'rc': rc(c), 'decoration.svg': frame_svg(c, radius)}
    for kind in ('close', 'minimize', 'maximize', 'restore'):
        files[kind + '.svg'] = button_svg(kind, c)
    return files


def theme_dir(data):
    return os.path.join(data, 'aurorae', 'themes', NAME)


def write_theme(data, colors, radius, write):
    """Writes the theme into DATA/aurorae/themes/hearth with WRITE(path, text) -> changed. True if anything changed."""
    d = theme_dir(data)
    changed = False
    for name, text in build(colors, radius).items():
        changed |= bool(write(os.path.join(d, name), text))
    return changed
