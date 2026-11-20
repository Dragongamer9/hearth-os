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
# --report: only the limits and the measurements from results.txt (a workflow step of its own, so GitHub's ten
# notices a step are left for them).
REPORT_ONLY=0; [ "${1:-}" = --report ] && REPORT_ONLY=1
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
  # Actions › Hearth apps (pure Rust) › "only": just the runs whose names match (the others are skipped).
  if [ -n "${HEARTH_SHOTS_ONLY:-}" ] && ! printf '%s' "$name" | grep -Eq "$HEARTH_SHOTS_ONLY"; then return 0; fi
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

# bench NAME APP_ID RENDERER KIND BINARY ARGS...: scrolling a long page for 3 s (wl-scroll.py: KIND touchpad = smooth
# pixel steps 60 a second, wheel = notches), with the app drawing with RENDERER (cpu: tiny-skia, gpu: wgpu, here on
# Mesa's software Vulkan / GL since CI has no graphics card). The frames come from hearth-ui's frame counter
# (HEARTH_UI_FRAMES); CPU from /proc. One BENCH line each; a picture after the scroll shows where it got to.
bench() {
  name=$1; appid=$2; rend=$3; kind=$4; shift 4
  if [ -n "${HEARTH_SHOTS_ONLY:-}" ] && ! printf '%s' "$name" | grep -Eq "$HEARTH_SHOTS_ONLY"; then return 0; fi
  fr="/tmp/frames-$name"; rm -f "$fr"
  t0=$(now_ms)
  HEARTH_UI_FRAMES="$fr" HEARTH_RENDERER="$rend" "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  ff=-1
  for i in $(seq 1 1200); do
    if mapped "$appid"; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.005
  done
  sleep 2.5
  if ! kill -0 $pid 2>/dev/null; then
    echo "::error title=$name::it ended by itself: $(tail -c 1500 /tmp/$name.log | tr '\n' '|')"; return
  fi
  rss0=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status)
  f0=$(cat "$fr" 2>/dev/null || echo 0); a=$(ticks $pid); s0=$(now_ms)
  steps=$(python3 "$ROOT/.github/scripts/wl-scroll.py" 3 "$kind" 2>&1 | tail -1)
  s1=$(now_ms); sleep 0.3
  f1=$(cat "$fr" 2>/dev/null || echo 0); b=$(ticks $pid)
  rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status)
  grim "$ROOT/shots/$name.png" 2>/dev/null || true
  how=$(grep -m1 -o 'renderer: .*' "/tmp/$name.log" || grep -m1 -o 'drawing with the CPU.*' "/tmp/$name.log" || echo '?')
  hz=$(getconf CLK_TCK)
  echo "BENCH $name renderer=$rend kind=$kind firstframe=$ff frames=$((f1 - f0)) ms=$((s1 - s0)) ticks=$((b - a)) hz=$hz rss_before=$rss0 rss=$rss $steps how=$(echo "$how" | tr ' =' '_:' | cut -c1-120)" | tee -a "$ROOT/results.txt"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.3
}

