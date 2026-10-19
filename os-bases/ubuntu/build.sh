#!/bin/bash
# Turns Ubuntu 26.04 LTS "Resolute Raccoon" (prepared by os-bases/ubuntu/Containerfile: all components, i386,
# snapd pinned out) into Hearth OS as a bootc image: "Hearth OS (Ubuntu edition)", Beta Experimental. Runs once,
# inside the image build. The Hearth parts follow os/build_files/build.sh (the Bazzite build) and
# os-bases/peppermint/build.sh (Debian 13, proven in CI) as closely as possible; what's Ubuntu-specific is
# marked. See docs/BASE-UBUNTU.md.
set -ouex pipefail

NVIDIA="${NVIDIA:-1}"
KERNEL="${KERNEL:-hwe}"
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get install -y --no-install-recommends -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold)
OPTIONAL_MISSING=()

have_pkg() { apt-cache show "$1" >/dev/null 2>&1; }
# Something nice to have that may not be in the archive (or not under this name): install it if it's there,
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

# ------------------------------------------------------------------ ostree comes from the builder
# The placeholder packages tell dpkg that libostree-1-1, ostree and ostree-boot are there (flatpak depends on
# libostree-1-1); holding them keeps the archive's ostree from replacing the one built for bootc.
dpkg -i /hearth-debs/*.deb
apt-mark hold libostree-1-1 ostree ostree-boot
# The libraries the builder's programs (ostree, bootc, bootupd, Quickshell, RyzenAdj, HHD) link against.
# Installed before anything from the builder runs.
mapfile -t BUILDER_DEPS < <(grep -v -e '^$' -e '^libostree-1-1$' /usr/share/hearth/builder-runtime-deps.txt)
"${APT[@]}" "${BUILDER_DEPS[@]}"
ldconfig
ostree --version
bootc --version

# ------------------------------------------------------------------ the system: kernel, firmware, boot pieces
# dracut (Ubuntu 26.04's default initramfs tool anyway) before the kernel, so the kernel's hooks use it. Until
# Hearth's own dracut settings are copied in (with the system files, below), only "generic": the hooks'
# initramfs in /boot is thrown away, the real one is built at the end into /usr/lib/modules.
mkdir -p /etc/dracut.conf.d
echo 'hostonly="no"' > /etc/dracut.conf.d/00-hearth-build.conf
"${APT[@]}" dracut systemd systemd-sysv systemd-resolved systemd-timesyncd udev kmod dbus
# Ubuntu-specific: the kernel. Ubuntu's kernel series are metapackages (linux-image-generic-hwe-26.04: the
# hardware-enablement series that moves to each new Ubuntu release's kernel; linux-image-generic: 26.04's own,
# 7.0). The meta's versioned image is installed directly, so the kernel's Recommends (every firmware package,
# Mellanox switch firmware included) stay out and Nvidia's prebuilt modules can be matched to it exactly.
# Signed by Canonical: boots with Secure Boot on.
KMETA=linux-image-generic
if [ "$KERNEL" = "hwe" ] && have_pkg linux-image-generic-hwe-26.04; then KMETA=linux-image-generic-hwe-26.04; fi
KIMG=$(apt-cache depends "$KMETA" | sed -n 's/^ *Depends: \(linux-image-[0-9][^ ]*-generic\)$/\1/p' | sed -n 1p || true)
[ -n "$KIMG" ] || { echo "couldn't find the kernel behind $KMETA:"; apt-cache depends "$KMETA" || true; exit 1; }
KVER="${KIMG#linux-image-}"
echo "kernel: $KMETA -> $KVER"
"${APT[@]}" "linux-image-$KVER" "linux-modules-$KVER"
optional "linux-modules-extra-$KVER"
KDIRS=(/usr/lib/modules/*/)
[ "${#KDIRS[@]}" -eq 1 ] || { echo "expected exactly one kernel, found: ${KDIRS[*]}"; exit 1; }
[ "$(basename "${KDIRS[0]}")" = "$KVER" ] || { echo "installed kernel ${KDIRS[0]} isn't $KVER"; exit 1; }
# bootc and ostree look for the kernel next to its modules (Ubuntu puts it in /boot, which the image doesn't
# keep). Newer kernel packages may already keep it there (/boot only links to it).
V="/usr/lib/modules/$KVER/vmlinuz"
if [ -L "$V" ]; then cp --remove-destination "$(readlink -f "$V")" "$V"
elif [ ! -e "$V" ]; then cp "/boot/vmlinuz-$KVER" "$V"
fi
chmod 0644 "$V"
# Firmware. Ubuntu-specific: 26.04 splits linux-firmware into 17 packages; these are the ones PCs and handhelds
# use (graphics, Wi-Fi/Bluetooth chips, Intel/AMD sound, Cirrus amplifiers for the ROG Ally's speakers live in
# -misc), without the data-centre ones (Mellanox, Netronome, QLogic, Marvell Prestera). If the split names
# aren't there, the whole linux-firmware.
FW=(linux-firmware-amd-graphics linux-firmware-intel-graphics linux-firmware-nvidia-graphics
  linux-firmware-intel-wireless linux-firmware-intel-misc linux-firmware-realtek linux-firmware-mediatek
  linux-firmware-broadcom-wireless linux-firmware-qualcomm-wireless linux-firmware-qualcomm-graphics
  linux-firmware-qualcomm-misc linux-firmware-marvell-wireless linux-firmware-misc)
