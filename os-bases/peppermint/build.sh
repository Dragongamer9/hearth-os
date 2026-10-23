#!/bin/bash
# Turns Debian 13 "trixie" (Peppermint OS's base, prepared by os-bases/peppermint/Containerfile) into Hearth OS as
# a bootc image: "Hearth OS (Peppermint edition)", Beta Experimental. Runs once, inside the image build.
# The Hearth parts follow os/build_files/build.sh (the Bazzite build) and os-bases/cachyos/build.sh as closely as
# possible; what's different is the base: apt instead of dnf/pacman, everything Bazzite already carried installed
# here, and the bootc layout bootcrew's debian-bootc uses. See docs/BASE-PEPPERMINT.md.
set -ouex pipefail

NVIDIA="${NVIDIA:-1}"
PEPPERMINT_DESKTOP="${PEPPERMINT_DESKTOP:-1}"
BPO=trixie-backports
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get install -y --no-install-recommends -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold)
OPTIONAL_MISSING=()

have_pkg() { apt-cache show "$1" >/dev/null 2>&1; }
# Something nice to have that may not be in the repositories (or not under this name): install it if it's there,
# say so if not.
optional() {
  local p
  for p in "$@"; do
    if have_pkg "$p"; then "${APT[@]}" "$p" || OPTIONAL_MISSING+=("$p(failed)"); else OPTIONAL_MISSING+=("$p"); fi
  done
}

# No services start during the build (there's no running system to start them in).
printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d
chmod 755 /usr/sbin/policy-rc.d

apt-get update -y

# ------------------------------------------------------------------ ostree comes from the builder (newer than trixie's)
# The placeholder packages tell dpkg that libostree-1-1, ostree and ostree-boot are there (flatpak depends on
# libostree-1-1); holding them keeps trixie's older ostree from replacing the one bootc needs.
dpkg -i /hearth-debs/*.deb
apt-mark hold libostree-1-1 ostree ostree-boot
# The libraries the builder's programs (ostree, bootc, bootupd, Quickshell, RyzenAdj, HHD) link against.
mapfile -t BUILDER_DEPS < <(grep -v -e '^$' -e '^libostree-1-1$' /usr/share/hearth/builder-runtime-deps.txt)
"${APT[@]}" "${BUILDER_DEPS[@]}"
ldconfig
ostree --version
bootc --version

# ------------------------------------------------------------------ the system: kernel, firmware, boot pieces
# dracut before the kernel, so the kernel's hooks use it (and not initramfs-tools). Until Hearth's own dracut
# settings are copied in (with the system files, below), only "generic": the hooks' initramfs in /boot is thrown
# away, the real one is built at the end into /usr/lib/modules.
mkdir -p /etc/dracut.conf.d
echo 'hostonly="no"' > /etc/dracut.conf.d/00-hearth-build.conf
"${APT[@]}" dracut systemd systemd-sysv systemd-resolved systemd-timesyncd udev kmod dbus
# The kernel from trixie-backports: newer than trixie's 6.12, which handhelds need (ROG Ally / Ally X, Legion Go,
# Legion Go S, Steam Deck OLED, MSI Claw: input, audio and power fixes land in every release). Signed by Debian,
# so it boots with Secure Boot on. Headers: only for building Nvidia's module below.
"${APT[@]}" -t "$BPO" linux-image-amd64 linux-headers-amd64
# Firmware (non-free-firmware): graphics (AMD, Intel, Nvidia GSP), Wi-Fi/Bluetooth chips handhelds and PCs use,
# Intel/AMD sound (SOF: the Steam Deck, Legion Go, MSI Claw), Cirrus amplifiers (ROG Ally speakers), microcode.
"${APT[@]}" -t "$BPO" firmware-linux-free firmware-misc-nonfree firmware-amd-graphics firmware-intel-graphics \
  firmware-iwlwifi firmware-realtek firmware-atheros firmware-mediatek firmware-brcm80211 \
  firmware-intel-sound firmware-sof-signed firmware-cirrus amd64-microcode intel-microcode wireless-regdb || \
  "${APT[@]}" -t "$BPO" firmware-linux firmware-iwlwifi firmware-realtek firmware-atheros firmware-sof-signed \
    amd64-microcode intel-microcode wireless-regdb
optional firmware-nvidia-graphics firmware-libertas
KDIRS=(/usr/lib/modules/*/)
[ "${#KDIRS[@]}" -eq 1 ] || { echo "expected exactly one kernel, found: ${KDIRS[*]}"; exit 1; }
KVER=$(basename "${KDIRS[0]}")
# bootc and ostree look for the kernel next to its modules (Debian puts it in /boot, which the image doesn't keep).
# Newer Debian kernels already keep it there (/boot only links to it).
[ "/boot/vmlinuz-$KVER" -ef "/usr/lib/modules/$KVER/vmlinuz" ] || cp "/boot/vmlinuz-$KVER" "/usr/lib/modules/$KVER/vmlinuz"

