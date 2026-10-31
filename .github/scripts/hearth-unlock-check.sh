#!/bin/bash
# Hearth OS on Peppermint is writable (hearth-unlock): the real script, real overlay mounts and real apt, in a
# privileged container, with "starts" played out. Run on the CI runner (it drives podman):
#   bash .github/scripts/hearth-unlock-check.sh IMAGE image   the built hearth-peppermint image (base-peppermint.yml)
#   bash .github/scripts/hearth-unlock-check.sh IMAGE plain   plain debian:trixie set up the same way (boot-guard.yml)
# What a start is here: /usr is the image's (made read-only with a bind mount, as ostree has it), then
# `hearth-unlock --boot` exactly as hearth-unlock.service runs it; a restart takes Hearth's layer off /usr again
# (umount -l) and starts over. HEARTH_UNLOCK_ID plays the OS version (an ostree commit): A, then an update to B,
# then a rollback to A. The state (/var/lib/hearth) and /etc/hearth are folders on the runner, kept between starts
# like on a device. Checks:
#   unlocked by default · an apt install (and a .deb from a file) works and survives a restart · the record follows
#   by itself (dpkg's and apt's hooks) · an update starts clean and the reapply puts everything back · the previous
#   version keeps its own layer · lock and unlock · a safe start after a start that never reached Hearth, and
#   turning the changes back on · the boot-chain pin (GRUB can't come from apt).
# Failures are ::error annotations and exit 1.
set -u
IMG=${1:?image}
MODE=${2:-image}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$(mktemp -d)
A=$(printf 'a%.0s' $(seq 64))
B=$(printf 'b%.0s' $(seq 64))
NAME=hearth-unlock-check-$$
fail=0
err() { echo "::error title=hearth-unlock::$*"; fail=1; }
ok() { echo "ok: $*"; }
mkdir -p "$WORK/state" "$WORK/etc-hearth"

sudo podman run -d --name "$NAME" --privileged --entrypoint sleep \
  -v "$ROOT:/src:ro" -v "$WORK/state:/var/lib/hearth" -v "$WORK/etc-hearth:/etc/hearth" "$IMG" infinity >/dev/null \
  || { echo "::error title=hearth-unlock::couldn't start the container"; exit 1; }
trap 'sudo podman rm -f "$NAME" >/dev/null 2>&1; sudo rm -rf "$WORK"' EXIT
x() { sudo podman exec -e LC_ALL=C -e DEBIAN_FRONTEND=noninteractive "$@"; }
ic() { x "$NAME" bash -c "$1"; }

# ---- set up
if [ "$MODE" = plain ]; then
  ic 'set -e
    apt-get update -qq && apt-get install -y -qq python3 >/dev/null
    install -m755 /src/os-bases/peppermint/system_files/usr/bin/hearth-unlock /usr/bin/hearth-unlock
    mkdir -p /usr/share/hearth /usr/lib/sysimage/apt
    apt-mark showmanual | sort -u > /usr/share/hearth/apt-baseline.list
    if [ -f /var/lib/apt/extended_states ]; then cp -a /var/lib/apt/extended_states /usr/lib/sysimage/apt/; else : > /usr/lib/sysimage/apt/extended_states; fi
    install -Dm644 /src/os-bases/peppermint/system_files/usr/share/hearth/unlock/apt.conf /etc/apt/apt.conf.d/50-hearth-unlock
    install -Dm644 /src/os-bases/peppermint/system_files/usr/share/hearth/unlock/dpkg.cfg /etc/dpkg/dpkg.cfg.d/hearth-unlock
    install -Dm644 /src/os-bases/peppermint/system_files/usr/share/hearth/unlock/boot-chain.pref /etc/apt/preferences.d/hearth-boot-chain
    mv /var/lib/dpkg /usr/lib/sysimage/dpkg && ln -s /usr/lib/sysimage/dpkg /var/lib/dpkg' \
    || { echo "::error title=hearth-unlock::setting up the plain container failed"; exit 1; }
