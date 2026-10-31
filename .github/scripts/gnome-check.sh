#!/bin/bash
# Hearth GNOME's checks inside one base (gnome-test.yml): run as root in fedora:44 (Anatase) or debian:trixie
# (Peppermint), privileged, with the runner's /dev/input bound in (the virtual pad and hearth-pad's devices show up
# there) and the repository at /src.
#   bash .github/scripts/gnome-check.sh anatase|peppermint
# 1. GNOME's minimal set as the images install it (and what it adds, measured here);
# 2. lint: every extension's JavaScript parses (node --check), metadata.json / the session mode are JSON, each uuid
#    matches its folder and lists this GNOME's version; the Python tests;
# 3. a headless GNOME Shell in Hearth's session mode (plus the CI helper hearth-ci@hearthos.fun), then
#    tools/e2e-hearth-gnome.py: the extensions, screenshots, and the controller under a virtual uinput pad.
# Pictures and logs go to /src/shots/<base>; /src/shots/<base>/FAILED when something failed.
set +e
BASE="${1:-anatase}"
OUT="/src/shots/$BASE"
mkdir -p "$OUT"
cd /src || exit 1
fail=0
err() { echo "::error title=$1 ($BASE)::$2"; fail=1; }
size() { du -sxm /usr /etc 2>/dev/null | awk '{s += $1} END {print s}'; }

# ------------------------------------------------------------------ 1. GNOME
if [ "$BASE" = anatase ]; then
  dnf -y install python3 python3-evdev git dbus-daemon dbus-tools glib2 mesa-dri-drivers mesa-libEGL google-noto-sans-fonts \
    procps-ng nodejs breeze-icon-theme hicolor-icon-theme gtk-update-icon-cache xorg-x11-server-Xwayland >/tmp/tools.log 2>&1 \
    || err setup "test tools: $(tail -3 /tmp/tools.log | tr '\n' ' ')"
  before=$(size)
  dnf -y install --setopt=install_weak_deps=False gnome-shell mutter gnome-session gnome-control-center >/tmp/gnome.log 2>&1 \
    || err "GNOME install" "$(tail -5 /tmp/gnome.log | tr '\n' ' ' | cut -c1-800)"
  dnf -y install --skip-unavailable --setopt=install_weak_deps=False xdg-desktop-portal-gnome gnome-settings-daemon \
    nautilus gnome-console gnome-text-editor >>/tmp/gnome.log 2>&1
  after=$(size)
else
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends python3 python3-evdev git dbus dbus-daemon libglib2.0-bin libgl1-mesa-dri \
    libegl-mesa0 fonts-noto-core procps nodejs breeze-icon-theme hicolor-icon-theme gtk-update-icon-cache ca-certificates xwayland \
    >/tmp/tools.log 2>&1 || err setup "test tools: $(tail -3 /tmp/tools.log | tr '\n' ' ')"
  before=$(size)
  apt-get install -y -qq --no-install-recommends gnome-shell mutter gnome-session gnome-control-center >/tmp/gnome.log 2>&1 \
    || err "GNOME install" "$(tail -5 /tmp/gnome.log | tr '\n' ' ' | cut -c1-800)"
  for p in xdg-desktop-portal-gnome nautilus gnome-console gnome-text-editor; do
    apt-get install -y -qq --no-install-recommends "$p" >>/tmp/gnome.log 2>&1 || echo "::warning title=GNOME ($BASE)::$p isn't installable"
  done
  after=$(size)
fi
GV=$(gnome-shell --version 2>/dev/null | grep -oE '[0-9]+' | head -1)
echo "::notice title=GNOME ($BASE)::$(gnome-shell --version 2>&1 | head -1); the minimal set added about $((after - before)) MB to this container (the image build measures the image itself)"
echo "$((after - before))" > "$OUT/gnome-size-container-mb"
for b in gnome-shell gnome-session; do command -v "$b" >/dev/null || err "GNOME install" "$b is missing"; done

