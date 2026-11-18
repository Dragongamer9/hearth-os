#!/bin/bash
# Mantel at rest must stay light (1.14: the owner's laptop and ROG Ally nearly locked up with quickshell at 70-90% of
# a core; 1.13 sat near 13%). Measures Mantel's quickshell over SECS seconds (60) twice:
#   1. at rest, the desktop only;
#   2. after a search typed in the app menu and the menu closed with the words still in it (what spun in 1.14: the
#      closed menu kept working its search out again and again).
# Fails (::error, shots/FAILED) when it averages over MAX_PCT % of one core (15) or its memory keeps growing (more than
# MAX_GROW_MB, 60, from the first third of the window to the end). Reduce motion is on while measuring: a moving
# wallpaper drawn in software (CI has no GPU) isn't what this is about.
#   shell-idle-cpu.sh OUT LABEL [ipc-display]       (needs SH = the shell folder; WAYLAND_DISPLAY set; Mantel running)
set -u +e
OUT=$1; LABEL=$2; DISP=${3:-$WAYLAND_DISPLAY}
SECS=${IDLE_SECS:-60}; MAX_PCT=${IDLE_MAX_PCT:-15}; MAX_GROW_MB=${IDLE_MAX_GROW_MB:-60}
mkdir -p "$OUT"
say() { echo "::notice title=Mantel at rest ($LABEL)::$*"; }
bad() { echo "::error title=Mantel at rest ($LABEL)::$*"; mkdir -p shots; touch shots/FAILED; }
HZ=$(getconf CLK_TCK 2>/dev/null || echo 100)
ipc() { WAYLAND_DISPLAY=$DISP quickshell ipc -p "$SH" call "$@" 2>/dev/null | tail -n1; }
D="$HOME/.local/share/nimbus/desktop.json"
merge() { python3 "$SH/hearth_settings_io.py" merge "$D" "$1" >/dev/null 2>&1; }
pid() { pgrep -u "$(id -u)" -n -x quickshell; }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null; }
rss_mb() { awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$1/status" 2>/dev/null; }
LOGF="${XDG_RUNTIME_DIR:-/tmp}/nimbus-shell.log"

measure() {   # $1: a name. Prints "pct rss0 rss1 loops"
  local p t0 t1 r0 r1 rmid l0 l1 i
  p=$(pid); [ -n "$p" ] || { echo "none"; return; }
  l0=$(grep -c "Binding loop" "$LOGF" 2>/dev/null); l0=${l0:-0}
  t0=$(ticks "$p"); r0=$(rss_mb "$p"); rmid=$r0
  : > "$OUT/$1-samples.txt"
  for i in $(seq 1 "$SECS"); do
    sleep 1
    kill -0 "$p" 2>/dev/null || { echo "gone"; return; }
    echo "$i $(ticks "$p") $(rss_mb "$p")" >> "$OUT/$1-samples.txt"
    [ "$i" = $((SECS / 3)) ] && rmid=$(rss_mb "$p")
  done
  t1=$(ticks "$p"); r1=$(rss_mb "$p")
  l1=$(grep -c "Binding loop" "$LOGF" 2>/dev/null); l1=${l1:-0}
  # Where its time goes (a few stack samples of the busiest threads), for the log only.
  if command -v gdb >/dev/null 2>&1; then
    timeout 60 gdb -p "$p" -batch -ex "thread apply all bt 12" > "$OUT/$1-stacks.txt" 2>&1 || true
  fi
  echo "$(( (t1 - t0) * 100 * 100 / HZ / SECS ))" "$rmid" "$r1" "$((l1 - l0))"
}
check() {   # $1 name, $2 what
  local r pct rm r1 loops
  r=$(measure "$1")
  case "$r" in none|gone) bad "$2: Mantel isn't running ($r)"; return ;; esac
  read -r pct rm r1 loops <<<"$r"
  local shown; shown=$(printf '%d.%02d' $((pct / 100)) $((pct % 100)))
  local grow=$(( r1 - rm ))
  local top; top=$(grep -E '^#[0-9]+ ' "$OUT/$1-stacks.txt" 2>/dev/null | grep -vE 'poll|futex|pthread_cond|epoll|nanosleep|clock_nanosleep|__GI___ppoll|syscall' | awk '{for(i=1;i<=NF;i++) if($i=="in"){print $(i+1); break}}' | sort | uniq -c | sort -rn | head -6 | awk '{printf "%s×%s ", $1, $2}')
  local msg="$2: ${shown}% of a core over ${SECS}s, memory ${rm} → ${r1} MB, $loops binding-loop warnings · busiest: ${top:-idle}"
  if [ "$pct" -gt $((MAX_PCT * 100)) ]; then bad "$msg (more than ${MAX_PCT}%)"
  elif [ "$grow" -gt "$MAX_GROW_MB" ]; then bad "$msg (memory grew ${grow} MB)"
  elif [ "$loops" -gt 5 ]; then bad "$msg (a binding keeps looping)"
  else say "$msg"; fi
}

merge '{"reduceMotion": true}'
sleep 20                               # settle: the start's own work done
check rest "at rest"
# The 1.14 freeze: a search typed in the Classic menu, then the menu closed with the words still in it.
merge '{"appMenu": "classic"}'; sleep 3
ipc launcher open >/dev/null; sleep 2
ipc launcher query logs >/dev/null; sleep 3
ipc launcher close >/dev/null; sleep 5
check search-left "after a search left in the closed menu"
merge '{"appMenu": null, "reduceMotion": null}'
exit 0
