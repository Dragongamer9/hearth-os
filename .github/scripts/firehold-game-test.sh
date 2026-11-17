#!/bin/bash
# Firehold's game rows (firehold.yml "game"; docs/FIREHOLD.md "Games: the Guide's rows"), nested (winit, on a virtual
# X screen) or headless (drawn into memory): the compositor alone at 1920 x 1080, a stand-in game
# (firehold-game-client.py: fullscreen, shared memory, as fast as its frame callbacks come) and the Guide's helper
# (hearth-firehold-game) choosing for it, exactly as the Guide does.
#   frame limit    the game's frame callbacks measured with no limit, at 30 and at 60
#   scaling        the configure size the game is told (Quality 1280 x 720, Performance 960 x 540) and the screen's
#                  pixels upscaled (grim: the game's 200-pixel square comes out 300 / 400 pixels wide)
#   settings       written only by the helper (atomic, .bak); `status` writes nothing; the compositor never writes
#   pictures       firehold-g<mode>-1-native, -2-quality, -3-performance (posted by firehold-shot.py)
#   firehold-game-test.sh nested|headless
set -u +e +o pipefail
MODE=${1:-nested}
M=${MODE:0:1}   # n / h, for the pictures' names
mkdir -p shots
SRC=$PWD
BIN=${FIREHOLD:-$SRC/firehold-comp}
chmod +x "$BIN" .github/scripts/firehold-game-client.py hosts/linux/desktop/shell/hearth-firehold-game
# GitHub keeps 10 notices a step: the findings are gathered and posted together at the end (report).
fail() { echo "$1: $2" >> "shots/errors-$MODE.txt"; touch "shots/FAILED-$MODE"; }
note() { echo "$1: $2" >> "shots/notes-$MODE.txt"; }
report() {
  python3 - "$MODE" <<'PY'
import sys
mode = sys.argv[1]
for kind, f, n in (('error', 'shots/errors-%s.txt' % mode, 4), ('notice', 'shots/notes-%s.txt' % mode, 5)):
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
        print('::%s title=game %s (%d)::%s' % (kind, mode, i, c))
PY
}
export XDG_RUNTIME_DIR=/tmp/xdg-game-$MODE HOME=/tmp/home-game-$MODE
rm -rf "$XDG_RUNTIME_DIR" "$HOME"
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.config" && chmod 700 "$XDG_RUNTIME_DIR"
export XDG_CONFIG_HOME="$HOME/.config" XDG_STATE_HOME="$HOME/.local/state"
export LIBGL_ALWAYS_SOFTWARE=1 FIREHOLD_LOG="info,smithay=warn"
unset WAYLAND_DISPLAY FIREHOLD_GAMES
LOG=shots/game-$MODE.log
GAMES="$HOME/.config/hearth/firehold-games.json"
STATUS="$XDG_RUNTIME_DIR/firehold.json"
STATS="$XDG_RUNTIME_DIR/game-stats.json"

if [ "$MODE" = nested ]; then
  Xvfb :77 -screen 0 1920x1080x24 >/dev/null 2>&1 &
  XPID=$!
  export DISPLAY=:77
  for i in $(seq 1 20); do xdpyinfo >/dev/null 2>&1 && break; [ -e /tmp/.X11-unix/X77 ] && break; sleep 0.5; done
  ARGS="--winit"
else
  unset DISPLAY
  ARGS="--headless"
fi
"$BIN" $ARGS --size 1920x1080 --socket wayland-game --no-xwayland >"$LOG" 2>&1 &
CPID=$!
for i in $(seq 1 60); do [ -S "$XDG_RUNTIME_DIR/firehold-game-wayland-game.sock" ] && break; sleep 0.5; done
export WAYLAND_DISPLAY=wayland-game
if [ ! -S "$XDG_RUNTIME_DIR/firehold-game-wayland-game.sock" ]; then
  fail "start" "no control socket: $(tail -c 1200 "$LOG" | tr '\n' '|')"
  report
  exit 1
fi
note "start" "the compositor is up with its control socket ($(grep -c . "$LOG") log lines)"

H() { python3 hosts/linux/desktop/shell/hearth-firehold-game "$@"; }
st() { H status | python3 -c 'import json,sys; d=json.load(sys.stdin); g=d.get("game") or {}; print(eval(sys.argv[1], {"d": d, "g": g}))' "$1" 2>/dev/null; }
stat_() { python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2], {"d": d}))' "$STATS" "$1" 2>/dev/null; }
wait_st() { for i in $(seq 1 $(( $1 * 2 ))); do [ "$(st "$2")" = True ] && return 0; sleep 0.5; done; return 1; }
wait_stat() { for i in $(seq 1 $(( $1 * 2 ))); do [ "$(stat_ "$2")" = True ] && return 0; sleep 0.5; done; return 1; }
fps_after() { sleep "$1"; stat_ 'd["fps"]'; }

