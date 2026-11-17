#!/bin/bash
# The lock screen on Debian trixie for real (kwin-input.yml's lock-debian job): KWin's screen locker with Hearth's
# lock screen, a user with a password, the right password typed with hearth-type unlocks (PAM), a wrong one doesn't.
#   kwin-debian-lock.sh setup     (root) packages, Hearth's lock screen (kwin-lock/install.sh), the test user
#   kwin-debian-lock.sh run       (root) labwc, then (the test user) KWin, lock, type, check
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
    python3-gi gir1.2-glib-2.0 libglib2.0-bin udev kmod libgl1-mesa-dri libegl-mesa0 xwayland passwd libpam-runtime \
    fonts-noto-core binutils ca-certificates libkf6config-bin >/tmp/apt.log 2>&1 || { echo "::error title=Debian lock (setup)::$(tail -5 /tmp/apt.log | tr '\n' '|')"; exit 1; }
  apt-get install -y -qq --no-install-recommends cmake ninja-build g++ extra-cmake-modules qt6-base-dev libkf6package-dev libkf6coreaddons-dev >/tmp/apt-dev.log 2>&1 \
    || { echo "::error title=Debian lock (build deps)::$(tail -3 /tmp/apt-dev.log | tr '\n' '|')"; exit 1; }
  cmake -S hosts/linux/desktop/kwin-helpers/shell-package -B /tmp/hearth-shell-package -G Ninja -DCMAKE_BUILD_TYPE=Release >/tmp/cmake.log 2>&1 \
    && cmake --build /tmp/hearth-shell-package >>/tmp/cmake.log 2>&1 \
    || { echo "::error title=Debian lock (Plasma/Shell type)::$(grep -iE 'error' /tmp/cmake.log | head -6 | tr '\n' '|' | cut -c1-1500)"; exit 1; }
  export HEARTH_SHELL_PACKAGE_SO=/tmp/hearth-shell-package/plasma_shell.so
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

# As root: labwc (it needs the input and render devices), then the rest as the test user.
run() {
  export XDG_RUNTIME_DIR=/tmp/xdg-$USER_NAME
  mkdir -p "$XDG_RUNTIME_DIR" && chown "$USER_NAME:$USER_NAME" "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
  printf '%s\n' '<labwc_config>' \
    '<windowRules><windowRule identifier="*" serverDecoration="no"><action name="MoveTo" x="0" y="0"/></windowRule></windowRules>' \
    '</labwc_config>' > /tmp/labwc-rc.xml
  mkdir -p /root/.config/labwc && cp /tmp/labwc-rc.xml /root/.config/labwc/rc.xml
  # The test user's typing keyboard has to exist before KWin's window appears (labwc tells a window only about the
  # keyboards it had then): a keyboard made here first, kept until the end.
  python3 - <<'PY' &
import evdev, time
e = evdev.ecodes
kb = evdev.UInput({e.EV_KEY: [e.KEY_A, e.KEY_ENTER]}, name='Hearth CI placeholder keyboard')
time.sleep(900)
PY
  sleep 2
  for renderer in gles2 pixman; do
    ( umask 000; WLR_RENDERER_ALLOW_SOFTWARE=1 WLR_RENDER_DRM_DEVICE=/dev/dri/renderD128 WLR_BACKENDS=headless,libinput LIBSEAT_BACKEND=noop WLR_RENDERER=$renderer WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
        WAYLAND_DISPLAY= labwc >"$REPO/shots/debian-outer-$renderer.log" 2>&1 & )
    for i in $(seq 1 20); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 1; done
    ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break
    pkill labwc; sleep 1
  done
  OUTER=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  [ -n "$OUTER" ] || { echo "::error title=Debian lock: labwc::didn't start: $(tail -4 "$REPO"/shots/debian-outer-*.log | tr '\n' '|')"; return 1; }
  chmod 777 "$XDG_RUNTIME_DIR/$OUTER"
  echo "::notice title=Debian lock (labwc)::renderer $renderer; gles2: $(grep -iE 'error|fail|egl|gbm|render' "$REPO/shots/debian-outer-gles2.log" 2>/dev/null | head -6 | tr '\n' '|' | cut -c1-900)"
  WAYLAND_DISPLAY=$OUTER wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1
  runuser -u "$USER_NAME" -- env OUTER="$OUTER" XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" bash "$REPO/.github/scripts/kwin-debian-lock.sh" user
  rc=$?
  pkill labwc
  return $rc
}

