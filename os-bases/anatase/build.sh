#!/bin/bash
# Turns Anatase (Fedora 45, bootc) into Hearth OS. Runs once, inside the image build (os-bases/anatase/Containerfile).
# The Hearth parts follow os/build_files/build.sh (the Bazzite build) as closely as possible: Anatase is Fedora
# too, so the same packages and the same steps. What's different is said where it happens:
#  - Handheld Daemon (HHD), already in Anatase, runs the built-in controller, power limit, fan and lights (Bazzite
#    has InputPlumber); Hearth talks to it (hosts/hearth-core/src/hhd.rs, desktop/shell/hearth-hhd).
#  - Steam is installed as Bazzite has it (the steam package, RPM Fusion), so a Bazzite home folder's Steam games
#    (~/.local/share/Steam) are found right after the move; Flathub's Steam at first boot if that can't be had.
#  - /usr/share/nimbus/image-info.json says this is the Anatase edition (Hearth's edition(); the move from the
#    Bazzite-based Hearth checks it), and hearth-clean-install finishes that move at the first start.
# See docs/BASE-ANATASE.md.
set -ouex pipefail

FEDORA=$(rpm -E %fedora)
echo "Anatase on Fedora $FEDORA"

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's Anatase-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/anatase_files/. /

# ------------------------------------------------------------------ Hearth itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit.
  install -Dm755 /ctx/nimbus-app/hearth /usr/lib/nimbus/nimbus
  dnf5 -y install webkit2gtk4.1
  # Hearth Browser's Electron build (hosts/linux): the Quick Access overlay app, and the old browser.
  if [ -d /ctx/nimbus-app/browser ]; then
    mkdir -p /usr/lib/hearth-browser && cp -a /ctx/nimbus-app/browser/. /usr/lib/hearth-browser/
    if [ -f /usr/lib/hearth-browser/chrome-sandbox ]; then chown root:root /usr/lib/hearth-browser/chrome-sandbox && chmod 4755 /usr/lib/hearth-browser/chrome-sandbox; fi
  fi
  # Hearth drawn by Chromium (hosts/electron): the `nimbus` program starts it on Nvidia graphics (docs/RENDERERS.md).
  if [ -x /ctx/nimbus-app/electron/hearth-electron ]; then
    mkdir -p /usr/lib/hearth-electron && cp -a /ctx/nimbus-app/electron/. /usr/lib/hearth-electron/
    if [ -f /usr/lib/hearth-electron/chrome-sandbox ]; then chown root:root /usr/lib/hearth-electron/chrome-sandbox && chmod 4755 /usr/lib/hearth-electron/chrome-sandbox; fi
  fi
else
  cp -a /ctx/nimbus-app/. /usr/lib/nimbus/
fi
test -x /usr/lib/nimbus/nimbus
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
# ---- hearthfetch (hosts/hearthfetch): Hearth OS's system summary for the terminal, built with the app.
if [ -x /ctx/nimbus-app/hearthfetch ]; then install -Dm755 /ctx/nimbus-app/hearthfetch /usr/bin/hearthfetch; fi
# ---- end hearthfetch

