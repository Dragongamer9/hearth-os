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
pull() {  # pull IMAGE: up to 5 tries (a registry's CDN can drop a download: EOF)
  local i
  for i in 1 2 3 4 5; do sudo podman pull -q "$1" >/dev/null && return 0; echo "pull $1 failed (try $i)"; sleep $((i * 15)); done
  return 1
}
pull quay.io/fedora/fedora-bootc:43 || { ann error "couldn't pull quay.io/fedora/fedora-bootc:43 after 5 tries"; exit 1; }
pull "$DEB" || { ann error "couldn't pull $DEB after 5 tries"; exit 1; }
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
# The EFI partition after a start: Debian's loader still there (the previous version stays startable), and every loader
# folder with a GRUB has the grub.cfg and bootuuid.cfg that find /boot's menu (the laptop's "grub>": EFI/anatase without).
esp_ok() {  # esp_ok STEP
  local d
  for f in EFI/debian/shimx64.efi EFI/debian/grubx64.efi EFI/debian/grub.cfg; do
    sudo test -f "$M/e/$f" || fail "$1: $f is gone from the EFI partition (Debian's loader must stay)"
  done
  for d in $(sudo find "$M/e/EFI" -mindepth 1 -maxdepth 1 -type d); do
    if sudo test -f "$d/grubx64.efi"; then
      sudo test -f "$d/grub.cfg" && sudo test -f "$d/bootuuid.cfg" || fail "$1: ${d#$M/e/} has a GRUB but no grub.cfg/bootuuid.cfg (that GRUB would stop at grub>)"
    fi
  done
}
# A one-time job for the next start, in the default deployment's /etc, that runs a script kept in /var (so it runs
# even if a first start's clean install sets /etc's extra files aside) and then powers the VM off.
job() {  # job NAME SCRIPT-TEXT
  local dep; dep=$(default_dep)
  sudo mkdir -p "$M/r/ostree/deploy/default/var/lib/hearth-ci" "$dep/etc/systemd/system/multi-user.target.wants"
  sudo test -s "$M/r/ostree/deploy/default/var/lib/hearth-ci/hearth-admin" || ann warning "job $1: hearth-admin isn't in /var/lib/hearth-ci"
  # Everything the job says goes to the serial console with "HCI| " in front (GRUB's own console text has none) and
  # to /var/lib/hearth-ci/NAME.log, which the host reads after the start (job_log).
  printf '%s\n' "#!/bin/bash" "exec > >(tee /var/lib/hearth-ci/$1.log | sed -u 's/^/HCI| /' > /dev/ttyS0) 2>&1" "$2" "echo HEARTH-CI-$1-END; sleep 2; systemctl poweroff" | sudo tee "$M/r/ostree/deploy/default/var/lib/hearth-ci/$1.sh" >/dev/null
  sudo chmod 755 "$M/r/ostree/deploy/default/var/lib/hearth-ci/$1.sh"
  sudo rm -f "$dep"/etc/systemd/system/hearth-ci-*.service "$dep"/etc/systemd/system/multi-user.target.wants/hearth-ci-*.service
  sudo tee "$dep/etc/systemd/system/hearth-ci-$1.service" >/dev/null <<UNIT
[Unit]
Description=CI: $1
After=multi-user.target hearth-clean-install.service
[Service]
Type=oneshot
ExecStartPre=/bin/sleep 20
ExecStart=/bin/bash /var/lib/hearth-ci/$1.sh
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
said() { tr -d '\r' < "$OUT/$1-serial.log" | sed -n 's/^.*HCI| //p' | grep -a "^$2" | tail -1; }
# What a start's job said (from the disk, after the start), for the annotations.
job_log() {  # job_log NAME
  sudo cat "$M/r/ostree/deploy/default/var/lib/hearth-ci/$1.log" 2>/dev/null | grep -av '^progress: \|^stage: ' | tail -40 | tee "$OUT/$1-job.log"
}
jobsaid() { tr -d '\r' < "$OUT/$1-serial.log" | sed -n 's/^.*HCI| //p'; }

# Step 1's job: say what started. Step 2's: switch with this branch's hearth-admin, from the local copy of the image.
mount_disk
look 1-installed
# (A fresh install's /var has no lib/ yet: tmpfiles makes it at the first start. So the folder first, then the copy,
# and the build stops here if it isn't there: run 44255858138 copied nothing and the switch never ran.)
CI=$M/r/ostree/deploy/default/var/lib/hearth-ci
sudo mkdir -p "$CI"
sudo install -m 755 "$REPO/os/system_files/usr/libexec/nimbus/hearth-admin" "$CI/hearth-admin"
sudo python3 -m py_compile "$CI/hearth-admin" 2>/dev/null; sudo rm -rf "$CI/__pycache__"
sudo test -s "$CI/hearth-admin" || { ann error "couldn't put this branch's hearth-admin on the VM's disk ($CI)"; umount_disk; exit 1; }
echo "export Anatase"; sudo skopeo copy -q "containers-storage:$ANA" "oci:$CI/anatase:img" || fail "couldn't put the Anatase image on the disk"
sudo test -f "$CI/anatase/index.json" || { ann error "the Anatase image isn't on the VM's disk"; umount_disk; exit 1; }
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
jobsaid 1-debian | grep -q 'HEARTH-CI-edition peppermint' || fail "1: Debian didn't start (or the job didn't run): $(tail -c 1500 "$OUT/1-debian-serial.log" | tr -d '\r')"
jobsaid 1-debian | grep -q 'HEARTH-CI-move 0' || fail "2: the switch to Anatase failed: $(jobsaid 1-debian | grep -v '^HEARTH-CI-status' | tail -25)"
jobsaid 1-debian | grep -q 'HEARTH-CI-status.*(pinned)\|Pinned: yes' || ann warning "2: Debian wasn't shown as pinned: $(grep -a HEARTH-CI-status "$OUT/1-debian-serial.log" | tr -d '\r' | tr '\n' ' ' | cut -c1-600)"
DEB_ID=$(said 1-debian HEARTH-CI-booted | awk '{print $2}')

# ---- 3. the restart into Anatase, nothing pressed
mount_disk
ann notice "1-2 job (Debian, the switch): $(job_log switch-to-anatase)"
look 2-switched; esp_ok 2-switched
sudo rm -rf "$M/r/ostree/deploy/default/var/lib/hearth-ci/anatase"   # room; not needed any more
# The laptop's broken state, half of it: a loader folder with a GRUB but no grub.cfg (EFI/anatase). Anatase's
# hearth-esp-guard must give it the config at this start (checked after it, esp_ok).
sudo mkdir -p "$M/e/EFI/anatase" && sudo cp "$M/e/EFI/debian/grubx64.efi" "$M/e/EFI/anatase/grubx64.efi"
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
jobsaid 3-anatase | grep -q 'HEARTH-CI-edition anatase' || fail "3: the restart after the switch didn't start Hearth OS on Anatase by itself: $(tr -d '\r' < "$OUT/3-anatase-serial.log" | grep -av '^\s*$' | tail -25)"
jobsaid 3-anatase | grep -q 'HEARTH-CI-home kept' || fail "3: the home folder wasn't left as it was: $(said 3-anatase HEARTH-CI-home)"
jobsaid 3-anatase | grep -q 'HEARTH-CI-accounts fedora' || ann warning "3: Debian's accounts are still in /etc/passwd on Anatase"
jobsaid 3-anatase | grep -q 'HEARTH-CI-move 0' || fail "4: the switch back to Debian failed: $(jobsaid 3-anatase | grep -v '^HEARTH-CI-status' | tail -25)"
ann notice "3: $(said 3-anatase HEARTH-CI-clean) · $(jobsaid 3-anatase | grep 'HEARTH-CI-status' | tr '\n' ' ' | cut -c1-900)"

# ---- 5. the restart back into Debian, nothing pressed
mount_disk
ann notice "3-4 job (Anatase, the switch back): $(job_log back-to-debian)"
look 4-back; esp_ok 3-after-anatase
job report '
echo "HEARTH-CI-edition $(python3 -c "import json; print(json.load(open(\"/usr/share/nimbus/image-info.json\"))[\"base\"])")"
echo "HEARTH-CI-booted $(ostree admin status | sed -n "s/^\* \S* \([0-9a-f]*\.[0-9]*\).*/\1/p")"
echo "HEARTH-CI-home $(cat /var/home/tester/Documents/hearth-ci.txt 2>&1)"
' >/dev/null
umount_disk
start 5-debian-again
jobsaid 5-debian-again | grep -q 'HEARTH-CI-edition peppermint' || fail "5: the restart after switching back didn't start Hearth OS on Debian by itself: $(tr -d '\r' < "$OUT/5-debian-again-serial.log" | grep -av '^\s*$' | tail -25)"
back_id=$(said 5-debian-again HEARTH-CI-booted | awk '{print $2}')
[ -n "$DEB_ID" ] && [ "$back_id" = "$DEB_ID" ] && ann notice "5: the very Debian version came back ($DEB_ID)" || fail "5: a different Debian deployment started ($back_id, was $DEB_ID)"
jobsaid 5-debian-again | grep -q 'HEARTH-CI-home kept' || fail "5: the home folder changed"
mount_disk; look 5-final; esp_ok 5-final; umount_disk
[ "$FAILED" = 0 ] && ann notice "Debian → Anatase → Debian through the firmware with nothing pressed: every start came up by itself, the home folder kept, the same Debian back"
exit "$FAILED"
