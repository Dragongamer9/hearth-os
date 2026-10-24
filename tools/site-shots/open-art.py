#!/usr/bin/env python3
"""Game art for the website's screenshots, from open games whose art is free to share (CC BY-SA / GPL).

ui/index.html's browser preview has a second sample library, `?preview=linux&samples=open`, made of real open-source
games (SuperTuxKart, 0 A.D., The Battle for Wesnoth, Veloren, OpenTTD, Endless Sky). Their pictures live in
site/img/games/ and are made here, from each project's own repository at a fixed commit. The art is only resized,
cropped (crops listed below and in site/img/games/CREDITS.md) and converted to WebP: nothing is drawn over it or
changed otherwise, as CC BY-SA asks of anything that isn't shared as an adaptation. The credits on the website
(site/index.html, "Game art") list every file with its author, licence and source; keep them in step with this list.

  python3 tools/site-shots/open-art.py      (needs Pillow; downloads about 25 MB)"""
import io, os, sys, urllib.request
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
OUT = os.path.join(ROOT, 'site', 'img', 'games')

GH = 'https://raw.githubusercontent.com/'
STK = GH + 'supertuxkart/stk-code/2997f192e6876047909d909498dbb4810e5dbb8d/'
ZAD = GH + '0ad/0ad/61a3b9507d974084e6badb88a0826bd89a6d5b8b/'
WES = GH + 'wesnoth/wesnoth/700df650eacad399a84e903d1560db154bd63abd/'
VEL = 'https://gitlab.com/veloren/veloren/-/raw/c5024ebfde27b983b497d8400b99d084ee28d809/'
TTD = GH + 'OpenTTD/website/91b26ac86c09842c9e115b4949107d8240141c13/'
ES = GH + 'endless-sky/endless-sky/c3c7908862cb18fef88459fd970c208ca06cba20/'
ESW = GH + 'endless-sky/endless-sky.github.io/22dd15b37af679546b32903563f803b50d0cf5fc/'

# output file: (source URL, crop box in the source's pixels or None, size to fit in (w, h))
#   cover: 2:3 at 600x900   hero: up to 1600 wide   wide: up to 640 wide   logo: up to 600 wide, keeps transparency
ART = {
    'stk-cover':  (STK + 'data/skins/cartoon/background.jpg', (780, 0, 1500, 1080), (600, 900)),
    'stk-hero':   (STK + 'data/skins/cartoon-forest/background.jpg', None, (1600, 900)),
    'stk-wide':   (STK + 'data/skins/cartoon-desert/background.jpg', None, (640, 360)),
    'stk-logo':   (STK + 'data/gui/icons/logo.png', None, (512, 512)),
    '0ad-cover':  (ZAD + 'binaries/data/mods/public/art/textures/ui/pregame/backgrounds/hellenes1-1.png', (500, 0, 1183, 1024), (600, 900)),
    '0ad-hero':   (ZAD + 'binaries/data/mods/public/art/textures/ui/pregame/backgrounds/hellenes1-1.png', None, (1600, 800)),
    '0ad-wide':   (ZAD + 'binaries/data/mods/public/art/textures/ui/pregame/backgrounds/hellenes1-1.png', None, (640, 360)),
    '0ad-logo':   (ZAD + 'binaries/data/mods/public/art/textures/ui/pregame/shell/logo/0ad_logo.png', None, (256, 128)),
    'wesnoth-cover': (WES + 'data/campaigns/The_Rise_Of_Wesnoth/images/story/trow_intro_04.webp', (200, 0, 920, 1080), (600, 900)),
    'wesnoth-hero':  (WES + 'data/campaigns/The_Rise_Of_Wesnoth/images/story/trow_story_03-A_Harrowing_Escape.webp', None, (1600, 900)),
    'wesnoth-wide':  (WES + 'data/campaigns/The_Rise_Of_Wesnoth/images/story/trow_story_15-A_New_Land.webp', None, (640, 360)),
    'wesnoth-logo':  (WES + 'images/misc/logo.png', None, (600, 200)),
    'veloren-cover': (VEL + 'assets/voxygen/background/bg_5.jpg', (240, 0, 960, 1080), (600, 900)),
    'veloren-hero':  (VEL + 'assets/voxygen/background/bg_12.jpg', None, (1600, 900)),
    'veloren-wide':  (VEL + 'assets/voxygen/background/bg_20.jpg', None, (640, 360)),
    'openttd-cover': (TTD + '_screenshots/1.4-02-opengfx-1920x1200.png', (0, 60, 760, 1200), (600, 900)),
    'openttd-hero':  (TTD + '_screenshots/1.4-02-opengfx-1920x1200.png', (0, 0, 1920, 480), (1600, 400)),
    'openttd-wide':  (TTD + '_screenshots/1.4-02-opengfx-1920x1200.png', (1130, 720, 1920, 1164), (640, 360)),
    'es-cover':   (ESW + 'screenshots/fight.jpg', (130, 0, 530, 600), (600, 900)),
    'es-hero':    (ESW + 'screenshots/beams.jpg', None, (800, 600)),
    'es-wide':    (ESW + 'screenshots/fight.jpg', (0, 75, 800, 525), (640, 360)),
    'es-logo':    (ES + 'images/_menu/title@1x.png', None, (300, 40)),
}

_cache = {}
def fetch(url):
    if url not in _cache:
        req = urllib.request.Request(url, headers={'User-Agent': 'hearth-os site art'})
        with urllib.request.urlopen(req, timeout=120) as r: _cache[url] = r.read()
    return Image.open(io.BytesIO(_cache[url]))


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, (url, crop, box) in ART.items():
        im = fetch(url)
        logo = name.endswith('-logo')
        im = im.convert('RGBA' if logo else 'RGB')
        if crop: im = im.crop(crop)
        if im.width > box[0] or im.height > box[1]:
            im.thumbnail(box, Image.LANCZOS)   # only ever made smaller
        path = os.path.join(OUT, name + '.webp')
        if logo: im.save(path, 'WEBP', lossless=True, method=6)
        else: im.save(path, 'WEBP', quality=82, method=6)
        print(f'{name}.webp {im.size[0]}x{im.size[1]} {os.path.getsize(path)//1024} KB')


if __name__ == '__main__':
    sys.exit(main())
