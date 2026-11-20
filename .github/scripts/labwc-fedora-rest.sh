#!/bin/bash
# Mantel (Hearth Classic) or Firehold on labwc on the Fedora edition (1.16; docs/LABWC-FEDORA.md), started the way
# nimbus-session's run_fedora_labwc starts it (labwc -C desktop/labwc, its autostart, Quickshell), on a virtual
# 1920x1080 screen with software drawing. Pictures first, then everything closed and measured at rest, then Hearth
# Files' opening time. The owner's red flags: never freeze or lag, nothing heavy at rest.
#   labwc-fedora-rest.sh classic|firehold OUT [SECONDS]
# Fails (exit 1, ::error annotations) when the desktop is over the same limits as KDE Plasma's check:
#   shell at rest       <= LIMIT_SHELL (0.25 % of a core) on average, and no minute over LIMIT_MINUTE (0.75 %)
#                          (measured Nov 20: 0.07 % Mantel, 0.09 % Firehold on labwc; 1.14 on the owner's devices: 13-27 %)
#   labwc at rest       <= LIMIT_COMP (0.50 %)
#   shell memory        <= LIMIT_MB (Mantel 340 MB; Firehold 420 MB: the Couch keeps the games' pictures; 1.14's
#                          shell held 900 MB-1.15 GB on the owner's Ally) and not growing (end - start <= LIMIT_GROW, 15 MB)
#   frames drawn        <= LIMIT_FRAMES (60) in the whole rest (the bar's clock: one a minute)
#   other processes     no other process of this user over 1 % at rest; no more processes at the end than at the start
#   Hearth Files        opens (first picture change) in <= LIMIT_FILES (1.5 s), warm or cold
# Writes OUT/*.png, OUT/rest.json, OUT/rest.md, OUT/execs.txt (what the shell started while at rest), logs.
set -u
KIND=${1:?classic or firehold}; OUT=${2:?out folder}; SECS=${3:-600}
LIMIT_SHELL=${LIMIT_SHELL:-25}; LIMIT_MINUTE=${LIMIT_MINUTE:-75}; LIMIT_COMP=${LIMIT_COMP:-50}   # hundredths of a %
LIMIT_MB=${LIMIT_MB:-$([ "$KIND" = firehold ] && echo 420 || echo 340)}; LIMIT_GROW=${LIMIT_GROW:-15}; LIMIT_FRAMES=${LIMIT_FRAMES:-60}; LIMIT_FILES=${LIMIT_FILES:-1.5}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SHARE=${NIMBUS_SHARE:-/usr/share/nimbus}
SH="$SHARE/desktop/shell"
fail=0
err() { echo "::error title=$KIND on labwc::$*"; echo "FAIL: $*" >>"$OUT/rest.md"; fail=1; }
note() { echo "::notice title=$KIND on labwc::$*"; echo "$*" >>"$OUT/rest.md"; }
: >"$OUT/rest.md"

export XDG_RUNTIME_DIR=/tmp/xdg-$KIND HOME=/tmp/home-$KIND LC_ALL=C.UTF-8 NIMBUS_SHARE="$SHARE"
rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.cache/hearth"; chmod 700 "$XDG_RUNTIME_DIR"
# The screen stays on during the measurement (Settings › Power's "turn the screen off after 10 minutes" would fall in
# the middle of it: the image run of Nov 20 saw one busy minute there, 0.8 % Mantel and 6.6 % Firehold), and the first-run Welcome window isn't open.
echo '{"screenOffMinutes": 0}' >"$HOME/.local/share/nimbus/desktop.json"
DESKJSON0=$(cat "$HOME/.local/share/nimbus/desktop.json")
mkdir -p "$HOME/.local/share/nimbus" && : >"$HOME/.local/share/nimbus/welcome-done"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""}]' >"$HOME/.cache/hearth/desktop-games.json"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1
# Every frame the shell draws is logged (one line each), so frames at rest can be counted.
export QT_LOGGING_RULES="qt.scenegraph.time.renderloop.debug=true"
export HEARTH_WM=labwc HEARTH_FEDORA_LABWC=1
if [ "$KIND" = firehold ]; then export HEARTH_FIREHOLD_SESSION=1; else unset HEARTH_FIREHOLD_SESSION; fi
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
# The XDG data dirs as nimbus-session sets them.
eval "$(sed -n '/^# >>> data dirs/,/^# <<< data dirs/p' "${SESSION_FILE:-/usr/bin/nimbus-session}")" 2>/dev/null || true

