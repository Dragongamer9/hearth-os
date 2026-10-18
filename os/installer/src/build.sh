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
  rpm-ostree bootupd \
  linux-firmware amd-gpu-firmware intel-gpu-firmware nvidia-gpu-firmware iwlwifi-mvm-firmware iwlwifi-dvm-firmware \
  atheros-firmware realtek-firmware mt7xxx-firmware brcmfmac-firmware \
  rsms-inter-fonts google-noto-sans-fonts

# ---------------------------------------------------------------- live boot (like the full live USB)
KVER=$(find /usr/lib/modules -maxdepth 1 -mindepth 1 -type d -printf '%P\n' | head -1)
# Fedora's small base image has only the core kernel: graphics (amdgpu, i915, nouveau, virtual machines),
# Wi-Fi, Bluetooth and controller drivers are in kernel-modules. Without them nothing shows on screen.
dnf install -y "kernel-modules-${KVER}" "kernel-modules-extra-${KVER}" || dnf install -y kernel-modules kernel-modules-extra
ls "/usr/lib/modules/${KVER}/kernel/drivers/gpu/drm" | head
# No boot splash on the installer USB: start-up waited for it to close, and the installer screen never came.
dnf remove -y plymouth || true
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "dmsquash-live dmsquash-live-autooverlay" --omit "plymouth" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"
# Titanoboa wants the EFI boot files under /boot/efi. Fedora's base image keeps them in a different place than
# Bazzite (bootupd's copy, or /usr/lib/efi), so take whichever is there.
dnf install -y shim-x64 grub2-efi-x64
mkdir -p /boot/efi
src=$( (ls -d /usr/lib/efi/*/*/EFI /usr/lib/bootupd/updates/EFI 2>/dev/null || true) | head -1)
[ -n "$src" ] && cp -av "$src" /boot/efi/
test -d /boot/efi/EFI/fedora
mkdir -p /boot/efi/EFI/BOOT
[ -e /boot/efi/EFI/BOOT/BOOTX64.EFI ] || cp -v /boot/efi/EFI/fedora/shimx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
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
mkdir -p /usr/share/nimbus && mv /usr/lib/nimbus/hearth-version /usr/share/nimbus/version 2>/dev/null || echo latest > /usr/share/nimbus/version
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
# Graphics this USB has no driver for (a brand-new card, or "basic graphics" picked at start): draw in software.
extra=""
if grep -qw nomodeset /proc/cmdline || [ ! -e /dev/dri/renderD128 ]; then extra="--disable-gpu"; fi
while :; do
  /usr/lib/nimbus/nimbus --ozone-platform=wayland --enable-features=UseOzonePlatform $extra >>"$HOME/hearth-installer.log" 2>&1
  sleep 1
done
SH
chmod 755 /usr/libexec/hearth-installer-session
cat >/usr/libexec/hearth-installer-cage <<'SH'
#!/usr/bin/bash
# cage draws with the graphics card when it has a driver for it, else in software (still works, just slower).
if grep -qw nomodeset /proc/cmdline || [ ! -e /dev/dri/renderD128 ]; then export WLR_RENDERER=pixman; fi
exec /usr/bin/cage -s -d -- /usr/libexec/hearth-installer-session
SH
chmod 755 /usr/libexec/hearth-installer-cage
cat >/etc/systemd/system/hearth-installer.service <<'UNIT'
[Unit]
Description=Hearth installer (the Hearth app, full screen)
After=systemd-user-sessions.service NetworkManager.service bluetooth.service
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
StandardOutput=journal+console
StandardError=journal+console
UtmpIdentifier=tty1
UtmpMode=user
ExecStart=/usr/libexec/hearth-installer-cage
Restart=always
RestartSec=1
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-installer.service
# A minute after start, what the installer screen is doing goes to the serial port (for the CI boot test;
# real PCs have nothing listening there).
cat >/etc/systemd/system/hearth-installer-report.service <<'UNIT'
[Unit]
Description=Hearth installer: report for the boot test
After=hearth-installer.service
[Service]
Type=oneshot
ExecStart=/usr/bin/bash -c 'sleep 60; { echo "=== hearth report"; ls -l /dev/dri; cat /proc/fb; lsmod | grep -E "^(drm|bochs|virtio_gpu|simpledrm|amdgpu|i915|nouveau)" ; journalctl -b -u hearth-installer --no-pager | tail -40; tail -40 /home/liveuser/hearth-installer.log; echo "=== end"; } >/dev/ttyS0 2>&1'
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-installer-report.service
systemctl disable getty@tty1.service || true
systemctl set-default graphical.target
# Nothing that updates or writes to disk in the background.
for s in bootc-fetch-apply-updates.timer rpm-ostreed-automatic.timer bootloader-update.service; do systemctl disable "$s" 2>/dev/null || true; done

# ---------------------------------------------------------------- titanoboa's ISO config
mkdir -p /usr/lib/bootc-image-builder
cp /src/iso.yaml /usr/lib/bootc-image-builder/iso.yaml
dnf clean all -q || true
