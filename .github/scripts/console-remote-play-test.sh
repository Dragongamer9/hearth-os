#!/bin/bash
# console-remote-play.yml's "run" job: Hearth Console Remote Play on a headless Wayland screen (sway), as an ordinary
# user, against CI's stand-in for Microsoft's services (HEARTH_CRP_FIXTURE: hosts/console-remote-play/tests/fixtures).
# Every page of the approved mockups is pictured (shots/crp-*.png): the desktop pages at 1920 x 1080, the Ally's Game
# Mode pages at 1280 x 800. Measured: CPU at rest (limit 0.3 % of a core on pages that only change when something
# happens), memory. Checked: opening pages writes no settings file; no token from the stand-in is in any log or any
# file in the home folder (only in CI's stand-in keyring); the sign-in window catches the redirect.
set -u
ROOT="$PWD"
BIN="$ROOT/bin"
FIX="$ROOT/hosts/console-remote-play/tests/fixtures"
mkdir -p "$ROOT/shots" && chmod 777 "$ROOT/shots"
: > "$ROOT/results.txt"; chmod 666 "$ROOT/results.txt"
id ci >/dev/null 2>&1 || useradd -m ci
CIH=/home/ci
RUNDIR=/tmp/xdg-ci
mkdir -p "$RUNDIR" && chown ci "$RUNDIR" && chmod 700 "$RUNDIR"
chmod -R a+rX "$BIN" "$ROOT/hosts/console-remote-play" "$ROOT/ui/osk" 2>/dev/null
cp /usr/bin/sway /tmp/sway-nocap && chmod 755 /tmp/sway-nocap

# CI's stand-in keyring (secret-tool's store / lookup / clear): stand-in tokens only, outside the home folder.
cat > /tmp/fake-secret-tool <<'EOF'
#!/bin/bash
f=/tmp/ci-keyring/item
mkdir -p /tmp/ci-keyring
case "$1" in
  store) cat > "$f" ;;
  lookup) [ -s "$f" ] && cat "$f" || exit 1 ;;
  clear) rm -f "$f" ;;
esac
EOF
chmod 755 /tmp/fake-secret-tool
mkdir -p /tmp/ci-keyring && chown ci /tmp/ci-keyring

cat > /tmp/sway.conf <<'EOF'
output HEADLESS-1 resolution 1920x1080 position 0 0 bg #101010 solid_color
default_border none
default_floating_border none
focus_follows_mouse no
for_window [app_id="hearth-console-remote-play-signin"] floating enable, move position center
EOF

# The Store's own files, as an installed app that's up to date (Settings › Hearth Store: Up to date).
mkdir -p "$CIH/.local/share/hearth/store/apps/console-remote-play" "$CIH/.cache/hearth/hearth-store"
echo '{"version": "stand-in"}' > "$CIH/.local/share/hearth/store/apps/console-remote-play/store.json"
echo '{"apps": {"console-remote-play": {"version": "stand-in"}}}' > "$CIH/.cache/hearth/hearth-store/releases.json"
chown -R ci "$CIH"

cat > /tmp/inner.sh <<'INNER'
#!/bin/bash
set -u
ROOT="$1"; BIN="$ROOT/bin"; FIX="$ROOT/hosts/console-remote-play/tests/fixtures"
export XDG_RUNTIME_DIR=/tmp/xdg-ci HOME=/home/ci WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
export HEARTH_CRP_FIXTURE="$FIX" HEARTH_SECRET_TOOL=/tmp/fake-secret-tool HEARTH_CRP_SIGNIN="$BIN/hearth-console-remote-play-signin"
export HEARTH_OSK_DIR="$ROOT/ui/osk" GDK_BACKEND=wayland WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
export HEARTH_RENDERER=cpu XDG_PICTURES_DIR=/home/ci/Pictures
/tmp/sway-nocap -c /tmp/sway.conf >/tmp/sway.log 2>&1 &
for i in $(seq 1 100); do ls "$XDG_RUNTIME_DIR"/wayland-* >/dev/null 2>&1 && break; sleep 0.1; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-* | grep -v lock | head -1)")
export SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* 2>/dev/null | head -1)
[ -n "$WAYLAND_DISPLAY" ] || { echo "::error title=sway::no Wayland screen: $(tail -5 /tmp/sway.log | tr '\n' '|')"; exit 1; }

