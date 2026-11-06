#!/usr/bin/env bash
# Hearth OS 1.1's boot look, proven in a virtual PC (base-peppermint.yml, after the image is built; never published):
# installs the just-built image onto a blank disk with Hearth's own installer engine (os/live/src/hearth-install, from
# a copy of the image the way the offline USB carries it), then starts it several times in QEMU + OVMF, with
# screenshots, each start having to reach the login prompt on the serial console:
#   1 themed     the Hearth GRUB theme (with a second entry, so "(previous version)" shows) and the default splash
#   2 steam      splash preset hearth-steam (written the way Settings does: /etc/hearth/boot-splash + hearth-bls-grub)
#   3 minimal    splash preset hearth-minimal
#   3b aurora, starlight, orbit   the splash presets added after 1.1 (hearth-aurora, hearth-starlight, hearth-orbit)
#   4 nopreset   plymouth.splash= naming a preset this version doesn't have: Plymouth's default instead, still starts
#   5 notheme    theme.txt missing on /boot: GRUB's plain text menu, still starts
#   6 badtheme   theme.txt garbage on /boot: GRUB says so, waits a moment, plain menu, still starts
#   7 secureboot Secure Boot on (OVMF with Microsoft's keys: Debian's signed shim, GRUB, kernel), themed (report only)
# Usage: boot-look-vm.sh IMAGE OUTDIR. Exit 1 if an install or a required start failed. GitHub annotations say why.
set -uo pipefail
IMAGE=$1; OUT=$2
mkdir -p "$OUT"
W=/mnt/boot-look; sudo mkdir -p "$W"; sudo chown "$(id -u)" "$W"
REPO=$(cd "$(dirname "$0")/../.." && pwd)
ann() { local t=$1; shift; echo "::$t title=Boot look (VM)::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3500)"; }
note() { echo "$*"; echo "$*" >> "$OUT/summary.txt"; }

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf socat python3-pil btrfs-progs >/dev/null
[ -e /dev/kvm ] && sudo chmod 666 /dev/kvm

# ---- 1. install the image (as the offline USB does: from a local copy, Hearth's own engine, inside Fedora's bootc
# image like the installer USB, which has ostree's container tools and bootupd)
note "export $IMAGE"
sudo skopeo copy -q "containers-storage:$IMAGE" "oci:$W/oci:hearth" || { ann error "couldn't export the image"; exit 1; }
echo ghcr.io/dragongamer9/hearth-peppermint:testing > "$W/oci.ref"
truncate -s 80G "$W/disk.img"   # sparse; hearth-install needs at least 64 GiB (MIN_DISK)
LOOP=$(sudo losetup -fP --show "$W/disk.img")
plan=$(printf '{"mode":"whole","disk":"%s","user":{"name":"tester","full":"Tester","password":"hearth"},"hostname":"hearth-look","timezone":"UTC","edition":"peppermint","firmware":false,"signin":"auto","kargs":["console=ttyS0,115200","console=tty0","plymouth.ignore-serial-consoles"]}' "$LOOP")
echo "$plan" | sudo podman run --rm -i --privileged --pid=host --net=host -v /dev:/dev -v /run/udev:/run/udev \
  -v "$REPO/os/live/src:/src:ro" -v "$W/oci:/usr/share/hearth/image:ro" -v "$W/oci.ref:/usr/share/hearth/image.ref:ro" \
  -e HEARTH_INSTALL_TEST=1 --security-opt label=disable quay.io/fedora/fedora-bootc:43 \
  sh -c 'dnf -y -q install dosfstools e2fsprogs btrfs-progs efibootmgr openssl >/dev/null 2>&1; exec /src/hearth-install run' 2>&1 | tee "$OUT/install.log" | grep -v '^stage: ' | tail -20
if ! grep -q '^result: ' "$OUT/install.log"; then
  ann error "install failed: $(grep -v '^stage: ' "$OUT/install.log" | tail -15)"; sudo losetup -d "$LOOP"; exit 1
fi
note "installed"
rm -rf "$W/oci"   # room

