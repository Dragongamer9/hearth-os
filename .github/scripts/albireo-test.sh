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
#   albireo-7-locked  Mantel's lock screen (ext-session-lock), nothing else on the screen
#   albireo-8-lock-prompt  the lock screen killed: still locked, the compositor's own prompt after a wrong password
#   albireo-9-unlocked  the right password in that prompt: the desktop back
#   albireo-10-grim   grim's own picture of the screen (screencopy), checked against the screen's
#   albireo-11-display  Settings › Display, its screen list from wlr-randr (wlr-output-management)
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
# How alike two pictures are (0 = the same; the virtual screen's own picture against an app's through screencopy).
alike() { python3 -c '
import sys
from PIL import Image, ImageChops, ImageStat
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size: print("size %sx%s vs %sx%s" % (a.size + b.size)); sys.exit()
print("%.1f" % (sum(ImageStat.Stat(ImageChops.difference(a, b)).mean) / 3))' "$1" "$2" 2>&1; }

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
           wp_fractional_scale_manager_v1 wp_presentation xdg_activation_v1 ext_idle_notifier_v1 zwp_idle_inhibit_manager_v1 \
           ext_session_lock_manager_v1 zwlr_screencopy_manager_v1 ext_image_copy_capture_manager_v1 \
           ext_output_image_capture_source_manager_v1 zwlr_output_manager_v1 zwlr_gamma_control_manager_v1; do
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
ALBIREO_RELOCK_AFTER=120 ALBIREO_BIN="$BIN" ALBIREO_ARGS="--winit --size 1920x1080 --socket wayland-hearth" HEARTH_KWIN_BIN=no-kwin-here \
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

# ---- 7: the lock screen (ext-session-lock): Super+L's command locks, and nothing behind it has the keyboard.
# (This run's compositor waits 120 s before asking Mantel to lock again, so step 8 sees its own prompt.)
LOCKPW=hearthtest42
echo "root:$LOCKPW" | chpasswd 2>>shots/lock.log || fail "lock" "couldn't give the test user a password"
qs() { WAYLAND_DISPLAY=wayland-hearth timeout 10 quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" "$@" >/dev/null 2>&1; }
focus() { status 'd["keyboardFocus"]'; }
qs call lock lock || fail "lock" "Mantel's ipc didn't answer the lock call"
if wait_for 20 'd["lock"]["locked"] and d["lock"]["client"] and d["lock"]["surfaces"] >= 1'; then
  note "lock" "locked by Mantel's lock screen ($(status 'd["lock"]["surfaces"]') lock surface(s)); the keyboard is the $(focus)'s"
else
  fail "lock" "Mantel's lock screen didn't lock: $(status 'd["lock"]') shell: $(grep -i lock "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null | tail -3 | tr '\n' '|' | cut -c1-300)"
fi
sleep 4
[ "$(focus)" = lock ] || fail "lock" "while locked the keyboard is the $(focus)'s, not the lock screen's"
shot albireo-7-locked
# A screen picture while it's locked shows the lock screen, never what's behind it.
if command -v grim >/dev/null; then
  W timeout 15 grim shots/grim-locked.png 2>shots/grim-locked.err
  d=$(alike shots/grim-locked.png shots/albireo-7-locked.png)
  note "screencopy" "grim while locked: $(head -c 200 shots/grim-locked.err | tr '\n' ' ') difference from the screen $d"
  awk -v d="$d" 'BEGIN { exit !(d + 0 < 12 && d ~ /^[0-9.]+$/) }' || fail "screencopy" "grim's picture while locked isn't the lock screen (difference $d)"
fi
grep -q "the lock screen is told it's locked" "$HOME/.local/share/nimbus/albireo.log" 2>/dev/null || note "lock" "the compositor's log doesn't say it confirmed the lock"

# ---- 8: the lock screen dies (Mantel killed): still locked, with the compositor's own prompt; a wrong password
# keeps it locked. Then Mantel (back again) locks once more and takes over, dies once more, and the right password
# in the prompt unlocks: never unlocked by itself, nobody locked out.
pkill -KILL -x quickshell
if wait_for 10 'd["lock"]["locked"] and d["lock"]["prompt"] and not d["lock"]["client"]'; then
  note "lock" "the lock screen died: still locked, the built-in prompt is up"
else
  fail "lock" "after the lock screen died: $(status 'd["lock"]')"
fi
case "$(focus)" in window:*|layer:*) fail "lock" "the lock screen died and the keyboard went to $(focus)";; esac
# The keys typed on the virtual screen go to the compositor's window (no window manager there gives it the focus).
XWIN=$(xdotool search --name 'Albireo' 2>/dev/null | head -1)
[ -n "$XWIN" ] && xdotool windowfocus --sync "$XWIN" 2>/dev/null
xdotool mousemove 960 540
sleep 2
xdotool type --delay 80 "not-the-password"; xdotool key Return
sleep 5
[ "$(status 'd["lock"]["locked"]')" = True ] && note "lock" "a wrong password: still locked ($(status 'd["lock"]["message"]'))" \
  || fail "lock" "a wrong password unlocked it: $(status 'd["lock"]')"