mapped() { swaymsg -t get_tree 2>/dev/null | grep -q "\"app_id\": \"$1\""; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
month=$(date +%Y-%m)
seed() { mkdir -p "$HOME/.local/share/hearth/console-remote-play"; printf '{"period": "%s", "bytes": %s}\n' "$month" "$1" > "$HOME/.local/share/hearth/console-remote-play/data.json"; }

# run NAME REST WAIT ARGS...: start, wait WAIT s, picture, and (REST > 0) measure at rest.
run() {
  name=$1; rest=$2; wait=$3; shift 3
  if [ -n "${HEARTH_SHOTS_ONLY:-}" ] && ! printf '%s' "$name" | grep -Eq "$HEARTH_SHOTS_ONLY"; then return 0; fi
  t0=$(now_ms)
  "$BIN/hearth-console-remote-play" "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  ff=-1
  for i in $(seq 1 600); do
    if mapped hearth-console-remote-play; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.01
  done
  sleep "$wait"
  grim "$ROOT/shots/$name.png" 2>/tmp/grim.log || echo "::warning title=$name::grim: $(cat /tmp/grim.log)"
  if [ "$rest" -gt 0 ] && kill -0 $pid 2>/dev/null; then
    sleep 2
    a=$(ticks $pid); sleep "$rest"; b=$(ticks $pid)
    hz=$(getconf CLK_TCK)
    cpu=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $hz / $rest))")
    rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status)
    echo "RESULT $name firstframe=$ff cpu=$cpu rss=$rss" | tee -a "$ROOT/results.txt"
  else
    echo "SHOT $name firstframe=$ff" | tee -a "$ROOT/results.txt"
  fi
  kill -0 $pid 2>/dev/null || echo "::error title=$name::it ended by itself: $(tail -c 1500 /tmp/$name.log | tr '\n' '|')"
  pkill -f hearth-console-remote-play-signin 2>/dev/null
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.4
}

# ---------------- the desktop and laptop pages (1920 x 1080)
seed 14600000000
run crp-1a-welcome 20 2.5 --page welcome
run crp-1b-sign-in-window 0 6 --page signin
# The whole sign-in: the stand-in page "signs in" and goes to the redirect address; the window catches it and
# closes, the app finishes signing in (stand-in tokens) and keeps it in the (stand-in) keyring.
HEARTH_CRP_AUTO_REDIRECT=1 run crp-1c-signed-in 0 9 --page signin
run crp-2-my-consoles 20 3 --page consoles
run crp-3-connecting 0 4 --page connecting
HEARTH_CRP_NET="wifi|5|Home" run crp-4-streaming-bar 0 4 --page stream --show bar,pill,disconnect-tip
seed 18100000000
run crp-4b-streaming-data-warning 0 5 --page stream --show warning,shot
seed 14600000000
HEARTH_CRP_NET="wifi|2.4|OfficeNet" run crp-5-connection-trouble 0 4 --page trouble
run crp-6-settings 20 3 --page settings
run crp-8-cloud-optional 20 4 --page cloud

# ---------------- Game Mode on the Ally (1280 x 800)
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null
sleep 0.5
run crp-g1-ally-sign-in 0 6 --game-mode --page signin
run crp-g2-ally-consoles 20 3 --game-mode --page consoles
run crp-g3-ally-connecting 0 4 --game-mode --page connecting
HEARTH_CRP_NET="wifi|2.4|OfficeNet" run crp-g5-ally-trouble 0 4 --game-mode --page trouble
run crp-g6-ally-settings 20 3 --game-mode --page settings
run crp-g7a-ally-streaming-overlay 0 4 --game-mode --page stream --show pill
run crp-g7b-ally-quick-menu 0 4 --game-mode --page stream --show menu
run crp-g8-ally-cloud-optional 0 4 --game-mode --page cloud

# ---------------- the red flags
# Opening every page wrote no setting.
[ -e "$HOME/.config/nimbus/console-remote-play.json" ] && echo "::error title=settings::opening pages wrote console-remote-play.json: $(cat "$HOME/.config/nimbus/console-remote-play.json")"
# No token in a log or in a file in the home folder (the stand-in's tokens are only in CI's stand-in keyring).
leak=$(grep -rl -e 'stand-in-refresh' -e 'stand-in-access' -e 'stand-in-xsts' -e 'stand-in-home' /tmp/*.log "$HOME" 2>/dev/null | head -5 | tr '\n' ' ')
[ -n "$leak" ] && echo "::error title=tokens::a sign-in token was written to: $leak" || echo "::notice title=tokens::no sign-in token in any log or any file in the home folder"
grep -q 'stand-in-refresh' /tmp/ci-keyring/item 2>/dev/null && echo "::notice title=keyring::the sign-in was kept in the (stand-in) keyring only" || echo "::warning title=keyring::the stand-in keyring has no sign-in (did 1b sign in?)"
ls "$HOME/Pictures/Screenshots" 2>/dev/null | head -2 | while read -r f; do echo "::notice title=screenshot::saved $f"; done
for f in /tmp/crp-*.log; do grep -h -m3 -E 'panicked|error' "$f" 2>/dev/null | while read -r l; do echo "::warning title=$(basename "$f" .log)::$l"; done; done
kill %1 2>/dev/null
exit 0
INNER
chmod 755 /tmp/inner.sh
runuser -u ci -- /tmp/inner.sh "$ROOT"
inner=$?

# The limits.
fail=$inner
while read -r kind name rest; do
  [ "$kind" = RESULT ] || continue
  eval "$(echo "$rest" | tr ' ' '\n' | sed 's/^/r_/')"
  echo "::notice title=$name::first frame ${r_firstframe} ms, at rest ${r_cpu} % of a core, ${r_rss} MB"
  python3 -c 'import sys; c=float(sys.argv[2]); print("::error title=%s::%.2f %% of a core at rest (limit 0.3 %%)" % (sys.argv[1], c)) if c > 0.3 else None; sys.exit(1 if c > 0.3 else 0)' "$name" "$r_cpu" || fail=1
done < "$ROOT/results.txt"
ls -la "$ROOT/shots" | tail -25
exit $fail
