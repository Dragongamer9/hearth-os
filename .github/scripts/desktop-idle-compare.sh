#!/bin/bash
# Smoothness, side by side (1.15, the owner: Anatase "ran a lot smoother than Debian"). Desktop Mode at rest in each
# image, on the same CI runner, the same way: a virtual 1920x1080 screen, software drawing (CI has no GPU), nothing
# open, for IDLE_SECS (600: ten minutes). Measured: how long until the desktop is up, then the processor time and
# memory of the shell and the compositor at rest.
#   desktop-idle-compare.sh IMAGE plasma   Hearth OS on Anatase: plasmashell on kwin_wayland --virtual
#   desktop-idle-compare.sh IMAGE mantel   Hearth OS on Debian: Mantel (quickshell) on labwc's headless backend
#   desktop-idle-compare.sh IMAGE firehold Hearth OS on Anatase: Firehold (Rust, test), firehold-shell on firehold-comp
#                                          --headless (every firehold-shell process added up)
# One ::notice line with the numbers (warn-only: it never fails the build), and "RESULT kind up_s shell_pct comp_pct
# shell_mb_start shell_mb_end comp_mb" on stdout for the summary.
set -u
IMG=${1:?image}; KIND=${2:?plasma, mantel or firehold}; SECS=${IDLE_SECS:-600}
NAME=idle-$KIND-$$
sudo podman run -d --name "$NAME" --privileged --entrypoint sleep "$IMG" infinity >/dev/null || { echo "::warning title=smoothness ($KIND)::couldn't start the image"; exit 0; }
trap 'sudo podman rm -f "$NAME" >/dev/null 2>&1' EXIT
cat > /tmp/idle-$KIND.sh <<'IN'
set -u
KIND=$1; SECS=$2
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home LC_ALL=C.UTF-8 QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1
mkdir -p "$XDG_RUNTIME_DIR" "$HOME"; chmod 700 "$XDG_RUNTIME_DIR"
HZ=$(getconf CLK_TCK)
ticks() { t=0; for p in "$@"; do t=$(( t + $(awk '{print $14 + $15}' "/proc/$p/stat" 2>/dev/null || echo 0) )); done; echo $t; }
rss() { m=0; for p in "$@"; do m=$(( m + $(awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$p/status" 2>/dev/null || echo 0) )); done; echo $m; }
t0=$(date +%s.%N)
if [ "$KIND" = plasma ]; then
  export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true
  [ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu | head -1)" applications.menu)
  kwin_wayland --virtual --width 1920 --height 1080 --socket hearth-idle >/tmp/comp.log 2>&1 &
  for i in $(seq 120); do [ -S "$XDG_RUNTIME_DIR/hearth-idle" ] && break; sleep 0.5; done
  WAYLAND_DISPLAY=hearth-idle plasmashell --no-respawn >/tmp/shell.log 2>&1 &
  up() { dbus-send --session --print-reply --dest=org.freedesktop.DBus / org.freedesktop.DBus.NameHasOwner string:org.kde.plasmashell 2>/dev/null | grep -q 'boolean true'; }
  SHELL_P=plasmashell; COMP_P=kwin_wayland
elif [ "$KIND" = firehold ]; then
  export ICED_BACKEND=tiny-skia FIREHOLD_NO_PAD=1 FIREHOLD_LOG=warn
  /usr/libexec/hearth/firehold-comp --headless --size 1920x1080 --socket hearth-idle -- firehold-shell hub >/tmp/comp.log 2>&1 &
  up() { [ -e "$XDG_RUNTIME_DIR/firehold-shell-up" ]; }
  SHELL_P=firehold-shell; COMP_P=firehold-comp
else
  export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 XDG_CURRENT_DESKTOP=Nimbus
  labwc >/tmp/comp.log 2>&1 &
  for i in $(seq 120); do ls "$XDG_RUNTIME_DIR"/wayland-* >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  quickshell -p /usr/share/nimbus/desktop/shell >/tmp/shell.log 2>&1 &
  up() { [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ]; }
  SHELL_P=quickshell; COMP_P=labwc
fi
upt=""
for i in $(seq 240); do up && { upt=$(python3 -c "print('%.1f' % ($(date +%s.%N) - $t0))"); break; }; sleep 0.5; done
[ -n "$upt" ] || upt=none
sleep 20     # the start's own work done
s=$(pgrep -x "$SHELL_P" | tr '\n' ' '); c=$(pgrep -n -x "$COMP_P")
if [ -z "$s" ] || [ -z "$c" ]; then echo "RESULT $KIND $upt gone gone 0 0 0"; tail -5 /tmp/shell.log /tmp/comp.log >&2; exit 0; fi
s0=$(ticks $s); c0=$(ticks "$c"); m0=$(rss $s)
sleep "$SECS"
s1=$(ticks $s); c1=$(ticks "$c"); m1=$(rss $s); cm=$(rss "$c")
kill -0 $s 2>/dev/null || { echo "RESULT $KIND $upt gone gone $m0 0 $cm"; exit 0; }
echo "RESULT $KIND $upt $(( (s1 - s0) * 10000 / HZ / SECS )) $(( (c1 - c0) * 10000 / HZ / SECS )) $m0 $m1 $cm"
IN
sudo podman cp /tmp/idle-$KIND.sh "$NAME:/tmp/idle.sh"
ver=$(sudo podman exec "$NAME" cat /usr/share/nimbus/version 2>/dev/null)
out=$(sudo podman exec "$NAME" bash -c "dbus-run-session -- bash /tmp/idle.sh $KIND $SECS" 2>/tmp/idle-$KIND.err | grep '^RESULT' | tail -1)
[ -n "$out" ] || { echo "::warning title=smoothness ($KIND)::no result: $(tail -c 600 /tmp/idle-$KIND.err | tr '\n' ' ')"; exit 0; }
read -r _ k up sp cp m0 m1 cm <<<"$out"
p() { case "$1" in ''|*[!0-9]*) echo "$1";; *) printf '%d.%02d%%' $(($1 / 100)) $(($1 % 100));; esac; }
case "$KIND" in plasma) label="Anatase ${ver}: KDE Plasma (plasmashell on KWin)" ;; firehold) label="Anatase ${ver}: Firehold (Rust, test) on firehold-comp" ;; *) label="Debian ${ver}: Mantel on labwc" ;; esac
echo "::notice title=smoothness ($KIND)::$label · up in ${up} s · at rest over ${SECS} s: shell $(p "$sp") of a core, compositor $(p "$cp") · shell memory ${m0} → ${m1} MB, compositor ${cm} MB (virtual screen, software drawing)"
echo "$out"
exit 0