"${APT[@]}" \
  ca-certificates curl less nano file sudo locales polkitd pkexec \
  util-linux fdisk parted gdisk efibootmgr btrfs-progs e2fsprogs xfsprogs dosfstools ntfs-3g exfatprogs \
  rsync cpio zstd skopeo podman pciutils usbutils lsof procps psmisc \
  network-manager wpasupplicant iw bluez bluez-obexd openssh-client \
  pipewire pipewire-audio pipewire-pulse pipewire-alsa pipewire-jack wireplumber libspa-0.2-bluetooth alsa-utils \
  upower udisks2 fwupd flatpak xdg-user-dirs xdg-utils \
  plymouth plymouth-themes \
  python3 python3-evdev python3-dbus python3-gi python3-yaml python3-rich python3-xlib python3-serial \
  python3-pyroute2 libusb-1.0-0 \
  shim-signed grub-efi-amd64-signed
optional power-profiles-daemon fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji fonts-liberation fonts-dejavu-core

# ------------------------------------------------------------------ graphics and Steam (AMD, Intel; Nvidia below)
# Mesa from trixie-backports with its 32-bit half (they must be the same version), then Steam (contrib) on top so
# its 32-bit dependencies agree. Steam's licence is accepted here (debconf), the way the Steam Deck ships it.
echo 'steam steam/question select I AGREE' | debconf-set-selections
echo 'steam steam/license note ' | debconf-set-selections
echo 'steam-installer steam/question select I AGREE' | debconf-set-selections
echo 'steam-installer steam/license note ' | debconf-set-selections
"${APT[@]}" -t "$BPO" \
  mesa-vulkan-drivers mesa-vulkan-drivers:i386 libgl1-mesa-dri libgl1-mesa-dri:i386 libglx-mesa0 libglx-mesa0:i386 \
  libegl-mesa0 libgbm1 mesa-va-drivers libvulkan1 libvulkan1:i386 vulkan-tools mesa-utils \
  intel-media-va-driver xwayland \
  steam-installer steam-devices
"${APT[@]}" -t "$BPO" gamescope mangohud mangoapp
# Debian puts games' programs in /usr/games, which isn't on every PATH (the session, the checks): link them.
for b in gamescope gamescopereaper gamescopestream gamescopectl mangoapp mangohud; do
  if [ -e "/usr/games/$b" ] && [ ! -e "/usr/bin/$b" ]; then ln -s "../games/$b" "/usr/bin/$b"; fi
done
command -v gamescope
optional gamemode libgamemode0:i386 zenity
# Steam's launcher lives in /usr/games on Debian; Hearth's session calls `steam` from the usual PATH.
STEAM_BIN=$(command -v steam || ls /usr/games/steam 2>/dev/null || true)
[ -n "$STEAM_BIN" ] || { echo "Steam's launcher wasn't installed"; exit 1; }
[ -e /usr/bin/steam ] || ln -s "$STEAM_BIN" /usr/bin/steam

