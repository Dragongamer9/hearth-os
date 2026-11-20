#!/bin/bash
# Capture (hearth-capture-app) on KDE's own KWin, as the Fedora edition has it (hearth-apps-rust.yml's kwin job;
# docs/mockups/checklists/capture-rust.md): KWin on a virtual screen (two screens side by side, then one), Capture
# installed the way os-bases/anatase/build.sh installs it ("Capture (Rust)": its menu entry, Print Screen and
# Super+Shift+S, hearth-screenshot, the recorder), and KWin's own permission checks on. Keys reach KWin through its
# fake input (ci-kwin-tools.sh), so its global shortcuts see them as typed keys. Pictures in OUT (kw-*.png), checks
# as ::error:: / ::notice:: lines (and OUT/FAILED).
#   hearth-apps-rust-kwin.sh OUT
set +e +o pipefail
OUT=${1:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/kwin-step.log") 2>&1
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }
ok() { echo "::notice title=$1::$2"; }
source "$(dirname "$0")/ci-kwin-tools.sh"

export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home USER=root
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Pictures/Screenshots" "$HOME/Videos" "$HOME/.config/nimbus" && chmod 700 "$XDG_RUNTIME_DIR"
# The folders as a desktop sign-in sets them (xdg-user-dirs; without it xdg-user-dir says the home folder).
printf 'XDG_PICTURES_DIR="$HOME/Pictures"\nXDG_VIDEOS_DIR="$HOME/Videos"\n' > "$HOME/.config/user-dirs.dirs"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 LIBGL_ALWAYS_SOFTWARE=1 HEARTH_RENDERER=cpu
export QT_LOGGING_RULES="kwin_scripting.debug=true;js.debug=true;kf.globalaccel*=true;kglobalaccel*=true"
export KWIN_LOG="$OUT/kwin.log"
SH=/usr/share/nimbus/desktop/shell
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 --noincremental >/dev/null 2>&1
pipewire >"$OUT/pipewire.log" 2>&1 & sleep 1
wireplumber >"$OUT/wireplumber.log" 2>&1 & sleep 2

