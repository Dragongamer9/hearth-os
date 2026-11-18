#!/bin/bash
# Firehold (Rust, test) in CI (firehold-rust.yml "screens"; docs/FIREHOLD-RUST.md): nimbus-session as on the Anatase
# test (Desktop Mode is KDE Plasma there; HEARTH_DESKTOP_MODE_FILE says so) with "Firehold (Rust, test)" picked, so it
# starts firehold-comp headless at 1920 x 1080 with the Rust shell (firehold-shell hub). KDE Plasma is a stand-in that
# only says it started. Hearth's library and apps are firehold-shell.yml's stand-ins (firehold-fixtures.py), the
# windows are foot, the game is the stand-in game (firehold-game-client.py) as "Ember Trails". Pictures through
# screencopy (grim), one per mockup:
#   firehold-r1-couch-home      Couch Home                          (mockup 1)
#   firehold-r2-desk            the Desk, "Switched to Desk"         (mockup 2)
#   firehold-r3-guide           the Guide over the game              (mockup 3)
#   firehold-r4-everything-open Everything open                      (mockup 4)
#   firehold-r5-library         the Library                          (mockup 5)
#   firehold-r6-settings        Settings › Couch and Desk            (mockup 6)
# Then what must never happen (red flags 2 and 9): a shell that doesn't come up, or a compositor that can't start,
# opens KDE Plasma instead; nothing picked means Plasma and no Firehold at all; the choice and the settings files are
# never changed by anything but a pick.
set -u +e +o pipefail
mkdir -p shots
SRC=$PWD
chmod +x firehold-comp firehold-shell .github/scripts/firehold-game-client.py
fail() { echo "$1: $2" >> shots/errors.txt; touch shots/FAILED; }
note() { echo "$1: $2" >> shots/notes.txt; }
report() {
  python3 - <<'PY'
for kind, f, n in (('error', 'shots/errors.txt', 4), ('notice', 'shots/notes.txt', 5)):
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
        print('::%s title=Firehold (Rust) (%d)::%s' % (kind, i, c))
PY
}
export XDG_RUNTIME_DIR=/tmp/xdg-rust HOME=/tmp/home-rust
rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export LANG=C.UTF-8 LC_ALL=C.UTF-8 LIBGL_ALWAYS_SOFTWARE=1 ICED_BACKEND=tiny-skia FIREHOLD_NO_PAD=1 FIREHOLD_LOG="info,smithay=warn"
export NIMBUS_SHARE="$SRC/hosts/linux" FIREHOLD_RUST_RC="$SRC/os-bases/anatase/system_files/usr/share/hearth/firehold-rust/rc.xml"
unset DISPLAY WAYLAND_DISPLAY
B=/tmp/bin-rust; mkdir -p "$B"
ln -sf "$SRC/firehold-comp" "$B/firehold-comp"; ln -sf "$SRC/firehold-shell" "$B/firehold-shell"
python3 .github/scripts/firehold-fixtures.py "$B" && chmod +x "$B"/*
# KDE Plasma's stand-in: says it started, then waits (the session is ended by the test).
printf '#!/bin/sh\necho "started $(date +%%s)" >> "$XDG_RUNTIME_DIR/plasma-started"\nexec sleep 100000\n' > "$B/startplasma-wayland"; chmod +x "$B/startplasma-wayland"
export PATH="$B:$PATH" NIMBUS_APP="$B/nimbus"
echo plasma > /tmp/desktop-mode-rust; export HEARTH_DESKTOP_MODE_FILE=/tmp/desktop-mode-rust
# A battery and Wi-Fi, for the bar and the Couch's top row (FIREHOLD_SYS).
S=/tmp/sys-rust; rm -rf "$S"; mkdir -p "$S/class/power_supply/BAT0" "$S/class/net/wlan0/wireless"
echo Battery > "$S/class/power_supply/BAT0/type"; echo 78 > "$S/class/power_supply/BAT0/capacity"; echo 18000000 > "$S/class/power_supply/BAT0/power_now"
echo up > "$S/class/net/wlan0/operstate"; export FIREHOLD_SYS="$S"
mkdir -p "$HOME/.config/nimbus" "$HOME/.local/share/nimbus" "$HOME/.local/state/hearth"
CONF="$HOME/.config/nimbus/session.conf"   # the test's own files: Desktop Mode at the start
printf 'NIMBUS_START_MODE=desktop\n' > "$CONF"
CHOICE="$HOME/.config/nimbus/firehold-rust.json"; FH="$HOME/.config/nimbus/firehold.json"
LOG="$HOME/.local/share/nimbus/session.log"; FLOG="$HOME/.local/share/nimbus/firehold-rust.log"

# ---- nothing picked: KDE Plasma, and Firehold isn't even tried (it's never the default)
firehold-shell chosen >/dev/null && fail "default" "firehold-shell chosen says Firehold with nothing picked"
[ -e "$CHOICE" ] && fail "default" "asking wrote the choice file"
sh hosts/linux/session/nimbus-session >shots/session-default.log 2>&1 &
SP=$!
for i in $(seq 30); do [ -s "$XDG_RUNTIME_DIR/plasma-started" ] && break; sleep 1; done
if [ -s "$XDG_RUNTIME_DIR/plasma-started" ] && ! grep -q "Firehold (Rust" "$LOG"; then note "default" "nothing picked: KDE Plasma opened, Firehold wasn't tried"
else fail "default" "with nothing picked: plasma started=$(cat "$XDG_RUNTIME_DIR/plasma-started" 2>/dev/null | wc -l), log: $(tail -4 "$LOG" | tr '\n' '|')"; fi
kill "$SP" 2>/dev/null; pkill -f startplasma-wayland; sleep 1; rm -f "$XDG_RUNTIME_DIR/plasma-started" "$XDG_RUNTIME_DIR/nimbus-next"

# ---- picked: Firehold (Rust, test)
[ "$(firehold-shell choose firehold)" = saved ] || fail "choice" "firehold-shell choose firehold didn't save"
[ "$(firehold-shell choose firehold)" = unchanged ] || fail "choice" "the same pick again wrote the file"
cp "$CHOICE" shots/choice-before.json
FIREHOLD_ARGS="--headless --size 1920x1080 --socket wayland-hearth" sh hosts/linux/session/nimbus-session >shots/session-out.log 2>&1 &
SP=$!
for i in $(seq 90); do [ -e "$XDG_RUNTIME_DIR/firehold-shell-up" ] && break; sleep 1; done
if [ -e "$XDG_RUNTIME_DIR/firehold-shell-up" ]; then
  note "session" "Firehold (Rust, test) up in $(cat "$XDG_RUNTIME_DIR/firehold-shell-up") s: $(grep -o 'firehold (Rust): up after [0-9]*s' "$LOG" | tail -1)"
else
  fail "session" "Firehold (Rust) didn't come up: $(tail -5 "$LOG" | tr '\n' '|' | cut -c1-500) shell: $(tail -c 1500 "$FLOG" | tr '\n' '|')"
  cp "$LOG" "$FLOG" shots/ 2>/dev/null; report; exit 1
fi
[ -s "$XDG_RUNTIME_DIR/plasma-started" ] && fail "session" "KDE Plasma started too"
export WAYLAND_DISPLAY=wayland-hearth
C() { timeout 10 firehold-shell ctl "$@" 2>/dev/null | tail -n1; }
st() { C state | python3 -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1], {"d": d}))' "$1" 2>/dev/null; }
shot() { sleep "${2:-3}"; timeout 20 grim "shots/$1.png" 2>"shots/$1.err" || fail "pictures" "grim failed for $1: $(head -c 300 "shots/$1.err")"; }
for i in $(seq 30); do [ "$(st 'd["games"] >= 10')" = True ] && break; sleep 1; done
note "hub" "posture $(st 'd["posture"]'), surfaces $(st 'sorted(d["hello"])'), $(st 'd["games"]') games, firehold-comp connected: $(st 'd["comp"]')"
[ "$(st 'd["comp"]')" = True ] || fail "hub" "the hub isn't connected to firehold-comp's window IPC"
[ "$(st '"bar" in d["hello"] and "dock" in d["hello"] and "couch" in d["hello"]')" = True ] || fail "hub" "not every Desk surface said hello: $(st 'd["hello"]')"

# The apps' windows (foot stands in for each).
win() { foot --app-id="$1" -T "$2" -W 80x20 sh -c 'sleep 100000' >/dev/null 2>&1 & }
win org.hearth.files "Pictures › Trips — Hearth Files"; sleep 1
win firefox "3 tabs — Firefox"; sleep 1
win discord "2 new messages — Discord"; sleep 1
win steam "Downloading 1 update — Steam"; sleep 3
for i in $(seq 20); do [ "$(st 'len(d["wins"]) >= 4')" = True ] && break; sleep 1; done
note "windows" "$(st '", ".join("%s (%s · %s)" % (w["id"], w["name"], w["sub"]) for w in d["wins"])')"

# 1. Couch Home (mockup 1)
C posture couch >/dev/null
for i in $(seq 10); do [ "$(st 'd["couch_up"]')" = True ] && break; sleep 0.5; done
shot firehold-r1-couch-home 3
# 5. The Library (mockup 5)
C tab library >/dev/null; shot firehold-r5-library 2
C tab home >/dev/null
# 3. The game, full screen (the Couch puts it there), and the Guide over it with its rows and the overlay (mockup 3)
C '{"op":"set","key":"overlay","value":"basic"}' >/dev/null
python3 .github/scripts/firehold-game-client.py "$XDG_RUNTIME_DIR/game-stats.json" --app-id steam_app_1000 --title "Ember Trails" --seconds 400 --windowed --ssd >/dev/null 2>&1 &
GP=$!
for i in $(seq 20); do [ "$(st 'any(w["game"] and w["fullscreen"] for w in d["wins"])')" = True ] && break; sleep 1; done
[ "$(st 'any(w["game"] and w["fullscreen"] for w in d["wins"])')" = True ] && note "game" "Ember Trails opened full screen on the Couch (Games: Follow the posture)" || fail "game" "the game didn't go full screen on the Couch: $(st 'd["wins"]')"
[ "$(st 'd["couch_up"]')" = False ] || fail "game" "the Couch stayed over the game"
C '{"op":"gameRow","key":"frameLimit","dir":"up"}' >/dev/null; C '{"op":"gameRow","key":"frameLimit","dir":"up"}' >/dev/null; C '{"op":"gameRow","key":"frameLimit","dir":"up"}' >/dev/null
C '{"op":"gameRow","key":"scaling","dir":"up"}' >/dev/null
C guide >/dev/null
for i in $(seq 10); do [ "$(st '"guide" in d["hello"]')" = True ] && break; sleep 0.5; done
note "guide" "up: $(st 'd["guide"]["up"]'), rows from firehold-comp: $(st 'd["guide"]["comp"]'), frame limit $(st 'd["guide"]["frame_limit"]'), scaling $(st 'd["guide"]["scaling"]'), VRR $(st 'd["guide"]["vrr"]'), playing $(st 'd["guide"]["playing"]'), overlay: $(st '"perf" in d["hello"]') ($(st 'd["perf"]["line"]'))"
[ "$(st 'd["guide"]["comp"]')" = True ] || fail "guide" "the Guide's game rows didn't come from firehold-comp: $(st 'd["guide"]')"
[ "$(st 'd["guide"]["frame_limit"]')" = 40 ] || fail "guide" "Frame limit isn't ‹ 40 › after three steps up: $(st 'd["guide"]["frame_limit"]')"
shot firehold-r3-guide 3
# 4. Twice: Everything open, with each window's real picture (mockup 4)
C guide >/dev/null
sleep 7
note "everything open" "pictures: $(st 'sum(1 for w in d["wins"] if w["pic"])') of $(st 'len(d["wins"])')"
[ "$(st 'sum(1 for w in d["wins"] if w["pic"]) >= 3')" = True ] || fail "everything open" "the windows' pictures didn't come: $(st '[w["pic"] for w in d["wins"]]')"
shot firehold-r4-everything-open 1
# 2. The mouse: the Desk, with "Switched to Desk", the game back in a window (mockup 2)
C pad b >/dev/null; sleep 1
C '{"op":"input","kind":"mouse"}' >/dev/null
for i in $(seq 10); do [ "$(st 'd["posture"]')" = desk ] && break; sleep 0.5; done
[ "$(st 'd["posture"]')" = desk ] || fail "posture" "the mouse didn't switch to the Desk"
[ "$(st 'not any(w["game"] and w["fullscreen"] for w in d["wins"])')" = True ] || fail "posture" "the game stayed full screen on the Desk"
GID=$(st '[w["id"] for w in d["wins"] if w["game"]][0]'); FID=$(st '[w["id"] for w in d["wins"] if w["name"] == "Hearth Files"][0]')
C "{\"op\":\"activate\",\"id\":\"$FID\"}" >/dev/null; sleep 0.5; C "{\"op\":\"activate\",\"id\":\"$GID\"}" >/dev/null
shot firehold-r2-desk 2
[ "$(st 'd["notice"] is not None')" = True ] || note "posture" "the notice had already gone when the picture was taken"
# 6. Settings › Couch and Desk (mockup 6): a window
C page settings >/dev/null
for i in $(seq 15); do [ "$(st '"settings" in d["hello"]')" = True ] && break; sleep 1; done
shot firehold-r6-settings 4

# ---- the settings: only the test's own pick changed them (red flag 9)
cmp -s "$CHOICE" shots/choice-before.json || fail "settings" "the Desktop Mode choice changed by itself: $(cat "$CHOICE")"
python3 - "$FH" <<'PY' || fail "settings" "firehold.json has more than the overlay the test picked: $(cat "$FH" 2>/dev/null)"
import json, sys
d = json.load(open(sys.argv[1]))
assert d == {"overlay": "basic"}, d
PY
[ -e "$HOME/.config/hearth/firehold-games.json" ] && note "settings" "the game rows the test stepped: $(cat "$HOME/.config/hearth/firehold-games.json" | tr -d '\n' | cut -c1-200)"

# ---- the CPU and memory of the shell in this run (the 10-minute check is its own job)
for p in $(pgrep -x firehold-shell); do echo "$p $(tr '\0' ' ' < /proc/$p/cmdline | cut -c1-60) $(awk '/^VmRSS:/{print int($2/1024)" MB"}' /proc/$p/status)"; done > shots/procs.txt
note "processes" "$(tr '\n' ';' < shots/procs.txt)"

# ---- the end: Back to Game Mode from inside (nimbus-mode, as the Settings button does): the session goes on
kill "$GP" 2>/dev/null
HEARTH_DESKTOP_PID=$(cat "$XDG_RUNTIME_DIR/firehold.pid") sh hosts/linux/session/nimbus-mode game
for i in $(seq 15); do pgrep -x firehold-comp >/dev/null || break; sleep 1; done
pgrep -x firehold-comp >/dev/null && fail "leave" "firehold-comp didn't end on nimbus-mode game" || note "leave" "nimbus-mode game ended Firehold: $(grep -o 'firehold (Rust) ended.*' "$LOG" | tail -1)"
pgrep -x firehold-shell >/dev/null && fail "leave" "firehold-shell processes stayed: $(pgrep -a -x firehold-shell | tr '\n' ';')"
kill "$SP" 2>/dev/null; pkill -f startplasma-wayland; pkill -x gamescope; sleep 1
cp "$LOG" shots/session.log; cp "$FLOG" shots/firehold-rust.log 2>/dev/null
rm -f "$XDG_RUNTIME_DIR/plasma-started" "$XDG_RUNTIME_DIR/nimbus-next"

# ---- safe to fail (red flag 2): a shell that never comes up, and a compositor that can't start: KDE Plasma instead
printf '#!/bin/sh\n[ "$1" = chosen ] && { echo firehold; exit 0; }\nexit 3\n' > "$B/broken-shell"; chmod +x "$B/broken-shell"
for case in shell comp; do
  rm -f "$XDG_RUNTIME_DIR/plasma-started"; : > "$LOG"
  if [ "$case" = shell ]; then
    FIREHOLD_SHELL_BIN=broken-shell FIREHOLD_UP_SECS=12 FIREHOLD_ARGS="--headless --size 1280x720 --socket wayland-fail" sh hosts/linux/session/nimbus-session >shots/session-fail-$case.log 2>&1 &
  else
    FIREHOLD_BIN=false FIREHOLD_UP_SECS=12 sh hosts/linux/session/nimbus-session >shots/session-fail-$case.log 2>&1 &
  fi
  SP=$!
  for i in $(seq 60); do [ -s "$XDG_RUNTIME_DIR/plasma-started" ] && break; sleep 1; done
  if [ -s "$XDG_RUNTIME_DIR/plasma-started" ]; then note "fallback ($case)" "KDE Plasma opened instead: $(grep -o 'Firehold (Rust, test) didn.t work.*' "$LOG" | tail -1 | cut -c1-200)"
  else fail "fallback ($case)" "KDE Plasma didn't open: $(tail -5 "$LOG" | tr '\n' '|' | cut -c1-500)"; fi
  cmp -s "$CHOICE" shots/choice-before.json || fail "fallback ($case)" "the failure changed the choice: $(cat "$CHOICE")"
  kill "$SP" 2>/dev/null; pkill -f startplasma-wayland; pkill -x firehold-comp; sleep 1
done
cp "$LOG" shots/session-fail.log
report
[ -e shots/FAILED ] && exit 1
exit 0
