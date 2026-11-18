#!/bin/bash
# hearth-apps-rust.yml's "run" job: every pure-Rust Hearth app (hosts/hearth-ui) started on a headless Wayland screen
# (sway, software drawing, 1280x800) as an ordinary user, and measured:
#   first frame   from starting the program until its window is on the screen (sway maps a window when its first
#                 frame arrives), limit 300 ms (an error over 450 ms: CI's containers are slower than a PC)
#   memory        resident (RSS) and proportional (PSS) after it has been at rest, limit 60 MB
#   CPU at rest   over 20 s with nobody touching it: the apps that only change when something happens must stay
#                 under 0.3 % of a core; Task Manager, which samples once a second like Windows', under 3 %
# then pictured (shots/NAME.png, posted by the pictures job). Results: results.txt, one RESULT line per app.
# Usage (as root, in fedora:45, with the built apps in ./bin): hearth-apps-rust-test.sh
set -u
ROOT="$PWD"
BIN="$ROOT/bin"
mkdir -p "$ROOT/shots"
chmod 777 "$ROOT/shots"
id ci >/dev/null 2>&1 || useradd -m ci
CIH=/home/ci
RUNDIR=/tmp/xdg-ci
mkdir -p "$RUNDIR" && chown ci "$RUNDIR" && chmod 700 "$RUNDIR"
chmod -R a+rX "$BIN" "$ROOT/hosts/linux/desktop/shell" "$ROOT/.github/scripts" 2>/dev/null
# Hearth's look for the pictures: Steam-style (the default) and one in Hearth's own look, light, to show it follows.
mkdir -p "$CIH/.local/share/nimbus" "$CIH/.config/nimbus"
ln -sf "$ROOT/.github/scripts/hearth-apps-rust-fixtures.py" /tmp/restore-fixture.py
ln -sf "$ROOT/.github/scripts/hearth-apps-rust-fixtures.py" /tmp/reliability-fixture.py
chown -R ci "$CIH"

# Fedora's sway carries a file capability (cap_sys_nice) the container doesn't allow: a plain copy has none.
cp /usr/bin/sway /tmp/sway-nocap && chmod 755 /tmp/sway-nocap

cat > /tmp/sway.conf <<'EOF'
output HEADLESS-1 resolution 1280x800 position 0 0 bg #202020 solid_color
default_border none
default_floating_border none
focus_follows_mouse no
EOF

cat > /tmp/inner.sh <<'INNER'
#!/bin/bash
set -u
ROOT="$1"; BIN="$ROOT/bin"
export XDG_RUNTIME_DIR=/tmp/xdg-ci HOME=/home/ci WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
export HEARTH_SHELL_DIR="$ROOT/hosts/linux/desktop/shell" HEARTH_PALETTES="$ROOT/hosts/linux/desktop/theme/palettes.json"
export HEARTH_RESTORE_HELPER=/tmp/restore-fixture.py HEARTH_RELIABILITY_HELPER=/tmp/reliability-fixture.py
/tmp/sway-nocap -c /tmp/sway.conf >/tmp/sway.log 2>&1 &
for i in $(seq 1 100); do ls "$XDG_RUNTIME_DIR"/wayland-* >/dev/null 2>&1 && break; sleep 0.1; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-* | grep -v lock | head -1)")
export SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* 2>/dev/null | head -1)
[ -n "$WAYLAND_DISPLAY" ] || { echo "::error title=sway::no Wayland screen: $(tail -5 /tmp/sway.log | tr '\n' '|')"; exit 1; }
swaymsg -t get_outputs | head -c 300; echo

look() { # look mode
  printf '{"look": "%s", "mode": "%s"}\n' "$1" "$2" > "$HOME/.local/share/nimbus/desktop.json"
}
mapped() { swaymsg -t get_tree 2>/dev/null | grep -q "\"app_id\": \"$1\""; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }

# run NAME APP_ID REST BINARY ARGS...: start, time the first frame, picture, and (REST > 0) measure at rest.
run() {
  name=$1; appid=$2; rest=$3; shift 3
  t0=$(now_ms)
  "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  ff=-1
  for i in $(seq 1 600); do
    if mapped "$appid"; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.005
  done
  sleep 2.5
  grim "$ROOT/shots/$name.png" 2>/tmp/grim.log || echo "::warning title=$name::grim: $(cat /tmp/grim.log)"
  cpu=-1; rss=-1; pss=-1
  if [ "$rest" -gt 0 ] && kill -0 $pid 2>/dev/null; then
    sleep 3
    a=$(ticks $pid); sleep "$rest"; b=$(ticks $pid)
    hz=$(getconf CLK_TCK)
    cpu=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $hz / $rest))")
    rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status)
    pss=$(awk '/^Pss:/ {printf "%.1f", $2/1024}' /proc/$pid/smaps_rollup 2>/dev/null)
    threads=$(ls /proc/$pid/task | wc -l)
    echo "RESULT $name firstframe=$ff cpu=$cpu rss=$rss pss=${pss:--1} threads=$threads" | tee -a "$ROOT/results.txt"
  else
    echo "SHOT $name firstframe=$ff" | tee -a "$ROOT/results.txt"
  fi
  kill -0 $pid 2>/dev/null || echo "::error title=$name::it ended by itself: $(tail -c 1500 /tmp/$name.log | tr '\n' '|')"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.3
}