if have_pkg linux-firmware-amd-graphics; then
  for p in "${FW[@]}"; do optional "$p"; done
else
  "${APT[@]}" linux-firmware
fi
optional firmware-sof-signed amd64-microcode intel-microcode wireless-regdb

"${APT[@]}" \
  ca-certificates curl less nano file locales polkitd pkexec \
  util-linux fdisk parted gdisk efibootmgr btrfs-progs e2fsprogs xfsprogs dosfstools ntfs-3g exfatprogs \
  rsync cpio zstd skopeo pciutils usbutils lsof procps psmisc \
  network-manager wpasupplicant iw bluez openssh-client \
  pipewire pipewire-pulse pipewire-alsa wireplumber libspa-0.2-bluetooth alsa-utils \
  upower udisks2 flatpak xdg-user-dirs xdg-utils \
  plymouth \
  python3 python3-evdev python3-dbus python3-gi python3-yaml python3-rich python3-xlib python3-serial \
  python3-pyroute2 python3-setuptools libusb-1.0-0
# sudo: Ubuntu 26.04's is sudo-rs (reads /etc/sudoers.d the same way).
optional sudo sudo-rs bluez-obexd pipewire-audio pipewire-jack fwupd plymouth-themes plymouth-label \
  power-profiles-daemon fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji fonts-liberation fonts-dejavu-core

# ------------------------------------------------------------------ graphics and Steam (AMD, Intel; Nvidia below)
# Mesa with its 32-bit half (they must be the same version), then Steam (multiverse) on top so its 32-bit
# dependencies agree. Steam's licence is accepted here (debconf), the way the Steam Deck ships it. Ubuntu's
# Steam is the steam-installer deb (the snap is never used: snapd is pinned out).
echo 'steam steam/question select I AGREE' | debconf-set-selections
echo 'steam steam/license note ' | debconf-set-selections
echo 'steam-installer steam/question select I AGREE' | debconf-set-selections
echo 'steam-installer steam/license note ' | debconf-set-selections
"${APT[@]}" \
  mesa-vulkan-drivers mesa-vulkan-drivers:i386 libgl1-mesa-dri libgl1-mesa-dri:i386 libglx-mesa0 libglx-mesa0:i386 \
  libegl-mesa0 libgbm1 mesa-va-drivers libvulkan1 libvulkan1:i386 vulkan-tools mesa-utils \
  intel-media-va-driver xwayland \
  steam-installer steam-devices
