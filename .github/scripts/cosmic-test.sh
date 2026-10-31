#!/bin/bash
# Hearth COSMIC's test (cosmic-test.yml; docs/COSMIC.md), run as root inside Anatase's Fedora (a fedora:45 container,
# privileged, with the host's /dev so uinput devices show up):
#   1. COSMIC installed from desktop/cosmic/fedora-packages, exactly as os-bases/anatase/build.sh does, and how much it
#      adds; Hearth's COSMIC defaults made by hearth-cosmic-defaults, as at image build time.
#   2. COSMIC's compositor nested in a headless labwc (1920x1080), with its panel, wallpaper and settings daemon, in
#      the environment nimbus-session's run_cosmic gives it, and Hearth's companion (desk-cosmic.qml).
#   3. Screenshots: the desktop, Hearth's launcher, Quick Access, Hearth menu, Hearth Link and Storage, the keyboard,
#      COSMIC Files, Terminal and Edit; then the controller checks (cosmic-pad-drive.py) with a virtual pad.
# Results: shots/ (screenshots, logs, results.txt), ::error / ::notice annotations. Exit 1 when something failed.
set -u
cd /src
export LANG=C.UTF-8 LC_ALL=C.UTF-8
mkdir -p shots
FAILED=0
fail() { echo "::error title=$1::$2"; echo "FAIL $1: $2" >>shots/results.txt; FAILED=1; }
note() { echo "::notice title=$1::$2"; echo "ok $1: $2" >>shots/results.txt; }

# ------------------------------------------------------------------ 1. install
dnf -y -q install dnf5-plugins >/dev/null 2>&1
rpm -qa --qf '%{NAME}\n' | sort >/tmp/before
dnf -y -q install strace labwc grim wlr-randr wtype python3 python3-evdev dbus-daemon dbus-tools procps-ng ImageMagick git \
  mesa-dri-drivers mesa-libEGL mesa-libgbm google-noto-sans-fonts breeze-icon-theme hicolor-icon-theme xorg-x11-server-Xwayland \
  qt6-qtsvg qt6-qtdeclarative >/tmp/tools.log 2>&1 || fail "setup" "tools: $(tail -3 /tmp/tools.log | tr '\n' ' ')"
dnf -y copr enable errornointernet/quickshell >/dev/null 2>&1 && dnf -y -q install quickshell >/dev/null 2>&1 || fail "setup" "Quickshell couldn't be installed"
# Hearth's COSMIC QML must parse.
dnf -y -q install qt6-qtdeclarative-devel >/dev/null 2>&1 || true
QMLFORMAT=$(command -v qmlformat || ls /usr/lib64/qt6/bin/qmlformat 2>/dev/null | head -1)
for f in hosts/linux/desktop/shell/desk-cosmic.qml hosts/linux/desktop/shell/CosmicHub.qml hosts/linux/desktop/shell/Help.qml; do
  [ -n "$QMLFORMAT" ] && { "$QMLFORMAT" "$f" >/dev/null 2>/tmp/fmt.err || fail "QML doesn't parse" "$f: $(tr '\n' ' ' </tmp/fmt.err | cut -c1-500)"; }
