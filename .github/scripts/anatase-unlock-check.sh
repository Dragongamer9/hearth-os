#!/bin/bash
# Hearth OS on Anatase (1.15 test), "Unlock the file system" (os-bases/anatase/system_files/usr/bin/hearth-unlock): the
# real script, real overlay mounts and real dnf, in a privileged container of the built image, with starts played out
# the way hearth-unlock-check.sh does it for Debian. Run on the CI runner (it drives podman):
#   bash .github/scripts/anatase-unlock-check.sh IMAGE
# A start: /usr is the image's (read-only, as composefs has it on a device), then `hearth-unlock --boot` exactly as
# hearth-unlock.service runs it; a restart takes Hearth's layer off /usr again and starts over. HEARTH_UNLOCK_ID plays
# the OS version (an ostree commit): A, then an update to B, then a rollback to A. /var/lib/hearth and /etc/hearth are
# folders on the runner, kept between starts like on a device. (The record is taken by hearth-unlock-record.path on a
# device whenever rpm's database changes; here --record is run where that unit would run it.)
# Checks: locked by default · the switch unlocks now · dnf install (and an rpm file through Hearth) works and survives
# a restart · an update starts clean and the reapply puts the dnf packages back · files copied into /usr by hand stay
# with the old version only (as Settings says) · the previous version keeps its own layer · lock and unlock · a safe
# start after a start that never reached Hearth · the boot chain can't come from dnf · the boot writers' drop-ins.
# Failures are ::error annotations and exit 1; a summary is a ::notice.
set -u
IMG=${1:?image}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$(mktemp -d)
A=$(printf 'a%.0s' $(seq 64))
B=$(printf 'b%.0s' $(seq 64))
NAME=anatase-unlock-check-$$
fail=0
err() { echo "::error title=anatase unlock::$*"; fail=1; }
ok() { echo "ok: $*"; }
mkdir -p "$WORK/state" "$WORK/etc-hearth"

sudo podman run -d --name "$NAME" --privileged --entrypoint sleep \
  -v "$ROOT:/src:ro" -v "$WORK/state:/var/lib/hearth" -v "$WORK/etc-hearth:/etc/hearth" "$IMG" infinity >/dev/null \
  || { echo "::error title=anatase unlock::couldn't start the container"; exit 1; }
trap 'sudo podman rm -f "$NAME" >/dev/null 2>&1; sudo rm -rf "$WORK"' EXIT
x() { sudo podman exec -e LC_ALL=C "$@"; }
ic() { x "$NAME" bash -c "$1"; }

for f in /usr/bin/hearth-unlock /etc/dnf/libdnf5.conf.d/20-hearth-boot-chain.conf /usr/share/hearth/rpm-baseline.list \
         /usr/lib/systemd/system/hearth-unlock.service /usr/lib/systemd/system/hearth-unlock-record.path; do
  ic "test -e $f" && ok "in the image: $f" || err "missing in the image: $f"
done
ic 'systemctl is-enabled hearth-unlock.service hearth-unlock-reapply.service hearth-unlock-good.service hearth-unlock-units.service hearth-unlock-record.path >/dev/null' \
  && ok "units enabled" || err "the hearth-unlock units aren't all enabled"
n=$(ic "diff <(rpm -qa --qf '%{NAME}\n' | grep -vx gpg-pubkey | sort -u) /usr/share/hearth/rpm-baseline.list | grep -c '^[<>]'")
[ "${n:-1}" = 0 ] && ok "the rpm baseline matches the image's packages" || err "rpm -qa differs from the baseline ($n lines)"
ic 'mkdir -p /var/tmp /var/cache/libdnf5 /var/lib/dnf; touch /run/ostree-booted; mount --bind /usr /usr && mount -o remount,bind,ro /usr' || err "couldn't make /usr read-only"
ic 'touch /usr/.rw-test 2>/dev/null' && err "/usr is writable before hearth-unlock (the test setup is wrong)"