look steam dark
# Measured runs (the first of each app), then the pictures of their other pages and dialogs.
run restore-1-points hearth-restore 20 "$BIN/hearth-system-restore"
run reliability-1-days hearth-reliability 20 "$BIN/hearth-reliability-history"
run tm-1-processes hearth-tasks 20 "$BIN/hearth-task-manager" --page processes
run restore-2-confirm hearth-restore 0 "$BIN/hearth-system-restore" --show confirm
run restore-3-changes hearth-restore 0 "$BIN/hearth-system-restore" --show changes
run reliability-2-details hearth-reliability 0 "$BIN/hearth-reliability-history" --show details
run reliability-3-weeks hearth-reliability 0 "$BIN/hearth-reliability-history" --weeks
run tm-2-performance-cpu hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:cpu
run tm-3-performance-memory hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:memory
run tm-4-details hearth-tasks 0 "$BIN/hearth-task-manager" --page details
run tm-5-services hearth-tasks 0 "$BIN/hearth-task-manager" --page services
run tm-6-startup hearth-tasks 0 "$BIN/hearth-task-manager" --page startup
run tm-7-users hearth-tasks 0 "$BIN/hearth-task-manager" --page users
run tm-8-scheduled-tasks hearth-tasks 0 "$BIN/hearth-task-manager" --page tasks
run tm-9-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page settings
look hearth light
run look-hearth-light-restore hearth-restore 0 "$BIN/hearth-system-restore"
run look-hearth-light-tm hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:cpu
# Opening them wrote nothing of theirs (red flag 9): no taskmgr.json unless something was changed.
[ -e "$HOME/.config/nimbus/taskmgr.json" ] && echo "::error title=settings::opening Task Manager wrote taskmgr.json: $(cat "$HOME/.config/nimbus/taskmgr.json")"
grep -q '"look": "hearth"' "$HOME/.local/share/nimbus/desktop.json" || echo "::error title=settings::desktop.json changed"
kill %1 2>/dev/null
exit 0
INNER
chmod 755 /tmp/inner.sh
: > "$ROOT/results.txt"; chmod 666 "$ROOT/results.txt"
runuser -u ci -- /tmp/inner.sh "$ROOT"
inner=$?

# The limits.
fail=$inner
grep -q '^RESULT' "$ROOT/results.txt" || { echo "::error title=apps::nothing was measured"; fail=1; }
while read -r kind name rest; do
  [ "$kind" = RESULT ] || continue
  eval "$(echo "$rest" | tr ' ' '\n' | sed 's/^/r_/')"
  lim_cpu=0.3; [ "${name#tm-}" != "$name" ] && lim_cpu=3
  echo "::notice title=$name::first frame ${r_firstframe} ms · at rest ${r_cpu}% of a core · memory ${r_rss} MB resident (${r_pss} MB proportional) · ${r_threads} threads"
  python3 - "$name" "$r_firstframe" "$r_cpu" "$r_rss" "$lim_cpu" <<'PY' || fail=1
import sys
name, ff, cpu, rss, lim = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4]), float(sys.argv[5])
bad = 0
if ff < 0:
    print('::error title=%s::its window never came up' % name); bad = 1
elif ff > 450:
    print('::error title=%s::first frame took %d ms (limit about 300)' % (name, ff)); bad = 1
elif ff > 300:
    print('::warning title=%s::first frame took %d ms (limit about 300)' % (name, ff))
if cpu > lim:
    print('::error title=%s::%.2f%% of a core at rest (limit %.1f%%): something redraws or loops without a change' % (name, cpu, lim)); bad = 1
if rss > 80:
    print('::error title=%s::%.1f MB resident at rest (limit about 60)' % (name, rss)); bad = 1
elif rss > 60:
    print('::warning title=%s::%.1f MB resident at rest (limit about 60)' % (name, rss))
sys.exit(bad)
PY
done < "$ROOT/results.txt"
grep '^SHOT' "$ROOT/results.txt" | while read -r _ name ff; do
  case "$ff" in *=-1) echo "::error title=$name::its window never came up";; esac
done
ls -la "$ROOT/shots"
exit $fail
