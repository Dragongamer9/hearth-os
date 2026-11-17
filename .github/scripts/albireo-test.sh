#!/bin/bash
# Hearth's own compositor, nested (albireo.yml; docs/ALBIREO.md "How it's tested"): the compositor built by
# the build job runs in a 1920x1080 window (its winit backend) on a virtual X screen (Xvfb, software GL), started by
# nimbus-session itself with the preview turned on, so the real Desktop Mode comes up on it: the autostart, Mantel
# (Quickshell) with its bar, dock and wallpaper (layer shell), and a test window (foot) on Mantel's taskbar (foreign
# toplevel). Then the window is minimized and brought back through the taskbar protocol, dragged to the screen's left
# edge with the pointer (snapping), and Settings › System › Your desktop shows the preview switch. A picture after each
# step (Xvfb's screen), each checked not to be one flat colour; failures are ::error:: annotations and shots/FAILED.
#   albireo-1-empty   the compositor on its own: Mantel's background colour
#   albireo-2-mantel  Mantel's bar, dock and wallpaper on it
#   albireo-3-window  a window with Hearth's frame, listed on Mantel's taskbar
#   albireo-4-minimized  minimized through the taskbar protocol, then back
#   albireo-5-snapped the window dragged to the left edge: the left half
#   albireo-6-your-desktop  Settings › System › Your desktop with the preview switch
set -u +e +o pipefail
mkdir -p shots
SRC=$PWD
BIN=${ALBIREO:-$SRC/albireo}
chmod +x "$BIN"
fail() { echo "::error title=$1::$2"; touch shots/FAILED; }
note() { echo "::notice title=$1::$2"; }
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1 QT_FORCE_STDERR_LOGGING=1
export NIMBUS_SHARE="$SRC/hosts/linux" NIMBUS_APP=/nonexistent
export ALBIREO_LOG="info,smithay=warn"
chmod +x hosts/linux/desktop/labwc/autostart hosts/linux/desktop/albireo/autostart hosts/linux/desktop/theme/apply.sh
mkdir -p "$HOME/.cache/hearth" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/share/nimbus/desktop.json" '{}' >/dev/null

# The virtual screen the compositor's window is on.
Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp >shots/xvfb.log 2>&1 &
for i in $(seq 1 20); do [ -e /tmp/.X11-unix/X99 ] && break; sleep 0.5; done
export DISPLAY=:99
unset WAYLAND_DISPLAY
shot() {   # the whole virtual screen
  timeout 20 import -window root -display :99 "shots/$1.png" 2>>shots/shot-errors.txt || echo "no screenshot $1" >>shots/shot-errors.txt
  sleep 0.5
}
STATUS="$XDG_RUNTIME_DIR/albireo.json"
status() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2], {"d": d}))' "$STATUS" "$1" 2>/dev/null; }
wait_for() {   # $1 seconds, $2 a Python expression on the status file (d)
  for i in $(seq 1 $(( $1 * 2 ))); do [ "$(status "$2")" = True ] && return 0; sleep 0.5; done; return 1
}
W() { WAYLAND_DISPLAY=wayland-hearth DISPLAY= "$@"; }

# ---- 1: the compositor on its own (and the protocols it offers).
"$BIN" --winit --size 1920x1080 --socket wayland-hearth >shots/compositor-alone.log 2>&1 &
CPID=$!
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 0.5; done
if [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ]; then
  note "compositor" "up on its own ($(grep -c . shots/compositor-alone.log) lines of log)"
else
  fail "compositor" "didn't open its socket: $(tail -c 900 shots/compositor-alone.log | tr '\n' '|')"