done
rpm -qa --qf '%{NAME}\n' | sort >/tmp/tools-done
PK=hosts/linux/desktop/cosmic/fedora-packages
pkgs=$(grep -v '^#' $PK | grep -v '^-' | tr '\n' ' ')
ex=$(grep '^-' $PK | sed 's/^-/--exclude=/' | tr '\n' ' ')
# shellcheck disable=SC2086
if dnf -y install --setopt=install_weak_deps=False $ex $pkgs >/tmp/cosmic-install.log 2>&1; then
  rpm -qa --qf '%{NAME}\n' | sort >/tmp/after
  mb=$(for n in $(comm -13 /tmp/tools-done /tmp/after); do rpm -q --qf '%{SIZE}\n' "$n"; done | awk '{s+=$1} END {print int(s/1048576)}')
  top=$(for n in $(comm -13 /tmp/tools-done /tmp/after); do rpm -q --qf '%{SIZE} %{NAME}\n' "$n"; done | sort -rn | head -12 | awk '{printf "%s %dMB, ", $2, $1/1048576}')
  note "COSMIC's size" "COSMIC adds ${mb} MB installed on plain Fedora $(rpm -E %fedora) (Anatase already has some of it). Biggest: $top"
  echo "$mb" > shots/cosmic-size-mb.txt
  rpm -q cosmic-session cosmic-comp cosmic-files cosmic-term cosmic-edit >shots/cosmic-version.txt
  for x in cosmic-store cosmic-player cosmic-wallpapers cosmic-monitor; do rpm -q "$x" >/dev/null 2>&1 && fail "left out" "$x got installed"; done
  # As os-bases/anatase/build.sh does: Fedora's presets may switch COSMIC's login screen on; Hearth switches it off.
  systemctl disable cosmic-greeter.service greetd.service >/dev/null 2>&1 || true
  for u in cosmic-greeter.service greetd.service; do systemctl is-enabled "$u" 2>/dev/null | grep -qx enabled && fail "login screen" "$u is switched on"; done
else
  fail "COSMIC install" "$(grep -iE 'problem|nothing provides|requires|error' /tmp/cosmic-install.log | head -5 | tr '\n' ' ' | cut -c1-600)"
  note "cosmic-session needs" "$(dnf -q repoquery --requires cosmic-session 2>/dev/null | tr '\n' ' ' | cut -c1-1500)"
  exit 1
fi
grep -hE '^\s*\((modifiers|description)' /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults | grep -iE 'Tab|"q"|Maximize|Minimize|"g"|"Up"' | head -20 > shots/cosmic-shortcuts.txt