# ------------------------------------------------------------------ Nvidia: the open kernel module, in the same image
# Debian's own Nvidia packages (550) don't build against trixie-backports' newer kernels, so the driver comes from
# NVIDIA's own repository for Debian 13 (newer driver, the same Debian package names). It only loads on an Nvidia
# card (GeForce GTX 16 series, RTX 20 series and newer: the open module's range). The module is built here for
# the image's kernel with DKMS.
if [ "$NVIDIA" = "1" ]; then
  curl -fsSL --retry 3 -o /tmp/cuda-keyring.deb \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
  dpkg -i /tmp/cuda-keyring.deb
  apt-get update -y
  if have_pkg nvidia-open; then "${APT[@]}" nvidia-open; else "${APT[@]}" nvidia-kernel-open-dkms nvidia-driver; fi
  dkms autoinstall -k "$KVER" || true
  dkms status
  find "/usr/lib/modules/$KVER" -name 'nvidia.ko*' | grep . >/dev/null || {
    echo "Nvidia's open kernel module didn't build for $KVER (see the DKMS output above)"
    find /var/lib/dkms -name make.log -exec tail -40 {} \; 2>/dev/null || true
    exit 1
  }
  # 32-bit Nvidia libraries (bughunt): Steam's own 32-bit parts and 32-bit games need them. Without them glvnd
  # falls back to Mesa's 32-bit GL, which can't drive an Nvidia card ("failed to load driver: nvidia-drm"). They
  # must be the same driver release as the 64-bit ones (they talk to the same kernel module), so that version is
  # asked for by name. Debian's own 32-bit ones are an older release (550) and would be worse than none, so there's
  # no fallback to them. If NVIDIA's repository has no 32-bit build of this release, the image is still built (64-bit
  # games and Steam's 64-bit parts work), with a warning in the build and in the image check.
  NV_VER=$(dpkg-query -W -f='${Version}' nvidia-driver-libs 2>/dev/null || true)
  if [ -n "$NV_VER" ]; then
    "${APT[@]}" "nvidia-driver-libs:i386=$NV_VER" || true
  else
    "${APT[@]}" nvidia-driver-libs:i386 || true
  fi
  if ! ls /usr/lib/i386-linux-gnu/libGLX_nvidia.so.0 >/dev/null 2>&1; then
    echo "::warning::Nvidia's 32-bit libraries (nvidia-driver-libs:i386${NV_VER:+ $NV_VER}) aren't in NVIDIA's repository: 32-bit games and Steam's 32-bit parts fall back to Mesa on Nvidia"
    OPTIONAL_MISSING+=("nvidia-driver-libs:i386${NV_VER:+=$NV_VER}")
  fi
  # Nvidia's suspend helpers only where an Nvidia driver is loaded.
  for s in nvidia-suspend nvidia-resume nvidia-hibernate nvidia-suspend-then-hibernate; do
    if [ -f "/usr/lib/systemd/system/$s.service" ]; then
      mkdir -p "/usr/lib/systemd/system/$s.service.d"
      printf '[Unit]\nConditionPathExists=/proc/driver/nvidia\n' > "/usr/lib/systemd/system/$s.service.d/10-hearth-only-nvidia.conf"
      systemctl enable "$s.service" || true
    fi
  done
  # Nvidia's persistence daemon isn't needed on a desktop/handheld (and fails without a card).
  systemctl disable nvidia-persistenced.service 2>/dev/null || true
fi
# acpi_call: Handheld Daemon sets the Legion Go's power limits through it. Built for the image's kernel too.
if have_pkg acpi-call-dkms; then
  "${APT[@]}" acpi-call-dkms && dkms autoinstall -k "$KVER" || OPTIONAL_MISSING+=("acpi-call-dkms(failed)")
fi

# ------------------------------------------------------------------ Handheld Daemon (controllers, TDP, power button)
# Built by the builder from hhd-dev/hhd (tag in builder.sh) into /usr/lib/hhd, its Python dependencies installed
# above from Debian. Hearth uses HHD, not InputPlumber (not in Debian anyway), and picks it up at runtime.
test -x /usr/bin/hhd
/usr/lib/hhd/bin/python3 -c 'import hhd, adjustor, evdev, yaml, rich, Xlib, serial, pyroute2, gi, dbus; print("hhd", hhd.__file__)'
systemctl enable hhd.service

# ------------------------------------------------------------------ Desktop Mode: labwc + Quickshell (+ what Bazzite carried)
"${APT[@]}" -t "$BPO" labwc
"${APT[@]}" grim slurp brightnessctl wlr-randr wlsunset xdotool x11-utils x11-xserver-utils \
  gnome-keyring libpam-gnome-keyring qt6-wayland \
  xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk foot fuse-overlayfs bubblewrap \
  libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav libnss3 libgbm1
# The rest of what Desktop Mode, Couch Co-op and Hearth Browser (Electron) use; names that can differ.
optional qt6-svg-plugins qt5ct breeze-icon-theme 7zip gstreamer1.0-plugins-bad gstreamer1.0-pipewire \
  libasound2t64 libcups2t64 libgtk-3-0t64 libxss1 libatk-bridge2.0-0t64
# Quickshell's QML modules (Hearth's shell imports QtQuick, Layouts, Particles, Window; Quickshell itself the rest).
optional qt6ct qml6-module-qtquick qml6-module-qtquick-layouts qml6-module-qtquick-particles qml6-module-qtquick-window \
  qml6-module-qtquick-controls qml6-module-qtquick-templates qml6-module-qtquick-shapes qml6-module-qtquick-effects \
  qml6-module-qtqml-workerscript qml6-module-qt5compat-graphicaleffects qml6-module-qtquick-dialogs
# Printing (Settings › Printers: CUPS, driverless IPP Everywhere, ipp-usb, Avahi) and clipboard history (Super+V).
optional cups cups-filters ipp-usb avahi-daemon libnss-mdns wl-clipboard cliphist wtype
# A polkit agent for Desktop Mode's password prompts. Debian keeps polkit-gnome's in a folder labwc's autostart
# doesn't look in: link it where it does.
optional policykit-1-gnome
if [ -x /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 ]; then
  ln -sf /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 /usr/libexec/polkit-gnome-authentication-agent-1
else
  optional lxqt-policykit
fi

