#!/bin/bash
# The Firehold shell (Couch and Desk) on Firehold's own compositor (firehold.yml "shell"): firehold-comp headless at
# 1920 x 1080, started by nimbus-session with the preview on (HEARTH_DESKTOP_WM=firehold), so Mantel runs with
# HEARTH_WM=firehold exactly as run_firehold starts it on a device, and the Firehold shell unlocked. The library and
# apps are firehold-shell.yml's stand-ins (firehold-fixtures.py); the game is the stand-in game
# (firehold-game-client.py) fullscreen as "Ember Trails", with the Guide's frame limit and scaling chosen through
# hearth-firehold-game. Pictures through screencopy (grim):
#   firehold-c1-couch-home      Couch Home
#   firehold-c2-library         Couch Library
#   firehold-c3-guide           the Guide over the fullscreen game: Frame limit ‹ 60 ›, Resolution scaling ‹ Quality · 720p ›,
#                               and VRR / FreeSync and Allow tearing as the game's own rows (game.rs), not "Needs KWin"
#   firehold-c4-everything-open Everything open with each window's real picture (the compositor's window shots, ipc.rs)
#   firehold-c5-desk            the Desk: Firehold's bar and dock, the apps' windows with Firehold's own title bars drawn by
#                               the compositor (frames.rs), and the game in a window with the accent border and its chip
#   firehold-c6-desktop-2       a second virtual desktop (the compositor's, through Mantel's WinList): Discord alone
#   firehold-c7-dock-2x         the Desk's dock at the Ally's scale (2.1, set the way Settings does, through wlr-randr),
#                               cropped at 100%: icons drawn at the screen's pixels
#   firehold-c8-home-2x         the Couch's Home tiles at 2.1, cropped at 100%
# Mantel's window list comes from the compositor's window IPC (hearth-firehold-bridge), as it does from KWin's script.
# It also says, as notices, what still falls back here. Any QML error in Mantel's log fails it.
set -u +e +o pipefail
mkdir -p shots
SRC=$PWD
BIN=${FIREHOLD:-$SRC/firehold-comp}
chmod +x "$BIN" .github/scripts/firehold-game-client.py hosts/linux/desktop/shell/hearth-firehold-game
fail() { echo "$1: $2" >> shots/errors-shell.txt; touch shots/FAILED; }
note() { echo "$1: $2" >> shots/notes-shell.txt; }
report() {   # GitHub keeps 10 notices a step: gathered and posted together
  python3 - <<'PY'
for kind, f, n in (('error', 'shots/errors-shell.txt', 4), ('notice', 'shots/notes-shell.txt', 5)):
    try:
        lines = [l.rstrip() for l in open(f) if l.strip()]
    except OSError:
        continue
    chunks, cur = [], ''
    for l in lines:
        l = l[:900].replace('%', '%25')
        if len(cur) + len(l) > 3500:
            chunks.append(cur); cur = ''
        cur += l + '%0A'
    if cur:
        chunks.append(cur)
    for i, c in enumerate(chunks[:n], 1):
        print('::%s title=shell on firehold-comp (%d)::%s' % (kind, i, c))
PY
}
export XDG_RUNTIME_DIR=/tmp/xdg-shell HOME=/tmp/home-shell
rm -rf "$XDG_RUNTIME_DIR" "$HOME"
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export LANG=C.UTF-8 LC_ALL=C.UTF-8 QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1 QT_FORCE_STDERR_LOGGING=1
export NIMBUS_SHARE="$SRC/hosts/linux" FIREHOLD_LOG="info,smithay=warn"
unset DISPLAY WAYLAND_DISPLAY
chmod +x hosts/linux/desktop/labwc/autostart hosts/linux/desktop/firehold/autostart hosts/linux/desktop/theme/apply.sh
mkdir -p /tmp/fakebin-shell
python3 .github/scripts/firehold-fixtures.py /tmp/fakebin-shell && chmod +x /tmp/fakebin-shell/*
export PATH="/tmp/fakebin-shell:$PATH" NIMBUS_APP=/tmp/fakebin-shell/nimbus
mkdir -p "$HOME/.cache/hearth" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$HOME/.local/state/hearth"
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/share/nimbus/desktop.json" '{}' >/dev/null
python3 hosts/linux/desktop/shell/hearth-browser asked >/dev/null 2>&1 || true
# The test's own files: Desktop Mode on Firehold, the Firehold shell unlocked, its first-time card already shown.
CONF="$HOME/.config/nimbus/session.conf"; FHJ="$HOME/.config/nimbus/firehold.json"   # the test's own files
printf 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=firehold\n' > "$CONF"
printf '{"firehold": {"unlocked": true}}\n' > "$FHJ"
: > "$HOME/.local/state/hearth/firehold-intro-told"
LOG="$HOME/.local/share/nimbus/session.log"
SHLOG="$XDG_RUNTIME_DIR/nimbus-shell.log"
SH="$NIMBUS_SHARE/desktop/shell"
W() { WAYLAND_DISPLAY=wayland-hearth "$@"; }
q() { W timeout 10 quickshell ipc -p "$SH" call "$@" 2>/dev/null | grep -v WARN | tail -n1; }
shot() { sleep "${2:-3}"; W timeout 20 grim "shots/$1.png" 2>"shots/$1.err" || fail "pictures" "grim failed for $1: $(head -c 300 "shots/$1.err")"; }
alike() {   # the mean difference of two pictures, 0-255
  python3 -c 'import sys; from PIL import Image, ImageChops, ImageStat; a, b = (Image.open(p).convert("L") for p in sys.argv[1:3]); print(round(ImageStat.Stat(ImageChops.difference(a, b.resize(a.size))).mean[0], 2))' "$1" "$2" 2>/dev/null || echo error
}
GH() { W python3 "$SH/hearth-firehold-game" "$@"; }
gst() { GH status | python3 -c 'import json,sys; d=json.load(sys.stdin); g=d.get("game") or {}; print(eval(sys.argv[1], {"d": d, "g": g}))' "$1" 2>/dev/null; }
fst() { q firehold state | python3 -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1], {"d": d}))' "$1" 2>/dev/null; }

FIREHOLD_BIN="$BIN" FIREHOLD_ARGS="--headless --size 1920x1080 --socket wayland-hearth" HEARTH_KWIN_BIN=no-kwin-here \
  sh hosts/linux/session/nimbus-session >shots/session-out.log 2>&1 &
SPID=$!
for i in $(seq 1 90); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 1; done
if [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ]; then
  note "Mantel" "up on firehold-comp headless ($(grep -o "firehold: Mantel up after [0-9]*s" "$LOG" | tail -1))"
else
  fail "Mantel" "didn't come up: $(tail -5 "$LOG" | tr '\n' '|' | cut -c1-400) compositor: $(tail -c 800 "$HOME/.local/share/nimbus/firehold.log" 2>/dev/null | tr '\n' '|')"
  report; exit 1
fi
grep -q "desktop mode: Mantel on Firehold (preview)" "$LOG" || fail "session" "nimbus-session didn't start Firehold: $(tail -5 "$LOG" | tr '\n' '|' | cut -c1-400)"
for i in $(seq 1 60); do [ -n "$(q firehold state)" ] && break; sleep 2; done
for i in $(seq 1 20); do [ "$(fst 'd["unlocked"]')" = True ] && break; sleep 1; done
[ "$(fst 'd["unlocked"]')" = True ] || fail "firehold" "the Firehold shell isn't unlocked: $(q firehold state | head -c 400)"
for i in $(seq 1 30); do [ "$(fst 'd["games"] >= 10')" = True ] && break; sleep 2; done
note "firehold" "unlocked, $(fst 'd["games"]') games in the library (the stand-ins)"
wm=$(q winlist state | python3 -c 'import json,sys; print(json.load(sys.stdin)["wm"])' 2>/dev/null)
[ "$wm" = firehold ] && note "session" "Mantel runs with HEARTH_WM=firehold" || fail "session" "Mantel's window manager is '$wm', not firehold"

# The apps' windows (foot stands in for each), for Everything open and the Desk.
win() { W foot --app-id="$1" -T "$2" -W 80x20 sh -c 'sleep 100000' >/dev/null 2>&1 & }
win org.hearth.files "Pictures › Trips — Hearth Files"; sleep 1
win firefox "3 tabs — Firefox"; sleep 1
win discord "2 new messages — Discord"; sleep 3

# Mantel's window list from the compositor's window IPC (firehold-comp ipc.rs, through hearth-firehold-bridge).
CTL() { W timeout 10 "$BIN" ctl "$@"; }
wst() { q winlist state | python3 -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1], {"d": d}))' "$1" 2>/dev/null; }
for i in $(seq 1 30); do [ "$(wst 'd.get("bridged") and len(d["windows"]) >= 3')" = True ] && break; sleep 1; done
if [ "$(wst 'd.get("bridged") and len(d["windows"]) >= 3')" = True ]; then
  note "window list" "Mantel's WinList takes Firehold's window IPC: $(wst '", ".join("%s #%s (stack %s, desktop %s)" % (w["appId"], w["id"], w["stack"], w["desktops"]) for w in d["windows"])'); desktops: $(wst 'len(d["desktops"])')"
else
  fail "window list" "Mantel's WinList isn't on Firehold's window IPC: $(q winlist state | head -c 400) bridge: $(CTL status 2>&1 | head -c 300)"
fi
CTL windows > shots/ipc-windows.json 2>&1
python3 - shots/ipc-windows.json <<'PY' || fail "window ipc" "firehold-comp ctl windows: $(head -c 400 shots/ipc-windows.json)"
import json, sys
d = json.loads(open(sys.argv[1]).read().strip().splitlines()[-1])
ws = d["windows"]
assert len(ws) >= 3 and all(w["id"].isdigit() and w["frame"].get("w", 0) > 0 for w in ws), ws
assert sorted(d["stack"]) == sorted(w["id"] for w in ws if not w["minimized"]), d["stack"]
assert d["desktops"] and sum(1 for x in d["desktops"] if x["active"]) == 1, d["desktops"]
PY

# 1-2. The Couch: Home, the Library
q firehold couch; sleep 4
q firehold tab home; shot firehold-c1-couch-home 5
q firehold tab library; shot firehold-c2-library 4
q firehold tab home

# 3. The Guide over the game, its game rows live
W python3 .github/scripts/firehold-game-client.py "$XDG_RUNTIME_DIR/game-stats.json" --app-id steam_app_1000 --title "Ember Trails" --seconds 300 \
  >shots/shell-game.log 2>&1 &
GAMEPID=$!
for i in $(seq 1 40); do [ "$(gst 'g.get("appId") == "steam_app_1000"')" = True ] && break; sleep 0.5; done
[ "$(gst 'g.get("appId") == "steam_app_1000"')" = True ] || fail "game" "the stand-in game isn't Firehold's fullscreen game: $(GH status | head -c 400) $(tail -c 300 shots/shell-game.log | tr '\n' '|')"
GH set frameLimit 60 >/dev/null; GH set scaling quality >/dev/null; GH set vrr on >/dev/null
for i in $(seq 1 30); do [ "$(gst 'g.get("frameLimit") == 60 and g.get("scaling") == "quality"')" = True ] && break; sleep 0.5; done
# Allow tearing (game.rs): with no frame limit the game doesn't wait for the screen; with one, the limit comes first.
GH set frameLimit off >/dev/null; GH set tearing on >/dev/null
for i in $(seq 1 20); do [ "$(gst 'g.get("tearing") is True and g.get("tearingActive") is True')" = True ] && break; sleep 0.5; done
u0=$(gst 'g.get("unpacedFrames", 0)'); sleep 2; u1=$(gst 'g.get("unpacedFrames", 0)')
if [ "$(gst 'g.get("tearingActive") is True')" = True ] && [ "${u1:-0}" -gt "${u0:-0}" ] 2>/dev/null; then
  note "tearing" "Allow tearing on: the game drew $(( u1 - u0 )) frames in 2 s without waiting for the screen"
else
  fail "tearing" "Allow tearing didn't take: $(GH status | head -c 500) (frames $u0 → $u1)"
fi
GH set frameLimit 60 >/dev/null
for i in $(seq 1 30); do [ "$(gst 'g.get("frameLimit") == 60 and g.get("scaling") == "quality" and g.get("tearingActive") is False')" = True ] && break; sleep 0.5; done
fl=$(gst 'g.get("frameLimitLabel")'); sc=$(gst 'g.get("scalingLabel")')
q firehold go "Ember Trails"; sleep 3
shot shell-game-only 2
q firehold open guide; shot firehold-c3-guide 6
case "$fl|$sc" in
  "60|Quality · "[0-9]*p) note "guide" "the rows: Frame limit ‹ $fl ›, Resolution scaling ‹ $sc › (the game draws at $(gst 'g.get("renderSize")'))";;
  *) fail "guide" "the helper's labels aren't real values: frame limit '$fl', scaling '$sc': $(GH status | head -c 400)";;
esac
[ "$(fst 'd["guide"]')" = True ] || fail "guide" "the Guide isn't up: $(q firehold state | head -c 400)"
# Shown, not only open in Mantel's state: the picture has to differ from the game on its own (a fullscreen game drawn
# alone, without the overlay layer above it, would look the same).
shown() {   # $1 picture, $2 what
  d=$(alike "shots/$1.png" shots/shell-game-only.png)
  awk -v d="$d" 'BEGIN { exit !(d ~ /^[0-9.]+$/ && d + 0 > 4) }' && note "$2" "on screen over the game (difference $d from the game alone)" \
    || fail "$2" "not on screen: the picture is the fullscreen game alone (difference $d): firehold-comp draws no layer surfaces over a fullscreen window"
}
shown firehold-c3-guide "guide"
# The Guide's rows as Mantel shows them: VRR and Allow tearing are the game's own here, never "Needs KWin".
rows=$(fst '"; ".join("%s ‹ %s ›%s" % (r["name"], r["value"], "" if r["works"] else " [not working]") for r in d["guideRows"])')
note "guide" "rows: $rows"
[ "$(fst 'any(r["name"] == "VRR / FreeSync" and r["works"] and r["value"] in ("On", "Off") for r in d["guideRows"])')" = True ] \
  || fail "guide" "the VRR row isn't the game's own: $rows"
[ "$(fst 'any(r["name"] == "Allow tearing" and r["works"] and r["value"] == "On" for r in d["guideRows"])')" = True ] \
  || fail "guide" "the Allow tearing row isn't real (On, from the game's choice): $rows"
q firehold press b; sleep 1
GH set tearing off >/dev/null

# 4. Everything open, with each window's picture (the compositor's window shots, every 3 seconds while it's up)
q firehold open switcher
for i in $(seq 1 20); do [ "$(fst 'd["pictures"] >= 3')" = True ] && break; sleep 1; done
shot firehold-c4-everything-open 4
shown firehold-c4-everything-open "everything open"
pics=$(fst 'd["pictures"]')
if [ "${pics:-0}" -ge 3 ] 2>/dev/null; then
  note "window pictures" "$pics windows' pictures for Everything open ($(ls "$XDG_RUNTIME_DIR/hearth-firehold/pictures" 2>/dev/null | tr '\n' ' '))"
  for f in "$XDG_RUNTIME_DIR"/hearth-firehold/pictures/*.png; do
    python3 -c 'import sys; from PIL import Image; im = Image.open(sys.argv[1]).convert("RGB"); print("%dx%d, %d colours" % (im.size + (len(im.getcolors(1 << 24) or []),))); sys.exit(0 if len(im.getcolors(1 << 24) or []) > 4 else 1)' "$f" >> shots/pictures.txt 2>&1 \
      || fail "window pictures" "$(basename "$f") is empty or one colour: $(tail -1 shots/pictures.txt)"
    cp "$f" "shots/window-$(basename "$f")" 2>/dev/null
  done
  note "window pictures" "$(tr '\n' ';' < shots/pictures.txt)"
else
  fail "window pictures" "Everything open has $pics pictures (3 wanted): $(q firehold state | head -c 300) ipc: $(CTL status 2>&1 | head -c 300)"
fi
[ "$(fst 'd["page"] == "switcher" and len(d["open"]) >= 4')" = True ] || fail "everything open" "not up with the game and the three apps: $(q firehold state | head -c 500)"
note "everything open" "$(fst '", ".join("%s (%s)" % (o["name"], o["kind"]) for o in d["open"])')"
q firehold press b; sleep 1

# 5. The Desk (the game ended first: fullscreen it would cover the Desk), then the game again in a window, with the
# compositor's own frame (it asks for it, like a game in a window does) in Firehold's look.
GH set frameLimit off >/dev/null; GH set scaling off >/dev/null
kill "$GAMEPID" 2>/dev/null; pkill -f firehold-game-client 2>/dev/null; sleep 2   # $! is the W wrapper
q firehold desk; sleep 2
W python3 .github/scripts/firehold-game-client.py "$XDG_RUNTIME_DIR/desk-game.json" --app-id steam_app_1000 --title "Ember Trails" --seconds 200 --windowed --ssd \
  >shots/desk-game.log 2>&1 &
for i in $(seq 1 30); do [ "$(CTL status | python3 -c 'import json,sys; print(json.loads(sys.stdin.read().splitlines()[-1])["frames"])' 2>/dev/null)" = firehold ] && break; sleep 1; done
fr=$(CTL status | python3 -c 'import json,sys; print(json.loads(sys.stdin.read().splitlines()[-1])["frames"])' 2>/dev/null)
[ "$fr" = firehold ] && note "frames" "the compositor draws Firehold's title bars (Mantel named the windows)" \
  || fail "frames" "the compositor's title bars aren't Firehold's on the Desk: '$fr' $(CTL status 2>&1 | head -c 300)"
sleep 4
q winlist act "Ember Trails" activate; sleep 3
shot firehold-c5-desk 2
python3 - shots/firehold-c5-desk.png <<'PY' || fail "frames" "Firehold's title bars aren't in the Desk picture: $(cat shots/frames-check.txt 2>/dev/null)"
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB')
# The accent (#ff8a3d): the close buttons and the game's border; the panel colour (#17110e) of the bars.
acc = sum(1 for p in im.getdata() if abs(p[0] - 255) < 18 and abs(p[1] - 138) < 24 and abs(p[2] - 61) < 30)
open('shots/frames-check.txt', 'w').write('accent pixels %d' % acc)
print('accent pixels', acc)
sys.exit(0 if acc > 400 else 1)
PY
python3 -c 'import sys; from PIL import Image; im = Image.open(sys.argv[1]).convert("RGB"); sys.exit(0 if im.getpixel((1700, 900)) != (0, 0, 0) else 1)' shots/firehold-c5-desk.png \
  || fail "desk" "the Desk picture is black where Mantel's wallpaper should be: the ended game is still drawn as fullscreen"
[ "$(fst 'd["posture"]')" = desk ] || fail "desk" "not on the Desk: $(q firehold state | head -c 400)"

# 6. A second virtual desktop (the compositor's own, ipc.rs), the way Mantel's taskbar and desktop buttons do it:
# Discord sent there through WinList, then Mantel goes there; the search's "desktops" plugin lists them all.
q winlist run '{"op":"desktop","title":"2 new messages — Discord","n":2}' >/dev/null
q winlist run '{"op":"goDesktop","n":2}' >/dev/null
for i in $(seq 1 20); do [ "$(wst 'd["desktops"][1]["active"] is True')" = True ] && break; sleep 0.5; done
shot firehold-c6-desktop-2 3
if [ "$(wst 'd["desktops"][1]["active"] is True and [w["desktops"] for w in d["windows"] if w["appId"] == "discord"] == [[2]]')" = True ]; then
  note "desktops" "on desktop 2 of $(wst 'len(d["desktops"])') with Discord: $(wst '[(w["appId"], w["desktops"], w["stack"]) for w in d["windows"]]')"
else
  fail "desktops" "desktop 2 with Discord didn't come: $(q winlist state | head -c 500)"
fi
d=$(alike shots/firehold-c6-desktop-2.png shots/firehold-c5-desk.png)
awk -v d="$d" 'BEGIN { exit !(d ~ /^[0-9.]+$/ && d + 0 > 2) }' && note "desktops" "desktop 2 looks different from desktop 1 (difference $d)" \
  || fail "desktops" "desktop 2's picture is desktop 1's (difference $d)"
[ "$(wst 'len(d["desktops"]) >= 2')" = True ] || fail "desktops" "Mantel has no desktops for its desktop buttons and the search's desktops plugin"
q winlist run '{"op":"goDesktop","n":1}' >/dev/null; sleep 1
[ "$(wst 'd["desktops"][0]["active"] is True')" = True ] || fail "desktops" "back to desktop 1 didn't work"
pkill -f firehold-game-client 2>/dev/null

# 7. The Ally's scale, 2.1 (wlr-randr sends it as 2.09765625; the compositor snaps it to what apps are told, 252/120):
# the dock and Home's tiles must be drawn at the screen's own pixels, not stretched.
if command -v wlr-randr >/dev/null; then
  out=$(W wlr-randr | head -1 | cut -d' ' -f1)
  W timeout 10 wlr-randr --output "$out" --scale 2.1 2>shots/wlr-randr.err
  for i in $(seq 1 20); do [ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["outputs"][0]["scale"])' "$XDG_RUNTIME_DIR/firehold.json" 2>/dev/null)" = 2.1 ] && break; sleep 0.5; done
  sc=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["outputs"][0]["scale"])' "$XDG_RUNTIME_DIR/firehold.json" 2>/dev/null)
  [ "$sc" = 2.1 ] && note "scale" "2.1 exactly (what apps are told in 1/120ths), not wlr-randr's 2.09765625" || fail "scale" "the screen's scale is $sc, not exactly 2.1"
  sleep 6
  shot firehold-scale-desk 2
  python3 - <<'PY'
from PIL import Image
im = Image.open('shots/firehold-scale-desk.png').convert('RGB')
w, h = im.size
im.crop((w // 2 - 300, h - 230, w // 2 + 300, h)).save('shots/firehold-c7-dock-2x.png')
PY
  q firehold couch; q firehold tab home; sleep 6
  shot firehold-scale-home 2
  python3 - <<'PY'
from PIL import Image
im = Image.open('shots/firehold-scale-home.png').convert('RGB')
w, h = im.size
im.crop((60, h // 3, 660, h // 3 + 360)).save('shots/firehold-c8-home-2x.png')
PY
  q firehold desk; sleep 1
  W timeout 10 wlr-randr --output "$out" --scale 1 2>>shots/wlr-randr.err
else
  fail "scale" "wlr-randr isn't installed"
fi

# What still falls back here (Mantel's own report on firehold-comp).
ws=$(q winlist state)
echo "$ws" > shots/winlist-state.json
grep -E "Mantel: (loaded|up)" "$SHLOG" 2>/dev/null | head -3 | while read -r l; do note "start" "$(echo "$l" | sed 's/^.*Mantel:/Mantel:/' | cut -c1-200)"; done
note "not here yet" "KWin's own views (Overview, Present windows, Desktop grid), window rules, Force close's click-a-window, and hiding a window from screen sharing need KWin; the dock's hover previews are KWin's drawing (hearth-previews.qml); the lock screen is Mantel's own ext-session-lock one with firehold-comp's prompt (nested: firehold-7/8/9)"

# ---- the end
echo logout > "$XDG_RUNTIME_DIR/nimbus-next"
HEARTH_DESKTOP_PID=$(pgrep -x firehold-comp | head -1) HEARTH_WM=firehold sh hosts/linux/session/nimbus-mode logout
for i in $(seq 1 30); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
kill -0 "$SPID" 2>/dev/null && { fail "logout" "the session didn't end"; kill -KILL "$SPID"; }
pkill -TERM quickshell; pkill -KILL -x firehold-comp; pkill -f 'sleep 100000'; sleep 1
cp "$LOG" shots/session.log 2>/dev/null; cp "$HOME/.local/share/nimbus/firehold.log" shots/firehold.log 2>/dev/null
cp "$SHLOG" shots/shell.log 2>/dev/null
grep -iE "panicked" shots/firehold.log 2>/dev/null | head -3 | while read -r l; do fail "panic" "$(echo "$l" | cut -c1-400)"; done
# Any QML error in Mantel's log fails the job.
QERR='\.(qml|js):[0-9]+.*(TypeError|ReferenceError|Cannot assign|non-existent|is not a type|is not defined|Unable to assign|Expected token|Syntax|Cannot read|Type error|unavailable)'
if grep -E "$QERR" shots/shell.log >/dev/null 2>&1; then
  grep -E "$QERR" shots/shell.log | sed 's/^.*file:\/\/[^ ]*shell\///' | sort -u | head -8 | while read -r l; do fail "QML" "${l:0:400}"; done
else
  note "QML" "no QML errors in Mantel's log ($(grep -c . shots/shell.log 2>/dev/null) lines)"
fi
for s in firehold-c1-couch-home firehold-c2-library firehold-c3-guide firehold-c4-everything-open firehold-c5-desk firehold-c6-desktop-2; do
  python3 - "shots/$s.png" <<'PY' || fail "pictures" "$s is missing, the wrong size or one flat colour"
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB')
sys.exit(0 if im.size == (1920, 1080) and len(im.getcolors(1 << 24) or []) > 1 else 1)
PY
done
report
[ ! -e shots/FAILED ]
