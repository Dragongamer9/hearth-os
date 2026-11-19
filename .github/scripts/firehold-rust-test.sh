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
# and more of each: r1b/r1c the Couch's Apps and Store, r3b the overlay over the game with no frame limit (fps from
# every frame), r6b Settings › System (the Desktop Mode choice), r7/r7b Settings and Couch Home in another look
# (Steam-style, picked in desktop.json: Firehold follows the look, through hearth-ui). Firehold's own parts: r2b the
# Desk bar's Edit menu, r2c a notification (notify-send, to Firehold's own notification service), r2d Quick Settings,
# r2e the on-screen keyboard (and what it types reaching a window), r9 Away. Then a second session with two screens
# (a monitor and a TV, Per screen: TV Couch, monitor Desk): r8a the monitor, r8b the TV, r6c Settings' Per screen rows.
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
printf '#!/bin/sh\necho "started $(date +%%s)" >> "$XDG_RUNTIME_DIR/plasma-started"\nexec sleep 100001\n' > "$B/startplasma-wayland"; chmod +x "$B/startplasma-wayland"
export PATH="$B:$PATH" NIMBUS_APP="$B/nimbus"
# Hearth Recorder's and Steam's stand-ins: they say what they were asked (no recording, no Steam, no account).
printf '#!/bin/sh\necho "$*" >> "$XDG_RUNTIME_DIR/calls-recorder"\n' > "$B/hearth-recorder"
printf '#!/bin/sh\necho "$*" >> "$XDG_RUNTIME_DIR/calls-steam"\n' > "$B/steam"
chmod +x "$B/hearth-recorder" "$B/steam"
echo plasma > /tmp/desktop-mode-rust; export HEARTH_DESKTOP_MODE_FILE=/tmp/desktop-mode-rust
# A battery and Wi-Fi, for the bar and the Couch's top row (FIREHOLD_SYS).
S=/tmp/sys-rust; rm -rf "$S"; mkdir -p "$S/class/power_supply/BAT0" "$S/class/net/wlan0/wireless"
echo Battery > "$S/class/power_supply/BAT0/type"; echo 78 > "$S/class/power_supply/BAT0/capacity"; echo 18000000 > "$S/class/power_supply/BAT0/power_now"
echo up > "$S/class/net/wlan0/operstate"; export FIREHOLD_SYS="$S"
mkdir -p "$HOME/.config/nimbus" "$HOME/.local/share/nimbus" "$HOME/.local/state/hearth"
# The chapter the game says it's on (Hearth's progress file, model::progress_chapter: written by the game itself).
mkdir -p "$HOME/.local/share/hearth/progress" && printf '{"chapter": "Chapter 4"}' > "$HOME/.local/share/hearth/progress/ember-trails.json"
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
plasma_gone() { pkill -f 'sleep 100001'; rm -f "$XDG_RUNTIME_DIR/hearth-plasma.pid"; }
kill "$SP" 2>/dev/null; plasma_gone; sleep 1; rm -f "$XDG_RUNTIME_DIR/plasma-started" "$XDG_RUNTIME_DIR/nimbus-next"

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
C tab apps >/dev/null; shot firehold-r1b-couch-apps 2
C tab store >/dev/null; shot firehold-r1c-couch-store 2
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
# Hearth Recorder's instant replay is on (its own state file, as its recording process writes it): "Last 30 s too".
sleep 100003 & RP=$!
mkdir -p "$XDG_RUNTIME_DIR/hearth-recorder"
printf '{"kind": "replay", "status": "recording", "pid": %d, "replaySecs": 30}' "$RP" > "$XDG_RUNTIME_DIR/hearth-recorder/state.json"
C guide >/dev/null
for i in $(seq 10); do [ "$(st '"guide" in d["hello"]')" = True ] && break; sleep 0.5; done
note "guide" "up: $(st 'd["guide"]["up"]'), rows from firehold-comp: $(st 'd["guide"]["comp"]'), frame limit $(st 'd["guide"]["frame_limit"]'), scaling $(st 'd["guide"]["scaling"]'), VRR $(st 'd["guide"]["vrr"]'), playing $(st 'd["guide"]["playing"]'), overlay: $(st '"perf" in d["hello"]') ($(st 'd["perf"]["line"]'))"
[ "$(st 'd["guide"]["comp"]')" = True ] || fail "guide" "the Guide's game rows didn't come from firehold-comp: $(st 'd["guide"]')"
[ "$(st 'd["guide"]["frame_limit"]')" = 60 ] || fail "guide" "Frame limit isn't ‹ 60 › after three steps up (off, 30, 40, 60): $(st 'd["guide"]["frame_limit"]')"
shot firehold-r3-guide 3
[ "$(st 'd["guide"]["record"]')" = "Last 30 s too" ] || fail "guide" "Record isn't \"Last 30 s too\" with instant replay on: $(st 'd["guide"]["record"]')"
[ "$(st 'd["guide"]["friends"]')" = True ] || fail "guide" "Friends doesn't see Steam's stand-in"
C '{"op":"guideTile","what":"record"}' >/dev/null; C '{"op":"guideTile","what":"friends"}' >/dev/null; sleep 2
grep -qx -- '--save-replay' "$XDG_RUNTIME_DIR/calls-recorder" 2>/dev/null && note "guide tiles" "Record saved the replay (hearth-recorder --save-replay); Friends: steam $(cat "$XDG_RUNTIME_DIR/calls-steam" 2>/dev/null)" \
  || fail "guide tiles" "Record didn't ask Hearth Recorder to save the replay: $(cat "$XDG_RUNTIME_DIR/calls-recorder" 2>/dev/null)"
