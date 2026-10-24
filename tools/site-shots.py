#!/usr/bin/env python3
"""Redraws the website's Game Mode screenshots (site/img/gm-*.webp) from Hearth's real page, ui/index.html, in a
browser with its preview's open sample set (?preview=linux&samples=open): real open-source games (SuperTuxKart, 0 A.D.,
Battle for Wesnoth, OpenTTD, Veloren, Endless Sky) with their own art from site/img/games, shared under CC BY-SA / GPL
(tools/site-shots/open-art.py makes it; credits in site/img/games/CREDITS.md and on the site). Each game is only on
the stores it's really on, at its real price (free). Hearth Link's page needs the Link service, so it's shown with a
sample state (two paired devices, one copy running).
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
        "transfers": [{"id": "t1", "title": "0 A.D.", "from": "Office desktop", "state": "copying", "done": 1.4e9, "total": 3.6e9, "speed": 248e6, "eta": 9},
                      {"id": "t2", "title": "SuperTuxKart", "from": "Steam Deck", "state": "done", "skipped": 0.6e9, "note": "Only the changed parts were copied"}],
        "settings": {"sync": True, "alwaysAccept": True, "relay": "default"}}
# name, what to run in the page, wait (ms), crop box in the 1920x1200 screenshot (or None), width to save at
SHOTS = [
    ('gm-home', "navigate('home',{},false)", 1200, None, 1600),
    ('gm-library', "S.tab='all'; navigate('library',{},false)", 1200, None, 1600),
    ('gm-store-steam', "SHOP.store='steam'; navigate('shop',{},false)", 2500, None, 1600),
    # A game's store page (the Steam section's Battle for Wesnoth), opened the way selecting it does
    ('gm-store', "SHOP.store='steam'; navigate('shop',{},false); setTimeout(()=>{ SHOP.page={store:'steam',id:'599390',item:shopFind('steam','599390')}; "
                 "shopLoadDetails('steam','599390'); render(null,true); }, 1200)", 3000, None, 1600),
    ('gm-downloads', "navigate('downloads',{},false)", 1500, (0, 0, 1150, 700), 1150),
    ('gm-link', "S.settingsTab='link'; navigate('settings',{},false)", 2000, None, 1600),
    ('gm-game', "navigate('game',{game:'o-wesnoth'},false)", 1500, None, 1600),
    # Quick Access over a running game (0 A.D., on its page)
    ('gm-qa', "S.running='o-0ad'; navigate('game',{game:'o-0ad'},false); setTimeout(()=>{ S.qtab='quick'; openLayer('qam'); }, 400)", 1800, None, 1600),
]
# The layout shots, at 1920x1080 like the layout audit's (tools/layout-audit.js): Home in Desk grid, the Library in
# Cover Flow. Same fields as above.
LAYOUT_SHOTS = [
    ('mk-desk', "S.running=null; S.tab='all'; LOOK.layout='mk-desk'; saveLook(); navigate('home',{},false)", 1500, None, 1600),
    ('lib-flow', "S.running=null; S.tab='all'; LOOK.layout='shelf'; LOOK.libLayout='x-lib-flow'; saveLook(); navigate('library',{},false)", 1500, None, 1600),
]

# Run once a shot's screen is up (Cover Flow: the third game selected, as if moved to with the D-pad)
AFTER = {'lib-flow': "setFocus(document.querySelectorAll('#view [data-f][data-id]')[2], true)"}


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
        await pg.goto('file://' + os.path.abspath(os.path.join(ROOT, 'ui', 'index.html')) + '?preview=linux&samples=open')
        await pg.wait_for_timeout(2500)
        await pg.evaluate("HOSTINFO.features=['quickResume','friends','screenshots','profiles','hearthLink']; try{localStorage.clear()}catch(e){}; "
                          "window.call=(n,a,t)=>n==='link.status'?Promise.resolve(%s):Promise.reject(new Error('preview')); 0" % json.dumps(LINK))
        for name, js, wait, crop, width in SHOTS + LAYOUT_SHOTS:
            if name == LAYOUT_SHOTS[0][0]: await pg.set_viewport_size({'width': 1600, 'height': 900})
            await pg.evaluate('closeAll(); ' + js + '; 0')
            await pg.wait_for_timeout(wait)
            if name in AFTER:
                await pg.evaluate(AFTER[name] + '; 0'); await pg.wait_for_timeout(900)
            # Lazy images the page hasn't asked for yet (it decides by scrolling): load them all, wait until they're in
            await pg.evaluate("document.querySelectorAll('img[loading=lazy]').forEach(i=>i.loading='eager'); 0")
            await pg.wait_for_function("[...document.images].every(i=>i.complete)", timeout=10000)
            await pg.wait_for_timeout(300)
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
#   app-text       ci/hearth-text-shots-main: app/4-python-colors.png, crop (0,0,1040,718)
#   app-paint      ci/apps-create-shots: paint-drawn.png
if __name__ == '__main__':
    asyncio.run(main())
