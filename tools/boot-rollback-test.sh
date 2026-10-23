#!/bin/bash
# CI test of "back to the previous version by itself" (hearth-boot-guard) and the emergency screen, on an image just
# built on this runner (base-anatase.yml / base-peppermint.yml with boot_test on). Run from the repository root:
#   tools/boot-rollback-test.sh <local podman image> <anatase|peppermint>
#  1. installs the image onto a loop disk with Hearth's installer, the way the full USB stick does (from an OCI copy)
#  2. adds a second, newer deployment of the same image that can't start (systemd.unit=emergency.target), as an
#     update that goes wrong would be: it becomes the default, the good one is the previous version
#  3. boots the disk in a virtual PC (UEFI, serial console) and expects: the emergency screen twice (each restarting by
#     itself after its countdown), then GRUB starting the previous version, which marks the start as good and makes
#     itself the default again (bootc rollback)
#  4. checks the disk afterwards: the good version is the default, GRUB's count is back to ok, the bad one remembered
# Needs: podman, qemu-system-x86_64 + OVMF (installed here), sudo. Writes to /mnt/boottest (or ./boottest).
set -euo pipefail
TAG="$1"
ED="$2"
case "$ED" in
  anatase) REF=ghcr.io/dragongamer9/hearth-os:latest ;;
  peppermint) REF=ghcr.io/dragongamer9/hearth-peppermint:testing ;;
  *) echo "edition must be anatase or peppermint"; exit 2 ;;
esac
TOOLS=ghcr.io/dragongamer9/hearth-os:latest   # the installer runs in the Anatase image, as on the USB stick
W=/mnt/boottest
if [ "$(df --output=avail -B1 /mnt 2>/dev/null | tail -1 || echo 0)" -lt "$(df --output=avail -B1 . | tail -1)" ]; then W="$PWD/boottest"; fi
sudo mkdir -p "$W" && sudo chown "$(id -u)" "$W"
fail() { echo "::error title=Boot rollback test::$*"; exit 1; }
clean() { sed 's/\x1b\[[0-9;?]*[a-zA-Z]//g; s/\r//g' "$@"; }

echo "--- 1. install $TAG as $REF (offline, from an OCI copy)"
sudo podman push --quiet "$TAG" "oci:$W/image:hearth"
echo "$REF" > "$W/image.ref"
truncate -s 70G "$W/disk.img"
LOOP=$(sudo losetup -fP --show "$W/disk.img")
plan=$(printf '{"mode":"whole","disk":"%s","user":{"name":"tester","full":"Tester","password":"hearth"},"hostname":"hearth-ci","timezone":"UTC","edition":"%s","firmware":false,"signin":"always","kargs":["console=ttyS0,115200"]}' "$LOOP" "$ED")
cat > "$W/run.sh" <<'EOF'
need=""
ostree container --help >/dev/null 2>&1 || need="$need rpm-ostree"
command -v bootupctl >/dev/null || need="$need bootupd"
for t in mkfs.vfat:dosfstools mkfs.ext4:e2fsprogs mkfs.btrfs:btrfs-progs efibootmgr:efibootmgr; do
  command -v "${t%%:*}" >/dev/null || need="$need ${t#*:}"
done
if [ -n "$need" ]; then dnf -y -q install $need </dev/null >&2 || true; fi
exec /src/hearth-install "$@"
EOF
echo "$plan" | sudo podman run --rm -i --privileged --pid=host --net=host \
  -v /dev:/dev -v /run/udev:/run/udev -v "$PWD/os/live/src:/src:ro" -v "$W/run.sh:/run.sh:ro" \
  -v "$W/image:/usr/share/hearth/image:ro" -v "$W/image.ref:/usr/share/hearth/image.ref:ro" -e HEARTH_INSTALL_TEST=1 \
  --security-opt label=disable "$TOOLS" sh /run.sh run 2>&1 | grep -v '^stage: ' | tee "$W/install.log" || true
grep -q '^result: ' "$W/install.log" || fail "install failed: $(tail -n 15 "$W/install.log" | tr '\n' '|')"
sudo rm -rf "$W/image"   # room for the rest

echo "--- 2. a newer deployment that can't start"
sudo partx -u "$LOOP" || true
H=/mnt/rbt && sudo mkdir -p $H
sudo mount -o subvol=root "${LOOP}p3" $H && sudo mount "${LOOP}p2" $H/boot && sudo mount "${LOOP}p1" $H/boot/efi
dep=$(sudo sh -c "ls -d $H/ostree/deploy/default/deploy/*.0 | head -1")
csum=$(basename "$dep" .0)
sudo test -f "$dep/usr/libexec/nimbus/hearth-boot-guard" || fail "the installed system isn't the image built here (no hearth-boot-guard)"
sudo podman run --rm --privileged -v $H:$H --security-opt label=disable "$TOOLS" \
  ostree admin deploy --sysroot=$H --os=default --origin-file="$dep.origin" --karg-append=systemd.unit=emergency.target "$csum"
