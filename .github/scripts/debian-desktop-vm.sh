#!/usr/bin/env bash
# Hearth OS on Debian, for real, in a virtual PC (the owner, Nov 23, 2026, after two Fedora test builds that couldn't
# sign in reached his laptop: nothing goes to a device until a virtual PC shows it signing in and opening the desktop).
#   1 The image installed with Hearth's installer engine (automatic sign-in, as on a new PC), started through the
#     firmware and the boot menu, nothing pressed.
#   2 Signed in by itself (loginctl has the account's session), Game Mode as it opens first: screenshot game.png.
#   3 nimbus-mode desktop: Mantel up (hearth-shell-up) on labwc: screenshot desktop.png (it must not be one colour).
#   4 At rest for REST_SECS (default 3600): Mantel's and labwc's memory and CPU every minute (the 1.10 leak: 1.5 GB
#     after 25 hours), screenshots every 10 minutes. Fails when Mantel keeps growing (the last 10 minutes more than
#     30% and 60 MB above the first 10) or stays busy (more than 15% of a core on average).
# Screenshots and the minute-by-minute numbers go to OUTDIR (the workflow posts them to the ci/debian-desktop branch).
#   debian-desktop-vm.sh IMAGE OUTDIR
set -uo pipefail
IMAGE=${1:?image}; OUT=${2:?out dir}; REST=${REST_SECS:-3600}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
REPO=$(cd "$(dirname "$0")/../.." && pwd)
W=/mnt/debian-desktop; sudo mkdir -p "$W"; sudo chown "$(id -u)" "$W"
ann() { local t=$1; shift; echo "::$t title=Debian desktop (VM)::$(echo "$*" | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' -e 's/%/%25/g' | tr '\n' '|' | cut -c1-3500)"; }
FAILED=0
fail() { FAILED=1; ann error "$*"; }

sudo apt-get update -qq >/dev/null
sudo apt-get install -y -qq qemu-system-x86 ovmf socat python3-pil btrfs-progs >/dev/null
[ -e /dev/kvm ] && sudo chmod 666 /dev/kvm

# ---- 1. install (as the offline USB does)
sudo skopeo copy -q "containers-storage:$IMAGE" "oci:$W/oci:hearth" || { ann error "couldn't export $IMAGE"; exit 1; }
echo ghcr.io/dragongamer9/hearth-peppermint:testing > "$W/oci.ref"
truncate -s 80G "$W/disk.img"
LOOP=$(sudo losetup -fP --show "$W/disk.img")
plan=$(printf '{"mode":"whole","disk":"%s","user":{"name":"tester","full":"Tester","password":"hearth"},"hostname":"hearth-desktop","timezone":"UTC","edition":"peppermint","firmware":false,"signin":"auto","kargs":["console=ttyS0,115200","console=tty0","plymouth.ignore-serial-consoles"]}' "$LOOP")
echo "$plan" | sudo podman run --rm -i --privileged --pid=host --net=host -v /dev:/dev -v /run/udev:/run/udev \
  -v "$REPO/os/live/src:/src:ro" -v "$W/oci:/usr/share/hearth/image:ro" -v "$W/oci.ref:/usr/share/hearth/image.ref:ro" \
  -e HEARTH_INSTALL_TEST=1 --security-opt label=disable quay.io/fedora/fedora-bootc:43 \
  sh -c 'dnf -y -q install dosfstools e2fsprogs btrfs-progs efibootmgr openssl >/dev/null 2>&1; exec /src/hearth-install run' 2>&1 | tee "$OUT/install.log" | grep -v '^stage: ' | tail -8
grep -q '^result: ' "$OUT/install.log" || { ann error "the install failed: $(grep -v '^stage: ' "$OUT/install.log" | tail -12)"; sudo losetup -d "$LOOP"; exit 1; }
rm -rf "$W/oci"

