#!/usr/bin/bash
# Turns Hearth OS on Anatase (Fedora, bootc) into Hearth Recovery: boots from a USB stick straight into the
# recovery screen (no login, no Hearth session), as root on tty1. GRUB on the stick starts it by itself after
# 3 seconds (iso.yaml), so a handheld with no keyboard gets there from its firmware's boot menu alone.
# The live boot part is the live ISO's (os/live/src/build.sh).
set -exo pipefail

mkdir -p "$(realpath /root)" /var/lib/rpm-state
dnf -qy versionlock clear || true

# ---------------------------------------------------------------- live boot (as os/live/src/build.sh)
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
test -n "$KVER"
dnf install -y dracut-live
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "dmsquash-live dmsquash-live-autooverlay plymouth" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"
dnf install -y livesys-scripts
systemctl enable livesys.service livesys-late.service
rm -f /usr/share/applications/liveinst.desktop /etc/xdg/autostart/liveinst-setup.desktop

# Titanoboa needs the cdboot EFI binary and the EFI tree under /boot/efi (Secure Boot: Fedora's signed shim and
# GRUB, the same as the live ISO).
dnf install -y grub2-efi-x64-cdboot grub2-efi-x64 shim-x64 xorriso isomd5sum
mkdir -p /boot/efi
for d in /usr/lib/efi/*/*/EFI /usr/lib/bootupd/updates/EFI; do
  if [ -d "$d" ]; then cp -av "$d" /boot/efi/; fi
done
vendor=""
for v in fedora anatase; do
  if [ -z "$vendor" ] && [ -e "/boot/efi/EFI/$v/shimx64.efi" ]; then vendor=$v; fi
done
test -n "$vendor"
mkdir -p /boot/efi/EFI/BOOT
if [ ! -e /boot/efi/EFI/BOOT/BOOTX64.EFI ]; then cp -v "/boot/efi/EFI/$vendor/shimx64.efi" /boot/efi/EFI/BOOT/BOOTX64.EFI; fi
cp -v "/boot/efi/EFI/$vendor/grubx64.efi" /boot/efi/EFI/BOOT/fbx64.efi || true

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
dnf install -y ntfsprogs ntfs-3g dosfstools exfatprogs e2fsprogs btrfs-progs efibootmgr openssl rsync NetworkManager-wifi python3-evdev || \
  dnf install -y --skip-unavailable ntfsprogs dosfstools e2fsprogs btrfs-progs efibootmgr rsync python3-evdev
if ! ostree container --help >/dev/null 2>&1; then dnf install -y rpm-ostree; fi
ostree container image deploy --help >/dev/null
ostree admin set-default --help >/dev/null
# Reinstalling keeps every version on the drive by pinning them while the new one is deployed.
ostree admin pin --help >/dev/null
if ! command -v bootupctl >/dev/null; then dnf install -y bootupd; fi
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
# Factory reset uses Hearth's own installer (the live USB's), offering every edition (it downloads what it installs):
# the whole drive when nothing else is on it, else only Hearth's own partitions (its "reuse" mode).
install -Dm755 /src/installer/hearth-install /usr/libexec/nimbus/hearth-install
install -Dm755 /src/installer/hearth-live-prep /usr/libexec/nimbus/hearth-live-prep
echo installer > /etc/hearth-live
echo recovery > /etc/hearth-recovery
source /etc/os-release
echo "Hearth Recovery ${VERSION_ID}" >/etc/system-release

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
After=systemd-user-sessions.service plymouth-quit-wait.service livesys.service livesys-late.service systemd-logind.service
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
# No login screen and no Hearth session on this stick.
for s in plasmalogin.service sddm.service gdm.service nimbus-autologin.service; do
  systemctl disable "$s" 2>/dev/null || true
  systemctl mask "$s" 2>/dev/null || true
done
systemctl mask getty@tty1.service || true
# The power button is "select" on the recovery screen (holding it 4 s still turns the handheld off: that's the
# firmware's); suspend from it would leave someone with a black screen.
mkdir -p /etc/systemd/logind.conf.d
printf '[Login]\nHandlePowerKey=ignore\nHandlePowerKeyLongPress=ignore\nHandleSuspendKey=ignore\nHandleLidSwitch=ignore\nHandleLidSwitchExternalPower=ignore\n' \
  > /etc/systemd/logind.conf.d/90-hearth-recovery.conf
# Nothing that writes to disk, updates or downloads in the background (Handheld Daemon stays on: it runs a
# handheld's built-in controller).
for s in bootc-fetch-apply-updates.timer rpm-ostreed-automatic.timer uupd.timer bootloader-update.service \
         rpm-ostree-countme.service podman-auto-update.timer hearth-steam.service hearth-clean-install.service \
         hearth-boot-first.service hearth-bls-grub.service hearth-bls-grub-refresh.path; do
  systemctl disable "$s" 2>/dev/null || true
done
systemctl --global disable podman-auto-update.timer 2>/dev/null || true

# ---------------------------------------------------------------- boot test (only with hearth.ci on the command line)
# recovery-iso.yml boots each ISO in a virtual PC with no keyboard input: the screen's state and what the scan
# found go to the serial console every 15 seconds, then HEARTH-RECOVERY-END.
cat >/usr/libexec/hearth-recovery/ci-report <<'SH'
#!/usr/bin/bash
export XDG_RUNTIME_DIR=/run/user/0
for i in $(seq 1 40); do
  sleep 15
  {
    echo "HEARTH-RECOVERY t=$((i * 15))s"
    echo "cmdline: $(cat /proc/cmdline)"
    systemctl is-active hearth-recovery-ui.service | sed 's/^/ui-service: /'
    echo "ui-state: $(timeout 10 quickshell ipc -p /usr/share/hearth-recovery/ui call recovery state 2>&1 | tail -n1 | cut -c1-3000)"
    tail -n 5 /run/hearth-recovery/ui.log 2>/dev/null | cut -c1-300 | sed 's/^/ui-log: /'
    tail -n 5 /run/hearth-recovery/labwc.log 2>/dev/null | cut -c1-300 | sed 's/^/labwc: /'
    if [ "$i" = 2 ] || [ -e /run/hearth-recovery/ci-rescan ]; then
      rm -f /run/hearth-recovery/ci-rescan
      echo "scan: $(timeout 120 /usr/libexec/hearth-recovery/hearth-recovery scan 2>&1 | tail -n1 | cut -c1-6000)"
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
Description=Hearth Recovery boot test: state on the serial console (only with hearth.ci)
ConditionKernelCommandLine=hearth.ci
After=hearth-recovery-ui.service
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
dnf clean all -q || true