else
  # what tmpfiles.d makes at boot on a device
  ic 'mkdir -p /var/lib /var/cache/apt/archives/partial /var/lib/apt/lists/partial /var/log/apt /var/tmp
      ln -sfn /usr/lib/sysimage/dpkg /var/lib/dpkg
      systemd-tmpfiles --create --prefix=/var/cache/debconf >/dev/null 2>&1 || true'
  for f in /usr/bin/hearth-unlock /etc/apt/apt.conf.d/50-hearth-unlock /etc/dpkg/dpkg.cfg.d/hearth-unlock \
           /etc/apt/preferences.d/hearth-boot-chain /usr/share/hearth/apt-baseline.list /usr/lib/sysimage/apt/extended_states \
           /usr/lib/systemd/system/hearth-unlock.service; do
    ic "test -e $f" && ok "in the image: $f" || err "missing in the image: $f"
  done
  ic 'systemctl is-enabled hearth-unlock.service hearth-unlock-reapply.service hearth-unlock-good.service hearth-unlock-units.service' \
    && ok "units enabled" || err "the hearth-unlock units aren't enabled"
  n=$(ic 'diff <(apt-mark showmanual | sort -u) /usr/share/hearth/apt-baseline.list | grep -c "^[<>]"')
  [ "${n:-1}" = 0 ] && ok "apt's marks in /usr match the image's baseline" || err "apt-mark showmanual differs from the baseline ($n lines)"
fi
ic 'touch /run/ostree-booted; mount --bind /usr /usr && mount -o remount,bind,ro /usr' || err "couldn't make /usr read-only"
ic 'touch /usr/.rw-test 2>/dev/null' && err "/usr is writable before hearth-unlock (the test setup is wrong)"

LAST=""
start() {   # start VERSION: a restart, then hearth-unlock.service
  ic 'while grep -q " /usr .* - overlay hearth-usr " /proc/self/mountinfo; do umount -l /usr || break; done'
  LAST=$(x -e HEARTH_UNLOCK_ID="$1" "$NAME" /usr/bin/hearth-unlock --boot 2>&1)
  echo "$LAST" | sed 's/^/  /'
}
diag() {   # what a failure needs to be understood (CI logs can't be downloaded: annotations)
  echo "::error title=hearth-unlock diag::boot said: $(echo "$LAST" | tr '\n' ' ' | cut -c1-400)"
  echo "::error title=hearth-unlock diag::status: $(st "$1" | cut -c1-600)"
  echo "::error title=hearth-unlock diag::mounts: $(ic 'grep " /usr " /proc/self/mountinfo | cut -d" " -f5,9- | cut -c1-200' | tr '\n' '|')"
  echo "::error title=hearth-unlock diag::layer $1: $(ic "cat /var/lib/hearth/unlock/layers/$1/layer.json | tr -d '\n' | cut -c1-300; echo; ls /var/lib/hearth/unlock/layers/$1/upper /var/lib/hearth/unlock/layers/$1/upper/bin /var/lib/hearth/unlock/layers/$1/upper/games 2>&1 | tr '\n' ' ' | cut -c1-300")"
}
st() { x -e HEARTH_UNLOCK_ID="$1" "$NAME" /usr/bin/hearth-unlock --status --json 2>/dev/null | tail -n1; }
field() { python3 -c "import json,sys; d=json.loads(sys.argv[1]); print(eval(sys.argv[2], {}, {'d': d}))" "$1" "$2" 2>/dev/null; }
U() { x -e HEARTH_UNLOCK_ID="$1" "$NAME" bash -c "$2"; }
python3 "$ROOT/hosts/linux/test/make_test_deb.py" "$WORK/state/dl.deb" >/dev/null

# ---- 1. first start: unlocked by default, apt works
echo "--- start 1 (version A)"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && ok "unlocked by default" || err "not unlocked by default: $s"
U "$A" 'touch /usr/lib/hearth-rw-test && rm /usr/lib/hearth-rw-test' && ok "/usr writable" || err "/usr isn't writable after --boot"
U "$A" 'apt-get update -qq && apt-get install -y -qq sl figlet >/tmp/apt.log 2>&1' && ok "apt-get install sl figlet" \
  || err "apt-get install failed: $(ic 'tail -3 /tmp/apt.log' | tr '\n' ' ')"
U "$A" 'cp /var/lib/hearth/dl.deb /tmp/hearth-hello_1.0_all.deb && apt-get install -y -qq /tmp/hearth-hello_1.0_all.deb >/tmp/apt2.log 2>&1' \
  && ok ".deb from a file installed with apt" || err ".deb install failed: $(ic 'tail -3 /tmp/apt2.log' | tr '\n' ' ')"
got=""
for i in $(seq 45); do   # the hooks record in the background, once apt and dpkg are done
  s=$(st "$A"); got=$(field "$s" "' '.join(d['packages'])")
  [ "$(field "$s" "all(p in d['packages'] for p in ('sl', 'figlet', 'hearth-hello'))")" = True ] && break
  sleep 2
