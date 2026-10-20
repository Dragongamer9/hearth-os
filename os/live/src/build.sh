#!/usr/bin/bash
# Turns Hearth OS on Anatase (ghcr.io/dragongamer9/hearth-os, Fedora 45, bootc) into a live system: boots from the
# USB stick straight into Hearth as "liveuser", with a welcome card (Install Hearth OS / Wi-Fi / Try it) after
# the controller pairing, and Hearth's own installer (hearth-install, with screens in the Hearth app and in
# Mantel), which installs the Hearth OS image carried on the stick and leaves the installed system updating from
# ghcr.io/dragongamer9/hearth-os:latest. Titanoboa makes the ISO.
# What's set up here follows os-bases/anatase/build.sh: the same session (nimbus-session), the same autologin
# (nimbus-autologin: Plasma Login Manager's /etc/plasmalogin.conf on Anatase, SDDM's files where SDDM is), and
# Handheld Daemon for the built-in controller (Anatase has no InputPlumber).
set -exo pipefail
EDITION=${EDITION:-anatase}
case "$EDITION" in anatase) ;; *) echo "unknown edition: $EDITION"; exit 1;; esac

mkdir -p "$(realpath /root)" /var/lib/rpm-state
dnf -qy versionlock clear || true   # nothing held back from the packages added below

# ---------------------------------------------------------------- live boot
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
test -n "$KVER"
dnf install -y dracut-live
DRACUT_NO_XATTR=1 dracut -v --force --zstd --reproducible --no-hostonly \
  --add "dmsquash-live dmsquash-live-autooverlay plymouth" \
  "/usr/lib/modules/${KVER}/initramfs.img" "${KVER}"

# livesys creates "liveuser" (no password, can use sudo) at boot. Its desktop session setup (KDE's, when Anatase
# has Plasma) only matters for its autologin, which Hearth's replaces right before the login screen starts (below).
dnf install -y livesys-scripts
if [ -e /usr/share/wayland-sessions/plasma.desktop ] || [ -e /usr/share/xsessions/plasma.desktop ]; then
  if grep -q '^livesys_session=' /etc/sysconfig/livesys; then
    sed -i "s/^livesys_session=.*/livesys_session=kde/" /etc/sysconfig/livesys
  else
    echo 'livesys_session=kde' >> /etc/sysconfig/livesys
  fi
fi
systemctl enable livesys.service livesys-late.service

# Titanoboa needs the cdboot EFI binary and the EFI tree under /boot/efi. On Fedora 45's bootc images the EFI
# files are under /usr/lib/efi/<package>/<version>/EFI (or bootupd's copy); take whichever is there.
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
rm -f /etc/localtime; systemd-firstboot --timezone UTC || true

