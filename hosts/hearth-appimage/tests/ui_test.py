#!/usr/bin/env python3
"""AppImage Maker's page in Chromium (Playwright), without the app (app.js's pretend conversions): packages added,
converted one at a time with their steps, a failure with Try again, the options saved, My AppImages with its
buttons and the delete question, and moving around with the arrow keys like a controller. Screenshots go to the
folder given as the first argument.

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
    pg = b.new_page(viewport={"width": 1100, "height": 720})
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.add_init_script("window.__demoStart = ['/home/you/Downloads/discord-0.0.90.deb', '/home/you/Downloads/broken-thing.rpm', '/home/you/Downloads/obsidian-1.7.7.tar.gz', '/home/you/Downloads/notes.txt']")
    pg.goto((HERE / "ui" / "index.html").as_uri())
    pg.wait_for_function("document.body.dataset.ready === '1'")
    check(pg.locator(".job").count() == 3, "three packages queued (the .txt refused)")
    pg.wait_for_timeout(60)
    pg.screenshot(path=str(SHOTS / "1-converting.png"))
    pg.wait_for_function("document.querySelectorAll('.job .state.done, .job .state.fail').length === 3", timeout=10000)
    check(pg.locator(".job .state.done").count() == 2, "two made")
    check(pg.locator(".job .state.fail").count() == 1, "the broken one failed")
    check("couldn't find a program" in pg.inner_text(".job:has(.state.fail) .error"), "the failure says why")
    check(pg.locator(".job:has(.state.done) [data-act=open]").count() == 2, "Open on each made one")
    check(pg.locator(".job .tag.menu").count() == 2, "added to the app menu (the default)")
    pg.click(".job:has(.state.done) [data-act=log] >> nth=0")
    check(pg.locator(".job .log").count() == 1, "details show the log")
    pg.screenshot(path=str(SHOTS / "2-made.png"))

    # Options are saved.
    pg.click("#tgMenu")
    pg.wait_for_timeout(300)
    check(pg.evaluate("window.__demoStore.settings.menu") is False, "turning off Add to the app menu is saved")
    check(pg.evaluate("window.__demoStore.settings.fetch") is True, "fetch stays on")

    # Try again on the failed one.
    pg.click(".job:has(.state.fail) [data-act=retry]")
    pg.wait_for_function("document.querySelectorAll('.job .state.fail').length === 1", timeout=5000)
    check(True, "Try again runs it again")

    # My AppImages.
    pg.click("[data-tab=mine]")
    pg.wait_for_function("document.querySelectorAll('.item').length === 2")
    check("(2)" in pg.inner_text("#mineCount"), "the tab counts them")
    pg.screenshot(path=str(SHOTS / "3-my-appimages.png"))
    pg.click(".item >> nth=0 >> [data-act=unmenu]")
    pg.wait_for_function("document.querySelectorAll('.item .tag.menu').length === 1")
    check(True, "Take out of menu")
    pg.click(".item >> nth=0 >> [data-act=delete]")
    check(pg.is_visible("#confirm"), "Delete asks first")
    pg.screenshot(path=str(SHOTS / "4-delete-question.png"))
    pg.keyboard.press("Escape")
    check(not pg.is_visible("#confirm") and pg.locator(".item").count() == 2, "Escape keeps it")
    pg.click(".item >> nth=0 >> [data-act=delete]")
    pg.click("#confirmYes")
    pg.wait_for_function("document.querySelectorAll('.item').length === 1")
    check(True, "Delete takes it off the list")

    # Arrow keys move like a controller's D-pad.
    pg.click("[data-tab=make]")
    pg.focus("#drop")
    pg.keyboard.press("ArrowRight")
    check(pg.evaluate("document.activeElement.closest('.side') !== null"), "right goes to the options")
    pg.keyboard.press("ArrowLeft")
    pg.keyboard.press("ArrowDown")
    check(pg.evaluate("document.activeElement.closest('.job') !== null"), "down goes into the list")
    pg.evaluate("document.body.classList.add('pad')")
    pg.screenshot(path=str(SHOTS / "5-controller-focus.png"))

    # Small window.
    pg.set_viewport_size({"width": 560, "height": 700})
    pg.wait_for_timeout(100)
    pg.screenshot(path=str(SHOTS / "6-narrow.png"))
    check(not errors, f"no page errors {errors}")
    b.close()
print("all good")
