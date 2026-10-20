#!/bin/bash
# Turns CachyOS (docker.io/cachyos/cachyos-v3, prepared by os-bases/cachyos/Containerfile) into Hearth OS as a
# bootc image: "Hearth OS (CachyOS edition)", Beta Experimental. Runs once, inside the image build.
# The Hearth parts follow os/build_files/build.sh (the Bazzite build) as closely as possible; what's different is
# the base: pacman instead of dnf, everything Bazzite already carried installed here, the boot loader laid out for
# bootupd, and the bootc layout bootcrew's arch-bootc uses. See docs/BASE-CACHYOS.md.
#
# CI pitfalls this script avoids (set -o pipefail + set -e): no `producer | grep -q` or `| head -N` (the producer
# dies of SIGPIPE and the pipeline fails); no `for` loop whose last command is a false `[ ... ] &&`; programs
# built in the builder stage only run after their runtime libraries are installed.
set -ouex pipefail

KERNEL="${KERNEL:-linux-cachyos-deckify}"
NVIDIA="${NVIDIA:-1}"
MARCH="${MARCH:-x86-64-v3}"
# Handheld Daemon, only if the repositories don't carry it: built from this tag's PKGBUILD.
HHD_REF="${HHD_REF:-v4.1.12}"
# pacman without questions: --ask=4 accepts replacing a conflicting package (e.g. jack2 by pipewire-jack).
PAC=(pacman -S --noconfirm --needed --ask=4)
OPTIONAL_MISSING=()

have_pkg() { pacman -Si "$1" >/dev/null 2>&1; }
# Something nice to have that may not be in the repositories (yet): install it if it's there, say so if not.
optional() {
  local p
  for p in "$@"; do
    if have_pkg "$p"; then "${PAC[@]}" "$p" || OPTIONAL_MISSING+=("$p(failed)"); else OPTIONAL_MISSING+=("$p"); fi
  done
}
# The first of these package names that exists (e.g. quickshell, else quickshell-git).
first_of() {
  local p
  for p in "$@"; do if have_pkg "$p"; then echo "$p"; return 0; fi; done
  return 1
}

pacman -Sy

# ------------------------------------------------------------------ no initramfs or boot loader from pacman hooks
# The kernel and dracut/mkinitcpio packages rebuild /boot/initramfs-* on every install (host-only, for the
# build container), and boot-manager hooks would try to write a boot loader. In a bootc image the initramfs lives
# next to the kernel in /usr/lib/modules and is built once, at the end of this script, and bootupd owns the boot
# loader: mask those hooks (a /dev/null file of the same name in /etc/pacman.d/hooks overrides a hook). DKMS's
# hooks stay: they build Nvidia's and acpi_call's modules for the image's kernel.
mkdir -p /etc/pacman.d/hooks
for h in 90-dracut-install.hook 60-dracut-remove.hook 90-mkinitcpio-install.hook 60-mkinitcpio-remove.hook \
         95-systemd-boot.hook sdboot-manage.hook zz-sdboot-manage.hook; do
  ln -sf /dev/null "/etc/pacman.d/hooks/$h"
done

# ------------------------------------------------------------------ the system: kernel, firmware, boot pieces
# dracut is named so pacman doesn't pick mkinitcpio for the kernel's "initramfs" dependency. ostree's libraries
# come first: bootc (from the builder stage) links against them.
"${PAC[@]}" ostree composefs skopeo
bootc --version
"${PAC[@]}" base dracut "$KERNEL" "$KERNEL-headers" linux-firmware amd-ucode intel-ucode \
  grub efibootmgr \
  btrfs-progs e2fsprogs xfsprogs dosfstools ntfs-3g exfatprogs util-linux rsync cpio zstd \
  systemd dbus polkit sudo shadow which less nano openssh curl pciutils usbutils lsof psmisc \
  networkmanager bluez bluez-utils iw wireless-regdb \
  pipewire pipewire-pulse pipewire-alsa pipewire-jack wireplumber lib32-pipewire alsa-utils \
  upower udisks2 fwupd flatpak xdg-user-dirs xdg-utils \
  plymouth python python-evdev python-yaml
