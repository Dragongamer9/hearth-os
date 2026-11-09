#!/usr/bin/env bash
# End-to-end test of the Hearth USB installer AS A PERSON USES IT (1.1): boots the ISO in a UEFI virtual PC (QEMU +
# OVMF) with a blank disk, then goes through every installer screen with the keyboard (QEMU's sendkey: the same
# focus moves a controller makes), screenshotting each one, holds Enter to confirm erasing the drive, waits for the
# install, presses Restart, and starts the installed system from the disk alone (screenshots of its boot menu, splash
# and the desktop it ends up in). The installer tells which screen it shows on the serial console
# ("HEARTH-UI screen=<name>", only under hearth.ci), so every key is sent to the screen it's meant for.
# Usage: tools/e2e-ui.sh ISO LABEL OUTDIR. Exit 0 only when the install finished and the installed system started.
set -uo pipefail
ISO=$1; LABEL=$2; OUT=$3
INSTALL_LIMIT=${INSTALL_LIMIT:-5400}
BOOT_LIMIT=${BOOT_LIMIT:-900}
mkdir -p "$OUT"
work=$(mktemp -d)
note() { echo "$*"; echo "$*" >> "$OUT/summary.txt"; }
ann() { local t=$1; shift; echo "::$t title=Installer UI test::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3800)"; }
SER=$OUT/usb-serial.log

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf xorriso python3-pil socat >/dev/null
code=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
cp "$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)" "$work/vars.fd"
xorriso -osirrox on -indev "$ISO" -extract /images/pxeboot/vmlinuz "$work/vmlinuz" -extract /images/pxeboot/initrd.img "$work/initrd.img" >/dev/null 2>&1
sudo chmod 644 "$work"/vmlinuz "$work"/initrd.img || true
[ -s "$work/vmlinuz" ] || { ann error "couldn't take the kernel out of $ISO"; exit 1; }
d=$(df --output=avail -B1 /mnt 2>/dev/null | tail -1); r=$(df --output=avail -B1 / | tail -1)
if [ "${d:-0}" -gt "$r" ]; then disk=/mnt/e2e-ui-disk.img; sudo rm -f "$disk"; sudo touch "$disk"; sudo chown "$(id -u)" "$disk"; else disk=$work/disk.img; fi
truncate -s 80G "$disk"
kvm=""; [ -e /dev/kvm ] && { sudo chmod 666 /dev/kvm; kvm="-enable-kvm -cpu host"; }

mon() { echo "$1" | socat - "unix-connect:$work/mon" >/dev/null 2>&1; }
shot() { mon "screendump $work/s.ppm" || return 0; sleep 0.8; python3 -c "from PIL import Image; Image.open('$work/s.ppm').save('$OUT/$1.png')" 2>/dev/null || true; }
key() { mon "sendkey $1 ${2:-}"; sleep "${3:-0.35}"; }
type_text() {  # letters, digits and spaces
  local s=$1 i c
  for ((i = 0; i < ${#s}; i++)); do
    c=${s:i:1}
    case "$c" in
      [a-z0-9]) key "$c" "" 0.12 ;;
      [A-Z]) key "shift-$(echo "$c" | tr 'A-Z' 'a-z')" "" 0.12 ;;
      ' ') key spc "" 0.12 ;;
    esac
  done
}
seen=0
# wait_screen NAME [SECONDS]: the next "HEARTH-UI screen=NAME" after the ones already seen.
wait_screen() {
  local name=$1 limit=${2:-120} t=0
  while [ $t -lt "$limit" ]; do
    local n; n=$(grep -ac "HEARTH-UI screen=$name\b" "$SER" 2>/dev/null || true)
    if [ "${n:-0}" -gt "$(eval echo "\${seen_$(echo "$name" | tr '-' '_'):-0}")" ]; then
      eval "seen_$(echo "$name" | tr '-' '_')=$n"; sleep 1.2; return 0
    fi
    sleep 1; t=$((t + 1))
  done
  shot "stuck-waiting-for-$name"
  ann error "the installer never showed the $name screen within $limit s. Serial (end): $(tr -d '\r' < "$SER" | grep -av '^\s*$' | tail -20)"
  return 1
}
fail() { ann error "$*"; kill "$q" 2>/dev/null; exit 1; }

# ---- 1. the USB: boot it (its normal menu entry's options, plus hearth.ci) and go through the screens
append="root=live:CDLABEL=$LABEL rd.live.image enforcing=0 quiet rhgb console=ttyS0,115200 console=tty0 plymouth.ignore-serial-consoles hearth.ci"
note "boot the USB: $append"
qemu-system-x86_64 -m 8192 -smp 4 $kvm -machine q35 -no-reboot \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$work/vars.fd" \
  -cdrom "$ISO" -kernel "$work/vmlinuz" -initrd "$work/initrd.img" -append "$append" \
  -drive file="$disk",format=raw,if=virtio -nic user,model=virtio-net-pci \
  -vga none -device VGA,edid=on,xres=1280,yres=800 -display none -serial file:"$SER" -monitor unix:"$work/mon",server,nowait &
