#!/usr/bin/python3
"""A pretend PC for the Mod Manager's pictures (mods.sh): a home folder with a real Hearth Mod Organizer instance for
Skyrim Special Edition (Mod Organizer 2's own files: ModOrganizer.ini, mods/<mod>/meta.ini with files that clash,
profiles with modlist.txt / plugins.txt / loadorder.txt and their INIs, plugins with real TES4 headers and masters),
a game folder and a Proton prefix with a Crash Logger SSE log and saves, earlier snapshots beside the instance, and
the Mod Manager's own notes (updates found, Describe it's last answer, the devices). The app reads all of it through
its real code (hearth-gaming's organizer, logic.rs), so each picture is its board drawn from data.
  mods-fixture.py HOME"""
import json
import os
import struct
import sys

home = sys.argv[1]
data = os.path.join(home, ".local/share/nimbus")
inst = os.path.join(data, "mods/organizer/skyrimse")
steam = os.path.join(home, ".local/share/Steam")
game = os.path.join(steam, "steamapps/common/Skyrim Special Edition")
pfx = os.path.join(home, "prefix/pfx")
docs = os.path.join(pfx, "drive_c/users/steamuser/Documents/My Games/Skyrim Special Edition")
own = os.path.join(data, "mods/manager")


def w(path, content, mode="w"):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, mode, **({} if "b" in mode else {"encoding": "utf-8"})) as f:
        f.write(content)


def z(s):
    return s.encode() + b"\0"


def field(out, t, b):
    out += t + struct.pack("<H", len(b)) + b


def esp(flags, masters, author="Dragongamer9"):
    f = bytearray()
    field(f, b"HEDR", struct.pack("<fII", 1.7, 3, 0x800))
    field(f, b"CNAM", z(author))
    for m in masters:
        field(f, b"MAST", z(m))
        field(f, b"DATA", struct.pack("<Q", 0))
    return b"TES4" + struct.pack("<II", len(f), flags) + b"\0" * 8 + struct.pack("<H", 44) + b"\0\0" + bytes(f)


w(os.path.join(data, "desktop.json"), json.dumps({"look": "hearth", "mode": "dark"}) + "\n")

# ---- the game folder (never changed) and its prefix
for f in ["SkyrimSE.exe", "SkyrimSELauncher.exe", "skse64_loader.exe"]:
    w(os.path.join(game, f), "MZ", "w")
for m in ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"]:
    w(os.path.join(game, "Data", m), esp(1, [] if m == "Skyrim.esm" else ["Skyrim.esm"], "Bethesda"), "wb")
w(os.path.join(game, "Data/ccBGSSSE001-Fish.esm"), esp(0x201, ["Skyrim.esm"], "Bethesda"), "wb")
for i in range(40):
    w(os.path.join(game, "Data/textures/architecture/whiterun/wrwood%02d.dds" % i), "game")

# ---- Steam's library: the games the Mod Manager finds by itself (appmanifest files, as Steam writes them)
for app, name in [(489830, "Skyrim Special Edition"), (377160, "Fallout 4"), (1091500, "Cyberpunk 2077"), (1086940, "Baldur's Gate 3"), (413150, "Stardew Valley"), (892970, "Valheim"), (1716740, "Starfield"), (1245620, "ELDEN RING"), (1145350, "Hades II")]:
    w(os.path.join(steam, "steamapps/appmanifest_%d.acf" % app), '"AppState"\n{\n\t"appid"\t\t"%d"\n\t"name"\t\t"%s"\n\t"installdir"\t\t"%s"\n\t"StateFlags"\t\t"4"\n\t"SizeOnDisk"\t\t"1000"\n}\n' % (app, name, name))
    w(os.path.join(steam, "steamapps/common", name, "game.txt"), name)
w(os.path.join(steam, "steamapps/libraryfolders.vdf"), '"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"\t\t"%s"\n\t}\n}\n' % steam)

