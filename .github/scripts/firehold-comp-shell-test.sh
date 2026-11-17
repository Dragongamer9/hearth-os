#!/bin/bash
# The Firehold shell (Couch and Desk) on Firehold's own compositor (firehold.yml "shell"): firehold-comp headless at
# 1920 x 1080, started by nimbus-session with the preview on (HEARTH_DESKTOP_WM=firehold), so Mantel runs with
# HEARTH_WM=firehold exactly as run_firehold starts it on a device, and the Firehold shell unlocked. The library and
# apps are firehold-shell.yml's stand-ins (firehold-fixtures.py); the game is the stand-in game
# (firehold-game-client.py) fullscreen as "Ember Trails", with the Guide's frame limit and scaling chosen through
# hearth-firehold-game. Pictures through screencopy (grim):
#   firehold-c1-couch-home      Couch Home
#   firehold-c2-library         Couch Library
#   firehold-c3-guide           the Guide over the fullscreen game: Frame limit ‹ 60 ›, Resolution scaling ‹ Quality · 720p ›
#   firehold-c4-everything-open Everything open (the windows' cards: their colour here, see "KWin only" below)
#   firehold-c5-desk            the Desk: Firehold's bar and dock with the apps' windows
# It also says, as notices, which parts fall back here because they use KWin-only interfaces, from what Mantel
# reports on this compositor (WinList and Firehold's state). Any QML error in Mantel's log fails it.
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
GH set frameLimit 60 >/dev/null; GH set scaling quality >/dev/null
for i in $(seq 1 30); do [ "$(gst 'g.get("frameLimit") == 60 and g.get("scaling") == "quality"')" = True ] && break; sleep 0.5; done
fl=$(gst 'g.get("frameLimitLabel")'); sc=$(gst 'g.get("scalingLabel")')
q firehold go "Ember Trails"; sleep 3
q firehold open guide; shot firehold-c3-guide 6
case "$fl|$sc" in
  "60|Quality · "[0-9]*p) note "guide" "the rows: Frame limit ‹ $fl ›, Resolution scaling ‹ $sc › (the game draws at $(gst 'g.get("renderSize")'))";;
  *) fail "guide" "the helper's labels aren't real values: frame limit '$fl', scaling '$sc': $(GH status | head -c 400)";;
esac
[ "$(fst 'd["guide"]')" = True ] || fail "guide" "the Guide isn't up: $(q firehold state | head -c 400)"
q firehold press b; sleep 1

# 4. Everything open
q firehold open switcher; shot firehold-c4-everything-open 5
[ "$(fst 'd["page"] == "switcher" and len(d["open"]) >= 4')" = True ] || fail "everything open" "not up with the game and the three apps: $(q firehold state | head -c 500)"
note "everything open" "$(fst '", ".join("%s (%s)" % (o["name"], o["kind"]) for o in d["open"])')"
q firehold press b; sleep 1

# 5. The Desk (the game ended first: fullscreen it would cover the Desk)
GH set frameLimit off >/dev/null; GH set scaling off >/dev/null
kill "$GAMEPID" 2>/dev/null; sleep 2
q firehold desk; shot firehold-c5-desk 6
[ "$(fst 'd["posture"]')" = desk ] || fail "desk" "not on the Desk: $(q firehold state | head -c 400)"

# What falls back here: the parts that use KWin-only interfaces, as Mantel itself reports them on firehold-comp.
ws=$(q winlist state)
echo "$ws" > shots/winlist-state.json
kw() { echo "$ws" | python3 -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1], {"d": d}))' "$1" 2>/dev/null; }
note "KWin only" "live window pictures (hearth-kwin-cast / KWin's window shots): off here (previews $(kw 'd.get("previews")')); Everything open's cards and the dock show the windows' colour and icon instead"
note "KWin only" "virtual desktops: none here ($(kw 'len(d.get("desktops") or [])') from KWin's script; firehold-comp offers no ext-workspace), so the taskbar shows no desktop buttons"
note "KWin only" "the title-bar overlay (FireholdFrames, KWin's window order and frames): not loaded here (it loads only when WinList.kwin); windows keep firehold-comp's own server-side title bars"
note "KWin only" "the lock screen: not KWin's locker (desktop/kwin-lock) but Mantel's own ext-session-lock lock screen, with firehold-comp's built-in prompt if Mantel dies (firehold.yml nested: firehold-7/8/9)"
note "KWin only" "also KWin-only, so off here: VRR in the Guide ('Not on this screen'), KWin's overview / present windows, window rules, screen-share hiding painted by KWin"

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
for s in firehold-c1-couch-home firehold-c2-library firehold-c3-guide firehold-c4-everything-open firehold-c5-desk; do
  python3 - "shots/$s.png" <<'PY' || fail "pictures" "$s is missing, the wrong size or one flat colour"
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB')
sys.exit(0 if im.size == (1920, 1080) and len(im.getcolors(1 << 24) or []) > 1 else 1)
PY
done
report
[ ! -e shots/FAILED ]