done
for p in sl figlet hearth-hello; do case " $got " in *" $p "*) ok "recorded by the hooks: $p";; *) err "the record has no $p (it has: $got)";; esac; done
ic 'test -f /var/lib/hearth/unlock/debs/hearth-hello.deb' && ok "a copy of the .deb kept for updates" || err "the .deb from a file wasn't kept"
case " $got " in *" bash "*|*" apt "*) err "image packages ended up in the record: $got";; esac
# (packages the image doesn't have: one it has is "already the newest version" either way)
for p in linux-image-rt-amd64 systemd-boot; do
  ic "apt-get install -s -y $p >/dev/null 2>&1" && err "$p could be installed with apt (the boot-chain pin isn't working)" || ok "the boot chain can't come from apt: $p"
done
echo "::notice title=hearth-unlock pin::$(ic 'apt-cache policy shim-signed grub-efi-amd64-signed 2>/dev/null | grep -E "^[a-z]|Installed|Candidate" | tr "\n" " "' | cut -c1-300)"

# ---- 2. restart, same version: everything's still there
echo "--- start 2 (version A again)"
start "$A"
U "$A" 'test -x /usr/games/sl && command -v figlet >/dev/null && command -v hearth-hello >/dev/null && dpkg -s sl >/dev/null' \
  && ok "survives a restart (files and dpkg's database)" || err "the packages didn't survive a restart"
# Hearth came up: hearth-unlock-good.service's job
ic 'python3 - <<EOF
import json; p="/var/lib/hearth/unlock/layers/'"$A"'/layer.json"; d=json.load(open(p)); d["pending"]=0; json.dump(d,open(p,"w"))
EOF'

# ---- 3. an update: version B starts clean, then the reapply
echo "--- start 3 (update to version B)"
start "$B"
U "$B" 'test -e /usr/games/sl' && err "the new version started with the old layer" || ok "the new version starts clean"
s=$(st "$B"); [ "$(field "$s" "d['reapply'].get('state')")" = pending ] && ok "reapply pending on the new version" || err "no reapply pending: $s"
U "$B" '/usr/bin/hearth-unlock --after-start 2>&1 | tail -5' | sed 's/^/  /'
s=$(st "$B"); r=$(field "$s" "d['reapply'].get('state')")
[ "$r" = done ] && ok "reapply done" || err "reapply: $r ($(ic 'cat /var/lib/hearth/unlock/reapply.log' | tail -5 | tr '\n' ' '))"
U "$B" 'test -x /usr/games/sl && command -v figlet >/dev/null && command -v hearth-hello >/dev/null' \
  && ok "packages back on the new version (repository and saved .deb)" || err "packages missing after the reapply"
echo "--- start 4 (version B again)"
start "$B"
U "$B" 'command -v figlet >/dev/null' && ok "the reapplied packages survive a restart" || err "reapplied packages gone after a restart"
U "$B" 'apt-get remove -y -qq figlet >/dev/null 2>&1' || err "apt-get remove failed"
sleep 8

# ---- 5. rollback: version A as it was
echo "--- start 5 (rollback to version A)"
start "$A"
# (dpkg and the files, not `command -v figlet`: figlet's /usr/bin/figlet is an alternative in /etc, which this
# container shares between versions; on a device each version has its own /etc)
U "$A" 'dpkg -s figlet >/dev/null && test -x /usr/bin/figlet-figlet && test -x /usr/games/sl' && ok "the previous version keeps its own layer, untouched" \
  || { err "the previous version lost its packages"; diag "$A"; }

# ---- 6. lock and unlock
echo "--- lock"
U "$A" '/usr/bin/hearth-unlock --lock --json >/dev/null' || err "--lock failed"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = locks-at-restart ] && ok "lock: from the next restart" || err "lock state: $s"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = locked ] && ok "locked after a restart" || err "not locked after a restart: $s"
U "$A" 'touch /usr/lib/hearth-rw-test 2>/dev/null' && err "/usr writable while locked" || ok "/usr read-only while locked"
U "$A" 'test -e /usr/games/sl' && err "your packages are on while locked" || ok "your packages are off while locked"
U "$A" '/usr/bin/hearth-unlock --unlock --json >/dev/null' || err "--unlock failed"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && ok "unlocked again, right away" || err "unlock state: $s"
U "$A" 'test -x /usr/games/sl' && ok "your packages are back after unlocking" || err "packages not back after unlocking"

