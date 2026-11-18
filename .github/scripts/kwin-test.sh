#!/bin/bash
# Mantel on KWin, for real (kwin-test.yml; docs/DESKTOPS.md "Mantel on KWin"): nimbus-session itself starts Desktop Mode
# four times on a 1920x1080 virtual screen (KWin's own, --virtual, with a vgem render device so it composites; pictures
# through KWin's screenshot API. labwc's fallback runs headless; grim takes its pictures):
#   A  KWin, with 1.13's KDE Plasma still picked: the "KDE Plasma and GNOME were retired" card     kwin-1-retired-card
#   B  KWin: Mantel's bar up (layer shell), the desktop and Settings › System › Your desktop         kwin-2-desktop, kwin-3-your-desktop
#      Settings › Keyboard on KWin: keyboard.json's layouts are KWin's at the start, and a layout set in Settings is
#      KWin's active one right away (org.kde.keyboard); the KWin-only rows                            kwin-9-keyboard-kwin
#   C  KWin ending right away: the safe start opens Mantel on labwc, with its card                  kwin-4-fallback-card
#   D  the same on the next sign-in: twice in a row, labwc until KWin is turned back on              kwin-5-twice-card, kwin-6-your-desktop-paused
#   E  KWin with your own kscreenlockerrc / kwinrc from KDE Plasma (Autolock, LockOnResume, an input method): Hearth's
#      fixed keys win and the card says which (once); Settings › Keyboard's "Bring back KDE's own shortcuts"
#                                                                                                    kwin-7-kde-settings-card, kwin-8-keyboard-kde-keys
# Failures are ::error:: annotations and shots/FAILED (logs can't always be read here).
set -u +e +o pipefail
mkdir -p shots
SRC=$PWD
fail() { echo "::error title=$1::$2"; touch shots/FAILED; }
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" /tmp/fakebin && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1
export NIMBUS_SHARE="$SRC/hosts/linux" NIMBUS_APP=/nonexistent
chmod +x hosts/linux/desktop/labwc/autostart hosts/linux/desktop/kwin/autostart hosts/linux/desktop/theme/apply.sh
# The launcher's Games: what Hearth listed last time (no Hearth app here).
mkdir -p "$HOME/.cache/hearth" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/share/nimbus/desktop.json" '{}' >/dev/null   # there from the start (the shell watches it)
# kwin_wayland carries a file capability (CAP_SYS_NICE) a container can't grant: a plain copy runs. The stand-in on
# PATH is that copy, or (C, D) a KWin that ends right away, as a broken one would.
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
cat > /tmp/fakebin/kwin_wayland <<'SH'
#!/bin/sh
[ -e /tmp/kwin-crash ] && { echo "kwin stand-in: ending right away (the safe start's test)"; exit 1; }
exec /tmp/kwin_wayland "$@"
SH
chmod +x /tmp/fakebin/kwin_wayland
export PATH="/tmp/fakebin:$PATH"

# A render device for KWin (without one it doesn't composite: no frames, and its screenshots crash it).
modprobe vgem 2>/dev/null || echo "::warning title=KWin::no vgem module"
echo "::notice title=render devices::$(ls -l /dev/dri 2>&1 | tr '\n' '|' | cut -c1-300)"
# KWin on its own virtual screen; labwc (the fallback) headless, pixman.
export WAYLAND_DISPLAY= WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1
export HEARTH_KWIN_ARGS="--virtual --width 1920 --height 1080 --socket wayland-hearth"