# A Desktop folder from the start, as every account has (the icons' pictures below put files in it).
mkdir -p "$HOME/Desktop"
# The one-time "Pick your browser" question answered, so it isn't over the pictures and Files' timing.
python3 "$SH/hearth-browser" asked >/dev/null 2>&1 || true
t0=$(date +%s.%N)
sh -c 'HEARTH_DESKTOP_PID=$$; export HEARTH_DESKTOP_PID; exec labwc -C "$1"' sh "$SHARE/desktop/labwc" >"$OUT/labwc.log" 2>&1 &
LPID=$!
for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
up=""
for i in $(seq 240); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && { up=$(python3 -c "print('%.1f' % ($(date +%s.%N) - $t0))"); break; }; sleep 0.5; done
if [ -z "$up" ]; then
  err "Mantel didn't come up within 120 s: $(tail -c 1500 "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null | tr '\n' ' ')"
  cp "$XDG_RUNTIME_DIR/nimbus-shell.log" "$OUT/" 2>/dev/null; kill "$LPID" 2>/dev/null; exit 1
fi
note "up in ${up} s"
sleep 15
ipc() { timeout 20 quickshell ipc -p "$SH" call "$@" >/dev/null 2>&1 || echo "ipc $* failed" >>"$OUT/ipc.txt"; sleep "${WAIT:-3}"; }
shot() { grim "$OUT/$1.png" 2>/dev/null || echo "no picture $1" >>"$OUT/ipc.txt"; }

# ---- Pictures: the desktop, its menus, Quick Access (quick settings), Settings' looks (the theme switcher)
if [ "$KIND" = classic ]; then
  shot 01-desktop
  ipc launcher toggle; shot 02-app-menu; ipc launcher toggle
  ipc launcher section games; shot 03-app-menu-games; ipc launcher close
  ipc menu open; shot 04-hearth-menu; ipc menu close
  ipc quick toggle; shot 05-quick-access; ipc quick toggle
  ipc notify toggle; shot 06-notifications; ipc notify toggle
  ipc settings page appearance; shot 07-settings-personalization
  ipc settings page looks; shot 08-settings-looks
  ipc settings page your-desktop; shot 09-settings-your-desktop
  ipc settings close
  ipc files home; shot 10-files; ipc files close
  ipc tasks open; shot 11-task-manager; ipc tasks close
  ipc searchbar open; shot 12-search-bar; ipc searchbar close
  ipc help toggle; shot 13-controls; ipc help toggle
else
  shot 01-couch-home
  ipc firehold tab library; shot 02-couch-library
  ipc firehold tab apps; shot 03-couch-apps
  ipc firehold tab store; shot 04-couch-store
  ipc firehold open switcher; shot 05-couch-everything-open; ipc firehold close
  ipc firehold open guide; shot 06-guide; ipc firehold close
  ipc firehold open settings; shot 07-couch-and-desk-settings; ipc firehold close
  ipc firehold desk; shot 08-desk
  ipc launcher toggle; shot 09-desk-app-menu; ipc launcher toggle
  ipc quick toggle; shot 10-desk-quick-access; ipc quick toggle
  ipc settings page looks; shot 11-settings-looks; ipc settings close
  ipc firehold couch; shot 12-back-to-couch
