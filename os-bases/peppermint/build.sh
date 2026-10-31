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
optional qt6-svg-plugins qt5ct breeze-icon-theme 7zip libarchive-tools gstreamer1.0-plugins-bad gstreamer1.0-pipewire \
  libasound2t64 libcups2t64 libgtk-3-0t64 libxss1 libatk-bridge2.0-0t64
# Claude Desktop (Hearth Settings › Claude: Anthropic's package, unpacked in the user's home folder by
# desktop/shell/hearth-claude, never shipped in this image): gpg checks Anthropic's signature; the app links
# GTK 3, NSS, libdrm/gbm, libxtst, PipeWire and AT-SPI (above), and opens libsecret (the sign-in kept in the
# password store), libnotify (notifications) and the tray library while it runs.
"${APT[@]}" gpg
optional libsecret-1-0 libnotify4 libxcb-dri3-0 libatspi2.0-0t64 libuuid1 xdg-utils libayatana-appindicator3-1
# Quickshell's QML modules (Hearth's shell imports QtQuick, Layouts, Particles, Window; Quickshell itself the rest).
optional qt6ct qml6-module-qtquick qml6-module-qtquick-layouts qml6-module-qtquick-particles qml6-module-qtquick-window \
  qml6-module-qtquick-controls qml6-module-qtquick-templates qml6-module-qtquick-shapes qml6-module-qtquick-effects \
  qml6-module-qtqml-workerscript qml6-module-qt5compat-graphicaleffects qml6-module-qtquick-dialogs
# Printing (Settings › Printers: CUPS, driverless IPP Everywhere, ipp-usb, Avahi) and clipboard history (Super+V).
optional cups cups-filters ipp-usb avahi-daemon libnss-mdns wl-clipboard cliphist wtype
# Desktop Mode: screenshot notifications (notify-send) and turning the screen off when idle (Settings › Power).
optional libnotify-bin wlopm
# A polkit agent for Desktop Mode's password prompts. Debian keeps polkit-gnome's in a folder labwc's autostart
# doesn't look in: link it where it does.
# Debian 13 has no policykit-1-gnome any more: MATE's agent (GTK, small) first, then LXDE's, then LXQt's. Without
# one, pkexec prompts (Apps' system installs, Hearth Text saving system files) fail.
if have_pkg policykit-1-gnome && "${APT[@]}" policykit-1-gnome 2>/dev/null && [ -x /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 ]; then
  ln -sf /usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1 /usr/libexec/polkit-gnome-authentication-agent-1
else
  optional mate-polkit
  if ! ls /usr/libexec/polkit-mate-authentication-agent-1 /usr/lib/*/mate-polkit/polkit-mate-authentication-agent-1 >/dev/null 2>&1; then optional lxpolkit lxqt-policykit; fi
fi
for a in /usr/libexec/polkit-mate-authentication-agent-1 /usr/lib/*/mate-polkit/polkit-mate-authentication-agent-1 /usr/libexec/polkit-gnome-authentication-agent-1 /usr/bin/lxpolkit /usr/bin/lxqt-policykit-agent; do
  if [ -x "$a" ]; then echo "polkit agent: $a"; break; fi
done

# ------------------------------------------------------------------ .deb files (Hearth's Debian box) and AppImages
# The system is an image (/usr is read-only), so a .deb is installed into a Debian 13 container made with Distrobox
# (hearth-debbox, the Install Package window): rootless podman needs newuidmap/newgidmap (uidmap) and a network
# helper (passt, or slirp4netns). Each account's id range in /etc/subuid and /etc/subgid comes from useradd, the
# installer and hearth-home-repair (every boot). AppImages: FUSE 2 (most AppImages' runtime loads libfuse.so.2) and
# a `fusermount` (fuse3 only has fusermount3, which does the same job: linked).
"${APT[@]}" distrobox uidmap libfuse2t64 fuse3
optional passt slirp4netns
if ! command -v fusermount >/dev/null && command -v fusermount3 >/dev/null; then ln -s "$(command -v fusermount3)" /usr/bin/fusermount; fi
touch /etc/subuid /etc/subgid
command -v distrobox && command -v newuidmap && ls /usr/lib/x86_64-linux-gnu/libfuse.so.2

# ------------------------------------------------------------------ Hearth Plasma (docs/PLASMA.md)
# KDE Plasma 6 as Desktop Mode's second desktop, in Hearth's look (os/system_files/usr/share/plasma and
# /usr/share/hearth-plasma). Only the workspace (no recommends, no kde-standard app bundle) plus Dolphin for folders.
# Hearth Classic (labwc + Mantel, above) stays installed and is the fallback whenever Plasma can't start.
# plasma-workspace needs gdb (its crash handler), whose libdebuginfod has to match the elfutils libraries the newer
# kernel/Mesa brought from backports: taken from backports too, or apt can't solve it.
if dpkg -l libelf1t64 2>/dev/null | grep -q '~bpo'; then "${APT[@]}" -t "$BPO" libdebuginfod1t64 libdebuginfod-common || true; fi
"${APT[@]}" plasma-workspace kwin-wayland plasma-desktop
optional plasma-nm plasma-pa powerdevil kscreen kscreenlocker bluedevil polkit-kde-agent-1 xdg-desktop-portal-kde \
  breeze kde-cli-tools systemsettings plasma-integration qml6-module-org-kde-kirigami libplasma5support6 dolphin \
  kde-config-screenlocker spectacle qml-module-org-kde-plasma-plasma5support qml6-module-org-kde-plasma-plasma5support
for b in startplasma-wayland kwin_wayland plasmashell; do command -v "$b" >/dev/null || { echo "::error title=Hearth Plasma::$b is missing"; exit 1; }; done

# ------------------------------------------------------------------ Hearth GNOME (docs/GNOME.md)
# GNOME Shell (trixie's release) as Desktop Mode's third desktop, in Hearth's look through Hearth's own Shell extensions
# (os/system_files/usr/share/gnome-shell). Only the shell, its session, its Settings and portal (no recommends, no
# GNOME app bundle, no GDM: SDDM stays), plus GNOME's own apps for the kinds it has (Files, Console, Text Editor: one
# app of each kind per desktop, through the app-kinds table's GNOME entries below). Hearth Classic stays the fallback.
gnome_before=$(du -sxm /usr /etc 2>/dev/null | awk '{s += $1} END {print s}')
"${APT[@]}" gnome-shell mutter gnome-session gnome-control-center
optional xdg-desktop-portal-gnome nautilus gnome-console gnome-text-editor
for b in gnome-shell gnome-session; do command -v "$b" >/dev/null || { echo "::error title=Hearth GNOME::$b is missing"; exit 1; }; done
gnome_after=$(du -sxm /usr /etc 2>/dev/null | awk '{s += $1} END {print s}')
mkdir -p /usr/share/nimbus && echo "$((gnome_after - gnome_before))" > /usr/share/nimbus/hearth-gnome-size-mb
echo "::notice title=Hearth GNOME::installed; it adds about $((gnome_after - gnome_before)) MB to the image"

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
chmod 440 /etc/sudoers.d/hearth-wheel

