#!/usr/bin/env bash
# Switch edition, both ways, in a virtual PC (red flag 2: a switch must never leave a PC unable to start; the owner's
# laptop, Nov 17, dropped to a bare "grub>" prompt on the restart after moving from Debian to the old Anatase).
# Run by base-anatase.yml after the Anatase image passed its checks. QEMU + OVMF (UEFI, Secure Boot off, with its own
# firmware variables kept between starts like a real PC), no key ever pressed: every start has to come up through the
# firmware's own boot entry and the boot menu by itself, and reach the login prompt on the serial console.
#   1 Debian (DEBIAN_IMAGE, the published one devices have) installed with Hearth's installer engine; first start
#     (hearth-boot-first makes the firmware's "Hearth OS" entry, as on a real PC).
#   2 Switch edition to Anatase with this branch's hearth-admin (move_to_new_os('anatase'): pin, forget kept images,
#     keepHome plan, bootc switch), from a local copy of the image just built (the VM has no network; only the
#     transport differs from a device's). The VM powers off (ostree finalizes the switch at shutdown, as on a restart).
#   3 Started again by the firmware: it must start Hearth OS on Anatase by itself (image-info base "anatase"), its first
#     start's clean install done, the home folder untouched, Debian still in the boot menu (pinned).
#   4 Switch back with hearth-admin move_to_new_os('peppermint'): the very Debian version again (rollback); powers off.
#   5 Started again: it must start Hearth OS on Debian by itself, the same deployment as before.
# The EFI partition's files and /boot's menu are listed after every step (annotations). Exit 1 when a start fails.
#   edition-switch-vm.sh ANATASE_IMAGE OUTDIR    (DEBIAN_IMAGE defaults to ghcr.io/dragongamer9/hearth-peppermint:testing)
set -uo pipefail
ANA=$1; OUT=$2; DEB=${DEBIAN_IMAGE:-ghcr.io/dragongamer9/hearth-peppermint:testing}
mkdir -p "$OUT"
REPO=$(cd "$(dirname "$0")/../.." && pwd)
W=/mnt/edition-switch; sudo mkdir -p "$W"; sudo chown "$(id -u)" "$W"
ann() { local t=$1; shift; echo "::$t title=Switch edition (VM)::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3500)"; }
FAILED=0
fail() { FAILED=1; ann error "$*"; }

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf >/dev/null
[ -e /dev/kvm ] && sudo chmod 666 /dev/kvm

# ---- 1. Debian, installed as the offline USB does
sudo podman pull -q "$DEB" >/dev/null || { ann error "couldn't pull $DEB"; exit 1; }
echo "::notice title=Switch edition (VM)::Debian: $DEB $(sudo podman run --rm "$DEB" cat /usr/share/nimbus/version 2>/dev/null)"
sudo skopeo copy -q "containers-storage:$DEB" "oci:$W/oci:hearth" || { ann error "couldn't export $DEB"; exit 1; }
echo "$DEB" > "$W/oci.ref"
truncate -s 96G "$W/disk.img"
LOOP=$(sudo losetup -fP --show "$W/disk.img")
plan=$(printf '{"mode":"whole","disk":"%s","user":{"name":"tester","full":"Tester","password":"hearth"},"hostname":"hearth-switch","timezone":"UTC","edition":"peppermint","firmware":false,"signin":"auto","kargs":["console=ttyS0,115200","console=tty0","plymouth.ignore-serial-consoles"]}' "$LOOP")
echo "$plan" | sudo podman run --rm -i --privileged --pid=host --net=host -v /dev:/dev -v /run/udev:/run/udev \
  -v "$REPO/os/live/src:/src:ro" -v "$W/oci:/usr/share/hearth/image:ro" -v "$W/oci.ref:/usr/share/hearth/image.ref:ro" \
  -e HEARTH_INSTALL_TEST=1 --security-opt label=disable quay.io/fedora/fedora-bootc:43 \
  sh -c 'dnf -y -q install dosfstools e2fsprogs btrfs-progs efibootmgr openssl >/dev/null 2>&1; exec /src/hearth-install run' 2>&1 | tee "$OUT/install.log" | grep -v '^stage: ' | tail -8
