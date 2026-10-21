#!/usr/bin/env bash
# End-to-end install test for a Hearth ISO (CI; gates every upload of an ISO to the download site):
#   1. boots the ISO's own kernel and initrd in a UEFI virtual PC (QEMU + OVMF) with a blank disk and
#      "hearth.ci hearth.autoinstall=<edition>" on the kernel command line, so the installer USB installs Hearth OS
#      onto the disk by itself (os/live/src/hearth-autoinstall, the same engine as the install screens) and
#      reports on the serial console, then powers off;
#   2. starts the virtual PC again from the disk alone (the same firmware settings, so it uses the boot entry the
#      installer wrote) and waits for the installed system to come up.
# Usage: tools/e2e-iso.sh ISO LABEL EDITION OUTDIR      (EDITION may be empty: the USB's own)
# Exit 0 only when both worked. Writes OUTDIR/{install-serial.log,boot-serial.log,*.png,summary.txt} and
# GitHub annotations with what went wrong.
set -uo pipefail
ISO=$1; LABEL=$2; ED=${3:-}; OUT=$4
INSTALL_LIMIT=${INSTALL_LIMIT:-9000}   # seconds (the small installer downloads ~5 GB)
BOOT_LIMIT=${BOOT_LIMIT:-900}
mkdir -p "$OUT"
work=$(mktemp -d)
note() { echo "$*"; echo "$*" >> "$OUT/summary.txt"; }
ann() { local t=$1; shift; echo "::$t title=E2E install test::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3800)"; }

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf xorriso python3-pil socat >/dev/null
code=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
cp "$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)" "$work/vars.fd"
xorriso -osirrox on -indev "$ISO" -extract /images/pxeboot/vmlinuz "$work/vmlinuz" -extract /images/pxeboot/initrd.img "$work/initrd.img" >/dev/null 2>&1
sudo chmod 644 "$work"/vmlinuz "$work"/initrd.img || true
[ -s "$work/vmlinuz" ] || { ann error "couldn't take the kernel out of $ISO"; exit 1; }
# The blank disk on the runner's big second drive when there is one.
d=$(df --output=avail -B1 /mnt 2>/dev/null | tail -1); r=$(df --output=avail -B1 / | tail -1)
if [ "${d:-0}" -gt "$r" ]; then disk=/mnt/e2e-disk.img; sudo rm -f "$disk"; sudo touch "$disk"; sudo chown "$(id -u)" "$disk"; else disk=$work/disk.img; fi
truncate -s 80G "$disk"
kvm=""; [ -e /dev/kvm ] && { sudo chmod 666 /dev/kvm; kvm="-enable-kvm -cpu host"; }

shot() {  # shot NAME: a screenshot through the QEMU monitor
  echo "screendump $work/s.ppm" | socat - "unix-connect:$work/mon" >/dev/null 2>&1 || return 0
  sleep 1; python3 -c "from PIL import Image; Image.open('$work/s.ppm').save('$OUT/$1.png')" 2>/dev/null || true
}

# ---- 1. install
note "install: $ISO (label $LABEL, edition ${ED:-the USB own edition}), disk $disk"
append="root=live:CDLABEL=$LABEL rd.live.image enforcing=0 console=tty0 console=ttyS0,115200 systemd.show_status=1 hearth.ci hearth.autoinstall${ED:+=$ED}"
qemu-system-x86_64 -m 8192 -smp 4 $kvm -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$work/vars.fd" \
  -cdrom "$ISO" -kernel "$work/vmlinuz" -initrd "$work/initrd.img" -append "$append" \
  -drive file="$disk",format=raw,if=virtio -nic user,model=virtio-net-pci \
  -vga std -display none -serial file:"$OUT/install-serial.log" -monitor unix:"$work/mon",server,nowait &
q=$!
start=$(date +%s); n=0
while kill -0 $q 2>/dev/null; do
  sleep 20; n=$((n + 1))
  [ $((n % 15)) -eq 0 ] && shot "install-$((n * 20))s"
  grep -q 'HEARTH-AUTOINSTALL-' "$OUT/install-serial.log" 2>/dev/null && { sleep 15; break; }
  [ $(( $(date +%s) - start )) -gt "$INSTALL_LIMIT" ] && { shot install-timeout; break; }
