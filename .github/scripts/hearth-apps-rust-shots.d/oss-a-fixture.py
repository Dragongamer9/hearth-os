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
elif app == "proton-manager":
    MB = 1048576

    def sparse(path, mb):
        p = os.path.join(home, path) if not os.path.isabs(path) else path
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.truncate(int(mb * MB))

    builtin = os.path.join(root, "builtin")
    w(os.path.join(builtin, "Proton-Hearth9-4/proton"), "#!/bin/sh\n", 0o755)
    sparse(os.path.join(builtin, "Proton-Hearth9-4/files/lib.bin"), 812)
    st = ".local/share/Steam"
    w(st + "/compatibilitytools.d/GE-Proton9-20/proton", "#!/bin/sh\n", 0o755)
    w(st + "/compatibilitytools.d/GE-Proton9-20/compatibilitytool.vdf", '"compatibilitytools"\n{\n}\n')
    sparse(st + "/compatibilitytools.d/GE-Proton9-20/files/lib.bin", 790)
    for folder, ver, mb in (("Proton 9.0", "proton-9.0-3", 735), ("Proton 8.0", "proton-8.0-5", 702)):
        w(st + "/steamapps/common/%s/proton" % folder, "#!/bin/sh\n", 0o755)
        w(st + "/steamapps/common/%s/version" % folder, "1712345678 %s\n" % ver)
        sparse(st + "/steamapps/common/%s/files/lib.bin" % folder, mb)
    sparse(".local/share/lutris/runners/wine/lutris-GE-Proton8-26-x86_64/lib/wine.bin", 410)
    cmd("wine", 'echo wine-9.20\n')
    sparse(os.path.join(root, "lib64/wine/wine.bin"), 380)
    # Steam's games: three with a pick of their own, the rest as their prefixes last ran
    named = {"1245620": ("ELDEN RING", "GE-Proton9-20"), "976730": ("Halo: The Master Chief Collection", "proton_9"), "1091500": ("Cyberpunk 2077", "GE-Proton9-20")}
    apps = list(named) + ["%d" % (300000 + i) for i in range(203)]
    for i, appid in enumerate(apps):
        name = named.get(appid, ("Game %03d" % i, ""))[0]
        w(st + "/steamapps/appmanifest_%s.acf" % appid, '"AppState"\n{\n\t"appid"\t\t"%s"\n\t"name"\t\t"%s"\n}\n' % (appid, name))
        if appid not in named:
            n = i - 3
            ran = "Proton-Hearth9-4" if n < 167 else "proton-9.0-3" if n < 196 else "GE-Proton9-20"
            w(st + "/steamapps/compatdata/%s/version" % appid, ran + "\n")
    maps = "".join('\t\t\t\t\t"%s"\n\t\t\t\t\t{\n\t\t\t\t\t\t"name"\t\t"%s"\n\t\t\t\t\t\t"config"\t\t""\n\t\t\t\t\t\t"priority"\t\t"%s"\n\t\t\t\t\t}\n' % (a_, t_, p_)
                   for a_, t_, p_ in [("0", "Proton-Hearth9-4", "75")] + [(a_, v[1], "250") for a_, v in named.items()])
    w(st + "/config/config.vdf", '"InstallConfigStore"\n{\n\t"Software"\n\t{\n\t\t"Valve"\n\t\t{\n\t\t\t"Steam"\n\t\t\t{\n\t\t\t\t"CompatToolMapping"\n\t\t\t\t{\n' + maps + '\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n')
    # Lutris' games (Hearth Game Fixer's): their wine versions
    lut = [("bfme2", "Battle for Middle-earth II", "Proton Hearth 9-4"), ("universe-at-war", "Universe at War", "lutris-GE-Proton8-26-x86_64"), ("star-citizen", "Star Citizen", "lutris-GE-Proton8-26-x86_64"),
           ("exigo", "Exigo", "lutris-GE-Proton8-26-x86_64"), ("rise-of-legends", "Rise of Legends", "lutris-GE-Proton8-26-x86_64"), ("age-of-mythology", "Age of Mythology", "lutris-GE-Proton8-26-x86_64"),
           ("black-and-white", "Black & White", "system"), ("star-wars-galactic-battlegrounds", "Star Wars: Galactic Battlegrounds", "system")]
    for i, (slug, name, ver) in enumerate(lut, 1):
        w(".config/lutris/games/%s-17%08d.yml" % (slug, i), "game:\n  exe: drive_c/game.exe\nwine:\n  version: %s\n" % ver)
        if slug in ("bfme2", "universe-at-war"):
            w(".local/share/hearth/game-fixer/games/%s.json" % slug, json.dumps({"script": {"asks": "Wine-GE 8-26 or newer"}}) + "\n")
    w(".local/share/hearth/game-fixer/names.json", json.dumps({s_: n_ for s_, n_, _ in lut}) + "\n")
    w(".local/share/nimbus/proton-worked.json", json.dumps({"epic:alanwake2": {"worked": "proton_9", "picked": True, "title": "Alan Wake 2", "why": "Ray tracing works", "at": 1764100000}}) + "\n")
    t0 = 1764140000
    w(".config/nimbus/proton-manager.json", json.dumps({
        "why": {"steam:1245620": "Cutscenes play", "steam:976730": "Anti-cheat", "lutris:universe-at-war": "Online lobby", "lutris:star-citizen": "Launcher needs it"},
        "changed": {"lutris:bfme2": t0 + 6, "steam:1245620": t0 + 5, "hearth:epic:alanwake2": t0 + 4, "lutris:universe-at-war": t0 + 3, "lutris:star-citizen": t0 + 2, "steam:976730": t0 + 1},
        "download": {"name": "Proton-GE 9-21", "family": 1, "url": "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton9-21/GE-Proton9-21.tar.gz",
                     "size": 790 * MB, "sha": "", "part": os.path.join(home, ".local/share/hearth/runners/GE-Proton9-21.tar.gz.part")}}) + "\n")
    sparse(".local/share/hearth/runners/GE-Proton9-21.tar.gz.part", 490)
    w(".config/machine-info", "")
    rel = lambda tag, day, body: {"tag_name": tag, "published_at": "2026-%s" % day, "body": body, "draft": False, "prerelease": False,
                                  "assets": [{"name": tag + ".tar.gz", "size": 812 * MB, "browser_download_url": "https://example.invalid/" + tag + ".tar.gz"}]}
    ph = [rel("Proton-Hearth9-5", "11-24T10:00:00Z", "Fixes for anti-cheat games, faster shader cache"), rel("Proton-Hearth9-4", "11-02T10:00:00Z", "x"),
          rel("Proton-Hearth9-3", "10-12T10:00:00Z", "x"), rel("Proton-Hearth9-2", "09-28T10:00:00Z", "x"), rel("Proton-Hearth9-1", "09-03T10:00:00Z", "x")]
    w(os.path.join(root, "ph.json"), json.dumps(ph))
    cmd("curl", 'case "$*" in\n  *" -o "*) sleep 25; exit 1;;\n  *proton-hearth/releases*) cat "%s";;\n  *) exit 22;;\nesac\n' % os.path.join(root, "ph.json"))
    cmd("bootc", 'echo \'{"status": {"booted": {"image": {"image": {"image": "ghcr.io/dragongamer9/hearth-os", "transport": "registry"}, "version": "1.18.0"}}}}\'\n')
    print("BUILTIN=" + builtin)