# ------------------------------------------------------------------ the login screen (SDDM, as on Bazzite)
# nimbus-autologin writes SDDM's autologin; the Hearth theme (os/system_files) is Qt 6, as trixie's SDDM is.
# SDDM's login screen itself runs on X (its default), so the X server comes along.
"${APT[@]}" sddm xserver-xorg-core xserver-xorg-input-libinput
optional qml6-module-qtquick-virtualkeyboard qml6-module-qtmultimedia

# ------------------------------------------------------------------ Peppermint's desktop (optional)
# Xfce, Peppermint's own desktop, as a second session on the login screen: what Peppermint users know, and a way
# back to a plain desktop if Hearth's session misbehaves. Minimal (no recommends): roughly 150 MB. Hearth's
# session stays the default and the autologin. PEPPERMINT_DESKTOP=0 leaves it out.
if [ "$PEPPERMINT_DESKTOP" = "1" ]; then
  "${APT[@]}" xfce4-session xfwm4 xfce4-panel xfdesktop4 xfce4-settings xfce4-terminal thunar
  optional xserver-xorg-video-amdgpu xfce4-whiskermenu-plugin xfce4-pulseaudio-plugin xfce4-notifyd network-manager-gnome
  # Shown as "Peppermint desktop (Xfce)" so it's clear what it is.
  if [ -f /usr/share/xsessions/xfce.desktop ]; then
    sed -i -e 's/^Name=.*/Name=Peppermint desktop (Xfce)/' /usr/share/xsessions/xfce.desktop
  fi
fi

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's Debian-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/peppermint_files/. /
# Apps you add with apt or a .deb file, kept through restarts and updates (shared with the other Debian-family
# edition; docs/APT-LAYER.md). Set up for real at the end (image-setup.sh), after the last package.
cp -avf /ctx/apt_layer/system_files/. /
chmod 440 /etc/sudoers.d/hearth-wheel

# ------------------------------------------------------------------ Hearth itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit (libwebkit2gtk-4.1-0, installed above).
  install -Dm755 /ctx/nimbus-app/hearth /usr/lib/nimbus/nimbus
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
if [ -f /usr/lib/nimbus/chrome-sandbox ]; then chown root:root /usr/lib/nimbus/chrome-sandbox && chmod 4755 /usr/lib/nimbus/chrome-sandbox; fi
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
# ---- hearthfetch (hosts/hearthfetch): Hearth OS's system summary for the terminal, built with the app.
if [ -x /ctx/nimbus-app/hearthfetch ]; then install -Dm755 /ctx/nimbus-app/hearthfetch /usr/bin/hearthfetch; fi
# ---- end hearthfetch

# ------------------------------------------------------------------ SVG icons for GTK (librsvg's gdk-pixbuf loader)
# Without it GTK can't draw an SVG icon and aborts the whole program the first time it needs one (Hearth's Game Mode
# crashed over and over on CachyOS: "image-missing.svg: Unrecognized image file format"). The loaders list is
# rebuilt so the SVG loader is in it.
"${APT[@]}" librsvg2-common libgdk-pixbuf2.0-bin
# Newer gdk-pixbuf loads SVG (and other images) through glycin's loaders instead.
"${APT[@]}" glycin-loaders || true
for q in /usr/bin/gdk-pixbuf-query-loaders /usr/bin/gdk-pixbuf-query-loaders-64 /usr/lib/*/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders; do
  [ -x "$q" ] && { "$q" --update-cache || true; break; }
done

# ------------------------------------------------------------------ Hearth Disks (hosts/hearth-disks)
# The disk manager, built with the app (base-peppermint.yml). udisks2 does all its changes (installed with the base packages above).
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
# CI builds the app in a debian:trixie container, so every library it links must be here under the same name.
if ldd /usr/lib/nimbus/nimbus | grep "not found"; then
  echo "The Hearth app needs libraries this image doesn't have (above)"; exit 1
fi

# ---- Hearth Terminal (hosts/hearth-terminal): built by CI next to the Hearth app (base-*.yml) and carried in
# os/nimbus-app. Mantel opens it for its terminal (desktop/open.sh); its menu entry (hearth-terminal.desktop) and
# Mantel's xdg-terminal-exec list come from os/system_files. foot stays installed as the fallback.
if [ -x /ctx/nimbus-app/hearth-terminal ]; then
  install -Dm755 /ctx/nimbus-app/hearth-terminal /usr/bin/hearth-terminal
  # A terminal that can't start is worse than none: then Mantel falls back to foot.
  if ldd /usr/bin/hearth-terminal | grep 'not found'; then echo "Hearth Terminal needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/hearth-terminal; fi
  # Debian's "the terminal" for apps that ask for one (foot stays registered as the fallback).
  if [ -x /usr/bin/hearth-terminal ]; then update-alternatives --install /usr/bin/x-terminal-emulator x-terminal-emulator /usr/bin/hearth-terminal 60 || true; fi
fi
# ---- end of Hearth Terminal

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh /usr/share/nimbus/desktop/shell/hearth-pad /usr/share/nimbus/desktop/shell/hearth-buttons /usr/share/nimbus/desktop/shell/hearth-controller /usr/share/nimbus/desktop/shell/hearth-power /usr/share/nimbus/desktop/shell/hearth-browser /usr/share/nimbus/desktop/shell/hearth-tasks /usr/share/nimbus/desktop/shell/hearth-files /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper
# Hearth Input (controller layouts for games; python3-evdev and the uinput rule are already in the image).
if [ -f /usr/share/nimbus/desktop/shell/hearth-input ]; then chmod 755 /usr/share/nimbus/desktop/shell/hearth-input; fi
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Quick Access over games: Handheld Daemon shows an overlay app over games (its QAM button) and holds the controllers
# for it. Here that app is Hearth's own Quick Access (desktop/shell/hearth-overlay: hosts/linux/src/overlay in Hearth
# Browser's Electron), installed as hhd-ui, and HHD_OVERLAY points at it so it wins over any other hhd-ui.
rm -f /usr/bin/hhd-ui.AppImage /usr/bin/hhd-ui-dbg
chmod 755 /usr/share/nimbus/desktop/shell/hearth-overlay
ln -sfn /usr/share/nimbus/desktop/shell/hearth-overlay /usr/bin/hhd-ui
mkdir -p /usr/lib/systemd/system/hhd.service.d
cat > /usr/lib/systemd/system/hhd.service.d/zz-hearth-overlay.conf <<'CONF'
# Hearth: Handheld Daemon's overlay app is Hearth's Quick Access (see os-bases/*/build.sh).
[Service]
Environment=HHD_OVERLAY=/usr/share/nimbus/desktop/shell/hearth-overlay
CONF
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; the CachyOS build
# writes this same file).
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-peppermint", "image-ref": "ghcr.io/dragongamer9/hearth-peppermint", "image-tag": "testing", "base": "peppermint", "base-detail": "Debian 13 trixie (Peppermint OS base)", "kernel": "${KVER}", "nvidia": ${NVIDIA}, "version": "${NIMBUS_VERSION:-0.0.0}"}
INFO

