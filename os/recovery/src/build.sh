#!/usr/bin/bash
# Turns Hearth OS on Debian (bootc) into Hearth Recovery (1.12): boots from a USB stick straight into the recovery
# screen (no login, no Hearth session), as root on tty1. GRUB on the stick starts it by itself after 3 seconds
# (iso.yaml), so a handheld with no keyboard gets there from its firmware's boot menu alone. The same kernel,
# drivers, Handheld Daemon, ostree, bootc and bootupd as the Debian devices it repairs.
# (The Anatase-based recipe this replaces: archive/os/recovery-anatase/build.sh.)
set -exo pipefail
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get install -y --no-install-recommends -o Dpkg::Options::=--force-confold)
mkdir -p "$(realpath /root)"
# The Debian image's /var is made at each start from its tmpfiles list (dpkg's record lives in /usr/lib/sysimage/dpkg,
# apt's folders are made then): make them now so apt works here.
systemd-tmpfiles --create /usr/lib/tmpfiles.d/hearth-var.conf || true
test -e /var/lib/dpkg/status
mkdir -p /var/lib/apt/lists/partial /var/cache/apt/archives/partial /var/log/apt
apt-get update -q

# ---------------------------------------------------------------- live boot (dracut's dmsquash-live)
# Titanoboa takes the one kernel in /usr/lib/modules and the initramfs next to it.
mapfile -t KDIRS < <(find /usr/lib/modules -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -V)
[ "${#KDIRS[@]}" -eq 1 ] || { echo "expected one kernel, found: ${KDIRS[*]}"; exit 1; }
KVER=${KDIRS[0]}
test -f "/usr/lib/modules/$KVER/vmlinuz"
"${APT[@]}" dracut-live squashfs-tools
# The live boot's initramfs: dmsquash-live finds the stick by its label (root=live:CDLABEL=HearthRecov) and mounts
# its squashfs with the changes kept in memory (overlayfs). Hearth's splash is in it, as on the devices.
mods="dmsquash-live plymouth"
for m in dmsquash-live-autooverlay overlayfs; do
  if ls -d /usr/lib/dracut/modules.d/[0-9][0-9]"$m" >/dev/null 2>&1; then mods="$mods $m"; fi
done
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "$mods" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"
chmod 0644 "/usr/lib/modules/${KVER}/initramfs.img"
lsinitrd "/usr/lib/modules/${KVER}/initramfs.img" > /tmp/initrd-files.txt || true
grep -q 'dmsquash-live-root' /tmp/initrd-files.txt || { echo "the initramfs has no live boot (dmsquash-live)"; exit 1; }

# Titanoboa wants the EFI boot files under /boot/efi: Debian's signed shim and GRUB (Microsoft → Debian → Debian's
# kernel, so Secure Boot works), as the image keeps them for bootupd (/usr/lib/efi/{shim,grub2}/<version>/EFI).
# GRUB goes next to shim in EFI/BOOT too (shim starts grubx64.efi from its own folder); Debian's signed GRUB reads
# grub.cfg from EFI/debian, which Titanoboa writes. No fallback loader (fbx64.efi): on a USB stick it would add a
# boot entry for the stick to the PC's firmware and restart.
rm -rf /boot/efi; mkdir -p /boot/efi
for d in /usr/lib/efi/*/*/EFI; do
  if [ -d "$d" ]; then cp -av "$d" /boot/efi/; fi
done
test -f /boot/efi/EFI/debian/shimx64.efi
test -f /boot/efi/EFI/debian/grubx64.efi
mkdir -p /boot/efi/EFI/BOOT
cp -v /boot/efi/EFI/debian/shimx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
cp -v /boot/efi/EFI/debian/grubx64.efi /boot/efi/EFI/BOOT/grubx64.efi
[ ! -f /boot/efi/EFI/debian/mmx64.efi ] || cp -v /boot/efi/EFI/debian/mmx64.efi /boot/efi/EFI/BOOT/mmx64.efi
rm -f /boot/efi/EFI/BOOT/fbx64.efi /boot/efi/EFI/debian/BOOTX64.CSV
find /boot/efi -type f | sort
"${APT[@]}" xorriso || true

rm -rf /var/tmp; mkdir -p /var/tmp
cat >/etc/systemd/system/var-tmp.mount <<'UNIT'
[Unit]
Description=Larger tmpfs for /var/tmp on the recovery USB
[Mount]
What=tmpfs
Where=/var/tmp
Type=tmpfs
Options=size=50%%,nr_inodes=1m
[Install]
WantedBy=local-fs.target
UNIT
systemctl enable var-tmp.mount
rm -f /etc/localtime; systemd-firstboot --timezone UTC || true

