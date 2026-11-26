# Hearth Retro (hosts/hearth-ui/apps/retro; board docs/mockups/open-source-apps/04-retro.png, checklist
# docs/mockups/checklists/open-source-04-retro.md): its pictures at the board's size (the screen is 1920 x 1080 for
# these, back to 1280 x 800 after), in Hearth's own look. A pretend library made for it in a home of its own: ROM files
# (empty) in ~/Hearth/Retro/roms (the board's 14 consoles, 1,284 games), tile art in ~/Hearth/Retro/art, save states,
# BIOS files (pretend ones: their sums are the fixture's good ones; the Saturn one is "wrong"), when Chrono Trigger was
# last played. HEARTH_RETRO_FIXTURE says which emulators are here, the controllers and RetroAchievements (nothing is
# installed, nothing reads a real controller). Sourced by hearth-apps-rust-test.sh inside its headless screen.
RH=/tmp/oss-retro-home
rm -rf "$RH"; mkdir -p "$RH"
python3 - "$RH" <<'PY' || echo "::error title=oss-retro fixture::it couldn't be made"
import hashlib, json, os, sys, time
from PIL import Image
home = sys.argv[1]
R = os.path.join(home, "Hearth", "Retro")
board = {
    "snes": ("sfc", 312, [("Chrono Trigger", "#4a7ad8", "#1a2a5a"), ("Super Metroid", "#c84a3a", "#2a0d0a"), ("A Link to the Past", "#4aa85a", "#0d2a14"), ("Donkey Kong Country", "#c8902a", "#3a2008"),
                          ("Secret of Mana", "#3ab0a8", "#0d2a28"), ("F-Zero", "#d84aa8", "#2a0d24"), ("Super Mario World", "#e8c04a", "#3a2a08"), ("EarthBound", "#8ad84a", "#1a2a0d")]),
    "psx": ("chd", 204, [("Final Fantasy VII", "#5a8ac8", "#0d1a2a"), ("Crash Bandicoot", "#e8802a", "#3a1a08"), ("Spyro the Dragon", "#a85ad8", "#200d2a"), ("Metal Gear Solid", "#6a7a6a", "#141a14"),
                         ("Tekken 3", "#c83a3a", "#2a0808"), ("Gran Turismo", "#3a6aa8", "#0d1424"), ("Castlevania: SotN", "#7a3a8a", "#1a0a20"), ("Resident Evil 2", "#6a5a4a", "#1a1410")]),
    "n64": ("z64", 96, [("GoldenEye 007", "#8a8a5a", "#2a2a14"), ("Mario Kart 64", "#e84a4a", "#3a0d0d"), ("Ocarina of Time", "#4ac87a", "#0d2a18"), ("Star Fox 64", "#4a8ad8", "#0d1a3a"),
                        ("Banjo-Kazooie", "#e8a84a", "#3a2a0d"), ("Paper Mario", "#d85a8a", "#2a0d1a"), ("F-Zero X", "#5ad8d8", "#0d2a2a"), ("Super Mario 64", "#d84a4a", "#1a0d3a")]),
}
others = {"gba": ("gba", 188), "gc": ("rvz", 42), "ps2": ("chd", 61), "dreamcast": ("chd", 28), "arcade": ("zip", 310), "nes": ("nes", 8), "genesis": ("md", 8),
          "psp": ("cso", 7), "nds": ("nds", 7), "saturn": ("chd", 7), "wii": ("wbfs", 6)}
def touch(p):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    open(p, "w").close()
def tile(path, a, b):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    ca = tuple(int(a[i:i + 2], 16) for i in (1, 3, 5)); cb = tuple(int(b[i:i + 2], 16) for i in (1, 3, 5))
    im = Image.new("RGB", (300, 400)); px = im.load()
    for y in range(400):
        for x in range(300):
            t = min(1.0, max(0.0, (y * 0.94 + x * 0.34) / 470.0))
            hl = max(0.0, 1.0 - (((x - 225) ** 2 + (y - 100) ** 2) ** 0.5) / 140.0) * 0.18
            px[x, y] = tuple(min(255, int(ca[i] + (cb[i] - ca[i]) * t + 255 * hl)) for i in range(3))
    im.save(path, "PNG")