grep -q '^result: ' "$OUT/install.log" || { ann error "the Debian install failed: $(grep -v '^stage: ' "$OUT/install.log" | tail -12)"; sudo losetup -d "$LOOP"; exit 1; }
rm -rf "$W/oci"
sudo losetup -d "$LOOP"

M=$W/m; sudo mkdir -p "$M/b" "$M/r" "$M/e"
mount_disk() { LOOP=$(sudo losetup -fP --show "$W/disk.img"); sudo partx -u "$LOOP" 2>/dev/null
  sudo mount "${LOOP}p1" "$M/e" && sudo mount "${LOOP}p2" "$M/b" && sudo mount -o subvol=root "${LOOP}p3" "$M/r"; }
umount_disk() { sync; sudo umount "$M/e" "$M/b" "$M/r" 2>/dev/null; sudo losetup -d "$LOOP"; }
# The deployment the boot menu starts first (ostree's highest-version entry), and its edition.
default_dep() { local e o; e=$(sudo sh -c "grep -l . $M/b/loader/entries/*.conf" | while read -r f; do echo "$(sudo sed -n 's/^version //p' "$f") $f"; done | sort -n | tail -1 | cut -d' ' -f2)
  o=$(sudo sed -n 's/^options .*ostree=\([^ ]*\).*/\1/p' "$e" | head -1); sudo readlink -f "$M/r$o" 2>/dev/null || echo "$M/r$o"; }
look() {  # look STEP: what the EFI partition and the boot menu are now
  { echo "--- EFI partition"; sudo find "$M/e" -type f -printf '%P %s\n' | sort
    for f in $(sudo find "$M/e/EFI" -name grub.cfg); do echo "--- $f"; sudo cat "$f"; done
    echo "--- /boot"; sudo ls -la "$M/b" "$M/b/grub2" "$M/b/loader/entries"; echo "--- hearth-bls.cfg"; sudo cat "$M/b/grub2/hearth-bls.cfg" 2>/dev/null
    echo "--- bootupd-state.json"; sudo head -c 3000 "$M/b/bootupd-state.json" 2>/dev/null; echo
    echo "--- deployments"; sudo ls "$M/r/ostree/deploy/default/deploy"; } > "$OUT/$1-disk.txt" 2>&1
  ann notice "$1: EFI files: $(sudo find "$M/e/EFI" -type f -printf '%P ' | cut -c1-700) · entries: $(sudo grep -h '^title' "$M/b/loader/entries/"*.conf | tr '\n' ';' | cut -c1-400)"
}
# A one-time job for the next start, in the default deployment's /etc, that runs a script kept in /var (so it runs
# even if a first start's clean install sets /etc's extra files aside) and then powers the VM off.
job() {  # job NAME SCRIPT-TEXT
  local dep; dep=$(default_dep)
  sudo mkdir -p "$M/r/ostree/deploy/default/var/lib/hearth-ci" "$dep/etc/systemd/system/multi-user.target.wants"
  printf '%s\n' "#!/bin/sh" "exec > /dev/ttyS0 2>&1" "$2" "echo HEARTH-CI-$1-END; systemctl poweroff" | sudo tee "$M/r/ostree/deploy/default/var/lib/hearth-ci/$1.sh" >/dev/null
  sudo chmod 755 "$M/r/ostree/deploy/default/var/lib/hearth-ci/$1.sh"
  sudo rm -f "$dep"/etc/systemd/system/hearth-ci-*.service "$dep"/etc/systemd/system/multi-user.target.wants/hearth-ci-*.service
  sudo tee "$dep/etc/systemd/system/hearth-ci-$1.service" >/dev/null <<UNIT
[Unit]
Description=CI: $1
After=multi-user.target hearth-clean-install.service
[Service]
Type=oneshot
ExecStartPre=/bin/sleep 20
ExecStart=/bin/sh /var/lib/hearth-ci/$1.sh
UNIT
  sudo ln -sf "/etc/systemd/system/hearth-ci-$1.service" "$dep/etc/systemd/system/multi-user.target.wants/hearth-ci-$1.service"
  echo "$dep"
}

