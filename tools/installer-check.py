#!/usr/bin/env python3
"""The USB installer's page (ui/installer.html) in a plain browser, against its pretend PC (MOCK):

- the Welcome screen shows the Controllers card, looks for controllers, and the pretend Xbox controller pairs by itself
  and shows as Connected;
- leaving Welcome (Let's go) stops the search (bt.autoPads off), coming back starts it again;
- a PC without Bluetooth (?nobt) says to plug in a controller with a cable, and Let's go still works;
- a controller that doesn't connect (?padfail) shows Couldn't connect with Try again, which looks again;
- no script errors.

The look picker (the same eight looks as Mantel, from ui/installer-looks.js, made from palettes.json):
- ui/installer-looks.js matches palettes.json, and the page's built-in look is Mantel's Hearth, dark;
- every screen shows in the pretend PC with no script errors (also on a sideways 800x1280 screen);
- the picker opens with X and with the Look chip, the D-pad moves through it, A picks, B closes;
- picking Synthwave recolours the page (its accent from palettes.json), Light switches to the light colours;
- the picked look goes along in the install request, and nothing goes along when no look was picked;
- with the looks file missing or broken the page stays in its own look and the install still goes ahead.

  pip install playwright && python3 -m playwright install chromium && python3 tools/installer-check.py [screenshot.png] [--shots DIR]
"""
import json
import os
import subprocess
import sys
import time
from playwright.sync_api import sync_playwright

PAGE = 'file://' + os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'ui', 'installer.html'))
ARGS = [a for a in sys.argv[1:]]
SHOTS = None
if '--shots' in ARGS:
    i = ARGS.index('--shots'); SHOTS = ARGS[i + 1]; del ARGS[i:i + 2]
    os.makedirs(SHOTS, exist_ok=True)
SHOT = ARGS[0] if ARGS else 'installer-welcome.png'
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
PAL = json.load(open(os.path.join(ROOT, 'hosts/linux/desktop/theme/palettes.json')))
fails = []