done
kill $q 2>/dev/null; sleep 2; kill -9 $q 2>/dev/null; wait $q 2>/dev/null
ser=$(tr -d '\r' < "$OUT/install-serial.log" 2>/dev/null)
ai=$(echo "$ser" | grep -a 'HEARTH-AI: ' | grep -av 'HEARTH-AI: stage: ' | tail -40)
if echo "$ser" | grep -aq 'HEARTH-AUTOINSTALL-OK'; then
  note "install: OK in $(( $(date +%s) - start )) s"
  ann notice "Installed (${ED:-the USB own edition}) in $(( $(date +%s) - start )) s. Last lines: $(echo "$ai" | tail -8)"
else
  why=$(echo "$ser" | grep -a 'HEARTH-AUTOINSTALL-FAIL' | tail -1)
  [ -n "$why" ] || why="no result on the serial console within $INSTALL_LIMIT s (did the automatic install start?)"
  note "install: FAILED: $why"
  ann error "Install FAILED: $why ||| installer output: $ai ||| serial (end): $(echo "$ser" | tail -25)"
  exit 1
fi

# ---- 2. start the installed system from the disk alone
note "boot: from the disk"
qemu-system-x86_64 -m 8192 -smp 4 $kvm -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$work/vars.fd" \
  -drive file="$disk",format=raw,if=virtio -nic user,model=virtio-net-pci \
  -vga std -display none -serial file:"$OUT/boot-serial.log" -monitor unix:"$work/mon",server,nowait &
q=$!
start=$(date +%s); ok=0; n=0
while kill -0 $q 2>/dev/null; do
  sleep 15; n=$((n + 1))
  [ $((n % 4)) -eq 0 ] && shot "boot-$((n * 15))s"
  if tr -d '\r' < "$OUT/boot-serial.log" 2>/dev/null | grep -aqE 'Reached target .*(Graphical Interface|Multi-User System)|hearth-e2e login:'; then
    sleep 45; shot booted; ok=1; break
  fi
  # The installed system may not talk on the serial console at all (its boot loader and kernel draw on the
  # screen): then a bright, detailed picture that stays put for a minute (a login screen, Hearth) counts.
  if [ $((n % 4)) -eq 0 ] && [ $(( $(date +%s) - start )) -gt 150 ]; then
    shot screen-now
    if [ -f "$OUT/screen-now.png" ] && [ -f "$work/screen-prev.png" ] && python3 - "$work/screen-prev.png" "$OUT/screen-now.png" <<'PY'
import sys
from PIL import Image, ImageChops, ImageStat
a, b = (Image.open(x).convert('L').resize((320, 200)) for x in sys.argv[1:3])
st = ImageStat.Stat(b)
diff = ImageStat.Stat(ImageChops.difference(a, b)).mean[0]
colors = len(set(b.getdata()))
# Firmware and boot loader screens are dark text on black; a desktop or login screen is bright and detailed.
sys.exit(0 if st.mean[0] > 35 and colors > 60 and diff < 3 else 1)
PY
    then cp "$OUT/screen-now.png" "$OUT/booted.png"; ok=2; break; fi
    [ -f "$OUT/screen-now.png" ] && cp "$OUT/screen-now.png" "$work/screen-prev.png"
  fi
  [ $(( $(date +%s) - start )) -gt "$BOOT_LIMIT" ] && { shot boot-timeout; break; }
done
kill $q 2>/dev/null; sleep 2; kill -9 $q 2>/dev/null; wait $q 2>/dev/null
bser=$(tr -d '\r' < "$OUT/boot-serial.log" 2>/dev/null)
if [ $ok = 1 ]; then
  note "boot: OK in $(( $(date +%s) - start )) s"
  ann notice "The installed system started from the disk: $(echo "$bser" | grep -aE 'Reached target|login:' | tail -3)"
  exit 0
fi
if [ $ok = 2 ]; then
  note "boot: OK in $(( $(date +%s) - start )) s (the screen settled on a login screen or Hearth; nothing on the serial console)"
  ann notice "The installed system started from the disk (its screen settled; see booted.png). Serial: $(echo "$bser" | grep -av '^\s*$' | tail -3)"
  exit 0
fi
note "boot: FAILED"
ann error "Installed, but it didn't start from the disk within $BOOT_LIMIT s. Serial (end): $(echo "$bser" | grep -av '^\s*$' | tail -30)"
exit 1
