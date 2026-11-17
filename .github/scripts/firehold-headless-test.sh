#!/bin/bash
# Firehold headless (firehold.yml "headless"; docs/FIREHOLD.md "How it's tested"): the compositor with no screen and no
# window (--headless: drawn into memory with Mesa's software GL), started by nimbus-session with the preview on, so
# Desktop Mode comes up on it exactly as on a device: no virtual X screen anywhere. Its pictures come from the
# compositor itself, through screencopy (grim).
#   firehold-h-mantel  Mantel's bar, dock and wallpaper and a window, as grim sees them
set -u +e +o pipefail
mkdir -p shots
SRC=$PWD
BIN=${FIREHOLD:-$SRC/firehold-comp}
chmod +x "$BIN"
fail() { echo "::error title=headless $1::$2"; touch shots/FAILED; }
note() { echo "::notice title=headless $1::$2"; }
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1 QT_FORCE_STDERR_LOGGING=1
export NIMBUS_SHARE="$SRC/hosts/linux" NIMBUS_APP=/nonexistent
export FIREHOLD_LOG="info,smithay=warn"
unset DISPLAY WAYLAND_DISPLAY
chmod +x hosts/linux/desktop/labwc/autostart hosts/linux/desktop/firehold/autostart hosts/linux/desktop/theme/apply.sh
mkdir -p "$HOME/.cache/hearth" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus"
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/share/nimbus/desktop.json" '{}' >/dev/null
STATUS="$XDG_RUNTIME_DIR/firehold.json"
status() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2], {"d": d}))' "$STATUS" "$1" 2>/dev/null; }
wait_for() { for i in $(seq 1 $(( $1 * 2 ))); do [ "$(status "$2")" = True ] && return 0; sleep 0.5; done; return 1; }
W() { WAYLAND_DISPLAY=wayland-hearth "$@"; }

printf 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=firehold\n' > "$HOME/.config/nimbus/session.conf"; printf '{"firehold": {"unlocked": true}}\n' > "$HOME/.config/nimbus/firehold.json"; mkdir -p "$HOME/.local/state/hearth" && : > "$HOME/.local/state/hearth/firehold-intro-told"
LOG="$HOME/.local/share/nimbus/session.log"
FIREHOLD_BIN="$BIN" FIREHOLD_ARGS="--headless --size 1920x1080 --socket wayland-hearth" HEARTH_KWIN_BIN=no-kwin-here \
  sh hosts/linux/session/nimbus-session >shots/session-out.log 2>&1 &
SPID=$!
for i in $(seq 1 90); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 1; done
if [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ]; then
  note "Mantel" "up on Firehold with no screen at all"
else
  fail "Mantel" "didn't come up: $(tail -5 "$LOG" | tr '\n' '|' | cut -c1-400) compositor: $(tail -c 800 "$HOME/.local/share/nimbus/firehold.log" 2>/dev/null | tr '\n' '|')"
fi
grep -q "headless: drawing into memory" "$HOME/.local/share/nimbus/firehold.log" 2>/dev/null \
  || fail "backend" "the compositor didn't say it's drawing into memory: $(head -c 600 "$HOME/.local/share/nimbus/firehold.log" | tr '\n' '|')"
wait_for 40 'len([l for o in d["outputs"] for l in o["layers"]]) >= 2' || fail "layer shell" "Mantel's bar and dock aren't up: $(head -c 500 "$STATUS")"
note "screen" "$(status '"%s %s %s %dx%d" % (d["outputs"][0]["name"], d["outputs"][0]["make"], d["outputs"][0]["model"], d["outputs"][0]["w"], d["outputs"][0]["h"])')"
W foot --app-id=hearth-test-window --title="Firehold headless" sh -c 'echo "Drawn into memory: no screen, no window"; sleep 600' >shots/foot.log 2>&1 &
FPID=$!
wait_for 30 'any(w["appId"] == "hearth-test-window" for w in d["windows"])' || fail "window" "the test window never showed up: $(tail -c 300 shots/foot.log | tr '\n' '|')"
sleep 8
W timeout 20 grim shots/firehold-h-mantel.png 2>shots/grim.err || fail "screencopy" "grim failed: $(head -c 400 shots/grim.err | tr '\n' ' ')"
note "screencopy" "captures: $(status 'd["captures"]')"
# Recording (wf-recorder, wlr-screencopy with damage), shared memory only: there's no dmabuf headless.
if command -v wf-recorder >/dev/null; then
  w0=$(status 'd["captures"]["wlr"] + d["captures"]["ext"]')
  ( W timeout -s INT 6 wf-recorder -y -c ffv1 -f shots/recording.mkv >shots/wf-recorder.log 2>&1 ) &
  RPID=$!
  sleep 2; W foot --app-id=hearth-rec sh -c 'sleep 3' >/dev/null 2>&1 &
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
if command -v wlr-randr >/dev/null; then
  W timeout 10 wlr-randr --output HEADLESS-1 --scale 1.5 && wait_for 10 'abs(d["outputs"][0]["scale"] - 1.5) < 0.01' \
    && note "output management" "scale 1.5 applied headless" || fail "output management" "scale didn't apply: $(status 'd["outputs"][0]')"
  W timeout 10 wlr-randr --output HEADLESS-1 --scale 1
fi

kill "$FPID" 2>/dev/null
echo logout > "$XDG_RUNTIME_DIR/nimbus-next"
HEARTH_DESKTOP_PID=$(pgrep -x firehold-comp | head -1) HEARTH_WM=firehold sh hosts/linux/session/nimbus-mode logout
for i in $(seq 1 30); do kill -0 "$SPID" 2>/dev/null || break; sleep 0.5; done
kill -0 "$SPID" 2>/dev/null && { fail "logout" "the session didn't end"; kill -KILL "$SPID"; }
grep -q "firehold ended (code 0)" "$LOG" && note "logout" "ended cleanly" || fail "logout" "didn't end cleanly: $(grep firehold "$LOG" | tail -3 | tr '\n' '|')"
pkill -TERM quickshell; pkill -KILL -x firehold-comp; sleep 1
cp "$HOME/.local/share/nimbus/firehold.log" shots/firehold.log 2>/dev/null
grep -iE "panicked" shots/firehold.log 2>/dev/null | head -3 | while read -r l; do fail "panic" "$(echo "$l" | cut -c1-400)"; done
python3 - <<'PY'
import os
from PIL import Image, ImageStat
p = 'shots/firehold-h-mantel.png'
if not os.path.exists(p):
    print('::error title=headless screenshot::no picture'); open('shots/FAILED', 'w').close()
else:
    im = Image.open(p).convert('RGB')
    colours = len(im.getcolors(1 << 24) or [])
    print('::notice title=headless picture::%dx%d, brightness %.0f, %d colours' % (im.width, im.height, ImageStat.Stat(im.convert('L')).mean[0], colours))
    if im.size != (1920, 1080) or colours <= 1:
        print('::error title=headless screenshot::%s is %dx%d with %d colour(s)' % (p, im.width, im.height, colours)); open('shots/FAILED', 'w').close()
PY
[ ! -e shots/FAILED ]
