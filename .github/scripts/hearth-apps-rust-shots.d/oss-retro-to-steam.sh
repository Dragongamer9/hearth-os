# Retro to Steam (hosts/hearth-ui/apps/retro-to-steam; board docs/mockups/open-source-apps/05-retro-to-steam.png,
# checklist docs/mockups/checklists/open-source-05-retro-to-steam.md): its pictures at the board's size (1920 x 1080,
# back to 1280 x 800 after), in Hearth's own look, on a pretend home of its own: Hearth Retro's ROM folder (the board's
# eight consoles, empty files), a pretend Steam's own files (userdata/<id>/config/shortcuts.vdf with 61 games already
# in it, config/loginusers.vdf) and HEARTH_R2S_FIXTURE's art counts (no art site is asked). Then --add writes the
# new games into that pretend shortcuts.vdf for real, and the file is read back (its .bak kept). No real Steam, no
# account, nothing signed in to. Sourced by hearth-apps-rust-test.sh inside its headless screen.
SH=/tmp/oss-r2s-home
rm -rf "$SH"; mkdir -p "$SH"
python3 - "$SH" <<'PY' || echo "::error title=oss-r2s fixture::it couldn't be made"
import json, os, struct, sys, time
home = sys.argv[1]
R = os.path.join(home, "Hearth", "Retro", "roms")
shown = ["Chrono Trigger", "Super Metroid", "A Link to the Past", "Donkey Kong Country", "Secret of Mana", "Star Ocean", "F-Zero", "Super Mario World", "EarthBound", "Mega Man X", "Kirby Super Star", "Tactics Ogre"]
consoles = [("snes", "sfc", 312), ("psx", "chd", 204), ("n64", "z64", 96), ("gba", "gba", 188), ("gc", "rvz", 42), ("ps2", "chd", 61), ("dreamcast", "chd", 28), ("arcade", "zip", 310)]
def touch(p):
    os.makedirs(os.path.dirname(p), exist_ok=True); open(p, "w").close()
now = int(time.time()); played = {}
for sysid, ext, n in consoles:
    names = shown if sysid == "snes" else []
    for k, t in enumerate(names):
        f = os.path.join(R, sysid, "%s (USA).%s" % (t, ext)); touch(f)
        played[f] = {"last": now - k * 3600, "count": 1}
    for k in range(n - len(names)):
        touch(os.path.join(R, sysid, "Game %03d (USA).%s" % (k + 1, ext)))
os.makedirs(os.path.join(home, ".local/share/hearth/retro"), exist_ok=True)
json.dump(played, open(os.path.join(home, ".local/share/hearth/retro/played.json"), "w"))
# a pretend Steam: one person, 61 SNES games already added (binary shortcuts.vdf, as Steam writes it)
S = os.path.join(home, ".local/share/Steam")
uid = "123456789"
os.makedirs(os.path.join(S, "userdata", uid, "config", "grid"), exist_ok=True)
os.makedirs(os.path.join(S, "config"), exist_ok=True)
open(os.path.join(S, "config/loginusers.vdf"), "w").write('"users"\n{\n\t"%d"\n\t{\n\t\t"AccountName"\t\t"player"\n\t\t"PersonaName"\t\t"Player"\n\t}\n}\n' % (76561197960265728 + int(uid)))
def s(k, v): return b"\x01" + k.encode() + b"\x00" + v.encode() + b"\x00"
def i(k, v): return b"\x02" + k.encode() + b"\x00" + struct.pack("<i", v)
body = b"\x00shortcuts\x00"
for k in range(61):
    body += b"\x00" + str(k).encode() + b"\x00" + i("appid", -1000 - k) + s("AppName", "Game %03d" % (k + 1)) + s("Exe", '"/usr/bin/snes9x"') + s("LaunchOptions", "") + b"\x00tags\x00\x08" + b"\x08"