# ---- the job for the first start: what it sees, said on the serial console with "HCI| " in front
M=$W/m; sudo mkdir -p "$M/r"
sudo partx -u "$LOOP" 2>/dev/null
sudo mount -o subvol=root "${LOOP}p3" "$M/r" || { ann error "couldn't mount the installed system"; sudo losetup -d "$LOOP"; exit 1; }
DEP=$(sudo sh -c "ls -d $M/r/ostree/deploy/default/deploy/*.0 | head -1")
CI=$M/r/ostree/deploy/default/var/lib/hearth-ci
sudo mkdir -p "$CI" "$DEP/etc/systemd/system/graphical.target.wants"
sudo tee "$CI/desktop.sh" >/dev/null <<JOB
#!/bin/bash
exec > >(tee /var/lib/hearth-ci/desktop.log | sed -u 's/^/HCI| /' > /dev/ttyS0) 2>&1
REST=$REST
JOB
sudo tee -a "$CI/desktop.sh" >/dev/null <<'JOB'
u=tester; uid=$(id -u $u); home=$(getent passwd $u | cut -d: -f6)
as() { runuser -u $u -- env XDG_RUNTIME_DIR=/run/user/$uid "$@"; }
mode=$(command -v nimbus-mode || ls /usr/local/bin/nimbus-mode /usr/bin/nimbus-mode 2>/dev/null | head -1)
for i in $(seq 120); do loginctl list-sessions --no-legend 2>/dev/null | grep -q " $u " && break; sleep 2; done
echo "HEARTH-CI-session $(loginctl list-sessions --no-legend 2>/dev/null | grep " $u " | head -1 | tr -s ' ')"
echo "HEARTH-CI-login $(systemctl is-active display-manager.service 2>&1) $(journalctl -b --no-pager -o cat 2>/dev/null | grep -aiE 'unable to exec|panicked|Permission denied|authentication failure' | head -2 | tr '\n' ' ')"
sleep 45
echo "HEARTH-CI-game $(as "$mode" current 2>&1 | head -1) · $(pgrep -a -u $u 'gamescope|steam|hearth-rs|nimbus|cage' 2>/dev/null | cut -c1-80 | head -6 | tr '\n' ';')"
echo "HEARTH-CI-SHOT game"; sleep 10
as "$mode" desktop >/dev/null 2>&1 &
for i in $(seq 90); do [ -e /run/user/$uid/hearth-shell-up ] && break; sleep 2; done
sleep 25
echo "HEARTH-CI-desktop up=$([ -e /run/user/$uid/hearth-shell-up ] && echo yes || echo no) · $(pgrep -a -u $u 'labwc|kwin_wayland|quickshell' 2>/dev/null | cut -c1-80 | head -5 | tr '\n' ';')"
echo "HEARTH-CI-SHOT desktop"; sleep 10
# At rest: Mantel (quickshell) and labwc, every minute: seconds, Mantel's memory (kB), labwc's, Mantel's CPU over the
# minute (% of a core), memory in use (MB).
hz=$(getconf CLK_TCK); last=""; start=$SECONDS
ticks() { awk '{print $14+$15}' /proc/$1/stat 2>/dev/null; }
while [ $((SECONDS - start)) -lt "$REST" ]; do
  q=$(pgrep -u $u -x quickshell | head -1); l=$(pgrep -u $u -x labwc | head -1)
  t=$(ticks "$q"); cpu=""
  [ -n "$last" ] && [ -n "$t" ] && cpu=$(( (t - last) * 100 / (hz * 60) ))
  last=$t
  echo "HEARTH-CI-mem $((SECONDS - start)) $(awk '/VmRSS/{print $2}' /proc/$q/status 2>/dev/null) $(awk '/VmRSS/{print $2}' /proc/$l/status 2>/dev/null) ${cpu:--} $(free -m | awk '/^Mem:/{print $3}')"
  [ $(( (SECONDS - start) % 600 )) -lt 60 ] && echo "HEARTH-CI-SHOT rest-$(( (SECONDS - start) / 60 ))"
  sleep 60
done
echo "HEARTH-CI-SHOT end"; sleep 10
tail -25 "$home/.local/share/nimbus/session.log" 2>/dev/null | sed 's/^/HEARTH-CI-log /'
echo HEARTH-CI-desktop-END
sleep 2; systemctl poweroff
JOB
sudo chmod 755 "$CI/desktop.sh"
sudo tee "$DEP/etc/systemd/system/hearth-ci-desktop.service" >/dev/null <<'UNIT'
[Unit]
Description=CI: Debian desktop
After=graphical.target
[Service]
Type=simple
ExecStart=/bin/bash /var/lib/hearth-ci/desktop.sh
UNIT
sudo ln -sf /etc/systemd/system/hearth-ci-desktop.service "$DEP/etc/systemd/system/graphical.target.wants/hearth-ci-desktop.service"
sync; sudo umount "$M/r"; sudo losetup -d "$LOOP"

# ---- 2-4. start it and watch
CODE=$(ls /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd 2>/dev/null | head -1)
VARS0=$(ls /usr/share/OVMF/OVMF_VARS_4M.fd /usr/share/OVMF/OVMF_VARS.fd 2>/dev/null | head -1)
cp "$VARS0" "$W/vars.fd"
kvm=""; [ -e /dev/kvm ] && kvm="-enable-kvm -cpu host"
mon() { echo "$1" | socat - "unix-connect:$W/mon" >/dev/null 2>&1; }
shot() { mon "screendump $W/s.ppm" || return 0; sleep 1; python3 -c "from PIL import Image; Image.open('$W/s.ppm').save('$OUT/$1.png')" 2>/dev/null || true; }
S=$OUT/serial.log
# shellcheck disable=SC2086
qemu-system-x86_64 -m 6144 -smp 4 $kvm -machine q35 \
  -drive if=pflash,format=raw,readonly=on,file="$CODE" -drive if=pflash,format=raw,file="$W/vars.fd" \
  -drive file="$W/disk.img",format=raw,if=virtio -vga none -device VGA,edid=on,xres=1280,yres=800 -display none \
  -net none -serial file:"$S" -monitor unix:"$W/mon",server,nowait &
