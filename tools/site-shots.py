#!/usr/bin/env python3
"""Redraws the website's Game Mode screenshots (site/img/gm-*.webp) from Hearth's real page, ui/index.html, in a
browser with its built-in sample library, store and friends (the same preview the layout audit uses). Hearth Link's
page needs the Link service, so it's shown with a sample state (two paired devices, one copy running).
The screenshots of Mantel, the keyboard and the apps come from the CI screenshot branches (ci/desktop-shots-*,
ci/layout-shots-*, ci/hearth-text-shots-*, ci/apps-create-shots); see the list at the bottom.
  python3 tools/site-shots.py            (needs: pip install playwright pillow; playwright install chromium)"""
import asyncio, json, os, sys
from playwright.async_api import async_playwright
from PIL import Image

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
IMG = os.path.join(ROOT, 'site', 'img')
LINK = {"running": True, "installed": True, "me": {"name": "Living room PC"}, "nearby": [], "offers": [],
        "peers": [{"id": "p1", "name": "Steam Deck", "online": True, "sync": {"at": "2026-10-24T09:24:00"}},
                  {"id": "p2", "name": "Office desktop", "online": False}],
        "transfers": [{"id": "t1", "title": "Starfall Drift", "from": "Office desktop", "state": "copying", "done": 21.4e9, "total": 38.2e9, "speed": 248e6, "eta": 68},
                      {"id": "t2", "title": "Lantern Keep", "from": "Steam Deck", "state": "done", "skipped": 6.1e9, "note": "Only the changed parts were copied"}],
        "settings": {"sync": True, "alwaysAccept": True, "relay": "default"}}
# name, what to run in the page, wait (ms), crop box in the 1920x1200 screenshot (or None), width to save at
SHOTS = [
    ('gm-home', "navigate('home',{},false)", 1200, None, 1600),
    ('gm-library', "navigate('library',{},false)", 1200, None, 1600),
    ('gm-store-steam', "SHOP.store='steam'; navigate('shop',{},false)", 2500, None, 1600),
    ('gm-store', "SHOP.store='gog'; navigate('shop',{},false)", 2500, None, 1600),
    ('gm-downloads', "navigate('downloads',{},false)", 1500, (0, 0, 1150, 700), 1150),
    ('gm-link', "S.settingsTab='link'; navigate('settings',{},false)", 2000, None, 1600),
]


def save(png, name, crop, width):
    im = Image.open(png).convert('RGB')
    if crop: im = im.crop(crop)
    if im.width > width: im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    im.save(os.path.join(IMG, name + '.webp'), 'WEBP', quality=80, method=6)
    print(name, im.size)


async def main():
    tmp = os.path.join(ROOT, 'site-shots-tmp'); os.makedirs(tmp, exist_ok=True)
    async with async_playwright() as p:
        b = await p.chromium.launch()
        pg = await b.new_page(viewport={'width': 1600, 'height': 1000}, device_scale_factor=1.2)
        errs = []; pg.on('pageerror', lambda e: errs.append(str(e)))
        await pg.goto('file://' + os.path.abspath(os.path.join(ROOT, 'ui', 'index.html')) + '?preview=linux')
        await pg.wait_for_timeout(2500)
        await pg.evaluate("HOSTINFO.features=['quickResume','friends','screenshots','profiles','hearthLink']; try{localStorage.clear()}catch(e){}; "
                          "window.call=(n,a,t)=>n==='link.status'?Promise.resolve(%s):Promise.reject(new Error('preview')); 0" % json.dumps(LINK))
        for name, js, wait, crop, width in SHOTS:
            await pg.evaluate('closeAll(); ' + js + '; 0')
            await pg.wait_for_timeout(wait)
            png = os.path.join(tmp, name + '.png')
            await pg.screenshot(path=png)
            save(png, name, crop, width)
        await b.close()
    if errs: print('page errors:', errs[:5], file=sys.stderr)

# The other screenshots (made by CI on real systems; cropped to the window and saved as WebP the same way):
#   friends-chat   ci/desktop-shots-friends-matrix: friends-2-chat.png, crop (420,160,1460,880)
#   osk-wheel      ci/desktop-shots-bughunt: osk-4-ember-wheel.png, crop (0,600,1920,1080)
#   osk-split      ci/desktop-shots-bughunt: osk-5-split-daylight.png, crop (0,600,1920,1080)
#   hearth-menu    ci/desktop-shots-bughunt: 3-hearth-menu.png, crop (560,160,1360,920)
#   mk-desk        ci/layout-shots-bughunt: n25/home-mk-desk-1920x1080.jpg
#   lib-flow       ci/layout-shots-bughunt: n25/library-x-lib-flow-1920x1080.jpg
#   app-text       ci/hearth-text-shots-main: app/4-python-colors.png, crop (0,0,1040,718)
#   app-paint      ci/apps-create-shots: paint-drawn.png
if __name__ == '__main__':
    asyncio.run(main())
