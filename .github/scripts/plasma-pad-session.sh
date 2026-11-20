#!/bin/bash
# The real KDE Plasma session for plasma-pad.yml: KDE's KWin on a virtual 1920x1080 screen (its input method from the
# system's /etc/xdg/kwinrc, as the Fedora image has it, unless the test gave the person a ~/.config/kwinrc of their
# own), kactivitymanagerd, kglobalacceld and plasmashell with KDE's panel.
#   source plasma-pad-session.sh; session_start NAME; ...; session_stop
# Logs in shots/kwin-NAME.log (and shots/kwin.log: the one the running session writes, which the e2e reads).
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home USER=chris
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.config" "$HOME/.local/share/nimbus" shots && chmod 700 "$XDG_RUNTIME_DIR"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
[ -S "$XDG_RUNTIME_DIR/bus" ] || dbus-daemon --session --fork --address="$DBUS_SESSION_BUS_ADDRESS"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1 HEARTH_PLASMA_PANEL_CI=1
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
# KWin scripts' print() in its log (the e2e reads where the keyboard's overlay is).
export QT_LOGGING_RULES="kwin_scripting.debug=true;js.debug=true"
[ -x /tmp/kwin_wayland ] || cp "$(command -v kwin_wayland)" /tmp/kwin_wayland

js() { timeout 15 dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript "string:$1" 2>&1; }

session_start() {
  local name=$1
  pgrep -x pipewire >/dev/null || for p in pipewire wireplumber; do command -v $p >/dev/null && { $p >"shots/$p.log" 2>&1 & }; done
  unset WAYLAND_DISPLAY
  /tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >shots/kwin.log 2>&1 &
  for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
  if ! [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ]; then
    echo "::error title=KWin ($name)::didn't start: $(tail -5 shots/kwin.log | tr '\n' '|')"; return 1
  fi
  export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
  for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"shots/kamd-$name.log" 2>&1 & break; }; done
  for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"shots/kglobalaccel-$name.log" 2>&1 & break; }; done
  plasmashell >"shots/plasmashell-$name.log" 2>&1 &
  local up=0
  for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && { up=1; break; }; sleep 1; done
  sleep 6
  js 'var d=desktops();for(var i=0;i<d.length;i++){d[i].wallpaperPlugin="org.kde.image";d[i].currentConfigGroup=["Wallpaper","org.kde.image","General"];d[i].writeConfig("Image","file:///usr/share/backgrounds/hearth/hearth-fireside.jpg");}' >/dev/null
  sleep 2
  [ "$up" = 1 ] || { echo "::error title=Plasma ($name)::plasmashell didn't come up"; return 1; }
  return 0
}

session_stop() {
  local name=$1
  pkill -f hearth-pad-host; pkill -f desktop/shell/hearth-pad; pkill -f padwin.py; pkill -x plasmashell
  pkill -f kactivitymanagerd; pkill -f kglobalacceld; pkill -f /tmp/kwin_wayland
  sleep 3
  pkill -9 -f /tmp/kwin_wayland; pkill -9 -x plasmashell; pkill -9 -f plasma-keyboard; pkill -9 -f hearth-kwin-im
  sleep 1
  cp shots/kwin.log "shots/kwin-$name.log" 2>/dev/null
  rm -f "$XDG_RUNTIME_DIR"/wayland-hearth*
  unset WAYLAND_DISPLAY
}

