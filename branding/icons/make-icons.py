#!/usr/bin/env python3
"""Hearth's app icon set: draws every icon as SVG, then renders the PNG sizes and the contact sheet.

    python3 branding/icons/make-icons.py            # SVGs + PNGs + contact sheet
    python3 branding/icons/make-icons.py --svg-only # just the SVGs

Writes branding/icons/apps/<name>.svg, branding/icons/png/<N>x<N>/<name>.png (16-256) and
branding/icons/contact-sheet.png. Rendering needs Playwright's Chromium and Pillow (no other tools).
The images install these into the hicolor theme (the "Hearth icon set" block in os-bases/*/build.sh).

The style, so new icons match:
  * 512 grid. Tile: rounded square 24..488 (464 px, corner 112), warm dark gradient lit from the top
    (#3a2b22 -> #140e0b) with a faint top highlight. Same tile as Hearth Browser's original icon.
  * One simple, bold glyph per app inside 112..400, centred on 256. Primary strokes 36-44, round caps and joins.
  * Two paints: cream (stone, #fff8f0 -> #d6c3ae, top to bottom) for the main shape and ember
    (#ffd166 -> #ff8a3d -> #e5483b, the Hearth flame) for the one thing that matters. Details cut into a
    shape use ink (#2a1f19). Gradients run top to bottom across the glyph area, so light always comes from above.
  * Readable at 16-24 px: no detail thinner than ~20 units, no text.
"""
import math, pathlib, sys

HERE = pathlib.Path(__file__).resolve().parent
APPS = HERE / 'apps'
PNG = HERE / 'png'
SIZES = (16, 24, 32, 48, 64, 128, 256)

INK = '#2a1f19'
SHADE = '#c2ae98'      # the darker cream: folds, back pages, side faces
LIGHT = '#fffaf4'

DEFS = """<defs>
  <linearGradient id="tile" x1="0" y1="24" x2="0" y2="488" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#3a2b22"/><stop offset="1" stop-color="#140e0b"/>
  </linearGradient>
  <linearGradient id="rim" x1="0" y1="24" x2="0" y2="260" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#ffe9d2" stop-opacity=".22"/><stop offset="1" stop-color="#ffe9d2" stop-opacity="0"/>
  </linearGradient>
  <linearGradient id="c" x1="0" y1="112" x2="0" y2="400" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#fff8f0"/><stop offset="1" stop-color="#d6c3ae"/>
  </linearGradient>
  <linearGradient id="e" x1="0" y1="112" x2="0" y2="400" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#ffd166"/><stop offset=".55" stop-color="#ff8a3d"/><stop offset="1" stop-color="#e5483b"/>
  </linearGradient>
  <linearGradient id="core" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#fff6d6"/><stop offset="1" stop-color="#ffc24f"/>
  </linearGradient>
</defs>"""

TILE = """<rect x="24" y="30" width="464" height="464" rx="112" fill="#000" opacity=".28"/>
<rect x="24" y="24" width="464" height="464" rx="112" fill="url(#tile)"/>
<rect x="27" y="27" width="458" height="458" rx="109" fill="none" stroke="url(#rim)" stroke-width="6"/>"""

C, E = 'url(#c)', 'url(#e)'


def stroke(d, paint=C, w=40, extra=''):
    return f'<path d="{d}" fill="none" stroke="{paint}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"{extra}/>'


def fill(d, paint=C, extra=''):
    return f'<path d="{d}" fill="{paint}"{extra}/>'


def rect(x, y, w, h, r, paint=C, extra=''):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{paint}"{extra}/>'


def circle(cx, cy, r, paint=C, extra=''):
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{paint}"{extra}/>'


# A cut-out ring in the tile's color around a shape that sits on another one, so they stay apart at small sizes.
SEP = ' stroke="#211813" stroke-width="18" paint-order="stroke"'


def page(x, y, w, h, fold=60, r=24, paint=C):
    """A sheet of paper with its top-right corner folded."""
    d = (f'M{x + r} {y}H{x + w - fold}L{x + w} {y + fold}V{y + h - r}a{r} {r} 0 0 1 -{r} {r}'
         f'H{x + r}a{r} {r} 0 0 1 -{r} -{r}V{y + r}a{r} {r} 0 0 1 {r} -{r}Z')
    corner = f'M{x + w - fold} {y}V{y + fold - 14}a14 14 0 0 0 14 14H{x + w}Z'
    return fill(d, paint) + fill(corner, SHADE)