now = int(time.time())
played = {}
for sysid, (ext, n, games) in board.items():
    for k, (title, a, b) in enumerate(games):
        f = os.path.join(R, "roms", sysid, "%s (USA).%s" % (title, ext))
        touch(f)
        tile(os.path.join(R, "art", sysid, "%s (USA).png" % title), a, b)
        # the board's order on each shelf: the most recently played first
        played[f] = {"last": now - 3 * 86400 - k * 3600 - (0 if sysid == "snes" else 86400 * 20), "count": 3}
    for k in range(n - len(games)):
        touch(os.path.join(R, "roms", sysid, "Game %03d (USA).%s" % (k + 1, ext)))
for sysid, (ext, n) in others.items():
    for k in range(n):
        touch(os.path.join(R, "roms", sysid, "Game %03d (Europe).%s" % (k + 1, ext)))
os.makedirs(os.path.join(home, ".local/share/hearth/retro"), exist_ok=True)
json.dump(played, open(os.path.join(home, ".local/share/hearth/retro/played.json"), "w"))
states = os.path.join(R, "states")
for s in ("", "1", "2", ".auto"):
    touch(os.path.join(states, "Chrono Trigger (USA).state" + s))
bios = os.path.join(R, "bios")
sums = {}
def b(name, console, good=True):
    p = os.path.join(bios, name)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    data = ("pretend BIOS " + name).encode()
    open(p, "wb").write(data)
    if good:
        sums.setdefault(console, []).append(hashlib.md5(data).hexdigest())
b("scph5501.bin", "PlayStation"); b("sega_101.bin", "Saturn", good=False); b("dc/dc_boot.bin", "Dreamcast"); b("dc/dc_flash.bin", "Dreamcast flash")
b("gba_bios.bin", "Game Boy Advance"); b("bios7.bin", "Nintendo DS (ARM7)"); b("bios9.bin", "Nintendo DS (ARM9)"); b("bios_CD_U.bin", "Sega CD")
b("syscard3.pce", "PC Engine CD"); b("neogeo.zip", "Neo Geo")
fx = {"installed": ["RetroArch", "Snes9x", "DuckStation", "PCSX2", "Dolphin", "PPSSPP", "Flycast", "melonDS", "mGBA", "Mupen64Plus", "Beetle Saturn", "Cemu", "ScummVM", "MAME"],
      "pads": [{"name": "Xbox Wireless Controller"}, {"name": "8BitDo SN30 Pro", "battery": 70}], "battery": 82, "biosSums": sums, "ra": {"chrono trigger": [31, 77]}}
json.dump(fx, open(os.path.join(home, "fixture.json"), "w"))
PY
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1
look hearth dark
mkdir -p "$RH/.local/share/nimbus" && cp "$HOME/.local/share/nimbus/desktop.json" "$RH/.local/share/nimbus/desktop.json"
RX="$RH/fixture.json"
RA="$BIN/hearth-retro-app"
retro() { # NAME ARGS...
  n=$1; shift
  SHOT_WAIT=3.5 run "oss-retro-$n" hearth-retro "${REST:-0}" env HOME="$RH" HEARTH_RETRO_FIXTURE="$RX" XDG_CONFIG_HOME="$RH/.config" "$@"
}
REST=20 retro 1-shelves "$RA" --game-mode --pick "Chrono Trigger"
retro 2-where-to-put-it "$RA" --game-mode --pick "Chrono Trigger" --show where
retro 3-why "$RA" --game-mode --pick "Chrono Trigger" --show why
retro 4-load-a-save "$RA" --game-mode --pick "Chrono Trigger" --show saves
retro 5-options "$RA" --game-mode --pick "Chrono Trigger" --show options
retro 6-set-up "$RA" --game-mode --page setup
retro 7-bios-check "$RA" --game-mode --page bios
retro 8-search "$RA" --game-mode --page search --search Mario
retro 9-in-a-window "$RA" --pick "Super Metroid"
retro 10-edit-layout env HEARTH_ARRANGE_OPEN=retro:shelves HEARTH_STUDIO_STAGE=retro:none "$RA" --game-mode --pick "Chrono Trigger"
retro 11-customize env HEARTH_ARRANGE_OPEN=retro:window HEARTH_STUDIO_STAGE=retro:customize "$RA" --game-mode --pick "Chrono Trigger"
# Opening it wrote no setting of its own (red flag 9): retro.json only when something is changed.
[ -e "$RH/.config/hearth/retro.json" ] && echo "::error title=oss-retro::opening Hearth Retro wrote retro.json: $(head -c 300 "$RH/.config/hearth/retro.json")"
grep -h "Would start\|error" /tmp/oss-retro-*.log 2>/dev/null | head -3 | while read -r l; do echo "::notice title=oss-retro log::$l"; done
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
look steam dark