look steam dark
# Measured runs (the first of each app), then the pictures of their other pages and dialogs.
run restore-1-points hearth-restore 20 "$BIN/hearth-system-restore"
run reliability-1-days hearth-reliability 20 "$BIN/hearth-reliability-history"
run tm-1-processes hearth-tasks 20 "$BIN/hearth-task-manager" --page processes
run settings-1-home hearth-settings 20 "$BIN/hearth-settings-app"
run settings-2-display hearth-settings 0 "$BIN/hearth-settings-app" --page display
run settings-3-wifi hearth-settings 0 "$BIN/hearth-settings-app" --page wifi
run settings-4-appearance hearth-settings 0 "$BIN/hearth-settings-app" --page appearance
run settings-5-updates hearth-settings 0 "$BIN/hearth-settings-app" --page updates
run settings-6-system hearth-settings 0 "$BIN/hearth-settings-app" --page system
run restore-2-confirm hearth-restore 0 "$BIN/hearth-system-restore" --show confirm
run restore-3-changes hearth-restore 0 "$BIN/hearth-system-restore" --show changes
run reliability-2-details hearth-reliability 0 "$BIN/hearth-reliability-history" --show details
run reliability-3-weeks hearth-reliability 0 "$BIN/hearth-reliability-history" --weeks
run tm-10-menu hearth-tasks 0 "$BIN/hearth-task-manager" --page processes --show menu
run tm-2-performance-cpu hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:cpu
run tm-3-performance-memory hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:memory
run tm-4-details hearth-tasks 0 "$BIN/hearth-task-manager" --page details
run tm-5-services hearth-tasks 0 "$BIN/hearth-task-manager" --page services
run tm-6-startup hearth-tasks 0 "$BIN/hearth-task-manager" --page startup
run tm-7-users hearth-tasks 0 "$BIN/hearth-task-manager" --page users
run tm-8-scheduled-tasks hearth-tasks 0 "$BIN/hearth-task-manager" --page tasks
run tm-9-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page settings
# A Global Theme that isn't Hearth's, picked in KDE (Breeze, light): hearth-plasma-looks writes theme.json from it and
# the apps draw with it (theme.rs from_plasma).
mkdir -p "$HOME/.config/hearth"
cat > "$HOME/.config/hearth/theme.json" <<'JSON'
{"look": "kde", "mode": "light", "source": "plasma", "scheme": "BreezeLight", "radius": 6, "corner": 6,
 "colors": {"bg": "#eff0f1", "bg2": "#e3e5e7", "panel": "#ffffff", "panel2": "#fcfcfc", "panel3": "#e3e5e7", "line": "#c9cbcd",
            "text": "#232629", "muted": "#707d8a", "danger": "#da4453", "ember": "#3daee9", "flame": "#f67400", "ink": "#ffffff"}}