def gear(cx, cy, ro, ri, teeth, hub):
    pts = []
    step = 2 * math.pi / teeth
    for i in range(teeth):
        a = i * step - math.pi / 2
        for da, r in ((-0.27, ri), (-0.15, ro), (0.15, ro), (0.27, ri)):
            pts.append((cx + r * math.cos(a + da * step), cy + r * math.sin(a + da * step)))
    d = 'M' + ' L'.join(f'{x:.1f} {y:.1f}' for x, y in pts) + 'Z'
    return (f'<path d="{d}" fill="{C}" stroke="{C}" stroke-width="22" stroke-linejoin="round"/>'
            + circle(cx, cy, hub, INK) + circle(cx, cy, hub - 26, E))


FLAME = ('M256 168 C298 222 340 262 340 332 C340 386 302 422 256 422 C210 422 172 386 172 332 '
         'C172 294 190 266 212 244 C210 276 222 298 242 306 C232 258 236 212 256 168 Z')
FLAME_CORE = 'M258 284 C280 314 300 338 300 368 C300 394 282 412 258 412 C234 412 216 394 216 368 C216 340 238 316 258 284 Z'


def flame(tx, ty, s, ring=False):
    t = f'translate({tx} {ty}) scale({s}) translate(-256 -295)'
    out = f'<g transform="{t}">'
    if ring:
        out += f'<path d="{FLAME}" fill="none" stroke="#211813" stroke-width="{36 / s:.0f}" stroke-linejoin="round"/>'
    out += fill(FLAME, E) + fill(FLAME_CORE, 'url(#core)') + '</g>'
    return out


def cloud(paint=C, grow=0):
    g = grow
    return (f'<g fill="{paint}">'
            f'<circle cx="214" cy="300" r="{56 + g}"/><circle cx="292" cy="262" r="{78 + g}"/><circle cx="352" cy="310" r="{48 + g}"/>'
            f'<rect x="{158 - g}" y="{300 - g}" width="{242 + 2 * g}" height="{58 + 2 * g}" rx="{29 + g}"/></g>')


G = {}

# ---------------------------------------------------------------- the apps Hearth already has
G['hearth'] = (  # Game Mode: Hearth's mark (the fireplace arch and flame) on the tile
    '<g transform="translate(256 262) scale(.72) translate(-256 -290)">'
    '<path d="M112 432 V252 A144 144 0 0 1 400 252 V432" fill="none" stroke="url(#c)" stroke-width="40" stroke-linecap="round"/>'
    '<path d="M76 462 H436" stroke="url(#c)" stroke-width="36" stroke-linecap="round"/>'
    f'{fill(FLAME, E)}{fill(FLAME_CORE, "url(#core)")}</g>')
G['hearth-settings'] = gear(256, 256, 146, 112, 8, 66)
G['hearth-files'] = (
    fill('M120 178a34 34 0 0 1 34-34h66q12 0 21 9l26 27h91a34 34 0 0 1 34 34v140H120Z')
    + fill('M120 238a22 22 0 0 1 22-22h228a22 22 0 0 1 22 22v104a34 34 0 0 1-34 34H154a34 34 0 0 1-34-34Z', E))
G['hearth-terminal'] = stroke('M150 180 L232 256 L150 332', E, 44) + stroke('M268 336 H364', C, 44)
G['hearth-disks'] = (
    rect(120, 156, 272, 200, 44) + stroke('M124 262 H388', INK, 14)
    + stroke('M164 310 H262', INK, 22) + circle(340, 310, 22, E) + circle(340, 208, 14, SHADE) + circle(172, 208, 14, SHADE))
G['hearth-browser'] = (
    '<g fill="none" stroke="#b8a593" stroke-width="28">'
    '<circle cx="256" cy="248" r="142"/><ellipse cx="256" cy="248" rx="62" ry="142"/><path d="M120 200H392M120 296H392"/></g>'
    + flame(256, 296, .62, ring=True))