M=$W/m; sudo mkdir -p "$M/b" "$M/r"
mount_disk() { sudo partx -u "$LOOP" 2>/dev/null; sudo mount "${LOOP}p2" "$M/b" && sudo mount -o subvol=root "${LOOP}p3" "$M/r"; DEP=$(sudo sh -c "ls -d $M/r/ostree/deploy/default/deploy/*.0 | head -1"); }
umount_disk() { sync; sudo umount "$M/b" "$M/r" 2>/dev/null; }
# The menu written the way the running system writes it (the image's own hearth-bls-grub), with a splash choice.
rewrite_menu() {  # rewrite_menu SPLASH|""
  local sp=$W/boot-splash; rm -f "$sp"; [ -n "$1" ] && echo "$1" > "$sp"
  sudo env HEARTH_SYSROOT="$M/r" HEARTH_SPLASH_CONF="$sp" HEARTH_GRUB_THEME_OFF=/nonexistent sh "$DEP/usr/libexec/nimbus/hearth-bls-grub" --boot "$M/b"
}

mount_disk
{ echo "--- grub.cfg (bootupd)"; sudo cat "$M/b/grub2/grub.cfg"; echo "--- hearth-bls.cfg"; sudo cat "$M/b/grub2/hearth-bls.cfg";
  echo "--- hearth-theme.cfg"; sudo cat "$M/b/grub2/hearth-theme.cfg"; echo "--- themes/hearth"; sudo ls -la "$M/b/grub2/themes/hearth"; } > "$OUT/grub-files.txt" 2>&1
sudo test -f "$M/b/grub2/themes/hearth/theme.txt" || ann error "the installer didn't put the GRUB theme on /boot"
sudo test -f "$M/b/grub2/hearth-theme.cfg" || ann error "no hearth-theme.cfg on /boot"
sudo sh -c "ls $M/b/grub2/themes/hearth/*.pf2" >/dev/null 2>&1 || ann warning "no GRUB fonts in the theme (the menu stays plain text)"
ann notice "menu entries: $(sudo grep '^menuentry' "$M/b/grub2/hearth-bls.cfg" | cut -c1-80)"
# A second entry (a copy of the first, an older "version") so the menu shows how the previous version is named.
first=$(sudo sh -c "ls $M/b/loader/entries/*.conf | head -1")
sudo sh -c "sed -e 's/^version .*/version 0/' '$first' > '$M/b/loader/entries/ostree-0-zz-previous.conf'"
rewrite_menu ""
umount_disk
sudo losetup -d "$LOOP"

CODE=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
VARS0=$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)
SB_CODE=$(ls /usr/share/OVMF/OVMF_CODE_4M.secboot.fd 2>/dev/null | head -1)
SB_VARS=$(ls /usr/share/OVMF/OVMF_VARS_4M.ms.fd 2>/dev/null | head -1)
cp "$VARS0" "$W/vars.fd"
kvm=""; [ -e /dev/kvm ] && kvm="-enable-kvm -cpu host"