# ------------------------------------------------------------------ SVG icons for GTK (librsvg's gdk-pixbuf loader)
# Without it GTK can't draw an SVG icon and aborts the whole program the first time it needs one (Hearth's Game Mode
# crashed over and over on CachyOS: "image-missing.svg: Unrecognized image file format"). The loaders list is
# rebuilt so the SVG loader is in it.
dnf5 -y install librsvg2 gdk-pixbuf2
# Newer gdk-pixbuf loads SVG (and other images) through glycin's loaders instead.
dnf5 -y install --skip-unavailable glycin-loaders || true
for q in /usr/bin/gdk-pixbuf-query-loaders /usr/bin/gdk-pixbuf-query-loaders-64 /usr/lib/*/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders; do
  [ -x "$q" ] && { "$q" --update-cache || true; break; }
done

# ---- Hearth Terminal (hosts/hearth-terminal): built by CI next to the Hearth app (base-*.yml) and carried in
# os/nimbus-app. Mantel opens it for its terminal (desktop/open.sh); its menu entry (hearth-terminal.desktop) and
# Mantel's xdg-terminal-exec list come from os/system_files. Any other terminal stays as the fallback.
if [ -x /ctx/nimbus-app/hearth-terminal ]; then
  install -Dm755 /ctx/nimbus-app/hearth-terminal /usr/bin/hearth-terminal
  # A terminal that can't start is worse than none: then Mantel falls back to the others (open.sh).
  if ldd /usr/bin/hearth-terminal | grep 'not found'; then echo "Hearth Terminal needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/hearth-terminal; fi
fi
# ---- end of Hearth Terminal

# ------------------------------------------------------------------ Hearth Disks (hosts/hearth-disks)
# The disk manager, built with the app (base-anatase.yml). udisks2 (all its changes go through it) and the tools it formats with.
dnf5 -y install udisks2 exfatprogs dosfstools || true
command -v udisksctl >/dev/null || echo "warning: udisks2 is missing, so Disks can't change drives"
if [ -x /ctx/nimbus-app/hearth-disks ]; then
  install -Dm755 /ctx/nimbus-app/hearth-disks /usr/bin/hearth-disks
  if ldd /usr/bin/hearth-disks | grep "not found"; then echo "warning: Disks is missing the libraries above"; fi
  install -d /usr/share/applications
  cat > /usr/share/applications/hearth-disks.desktop <<DESKTOP
[Desktop Entry]
Name=Disks
GenericName=Disk Manager
Comment=See your drives and their health, format USB drives and SD cards, and set up drives for games
Exec=/usr/bin/hearth-disks
Icon=hearth
Type=Application
Categories=System;Utility;
Keywords=disk;drive;partition;format;usb;sd;card;mount;eject;storage;steam;library;
DESKTOP
fi

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh
find /usr/share/nimbus/desktop/shell -maxdepth 1 -type f -name 'hearth-*' -exec chmod 755 {} +
chmod 755 /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper /usr/bin/hearth-browser /usr/bin/hearth-send-logs
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; the other editions
# write this same file).
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-os", "image-ref": "ghcr.io/dragongamer9/hearth-os", "image-tag": "testing", "base": "anatase", "base-detail": "Anatase (Fedora ${FEDORA})", "base-image": "${BASE_IMAGE:-i.anatase.org/anatase:stable}", "version": "${NIMBUS_VERSION:-0.0.0}"}
INFO

# Hearth's app menu entry for Desktop Mode / Plasma too.
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
# (gamescope comes with Anatase.)
dnf5 -y install labwc grim brightnessctl wlr-randr wlsunset python3-evdev \
  qt6ct qt5ct xdg-desktop-portal-wlr xdg-desktop-portal-gtk xprop xwininfo xdotool gnome-keyring cpio rsync
# Quickshell: Fedora's own package when it has one, else the COPR the Bazzite build uses.
if ! dnf5 -y install quickshell; then
  dnf5 -y copr enable errornointernet/quickshell
  dnf5 -y install quickshell
  dnf5 -y copr disable errornointernet/quickshell
fi
# Couch Co-op (Beta Experimental), Move games / Make Hearth bigger, Proton for games added by hand: not fatal,
# Hearth says what's missing.
dnf5 -y install 7zip || dnf5 -y install p7zip p7zip-plugins || true
# bsdtar: Hearth Files opens .rar, .iso, .cbr and the rest of the archive kinds 7-Zip doesn't.
dnf5 -y install bsdtar || true
dnf5 -y install fuse-overlayfs bubblewrap xdpyinfo xrandr || true
dnf5 -y install ntfsprogs ntfs-3g e2fsprogs btrfs-progs util-linux || true
dnf5 -y install umu-launcher || true
# Claude Desktop (Hearth Settings › Claude: Anthropic's package, unpacked in the user's home folder by
# desktop/shell/hearth-claude, never shipped in this image): gpg checks Anthropic's signature; the app links
# GTK 3, NSS, libdrm/gbm, libxtst, PipeWire and AT-SPI (above), and opens libsecret (the sign-in kept in the
# password store), libnotify (notifications) and the tray library while it runs.
dnf5 -y install gnupg2
for p in libsecret libnotify libxcb at-spi2-core libuuid xdg-utils libayatana-appindicator-gtk3 pipewire-libs libdrm mesa-libgbm; do
  dnf5 -y install "$p" || echo "warning: $p isn't available"
done
# Printing (Settings › Printers: CUPS with driverless IPP Everywhere, USB printers through ipp-usb, network printers
# found with Avahi), clipboard history (Super+V: wl-clipboard, cliphist; wtype pastes) and Flatpak for Apps.
# Desktop Mode also: choosing an area for a screenshot (slurp), screenshot notifications (notify-send) and turning
# the screen off when idle (wlopm, Settings › Power).
for p in cups cups-filters ipp-usb avahi nss-mdns wl-clipboard cliphist wtype flatpak slurp libnotify wlopm; do
  dnf5 -y install "$p" || echo "warning: $p isn't available"
done
systemctl enable cups.socket || true
systemctl enable avahi-daemon.socket 2>/dev/null || true
# The move's first start (hearth-clean-install) reads Lutris' database with Python's sqlite3, and the controller
# on the login screen needs evdev (above).
python3 -c 'import sqlite3, json'

# ------------------------------------------------------------------ Steam
# As on the Bazzite-based Hearth OS: Steam installed in the system, its games in ~/.local/share/Steam. Anatase's
# own Steam isn't relied on (Hearth doesn't know its folders). From RPM Fusion when Anatase doesn't have it: its
# nonfree repository (on once rpmfusion-nonfree-release is in). There's no "rpmfusion-nonfree-steam" repository in
# those release packages (that one comes with Fedora Workstation's third-party repos), so asking for it failed the
# install every time and Hearth OS shipped without Steam.
if ! rpm -q steam >/dev/null 2>&1; then
  if { dnf5 -y install "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${FEDORA}.noarch.rpm" \
                       "https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${FEDORA}.noarch.rpm" \
       && dnf5 -y install steam; } > /tmp/steam-install.log 2>&1; then
    echo "Steam installed from RPM Fusion"
  else
    tail -n 15 /tmp/steam-install.log 2>/dev/null || true
    # Then Flathub's Steam at the first start (hearth-steam.service; Hearth knows its folders too). Anatase's own
    # /usr/bin/steam only starts its own Steam Flatpak (org.anatase.Steam), which isn't installed: Hearth's launcher
    # takes its place and starts whichever Steam there is.
    echo "warning: couldn't install Steam from RPM Fusion ($(tail -n 1 /tmp/steam-install.log 2>/dev/null | cut -c1-200)); Flathub's Steam is installed at first boot instead"
    if [ -e /usr/bin/steam ]; then mkdir -p /usr/libexec/nimbus && mv /usr/bin/steam /usr/libexec/nimbus/steam-anatase; fi
    install -Dm755 /ctx/anatase_files/usr/libexec/nimbus/hearth-steam-launch /usr/bin/steam
    mkdir -p /usr/share/nimbus && touch /usr/share/nimbus/steam-flatpak
    systemctl enable hearth-steam.service
  fi
  # The repositories aren't left on for the installed system (the image brings updates).
  for r in /etc/yum.repos.d/rpmfusion-*.repo; do
    if [ -f "$r" ]; then sed -i 's/^enabled=1/enabled=0/' "$r"; fi
  done
fi

# ------------------------------------------------------------------ Beacon (BFME, Age of the Ring, LOTR: Conquest...)
# Beacon's release package: `beacon`, the tool Hearth's library talks to (hosts/hearth-core/src/beacon.rs), and
# `beacon-app`, its window (beacon.desktop: in Mantel's launcher and pinned to its taskbar). See docs/BEACON.md.
# Optional: when there's no release to download yet, the build says so and carries on.
BEACON_URL=https://github.com/Dragongamer9/beacon/releases/latest/download
if [ "$(uname -m)" = x86_64 ] && dnf5 -y install "$BEACON_URL/beacon.x86_64.rpm"; then
  echo "Beacon installed"
else
  echo "warning: couldn't install Beacon from $BEACON_URL (no release yet?); Hearth works without it"
fi

# ------------------------------------------------------------------ Sunshine (Multi-seat's streamed players, Remote play)
# Settings › Multi-seat can stream an extra player's own Game Mode to Moonlight on a TV, phone, tablet or laptop (one
# Sunshine per streamed player, see docs/MULTI-SEAT.md); Settings › Remote play uses it too. LizardByte's COPR
# (the package is "Sunshine"), else the release's Fedora RPM. Optional: without it Hearth says streaming isn't available.
if ! rpm -q Sunshine >/dev/null 2>&1 && ! rpm -q sunshine >/dev/null 2>&1; then
  if dnf5 -y copr enable lizardbyte/stable && dnf5 -y install Sunshine; then
    echo "Sunshine installed (LizardByte COPR)"
  elif dnf5 -y copr enable lizardbyte/beta && dnf5 -y install Sunshine; then
    echo "Sunshine installed (LizardByte beta COPR)"
  else
    SUNSHINE_RPM="$(curl -fsSL --retry 3 https://api.github.com/repos/LizardByte/Sunshine/releases/latest 2>/dev/null \
      | python3 -c 'import json, re, sys; print(next((a["browser_download_url"] for a in json.load(sys.stdin).get("assets", []) if re.search(r"\.fc" + sys.argv[1] + r"\.x86_64\.rpm$", a["name"])), ""))' "$FEDORA" 2>/dev/null || true)"
    if [ "$(uname -m)" = x86_64 ] && [ -n "$SUNSHINE_RPM" ] && dnf5 -y install "$SUNSHINE_RPM"; then
      echo "Sunshine installed ($SUNSHINE_RPM)"
    else
      echo "warning: couldn't install Sunshine; streamed multi-seat players and Remote play show as unavailable"
    fi
  fi
  dnf5 -y copr disable lizardbyte/stable >/dev/null 2>&1 || true
  dnf5 -y copr disable lizardbyte/beta >/dev/null 2>&1 || true
fi
# The streamed players' address as a QR code in Settings (optional), and the udev helper that gives each streamed
# player their own virtual controllers (copied in with system_files; executable to be sure).
dnf5 -y install qrencode || echo "warning: qrencode isn't available (Settings shows the address without a QR code)"
chmod 755 /usr/libexec/nimbus/hearth-seat-input 2>/dev/null || true

# ------------------------------------------------------------------ Handheld Daemon
# Anatase has it; Hearth's drop-in (hhd.service.d/zz-hearth.conf) turns off its updater and its Game Mode autologin.
if [ -f /usr/lib/systemd/system/hhd.service ]; then
  systemctl enable hhd.service || true
else
  echo "warning: Handheld Daemon (hhd.service) isn't in this Anatase image; built-in controllers fall back to plain evdev"
fi

# ------------------------------------------------------------------ Quick Access over games: Handheld Daemon's overlay
# HHD shows an overlay app over games (its QAM button) and holds the controllers for it. Here that app is Hearth's own
# Quick Access (desktop/shell/hearth-overlay: hosts/linux/src/overlay in Hearth Browser's Electron), installed as
# hhd-ui, and HHD_OVERLAY points at it so it wins over any other hhd-ui (HHD's own, if Anatase brings it).
if rpm -q hhd-ui >/dev/null 2>&1; then rpm -e --nodeps hhd-ui || true; fi
rm -f /usr/bin/hhd-ui.AppImage /usr/bin/hhd-ui-dbg
chmod 755 /usr/share/nimbus/desktop/shell/hearth-overlay
ln -sfn /usr/share/nimbus/desktop/shell/hearth-overlay /usr/bin/hhd-ui
mkdir -p /usr/lib/systemd/system/hhd.service.d
cat > /usr/lib/systemd/system/hhd.service.d/zz-hearth-overlay.conf <<'CONF'
# Hearth: Handheld Daemon's overlay app is Hearth's Quick Access (see os-bases/*/build.sh).
[Service]
Environment=HHD_OVERLAY=/usr/share/nimbus/desktop/shell/hearth-overlay
CONF

