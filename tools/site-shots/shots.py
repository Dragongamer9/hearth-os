# The website's screenshots (site/img/game-*.webp): Hearth's page (ui/index.html) in its browser preview, with the
# original cover art from art.py put on the preview's invented games. Only this script adds the art (it sets
# game.art in the page after it loads); Hearth itself is unchanged. Real games (Beacon's) are taken out of the
# library, except on the Beacon shot, where they get plain generic landscapes and their names in plain type.
#
#   python3 tools/site-shots/art.py      # once, or after changing the art
#   python3 tools/site-shots/shots.py    # writes site/img/game-*.webp (1280x800)
#
# The website itself now shows tools/site-shots.py's screenshots (real open-source games with their own art); these
# game-*.webp are the README's.
import asyncio, os, sys
from io import BytesIO

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
ART = os.path.join(HERE, 'art')
OUT = os.path.join(ROOT, 'site', 'img')

SETUP = """
([artDir, keepBeacon]) => {
  const art = id => { const n = id.replace(':', '-'); return {cover: `${artDir}/${n}-cover.webp`, hero: `${artDir}/${n}-hero.webp`,
    wide: `${artDir}/${n}-wide.webp`, ...(id.startsWith('beacon:') ? {} : {logo: `${artDir}/${n}-logo.png`})}; };
  for (let i = GAMES.length - 1; i >= 0; i--) {
    const g = GAMES[i];
    if (g.src === 'beacon' && !keepBeacon) GAMES.splice(i, 1); else g.art = art(g.id);
  }
}
"""

# name: (javascript that sets the screen up, keep Beacon's games)
SHOTS = {
    'game-home': ("() => { LOOK.layout='shelf'; saveLook(); S.layers=[]; navigate('home', {}, false); }", False),
    'game-spotlight': ("() => { LOOK.layout='spotlight'; saveLook(); S.layers=[]; navigate('home', {}, false); }", False),
    'game-library': ("() => { LOOK.libLayout='covers'; saveLook(); S.tab='all'; S.layers=[]; navigate('library', {}, false); }", False),
    'game-library-list': ("() => { LOOK.libLayout='list'; saveLook(); S.tab='all'; S.layers=[]; navigate('library', {}, false); }", False),
    'game-library-shelves': ("() => { LOOK.libLayout='shelves'; saveLook(); S.tab='all'; S.layers=[]; navigate('library', {}, false); }", False),
    'game-page': ("() => { S.layers=[]; navigate('game', {game:'g05'}, false); }", False),
    'game-downloads': ("() => { S.layers=[]; navigate('downloads', {}, false); }", False),
    'game-qa': ("() => { LOOK.layout='shelf'; saveLook(); S.layers=[]; navigate('home', {}, false); setTimeout(() => { S.qtab='quick'; openLayer('qam'); }, 300); }", False),
    'game-beacon': ("() => { LOOK.libLayout='covers'; saveLook(); S.tab='beacon'; S.layers=[]; navigate('library', {}, false); }", True),
}


async def main(only=None):
    from playwright.async_api import async_playwright
    from PIL import Image
    async with async_playwright() as p:
        b = await p.chromium.launch()
        for name, (js, beacon) in SHOTS.items():
            if only and name not in only: continue
            ctx = await b.new_context(viewport={'width': 1280, 'height': 800})
            # Hearth's orange accent (one of its own accent choices), so the shots match the site.
            await ctx.add_init_script("try{localStorage.setItem('nimbus.profile', JSON.stringify({accent:'#ff9f43'}));}catch(e){}")
            pg = await ctx.new_page()
            errors = []
            pg.on('pageerror', lambda e: errors.append(str(e)))
            await pg.goto('file://' + os.path.join(ROOT, 'ui', 'index.html'))
            await pg.wait_for_timeout(1200)
            await pg.evaluate(SETUP, ['file://' + ART, beacon])
            await pg.evaluate(js)
            await pg.wait_for_timeout(1500)
            await pg.evaluate("Promise.all([...document.images].map(i => i.complete ? 0 : new Promise(r => { i.onload = i.onerror = r; setTimeout(r, 3000); })))")
            await pg.wait_for_timeout(300)
            im = Image.open(BytesIO(await pg.screenshot())).convert('RGB')
            path = os.path.join(OUT, name + '.webp')
            im.save(path, quality=80, method=6)
            print(f'{name}: {os.path.getsize(path) // 1024} KB', 'ERRORS: ' + '; '.join(errors) if errors else '', flush=True)
            await ctx.close()
        await b.close()


if __name__ == '__main__':
    asyncio.run(main(sys.argv[1:] or None))
