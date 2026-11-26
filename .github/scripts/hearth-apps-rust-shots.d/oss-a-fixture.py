#!/usr/bin/python3
"""Pretend worlds for the open-source apps' pictures (oss-a.sh): a home folder per app with what the project it's built
on keeps there, and stand-ins for its commands (on PATH first), so each app shows its approved board
(docs/mockups/open-source-apps) through its real code paths: it runs the commands and reads the files as on a PC.
  oss-a-fixture.py OUT APP     (APP: game-fixer)
Prints the folder of stand-in commands; the home is OUT/APP/home."""
import json
import os
import stat
import sys

out, app = sys.argv[1], sys.argv[2]
root = os.path.join(out, app)
home = os.path.join(root, "home")
bin_ = os.path.join(root, "bin")
os.makedirs(bin_, exist_ok=True)


def w(path, text, mode=None):
    p = os.path.join(home, path) if not os.path.isabs(path) else path
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)
    if mode:
        os.chmod(p, mode)


def cmd(name, body):
    w(os.path.join(bin_, name), "#!/bin/sh\n" + body, 0o755)


look = {"look": "hearth", "mode": "dark"}
w(".local/share/nimbus/desktop.json", json.dumps(look) + "\n")
cmd("xdg-user-dir", 'echo "$HOME/Desktop"\n')
cmd("xdg-open", "exit 0\n")
cmd("hearth-send-logs", 'echo "Sent: code HX-TEST"\n')