JSON
export HEARTH_APPS_THEME="$HOME/.config/hearth/theme.json"
run settings-7-kde-breeze hearth-settings 0 "$BIN/hearth-settings-app"
unset HEARTH_APPS_THEME
rm -f "$HOME/.config/hearth/theme.json"
# Scrolling (the owner, Nov 19: "on all the rust apps the scrolling down is laggy and slow"): Settings' long Accessibility
# page (Display needs a screen CI hasn't got) and a folder of 3000 files in Files, on the CPU (tiny-skia, what CI and a PC without a graphics card use) and
# through wgpu (here Mesa's software drivers: a PC draws it on its graphics card). And the graphics card's start and
# memory (gpu-*: measured like the others, reported, no limits: CI's software Vulkan isn't a PC's).
mkdir -p "$HOME/bench-3000"
python3 -c 'import os,sys; [open(os.path.join(sys.argv[1], "file-%04d.txt" % i), "w").close() for i in range(3000)]' "$HOME/bench-3000"
bench scroll-cpu-settings-long hearth-settings cpu touchpad "$BIN/hearth-settings-app" --page accessibility
bench scroll-cpu-settings-long-wheel hearth-settings cpu wheel "$BIN/hearth-settings-app" --page accessibility
bench scroll-cpu-files-3000 hearth-files cpu touchpad "$BIN/hearth-files-app" "$HOME/bench-3000" --view details
bench scroll-gpu-settings-long hearth-settings gpu touchpad "$BIN/hearth-settings-app" --page accessibility
bench scroll-gpu-files-3000 hearth-files gpu touchpad "$BIN/hearth-files-app" "$HOME/bench-3000" --view details
HEARTH_RENDERER=gpu run gpu-settings-1-home hearth-settings 20 "$BIN/hearth-settings-app"
HEARTH_RENDERER=gpu run gpu-files-1-details hearth-files 20 "$BIN/hearth-files-app" "$HOME/bench-3000" --view details
# Each app's own pictures (one file per app or Settings category, so they're worked on apart): .d/NAME.sh uses run().
for f in "$ROOT"/.github/scripts/hearth-apps-rust-shots.d/*.sh; do [ -f "$f" ] && . "$f"; done
look steam dark
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
inner=0
if [ "$REPORT_ONLY" = 0 ]; then
  : > "$ROOT/results.txt"; chmod 666 "$ROOT/results.txt"
  runuser -u ci -- /tmp/inner.sh "$ROOT"
  inner=$?
  # The measurements are reported by the next step (--report).
  [ -n "${HEARTH_REPORT_LATER:-}" ] && { ls -la "$ROOT/shots" | tail -3; exit $inner; }
fi

# The limits.
fail=$inner
grep -q '^RESULT' "$ROOT/results.txt" || { echo "::error title=apps::nothing was measured"; fail=1; }
while read -r kind name rest; do
  [ "$kind" = RESULT ] || continue
  eval "$(echo "$rest" | tr ' ' '\n' | sed 's/^/r_/')"
  lim_cpu=0.3; [ "${name#tm-}" != "$name" ] && lim_cpu=3
  # Server mode's dashboard while its AIs are at work (they move only then, and only while it's on the screen and in
  # front), drawn here by the CPU 5 times a second: live, like Task Manager, with its own limit (a PC draws it with
  # its graphics card). Nothing at work: the ordinary 0.3 % (server-quiet-*).
  [ "${name#server-live}" != "$name" ] && lim_cpu=8
  echo "$name: first frame ${r_firstframe} ms, at rest ${r_cpu}% of a core, ${r_rss} MB resident (${r_pss} MB proportional), ${r_threads} threads" >> /tmp/at-rest.txt
  # Server mode's smooth mode through wgpu (the owner, Nov 20: "smooth like Mantel"): it moves at the screen's rate,
  # and CI's graphics card is Mesa's software Vulkan, so its CPU here is the graphics card's work done by the CPU.
  # Reported, no limit; a PC draws it on its graphics card. (Drawn with the CPU it falls back to the light mode,
  # measured as server-live-* with its own limit.)
  if [ "${name#gpu-server-live}" != "$name" ]; then continue; fi
  if [ "${name#gpu-}" != "$name" ]; then
    # Through wgpu on CI's software drivers: only CPU at rest has a limit (it must stay ~0 whatever draws).
    python3 -c 'import sys; c=float(sys.argv[2]); print("::error title=%s::%.2f%% of a core at rest with wgpu" % (sys.argv[1], c)) if c > 0.3 else None; sys.exit(1 if c > 0.3 else 0)' "$name" "$r_cpu" || fail=1
    continue
  fi
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
# The scroll measurements: frames a second while scrolling, and the CPU each frame took.
grep '^BENCH' "$ROOT/results.txt" | while read -r _ name rest; do
  python3 - "$name" $rest <<'PY'
import sys
name, kv = sys.argv[1], dict(a.split('=', 1) for a in sys.argv[2:] if '=' in a)
fr, ms, ticks, hz = int(kv.get('frames', 0)), max(1, int(kv.get('ms', 1))), int(kv.get('ticks', 0)), int(kv.get('hz', 100))
fps = fr * 1000.0 / ms
cpu_ms = ticks * 1000.0 / hz
per = cpu_ms / fr if fr else 0.0
with open('/tmp/scrolling.txt', 'a') as f:
    f.write('%s (%s, %s): %d frames in %.1f s = %.1f fps, %.1f ms CPU a frame (%.0f%% of a core), first frame %s ms, %s -> %s MB, %s, %s\n'
            % (name, kv.get('renderer'), kv.get('kind'), fr, ms / 1000.0, fps, per, 100.0 * cpu_ms / ms, kv.get('firstframe'), kv.get('rss_before'), kv.get('rss'), kv.get('steps', 'steps=?'), kv.get('how', '').replace('_', ' ')))
if fr == 0:
    print('::warning title=%s::no frames were counted while scrolling (the frame counter or the scroll didn\'t work)' % name)
PY
done
# One notice each, the lines joined (GitHub keeps ten notices a step).
for f in at-rest scrolling; do
  [ -s /tmp/$f.txt ] && echo "::notice title=$f::$(sed ':a;N;$!ba;s/\n/%0A/g' /tmp/$f.txt)"
done
grep '^SHOT' "$ROOT/results.txt" | while read -r _ name ff; do
  case "$ff" in *=-1) echo "::error title=$name::its window never came up";; esac
done
ls -la "$ROOT/shots"
exit $fail