grep -q 'steam://open/friends' "$XDG_RUNTIME_DIR/calls-steam" 2>/dev/null || fail "guide tiles" "Friends didn't open Steam's friends list"
comp_layers() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print("focus", d.get("keyboardFocus") or d.get("focus"), [(l["namespace"], l["layer"], l["w"], l["h"]) for o in d.get("outputs", []) for l in o.get("layers", [])])' "$XDG_RUNTIME_DIR/firehold.json" 2>&1 | cut -c1-700; }
note "layers (guide)" "couch_up $(st 'd["couch_up"]'), $(comp_layers)"
comp_layers | grep -q "hearth-firehold-perf" || fail "overlay" "the performance overlay isn't on the screen with the Guide open: $(comp_layers) · hub: $(st 'sorted(d["hello"])') · log: $(grep -i 'perf' "$FLOG" | tail -3 | tr '\n' '|')"
# The overlay with no frame limit (60 → 90 → 120 → screen → off): fps from every frame the game hands in.
C guideClose >/dev/null
for i in 1 2 3 4; do C '{"op":"gameRow","key":"frameLimit","dir":"up"}' >/dev/null; done
sleep 4
note "overlay" "no frame limit: $(st 'd["perf"]["line"]') · firehold-comp: $(python3 -c 'import json,socket,sys; s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b"status\n"); d=json.loads(s.makefile().readline()); g=d.get("game") or {}; print("fps", g.get("fps"), "paced", g.get("pacedFps"), "frames", g.get("frames"), "limit", g.get("frameLimit"))' "$XDG_RUNTIME_DIR/firehold-game-wayland-hearth.sock" 2>&1 | cut -c1-200)"
case "$(st 'd["perf"]["line"]')" in *fps*) ;; *) fail "overlay" "no fps in the overlay without a frame limit: $(st 'd["perf"]["line"]')";; esac
shot firehold-r3b-overlay 1
# Back to ‹ 60 › (the mockup's) for the rest.
for i in 1 2 3; do C '{"op":"gameRow","key":"frameLimit","dir":"up"}' >/dev/null; done
C guide >/dev/null; sleep 1
# 4. Twice: Everything open, with each window's real picture (mockup 4)
C guide >/dev/null
sleep 7
note "everything open" "pictures: $(st 'sum(1 for w in d["wins"] if w["pic"])') of $(st 'len(d["wins"])')"
[ "$(st 'sum(1 for w in d["wins"] if w["pic"]) >= 3')" = True ] || fail "everything open" "the windows' pictures didn't come: $(st '[w["pic"] for w in d["wins"]]')"
note "layers (everything open)" "couch_up $(st 'd["couch_up"]'), $(comp_layers)"
shot firehold-r4-everything-open 1
# 2. The mouse: the Desk, with "Switched to Desk", the game back in a window (mockup 2)
C pad b >/dev/null; sleep 1
C '{"op":"input","kind":"mouse"}' >/dev/null
for i in $(seq 10); do [ "$(st 'd["posture"]')" = desk ] && break; sleep 0.5; done
[ "$(st 'd["posture"]')" = desk ] || fail "posture" "the mouse didn't switch to the Desk"
for i in $(seq 10); do [ "$(st 'not any(w["game"] and w["fullscreen"] for w in d["wins"])')" = True ] && break; sleep 0.5; done
[ "$(st 'not any(w["game"] and w["fullscreen"] for w in d["wins"])')" = True ] || fail "posture" "the game stayed full screen on the Desk"
GID=$(st '[w["id"] for w in d["wins"] if w["game"]][0]'); FID=$(st '[w["id"] for w in d["wins"] if w["name"] == "Hearth Files"][0]')
# As the mockup sets the scene: Hearth Files on the left, the game on the right (firehold-comp's own snap, through its
# window IPC), Hearth Files in front (the bar names it).
W() { python3 -c 'import socket,sys; s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall((sys.argv[2]+"\n").encode()); s.settimeout(3); print(s.makefile().readline().strip()[:200])' "$XDG_RUNTIME_DIR/firehold-wm-wayland-hearth.sock" "$1" 2>&1; }
note "desk scene" "$(W "{\"op\":\"snap\",\"id\":$FID,\"zone\":\"left-half\"}") · $(W "{\"op\":\"snap\",\"id\":$GID,\"zone\":\"right-half\"}")"
C "{\"op\":\"activate\",\"id\":\"$GID\"}" >/dev/null; sleep 0.5; C "{\"op\":\"activate\",\"id\":\"$FID\"}" >/dev/null
# The scene took a few seconds: the Couch and the mouse once more, so "Switched to Desk" is fresh for the picture.
C posture couch >/dev/null; sleep 1; C '{"op":"input","kind":"mouse"}' >/dev/null
for i in $(seq 10); do [ "$(st 'd["posture"]')" = desk ] && break; sleep 0.3; done
shot firehold-r2-desk 1.5
[ "$(st 'd["notice"] is not None')" = True ] || note "posture" "the notice had already gone when the picture was taken"
# 2b. The Desk bar's Edit menu, under its title (the app in front's standard commands)
sleep 6
C menu Edit >/dev/null
for i in $(seq 10); do [ "$(st '"menu" in d["hello"]')" = True ] && break; sleep 0.5; done
note "menu" "open: $(st 'd["menu"]["open"]') at x=$(st 'd["menu"]["x"]'): $(st '", ".join(i["label"] for i in d["menu"]["items"])')"
[ "$(st 'd["menu"]["open"]')" = Edit ] || fail "menu" "the Edit menu didn't open: $(st 'd["menu"]')"
shot firehold-r2b-desk-menu 1.5
C menu >/dev/null; sleep 0.5
# 2c. A notification from an app (notify-send: libnotify, to Firehold's own org.freedesktop.Notifications)
if command -v notify-send >/dev/null; then
  # (With a button, notify-send waits for it to be pressed: in the background, for at most 40 s.)
  timeout 40 notify-send -a Discord "Night Harbor" "Sam is playing now. Join them from Discord." -A join=Join >/dev/null 2>&1 &
  for i in $(seq 10); do [ "$(st 'len(d["notes"]) >= 1')" = True ] && break; sleep 0.5; done
  note "notifications" "on screen: $(st '[(n["app"], n["title"]) for n in d["notes"]]')"
  [ "$(st 'len(d["notes"]) >= 1')" = True ] || fail "notifications" "notify-send's notification didn't reach Firehold: $(grep -i notif "$FLOG" | tail -3 | tr '\n' '|')"