# As the test user: KWin (with the KWin session's environment, also written to kwin.env as the session does),
# hearth-lockcheck, lock, a wrong password, the right password.
user() {
  fail=0
  ok() { echo "::notice title=Debian lock: $1::${2:-passed}"; }
  bad() { echo "::error title=Debian lock: $1::${2:-failed}"; fail=1; }
  export HOME=/home/$USER_NAME
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
  export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  python3 /usr/share/nimbus/desktop/shell/hearth-type --serve >"$REPO/shots/debian-type.log" 2>&1 &
  sleep 3
  export HEARTH_WM=kwin PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock XDG_CURRENT_DESKTOP=Hearth:KDE LIBGL_ALWAYS_SOFTWARE=1
  export XDG_CONFIG_DIRS="$REPO/hosts/linux/desktop/kwin/xdg:$REPO/hosts/linux/desktop/kwin/windows/xdg:/etc/xdg"
  # Your own files left from KDE Plasma (1.13) turning KWin's own locking on (and its input method, the Super key):
  # Hearth's fixed keys ([$i] in kwin/xdg) must win, so KWin never locks by itself (review, Nov 17).
  mkdir -p "$HOME/.config"
  printf '[Daemon]\nAutolock=true\nLockOnResume=true\nTimeout=1\n' > "$HOME/.config/kscreenlockerrc"
  printf '[Wayland]\nInputMethod[$e]=/usr/share/applications/com.github.maliit.keyboard.desktop\n\n[ModifierOnlyShortcuts]\nMeta=org.kde.plasmashell,/PlasmaShell,org.kde.PlasmaShell,activateLauncherMenu\n\n[NightColor]\nActive=true\n' > "$HOME/.config/kwinrc"
  kr() { kreadconfig6 --file "$1" --group "$2" --key "$3" 2>/dev/null; }
  mine=$(XDG_CONFIG_DIRS=/etc/xdg kr kscreenlockerrc Daemon Autolock)
  [ "$mine" = true ] && ok "your own kscreenlockerrc is read (without Hearth's folder: Autolock=$mine)" || bad "the test's own kscreenlockerrc isn't read" "Autolock=$mine"
  got="$(kr kscreenlockerrc Daemon Autolock) $(kr kscreenlockerrc Daemon LockOnResume) $(kr kscreenlockerrc Daemon Timeout)"
  [ "$got" = "false false 0" ] && ok "Hearth's fixed kscreenlockerrc keys win over yours (KConfig on KWin 6.3: Autolock LockOnResume Timeout = $got)" \
    || bad "your kscreenlockerrc overrides Hearth's (Autolock LockOnResume Timeout = $got)"
  im=$(kr kwinrc Wayland InputMethod); meta=$(kr kwinrc ModifierOnlyShortcuts Meta); night=$(kr kwinrc NightColor Active)
  [ "$im" = /usr/share/applications/hearth-kwin-im.desktop ] && [ "$meta" = org.hearth.Windows,/org/hearth/Windows,org.hearth.Windows,Launcher ] \
    && ok "Hearth's input method and Super key win over yours" "InputMethod=$im Meta=$meta" || bad "your kwinrc overrides Hearth's input method or Super key" "InputMethod=$im Meta=$meta"
  [ "$night" = true ] && ok "your own Night Light (a setting Hearth exposes) is still yours" || bad "a setting Hearth exposes was fixed" "NightColor Active=$night"
  mkdir -p "$XDG_RUNTIME_DIR/hearth" && chmod 700 "$XDG_RUNTIME_DIR/hearth"
  env -0 > "$XDG_RUNTIME_DIR/hearth/kwin.env" && chmod 600 "$XDG_RUNTIME_DIR/hearth/kwin.env"
  WAYLAND_DISPLAY=$OUTER kwin_wayland --wayland-display "$OUTER" --width 1920 --height 1080 --socket wayland-hearth >"$REPO/shots/debian-kwin.log" 2>&1 &
  for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
  [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { bad "KWin" "didn't start: $(tail -5 "$REPO/shots/debian-kwin.log" | tr '\n' '|')"; return 1; }
  export WAYLAND_DISPLAY=wayland-hearth
  sleep 5
  shot() { WAYLAND_DISPLAY=$OUTER grim "$REPO/shots/$1.png" 2>>"$REPO/shots/debian-grim.log" || echo "no picture $1"; }
  active() { gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.GetActive 2>/dev/null | grep -q true; }
  HT="python3 /usr/share/nimbus/desktop/shell/hearth-type"
  echo "::notice title=Debian lock (KWin's /proc environ readable)::$(for p in $(pgrep -u "$USER_NAME" -x kwin_wayland); do head -c 1 /proc/$p/environ >/dev/null 2>&1 && echo yes || echo 'no (file capability): kwin.env used'; done)"

  r=$(python3 /usr/share/nimbus/desktop/shell/hearth-lockcheck); rc=$?
  [ $rc = 0 ] && ok "hearth-lockcheck (KWin's own environment, the greeter's test run shows Hearth's lock screen loading)" "$r" \
    || bad "hearth-lockcheck says it's not safe to lock" "$r"
  [ $rc = 0 ] || return 1

  # Your Autolock=true with Timeout=1 (a minute): KWin would lock by itself after a minute idle. It must not.
  echo "::notice title=Debian lock: idle::waiting 80 s with no input (your own kscreenlockerrc says Autolock=true, Timeout=1)"
  sleep 80
  active && bad "KWin locked by itself (your kscreenlockerrc's Autolock won over Hearth's)" "$(grep -iE 'lock|greet' "$REPO/shots/debian-kwin.log" | tail -5 | tr '\n' '|')" \
    || ok "KWin didn't lock by itself after 80 s idle, though your own kscreenlockerrc says Autolock=true, Timeout=1"
  gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.Lock >/dev/null 2>&1
  for i in $(seq 1 20); do active && break; sleep 0.5; done
  active && ok "locked by KWin's locker" || { bad "KWin's locker didn't lock" "$(grep -iE 'lock|greet' "$REPO/shots/debian-kwin.log" | tail -5 | tr '\n' '|')"; return 1; }
  sleep 8
  shot debian-locked
  gp=$(pgrep -u "$USER_NAME" -f kscreenlocker_greet | head -1)
  [ -n "$gp" ] && ok "KWin's greeter is up (pid $gp): $(tr '\0' ' ' < /proc/$gp/cmdline 2>/dev/null)" || bad "KWin's greeter isn't running" "$(tail -8 "$REPO/shots/debian-kwin.log" | tr '\n' '|')"
  echo "::notice title=Debian lock (greeter's environment)::$(tr '\0' '\n' < /proc/$gp/environ 2>/dev/null | grep -E '^(PLASMA_DEFAULT_SHELL|WAYLAND_SOCKET|QT_QPA_PLATFORM)=' | tr '\n' ' ')"

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
  pkill -u "$USER_NAME" kwin_wayland; pkill -u "$USER_NAME" -f hearth-type; pkill -u "$USER_NAME" dbus-daemon
  return $fail
}

case "${1:-}" in
  setup) setup ;;
  run) run ;;
  user) user ;;
  *) echo "kwin-debian-lock.sh setup|run"; exit 2 ;;
esac