# ---- 7. a start with changes that never reached Hearth: the next one is a safe start
echo "--- safe start"
start "$A"                                  # pending=1, and Hearth "never comes up"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = suspended ] && ok "safe start after a start that never reached Hearth" || err "no safe start: $s"
[ "$(field "$s" "(d['notice'] or {}).get('kind')")" = safe-start ] && ok "Hearth gets a notice" || err "no notice: $s"
U "$A" 'test -e /usr/games/sl' && err "the safe start still had your changes" || ok "the safe start is the clean image"
U "$A" '/usr/bin/hearth-unlock --resume --json >/dev/null' || err "--resume failed"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && [ "$(field "$s" "d['notice']")" = None ] \
  && ok "turned back on, notice gone" || err "after --resume: $s"

# ---- 7b. an install cut off by a power loss: that start goes without the layer, then the install is finished
echo "--- interrupted dpkg"
U "$A" 'dpkg -s sl | sed "s/^Status: .*/Status: install ok half-configured/" > /usr/lib/sysimage/dpkg/updates/0001' \
  || err "couldn't fake an interrupted dpkg"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = repairing ] && ok "an interrupted dpkg: this start is without the layer" || err "no repair start after an interrupted dpkg: $s"
[ "$(field "$s" "(d['notice'] or {}).get('kind')")" = dpkg-interrupted ] && ok "Hearth gets the interrupted-install notice" || err "no interrupted-install notice: $s"
U "$A" 'test -e /usr/games/sl' && err "the half-written layer was mounted" || ok "the half-written layer stays off at start"
U "$A" '/usr/bin/hearth-unlock --after-start 2>&1 | tail -3' | sed 's/^/  /'
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && [ "$(field "$s" "d['repair']")" = False ] \
  && ok "the interrupted install finished after the start (dpkg --configure -a)" || err "repair didn't finish: $s"
U "$A" 'test -x /usr/games/sl && dpkg -s sl | grep -q "^Status: install ok installed"' && ok "the layer is back and consistent" || err "after the repair: sl isn't right"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && ok "the next start uses the layer again" || err "after the repair, the next start: $s"

# ---- 7c. the boot-menu writers, the update's finalizing and Recovery run from the image's /usr
echo "--- the image's /usr for the boot writers"
ic 'grep -q " /run/hearth/usr-image " /proc/self/mountinfo' && ok "the image's /usr is kept at /run/hearth/usr-image" || err "/run/hearth/usr-image isn't mounted"
ic 'test -e /usr/games/sl && test ! -e /run/hearth/usr-image/games/sl' && ok "/run/hearth/usr-image doesn't see the layer (private mount)" \
  || err "/run/hearth/usr-image shows the layer's files (or the layer isn't on)"
# A layer that breaks a tool the boot-menu writer uses: in a mount namespace like BindReadOnlyPaths= gives the units,
# the image's tool still works.
U "$A" 'cp -a /usr/bin/sed /tmp/sed.keep && printf "#!/bin/sh\nexit 3\n" > /usr/bin/sed && chmod 755 /usr/bin/sed'
U "$A" 'sed --version >/dev/null 2>&1' && err "the broken sed wasn't broken (test setup)"
U "$A" "unshare -m sh -c 'mount --bind /run/hearth/usr-image /usr && sed --version >/dev/null && python3 -c \"import json\" && test ! -e /usr/games/sl'" \
  && ok "with the image's /usr bound (as the units have it) the tools work despite a broken layer" || err "the image's /usr bound over /usr didn't give working tools"