# ------------------------------------------------------------------ power button
# Hearth handles the power button in its session (Settings › Power button), not Steam's helper.
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ Hearth Settings and the Hearth look
cat > /usr/share/applications/hearth-files.desktop <<DESKTOP
[Desktop Entry]
Name=Files
GenericName=File Manager
Comment=Browse, copy and move your files, games, USB drives and SD cards
Exec=/usr/bin/hearth-filemanager %U
Icon=hearth
Type=Application
Categories=System;FileManager;Utility;
MimeType=inode/directory;
Keywords=files;folder;explorer;browse;usb;sd;card;copy;move;
OnlyShowIn=Nimbus;
DESKTOP
# Folders open in Hearth Files in Desktop Mode (XDG_CURRENT_DESKTOP=Nimbus); Plasma keeps Dolphin.
mkdir -p /etc/xdg
printf '[Default Applications]\ninode/directory=hearth-files.desktop\n' > /etc/xdg/nimbus-mimeapps.list
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
cat > /usr/share/applications/hearth-apps.desktop <<DESKTOP
[Desktop Entry]
Name=Apps
GenericName=App Store
Comment=Find, install and update apps from Flathub
Exec=quickshell ipc -p /usr/share/nimbus/desktop/shell call apps open
Icon=hearth
Type=Application
Categories=System;PackageManager;
Keywords=store;software;install;update;flathub;flatpak;
OnlyShowIn=Nimbus;
DESKTOP
cat > /usr/share/applications/hearth-browser.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Browser
GenericName=Web Browser
Comment=Browse the web: Firefox with Hearth's settings
Exec=/usr/bin/hearth-browser %U
Icon=hearth-browser
Type=Application
Categories=Network;WebBrowser;
MimeType=text/html;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=hearth-browser
StartupNotify=true
Actions=new-window;private-window;
Keywords=web;internet;browser;firefox;