optional steam-libs-amd64 steam-libs-i386:i386
# gamescope: Ubuntu's (multiverse) is Valve's upstream gamescope (3.16.x), which has every atom Hearth's Game
# Mode uses; CI checks each one.
"${APT[@]}" gamescope mangohud
optional mangoapp
# Ubuntu puts games' programs in /usr/games, which isn't on every PATH (the session, the checks): link them.
for b in gamescope gamescopereaper gamescopestream gamescopectl mangoapp mangohud; do
  if [ -e "/usr/games/$b" ] && [ ! -e "/usr/bin/$b" ]; then ln -s "../games/$b" "/usr/bin/$b"; fi
done
command -v gamescope
optional gamemode libgamemode0:i386 zenity
# Steam's launcher lives in /usr/games; Hearth's session calls `steam` from the usual PATH.
STEAM_BIN=$(command -v steam || ls /usr/games/steam 2>/dev/null || true)
[ -n "$STEAM_BIN" ] || { echo "Steam's launcher wasn't installed"; exit 1; }
[ -e /usr/bin/steam ] || ln -s "$STEAM_BIN" /usr/bin/steam

# ------------------------------------------------------------------ Nvidia: the open kernel module, in the same image
# Ubuntu-specific (and better than the Peppermint build's DKMS): Ubuntu's restricted archive has Nvidia's open
# module prebuilt for each kernel and signed by Canonical (linux-modules-nvidia-<branch>-open-<kernel>), so it
# loads with Secure Boot on and the image needs no compiler for it. The newest driver branch built for this
# kernel is used; the userspace is the "no-dkms" set (nvidia-driver-<branch>-open would pull DKMS in). It only
# loads on an Nvidia card (GeForce GTX 16 series, RTX 20 series and newer: the open module's range).
# Fallback when no prebuilt module exists for this kernel (e.g. a brand-new HWE kernel): DKMS, as on Peppermint.
NV_DKMS=0
if [ "$NVIDIA" = "1" ]; then
  KVER_RE="${KVER//./\\.}"
  NV_N=$(apt-cache pkgnames linux-modules-nvidia- | sed -n "s/^linux-modules-nvidia-\([0-9][0-9]*\)-open-${KVER_RE}\$/\1/p" | sort -n | tail -1 || true)
  if [ -n "$NV_N" ]; then
    echo "Nvidia: driver branch $NV_N, Canonical's signed prebuilt module for $KVER"
    "${APT[@]}" "linux-modules-nvidia-${NV_N}-open-${KVER}" || {
      echo "installing Nvidia's module failed; dpkg's log:"; tail -n 25 /var/log/apt/term.log 2>/dev/null || true
      dpkg --audit || true; exit 1; }
  else
    NV_N=$(apt-cache pkgnames nvidia-dkms- | sed -n 's/^nvidia-dkms-\([0-9][0-9]*\)-open$/\1/p' | sort -n | tail -1 || true)
    [ -n "$NV_N" ] || { echo "no Nvidia open driver in the archive"; exit 1; }
    echo "::warning title=nvidia::no prebuilt Nvidia module for $KVER; building nvidia-dkms-${NV_N}-open (unsigned: no Secure Boot on Nvidia)"
    "${APT[@]}" "linux-headers-$KVER" "nvidia-dkms-${NV_N}-open"
    dkms autoinstall -k "$KVER" || true
    dkms status
    NV_DKMS=1
  fi
  "${APT[@]}" "nvidia-headless-no-dkms-${NV_N}-open" "nvidia-utils-${NV_N}" "libnvidia-gl-${NV_N}"
  optional "libnvidia-decode-${NV_N}" "libnvidia-encode-${NV_N}" "libnvidia-fbc1-${NV_N}" "libnvidia-cfg1-${NV_N}" \
    "libnvidia-extra-${NV_N}" "xserver-xorg-video-nvidia-${NV_N}"
  # 32-bit Nvidia libraries (older 32-bit games through Steam): the same branch as the 64-bit ones.
  optional "libnvidia-gl-${NV_N}:i386" "libnvidia-compute-${NV_N}:i386" "libnvidia-decode-${NV_N}:i386" \
    "libnvidia-encode-${NV_N}:i386" "libnvidia-fbc1-${NV_N}:i386"
  find "/usr/lib/modules/$KVER" -name 'nvidia.ko*' | grep . >/dev/null || {
    echo "Nvidia's open kernel module isn't there for $KVER (see above)"
    find /var/lib/dkms -name make.log -exec tail -40 {} \; 2>/dev/null || true
    exit 1
  }
  if [ "$NV_DKMS" = "0" ] && dpkg -l 'nvidia-dkms-*' 2>/dev/null | grep '^ii' >/dev/null; then
    echo "warning: a DKMS Nvidia package came along with the prebuilt module (its build is unused)"
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
  echo "$NV_N" > /usr/share/hearth-nvidia-branch