fi
sleep 3
shot albireo-1-empty
if command -v wayland-info >/dev/null; then
  W timeout 10 wayland-info >shots/wayland-info.txt 2>&1
  for g in xdg_wm_base zwlr_layer_shell_v1 zwlr_foreign_toplevel_manager_v1 zxdg_decoration_manager_v1 wl_data_device_manager \
           zwp_primary_selection_device_manager_v1 zwlr_data_control_manager_v1 zwp_virtual_keyboard_manager_v1 \
           zwp_input_method_manager_v2 zwp_text_input_manager_v3 zwp_pointer_constraints_v1 wp_viewporter \
           wp_fractional_scale_manager_v1 wp_presentation xdg_activation_v1 ext_idle_notifier_v1 zwp_idle_inhibit_manager_v1; do
    grep -q "$g" shots/wayland-info.txt || fail "protocols" "$g isn't offered"
  done
  note "protocols" "$(grep -o "interface: '[a-z_0-9]*'" shots/wayland-info.txt | cut -d"'" -f2 | tr '\n' ' ' | cut -c1-900)"
fi
kill -TERM "$CPID"; for i in $(seq 1 20); do kill -0 "$CPID" 2>/dev/null || break; sleep 0.25; done
if kill -0 "$CPID" 2>/dev/null; then fail "SIGTERM" "still running 5 s after SIGTERM"; kill -KILL "$CPID"; fi
wait "$CPID"; rc=$?
[ "$rc" = 0 ] && note "SIGTERM" "ended cleanly (code 0)" || fail "SIGTERM" "ended with code $rc: $(tail -c 400 shots/compositor-alone.log | tr '\n' '|')"
grep -q "ending cleanly" shots/compositor-alone.log || fail "SIGTERM" "didn't say it was ending cleanly"
[ -e "$STATUS" ] && fail "SIGTERM" "left its status file behind"

# ---- 2: Desktop Mode for real: nimbus-session with the preview on starts the compositor, which starts Mantel.
CONF="$HOME/.config/nimbus/session.conf"
printf 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=albireo\n' > "$CONF"
LOG="$HOME/.local/share/nimbus/session.log"
rm -f "$XDG_RUNTIME_DIR/nimbus-next" "$XDG_RUNTIME_DIR/hearth-shell-up"
ALBIREO_BIN="$BIN" ALBIREO_ARGS="--winit --size 1920x1080 --socket wayland-hearth" HEARTH_KWIN_BIN=no-kwin-here \
  sh hosts/linux/session/nimbus-session >shots/session-out.log 2>&1 &
SPID=$!
for i in $(seq 1 90); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 1; done
if [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ]; then
  note "Mantel" "says it's up on Albireo ($(grep -o "albireo: Mantel up after [0-9]*s" "$LOG" | tail -1))"
else
  fail "Mantel" "didn't come up: $(tail -6 "$LOG" | tr '\n' '|' | cut -c1-400) compositor: $(tail -c 700 "$HOME/.local/share/nimbus/albireo.log" 2>/dev/null | tr '\n' '|')"
fi
grep -q "desktop mode: Mantel on Albireo (preview)" "$LOG" || fail "session" "nimbus-session didn't start Albireo: $(tail -6 "$LOG" | tr '\n' '|' | cut -c1-400)"
# The bar and the dock (and the wallpaper): Mantel's layer surfaces, on the screen.
wait_for 40 'len([l for o in d["outputs"] for l in o["layers"]]) >= 2' \
  || fail "layer shell" "Mantel's bar and dock aren't up: $(head -c 700 "$STATUS" 2>/dev/null)"
note "layer shell" "$(status '"; ".join("%s %s %dx%d at %d,%d" % (l["namespace"], l["layer"], l["w"], l["h"], l["x"], l["y"]) for o in d["outputs"] for l in o["layers"])' | cut -c1-900)"
note "work area" "$(status 'd["outputs"][0]["workArea"]') of $(status '[d["outputs"][0]["w"], d["outputs"][0]["h"]]') (Mantel's bar and dock keep their room)"
sleep 8   # Mantel's cards and the wallpaper settle
shot albireo-2-mantel
grep -iE "layer.?shell.*(not|fail|unsupported)|ToplevelManager.*(not|fail)|foreign.?toplevel.*(not|unsupported)" "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null \
  | head -3 | while read -r l; do fail "Mantel" "$(echo "$l" | cut -c1-300)"; done