[Desktop Action new-window]
Name=New window
Exec=/usr/bin/hearth-browser

[Desktop Action private-window]
Name=New private window
Exec=/usr/bin/hearth-browser --private-window
DESKTOP
# ------------------------------------------------------------------ Hearth Browser (Firefox)
# Hearth Browser is Mozilla's Firefox, unchanged, with Hearth's settings (os/system_files/usr/share/hearth-browser):
# install-firefox puts the newest release, checked against Mozilla's signed SHA256SUMS, in
# /usr/lib/hearth-browser/firefox and adds Hearth's parts (policies, look, start page, on-screen keyboard). It updates
# with the image (Firefox's own updater is off). /usr/bin/hearth-browser starts it with its own profile. The Electron
# build in /usr/lib/hearth-browser stays: it's the Quick Access overlay app (hearth-overlay).
python3 /usr/share/hearth-browser/install-firefox /usr/lib/hearth-browser/firefox

# Firefox's own libraries come with it; the system ones it links (GTK 3, X11, D-Bus...) must be in the image. Any
# still missing are looked up by file name and installed, so a new Firefox needing one more never ships broken.
ff_missing() { LD_LIBRARY_PATH=/usr/lib/hearth-browser/firefox ldd /usr/lib/hearth-browser/firefox/libxul.so /usr/lib/hearth-browser/firefox/firefox 2>/dev/null | awk '/not found/{print $1}' | sort -u; }   # Firefox's own libraries sit next to it
dnf5 -y install --setopt=install_weak_deps=False gtk3 dbus-glib dbus-libs libX11-xcb libXcomposite libXdamage libXrandr libXtst libXt alsa-lib pango cairo-gobject || true
for l in $(ff_missing); do dnf5 -y install --setopt=install_weak_deps=False "$l()(64bit)" || true; done
[ -z "$(ff_missing)" ] || echo "warning: Hearth Browser's Firefox is still missing: $(ff_missing | tr '\n' ' ')"
chmod 755 /usr/bin/hearth-browser /usr/share/hearth-browser/install-firefox
# Links open in it in Desktop Mode and Game Mode (XDG_CURRENT_DESKTOP=Nimbus), unless you pick another browser.
printf 'x-scheme-handler/http=hearth-browser.desktop\nx-scheme-handler/https=hearth-browser.desktop\ntext/html=hearth-browser.desktop\napplication/xhtml+xml=hearth-browser.desktop\n' >> /etc/xdg/nimbus-mimeapps.list
install -Dm644 /usr/share/nimbus/desktop/theme/Hearth.colors /usr/share/color-schemes/Hearth.colors
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
mkdir -p /usr/share/xdg-desktop-portal
cat > /usr/share/xdg-desktop-portal/nimbus-portals.conf <<PORTALS
[preferred]
default=gtk
org.freedesktop.impl.portal.ScreenCast=wlr
org.freedesktop.impl.portal.Screenshot=wlr
org.freedesktop.impl.portal.FileChooser=kde
PORTALS