G['hearth-tasks'] = (  # Task Manager
    '<rect x="128" y="148" width="256" height="216" rx="44" fill="none" stroke="url(#c)" stroke-width="34"/>'
    + stroke('M168 264 H206 L234 206 L276 318 L304 256 H344', E, 30))
G['hearth-apps'] = (  # Apps (store)
    rect(128, 128, 112, 112, 32) + rect(272, 128, 112, 112, 32) + rect(128, 272, 112, 112, 32) + rect(272, 272, 112, 112, 32, E))
G['hearth-installer'] = (  # Install Hearth OS: the Hearth arch with an arrow going in
    stroke('M160 372 V254 A96 96 0 0 1 352 254 V372', C, 38) + stroke('M124 394 H388', C, 34)
    + stroke('M256 196 V330 M212 288 L256 332 L300 288', E, 38))
G['hearth-get-claude'] = (  # Get Claude: download into a tray (Claude's own icon is used once it's installed)
    stroke('M144 300 V340 a28 28 0 0 0 28 28 H340 a28 28 0 0 0 28-28 V300', C, 38)
    + stroke('M256 136 V290 M196 232 L256 292 L316 232', E, 42))

# ---------------------------------------------------------------- the new apps
G['hearth-text'] = (  # text editor: lines with a pencil
    stroke('M136 176 H300 M136 240 H252 M136 304 H204', C, 32)
    + '<g transform="rotate(40 318 280)">'
    + rect(290, 136, 56, 176, 14, E) + rect(290, 136, 56, 34, 12, SHADE)
    + fill('M290 318 H346 L318 380 Z', C) + '</g>')
G['hearth-documents'] = (  # document viewer: a page with a bookmark ribbon
    page(196, 132, 196, 252, paint=SHADE) + page(140, 112, 212, 280)
    + stroke('M180 270 H312 M180 318 H312 M180 366 H268', INK, 20).replace('d="M180 366', 'd="M180 366')
    + fill('M184 112 V218 L210 198 L236 218 V112 Z', E))
G['hearth-reader'] = (  # Reader (KDE Okular): an open book with a bookmark ribbon
    fill('M252 172C212 142 166 134 116 142V368C166 360 212 366 252 394Z', C)
    + fill('M260 172C300 142 346 134 396 142V368C346 360 300 366 260 394Z', SHADE)
    + fill('M318 146V236L340 218L362 236V140Z', E))
G['hearth-capture'] = (  # Capture (KDE Spectacle): a viewfinder around the picture taken
    stroke('M128 200V164a36 36 0 0 1 36-36H200M312 128H348a36 36 0 0 1 36 36V200'
           'M384 312V348a36 36 0 0 1-36 36H312M200 384H164a36 36 0 0 1-36-36V312', C, 36)
    + rect(196, 196, 120, 120, 28, E))
G['hearth-photos'] = (
    rect(116, 148, 280, 216, 40) + fill('M146 338 L226 246 L276 300 L310 266 L366 338 Z', INK)
    + circle(318, 210, 28, E))
G['hearth-player'] = (
    '<circle cx="256" cy="256" r="132" fill="none" stroke="url(#c)" stroke-width="34"/>'
    + fill('M228 190 L322 256 L228 322 Z', E, ' stroke="url(#e)" stroke-width="26" stroke-linejoin="round"'))
G['hearth-calculator'] = (
    rect(148, 112, 216, 288, 44) + rect(180, 146, 152, 64, 16, INK)
    + rect(180, 238, 64, 60, 16, INK) + rect(268, 238, 64, 60, 16, INK)
    + rect(180, 318, 64, 52, 16, INK) + rect(268, 318, 64, 52, 16, E))
G['hearth-clock'] = (
    circle(256, 256, 144) + stroke('M256 256 V170 M256 256 L318 294', INK, 28) + circle(256, 256, 24, E))