fi

# acpi_call: Handheld Daemon sets the Legion Go's power limits through it on kernels without Lenovo's WMI
# firmware attributes. Built here with DKMS for the image's kernel, then the module is kept and the compiler,
# headers and DKMS go (unless Nvidia's fallback needs DKMS kept). Unsigned: doesn't load with Secure Boot on.
if have_pkg acpi-call-dkms; then
  if "${APT[@]}" "linux-headers-$KVER" acpi-call-dkms; then
    dkms autoinstall -k "$KVER" || true
    ACPI_KO=$(find "/usr/lib/modules/$KVER" -name 'acpi_call.ko*' | sed -n 1p)
    if [ -n "$ACPI_KO" ]; then
      if [ "$NV_DKMS" = "0" ]; then
        install -Dm644 "$ACPI_KO" "/usr/lib/modules/$KVER/updates/hearth/$(basename "$ACPI_KO")"
        apt-get purge -y acpi-call-dkms dkms "linux-headers-$KVER"
        apt-get autoremove --purge -y
        depmod -a "$KVER"
      fi
    else
      OPTIONAL_MISSING+=("acpi_call(build failed)")
    fi
  else
    OPTIONAL_MISSING+=("acpi-call-dkms(failed)")
  fi
else
  OPTIONAL_MISSING+=("acpi-call-dkms")
fi

# ------------------------------------------------------------------ Handheld Daemon (controllers, TDP, power button)
# Built by the builder from hhd-dev/hhd (tag in builder.sh) into /usr/lib/hhd, its Python dependencies installed
# above from Ubuntu. Hearth uses HHD (not InputPlumber) and talks to it at /run/hhd/api.
test -x /usr/bin/hhd
/usr/lib/hhd/bin/python3 -c 'import hhd, adjustor, evdev, yaml, rich, Xlib, serial, pyroute2, gi, dbus; print("hhd", hhd.__file__)'
systemctl enable hhd.service

# ------------------------------------------------------------------ Desktop Mode: labwc + Quickshell (+ what Bazzite carried)
"${APT[@]}" labwc grim slurp brightnessctl wlr-randr wlsunset xdotool x11-utils x11-xserver-utils \
  gnome-keyring libpam-gnome-keyring qt6-wayland \
  xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk foot fuse-overlayfs bubblewrap \
  libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav libnss3 libgbm1
# The rest of what Desktop Mode, Couch Co-op and Hearth Browser (Electron) use; names that can differ.
optional qt6-svg-plugins qt5ct breeze-icon-theme 7zip gstreamer1.0-plugins-bad gstreamer1.0-pipewire \
  libasound2t64 libcups2t64 libgtk-3-0t64 libxss1 libatk-bridge2.0-0t64 x11-apps
# Quickshell's QML modules (Hearth's shell imports QtQuick, Layouts, Particles, Window; Quickshell itself the rest).
optional qt6ct qml6-module-qtquick qml6-module-qtquick-layouts qml6-module-qtquick-particles qml6-module-qtquick-window \
  qml6-module-qtquick-controls qml6-module-qtquick-templates qml6-module-qtquick-shapes qml6-module-qtquick-effects \
  qml6-module-qtqml-workerscript qml6-module-qt5compat-graphicaleffects qml6-module-qtquick-dialogs
