#!/bin/bash
# Firehold on labwc's approved screens (1.16; docs/mockups/firehold-plasma 01-12, checklist
# docs/mockups/checklists/firehold-labwc.md), pictured on labwc on a virtual screen with the mockups' sample games,
# devices and people (firehold-plasma-fixtures.py, the same stand-ins Firehold on Plasma's pictures use).
#   labwc-firehold-mockups.sh OUT
# Pictures OUT/lw-NN-*.png (named as the mockups), next to each one the mockup itself (OUT/mockup-NN-*.png).
# Fails (::error, exit 1) when the shared Couch doesn't load, or when its files print QML errors.
set -u
OUT=${1:?out folder}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SHARE=${NIMBUS_SHARE:-/usr/share/nimbus}
SH="$SHARE/desktop/shell"
FHP="$SHARE/desktop/firehold-plasma/hearth-firehold-plasma"
HERE=$(cd "$(dirname "$0")" && pwd)
MOCK=${MOCKUPS:-$HERE/../../docs/mockups/firehold-plasma}
fail=0
err() { echo "::error title=Firehold on labwc mockups::$*"; echo "FAIL: $*" >>"$OUT/mockups.md"; fail=1; }
note() { echo "::notice title=Firehold on labwc mockups::$*"; echo "$*" >>"$OUT/mockups.md"; }
: >"$OUT/mockups.md"

export XDG_RUNTIME_DIR=/tmp/xdg-mock HOME=/tmp/home-mock LC_ALL=C.UTF-8 NIMBUS_SHARE="$SHARE"
rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.cache/hearth" "$HOME/Desktop"; chmod 700 "$XDG_RUNTIME_DIR"
echo '{"screenOffMinutes": 0}' >"$HOME/.local/share/nimbus/desktop.json"
: >"$HOME/.local/share/nimbus/welcome-done"
# The mockups' stand-ins: the library (hearth-firehold's cache), each game's art, the home's cards, the bars' line.
FX=/tmp/fhp-fixtures-lw
python3 "$HERE/firehold-plasma-fixtures.py" "$HOME" "$FX" || err "the stand-ins couldn't be written"
export HEARTH_FIREHOLD_HOME_FIXTURE="$FX/home.json" HEARTH_FIREHOLD_SYS_FIXTURE="$FX/sys.json"
# Two screens, as the mockups have them: the TV (HEADLESS-1) and the monitor (HEADLESS-2), side by side.
export HEARTH_FIREHOLD_TV_SCREENS=HEADLESS-1 HEARTH_FIREHOLD_SCREENS="HEADLESS-1:tv:TCL 55″ TV:3840 × 2160 · 60 Hz · HDMI 2,HEADLESS-2::LG 27GP850:2560 × 1440 · 165 Hz · DisplayPort 1"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=2
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1
export HEARTH_WM=labwc HEARTH_FEDORA_LABWC=1 HEARTH_FIREHOLD_SESSION=1
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
eval "$(sed -n '/^# >>> data dirs/,/^# <<< data dirs/p' "${SESSION_FILE:-/usr/bin/nimbus-session}")" 2>/dev/null || true
python3 "$SH/hearth-browser" asked >/dev/null 2>&1 || true

sh -c 'HEARTH_DESKTOP_PID=$$; export HEARTH_DESKTOP_PID; exec labwc -C "$1"' sh "$SHARE/desktop/labwc" >"$OUT/labwc.log" 2>&1 &
LPID=$!
for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 --pos 0,0 >/dev/null 2>&1 || true
wlr-randr --output HEADLESS-2 --custom-mode 1920x1080 --pos 1920,0 >/dev/null 2>&1 || true
for i in $(seq 240); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 0.5; done
[ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] || { err "Mantel didn't come up: $(tail -c 1500 "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null | tr '\n' ' ')"; exit 1; }
SLOG="$XDG_RUNTIME_DIR/nimbus-shell.log"
python3 -c 'import gi, dbus' 2>/dev/null && { python3 "$FX/mpris.py" >"$OUT/mpris.log" 2>&1 & }
sleep 12

