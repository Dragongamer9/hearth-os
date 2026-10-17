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
Icon=hearth
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

# ------------------------------------------------------------------ boot splash
# Hearth's Plymouth theme (system_files/usr/share/plymouth/themes/hearth, drawn by os/branding/make-assets.py).
# The splash runs from the initramfs, so that's rebuilt to carry it (no host-only bits: it boots any PC).
plymouth-set-default-theme hearth
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
export DRACUT_NO_XATTR=1
dracut --no-hostonly --kver "$KVER" --reproducible --zstd --add ostree --add plymouth \
  -f "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"

# ------------------------------------------------------------------ name
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth (based on Bazzite)"/' \
  /usr/lib/os-release