G['hearth-calendar'] = (
    rect(124, 148, 264, 240, 40)
    + fill('M124 188a40 40 0 0 1 40-40H348a40 40 0 0 1 40 40V230H124Z', E)
    + rect(182, 118, 28, 64, 14, C, SEP) + rect(302, 118, 28, 64, 14, C, SEP)
    + rect(166, 262, 44, 40, 10, INK) + rect(234, 262, 44, 40, 10, INK) + rect(302, 262, 44, 40, 10, E)
    + rect(166, 322, 44, 40, 10, INK) + rect(234, 322, 44, 40, 10, INK))
G['hearth-notes'] = (  # a sticky note
    fill('M164 132H348a32 32 0 0 1 32 32V298L298 380H164a32 32 0 0 1-32-32V164a32 32 0 0 1 32-32Z', E)
    + fill('M380 298L298 380V326a28 28 0 0 1 28-28Z', C)
    + stroke('M180 196 H332 M180 250 H332 M180 304 H252', INK, 22))
G['hearth-weather'] = (
    stroke('M196 112 V128 M120 200 H136 M142 146 L154 158 M250 146 L238 158', E, 22)
    + circle(196, 204, 58, E) + cloud('#211813', 10) + cloud(C))
G['hearth-recorder'] = (  # voice recorder: a microphone
    rect(208, 112, 96, 176, 48, E) + stroke('M160 236 a96 96 0 0 0 192 0', C, 34)
    + stroke('M256 334 V378 M204 386 H308', C, 34))
G['hearth-camera'] = (
    rect(196, 140, 120, 60, 20) + rect(112, 172, 288, 208, 44)
    + circle(256, 278, 76, INK) + circle(256, 278, 48, E) + circle(354, 216, 14, INK))
G['hearth-share'] = (
    stroke('M178 256 L334 172 M178 256 L334 340', C, 30)
    + circle(178, 256, 44, C) + circle(334, 172, 44, E) + circle(334, 340, 44, E))
G['hearth-paint'] = (  # a palette
    fill('M256 124c-82 0-140 60-140 134 0 74 58 130 132 130 26 0 40-14 40-34 0-20-14-28-14-44 0-18 14-30 34-30h40'
         'c40 0 48-26 48-54C396 182 336 124 256 124Z')
    + circle(190, 226, 26, '#ffd166') + circle(254, 180, 26, '#ff8a3d') + circle(322, 200, 26, '#e5483b') + circle(180, 304, 26, INK))
G['hearth-emoji'] = (
    circle(256, 256, 144, E) + circle(206, 224, 20, INK) + circle(306, 224, 20, INK)
    + stroke('M188 290 a72 64 0 0 0 136 0', INK, 26))
G['hearth-fonts'] = (
    stroke('M128 380 L206 140 L284 380 M158 300 H254', C, 40)
    + '<circle cx="338" cy="338" r="40" fill="none" stroke="url(#e)" stroke-width="32"/>' + stroke('M378 296 V380', E, 32))
G['hearth-accessibility'] = (
    '<circle cx="256" cy="256" r="146" fill="none" stroke="url(#c)" stroke-width="28"/>'
    + circle(256, 178, 28, E) + stroke('M176 226 H336 M256 226 V292 L214 360 M256 292 L298 360', E, 32))
G['hearth-controller-lab'] = (  # controller test: a gamepad with a check pulse above
    fill('M176 204H336c44 0 70 26 78 66l18 78c8 38-34 58-58 30l-40-44H198l-40 44c-24 28-66 8-58-30l18-78c8-40 34-66 78-66Z')
    + stroke('M196 254 V306 M170 280 H222', INK, 22) + circle(324, 262, 17, E) + circle(358, 296, 17, E)
    + stroke('M176 140 H214 L234 112 L262 166 L284 140 H336', E, 22))
G['hearth-saves'] = (  # game saves: a floppy disk
    fill('M156 116H318L396 194V356a40 40 0 0 1-40 40H156a40 40 0 0 1-40-40V156a40 40 0 0 1 40-40Z')
    + rect(180, 116, 124, 84, 12, INK) + rect(262, 132, 26, 52, 8, C) + rect(168, 254, 176, 112, 18, E))
G['hearth-mods'] = (  # a puzzle piece
    f'<g fill="{E}">' '<rect x="128" y="180" width="208" height="208" rx="32"/>'
    '<circle cx="232" cy="176" r="46"/><circle cx="340" cy="284" r="46"/></g>'
    + circle(128, 284, 36, '#241a15'))