CODE=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
VARS0=$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)
cp "$VARS0" "$W/vars.fd"   # kept between starts: the firmware's boot entries, as on a real PC
kvm=""; [ -e /dev/kvm ] && kvm="-enable-kvm -cpu host"
start() {  # start NAME: one start through the firmware, nothing pressed, until the VM powers itself off (or 25 min)
  rm -f "$OUT/$1-serial.log"
  # shellcheck disable=SC2086
  timeout 1500 qemu-system-x86_64 -m 6144 -smp 4 $kvm -machine q35 \
    -drive if=pflash,format=raw,readonly=on,file="$CODE" -drive if=pflash,format=raw,file="$W/vars.fd" \
    -drive file="$W/disk.img",format=raw,if=virtio -display none -net none -serial file:"$OUT/$1-serial.log"
  tr -d '\r' < "$OUT/$1-serial.log" | grep -a 'HEARTH-CI\|grub>\|error:' | tail -20 > "$OUT/$1-marks.txt"
  if tr -d '\r' < "$OUT/$1-serial.log" | grep -aq '^grub> *$\|grub rescue>'; then fail "$1: the PC stopped at GRUB's prompt (no menu)"; fi
}
said() { tr -d '\r' < "$OUT/$1-serial.log" | grep -a "^$2" | tail -1; }