python3 .github/scripts/firehold-game-client.py "$STATS" --app-id firehold-test-game --seconds 200 >shots/game-client-$MODE.log 2>&1 &
GPID=$!
wait_st 20 'g.get("appId") == "firehold-test-game"' || fail "game" "the stand-in game isn't the fullscreen game: $(H status | head -c 400) client: $(tail -c 400 shots/game-client-$MODE.log | tr '\n' '|')"
wait_stat 20 'd["frames"] > 30' || fail "game" "the stand-in game isn't drawing: $(head -c 400 "$STATS")"
[ "$(stat_ 'd["bufferSize"] == [1920, 1080]')" = True ] && note "game" "fullscreen at 1920 x 1080: $(st 'g')" \
  || fail "game" "not configured to the screen's size: $(head -c 300 "$STATS")"
sleep 1
base=$(fps_after 3)
note "frame limit" "no limit: the game's frame callbacks come at $base a second"
grim_shot() {   # $1 name: the screen as grim sees it (screencopy)
  timeout 20 grim "shots/$1.png" 2>"shots/$1.err" || fail "pictures" "grim failed for $1: $(head -c 300 "shots/$1.err")"
}
grim_shot "firehold-g$M-1-native"

# ---------------------------------------------------------------- frame limit
[ -e "$GAMES" ] && fail "settings" "something wrote the per-game file before anyone chose anything"
before_status=$(ls -la --time-style=full-iso "$HOME/.config" 2>/dev/null | md5sum)
H status >/dev/null; H status >/dev/null
[ "$(ls -la --time-style=full-iso "$HOME/.config" 2>/dev/null | md5sum)" = "$before_status" ] || fail "settings" "status (opening the Guide) wrote something"
out=$(H set frameLimit 30)
case "$out" in error:*) fail "frame limit" "set 30: $out";; esac
wait_st 10 'g.get("frameLimit") == 30 and g.get("frameLimitActive")' || fail "frame limit" "30 isn't active: $(H status | head -c 400)"
f30=$(fps_after 4)
if python3 -c "import sys; sys.exit(0 if 26.5 <= float('$f30') <= 33.5 else 1)"; then
  note "frame limit" "30: the game's frame callbacks come at $f30 a second (paced by the compositor: $(st 'g["pacedFps"]'))"
else
  fail "frame limit" "30: measured $f30 a second, wanted 27-33 (no limit: $base)"
fi
H set frameLimit 60 >/dev/null
wait_st 10 'g.get("frameLimit") == 60' || fail "frame limit" "60 wasn't taken: $(H status | head -c 300)"
f60=$(fps_after 4)
if python3 -c "import sys; sys.exit(0 if 53 <= float('$f60') <= 63.5 else 1)"; then
  note "frame limit" "60: measured $f60 a second (no limit: $base)"
elif python3 -c "import sys; sys.exit(0 if float('$base') < 66 and float('$f60') <= 63.5 else 1)"; then
  note "frame limit" "60: measured $f60 a second; this runner only manages $base unlimited, so 60 can't be shown higher"
else
  fail "frame limit" "60: measured $f60 a second, wanted 53-63 (no limit: $base)"
fi
H set frameLimit off >/dev/null
wait_st 10 'g.get("frameLimit") == "off" and not g.get("frameLimitActive")' || fail "frame limit" "off wasn't taken"
foff=$(fps_after 3)
note "frame limit" "off again: $foff a second"
[ "$(python3 -c "import json; print(json.load(open('$GAMES'))['games']['firehold-test-game'])")" = "{'frameLimit': 'off'}" ] \
  || fail "settings" "the file isn't what was chosen: $(cat "$GAMES")"
[ -e "$GAMES.bak" ] && note "settings" "saved atomically with its .bak: $(cat "$GAMES" | tr -d '\n ')" || fail "settings" "no .bak beside the file"