# A polkit agent for Desktop Mode's password prompts, linked where labwc's autostart looks.
optional policykit-1-gnome
if [ -x /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 ]; then
  ln -sf /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 /usr/libexec/polkit-gnome-authentication-agent-1
else
  optional lxqt-policykit
fi

# ------------------------------------------------------------------ the login screen (SDDM, as on Bazzite)
# nimbus-autologin writes SDDM's autologin; the Hearth theme (os/system_files) is Qt 6, as Ubuntu's SDDM is.
# SDDM's login screen itself runs on X (its default), so the X server comes along.
"${APT[@]}" sddm xserver-xorg-core xserver-xorg-input-libinput
optional qml6-module-qtquick-virtualkeyboard qml6-module-qtmultimedia xserver-xorg-video-amdgpu

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's Ubuntu-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/ubuntu_files/. /
chmod 440 /etc/sudoers.d/hearth-wheel

# ------------------------------------------------------------------ Hearth itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit (libwebkit2gtk-4.1-0, installed above).
  install -Dm755 /ctx/nimbus-app/hearth /usr/lib/nimbus/nimbus
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
if [ -f /usr/lib/nimbus/chrome-sandbox ]; then chown root:root /usr/lib/nimbus/chrome-sandbox && chmod 4755 /usr/lib/nimbus/chrome-sandbox; fi
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
# CI builds the app in an ubuntu:26.04 container, so every library it links must be here under the same name.
if ldd /usr/lib/nimbus/nimbus | grep "not found"; then
  echo "The Hearth app needs libraries this image doesn't have (above)"; exit 1
fi

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh /usr/share/nimbus/desktop/shell/hearth-pad /usr/share/nimbus/desktop/shell/hearth-buttons /usr/share/nimbus/desktop/shell/hearth-controller /usr/share/nimbus/desktop/shell/hearth-power /usr/share/nimbus/desktop/shell/hearth-browser /usr/share/nimbus/desktop/shell/hearth-tasks /usr/share/nimbus/desktop/shell/hearth-files /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper
# Hearth's HHD helper for Settings (added for the Anatase build; executable if this checkout has it).
if [ -f /usr/share/nimbus/desktop/shell/hearth-hhd ]; then chmod 755 /usr/share/nimbus/desktop/shell/hearth-hhd; fi
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; the Peppermint
# and CachyOS builds write this same file).
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-ubuntu", "image-ref": "ghcr.io/dragongamer9/hearth-ubuntu", "image-tag": "testing", "base": "ubuntu", "base-detail": "Ubuntu 26.04 LTS (snap-free)", "kernel": "${KVER}", "nvidia": ${NVIDIA}, "version": "${NIMBUS_VERSION:-0.0.0}"}
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

# ------------------------------------------------------------------ system settings Bazzite/Fedora came with
# Hearth's administrators group (the installer puts the first account in it; the polkit rules use it). In
# /etc/group now (the installer adds the account against the image's /etc) and in sysusers.d for later.
getent group wheel >/dev/null || groupadd -r wheel
# Ubuntu's locale-gen takes the locale as an argument (and also reads /etc/locale.gen where there is one).
if [ -f /etc/locale.gen ]; then sed -i 's/^#\s*\(en_US.UTF-8 UTF-8\)/\1/' /etc/locale.gen; fi
locale-gen en_US.UTF-8
echo 'LANG=en_US.UTF-8' > /etc/locale.conf
echo 'LANG=en_US.UTF-8' > /etc/default/locale
# Flathub for system apps (Hearth's updates update them). Flatpak is the app store here: no snaps.
mkdir -p /etc/flatpak/remotes.d
curl -fsSL --retry 3 -o /etc/flatpak/remotes.d/flathub.flatpakrepo https://dl.flathub.org/repo/flathub.flatpakrepo || \
  echo "warning: couldn't fetch Flathub's remote file"