q=$!
said() { tr -d '\r' < "$S" | sed -n 's/^.*HCI| //p'; }
done_shots=" "; t0=$(date +%s); limit=$((REST + 1800))
while kill -0 $q 2>/dev/null; do
  sleep 3
  for n in $(said 2>/dev/null | sed -n 's/^HEARTH-CI-SHOT //p'); do
    case "$done_shots" in *" $n "*) ;; *) shot "$n"; done_shots="$done_shots$n " ;; esac
  done
  if [ $(( $(date +%s) - t0 )) -gt $limit ]; then shot timeout; ann warning "the virtual PC didn't finish in time; stopped"; kill $q; break; fi
done
wait $q 2>/dev/null
said > "$OUT/job.txt" 2>/dev/null
said | grep '^HEARTH-CI-mem ' | cut -d' ' -f2- > "$OUT/memory.txt"

# ---- the verdicts
v() { grep -a "^$1" "$OUT/job.txt" | tail -1; }
ann notice "signed in: $(v HEARTH-CI-session) · login screen: $(v HEARTH-CI-login)"
ann notice "Game Mode: $(v HEARTH-CI-game)"
ann notice "Desktop Mode: $(v HEARTH-CI-desktop)"
v HEARTH-CI-session | grep -q ' tester ' || fail "nobody was signed in: no session for the account (the PC would stay at the sign-in). $(v HEARTH-CI-login)"
v HEARTH-CI-login | grep -qiE 'unable to exec|panicked|Permission denied' && fail "the sign-in failed: $(v HEARTH-CI-login)"
v HEARTH-CI-desktop | grep -q 'up=yes' || fail "Mantel didn't come up after Desktop Mode was asked for: $(v HEARTH-CI-desktop) · $(grep -a '^HEARTH-CI-log' "$OUT/job.txt" | tail -8 | cut -c15-)"
v HEARTH-CI-desktop | grep -q 'labwc' || fail "Desktop Mode isn't on labwc (1.13's, the smooth one): $(v HEARTH-CI-desktop)"
python3 - "$OUT" <<'PY' || FAILED=1
import os, sys
from PIL import Image, ImageStat
out = sys.argv[1]
bad = []
for n in ('game', 'desktop'):
    p = os.path.join(out, n + '.png')
    if not os.path.exists(p):
        bad.append('%s: no screenshot' % n); continue
    s = ImageStat.Stat(Image.open(p).convert('L'))
    if s.stddev[0] < 6:
        bad.append('%s: the screen is one colour (%s, brightness %d)' % (n, 'black' if s.mean[0] < 20 else 'blank', s.mean[0]))
rows = []
for line in open(os.path.join(out, 'memory.txt')):
    f = line.split()
    if len(f) >= 4 and f[1].isdigit():
        rows.append((int(f[0]), int(f[1]), int(f[2]) if f[2].isdigit() else 0, int(f[3]) if f[3].lstrip('-').isdigit() else None))
if len(rows) < 12:
    bad.append('at rest: only %d minutes measured' % len(rows))
else:
    first = sum(r[1] for r in rows[:10]) / 10; last = sum(r[1] for r in rows[-10:]) / 10
    cpus = [r[3] for r in rows if r[3] is not None]
    avg = sum(cpus) / len(cpus) if cpus else 0
    print('::notice title=Debian desktop (VM)::at rest %d min: Mantel %d MB at first, %d MB at the end; labwc %d MB; Mantel CPU %.1f%% of a core on average' % (
        rows[-1][0] // 60, first / 1024, last / 1024, rows[-1][2] / 1024, avg))
    if last > first * 1.3 and last - first > 60 * 1024:
        bad.append('Mantel kept growing at rest: %d MB to %d MB' % (first / 1024, last / 1024))
    if avg > 15:
        bad.append('Mantel stayed busy at rest: %.1f%% of a core' % avg)
for b in bad:
    print('::error title=Debian desktop (VM)::' + b)
sys.exit(1 if bad else 0)
PY
[ "$FAILED" = 0 ] && ann notice "passed: signed in, Game Mode, Mantel on labwc, at rest without growing"
exit $FAILED