def check(ok, what):
    if not ok:
        fails.append(what)
        print(f'::error title=Installer check::{what}')
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

    # ---- the looks
    r = subprocess.run([sys.executable, os.path.join(ROOT, 'tools', 'installer-looks.py'), '--check'], capture_output=True, text=True)
    check(r.returncode == 0, 'ui/installer-looks.js matches palettes.json (' + (r.stdout or r.stderr).strip() + ')')

    def shot(page, name):
        if SHOTS:
            page.wait_for_timeout(350)
            page.screenshot(path=os.path.join(SHOTS, f'instlook-{name}.png'))

    def var(page, name):
        return page.evaluate(f"getComputedStyle(document.documentElement).getPropertyValue('{name}').trim()")

    def fkey(page):
        return page.evaluate("(document.querySelector('.f') || {dataset: {}}).dataset.k || ''")

    def walk(page, tag):
        """Every screen in order, as a person goes through them; returns the plan the page sent."""
        seen = []
        def at(step):
            ok = wait_for(page, f"S.step === '{step}'", 6)
            seen.append(step if ok else f'!{step}')
            return ok
        page.click('[data-k="next"]'); at('wifi'); shot(page, f'{tag}2-internet')
        page.click('[data-k="next"]'); at('drive')
        wait_for(page, "!!document.querySelector('#drives .dcard')", 6)
        page.keyboard.press('ArrowRight'); page.wait_for_timeout(250)
        check(page.evaluate("S.diskCur") == '/dev/sda', f'{tag}: ◀ ▶ browse the drive cards')
        shot(page, f'{tag}3-drive')
        page.click('[data-k="use"]'); at('confirm'); shot(page, f'{tag}4-erase')
        page.evaluate("act(document.querySelector('[data-k=erase]'))"); at('you')
        for k, v in (('full', 'Alex Smith'), ('pw', 'pw'), ('pw2', 'pw')):
            page.fill(f'[data-field="{k}"]', v)
        shot(page, f'{tag}5-you')
        page.click('[data-k="next"]'); at('time'); shot(page, f'{tag}6-time')
        page.click('[data-k="tzmore"]'); page.wait_for_timeout(200)
        check(page.evaluate("document.querySelectorAll('.zones .row').length") > 3, f'{tag}: the full time zone list is still there')
        page.click('[data-k="tzmore"]'); page.wait_for_timeout(200)
        page.click('[data-k="next"]'); at('review'); shot(page, f'{tag}7-review')
        page.click('[data-k="install"]'); at('install')
        page.wait_for_timeout(700)
        check(page.evaluate("document.querySelectorAll('#ip-list .done, #ip-list .now').length") >= 1, f'{tag}: the install checklist follows the stages')
        shot(page, f'{tag}8-installing')
        at_done = wait_for(page, "S.step === 'done'", 12); seen.append('done' if at_done else '!done')
        shot(page, f'{tag}9-done')
        check(all(not x.startswith('!') for x in seen), f'{tag}: every screen shows ({" > ".join(seen)})')
        st = page.evaluate("(MOCK.log.filter(x => x[0] === 'osinstall.start').pop() || [0, {}])[1]")
        return (st or {}).get('plan') or {}

    hearth = PAL['hearth']; synth = PAL['synthwave']
    page, errs = open_page('?fast')
    check(page.evaluate("S.step") == 'welcome', 'looks: the installer opens on Welcome')
    check(var(page, '--accent') == hearth['dark']['ember'] and var(page, '--text') == hearth['dark']['text'], "the page's own look is Hearth, dark (palettes.json)")
    same = page.evaluate("""(() => { const v = lookVars('hearth', 'dark'), cs = getComputedStyle(document.documentElement), probe = document.createElement('i');
        document.body.appendChild(probe);
        const col = c => { probe.style.color = ''; probe.style.color = c; return getComputedStyle(probe).color; };
        const bad = Object.keys(v).filter(k => k !== '--font' && (k === '--r' ? cs.getPropertyValue(k).trim() !== v[k] : col(cs.getPropertyValue(k).trim()) !== col(v[k])));
        probe.remove(); return bad; })()""")
    check(not same, "the built-in look (:root) is exactly lookVars('hearth', 'dark'): " + ', '.join(same or []))
    check(page.evaluate("!!document.querySelector('[data-k=look]') && /Look: Hearth/.test(document.querySelector('[data-k=look]').textContent)"), 'Welcome shows the Look: Hearth chip')
    page.wait_for_timeout(3600)
    shot(page, 'welcome')
    # X opens the picker, the D-pad moves through it, B closes
    page.evaluate("padPress('x', true)"); page.wait_for_timeout(300)
    check(page.evaluate("PICK.on && document.querySelectorAll('#picker .lk').length") == 8, 'X opens the picker with the eight looks')
    check(fkey(page) == 'lk-hearth', 'the picker opens on the look in use')
    shot(page, 'picker')
    page.evaluate("padPress('right', true)"); page.wait_for_timeout(150)
    check(fkey(page) == 'lk-steam', 'the D-pad moves through the looks')
    page.evaluate("padPress('down', true)"); page.wait_for_timeout(150)
    check(fkey(page).startswith('lk-') and fkey(page) != 'lk-steam', 'down goes to the next row of looks')
    page.evaluate("padPress('b', true)"); page.wait_for_timeout(250)
    check(page.evaluate("!PICK.on") and fkey(page) == 'look', 'B closes the picker, back on the Look chip')
    check(page.evaluate("S.lookPicked || S.modePicked") is False and var(page, '--accent') == hearth['dark']['ember'], 'opening and closing the picker changes nothing')
    # the Look chip opens it too; A on Synthwave picks it
    page.click('[data-k="look"]'); page.wait_for_timeout(300)
    check(page.evaluate("PICK.on"), 'the Look chip opens the picker')
    page.evaluate("focus(document.querySelector('#picker [data-k=lk-synthwave]'))")
    page.evaluate("padPress('a', true)"); page.wait_for_timeout(300)
    check(page.evaluate("!PICK.on && S.look") == 'synthwave', 'A picks Synthwave and closes the picker')
    check(var(page, '--accent') == synth['dark']['ember'] and var(page, '--bg') == synth['dark']['bg'], 'Synthwave recolours the page (--accent ' + var(page, '--accent') + ')')
    check(page.evaluate("document.documentElement.classList.contains('frame-neon') && document.documentElement.classList.contains('scanlines') && !!document.querySelector('#deco .floor')"), "Synthwave adds its grid and scanlines")
    check(page.evaluate("/Look: Synthwave/.test(document.querySelector('[data-k=look]').textContent)"), 'the chip says Look: Synthwave')
    plan = walk(page, 'synthwave-')
    check(plan.get('look') == {'name': 'synthwave'}, 'the picked look goes along in the install request: ' + json.dumps(plan.get('look')))
    check(not errs, 'no script errors (looks, every screen): ' + ' | '.join(errs)[:600])
    page.close()

    # Light, and no look picked at all
    page, errs = open_page('?fast')
    page.click('[data-k="mode"]'); page.wait_for_timeout(300)
    check(var(page, '--text') == hearth['light']['text'] and var(page, '--bg') == hearth['light']['bg'] and page.evaluate("document.documentElement.classList.contains('mode-light')"), 'Light switches to the light colours')
    check(page.evaluate("/Light/.test(document.querySelector('[data-k=mode]').textContent)"), 'the chip says Light')
    page.wait_for_timeout(3600)
    shot(page, 'welcome-light')
    plan = walk(page, 'light-')
    check(plan.get('look') == {'mode': 'light'}, 'only light goes along when only light was picked: ' + json.dumps(plan.get('look')))
    check(not errs, 'no script errors (light): ' + ' | '.join(errs)[:600])
    page.close()

    page, errs = open_page('?fast')
    plan = walk(page, 'default-')
    check('look' not in plan and plan.get('disk') == '/dev/sda', 'nothing about the look goes along when none was picked')
    check(not errs, 'no script errors (no look picked): ' + ' | '.join(errs)[:600])
    page.close()

    # The install stopping still shows its screen, in the picked look
    page, errs = open_page('?fast&fail')
    page.evaluate("pickLook('gaming')")
    walk_ok = True
    try:
        page.click('[data-k="next"]'); wait_for(page, "S.step === 'wifi'"); page.click('[data-k="next"]'); wait_for(page, "!!document.querySelector('#drives .dcard')")
        page.click('[data-k="use"]'); page.evaluate("act(document.querySelector('[data-k=erase]'))")
        for k, v in (('full', 'Alex'), ('pw', 'pw'), ('pw2', 'pw')):
            page.fill(f'[data-field="{k}"]', v)
        page.click('[data-k="next"]'); page.click('[data-k="next"]'); page.click('[data-k="install"]')
    except Exception as e:  # noqa: BLE001
        walk_ok = False; print('fail walk:', e)
    check(walk_ok and wait_for(page, "S.step === 'failed'", 10) and page.evaluate("/install stopped/.test(document.body.textContent)"), 'a stopped install shows The install stopped')
    shot(page, 'failed-gaming')
    check(not errs, 'no script errors (install stopped): ' + ' | '.join(errs)[:600])
    page.close()

    # Every look draws every screen; sideways screens too
    for size in ({'width': 1280, 'height': 800}, {'width': 800, 'height': 1280}):
        pg = browser.new_page(viewport=size)
        errs2 = []
        pg.on('pageerror', lambda e: errs2.append(str(e)))
        pg.goto(PAGE + '?fast'); pg.wait_for_timeout(2300)
        bad = pg.evaluate("""(() => { const out = []; S.disks = S.disks || []; S.full = 'Alex'; S.user = 'alex'; S.host = 'alex-hearth'; S.disk = (S.disks[1] || {}).dev;
          for (const id of lookOrder()) for (const m of ['dark', 'light']) { S.look = id; S.mode = m; if (!applyLook()) out.push(id + '/' + m);
            for (const st of ['welcome', 'wifi', 'drive', 'confirm', 'you', 'time', 'review', 'install', 'done', 'failed']) { S.step = st; try { render(); } catch (e) { out.push(id + '/' + m + '/' + st + ': ' + e.message); } } }
          S.step = 'welcome'; S.look = 'hearth'; S.mode = 'dark'; applyLook(); render(); return out; })()""")
        check(not bad and not errs2, f"every look draws every screen at {size['width']}x{size['height']}: " + ', '.join(bad + errs2)[:600])
        if SHOTS and size['width'] == 800:
            shot(pg, 'portrait-welcome')
        pg.close()

    # A missing or broken looks file: the page's own look, and the install still goes ahead
    for what, script in (('missing', "Object.defineProperty(window, 'HEARTH_LOOKS', {get() { return undefined; }, set() {}});"),
                         ('broken', "Object.defineProperty(window, 'HEARTH_LOOKS', {get() { return {hearth: {label: 'Hearth', dark: {bg: 'red'}}, synthwave: {dark: {}}}; }, set() {}});")):
        pg = browser.new_page(viewport={'width': 1280, 'height': 800})
        errs2 = []
        pg.on('pageerror', lambda e: errs2.append(str(e)))
        pg.add_init_script(script)
        pg.goto(PAGE + '?fast'); pg.wait_for_timeout(2300)
        if what == 'broken':
            pg.evaluate("pickerOpen(); pickLook('synthwave')"); pg.wait_for_timeout(200)
        check(var(pg, '--accent') == hearth['dark']['ember'] and pg.evaluate("!S.lookPicked"), f'a {what} looks file leaves the page in its own look')
        p2 = walk(pg, f'{what}-')
        check('look' not in p2 and p2.get('disk'), f'with a {what} looks file the install still goes ahead, without a look')
        check(not errs2, f'no script errors ({what} looks file): ' + ' | '.join(errs2)[:600])
        pg.close()
    browser.close()

if fails:
    sys.exit(1)
print('::notice title=Installer check::all checks passed')
