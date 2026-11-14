#!/usr/bin/env bash
# Boot test for the Hearth Recovery ISO (recovery-iso.yml; gates its upload). Starts the ISO in a UEFI virtual PC
# (QEMU + OVMF) the way a PC starts the stick: the firmware with Secure Boot on (Microsoft's keys, when the runner's
# OVMF has them), the stick's own shim and GRUB, its 3-second boot menu, its kernel and live boot, then the
# recovery screen. A small disk labelled HEARTH-CI makes the stick report on the serial console every 15 s
# (HEARTH-RECOVERY lines from os/recovery/src/build.sh's ci-report).
# Usage: tools/recovery-boot-test.sh ISO OUTDIR. Exit 0 only when the recovery screen came up.
# Writes OUTDIR/{serial.log,*.png,summary.txt} and GitHub annotations.
set -uo pipefail
ISO=$1; OUT=$2
LIMIT=${LIMIT:-900}
mkdir -p "$OUT"
work=$(mktemp -d)
note() { echo "$*"; echo "$*" >> "$OUT/summary.txt"; }
ann() { local t=$1; shift; echo "::$t title=Recovery boot test::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3800)"; }

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf python3-pil socat dosfstools mtools >/dev/null
sb=0
code=$(ls /usr/share/OVMF/OVMF_CODE_4M.secboot.fd 2>/dev/null | head -1)
vars=$(ls /usr/share/OVMF/OVMF_VARS_4M.ms.fd 2>/dev/null | head -1)
if [ -n "$code" ] && [ -n "$vars" ]; then
  sb=1
else
  code=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
  vars=$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)
fi
cp "$vars" "$work/vars.fd"
# The HEARTH-CI disk (a 16 MB FAT disk: nothing on it).
truncate -s 16M "$work/ci.img"; mkfs.vfat -n HEARTH-CI "$work/ci.img" >/dev/null
kvm=""; [ -e /dev/kvm ] && { sudo chmod 666 /dev/kvm; kvm="-enable-kvm -cpu host"; }
machine="-machine q35"
[ "$sb" = 1 ] && machine="-machine q35,smm=on -global driver=cfi.pflash01,property=secure,value=on"

shot() {  # shot NAME: a screenshot through the QEMU monitor
  echo "screendump $work/s.ppm" | socat - "unix-connect:$work/mon" >/dev/null 2>&1 || return 0
  sleep 1; python3 -c "from PIL import Image; Image.open('$work/s.ppm').save('$OUT/$1.png')" 2>/dev/null || true
}

note "boot: $ISO, Secure Boot $([ "$sb" = 1 ] && echo on || echo 'off (no Secure Boot firmware on the runner)')"
qemu-system-x86_64 -m 6144 -smp 4 $kvm $machine \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$work/vars.fd" \
  -drive file="$ISO",media=cdrom,readonly=on,if=none,id=cd -device ide-cd,drive=cd,bootindex=0 \
  -drive file="$work/ci.img",format=raw,if=virtio \
  -nic user,model=virtio-net-pci -vga std -display none \
  -serial file:"$OUT/serial.log" -monitor unix:"$work/mon",server,nowait &
q=$!
start=$(date +%s); n=0; ok=0
while kill -0 $q 2>/dev/null; do
  sleep 15; n=$((n + 1))
  case $n in 1|2|4|6|8|12|16) shot "screen-$((n * 15))s" ;; esac
  ser=$(tr -d '\r' < "$OUT/serial.log" 2>/dev/null)
  # Up: the screen's service runs and the screen answers with its state, twice in a row.
  # (Or the screen's own log saying it has drawn what the scan found, when its state can't be asked for.)
  if grep -aq '^ui-service: active' <<<"$ser" && { [ "$(grep -a '^ui-state: {' <<<"$ser" | wc -l)" -ge 2 ] || grep -aq 'HEARTH-RECOVERY-UI scanned' <<<"$ser"; }; then
    ok=1; sleep 20; shot screen-up; break
  fi
  grep -aq 'HEARTH-RECOVERY-END' <<<"$ser" && break
  [ $(( $(date +%s) - start )) -gt "$LIMIT" ] && { shot timeout; break; }
done
kill $q 2>/dev/null; sleep 2; kill -9 $q 2>/dev/null; wait $q 2>/dev/null
ser=$(tr -d '\r' < "$OUT/serial.log" 2>/dev/null)
sbline=$(grep -a '^secureboot: ' <<<"$ser" | tail -1)
state=$(grep -a '^ui-state: ' <<<"$ser" | tail -1 | cut -c1-1500)
scan=$(grep -a '^scan: ' <<<"$ser" | tail -1 | cut -c1-800)
failed=$(grep -a '^failed: ' <<<"$ser" | sort -u | head -12)
if [ "$ok" = 1 ]; then
  note "boot: the recovery screen came up in $(( $(date +%s) - start )) s"
  ann notice "The recovery screen came up in $(( $(date +%s) - start )) s ($sbline, 1 = on). $state ||| $scan"
  [ -n "$failed" ] && ann warning "units that failed on the stick: $failed"
  exit 0
fi
note "boot: FAILED"
ann error "The recovery screen didn't come up within $LIMIT s. $sbline ||| state: $state ||| labwc/ui: $(grep -a -e '^labwc: ' -e '^ui-log: ' <<<"$ser" | tail -8) ||| failed: $failed ||| serial (end): $(tail -25 <<<"$ser")"
exit 1