else
  C '{"op":"notify","app":"Discord","title":"Night Harbor","body":"Sam is playing now. Join them from Discord."}' >/dev/null
  note "notifications" "notify-send isn't here: the hub's own test notification"
fi
shot firehold-r2c-notification 1.5
# 2d. Quick Settings (from the bar's Wi-Fi and battery)
C '{"op":"quick","arg":"on"}' >/dev/null
for i in $(seq 12); do [ "$(st '"quick" in d["hello"]')" = True ] && break; sleep 0.5; done
note "quick settings" "$(st '{k: d["quick"][k] for k in ("up", "wifi", "bt", "volume", "brightness", "profile", "dnd")}'), notifications kept: $(st 'len(d["history"])')"
[ "$(st 'd["quick"]["up"]')" = True ] || fail "quick settings" "Quick Settings didn't open"
shot firehold-r2d-quick-settings 3
C '{"op":"quick","arg":"off"}' >/dev/null; sleep 0.5
# 2e. The on-screen keyboard: up at the bottom, and what it types reaches the window that has the keyboard
foot --app-id=typetest -T "Typing test" -W 60x8 sh -c 'cat > "$XDG_RUNTIME_DIR/typed.txt"' >/dev/null 2>&1 &
for i in $(seq 20); do [ "$(st 'any(w["app_id"] == "typetest" for w in d["wins"])')" = True ] && break; sleep 0.5; done
TID=$(st '[w["id"] for w in d["wins"] if w["app_id"] == "typetest"][0]')
C "{\"op\":\"activate\",\"id\":\"$TID\"}" >/dev/null; sleep 1
C '{"op":"osk","arg":"on"}' >/dev/null
for i in $(seq 12); do [ "$(st '"osk" in d["hello"]')" = True ] && break; sleep 0.5; done
C '{"op":"oskType","text":"Hello Firehold"}' >/dev/null; C '{"op":"oskType","keys":["enter"]}' >/dev/null
sleep 1.5
grep -q "Hello Firehold" "$XDG_RUNTIME_DIR/typed.txt" 2>/dev/null && note "keyboard" "the on-screen keyboard typed \"Hello Firehold\" into a window" \
  || fail "keyboard" "what the on-screen keyboard typed didn't reach the window: $(cat "$XDG_RUNTIME_DIR/typed.txt" 2>/dev/null | head -c 100)"