fi
# ---- Desktop icons keep their places (the owner's rule, Nov 19): one dragged to a place, icons off and on, the shell
# restarted (pkill quickshell, as on his dad's PC): every saved place the same, pictured before and after.
if [ "$KIND" = classic ]; then
  mkdir -p "$HOME/Desktop" && for n in Notes.txt Budget.ods Photos Trains.desktop; do
    case "$n" in Photos) mkdir -p "$HOME/Desktop/$n" ;; *) echo x >"$HOME/Desktop/$n" ;; esac; done
  sleep 6
  python3 "$SH/hearth-desk" place Notes.txt 9 3 18 8 >/dev/null 2>&1
  python3 "$SH/hearth-desk" place Photos 14 6 18 8 >/dev/null 2>&1
  sleep 4; shot 20-icons-placed
  python3 "$SH/hearth-desk" icons 18 8 --look >"$OUT/icons-placed.json" 2>&1
  before=$(python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1])).get("icons"), sort_keys=True))' "$HOME/.config/nimbus/desk.json")
  python3 "$SH/hearth-desk" set desktopIcons off >/dev/null 2>&1; sleep 4; shot 21-icons-off
  python3 "$SH/hearth-desk" set desktopIcons on >/dev/null 2>&1; sleep 4
  rm -f "$XDG_RUNTIME_DIR/hearth-shell-up"; pkill -x quickshell 2>/dev/null
  for i in $(seq 60); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 1; done
  sleep 10; shot 22-icons-after-restart
  after=$(python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1])).get("icons"), sort_keys=True))' "$HOME/.config/nimbus/desk.json")
  if [ "$before" = "$after" ] && echo "$after" | grep -q '"Notes.txt": \[9, 3\]'; then note "desktop icons kept their places through off/on and a shell restart: $after"
  else err "desktop icons moved by themselves: before $before, after $after"; fi
fi
# Settings: opening pages wrote nothing that wasn't there before the pictures? (red flag 9: checked on desktop.json)
[ "$(cat "$HOME/.local/share/nimbus/desktop.json")" = "$DESKJSON0" ] || err "a settings file changed by itself while the pictures were taken: $(head -c 300 "$HOME/.local/share/nimbus/desktop.json")"