LAST=""
start() {   # start VERSION: a restart, then hearth-unlock.service
  ic 'while grep -q " /usr .* - overlay hearth-usr " /proc/self/mountinfo; do umount -l /usr || break; done'
  LAST=$(x -e HEARTH_UNLOCK_ID="$1" "$NAME" /usr/bin/hearth-unlock --boot 2>&1)
  echo "$LAST" | sed 's/^/  /'
}
st() { x -e HEARTH_UNLOCK_ID="$1" "$NAME" /usr/bin/hearth-unlock --status --json 2>/dev/null | tail -n1; }
field() { python3 -c "import json,sys; d=json.loads(sys.argv[1]); print(eval(sys.argv[2], {}, {'d': d}))" "$1" "$2" 2>/dev/null; }
U() { x -e HEARTH_UNLOCK_ID="$1" "$NAME" bash -c "$2"; }
diag() {
  echo "::error title=anatase unlock diag::boot said: $(echo "$LAST" | tr '\n' ' ' | cut -c1-400)"
  echo "::error title=anatase unlock diag::status: $(st "$1" | cut -c1-600)"
  echo "::error title=anatase unlock diag::mounts: $(ic 'grep " /usr " /proc/self/mountinfo | cut -d" " -f5,9- | cut -c1-200' | tr '\n' '|')"
}

# ---- 1. first start: locked (the default on Anatase)
echo "--- start 1 (version A)"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = locked ] && ok "locked by default" || err "not locked by default: $s"
[ "$(field "$s" "d['tool']")" = dnf ] && ok "status says dnf" || err "status has no tool=dnf: $s"
U "$A" 'touch /usr/lib/hearth-rw-test 2>/dev/null' && err "/usr writable while locked" || ok "/usr read-only while locked"
# ---- the switch (Settings › Updates & Recovery: hearth-admin unlock on → hearth-unlock --unlock)
U "$A" '/usr/bin/hearth-unlock --unlock --json >/dev/null' || err "--unlock failed"
ic 'test -f /etc/hearth/system-unlocked' && ok "the switch is remembered (/etc/hearth/system-unlocked)" || err "no /etc/hearth/system-unlocked after --unlock"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && ok "unlocked right away" || { err "not unlocked: $s"; diag "$A"; }
U "$A" 'touch /usr/lib/hearth-rw-test && rm /usr/lib/hearth-rw-test' && ok "/usr writable" || err "/usr isn't writable after --unlock"
U "$A" 'dnf5 -y install --setopt=install_weak_deps=False sl figlet >/tmp/dnf.log 2>&1' && ok "dnf5 install sl figlet (into /usr)" \
  || err "dnf5 install failed: $(ic 'tail -4 /tmp/dnf.log' | tr '\n' ' ' | cut -c1-500)"
U "$A" 'rm -rf /tmp/rpm && mkdir -p /tmp/rpm && cd /tmp/rpm && dnf5 -q download cowsay >/tmp/dl.log 2>&1 && ls /tmp/rpm/*.rpm >/dev/null' \
  || err "couldn't download cowsay's rpm for the file test: $(ic 'tail -2 /tmp/dl.log' | tr '\n' ' ')"
U "$A" '/usr/bin/hearth-unlock --install-rpm "$(ls /tmp/rpm/cowsay-*.rpm | head -1)" >/tmp/irpm.log 2>&1' && ok "an rpm file installed through Hearth (--install-rpm)" \
  || err "--install-rpm failed: $(ic 'tail -3 /tmp/irpm.log' | tr '\n' ' ' | cut -c1-400)"
U "$A" 'printf "#!/bin/sh\necho hand\n" > /usr/bin/hearth-by-hand && chmod 755 /usr/bin/hearth-by-hand' || err "couldn't copy a file into /usr by hand"
U "$A" '/usr/bin/hearth-unlock --record'
s=$(st "$A"); got=$(field "$s" "' '.join(d['packages'])")
for p in sl figlet cowsay; do case " $got " in *" $p "*) ok "recorded: $p";; *) err "the record has no $p (it has: $got)";; esac; done
ic 'ls /var/lib/hearth/unlock/rpms/cowsay.rpm >/dev/null' && ok "a copy of the rpm file kept for updates" || err "the rpm file wasn't kept"
case " $got " in *" bash "*|*" dnf5 "*|*" kernel "*) err "image packages ended up in the record: $got";; esac
for p in kernel-devel grub2-tools-extra shim-ia32; do
  U "$A" "dnf5 install -y --assumeno $p >/tmp/bc.log 2>&1; grep -qiE 'excluded|No match' /tmp/bc.log" \
    && ok "the boot chain can't come from dnf: $p" || err "$p could be installed with dnf (the boot-chain exclude isn't working): $(ic 'tail -2 /tmp/bc.log' | tr '\n' ' ')"
done

# ---- 2. restart, same version
echo "--- start 2 (version A again)"
start "$A"
U "$A" 'command -v sl >/dev/null && command -v figlet >/dev/null && command -v cowsay >/dev/null && rpm -q sl >/dev/null && test -x /usr/bin/hearth-by-hand' \
  && ok "survives a restart (dnf's packages, rpm's database, a file copied by hand)" || { err "the changes didn't survive a restart"; diag "$A"; }