status 'd["lock"]["message"]' | grep -q "isn't right" || fail "lock" "the prompt didn't say the password was wrong: $(status 'd["lock"]') (window $XWIN; log: $(grep -E 'lock:' "$HOME/.local/share/nimbus/albireo.log" | tail -3 | tr '\n' '|' | cut -c1-400))"
shot albireo-8-lock-prompt
for i in $(seq 1 40); do qs call lock lock && break; sleep 1; done
if wait_for 15 'd["lock"]["locked"] and d["lock"]["client"] and not d["lock"]["prompt"]'; then
  note "lock" "Mantel came back and its lock screen took over the lock"
else
  fail "lock" "Mantel's lock screen didn't take over: $(status 'd["lock"]')"
fi
sleep 3
pkill -KILL -x quickshell
wait_for 10 'd["lock"]["prompt"]' || fail "lock" "the second time the lock screen died there was no prompt: $(status 'd["lock"]')"
sleep 7   # the last wrong try's wait is long over; type the right one
[ -n "$XWIN" ] && xdotool windowfocus --sync "$XWIN" 2>/dev/null
xdotool type --delay 80 "$LOCKPW"; xdotool key Return
if wait_for 15 'not d["lock"]["locked"]'; then
  note "lock" "the right password in the built-in prompt unlocked it; the keyboard is back with $(focus)"
else
  fail "lock" "the right password didn't unlock it: $(status 'd["lock"]') ($(ls -l /usr/sbin/unix_chkpwd 2>&1))"
fi
for i in $(seq 1 30); do qs call settings page your-desktop && break; sleep 1; done
sleep 4
shot albireo-9-unlocked

# ---- 10: screen pictures for apps: grim (ext-image-copy-capture or wlr-screencopy), a region, and wf-recorder
# recording (wlr-screencopy with damage). Each picture has to match what the screen shows.
if command -v grim >/dev/null; then
  before=$(status '"%d %d" % (d["captures"]["wlr"], d["captures"]["ext"])')
  shot shots-tmp-screen
  W timeout 15 grim shots/albireo-10-grim.png 2>shots/grim.err || fail "screencopy" "grim failed: $(head -c 400 shots/grim.err | tr '\n' ' ')"
  d=$(alike shots/albireo-10-grim.png shots/shots-tmp-screen.png)
  note "screencopy" "grim's picture vs the screen: difference $d (0 is the same); captures before $before, now $(status '"%d %d" % (d["captures"]["wlr"], d["captures"]["ext"])')"
  awk -v d="$d" 'BEGIN { exit !(d + 0 < 12 && d ~ /^[0-9.]+$/) }' || fail "screencopy" "grim's picture doesn't match the screen (difference $d)"
  W timeout 15 grim -g "100,200 640x360" shots/grim-region.png 2>>shots/grim.err
  python3 -c 'from PIL import Image; import sys; im = Image.open(sys.argv[1]); print(im.size); sys.exit(im.size != (640, 360))' shots/grim-region.png >>shots/grim.err 2>&1 \
    && note "screencopy" "a region (grim -g) is 640x360" || fail "screencopy" "grim -g didn't give 640x360: $(tail -c 300 shots/grim.err | tr '\n' ' ')"
  rm -f shots/shots-tmp-screen.png shots/grim-region.png
