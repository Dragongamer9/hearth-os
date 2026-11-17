#!/bin/bash
# KWin for .github/workflows/kwin-input.yml, with what Hearth's KWin session gives it (here only what the checks need).
#   source .github/scripts/kwin-ci.sh env              the environment (every step sources it)
#   bash .github/scripts/kwin-ci.sh start nested       a headless labwc holding KWin as a window (uinput keyboards
#                                                      reach KWin through labwc; grim takes pictures from labwc)
#   bash .github/scripts/kwin-ci.sh start virtual      KWin on its own virtual screen (pictures: KWin's own)
#   bash .github/scripts/kwin-ci.sh stop
# Logs go to shots/.
if [ "${1:-}" = env ] || [ "${BASH_SOURCE[0]}" != "$0" ]; then
  export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
  export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  export HEARTH_WM=kwin XDG_CURRENT_DESKTOP=Hearth:KDE XDG_SESSION_DESKTOP=Hearth XDG_SESSION_TYPE=wayland
  export XDG_CONFIG_DIRS="$PWD/hosts/linux/desktop/kwin/xdg:/tmp/kwin-ci-xdg:/etc/xdg"
  export QT_FORCE_STDERR_LOGGING=1 LIBGL_ALWAYS_SOFTWARE=1 QT_QUICK_BACKEND=software QT_QPA_PLATFORM=wayland GDK_BACKEND=wayland
  export WAYLAND_DISPLAY=wayland-hearth
  [ -f /tmp/kwin-ci-outer ] && export OUTER="$(cat /tmp/kwin-ci-outer)"
  return 0 2>/dev/null || exit 0
fi

set +e
cd "$(dirname "$0")/../.." || exit 1
source .github/scripts/kwin-ci.sh env
mkdir -p shots "$XDG_RUNTIME_DIR" "$HOME" && chmod 700 "$XDG_RUNTIME_DIR"

start() {
  mode=$1
  if ! [ -S "$XDG_RUNTIME_DIR/bus" ]; then
    dbus-daemon --session --fork --address="$DBUS_SESSION_BUS_ADDRESS"
  fi
  mkdir -p /tmp/kwin-ci-xdg
  # The session branch's kwinrc line for Hearth's input method.
  printf '[Wayland]\nInputMethod[$e]=/usr/share/applications/hearth-kwin-im.desktop\nVirtualKeyboardEnabled=true\n' > /tmp/kwin-ci-xdg/kwinrc
  command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 --noincremental >/dev/null 2>&1
  pgrep -x pipewire >/dev/null || { pipewire >shots/pipewire.log 2>&1 & sleep 1; wireplumber >shots/wireplumber.log 2>&1 & sleep 2; }
  cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
  rm -f /tmp/kwin-ci-outer
  if [ "$mode" = nested ]; then
    modprobe uinput 2>/dev/null
    for u in /usr/lib/systemd/systemd-udevd /lib/systemd/systemd-udevd; do [ -x "$u" ] && { pgrep -f systemd-udevd >/dev/null || "$u" --daemon; break; }; done
    udevadm trigger --action=add 2>/dev/null; udevadm settle -t 10 2>/dev/null
    mkdir -p "$HOME/.config/labwc"
    printf '%s\n' '<labwc_config>' \
      '<keyboard><keybind key="W-F12"><action name="Execute" command="true"/></keybind></keyboard>' \
      '<windowRules><windowRule identifier="*" serverDecoration="no"><action name="MoveTo" x="0" y="0"/></windowRule></windowRules>' \
      '</labwc_config>' > "$HOME/.config/labwc/rc.xml"
    ( unset HEARTH_WM XDG_CONFIG_DIRS WAYLAND_DISPLAY QT_QPA_PLATFORM GDK_BACKEND
      WLR_BACKENDS=headless,libinput LIBSEAT_BACKEND=noop WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
        labwc -d >shots/outer.log 2>&1 & )
    for i in $(seq 1 20); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 1; done
    OUTER=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
    [ -n "$OUTER" ] || { echo "::error title=labwc::didn't start: $(tail -5 shots/outer.log | tr '\n' '|' | cut -c1-500)"; return 1; }
    echo "$OUTER" > /tmp/kwin-ci-outer
    WAYLAND_DISPLAY=$OUTER wlr-randr --output HEADLESS-1 --custom-mode 1920x1080
    # A keyboard before KWin's window appears: labwc only tells a window about keyboards that were there when it got
    # the focus (on a real PC KWin reads every keyboard itself). hearth-type's service is that keyboard.
    python3 /usr/share/nimbus/desktop/shell/hearth-type --serve >shots/hearth-type-service.log 2>&1 &
    sleep 3
    WAYLAND_DISPLAY=$OUTER /tmp/kwin_wayland --wayland-display "$OUTER" --width 1920 --height 1080 --socket wayland-hearth >shots/kwin-$mode.log 2>&1 &
  else
    /tmp/kwin_wayland --virtual --width 1920 --height 1080 --socket wayland-hearth >shots/kwin-$mode.log 2>&1 &
  fi
  for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
  [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { echo "::error title=KWin ($mode)::didn't start: $(tail -5 shots/kwin-$mode.log | tr '\n' '|' | cut -c1-600)"; return 1; }
  sleep 4
  # D-Bus starts hearth-portal (and xdg-permission-store) with this environment, as the session does with
  # dbus-update-activation-environment.
  dbus-update-activation-environment --all 2>/dev/null || dbus-update-activation-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HEARTH_WM \
    XDG_CONFIG_DIRS XDG_RUNTIME_DIR HOME QT_QPA_PLATFORM GDK_BACKEND LIBGL_ALWAYS_SOFTWARE QT_QUICK_BACKEND
  XDP=""; for x in /usr/libexec/xdg-desktop-portal /usr/lib/xdg-desktop-portal; do [ -x "$x" ] && { XDP="$x"; break; }; done
  "$XDP" -r -v > shots/xdg-desktop-portal-$mode.log 2>&1 &
  sleep 2
  echo "::notice title=KWin ($mode) started::$(grep -iE 'opengl|qpainter|compositing|egl|renderer|llvmpipe' shots/kwin-$mode.log | head -6 | tr '\n' '|' | cut -c1-900)"
}

stop() {
  pkill -f kscreenlocker_greet; pkill quickshell; pkill -f hearth-kwin; pkill -f 'hearth-type --serve'; pkill -f hearth-portal
  pkill -f xdg-desktop-portal; pkill -f /tmp/kwin_wayland; pkill labwc
  sleep 2; pkill -9 -f /tmp/kwin_wayland; pkill -9 labwc
  rm -f "$XDG_RUNTIME_DIR"/wayland-hearth* /tmp/kwin-ci-outer
  true
}

case "$1" in
  start) start "$2" ;;
  stop) stop ;;
esac