elif app == "overlay":
    on = ["fps", "frametime", "frame_timing", "gpu_stats", "gpu_temp", "gpu_power", "cpu_stats", "cpu_temp", "cpu_mhz", "vram", "ram", "exec_name", "wine", "engine_version"]
    off = ["core_load", "fan", "battery", "battery_time", "time", "network"]
    conf = ("# Hearth Overlay (MangoHud's settings)\nlegacy_layout=false\n" + "".join(k + "\n" for k in on) + "".join(k + "=0\n" for k in off) +
            "position=top-left\nfont_size=14\nbackground_alpha=0.38\nfps_limit=120\nfps_limit_method=early\ntoggle_hud=Shift_R+F12\n"
            "gpu_color=9be36b\ncpu_color=5ec8ff\nvram_color=e36bd3\nframetime_color=ff8a3d\nram_color=ffd36b\nfps_color_change\n")
    w(".config/MangoHud/MangoHud.conf", conf)
    w(".config/MangoHud/wine-ForzaHorizon5.conf", conf)
    w(".config/MangoHud/wine-eldenring.conf", conf.replace("fps_limit=120", "fps_limit=60"))
    w(".config/environment.d/90-hearth-overlay.conf", "MANGOHUD=1\n")
    w(".config/nimbus/overlay.json", json.dumps({"names": {"wine-ForzaHorizon5.conf": "Forza Horizon 5", "wine-eldenring.conf": "Elden Ring"}, "gameModeLevel": 3, "warnGpu": 83}) + "\n")