start_kwin() {   # start_kwin SCREENS
  pkill -f /tmp/kwin_wayland; sleep 1
  cp "$(command -v kwin_wayland)" /tmp/kwin_wayland 2>/dev/null
  rm -f "$XDG_RUNTIME_DIR/wayland-hearth"
  /tmp/kwin_wayland --virtual --width 1920 --height 1080 --output-count "$1" --no-lockscreen --socket wayland-hearth >>"$KWIN_LOG" 2>&1 &
  for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
  [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start: $(tail -5 "$KWIN_LOG" | tr '\n' '|' | cut -c1-600)"; return 1; }
  export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
  dbus-update-activation-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_RUNTIME_DIR HOME QT_QPA_PLATFORM \
    KDE_FULL_SESSION KDE_SESSION_VERSION LIBGL_ALWAYS_SOFTWARE PATH XDG_DATA_DIRS 2>/dev/null
  sleep 4
  # KDE's global shortcuts (Plasma 6: inside KWin; else its own kglobalacceld, as the session starts it).
  if ! dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.NameHasOwner string:org.kde.kglobalaccel 2>/dev/null | grep -q true; then
    for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >>"$OUT/kglobalaccel.log" 2>&1 & sleep 3; break; }; done
  fi
  echo "::notice title=KWin ($1 screens)::shortcuts by $(dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetConnectionUnixProcessID string:org.kde.kglobalaccel 2>/dev/null | awk '/uint32/{print $2}' | xargs -r -I{} cat /proc/{}/comm); screens: $(timeout 10 kscreen-doctor -o 2>/dev/null | grep -o 'Output: [0-9]* [^ ]*' | tr '\n' ' ')"
}
newest() { ls -t "$HOME/Pictures/Screenshots"/*.png 2>/dev/null | head -1; }
count() { ls "$HOME/Pictures/Screenshots"/*.png 2>/dev/null | wc -l; }
halves() {   # halves NAME: the two screens of a side-by-side picture, each on its own
  python3 - "$OUT/$1.png" "$OUT/$1-a.png" "$OUT/$1-b.png" <<'PY' 2>/dev/null
import sys
from PIL import Image
im = Image.open(sys.argv[1]); w, h = im.size
if w >= 2 * h:
    im.crop((0, 0, w // 2, h)).save(sys.argv[2]); im.crop((w // 2, 0, w, h)).save(sys.argv[3])
PY
}
win() { grep -a "^$1 |" "$2" | tail -1; }   # win CLASS FILE: that window's line
geo() { echo "$1" | awk -F' [|] ' '{print $3}'; }   # "x,y wxh"

# ============================================================== two screens side by side
start_kwin 2 || exit 0
# A window to take pictures of (Hearth's Task Manager).
/usr/bin/hearth-task-manager --page performance:cpu >"$OUT/tm.log" 2>&1 &
sleep 4
kshot "$OUT/kw-00-desktop.png"

# ---- KWin's screenshots through Capture's entry (kwin.rs `shot`), with KWin's own checks on
python3 - <<'PY' > "$OUT/refused.txt" 2>&1
import dbus, os
i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
try:
    i.CaptureWorkspace({}, dbus.types.UnixFd(w), timeout=10)
    print('ALLOWED')
except dbus.DBusException as e:
    print('REFUSED', e.get_dbus_name())
PY
grep -q REFUSED "$OUT/refused.txt" && ok "KWin's checks" "KWin refuses a screenshot to a program without its own entry (plain Python): $(cut -c1-200 "$OUT/refused.txt")" \
  || echo "::warning title=KWin's checks::this KWin gave a screenshot to a program without an entry too (as root in CI), so here it doesn't show KWin refusing others: $(cut -c1-200 "$OUT/refused.txt")"
/usr/bin/hearth-capture-app shot workspace >/dev/null 2>"$OUT/direct.txt"; rc=$?
[ "$rc" = 3 ] && ok "Capture's shot" "answers only Hearth's hearth-screens (run by anyone else: exit 3)" || fail "Capture's shot" "run directly it answered (exit $rc): $(head -c 200 "$OUT/direct.txt")"
HEARTH_WM=kwin timeout 30 /usr/bin/hearth-screenshot "$OUT/kw-01-hearth-screenshot.png" > "$OUT/hearth-screenshot.txt" 2>&1; rc=$?
size=$(python3 -c 'import sys; from PIL import Image; print("%dx%d" % Image.open(sys.argv[1]).size)' "$OUT/kw-01-hearth-screenshot.png" 2>/dev/null)
[ "$rc" = 0 ] && [ -n "$size" ] && ok "KWin's screenshots" "hearth-screenshot (hearth-screens → hearth-capture-app shot → KWin's ScreenShot2, allowed by hearth-capture.desktop): $size" \
  || fail "KWin's screenshots" "hearth-screenshot didn't get a picture from KWin (exit $rc): $(tr '\n' '|' <"$OUT/hearth-screenshot.txt" | cut -c1-500)"

# ---- Print Screen: the whole screen saved right away (KDE's global shortcuts: /usr/share/kglobalaccel/hearth-capture.desktop)
# What KDE's shortcuts know of Capture (its component, and the keys of its two actions).
ka() { timeout 10 dbus-send --session --print-reply --dest=org.kde.kglobalaccel "$@" 2>&1 | grep -oE '"[^"]*"|int32 [0-9]+' | tr '\n' ' ' | cut -c1-600; }
ok "Shortcuts" "components: $(ka /kglobalaccel org.kde.KGlobalAccel.allComponents) · Capture's: $(ka /component/hearth_capture_desktop org.kde.kglobalaccel.Component.shortcutNames)"
n0=$(count)
fake "k 99 1" "k 99 0"
for i in $(seq 1 20); do [ "$(count)" -gt "$n0" ] && break; sleep 1; done
sleep 1
p=$(newest)
if [ "$(count)" -gt "$n0" ] && [ -s "$p" ]; then
  cp "$p" "$OUT/kw-02-print-screen-saved.png"
  ok "Print Screen" "saved $(basename "$p") ($(python3 -c 'import sys; from PIL import Image; print("%dx%d" % Image.open(sys.argv[1]).size)' "$p"))"
else
  # Is it the key or the launch? KDE's shortcut asked for directly.
  timeout 10 dbus-send --session --print-reply --dest=org.kde.kglobalaccel /component/hearth_capture_desktop org.kde.kglobalaccel.Component.invokeShortcut string:print > "$OUT/invoke.txt" 2>&1
  for i in $(seq 1 15); do [ "$(count)" -gt "$n0" ] && break; sleep 1; done
  echo "::warning title=Print Screen asked directly::$( [ "$(count)" -gt "$n0" ] && echo "saved a picture (so the key didn't reach the shortcut)" || echo "saved nothing either: $(tr '\n' ' ' <"$OUT/invoke.txt" | cut -c1-300)")"
  fail "Print Screen" "pressing Print Screen saved nothing in Pictures/Screenshots: $(grep -aiE 'capture|kglobalaccel|shortcut|hearth' "$KWIN_LOG" | tail -6 | tr '\n' '|' | cut -c1-700)"
fi
sleep 2; kshot "$OUT/kw-03-print-screen-notification.png"

# The screens Capture's picker goes on (hearth-screens get, KDE's own screen settings).
ok "Screens" "$(HEARTH_WM=kwin timeout 15 python3 "$SH/hearth-screens" get 2>&1 | python3 -c 'import json,sys
t=sys.stdin.read()
try: print(" · ".join("%s at %s,%s %s" % (o["name"], o["position"]["x"], o["position"]["y"], [m for m in o["modes"] if m["current"]][:1]) for o in json.loads(t)))
except Exception: print(t[-400:])')"
# ---- Super+Shift+S: the area picker, one on every screen at once
fake "k 125 1" "k 42 1" "k 31 1" "k 31 0" "k 42 0" "k 125 0"
for i in $(seq 1 25); do pgrep -f "hearth-capture-app area" >/dev/null && break; sleep 0.5; done
sleep 6
kwins "$OUT/kw-windows-picker.txt"
kshot "$OUT/kw-04-picker-every-screen.png"; halves kw-04-picker-every-screen
p0=$(win hearth-capture-pick-0 "$OUT/kw-windows-picker.txt"); p1=$(win hearth-capture-pick-1 "$OUT/kw-windows-picker.txt")
if [ -n "$p0" ] && [ -n "$p1" ]; then
  g0=$(geo "$p0"); g1=$(geo "$p1")
  if [ "$(printf '%s\n%s\n' "$g0" "$g1" | sort | tr '\n' ' ')" = "0,0 1920x1080 1920,0 1920x1080 " ]; then
    ok "One picker on every screen" "Super+Shift+S: hearth-capture-pick-0 at $g0, hearth-capture-pick-1 at $g1 (full screen each: $(echo "$p0" | grep -o 'fullscreen=[a-z]*') $(echo "$p1" | grep -o 'fullscreen=[a-z]*'))"
  else
    fail "One picker on every screen" "the pickers aren't one on each screen: $g0 / $g1 ($(tr '\n' '|' <"$OUT/kw-windows-picker.txt" | cut -c1-600))"
  fi
else
  fail "One picker on every screen" "Super+Shift+S didn't open a picker on each screen: $(tr '\n' '|' <"$OUT/kw-windows-picker.txt" | cut -c1-700)"
fi
# A box dragged on the second screen (the first one stays dimmed), then Enter takes it into the editor.
fake "a 2300 260" "s 200" "b 272 1" "s 100" "a 2600 420" "s 100" "a 2900 640" "s 200" "b 272 0"
sleep 1.5
kshot "$OUT/kw-05-picker-box-second-screen.png"; halves kw-05-picker-box-second-screen
fake "k 28 1" "k 28 0"
sleep 5
kwins "$OUT/kw-windows-editor.txt"
kshot "$OUT/kw-06-picker-taken-editor.png"; halves kw-06-picker-taken-editor
if grep -q "^hearth-capture |" "$OUT/kw-windows-editor.txt" && ! grep -q "^hearth-capture-pick" "$OUT/kw-windows-editor.txt"; then
  ok "Picker to editor" "the box on the second screen went to the editor, the pickers closed"
else
  fail "Picker to editor" "after Enter: $(tr '\n' '|' <"$OUT/kw-windows-editor.txt" | cut -c1-600)"
fi
pkill -f "hearth-capture-app area"; sleep 1

# ---- the countdown, the capture dot and the scrolling capture's pill: where KWin's script puts them (kwin.rs
# script_for), and out of the pictures (KWin's excludeFromCapture). Each twice: as it is (checked: not in KWin's own
# screenshot), then with HEARTH_CAPTURE_CI_IN_PICTURES (only for these pictures) so the picture shows where it went.
pill() {   # pill NAME CLASS ARGS...: its line in KWin's window list in $PL
  local name=$1 cls=$2; shift 2
  /usr/bin/hearth-capture-app "$@" >"$OUT/$name.log" 2>&1 & local p=$!
  sleep 3.5; kwins "$OUT/kw-windows-$name.txt"; kshot "$OUT/$name-out.png"
  kill $p 2>/dev/null; wait $p 2>/dev/null; sleep 0.5
  HEARTH_CAPTURE_CI_IN_PICTURES=1 /usr/bin/hearth-capture-app "$@" >>"$OUT/$name.log" 2>&1 & p=$!
  sleep 3.5; kshot "$OUT/$name.png"; halves "$name"
  kill $p 2>/dev/null; wait $p 2>/dev/null; sleep 0.5
  PL=$(win "$cls" "$OUT/kw-windows-$name.txt")
  local g; g=$(geo "$PL")
  # Out of the pictures: at its place, KWin's screenshot with it up is the desktop, not the pill.
  local d; d=$(python3 - "$OUT/$name-out.png" "$OUT/$name.png" "$g" <<'PY' 2>/dev/null
import sys
from PIL import Image, ImageChops, ImageStat
a, b = Image.open(sys.argv[1]).convert('RGB'), Image.open(sys.argv[2]).convert('RGB')
x, y = map(int, sys.argv[3].split(' ')[0].split(',')); w, h = map(int, sys.argv[3].split(' ')[1].split('x'))
box = (x, y, x + w, y + h)
print('%.1f' % (sum(ImageStat.Stat(ImageChops.difference(a.crop(box), b.crop(box))).mean) / 3))
PY
)
  if [ -n "$d" ] && python3 -c "import sys; sys.exit(0 if float('$d') > 8 else 1)"; then
    ok "$name out of the pictures" "KWin's screenshot leaves it out ($(echo "$PL" | grep -o 'excludeFromCapture=[a-z/]*'); its place differs by $d with and without)"
  else
    fail "$name out of the pictures" "it's in KWin's screenshots (difference at its place '$d'; $(echo "$PL" | grep -o 'excludeFromCapture=[a-z/]*'))"
  fi
}
pill kw-07-countdown hearth-capture-pill --take all --delay 10
g=$(geo "$PL")
case "$g" in
  "830,40 260x64"|"2750,40 260x64") ok "Countdown's place" "at the top, in the middle of its screen: $g ($(echo "$PL" | grep -o 'keepAbove=[a-z]*'))" ;;
  *) fail "Countdown's place" "not at the top in the middle: '$g'" ;;
esac
mkdir -p "$XDG_RUNTIME_DIR/hearth-record"
printf '{"state": "recording", "since": %s}\n' "$(( $(date +%s) - 83 ))" > "$XDG_RUNTIME_DIR/hearth-record/state.json"
pill kw-08-capture-dot hearth-capture-dot --indicator
g=$(geo "$PL")
case "$g" in
  "1608,56 "*|"3528,56 "*) ok "Capture dot's place" "at the top right of its screen: $g" ;;
  *) fail "Capture dot's place" "not at the top right: '$g'" ;;
esac
rm -f "$XDG_RUNTIME_DIR/hearth-record/state.json"
pill kw-09-scroll-pill-top hearth-capture-pill --scroll-pill; gt=$(geo "$PL")
pill kw-10-scroll-pill-bottom hearth-capture-pill --scroll-pill --scroll-pill-area 200,40,700,400; gb=$(geo "$PL")
yt=$(echo "$gt" | sed -E 's/^[0-9-]+,([0-9-]+) .*/\1/'); yb=$(echo "$gb" | sed -E 's/^[0-9-]+,([0-9-]+) .*/\1/')
hb=$(echo "$gb" | sed -E 's/.*x([0-9]+)$/\1/')
if [ "$yt" = 24 ] && [ -n "$yb" ] && [ -n "$hb" ] && [ $((yb + hb)) = 1056 ]; then
  ok "Scroll pill's place" "at the top ($gt); for an area near the top, at the bottom ($gb)"
else
  fail "Scroll pill's place" "top '$gt', bottom '$gb'"
fi

# ============================================================== one screen: recording (KDE's screen-sharing portal)
# (CI: each gst-launch hearth-record runs is written down, to say what it was asked when it fails)
mkdir -p /tmp/gstshim && printf '#!/bin/sh\nprintf "%%s\\n" "$*" >> %s/gst-launch-args.txt\nGST_DEBUG=GST_PIPELINE:5 GST_DEBUG_NO_COLOR=1 exec /usr/bin/gst-launch-1.0 "$@" 2>>%s/gst-debug.txt\n' "$OUT" "$OUT" > /tmp/gstshim/gst-launch-1.0 && chmod +x /tmp/gstshim/gst-launch-1.0
export PATH=/tmp/gstshim:$PATH
start_kwin 1 || exit 0
XDP=$(ls /usr/libexec/xdg-desktop-portal 2>/dev/null | head -1)
[ -n "$XDP" ] && { "$XDP" -r -v > "$OUT/xdg-desktop-portal.log" 2>&1 & sleep 3; }
/usr/bin/hearth-task-manager --page performance:cpu >"$OUT/tm2.log" 2>&1 &
sleep 3
# What Capture sees (its Record button is on when the helper names a recorder).
HEARTH_WM=kwin python3 "$SH/hearth-capture-helper" tools > "$OUT/tools.json" 2>&1
ok "Capture's tools" "$(cut -c1-400 "$OUT/tools.json")"
grep -q '"record": "hearth-record"' "$OUT/tools.json" || fail "Record" "Capture's helper doesn't name hearth-record as the recorder: $(cut -c1-300 "$OUT/tools.json")"
# H.264 for MP4 (the default) needs an encoder this CI may not have (Fedora has no x264): WebM (VP8) then.
if ! gst-inspect-1.0 openh264enc >/dev/null 2>&1 && ! gst-inspect-1.0 x264enc >/dev/null 2>&1; then
  python3 "$SH/hearth-record" set format webm >/dev/null 2>&1
  echo "::notice title=Record::no H.264 encoder in this CI, so the check records WebM (VP8)"
fi
python3 "$SH/hearth-record" set what screen >/dev/null 2>&1
# Record as Capture's button does: the capture dot, then the helper's record (hearth-record toggle).
/usr/bin/hearth-capture-app --indicator >"$OUT/dot2.log" 2>&1 &
dpid=$!
( HEARTH_WM=kwin timeout 220 python3 "$SH/hearth-capture-helper" record > "$OUT/record-start.json" 2>&1 ) &
rpid=$!
sleep 6
kshot "$OUT/kw-11-record-portal-asks.png"
kwins "$OUT/kw-windows-portal.txt"
# KDE's question: which screen to share. The one screen, then Share (Enter), as a person would.
for keys in "k 28 1|k 28 0" "k 15 1|k 15 0|k 57 1|k 57 0|k 28 1|k 28 0" "k 108 1|k 108 0|k 28 1|k 28 0"; do
  grep -q '"state": "recording"' "$XDG_RUNTIME_DIR/hearth-record/state.json" 2>/dev/null && break
  IFS='|' read -r -a ks <<<"$keys"; fake "${ks[@]}"; sleep 4
done
for i in $(seq 1 15); do grep -q '"state": "recording"' "$XDG_RUNTIME_DIR/hearth-record/state.json" 2>/dev/null && break; sleep 1; done
sleep 4
kwins "$OUT/kw-windows-recording.txt"
kshot "$OUT/kw-12-recording-dot.png"
if grep -q '"state": "recording"' "$XDG_RUNTIME_DIR/hearth-record/state.json" 2>/dev/null; then
  ok "Record" "recording through KDE's screen-sharing portal: $(cut -c1-300 "$XDG_RUNTIME_DIR/hearth-record/state.json")"
  sleep 5
  HEARTH_WM=kwin timeout 60 python3 "$SH/hearth-capture-helper" record > "$OUT/record-stop.json" 2>&1
  sleep 3
  f=$(ls -t "$HOME/Videos/Recordings"/* 2>/dev/null | head -1)
  sz=$(stat -c %s "$f" 2>/dev/null || echo 0)
  dur=$(timeout 30 gst-discoverer-1.0 "$f" 2>/dev/null | grep -i duration | head -1 | tr -s ' ')
  if [ "$sz" -gt 20000 ]; then
    ok "Record" "saved $(basename "$f"): $sz bytes, $dur"
    timeout 60 gst-launch-1.0 -q filesrc location="$f" ! decodebin ! videoconvert ! videoscale ! video/x-raw,width=1920,height=1080 ! pngenc snapshot=true ! filesink location="$OUT/kw-13-recording-frame.png" >/dev/null 2>&1
  else
    fail "Record" "the recording didn't make a file: '$f' $sz bytes; $(tr '\n' '|' <"$XDG_RUNTIME_DIR/hearth-record/last.log" 2>/dev/null | tail -c 600)"
  fi
else
  echo "::warning title=Record's pipeline::$(tail -c 1500 "$OUT/gst-launch-args.txt" 2>/dev/null)"
  echo "::warning title=Record's parse::$(grep -aiE 'error|syntax|warn|no element|no property|could not' "$OUT/gst-debug.txt" 2>/dev/null | tail -8 | cut -c1-300 | tr '\n' '|')"
  # Which part GStreamer here can't read (each piece on its own, parsed only).
  bis=""
  while IFS= read -r t; do
    timeout 8 /usr/bin/gst-launch-1.0 -q $t >/tmp/gp.txt 2>&1 </dev/null
    grep -q "syntax error" /tmp/gp.txt && bis="$bis SYNTAX[$t]" || bis="$bis ok[$(echo "$t" | cut -c1-40)]"
  done <<'GP'
fakesrc num-buffers=1 ! fakesink
matroskamux name=mux ! fakesink videotestsrc num-buffers=3 ! queue ! x264enc ! queue ! mux.
matroskamux name=mux ! filesink location="/tmp/a b.mkv" videotestsrc num-buffers=3 ! queue ! openh264enc ! h264parse ! queue ! mux.
videotestsrc num-buffers=3 ! video/x-raw,framerate=30/1 ! fakesink
videotestsrc num-buffers=3 ! videorate ! video/x-raw,framerate=30/1 ! fakesink
pulsesrc device="@DEFAULT_MONITOR@" num-buffers=3 ! fakesink
pulsesrc device=@DEFAULT_MONITOR@ num-buffers=3 ! fakesink
pipewiresrc fd=9 path=50 do-timestamp=true ! fakesink
pipewiresrc path=50 ! fakesink
audiotestsrc num-buffers=3 ! opusenc bitrate=160000 ! fakesink
GP
  echo "::warning title=Record's pieces::$bis"
  echo "::warning title=Record's log::$(tr '\n' '|' <"$XDG_RUNTIME_DIR/hearth-record/last.log" 2>/dev/null | tail -c 1500)"
  fail "Record" "KDE's screen-sharing portal didn't start a recording: $(cut -c1-300 "$OUT/record-start.json") · $(tr '\n' '|' <"$XDG_RUNTIME_DIR/hearth-record/last.log" 2>/dev/null | tail -c 500) · portal: $(grep -aiE 'screencast|error|warn' "$OUT/xdg-desktop-portal.log" | tail -4 | tr '\n' '|' | cut -c1-400)"
fi
kill $dpid $rpid 2>/dev/null
python3 "$SH/hearth-record" stop >/dev/null 2>&1

# What KWin said about Capture, its scripts and shortcuts.
grep -aiE "hearth|capture|kglobalaccel|screenshot" "$KWIN_LOG" | grep -v CIWIN | sort | uniq -c | sort -rn | head -40 > "$OUT/kwin-about-capture.txt"
pkill -f hearth-task-manager; pkill -f /tmp/kwin_wayland
ls "$OUT"/kw-*.png 2>/dev/null | wc -l > "$OUT/kw-count.txt"
exit 0
