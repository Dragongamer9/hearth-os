#!/usr/bin/env python3
"""The USB installer's page (ui/installer.html) in a plain browser, against its pretend PC (MOCK):

- the Welcome screen shows the Controllers card, looks for controllers, and the pretend Xbox controller pairs by itself
  and shows as Connected;
- leaving Welcome (Let's go) stops the search (bt.autoPads off), coming back starts it again;
- a PC without Bluetooth (?nobt) says to plug in a controller with a cable, and Let's go still works;
- a controller that doesn't connect (?padfail) shows Couldn't connect with Try again, which looks again;
- no script errors.

  pip install playwright && python3 -m playwright install chromium && python3 tools/installer-check.py [screenshot.png]
"""
import os
import sys
import time
from playwright.sync_api import sync_playwright

PAGE = 'file://' + os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'ui', 'installer.html'))
SHOT = sys.argv[1] if len(sys.argv) > 1 else 'installer-welcome.png'
fails = []


def check(ok, what):
    if not ok:
        fails.append(what)
        print(f'::error title=Installer controllers::{what}')
    else:
        print(f'ok: {what}')


def wait_for(page, js, secs=8.0):
    end = time.time() + secs
    while time.time() < end:
        if page.evaluate(js):
            return True
        page.wait_for_timeout(200)
    return False


def pads_text(page):
    return page.evaluate("(document.getElementById('pads') || {}).textContent || ''")


def auto_calls(page):
    return page.evaluate("MOCK.log.filter(x => x[0] === 'bt.autoPads').map(x => !!(x[1] && x[1].on))")


with sync_playwright() as p:
    browser = p.chromium.launch()

    def open_page(query=''):
        pg = browser.new_page(viewport={'width': 1280, 'height': 800})
        errs = []
        pg.on('pageerror', lambda e: errs.append(str(e)))
        pg.on('console', lambda m: errs.append('console: ' + m.text) if m.type == 'error' and 'Failed to load resource' not in m.text else None)
        pg.goto(PAGE + query)
        pg.wait_for_timeout(2300)  # the splash
        return pg, errs

    # Welcome: the card, the search, the pretend Xbox controller pairing by itself.
    page, errs = open_page()
    check(page.evaluate("S.step") == 'welcome', 'the installer opens on Welcome')
    t = pads_text(page)
    check('Controllers' in t, 'Welcome shows the Controllers card')
    check('Looking for controllers' in t, 'it says it is looking for controllers')
    check('pairing mode' in t and 'Share + PS' in t, 'it shows the pairing-mode tip')
    check(auto_calls(page)[-1:] == [True], 'Welcome starts pairing Bluetooth controllers (bt.autoPads on)')
    check(wait_for(page, "/Pairing…/.test(document.getElementById('pads').textContent)"), 'the found controller shows Pairing…')
    check(wait_for(page, "/Xbox Wireless Controller\\s*Connected/.test(document.getElementById('pads').textContent)"), 'the Xbox controller shows as Connected')
    check(page.evaluate("document.querySelector('.f') && document.querySelector('.f').dataset.k") == 'next', "Let's go keeps the focus while controllers come and go")
    page.wait_for_timeout(3500)  # let the toast go for the picture
    page.screenshot(path=SHOT)
    print(f'::notice title=Installer controllers::screenshot {SHOT}')
    # Leaving Welcome stops the search; coming back starts it again.
    page.click('[data-k="next"]')
    page.wait_for_timeout(500)
    check(page.evaluate("S.step") == 'wifi', "Let's go goes on to Wi-Fi")
    check(auto_calls(page)[-1:] == [False], 'leaving Welcome stops the search (bt.autoPads off)')
    check(page.evaluate("PADS.bt") == 'idle' and page.evaluate("PADS.poll") is None, 'nothing is looking for controllers after Welcome')
    check(page.evaluate("!document.getElementById('pads')"), 'the Controllers card is only on Welcome')
    page.keyboard.press('Escape')
    page.wait_for_timeout(800)
    check(page.evaluate("S.step") == 'welcome' and auto_calls(page)[-1:] == [True], 'back on Welcome the search starts again')
    check(not errs, 'no script errors: ' + ' | '.join(errs)[:600])
    page.close()

    # No Bluetooth: says so, and nothing is blocked.
    page, errs = open_page('?nobt')
    check(wait_for(page, "/No Bluetooth on this PC: plug in a controller with a cable/.test(document.getElementById('pads').textContent)", 4), 'no Bluetooth says to plug in a controller with a cable')
    check(True not in auto_calls(page), 'without Bluetooth nothing tries to pair')
    page.click('[data-k="next"]')
    page.wait_for_timeout(500)
    check(page.evaluate("S.step") == 'wifi', "without Bluetooth Let's go still works")
    check(not errs, 'no script errors (no Bluetooth): ' + ' | '.join(errs)[:600])
    page.close()

    # A controller that doesn't connect: Couldn't connect, Try again looks again and it connects.
    page, errs = open_page('?padfail')
    check(wait_for(page, "/Couldn't connect/.test(document.getElementById('pads').textContent) && !!document.querySelector('#pads [data-act=\"act-padretry\"]')"), "a controller that didn't connect shows Couldn't connect and Try again")
    n = len(auto_calls(page))
    page.click('#pads [data-act="act-padretry"]')
    page.wait_for_timeout(600)
    check(auto_calls(page)[n:] == [False, True], 'Try again starts the search over')
    check(wait_for(page, "/Xbox Wireless Controller\\s*Connected/.test(document.getElementById('pads').textContent)"), 'after Try again the controller connects')
    check(not errs, 'no script errors (Try again): ' + ' | '.join(errs)[:600])
    page.close()
    browser.close()

if fails:
    sys.exit(1)
print('::notice title=Installer controllers::all checks passed')
