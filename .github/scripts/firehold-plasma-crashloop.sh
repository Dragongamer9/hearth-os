#!/bin/bash
# Firehold on Plasma: does the two-screen Desk crash when it starts, and is it ours? (firehold-plasma.yml, crashloop)
#   firehold-plasma-crashloop.sh OUT [N]
# On KWin's virtual backend with two outputs, as the pictures' two-screen part has it (KDE's daemon kded6 started just
# before plasmashell), N starts (default 15) of Firehold's Desk (org.hearthos.firehold.desk), then N of stock Plasma
# (org.kde.plasma.desktop: no Hearth containment or widgets). Each start: up within 40 s and still running 10 s later,
# else a crash. KDE's crash handler is off (KDE_DEBUG=1), so a crash leaves a core; its backtrace (gdb, with Fedora's
# debuginfod for plasma-workspace, Qt and the rest) goes in OUT/bt-*.txt. Summary: OUT/crashloop.txt and annotations.
set +e +o pipefail
OUT=${1:-crash}; N=${2:-15}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/step.log") 2>&1

export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Pictures" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 LIBGL_ALWAYS_SOFTWARE=1 HEARTH_FIREHOLD_PLASMA=1
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
RUN="$XDG_RUNTIME_DIR/hearth-firehold-plasma"
FX=/tmp/fhp-fixtures
python3 "$(dirname "$0")/firehold-plasma-fixtures.py" "$HOME" "$FX" >/dev/null 2>&1
export HEARTH_FIREHOLD_HOME_FIXTURE="$FX/home.json" HEARTH_FIREHOLD_SYS_FIXTURE="$FX/sys.json"
export HEARTH_FIREHOLD_SCREENS="Virtual-0::LG 27GP850:2560 × 1440 · 165 Hz · DisplayPort 1,Virtual-1:tv:TCL 55″ TV:3840 × 2160 · 60 Hz · HDMI 2" HEARTH_FIREHOLD_TV_SCREENS=Virtual-1

# Cores: into OUT's cores folder, whole.
ulimit -c unlimited
mkdir -p /tmp/cores && chmod 1777 /tmp/cores
echo '/tmp/cores/core.%e.%p.%t' > /proc/sys/kernel/core_pattern 2>/dev/null
echo "core_pattern: $(cat /proc/sys/kernel/core_pattern) · ulimit -c $(ulimit -c)" > "$OUT/crashloop.txt"

for p in pipewire wireplumber; do command -v $p >/dev/null && { $p >"$OUT/$p.log" 2>&1 & }; done
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --output-count 2 --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin.log" 2>&1 &
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$OUT/kamd.log" 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"$OUT/kglobalaccel.log" 2>&1 & break; }; done
sleep 3

# start SHELL TAG I: one start; prints "up", "slow" or "crash"
start() {
  local shell=$1 tag=$2 i=$3 log="$OUT/$2-$3.log"
  pkill -x kded6; sleep 0.3
  command -v kded6 >/dev/null && { kded6 >"$OUT/$tag-$i-kded.log" 2>&1 & }
  local since=$(date +%s)
  KDE_DEBUG=1 HEARTH_FIREHOLD_TIP=always PLASMA_DEFAULT_SHELL=$shell plasmashell -p "$shell" >"$log" 2>&1 &
  local pid=$! up=""
  for s in $(seq 1 40); do
    kill -0 $pid 2>/dev/null || break
    if [ "$shell" = org.hearthos.firehold.desk ]; then
      f="$RUN/ready-$shell"; [ -f "$f" ] && [ "$(stat -c %Y "$f")" -ge "$since" ] && { up=1; break; }
    else
      dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript string:'print(desktops().length)' 2>/dev/null | grep -q 'string "[1-9]' && { up=1; break; }
    fi
    sleep 1
  done
  sleep 10
  if ! kill -0 $pid 2>/dev/null; then
    wait $pid; echo "crash (exit $?)"
  elif [ -z "$up" ]; then
    echo slow
  else
    echo up
  fi
  kill $pid 2>/dev/null; for s in $(seq 1 10); do kill -0 $pid 2>/dev/null || break; sleep 0.5; done
  kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 1
}

PLASMA_DEFAULT_SHELL=org.hearthos.firehold.desk hearth-firehold-plasma session >"$OUT/session.log" 2>&1 &
for tag in firehold stock; do
  shell=org.hearthos.firehold.desk; [ $tag = stock ] && shell=org.kde.plasma.desktop
  [ $tag = firehold ] && { hearth-firehold-plasma begin $shell >/dev/null 2>&1
    now=$(($(date +%s) * 1000)); mkdir -p "$RUN"
    printf '{"screens": {"Virtual-1": {"mode": "couch", "why": "pad", "at": %s}, "Virtual-0": {"mode": "desk", "why": "mouse", "at": %s}}}' "$now" "$now" > "$RUN/modes.json"; }
  [ $tag = stock ] && pkill -f "hearth-firehold-plasma session"
  ups=0; crashes=0; slow=0
  for i in $(seq 1 "$N"); do
    r=$(start "$shell" "$tag" "$i")
    echo "$tag $i: $r" >> "$OUT/crashloop.txt"
    case "$r" in up) ups=$((ups + 1));; slow) slow=$((slow + 1));; *) crashes=$((crashes + 1));; esac
  done
  echo "::notice title=Starts ($tag)::$ups of $N up, $crashes crashed, $slow slow"
  echo "$tag: $ups of $N up, $crashes crashed, $slow slow" >> "$OUT/crashloop.txt"
done

# The backtraces (at most three cores), with Fedora's debuginfod for the symbols.
export DEBUGINFOD_URLS="${DEBUGINFOD_URLS:-https://debuginfod.fedoraproject.org/}"
n=0
for c in $(ls -t /tmp/cores/core.* 2>/dev/null | head -3); do
  n=$((n + 1))
  timeout 900 gdb -batch -ex "set debuginfod enabled on" -ex "info sharedlibrary" -ex "thread apply all bt 30" -ex "echo \n=== the crashing thread\n" -ex "bt full 25" \
    "$(command -v plasmashell)" "$c" > "$OUT/bt-$n.txt" 2>&1
  echo "::notice title=Backtrace $n ($(basename "$c"))::$(sed -n '/=== the crashing thread/,$p' "$OUT/bt-$n.txt" | grep -E '^#' | head -18 | cut -c1-150 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
done
ls -la /tmp/cores >> "$OUT/crashloop.txt" 2>&1
pkill plasmashell; pkill -f /tmp/kwin_wayland; pkill kded6; pkill dbus-daemon
exit 0