G['hearth-disk-usage'] = (  # a pie chart with a slice out
    fill('M248 264V124A140 140 0 1 0 388 264Z') + fill('M270 242V102A140 140 0 0 1 410 242Z', E))
G['hearth-system-info'] = (  # a chip
    stroke('M206 128 V160 M256 128 V160 M306 128 V160 M206 352 V384 M256 352 V384 M306 352 V384 '
           'M128 206 H160 M128 256 H160 M128 306 H160 M352 206 H384 M352 256 H384 M352 306 H384', C, 22)
    + rect(160, 160, 192, 192, 36) + rect(212, 212, 88, 88, 18, E))
G['hearth-benchmark'] = (  # a speed gauge with its needle up
    stroke('M135 370 A140 140 0 1 1 377 370', C, 38)
    + stroke('M256 300 L338 214', E, 34) + circle(256, 300, 34, C) + circle(256, 300, 12, INK))
G['hearth-logs'] = (
    circle(152, 172, 18, E) + circle(152, 228, 18, E) + circle(152, 284, 18, E) + circle(152, 340, 18, E)
    + stroke('M200 172 H364 M200 228 H324 M200 284 H364 M200 340 H296', C, 28))
G['hearth-network'] = (  # Wi-Fi
    circle(256, 352, 30, E)
    + stroke(f'M{256 - 76 * .7071:.0f} {352 - 76 * .7071:.0f} A76 76 0 0 1 {256 + 76 * .7071:.0f} {352 - 76 * .7071:.0f}', C, 36)
    + stroke(f'M{256 - 146 * .7071:.0f} {352 - 146 * .7071:.0f} A146 146 0 0 1 {256 + 146 * .7071:.0f} {352 - 146 * .7071:.0f}', C, 36)
    + stroke(f'M{256 - 216 * .7071:.0f} {352 - 216 * .7071:.0f} A216 216 0 0 1 {256 + 216 * .7071:.0f} {352 - 216 * .7071:.0f}', C, 36))
G['hearth-passwords'] = (  # a padlock
    stroke('M190 236 V196 a66 66 0 0 1 132 0 V236', C, 38) + rect(148, 226, 216, 164, 38, E)
    + circle(256, 294, 24, INK) + rect(245, 300, 22, 50, 10, INK))
G['hearth-printers'] = (
    rect(180, 112, 152, 96, 14, SHADE) + rect(112, 180, 288, 144, 40)
    + rect(156, 276, 200, 18, 9, INK) + rect(176, 284, 160, 112, 14, LIGHT, ' stroke="#211813" stroke-width="10"')
    + stroke('M206 326 H306 M206 360 H276', INK, 16) + circle(350, 222, 15, E))
G['hearth-welcome'] = (  # a door left open, firelight coming through
    fill('M190 154H322V386H190Z', E)
    + fill('M190 148 L276 176 V392 L190 384 Z', SHADE) + circle(254, 284, 10, INK)
    + stroke('M174 386 V164 a24 24 0 0 1 24-24 H314 a24 24 0 0 1 24 24 V386', C, 30)
    + stroke('M130 392 H382', C, 30))
G['hearth-nearby'] = (  # share with devices near you
    circle(256, 256, 34, E)
    + stroke('M332 196 A84 84 0 0 1 332 316 M180 196 A84 84 0 0 0 180 316', C, 32)
    + stroke('M378 160 A146 146 0 0 1 378 352 M134 160 A146 146 0 0 0 134 352', C, 32))
G['hearth-connect'] = (  # your phone and this PC
    '<rect x="112" y="156" width="208" height="144" rx="20" fill="none" stroke="url(#c)" stroke-width="30"/>'
    + stroke('M100 340 H330', C, 34)
    + rect(292, 196, 104, 188, 26, E, SEP) + rect(330, 354, 28, 10, 5, INK))
G['hearth-debbox'] = (  # Debian box: a package
    fill('M256 124 L380 186 L256 248 L132 186 Z', LIGHT) + fill('M132 186 L256 248 V388 L132 326 Z', 'url(#c)')
    + fill('M256 248 L380 186 V326 L256 388 Z', SHADE)
    + fill('M172 166 L212 146 L336 208 L296 228 Z', E) + fill('M296 228 L336 208 V264 L316 274 L296 264 Z', E))