cat > /usr/share/applications/nimbus.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth
Comment=Controller-first game library
Exec=/usr/bin/nimbus --windowed
Icon=hearth
Type=Application
Categories=Game;
DESKTOP

# ------------------------------------------------------------------ Hearth Settings and the Hearth look
# (the same as os/build_files/build.sh, except the file dialogs: no Plasma here, so GTK's)
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
"${APT[@]}" libgtk-3-0t64 libdbus-glib-1-2 libx11-xcb1 libxcomposite1 libxdamage1 libxrandr2 libxtst6 libxt6t64 libasound2t64 || "${APT[@]}" libgtk-3-0 libdbus-glib-1-2 libx11-xcb1 libxcomposite1 libxdamage1 libxrandr2 libxtst6 libxt6 libasound2 || true
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
done
mkdir -p /usr/share/xdg-desktop-portal
cat > /usr/share/xdg-desktop-portal/nimbus-portals.conf <<PORTALS
[preferred]
default=gtk
org.freedesktop.impl.portal.ScreenCast=wlr
org.freedesktop.impl.portal.Screenshot=wlr
org.freedesktop.impl.portal.FileChooser=gtk
PORTALS

# ------------------------------------------------------------------ Beacon (BFME, Age of the Ring, LOTR: Conquest...)
# Beacon's release package: `beacon`, the tool Hearth's library talks to (hosts/hearth-core/src/beacon.rs), and
# `beacon-app`, its window (beacon.desktop: in Mantel's launcher and pinned to its taskbar). See docs/BEACON.md.
# Optional: when there's no release to download yet, the build says so and carries on.
BEACON_URL=https://github.com/Dragongamer9/beacon/releases/latest/download
if [ "$(uname -m)" = x86_64 ] && curl -fsSL --retry 3 -o /tmp/beacon_amd64.deb "$BEACON_URL/beacon_amd64.deb" \
   && "${APT[@]}" /tmp/beacon_amd64.deb; then
  echo "Beacon installed"
else
  echo "warning: couldn't install Beacon from $BEACON_URL (no release yet?); Hearth works without it"
fi
rm -f /tmp/beacon_amd64.deb