# Step 1's job: say what started. Step 2's: switch with this branch's hearth-admin, from the local copy of the image.
mount_disk
look 1-installed
sudo cp "$REPO/os/system_files/usr/libexec/nimbus/hearth-admin" "$M/r/ostree/deploy/default/var/lib/hearth-ci-admin"
sudo mkdir -p "$M/r/ostree/deploy/default/var/lib/hearth-ci"
sudo mv "$M/r/ostree/deploy/default/var/lib/hearth-ci-admin" "$M/r/ostree/deploy/default/var/lib/hearth-ci/hearth-admin"
echo "export Anatase"; sudo skopeo copy -q "containers-storage:$ANA" "oci:$M/r/ostree/deploy/default/var/lib/hearth-ci/anatase:img" || fail "couldn't put the Anatase image on the disk"
DEB_DEP=$(job switch-to-anatase '
echo "HEARTH-CI-edition $(python3 -c "import json; print(json.load(open(\"/usr/share/nimbus/image-info.json\"))[\"base\"])")"
echo "HEARTH-CI-booted $(ostree admin status | sed -n "s/^\* \S* \([0-9a-f]*\.[0-9]*\).*/\1/p")"
mkdir -p /var/home/tester/Documents && echo kept > /var/home/tester/Documents/hearth-ci.txt && chown -R tester /var/home/tester/Documents
PKEXEC_UID=$(id -u tester) python3 - <<PY
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("ha", "/var/lib/hearth-ci/hearth-admin")
spec = importlib.util.spec_from_loader("ha", loader); ha = importlib.util.module_from_spec(spec); loader.exec_module(ha)
# Only the transport differs from a device: the image comes from the disk, not ghcr.io (the VM has no network).
real = ha.bootc_upgrade_with_progress
def local(lo, hi, stage, cmd):
    if cmd[:2] == ["bootc", "switch"]:
        cmd = ["bootc", "switch", "--transport", "oci", "/var/lib/hearth-ci/anatase:img"]
    return real(lo, hi, stage, cmd)
ha.bootc_upgrade_with_progress = local
ha.move_to_new_os("anatase", False)
PY
echo "HEARTH-CI-move $?"
ostree admin status | sed "s/^/HEARTH-CI-status /"
')
umount_disk
start 1-debian
grep -aq 'HEARTH-CI-edition peppermint' "$OUT/1-debian-serial.log" || fail "1: Debian didn't start (or the job didn't run): $(tail -c 1500 "$OUT/1-debian-serial.log" | tr -d '\r')"
grep -aq 'HEARTH-CI-move 0' "$OUT/1-debian-serial.log" || fail "2: the switch to Anatase failed: $(tr -d '\r' < "$OUT/1-debian-serial.log" | grep -a 'error\|log:' | tail -8)"
grep -aq 'HEARTH-CI-status.*(pinned)\|Pinned: yes' "$OUT/1-debian-serial.log" || ann warning "2: Debian wasn't shown as pinned: $(grep -a HEARTH-CI-status "$OUT/1-debian-serial.log" | tr -d '\r' | tr '\n' ' ' | cut -c1-600)"
DEB_ID=$(said 1-debian HEARTH-CI-booted | awk '{print $2}')

# ---- 3. the restart into Anatase, nothing pressed
mount_disk
look 2-switched
sudo rm -rf "$M/r/ostree/deploy/default/var/lib/hearth-ci/anatase"   # room; not needed any more
job back-to-debian '
echo "HEARTH-CI-edition $(python3 -c "import json; print(json.load(open(\"/usr/share/nimbus/image-info.json\"))[\"base\"])")"
echo "HEARTH-CI-home $(cat /var/home/tester/Documents/hearth-ci.txt 2>&1)"
echo "HEARTH-CI-clean $(systemctl is-active hearth-clean-install.service) $(ls /var/lib/hearth/clean-install.done.json 2>&1)"
grep -q "^_apt:" /etc/passwd && echo "HEARTH-CI-accounts debian" || echo "HEARTH-CI-accounts fedora"
ostree admin status | sed "s/^/HEARTH-CI-status /"
PKEXEC_UID=$(id -u tester) python3 /var/lib/hearth-ci/hearth-admin move-to-new-os --edition peppermint
echo "HEARTH-CI-move $?"
ostree admin status | sed "s/^/HEARTH-CI-status /"
' >/dev/null
umount_disk
start 3-anatase
grep -aq 'HEARTH-CI-edition anatase' "$OUT/3-anatase-serial.log" || fail "3: the restart after the switch didn't start Hearth OS on Anatase by itself: $(tr -d '\r' < "$OUT/3-anatase-serial.log" | grep -av '^\s*$' | tail -25)"
grep -aq 'HEARTH-CI-home kept' "$OUT/3-anatase-serial.log" || fail "3: the home folder wasn't left as it was: $(said 3-anatase HEARTH-CI-home)"
grep -aq 'HEARTH-CI-accounts fedora' "$OUT/3-anatase-serial.log" || ann warning "3: Debian's accounts are still in /etc/passwd on Anatase"
grep -aq 'HEARTH-CI-move 0' "$OUT/3-anatase-serial.log" || fail "4: the switch back to Debian failed: $(tr -d '\r' < "$OUT/3-anatase-serial.log" | grep -a 'error\|log:' | tail -8)"
ann notice "3: $(said 3-anatase HEARTH-CI-clean) · $(tr -d '\r' < "$OUT/3-anatase-serial.log" | grep -a 'HEARTH-CI-status' | tr '\n' ' ' | cut -c1-900)"

# ---- 5. the restart back into Debian, nothing pressed
mount_disk
look 4-back
job report '
echo "HEARTH-CI-edition $(python3 -c "import json; print(json.load(open(\"/usr/share/nimbus/image-info.json\"))[\"base\"])")"
echo "HEARTH-CI-booted $(ostree admin status | sed -n "s/^\* \S* \([0-9a-f]*\.[0-9]*\).*/\1/p")"
echo "HEARTH-CI-home $(cat /var/home/tester/Documents/hearth-ci.txt 2>&1)"
' >/dev/null
umount_disk
start 5-debian-again
grep -aq 'HEARTH-CI-edition peppermint' "$OUT/5-debian-again-serial.log" || fail "5: the restart after switching back didn't start Hearth OS on Debian by itself: $(tr -d '\r' < "$OUT/5-debian-again-serial.log" | grep -av '^\s*$' | tail -25)"
back_id=$(said 5-debian-again HEARTH-CI-booted | awk '{print $2}')
[ -n "$DEB_ID" ] && [ "$back_id" = "$DEB_ID" ] && ann notice "5: the very Debian version came back ($DEB_ID)" || fail "5: a different Debian deployment started ($back_id, was $DEB_ID)"
grep -aq 'HEARTH-CI-home kept' "$OUT/5-debian-again-serial.log" || fail "5: the home folder changed"
mount_disk; look 5-final; umount_disk
[ "$FAILED" = 0 ] && ann notice "Debian → Anatase → Debian through the firmware with nothing pressed: every start came up by itself, the home folder kept, the same Debian back"
exit "$FAILED"