ic "python3 - <<EOF
import json; p='/var/lib/hearth/unlock/layers/$A/layer.json'; d=json.load(open(p)); d['pending']=0; json.dump(d,open(p,'w'))
EOF"

# ---- 3. an update: version B starts clean, then the reapply
echo "--- start 3 (update to version B)"
start "$B"
U "$B" 'command -v sl >/dev/null' && err "the new version started with the old layer" || ok "the new version starts clean"
s=$(st "$B"); [ "$(field "$s" "d['reapply'].get('state')")" = pending ] && ok "reapply pending on the new version" || err "no reapply pending: $s"
U "$B" '/usr/bin/hearth-unlock --after-start 2>&1 | tail -5' | sed 's/^/  /'
s=$(st "$B"); r=$(field "$s" "d['reapply'].get('state')")
[ "$r" = done ] && ok "reapply done" || err "reapply: $r ($(ic 'tail -5 /var/lib/hearth/unlock/reapply.log' | tr '\n' ' ' | cut -c1-500))"
U "$B" 'command -v sl >/dev/null && command -v figlet >/dev/null && command -v cowsay >/dev/null' \
  && ok "packages back on the new version (repository and saved rpm file)" || err "packages missing after the reapply"
U "$B" 'test -e /usr/bin/hearth-by-hand' && err "a file copied by hand came along (Settings says it doesn't)" \
  || ok "a file copied into /usr by hand stays with the old version only, as Settings says"
echo "--- start 4 (version B again)"
start "$B"
U "$B" 'command -v figlet >/dev/null' && ok "the reapplied packages survive a restart" || err "reapplied packages gone after a restart"

# ---- 5. rollback: version A as it was
echo "--- start 5 (rollback to version A)"
start "$A"
U "$A" 'rpm -q sl figlet >/dev/null && test -x /usr/bin/hearth-by-hand' && ok "the previous version keeps its own layer, untouched" \
  || { err "the previous version lost its changes"; diag "$A"; }

# ---- 6. lock and unlock
echo "--- lock"
U "$A" '/usr/bin/hearth-unlock --lock --json >/dev/null' || err "--lock failed"
ic 'test ! -e /etc/hearth/system-unlocked' && ok "locking takes the switch's file away" || err "/etc/hearth/system-unlocked still there after --lock"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = locks-at-restart ] && ok "lock: from the next restart" || err "lock state: $s"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = locked ] && ok "locked after a restart" || err "not locked after a restart: $s"
U "$A" 'command -v sl >/dev/null' && err "your packages are on while locked" || ok "your packages are off while locked"
U "$A" '/usr/bin/hearth-unlock --unlock --json >/dev/null' || err "--unlock failed"
U "$A" 'command -v sl >/dev/null' && ok "your packages are back after unlocking" || err "packages not back after unlocking"

# ---- 7. safe start
echo "--- safe start"
start "$A"
start "$A"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = suspended ] && ok "safe start after a start that never reached Hearth" || err "no safe start: $s"
U "$A" 'command -v sl >/dev/null' && err "the safe start still had your changes" || ok "the safe start is the clean image"
U "$A" '/usr/bin/hearth-unlock --resume --json >/dev/null' || err "--resume failed"
s=$(st "$A"); [ "$(field "$s" "d['state']")" = unlocked ] && ok "turned back on" || err "after --resume: $s"

# ---- 8. the boot writers run from the image's /usr
ic 'grep -q " /run/hearth/usr-image " /proc/self/mountinfo' && ok "the image's /usr is kept at /run/hearth/usr-image" || err "/run/hearth/usr-image isn't mounted"
for u in hearth-bls-grub hearth-boot-guard hearth-boot-good ostree-finalize-staged bootc-fetch-apply-updates hearth-recovery; do
  ic "grep -q 'BindReadOnlyPaths=-/run/hearth/usr-image:/usr' /usr/lib/systemd/system/$u.service.d/20-hearth-image-usr.conf" || err "$u has no image-/usr drop-in"
done
ok "checked the boot writers' drop-ins"

if [ "$fail" = 0 ]; then
  echo "::notice title=anatase unlock::locked by default, the switch unlocks now, dnf + rpm file into /usr, survives restarts, update: dnf packages put back and hand-copied files stay behind, rollback keeps its layer, lock/unlock, safe start, boot chain excluded from dnf: all ok"
fi
exit "$fail"