# ------------------------------------------------------------------ Sunshine (Multi-seat's streamed players, Remote play)
# Settings › Multi-seat can stream an extra player's own Game Mode to Moonlight on a TV, phone, tablet or laptop (one
# Sunshine per streamed player, see docs/MULTI-SEAT.md); Settings › Remote play uses it too. The archive's package
# when there is one, else the .deb LizardByte builds for this release (sunshine_<version>-1+<distro><version>_amd64.deb).
# Optional: without it Hearth says streaming isn't available.
sunshine_deb() {
  local want url
  # LizardByte names its Debian packages by codename (sunshine_<version>-1+debiantrixie_amd64.deb: the release field is
  # "1+<distro><docker tag>", and Debian's tags are codenames), Ubuntu's by number (+ubuntu26.04). Only "debian13" was
  # looked for, so Peppermint never found its package.
  want="$(. /etc/os-release && echo "${ID}${VERSION_ID}")"
  want2="$(. /etc/os-release && if [ -n "${VERSION_CODENAME:-}" ]; then echo "${ID}${VERSION_CODENAME}"; fi)"
  url="$(curl -fsSL --retry 3 https://api.github.com/repos/LizardByte/Sunshine/releases/latest \
    | python3 -c 'import json, re, sys; ws = [re.escape(w) for w in sys.argv[1:] if w]; assets = json.load(sys.stdin).get("assets", []); print(next((a["browser_download_url"] for w in ws for a in assets if re.search(w + r"_amd64\.deb$", a["name"]) or re.search(w + r".*amd64\.deb$", a["name"].replace("-", ""))), ""))' "$want" "$want2")" || return 1
  [ -n "$url" ] || return 1
  curl -fsSL --retry 3 -o /tmp/sunshine.deb "$url" && "${APT[@]}" /tmp/sunshine.deb
}
if [ "$(uname -m)" = x86_64 ] && { { have_pkg sunshine && "${APT[@]}" sunshine; } || sunshine_deb; }; then
  echo "Sunshine installed"
else
  echo "warning: couldn't install Sunshine; streamed multi-seat players and Remote play show as unavailable"
  OPTIONAL_MISSING+=("sunshine")
fi
rm -f /tmp/sunshine.deb
# The streamed players' address as a QR code in Settings (optional), and the udev helper that gives each streamed
# player their own virtual controllers (copied in with system_files; executable to be sure).
optional qrencode
chmod 755 /usr/libexec/nimbus/hearth-seat-input 2>/dev/null || true

# ------------------------------------------------------------------ system settings Bazzite/Fedora came with
# Hearth's administrators group (the installer puts the first account in it; the polkit rules use it). In
# /etc/group now (the installer adds the account against the image's /etc) and in sysusers.d for later.
getent group wheel >/dev/null || groupadd -r wheel
# The device's own name resolves (sudo warned "unable to resolve host <name>" without it): nss-myhostname answers
# for the hostname whatever /etc/hosts says (the installer and hearth-admin hostname also write 127.0.1.1 there).
"${APT[@]}" libnss-myhostname
if ! grep -Eq '^hosts:.*[[:space:]]myhostname([[:space:]]|$)' /etc/nsswitch.conf; then
  if grep -Eq '^hosts:.*[[:space:]]files([[:space:]]|$)' /etc/nsswitch.conf; then
    sed -i -E '/^hosts:/ s/([[:space:]])files([[:space:]]|$)/\1files myhostname\2/' /etc/nsswitch.conf
  else
    sed -i -E 's/^hosts:(.*)$/hosts:\1 myhostname/' /etc/nsswitch.conf
  fi
fi
grep '^hosts:' /etc/nsswitch.conf | grep -qw myhostname || { echo "nsswitch.conf has no myhostname"; exit 1; }
sed -i 's/^#\s*\(en_US.UTF-8 UTF-8\)/\1/' /etc/locale.gen && locale-gen
echo 'LANG=en_US.UTF-8' > /etc/locale.conf
echo 'LANG=en_US.UTF-8' > /etc/default/locale
# Flathub for system apps (Hearth's updates update them).
mkdir -p /etc/flatpak/remotes.d
curl -fsSL --retry 3 -o /etc/flatpak/remotes.d/flathub.flatpakrepo https://dl.flathub.org/repo/flathub.flatpakrepo || \
  echo "warning: couldn't fetch Flathub's remote file"
systemctl mask systemd-firstboot.service
# Debian's network config tool isn't used: NetworkManager manages every device.
rm -f /etc/network/interfaces

# ------------------------------------------------------------------ power button
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ services
systemctl enable NetworkManager.service bluetooth.service systemd-resolved.service systemd-timesyncd.service \
  fstrim.timer bootc-fetch-apply-updates.timer sddm.service
systemctl enable hearth-bls-grub.service hearth-bls-grub-refresh.path
# power-profiles-daemon only runs when HHD is switched off (its drop-in says so); enabled so it can then.
systemctl enable power-profiles-daemon.service 2>/dev/null || true
# Printing starts when something prints (Settings › Printers).
systemctl enable cups.socket 2>/dev/null || true
# SDDM is the display manager (Xfce's packages may have pulled in another).
ln -sf /usr/lib/systemd/system/sddm.service /etc/systemd/system/display-manager.service
if [ -d /etc/X11 ]; then echo /usr/bin/sddm > /etc/X11/default-display-manager; fi