G['hearth-write'] = (  # Hearth Office: documents
    page(136, 112, 240, 288) + rect(172, 188, 132, 30, 12, E)
    + stroke('M176 262 H336 M176 306 H336 M176 350 H284', INK, 20))
G['hearth-sheets'] = (  # Hearth Office: spreadsheets
    page(136, 112, 240, 288)
    + rect(172, 192, 52, 172, 10, E)
    + '<path d="M172 192H340V364H172ZM172 249H340M172 306H340M228 192V364M284 192V364" fill="none" stroke="#2a1f19" stroke-width="12" stroke-linejoin="round"/>')
G['hearth-slides'] = (  # Hearth Office: presentations
    rect(112, 132, 288, 196, 34) + stroke('M256 330 V372 M206 384 H306', C, 30)
    + rect(160, 252, 40, 48, 8, E) + rect(226, 214, 40, 86, 8, E) + rect(292, 176, 40, 124, 8, E))
G['hearth-contacts'] = (
    rect(112, 152, 288, 208, 36) + circle(194, 230, 34, INK)
    + fill('M138 326 a56 50 0 0 1 112 0 V332 H138 Z', INK)
    + stroke('M284 222 H360 M284 274 H344', E, 22) + stroke('M284 320 H360', INK, 18))
G['hearth-maps'] = (
    fill('M120 180 L208 148 V356 L120 388 Z', C) + fill('M208 148 L304 180 V388 L208 356 Z', SHADE)
    + fill('M304 180 L392 148 V356 L304 388 Z', C)
    + fill('M256 304 C222 262 200 234 200 204 a56 56 0 0 1 112 0 c0 30-22 58-56 100Z', E, SEP)
    + circle(256, 204, 20, INK))

G['hearth-theater'] = (  # Hearth Theater: a screen with a play mark, on a little stand
    '<rect x="112" y="140" width="288" height="196" rx="36" fill="none" stroke="url(#c)" stroke-width="36"/>'
    + fill('M228 196 L312 238 L228 280 Z', E, ' stroke="url(#e)" stroke-width="24" stroke-linejoin="round"')
    + stroke('M196 392 H316 M256 340 V392', C, 32))
G['hearth-generic-app'] = (  # any app without an icon of its own
    '<rect x="128" y="148" width="256" height="216" rx="40" fill="none" stroke="#c2ae98" stroke-width="32"/>'
    + stroke('M132 214 H380', SHADE, 28) + circle(180, 188, 10, E) + rect(176, 254, 72, 72, 16, SHADE) + stroke('M284 266 H336 M284 314 H320', SHADE, 22))

WINDOW_ICONS = {'hearth-terminal': 'hosts/hearth-terminal/icons/icon.png', 'hearth-disks': 'hosts/hearth-disks/icons/icon.png',
                'hearth-text': 'hosts/hearth-text/icons/icon.png', 'hearth-theater': 'hosts/hearth-theater/icons/icon.png'}

ALIASES = {'hearth-task-manager': 'hearth-tasks', 'hearth-claude-get': 'hearth-get-claude', 'hearth-install': 'hearth-installer'}

NAMES = {
    'hearth': 'Hearth', 'hearth-settings': 'Settings', 'hearth-files': 'Files', 'hearth-terminal': 'Terminal',
    'hearth-disks': 'Disks', 'hearth-browser': 'Browser', 'hearth-tasks': 'Task Manager', 'hearth-apps': 'Apps',
    'hearth-installer': 'Install', 'hearth-get-claude': 'Get Claude',
}


def svg(name):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">\n'
            f'<!-- Hearth icon set: {name} (branding/icons/make-icons.py) -->\n{DEFS}\n{TILE}\n{G[name]}\n</svg>\n')


