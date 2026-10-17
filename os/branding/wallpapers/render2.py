# Renders the second set of Hearth wallpapers (walls2.html) at 3840x2160 JPEG, and small previews of the
# moving ones for Hearth Settings. Needs Playwright with Chromium and Pillow.
#   python3 render2.py [names...]   (default: all)    PREVIEW=1 for quick 960x540 contact sheets only
import io, os, pathlib, sys
from PIL import Image
from playwright.sync_api import sync_playwright
d = pathlib.Path(__file__).parent
STILL = ['cat', 'cabin', 'lanterns', 'bloom', 'peaks', 'forge', 'topo', 'fox']
MOVING = ['lava', 'aurora']
ANIME = []
names = sys.argv[1:] or STILL + MOVING + ANIME
shell = d.parent.parent.parent / 'hosts' / 'linux' / 'desktop' / 'shell'
quick = os.environ.get('PREVIEW') == '1'
exe = '/opt/pw-browsers/chromium-1194/chrome-linux/chrome'
with sync_playwright() as p:
    b = p.chromium.launch(executable_path=exe if os.path.exists(exe) else None)
    pg = b.new_page(viewport={'width': 1920, 'height': 1080}, device_scale_factor=1 if quick else 2)
    for s in names:
        pg.goto((d / ('walls-anime.html' if s in ANIME else 'walls2.html')).as_uri() + '?s=' + s)
        pg.wait_for_function("document.title === 'ready'")
        pg.wait_for_timeout(300)
        img = Image.open(io.BytesIO(pg.screenshot(timeout=300000))).convert('RGB')
        if quick:
            img.resize((960, 540), Image.LANCZOS).save(f'/tmp/claude-0/pv/w2-{s}.png')
        elif s in MOVING:
            img.resize((640, 360), Image.LANCZOS).save(shell / f'wall-{s}.jpg', quality=88)
        else:
            img.save(d / f'hearth-{s}.jpg', quality=90, optimize=True, progressive=True)
        print(s)
    b.close()