# Hearth's files, where the image puts them.
mkdir -p /usr/share/nimbus && ln -sfn /src/hosts/linux/desktop /usr/share/nimbus/desktop
cp -a os/system_files/usr/share/hearth-cosmic /usr/share/
cp os/system_files/usr/share/applications/hearth-cosmic-*.desktop os/system_files/usr/share/applications/hearth-gamemode.desktop /usr/share/applications/
install -Dm644 os/branding/wallpapers/hearth-fireside.jpg /usr/share/wallpapers/Hearth-fireside/contents/images/3840x2160.jpg
for d in branding/icons/png/*x*; do mkdir -p "/usr/share/icons/hicolor/$(basename "$d")/apps" && cp "$d"/*.png "/usr/share/icons/hicolor/$(basename "$d")/apps/"; done
mkdir -p /usr/share/icons/hicolor/scalable/apps && cp branding/icons/apps/*.svg /usr/share/icons/hicolor/scalable/apps/
gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true
printf '[Desktop Entry]\nType=Application\nName=Hearth\nExec=true\nIcon=hearth\n' > /usr/share/applications/nimbus.desktop
python3 hosts/linux/desktop/cosmic/hearth-cosmic-defaults /usr/share/cosmic /usr/share/hearth-cosmic/data/cosmic >shots/defaults.txt 2>&1 \
  || fail "defaults" "hearth-cosmic-defaults: $(tail -3 shots/defaults.txt | tr '\n' ' ')"
grep -q 'hearth-cosmic launcher' /usr/share/hearth-cosmic/data/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions \
  || fail "defaults" "Super doesn't open Hearth's launcher (system_actions has no Launcher line?)"
grep -q 'red: 0.0' /usr/share/hearth-cosmic/data/cosmic/com.system76.CosmicTheme.Dark/v1/background || fail "defaults" "the theme wasn't recoloured"
grep -rn 'red: 0.3882353' /usr/share/hearth-cosmic/data/cosmic/com.system76.CosmicTheme.Dark/v1/ | head -3 > /tmp/cyan.txt
[ -s /tmp/cyan.txt ] && fail "defaults" "COSMIC's cyan accent is still in the theme: $(head -c 300 /tmp/cyan.txt | tr '\n' ' ')"
cp -a /usr/share/hearth-cosmic/data/cosmic shots/hearth-cosmic-defaults

# ------------------------------------------------------------------ 2. COSMIC nested in a headless labwc
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Documents" "$HOME/Pictures" "$HOME/Music" && chmod 700 "$XDG_RUNTIME_DIR"
printf 'Hearth notes\nLine two\n' > "$HOME/Documents/notes.txt"; cp os/branding/wallpapers/hearth-fox.jpg "$HOME/Pictures/" 2>/dev/null
mkdir -p "$HOME/.cache/hearth"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
# The outer labwc: every window maximized and undecorated (COSMIC's window fills the 1920x1080 picture), no keybinds of
# its own (Super+Q and Alt+Tab go through to COSMIC).
mkdir -p /tmp/outer
cat > /tmp/outer/rc.xml <<'XML'
<?xml version="1.0"?>
<labwc_config>
  <core><decoration>client</decoration></core>
  <keyboard><keybind key="W-F12"><action name="None"/></keybind></keyboard>
  <windowRules><windowRule identifier="*" serverDecoration="no"><action name="Maximize"/></windowRule></windowRules>
</labwc_config>
XML
: > /tmp/outer/autostart
WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 WAYLAND_DISPLAY= labwc -C /tmp/outer >shots/outer.log 2>&1 &
for i in $(seq 1 30); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 1; done
OUTER=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
[ -n "$OUTER" ] || { fail "labwc" "the outer compositor didn't start: $(tail -3 shots/outer.log | tr '\n' ' ')"; exit 1; }
WAYLAND_DISPLAY=$OUTER wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 || true
export OUTER_WAYLAND=$OUTER
printf '#!/bin/sh\nWAYLAND_DISPLAY=%s timeout 20 grim "$1" 2>>/src/shots/shot-errors.txt || echo "no screenshot $1" >>/src/shots/shot-errors.txt\n' "$OUTER" > /tmp/shot
chmod +x /tmp/shot
shot() { /tmp/shot "$1"; }

# What nimbus-session's run_cosmic gives COSMIC (and start-cosmic then adds).
export XDG_DATA_DIRS="/usr/share/hearth-cosmic/data:/usr/local/share:/usr/share" XDG_CONFIG_DIRS="/usr/share/hearth-cosmic/xdg:/etc/xdg"
export HEARTH_DESKTOP=cosmic DESKTOP_SESSION=cosmic XDG_CURRENT_DESKTOP=COSMIC XDG_SESSION_DESKTOP=COSMIC XDG_SESSION_TYPE=wayland
export LIBGL_ALWAYS_SOFTWARE=1 RUST_LOG=warn NIMBUS_SHARE=/src/hosts/linux QT_QUICK_BACKEND=software
before=$(ls "$XDG_RUNTIME_DIR" | grep -E '^wayland-[0-9]+$' | sort)
RUST_LOG=info WAYLAND_DISPLAY=$OUTER cosmic-comp >shots/cosmic-comp.log 2>&1 &
COMP=$!
INNER=""
for i in $(seq 1 40); do
  now=$(ls "$XDG_RUNTIME_DIR" | grep -E '^wayland-[0-9]+$' | sort)
  INNER=$(comm -13 <(echo "$before") <(echo "$now") | head -1)
  [ -n "$INNER" ] && break
  kill -0 $COMP 2>/dev/null || break
  sleep 1
done
if [ -z "$INNER" ]; then
  fail "cosmic-comp" "COSMIC's compositor didn't start nested: $(grep -iE 'error|panic|failed' shots/cosmic-comp.log | tail -5 | tr '\n' ' ' | cut -c1-700)"
  exit 1
fi
export WAYLAND_DISPLAY=$INNER
note "cosmic-comp" "nested on $INNER"
cosmic-settings-daemon >shots/cosmic-settings-daemon.log 2>&1 &
sleep 2
cosmic-bg >shots/cosmic-bg.log 2>&1 &
cosmic-panel >shots/cosmic-panel.log 2>&1 &
sleep 8
# The companion, the way its autostart entry starts it.
autostart_exec=$(sed -n 's/^Exec=//p' /usr/share/hearth-cosmic/xdg/autostart/hearth-cosmic.desktop)
sh -c "$autostart_exec" >shots/companion.log 2>&1 &
sleep 10
pgrep -f desk-cosmic.qml >/dev/null || fail "companion" "desk-cosmic.qml didn't start: $(grep -iE 'error|warn' shots/companion.log | head -6 | tr '\n' ' ' | cut -c1-700)"
QS="timeout -k 2 10 quickshell ipc -p /src/hosts/linux/desktop/shell/desk-cosmic.qml call hearth"
RESTARTS=0
# Through hearth-cosmic, as the panel's entries and Super do: it starts the companion again if it has ended.
ipc() {
  pgrep -f 'quickshell -p .*desk-cosmic.qml' >/dev/null || { RESTARTS=$((RESTARTS + 1)); echo "companion had ended before: $*" >>shots/shot-errors.txt; }
  timeout -k 3 25 hosts/linux/desktop/cosmic/hearth-cosmic "$@" >/dev/null 2>&1 || echo "hearth-cosmic $* failed" >>shots/shot-errors.txt
  sleep 3
}

# ------------------------------------------------------------------ 3. screenshots
echo "$(date +%T) screenshots" >>shots/progress.txt
shot shots/cosmic-1-desktop.png
ipc quick; shot shots/cosmic-2-quick-access.png; ipc close
ipc menu; shot shots/cosmic-3-hearth-menu.png; ipc close
ipc launcher; shot shots/cosmic-4-launcher.png; ipc close
ipc games; shot shots/cosmic-4b-launcher-games.png; ipc close
ipc link; shot shots/cosmic-5-hearth-link.png; ipc close
ipc storage; shot shots/cosmic-6-hearth-storage.png; ipc close
ipc keyboard; shot shots/cosmic-7-keyboard.png; ipc keyboard
ipc help; shot shots/cosmic-8-controls.png; ipc close
[ "$RESTARTS" -gt 0 ] && echo "::warning title=companion::the companion ended $RESTARTS time(s) while its windows were opened (see companion.log, cosmic-comp.log)"
# Which theme files COSMIC's apps really read (the theme check below).
timeout -k 3 10 strace -f -e trace=openat,statx,newfstatat,access -o shots/strace-term.txt cosmic-term >/dev/null 2>&1
grep -E 'CosmicTheme|CosmicTk' shots/strace-term.txt | grep -v ENOENT | head -20 > shots/theme-files-read.txt
grep -E 'CosmicTheme.Dark/v1' shots/strace-term.txt | head -40 > shots/theme-lookups.txt
echo "::notice title=theme files COSMIC reads::$(cut -c1-200 shots/theme-files-read.txt | tr '\n' '|' | cut -c1-2500)"
find "$HOME/.config/cosmic" "$HOME/.local/state/cosmic" -maxdepth 3 2>/dev/null | head -60 > shots/user-cosmic-config.txt
# Errors from Hearth's QML in the companion's log fail the run.
bad=$(grep -E "qml|QML" shots/companion.log | grep -iE "error|TypeError|ReferenceError|is not a type|not defined|Cannot assign|failed to load" | sort -u | head -10)
[ -n "$bad" ] && fail "companion QML" "$(printf '%s' "$bad" | cut -c1-300 | tr '\n' '|')"
grep -iE "hearth-pad" shots/companion.log | head -5 > shots/pad-messages.txt
pad_problem=$($QS problem 2>/dev/null)
[ -n "$pad_problem" ] && fail "hearth-pad" "the companion says: $pad_problem"

APPID="$QS app"
export APPID SHOT=/tmp/shot
appshot() {   # appshot NAME CMD...: start an app, picture it, drive it with the pad, close it
  name=$1; shift
  ipc close
  "$@" >"shots/$name.log" 2>&1 &
  apid=$!
  sleep 12
  echo "$(date +%T) $name started" >>shots/progress.txt
  shot "shots/cosmic-$name-1.png"
  note "$name app id" "the companion sees: $($QS app 2>/dev/null)"
  timeout -k 5 150 python3 .github/scripts/cosmic-pad-drive.py "$name" shots >"shots/pad-$name.log" 2>&1 || fail "controller in $name" "$(grep FAIL "shots/pad-$name.txt" 2>/dev/null | head -4 | tr '\n' '|' | cut -c1-700)"
  kill $apid 2>/dev/null; sleep 2; kill -9 $apid 2>/dev/null
}
appshot files cosmic-files "$HOME/Documents"
appshot term cosmic-term
appshot edit cosmic-edit "$HOME/Documents/notes.txt"
ipc close
timeout -k 5 150 python3 .github/scripts/cosmic-pad-drive.py desk shots >shots/pad-desk.log 2>&1 || fail "controller on the desktop" "$(grep FAIL shots/pad-desk.txt 2>/dev/null | head -4 | tr '\n' '|' | cut -c1-700)"

cp "$HOME/.local/share/nimbus/cosmic-companion.log" shots/ 2>/dev/null
grep -iE "protocol|error [0-9]|broke|wl_" shots/cosmic-companion.log shots/companion.log 2>/dev/null | sort | uniq -c | head -8 > shots/companion-errors.txt
[ -s shots/companion-errors.txt ] && echo "::warning title=companion's Wayland errors::$(cut -c1-300 shots/companion-errors.txt | tr '\n' '|')"
# Why the companion ends when one of its windows closes (seen in CI): its Wayland conversation around Quick Access
# opening and closing.
pkill -f 'desk-cosmic.qml' ; sleep 2
QT_IM_MODULE=compose WAYLAND_DEBUG=client quickshell -p /src/hosts/linux/desktop/shell/desk-cosmic.qml >/tmp/wl-debug.log 2>&1 &
sleep 10
timeout 10 quickshell ipc -p /src/hosts/linux/desktop/shell/desk-cosmic.qml call hearth quick >/dev/null 2>&1; sleep 3
timeout 10 quickshell ipc -p /src/hosts/linux/desktop/shell/desk-cosmic.qml call hearth quick >/dev/null 2>&1; sleep 4
pgrep -f desk-cosmic.qml >/dev/null && echo "companion still running after Quick Access opened and closed" > shots/wl-debug-result.txt || echo "companion ENDED after Quick Access opened and closed" > shots/wl-debug-result.txt
grep -nE "error|wl_display|delete_id|zwlr_layer_surface|destroy" /tmp/wl-debug.log | tail -60 > shots/wl-debug-tail.txt
tail -40 /tmp/wl-debug.log >> shots/wl-debug-tail.txt
grep -iE "\.error\(" /tmp/wl-debug.log | head -5 > shots/wl-protocol-errors.txt
echo "::notice title=Wayland check::$(cat shots/wl-debug-result.txt) $(head -c 600 shots/wl-protocol-errors.txt | tr '\n' '|')"
for f in shots/pad-*.txt; do [ -f "$f" ] && echo "::notice title=$(basename "$f" .txt)::$(head -c 3000 "$f" | sed 's/%/%25/g' | tr '\n' '|')"; done
ls shots/*.png >/dev/null 2>&1 || fail "screenshots" "none were taken: $(head -c 500 shots/shot-errors.txt 2>/dev/null | tr '\n' '|')"
pkill -f desk-cosmic.qml; pkill cosmic-panel; pkill cosmic-bg; pkill cosmic-settings; kill $COMP 2>/dev/null; pkill labwc; pkill dbus-daemon
exit $FAILED