# ---------------------------------------------------------------- the live session
echo "$EDITION" > /etc/hearth-live   # Hearth's session and desktop look for this; the installer reads the edition
# Boot into Hearth as the live user. nimbus-autologin (enabled in the Hearth OS image) writes the login screen's
# autologin; on the live USB it signs in "liveuser". livesys makes liveuser and writes its own autologin (into
# the KDE desktop), so Hearth's runs again right before the login manager starts, after livesys, and takes
# livesys' out first. (Ordering it after livesys-late as well could loop with the login manager, and systemd
# would then drop Hearth's step: that's how 0.0.22's USB ended up in the wrong desktop.) Anatase's login screen is
# Plasma Login Manager (plasmalogin.service); sddm.service is covered too, in case.
mkdir -p /usr/libexec/nimbus
cat >/usr/libexec/nimbus/hearth-live-autologin <<'SH'
#!/usr/bin/bash
# The live USB: Hearth's autologin (liveuser into the Hearth session) wins over the one livesys wrote.
set -uo pipefail
for d in /etc/sddm.conf.d /etc/plasmalogin.conf.d; do
  [ -d "$d" ] || continue
  for f in "$d"/*.conf; do
    [ -f "$f" ] || continue
    [ "$(basename "$f")" != zzz-nimbus-autologin.conf ] || continue
    if grep -q '^\[Autologin\]' "$f"; then
      awk 'BEGIN{skip=0} /^\[/{skip=($0=="[Autologin]")} !skip' "$f" > "$f.hearth-tmp" && mv "$f.hearth-tmp" "$f"
    fi
  done
done
exec /usr/libexec/nimbus/nimbus-autologin
SH
chmod 755 /usr/libexec/nimbus/hearth-live-autologin
mkdir -p /etc/systemd/system/nimbus-autologin.service.d
printf '[Unit]\nAfter=livesys.service\n' > /etc/systemd/system/nimbus-autologin.service.d/live.conf
for dm in plasmalogin sddm; do
  mkdir -p "/etc/systemd/system/$dm.service.d"
  printf '[Service]\nExecStartPre=-/usr/libexec/nimbus/hearth-live-autologin\n' > "/etc/systemd/system/$dm.service.d/hearth-live.conf"
done
systemctl enable nimbus-autologin.service
# Hearth (Game Mode) first: it asks for a controller (Skip with a keyboard or mouse), then Install / Wi-Fi /
# Try it. "Install with Mantel" switches to the desktop for a mouse and keyboard.
mkdir -p /etc/skel/.config/nimbus
echo 'NIMBUS_START_MODE=game' > /etc/skel/.config/nimbus/session.conf
# The live user may do anything without a password prompt (it's a live USB).
mkdir -p /etc/polkit-1/rules.d
cat >/etc/polkit-1/rules.d/10-hearth-live.rules <<'RULE'
polkit.addRule(function(action, subject) { if (subject.user == "liveuser") return polkit.Result.YES; });
RULE
# Nothing that writes to disk, updates or downloads in the background (Handheld Daemon stays on: it runs a
# handheld's built-in controller).
for s in bootc-fetch-apply-updates.timer rpm-ostreed-automatic.timer uupd.timer bootloader-update.service \
         rpm-ostree-countme.service podman-auto-update.timer hearth-steam.service hearth-clean-install.service \
         hearth-boot-first.service; do
  systemctl disable "$s" 2>/dev/null || true
done
systemctl --global disable podman-auto-update.timer 2>/dev/null || true

# Boot test (live-iso.yml boots each new ISO in a virtual PC with "hearth.ci" on the kernel command line): the
# live session's Hearth logs go to the serial console every 30 seconds for 5 minutes, then HEARTH-CI-END.
# Does nothing on a normal boot.
cat >/usr/libexec/nimbus/hearth-ci-report <<'SH'
#!/usr/bin/bash
d=/home/liveuser/.local/share/nimbus
for i in $(seq 1 10); do
  sleep 30
  {
    echo "HEARTH-CI t=$((i * 30))s"
    tail -n 40 "$d/session.log" 2>/dev/null | sed 's/^/session: /'
    grep -E 'graphics:|watchdog|panic' "$d/bridge.log" 2>/dev/null | tail -n 6 | sed 's/^/bridge: /'
    echo "pages: $(grep -c '\[rust\]' "$d/bridge.log" 2>/dev/null) bridge lines"
    systemctl --failed --no-legend --plain 2>/dev/null | sed 's/^/failed: /'
  } > /dev/ttyS0 2>&1
done
echo "HEARTH-CI-END" > /dev/ttyS0
SH
chmod 755 /usr/libexec/nimbus/hearth-ci-report
cat >/etc/systemd/system/hearth-ci-report.service <<'UNIT'
[Unit]
Description=Hearth boot test: the live session's logs on the serial console (only with hearth.ci)
ConditionKernelCommandLine=hearth.ci
After=graphical.target
[Service]
Type=simple
ExecStart=/usr/libexec/nimbus/hearth-ci-report
[Install]
WantedBy=graphical.target
UNIT
systemctl enable hearth-ci-report.service

# ---------------------------------------------------------------- the installer (Hearth's own)
# The engine (hearth-install) partitions, copies Hearth OS onto the drive with ostree (from the copy on this
# stick) and sets up the boot loader with bootupd; its screens are in the Hearth app (controller) and Mantel's
# Settings (mouse/keyboard). "Install next to Windows" shrinks Windows' NTFS drive first (hearth-live-prep).
dnf install -y ntfsprogs dosfstools e2fsprogs btrfs-progs efibootmgr
# `ostree container image deploy` (rpm-ostree provides it on Fedora) and bootupd: an image without them gets them.
if ! ostree container --help >/dev/null 2>&1; then dnf install -y rpm-ostree; fi
ostree container image deploy --help >/dev/null
if ! command -v bootupctl >/dev/null; then dnf install -y bootupd; fi
install -Dm755 /src/hearth-live-prep /usr/libexec/nimbus/hearth-live-prep
install -Dm755 /src/hearth-install /usr/libexec/nimbus/hearth-install
source /etc/os-release
echo "Hearth release ${VERSION_ID}" >/etc/system-release
# "Install Hearth OS" in the app launcher: the welcome card's "how do you want to install" choice.
mkdir -p /usr/share/applications
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