# ---------------------------------------------------------------- tools the repairs use
# (Most are in the image already; whatever's missing is added, each on its own so one missing name can't stop the rest.)
for p in ntfs-3g dosfstools exfatprogs e2fsprogs btrfs-progs efibootmgr openssl rsync network-manager python3-evdev \
         smartmontools fdisk gdisk util-linux parted; do
  dpkg -s "$p" >/dev/null 2>&1 || "${APT[@]}" "$p" || echo "warning: $p isn't available"
done
# Reinstalling deploys the edition's image with ostree's container commands (bootc's ostree hooks) and pins every
# version first; switching versions and the boot menu use ostree admin.
ostree container image deploy --help >/dev/null
ostree admin set-default --help >/dev/null
ostree admin pin --help >/dev/null
command -v bootupctl && command -v bootc
command -v labwc && command -v quickshell && python3 -c 'import evdev'

# ---------------------------------------------------------------- the recovery screen
install -Dm755 /src/hearth-recovery /usr/libexec/hearth-recovery/hearth-recovery
ln -sf /usr/libexec/hearth-recovery/hearth-recovery /usr/bin/hearth-recovery
install -Dm755 /src/recovery-pad /usr/libexec/hearth-recovery/recovery-pad
install -Dm755 /src/hearth-recovery-session /usr/libexec/hearth-recovery/session
install -Dm644 /src/ui/shell.qml /usr/share/hearth-recovery/ui/shell.qml
install -Dm644 /src/labwc/rc.xml /usr/share/hearth-recovery/labwc/rc.xml
install -Dm644 /src/labwc/menu.xml /usr/share/hearth-recovery/labwc/menu.xml
install -Dm755 /src/labwc/autostart /usr/share/hearth-recovery/labwc/autostart
install -Dm644 /src/branding/hearth-512.png /usr/share/pixmaps/hearth.png
# Factory reset uses Hearth's own installer (the installer USB's): the whole drive when nothing else is on it, else
# only Hearth's own partitions (its "reuse" mode). It downloads what it installs.
install -Dm755 /src/installer/hearth-install /usr/libexec/nimbus/hearth-install
install -Dm755 /src/installer/hearth-live-prep /usr/libexec/nimbus/hearth-live-prep
echo installer > /etc/hearth-live
echo recovery > /etc/hearth-recovery
. /usr/lib/os-release
echo "Hearth Recovery (Debian ${VERSION_ID:-13})" >/etc/system-release

# Root's own session on tty1: a logind session (so labwc gets the screen and input through logind), no password.
cat >/etc/pam.d/hearth-recovery <<'PAM'
auth     required  pam_permit.so
account  required  pam_permit.so
session  optional  pam_keyinit.so force revoke
session  required  pam_limits.so
session  optional  pam_systemd.so
PAM
cat >/etc/systemd/system/hearth-recovery-ui.service <<'UNIT'
[Unit]
Description=Hearth Recovery screen
After=systemd-user-sessions.service plymouth-quit-wait.service systemd-logind.service hhd.service
Wants=systemd-logind.service
Conflicts=getty@tty1.service
Before=graphical.target
[Service]
Type=simple
User=root
PAMName=hearth-recovery
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
StandardInput=tty-fail
StandardOutput=journal
StandardError=journal
UtmpIdentifier=tty1
UtmpMode=user
ExecStartPre=-/usr/bin/plymouth quit
ExecStart=/usr/libexec/hearth-recovery/session
Restart=always
RestartSec=2
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-recovery-ui.service
systemctl set-default graphical.target
# No login screen and no Hearth session on this stick (Hearth's own start, its greeter and its on-device Recovery menu).
for s in nimbus-autologin.service hearth-greeter-pad.service hearth-recovery.service sddm.service gdm.service \
         plasmalogin.service display-manager.service; do
  systemctl disable "$s" 2>/dev/null || true
  systemctl mask "$s" 2>/dev/null || true
done
systemctl mask getty@tty1.service || true
# The power button is "select" on the recovery screen (holding it 4 s still turns the handheld off: that's the
# firmware's); suspend from it would leave someone with a black screen.
mkdir -p /etc/systemd/logind.conf.d
printf '[Login]\nHandlePowerKey=ignore\nHandlePowerKeyLongPress=ignore\nHandleSuspendKey=ignore\nHandleLidSwitch=ignore\nHandleLidSwitchExternalPower=ignore\n' \
  > /etc/systemd/logind.conf.d/90-hearth-recovery.conf