[ "$(status 'd["taskbars"] >= 1')" = True ] && note "foreign toplevel" "Mantel's taskbar is listening ($(status 'd["taskbars"]') listener(s))" \
  || fail "foreign toplevel" "Mantel's taskbar (Quickshell's ToplevelManager) never asked for the windows"

# ---- 3: a window: Hearth's frame on it, and on Mantel's taskbar.
W foot --app-id=hearth-test-window --title="Albireo test" sh -c 'echo "A window on Hearth'"'"'s compositor"; sleep 600' >shots/foot.log 2>&1 &
FPID=$!
wait_for 30 'any(w["appId"] == "hearth-test-window" for w in d["windows"])' \
  || fail "window" "the test window never showed up: $(head -c 500 "$STATUS") foot: $(tail -c 300 shots/foot.log | tr '\n' '|')"
wait_for 10 'any(w["appId"] == "hearth-test-window" and w["activated"] for w in d["windows"])' \
  || fail "focus" "the new window didn't get the keyboard"
[ "$(status 'any(w["frame"] for w in d["windows"] if w["appId"] == "hearth-test-window")')" = True ] \
  && note "frames" "the window has Hearth's frame (server-side)" || note "frames" "the window draws its own frame (it asked to)"
if command -v wlrctl >/dev/null; then
  W timeout 10 wlrctl toplevel list >shots/toplevels.txt 2>&1
  grep -q "hearth-test-window" shots/toplevels.txt && note "foreign toplevel" "listed: $(tr '\n' '|' < shots/toplevels.txt | cut -c1-300)" \
    || fail "foreign toplevel" "wlrctl doesn't list the window: $(tr '\n' '|' < shots/toplevels.txt | cut -c1-300)"
fi
sleep 3
shot albireo-3-window

# ---- 4: minimized and back, as Mantel's taskbar does it (foreign toplevel requests).
if command -v wlrctl >/dev/null; then
  W timeout 10 wlrctl toplevel minimize app_id:hearth-test-window
  wait_for 10 'any(w["appId"] == "hearth-test-window" and w["minimized"] for w in d["windows"])' \
    || fail "minimize" "the taskbar's minimize didn't minimize it"
  sleep 2
  shot albireo-4-minimized
  W timeout 10 wlrctl toplevel focus app_id:hearth-test-window
  wait_for 10 'any(w["appId"] == "hearth-test-window" and not w["minimized"] and w["activated"] for w in d["windows"])' \
    || fail "minimize" "activating it from the taskbar didn't bring it back"
else
  note "minimize" "no wlrctl here: skipped"
  shot albireo-4-minimized
fi

# ---- 5: dragged by its title bar to the left edge: the left half of the work area.
if command -v xdotool >/dev/null; then
  read -r x y w <<<"$(status '"%d %d %d" % next((w["x"], w["y"], w["w"]) for w in d["windows"] if w["appId"] == "hearth-test-window")')"
  if [ -n "${x:-}" ]; then
    tx=$((x + w / 3)); ty=$((y + 16))
    xdotool mousemove "$tx" "$ty"; sleep 0.5
    xdotool mousedown 1; sleep 0.3
    for step in 1 2 3 4 5 6 7 8; do xdotool mousemove $(( tx - (tx - 1) * step / 8 )) $(( ty + 300 * step / 8 )); sleep 0.15; done
    xdotool mousemove 0 $((ty + 300)); sleep 0.4
    xdotool mouseup 1; sleep 2
    wa_x=$(status 'd["outputs"][0]["workArea"][0]'); wa_w=$(status 'd["outputs"][0]["workArea"][2]')
    got=$(status '"%d %d" % next((w["x"], w["w"]) for w in d["windows"] if w["appId"] == "hearth-test-window")')
    note "snapping" "work area x=$wa_x w=$wa_w; window now x,w = $got"
    [ "$got" = "$wa_x $((wa_w / 2))" ] || fail "snapping" "the window isn't on the left half ($got, wanted $wa_x $((wa_w / 2)))"
  fi
else
  note "snapping" "no xdotool here: skipped"
fi
sleep 2
shot albireo-5-snapped

