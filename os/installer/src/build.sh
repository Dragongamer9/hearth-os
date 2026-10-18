#!/usr/bin/bash
# Turns Fedora's small bootable image into Hearth's installer USB. See ../Containerfile.
set -exo pipefail
mkdir -p "$(realpath /root)" /var/lib/rpm-state

# ---------------------------------------------------------------- what the installer needs
dnf install -y \
  dracut-live grub2-efi-x64-cdboot xorriso isomd5sum \
  cage xorg-x11-server-Xwayland mesa-dri-drivers libglvnd-egl \
  gtk3 nss alsa-lib libXScrnSaver libxkbcommon at-spi2-atk cups-libs libdrm mesa-libgbm \
  NetworkManager NetworkManager-wifi wpa_supplicant bluez polkit pciutils usbutils \
  ntfsprogs ntfs-3g dosfstools e2fsprogs btrfs-progs xfsprogs lvm2 efibootmgr policycoreutils shadow-utils \
  rpm-ostree bootupd plymouth-plugin-script \
  linux-firmware amd-gpu-firmware intel-gpu-firmware nvidia-gpu-firmware iwlwifi-mvm-firmware iwlwifi-dvm-firmware \
  atheros-firmware realtek-firmware mt7xxx-firmware brcmfmac-firmware \
  rsms-inter-fonts google-noto-sans-fonts

# ---------------------------------------------------------------- live boot (like the full live USB)
KVER=$(find /usr/lib/modules -maxdepth 1 -mindepth 1 -type d -printf '%P\n' | head -1)
plymouth-set-default-theme hearth || true
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "dmsquash-live dmsquash-live-autooverlay plymouth" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"
mkdir -p /boot/efi
cp -av /usr/lib/efi/*/*/EFI /boot/efi/
cp -v /boot/efi/EFI/fedora/grubx64.efi /boot/efi/EFI/BOOT/fbx64.efi || true
rm -rf /var/tmp; mkdir -p /var/tmp
cat >/etc/systemd/system/var-tmp.mount <<'UNIT'
[Unit]
Description=Larger tmpfs for /var/tmp on the installer USB
[Mount]
What=tmpfs
Where=/var/tmp
Type=tmpfs
Options=size=50%%,nr_inodes=1m
[Install]
WantedBy=local-fs.target
UNIT
systemctl enable var-tmp.mount NetworkManager bluetooth
rm -f /etc/localtime; systemd-firstboot --timezone UTC || true

# ---------------------------------------------------------------- the Hearth app and the installer
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
[ -f /usr/lib/nimbus/chrome-sandbox ] && chown root:root /usr/lib/nimbus/chrome-sandbox && chmod 4755 /usr/lib/nimbus/chrome-sandbox
install -Dm755 /live/hearth-install /usr/libexec/nimbus/hearth-install
install -Dm755 /live/hearth-live-prep /usr/libexec/nimbus/hearth-live-prep
install -Dm644 /live/branding/hearth-512.png /usr/share/pixmaps/hearth.png
echo auto > /etc/hearth-live        # the installer picks the edition for this PC's graphics
touch /etc/hearth-installer          # the Hearth app shows only the install screens
echo "Hearth installer" >/etc/system-release

# The person using it: "liveuser", no password, may do anything (it's an installer USB).
useradd -m -G wheel -c "Hearth installer" liveuser
passwd -d liveuser
cat >/etc/polkit-1/rules.d/10-hearth-installer.rules <<'RULE'
polkit.addRule(function(action, subject) { if (subject.user == "liveuser") return polkit.Result.YES; });
RULE
echo 'liveuser ALL=(ALL) NOPASSWD: ALL' >/etc/sudoers.d/liveuser

# Boots straight into the Hearth app, full screen (cage: a one-app Wayland screen), on the first screen.
cat >/etc/pam.d/hearth-installer <<'PAM'
auth     required pam_permit.so
account  required pam_permit.so
session  required pam_loginuid.so
session  required pam_systemd.so
session  required pam_unix.so
PAM
cat >/usr/libexec/hearth-installer-session <<'SH'
#!/usr/bin/bash
# The Hearth app on its own; it comes back if it closes.
export XDG_SESSION_TYPE=wayland ELECTRON_OZONE_PLATFORM_HINT=wayland
while :; do
  /usr/lib/nimbus/nimbus --ozone-platform=wayland --enable-features=UseOzonePlatform >>"$HOME/hearth-installer.log" 2>&1
  sleep 1
done
SH
chmod 755 /usr/libexec/hearth-installer-session
cat >/etc/systemd/system/hearth-installer.service <<'UNIT'
[Unit]
Description=Hearth installer (the Hearth app, full screen)
After=systemd-user-sessions.service plymouth-quit-wait.service NetworkManager.service bluetooth.service
Conflicts=getty@tty1.service
[Service]
User=liveuser
WorkingDirectory=~
PAMName=hearth-installer
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
StandardInput=tty-fail
StandardOutput=journal
UtmpIdentifier=tty1
UtmpMode=user
ExecStart=/usr/bin/cage -s -d -- /usr/libexec/hearth-installer-session
Restart=always
RestartSec=1
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-installer.service
systemctl disable getty@tty1.service || true
systemctl set-default graphical.target
# Nothing that updates or writes to disk in the background.
for s in bootc-fetch-apply-updates.timer rpm-ostreed-automatic.timer; do systemctl disable "$s" 2>/dev/null || true; done

# ---------------------------------------------------------------- titanoboa's ISO config
mkdir -p /usr/lib/bootc-image-builder
cp /src/iso.yaml /usr/lib/bootc-image-builder/iso.yaml
dnf clean all -q || true
