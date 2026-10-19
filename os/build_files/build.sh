#!/bin/bash
# Turns Bazzite (deck edition) into Hearth OS. Runs once, inside the image build.
set -ouex pipefail

# ------------------------------------------------------------------ files that just get copied in
cp -avf /ctx/system_files/. /

# ------------------------------------------------------------------ Nimbus itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit.
  install -Dm755 /ctx/nimbus-app/hearth /usr/lib/nimbus/nimbus
  dnf5 -y install webkit2gtk4.1
  # Hearth Browser (still Electron) in its own folder.
  if [ -d /ctx/nimbus-app/browser ]; then
    mkdir -p /usr/lib/hearth-browser && cp -a /ctx/nimbus-app/browser/. /usr/lib/hearth-browser/
    if [ -f /usr/lib/hearth-browser/chrome-sandbox ]; then chown root:root /usr/lib/hearth-browser/chrome-sandbox && chmod 4755 /usr/lib/hearth-browser/chrome-sandbox; fi
  fi
  # Hearth Browser on Chromium (CEF): /usr/bin/hearth-browser uses it when it's there.
  if [ -x /ctx/nimbus-app/browser-cef/hearth-browser ]; then
    mkdir -p /usr/lib/hearth-browser-cef && cp -a /ctx/nimbus-app/browser-cef/. /usr/lib/hearth-browser-cef/
    if [ -f /usr/lib/hearth-browser-cef/chrome-sandbox ]; then chown root:root /usr/lib/hearth-browser-cef/chrome-sandbox && chmod 4755 /usr/lib/hearth-browser-cef/chrome-sandbox; fi
  fi
else
  cp -a /ctx/nimbus-app/. /usr/lib/nimbus/
fi
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
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh /usr/share/nimbus/desktop/shell/hearth-pad /usr/share/nimbus/desktop/shell/hearth-buttons /usr/share/nimbus/desktop/shell/hearth-controller /usr/share/nimbus/desktop/shell/hearth-power /usr/share/nimbus/desktop/shell/hearth-browser /usr/share/nimbus/desktop/shell/hearth-tasks /usr/bin/hearth-settings /usr/bin/hearth-wallpaper
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
  qt6ct qt5ct xdg-desktop-portal-wlr xdg-desktop-portal-gtk xprop xwininfo xdotool gnome-keyring cpio rsync
dnf5 -y copr enable errornointernet/quickshell
dnf5 -y install quickshell
dnf5 -y copr disable errornointernet/quickshell

# ------------------------------------------------------------------ power button
# Bazzite gives the power button to Steam's UI (steamos-powerbuttond, logind ignores it). In Hearth's
# session Hearth handles it (Settings › Power button), so don't start that for every session; Bazzite's own
# Steam session still pulls it in where it needs it.
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ Hearth Settings and the Hearth look
cat > /usr/share/applications/hearth-tasks.desktop <<DESKTOP
[Desktop Entry]
Name=Task Manager
Comment=See what's running, close apps that stopped responding, and check performance
Exec=quickshell ipc -p /usr/share/nimbus/desktop/shell call tasks open
Icon=hearth
Type=Application
Categories=System;Monitor;
Keywords=task;process;end;kill;cpu;memory;gpu;performance;startup;
OnlyShowIn=Nimbus;
DESKTOP
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
# Hearth Browser: Hearth's own web browser (the Hearth app with --browser).
cat > /usr/share/applications/hearth-browser.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Browser
GenericName=Web Browser
Comment=Browse the web
Exec=/usr/bin/hearth-browser %U
Icon=hearth
Type=Application
Categories=Network;WebBrowser;
MimeType=text/html;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=Hearth Browser
Actions=new-window;
Keywords=web;internet;browser;

[Desktop Action new-window]
Name=New window
Exec=/usr/bin/hearth-browser
DESKTOP
chmod 755 /usr/bin/hearth-browser
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
systemctl enable hearth-greeter-pad.service   # the controller on the login screen

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
