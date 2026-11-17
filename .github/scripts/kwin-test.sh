#!/bin/bash
# Mantel on KWin, for real (kwin-test.yml; docs/DESKTOPS.md "Mantel on KWin"): nimbus-session itself starts Desktop Mode
# four times on a 1920x1080 virtual screen (KWin's own, --virtual, with a vgem render device so it composites; pictures
# through KWin's screenshot API. labwc's fallback runs headless; grim takes its pictures):
#   A  KWin, with 1.13's KDE Plasma still picked: the "KDE Plasma and GNOME were retired" card     kwin-1-retired-card
#   B  KWin: Mantel's bar up (layer shell), the desktop and Settings › System › Your desktop         kwin-2-desktop, kwin-3-your-desktop
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
start_session B
wait_up 90 || fail "KWin" "Mantel didn't come up under KWin the second time"
sleep 10
shot kwin-2-desktop
grep -iE 'layer.?shell.*(not|fail|unsupported)|wlr_layer.*(not|fail)' "$XDG_RUNTIME_DIR/nimbus-shell.log" | head -3 | while read -r l; do fail "layer shell" "$(echo "$l" | cut -c1-300)"; done
ipc wayland-hearth settings page your-desktop; sleep 3
shot kwin-3-your-desktop
ipc wayland-hearth settings close
python3 hosts/linux/desktop/shell/hearth-your-desktop status > shots/status-B.json
cp "$HOME/.local/share/nimbus/kwin.log" shots/kwin-B.log 2>/dev/null
cp "$XDG_RUNTIME_DIR/nimbus-shell.log" shots/shell-B.log 2>/dev/null
end_session

# ---- E: your own KDE files from 1.13: Hearth's fixed keys win, the card says which; the KDE shortcuts button.
mkdir -p "$HOME/.config" "$HOME/.local/state/nimbus"
printf '[Daemon]\nAutolock=true\nLockOnResume=true\nTimeout=1\n' > "$HOME/.config/kscreenlockerrc"
printf '[Wayland]\nInputMethod[$e]=/usr/share/applications/com.github.maliit.keyboard.desktop\n\n[NightColor]\nActive=false\n' > "$HOME/.config/kwinrc"
cp "$HOME/.config/kscreenlockerrc" /tmp/lockrc-before; cp "$HOME/.config/kwinrc" /tmp/kwinrc-before
# What the bridge changed once (as on a PC that had KDE's default keys): the button puts them back.
python3 hosts/linux/desktop/shell/hearth_settings_io.py merge "$HOME/.local/state/nimbus/kwin-keys.json" \
  '{"done": {"kwin/Window Quick Tile Left": {"was": [[285212690]], "now": [], "title": "", "at": 1}, "kwin/Show Desktop": {"was": [[268435524]], "now": [], "title": "", "at": 1}, "kwin/Grid View": {"was": [[268435527]], "now": [], "title": "", "at": 1}}}' >/dev/null
start_session E
wait_up 90 || fail "KWin" "Mantel didn't come up under KWin with your own KDE files"
sleep 10
shot kwin-7-kde-settings-card
grep -q '^kwin-mine autolock+lockonresume+inputmethod' "$STATE/desktop-notice.shown" 2>/dev/null \
  || fail "your KDE settings" "the card wasn't shown (state: $(cat "$STATE"/desktop-notice* 2>/dev/null | tr '\n' '|'))"
cmp -s "$HOME/.config/kscreenlockerrc" /tmp/lockrc-before && cmp -s "$HOME/.config/kwinrc" /tmp/kwinrc-before \
  || fail "your KDE settings" "your own files were changed"
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
for s in kwin-1-retired-card kwin-2-desktop kwin-3-your-desktop kwin-4-fallback-card kwin-5-twice-card kwin-6-your-desktop-paused kwin-7-kde-settings-card kwin-8-keyboard-kde-keys; do
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
