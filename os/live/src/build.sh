#!/usr/bin/bash
# Turns the Hearth image into a live system: boots from the USB stick into Hearth Desktop Mode as "liveuser",
# with a welcome card (Install Hearth OS / Wi-Fi / Try Game Mode) and Hearth's own installer (hearth-install,
# with screens in the Hearth app and in Mantel), which installs ghcr.io/dragongamer9/hearth. Based on
# Titanoboa's Bazzite example.
set -exo pipefail
IMAGE_REF=ghcr.io/dragongamer9/${IMAGE_NAME:-hearth}
IMAGE_TAG=latest

mkdir -p "$(realpath /root)" /var/lib/rpm-state
dnf -qy versionlock clear || true

# ---------------------------------------------------------------- live boot
KVER=$(find /usr/lib/modules -maxdepth 1 -mindepth 1 -type d -printf '%P\n' | head -1)
dnf install -y dracut-live
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "dmsquash-live dmsquash-live-autooverlay plymouth" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"

# livesys creates "liveuser" (no password, can use sudo) at boot.
dnf install -y livesys-scripts
sed -i "s/^livesys_session=.*/livesys_session=kde/" /etc/sysconfig/livesys
systemctl enable livesys.service livesys-late.service

# Titanoboa needs the cdboot EFI binary and the EFI tree under /boot/efi.
dnf install -y grub2-efi-x64-cdboot xorriso isomd5sum
mkdir -p /boot/efi
cp -av /usr/lib/efi/*/*/EFI /boot/efi/
cp -v /boot/efi/EFI/fedora/grubx64.efi /boot/efi/EFI/BOOT/fbx64.efi || true

# /var/tmp on a live system lives in a small tmpfs under /run: give it room (the installer needs it).
rm -rf /var/tmp; mkdir -p /var/tmp
cat >/etc/systemd/system/var-tmp.mount <<'UNIT'
[Unit]
Description=Larger tmpfs for /var/tmp on the live system
[Mount]
What=tmpfs
Where=/var/tmp
Type=tmpfs
Options=size=50%%,nr_inodes=1m
[Install]
WantedBy=local-fs.target
UNIT
systemctl enable var-tmp.mount
rm -f /etc/localtime; systemd-firstboot --timezone UTC

# ---------------------------------------------------------------- the live session
echo "${IMAGE_NAME:-hearth}" > /etc/hearth-live   # Hearth's session and desktop look for this; the installer reads the edition
# Boot into Hearth (the live user is the first account, so Hearth's own autologin picks it up), in Desktop
# Mode, where the installer is easy to use with the controller mouse and on-screen keyboard.
mkdir -p /etc/systemd/system/nimbus-autologin.service.d
printf '[Unit]\nAfter=livesys.service livesys-late.service\n' > /etc/systemd/system/nimbus-autologin.service.d/live.conf
mkdir -p /etc/skel/.config/nimbus
echo 'NIMBUS_START_MODE=desktop' > /etc/skel/.config/nimbus/session.conf
# The live user may do anything without a password prompt (it's a live USB).
cat >/etc/polkit-1/rules.d/10-hearth-live.rules <<'RULE'
polkit.addRule(function(action, subject) { if (subject.user == "liveuser") return polkit.Result.YES; });
RULE
# Nothing that writes to disk, updates or downloads in the background.
for s in rpm-ostreed-automatic.timer uupd.timer bootloader-update.service rpm-ostree-countme.service \
         ublue-system-setup.service bazzite-hardware-setup.service check-sb-key.service brew-setup.service; do
  systemctl disable "$s" 2>/dev/null || true
done
for s in ublue-flatpak-manager.service ublue-user-setup.service podman-auto-update.timer; do
  systemctl --global disable "$s" 2>/dev/null || true
done

# ---------------------------------------------------------------- the installer (Hearth's own)
# The engine (hearth-install) partitions, downloads Hearth OS onto the drive with ostree and sets up the boot
# loader with bootupd; its screens are in the Hearth app (controller) and Mantel's Settings (mouse/keyboard).
# "Install next to Windows" shrinks Windows' NTFS drive first (hearth-live-prep, ntfsresize).
dnf install -y ntfsprogs dosfstools e2fsprogs btrfs-progs efibootmgr
install -Dm755 /src/hearth-live-prep /usr/libexec/nimbus/hearth-live-prep
install -Dm755 /src/hearth-install /usr/libexec/nimbus/hearth-install
source /etc/os-release
echo "Hearth release ${VERSION_ID}" >/etc/system-release
# "Install Hearth OS" in the app launcher: the welcome card's "how do you want to install" choice.
cat >/usr/share/applications/hearth-install.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Install Hearth OS
Comment=Install Hearth OS on this PC, next to Windows or on a whole drive
Exec=sh -c 'quickshell ipc -p "${NIMBUS_SHARE:-/usr/share/nimbus}/desktop/shell" call live install'
Icon=/usr/share/pixmaps/hearth.png
Categories=System;
DESKTOP
install -Dm644 /src/branding/hearth-512.png /usr/share/pixmaps/hearth.png

# ---------------------------------------------------------------- titanoboa's ISO config
mkdir -p /usr/lib/bootc-image-builder
cp /src/iso.yaml /usr/lib/bootc-image-builder/iso.yaml
dnf clean all -q || true
