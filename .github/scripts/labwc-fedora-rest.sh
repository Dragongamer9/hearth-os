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
# Without the Hearth app (this container), Firehold's library comes from its cache: 60 games with pictures the sizes
# the stores use (cover 600x900, wide 920x430, hero 3840x1240), as the image's library has, so memory and the busy
# moments are measured with pictures to load (the image run of Nov 20: 466 MB with them, 251 MB without).
if ! command -v nimbus >/dev/null 2>&1 && python3 -c 'import PIL' 2>/dev/null; then
  python3 - "$HOME/.cache/hearth" <<'PY'
import json, os, sys, random
from PIL import Image, ImageDraw
d = sys.argv[1]; art = os.path.join(d, 'test-art'); os.makedirs(art, exist_ok=True)
random.seed(7); games = []
def pic(name, w, h, kind):
    p = os.path.join(art, name); c1 = tuple(random.randrange(40, 220) for _ in range(3)); c2 = tuple(random.randrange(10, 120) for _ in range(3))
    im = Image.new('RGB', (w, h), c1); dr = ImageDraw.Draw(im)
    for i in range(0, w, max(8, w // 40)): dr.line([(i, 0), (i + h // 2, h)], fill=c2, width=max(2, w // 120))
    im.save(p, 'PNG' if kind == 'png' else 'JPEG', quality=88); return 'file://' + p
for i in range(60):
    stores = ['steam', 'gog', 'epic', 'emu', 'manual']; src = stores[i % 5]
    games.append({'id': '%s:%d' % (src, 1000 + i), 'title': 'Test Game %02d' % i, 'source': src, 'store': src.title(), 'kind': src if src != 'manual' else 'yours',
                  'installed': i % 3 != 0, 'update': False, 'savesBackedUp': False, 'cloudSaves': False, 'devBuild': False,
                  'cover': pic('c%d.jpg' % i, 600, 900, 'jpg'), 'wide': pic('w%d.jpg' % i, 920, 430, 'jpg'), 'hero': pic('h%d.png' % i, 3840, 1240, 'png'),
                  'icon': '', 'lastPlayed': (1763600000000 - i * 3600000) if i < 12 else 0, 'minutes': 30 * i, 'system': '', 'chapter': ''})
c = {}
for g in games: c[g['kind']] = c.get(g['kind'], 0) + 1
c['all'] = len(games); c['installed'] = sum(1 for g in games if g['installed'])
json.dump({'games': games, 'counts': c}, open(os.path.join(d, 'firehold-library.json'), 'w'))
PY
  echo "seeded 60 test games with pictures" >>"$OUT/ipc.txt"
fi
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
# The start's own work done first: the game lists Hearth's app answers (Firehold's library, the app menu's Games) can
# take minutes in CI (stores that can't be reached) and arrive in the middle of the rest. Up to 5 minutes.
w0=$(date +%s)
while pgrep -f 'hearth-firehold library|hearth-games list|--list-games' >/dev/null 2>&1 && [ $(( $(date +%s) - w0 )) -lt 300 ]; do sleep 5; done
note "the start's game lists answered after $(( $(date +%s) - w0 )) s more$(pgrep -f -- '--list-games' >/dev/null && echo ' (still running after 5 minutes)')"
sleep 10
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
# Font caches out of date (a folder of fonts newer than every cache: each program then reads the fonts itself, a
# one-off stall and memory the cache would share; the image's, Nov 20)
note "font caches: $(python3 - <<'PY'
import glob, os
caches = [f for d in ('/usr/lib/fontconfig/cache', '/var/cache/fontconfig') for f in glob.glob(d + '/*cache*')]
newest = max([os.path.getmtime(f) for f in caches] or [0])
dirs = set()
for root in ('/usr/share/fonts', '/usr/local/share/fonts'):
    for d, _, _ in os.walk(root):
        dirs.add(d)
stale = sorted(d for d in dirs if os.path.getmtime(d) > newest)
print('%d cache files, %d font folders, %d newer than the caches%s' % (len(caches), len(dirs), len(stale), (': ' + ', '.join(stale[:6])) if stale else ''))
PY
)"
allticks >"$OUT/procs-start.txt"; n0=$(wc -l <"$OUT/procs-start.txt")
s0=$(ticks "$qs"); c0=$(ticks "$LPID"); m0=$(rss "$qs"); f0=$(frames)
# Every 5 s through the rest: the shell's threads, its children and every busy process, and how far the shell's log
# has got, so a busy moment can be pinned on what did it (rest-trace.txt; the busiest 5 s are named in the summary).
python3 - "$qs" "$SECS" "$SLOG" "$OUT/rest-trace.txt" "$OUT/rest-busiest.txt" <<'PY' &
import os, sys, time
qs, secs, slog, out, best = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
hz = os.sysconf('SC_CLK_TCK')
def stat(path):
    try:
        s = open(path).read(); c = s[s.index('(') + 1:s.rindex(')')]; f = s[s.rindex(')') + 2:].split()
        return c, int(f[11]) + int(f[12]), f[1]
    except (OSError, ValueError, IndexError):
        return None
def snap():
    th = {}
    for t in os.listdir('/proc/%s/task' % qs) if os.path.isdir('/proc/%s/task' % qs) else []:
        r = stat('/proc/%s/task/%s/stat' % (qs, t))
        if r: th[t] = (r[0], r[1])
    pr = {}
    for p in os.listdir('/proc'):
        if p.isdigit():
            r = stat('/proc/%s/stat' % p)
            if r:
                try: cmd = open('/proc/%s/cmdline' % p, 'rb').read().replace(b'\0', b' ').decode(errors='replace')[:100]
                except OSError: cmd = r[0]
                pr[p] = (cmd or r[0], r[1], r[2])
    try: lines = sum(1 for _ in open(slog, errors='replace'))
    except OSError: lines = 0
    return th, pr, lines
t0 = time.monotonic(); a = snap(); rows = []
with open(out, 'w') as f:
    while time.monotonic() - t0 < secs:
        time.sleep(5)
        b = snap(); el = time.monotonic() - t0
        thr = sorted(((v[1] - a[0].get(k, (v[0], v[1]))[1], v[0]) for k, v in b[0].items()), reverse=True)[:3]
        procs = sorted(((v[1] - (a[1][k][1] if k in a[1] else 0), v[0]) for k, v in b[1].items() if k != qs), reverse=True)[:3]
        kids = [v[0] for k, v in b[1].items() if v[2] == qs and k not in a[1]]
        shell = sum(d for d, _ in sorted(((v[1] - a[0].get(k, (v[0], v[1]))[1], v[0]) for k, v in b[0].items())))
        row = '%5.0fs shell %3d ticks | threads %s | others %s | new children %s | log %d→%d' % (
            el, shell, ', '.join('%s %d' % (n, d) for d, n in thr if d), ', '.join('%s %d' % (n, d) for d, n in procs if d),
            '; '.join(kids) or '-', a[2], b[2])
        f.write(row + '\n'); f.flush(); rows.append((shell, row, a[2], b[2]))
        a = b
rows.sort(reverse=True)
with open(best, 'w') as f:
    for shell, row, l0, l1 in rows[:3]:
        f.write(row + '\n')
        try:
            lines = open(slog, errors='replace').read().splitlines()[l0:l1]
            for l in lines[-8:]:
                if 'Frame rendered' not in l: f.write('    log: ' + l[:200] + '\n')
        except OSError: pass
PY
TRACE=$!
# PERF_REST=1 (the image-like check in labwc-fedora.yml): the shell's processor time sampled through the whole rest, so
# a busy moment that comes once (the image's, Nov 20) can be pinned on what the shell was doing (perf-rest.txt).
RPERF=""
if [ "${PERF_REST:-0}" = 1 ] && command -v perf >/dev/null 2>&1; then
  perf record -q -F 199 -g -p "$qs" -o /tmp/perf-rest-$KIND.data -- sleep "$SECS" >/dev/null 2>&1 & RPERF=$!
fi
mins=""; last=$s0; worst=0
for i in $(seq $((SECS / 60))); do
  sleep 60
  s=$(ticks "$qs"); d=$(( (s - last) * 10000 / HZ / 60 )); last=$s
  mins="$mins $d"; [ "$d" -gt "$worst" ] && worst=$d
  echo "minute $i: shell $d, memory $(rss "$qs") MB, frames $(( $(frames) - f0 ))" >>"$OUT/rest-minutes.txt"
done
wait "$TRACE" 2>/dev/null
if [ -n "$RPERF" ]; then
  wait "$RPERF" 2>/dev/null
  perf report -i /tmp/perf-rest-$KIND.data --stdio --no-children --percent-limit 1 --sort dso,sym 2>/dev/null | grep -v '^$' | head -200 >"$OUT/perf-rest.txt" || true
  perf report -i /tmp/perf-rest-$KIND.data --stdio --children --percent-limit 4 --sort sym -g caller,0.5,callee --max-stack 40 2>/dev/null | grep -v '^$' | head -400 >"$OUT/perf-rest-calls.txt" || true
  perf script -i /tmp/perf-rest-$KIND.data -F time,sym 2>/dev/null | awk 'NF {t=int($1); n[t]++} END {for (k in n) print k, n[k]}' | sort -n >"$OUT/perf-rest-seconds.txt" || true
  pk=$(sort -k2 -n "$OUT/perf-rest-seconds.txt" 2>/dev/null | tail -1 | cut -d' ' -f1)
  if [ -n "$pk" ]; then
    perf report -i /tmp/perf-rest-$KIND.data --time "$((pk - 4)),$((pk + 4))" --stdio --no-children --percent-limit 1 --sort dso,sym 2>/dev/null | grep -v '^$' | head -150 >"$OUT/perf-rest-peak.txt" || true
    perf report -i /tmp/perf-rest-$KIND.data --time "$((pk - 4)),$((pk + 4))" --stdio --children --percent-limit 3 --sort sym -g caller,0.5,callee --max-stack 60 2>/dev/null | grep -v '^$' | head -500 >"$OUT/perf-rest-peak-calls.txt" || true
    note "the busiest second at rest (perf): $pk, $(head -c 0 /dev/null)$(grep -m6 '%' "$OUT/perf-rest-peak.txt" | tr -s ' ' | cut -c1-90 | tr '\n' '|')"
  fi
  rm -f /tmp/perf-rest-$KIND.data
fi
note "the busiest 5 s at rest: $(head -1 "$OUT/rest-busiest.txt" 2>/dev/null | cut -c1-400)$(awk 'NR > 1 && /^    log: / {print} /^ *[0-9]+s shell/ && NR > 1 {exit}' "$OUT/rest-busiest.txt" | head -6 | cut -c1-200 | tr '\n' '|')"
[ "$KIND" = firehold ] && note "Firehold after the rest: $(timeout 10 quickshell ipc -p "$SH" call firehold state 2>/dev/null | cut -c1-160)"
note "the drive while at rest: $(df -h / 2>/dev/null | awk 'NR == 2 {print $4 " free of " $2}'); the shell's memory: $(awk '/^(Rss|Anonymous|Pss_File|Shared_Clean|Private_Dirty):/ {printf "%s %d MB, ", $1, $2 / 1024}' /proc/$qs/smaps_rollup 2>/dev/null)"
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

# ---- Something happening at rest (the image's busy minute, Nov 20: a low-drive warning's notification and password
# prompt came and went, and Firehold made every tile and picture again for it): a notification, then a window opening
# and closing. The shell's share of it over 20 s: at most 1 s of the processor, and nothing made again for nothing.
t0x=$(ticks "$qs"); mx0=$(rss "$qs")
notify-send -a Test "A test notification" "Nothing to do" >/dev/null 2>&1 || true
if command -v foot >/dev/null 2>&1; then foot -e sleep 4 >/dev/null 2>&1 & fi
sleep 20
ev=$(( ($(ticks "$qs") - t0x) * 100 / HZ ))
note "a notification and a window coming and going at rest: the shell used $(printf "%d.%02d s" $((ev / 100)) $((ev % 100))) of the processor ($(rss "$qs") MB, was ${mx0} MB)"
[ "$ev" -le 100 ] || err "a notification and a window coming and going at rest cost the shell $(printf "%d.%02d s" $((ev / 100)) $((ev % 100))) of the processor (limit 1 s): something is made again for nothing"
for t in notify quick; do WAIT=0 ipc "$t" close; done

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

# ---- The Welcome & Tips window (hearth-tools, WebKit) just showing, 60 s: it asked for a frame 60 times a second
# (the controller loop, ui/pad/hearth-gamepad.js), about 1.9 % of a core in the image (Nov 20). Limit 0.5 %.
if [ "$KIND" = classic ] && command -v hearth-tools >/dev/null 2>&1; then
  hearth-tools --app welcome >/dev/null 2>&1 &
  WPID=$!; sleep 20
  wt() { t=0; for p in $(pgrep -f 'hearth-tools --app welcome|WebKitWebProcess|WebKitNetworkProcess'); do t=$(( t + $(awk '{print $14 + $15}' /proc/$p/stat 2>/dev/null || echo 0) )); done; echo $t; }
  w0=$(wt); sleep 60; w1=$(wt)
  wp=$(( (w1 - w0) * 10000 / HZ / 60 ))
  note "the Welcome window just showing: $(pct "$wp") of a core"
  [ "$wp" -le 50 ] || err "the Welcome window used $(pct "$wp") of a core just showing (limit 0.50 %)"
  shot 30-welcome
  kill "$WPID" 2>/dev/null; pkill -f 'hearth-tools --app welcome' 2>/dev/null; sleep 2
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
cp "$OUT/rest.md" "$OUT/rest-notes.txt" 2>/dev/null || true     # (the image's looks branch keeps .txt, not .md)
kill "$LPID" 2>/dev/null; sleep 2; pkill -u "$(id -u)" -x quickshell 2>/dev/null
exit "$fail"
