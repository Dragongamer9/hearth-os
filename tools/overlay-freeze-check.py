#!/usr/bin/env python3
"""Quick Access over a Steam game must never freeze the game (CI: rust.yml).

On the Handheld Daemon editions HHD stops Steam (SIGSTOP) while its overlay app (Hearth's Quick Access) holds the
controllers ("Froze Steam (to avoid HID device dual input)"), and a Steam game froze with it; HHD's SIGCONT never
comes if its overlay thread ends meanwhile. Hearth lets Steam run again (hosts/hearth-core/src/steam_thaw.rs) and
holds Steam's virtual controllers instead (guide.rs hold_steam_pads), wired up in hosts/hearth-rs/src/gamemode.rs.
This checks that wiring stays in place, and (warn-only, needs the network) that HHD still stops Steam the way
steam_thaw.rs expects: the first process whose first or second argument ends in "/steam".

Exit 1 with ::error annotations when the wiring is broken.
"""
import os
import re
import sys
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HHD_PLUGIN = "https://raw.githubusercontent.com/hhd-dev/hhd/master/src/hhd/plugins/plugin.py"
HHD_CONTROLLERS = "https://raw.githubusercontent.com/hhd-dev/hhd/master/src/hhd/plugins/overlay/controllers.py"

errors = []


def read(rel):
    try:
        with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
            return f.read()
    except OSError as e:
        errors.append((rel, 1, f"can't read it: {e}"))
        return ""


def line_of(text, needle):
    i = text.find(needle)
    return text.count("\n", 0, i) + 1 if i >= 0 else 1


def fn_body(text, name):
    """The body of `fn name` (up to the next fn at the same indentation), or ''."""
    m = re.search(r"^( *)(pub )?fn " + re.escape(name) + r"\b", text, re.M)
    if not m:
        return ""
    indent = m.group(1)
    nxt = re.compile(r"^" + indent + r"(pub )?fn |^" + indent + r"/// ", re.M)
    rest = text[m.end():]
    n = nxt.search(rest)
    return rest[: n.start()] if n else rest


def need(rel, text, body, needle, why, where=None):
    if needle not in body:
        errors.append((rel, line_of(text, where or needle), why))


GM = "hosts/hearth-rs/src/gamemode.rs"
GUIDE = "hosts/hearth-core/src/guide.rs"
THAW = "hosts/hearth-core/src/steam_thaw.rs"
LIB = "hosts/hearth-core/src/lib.rs"

gm, guide, thaw, lib = read(GM), read(GUIDE), read(THAW), read(LIB)

need(LIB, lib, lib, "pub mod steam_thaw;", "hearth-core no longer has steam_thaw: Steam stopped by Handheld Daemon is never let go, and Steam games freeze under Quick Access")

hold = fn_body(gm, "hold_game")
if not hold:
    errors.append((GM, 1, "fn hold_game is gone: Quick Access over a game no longer keeps Steam running on the HHD editions"))
else:
    need(GM, gm, hold, "steam_while_open(on)", "hold_game doesn't call steam_while_open on the HHD editions: HHD's SIGSTOP of Steam freezes the Steam game while Quick Access is open", "fn hold_game")

sw = fn_body(gm, "steam_while_open")
if not sw:
    errors.append((GM, 1, "fn steam_while_open is gone: Steam stopped by Handheld Daemon freezes Steam games under Quick Access"))
else:
    for needle, why in [
        ("hold_steam_pads(true)", "Steam's virtual controllers aren't held while Quick Access is open: the Steam game would get Quick Access's presses"),
        ("hold_steam_pads(false)", "Steam's virtual controllers are never let go after Quick Access: the Steam game would get no controller"),
        ("steam_thaw::stopped", "Quick Access doesn't watch for Steam being stopped while it's open"),
        ("steam_thaw::let_go", "Quick Access doesn't let Steam go when HHD stops it"),
        ("let_steam_run(", "nothing lets Steam go after Quick Access closes (HHD's own SIGCONT never comes if its overlay thread ended)"),
    ]:
        need(GM, gm, sw, needle, why, "fn steam_while_open")

keeper = fn_body(gm, "start_overlay_keeper")
need(GM, gm, keeper, "let_steam_run(", "Steam isn't let go when Handheld Daemon's overlay app goes away (its thread died with Steam stopped: frozen for good)", "fn start_overlay_keeper")

show = fn_body(gm, "show_overlay")
hide = fn_body(gm, "hide_overlay")
need(GM, gm, show, "hold_game(true)", "show_overlay no longer calls hold_game(true)", "fn show_overlay")
need(GM, gm, hide, "hold_game(false)", "hide_overlay no longer calls hold_game(false): closing Quick Access wouldn't give the controller back", "fn hide_overlay")

for needle, why in [
    ("pub fn hold_steam_pads", "guide.rs has no hold_steam_pads"),
    ("is_steam_virtual(", "guide.rs doesn't tell Steam's virtual controllers apart"),
    ("steam_pad && self.steam_held.load", "guide.rs passes on Steam's virtual controller presses while held: every press would reach Quick Access twice"),
]:
    need(GUIDE, guide, guide, needle, why)

for needle, why in [
    ('ends_with(b"/steam")', "steam_thaw.rs no longer matches HHD's rule (first or second argument ends in /steam)"),
    (".take(2)", "steam_thaw.rs no longer looks at the first two arguments only, as HHD does"),
    ("Some('T')", "steam_thaw.rs no longer only lets go processes stopped by a signal"),
    ("m.uid() == uid", "steam_thaw.rs no longer keeps to this user's processes"),
]:
    need(THAW, thaw, thaw, needle, why)

# HHD's own rule, from its source (warn-only: no network, or HHD moved things).
def fetch(url):
    try:
        with urllib.request.urlopen(url, timeout=20) as r:
            return r.read().decode("utf-8", "replace")
    except Exception as e:  # noqa: BLE001
        print(f"::warning title=HHD source::couldn't read {url} ({e}); HHD's Steam freeze rule not compared")
        return None

plugin = fetch(HHD_PLUGIN)
if plugin is not None:
    body = plugin[plugin.find("def get_steam_location"):][:1500]
    if "def freeze_steam" not in plugin:
        print("::notice title=HHD source::HHD has no freeze_steam any more: it may not stop Steam now (steam_thaw.rs stays harmless)")
    elif 'endswith(b"/steam")' not in body or "cmd[:2]" not in body:
        print("::warning title=HHD source::HHD's get_steam_location changed: check that steam_thaw.rs hhd_would_stop still matches the process HHD stops")
    else:
        print("::notice title=HHD source::HHD still stops the first process whose first or second argument ends in /steam (steam_thaw.rs matches)")
ctl = fetch(HHD_CONTROLLERS)
if ctl is not None and "freeze_steam(True)" in ctl and "freeze_steam(False)" in ctl:
    print("::notice title=HHD source::HHD still stops Steam while its overlay holds the controllers and lets it go only from its loop")

if errors:
    for rel, ln, why in errors:
        print(f"::error file={rel},line={ln},title=Overlay freeze::{why}")
    sys.exit(1)
print("::notice title=Overlay freeze::Quick Access over Steam games keeps Steam running (HHD's stop undone), holds Steam's virtual controllers meanwhile, and lets Steam go after closing and when HHD's overlay app goes away")