ipc() { timeout 20 quickshell ipc -p "$SH" call firehold "$@" >/dev/null 2>&1 || echo "ipc $* failed" >>"$OUT/ipc.txt"; sleep "${WAIT:-2}"; }
press() { for b in "$@"; do ipc press "$b"; done; }
shot() { grim -o "${3:-HEADLESS-1}" "$OUT/$1.png" 2>/dev/null || echo "no picture $1" >>"$OUT/ipc.txt"; cp "$MOCK/$2" "$OUT/mockup-$2" 2>/dev/null || true; }
fhp() { python3 "$FHP" "$@"; }

grep -q "the shared Couch didn't load" "$SLOG" && err "the approved Couch didn't load: $(grep -iE 'fhcouch|Couch.qml' "$SLOG" | head -6 | tr '\n' '|' | cut -c1-900)"
note "state: $(timeout 10 quickshell ipc -p "$SH" call firehold state 2>/dev/null | cut -c1-300)"

# ---- 01 Couch home, 02 further down (the low controller's notice), 03 game options from the Library
ipc couch; ipc ask cards; sleep 3
shot lw-01-couch-home 01-couch-home.png
press down down down down right
python3 "$HERE/firehold-plasma-fixtures.py" --low "$FX"; ipc ask cards; sleep 4
shot lw-02-couch-home-rows 02-couch-home-rows.png
press b; ipc tab library; sleep 2
press right x down; sleep 2
shot lw-03-couch-game-options 03-couch-game-options.png
press b; ipc tab home; sleep 2

# ---- 06 Away (this screen Away: hearth-firehold-plasma's live modes; never a setting), then back with the controller
cp "$HOME/.config/nimbus/firehold.json" /tmp/fh-before.json 2>/dev/null
fhp mode all away --why pick; sleep 5
shot lw-06-away 06-away.png
press a; sleep 4
fhp modes >"$OUT/modes-after.json" 2>&1
grep -q '"mode": "away"' "$OUT/modes-after.json" && err "the controller didn't bring the screen back from Away: $(head -c 300 "$OUT/modes-after.json")"
python3 -c 'import json,sys; sys.exit(json.load(open(sys.argv[1])) != json.load(open(sys.argv[2])))' "$HOME/.config/nimbus/firehold.json" /tmp/fh-before.json || err "switching to Away and back wrote Firehold's settings (red flag 9): $(head -c 300 "$HOME/.config/nimbus/firehold.json")"
shot lw-06b-back-from-away 01-couch-home.png

# ---- 04 Modes for each screen: the TV Couch, the monitor Desk (the mouse is there), the Modes card from its pill
fhp mode HEADLESS-2 desk --why mouse; sleep 5
note "modes: $(fhp modes 2>/dev/null | head -c 300)"
ipc modes true; sleep 3
shot lw-04-modes-per-screen 04-modes-per-screen.png HEADLESS-2
grim "$OUT/lw-04b-both-screens.png" 2>/dev/null
ipc modes false

# ---- QML errors from the shared Couch's files or the labwc host
errs=$(grep -E 'fhcouch/|FhpCouch|FhCouchHost|FhMedia|FhIcon' "$SLOG" | grep -iE 'error|TypeError|ReferenceError|is not defined|Cannot|Unable|failed' | sort | uniq -c | sort -rn | head -12)
[ -z "$errs" ] || err "QML messages from the approved Couch: $(echo "$errs" | cut -c1-200 | tr '\n' '|')"
grep -v -E 'Frame rendered|Frame prepared' "$SLOG" | tail -c 200000 >"$OUT/shell.log" 2>/dev/null
note "pictures: $(ls "$OUT"/lw-*.png 2>/dev/null | wc -l)"
kill "$LPID" 2>/dev/null; sleep 2; pkill -x quickshell 2>/dev/null
exit "$fail"