# ------------------------------------------------------------------ 2. lint and Python tests
EXT=os/system_files/usr/share/gnome-shell/extensions
python3 -m json.tool os/system_files/usr/share/gnome-shell/modes/hearth.json >/dev/null || err "session mode" "hearth.json isn't JSON"
for d in "$EXT"/*@hearthos.fun hosts/linux/desktop/gnome; do
  if [ -f "$d/metadata.json" ]; then
    python3 - "$d" "$GV" <<'PY' || fail=1
import json, os, sys
d, gv = sys.argv[1], sys.argv[2]
m = json.load(open(os.path.join(d, 'metadata.json')))
if m.get('uuid') != os.path.basename(d):
    print('::error title=extension %s::uuid %s doesn\'t match its folder' % (d, m.get('uuid'))); sys.exit(1)
if gv and gv not in m.get('shell-version', []):
    print('::error title=extension %s::shell-version %s doesn\'t list GNOME %s' % (m['uuid'], m.get('shell-version'), gv)); sys.exit(1)
PY
  fi
  for f in "$d"/*.js; do
    [ -f "$f" ] || continue
    cp "$f" /tmp/check.mjs
    node --check /tmp/check.mjs 2>/tmp/js.err || err "JavaScript" "$f: $(tr '\n' ' ' </tmp/js.err | cut -c1-600)"
  done
done
for t in test_hearth_gnome test_hearth_plasma test_hearth_pad; do
  timeout 300 python3 -m unittest discover -s hosts/linux/test -p "$t.py" >"$OUT/$t.txt" 2>&1 \
    || err "Python tests" "$t: $(grep -E '^(FAIL|ERROR|AssertionError|[A-Za-z]*Error)' "$OUT/$t.txt" | head -6 | tr '\n' '|' | cut -c1-800)"
done

# ------------------------------------------------------------------ 3. GNOME Shell in Hearth's session mode
cp -a os/system_files/usr/share/gnome-shell/. /usr/share/gnome-shell/
cp -a os/system_files/usr/share/hearth-gnome /usr/share/
cp -a .github/scripts/gnome-ci/hearth-ci@hearthos.fun /usr/share/gnome-shell/extensions/
python3 - <<'PY'
import json
m = json.load(open('/usr/share/gnome-shell/modes/hearth.json'))
m['enabledExtensions'] = m['enabledExtensions'] + ['hearth-ci@hearthos.fun']
json.dump(m, open('/usr/share/gnome-shell/modes/hearth-ci.json', 'w'))
PY
mkdir -p /usr/share/nimbus /usr/share/backgrounds/hearth
rm -rf /usr/share/nimbus/desktop && cp -a hosts/linux/desktop /usr/share/nimbus/desktop
cp os/branding/wallpapers/hearth-fireside.jpg /usr/share/backgrounds/hearth/
for d in branding/icons/png/*x*; do mkdir -p "/usr/share/icons/hicolor/$(basename "$d")/apps" && cp "$d"/*.png "/usr/share/icons/hicolor/$(basename "$d")/apps/"; done
mkdir -p /usr/share/icons/hicolor/scalable/apps && cp branding/icons/apps/*.svg /usr/share/icons/hicolor/scalable/apps/
gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null
# GNOME's own apps only in Hearth GNOME, as the images do.
for id in org.gnome.Nautilus org.gnome.Console org.gnome.TextEditor; do
  f="/usr/share/applications/$id.desktop"
  [ -f "$f" ] && { sed -i '/^OnlyShowIn=/d; /^NotShowIn=/d' "$f"; sed -i '0,/^\[Desktop Entry\]/s//[Desktop Entry]\nOnlyShowIn=GNOME;/' "$f"; }
done

export HOME=/tmp/home XDG_RUNTIME_DIR=/tmp/xdg
mkdir -p "$HOME" "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
command -v dbus-uuidgen >/dev/null && dbus-uuidgen --ensure 2>/dev/null
[ -s /etc/machine-id ] || cat /var/lib/dbus/machine-id > /etc/machine-id 2>/dev/null
# A system bus without its activatable services: there's no systemd here, so asking D-Bus to start logind (and the
# like) fails and GNOME Shell stops at startup; with nothing to activate, its proxies simply have no owner.
rm -f /usr/share/dbus-1/system-services/*.service
mkdir -p /run/dbus && dbus-daemon --system --fork 2>/dev/null
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
# What nimbus-session's run_gnome gives GNOME (the CI mode adds hearth-ci).
export XDG_CURRENT_DESKTOP=GNOME XDG_SESSION_DESKTOP=gnome HEARTH_DESKTOP=gnome XDG_SESSION_TYPE=wayland \
  GNOME_SHELL_SESSION_MODE=hearth-ci XDG_DATA_DIRS=/usr/share/hearth-gnome:/usr/local/share:/usr/share \
  NIMBUS_SHARE=/usr/share/nimbus LIBGL_ALWAYS_SOFTWARE=1 NO_AT_BRIDGE=1
# Hearth's GTK colours (theme/apply.sh writes ~/.config/gtk-4.0/gtk.css), as at every Hearth sign-in.
sh /usr/share/nimbus/desktop/theme/apply.sh >/dev/null 2>&1
# The launcher's Games: what Hearth listed last time (no Hearth app here).
mkdir -p "$HOME/.cache/hearth"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"

HELP=$(gnome-shell --help-all 2>&1; gnome-shell --help 2>&1)
OPTS="--headless --virtual-monitor 1920x1080 --mode=hearth-ci"
echo "$HELP" | grep -q -- '--wayland' && OPTS="$OPTS --wayland"
echo "$HELP" | grep -q -- '--no-x11' && OPTS="$OPTS --no-x11"
echo "$HELP" | grep -q -- '--unsafe-mode' && OPTS="$OPTS --unsafe-mode"
echo "gnome-shell $OPTS"
gnome-shell $OPTS >"$OUT/gnome-shell.log" 2>&1 &
SHELL_PID=$!
timeout 1200 python3 tools/e2e-hearth-gnome.py "$OUT" 2>&1 | tee "$OUT/e2e.log"
[ "${PIPESTATUS[0]}" = 0 ] || fail=1
kill -0 "$SHELL_PID" 2>/dev/null || err "GNOME Shell" "it ended: $(tail -8 "$OUT/gnome-shell.log" | tr '\n' '|' | cut -c1-900)"
# Errors from Hearth's extensions in GNOME Shell's log.
bad=$(grep -E "hearthos\.fun|Hearth" "$OUT/gnome-shell.log" | grep -iE "error|TypeError|ReferenceError|SyntaxError|not a function|undefined" | grep -v '^Gjs-Message.*Hearth timer$' | sort -u | head -10)
if [ -n "$bad" ]; then
  echo "::error title=Hearth extensions ($BASE)::$(printf '%s' "$bad" | cut -c1-300 | sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g')"
  fail=1
fi
tail -40 "$OUT/gnome-shell.log" | sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g' | cut -c1-3800 | sed 's/^/::notice title=GNOME Shell log tail ('"$BASE"')::/'; echo
kill "$SHELL_PID" 2>/dev/null; sleep 2; kill -9 "$SHELL_PID" 2>/dev/null
pkill -f hearth-pad; pkill nautilus; pkill kgx; pkill gnome-text-editor; pkill dbus-daemon
[ "$fail" = 0 ] || touch "$OUT/FAILED"
chmod -R a+rwX /src/shots
exit 0