if app == "game-fixer":
    games = [
        ("bfme2", "Battle for Middle-earth II", "Proton Hearth 9-4", {}, ["#6a7a3a", "#1e2614"]),
        ("universe-at-war", "Universe at War", "Wine-GE 8-26", {}, ["#3a6aa8", "#0d1424"]),
        ("star-wars-galactic-battlegrounds", "Star Wars: Galactic Battlegrounds", "Wine 9.20", {}, ["#c8a050", "#3a2a10"]),
        ("rise-of-legends", "Rise of Legends", "Proton 9.0", {}, ["#a85a3a", "#2a120a"]),
        ("exigo", "Exigo", "Wine 9.20", {"needsFix": True}, ["#7a4aa8", "#1a0d2a"]),
        ("trainz-2010", "Trainz 2010", "Proton Hearth 9-4", {}, ["#3a8a6a", "#0d2a20"]),
        ("battle-realms", "Battle Realms", "Wine 9.20", {"installing": 64}, ["#a83a4a", "#2a0d14"]),
        ("age-of-mythology", "Age of Mythology", "Wine-GE 8-26", {}, ["#4a8aa8", "#0d202a"]),
        ("black-and-white", "Black & White", "Proton 9.0", {}, ["#8a8a8a", "#2a2a2a"]),
    ]
    listing = []
    for i, (slug, name, wine, note, art) in enumerate(games, 1):
        gdir = os.path.join(home, "Games", slug)
        listing.append({"id": i, "slug": slug, "name": name, "runner": "wine", "platform": "Windows", "year": 2006, "directory": gdir, "playtime": 2.5, "lastplayed": 0})
        yml = "game:\n  exe: drive_c/game/game.exe\n  prefix: ~/Games/%s/prefix\n  arch: win64\nsystem: {}\nwine:\n  version: %s\n" % (slug, wine)
        if slug == "bfme2":
            yml = ("game:\n  exe: drive_c/Program Files/BFME2/lotrbfme2ep1.exe\n  prefix: ~/Games/bfme2/prefix\n  arch: win64\n"
                   "system:\n  env:\n    WINEDLLOVERRIDES: d3d9=n\n    WINE_LARGE_ADDRESS_AWARE: '1'\n    MANGOHUD_CONFIG: preset=1\n"
                   "  prelaunch_command: set-hd-mode.sh\n  resolution: 1920x1080\n  mangohud: true\n"
                   "wine:\n  version: Proton Hearth 9-4\n  dxvk: true\n  dxvk_version: v2.4\n  esync: true\n  fsync: true\n  overrides:\n    d3dx9_43: native\n")
            for f in ("data/_patch_widescreen.big", "lobby/hearth-lobby.ini", "hearth-fixes/nocd.dat"):
                w(os.path.join(gdir, f), "x\n")
        w(".config/lutris/games/%s-17%08d.yml" % (slug, i), yml)
        n = dict(note)
        n["art"] = art
        if slug == "bfme2":
            n.update({
                "about": "Rise of the Witch-king · installed from the disc image with the community script",
                "size": 8375186227,
                "script": {"name": "RotWK + Edain", "v": "3.2", "by": "", "installs": 2314, "updated": "2026-11-09T18:20:00Z", "asks": "Wine-GE 8-26 or newer",
                           "steps": [{"text": t, "done": True} for t in ("Make a 64-bit prefix with Proton Hearth 9-4", "Install BFME II and Rise of the Witch-king from your disc images",
                                                                       "Patch 1.09, then the widescreen fix", "Edain 4.7 and the Völker submod", "Fixes above, then a test start (passed in 41 s)")]},
                "fixes": [
                    {"id": "laa", "name": "Large address aware", "about": "Lets the game use 4 GB, stops the late-game crash", "set": {"system.env.WINE_LARGE_ADDRESS_AWARE": "1"}},
                    {"id": "widescreen", "name": "Widescreen and HUD fix", "about": "16:9 without stretching, menus scaled", "files": ["data/_patch_widescreen.big"]},
                    {"id": "online", "name": "Online play (GameRanger-style lobby)", "about": "Opens the ports the game needs", "files": ["lobby/hearth-lobby.ini"]},
                    {"id": "dx9", "name": "DirectX 9 parts · Visual C++ 2005", "about": "Installed into this game's prefix", "set": {"wine.overrides.d3dx9_43": "native"}},
                    {"id": "sync", "name": "Esync / Fsync", "about": "Smoother with many units on screen", "set": {"wine.esync": "true", "wine.fsync": "true"}},
                    {"id": "nocd", "name": "Disc check skip", "about": "Uses your own disc image, no disc needed", "files": ["hearth-fixes/nocd.dat"]},
                ],
            })
        w(".local/share/hearth/game-fixer/games/%s.json" % slug, json.dumps(n, indent=1, ensure_ascii=False) + "\n")
    w(os.path.join(root, "lutris-games.json"), json.dumps(listing))
    w(".local/share/hearth/game-fixer/logs/bfme2-start.log",
      "01:12:04 Starting game.dat with Proton Hearth 9-4\n01:12:04 dxvk: D3D9 device ready · RTX 5070 · 1920x1080\n01:12:05 fsync: up and running\n"
      "01:12:05 fixme: dsound: buffer flags 0x4 not handled\n01:12:07 lotrbfme2ep1.exe: large address aware on (4 GB)\n01:12:09 Edain 4.7 big files loaded (212)\n"
      "01:12:11 First frame shown after 6.8 s\n01:48:30 Game closed normally · played 36 min\n")
    for r in ("dosbox", "scummvm", "wine"):
        os.makedirs(os.path.join(home, ".local/share/lutris/runners", r), exist_ok=True)
    cmd("lutris", 'case "$*" in\n  *--list-games*) cat "%s";;\n  *--list-wine-versions*) printf "Proton Hearth 9-4\\nProton 9.0\\nWine-GE 8-26\\nWine 9.20\\n";;\n  *) echo "lutris $*";;\nesac\n' % os.path.join(root, "lutris-games.json"))
    cmd("curl", 'for a; do u="$a"; done\ncase "$u" in\n  *installers\\?page_size=1*) echo \'{"count": 4812}\';;\n'
        '  *api/games\\?search=*) echo \'{"count": 2, "results": [{"slug": "the-battle-for-middle-earth-ii", "name": "The Battle for Middle-earth II", "year": 2006}, {"slug": "the-lord-of-the-rings-the-battle-for-middle-earth-ii-the-rise-of-the-witch-king", "name": "The Rise of the Witch-king", "year": 2006}]}\';;\n'
        '  *) exit 22;;\nesac\n')
else:
    sys.exit("no such app: " + app)

print(bin_)
