#!/bin/bash
# Mantel at rest must stay light (1.14: the owner's laptop and ROG Ally nearly locked up with quickshell at 70-90% of
# a core; 1.13 sat near 13% on the laptop). Measures Mantel's quickshell and the compositor (labwc):
#   rest          IDLE_SECS (600: ten minutes) at rest, the desktop only;
#   search-left   SEARCH_SECS (60) after a search typed in the Classic menu and the menu closed with the words still in
#                 it (what spun in 1.14: the closed menu kept working its search out again and again).
# Fails (::error, shots/FAILED) when quickshell averages over MAX_PCT % of one core (15), or more than BASE_PCT plus
# 5 points when a 1.13 figure is given (the same check of 1.13's Mantel, run just before), when the compositor averages
# over COMP_MAX_PCT (10), when quickshell's memory keeps growing (more than MAX_GROW_MB, 60, from the first third of
# the window to its end), or when a binding keeps looping. Reduce motion is on while measuring: a moving wallpaper
# drawn in software (CI has no GPU) isn't what this is about.
#   shell-idle-cpu.sh OUT LABEL [--baseline]   (needs SH = the shell folder, WAYLAND_DISPLAY, Mantel running)
#   --baseline: only the rest measure (BASELINE_SECS, 120), printed as "pct" for the next run's BASE_PCT; never fails.
set -u +e
OUT=$1; LABEL=$2; MODE=${3:-}
SECS=${IDLE_SECS:-600}; SSECS=${SEARCH_SECS:-60}; MAX_PCT=${IDLE_MAX_PCT:-15}; COMP_MAX=${COMP_MAX_PCT:-10}
MAX_GROW_MB=${IDLE_MAX_GROW_MB:-60}; BASE=${BASE_PCT:-}
mkdir -p "$OUT"
say() { echo "::notice title=Mantel at rest ($LABEL)::$*" >&2; }
bad() { echo "::error title=Mantel at rest ($LABEL)::$*" >&2; mkdir -p shots; touch shots/FAILED; }
HZ=$(getconf CLK_TCK 2>/dev/null || echo 100)
ipc() { quickshell ipc -p "$SH" call "$@" 2>/dev/null | tail -n1; }
D="$HOME/.local/share/nimbus/desktop.json"
merge() { python3 "$SH/hearth_settings_io.py" merge "$D" "$1" >/dev/null 2>&1; }
pid_of() { pgrep -u "$(id -u)" -n -x "$1"; }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null; }
rss_mb() { awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$1/status" 2>/dev/null; }
LOGF="${XDG_RUNTIME_DIR:-/tmp}/nimbus-shell.log"
pct() { printf '%d.%02d' $(($1 / 100)) $(($1 % 100)); }

measure() {   # $1 name, $2 seconds. Prints "qs_pct100 comp_pct100 rss_mid rss_end loops" (or "none"/"gone")
  local p c t0 t1 c0 c1 r0 rmid r1 l0 l1 i
  p=$(pid_of quickshell); [ -n "$p" ] || { echo none; return; }
  c=$(pid_of labwc)
  l0=$(grep -c "Binding loop" "$LOGF" 2>/dev/null); l0=${l0:-0}
  t0=$(ticks "$p"); c0=$( [ -n "$c" ] && ticks "$c" || echo 0); r0=$(rss_mb "$p"); rmid=$r0
  : > "$OUT/$1-samples.txt"
  for i in $(seq 1 "$2"); do
    sleep 1
    kill -0 "$p" 2>/dev/null || { echo gone; return; }
    [ $((i % 10)) = 0 ] && echo "$i $(ticks "$p") $(rss_mb "$p") $( [ -n "$c" ] && ticks "$c")" >> "$OUT/$1-samples.txt"
    [ "$i" = $(($2 / 3)) ] && rmid=$(rss_mb "$p")
  done
  t1=$(ticks "$p"); c1=$( [ -n "$c" ] && ticks "$c" || echo 0); r1=$(rss_mb "$p")
  l1=$(grep -c "Binding loop" "$LOGF" 2>/dev/null); l1=${l1:-0}
  if command -v gdb >/dev/null 2>&1; then
    timeout 60 gdb -p "$p" -batch -ex "thread apply all bt 12" > "$OUT/$1-stacks.txt" 2>&1 || true
  fi
  echo "$(( (t1 - t0) * 10000 / HZ / $2 )) $(( (c1 - c0) * 10000 / HZ / $2 )) $rmid $r1 $((l1 - l0))"
}
check() {   # $1 name, $2 seconds, $3 what
  local r q cp rm r1 loops grow top msg lim
  r=$(measure "$1" "$2")
  case "$r" in none|gone) bad "$3: Mantel isn't running ($r)"; return ;; esac
  read -r q cp rm r1 loops <<<"$r"
  grow=$(( r1 - rm ))
  top=$(grep -E '^#[0-9]+ ' "$OUT/$1-stacks.txt" 2>/dev/null | grep -vE 'poll|futex|pthread_cond|epoll|nanosleep|__GI___ppoll|syscall|read \(' | awk '{for(i=1;i<=NF;i++) if($i=="in"){print $(i+1); break}}' | sort | uniq -c | sort -rn | head -5 | awk '{printf "%s×%s ", $1, $2}')
  msg="$3: quickshell $(pct "$q")% of a core over $2 s (1.13: ${BASE:-not measured}%), labwc $(pct "$cp")%, memory $rm → $r1 MB, $loops binding-loop warnings · busiest: ${top:-idle}"
  lim=$((MAX_PCT * 100))
  if [ -n "$BASE" ]; then b=$(awk -v b="$BASE" 'BEGIN{printf "%d", (b + 5) * 100}'); [ "$b" -lt "$lim" ] && lim=$b; fi
  if [ "$q" -gt "$lim" ]; then bad "$msg (over $(pct "$lim")%)"
  elif [ "$cp" -gt $((COMP_MAX * 100)) ]; then bad "$msg (the compositor over ${COMP_MAX}%)"
  elif [ "$grow" -gt "$MAX_GROW_MB" ]; then bad "$msg (memory grew ${grow} MB)"
  elif [ "$loops" -gt 5 ]; then bad "$msg (a binding keeps looping)"
  else say "$msg"; fi
}

merge '{"reduceMotion": true}'
sleep 20                               # settle: the start's own work done
if [ "$MODE" = --baseline ]; then
  r=$(measure baseline "${BASELINE_SECS:-120}")
  case "$r" in none|gone) say "1.13's Mantel isn't running ($r): no baseline"; exit 0 ;; esac
  read -r q cp rm r1 loops <<<"$r"
  say "1.13 at rest: quickshell $(pct "$q")% of a core, labwc $(pct "$cp")%, memory $rm → $r1 MB"
  pct "$q"; echo
  exit 0
fi
check rest "$SECS" "at rest"
# The 1.14 freeze: a search typed in the Classic menu, then the menu closed with the words still in it.
merge '{"appMenu": "classic"}'; sleep 3
ipc launcher open >/dev/null; sleep 2
ipc launcher query logs >/dev/null; sleep 3
ipc launcher close >/dev/null; sleep 5
check search-left "$SSECS" "after a search left in the closed menu"
merge '{"appMenu": null, "reduceMotion": null}'
exit 0