# ---- the mods (separators and all), lowest priority first
S = "separator"
mods = [
    (S, "Fixes and frameworks"),
    ("Address Library for SKSE Plugins", "11", 32444, ["SKSE/Plugins/versionlib-1-6-1170-0.bin"], None),
    ("Unofficial Skyrim SE Patch", "4.3.2", 266, ["textures/architecture/whiterun/wrwood%02d.dds" % i for i in range(20)], ("unofficial skyrim special edition patch.esp", 1, ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"])),
    ("SSE Engine Fixes", "7.0.18", 17230, ["SKSE/Plugins/EngineFixes.dll"], None),
    ("powerofthree's Tweaks", "1.13", 51073, ["SKSE/Plugins/po3_Tweaks.dll"], None),
    (S, "Interface"),
    ("SkyUI", "5.2", 12604, ["SKSE/Plugins/SkyUI_SE.dll", "interface/skyui/config.txt"], ("SkyUI_SE.esp", 0, ["Skyrim.esm"])),
    ("MCM Helper", "1.5", 53000, ["SKSE/Plugins/MCMHelper.dll"], None),
    ("moreHUD", "5.1.3", 12688, ["SKSE/Plugins/AHZmoreHUDPlugin.dll"], None),
    (S, "Textures and world"),
    ("Static Mesh Improvement Mod", "2.09", 659, ["textures/architecture/whiterun/wrwood%02d.dds" % i for i in range(10, 30)] + ["meshes/architecture/solitude/sbluepalace.nif"], None),
    ("Noble Skyrim", "5.4", 21423, ["textures/architecture/whiterun/wrwood%02d.dds" % i for i in range(0, 34)] + ["meshes/architecture/solitude/sbluepalace.nif", "textures/landscape/mountains/mountainslab01.dds", "textures/dungeons/nordic/nordicwall01.dds"], None),
    ("Skyrim 202X Architecture", "10.0", 2347, ["meshes/architecture/solitude/sbluepalace.nif", "textures/architecture/whiterun/wrwood05.dds"], None),
    ("Majestic Mountains", "2.0", 11052, ["textures/landscape/mountains/mountainslab01.dds"], ("MajesticMountains.esp", 0x200, ["Skyrim.esm"])),
    (S, "Lighting and weather"),
    ("Community Shaders", "1.1", 86492, ["SKSE/Plugins/CommunityShaders.dll"], None),
    ("Lux", "5.4", 43158, ["meshes/lux/candle01.nif"], ("Lux.esp", 0, ["Skyrim.esm", "Update.esm"])),
    ("Cathedral Weathers", "2.4", 24791, ["textures/sky/clouds01.dds"], ("Cathedral Weathers.esp", 0x200, ["Skyrim.esm"])),
    (S, "Gameplay"),
    ("Alternate Start - Live Another Life", "4.2", 272, ["scripts/arth_lal.pex"], ("Alternate Start - Live Another Life.esp", 0, ["Skyrim.esm", "Update.esm", "unofficial skyrim special edition patch.esp"])),
    ("Ordinator - Perks of Skyrim", "9.31", 1137, ["scripts/ordinator.pex"], ("Ordinator - Perks of Skyrim.esp", 0, ["Skyrim.esm", "Update.esm"])),
    (S, "Patches"),
    ("Lux - Ordinator Patch", "1.0", 43158, ["scripts/luxord.pex"], ("Lux - Ordinator Patch.esp", 0, ["Lux.esp", "Ordinator - Perks of Skyrim.esp"])),
    ("Lux - USSEP Patch", "1.0", 43158, ["scripts/luxussep.pex"], ("Lux - USSEP Patch.esp", 0, ["Lux.esp", "unofficial skyrim special edition patch.esp"])),
    ("My tweaks (made by hand)", "", 0, ["interface/skyui/config.txt", "textures/architecture/whiterun/wrwood33.dds"], None),
]
off = {"Lux"}
order = []  # lowest first
plugins = []
for m in mods:
    if m[0] == S:
        name = m[1] + "_separator"
        os.makedirs(os.path.join(inst, "mods", name), exist_ok=True)
        w(os.path.join(inst, "mods", name, "meta.ini"), "[General]\n")
        order.append(("-", name))
        continue
    name, ver, nid, files, plugin = m
    d = os.path.join(inst, "mods", name)
    meta = "[General]\nversion=%s\ninstallationFile=%s.7z\n" % (ver, name)
    if nid:
        meta += "modid=%d\ngameName=SkyrimSE\n" % nid
    w(os.path.join(d, "meta.ini"), meta)
    for f in files:
        w(os.path.join(d, f), name)
    if plugin:
        pn, flags, masters = plugin
        w(os.path.join(d, pn), esp(flags, masters), "wb")
        plugins.append(pn)
    order.append(("-" if name in off else "+", name))

w(os.path.join(inst, "ModOrganizer.ini"), "[General]\ngameName=Skyrim Special Edition\nselected_profile=Survival playthrough\n\n[Settings]\n")
w(os.path.join(inst, "overwrite/SKSE/Plugins/po3_Tweaks.log"), "log")
for prof, local in [("Survival playthrough", True), ("Vanilla+ (Ally)", True), ("Screenshots", False)]:
    pd = os.path.join(inst, "profiles", prof)
    # modlist.txt: highest priority first
    w(os.path.join(pd, "modlist.txt"), "\n".join("%s%s" % (s, n) for s, n in reversed(order)) + "\n")
    active = [p for p in plugins if p != "Lux.esp"]
    w(os.path.join(pd, "plugins.txt"), "\n".join(("*" if p in active else "") + p for p in plugins) + "\n")
    w(os.path.join(pd, "loadorder.txt"), "\n".join(["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm", "ccBGSSSE001-Fish.esm"] + plugins) + "\n")
    w(os.path.join(pd, "settings.ini"), "[General]\nLocalSaves=%s\nLocalSettings=%s\n" % ("true" if local else "false", "true" if local else "false"))
    w(os.path.join(pd, "Skyrim.ini"), "[Display]\nfShadowDistance=3000\n")
reg = {"instances": [{"id": "skyrimse", "title": "Skyrim Special Edition", "dir": inst, "kind": "skyrimse", "gameDir": game, "libraryId": "steam:489830", "prefix": pfx, "steamApp": 489830, "wabbajack": "", "launcher": "hearth", "added": 1763700000}]}
w(os.path.join(data, "mods/organizer/instances.json"), json.dumps(reg, indent=1))

# ---- earlier snapshots (the format logic.rs takes them in)
T = 1764137400  # Nov 26, 2026, about 1:10 AM US Central
snaps = [
    (T - 60, "before Lux went in", "", False, {"Lux": "+"}),
    (T - 4980, "before 3 updates (SkyUI 5.2, Campfire, MCM Helper)", "", False, {}),
    (T - 102000, "before the Sort moved 4 plugins · the last time it never crashed", "Tuesday's setup", False, {"Lux": "-", "Lux - Ordinator Patch": "-", "Lux - USSEP Patch": "-"}),
    (T - 285000, "before installing Vanilla+ essentials (72 mods)", "", False, {}),
    (T - 427000, "before turning off 6 mods to test a stutter", "", False, {}),
    (T - 530000, "you took it by hand: \"Before the big texture pass\"", "", True, {}),
]
for at, why, name, by_hand, change in snaps:
    sd = os.path.join(inst, ".hearth/snapshots", str(at))
    lines = []
    for s, n in reversed(order):
        lines.append("%s%s" % (change.get(n, s), n))
    w(os.path.join(sd, "modlist.txt"), "\n".join(lines) + "\n")
    w(os.path.join(sd, "loadorder.txt"), "\n".join(["Skyrim.esm", "Update.esm"] + list(reversed(plugins))) + "\n")
    w(os.path.join(sd, "inis/Skyrim.ini"), "[Display]\nfShadowDistance=2500\n")
    vers = {m[0]: m[1] for m in mods if m[0] != S}
    if name:
        vers["SkyUI"] = "5.1"
    w(os.path.join(sd, "snapshot.json"), json.dumps({"at": at, "why": why, "name": name, "profile": "Survival playthrough", "mods": 412 if at > T - 200000 else 340, "byHand": by_hand, "versions": vers}))

# ---- the crash (Crash Logger SSE's own words)
crash = """Skyrim SSE v1.6.1170
CrashLoggerSSE v1-15-0-0 Nov 2 2026 14:02:11

Unhandled exception "EXCEPTION_ACCESS_VIOLATION" at 0x7FF6A41C2D10 SkyrimSE.exe+0C42D10

PROBABLE CALL STACK:
\t[0] 0x7FF6A41C2D10 SkyrimSE.exe+0C42D10 -> GFxMovieView::Invoke
\t[1] 0x7FFB1E3A4410 SkyUI_SE.dll+0024410
\t[2] 0x7FFB1E3A1B02 SkyUI_SE.dll+0021B02
\t[3] 0x7FF6A3F01C3A SkyrimSE.exe+0881C3A -> UI::ProcessMessages
\t[4] 0x7FFB2A0D1C02 MCMHelper.dll+0011C02
\t[5] 0x7FF6A45E1F04 SkyrimSE.exe+1061F04 -> Main::Update

REGISTERS:
\tRAX 0x0
\tRCX 0x2A1F3C80 (GFxValue)
\tRDX 0x86

POSSIBLE RELEVANT OBJECTS:
\tMagicMenu (open) · 134 spells in the list
\tPlayerCharacter · Whiterun (Dragonsreach)

MODULES:
\tSkyUI_SE.dll 5.2.0.0
\tMCMHelper.dll 1.5.0.0
\tCommunityShaders.dll 1.1.0.0
\tEngineFixes.dll 7.0.18 · memory manager on

PLUGINS:
\tSkyrim.esm
\tUpdate.esm
\tunofficial skyrim special edition patch.esp
\tSkyUI_SE.esp
"""
w(os.path.join(docs, "SKSE/crash-2026-11-26-01-22-13.log"), crash)
w(os.path.join(docs, "SKSE/crash-2026-11-21-02-10-40.log"), crash.replace("SkyUI_SE.dll+0024410", "EngineFixes.dll+0001410").replace("MagicMenu (open) · 134 spells in the list", "out of memory in Riften"))
os.utime(os.path.join(docs, "SKSE/crash-2026-11-26-01-22-13.log"), (T + 720, T + 720))
os.utime(os.path.join(docs, "SKSE/crash-2026-11-21-02-10-40.log"), (T - 430000, T - 430000))


# ---- saves (Skyrim SE's own format, uncompressed), each with the plugins it was made with
def wstr(s):
    b = s.encode()
    return struct.pack("<H", len(b)) + b


def save(path, place, level, plugs, at):
    hdr = struct.pack("<II", 12, 41) + wstr("Dragongamer9") + struct.pack("<I", level) + wstr(place) + wstr("041.07.12") + wstr("NordRace") + struct.pack("<HffQ", 0, 0.0, 0.0, 0) + struct.pack("<II", 0, 0) + struct.pack("<H", 0)
    body = struct.pack("<BI", 78, 0) + struct.pack("<B", len(plugs)) + b"".join(wstr(p) for p in plugs)
    w(path, b"TESV_SAVEGAME" + struct.pack("<I", len(hdr)) + hdr + body, "wb")
    os.utime(path, (at, at))


sd = os.path.join(inst, "profiles/Survival playthrough/saves")
base = ["Skyrim.esm", "Update.esm", "unofficial skyrim special edition patch.esp", "SkyUI_SE.esp"]
save(os.path.join(sd, "Save1.ess"), "Whiterun, day 41", 23, base + ["Lux.esp"], T + 600)
save(os.path.join(sd, "Save2.ess"), "Bleak Falls Barrow", 23, base + ["Lux.esp"], T + 720)
save(os.path.join(sd, "Save3.ess"), "Riverwood, day 40", 22, base, T - 6000)
save(os.path.join(sd, "Save4.ess"), "Helgen, start", 1, base, T - 600000)

# ---- an archive waiting to go in, with a FOMOD installer (its steps, pictures and patches)
fo = os.path.join(home, "Downloads/Lux - Lighting")
w(os.path.join(fo, "fomod/info.xml"), "<fomod><Name>Lux</Name><Author>its author</Author><Version>5.4</Version><Description>lighting for every interior</Description><Website>https://www.nexusmods.com/skyrimspecialedition/mods/43158</Website></fomod>")
w(os.path.join(fo, "00 Main/Lux.esp"), esp(0, ["Skyrim.esm", "Update.esm"]), "wb")
for d in ["10 Darker", "10 Author", "10 Brighter", "20 Ordinator", "20 USSEP", "20 Cathedral", "20 JK"]:
    w(os.path.join(fo, d, "meshes/lux/%s.nif" % d.split()[1].lower()), d)
import zlib


def png(path, top, bottom, wd=64, ht=80):
    """A small picture with a candle's glow (top colour fading to the bottom one), as a PNG."""
    raw = b""
    for y in range(ht):
        f = y / (ht - 1)
        c = bytes(int(top[i] * (1 - f) + bottom[i] * f) for i in range(3))
        raw += b"\0" + c * wd
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    w(path, b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", wd, ht, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""), "wb")


for k_, top, bottom in [("darker", (40, 26, 12), (10, 8, 6)), ("author", (150, 96, 40), (40, 24, 10)), ("brighter", (230, 200, 140), (120, 90, 50))]:
    png(os.path.join(fo, "fomod/images/%s.png" % k_), top, bottom)
w(os.path.join(fo, "fomod/ModuleConfig.xml"), """<config><moduleName>Lux</moduleName>
<requiredInstallFiles><folder source="00 Main" destination=""/></requiredInstallFiles>
<installSteps order="Explicit">
 <installStep name="Main files"><optionalFileGroups order="Explicit"><group name="Main files" type="SelectAll"><plugins order="Explicit">
  <plugin name="Lux"><description>The main files.</description><files/><typeDescriptor><type name="Required"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep>
 <installStep name="Interior light"><optionalFileGroups order="Explicit"><group name="How bright should rooms be?" type="SelectExactlyOne"><plugins order="Explicit">
  <plugin name="Darker"><description>Candles and fires only; caves are pitch black</description><image path="fomod\\images\\darker.png"/><files><folder source="10 Darker" destination=""/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  <plugin name="As the author meant"><description>Warm and readable; the most played choice</description><image path="fomod\\images\\author.png"/><files><folder source="10 Author" destination=""/></files><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
  <plugin name="Brighter"><description>For TVs in bright rooms and the Ally's screen</description><image path="fomod\\images\\brighter.png"/><files><folder source="10 Brighter" destination=""/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
 </plugins></group></optionalFileGroups></installStep>
 <installStep name="Patches"><optionalFileGroups order="Explicit"><group name="Next: patches for what you have" type="SelectAny"><plugins order="Explicit">
  <plugin name="Ordinator patch"><description>you have Ordinator 9.31 on</description><files><folder source="20 Ordinator" destination=""/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns><pattern><dependencies><fileDependency file="Ordinator - Perks of Skyrim.esp" state="Active"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
  <plugin name="Unofficial Skyrim SE Patch patch"><description>you have USSEP 4.3.2 on</description><files><folder source="20 USSEP" destination=""/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns><pattern><dependencies><fileDependency file="unofficial skyrim special edition patch.esp" state="Active"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
  <plugin name="Cathedral Weathers patch"><description>you have it on</description><files><folder source="20 Cathedral" destination=""/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns><pattern><dependencies><fileDependency file="Cathedral Weathers.esp" state="Active"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
  <plugin name="JK's Skyrim patch"><description>you don't have JK's Skyrim</description><files><folder source="20 JK" destination=""/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns><pattern><dependencies><fileDependency file="JKs Skyrim.esp" state="Missing"/></dependencies><type name="NotUsable"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
 </plugins></group></optionalFileGroups></installStep>
 <installStep name="Optional"><optionalFileGroups order="Explicit"><group name="Extras" type="SelectAny"><plugins order="Explicit">
  <plugin name="Lantern glow"><description>Lanterns light the road</description><files/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep>
</installSteps></config>""")

# ---- the download queue (Hearth Mods' own file): one ready, one part-way
dl = os.path.join(data, "mods/downloads/skyrimspecialedition")
w(os.path.join(dl, "Lux - main file 5.4.7z"), "7z")
q = [
    {"id": "d1", "domain": "skyrimspecialedition", "modId": 43158, "fileId": 1, "name": "Lux · main file 5.4", "fileName": "Lux - main file 5.4.7z", "size": 412000000, "done": 412000000, "state": "done", "dir": dl, "added": T, "finished": T},
    {"id": "d2", "domain": "skyrimspecialedition", "modId": 56000, "fileId": 2, "name": "Lux Orbis 2.1", "fileName": "Lux Orbis 2.1.7z", "size": 300000000, "done": 192000000, "state": "paused", "dir": dl, "added": T},
]
w(os.path.join(data, "mods/downloads.json"), json.dumps(q))

# ---- what the Mod Manager found and keeps (its own notes)
w(os.path.join(own, "updates.json"), json.dumps({"checked": T - 1200, "updates": [
    {"id": "u1", "game": "Skyrim Special Edition", "mod": "SkyUI", "from": "5.2", "to": "5.2.1", "source": "nexus", "domain": "skyrimspecialedition", "modId": 12604, "size": 2516582, "date": "2026-11-25", "tag": "crash", "picked": True,
     "changelog": ["Fixed a crash opening the Magic menu with more than 120 spells", "Fixed favourites not saving on some saves", "Works with SKSE 2.2.3 and newer"]},
    {"id": "u2", "game": "Skyrim Special Edition", "mod": "moreHUD", "from": "5.1.3", "to": "5.1.4", "source": "nexus", "size": 1153434, "date": "2026-11-22", "tag": "safe", "picked": True, "changelog": ["Shows enchantments known"]},
    {"id": "u3", "game": "Skyrim Special Edition", "mod": "Ordinator - Perks of Skyrim", "from": "9.31", "to": "10.0", "source": "nexus", "date": "2026-11-20", "tag": "new-game", "changelog": ["New perks"]},
    {"id": "u4", "game": "Skyrim Special Edition", "mod": "Community Shaders", "from": "1.1", "to": "1.2", "source": "nexus", "date": "2026-11-19", "tag": "safe", "changelog": []},
], "gone": [{"mod": "Rally's Fences", "date": "2026-11-20"}]}))
w(os.path.join(own, "devices.json"), json.dumps({"here": "Nvidia PC", "devices": [
    {"name": "ROG Ally", "games": "Skyrim SE, Stardew, BFME II", "profile": "Vanilla+ (Ally)", "state": "1 update"},
    {"name": "Laptop", "games": "Skyrim SE, Minecraft", "profile": "Survival playthrough", "state": ""},
    {"name": "Truck laptop", "games": "Stardew Valley", "profile": "Default", "state": "away"}]}))
w(os.path.join(own, "measured.json"), json.dumps({"settings": "1080p · 15 W · FSR on", "places": [{"name": "Whiterun", "fps": 62}, {"name": "Falkreath forest", "fps": 58}, {"name": "Markarth", "fps": 65}, {"name": "Interiors", "fps": 78}]}))
w(os.path.join(own, "describe.json"), json.dumps({"talk": [
    {"you": "Make Skyrim look realistic but keep 60 fps on my Ally"},
    {"ai": "Here's a set of 18 mods for the Ally. It swaps your 4K textures for 2K ones made for handhelds, adds Community Shaders with only the cheap effects on, real-looking grass that stops at a shorter distance, and a calmer weather mod. About 62 fps in Whiterun (you have 71 now) and 58 in the busiest forest, so I left the 60 fps cap on. It goes into a new profile; your Survival playthrough stays as it is."},
    {"you": "Keep my Survival mods in it"},
    {"ai": "Done: the 63 Gameplay mods come along, nothing of theirs clashes. Two of them needed a patch, which I added (now 20)."}],
    "set": {"profile": "Realistic (Ally)", "groups": [
        {"name": "Looks", "mods": [{"name": "Skyrim 2K textures for handhelds", "source": "Nexus Mods", "fps": -1, "size": "3.8 GB"}, {"name": "Majestic Mountains (lite)", "source": "Nexus Mods", "fps": 0, "size": "420 MB"}, {"name": "Realistic water, low", "source": "Nexus Mods", "fps": -1, "size": "310 MB"}]},
        {"name": "Light and weather", "mods": [{"name": "Community Shaders · cheap effects only", "source": "Nexus Mods", "fps": -3, "size": "40 MB"}, {"name": "Cathedral Weathers", "source": "Home server", "fps": 0, "size": "—"}, {"name": "Lux (you have it)", "source": "Home server", "fps": -1, "size": "—"}]},
        {"name": "Grass and trees", "mods": [{"name": "Grass for handhelds", "source": "Nexus Mods", "fps": -2, "size": "610 MB"}, {"name": "Grass shorter view distance", "source": "Nexus Mods", "fps": 1, "size": "1 MB"}, {"name": "Trees, 2K", "source": "Nexus Mods", "fps": -1, "size": "2.2 GB"}]},
        {"name": "Keeping it smooth", "mods": [{"name": "SSE Engine Fixes (you have it)", "source": "Home server", "fps": 1, "size": "—"}, {"name": "Display tweaks · 60 fps cap", "source": "Nexus Mods", "fps": 0, "size": "2 MB"}, {"name": "2 patches for your Survival mods", "source": "Nexus Mods", "fps": 0, "size": "6 MB"}]}],
        "why": ["Textures first, so Majestic Mountains' rocks win over the 2K pack's (your choice from Sunday)", "Lux after Cathedral Weathers: Lux's own page asks for it", "Grass view distance last in its group: it only changes a setting", "Plugins: Sort's rules place all 6 new ones · 0 missing masters"]}}))
print(home)