shot firehold-r2e-keyboard 1.5
C '{"op":"osk","arg":"off"}' >/dev/null
# The menus' commands reach the app in front: View › Maximize on the typing window
C "{\"op\":\"activate\",\"id\":\"$TID\"}" >/dev/null; sleep 0.5
C menu View >/dev/null; sleep 0.5; C menuPick w:maximize >/dev/null; sleep 1
python3 - "$XDG_RUNTIME_DIR/firehold-wm-wayland-hearth.sock" "$TID" <<'PY' && note "menus" "View › Maximize maximized the window in front" || fail "menus" "View › Maximize didn't reach the window in front"
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.sendall(b'{"op":"windows"}\n'); s.settimeout(3)
f = s.makefile()
for _ in range(5):
    d = json.loads(f.readline())
    if d.get("t") == "windows":
        w = [w for w in d["windows"] if w["id"] == sys.argv[2]]
        sys.exit(0 if w and w[0]["maximized"] else 1)
sys.exit(1)
PY
C "{\"op\":\"close\",\"id\":\"$TID\"}" >/dev/null
# 9. Away: the home server keeps running (hearth-home-server quick), everything stays open; the mouse brings it back
C posture away >/dev/null
for i in $(seq 12); do [ "$(st '"away" in d["hello"]')" = True ] && break; sleep 0.5; done
sleep 3
note "away" "posture $(st 'd["posture"]'): $(st 'd["away"]["lines"]') $(st 'd["away"]["note"]')"
[ "$(st 'd["posture"]')" = away ] || fail "away" "Away didn't come up"
shot firehold-r9-away 1
C '{"op":"input","kind":"mouse"}' >/dev/null
for i in $(seq 10); do [ "$(st 'd["posture"]')" = desk ] && break; sleep 0.5; done
[ "$(st 'd["posture"]')" = desk ] && note "away" "the mouse brought back the Desk" || fail "away" "the mouse didn't end Away: $(st 'd["posture"]')"
# 6. Settings › Couch and Desk (mockup 6): a window in the middle, over the Couch's background (the other windows put
# away first, as the mockup has nothing else on the screen)
for id in $(st '" ".join(w["id"] for w in d["wins"])'); do C "{\"op\":\"minimize\",\"id\":\"$id\"}" >/dev/null; done
C posture couch >/dev/null; sleep 1
C page settings >/dev/null
for i in $(seq 15); do [ "$(st '"settings" in d["hello"]')" = True ] && break; sleep 1; done
for i in $(seq 10); do [ "$(st 'any(w["app_id"] == "firehold-settings" for w in d["wins"])')" = True ] && break; sleep 0.5; done
note "settings" "window: $(st '[(w["name"], w["fullscreen"]) for w in d["wins"] if w["app_id"] == "firehold-settings"]'), couch_up $(st 'd["couch_up"]'), posture $(st 'd["posture"]')"
[ "$(st 'any(w["app_id"] == "firehold-settings" and w["fullscreen"] for w in d["wins"])')" = False ] || fail "settings" "Settings went full screen on the Couch (it stays a window)"
shot firehold-r6-settings 4
C settingsPage system >/dev/null; shot firehold-r6b-settings-system 2
# Another look, picked in Settings (desktop.json, as Mantel's and Plasma's looks write it): Firehold follows at once.
LOOK="$HOME/.local/share/nimbus/desktop.json"
printf '{"look": "steam", "mode": "dark"}' > "$LOOK.tmp" && mv "$LOOK.tmp" "$LOOK"
C settingsPage couch >/dev/null; shot firehold-r7-look-steam 3
C home >/dev/null; shot firehold-r7b-look-steam-couch 3
rm -f "$LOOK"; sleep 1

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
# (Ones that ended but weren't collected yet, "defunct": the container has no init to collect them; they don't run.)
live=$(for p in $(pgrep -x firehold-shell); do [ "$(sed -n 's/^State:[[:space:]]*\([A-Z]\).*/\1/p' /proc/$p/status 2>/dev/null)" = Z ] || echo "$p"; done | tr '\n' ' ')
[ -n "${live// }" ] && fail "leave" "firehold-shell processes stayed: $live"
kill "$SP" 2>/dev/null; plasma_gone; pkill -x gamescope; sleep 1
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
  kill "$SP" 2>/dev/null; plasma_gone; pkill -x firehold-comp; sleep 1