# ------------------------------------------------------------------ boot straight into Hearth
# nimbus-autologin writes the login screen's autologin: Plasma Login Manager's /etc/plasmalogin.conf on Anatase
# (and SDDM's file, harmless when SDDM isn't there).
systemctl enable nimbus-autologin.service
systemctl enable hearth-greeter-pad.service   # the controller on the login screen
systemctl enable hearth-boot-first.service    # Hearth stays first in the boot order
systemctl enable hearth-groups.service        # accounts in video (screen brightness)
systemctl enable hearth-bluetooth-labels.service  # Bluetooth pairings readable under SELinux before bluetoothd
# The first start after moving here from the Bazzite-based Hearth OS: a clean install that keeps your games
# (os/system_files; does nothing without the plan the old system leaves).
systemctl enable hearth-clean-install.service

# ------------------------------------------------------------------ Quick Resume through turning off and restarting
# Quick Resume keeps games through Shut down and Restart by hibernating (hearth-admin hibernate-setup makes the room,
# hibernate-reboot restarts with them kept; the sleep hook hearth-restart-keep comes from os/system_files).
chmod 755 /usr/lib/systemd/system-sleep/hearth-restart-keep
# Nvidia: the driver must save the graphics card's memory before the PC turns off (PreserveVideoMemoryAllocations),
# to a folder on the drive with room for it (/var/tmp; /tmp is in memory), and its helpers must run around sleep and
# hibernation, or the picture comes back black or broken (RTX desktops). Written before the boot image (dracut,
# below) so the setting is in it too. Harmless without an Nvidia card (the module never loads).
echo "Nvidia power management Anatase brings: $(grep -rhs 'NVreg_PreserveVideoMemoryAllocations\|NVreg_TemporaryFilePath\|NVreg_UseKernelSuspendNotifiers' /usr/lib/modprobe.d /etc/modprobe.d | tr '\n' ' ')"
mkdir -p /usr/lib/modprobe.d
cat > /usr/lib/modprobe.d/zz-hearth-nvidia-power.conf <<'CONF'
# Hearth: Quick Resume keeps games through sleep and turning off (hibernation). Nvidia's driver saves the graphics
# card's memory first, to the drive (/var/tmp: /tmp is in memory and lost when the PC turns off).
options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp
CONF
NV_HELPERS=0
for s in nvidia-suspend nvidia-resume nvidia-hibernate nvidia-suspend-then-hibernate; do
  if [ -f "/usr/lib/systemd/system/$s.service" ]; then
    echo "$s.service: $(systemctl is-enabled "$s.service" 2>&1 || true) in Anatase"
    mkdir -p "/usr/lib/systemd/system/$s.service.d"
    printf '[Unit]\nConditionPathExists=/proc/driver/nvidia\n' > "/usr/lib/systemd/system/$s.service.d/10-hearth-only-nvidia.conf"
    systemctl enable "$s.service" || true
    NV_HELPERS=1
  fi
