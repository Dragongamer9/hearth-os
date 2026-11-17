#!/bin/bash
# The lock screen on Debian trixie for real (kwin-input.yml's lock-debian job): KWin's screen locker with Hearth's
# lock screen, a user with a password, the right password typed with hearth-type unlocks (PAM), a wrong one doesn't.
#   kwin-debian-lock.sh setup     (root) packages, Hearth's lock screen (kwin-lock/install.sh), the test user
#   kwin-debian-lock.sh run       (the test user) labwc holding KWin, lock, type, check
# Pictures to shots/, results as ::notice:: / ::error:: lines; exit 1 if a check failed.
set -u
cd "$(dirname "$0")/../.." || exit 1
REPO=$PWD
USER_NAME=hearthtest
PASSWORD='Ember-4321'

setup() {
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends kwin-wayland labwc grim wlr-randr dbus dbus-bin python3 python3-evdev \
    python3-gi gir1.2-glib-2.0 udev kmod libgl1-mesa-dri libegl-mesa0 xwayland passwd libpam-runtime \
    fonts-noto-core binutils ca-certificates >/tmp/apt.log 2>&1 || { echo "::error title=Debian lock (setup)::$(tail -5 /tmp/apt.log | tr '\n' '|')"; exit 1; }
  apt-get install -y -qq --no-install-recommends libkscreenlocker6 plasma-framework >/dev/null 2>&1
  echo "::notice title=Debian Plasma/Shell::$(grep -rlF 'Plasma/Shell' /usr/lib/x86_64-linux-gnu/qt6/plugins/kf6 /usr/lib/x86_64-linux-gnu/libPlasma*.so* /usr/lib/x86_64-linux-gnu/libKScreenLocker*.so* /usr/lib/x86_64-linux-gnu/libexec/kscreenlocker_greet 2>/dev/null | tr '\n' ' ') types: $(ls /usr/lib/x86_64-linux-gnu/qt6/plugins/kf6/packagestructure/ 2>/dev/null | tr '\n' ' ')"
  bash -x hosts/linux/desktop/kwin-lock/install.sh hosts/linux/desktop/kwin-lock >/tmp/install.log 2>&1 \
    || { echo "::error title=Debian lock (install.sh)::$(grep -E 'install.sh' /tmp/install.log | tail -6 | tr '\n' '|')"; exit 1; }
  echo "::notice title=Debian lock (install.sh)::$(grep -E '^install.sh' /tmp/install.log | tr '\n' '|')"
  mkdir -p /usr/share/nimbus && rm -rf /usr/share/nimbus/desktop && cp -a hosts/linux/desktop /usr/share/nimbus/desktop
  echo "::notice title=Debian lock (versions)::$(dpkg-query -W kwin-wayland labwc libkscreenlocker6 2>/dev/null | tr '\n' ' ') PAM kde: $(ls /etc/pam.d/ | grep -iE '^kde|screen' | tr '\n' ' ')"
  # The test user, with a password PAM checks.
  id "$USER_NAME" >/dev/null 2>&1 || useradd -m -s /bin/bash "$USER_NAME"
  echo "$USER_NAME:$PASSWORD" | chpasswd
  # Input and render devices for a user without a seat manager here (CI only): uinput, evdev, vgem.
  modprobe uinput 2>/dev/null; modprobe vgem 2>/dev/null
  mkdir -p /etc/udev/rules.d
  printf 'KERNEL=="event*", MODE="0666"\nKERNEL=="uinput", MODE="0666"\nSUBSYSTEM=="drm", MODE="0666"\n' > /etc/udev/rules.d/99-hearth-ci.rules
  pgrep -f systemd-udevd >/dev/null || /usr/lib/systemd/systemd-udevd --daemon
  udevadm control --reload 2>/dev/null; udevadm trigger --action=add 2>/dev/null; udevadm settle -t 10 2>/dev/null
  chmod 666 /dev/uinput /dev/input/event* /dev/dri/* 2>/dev/null
  mkdir -p shots && chmod 777 shots
  echo "::notice title=Debian lock (devices)::$(ls -l /dev/uinput /dev/dri 2>&1 | tr '\n' ' ' | cut -c1-400)"
}

run() {
  fail=0
  ok() { echo "::notice title=Debian lock: $1::${2:-passed}"; }
  bad() { echo "::error title=Debian lock: $1::${2:-failed}"; fail=1; }
  export XDG_RUNTIME_DIR=/tmp/xdg-$USER_NAME HOME=/home/$USER_NAME
  mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
  export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  mkdir -p "$HOME/.config/labwc"
  printf '%s\n' '<labwc_config>' \
    '<windowRules><windowRule identifier="*" serverDecoration="no"><action name="MoveTo" x="0" y="0"/></windowRule></windowRules>' \
    '</labwc_config>' > "$HOME/.config/labwc/rc.xml"
  for renderer in gles2 pixman; do
    WLR_BACKENDS=headless,libinput LIBSEAT_BACKEND=noop WLR_RENDERER=$renderer WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
      labwc >"$REPO/shots/debian-outer.log" 2>&1 &
    for i in $(seq 1 20); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 1; done
    ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break
    pkill -u "$USER_NAME" labwc; sleep 1
  done
  OUTER=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  [ -n "$OUTER" ] || { bad "labwc" "didn't start: $(tail -4 "$REPO/shots/debian-outer.log" | tr '\n' '|')"; return 1; }
  echo "::notice title=Debian lock (labwc)::renderer $renderer"
  WAYLAND_DISPLAY=$OUTER wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1
  # A keyboard before KWin's window appears (labwc tells a window only about keyboards it had then).
  python3 /usr/share/nimbus/desktop/shell/hearth-type --serve >"$REPO/shots/debian-type.log" 2>&1 &
  sleep 3
  # What Hearth's KWin session gives KWin for the lock screen (the session branch's environment).
  export HEARTH_WM=kwin PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock XDG_CURRENT_DESKTOP=Hearth:KDE LIBGL_ALWAYS_SOFTWARE=1
  export XDG_CONFIG_DIRS="$REPO/hosts/linux/desktop/kwin/xdg:/etc/xdg"
  WAYLAND_DISPLAY=$OUTER kwin_wayland --wayland-display "$OUTER" --width 1920 --height 1080 --socket wayland-hearth >"$REPO/shots/debian-kwin.log" 2>&1 &
  for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
  [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { bad "KWin" "didn't start: $(tail -5 "$REPO/shots/debian-kwin.log" | tr '\n' '|')"; return 1; }
  export WAYLAND_DISPLAY=wayland-hearth
  sleep 5
  shot() { WAYLAND_DISPLAY=$OUTER grim "$REPO/shots/$1.png" 2>>"$REPO/shots/debian-grim.log" || echo "no picture $1"; }
  active() { gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.GetActive 2>/dev/null | grep -q true; }
  HT="python3 /usr/share/nimbus/desktop/shell/hearth-type"

  r=$(python3 /usr/share/nimbus/desktop/shell/hearth-lockcheck); rc=$?
  [ $rc = 0 ] && ok "hearth-lockcheck (KWin's own environment, the greeter's test run shows Hearth's lock screen loading)" "$r" \
    || bad "hearth-lockcheck says it's not safe to lock" "$r"
  [ $rc = 0 ] || return 1

  gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.Lock >/dev/null 2>&1
  for i in $(seq 1 20); do active && break; sleep 0.5; done
  active && ok "locked by KWin's locker" || { bad "KWin's locker didn't lock" "$(grep -iE 'lock|greet' "$REPO/shots/debian-kwin.log" | tail -5 | tr '\n' '|')"; return 1; }
  sleep 8
  shot debian-locked
  grep -q "hearth-lock-screen: loaded" "$REPO/shots/debian-kwin.log" && ok "the lock screen up is Hearth's (its 'loaded' line)" \
    || bad "the lock screen up isn't Hearth's" "$(grep -iE 'greet|lockscreen|qml' "$REPO/shots/debian-kwin.log" | tail -6 | tr '\n' '|')"

  $HT -- 'not-the-password' >/dev/null 2>&1; sleep 0.5; $HT -k Return >/dev/null 2>&1
  sleep 6
  shot debian-wrong-password
  active && ok "a wrong password doesn't unlock" || bad "a wrong password unlocked the screen"

  $HT -k Escape >/dev/null 2>&1; sleep 0.5
  $HT -- "$PASSWORD" >/dev/null 2>&1; sleep 0.5; $HT -k Return >/dev/null 2>&1
  for i in $(seq 1 30); do active || break; sleep 0.5; done
  shot debian-unlocked
  active && bad "the right password didn't unlock" "$(grep -iE 'pam|auth|greet|unlock' "$REPO/shots/debian-kwin.log" | tail -8 | tr '\n' '|')" \
    || ok "the right password, typed with hearth-type, unlocks (PAM)"
  echo "::notice title=Debian lock (KWin log)::$(grep -iE 'greet|lock|pam' "$REPO/shots/debian-kwin.log" | grep -v 'notify signal' | tail -12 | tr '\n' '|' | cut -c1-1800)"
  pkill -u "$USER_NAME" kwin_wayland; pkill -u "$USER_NAME" labwc; pkill -u "$USER_NAME" -f hearth-type; pkill -u "$USER_NAME" dbus-daemon
  return $fail
}

case "${1:-}" in
  setup) setup ;;
  run) run ;;
  *) echo "kwin-debian-lock.sh setup|run"; exit 2 ;;
esac
