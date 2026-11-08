#!/usr/bin/python3
"""Hearth's own apps next to Mantel's bar: how big is their text? (desktop-test.yml runs this in Desktop Mode.)

Opens real Hearth app pages (Saves, Mods, Disks, Contacts) in WebKitGTK, the engine their Tauri windows use, with
the look put in the way hosts/hearth-apptheme/apptheme.rs puts it (a <style> added before the page's own scripts):
once as Game Mode gets it (the look only) and once as a desktop window gets it (the look and desktop.css). Each at
the apps' own window size (1280x800) and maximized. Measures the page's text size (getComputedStyle on <body>) and
devicePixelRatio, compares it with the bar's text (Bar.qml), prints ::notice lines and takes screenshots with grim.

  app_size_probe.py OUT_DIR       exits 1 when a desktop window's text is more than 15% off the bar's
"""
import json
import os
import pathlib
import re
import subprocess
import sys

import gi
gi.require_version('Gtk', '3.0')
try:
    gi.require_version('WebKit2', '4.1')
except ValueError:
    gi.require_version('WebKit2', '4.0')
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[3]
HOSTS = ROOT / 'hosts'
APPTHEME = (HOSTS / 'hearth-apptheme' / 'apptheme.rs').read_text()
DESKTOP_CSS = (HOSTS / 'hearth-apptheme' / 'desktop.css').read_text()
THEME_CSS = re.search(r'const DEFAULT_CSS: &str = r##"(.*?)"##;', APPTHEME, re.S).group(1)
m = re.search(r'font\.pixelSize: bar\.portrait \? \d+ : (\d+)', (HOSTS / 'linux' / 'desktop' / 'shell' / 'Bar.qml').read_text())
BAR_TEXT = int(m.group(1)) if m else 14       # the bar's main text (the clock), in the same logical pixels

PAGES = [('Saves', HOSTS / 'hearth-gaming' / 'ui' / 'saves.html'),
         ('Mods', HOSTS / 'hearth-gaming' / 'ui' / 'mods.html'),
         ('Disks', HOSTS / 'hearth-disks' / 'ui' / 'index.html'),
         ('Contacts', HOSTS / 'hearth-contacts' / 'ui' / 'index.html')]

MEASURE = ("JSON.stringify({font: parseFloat(getComputedStyle(document.body).fontSize), dpr: window.devicePixelRatio,"
           " w: window.innerWidth, h1: (function(){var e=document.querySelector('h1,h2');"
           " return e ? parseFloat(getComputedStyle(e).fontSize) : 0;})(),"
           " size: getComputedStyle(document.documentElement).getPropertyValue('--hearth-size').trim()})")


def inject(css):
    """apptheme.rs's way: a <style id="hearth-theme"> in the page before its own scripts run."""
    return ("(function(){var css=%s;function put(){var d=document,r=d.documentElement;if(!r)return false;"
            "var s=d.getElementById('hearth-theme');if(!s){s=d.createElement('style');s.id='hearth-theme';"
            "(d.head||r).appendChild(s);}s.textContent=css;return true;}"
            "if(!put()){var o=new MutationObserver(function(){if(put())o.disconnect();});"
            "o.observe(document,{childList:true,subtree:true});}"
            "document.addEventListener('DOMContentLoaded',put);})();" % json.dumps(css))


def measure(name, page, css, maximized, shot):
    """Opens the page in a window; returns what MEASURE says, after taking `shot` (a grim screenshot) if given."""
    result = {}
    ucm = WebKit2.UserContentManager()
    ucm.add_script(WebKit2.UserScript(inject(css), WebKit2.UserContentInjectedFrames.ALL_FRAMES,
                                      WebKit2.UserScriptInjectionTime.START, None, None))
    view = WebKit2.WebView.new_with_user_content_manager(ucm)
    s = view.get_settings()
    s.set_allow_file_access_from_file_urls(True)
    win = Gtk.Window(title='Hearth %s (%s)' % (name, 'desktop' if 'hearth-size' in css else 'Game Mode size'))
    win.set_default_size(1280, 800)
    win.add(view)
    win.show_all()
    if maximized:
        win.maximize()
    loop = GLib.MainLoop()

    def done(obj, res):
        try:
            if hasattr(obj, 'evaluate_javascript_finish'):
                v = obj.evaluate_javascript_finish(res)
            else:
                v = obj.run_javascript_finish(res).get_js_value()
            result.update(json.loads(v.to_string()))
        except Exception as e:  # noqa: BLE001
            result['error'] = str(e)
        loop.quit()

    def ask():
        if hasattr(view, 'evaluate_javascript'):
            view.evaluate_javascript(MEASURE, -1, None, None, None, done)
        else:
            view.run_javascript(MEASURE, None, done)
        return False

    def loaded(_v, ev):
        if ev == WebKit2.LoadEvent.FINISHED:
            GLib.timeout_add(2500, ask)     # the window placed (and maximized) and the page drawn

    view.connect('load-changed', loaded)
    GLib.timeout_add(30000, loop.quit)
    view.load_uri(page.as_uri())
    loop.run()
    if shot:
        subprocess.run(['grim', shot], check=False)
    win.destroy()
    while Gtk.events_pending():
        Gtk.main_iteration()
    return result


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else 'shots/app-size')
    out.mkdir(parents=True, exist_ok=True)
    rows, bad = [], []
    for name, page in PAGES:
        for maximized in (False, True):
            where = 'maximized' if maximized else '1280x800 window'
            tag = '%s-%s' % (name.lower(), 'max' if maximized else 'window')
            game = measure(name, page, THEME_CSS, maximized, None)
            desk = measure(name, page, THEME_CSS + '\n' + DESKTOP_CSS, maximized,
                           str(out / ('%s-desktop.png' % tag)) if name in ('Saves', 'Disks') else None)
            if name == 'Saves' and not maximized:
                # The same window without the fix, as the owner saw it, for the before/after pair.
                measure(name, page, THEME_CSS, maximized, str(out / ('%s-before.png' % tag)))
            row = {'app': name, 'where': where, 'before': game, 'after': desk, 'bar': BAR_TEXT}
            rows.append(row)
            if 'error' in desk or not desk.get('font'):
                bad.append('%s (%s): couldn\'t measure the page (%s)' % (name, where, desk.get('error', 'no size')))
                continue
            ratio_before = (game.get('font') or 0) / BAR_TEXT
            ratio_after = desk['font'] / BAR_TEXT
            print('::notice title=App size %s, %s::bar text %d px; before %.1f px (%.2fx the bar), after %.1f px '
                  '(%.2fx); heading %.1f -> %.1f px; window %s px wide, devicePixelRatio %s; Game Mode keeps %.1f px'
                  % (name, where, BAR_TEXT, game.get('font') or 0, ratio_before, desk['font'], ratio_after,
                     game.get('h1') or 0, desk.get('h1') or 0, desk.get('w'), desk.get('dpr'), game.get('font') or 0))
            if desk.get('size') != 'desktop':
                bad.append('%s (%s): desktop.css wasn\'t applied' % (name, where))
            if abs(ratio_after - 1) > 0.15:
                bad.append('%s (%s): text is %.1f px next to the bar\'s %d px' % (name, where, desk['font'], BAR_TEXT))
            if (game.get('font') or 0) < 15:
                bad.append('%s (%s): Game Mode\'s size shrank to %.1f px' % (name, where, game.get('font') or 0))
    (out / 'sizes.json').write_text(json.dumps(rows, indent=1))
    for b in bad:
        print('::error title=App size::%s' % b)
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
