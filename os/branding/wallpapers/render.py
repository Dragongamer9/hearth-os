# Renders the Hearth wallpapers (wallpapers.html) at 3840x2160. Needs Playwright with Chromium. Then convert to JPEG.
import pathlib
from playwright.sync_api import sync_playwright
d = pathlib.Path(__file__).parent
with sync_playwright() as p:
    b = p.chromium.launch(args=['--allow-file-access-from-files'])
    pg = b.new_page(viewport={'width': 1920, 'height': 1080}, device_scale_factor=2)
    for s in ['fireside', 'embers', 'ridge', 'mark', 'kiln']:
        pg.goto((d / 'walls.html').as_uri() + '?s=' + s)
        pg.wait_for_function("document.title === 'ready'")
        pg.wait_for_timeout(200)
        pg.screenshot(path=str(d / f'hearth-{s}.png'))
        pg.set_viewport_size({'width': 1920, 'height': 1080})
    b.close()