# ------------------------------------------------------------------ boot straight into Hearth
systemctl enable nimbus-autologin.service
systemctl enable hearth-greeter-pad.service
systemctl enable hearth-boot-first.service
systemctl enable hearth-groups.service        # accounts in video (screen brightness)
# The first start after moving here from the Bazzite-based Hearth OS: a clean install that keeps your games
# (os/system_files; does nothing without the plan the old system leaves).
systemctl enable hearth-clean-install.service

# ------------------------------------------------------------------ name
# /etc/os-release links to /usr/lib/os-release on Debian. ID stays debian (VERSION_CODENAME trixie) so tools
# still know what it is underneath.
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (Peppermint edition, Debian 13)"/' \
  /usr/lib/os-release
grep -q '^VARIANT_ID=' /usr/lib/os-release || printf 'VARIANT="Peppermint edition"\nVARIANT_ID=hearth-peppermint\n' >> /usr/lib/os-release
grep -q '^ID_LIKE=' /usr/lib/os-release || echo 'ID_LIKE=debian' >> /usr/lib/os-release

# ------------------------------------------------------------------ boot loader for Hearth's installer (bootupd)
# bootupd copies the boot loader from /usr/lib/efi/<component>/<version>/EFI (the layout Fedora uses too) onto
# the EFI partition. Debian's signed shim and GRUB (Microsoft → Debian → Debian's kernel: Secure Boot works) go
# in EFI/debian: Debian's signed GRUB has that folder built in as where it finds grub.cfg.
SHIM_VER=$(dpkg-query -W -f '${Version}' shim-signed | tr ':~' '._')
GRUB_VER=$(dpkg-query -W -f '${Version}' grub-efi-amd64-signed | tr ':~' '._')
pick() { local f; for f in "$@"; do [ -f "$f" ] && { echo "$f"; return 0; }; done; return 1; }
SHIM=$(pick /usr/lib/shim/shimx64.efi.signed /usr/lib/shim/shimx64.efi.signed.latest) || { echo "no signed shim"; exit 1; }
MOK=$(pick /usr/lib/shim/mmx64.efi.signed /usr/lib/shim/mmx64.efi) || MOK=
FB=$(pick /usr/lib/shim/fbx64.efi.signed /usr/lib/shim/fbx64.efi) || FB=
GRUBEFI=$(pick /usr/lib/grub/x86_64-efi-signed/grubx64.efi.signed) || { echo "no signed GRUB"; exit 1; }
S="/usr/lib/efi/shim/$SHIM_VER/EFI"
G="/usr/lib/efi/grub2/$GRUB_VER/EFI"
install -Dm644 "$SHIM" "$S/debian/shimx64.efi"
install -Dm644 "$SHIM" "$S/BOOT/BOOTX64.EFI"
if [ -n "$MOK" ]; then install -Dm644 "$MOK" "$S/debian/mmx64.efi"; install -Dm644 "$MOK" "$S/BOOT/mmx64.efi"; fi
if [ -n "$FB" ]; then install -Dm644 "$FB" "$S/BOOT/fbx64.efi"; fi
if [ -f /usr/lib/shim/BOOTX64.CSV ]; then
  install -Dm644 /usr/lib/shim/BOOTX64.CSV "$S/debian/BOOTX64.CSV"
else
  # The fallback loader's list of what to start: "shimx64.efi,<label>,,<comment>" in UTF-16.
  printf 'shimx64.efi,Hearth OS,,Hearth OS\n' | iconv -t UTF-16 > "$S/debian/BOOTX64.CSV"
fi
install -Dm644 "$GRUBEFI" "$G/debian/grubx64.efi"
find /usr/lib/efi -type f | sort
bootupctl backend generate-update-metadata /
ls -la /usr/lib/bootupd/updates/

