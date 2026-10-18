#!/bin/bash
# Turns Bazzite (deck edition) into Hearth OS. Runs once, inside the image build.
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
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh /usr/share/nimbus/desktop/shell/hearth-pad /usr/share/nimbus/desktop/shell/hearth-buttons /usr/share/nimbus/desktop/shell/hearth-controller /usr/share/nimbus/desktop/shell/hearth-power /usr/share/nimbus/desktop/shell/hearth-browser /usr/bin/hearth-settings /usr/bin/hearth-wallpaper
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
dnf5 -y install labwc grim brightnessctl wlr-randr wlsunset python3-evdev \
  qt6ct qt5ct xdg-desktop-portal-wlr xdg-desktop-portal-gtk xprop xwininfo cpio
dnf5 -y copr enable errornointernet/quickshell
dnf5 -y install quickshell
dnf5 -y copr disable errornointernet/quickshell

# ------------------------------------------------------------------ power button
# Bazzite gives the power button to Steam's UI (steamos-powerbuttond, logind ignores it). In Hearth's
# session Hearth handles it (Settings › Power button), so don't start that for every session; Bazzite's own
# Steam session still pulls it in where it needs it.
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ Hearth Settings and the Hearth look
cat > /usr/share/applications/hearth-settings.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Settings
Comment=Wi-Fi, Bluetooth, sound, display, controllers, accounts and updates
Exec=/usr/bin/hearth-settings
Icon=hearth
Type=Application
Categories=Settings;
Keywords=wifi;bluetooth;sound;display;controller;update;
DESKTOP
# The Hearth color scheme, also selectable in Plasma (System Settings › Colors).
install -Dm644 /usr/share/nimbus/desktop/theme/Hearth.colors /usr/share/color-schemes/Hearth.colors
# Wallpapers: /usr/share/backgrounds/hearth, and as Plasma wallpapers (System Settings › Wallpaper).
declare -A WALL=([fireside]="Fireside" [embers]="Embers" [ridge]="Dusk Ridge" [mark]="The Mark" [kiln]="Kiln"
  [cat]="Hearth Cat" [cabin]="Night Cabin" [lanterns]="Lanterns" [bloom]="Ember Bloom" [peaks]="Low Poly Peaks"
  [forge]="Forge Grid" [topo]="Cinder Topo" [fox]="Fox Moon")
for id in "${!WALL[@]}"; do
  install -Dm644 "/ctx/wallpapers/hearth-$id.jpg" "/usr/share/backgrounds/hearth/hearth-$id.jpg"
  dir="/usr/share/wallpapers/Hearth-$id"
  install -Dm644 "/ctx/wallpapers/hearth-$id.jpg" "$dir/contents/images/3840x2160.jpg"
  printf '{ "KPlugin": { "Id": "Hearth-%s", "Name": "Hearth %s", "Authors": [{ "Name": "Hearth" }], "License": "CC-BY-SA-4.0" } }\n' \
    "$id" "${WALL[$id]}" > "$dir/metadata.json"
done
# Portals for Desktop Mode (XDG_CURRENT_DESKTOP=Nimbus): screen sharing and screenshots through wlroots,
# KDE file dialogs, everything else from GTK.
mkdir -p /usr/share/xdg-desktop-portal
cat > /usr/share/xdg-desktop-portal/nimbus-portals.conf <<PORTALS
[preferred]
default=gtk
org.freedesktop.impl.portal.ScreenCast=wlr
org.freedesktop.impl.portal.Screenshot=wlr
org.freedesktop.impl.portal.FileChooser=kde
PORTALS

# ------------------------------------------------------------------ boot straight into Nimbus
systemctl enable nimbus-autologin.service

# ------------------------------------------------------------------ boot splash
# Hearth's Plymouth theme (system_files/usr/share/plymouth/themes/hearth, drawn by os/branding/make-assets.py).
# The splash runs from the initramfs, so that's rebuilt to carry it (no host-only bits: it boots any PC).
dnf5 -y install plymouth-plugin-script
plymouth-set-default-theme hearth
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
export DRACUT_NO_XATTR=1
dracut --no-hostonly --kver "$KVER" --reproducible --zstd --add ostree --add plymouth \
  -f "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"

# ------------------------------------------------------------------ name
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (based on Bazzite)"/' \
  /usr/lib/os-release