done
if [ "$NV_HELPERS" = 1 ]; then
  # systemd 256 and later freeze the session before sleeping, which Nvidia's helpers can't work with (a black screen
  # after waking). Nvidia's own packages turn that off; done here too when Anatase's don't. Hearth's put-aside games
  # are paused with SIGSTOP, which this doesn't touch.
  if ! grep -rqs SYSTEMD_SLEEP_FREEZE_USER_SESSIONS /usr/lib/systemd/system/systemd-suspend.service.d /usr/lib/systemd/system/systemd-hibernate.service.d; then
    for u in systemd-suspend systemd-hibernate systemd-hybrid-sleep systemd-suspend-then-hibernate; do
      mkdir -p "/usr/lib/systemd/system/$u.service.d"
      printf '# Hearth: Nvidia helpers need the session running while they save the graphics memory.\n[Service]\nEnvironment="SYSTEMD_SLEEP_FREEZE_USER_SESSIONS=false"\n' > "/usr/lib/systemd/system/$u.service.d/10-hearth-nvidia.conf"
    done
    echo "Nvidia: session freezing during sleep turned off (Anatase's packages didn't)"
  fi
else
  # Anatase's driver has none (checked in CI): it saves the graphics memory itself from the kernel's suspend
  # notifications, and Anatase already sets PreserveVideoMemoryAllocations=1 with /var/tmp.
  echo "Nvidia's sleep helpers (nvidia-suspend/hibernate/resume.service) aren't in this image: the driver handles sleep and hibernation itself"
fi

# ------------------------------------------------------------------ boot splash
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
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (based on Anatase)"/' \
  /usr/lib/os-release

dnf5 clean all

# bootc keeps images in podman's storage (imgstorage): without podman, updates and Switch edition fail
# ("podman executable not found in PATH"). Peppermint 0.3.0 shipped without it.
command -v podman >/dev/null || { echo "podman is missing: bootc upgrade and bootc switch need it"; exit 1; }