# ---------------------------------------------------------------- resolution scaling
square() {   # $1 picture: the white square's box on the screen, "x0 y0 x1 y1"
  python3 - "$1" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('L')
w, h = im.size
px = im.load()
xs, ys = [], []
for y in range(0, min(h, 700)):
    for x in range(0, min(w, 700)):
        if px[x, y] > 235:
            xs.append(x); ys.append(y)
print('%d %d %d %d' % (min(xs), min(ys), max(xs) + 1, max(ys) + 1) if xs else 'none')
PY
}
check_scale() {   # $1 value, $2 configure "WxH", $3 expected square size on the screen, $4 picture name
  H set scaling "$1" >/dev/null
  cw=${2%x*}; ch=${2#*x}
  wait_stat 15 "d['bufferSize'] == [$cw, $ch]" || fail "scaling" "$1: the game wasn't told $2: $(head -c 300 "$STATS") status: $(H status | head -c 300)"
  [ "$(st "g['configureSize'] == [$cw, $ch] and g['renderSize'] == [$cw, $ch] and g['scaling'] == '$1'")" = True ] \
    || fail "scaling" "$1: the status doesn't say $2: $(H status | head -c 400)"
  sleep 2
  grim_shot "$4"
  box=$(square "shots/$4.png")
  want=$3
  if python3 -c "
b='$box'.split(); w=$want; s=w/200.0
ok = len(b) == 4 and all(abs(int(v) - e) <= 4 for v, e in zip(b, (100*s, 100*s, 100*s + w, 100*s + w)))
raise SystemExit(0 if ok else 1)"; then
    note "scaling" "$1: the game draws at $2 and fills the screen: its 200-pixel square is $box on the 1920 x 1080 screen (upscaled $(python3 -c "print(round($want/200, 2))")x); $(st '"sharpening " + ("on" if g["sharpenActive"] else "off")'); the Guide's label: $(st 'g["scalingLabel"]')"
  else
    fail "scaling" "$1: the square should be ${want}px at $((want / 2)),$((want / 2)), it's at $box"
  fi
}
check_scale quality 1280x720 300 "firehold-g$M-2-quality"
check_scale performance 960x540 400 "firehold-g$M-3-performance"
grep -q "sharpening shader is ready" "$LOG" && note "scaling" "the sharpening shader compiled" \
  || { grep -q "no sharpening shader" "$LOG" && note "scaling" "no sharpening shader here: plain linear ($(grep "no sharpening shader" "$LOG" | head -1 | cut -c1-300))" || note "scaling" "the sharpening shader wasn't needed"; }
H set scaling off >/dev/null
wait_stat 15 'd["bufferSize"] == [1920, 1080]' && note "scaling" "off: back to 1920 x 1080" || fail "scaling" "off: not back at the screen's size: $(head -c 300 "$STATS")"

# ---------------------------------------------------------------- direct to the display and VRR
note "direct" "$(st '"scanout: %s (choice %s), VRR: %s on this %s backend" % (g["scanout"], g["scanoutChoice"], g["vrrScreen"], d["backend"])') (real screens only; the decision is unit-tested in game.rs)"
[ "$(st 'g["scanoutChoice"]')" = PreferGame ] || fail "direct" "a fullscreen game with Direct on should prefer the primary plane: $(st 'g')"
H set direct off >/dev/null
wait_st 10 'g["scanoutChoice"] == "Never"' && note "direct" "Direct off: always composited" || fail "direct" "Direct off wasn't taken: $(st 'g')"
H set direct on >/dev/null

# ---------------------------------------------------------------- the game leaves
kill "$GPID" 2>/dev/null; wait "$GPID" 2>/dev/null
wait_st 10 'd.get("game") is None' && note "game" "game closed: nothing applied any more" || fail "game" "still a game after it closed: $(H status | head -c 300)"
printf '{"games": ' > "$GAMES.cut"
cp "$GAMES" "$GAMES.good"; cp "$GAMES.cut" "$GAMES"
H status >/dev/null; sleep 3
grep -q "per-game file can't be read; keeping" "$LOG" && note "settings" "a damaged file is read as nothing new (the compositor keeps what it had and never writes it)" \
  || fail "settings" "the compositor didn't say it kept its choices when the file was damaged"
[ "$(cat "$GAMES")" = "$(cat "$GAMES.cut")" ] || fail "settings" "the compositor changed the damaged file"
cp "$GAMES.good" "$GAMES"
kill -TERM "$CPID"
for i in $(seq 1 20); do kill -0 "$CPID" 2>/dev/null || break; sleep 0.5; done
kill -0 "$CPID" 2>/dev/null && fail "end" "didn't end on SIGTERM"
grep -q "panicked" "$LOG" && fail "end" "a panic: $(grep -A3 panicked "$LOG" | head -c 600 | tr '\n' '|')"
[ -e "$XDG_RUNTIME_DIR/firehold-game-wayland-game.sock" ] && fail "end" "the control socket was left behind"
grep "Firehold game" "$LOG" | head -8 | while read -r l; do note "log" "$(echo "$l" | sed 's/\x1b\[[0-9;]*m//g' | cut -c1-260)"; done
report
[ -n "${XPID:-}" ] && kill "$XPID" 2>/dev/null
[ -e "shots/FAILED-$MODE" ] && exit 1
exit 0
