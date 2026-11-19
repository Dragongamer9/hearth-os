#!/bin/bash
# The owner's red flag (Nov 17): his devices must never freeze or lag again. Desktop Mode at rest for SECS (600: ten
# minutes) on a 1920 x 1080 screen with nothing to do, measured the same way for each (firehold-rust.yml "idle"):
#   firehold-rust-idle.sh firehold SECS   firehold-comp --headless with Firehold's Rust shell (the hub, the Desk's
#                                         bar and dock, the Couch as the wallpaper), software drawing (tiny-skia)
#   firehold-rust-idle.sh plasma SECS     plain KDE Plasma: plasmashell on kwin_wayland --virtual, software drawing
# Out: "RESULT kind up_s shell_cpu comp_cpu shell_mb_start shell_mb_end comp_mb_start comp_mb_end restarts" (cpu in
# hundredths of a percent of one core; the shell is every process of it added up) and a notice.
set -u
KIND=${1:?firehold or plasma}; SECS=${2:-600}
export XDG_RUNTIME_DIR=/tmp/xdg-idle-$KIND HOME=/tmp/home-idle-$KIND LC_ALL=C.UTF-8 LIBGL_ALWAYS_SOFTWARE=1 QT_QUICK_BACKEND=software
rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME"; chmod 700 "$XDG_RUNTIME_DIR"
HZ=$(getconf CLK_TCK)
ticks() { local t=0 p; for p in "$@"; do t=$(( t + $(awk '{print $14 + $15}' "/proc/$p/stat" 2>/dev/null || echo 0) )); done; echo $t; }
rss() { local m=0 p; for p in "$@"; do m=$(( m + $(awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$p/status" 2>/dev/null || echo 0) )); done; echo $m; }
t0=$(date +%s.%N)
if [ "$KIND" = firehold ]; then
  export ICED_BACKEND=tiny-skia FIREHOLD_NO_PAD=1 NIMBUS_SHARE="$PWD/hosts/linux" FIREHOLD_LOG="warn"
  # The headless screen draws 60 times a second unless told to wait for changes, as a real screen does (headless.rs).
  [ "${FIREHOLD_IDLE_STEADY:-}" = 1 ] || export FIREHOLD_HEADLESS_IDLE=1
  B=/tmp/bin-idle; mkdir -p "$B"; python3 .github/scripts/firehold-fixtures.py "$B" && chmod +x "$B"/*
  export PATH="$PWD:$B:$PATH" NIMBUS_APP="$B/nimbus"
  ./firehold-comp --headless --size 1920x1080 --socket wayland-idle -- ./firehold-shell hub >/tmp/idle-comp.log 2>&1 &
  up() { [ -e "$XDG_RUNTIME_DIR/firehold-shell-up" ]; }
  shell_pids() { pgrep -x firehold-shell | sort; }
  comp_pid() { pgrep -n -x firehold-comp; }
else
  export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true
  [ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
  # KWin's file capability (CAP_SYS_NICE) isn't allowed in a CI container ("Operation not permitted"): a plain copy.
  mkdir -p /tmp/kwbin && cp "$(command -v kwin_wayland)" /tmp/kwbin/kwin_wayland && export PATH="/tmp/kwbin:$PATH"
  kwin_wayland --virtual --width 1920 --height 1080 --socket wayland-idle >/tmp/idle-comp.log 2>&1 &
  for i in $(seq 120); do [ -S "$XDG_RUNTIME_DIR/wayland-idle" ] && break; sleep 0.5; done
  WAYLAND_DISPLAY=wayland-idle plasmashell --no-respawn >/tmp/idle-shell.log 2>&1 &
  up() { dbus-send --session --print-reply --dest=org.freedesktop.DBus / org.freedesktop.DBus.NameHasOwner string:org.kde.plasmashell 2>/dev/null | grep -q 'boolean true'; }
  shell_pids() { pgrep -x plasmashell | sort; }
  comp_pid() { pgrep -n -x kwin_wayland; }
fi
upt=none
for i in $(seq 240); do up && { upt=$(python3 -c "print('%.1f' % ($(date +%s.%N) - $t0))"); break; }; sleep 0.5; done
sleep 20   # the start's own work done
S0=$(shell_pids | tr '\n' ' '); C=$(comp_pid)
if [ -z "$S0" ] || [ -z "$C" ]; then
  echo "RESULT $KIND $upt gone gone 0 0 0 0 0"
  echo "::error title=at rest ($KIND)::it didn't start: $(tail -c 700 /tmp/idle-comp.log | tr '\n' ' ')"
  exit 0
fi
# shellcheck disable=SC2086
s0=$(ticks $S0); c0=$(ticks "$C"); m0=$(rss $S0); cm0=$(rss "$C")
sleep "$SECS"
S1=$(shell_pids | tr '\n' ' ')
restarts=0; [ "$S0" = "$S1" ] || restarts=1
# shellcheck disable=SC2086
s1=$(ticks $S0); c1=$(ticks "$C"); m1=$(rss $S0); cm1=$(rss "$C")
echo "RESULT $KIND $upt $(( (s1 - s0) * 10000 / HZ / SECS )) $(( (c1 - c0) * 10000 / HZ / SECS )) $m0 $m1 $cm0 $cm1 $restarts"
exit 0