elif app == "bottles":
    MB = 1048576
    user = os.environ.get("USER") or "ci"
    bottles = [
        ("office", "Office and tools", "Application", "wine-ge-8-26", "2026-10-30 21:10:00", 3.8 * 1024, [("Office 2016", "WINWORD.EXE"), ("Paint.NET", "PaintDotNet.exe"), ("Notepad++", "notepad++.exe"), ("WorldBuilder", "WorldBuilder.exe"), ("foobar2000", "foobar2000.exe"), ("7-Zip File Manager", "7zFM.exe")],
         ["dotnet48", "vcrun2022", "corefonts", "mf"]),
        ("games-old", "Games, old", "Gaming", "proton-hearth-9-4", "2026-11-01 18:00:00", 6.2 * 1024, [("a", "a.exe"), ("b", "b.exe"), ("c", "c.exe"), ("d", "d.exe")], []),
        ("music", "Music and audio", "Application", "sys-wine-9.20", "2026-11-03 18:00:00", 2.1 * 1024, [("a", "a.exe"), ("b", "b.exe"), ("c", "c.exe")], []),
        ("mod-tools", "Mod tools", "Application", "wine-ge-8-26", "2026-11-05 18:00:00", 4.6 * 1024, [("a", "a.exe"), ("b", "b.exe"), ("c", "c.exe"), ("d", "d.exe"), ("e", "e.exe")], []),
        ("32-bit", "32-bit only", "Custom", "sys-wine-9.20", "2026-11-08 18:00:00", 1.9 * 1024, [("a", "a.exe"), ("b", "b.exe")], []),
        ("trying", "Trying something", "Custom", "wine-ge-8-26", "2026-11-20 18:00:00", 2.8 * 1024, [], []),
    ]
    base = os.path.join(home, ".local/share/hearth/bottles")
    for i, (d, name, env, runner, made, size_mb, progs, deps) in enumerate(bottles):
        bd = os.path.join(base, d)
        y = "Name: %s\nArch: %s\nWindows: win10\nRunner: %s\nDXVK: dxvk-2.4\nVKD3D: vkd3d-proton-2.12\nPath: %s\nCustom_Path: true\nEnvironment: %s\nCreation_Date: '%s'\n" % (
            "'" + name.replace("'", "''") + "'" if "," in name else name, "win32" if d == "32-bit" else "win64", runner, bd, env, made)
        y += "Parameters:\n  dxvk: true\n  vkd3d: false\n  hearth_sharp_fonts: true\n  hearth_fit_scale: true\n  versioning_automatic: true\n  sandbox: true\n"
        y += "Sandbox:\n  share_net: true\n  share_sound: true\n"
        y += "Installed_Dependencies:\n" + "".join("- %s\n" % x for x in deps) if deps else "Installed_Dependencies: []\n"
        if progs:
            y += "External_Programs:\n" + "".join("  p%d:\n    name: %s\n    executable: %s\n    path: C:\\\\Program Files\\\\%s\n" % (n, pn if "+" not in pn else "'%s'" % pn, exe, exe) for n, (pn, exe) in enumerate(progs))
        w(os.path.join(bd, "bottle.yml"), y)
        os.makedirs(os.path.join(bd, "drive_c/users", user), exist_ok=True)
        with open(os.path.join(bd, "drive_c/space.bin"), "wb") as f:
            f.truncate(int(size_mb * MB))
        if d == "office":
            for fo in ("Documents", "Downloads"):
                os.makedirs(os.path.join(home, fo), exist_ok=True)
                os.symlink(os.path.join(home, fo), os.path.join(bd, "drive_c/users", user, fo))
    # Hearth's snapshots of Office and tools (empty stand-ins, named and timed as the board's)
    import time
    now = int(time.time())
    sd = os.path.join(home, ".local/share/hearth/bottles-snapshots/Office and tools")
    os.makedirs(sd, exist_ok=True)
    for ts, nm in ((now - 2 * 3600, "Before_Office_update"), (now - 5 * 86400, "After_installing_Paint.NET"), (now - 27 * 86400, "Fresh_bottle")):
        open(os.path.join(sd, "%d-%s.tar.zst" % (ts, nm)), "wb").close()
    w(".config/nimbus/bottles.json", json.dumps({
        "about": {"Office 2016": "Word, Excel, PowerPoint", "Paint.NET": "Picture editor", "Notepad++": "Text editor", "WorldBuilder": "BFME map maker", "foobar2000": "Music player", "7-Zip File Manager": "Archives"},
        "marks": {"Office 2016": ["O", 0], "Paint.NET": ["PN", 1], "Notepad++": ["N+", 2], "WorldBuilder": ["WB", 3], "foobar2000": ["FB", 4], "7-Zip File Manager": ["7Z", 5]},
        "kinds": {"Office and tools": "Apps", "Games, old": "Games", "Music and audio": "Apps", "Mod tools": "Apps", "32-bit only": "Old apps", "Trying something": "Empty"},
        "extras": {"Office and tools": {"dotnet48": "for Paint.NET", "vcrun2022": "for 3 programs", "corefonts": "documents look right", "mf": "video in apps"}},
        "names": {"WINWORD": "Word"}, "procMarks": {"WINWORD": "Office 2016", "foobar2000": "foobar2000"}}) + "\n")
    # Office's programs in the Apps menu (as Bottles' own entries would be)
    for pn in ("Office 2016", "Paint.NET", "Notepad++", "WorldBuilder", "foobar2000", "7-Zip File Manager"):
        slug = "".join(c.lower() if c.isalnum() else "-" for c in "Office and tools-" + pn)
        w(".local/share/applications/hearth-bottles-%s.desktop" % slug, "[Desktop Entry]\nType=Application\nName=%s\n" % pn)
    cmd("bottles-cli", 'echo "bottles-cli $*"\n')
    cmd("curl", 'case "$*" in\n  *bottlesdevs/programs*) printf "%s";;\n  *) exit 22;;\nesac\n' % "".join(
        "%s:\n  Name: %s\n  Description: x\n  Grade: Platinum\n" % (k_, n_) for k_, n_ in (("office-2016", "Office 2016 installer"), ("paint-net", "Paint.NET"), ("msi-afterburner", "MSI Afterburner"), ("notepad-plus-plus", "Notepad++"), ("irfanview", "IrfanView"))).replace("%", "%%"))
    # two programs running in Office and tools (stand-ins with its WINEPREFIX, named as Wine names them)
    w(os.path.join(root, "run-office.sh"), "#!/bin/sh\nWINEPREFIX='%s' exec -a WINWORD.EXE python3 -c 'import time; x = bytearray(412 * 1048576); x[::4096] = b\"1\" * len(x[::4096]); time.sleep(90)' 'Story notes.docx' &\nWINEPREFIX='%s' exec -a foobar2000.exe python3 -c 'import time; x = bytearray(64 * 1048576); x[::4096] = b\"1\" * len(x[::4096]); time.sleep(90)' &\n" % (os.path.join(base, "office"), os.path.join(base, "office")), 0o755)
