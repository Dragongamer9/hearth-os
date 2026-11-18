#!/bin/bash
# Hearth's own apps, measured on KWin (1.15 "Hearth's own apps made faster"; apps-speed.yml): the desktop shell from
# BEFORE (bughunt's, unpacked by the workflow) and from this checkout, one after the other on the same KWin virtual
# screen (1920x1080, software drawing as in kwin-test.yml), so the numbers compare:
#   - the shell's memory and CPU with nothing open (tools/perf-measure.py idle), and again after every app was opened
#     once and closed;
#   - how long Hearth Files, Settings, Task Manager, the app menu and Quick Access take to show and to settle, each
#     opened twice (cold, then warm), and how much of that is only `quickshell ipc` starting;
#   - with BREAKDOWN=1, where the memory goes: the shell started again without each of its parts
#     (tools/perf-breakdown.py).
# Report only (notices and the step summary); pictures of Files and Settings on KWin go in shots/.
set -u +e +o pipefail
SRC=$PWD
mkdir -p shots/speed
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1
export NIMBUS_APP=/nonexistent HEARTH_PERF_KWIN=1 HEARTH_PERF_QUIET=1
# As kwin-test.sh: the launcher's Games, desktop.json there from the start, no one-time cards in the pictures.
mkdir -p "$HOME/.cache/hearth" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$HOME/.local/state/hearth" "$HOME/Documents" "$HOME/Pictures"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/share/nimbus/desktop.json" '{}' >/dev/null
python3 hosts/linux/desktop/shell/hearth-browser asked >/dev/null 2>&1 || true
: >"$HOME/.local/state/hearth/kwin-lock-off-told"
for i in $(seq 1 40); do echo "file $i" > "$HOME/Documents/note-$i.txt"; done

# KWin on its own virtual screen (a plain copy: the installed one has a file capability a container can't grant).
modprobe vgem 2>/dev/null || echo "::warning title=apps speed::no vgem module"
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
WAYLAND_DISPLAY= /tmp/kwin_wayland --virtual --width 1920 --height 1080 --socket wayland-hearth >shots/kwin.log 2>&1 &
for i in $(seq 1 30); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { echo "::error title=apps speed::KWin didn't start: $(tail -5 shots/kwin.log | tr '\n' '|')"; exit 1; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
sleep 3

cat > /tmp/shot.py <<'PY'
import dbus, os, sys
from PIL import Image
i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
res = i.CaptureActiveScreen({'native-resolution': True}, dbus.types.UnixFd(w), timeout=15)
os.close(w)
data = b''
while True:
    c = os.read(r, 1 << 20)
    if not c:
        break
    data += c
W, H, S = int(res['width']), int(res['height']), int(res['stride'])
Image.frombuffer('RGBA', (W, H), data, 'raw', 'BGRA', S, 1).convert('RGB').save(sys.argv[1])
PY
shot() { timeout 20 python3 /tmp/shot.py "shots/$1.png" 2>>shots/shot-errors.txt || echo "no screenshot $1" >>shots/shot-errors.txt; }

SPID=""
start_shell() {   # $1: the tree (hosts/linux inside it)
  export NIMBUS_SHARE="$1/hosts/linux"
  rm -f "$XDG_RUNTIME_DIR/hearth-shell-up"
  quickshell -p "$NIMBUS_SHARE/desktop/shell" >>"shots/shell-$(basename "$1").log" 2>&1 &
  SPID=$!
  for i in $(seq 1 60); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && return 0; sleep 1; done
  echo "::warning title=apps speed::the shell from $1 didn't say it was up"
}
stop_shell() { kill -TERM "$SPID" 2>/dev/null; sleep 3; kill -KILL "$SPID" 2>/dev/null; pkill -KILL -x quickshell 2>/dev/null; sleep 2; }

measure() {   # $1: before | after, $2: the tree
  local P="$SRC/shots/speed/$1.json"
  rm -f "$P"
  start_shell "$2"
  local D="$NIMBUS_SHARE/desktop/shell"
  q() { quickshell ipc -p "$D" call "$@" >/dev/null 2>&1; }
  sleep 20
  python3 "$SRC/tools/perf-measure.py" idle 30 "$P"
  python3 "$SRC/tools/perf-measure.py" ipc "IPC" "$P" -- quickshell ipc -p "$D" call filemanager where
  m() { python3 "$SRC/tools/perf-measure.py" open "$1" "$P" -- quickshell ipc -p "$D" call $2; q $3; sleep 3; }
  for run in cold warm; do
    m "Hearth Files ($run)" "files home" "files close"
    m "Settings ($run)" "settings open" "settings close"
    m "Settings › Display ($run)" "settings page display" "settings close"
    m "Task Manager ($run)" "tasks open" "tasks close"
    m "App menu ($run)" "launcher open" "launcher toggle"
    m "Quick Access ($run)" "quick toggle" "quick toggle"
  done
  m "Hearth Files (warm, a folder)" "files open $HOME/Documents" "files close"
  sleep 5
  python3 "$SRC/tools/perf-measure.py" idle 20 "$P" idle-after
  # Pictures on KWin: Files and Settings as they open here.
  q files home; sleep 4; shot "apps-$1-files"; q files close
  q settings open; sleep 4; shot "apps-$1-settings"; q settings close
  stop_shell
  grep -iE "TypeError|ReferenceError|is not defined|Cannot assign|non-existent" "shots/shell-$(basename "$2").log" | sort | uniq -c | sort -rn | head -5 | cut -c1-240 \
    | while read -r l; do echo "::warning title=apps speed: shell log ($1)::$l"; done
}

measure before "${BEFORE:?the tree to compare against}"
measure after "$SRC"
# (Its notices are posted by the next step: a step keeps 10.)
python3 tools/perf-measure.py compare shots/speed/before.json shots/speed/after.json > shots/speed/compare.md
grep '^::' shots/speed/compare.md > shots/speed/compare-notices.txt
{ echo '### Hearth apps on KWin, before → after (virtual screen in CI: compare runs with runs)'; echo; grep -v '^::' shots/speed/compare.md; echo;
  echo '#### Before'; python3 tools/perf-measure.py report shots/speed/before.json; echo; echo '#### After'; python3 tools/perf-measure.py report shots/speed/after.json; } >> "${GITHUB_STEP_SUMMARY:-/dev/null}"

if [ "${BREAKDOWN:-0}" = 1 ]; then
  export NIMBUS_SHARE="$SRC/hosts/linux"
  python3 tools/perf-breakdown.py run "$SRC/hosts/linux/desktop/shell" shots/speed/breakdown.json 2>&1 | tail -3
  python3 tools/perf-breakdown.py report shots/speed/breakdown.json > shots/speed/breakdown.md
  { echo '### Where the shell'"'"'s memory goes (left out one at a time)'; echo; cat shots/speed/breakdown.md; } >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
  # The table as notices (3800 characters each).
  python3 - <<'PY' > shots/speed/breakdown-notices.txt
t = open('shots/speed/breakdown.md').read().replace('\n', ' ⏎ ')
for i in range(0, min(len(t), 9 * 3800), 3800):
    print('::notice title=breakdown %d::%s' % (i // 3800 + 1, t[i:i + 3800]))
PY
fi
pkill -TERM -f /tmp/kwin_wayland
exit 0