# ------------------------------------------------------------------ Hearth icon set (branding/icons)
# Every Hearth app's icon (Hearth, Settings, Files, Terminal, Disks, Browser, Task Manager, Apps, the installer and
# the newer apps: hearth-text, hearth-photos...) in the hicolor theme, so Mantel, labwc's Alt+Tab and any other
# desktop find them by name (Icon=hearth-settings). Drawn by branding/icons/make-icons.py: one SVG each plus PNGs
# from 16 to 256 px. Apps on other branches only need their .desktop file's Icon= set to their name.
optional hicolor-icon-theme adwaita-icon-theme libgtk-3-bin
for svg in /ctx/icons/apps/*.svg; do
  install -Dm644 "$svg" "/usr/share/icons/hicolor/scalable/apps/$(basename "$svg")"
done
for dir in /ctx/icons/png/*x*; do
  install -d "/usr/share/icons/hicolor/$(basename "$dir")/apps"
  install -m644 "$dir"/*.png "/usr/share/icons/hicolor/$(basename "$dir")/apps/"
done
# The old 512 px Hearth logo (os/system_files, without the tile) would win over the set at large sizes.
rm -f /usr/share/icons/hicolor/512x512/apps/hearth.png
test -f /usr/share/icons/hicolor/scalable/apps/hearth-settings.svg
test -f /usr/share/icons/hicolor/48x48/apps/hearth-files.png
gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
# ---- end of the Hearth icon set

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
  # Hearth drawn by Qt Quick (hosts/hearth-qml, Experimental): only when chosen in Settings › Display › Renderer
  # (docs/RENDERERS.md). Built by CI inside this image's base (.github/scripts/hearth-qml-build.sh), so it links this Qt.
  # Its packages: the same list as hearth-qml-build.sh runtime_pkgs.
  if [ -x /ctx/nimbus-app/qml/hearth-qml ]; then
    "${APT[@]}" qml6-module-qtquick qml6-module-qtquick-window qml6-module-qtquick-shapes qml6-module-qtquick-layouts qml6-module-qtqml-workerscript qml6-module-qtqml-models qt6-qpa-plugins qt6-image-formats-plugins libsdl2-2.0-0 || echo "warning: some of the Qt renderer's packages didn't install"
    mkdir -p /usr/lib/hearth-qml && cp -a /ctx/nimbus-app/qml/. /usr/lib/hearth-qml/
    chmod 755 /usr/lib/hearth-qml/hearth-qml
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
# ---- hearth-steam (hosts/hearth-steam): the lightweight Steam engine, experimental and off by default. Hearth starts its
# user service (hearth-steam-engine.service, not enabled) only while Settings › Library › Steam engine is Lightweight.
# Not in this build, or it doesn't install: skipped with a warning (Hearth then always uses the normal Steam client).
if [ -x /ctx/nimbus-app/hearth-steam/hearth-steam ] && [ -f /ctx/nimbus-app/hearth-steam/hearth-steam-engine.service ]; then
  if mkdir -p /usr/lib/hearth-steam && cp -a /ctx/nimbus-app/hearth-steam/. /usr/lib/hearth-steam/ \
     && install -Dm644 /usr/lib/hearth-steam/hearth-steam-engine.service /usr/lib/systemd/user/hearth-steam-engine.service; then
    chmod 755 /usr/lib/hearth-steam/hearth-steam
    echo "hearth-steam installed: $(du -sh /usr/lib/hearth-steam | cut -f1)"
  else
    echo "warning: hearth-steam didn't install; this image goes without the lightweight Steam engine"
    rm -rf /usr/lib/hearth-steam /usr/lib/systemd/user/hearth-steam-engine.service
  fi
else
  echo "warning: hearth-steam isn't in this build; this image goes without the lightweight Steam engine"
fi
# ---- end hearth-steam

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
# Make a bootable USB (Disks' USB maker, src/usb): parted and gdisk lay out a Windows USB drive, wimtools splits and reads
# Windows' install.wim, gpg checks the projects' signatures, xz and zstd unpack compressed disk images.
"${APT[@]}" parted gdisk wimtools gnupg xz-utils zstd dosfstools curl || true
for t in parted sgdisk wimlib-imagex wiminfo gpg xz zstd mkfs.vfat fsck.vfat; do command -v "$t" >/dev/null || echo "warning: $t is missing, so Make a bootable USB can't do everything"; done
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
Icon=hearth-disks
Type=Application
Categories=System;Utility;
Keywords=disk;drive;partition;format;usb;sd;card;mount;eject;storage;steam;library;
DESKTOP
  cat > /usr/share/applications/hearth-usb-maker.desktop <<DESKTOP
[Desktop Entry]
Name=Make a Bootable USB
GenericName=Bootable USB Maker
Comment=Download Linux or Windows and put it on a USB drive to install it, checked before it's written
Exec=/usr/bin/hearth-disks --usb %f
Icon=hearth-disks
Type=Application
Categories=System;Utility;
MimeType=application/x-cd-image;application/x-iso9660-image;application/x-raw-disk-image;application/x-raw-disk-image-xz-compressed;
Keywords=usb;iso;bootable;installer;flash;burn;write;linux;windows;fedora;ubuntu;debian;arch;rufus;etcher;
DESKTOP
fi

# ------------------------------------------------------------------ Hearth Time (hosts/hearth-time)
# Clock, Calendar, Notes and Weather: one program, /usr/bin/hearth-time, with a link per app (hearth-clock,
# hearth-calendar, hearth-notes, hearth-weather), their menu entries, and the small background service
# that rings alarms and timers with the apps closed, shows calendar reminders and keeps the weather summary fresh
# (hearth-time.service, a systemd user service, on for every user). Built with the app (base-*.yml), carried in
# os/nimbus-app/hearth-time/. Alarms open their window with xprop/xdotool in Game Mode and wlopm in Desktop Mode
# (installed above), and play their tone with pw-play.
if [ -x /ctx/nimbus-app/hearth-time/hearth-time ]; then
  install -Dm755 /ctx/nimbus-app/hearth-time/hearth-time /usr/bin/hearth-time
  if ldd /usr/bin/hearth-time | grep "not found"; then echo "warning: Hearth Time is missing the libraries above"; fi
  install -d /usr/share/applications /usr/lib/systemd/user
  # Their icons (hearth-clock, hearth-calendar, hearth-notes, hearth-weather) come with Hearth's icon set.
  for a in clock calendar notes weather; do ln -sf hearth-time "/usr/bin/hearth-$a"; done
  hearth_time_entry() {  # id, name, comment, categories, keywords[, file types it opens]
    cat > "/usr/share/applications/hearth-$1.desktop" <<DESKTOP
[Desktop Entry]
Name=$2
Comment=$3
Exec=/usr/bin/hearth-$1${6:+ %u}
TryExec=/usr/bin/hearth-time
Icon=hearth-$1
Type=Application
Categories=$4
Keywords=$5
StartupNotify=true
StartupWMClass=io.github.dragongamer9.hearth.$1
${6:+MimeType=$6}
DESKTOP
  }
  hearth_time_entry clock Clock "Alarms, timers, a stopwatch and world clocks" "Utility;Clock;" "alarm;timer;stopwatch;world;clock;time;"
  hearth_time_entry calendar Calendar "Your events and reminders, and .ics calendars" "Office;Calendar;" "calendar;event;reminder;ics;schedule;agenda;" "text/calendar;x-scheme-handler/webcal;x-scheme-handler/webcals;"
  hearth_time_entry notes Notes "Quick notes and checklists, kept as plain files in Documents/Notes" "Office;Utility;" "note;notes;checklist;todo;markdown;memo;"
  hearth_time_entry weather Weather "The forecast for a place you choose (Open-Meteo)" "Utility;" "weather;forecast;temperature;rain;"
  cat > /usr/lib/systemd/user/hearth-time.service <<'UNIT'
[Unit]
Description=Hearth Clock alarms and timers, Calendar reminders and the weather summary

[Service]
ExecStart=/usr/bin/hearth-time --service
Restart=on-failure
RestartSec=5
Nice=5

[Install]
WantedBy=default.target
UNIT
  systemctl --global enable hearth-time.service 2>/dev/null \
    || { mkdir -p /etc/systemd/user/default.target.wants && ln -sf /usr/lib/systemd/user/hearth-time.service /etc/systemd/user/default.target.wants/hearth-time.service; }
fi
# ---- end of Hearth Time
# ---- Hearth Tools (hosts/hearth-tools): Disk Usage, System Info, Logs, Network Tools, Passwords & Keys, Printers &
# Scanner and Welcome & Tips in one program (/usr/bin/hearth-tools --app NAME), built with the app (base-peppermint.yml)
# and carried in os/nimbus-app/hearth-tools/ with its menu entries (hearth-*.desktop; their icons come with Hearth's icon set). What the
# tools run: SANE's scanimage and sane-airscan (network scanners), traceroute and tracepath, iw (Wi-Fi link details).
optional sane-utils sane-airscan traceroute iputils-tracepath iw
if [ -x /ctx/nimbus-app/hearth-tools/hearth-tools ]; then
  install -Dm755 /ctx/nimbus-app/hearth-tools/hearth-tools /usr/bin/hearth-tools
  if ldd /usr/bin/hearth-tools | grep "not found"; then echo "warning: Hearth Tools is missing the libraries above"; fi
  install -d /usr/share/applications
  install -m644 /ctx/nimbus-app/hearth-tools/hearth-*.desktop /usr/share/applications/
fi
# ---- end Hearth Tools
# ---- Hearth Benchmark (Hearth Tools' --app benchmark): the programs its tests run. 7-Zip (processor), fio (storage),
# glmark2 for X11 and Wayland (OpenGL and the game-style scenes) and vkmark (Vulkan). All optional: a missing one is
# listed at the end of the build and its test is skipped in the app, never a failed image.
optional 7zip fio vkmark glmark2-x11 glmark2-wayland
command -v glmark2 >/dev/null || command -v glmark2-wayland >/dev/null || optional glmark2
command -v 7z >/dev/null || command -v 7zz >/dev/null || command -v 7za >/dev/null || optional p7zip-full
# ---- end Hearth Benchmark
# ---- Nearby Share and Hearth Connect (hosts/hearth-nearby, hosts/hearth-connect) ------------------------------
# Hearth's apps for phones and nearby devices, built with the app (base-peppermint.yml) and carried in os/nimbus-app
# (their icons, hearth-nearby and hearth-connect, come with Hearth's icon set). Nearby Share speaks LocalSend's protocol, Hearth Connect KDE Connect's. playerctl lets a phone
# control this PC's music, wl-clipboard/wtype give Hearth Connect the clipboard and typing in Desktop Mode, and
# notify-send's library its notifications. Their firewall helper and polkit rule come from os/system_files
# (hearth-link-firewall, 49-hearth-link.rules): ports open only while the apps listen, and only to local addresses.
optional playerctl wl-clipboard wtype libnotify-bin
for app in hearth-nearby hearth-connect; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
  fi
done
chmod 755 /usr/libexec/nimbus/hearth-link-firewall 2>/dev/null || true
# Hearth Storage's root helper (Windows sharing, waking from the network; 49-hearth-storage.rules).
chmod 755 /usr/libexec/nimbus/hearth-storage-helper 2>/dev/null || true
if dpkg -s firewalld >/dev/null 2>&1 || dpkg -s ufw >/dev/null 2>&1; then echo "a firewall is in this image: Nearby Share and Hearth Connect open their ports with it"; else echo "no firewall in this image: nothing to open"; fi
install -d /usr/share/applications /usr/lib/systemd/user
if [ -x /usr/bin/hearth-nearby ]; then
  cat > /usr/share/applications/hearth-nearby.desktop <<DESKTOP
[Desktop Entry]
Name=Nearby Share
GenericName=File Sharing
Comment=Send and receive files with phones and computers on the same network (works with LocalSend)
Exec=/usr/bin/hearth-nearby %F
Icon=hearth-nearby
Type=Application
Categories=Network;FileTransfer;Utility;
Keywords=localsend;share;send;receive;nearby;airdrop;transfer;phone;files;
DESKTOP
fi
if [ -x /usr/bin/hearth-connect ]; then
  cat > /usr/share/applications/hearth-connect.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Connect
GenericName=Phone Link
Comment=Link your phone: files, notifications, clipboard, phone as touchpad, music, find my phone (works with KDE Connect)
Exec=/usr/bin/hearth-connect
Icon=hearth-connect
Type=Application
Categories=Network;Utility;
Keywords=kde connect;kdeconnect;phone;android;iphone;notifications;clipboard;remote;touchpad;
DESKTOP
  # The background service that keeps the phone linked. Off until the person opens Hearth Connect (which turns it on
  # for their account), so nothing listens on the network before they ask for it.
  cat > /usr/lib/systemd/user/hearth-connect.service <<UNIT
[Unit]
Description=Hearth Connect (your phone, linked; KDE Connect's protocol)

[Service]
ExecStart=/usr/bin/hearth-connect --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
UNIT
fi
# ---- end of Nearby Share and Hearth Connect

# ---- Hearth Link (hosts/hearth-link): your Hearth devices paired, Hearth data kept in sync, games copied between them.
# Built with the app (base-peppermint.yml) and carried in os/nimbus-app. A small user service, started with every session
# (default.target): it answers only paired devices (and pairing while a code is showing), stays quiet while a game
# runs (Hearth's hearth-playing file), and gives way to anything else under load (low CPU/IO weight). Its UDP ports
# open to the local network only, through hearth-link-firewall.
if [ -x "/ctx/nimbus-app/hearth-link" ]; then
  install -Dm755 /ctx/nimbus-app/hearth-link /usr/bin/hearth-link
  if ldd /usr/bin/hearth-link | grep "not found"; then echo "warning: hearth-link is missing the libraries above"; rm -f /usr/bin/hearth-link; fi
fi
if [ -x /usr/bin/hearth-link ]; then
  install -d /usr/lib/systemd/user/default.target.wants
  cat > /usr/lib/systemd/user/hearth-link.service <<UNIT
[Unit]
Description=Hearth Link (your Hearth devices: sync and game copies)
After=network-online.target

[Service]
ExecStart=/usr/bin/hearth-link serve
Restart=on-failure
RestartSec=3
Nice=10
CPUWeight=20
IOWeight=20
IOSchedulingClass=best-effort
IOSchedulingPriority=7

[Install]
WantedBy=default.target
UNIT
  ln -sf ../hearth-link.service /usr/lib/systemd/user/default.target.wants/hearth-link.service
fi
# ---- end of Hearth Link

# ==== Hearth Paint, the emoji picker, Hearth Fonts, Hearth Theater, and Settings › Accessibility (hosts/hearth-paint,
# hearth-emoji, hearth-fonts, hearth-gamma, hearth-theater, desktop/shell/hearth-access): built by CI next to the Hearth app and carried in
# os/nimbus-app (their icons are in the Hearth icon set, installed above). The emoji picker types with wtype and copies with wl-copy; Fonts uses fontconfig;
# Settings › Accessibility uses Orca (the screen reader, with speech-dispatcher and eSpeak NG) and xprop (gamescope's
# zoom and colour filters in Game Mode). Each app is left out if it didn't build or can't find its libraries.
optional orca speech-dispatcher espeak-ng at-spi2-core fontconfig wtype wl-clipboard x11-utils
for app in hearth-paint hearth-emoji hearth-fonts hearth-gamma hearth-theater; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so it's left out"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
done
install -d /usr/share/applications
if [ -x /usr/bin/hearth-paint ]; then
  cat > /usr/share/applications/hearth-paint.desktop <<DESKTOP
[Desktop Entry]
Name=Paint
GenericName=Drawing
Comment=Draw, and mark up pictures and screenshots
Exec=/usr/bin/hearth-paint %f
Icon=hearth-paint
Type=Application
Categories=Graphics;2DGraphics;RasterGraphics;
MimeType=image/png;image/jpeg;image/webp;image/bmp;image/gif;
Keywords=paint;draw;drawing;sketch;annotate;markup;screenshot;picture;image;crop;
StartupWMClass=hearth-paint
DESKTOP
fi
if [ -x /usr/bin/hearth-emoji ]; then
  cat > /usr/share/applications/hearth-emoji.desktop <<DESKTOP
[Desktop Entry]
Name=Emoji & Symbols
Comment=Type emoji, symbols and special characters into any app (Super+.)
Exec=/usr/bin/hearth-emoji
Icon=hearth-emoji
Type=Application
Categories=Utility;
Keywords=emoji;emoticon;smiley;symbol;character;special;accent;kaomoji;unicode;
StartupWMClass=hearth-emoji
DESKTOP
fi
if [ -x /usr/bin/hearth-fonts ]; then
  cat > /usr/share/applications/hearth-fonts.desktop <<DESKTOP
[Desktop Entry]
Name=Fonts
GenericName=Font Viewer
Comment=See your fonts, and install or remove fonts
Exec=/usr/bin/hearth-fonts %f
Icon=hearth-fonts
Type=Application
Categories=Utility;
MimeType=font/ttf;font/otf;font/collection;font/sfnt;application/x-font-ttf;application/x-font-otf;application/vnd.ms-opentype;
Keywords=font;fonts;typeface;ttf;otf;install;
StartupWMClass=hearth-fonts
DESKTOP
  # Font files open in it (Hearth Files too): its MimeType, made the default with the others (Hearth's default apps, below).
fi
if [ -x /usr/bin/hearth-theater ]; then
  cat > /usr/share/applications/hearth-theater.desktop <<DESKTOP
[Desktop Entry]
Name=Theater
GenericName=Video Sites
Comment=Your own wall of video sites, full screen and controller-ready
Exec=/usr/bin/hearth-theater
Icon=hearth-theater
Type=Application
Categories=AudioVideo;Video;
Keywords=video;tv;movies;anime;shows;watch;streaming;theater;
StartupWMClass=hearth-theater
DESKTOP
fi
cat > /usr/share/applications/hearth-accessibility.desktop <<DESKTOP
[Desktop Entry]
Name=Accessibility
Comment=Magnifier, colour filters, screen reader, bigger text
Exec=/usr/bin/hearth-settings accessibility
Icon=hearth-accessibility
Type=Application
Categories=Settings;Accessibility;
Keywords=accessibility;a11y;magnifier;zoom;screen reader;orca;colour;color;filter;contrast;text size;
DESKTOP
update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
# ==== end Hearth Paint, Emoji, Fonts and Accessibility

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

# ==================================================================== Hearth Recorder, Camera and Share
# (hosts/hearth-recorder, hosts/hearth-camera, hosts/hearth-share; built with the app by base-peppermint.yml, tested
# by capture-apps.yml.) Hearth's screen recorder (instant replay and a sound recorder too), webcam app and screen
# sharing. What they run:
#  - ffmpeg (Debian's has every encoder: VA-API, NVENC, libx264, Opus, MP3)
#  - wf-recorder and slurp: capturing Mantel's desktop, an area too
#  - gpu-screen-recorder: capture and encoding on the graphics card in Game Mode and Desktop Mode, and its own
#    instant replay. Debian 13 doesn't package it, so CI builds it in trixie (gsr/ in the app's folder);
#    gsr-kms-server needs CAP_SYS_ADMIN to read the screen.
#  - GStreamer with PipeWire's plugin: Game Mode (gamescope's own stream) without gpu-screen-recorder, and Share there
#  - pactl and pw-dump (sound sources, gamescope's stream), v4l-utils (cameras), qrencode (Share's QR code)
optional ffmpeg wf-recorder slurp pulseaudio-utils pipewire-bin gstreamer1.0-tools gstreamer1.0-pipewire gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav v4l-utils libnotify-bin qrencode libcap2-bin
if [ -x /ctx/nimbus-app/gsr/gpu-screen-recorder ] && ! command -v gpu-screen-recorder >/dev/null; then
  # Its libraries (most come with ffmpeg and PipeWire already).
  optional libpulse0 libva2 libva-drm2 libdrm2 libwayland-client0 libwayland-egl1 libcap2 libxdamage1 libxcomposite1 libxrandr2 libvulkan1 libegl1 libgl1
  install -Dm755 /ctx/nimbus-app/gsr/gpu-screen-recorder /usr/bin/gpu-screen-recorder
  [ -x /ctx/nimbus-app/gsr/gsr-kms-server ] && install -Dm755 /ctx/nimbus-app/gsr/gsr-kms-server /usr/bin/gsr-kms-server
  if ldd /usr/bin/gpu-screen-recorder | grep 'not found'; then
    echo "warning: gpu-screen-recorder needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/gpu-screen-recorder /usr/bin/gsr-kms-server
  fi
fi
# Without its capability gpu-screen-recorder asks for the password every time it records.
if [ -x /usr/bin/gsr-kms-server ]; then setcap cap_sys_admin+ep /usr/bin/gsr-kms-server || echo "warning: couldn't give gsr-kms-server its capability"; fi
for app in hearth-recorder hearth-camera hearth-share; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    # One that can't start is left out (its menu entry too).
    if ldd "/usr/bin/$app" | grep 'not found'; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
  fi
done
if [ -x /usr/bin/hearth-recorder ] || [ -x /usr/bin/hearth-camera ] || [ -x /usr/bin/hearth-share ]; then
  # Their menu entries (ids starting with "hearth": Mantel's app menu lists them as Hearth's own apps).
  install -d /usr/share/applications
  if [ -x /usr/bin/hearth-recorder ]; then
    cat > /usr/share/applications/hearth-recorder.desktop <<DESKTOP
[Desktop Entry]
Name=Recorder
GenericName=Screen Recorder
Comment=Record the screen, a game or your voice, and keep instant replays of games
Exec=/usr/bin/hearth-recorder
Icon=hearth-recorder
Type=Application
Categories=AudioVideo;Video;Recorder;
Keywords=record;screen;recording;video;capture;replay;clip;game;microphone;voice;sound;
StartupNotify=true
StartupWMClass=hearth-recorder
Actions=sound;

[Desktop Action sound]
Name=Sound recorder
Exec=/usr/bin/hearth-recorder --sound
DESKTOP
  fi
  if [ -x /usr/bin/hearth-camera ]; then
    cat > /usr/share/applications/hearth-camera.desktop <<DESKTOP
[Desktop Entry]
Name=Camera
GenericName=Webcam
Comment=Take photos and videos with your webcam
Exec=/usr/bin/hearth-camera
Icon=hearth-camera
Type=Application
Categories=AudioVideo;Video;Graphics;Photography;
Keywords=camera;webcam;photo;selfie;video;picture;
StartupNotify=true
StartupWMClass=hearth-camera
DESKTOP
  fi
  if [ -x /usr/bin/hearth-share ]; then
    cat > /usr/share/applications/hearth-share.desktop <<DESKTOP
[Desktop Entry]
Name=Share Screen
GenericName=Screen Sharing
Comment=Show this screen on a TV, phone or another computer (web browser or Moonlight)
Exec=/usr/bin/hearth-share
Icon=hearth-share
Type=Application
Categories=Network;AudioVideo;
Keywords=share;screen;cast;mirror;tv;stream;moonlight;miracast;chromecast;browser;
StartupNotify=true
StartupWMClass=hearth-share
DESKTOP
  fi
fi
# ==================================================================== end of Hearth Recorder, Camera and Share
# ------------------------------------------------------------------ Hearth's gaming apps (hosts/hearth-gaming)
# Controller Lab, Saves and Mods: built with the app (base-peppermint.yml) and carried in os/nimbus-app; their icons come
# with Hearth's icon set (branding/icons, above: Icon=hearth-controller-lab ...). Desktop Mode lists them
# with Hearth's own apps (their ids start with "hearth"); Game Mode opens them from Settings › Controls and a
# game's ⋯ menu (hearth-core gaming_apps.rs). Saves' hourly timer is on for everyone and does nothing until a
# schedule is chosen in Saves (hearth-saves --auto). Mods unpacks .7z and .rar files with bsdtar.
command -v bsdtar >/dev/null || echo "warning: bsdtar is missing, so Mods can only unpack .zip files"
gaming_entry() {   # id, name, generic name, comment, keywords
  [ -x "/usr/bin/$1" ] || return 0
  cat > "/usr/share/applications/$1.desktop" <<DESKTOP
[Desktop Entry]
Name=$2
GenericName=$3
Comment=$4
Exec=/usr/bin/$1
Icon=$1
Type=Application
Categories=Game;Utility;
Keywords=$5
StartupNotify=true
StartupWMClass=$1
DESKTOP
}
install -d /usr/share/applications
for app in hearth-controller-lab hearth-saves hearth-mods; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so this image goes without it"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app needs the libraries above; left out"; rm -f "/usr/bin/$app"; continue; fi
done
gaming_entry hearth-controller-lab "Controller Lab" "Controller Tester" "Test every button, stick, trigger, gyro and rumble; measure stick drift and calibrate" "controller;gamepad;joystick;test;tester;calibrate;drift;deadzone;rumble;gyro;battery;"
gaming_entry hearth-saves "Saves" "Game Save Backup" "Back up and restore your games' saves, and move them to another PC" "save;saves;backup;restore;game;steam;proton;usb;"
gaming_entry hearth-mods "Mods" "Mod Manager" "Install mods for your games, turn them on and off, and choose their order" "mod;mods;modding;manager;nexus;game;"
if [ -x /usr/bin/hearth-saves ]; then
  install -d /usr/lib/systemd/user/timers.target.wants
  cat > /usr/lib/systemd/user/hearth-saves-auto.service <<'UNIT'
[Unit]
Description=Hearth Saves: scheduled game save backup

[Service]
Type=oneshot
ExecStart=/usr/bin/hearth-saves --auto
Nice=10
IOSchedulingClass=idle
UNIT
  cat > /usr/lib/systemd/user/hearth-saves-auto.timer <<'UNIT'
[Unit]
Description=Hearth Saves: look every hour whether a scheduled backup is due

[Timer]
OnBootSec=10min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
UNIT
  ln -sf ../hearth-saves-auto.timer /usr/lib/systemd/user/timers.target.wants/hearth-saves-auto.timer
fi
# Hearth Mods and Nexus Mods: the site's "Mod Manager Download" button opens nxm:// links, which go to Hearth Mods'
# download queue (hearth-mods --nxm; hearth-system makes it the default for them with Hearth's other apps). Mods put in
# with the overlay (fuse-overlayfs) are laid over their games again at each sign-in (hearth-mods --remount).
if [ -x /usr/bin/hearth-mods ]; then
  cat > /usr/share/applications/hearth-mods-nxm.desktop <<'DESKTOP'
[Desktop Entry]
Name=Mods (Nexus Mods downloads)
Comment=Downloads a mod from Nexus Mods' "Mod Manager Download" button
Exec=/usr/bin/hearth-mods --nxm %u
Icon=hearth-mods
Type=Application
NoDisplay=true
MimeType=x-scheme-handler/nxm;
DESKTOP
  install -d /usr/lib/systemd/user/default.target.wants
  cat > /usr/lib/systemd/user/hearth-mods-remount.service <<'UNIT'
[Unit]
Description=Hearth Mods: lay overlay mods over their games again

[Service]
Type=oneshot
ExecStart=/usr/bin/hearth-mods --remount

[Install]
WantedBy=default.target
UNIT
  ln -sf ../hearth-mods-remount.service /usr/lib/systemd/user/default.target.wants/hearth-mods-remount.service
fi
# Wabbajack modlists: jackify-engine (Jackify's native Linux build of wabbajack-cli; GPL-3.0, source:
# https://github.com/Omni-guides/dev-jackify-engine), the newest Linux release, in /usr/lib/hearth/jackify-engine
# (hearth-gaming wabbajack.rs). Without it, Hearth Mods downloads it on first use.
if [ "$(uname -m)" = x86_64 ]; then
  jurl="$(curl -fsSL --retry 3 https://api.github.com/repos/Omni-guides/dev-jackify-engine/releases/latest \
    | grep -oE '"browser_download_url": *"[^"]*linux[^"]*x64[^"]*\.tar\.gz"' | head -n1 | sed -E 's/.*"(https[^"]+)"/\1/')" || true
  jtag="$(curl -fsSL --retry 3 https://api.github.com/repos/Omni-guides/dev-jackify-engine/releases/latest | grep -oE '"tag_name": *"[^"]+"' | head -n1 | sed -E 's/.*"([^"]+)"$/\1/')" || true
  if [ -n "$jurl" ] && curl -fsSL --retry 3 -o /tmp/jackify-engine.tar.gz "$jurl"; then
    rm -rf /tmp/jackify-engine && mkdir -p /tmp/jackify-engine && tar -xzf /tmp/jackify-engine.tar.gz -C /tmp/jackify-engine
    jbin="$(find /tmp/jackify-engine -type f -name jackify-engine | head -n1)"
    if [ -n "$jbin" ]; then
      rm -rf /usr/lib/hearth/jackify-engine && mkdir -p /usr/lib/hearth
      cp -a "$(dirname "$jbin")" /usr/lib/hearth/jackify-engine
      chmod 755 /usr/lib/hearth/jackify-engine/jackify-engine
      printf '%s\n' "$jtag" > /usr/lib/hearth/jackify-engine/hearth-version.txt
      printf 'jackify-engine %s, GPL-3.0 (based on Wabbajack, GPL-3.0).\nSource: https://github.com/Omni-guides/dev-jackify-engine\n' "$jtag" > /usr/lib/hearth/jackify-engine/SOURCE.txt
    else echo "warning: the jackify-engine download has no jackify-engine in it; Hearth Mods downloads it on first use"; fi
    rm -rf /tmp/jackify-engine /tmp/jackify-engine.tar.gz
  else echo "warning: couldn't download jackify-engine; Hearth Mods downloads it on first use"; fi
fi
# ---- end of Hearth's gaming apps

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
# The login screen's desktop choice (saved as yours; "Hearth" itself starts the saved one): Hearth Classic, and Hearth
# Plasma (hidden by its TryExec where Plasma isn't installed).
for s in hearth-classic hearth-plasma hearth-gnome; do
  sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' "/ctx/session/$s.desktop" > "/usr/share/wayland-sessions/$s.desktop"
done
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh /usr/share/nimbus/desktop/shell/hearth-pad /usr/share/nimbus/desktop/shell/hearth-buttons /usr/share/nimbus/desktop/shell/hearth-controller /usr/share/nimbus/desktop/shell/hearth-power /usr/share/nimbus/desktop/shell/hearth-browser /usr/share/nimbus/desktop/shell/hearth-tasks /usr/share/nimbus/desktop/shell/hearth-files /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper
# hearth-debbox (.deb files through Hearth's Debian box) for terminal users too, and the Install Package window's
# launcher (the .deb handler).
chmod 755 /usr/share/nimbus/desktop/shell/hearth-debbox /usr/share/nimbus/desktop/shell/hearth-flatpak /usr/bin/hearth-install-package
ln -sfn /usr/share/nimbus/desktop/shell/hearth-debbox /usr/bin/hearth-debbox
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
Icon=hearth-files
Type=Application
Categories=System;FileManager;Utility;
MimeType=inode/directory;x-directory/normal;application/zip;application/x-tar;application/x-compressed-tar;application/x-xz-compressed-tar;application/x-bzip2-compressed-tar;application/x-zstd-compressed-tar;application/gzip;application/x-gzip;application/x-xz;application/x-bzip2;application/zstd;application/x-7z-compressed;application/vnd.rar;application/x-rar;
Keywords=files;folder;explorer;browse;usb;sd;card;copy;move;
OnlyShowIn=Nimbus;
DESKTOP
# Folders open in Hearth Files in Desktop Mode (XDG_CURRENT_DESKTOP=Nimbus), and archives (it unpacks them): Hearth's
# default apps, below.
# ---- Hearth Text (hosts/hearth-text): Hearth OS's text editor, built by CI next to the Hearth app (base-*.yml) and
# carried in os/nimbus-app. Its menu entry (hearth-text.desktop, icon hearth-text from the Hearth icon set) comes from
# os/system_files. In Desktop
# Mode it opens text files, scripts, settings files and logs (its MimeType: Hearth's default apps, below), and Hearth
# Files opens text-like files without an app in it.
if [ -x /ctx/nimbus-app/hearth-text ]; then
  install -Dm755 /ctx/nimbus-app/hearth-text /usr/bin/hearth-text
  # An editor that can't start is worse than none: then text files open in whatever else is installed.
  if ldd /usr/bin/hearth-text | grep 'not found'; then echo "Hearth Text needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/hearth-text; fi
fi
if [ -x /usr/bin/hearth-text ]; then
  command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
fi
# ---- end of Hearth Text
cat > /usr/share/applications/hearth-tasks.desktop <<DESKTOP
[Desktop Entry]
Name=Task Manager
Comment=See what's running, close apps that stopped responding, and check performance
Exec=quickshell ipc -p /usr/share/nimbus/desktop/shell call tasks open
Icon=hearth-tasks
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
Icon=hearth-settings
Type=Application
Categories=Settings;
Keywords=wifi;bluetooth;sound;display;controller;update;
DESKTOP
# (Hearth's own app store, Apps / hearth-apps.desktop, is archived: Bazaar is the app store, installed for everyone by
# hearth-flatpak-setup. docs/ARCHIVED.md)
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
# Links open in it in Desktop Mode and Game Mode (XDG_CURRENT_DESKTOP=Nimbus), unless you pick another browser
# (Hearth's default apps, below). .deb files open in Install Package (Hearth's Debian box) the same way: its MimeType.

# ------------------------------------------------------------------ Hearth Player and Hearth Calculator
# ---- hosts/hearth-player and hosts/hearth-calculator, built by CI with the app (base-peppermint.yml) and carried in
# os/nimbus-app; their menu entries are os/system_files' hearth-player.desktop and hearth-calculator.desktop.
# The player plays through WebKitGTK (GStreamer: gstreamer1.0-libav is Debian's full FFmpeg) and edits with ffmpeg
# (Debian's has libx264, libx265 and every common decoder). What ended up in is in /usr/share/nimbus/media-codecs.txt.
"${APT[@]}" ffmpeg gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav gstreamer1.0-tools
optional gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-vaapi
for app in hearth-player hearth-calculator; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    # An app that can't start is worse than none (its menu entry hides itself without it: TryExec).
    if ldd "/usr/bin/$app" | grep 'not found'; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
  else
    echo "warning: $app wasn't built for this image"
  fi
done
# Videos, music and playlists open in Hearth Player: every type in hearth-player.desktop (os/system_files) is made
# its default with the other Hearth apps' (Hearth's default apps, below).
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
# What plays and what edits: in the build log and in the image.
{
  echo "Hearth Player media support, $(date -u +%F)"
  echo "ffmpeg: $(ffmpeg -hide_banner -version 2>/dev/null | head -1 || echo missing)"
  echo "--- ffmpeg decoders"
  ffmpeg -hide_banner -decoders 2>/dev/null | awk '{print $2}' | grep -xE 'h264|hevc|av1|libdav1d|vp8|vp9|libvpx-vp9|mpeg2video|mpeg4|aac|mp3|mp3float|flac|opus|libopus|vorbis|ac3|eac3|dca|truehd|alac|wmav2|subrip|ass|mov_text' | tr '\n' ' ' || true
  echo; echo "--- ffmpeg encoders"
  ffmpeg -hide_banner -encoders 2>/dev/null | awk '{print $2}' | grep -xE 'libx264|libopenh264|libx265|h264_vaapi|hevc_vaapi|libsvtav1|libaom-av1|libvpx-vp9|libvpx|mpeg4|aac|libfdk_aac|libmp3lame|libopus|libvorbis|flac|pcm_s16le|wavpack|wmav2' | tr '\n' ' ' || true
  echo; echo "--- GStreamer (what WebKitGTK plays with)"
  for e in avdec_h264 avdec_h265 avdec_av1 openh264dec dav1ddec vp9dec vah264dec vah265dec avdec_aac faad avdec_mp3 mpg123audiodec flacdec opusdec vorbisdec avdec_ac3 matroskademux qtdemux webmmux; do
    if gst-inspect-1.0 --exists "$e" 2>/dev/null; then echo "$e yes"; else echo "$e no"; fi
  done | tr '\n' ' ' || true
  echo
} | tee /usr/share/nimbus/media-codecs.txt || true
# The same as a notice on the CI run (the image build's output reaches the workflow).
echo "::notice title=Hearth Player codecs::$(tr '\n' ' ' < /usr/share/nimbus/media-codecs.txt | cut -c1-3500)" || true
# ---- end of Hearth Player and Hearth Calculator


# ------------------------------------------------------------------ Hearth Photos and Hearth Documents
# (hosts/hearth-photos, hosts/hearth-documents) Hearth's picture viewer/editor and its reader for PDFs, e-books (EPUB)
# and comic books (CBZ), built by CI with the app (base-peppermint.yml) and carried in os/nimbus-app (their icons,
# hearth-photos and hearth-documents, come with the Hearth icon set above).
# Their menu entries and file types are set here, and they're the default apps for pictures and documents (Hearth's
# default apps, after the last app).
# libheif's tools let Photos open HEIC pictures from phones; wl-clipboard is how it copies a picture.
"${APT[@]}" libheif-examples libheif-plugin-libde265 wl-clipboard || true
for app in hearth-photos hearth-documents; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
  fi
done
HEARTH_PICTURE_TYPES="image/jpeg;image/png;image/gif;image/webp;image/bmp;image/x-bmp;image/x-ms-bmp;image/tiff;image/svg+xml;image/avif;image/heic;image/heif;image/jxl;image/x-icon;image/vnd.microsoft.icon;image/apng;"
HEARTH_DOCUMENT_TYPES="application/pdf;application/x-pdf;application/epub+zip;application/vnd.comicbook+zip;application/x-cbz;"
if [ -x /usr/bin/hearth-photos ]; then
  cat > /usr/share/applications/hearth-photos.desktop <<DESKTOP
[Desktop Entry]
Name=Photos
GenericName=Picture Viewer
Comment=Look through your pictures, and crop, straighten or brighten them
Exec=/usr/bin/hearth-photos %U
TryExec=/usr/bin/hearth-photos
Icon=hearth-photos
Type=Application
Categories=Graphics;Viewer;RasterGraphics;2DGraphics;Photography;
MimeType=${HEARTH_PICTURE_TYPES}
Keywords=photo;picture;image;viewer;gallery;slideshow;crop;rotate;edit;
StartupNotify=true
StartupWMClass=hearth-photos
Actions=slideshow;

[Desktop Action slideshow]
Name=Slideshow of your Pictures
Exec=/usr/bin/hearth-photos --slideshow
DESKTOP
fi
if [ -x /usr/bin/hearth-documents ]; then
  cat > /usr/share/applications/hearth-documents.desktop <<DESKTOP
[Desktop Entry]
Name=Documents
GenericName=Document Viewer
Comment=Read PDFs, e-books and comic books
Exec=/usr/bin/hearth-documents %U
TryExec=/usr/bin/hearth-documents
Icon=hearth-documents
Type=Application
Categories=Office;Viewer;Graphics;
MimeType=${HEARTH_DOCUMENT_TYPES}
Keywords=pdf;document;viewer;reader;ebook;epub;book;comic;cbz;
StartupNotify=true
StartupWMClass=hearth-documents
DESKTOP
fi
# They're the defaults for those types with the other Hearth apps (Hearth's default apps, below).
gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database -q /usr/share/applications 2>/dev/null || true
# ---- end of Hearth Photos and Hearth Documents
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
org.freedesktop.impl.portal.FileChooser=hearth
PORTALS
# ---- Hearth Files: every app's file chooser, and the file manager apps open (desktop/shell/hearth-portal, picker.qml)
# Open / Save dialogs: xdg-desktop-portal's FileChooser is Hearth's in Hearth's session (above; XDG_CURRENT_DESKTOP is
# Nimbus in both modes), the rest stays on the backends it had. hearth.portal and the D-Bus service files come from
# os/system_files. nimbus-session asks GTK and Qt apps to use the portal; Hearth Browser's prefs ask Firefox.
# Folders: Hearth Files is the default in every desktop's list as well as Desktop Mode's (hearth-filemanager hands
# them to Thunar outside Hearth's session), and "Show in folder" (org.freedesktop.FileManager1) is
# held by Hearth's session, so no other file manager in this image takes over.
# (hearth-system defaults-system writes them with Hearth's other default apps, after the last app: EVERY_DESKTOP_TOO.)
# The backend is Python with GLib's D-Bus (python3-gi, above); Qt apps' portal dialogs need the xdgdesktopportal
# platform theme, and the picker in Game Mode (gamescope) Qt's X11 platform plugin.
optional qt6-qpa-plugins qt6-xdgdesktopportal-platformtheme qt5-xdgdesktopportal-platformtheme
chmod 755 /usr/share/nimbus/desktop/shell/hearth-portal
# Firefox from the distribution, if there is one, also uses the portal (Hearth Browser has it in its own prefs).
for d in /usr/lib64/firefox /usr/lib/firefox /usr/lib/firefox-esr; do
  if [ -d "$d" ]; then install -Dm644 /usr/share/hearth-browser/firefox/defaults/pref/hearth-portal.js "$d/defaults/pref/hearth-portal.js"; fi
done
command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
# ---- end of Hearth Files as the file chooser

# ------------------------------------------------------------------ Hearth Contacts and Hearth Maps (hosts/hearth-contacts, hosts/hearth-maps)
# Built with the app (base-peppermint.yml) and carried in os/nimbus-app; their icons (Icon=hearth-contacts, hearth-maps)
# come with Hearth's icon set above.
# Contacts opens .vcf files and keeps CardDAV passwords in GNOME Keyring (Hearth's session starts it); Maps opens
# geo: links and asks GeoClue for "Where am I" only when that button is pressed (docs/HEARTH-MAPS.md).
"${APT[@]}" geoclue-2.0 libsecret-1-0 || true
install -d /usr/share/applications
for app in hearth-contacts hearth-maps; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so this image goes without it"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
done
if [ -x /usr/bin/hearth-contacts ]; then
  cat > /usr/share/applications/hearth-contacts.desktop <<DESKTOP
[Desktop Entry]
Name=Contacts
GenericName=Address Book
Comment=Names, numbers, emails and addresses, kept in step with Nextcloud, iCloud or Fastmail if you like
Exec=/usr/bin/hearth-contacts %f
Icon=hearth-contacts
Type=Application
Categories=Office;ContactManagement;
MimeType=text/vcard;text/x-vcard;text/directory;
Keywords=contacts;address;book;people;phone;email;vcard;vcf;carddav;
StartupWMClass=hearth-contacts
DESKTOP
  # .vcf files open in it (Hearth's default apps, below).
fi
if [ -x /usr/bin/hearth-maps ]; then
  cat > /usr/share/applications/hearth-maps.desktop <<DESKTOP
[Desktop Entry]
Name=Maps
GenericName=Maps
Comment=OpenStreetMap maps: find places and get directions
Exec=/usr/bin/hearth-maps %u
Icon=hearth-maps
Type=Application
Categories=Utility;Maps;
MimeType=x-scheme-handler/geo;
Keywords=map;maps;openstreetmap;osm;directions;route;navigation;places;geo;location;
StartupWMClass=hearth-maps
DESKTOP
  # GeoClue may tell Hearth Maps the location without asking an agent (Hearth's session has none). Maps only asks
  # when the person presses Where am I, and GeoClue still decides where the location comes from.
  install -d /etc/geoclue/conf.d
  printf '[hearth-maps]\nallowed=true\nsystem=false\nusers=\n' > /etc/geoclue/conf.d/90-hearth-maps.conf
fi
update-desktop-database -q /usr/share/applications 2>/dev/null || true
# ---- end of Hearth Contacts and Hearth Maps

# ------------------------------------------------------------------ Hearth Friends (hosts/hearth-friends)
# Friends, chats and game invites on Matrix (end-to-end encrypted), for Game Mode's Friends screen, Quick Access and
# Mantel's Friends panel. Built with the app (base-peppermint.yml) and carried in os/nimbus-app. Its session and store key
# live in GNOME Keyring (the Secret Service, which Hearth's session starts). A user service, started with the session
# (Hearth's core and `hearth-friends call` also start it when it isn't running). docs/FRIENDS.md.
"${APT[@]}" libsecret-1-0 || true
if [ -x /ctx/nimbus-app/hearth-friends ]; then
  install -Dm755 /ctx/nimbus-app/hearth-friends /usr/bin/hearth-friends
  if ldd /usr/bin/hearth-friends | grep "not found"; then echo "warning: hearth-friends is missing the libraries above"; fi
  install -d /usr/lib/systemd/user
  cat > /usr/lib/systemd/user/hearth-friends.service <<'UNIT'
[Unit]
Description=Hearth Friends (Matrix chat, presence and game invites)
After=graphical-session-pre.target

[Service]
ExecStart=/usr/bin/hearth-friends serve
Restart=on-failure
RestartSec=5
Nice=5

[Install]
WantedBy=default.target
UNIT
  systemctl --global enable hearth-friends.service 2>/dev/null \
    || { mkdir -p /etc/systemd/user/default.target.wants && ln -sf /usr/lib/systemd/user/hearth-friends.service /etc/systemd/user/default.target.wants/hearth-friends.service; }
else
  echo "warning: hearth-friends wasn't built, so this image goes without Hearth Friends"
fi
# The chat's emoji picker uses Emoji & symbols' list (hosts/hearth-emoji/ui/emoji-data.js).
[ -s /ctx/nimbus-app/emoji-data.js ] && install -Dm644 /ctx/nimbus-app/emoji-data.js /usr/share/nimbus/emoji-data.js
# ---- end of Hearth Friends
# ---- Hearth Office (hosts/hearth-office): Write, Sheets and Slides, built by CI with the app (base-peppermint.yml) and
# carried in os/nimbus-app/office. office/install.sh puts in the program (hearth-write, hearth-sheets and
# hearth-slides are links to it), the menu entries and the .hslides file type (the icons are in Hearth's icon set); they're
# the default apps for office files with the others (Hearth's default apps, below). Fonts with
# the same sizes as Office's (Calibri, Cambria, Arial, Times New Roman), so documents keep their layout.
"${APT[@]}" fonts-crosextra-carlito fonts-crosextra-caladea fonts-liberation2 || true
if [ -x /ctx/nimbus-app/office/hearth-office ]; then sh /ctx/nimbus-app/office/install.sh
else echo "Hearth Office wasn't built; this image goes without it"; fi
# ---- end of Hearth Office

# ------------------------------------------------------------------ Hearth's KDE apps (hosts/hearth-kde, docs/KDE-APPS.md)
# Hearth Terminal (Konsole), Hearth Text (Kate), Reader (Okular), Capture (Spectacle) and Hearth Calculator (KCalc), the
# base's own KDE packages with Hearth's names, icons and colors; Hearth Files built from Dolphin and kio-hearth (hearth:/
# in every KDE app) from CI's build inside this base (os/nimbus-app/hearth-kde.tar), only when they built, passed their
# tests and start here (otherwise the old Hearth Files stays the file manager). The old Terminal, Text Editor and
# Calculator stay installed as "… (classic)". Never stops the image build.
bash /ctx/hearth-kde/install.sh /ctx/nimbus-app/hearth-kde.tar || echo "warning: Hearth's KDE apps didn't install"

# ------------------------------------------------------------------ Hearth's default apps
# Every Hearth app in the image is the default for the files and links it opens (its .desktop file's MimeType), in
# Desktop Mode's and Game Mode's list (/etc/xdg/nimbus-mimeapps.list, XDG_CURRENT_DESKTOP=Nimbus: it comes before
# every list the base's own apps bring) and in every desktop's (/usr/share/applications/mimeapps.list). One place,
# written once every app is in: hearth-system defaults-system (desktop/shell), which also picks between two Hearth
# apps for the same type (Photos before Paint, Text before Write, Sheets for CSV). Settings › Default apps saves
# your own choices in ~/.config/mimeapps.list, which comes first; nimbus-session takes Hearth apps that aren't
# installed any more out of it at each sign-in.
mkdir -p /etc/xdg
python3 /usr/share/nimbus/desktop/shell/hearth-system defaults-system / | tee /tmp/hearth-defaults.json
echo "::notice title=Hearth's default apps::$(cat /tmp/hearth-defaults.json)" || true
# Lists the base brings that Hearth's session would read after its own (shown, not changed).
echo "Other default-app lists in the image: $(ls /etc/xdg/*mimeapps.list /usr/share/applications/*mimeapps.list 2>/dev/null | tr '\n' ' ')"
update-desktop-database -q /usr/share/applications 2>/dev/null || true

# ------------------------------------------------------------------ one app of each kind per desktop (docs/APP-KINDS.md)
# >>> app kinds (feature/app-per-desktop)
# The table /usr/share/hearth/app-kinds.json says which app each desktop shows for each kind (Hearth Classic: Hearth's
# own; Hearth Plasma: the KDE-based ones; apps with no other of their kind everywhere). hearth-app-kinds writes copies
# of the menu entries with OnlyShowIn/NotShowIn into /usr/share/hearth/app-kinds/applications (nimbus-session puts it
# first in XDG_DATA_DIRS, so the packages' own files are never edited), each desktop's default apps
# (/etc/xdg/<desktop>-mimeapps.list), and stops the build if a desktop would show two apps of one kind or none
# where one is needed. Runs after Hearth's default apps (above), once every app is in.
python3 /usr/libexec/hearth-app-kinds apply --root / > /tmp/hearth-app-kinds.json || { cat /tmp/hearth-app-kinds.json; echo "::error title=app kinds::a desktop would show two apps of one kind, or none (above)"; exit 1; }
echo "::notice title=One app of each kind::$(python3 -c 'import json; d=json.load(open("/tmp/hearth-app-kinds.json")); print("; ".join("%s: %s" % (k, ", ".join("%s=%s" % (a, b) for a, b in sorted(v["apps"].items()) if b)) for k, v in d["desktops"].items() if v["present"]))' 2>/dev/null)" || true
chmod 755 /usr/libexec/hearth-app-kinds
# Writable Peppermint: apt brings new or updated menu entries, so the copies are written again after every apt run
# (never fails apt; an app you install yourself isn't hidden: --refresh).
mkdir -p /etc/apt/apt.conf.d
cat > /etc/apt/apt.conf.d/80-hearth-app-kinds <<'APT'
DPkg::Post-Invoke { "if [ -x /usr/libexec/hearth-app-kinds ]; then /usr/libexec/hearth-app-kinds apply --refresh >/dev/null 2>&1 || true; fi"; };
APT
# <<< app kinds

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

# ------------------------------------------------------------------ Xodus Nexus (Xbox PC and PC Game Pass games)
# Shipped like Beacon: the AppImage in /usr/lib/xodus-nexus (with its version), `xodus-nexus` to start it (the same
# script hosts/hearth-core/src/xodus.rs writes for a copy it downloads) and xodus-nexus.desktop (in Mantel's launcher
# and pinned to its taskbar). Hearth's library shows it under Xbox PC; a newer release is offered in Settings and goes
# in the user's own data folder, since the image is read-only. Nexus is an independent project built on Xodus
# (github.com/domsvoboda/xodus-nexus), not made by Microsoft. It publishes only an AppImage, so its download address
# comes from the latest release's asset list. Optional: without a release to download, the build says so and carries on.
XODUS_API=https://api.github.com/repos/domsvoboda/xodus-nexus/releases/latest
xodus_json="$(curl -fsSL --retry 3 "$XODUS_API" 2>/dev/null || true)"
XODUS_URL="$(printf '%s' "$xodus_json" | python3 -c 'import json, sys; a=[x for x in json.load(sys.stdin).get("assets", []) if x["name"].lower().endswith(".appimage") and ("x86_64" in x["name"].lower() or "amd64" in x["name"].lower())]; print(a[0]["browser_download_url"] if a else "")' 2>/dev/null || true)"
XODUS_VER="$(printf '%s' "$xodus_json" | python3 -c 'import json, sys; print(json.load(sys.stdin).get("tag_name", "").lstrip("v"))' 2>/dev/null || true)"
if [ "$(uname -m)" = x86_64 ] && [ -n "$XODUS_URL" ] && install -d /usr/lib/xodus-nexus \
   && curl -fsSL --retry 3 -o /usr/lib/xodus-nexus/Xodus-Nexus.AppImage "$XODUS_URL"; then
  chmod 755 /usr/lib/xodus-nexus/Xodus-Nexus.AppImage
  printf '%s\n' "$XODUS_VER" > /usr/lib/xodus-nexus/version
  cat > /usr/bin/xodus-nexus <<'EOF'
#!/bin/sh
# Starts Xodus Nexus (shipped with Hearth OS; Hearth's library runs this).
# AppImages need FUSE; without it, unpack and run instead.
if ! command -v fusermount >/dev/null 2>&1 && ! command -v fusermount3 >/dev/null 2>&1; then export APPIMAGE_EXTRACT_AND_RUN=1; fi
# WebKit's newer GPU path draws a black window on some Nvidia drivers.
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
# Sign-in pages open in your default browser (Hearth Browser unless you picked another), not whichever it finds first.
export BROWSER="${BROWSER:-xdg-open}"
exec /usr/lib/xodus-nexus/Xodus-Nexus.AppImage "$@"
EOF
  chmod 755 /usr/bin/xodus-nexus
  cat > /usr/share/applications/xodus-nexus.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Xodus Nexus
GenericName=Xbox PC games
Comment=Xbox PC and PC Game Pass games (an independent launcher, not made by Microsoft)
Exec=xodus-nexus %U
Icon=applications-games
Terminal=false
Categories=Game;
EOF
  echo "Xodus Nexus ${XODUS_VER:-?} installed"
else
  rm -rf /usr/lib/xodus-nexus
  echo "warning: couldn't install Xodus Nexus from $XODUS_API (no release with an AppImage?); Hearth works without it"
fi

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
getent group netdev >/dev/null || groupadd -r netdev   # Wi-Fi and network changes (49-hearth-network.rules, hearth-groups)
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
# When the device last woke up (/run/hearth-resumed) and each sleep in the journal (hearth-sleep): the press that
# woke it is then not taken as a new one (os/system_files).
chmod 755 /usr/lib/systemd/system-sleep/hearth-resumed

# ------------------------------------------------------------------ services
systemctl enable NetworkManager.service bluetooth.service systemd-resolved.service systemd-timesyncd.service \
  fstrim.timer bootc-fetch-apply-updates.timer sddm.service
systemctl enable hearth-bls-grub.service hearth-bls-grub-refresh.path
systemctl enable hearth-boot-guard.service hearth-boot-guard-refresh.path hearth-boot-good.service hearth-recovery.service  # automatic fallback, Recovery on the device to the previous version
# Writable system (hearth-unlock, docs/BASE-PEPPERMINT.md): each version's own layer over /usr at every start, the
# packages you added put back after an update, a safe start if your changes stop Hearth from starting.
systemctl enable hearth-unlock.service hearth-unlock-units.service hearth-unlock-good.service hearth-unlock-reapply.service
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
systemctl enable hearth-home-repair.service   # home folders' Flatpak/cache folders yours, container id ranges
# Flatpak's system installation set up (and repaired), Flathub in it, Bazaar (the Flathub store) for everyone. Debian
# 13 has no Bazaar package, so it's Flathub's, installed at the first boot that's online (existing installs too).
systemctl enable hearth-flatpak-setup.service
# /etc/hosts maps the device's name (sudo: "unable to resolve host <name>"), at boot and after a rename; glibc's
# myhostname module answers for the name too, whatever /etc/hosts says.
systemctl enable hearth-hosts.service hearth-hosts.path
optional libnss-myhostname
if [ -f /etc/nsswitch.conf ] && ! grep -Eq '^hosts:.*myhostname' /etc/nsswitch.conf && [ -e /usr/lib/x86_64-linux-gnu/libnss_myhostname.so.2 ]; then
  sed -i -E 's/^(hosts:[[:space:]]+files)/\1 myhostname/' /etc/nsswitch.conf
fi
grep '^hosts:' /etc/nsswitch.conf || true
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
# Writable system (hearth-unlock): what the image itself asked apt for (the start of the record of your packages),
# and apt's auto/manual marks moved next to dpkg's database, so each version's marks match its own packages. Then
# apt's and dpkg's hooks and the boot-chain pin (from here on: none of them may run during the build).
mkdir -p /usr/share/hearth /usr/lib/sysimage/apt
apt-mark showmanual | sort -u > /usr/share/hearth/apt-baseline.list
test -s /usr/share/hearth/apt-baseline.list
if [ -f /var/lib/apt/extended_states ]; then cp -a /var/lib/apt/extended_states /usr/lib/sysimage/apt/extended_states; else : > /usr/lib/sysimage/apt/extended_states; fi
install -Dm644 /usr/share/hearth/unlock/apt.conf /etc/apt/apt.conf.d/50-hearth-unlock
install -Dm644 /usr/share/hearth/unlock/dpkg.cfg /etc/dpkg/dpkg.cfg.d/hearth-unlock
install -Dm644 /usr/share/hearth/unlock/boot-chain.pref /etc/apt/preferences.d/hearth-boot-chain
chmod 755 /usr/bin/hearth-unlock
python3 -m py_compile /usr/bin/hearth-unlock && rm -rf /usr/bin/__pycache__
# The same marks through the new path (a mismatch would put the image's own packages in the record).
diff <(apt-mark showmanual | sort -u) /usr/share/hearth/apt-baseline.list
mv /var/lib/dpkg /usr/lib/sysimage/dpkg
VARCONF=/usr/lib/tmpfiles.d/hearth-var.conf
: > "$VARCONF"
# Not /var/lib/flatpak: flatpak makes its system installation itself; an empty repo folder made at boot (as up to
# 0.6.0) is one it can't open ("opening repo: openat(config): No such file or directory").
find /var -mindepth 1 -type d \
  ! -path '/var/cache*' ! -path '/var/log*' ! -path '/var/tmp*' ! -path '/var/run*' ! -path '/var/lock*' \
  ! -path '/var/home*' ! -path '/var/roothome*' ! -path '/var/opt*' ! -path '/var/srv*' \
  ! -path '/var/mnt*' ! -path '/var/usrlocal*' ! -path '/var/lib/apt/lists*' ! -path '/var/lib/dkms*' \
  ! -path '/var/lib/flatpak*' \
  -printf 'd /var/%P %m %u %g -\n' | sort >> "$VARCONF"
echo 'L+ /var/lib/dpkg - - - - /usr/lib/sysimage/dpkg' >> "$VARCONF"
# apt's own folders (apt makes the rest), for apt on the unlocked system.
{
  echo 'd /var/lib/apt/lists 0755 root root -'
  echo 'd /var/lib/apt/lists/partial 0700 _apt root -'
  echo 'd /var/cache/apt 0755 root root -'
  echo 'd /var/cache/apt/archives 0755 root root -'
  echo 'd /var/cache/apt/archives/partial 0700 _apt root -'
  echo 'd /var/log/apt 0755 root root -'
} >> "$VARCONF"
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