systemctl mask systemd-firstboot.service
# Ubuntu-specific: no snaps. snapd is pinned out (the Containerfile); make sure nothing slipped through.
if dpkg -l snapd 2>/dev/null | grep '^ii' >/dev/null; then echo "snapd got installed: something depends on it"; exit 1; fi
# Ubuntu-specific: netplan isn't used (NetworkManager manages every device; its Ubuntu default leaves wired ones
# alone, which system_files' 10-globally-managed-devices.conf undoes). Nothing to remove if it isn't there.
rm -f /etc/network/interfaces

# ------------------------------------------------------------------ power button
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ services
systemctl enable NetworkManager.service bluetooth.service systemd-resolved.service systemd-timesyncd.service \
  fstrim.timer bootc-fetch-apply-updates.timer sddm.service
systemctl enable hearth-bls-grub.service hearth-bls-grub-refresh.path
# power-profiles-daemon only runs when HHD is switched off (its drop-in says so); enabled so it can then.
systemctl enable power-profiles-daemon.service 2>/dev/null || true
ln -sf /usr/lib/systemd/system/sddm.service /etc/systemd/system/display-manager.service
mkdir -p /etc/X11 && echo /usr/bin/sddm > /etc/X11/default-display-manager

# ------------------------------------------------------------------ boot straight into Hearth
systemctl enable nimbus-autologin.service
systemctl enable hearth-greeter-pad.service
systemctl enable hearth-boot-first.service
# The first start after moving here from the Bazzite-based Hearth OS: a clean install that keeps your games
# (os/system_files; does nothing without the plan the old system leaves).
systemctl enable hearth-clean-install.service

# ------------------------------------------------------------------ name
# /etc/os-release links to /usr/lib/os-release. ID stays ubuntu (VERSION_CODENAME resolute) so tools still
# know what it is underneath.
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (Ubuntu edition, 26.04 LTS)"/' \
  /usr/lib/os-release
grep -q '^VARIANT_ID=' /usr/lib/os-release || printf 'VARIANT="Ubuntu edition"\nVARIANT_ID=hearth-ubuntu\n' >> /usr/lib/os-release
grep -q '^ID_LIKE=' /usr/lib/os-release || echo 'ID_LIKE=debian' >> /usr/lib/os-release
cat /usr/lib/os-release

