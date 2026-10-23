#!/usr/bin/env python3
"""Hearth Calculator's page in Chromium (Playwright), without the app: typing sums with the keyboard and the
buttons, each mode, memory, history, copy, and moving around like a controller. Screenshots go to the folder given
as the first argument.

    python3 tests/ui_test.py shots
"""
import pathlib
import sys

from playwright.sync_api import sync_playwright

HERE = pathlib.Path(__file__).resolve().parent.parent
SHOTS = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "shots")
SHOTS.mkdir(parents=True, exist_ok=True)
errors = []


def check(cond, what):
    if not cond:
        raise AssertionError(what)
    print("ok:", what)


with sync_playwright() as p:
    b = p.chromium.launch()
    pg = b.new_page(viewport={"width": 900, "height": 680})
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.goto((HERE / "ui" / "index.html").as_uri())
    pg.wait_for_function("document.body.dataset.ready === '1'")
    entry = lambda: pg.inner_text("#entry")
    expr = lambda: pg.inner_text("#expr")

    def keys(s):
        for ch in s:
            pg.keyboard.press({"\n": "Enter", "\b": "Backspace", "\x1b": "Escape"}.get(ch, ch))

    def btn(*ids):
        for i in ids:
            pg.click(f'#pad [data-key="{i}"]')

    keys("0.1+0.2\n")
    check(entry() == "0.3", "0.1 + 0.2 = 0.3")
    check(expr() == "0.1 + 0.2 =", f"the sum shows above the answer ({expr()})")
    keys("\x1b2+3*4\n")
    check(entry() == "20", "Standard works left to right: 2 + 3 × 4 = 20")
    keys("\x1b5+3\n\n")
    check(entry() == "11", "pressing = again repeats the last step")
    keys("\x1b1/3\n*3\n")
    check(entry() == "1", "1 ÷ 3 × 3 = 1")
    keys("\x1b200+10%")
    check(entry() == "20", "200 + 10% takes 10% of 200")
    keys("\n")
    check(entry() == "220", "… and adds it: 220")
    keys("\x1b123456789*1000\n")
    check(entry() == "123,456,789,000", "big answers are grouped in thousands")
    keys("\x1b1/0\n")
    check("divide by zero" in entry(), "dividing by zero says so")
    keys("\x1b9")
    btn("sqrt")
    check(entry() == "3", "√9 = 3")
    keys("\x1b12")
    pg.keyboard.press("Control+m")
    keys("\x1b8")
    pg.keyboard.press("Control+p")
    keys("\x1b")
    pg.keyboard.press("Control+r")
    check(entry() == "20", "MS, M+ and MR work like Windows' (12 + 8 = 20)")
    pg.keyboard.press("Control+c")
    check(pg.evaluate("window.__clip") == "20", "Ctrl+C copies the number")
    pg.evaluate("window.__clip = '1,234.5'")
    pg.keyboard.press("Control+v")
    check(entry() == "1,234.5", "Ctrl+V pastes a number")
    pg.screenshot(path=str(SHOTS / "standard.png"))

    pg.click("[data-mode=scientific]")
    keys("\x1b2+3*4\n")
    check(entry() == "14", "Scientific respects precedence: 2 + 3 × 4 = 14")
    keys("\x1b(2+3)*4\n")
    check(entry() == "20", "brackets: (2 + 3) × 4 = 20")
    keys("\x1b30s")
    check(entry() == "0.5", "sin 30° = 0.5")
    keys("\x1b2^10\n")
    check(entry() == "1,024", "2 ^ 10 = 1024")
    keys("\x1b5!")
    check(entry() == "120", "5! = 120")
    btn("2nd")
    keys("\x1b27")
    btn("sqrt")
    check(entry() == "3", "2nd turns √ into ∛: ∛27 = 3")
    pg.screenshot(path=str(SHOTS / "scientific.png"))

    pg.click("[data-mode=programmer]")
    pg.click("[data-base='16']")
    keys("\x1bff&0f\n")
    check(entry() == "F", "HEX: FF AND 0F = F")
    check(pg.inner_text("#bases").find("1111") >= 0, "the binary row shows the bits")
    check(pg.is_disabled('#pad [data-key="F"]') is False, "hex digits work in HEX")
    pg.click("[data-base='10']")
    check(pg.is_disabled('#pad [data-key="F"]'), "hex digits are greyed out in DEC")
    for _ in range(3):
        pg.click("#wordBtn")
    check(pg.inner_text("#wordBtn") == "BYTE", "the word size goes down to BYTE")
    keys("\x1b0")
    btn("NOT")
    check(entry() == "-1", "NOT 0 in a byte is -1 (two's complement)")
    keys("\x1b1<4\n")
    check(entry() == "16", "1 << 4 = 16")
    pg.screenshot(path=str(SHOTS / "programmer.png"))

    pg.click("[data-mode=convert]")
    keys("12")
    check(pg.inner_text("#convOut") == "30.48", "12 inches is 30.48 cm")
    pg.click("[data-pick=cat]")
    pg.click(".picker [data-id='Temperature']")
    keys("\b\b100")
    check(pg.inner_text("#convOut") == "212", "100 °C is 212 °F")
    pg.screenshot(path=str(SHOTS / "convert.png"))

    pg.click("[data-mode=standard]")
    hist = pg.evaluate("st.history.length")
    check(hist >= 8, f"sums are kept in the history ({hist})")
    pg.click("[data-hist='0']")
    check(entry() != "", "picking a history line brings its answer back")
    pg.wait_for_timeout(600)
    stored = pg.evaluate("window.__demoStore.calc")
    check(stored and len(stored["history"]) == hist, "history and memory are saved")

    # Moving around with arrows (as a controller does).
    pg.evaluate("document.activeElement.blur()")
    pg.keyboard.press("ArrowDown")
    check(pg.evaluate("document.activeElement.dataset.key") == "5", "the first move lands on 5")
    pg.keyboard.press("ArrowUp")
    check(pg.evaluate("document.activeElement.dataset.key") == "8", "up from 5 is 8")
    pg.set_viewport_size({"width": 420, "height": 680})
    pg.screenshot(path=str(SHOTS / "narrow.png"))
    b.close()

if errors:
    print("page errors:", errors)
    sys.exit(1)
print("all good")