U "$A" 'cp -a /tmp/sed.keep /usr/bin/sed'
if [ "$MODE" = image ]; then
  for u in hearth-bls-grub hearth-bls-grub-refresh hearth-boot-guard hearth-boot-guard-refresh hearth-boot-good ostree-finalize-staged bootc-fetch-apply-updates hearth-recovery; do
    ic "grep -q 'BindReadOnlyPaths=-/run/hearth/usr-image:/usr' /usr/lib/systemd/system/$u.service.d/20-hearth-image-usr.conf" || err "$u has no image-/usr drop-in"
  done
  ok "the boot writers' drop-ins are in the image"
  ic 'systemctl is-enabled hearth-unlock-ldcache.service >/dev/null' && ok "the library cache is rebuilt from the image before finalizing" || err "hearth-unlock-ldcache.service isn't enabled"

  # ---- 7d. /etc pointing into the layer: a display manager from a package, on a start without the layer
  echo "--- /etc guard"
  ic 'test -x /usr/lib/systemd/system-generators/hearth-etc-guard' && ok "the /etc guard generator is in the image" || err "no hearth-etc-guard generator"
  echo "::notice title=hearth-unlock sddm::$(ic 'grep -E "^Exec" /usr/lib/systemd/system/sddm.service | tr "\n" " "' | cut -c1-250)"
  ic 'rm -rf /tmp/g && mkdir -p /tmp/g/etc /tmp/g/early && ln -s /usr/lib/systemd/system/gdm3.service /tmp/g/etc/display-manager.service && echo /usr/sbin/gdm3 > /tmp/g/dm
      HEARTH_ETC_UNITS=/tmp/g/etc HEARTH_X11_DM=/tmp/g/dm /usr/lib/systemd/system-generators/hearth-etc-guard /tmp/g/n /tmp/g/early /tmp/g/l
      [ "$(readlink /tmp/g/early/display-manager.service)" = /usr/lib/systemd/system/sddm.service ] && [ -e /usr/lib/systemd/system/sddm.service ] && grep -q "^ExecStartPre=$" /tmp/g/early/sddm.service.d/50-hearth-etc-guard.conf' \
    && ok "a display manager that isn't there (gdm3 from the layer): Hearth's SDDM starts instead" || err "the /etc guard didn't bring back Hearth's display manager"
  ic 'systemctl is-enabled nimbus-autologin.service >/dev/null' && ok "Hearth's autologin is on (the session SDDM starts)" || err "nimbus-autologin isn't enabled"
  ic 'grep -A4 "^Name: shared/default-x-display-manager" /var/cache/debconf/config.dat | grep -q "^Value: sddm"' \
    && ok "debconf's display manager is SDDM (copied at the first start)" || err "debconf's shared/default-x-display-manager isn't sddm: $(ic 'ls -la /var/cache/debconf /usr/share/hearth/debconf 2>&1; grep -A4 "^Name: shared/default-x-display-manager" /usr/share/hearth/debconf/config.dat' | tr '\n' ' ' | cut -c1-400)"
  U "$A" 'apt-get install -y -qq xdm >/tmp/xdm.log 2>&1' || echo "::warning title=hearth-unlock::installing xdm failed: $(ic 'tail -2 /tmp/xdm.log' | tr '\n' ' ')"
  U "$A" '[ "$(cat /etc/X11/default-display-manager)" = /usr/bin/sddm ] && [ "$(readlink -f /etc/systemd/system/display-manager.service)" = /usr/lib/systemd/system/sddm.service ]' \
    && ok "installing another display manager (xdm) leaves Hearth's in place" || err "installing xdm took over the display manager: $(ic 'cat /etc/X11/default-display-manager; readlink /etc/systemd/system/display-manager.service' | tr '\n' ' ')"

  # ---- 7e. nothing from apt reaches /boot or swaps the system's core
  echo "--- /boot and the core"
  ic 'grep -q "not used on Hearth OS" /usr/sbin/update-initramfs && grep -q "not used on Hearth OS" /usr/sbin/update-grub' && ok "update-initramfs and update-grub are no-ops" || err "update-initramfs/update-grub aren't Hearth's no-ops"
  ic '[ -z "$(ls -A /etc/kernel/postinst.d 2>/dev/null)" ]' && ok "no kernel install hooks" || err "kernel install hooks left: $(ic 'ls /etc/kernel/postinst.d' | tr '\n' ' ')"
  ic '[ -z "$(ls -A /boot 2>/dev/null)" ]' && ok "/boot untouched by the installs" || err "something wrote /boot: $(ic 'ls -A /boot' | tr '\n' ' ')"
  for p in libc6 systemd udev libsystemd-shared dbus; do
    ic "apt-cache policy $p | awk '/Installed:/{i=\$2} /Candidate:/{c=\$2} END{exit !(i!=\"\" && i==c)}'" && ok "$p stays at the image's version" || err "$p could be changed by apt: $(ic "apt-cache policy $p | head -3" | tr '\n' ' ')"
  done
  echo "::notice title=hearth-unlock dracut triggers::$(ic 'cat /usr/lib/sysimage/dpkg/info/dracut*.triggers 2>/dev/null | tr "\n" " "' | cut -c1-250)"
fi

# ---- 8. the boot guard's own tests (in this system's Python)
ic 'cd /src/hosts/linux && python3 -B -m unittest discover -s test -p test_hearth_boot_guard.py 2>&1 | tail -3' | sed 's/^/  /'
ic 'cd /src/hosts/linux && python3 -B -m unittest discover -s test -p test_hearth_boot_guard.py >/dev/null 2>&1' \
  && ok "boot guard tests pass here" || err "boot guard tests failed in this system"

if [ "$fail" = 0 ]; then echo "::notice title=hearth-unlock ($MODE)::writable by default, apt survives restarts, update + reapply, rollback, lock/unlock, safe start, interrupted dpkg, image /usr for the boot writers$( [ "$MODE" = image ] && echo ", /etc guard, /boot and core pins"): all ok"; fi
exit "$fail"