# ------------------------------------------------------------------ boot loader for Hearth's installer (bootupd)
# bootupd copies the boot loader from /usr/lib/efi/<component>/<version>/EFI (the layout Fedora uses too) onto
# the EFI partition. Ubuntu's signed shim and GRUB (Microsoft -> Canonical -> Ubuntu's kernel: Secure Boot works)
# go in EFI/ubuntu: Ubuntu's signed GRUB has that folder built in as where it finds grub.cfg.
# Ubuntu-specific: the packages are unpacked, not installed. Their maintainer scripts run grub-install and
# update-grub against the build container, which neither works nor is wanted (bootupd owns the EFI partition).
mkdir -p /tmp/efi-debs /tmp/efi-root
(cd /tmp/efi-debs && apt-get download shim-signed grub-efi-amd64-signed)
(cd /tmp/efi-debs && apt-get download shim) || echo "no separate shim package"
for d in /tmp/efi-debs/*.deb; do dpkg-deb -x "$d" /tmp/efi-root; done
SHIM_VER=$(dpkg-deb -f /tmp/efi-debs/shim-signed_*.deb Version | tr ':~' '._')
GRUB_VER=$(dpkg-deb -f /tmp/efi-debs/grub-efi-amd64-signed_*.deb Version | tr ':~' '._')
pick() { local f; for f in "$@"; do if [ -f "$f" ]; then echo "$f"; return 0; fi; done; return 1; }
E=/tmp/efi-root/usr/lib/shim
SHIM=$(pick "$E/shimx64.efi.signed.latest" "$E/shimx64.efi.signed" "$E/shimx64.efi.dualsigned") || { echo "no signed shim"; find /tmp/efi-root -name '*.efi*'; exit 1; }
MOK=$(pick "$E/mmx64.efi.signed" "$E/mmx64.efi") || MOK=
FB=$(pick "$E/fbx64.efi.signed" "$E/fbx64.efi") || FB=
GRUBEFI=$(pick /tmp/efi-root/usr/lib/grub/x86_64-efi-signed/grubx64.efi.signed) || { echo "no signed GRUB"; find /tmp/efi-root -name '*.efi*'; exit 1; }
S="/usr/lib/efi/shim/$SHIM_VER/EFI"
G="/usr/lib/efi/grub2/$GRUB_VER/EFI"
install -Dm644 "$SHIM" "$S/ubuntu/shimx64.efi"
install -Dm644 "$SHIM" "$S/BOOT/BOOTX64.EFI"
if [ -n "$MOK" ]; then install -Dm644 "$MOK" "$S/ubuntu/mmx64.efi"; install -Dm644 "$MOK" "$S/BOOT/mmx64.efi"; fi
if [ -n "$FB" ]; then install -Dm644 "$FB" "$S/BOOT/fbx64.efi"; fi
if [ -f "$E/BOOTX64.CSV" ]; then
  install -Dm644 "$E/BOOTX64.CSV" "$S/ubuntu/BOOTX64.CSV"
else
  # The fallback loader's list of what to start: "shimx64.efi,<label>,,<comment>" in UTF-16.
  printf 'shimx64.efi,Hearth OS,,Hearth OS\n' | iconv -t UTF-16 > "$S/ubuntu/BOOTX64.CSV"
fi
install -Dm644 "$GRUBEFI" "$G/ubuntu/grubx64.efi"
find /usr/lib/efi -type f | sort
bootupctl backend generate-update-metadata /
ls -la /usr/lib/bootupd/updates/

# ------------------------------------------------------------------ boot splash and initramfs
if command -v plymouth-set-default-theme >/dev/null; then
  plymouth-set-default-theme hearth
else
  mkdir -p /etc/plymouth && printf '[Daemon]\nTheme=hearth\n' > /etc/plymouth/plymouthd.conf
fi
# Ubuntu-specific: Ubuntu also picks the theme through an alternative; point it at Hearth's too.
if [ -f /usr/share/plymouth/themes/hearth/hearth.plymouth ] && update-alternatives --list default.plymouth >/dev/null 2>&1; then
  update-alternatives --install /usr/share/plymouth/themes/default.plymouth default.plymouth \
    /usr/share/plymouth/themes/hearth/hearth.plymouth 200
  update-alternatives --set default.plymouth /usr/share/plymouth/themes/hearth/hearth.plymouth
fi
ls -d /usr/lib/dracut/modules.d/*ostree >/dev/null 2>&1 || { echo "ostree's dracut module is missing: this image couldn't boot"; exit 1; }
export DRACUT_NO_XATTR=1
dracut --force --kver "$KVER" "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"
lsinitrd "/usr/lib/modules/$KVER/initramfs.img" > /tmp/initramfs-files.txt
grep -q ostree-prepare-root /tmp/initramfs-files.txt || { echo "the initramfs has no ostree-prepare-root"; exit 1; }
grep -q plymouth /tmp/initramfs-files.txt || echo "warning: the initramfs has no boot splash"

# ------------------------------------------------------------------ tidy up
rm -f /usr/sbin/policy-rc.d
apt-get clean
rm -rf /var/lib/apt/lists/* /boot/*
rm -f /etc/dracut.conf.d/00-hearth-build.conf
if [ "${#OPTIONAL_MISSING[@]}" -gt 0 ]; then echo "Not installed (not in the repositories): ${OPTIONAL_MISSING[*]}"; fi

# ------------------------------------------------------------------ system accounts that later images add
# On an image-based system the machine's /etc/passwd and /etc/group are kept across updates (they hold the
# person's account), so an account a newer image's package adds (sddm, a daemon) would never appear there.
# systemd-sysusers creates them at boot from this list (its drop-in in system_files makes it run every boot).
{
  echo "# Written by os-bases/ubuntu/build.sh: every system group and account in this image."
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