def write_svgs():
    APPS.mkdir(parents=True, exist_ok=True)
    for old in APPS.glob('*.svg'):
        old.unlink()
    for n in G:
        (APPS / f'{n}.svg').write_text(svg(n))
    for a, n in ALIASES.items():
        (APPS / f'{a}.svg').write_text(svg(n))
    # Mantel's own copy of the generic picture, for when the icon set isn't installed (Hearth from the app).
    (HERE.parent.parent / 'hosts/linux/desktop/shell/images/app-generic.svg').write_text(svg('hearth-generic-app'))


def render():
    from PIL import Image
    from playwright.sync_api import sync_playwright
    names = sorted(p.stem for p in APPS.glob('*.svg'))
    with sync_playwright() as p:
        b = p.chromium.launch()
        for s in SIZES:
            cols = 10
            rows = (len(names) + cols - 1) // cols
            cell = s + 8
            pg = b.new_page(viewport={'width': cols * cell, 'height': rows * cell})
            imgs = ''.join(
                f'<img src="data:image/svg+xml;base64,{__import__("base64").b64encode((APPS / (n + ".svg")).read_bytes()).decode()}" '
                f'style="position:absolute;left:{(i % cols) * cell}px;top:{(i // cols) * cell}px;width:{s}px;height:{s}px">'
                for i, n in enumerate(names))
            pg.set_content(f"<body style='margin:0;background:transparent'>{imgs}</body>")
            pg.wait_for_timeout(300)
            shot = HERE / f'.sheet-{s}.png'
            pg.screenshot(path=str(shot), omit_background=True)
            pg.close()
            sheet = Image.open(shot)
            out = PNG / f'{s}x{s}'
            out.mkdir(parents=True, exist_ok=True)
            for i, n in enumerate(names):
                x, y = (i % cols) * cell, (i // cols) * cell
                sheet.crop((x, y, x + s, y + s)).save(out / f'{n}.png', optimize=True)
            shot.unlink()
        # The Linux window icons of the Tauri apps (tauri.conf.json's bundle icon; their icon.ico stays as it is).
        root = HERE.parent.parent
        for n, dest in WINDOW_ICONS.items():
            pg = b.new_page(viewport={'width': 512, 'height': 512})
            pg.set_content(f"<body style='margin:0;background:transparent'>{(APPS / (n + '.svg')).read_text()}</body>")
            pg.wait_for_timeout(200)
            pg.screenshot(path=str(root / dest), omit_background=True)
            pg.close()
        b.close()


def contact_sheet():
    from PIL import Image, ImageDraw, ImageFont
    names = sorted(n for n in G)      # aliases are copies, left out
    cols, cw, ch = 8, 150, 108
    rows = (len(names) + cols - 1) // cols
    W, H = cols * cw + 40, rows * ch + 110
    im = Image.new('RGBA', (W, H), (15, 11, 9, 255))
    d = ImageDraw.Draw(im)
    try:
        f = ImageFont.truetype('DejaVuSans.ttf', 13)
        fb = ImageFont.truetype('DejaVuSans-Bold.ttf', 22)
    except OSError:
        f = fb = ImageFont.load_default()
    d.text((20, 18), 'Hearth icon set: 64 px and 24 px (also 16 px)', fill=(244, 236, 227), font=fb)
    d.text((20, 50), 'branding/icons/apps/*.svg, rendered by branding/icons/make-icons.py', fill=(166, 148, 132), font=f)
    for i, n in enumerate(names):
        x, y = 20 + (i % cols) * cw, 90 + (i // cols) * ch
        big = Image.open(PNG / '64x64' / f'{n}.png')
        small = Image.open(PNG / '24x24' / f'{n}.png')
        tiny = Image.open(PNG / '16x16' / f'{n}.png')
        im.alpha_composite(big, (x + 20, y))
        im.alpha_composite(small, (x + 94, y + 4))
        im.alpha_composite(tiny, (x + 98, y + 40))
        label = n.replace('hearth-', '') if n != 'hearth' else 'hearth'
        d.text((x + 4, y + 76), label, fill=(244, 236, 227), font=f)
    im.convert('RGB').save(HERE / 'contact-sheet.png', optimize=True)


if __name__ == '__main__':
    write_svgs()
    if '--svg-only' not in sys.argv:
        render()
        contact_sheet()
    print(len(G), 'icons,', len(ALIASES), 'aliases')
