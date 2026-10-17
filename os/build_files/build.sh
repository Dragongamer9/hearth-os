#!/bin/bash
# Turns Bazzite (deck edition) into Hearth, the Linux OS of the Nimbus project. Runs once, inside the image build.
set -ouex pipefail

# ------------------------------------------------------------------ files that just get copied in
cp -avf /ctx/system_files/. /

# ------------------------------------------------------------------ Nimbus itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
cp -a /ctx/nimbus-app/. /usr/lib/nimbus/
test -x /usr/lib/nimbus/nimbus
# Chromium's sandbox helper must be setuid root to work.
if [ -f /usr/lib/nimbus/chrome-sandbox ]; then chown root:root /usr/lib/nimbus/chrome-sandbox && chmod 4755 /usr/lib/nimbus/chrome-sandbox; fi
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version

# Let Nimbus's app menu entry exist for Desktop Mode / Plasma too.
cat > /usr/share/applications/nimbus.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth
Comment=Controller-first game library
Exec=/usr/bin/nimbus --windowed
Icon=applications-games
Type=Application
Categories=Game;
DESKTOP

# ------------------------------------------------------------------ Desktop Mode: labwc + Quickshell
# (gamescope, wlr-randr and umu-launcher already come with Bazzite.)
dnf5 -y install labwc grim brightnessctl
dnf5 -y copr enable errornointernet/quickshell
dnf5 -y install quickshell
dnf5 -y copr disable errornointernet/quickshell

# ------------------------------------------------------------------ boot straight into Nimbus
systemctl enable nimbus-autologin.service

# ------------------------------------------------------------------ package repos the installer can read
# The ISO builder reads every enabled repo in the image, and refuses one whose signing key file is missing
# (Bazzite's terra-mesa points at a key it doesn't ship). Switch those off: their packages are already in
# the image, and system updates come as whole images, not from these repos.
for f in /etc/yum.repos.d/*.repo; do
  [ -f "$f" ] || continue
  python3 - "$f" <<'PY'
import configparser, os, re, sys
path = sys.argv[1]
VER = re.search(r'^VERSION_ID="?(\d+)', open('/usr/lib/os-release').read(), re.M).group(1)
cp = configparser.RawConfigParser(strict=False)
cp.read(path)
changed = False
for sec in cp.sections():
    if cp.get(sec, 'enabled', fallback='1').strip() != '1':
        continue
    keys = re.split(r'[\s,]+', cp.get(sec, 'gpgkey', fallback='').strip())
    keys = [k.replace('$releasever', VER).replace('$basearch', 'x86_64') for k in keys]
    missing = [k for k in keys if k.startswith('file://') and not os.path.exists(k[7:])]
    if missing:
        print(f'{path}: [{sec}] off, missing key {missing[0]}')
        cp.set(sec, 'enabled', '0')
        changed = True
if changed:
    with open(path, 'w') as fh:
        cp.write(fh)
PY
done

# ------------------------------------------------------------------ name
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth (based on Bazzite)"/' \
  /usr/lib/os-release