# ------------------------------------------------------------------ boot splash and initramfs
plymouth-set-default-theme hearth
ls -d /usr/lib/dracut/modules.d/*ostree >/dev/null 2>&1 || { echo "ostree's dracut module is missing: this image couldn't boot"; exit 1; }
export DRACUT_NO_XATTR=1
dracut --force --kver "$KVER" "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"
lsinitrd "/usr/lib/modules/$KVER/initramfs.img" > /tmp/initramfs-files.txt
grep -q ostree-prepare-root /tmp/initramfs-files.txt || { echo "the initramfs has no ostree-prepare-root"; exit 1; }
grep -q plymouth /tmp/initramfs-files.txt || echo "warning: the initramfs has no boot splash"

# ------------------------------------------------------------------ tidy up
rm -f /usr/sbin/policy-rc.d
# The C compiler and kernel headers stay (DKMS's packages need them installed); everything apt downloaded goes.
apt-get clean
rm -rf /var/lib/apt/lists/* /boot/*
rm -f /etc/dracut.conf.d/00-hearth-build.conf
# ------------------------------------------------------------------ apps you add (apt and .deb files)
# The image's package list and pins, apt's and dpkg's hooks, the services (docs/APT-LAYER.md). After the last
# package, before dpkg's database moves to /usr/lib/sysimage below.
bash /ctx/apt_layer/image-setup.sh
if [ "${#OPTIONAL_MISSING[@]}" -gt 0 ]; then echo "Not installed (not in the repositories): ${OPTIONAL_MISSING[*]}"; fi

# ------------------------------------------------------------------ system accounts that later images add
# On an image-based system the machine's /etc/passwd and /etc/group are kept across updates (they hold the
# person's account), so an account a newer image's package adds (sddm, a daemon) would never appear there.
# systemd-sysusers creates them at boot from this list (its drop-in in system_files makes it run every boot).
{
  echo "# Written by os-bases/peppermint/build.sh: every system group and account in this image."
  while IFS=: read -r name _ gid _; do
    if [ "$gid" -gt 0 ] && [ "$gid" -lt 1000 ]; then echo "g $name $gid"; fi
  done < /etc/group
  while IFS=: read -r name _ uid gid gecos home shell; do
    if [ "$uid" -gt 0 ] && [ "$uid" -lt 1000 ]; then
      gecos="${gecos%%,*}"; gecos="${gecos//\"/}"
      echo "u $name $uid:$gid \"${gecos:-$name}\" ${home:--} ${shell:--}"
    fi
  done < /etc/passwd
} > /usr/lib/sysusers.d/hearth-image.conf
cat /usr/lib/sysusers.d/hearth-image.conf

# ------------------------------------------------------------------ the bootc layout (bootcrew's debian-bootc)
# /var belongs to the machine: everything packages put there becomes a tmpfiles.d entry (created at boot) and
# the image's /var is left empty. dpkg's database moves to /usr/lib/sysimage/dpkg (linked back from /var/lib/dpkg),
# so the list of installed packages stays with the packages, read-only. /opt contents move to /usr/lib/opt.
mkdir -p /usr/lib/sysimage
mv /var/lib/dpkg /usr/lib/sysimage/dpkg
VARCONF=/usr/lib/tmpfiles.d/hearth-var.conf
: > "$VARCONF"
find /var -mindepth 1 -type d \
  ! -path '/var/cache*' ! -path '/var/log*' ! -path '/var/tmp*' ! -path '/var/run*' ! -path '/var/lock*' \
  ! -path '/var/home*' ! -path '/var/roothome*' ! -path '/var/opt*' ! -path '/var/srv*' \
  ! -path '/var/mnt*' ! -path '/var/usrlocal*' ! -path '/var/lib/apt/lists*' ! -path '/var/lib/dkms*' \
  -printf 'd /var/%P %m %u %g -\n' | sort >> "$VARCONF"
echo 'L+ /var/lib/dpkg - - - - /usr/lib/sysimage/dpkg' >> "$VARCONF"
if [ -d /opt ] && [ -n "$(ls -A /opt 2>/dev/null)" ]; then
  mkdir -p /usr/lib/opt
  for d in /opt/*; do
    mv "$d" /usr/lib/opt/
    echo "L+ /var/opt/$(basename "$d") - - - - /usr/lib/opt/$(basename "$d")" >> "$VARCONF"
  done
fi
sed -i 's|^#\?\s*HOME=.*|HOME=/var/home|' /etc/default/useradd
grep -q '^HOME=/var/home' /etc/default/useradd || echo 'HOME=/var/home' >> /etc/default/useradd
echo "uninitialized" > /etc/machine-id
rm -rf /boot /home /root /usr/local /srv /opt /mnt /var
mkdir -p /sysroot /boot /usr/lib/ostree /var
ln -sT sysroot/ostree /ostree
ln -sT var/roothome /root
ln -sT var/srv /srv
ln -sT var/opt /opt
ln -sT var/mnt /mnt
ln -sT var/home /home
ln -sT ../var/usrlocal /usr/local
{
  for dir in opt home srv mnt usrlocal; do echo "d /var/$dir 0755 root root -"; done
  echo "d /var/roothome 0700 root root -"
  echo "d /run/media 0755 root root -"
} > /usr/lib/tmpfiles.d/bootc-base-dirs.conf

# bootc keeps images in podman's storage (imgstorage): without podman, updates and Switch edition fail
# ("podman executable not found in PATH"). Peppermint 0.3.0 shipped without it.
command -v podman >/dev/null || { echo "podman is missing: bootc upgrade and bootc switch need it"; exit 1; }