mon() { echo "$1" | socat - "unix-connect:$W/mon" >/dev/null 2>&1; }
shot() {  # shot NAME
  mon "screendump $W/s.ppm" || return 0
  sleep 0.6; python3 -c "from PIL import Image; Image.open('$W/s.ppm').save('$OUT/$1.png')" 2>/dev/null || true
}
# boot NAME REQUIRED [secure]: start, hold the GRUB menu (arrow keys stop its countdown), screenshot it, start the
# first entry, screenshots through the splash, then wait for the login prompt.
FAILED=0
boot() {
  local name=$1 required=$2 sb=${3:-}
  local code=$CODE vars=$W/vars.fd mach="-machine q35"
  if [ -n "$sb" ]; then
    [ -n "$SB_CODE" ] && [ -n "$SB_VARS" ] || { ann warning "$name: no Secure Boot firmware on this runner"; return 0; }
    cp "$SB_VARS" "$W/sbvars.fd"; code=$SB_CODE; vars=$W/sbvars.fd
    mach="-machine q35,smm=on -global driver=cfi.pflash01,property=secure,value=on"
  fi
  rm -f "$W/mon" "$OUT/$name-serial.log"
  # shellcheck disable=SC2086
  qemu-system-x86_64 -m 4096 -smp 2 $kvm $mach \
    -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$vars" \
    -drive file="$W/disk.img",format=raw,if=virtio -vga none -device VGA,edid=on,xres=1280,yres=800 -display none \
    -net none -serial file:"$OUT/$name-serial.log" -monitor unix:"$W/mon",server,nowait &
  local q=$!
  sleep 2
  # Up arrow every 0.3 s for 35 s: GRUB stops counting down at the first key (Up on the first entry changes nothing).
  local i
  for i in $(seq 1 115); do mon "sendkey up"; sleep 0.3; done
  shot "$name-grub-menu"
  mon "sendkey ret"
  for i in 1 2 3 4 5 6 7 8 9 10 11 12; do sleep 1.5; shot "$name-splash-$i"; done
  local start; start=$(date +%s); local ok=0
  while kill -0 $q 2>/dev/null; do
    sleep 5
    if grep -aq 'login:' "$OUT/$name-serial.log" 2>/dev/null; then ok=1; sleep 25; shot "$name-booted"; break; fi
    [ $(( $(date +%s) - start )) -gt 600 ] && { shot "$name-timeout"; break; }
  done
  kill $q 2>/dev/null; sleep 2; kill -9 $q 2>/dev/null; wait $q 2>/dev/null
  if [ $ok = 1 ]; then
    note "$name: started (login prompt) $(grep -ao 'plymouth.splash=[a-z-]*' "$OUT/$name-serial.log" | head -1)"
    ann notice "$name: started to the login prompt"
  else
    note "$name: DID NOT START"
    if [ "$required" = yes ]; then FAILED=1; ann error "$name: didn't reach the login prompt. Serial (end): $(tr -d '\r' < "$OUT/$name-serial.log" | grep -av '^\s*$' | tail -25)"
    else ann warning "$name: didn't reach the login prompt (report only). Serial (end): $(tr -d '\r' < "$OUT/$name-serial.log" | grep -av '^\s*$' | tail -15)"; fi
  fi
}
with_disk() { LOOP=$(sudo losetup -fP --show "$W/disk.img"); mount_disk; "$@"; umount_disk; sudo losetup -d "$LOOP"; }

boot themed yes
# The pretend previous version goes again (the automatic fallback would otherwise count starts against it).
steam_menu() { sudo rm -f "$M/b/loader/entries/ostree-0-zz-previous.conf"; rewrite_menu hearth-steam; }
with_disk steam_menu
boot steam yes
with_disk rewrite_menu hearth-minimal
boot minimal yes
for sp in aurora starlight orbit; do
  with_disk rewrite_menu "hearth-$sp"
  boot "$sp" yes
done
nopreset() { rewrite_menu ""; sudo sed -i 's|^\(  linux .*\)$|\1 plymouth.splash=hearth-not-here|' "$M/b/grub2/hearth-bls.cfg"; }
with_disk nopreset
boot nopreset yes
notheme() { rewrite_menu ""; sudo rm -f "$M/b/grub2/themes/hearth/theme.txt"; }
with_disk notheme
boot notheme yes
badtheme() { rewrite_menu ""; echo 'this is { not a theme' | sudo tee "$M/b/grub2/themes/hearth/theme.txt" >/dev/null; sudo dd if=/dev/urandom of="$M/b/grub2/themes/hearth/background.png" bs=1k count=4 status=none; }
with_disk badtheme
boot badtheme yes
# The running system put the theme back by itself (hearth-bls-grub at start compares the copy with the image's).
healed() { if sudo cmp -s "$M/b/grub2/themes/hearth/theme.txt" "$DEP/usr/share/hearth/grub-theme/theme.txt"; then ann notice "the broken theme on /boot was put back by the running system"; else ann warning "the broken theme on /boot wasn't put back at start"; fi; rewrite_menu ""; }
with_disk healed
boot secureboot no secure
note "done (failed=$FAILED)"
exit $FAILED