else
  fail "screencopy" "grim isn't installed"
fi
if command -v wf-recorder >/dev/null; then
  w0=$(status 'd["captures"]["wlr"] + d["captures"]["ext"]')
  ( W timeout -s INT 6 wf-recorder -y -c ffv1 -f shots/recording.mkv >shots/wf-recorder.log 2>&1 ) &
  RPID=$!
  sleep 1.5; W wtype "recorded" 2>/dev/null; sleep 1; xdotool mousemove 600 500; sleep 1; xdotool mousemove 900 300
  wait "$RPID"
  w1=$(status 'd["captures"]["wlr"] + d["captures"]["ext"]')
  size=$(stat -c %s shots/recording.mkv 2>/dev/null || echo 0)
  if [ "$size" -gt 10000 ] && [ "$((w1 - w0))" -ge 2 ]; then
    note "screencopy" "wf-recorder recorded $size bytes, $((w1 - w0)) frames copied"
  else
    fail "screencopy" "wf-recorder: $size bytes, $((w1 - w0)) frames: $(tail -c 500 shots/wf-recorder.log | tr '\n' ' ')"
  fi
  rm -f shots/recording.mkv
else
  fail "screencopy" "wf-recorder isn't installed"
fi

# ---- 11: the screens for wlr-randr (wlr-output-management), as Settings › Display uses it: listed with their modes,
# a scale change applies and goes back, a mode the nested window can't take is refused; then Settings › Display.
if command -v wlr-randr >/dev/null; then
  W timeout 10 wlr-randr --json > shots/wlr-randr.json 2>shots/wlr-randr.err
  heads=$(python3 -c 'import json,sys; o=json.load(open(sys.argv[1])); print("%d screen(s): %s, %s %s, %d mode(s), scale %s" % (len(o), o[0]["name"], o[0]["make"], o[0]["model"], len(o[0]["modes"]), o[0]["scale"]))' shots/wlr-randr.json 2>&1)
  case "$heads" in *"screen(s): winit"*) note "output management" "wlr-randr: $heads";; *) fail "output management" "wlr-randr --json: $heads $(head -c 300 shots/wlr-randr.err)";; esac
  W timeout 10 wlr-randr --output winit --scale 1.25 2>>shots/wlr-randr.err
  wait_for 10 'abs(d["outputs"][0]["scale"] - 1.25) < 0.01' && note "output management" "scale 1.25 applied (the work area is now $(status 'd["outputs"][0]["workArea"]'))" \
    || fail "output management" "wlr-randr --scale 1.25 didn't apply: $(status 'd["outputs"][0]') $(tail -c 300 shots/wlr-randr.err)"
  W timeout 10 wlr-randr --output winit --scale 1 2>>shots/wlr-randr.err
  wait_for 10 'abs(d["outputs"][0]["scale"] - 1.0) < 0.01' || fail "output management" "the scale didn't go back to 1"
  if W timeout 10 wlr-randr --output winit --mode 1280x720 2>>shots/wlr-randr.err; then
    fail "output management" "nested, a different mode was accepted"
  else
    note "output management" "a mode the nested window can't take was refused, as it should be"
  fi
  [ "$(status 'd["outputs"][0]["w"]')" = 1920 ] || fail "output management" "the screen isn't 1920 wide any more: $(status 'd["outputs"][0]')"
else
  fail "output management" "wlr-randr isn't installed"
fi
qs call settings page display || fail "settings" "Mantel's ipc didn't open Settings › Display"
sleep 6
shot albireo-11-display

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
for s in albireo-1-empty albireo-2-mantel albireo-3-window albireo-4-minimized albireo-5-snapped albireo-6-your-desktop \
         albireo-7-locked albireo-8-lock-prompt albireo-9-unlocked albireo-10-grim albireo-11-display; do
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