bootupctl --version
KDIRS=(/usr/lib/modules/*/vmlinuz)
[ "${#KDIRS[@]}" -eq 1 ] || { echo "expected exactly one kernel, found: ${KDIRS[*]}"; exit 1; }
KVER=$(basename "$(dirname "${KDIRS[0]}")")
echo "kernel: $KERNEL $KVER"
# DKMS (for modules CachyOS doesn't prebuild for this kernel), with clang/lld when the kernel itself was built with
# clang (CachyOS builds some kernels that way, and modules must use the same compiler).
dkms_tools() {
  "${PAC[@]}" dkms
  if grep -q '^CONFIG_CC_IS_CLANG=y' "/usr/lib/modules/$KVER/build/.config"; then "${PAC[@]}" clang llvm lld; fi
}
# CachyOS's own tuning (zram, sysctl, ananicy-cpp, udev rules) and its sched_ext schedulers (scx_lavd is what
# CachyOS's handheld edition uses).
optional cachyos-settings scx-scheds

# ------------------------------------------------------------------ graphics: AMD, Intel and Nvidia in one image
# Mesa for AMD/Intel; the lib32 drivers are named so Steam's lib32-vulkan-driver dependency doesn't pick one by
# itself.
"${PAC[@]}" mesa lib32-mesa vulkan-radeon lib32-vulkan-radeon vulkan-intel lib32-vulkan-intel \
  vulkan-icd-loader lib32-vulkan-icd-loader intel-media-driver libva-utils vulkan-tools mesa-utils xorg-xwayland
if [ "$NVIDIA" = "1" ]; then
  # Nvidia's open driver alongside (only loads on an Nvidia card: GeForce GTX 16 series, RTX 20 series and newer).
  "${PAC[@]}" nvidia-utils lib32-nvidia-utils
  # The kernel module: CachyOS's prebuilt one for this kernel if there is one, else built here by DKMS for the
  # image's kernel.
  if have_pkg "$KERNEL-nvidia-open"; then
    "${PAC[@]}" "$KERNEL-nvidia-open"
  else
    dkms_tools
    "${PAC[@]}" nvidia-open-dkms
    dkms autoinstall -k "$KVER" || true
    dkms status
  fi
  if ! find "/usr/lib/modules/$KVER" -name 'nvidia.ko*' > /tmp/nvidia-ko.txt || ! [ -s /tmp/nvidia-ko.txt ]; then
    echo "Nvidia's open kernel module isn't there for $KVER (see the DKMS output above)"
    find /var/lib/dkms -name make.log -exec tail -40 {} \; 2>/dev/null || true
    exit 1
  fi
  cat /tmp/nvidia-ko.txt
  # Nvidia's suspend helpers only where an Nvidia driver is loaded.
  for s in nvidia-suspend nvidia-resume nvidia-hibernate nvidia-suspend-then-hibernate; do
    if [ -f "/usr/lib/systemd/system/$s.service" ]; then
      mkdir -p "/usr/lib/systemd/system/$s.service.d"
      printf '[Unit]\nConditionPathExists=/proc/driver/nvidia\n' > "/usr/lib/systemd/system/$s.service.d/10-hearth-only-nvidia.conf"
      systemctl enable "$s.service" || true
    fi
  done
  systemctl disable nvidia-persistenced.service 2>/dev/null || true
fi
# acpi_call: Handheld Daemon sets the Legion Go's power limits through it. CachyOS's prebuilt module for this
# kernel if there is one, else DKMS. Not fatal (only the Legion Go needs it): CI warns when it's missing.
if have_pkg "$KERNEL-acpi_call"; then
  "${PAC[@]}" "$KERNEL-acpi_call" || OPTIONAL_MISSING+=("$KERNEL-acpi_call(failed)")
elif have_pkg acpi_call-dkms; then
  dkms_tools
  if "${PAC[@]}" acpi_call-dkms; then dkms autoinstall -k "$KVER" || OPTIONAL_MISSING+=("acpi_call-dkms(build failed)"); else OPTIONAL_MISSING+=("acpi_call-dkms(failed)"); fi
else
  OPTIONAL_MISSING+=("acpi_call")
fi
depmod -a "$KVER"

# ------------------------------------------------------------------ gaming
# gamescope: Valve's own (CachyOS and Arch package ValveSoftware/gamescope), which has everything Hearth's Game
# Mode talks to (GAMESCOPECTRL_BASELAYER_WINDOW/APPID, GAMESCOPE_FPS_LIMIT, GAMESCOPECTRL_REQUEST_SCREENSHOT,
# --mangoapp; CI checks each one). mangohud carries mangoapp. Steam is Arch's bootstrapper (the real client is
# downloaded into the home folder on first launch).
"${PAC[@]}" steam gamescope mangohud lib32-mangohud gamemode lib32-gamemode \
  ttf-liberation noto-fonts noto-fonts-cjk noto-fonts-emoji ttf-dejavu
# 32-bit gamescope WSI layer (CachyOS's handheld repo), Proton outside Steam, RyzenAdj (hearth-admin's fallback for
# AMD power limits), the Steam Deck's speaker processing and fan daemon (both only act on a Deck).
optional lib32-gamescope umu-launcher ryzenadj steamdeck-dsp jupiter-fan-control
if [ -f /usr/lib/systemd/system/jupiter-fan-control.service ]; then
  systemctl enable jupiter-fan-control.service || true
fi

# ------------------------------------------------------------------ Handheld Daemon (controllers, TDP, power button)
# Hearth on CachyOS uses Handheld Daemon (hhd-dev/hhd), not InputPlumber: Hearth picks HHD up at runtime through
# its API socket (/run/hhd/api). From CachyOS's repositories when they carry it; otherwise built here from hhd's
# own PKGBUILD at the pinned tag.
if have_pkg hhd; then
  "${PAC[@]}" hhd
else
  "${PAC[@]}" python python-setuptools python-evdev python-rich python-yaml python-xlib python-dbus libusb \
    python-pyserial lsof python-pyroute2 python-gobject
  "${PAC[@]}" --asdeps base-devel git python-build python-installer python-wheel python-babel
  # makepkg doesn't run as root. The PKGBUILD goes into a folder of its own: the repository's own src/ would be
  # mixed up with makepkg's src/.
  useradd -r -M -d /tmp/hhd-build hearth-build
  git clone --depth 1 --branch "$HHD_REF" https://github.com/hhd-dev/hhd /tmp/hhd-src
  mkdir -p /tmp/hhd-build/pkgbuild
  cp /tmp/hhd-src/PKGBUILD /tmp/hhd-build/pkgbuild/
  chown -R hearth-build: /tmp/hhd-build
  (cd /tmp/hhd-build/pkgbuild && runuser -u hearth-build -- env HOME=/tmp/hhd-build makepkg -f --noconfirm --nodeps)
  pacman -U --noconfirm /tmp/hhd-build/pkgbuild/hhd-[0-9]*.pkg.tar.zst
  userdel hearth-build
  rm -rf /tmp/hhd-build /tmp/hhd-src
fi
test -x /usr/bin/hhd
python3 -c 'import hhd, adjustor, evdev, yaml, rich, Xlib, serial, pyroute2, gi, dbus; print("hhd", hhd.__file__)'
pacman -Q hhd
# The system-wide service (no user needed, so it works from the first boot and on the live USB). HHD provides the
# power-profiles D-Bus service itself (Hearth's power modes use it), so power-profiles-daemon only starts when HHD
# is switched off (/etc/hearth/no-hhd; its drop-in says so). InputPlumber would fight HHD over the controller, and
# SteamOS Manager over the power limit (Hearth isn't Steam's UI), so both are kept off; tuned too.
systemctl enable hhd.service
"${PAC[@]}" power-profiles-daemon
for s in inputplumber.service steamos-manager.service tuned.service tuned-ppd.service; do
  systemctl mask "$s" 2>/dev/null || true
done
systemctl --global mask steamos-manager.service 2>/dev/null || true

# ------------------------------------------------------------------ Desktop Mode: labwc + Quickshell (+ what Bazzite carried)
QUICKSHELL=$(first_of quickshell quickshell-git) || { echo "quickshell isn't in the repositories"; exit 1; }
"${PAC[@]}" labwc "$QUICKSHELL" grim slurp brightnessctl wlr-randr wlsunset \
  qt6-wayland qt6-svg qt6-declarative breeze-icons xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk \
  xorg-xprop xorg-xwininfo xorg-xdpyinfo xorg-xrandr xdotool gnome-keyring polkit-gnome foot \
  7zip fuse-overlayfs bubblewrap \
  webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-plugins-bad gst-libav \
  gtk3 nss libcups alsa-lib libxss at-spi2-core
optional qt6ct qt5ct gst-plugin-pipewire qt6-5compat
# Printing (Settings › Printers: CUPS, driverless IPP Everywhere, ipp-usb, Avahi) and clipboard history (Super+V).
optional cups cups-filters ipp-usb avahi nss-mdns wl-clipboard cliphist wtype

# ------------------------------------------------------------------ the login screen (SDDM, as on Bazzite)
# nimbus-autologin writes SDDM's autologin; the Hearth theme (os/system_files) is Qt 6, as Arch's SDDM is. SDDM's
# login screen itself runs on X (its default), so the X server comes along.
"${PAC[@]}" sddm xorg-server xf86-input-libinput

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's CachyOS-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/cachyos_files/. /
chmod 440 /etc/sudoers.d/hearth-wheel

# ------------------------------------------------------------------ Hearth itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit (webkit2gtk-4.1, installed above).
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
# CI builds the app inside CachyOS's own container, so every library it links must be here under the same name.
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
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh \
  /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper /usr/bin/hearth-browser
# Every helper in the shell folder (hearth-pad, -buttons, -controller, -power, -browser, -tasks, -files, -hhd, ...).
find /usr/share/nimbus/desktop/shell -maxdepth 1 -type f -name 'hearth-*' -exec chmod 755 {} +
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Quick Access over games: Handheld Daemon shows an overlay app over games (its QAM button) and holds the controllers
# for it. Here that app is Hearth's own Quick Access (desktop/shell/hearth-overlay: hosts/linux/src/overlay in Hearth
# Browser's Electron), installed as hhd-ui, and HHD_OVERLAY points at it so it wins over any other hhd-ui.
if pacman -Q hhd-ui >/dev/null 2>&1; then pacman -Rdd --noconfirm hhd-ui || true; fi
rm -f /usr/bin/hhd-ui.AppImage /usr/bin/hhd-ui-dbg
chmod 755 /usr/share/nimbus/desktop/shell/hearth-overlay
ln -sfn /usr/share/nimbus/desktop/shell/hearth-overlay /usr/bin/hhd-ui
mkdir -p /usr/lib/systemd/system/hhd.service.d
cat > /usr/lib/systemd/system/hhd.service.d/zz-hearth-overlay.conf <<'CONF'
# Hearth: Handheld Daemon's overlay app is Hearth's Quick Access (see os-bases/*/build.sh).
[Service]
Environment=HHD_OVERLAY=/usr/share/nimbus/desktop/shell/hearth-overlay
CONF
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; Hearth's edition()
# reads "base"). "march": what the packages need from the CPU, for an installer to check.
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-cachyos", "image-ref": "ghcr.io/dragongamer9/hearth-cachyos", "image-tag": "testing", "base": "cachyos", "base-detail": "CachyOS (${MARCH} repositories)", "kernel": "${KERNEL} ${KVER}", "nvidia": ${NVIDIA}, "march": "${MARCH}", "version": "${NIMBUS_VERSION:-0.0.0}"}
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
# The tarball holds bin/, share/ (or usr/bin, usr/share, possibly inside one folder): it goes under /usr.
beacon_tar() {
  local t; t=$(mktemp -d)
  curl -fsSL --retry 3 -o "$t/beacon.tar.gz" "$BEACON_URL/beacon-x86_64-linux.tar.gz" || return 1
  mkdir "$t/x" && tar -xzf "$t/beacon.tar.gz" -C "$t/x" || return 1
  local root=""
  for r in "$t/x/usr" "$t/x" "$t"/x/*/usr "$t"/x/*; do
    if [ -x "$r/bin/beacon" ]; then root="$r"; break; fi
  done
  [ -n "$root" ] || { echo "warning: Beacon's tarball has no bin/beacon"; return 1; }
  cp -a "$root/." /usr/
  rm -rf "$t"
}
if [ "$(uname -m)" = x86_64 ] && beacon_tar; then
  echo "Beacon installed"
else
  echo "warning: couldn't install Beacon from $BEACON_URL (no release yet?); Hearth works without it"
fi

# ------------------------------------------------------------------ Sunshine (Multi-seat's streamed players, Remote play)
# Settings › Multi-seat can stream an extra player's own Game Mode to Moonlight on a TV, phone, tablet or laptop (one
# Sunshine per streamed player, see docs/MULTI-SEAT.md); Settings › Remote play uses it too. CachyOS's repositories
# when they have it, else LizardByte's own pacman repository (prebuilt, with CUDA), added only for this install.
# Optional: without it Hearth says streaming isn't available.
sunshine_lizardbyte() {
  cp /etc/pacman.conf /tmp/pacman.conf.hearth
  printf '\n[lizardbyte]\nSigLevel = Optional\nServer = https://github.com/LizardByte/pacman-repo/releases/latest/download\n' >> /etc/pacman.conf
  local ok=1
  pacman -Sy && "${PAC[@]}" lizardbyte/sunshine && ok=0
  cp /tmp/pacman.conf.hearth /etc/pacman.conf && rm -f /tmp/pacman.conf.hearth
  pacman -Sy || true
  return "$ok"
}
if [ "$(uname -m)" = x86_64 ] && { { have_pkg sunshine && "${PAC[@]}" sunshine; } || sunshine_lizardbyte; }; then
  echo "Sunshine installed"
else
  echo "warning: couldn't install Sunshine; streamed multi-seat players and Remote play show as unavailable"
  OPTIONAL_MISSING+=("sunshine")
fi
# The streamed players' address as a QR code in Settings (optional), and the udev helper that gives each streamed
# player their own virtual controllers (copied in with system_files; executable to be sure).
optional qrencode
chmod 755 /usr/libexec/nimbus/hearth-seat-input 2>/dev/null || true

# ------------------------------------------------------------------ system settings Bazzite/Fedora came with
# Hearth's administrators group (the installer puts the first account in it; sudo and the polkit rules use it).
# Arch's filesystem package has it already.
getent group wheel >/dev/null || groupadd -r wheel
sed -i 's/^#\s*\(en_US.UTF-8 UTF-8\)/\1/' /etc/locale.gen && locale-gen
echo 'LANG=en_US.UTF-8' > /etc/locale.conf
# Flathub for system apps (Hearth's updates update them).
mkdir -p /etc/flatpak/remotes.d
curl -fsSL --retry 3 -o /etc/flatpak/remotes.d/flathub.flatpakrepo https://dl.flathub.org/repo/flathub.flatpakrepo || \
  echo "warning: couldn't fetch Flathub's remote file"
systemctl mask systemd-firstboot.service

# ------------------------------------------------------------------ power button
# Hearth handles the power button in its session (Settings › Power button); Valve's powerbuttond, if a package
# pulled it in, doesn't start for every session.
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true

# ------------------------------------------------------------------ services
systemctl enable NetworkManager.service bluetooth.service systemd-resolved.service systemd-timesyncd.service \
  fstrim.timer bootc-fetch-apply-updates.timer sddm.service
systemctl enable hearth-bls-grub.service hearth-bls-grub-refresh.path
# power-profiles-daemon only runs when HHD is switched off (its drop-in says so); enabled so it can then.
systemctl enable power-profiles-daemon.service
# Printing starts when something prints (Settings › Printers); Avahi finds network printers.
systemctl enable cups.socket 2>/dev/null || true
systemctl enable avahi-daemon.service 2>/dev/null || true
# SDDM is the display manager.
ln -sf /usr/lib/systemd/system/sddm.service /etc/systemd/system/display-manager.service
# CachyOS's sched_ext scheduler loader: scx_lavd (what CachyOS's handheld edition uses), in its automatic mode.
if [ -f /usr/lib/systemd/system/scx_loader.service ]; then
  printf 'default_sched = "scx_lavd"\ndefault_mode = "Auto"\n' > /etc/scx_loader.toml
  systemctl enable scx_loader.service || true
fi

# ------------------------------------------------------------------ boot straight into Hearth
systemctl enable nimbus-autologin.service
systemctl enable hearth-greeter-pad.service
systemctl enable hearth-boot-first.service
# The first start after moving here from the Bazzite-based Hearth OS: a clean install that keeps your games
# (os/system_files; does nothing without the plan the old system leaves).
systemctl enable hearth-clean-install.service

# ------------------------------------------------------------------ name
# /etc/os-release links to /usr/lib/os-release on Arch. ID stays cachyos (ID_LIKE arch) so tools still know what
# it is underneath.
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (CachyOS edition)"/' \
  /usr/lib/os-release
grep -q '^VARIANT_ID=' /usr/lib/os-release || printf 'VARIANT="CachyOS edition"\nVARIANT_ID=hearth-cachyos\n' >> /usr/lib/os-release
grep -q '^ID_LIKE=' /usr/lib/os-release || echo 'ID_LIKE=arch' >> /usr/lib/os-release
cat /usr/lib/os-release

# ------------------------------------------------------------------ boot loader for Hearth's installer (bootupd)
# bootupd copies the boot loader from /usr/lib/efi/<component>/<version>/EFI (the layout Fedora uses too) onto the
# EFI partition, then writes its static grub.cfg next to the loader. Arch and CachyOS have no signed shim, so:
#   - GRUB is built here as one EFI file with everything it needs inside and its folder (/EFI/cachyos) built in, so
#     it finds bootupd's grub.cfg however the firmware started it.
#   - bootupd needs a shimx64.efi to find the vendor folder and to name in the firmware's boot entry: it gets the
#     same GRUB under that name (there's nothing for a shim to do without Secure Boot). BOOT/BOOTX64.EFI is the
#     same file again (the fallback path firmware uses when its boot entries are gone).
# This boots with Secure Boot OFF only (see docs/BASE-CACHYOS.md).
GRUB_VER=$(pacman -Q grub | cut -d' ' -f2 | tr ':~' '._')
GRUBMODS=/usr/lib/grub/x86_64-efi
WANT_MODS="all_video boot btrfs cat chain configfile echo efifwsetup efinet ext2 exfat fat font gfxmenu gfxterm
  gfxterm_background gzio halt iso9660 jpeg loadenv loopback linux ls lsefi lvm lzopio mdraid09 mdraid1x minicmd
  normal ntfs part_gpt part_msdos password_pbkdf2 png probe read reboot regexp search search_fs_file search_fs_uuid
  search_label serial sleep test true video video_bochs video_cirrus videoinfo xfs xzio zstd"
MODS=()
for m in $WANT_MODS; do
  if [ -f "$GRUBMODS/$m.mod" ]; then MODS+=("$m"); else echo "GRUB has no module $m (skipped)"; fi
done
G="/usr/lib/efi/grub2/$GRUB_VER/EFI"
S="/usr/lib/efi/shim/$GRUB_VER/EFI"
mkdir -p "$G/cachyos" "$S/cachyos" "$S/BOOT"
grub-mkimage -O x86_64-efi -p /EFI/cachyos -o "$G/cachyos/grubx64.efi" "${MODS[@]}"
install -Dm644 "$G/cachyos/grubx64.efi" "$S/cachyos/shimx64.efi"
install -Dm644 "$G/cachyos/grubx64.efi" "$S/BOOT/BOOTX64.EFI"
find /usr/lib/efi -type f | sort
bootupctl backend generate-update-metadata /
ls -la /usr/lib/bootupd/updates/
test -f /usr/lib/bootupd/updates/EFI.json

# ------------------------------------------------------------------ boot splash and initramfs
# Hearth's Plymouth theme (os/system_files/usr/share/plymouth/themes/hearth); Arch's plymouth has the script plugin
# built in. The initramfs: dracut with this image's settings (system_files/usr/lib/dracut/dracut.conf.d): generic,
# ostree's prepare-root (mounts the deployment), bootc's module, the splash.
plymouth-set-default-theme hearth
ls -d /usr/lib/dracut/modules.d/*ostree >/dev/null 2>&1 || { echo "ostree's dracut module is missing: this image couldn't boot"; exit 1; }
ls -d /usr/lib/dracut/modules.d/*bootc >/dev/null 2>&1 || { echo "bootc's dracut module is missing"; exit 1; }
test -x /usr/lib/ostree/ostree-prepare-root || test -x /usr/libexec/ostree-prepare-root || \
  { echo "ostree-prepare-root is missing: this image couldn't boot"; exit 1; }
export DRACUT_NO_XATTR=1
dracut --force --kver "$KVER" "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"
lsinitrd "/usr/lib/modules/$KVER/initramfs.img" > /tmp/initramfs-files.txt
grep ostree-prepare-root /tmp/initramfs-files.txt > /dev/null || { echo "the initramfs has no ostree-prepare-root"; exit 1; }
grep plymouth /tmp/initramfs-files.txt > /dev/null || echo "warning: the initramfs has no boot splash"

# ------------------------------------------------------------------ tidy up
# Packages that were only needed to build something (Handheld Daemon's build tools, if it was built here).
orphans=$(pacman -Qtdq || true)
if [ -n "$orphans" ]; then pacman -Rns --noconfirm $orphans || true; fi
# The repositories' package lists (pacman -Q keeps working from the local database) and the build container's
# package-signing keyring: every installed Hearth would otherwise share the same local signing key. On the device
# pacman doesn't install anything (/usr is read-only); `pacman-key --init` recreates the keyring if needed.
rm -rf /usr/lib/sysimage/lib/pacman/sync/* /etc/pacman.d/gnupg
if [ "${#OPTIONAL_MISSING[@]}" -gt 0 ]; then echo "Not installed (not in the repositories): ${OPTIONAL_MISSING[*]}"; fi

# ------------------------------------------------------------------ system accounts that later images add
# On an image-based system the machine's /etc/passwd and /etc/group are kept across updates (they hold the person's
# account), so an account a newer image adds would never appear there. Arch packages mostly ship sysusers.d files;
# this list covers every account in the image anyway (one made by an install script, say), and systemd-sysusers
# runs every boot (its drop-in in system_files).
{
  echo "# Written by os-bases/cachyos/build.sh: every system group and account in this image."
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

# ------------------------------------------------------------------ the bootc layout (bootcrew's arch-bootc)
# /var belongs to the machine: everything packages put there becomes a tmpfiles.d entry (created at boot) and the
# image's /var is left empty. /opt contents move to /usr/lib/opt (linked back from /var/opt). pacman's database
# already lives in /usr/lib/sysimage (the Containerfile moved it).
VARCONF=/usr/lib/tmpfiles.d/hearth-var.conf
: > "$VARCONF"
find /var -mindepth 1 -type d \
  ! -path '/var/cache*' ! -path '/var/log*' ! -path '/var/tmp*' ! -path '/var/run*' ! -path '/var/lock*' \
  ! -path '/var/lib/pacman*' ! -path '/var/lib/dkms*' ! -path '/var/home*' ! -path '/var/roothome*' \
  ! -path '/var/opt*' ! -path '/var/srv*' ! -path '/var/mnt*' ! -path '/var/usrlocal*' \
  -printf 'd /var/%P %m %u %g -\n' | sort >> "$VARCONF"
if [ -d /opt ] && [ -n "$(ls -A /opt 2>/dev/null)" ]; then
  mkdir -p /usr/lib/opt
  for d in /opt/*; do
    mv "$d" /usr/lib/opt/
    echo "L+ /var/opt/$(basename "$d") - - - - /usr/lib/opt/$(basename "$d")" >> "$VARCONF"
  done
fi
cat "$VARCONF"
sed -i 's|^#\?\s*HOME=.*|HOME=/var/home|' /etc/default/useradd
grep -q '^HOME=/var/home' /etc/default/useradd || echo 'HOME=/var/home' >> /etc/default/useradd
echo "uninitialized" > /etc/machine-id
rm -rf /boot /home /root /usr/local /srv /opt /mnt /var /usr/lib/sysimage/log
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