# ---- At rest: everything closed, nothing touched
for t in settings tasks files launcher quick notify; do WAIT=0 ipc "$t" close; done
sleep 30
HZ=$(getconf CLK_TCK)
qs=$(pgrep -u "$(id -u)" -x quickshell | head -1)
[ -n "$qs" ] || qs=$(pgrep -u "$(id -u)" -f '^quickshell -p' | head -1)
[ -n "$qs" ] || { err "the shell isn't running before the rest"; exit 1; }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
rss() { awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$1/status" 2>/dev/null || echo 0; }
# (live ones only: in this container nothing reaps orphans, so finished ones stay as zombies; on a PC systemd reaps them)
allticks() { for p in /proc/[0-9]*; do [ -O "$p" ] || continue; grep -q '^State:[[:space:]]*Z' "$p/status" 2>/dev/null && continue; n=${p#/proc/}; printf '%s %s %s\n' "$n" "$(awk '{print $14 + $15}' "$p/stat" 2>/dev/null)" "$(tr '\0' ' ' <"$p/cmdline" 2>/dev/null | cut -c1-120)"; done; }
SLOG="$XDG_RUNTIME_DIR/nimbus-shell.log"
frames() { grep -c -E 'Frame rendered|Frame prepared' "$SLOG" 2>/dev/null || echo 0; }
allticks >"$OUT/procs-start.txt"; n0=$(wc -l <"$OUT/procs-start.txt")
s0=$(ticks "$qs"); c0=$(ticks "$LPID"); m0=$(rss "$qs"); f0=$(frames)
mins=""; last=$s0; worst=0
for i in $(seq $((SECS / 60))); do
  sleep 60
  s=$(ticks "$qs"); d=$(( (s - last) * 10000 / HZ / 60 )); last=$s
  mins="$mins $d"; [ "$d" -gt "$worst" ] && worst=$d
  echo "minute $i: shell $d, memory $(rss "$qs") MB, frames $(( $(frames) - f0 ))" >>"$OUT/rest-minutes.txt"
done
kill -0 "$qs" 2>/dev/null || err "the shell ended during the rest (crashed or restarted)"
s1=$(ticks "$qs"); c1=$(ticks "$LPID"); m1=$(rss "$qs"); f1=$(frames)
allticks >"$OUT/procs-end.txt"; n1=$(wc -l <"$OUT/procs-end.txt")
sp=$(( (s1 - s0) * 10000 / HZ / SECS )); cp=$(( (c1 - c0) * 10000 / HZ / SECS )); fr=$(( f1 - f0 ))
pct() { printf '%d.%02d %%' $(($1 / 100)) $(($1 % 100)); }
# Other processes' processor time over the rest (by pid; new ones counted from 0).
python3 - "$OUT/procs-start.txt" "$OUT/procs-end.txt" "$SECS" "$HZ" "$qs" "$LPID" >"$OUT/procs-busy.txt" <<'PY'
import sys
a = {l.split(' ', 2)[0]: l.rstrip('\n').split(' ', 2) for l in open(sys.argv[1])}
secs, hz, skip = int(sys.argv[3]), int(sys.argv[4]), {sys.argv[5], sys.argv[6]}
for l in open(sys.argv[2]):
    p = l.rstrip('\n').split(' ', 2)
    if len(p) < 3 or p[0] in skip or not p[1].isdigit():
        continue
    before = int(a[p[0]][1]) if p[0] in a and a[p[0]][1].isdigit() else 0
    pc = (int(p[1]) - before) * 100.0 / hz / secs
    if pc >= 0.2:
        print('%.2f %s %s' % (pc, p[0], p[2]))
PY
note "at rest over ${SECS} s: shell $(pct "$sp") of a core (worst minute $(pct "$worst"); minutes:$mins), labwc $(pct "$cp"), shell memory ${m0} → ${m1} MB, ${fr} frames drawn, processes ${n0} → ${n1}"
[ "$sp" -le "$LIMIT_SHELL" ] || err "the shell used $(pct "$sp") of a core at rest (limit $(pct "$LIMIT_SHELL"))"
[ "$worst" -le "$LIMIT_MINUTE" ] || err "the shell's busiest minute at rest was $(pct "$worst") (limit $(pct "$LIMIT_MINUTE"))"
[ "$cp" -le "$LIMIT_COMP" ] || err "labwc used $(pct "$cp") of a core at rest (limit $(pct "$LIMIT_COMP"))"
[ "$m1" -le "$LIMIT_MB" ] || err "the shell holds ${m1} MB at rest (limit ${LIMIT_MB} MB)"
[ $((m1 - m0)) -le "$LIMIT_GROW" ] || err "the shell's memory grew ${m0} → ${m1} MB in ${SECS} s at rest"
[ "$fr" -le "$LIMIT_FRAMES" ] || err "the shell drew ${fr} frames in ${SECS} s at rest (limit ${LIMIT_FRAMES}): something keeps animating"
[ "$n1" -le $((n0 + 1)) ] || err "more processes at the end of the rest (${n0} → ${n1}): $(comm -13 <(cut -d' ' -f3- "$OUT/procs-start.txt" | sort) <(cut -d' ' -f3- "$OUT/procs-end.txt" | sort) | head -5 | tr '\n' '|')"
busy=$(awk '$1 >= 1.0' "$OUT/procs-busy.txt")
[ -z "$busy" ] || err "busy at rest: $(echo "$busy" | head -5 | tr '\n' '|')"

# ---- Hearth Files: cold (first time) and warm, from the command to the first change on screen (grim, unscaled ppm)
if [ "$KIND" = classic ]; then
  # (after 10 minutes nobody's there, Mantel turned the screen off, as Settings › Power says: back on first)
  wlopm --on '*' >/dev/null 2>&1 || true; sleep 2
  python3 - "$SH" "$LIMIT_FILES" >"$OUT/files-check.txt" 2>&1 <<'PY'
import hashlib, subprocess, sys, time
sh, lim = sys.argv[1], float(sys.argv[2])
def frame():
    try:
        r = subprocess.run(['grim', '-t', 'ppm', '-'], capture_output=True, timeout=20)
    except subprocess.TimeoutExpired:
        print('grim took over 20 s'); return ''
    if r.returncode != 0:
        print('grim failed: %s' % r.stderr.decode()[:200]); return ''
    return hashlib.sha1(r.stdout).hexdigest()
def ipc(*a):
    return subprocess.Popen(['quickshell', 'ipc', '-p', sh, 'call'] + list(a), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for run in ('cold', 'warm', 'warm2'):
    base = frame(); t0 = time.monotonic(); ipc('files', 'home'); first = None
    while time.monotonic() - t0 < 10:
        f = frame()
        if f and f != base:
            first = time.monotonic() - t0; break
        time.sleep(0.02)
    print('%s Hearth Files (%s) %s' % ('OK' if first is not None and first <= lim else 'SLOW', run, 'never' if first is None else '%.2f s' % first))
    ipc('files', 'close').wait(); time.sleep(3)
PY
  note "Hearth Files: $(grep -E '^(OK|SLOW)' "$OUT/files-check.txt" | tr '\n' ' ')"
  ! grep -q '^SLOW' "$OUT/files-check.txt" || err "Hearth Files opened slowly: $(grep -E '^SLOW|grim' "$OUT/files-check.txt" | tr '\n' ' ') (limit ${LIMIT_FILES} s)"
fi

# ---- What the shell starts while at rest (2 minutes, traced after the measurement so it doesn't count)
if command -v strace >/dev/null 2>&1; then
  timeout 120 strace -f -qq --seccomp-bpf -e trace=execve -e signal=none -o "$OUT/execs.raw" -p "$qs" >/dev/null 2>&1
  grep -o 'execve("[^"]*", \[[^]]*' "$OUT/execs.raw" 2>/dev/null | sed 's/^execve("//; s/", \[/ /' | cut -c1-200 | sort | uniq -c | sort -rn >"$OUT/execs.txt"
  ne=$(awk '{s += $1} END {print s + 0}' "$OUT/execs.txt")
  note "programs the shell started in 2 minutes at rest: ${ne}$( [ "$ne" -gt 0 ] && echo " ($(head -3 "$OUT/execs.txt" | sed 's/^ *//' | cut -c1-90 | tr '\n' '|'))")"
  [ "$ne" -le 2 ] || err "the shell keeps starting programs at rest (${ne} in 2 minutes): $(head -4 "$OUT/execs.txt" | sed 's/^ *//' | cut -c1-120 | tr '\n' '|')"
fi
rm -f "$OUT/execs.raw"
# Diagnostics (never fail the check): which system calls the shell makes at rest (each wake-up), and where its time goes.
if command -v strace >/dev/null 2>&1; then
  timeout 60 strace -c -f -qq -p "$qs" -o "$OUT/syscalls.txt" >/dev/null 2>&1 || true
  # ...and which files it looks at (by path, 30 s), the threads that wake up most, and what's left over (zombies)
  timeout 30 strace -f -qq -e trace=newfstatat,statx,inotify_add_watch,openat,access -e signal=none -o /tmp/st-$KIND.raw -p "$qs" >/dev/null 2>&1 || true
  grep -o '"[^"]*"' /tmp/st-$KIND.raw 2>/dev/null | sort | uniq -c | sort -rn | head -40 >"$OUT/files-looked-at.txt"
  timeout 20 strace -f -qq -e trace=ppoll,poll,epoll_wait -e signal=none -o /tmp/pp-$KIND.raw -p "$qs" >/dev/null 2>&1 || true
  awk '{print $1}' /tmp/pp-$KIND.raw 2>/dev/null | sort | uniq -c | sort -rn | head -12 | while read -r n tid; do
    printf '%s wakeups in 20 s: thread %s %s\n' "$n" "$tid" "$(cat /proc/$qs/task/$tid/comm 2>/dev/null)"; done >"$OUT/wakeups.txt"
  rm -f /tmp/st-$KIND.raw /tmp/pp-$KIND.raw
fi
kill -CONT "$qs" 2>/dev/null
ps -eo pid,ppid,stat,etimes,comm,args 2>/dev/null | awk '$3 ~ /Z/' >"$OUT/zombies.txt"
if command -v perf >/dev/null 2>&1; then
  timeout 90 perf record -q -F 499 -g -p "$qs" -o /tmp/perf-$KIND.data -- sleep 60 >/dev/null 2>&1 || true
  perf report -i /tmp/perf-$KIND.data --stdio --no-children --percent-limit 1.5 --sort dso,sym 2>/dev/null | grep -v '^$' | head -150 >"$OUT/perf.txt" || true
  rm -f /tmp/perf-$KIND.data
fi

python3 - "$OUT/rest.json" <<PY
import json
json.dump({"kind": "$KIND", "up_s": "$up", "shell_pct": $sp / 100, "worst_minute_pct": $worst / 100, "labwc_pct": $cp / 100,
           "shell_mb_start": $m0, "shell_mb_end": $m1, "frames": $fr, "procs": [$n0, $n1], "secs": $SECS}, open("$OUT/rest.json", "w"))
PY
grep -v -E 'Frame rendered|Frame prepared' "$SLOG" | tail -c 200000 >"$OUT/shell.log" 2>/dev/null
grep -E 'Frame rendered|Frame prepared' "$SLOG" | tail -n 200 >"$OUT/frames-tail.log" 2>/dev/null
kill "$LPID" 2>/dev/null; sleep 2; pkill -u "$(id -u)" -x quickshell 2>/dev/null
exit "$fail"