# Nothing that writes to the PC's drives, changes its settings, updates or downloads in the background (Handheld
# Daemon stays on: it runs a handheld's built-in controller; memory and zram stay: the stick runs from memory).
for s in bootc-fetch-apply-updates.timer uupd.timer bootloader-update.service podman-auto-update.timer \
         hearth-boot-first.service hearth-clean-install.service hearth-bls-grub.service hearth-bls-grub-refresh.path \
         hearth-boot-guard.service hearth-boot-guard-refresh.path hearth-boot-good.service \
         hearth-unlock.service hearth-unlock-units.service hearth-unlock-good.service hearth-unlock-reapply.service hearth-unlock-ldcache.service \
         hearth-flatpak-setup.service hearth-home-repair.service hearth-groups.service hearth-hosts.service hearth-hosts.path \
         hearth-hwset.service hearth-diskguard.service hearth-gamecores.service hearth-app-rules.service \
         hearth-scrub.timer hearth-snapshots.timer hearth-update-mirror.socket hearth-steam.service \
         smartd.service smartmontools.service tailscaled.service sunshine.service cups.socket; do
  systemctl disable "$s" 2>/dev/null || true
done
systemctl --global disable podman-auto-update.timer 2>/dev/null || true

# ---------------------------------------------------------------- boot test
# recovery-iso.yml boots the ISO in a virtual PC with no keyboard input, through the stick's own GRUB (Secure Boot
# on). It runs only there: with hearth.ci on the command line, or a disk labelled HEARTH-CI (which the test adds, so
# the stick's own boot menu is what's tested). The screen's state and what the scan found go to the serial console
# every 15 seconds, then HEARTH-RECOVERY-END.
cat >/usr/libexec/hearth-recovery/ci-report <<'SH'
#!/usr/bin/bash
grep -qw hearth.ci /proc/cmdline || { udevadm settle -t 30 || true; [ -e /dev/disk/by-label/HEARTH-CI ]; } || exit 0
export XDG_RUNTIME_DIR=/run/user/0
for i in $(seq 1 40); do
  sleep 15
  {
    echo "HEARTH-RECOVERY t=$((i * 15))s"
    echo "cmdline: $(cat /proc/cmdline)"
    echo "secureboot: $(od -An -t u1 /sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d0-aa4b-00e098032b8c 2>/dev/null | awk '{print $NF}')"
    systemctl is-active hearth-recovery-ui.service | sed 's/^/ui-service: /'
    echo "ui-state: $(timeout 10 quickshell ipc -p /usr/share/hearth-recovery/ui call recovery state 2>&1 | tail -n1 | cut -c1-3000)"
    tail -n 5 /run/hearth-recovery/ui.log 2>/dev/null | cut -c1-300 | sed 's/^/ui-log: /'
    tail -n 5 /run/hearth-recovery/labwc.log 2>/dev/null | cut -c1-300 | sed 's/^/labwc: /'
    if [ "$i" = 2 ] || [ -e /run/hearth-recovery/ci-rescan ]; then
      rm -f /run/hearth-recovery/ci-rescan
      echo "scan: $(timeout 120 /usr/libexec/hearth-recovery/hearth-recovery scan 2>&1 | tail -n1 | cut -c1-6000)"
      echo "health: $(timeout 120 /usr/libexec/hearth-recovery/hearth-recovery health 2>&1 | tail -n1 | cut -c1-3000)"
    fi
    systemctl --failed --no-legend --plain 2>/dev/null | sed 's/^/failed: /'
  } > /dev/ttyS0 2>&1
  [ -e /run/hearth-recovery/ci-done ] && break
done
echo "HEARTH-RECOVERY-END" > /dev/ttyS0
SH
chmod 755 /usr/libexec/hearth-recovery/ci-report
cat >/etc/systemd/system/hearth-recovery-ci.service <<'UNIT'
[Unit]
Description=Hearth Recovery boot test: state on the serial console (only in recovery-iso.yml's virtual PC)
After=hearth-recovery-ui.service systemd-udev-settle.service
[Service]
Type=simple
ExecStart=/usr/libexec/hearth-recovery/ci-report
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-recovery-ci.service

# ---------------------------------------------------------------- titanoboa's ISO config
mkdir -p /usr/lib/bootc-image-builder
cp /src/iso.yaml /usr/lib/bootc-image-builder/iso.yaml
apt-get clean
rm -rf /var/lib/apt/lists/*