cat > /tmp/shot.py <<'PY'
import dbus, os, sys
from PIL import Image
bus = dbus.SessionBus()
i = dbus.Interface(bus.get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
res = None
for how in ('CaptureActiveScreen', 'CaptureWorkspace'):
    try:
        res = getattr(i, how)({'native-resolution': True}, dbus.types.UnixFd(w), timeout=15)
        break
    except dbus.exceptions.DBusException as e:
        print(how, e, file=sys.stderr)
os.close(w)
if res is None:
    sys.exit(1)
data = b''
while True:
    c = os.read(r, 1 << 20)
    if not c:
        break
    data += c
W, H, S = int(res['width']), int(res['height']), int(res['stride'])
Image.frombuffer('RGBA', (W, H), data, 'raw', 'BGRA', S, 1).convert('RGB').save(sys.argv[1])
PY
# A picture of the whole 1920x1080 screen.
# (KWin: its screenshot API; labwc: grim on its display.)
shot() {
  if [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && pgrep -f /tmp/kwin_wayland >/dev/null; then
    timeout 20 python3 /tmp/shot.py "shots/$1.png" 2>>shots/shot-errors.txt || echo "no screenshot $1" >>shots/shot-errors.txt
  else
    d=$(labwc_display); WAYLAND_DISPLAY=$d timeout 20 grim "shots/$1.png" 2>>shots/shot-errors.txt || echo "no screenshot $1 ($d)" >>shots/shot-errors.txt
  fi
  sleep 1
}
LOG="$HOME/.local/share/nimbus/session.log"
CONF="$HOME/.config/nimbus/session.conf"
STATE="$HOME/.local/state/hearth"
SPID=""
start_session() {   # $1: a name for its output
  rm -f "$XDG_RUNTIME_DIR/nimbus-next" "$XDG_RUNTIME_DIR/hearth-shell-up"
  echo "=== session $1" >>shots/session-out.log
  sh hosts/linux/session/nimbus-session >>shots/session-out.log 2>&1 &
  SPID=$!
}
wait_up() {   # Mantel says it's up, or $1 seconds
  for i in $(seq 1 "$1"); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && return 0; sleep 1; done; return 1
}
end_session() {
  echo logout > "$XDG_RUNTIME_DIR/nimbus-next"
  pkill -TERM -f /tmp/kwin_wayland; pkill -TERM -x labwc
  for i in $(seq 1 15); do kill -0 "$SPID" 2>/dev/null || break; sleep 1; done
  kill -KILL "$SPID" 2>/dev/null; pkill -KILL -f /tmp/kwin_wayland; pkill -TERM quickshell; sleep 2
  pkill -KILL -x labwc; sleep 1
}
ipc() { WAYLAND_DISPLAY="${1}" quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" call "${@:2}" || echo "ipc ${*:2} failed"; sleep 3; }
labwc_display() { basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)"; }
labwc_size() { d=$(labwc_display); [ -n "$d" ] && WAYLAND_DISPLAY=$d wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1; }

# Pictures without the two one-time cards that aren't what's checked here: "Pick your browser" (1.12) and the lock
# guard's "The lock screen is off" (this runner has no lock screen package; test_kwin_session.py checks that card).
python3 hosts/linux/desktop/shell/hearth-browser asked >/dev/null 2>&1 || true
mkdir -p "$STATE" && : >"$STATE/kwin-lock-off-told"
# ---- A: KWin, with 1.13's KDE Plasma still picked.
printf 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP=kde\n' > "$CONF"
cp "$CONF" /tmp/conf-before
start_session A
if wait_up 90; then
  echo "::notice title=KWin::Mantel says it's up under KWin ($(grep -o 'kwin: Mantel up after [0-9]*s' "$LOG" | tail -1))"
else
  fail "KWin" "Mantel didn't come up under KWin: $(tail -8 "$LOG" | tr '\n' '|' | cut -c1-500) kwin.log: $(tail -6 "$HOME/.local/share/nimbus/kwin.log" 2>/dev/null | tr '\n' '|' | cut -c1-500)"
fi
grep -q 'desktop mode: Mantel on KWin' "$LOG" || fail session "nimbus-session didn't start KWin: $(tail -6 "$LOG" | tr '\n' '|' | cut -c1-400)"
# Layer shell: KWin offers it (Mantel's bar, dock and cards are layer surfaces).
if command -v wayland-info >/dev/null; then
  WAYLAND_DISPLAY=wayland-hearth timeout 10 wayland-info >shots/wayland-info.txt 2>&1
  grep -q zwlr_layer_shell_v1 shots/wayland-info.txt && echo "::notice title=KWin::layer shell is there (zwlr_layer_shell_v1)" || fail "layer shell" "KWin doesn't offer zwlr_layer_shell_v1"
fi
sleep 8   # DesktopNotice shows 4 s after the shell starts
shot kwin-1-retired-card
cmp -s "$CONF" /tmp/conf-before || fail "retired" "session.conf changed: $(tr '\n' '|' < "$CONF")"
grep -q '^kde retired' "$STATE/desktop-notice.shown" 2>/dev/null || fail "retired" "the retired notice wasn't shown (state: $(cat "$STATE"/desktop-notice* 2>/dev/null | tr '\n' '|'))"
end_session

# ---- B: KWin: the desktop and Settings › System › Your desktop.
printf 'NIMBUS_START_MODE=desktop\n' > "$CONF"
# Settings › Keyboard as someone saved it on labwc: two layouts, faster repeat, Num Lock on, a compose key.
printf '%s' '{"layouts": [{"layout": "de", "variant": ""}, {"layout": "fr", "variant": ""}], "switch": "grp:alt_shift_toggle", "delay": 300, "rate": 40, "numlock": "on", "compose": "compose:ralt", "switchMode": "Window"}' > "$HOME/.config/nimbus/keyboard.json.tmp" && mv -f "$HOME/.config/nimbus/keyboard.json.tmp" "$HOME/.config/nimbus/keyboard.json"
# Hearth's own apps as the image has them (the owner's ROG Xbox Ally, Nov 17: Hearth Settings, Logs and the rest
# were missing from Mantel's menu under KWin): hearth-app-kinds' copies say NotShowIn=KDE;GNOME; (own_apps), and KWin's
# XDG_CURRENT_DESKTOP is Hearth:Nimbus:KDE. They must still be listed.
install -Dm755 os/system_files/usr/libexec/hearth-app-kinds /usr/libexec/hearth-app-kinds
install -Dm644 os/system_files/usr/share/hearth/app-kinds.json /usr/share/hearth/app-kinds.json
mkdir -p /usr/share/hearth/app-kinds/applications /usr/share/applications
for a in "hearth-settings:Hearth Settings" "hearth-logs:Logs"; do
  printf '[Desktop Entry]\nType=Application\nName=%s\nExec=/bin/true\nIcon=%s\n' "${a#*:}" "${a%%:*}" > "/usr/share/applications/${a%%:*}.desktop"
  printf '[Desktop Entry]\nType=Application\nName=%s\nExec=/bin/true\nIcon=%s\nNotShowIn=KDE;GNOME;\nX-Hearth-App-Kinds=true\n' "${a#*:}" "${a%%:*}" > "/usr/share/hearth/app-kinds/applications/${a%%:*}.desktop"
done
start_session B
wait_up 90 || fail "KWin" "Mantel didn't come up under KWin the second time"
sleep 10
shot kwin-2-desktop
listed=$(WAYLAND_DISPLAY=wayland-hearth quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" call launcher listed 2>/dev/null)
for a in hearth-settings hearth-logs; do
  printf '%s\n' "$listed" | grep -qx "$a" && echo "::notice title=Hearth's apps under KWin::$a is in Mantel's menu" \
    || fail "Hearth's apps under KWin" "$a isn't in Mantel's menu under KWin (hidden: $(XDG_CURRENT_DESKTOP=Hearth:Nimbus:KDE python3 /usr/libexec/hearth-app-kinds hidden | tr '\n' ' ' | cut -c1-300); listed: $(printf '%s' "$listed" | grep -c .) apps)"
done
# 1.14's freeze: Mantel at rest under KWin, and after a search left in the closed menu (shell-idle-cpu.sh).
dnf -y install gdb >/dev/null 2>&1 || true
SH="$NIMBUS_SHARE/desktop/shell" bash .github/scripts/shell-idle-cpu.sh shots/idle kwin wayland-hearth
grep -iE 'layer.?shell.*(not|fail|unsupported)|wlr_layer.*(not|fail)' "$XDG_RUNTIME_DIR/nimbus-shell.log" | head -3 | while read -r l; do fail "layer shell" "$(echo "$l" | cut -c1-300)"; done
ipc wayland-hearth settings page your-desktop; sleep 3
shot kwin-3-your-desktop
ipc wayland-hearth settings close
python3 hosts/linux/desktop/shell/hearth-your-desktop status > shots/status-B.json
# Keyboard: KWin started with keyboard.json in its kxkbrc / kcminputrc (input-watch, in the background).
kbd_layouts() { gdbus call --session --dest org.kde.keyboard --object-path /Layouts --method org.kde.KeyboardLayouts.getLayoutsList 2>&1 | tr '\n' ' ' | cut -c1-300; }
kbd_active() {   # the short name of KWin's active layout
  i=$(gdbus call --session --dest org.kde.keyboard --object-path /Layouts --method org.kde.KeyboardLayouts.getLayout 2>/dev/null | sed -n 's/.*uint32 \([0-9]*\).*/\1/p')
  gdbus call --session --dest org.kde.keyboard --object-path /Layouts --method org.kde.KeyboardLayouts.getLayoutsList 2>/dev/null \
    | python3 -c 'import re,sys; l=re.findall(r"\(\x27([^\x27]*)\x27, \x27[^\x27]*\x27, \x27[^\x27]*\x27\)", sys.stdin.read()); i=int(sys.argv[1] or -1); print(l[i] if 0 <= i < len(l) else "?")' "$i"
}
for i in $(seq 1 15); do grep -q '^LayoutList=de,fr$' "$HOME/.config/kxkbrc" 2>/dev/null && break; sleep 1; done
cp "$HOME/.config/kxkbrc" shots/kxkbrc-B.txt 2>/dev/null; cp "$HOME/.config/kcminputrc" shots/kcminputrc-B.txt 2>/dev/null
grep -q '^LayoutList=de,fr$' "$HOME/.config/kxkbrc" 2>/dev/null || fail "keyboard" "KWin's kxkbrc didn't get your layouts at the start: $(tr '\n' '|' < "$HOME/.config/kxkbrc" 2>/dev/null | cut -c1-300)"
grep -q '^Options=grp:alt_shift_toggle,compose:ralt$' "$HOME/.config/kxkbrc" 2>/dev/null || fail "keyboard" "kxkbrc's Options: $(grep Options "$HOME/.config/kxkbrc" 2>/dev/null)"
grep -q '^SwitchMode=Window$' "$HOME/.config/kxkbrc" 2>/dev/null || fail "keyboard" "kxkbrc has no SwitchMode=Window"
for l in RepeatDelay=300 RepeatRate=40 NumLock=0; do grep -q "^$l\$" "$HOME/.config/kcminputrc" 2>/dev/null || fail "keyboard" "kcminputrc has no $l: $(tr '\n' '|' < "$HOME/.config/kcminputrc" 2>/dev/null | cut -c1-300)"; done
sleep 2
a=$(kbd_active); echo "::notice title=keyboard at the start::KWin's layouts: $(kbd_layouts); active: $a"
[ "$a" = de ] || fail "keyboard" "KWin's active layout at the start is '$a', not de (KWin: $(kbd_layouts))"
# Set in Settings (what SetKeyboard.qml sends): French first. KWin's active layout is French right away.
m=$(printf '%s' '{"layouts": [{"layout": "fr", "variant": ""}, {"layout": "de", "variant": ""}], "switch": "grp:alt_shift_toggle", "delay": 300, "rate": 40}' \
  | HEARTH_WM=kwin python3 hosts/linux/desktop/shell/hearth-system keyboard-set 2>&1 | tail -1)
echo "::notice title=keyboard-set on KWin::$m"
echo "$m" | grep -q '"reloaded": true' || fail "keyboard" "keyboard-set on KWin didn't say KWin took it: $(echo "$m" | cut -c1-300)"
sleep 2
a=$(kbd_active); echo "::notice title=keyboard after Settings::KWin's layouts: $(kbd_layouts); active: $a"
[ "$a" = fr ] || fail "keyboard" "the layout set in Settings (fr) isn't KWin's active one: '$a' (KWin: $(kbd_layouts))"
grep -q '"numlock": "on"' "$HOME/.config/nimbus/keyboard.json" || fail "keyboard" "keyboard.json lost Num Lock (merge-on-write): $(head -c 300 "$HOME/.config/nimbus/keyboard.json")"
command -v wayland-info >/dev/null && echo "::notice title=keyboard repeat (wayland-info)::$(WAYLAND_DISPLAY=wayland-hearth timeout 10 wayland-info 2>/dev/null | grep -i 'repeat' | head -2 | tr '\n\t' '| ')"
ipc wayland-hearth settings page keyboard; sleep 4
shot kwin-9-keyboard-kwin
ipc wayland-hearth settings close
# ---- Wave 3 under KWin (1.14's Desktop Mode is Mantel on KWin): the search bar, Activities, the App store's Drivers.
qk() { WAYLAND_DISPLAY=wayland-hearth quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" call "$@" 2>/dev/null | grep -v WARN | tail -n1; }
sb_shown() { qk searchbar state | python3 -c 'import json,sys; print(json.load(sys.stdin)["shown"])' 2>/dev/null; }
ipc wayland-hearth searchbar open; sleep 2
[ "$(sb_shown)" = True ] || fail "search bar (KWin)" "it didn't open under KWin: $(qk searchbar state | head -c 300)"
shot kwin-w3-search-empty
ipc wayland-hearth searchbar type "(3+4)*2^3"; sleep 2
qk searchbar state | grep -q '"plugin":"calc"' || fail "search bar (KWin)" "the calculator didn't answer under KWin: $(qk searchbar state | head -c 300)"
shot kwin-w3-search-calc
ipc wayland-hearth searchbar type "display"; sleep 2
shot kwin-w3-search-settings
ipc wayland-hearth searchbar close
r=$(qk searchbar desk "terminal"); sleep 2
[ "$r" = opened ] && [ "$(sb_shown)" = True ] || fail "search bar (KWin)" "typing on the desktop didn't open it under KWin: $r"
shot kwin-w3-search-typed-on-desktop
ipc wayland-hearth searchbar close
ipc wayland-hearth searchbar pad; sleep 3
shot kwin-w3-search-controller
ipc wayland-hearth searchbar close
ipc wayland-hearth activities create "Work" >/dev/null; sleep 1
ipc wayland-hearth activities switcher; sleep 2
shot kwin-w3-activities
ipc wayland-hearth activities close
ipc wayland-hearth settings page store:drivers; sleep 5
shot kwin-w3-store-drivers
ipc wayland-hearth settings close
# Settings › Users on a pre-installed PC (w3-firstrun: hearth-oem-account's marker for this account), then taken away.
OEM_M=/var/lib/hearth/oem-setup.json; OEM_MADE=""
if [ ! -e "$OEM_M" ]; then mkdir -p /var/lib/hearth && chmod 755 /var/lib/hearth && printf '{"user": "%s", "at": 1}\n' "$(id -un)" > "$OEM_M" && OEM_MADE=1; fi
ipc wayland-hearth settings page users; sleep 5
shot kwin-w3-users-oem
ipc wayland-hearth settings close
[ -n "$OEM_MADE" ] && rm -f "$OEM_M"
# The App store's app page and Hearth Files' terminal panel (search-store).
ipc wayland-hearth settings page store:app-org_kde_krita; sleep 12
shot kwin-w3-store-app
ipc wayland-hearth settings close
ipc wayland-hearth filemanager home; sleep 3
ipc wayland-hearth filemanager terminal on; sleep 3
shot kwin-w3-files-terminal
ipc wayland-hearth filemanager terminal off
ipc wayland-hearth filemanager close
# The search plugins (w3-search-plugins) under KWin: world clock, kill asks first, a colour, the bar's "?" list, and
# virtual desktops (KWin's own: WinList.desktops) listed by the desktops plugin.
cp "$(command -v sleep)" /tmp/hearth-demo-timer; /tmp/hearth-demo-timer 600 &
ipc wayland-hearth launcher open; sleep 3; ipc wayland-hearth launcher close
# Some answers come a moment later (kill looks the program up first: "Looking for …"): asked again for up to 10 s.
chk() { for _ in $(seq 1 10); do got=$(qk launcher answers "$1"); case "$got" in *"$2"*) break;; esac; sleep 1; done; case "$got" in *"$2"*) echo "::notice title=search plugins (KWin)::$1 → $(echo "$got" | cut -c1-200)" ;; *) fail "search plugins (KWin)" "\"$1\" didn't answer $2: $(echo "$got" | cut -c1-300)" ;; esac; }
chk "3pm london to tokyo" 'in Tokyo'
chk "#ff8800" 'rgb(255, 136, 0)'
chk "kill hearth-demo" '"type":"kill"'
chk "desktop 2" '"type":"desktop"'
ipc wayland-hearth searchbar type "3pm london to tokyo"; sleep 3
shot kwin-w3-search-world-clock
ipc wayland-hearth searchbar type "#ff8800"; sleep 3
shot kwin-w3-search-colour
ipc wayland-hearth searchbar type "?"; sleep 3
qk searchbar state | grep -q '"plugin":"worldclock"' || fail "search plugins (KWin)" "the search bar's ? doesn't list the new plugins: $(qk searchbar state | cut -c1-300)"
shot kwin-w3-search-help
ipc wayland-hearth searchbar type "kill hearth-demo"; sleep 3
ipc wayland-hearth searchbar enter; sleep 2
shot kwin-w3-search-kill-asks-first
ipc wayland-hearth searchbar close; sleep 2
pgrep -f hearth-demo-timer >/dev/null && echo "::notice title=search plugins (KWin)::kill asked first; closing it kept the program running" \
  || fail "search plugins (KWin)" "kill ended the program without asking"
pkill -f hearth-demo-timer 2>/dev/null
grep -E "(Launcher|SearchSources|FilesTerminal|FileManager)\.qml.*(TypeError|ReferenceError|Cannot assign|is not a type|is not defined)" "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null \
  | head -5 | while read -r l; do fail "Wave 3 (KWin)" "$(echo "$l" | cut -c1-300)"; done
grep -E "(SearchBar|Activit|StoreDrivers|SetStore)[A-Za-z]*\.qml.*(TypeError|ReferenceError|Cannot assign|is not a type|is not defined)" "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null \
  | head -5 | while read -r l; do fail "Wave 3 (KWin)" "$(echo "$l" | cut -c1-300)"; done
# The shortcuts Settings names (w3-effects): Super+Shift+D (KWin's Minimize all script) and Super+C (the Desktop Cube),
# against KWin's real defaults: switched on as the pages do (hearth-kwin-settings), then read back from kglobalaccel
# (KWin's D-Bus; Qt key codes: Meta+Shift+D 301989956, Meta+C 268435523) or kglobalshortcutsrc.
printf '%s' '{"Plugins": {"minimizeallEnabled": true, "cubeEnabled": true}}' | python3 "$NIMBUS_SHARE/desktop/shell/hearth-kwin-settings" set kwinrc >/dev/null 2>&1
gdbus call --session --dest org.kde.KWin --object-path /Scripting --method org.kde.kwin.Scripting.start >/dev/null 2>&1
gdbus call --session --dest org.kde.KWin --object-path /Effects --method org.kde.kwin.Effects.loadEffect cube >/dev/null 2>&1
sleep 4
kshort() {   # $1 action, $2 key as kglobalshortcutsrc writes it, $3 Qt key code, $4 what
  local infos line
  infos=$(gdbus call --session --dest org.kde.kglobalaccel --object-path /component/kwin --method org.kde.kglobalaccel.Component.allShortcutInfos 2>&1 | tr -d '\n')
  line=$(awk -v a="$1" '/^\[/ { g = $0 } g == "[kwin]" && index($0, a "=") == 1' "$HOME/.config/kglobalshortcutsrc" 2>/dev/null | head -1)
  if echo "$infos" | grep -oE "'$1', '[^']*', [^']*" | head -1 | grep -q "$3" || echo "$line" | grep -q "^$1=$2,"; then
    echo "$4: $2 is KWin's ($1; kglobalshortcutsrc: ${line:-not written yet}; D-Bus: $(echo "$infos" | grep -oE "'$1'[^)]{0,120}" | head -1))" >> shots/shortcuts.txt
  else
    fail "shortcuts (KWin)" "$4: KWin doesn't have $2 for $1: kglobalshortcutsrc '${line:-none}', D-Bus $(echo "$infos" | grep -oE "'$1'[^)]{0,160}" | head -1 | cut -c1-200)"
  fi
}
kshort MinimizeAll Meta+Shift+D 301989956 "Minimize every window"
if gdbus call --session --dest org.kde.KWin --object-path /Effects --method org.kde.kwin.Effects.isEffectSupported cube 2>/dev/null | grep -q true \
   || ls /usr/lib*/qt6/plugins/kwin/effects/plugins/*cube* /usr/lib*/*/qt6/plugins/kwin/effects/plugins/*cube* >/dev/null 2>&1; then
  kshort Cube Meta+C 268435523 "Desktop Cube"
else
  echo "::warning title=shortcuts (KWin)::no Desktop Cube effect on this runner (kdeplasma-addons): Super+C not checked"
  echo "Desktop Cube: no cube effect on this runner, Super+C not checked" >> shots/shortcuts.txt
fi
cp "$HOME/.local/share/nimbus/kwin.log" shots/kwin-B.log 2>/dev/null
cp "$XDG_RUNTIME_DIR/nimbus-shell.log" shots/shell-B.log 2>/dev/null
end_session

# ---- E: your own KDE files from 1.13: Hearth's fixed keys win, the card says which; the KDE shortcuts button.
mkdir -p "$HOME/.config" "$HOME/.local/state/nimbus"
printf '[Daemon]\nAutolock=true\nLockOnResume=true\nTimeout=1\n' > "$HOME/.config/kscreenlockerrc"
printf '[Wayland]\nInputMethod[$e]=/usr/share/applications/com.github.maliit.keyboard.desktop\n\n[NightColor]\nActive=false\n' > "$HOME/.config/kwinrc"
cp "$HOME/.config/kscreenlockerrc" /tmp/kscreenlockerrc-before; cp "$HOME/.config/kwinrc" /tmp/kwinrc-before
# What the bridge changed once (as on a PC that had KDE's default keys): the button puts them back.
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/state/nimbus/kwin-keys.json" \
  '{"done": {"kwin/Window Quick Tile Left": {"was": [[285212690]], "now": [], "title": "", "at": 1}, "kwin/Show Desktop": {"was": [[268435524]], "now": [], "title": "", "at": 1}, "kwin/Grid View": {"was": [[268435527]], "now": [], "title": "", "at": 1}}}' >/dev/null
start_session E
wait_up 90 || fail "KWin" "Mantel didn't come up under KWin with your own KDE files"
sleep 10
shot kwin-7-kde-settings-card
grep -q '^kwin-mine autolock+lockonresume+inputmethod' "$STATE/desktop-notice.shown" 2>/dev/null \
  || fail "your KDE settings" "the card wasn't shown (state: $(cat "$STATE"/desktop-notice* 2>/dev/null | tr '\n' '|'))"
# Your own lines are all still there (KDE's own kconf_update may add its [$Version] lines; Hearth never changes yours).
for f in kscreenlockerrc kwinrc; do
  cmp -s "$HOME/.config/$f" /tmp/$f-before 2>/dev/null || echo "::notice title=your KDE settings ($f)::$(diff /tmp/$f-before "$HOME/.config/$f" 2>/dev/null | tr '\n' '|' | cut -c1-400)"
done
for l in 'Autolock=true' 'LockOnResume=true' 'Timeout=1'; do grep -qxF "$l" "$HOME/.config/kscreenlockerrc" || fail "your KDE settings" "your kscreenlockerrc lost $l"; done
grep -qxF 'InputMethod[$e]=/usr/share/applications/com.github.maliit.keyboard.desktop' "$HOME/.config/kwinrc" || fail "your KDE settings" "your kwinrc's InputMethod was changed"
gdbus call --session --dest org.freedesktop.ScreenSaver --object-path /ScreenSaver --method org.freedesktop.ScreenSaver.GetActive 2>/dev/null | grep -q true \
  && fail "your KDE settings" "KWin is locked by itself"
pkill -TERM -x quickshell; sleep 12      # the card off the screen (shown once: it doesn't come back)
ipc wayland-hearth settings page keyboard:kde-keys; sleep 4
shot kwin-8-keyboard-kde-keys
ipc wayland-hearth settings close
python3 hosts/linux/desktop/shell/hearth-kwin-bridge keys changed > shots/kde-keys-E.json
end_session
rm -f "$HOME/.config/kscreenlockerrc" "$HOME/.config/kwinrc"

# ---- C: KWin ending right away: Mantel on labwc, with the card.
: > /tmp/kwin-crash
start_session C
for i in $(seq 1 60); do grep -q 'desktop mode: Mantel on labwc' "$LOG" && break; sleep 1; done
grep -q "desktop: KWin didn't start; labwc for this sign-in" "$LOG" || fail "safe start" "KWin ending early didn't open labwc: $(tail -6 "$LOG" | tr '\n' '|' | cut -c1-400)"
sleep 4
labwc_size; IN=$(labwc_display)
wait_up 60 || fail "safe start" "Mantel didn't come up on labwc after KWin ended"
sleep 8
shot kwin-4-fallback-card
[ "$(grep '^kwin ' "$STATE/desktop-fails" 2>/dev/null)" = "kwin 1" ] || fail "safe start" "the count isn't 1: $(cat "$STATE/desktop-fails" 2>/dev/null)"
end_session

# ---- D: the next sign-in, the same again: twice in a row → labwc until it's turned back on.
rm -f "$XDG_RUNTIME_DIR/hearth-kwin-failed"
start_session D
for i in $(seq 1 60); do grep -q "KWin didn't start twice in a row" "$LOG" && break; sleep 1; done
grep -q "KWin didn't start twice in a row" "$LOG" || fail "safe start" "the second failure wasn't counted: $(tail -6 "$LOG" | tr '\n' '|' | cut -c1-400)"
sleep 4
labwc_size; IN=$(labwc_display)
wait_up 60
sleep 8
shot kwin-5-twice-card
# The card off the screen for the page's picture: Mantel restarted (the autostart's watcher starts it again; the
# notice was shown, so the card doesn't come back).
pkill -TERM -x quickshell; sleep 12
ipc "$IN" settings page your-desktop; sleep 3
shot kwin-6-your-desktop-paused
ipc "$IN" settings close
python3 hosts/linux/desktop/shell/hearth-your-desktop status > shots/status-D.json
grep -q '"kwinPaused": true' shots/status-D.json || fail "safe start" "Your desktop doesn't say KWin is paused: $(head -c 300 shots/status-D.json)"
end_session
rm -f /tmp/kwin-crash

cp "$LOG" shots/session.log 2>/dev/null; cp "$HOME/.local/share/nimbus/kwin.log" shots/kwin.log 2>/dev/null
cp "$HOME/.local/share/nimbus/labwc.log" shots/labwc.log 2>/dev/null
grep -E 'desktop|kwin|labwc' "$LOG" | tail -30 | cut -c1-200 | while read -r l; do echo "::notice title=session::$l"; done | head -8
ls -la shots
for s in kwin-1-retired-card kwin-2-desktop kwin-3-your-desktop kwin-4-fallback-card kwin-5-twice-card kwin-6-your-desktop-paused kwin-7-kde-settings-card kwin-8-keyboard-kde-keys kwin-9-keyboard-kwin; do
  [ -s "shots/$s.png" ] || fail "screenshot" "$s is missing ($(tail -2 shots/shot-errors.txt 2>/dev/null | tr '\n' '|'))"
done
python3 - <<'PY'
import os
from PIL import Image, ImageStat
for n in sorted(os.listdir('shots')):
    if n.endswith('.png'):
        im = Image.open('shots/' + n)
        st = ImageStat.Stat(im.convert('L'))
        print('::notice title=picture %s::%dx%d, brightness %.0f, spread %.0f' % (n, im.width, im.height, st.mean[0], st.stddev[0]))
        if st.stddev[0] < 3:
            print('::error title=screenshot::%s is one flat colour (nothing drawn)' % n)
            open('shots/FAILED', 'w').close()
PY
[ ! -e shots/FAILED ]