# ---- 6: Settings › System › Your desktop, with the preview switch.
WAYLAND_DISPLAY=wayland-hearth quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" call settings page your-desktop >/dev/null 2>&1 || fail "settings" "Mantel's ipc didn't answer"
sleep 5
shot albireo-6-your-desktop
python3 hosts/linux/desktop/shell/hearth-your-desktop status > shots/status.json
grep -q '"wmNow": "albireo"' shots/status.json || fail "Your desktop" "doesn't say Mantel is on Albireo: $(head -c 400 shots/status.json)"
grep -q '"albireoPreview": true' shots/status.json || fail "Your desktop" "doesn't show the preview as on"

# ---- The end: Desktop Mode ends as a switch does (nimbus-mode), the compositor ends cleanly.
kill "$FPID" 2>/dev/null
echo logout > "$XDG_RUNTIME_DIR/nimbus-next"
CP=$(cat "$XDG_RUNTIME_DIR/albireo.pid" 2>/dev/null)
HEARTH_DESKTOP_PID=$(pgrep -x albireo | head -1) HEARTH_WM=albireo sh hosts/linux/session/nimbus-mode logout
for i in $(seq 1 30); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
kill -0 "$SPID" 2>/dev/null && { fail "logout" "the session didn't end after nimbus-mode logout"; kill -KILL "$SPID"; }
grep -q "albireo ended (code 0)" "$LOG" && note "logout" "the compositor ended cleanly" \
  || fail "logout" "the compositor didn't end cleanly: $(grep albireo "$LOG" | tail -3 | tr '\n' '|')"
pkill -TERM quickshell; pkill -KILL -x albireo; sleep 1

cp "$LOG" shots/session.log 2>/dev/null; cp "$HOME/.local/share/nimbus/albireo.log" shots/albireo.log 2>/dev/null
cp "$XDG_RUNTIME_DIR/nimbus-shell.log" shots/shell.log 2>/dev/null
grep -iE "panic|thread .* panicked" shots/albireo.log shots/compositor-alone.log 2>/dev/null | head -3 | while read -r l; do fail "panic" "$(echo "$l" | cut -c1-400)"; done
grep -E "WARN|ERROR" shots/albireo.log 2>/dev/null | sort | uniq -c | sort -rn | head -6 | cut -c1-300 | while read -r l; do note "compositor log" "$l"; done
grep -E 'desktop|albireo' "$LOG" | tail -8 | cut -c1-200 | while read -r l; do note "session" "$l"; done
ls -la shots
for s in albireo-1-empty albireo-2-mantel albireo-3-window albireo-4-minimized albireo-5-snapped albireo-6-your-desktop; do
  [ -s "shots/$s.png" ] || fail "screenshot" "$s is missing ($(tail -2 shots/shot-errors.txt 2>/dev/null | tr '\n' '|'))"
done
python3 - <<'PY'
import os
from PIL import Image, ImageStat
for n in sorted(os.listdir('shots')):
    if n.endswith('.png'):
        im = Image.open('shots/' + n).convert('RGB')
        st = ImageStat.Stat(im.convert('L'))
        colours = len(im.getcolors(1 << 24) or [])
        print('::notice title=picture %s::%dx%d, brightness %.0f, spread %.1f, %d colours' % (n, im.width, im.height, st.mean[0], st.stddev[0], colours))
        # The first picture is the background on its own: one colour on purpose, and it must be Mantel's (Hearth
        # dark's bg, #0f0b09: the compositor drew it; black would be nothing drawn). Every other one must show something.
        if n.startswith('albireo-1-'):
            px = im.getpixel((im.width // 2, im.height // 2))
            if max(abs(a - b) for a, b in zip(px, (15, 11, 9))) > 3:
                print('::error title=screenshot::%s: the middle is %s, not Mantel\'s background #0f0b09' % (n, px))
                open('shots/FAILED', 'w').close()
        elif colours <= 1:
            print('::error title=screenshot::%s is one flat colour (nothing drawn)' % n)
            open('shots/FAILED', 'w').close()
PY
[ ! -e shots/FAILED ]
