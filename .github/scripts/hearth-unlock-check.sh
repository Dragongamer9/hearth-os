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
      ln -sfn /usr/lib/sysimage/dpkg /var/lib/dpkg'
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
ic 'apt-get install -s -y grub-pc >/dev/null 2>&1' && err "GRUB could be installed with apt (the boot-chain pin isn't working)" || ok "the boot chain can't come from apt"

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

# ---- 8. the boot guard's own tests (in this system's Python)
ic 'cd /src/hosts/linux && python3 -B -m unittest discover -s test -p test_hearth_boot_guard.py 2>&1 | tail -3' | sed 's/^/  /'
ic 'cd /src/hosts/linux && python3 -B -m unittest discover -s test -p test_hearth_boot_guard.py >/dev/null 2>&1' \
  && ok "boot guard tests pass here" || err "boot guard tests failed in this system"

if [ "$fail" = 0 ]; then echo "::notice title=hearth-unlock ($MODE)::writable by default, apt survives restarts, update + reapply, rollback, lock/unlock, safe start: all ok"; fi
exit "$fail"