sudo python3 -I "$dep/usr/libexec/nimbus/hearth-boot-guard" setup --boot $H/boot
if sudo grep -qs hearth-bls.cfg $H/boot/grub2/grub.cfg; then sudo sh "$dep/usr/libexec/nimbus/hearth-bls-grub" --boot $H/boot; fi
sudo sh -c "grep -H . $H/boot/loader/entries/*.conf; cat $H/boot/grub2/custom.cfg $H/boot/grub2/hearth-boot.cfg; head -c 300 $H/boot/grub2/grubenv; echo"
newest=$(sudo sh -c "grep -l '^version 2' $H/boot/loader/entries/*.conf")
sudo grep -q 'systemd.unit=emergency.target' "$newest" || fail "the broken deployment isn't the default"
sudo umount -R $H && sudo losetup -d "$LOOP"

echo "--- 3. boot it"
sudo apt-get install -y -qq qemu-system-x86 ovmf >/dev/null
cp /usr/share/OVMF/OVMF_VARS_4M.fd "$W/vars.fd" 2>/dev/null || cp /usr/share/OVMF/OVMF_VARS.fd "$W/vars.fd"
code=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
timeout 2400 sudo qemu-system-x86_64 -m 4096 -smp 2 $( [ -e /dev/kvm ] && echo -enable-kvm -cpu host ) \
  -drive if=pflash,format=raw,readonly=on,file="$code" -drive if=pflash,format=raw,file="$W/vars.fd" \
  -drive file="$W/disk.img",format=raw,if=virtio -nographic -serial mon:stdio -display none -net none \
  > "$W/boot.log" 2>&1 < /dev/null &
ok=0
for i in $(seq 1 450); do
  if clean "$W/boot.log" | grep -q 'hearth-boot-guard: this start was good, after starting the previous version'; then ok=1; sleep 60; break; fi
  sleep 5
done
sudo pkill qemu-system || true
sleep 3
screens=$(clean "$W/boot.log" | grep -c "What would you like to do?" || true)
starts=$(clean "$W/boot.log" | grep -c "couldn't finish starting" || true)
echo "emergency screen drawn $screens times; 'couldn't finish starting' $starts times"
clean "$W/boot.log" | grep -a -E "hearth-boot-guard|Restarting|previous version|Command line" | uniq | tail -n 40 || true
cp "$W/boot.log" boot-rollback.log
[ "$ok" = 1 ] || fail "the previous version never came up by itself: $(clean "$W/boot.log" | tail -c 2500 | tr '\n' '|')"
[ "$starts" -ge 1 ] || fail "the emergency screen didn't show"

echo "--- 4. the disk afterwards"
LOOP=$(sudo losetup -fP --show "$W/disk.img")
sudo partx -u "$LOOP" || true
sudo mount -o subvol=root "${LOOP}p3" $H && sudo mount "${LOOP}p2" $H/boot
sudo sh -c "grep -H . $H/boot/loader/entries/*.conf; grep -a hearth_tries $H/boot/grub2/grubenv; cat $H/ostree/deploy/default/var/lib/hearth/boot-guard.json" || true
res=0
newest=$(sudo sh -c "grep -l '^version 2' $H/boot/loader/entries/*.conf")
if sudo grep -q 'systemd.unit=emergency.target' "$newest"; then echo "::error title=Boot rollback test::the version that didn't start is still the default"; res=1; else echo "ok: the good version is the default again"; fi
if sudo grep -aq '^hearth_tries=ok' $H/boot/grub2/grubenv; then echo "ok: GRUB's count is back to ok"; else echo "::error title=Boot rollback test::grubenv's hearth_tries isn't ok"; res=1; fi
if sudo grep -q '"rolledBack": true' $H/ostree/deploy/default/var/lib/hearth/boot-guard.json; then echo "ok: the automatic return is recorded"; else echo "::error title=Boot rollback test::no record of the return"; res=1; fi
sudo umount -R $H || true
sudo losetup -d "$LOOP" || true
[ $res = 0 ] && echo "::notice title=Boot rollback test::$ED: an update that can't start was rolled back by itself (emergency screen $starts times, then the previous version, now the default)"
exit $res
