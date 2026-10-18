#!/usr/bin/bash
# Turns the Hearth image into a live system: boots from the USB stick into Hearth Desktop Mode as "liveuser",
# with a welcome card (Install Hearth / Wi-Fi / Try Game Mode) and Fedora's installer (Anaconda), branded and
# set up to install ghcr.io/dragongamer9/hearth. Based on Titanoboa's Bazzite example.
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
touch /etc/hearth-live                     # Hearth's session and desktop look for this
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

# ---------------------------------------------------------------- the installer (Anaconda, web UI)
dnf install -y --allowerasing firefox anaconda-live libblockdev-btrfs libblockdev-lvm libblockdev-dm
source /etc/os-release
cat >/etc/anaconda/profile.d/hearth.conf <<CONF
[Profile]
profile_id = hearth

[Profile Detection]
os_id = ${ID}

[Network]
default_on_boot = FIRST_WIRED_WITH_LINK

[Bootloader]
efi_dir = fedora
menu_auto_hide = True

[Storage]
default_scheme = BTRFS
btrfs_compression = zstd:1
default_partitioning =
    /     (min 1 GiB, max 70 GiB)
    /home (min 500 MiB, free 50 GiB)
    /var  (btrfs)

[User Interface]
hidden_spokes =
    PasswordSpoke
hidden_webui_pages =
    root-password

[Localization]
use_geolocation = False
CONF
echo "Hearth release ${VERSION_ID}" >/etc/system-release

# Hearth's mark wherever the installer shows a logo.
for f in /usr/share/anaconda/pixmaps/*logo*.png /usr/share/anaconda/pixmaps/*/*logo*.png; do
  [ -f "$f" ] && cp /src/branding/hearth-512.png "$f"
done
for f in /usr/share/icons/hicolor/scalable/apps/org.fedoraproject.AnacondaInstaller.svg \
         /usr/share/icons/hicolor/48x48/apps/org.fedoraproject.AnacondaInstaller.svg; do
  [ -e "$f" ] && cp /src/branding/hearth.svg "$f"
done
sed -i -e 's/^Name=.*/Name=Install Hearth/' -e 's/^Comment=.*/Comment=Install Hearth on this device/' \
  /usr/share/applications/liveinst.desktop 2>/dev/null || true

# What it installs: Hearth straight from the registry (needs internet while installing; keeps the USB image
# small), updating from there afterwards like any Hearth.
cat >>/usr/share/anaconda/interactive-defaults.ks <<KS
%pre-install --erroronfail
rm -rf /mnt/sysroot/boot/efi/EFI/fedora
%end

ostreecontainer --url=${IMAGE_REF}:${IMAGE_TAG} --transport=registry --no-signature-verification

%post --erroronfail
systemctl disable flatpak-add-fedora-repos.service || :
%end
KS

# ---------------------------------------------------------------- titanoboa's ISO config
mkdir -p /usr/lib/bootc-image-builder
cp /src/iso.yaml /usr/lib/bootc-image-builder/iso.yaml
dnf clean all -q || true