body += b"\x08\x08"
open(os.path.join(S, "userdata", uid, "config", "shortcuts.vdf"), "wb").write(body)
# Retro to Steam's own choices, as the person left them: PlayStation 2 and Arcade off, Super Metroid's 2nd grid, 3rd hero
os.makedirs(os.path.join(home, ".config/hearth"), exist_ok=True)
sm = os.path.join(R, "snes", "Super Metroid (USA).sfc")
json.dump({"off": ["ps2", "arcade"], "picks": {sm: {"grid": 1, "hero": 2}}}, open(os.path.join(home, ".config/hearth/retro-to-steam.json"), "w"))
art = {"*": {"grid": 1, "wide": 1, "hero": 1}, "Chrono Trigger": {"grid": 6, "wide": 3, "hero": 5, "logo": 1, "icon": 1}, "Super Metroid": {"grid": 8, "wide": 4, "hero": 9, "logo": 2, "icon": 1},
       "A Link to the Past": {"grid": 11, "wide": 2, "hero": 3}, "Donkey Kong Country": {"grid": 5}, "Secret of Mana": {"grid": 4}, "F-Zero": {"grid": 3}, "Super Mario World": {"grid": 9},
       "EarthBound": {"grid": 7}, "Mega Man X": {"grid": 6}, "Kirby Super Star": {"grid": 4}, "Star Ocean": {}, "Tactics Ogre": {}}
for k in range(12):
    art["Game %03d" % (100 + k)] = {}
json.dump({"art": art, "steamRunning": False}, open(os.path.join(home, "fixture.json"), "w"))
json.dump({"art": art, "steamRunning": True}, open(os.path.join(home, "fixture-open.json"), "w"))
PY
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1
look hearth dark
mkdir -p "$SH/.local/share/nimbus" && cp "$HOME/.local/share/nimbus/desktop.json" "$SH/.local/share/nimbus/desktop.json"
R2="$BIN/hearth-retro-to-steam-app"
r2s() { # NAME FIXTURE ARGS...
  n=$1; fx=$2; shift 2
  SHOT_WAIT=3.5 run "oss-r2s-$n" hearth-retro-to-steam "${REST:-0}" env HOME="$SH" HEARTH_R2S_FIXTURE="$SH/$fx" XDG_CONFIG_HOME="$SH/.config" "$@"
}
REST=20 r2s 1-board fixture.json "$R2" --pick "Super Metroid"
r2s 2-close-steam-first fixture-open.json "$R2" --pick "Super Metroid" --show close-steam
r2s 3-preview-only fixture.json "$R2" --pick "Super Metroid" --show preview
r2s 4-art-sources fixture.json "$R2" --pick "Super Metroid" --show sources
r2s 5-edit-layout fixture.json env HEARTH_ARRANGE_OPEN=retro-to-steam:window HEARTH_STUDIO_STAGE=retro-to-steam:none "$R2" --pick "Super Metroid"
r2s 6-customize fixture.json env HEARTH_ARRANGE_OPEN=retro-to-steam:games HEARTH_STUDIO_STAGE=retro-to-steam:customize "$R2" --pick "Super Metroid"
# Adding for real, into the pretend Steam: the new games go in, the 61 there stay, the old file is kept as .bak.
V="$SH/.local/share/Steam/userdata/123456789/config/shortcuts.vdf"
if env HOME="$SH" HEARTH_R2S_FIXTURE="$SH/fixture.json" "$R2" --add > /tmp/oss-r2s-add.log 2>&1; then
  python3 - "$V" <<'PY'
import sys
b = open(sys.argv[1], "rb").read()
n = b.count(b"\x01AppName\x00")
ok = b.count(b"hearth-retro-app") > 0 and n > 61
print(("::notice title=oss-r2s add::" if ok else "::error title=oss-r2s add::") + "shortcuts.vdf now has %d games (61 were there); %d start in Hearth Retro" % (n, b.count(b"--play")))
PY
  [ -s "$V.bak" ] && echo "::notice title=oss-r2s add::the old shortcuts.vdf was kept as shortcuts.vdf.bak ($(stat -c %s "$V.bak") bytes); $(head -c 200 /tmp/oss-r2s-add.log)" || echo "::error title=oss-r2s add::no shortcuts.vdf.bak"
  REST=0 r2s 7-after-adding fixture.json "$R2" --pick "Super Metroid"
else
  echo "::error title=oss-r2s add::--add failed: $(tail -c 600 /tmp/oss-r2s-add.log | tr '\n' '|')"
fi
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
look steam dark