elif app == "hearth-3d":
    import math
    def box(w, d, h):
        v = [(x * w / 2, y * d / 2, z * h / 2) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
        q = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
        t = [i for a_, b_, c_, d_ in q for i in (a_, b_, c_, a_, c_, d_)]
        return [c for p_ in v for c in p_], t
    th = math.radians(35)
    def mat(x, y, z):
        c_, s_ = math.cos(th), math.sin(th)
        return [c_, s_, 0, 0, -s_, c_, 0, 0, 0, 0, 1, 0, c_ * x - s_ * y, s_ * x + c_ * y, z, 1]
    objs = []
    def add(name, size, pos, col, faces, points, **kw):
        v, t = box(*size) if size else ([], [])
        d = {"name": name, "type": kw.pop("type", "MESH"), "loc": list(pos), "rot": [0, 0, 35], "scale": [1, 1, 1], "verts": v, "tris": t, "mat": mat(*pos), "color": col, "faces": faces, "points": points, "mods": [], "render": True, "hide": False}
        d.update(kw)
        objs.append(d)
    green = [0.42, 0.48, 0.36]
    add("Body", (4, 4, 1.2), (0, 0, 2.4), [0.46, 0.53, 0.40], 2412, 4880, keys=[12, 36, 60, 84],
        mods=[{"name": "Mirror", "type": "MIRROR", "words": "X", "show": True}, {"name": "Bevel", "type": "BEVEL", "words": "2 cm · 3 steps", "show": True}, {"name": "Smooth by angle", "type": "NODES", "words": "30°", "show": True}])
    add("Cockpit", (1.5, 1.5, 0.8), (0, 1.5, 3.4), [0.52, 0.59, 0.46], 1800, 3640)
    add("Gun", (0.5, 1.9, 0.3), (0, 3.1, 3.55), [0.36, 0.42, 0.31], 900, 1820)
    legs = []
    n = 0
    for x, y in ((-2, -2), (2, -2), (-2, 2), (2, 2)):
        for part, size, z in (("foot", (1.0, 1.0, 0.5), 0.25), ("shin", (0.7, 0.7, 1.7), 1.35)):
            n += 1
            nm = "Leg %d %s" % ((n + 1) // 2, part)
            legs.append(nm)
            add(nm, size, (x, y, z), green, 1724, 3460, keys=list(range(0, 96, 8)))
    add("Key light", None, (6, -6, 9), [1, 0.95, 0.85], 0, 0, type="LIGHT")
    add("Rim light", None, (-6, 6, 7), [0.8, 0.85, 1], 0, 0, type="LIGHT")
    add("Camera", None, (14, -14, 9), [0.5, 0.5, 0.5], 0, 0, type="CAMERA")
    add("Ground", None, (0, 0, 0), [0.2, 0.2, 0.2], 0, 0)
    scene = {"collections": {"name": "Scene Collection", "objects": ["Camera", "Ground"], "children": [
        {"name": "Mech", "objects": ["Body", "Cockpit", "Gun"], "children": [{"name": "Legs", "objects": legs, "children": []}]},
        {"name": "Lights", "objects": ["Key light", "Rim light"], "children": []}]},
        "frame": 32, "start": 0, "end": 96, "fps": 24, "action": "Walk cycle", "world": [0.05, 0.05, 0.05], "objects": objs}
    w(os.path.join(root, "scene.json"), json.dumps(scene))
    w(os.path.join(home, "Models/quad-mech.blend"), "BLENDER-v403 (a stand-in)\n")
    cmd("blender", 'out=; take=; for a; do [ -n "$take" ] && { out="$a"; take=; }; [ "$a" = "--" ] && take=1; done\n[ -n "$out" ] && cp "%s" "$out"\nexit 0\n' % os.path.join(root, "scene.json"))
else:
    sys.exit("no such app: " + app)

print(bin_)