q=$!
start=$(date +%s)
for i in $(seq 1 12); do sleep 2.5; shot "usb-boot-$(printf %02d "$i")"; done   # the splash while the USB starts

wait_screen welcome 600 || fail "no installer"
shot 01-welcome
key right; key right; key ret 0.2; sleep 2; shot 01b-welcome-rotated-90   # Let's go > Language > Screen rotation
key ret; key ret; key ret; sleep 2                                    # round to normal again
key left; key left; shot 01c-welcome-focus-next; key ret
wait_screen wifi || fail "no Wi-Fi screen"
sleep 4; shot 02-wifi
key ret
wait_screen drive || fail "no drive screen"
sleep 6; shot 03-drive
key ret
wait_screen confirm || fail "no erase confirmation"
shot 04-confirm-erase
mon "sendkey ret 600"; sleep 1.5; shot 04b-released-too-early     # too short: nothing happens
mon "sendkey ret 2600"; sleep 1.1; shot 04c-holding; sleep 2   # held long enough (QEMU lets go after 2.6 s)
wait_screen you || fail "holding Enter didn't confirm"
shot 05-you
key f2; wait_screen osk 20 && shot 05b-on-screen-keyboard; key esc
type_text "Tester"; key ret; key ret; key ret
type_text "hearth"; key ret; type_text "hearth"; key ret
shot 05c-you-filled
key ret
wait_screen time || fail "the account screen didn't go on"
sleep 2; shot 06-time-zone
key ret
wait_screen review || fail "no review screen"
shot 07-review
key ret
wait_screen install || fail "Install didn't start"
istart=$(date +%s); n=0
while :; do
  sleep 20; n=$((n + 1))
  [ $((n % 3)) -eq 1 ] && shot "08-install-$(printf %04d $(( $(date +%s) - istart )))s"
  grep -aq 'HEARTH-UI screen=done' "$SER" && break
  grep -aq 'HEARTH-UI screen=failed' "$SER" && { sleep 2; shot 08-install-failed; fail "the install failed: $(grep -a 'osinstall' "$SER" | tail -5)"; }
  kill -0 $q 2>/dev/null || fail "the virtual PC stopped during the install"
  [ $(( $(date +%s) - istart )) -gt "$INSTALL_LIMIT" ] && { shot 08-install-timeout; fail "no result within $INSTALL_LIMIT s"; }
done
sleep 2; shot 09-done-remove-usb
note "install: done in $(( $(date +%s) - istart )) s"
ann notice "Installed through the screens in $(( $(date +%s) - istart )) s"
key ret   # Restart now (-no-reboot: QEMU ends as the PC restarts)
for i in $(seq 1 60); do kill -0 $q 2>/dev/null || break; sleep 2; done
kill $q 2>/dev/null; sleep 1; kill -9 $q 2>/dev/null; wait $q 2>/dev/null

# ---- 2. the installed system, from the disk alone (the USB taken out)
BSER=$OUT/installed-serial.log
note "boot the installed system"
qemu-system-x86_64 -m 8192 -smp 4 $kvm -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$work/vars.fd" \
  -drive file="$disk",format=raw,if=virtio -nic user,model=virtio-net-pci \
  -vga none -device VGA,edid=on,xres=1280,yres=800 -display none -serial file:"$BSER" -monitor unix:"$work/mon",server,nowait &
q=$!
for i in $(seq 1 15); do sleep 2; shot "10-installed-boot-$(printf %02d "$i")"; done
start=$(date +%s); ok=0
while kill -0 $q 2>/dev/null; do
  sleep 10
  if grep -aqE 'login:' "$BSER"; then ok=1; break; fi
  [ $(( $(date +%s) - start )) -gt "$BOOT_LIMIT" ] && break
done
if [ $ok = 1 ]; then
  sleep 60; shot 11-installed-session; sleep 45; shot 11b-installed-session-later
fi
kill $q 2>/dev/null; sleep 2; kill -9 $q 2>/dev/null; wait $q 2>/dev/null
if [ $ok = 1 ]; then
  note "installed system: started (login prompt on the serial console)"
  ann notice "The installed system started from the disk: $(tr -d '\r' < "$BSER" | grep -a 'login:' | tail -1)"
  exit 0
fi
note "installed system: DID NOT START"
ann error "Installed, but it didn't start from the disk within $BOOT_LIMIT s. Serial (end): $(tr -d '\r' < "$BSER" | grep -av '^\s*$' | tail -30)"
exit 1