done
cp "$LOG" shots/session-fail.log

# ---- two screens (a monitor and a TV): Per screen, TV Couch and monitor Desk (mockup 6's "TV: Couch · Monitor: Desk")
printf '{"perScreen": {"HDMI-A-1": "couch", "HEADLESS-1": "desk"}}' > "$FH"
cp "$FH" shots/per-screen-before.json
rm -f "$XDG_RUNTIME_DIR/plasma-started"; : > "$LOG"
FIREHOLD_ARGS="--headless --size 1920x1080 --outputs HEADLESS-1,HDMI-A-1 --socket wayland-two" sh hosts/linux/session/nimbus-session >shots/session-two.log 2>&1 &
SP=$!
for i in $(seq 90); do [ -e "$XDG_RUNTIME_DIR/firehold-shell-up" ] && break; sleep 1; done
export WAYLAND_DISPLAY=wayland-two
for i in $(seq 20); do [ "$(st 'len(d["screens"]) == 2 and "couch@HDMI-A-1" in d["hello"] and "bar@HEADLESS-1" in d["hello"]')" = True ] && break; sleep 1; done
note "per screen" "screens $(st '[(s["name"], s["label"]) for s in d["screens"]]'), postures $(st 'd["postures"]'), per screen $(st 'd["per_screen"]'), surfaces $(st 'sorted(d["hello"])')"
[ "$(st 'd["postures"] == {"HDMI-A-1": "couch", "HEADLESS-1": "desk"}')" = True ] || fail "per screen" "the screens aren't each in their own posture: $(st 'd["postures"]')"
[ "$(st '"couch@HDMI-A-1" in d["hello"] and "bar@HEADLESS-1" in d["hello"] and "dock@HEADLESS-1" in d["hello"] and "bar@HDMI-A-1" not in d["hello"]')" = True ] \
  || fail "per screen" "the TV should have the Couch and the monitor the bar and the dock: $(st 'sorted(d["hello"])')"
win org.hearth.files "Pictures › Trips — Hearth Files"; sleep 1
win firefox "3 tabs — Firefox"; sleep 3
C home >/dev/null; sleep 1
timeout 20 grim -o HEADLESS-1 shots/firehold-r8a-monitor-desk.png 2>shots/r8a.err || fail "pictures" "grim -o HEADLESS-1: $(head -c 200 shots/r8a.err)"
timeout 20 grim -o HDMI-A-1 shots/firehold-r8b-tv-couch.png 2>shots/r8b.err || fail "pictures" "grim -o HDMI-A-1: $(head -c 200 shots/r8b.err)"
C settingsPage couch-screens >/dev/null; C page settings >/dev/null
for i in $(seq 15); do [ "$(st 'any(w["app_id"] == "firehold-settings" for w in d["wins"])')" = True ] && break; sleep 1; done
C settingsPage couch-screens >/dev/null; sleep 3
SO=$(st '[w["output"] for w in d["wins"] if w["app_id"] == "firehold-settings"][0]')
timeout 20 grim -o "${SO:-HEADLESS-1}" shots/firehold-r6c-settings-per-screen.png 2>shots/r6c.err || fail "pictures" "grim -o $SO: $(head -c 200 shots/r6c.err)"
cmp -s "$FH" shots/per-screen-before.json || fail "per screen" "firehold.json changed by itself: $(cat "$FH")"
HEARTH_DESKTOP_PID=$(cat "$XDG_RUNTIME_DIR/firehold.pid") sh hosts/linux/session/nimbus-mode game
for i in $(seq 15); do pgrep -x firehold-comp >/dev/null || break; sleep 1; done
kill "$SP" 2>/dev/null; plasma_gone; pkill -x firehold-comp; sleep 1
cp "$LOG" shots/session-two-full.log; cp "$FLOG" shots/firehold-rust-two.log 2>/dev/null
report
[ -e shots/FAILED ] && exit 1
exit 0
