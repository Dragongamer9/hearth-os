#!/bin/bash
# CI (base-peppermint.yml): proves the writable layer works in the built image, inside a privileged container
# (podman run --privileged, with a real directory mounted on /var/lib/hearth: an overlay's upper layer can't be
# on the container's own overlay). What it does, as on a booted system:
#   1. mounts the layer on the image's /usr (hearth-apt-layer mount, the boot service's command)
#   2. apt install of a small package works, and it's recorded
#   3. removing an image package works and is recorded; the guard rails refuse removing what Hearth's tools need
#      and installing a boot package; apt full-upgrade works (and upgrades are recorded if there are any)
#   4. a local .deb through hearth-admin install-deb is installed and kept, its app is in added-apps; a package
#      that installs into /opt works; /opt and /usr/local are writable
#   5. a simulated system update (a new deployment: an empty layer) and the put-back: the package comes back,
#      the removed image package is removed again, a package that no longer fits is skipped and reported
# Not covered here (no booted machine): systemd ordering at boot, the deployment key from the real ostree= boot
# option, notifications, polkit's password prompt, the .deb double-click.
set -u
fail=0
passed=0
ok() { echo "ok: $1"; passed=$((passed + 1)); }
# Installed (not just its configuration files left, which `dpkg -s` also answers for).
installed() { [ "$(dpkg-query -W -f='${db:Status-Status}' "$1" 2>/dev/null)" = installed ]; }
bad() { echo "::error title=writable layer::$1"; fail=1; }
L=/usr/libexec/nimbus/hearth-apt-layer
export DEBIAN_FRONTEND=noninteractive

# What tmpfiles does at boot (podman run doesn't): /var/lib/dpkg -> /usr/lib/sysimage/dpkg, apt's folders.
mkdir -p /var/lib && ln -sfn /usr/lib/sysimage/dpkg /var/lib/dpkg
systemd-tmpfiles --create /usr/lib/tmpfiles.d/hearth-apt-layer.conf >/dev/null 2>&1 || true
mkdir -p /var/lib/apt/lists /var/cache/apt/archives /var/log/apt /var/cache/debconf

echo "--- image side"
[ -s /usr/share/hearth/apt-layer/image-packages ] && ok "the image's package list" || bad "no /usr/share/hearth/apt-layer/image-packages"
[ -L /etc/apt/preferences.d/zz-hearth-image ] && grep -q '^Package: linux-image' /etc/apt/preferences.d/zz-hearth-image && ok "the kernel and boot packages are pinned" || bad "the kernel pins are missing"
grep -q hearth-apt-layer /etc/apt/apt.conf.d/80-hearth-apt-layer && grep -q 'hearth-apt-layer record' /etc/dpkg/dpkg.cfg.d/hearth-apt-layer && ok "apt's and dpkg's hooks" || bad "apt/dpkg hooks are missing"
[ -L /etc/systemd/system/sysinit.target.wants/hearth-apt-layer.service ] && [ -L /etc/systemd/system/multi-user.target.wants/hearth-apt-layer-sync.service ] && ok "the services are enabled" || bad "the services aren't enabled"
grep -q 'application/vnd.debian.binary-package=hearth-install-deb.desktop' /usr/share/applications/mimeapps.list && ok ".deb files open with Hearth's installer" || bad "no .deb handler"
systemd-analyze verify /usr/lib/systemd/system/hearth-apt-layer.service /usr/lib/systemd/system/hearth-apt-layer-sync.service 2>&1 | grep -v -e 'ostree-remount' -e 'Failed to prepare filename' -e 'not found' | grep -i 'error\|invalid\|unknown' && echo "::warning title=writable layer::systemd-analyze verify complained (above)" || ok "systemd accepts the units"

echo "--- 1. mount"
HEARTH_APT_LAYER_KEY=ci-deployment-a $L mount || bad "hearth-apt-layer mount failed"
[ "$(findmnt -n -o FSTYPE /usr)" = overlay ] && ok "/usr has the layer on it" || bad "/usr isn't an overlay"
if touch /usr/.hearth-write-test 2>/dev/null; then rm -f /usr/.hearth-write-test; ok "/usr is writable"; else bad "/usr is still read-only"; fi
[ -s /run/hearth/apt-layer/active.json ] && ok "the hooks see the layer as active" || bad "no active.json"

echo "--- 2. apt install"
apt-get update -q >/tmp/update.log 2>&1 || { tail -5 /tmp/update.log; bad "apt-get update failed"; }
if apt-get install -y hello >/tmp/i.log 2>&1 && hello | grep -q 'Hello, world'; then ok "apt install hello works"; else tail -15 /tmp/i.log; bad "apt install hello failed"; fi
grep -q 'kept through restarts' /tmp/i.log && ok "apt says it's kept" || echo "::warning title=writable layer::apt didn't print Hearth's line"
$L status --json | grep -q '"hello"' && ok "hello is on the list" || bad "hello isn't on the list"
[ -s /var/lib/hearth/layer/usr/ci-deployment-a/upper/bin/hello ] && ok "its files are in the layer (not the image)" || bad "hello isn't in the layer's upper dir"

echo "--- 3. removing and upgrading image packages; guard rails"
listed() { $L status --json | python3 -c "import json,sys; sys.exit(0 if sys.argv[2] in json.load(sys.stdin)[sys.argv[1]] else 1)" "$1" "$2"; }
if apt-get remove -y nano >/tmp/r.log 2>&1 && ! installed nano; then ok "removing an image package (nano) works"; else tail -5 /tmp/r.log; bad "removing nano failed"; fi
listed removed nano && ok "the removal is recorded" || bad "nano isn't recorded as removed"
out=$(apt-get remove -y python3 2>&1); if installed python3 && echo "$out" | grep -q "Hearth:.*python3 stays"; then ok "removing what Hearth's tools need (python3) is refused"; else echo "$out" | tail -5; bad "removing python3 wasn't refused"; fi
[ -s /usr/share/hearth/apt-layer/protected ] && grep -qx apt /usr/share/hearth/apt-layer/protected && ok "the protected list is in the image" || bad "no protected list"
boot=""; for p in dracut-network dracut-squash dracut-live linux-image-cloud-amd64 linux-image-rt-amd64 linux-image-virtual; do
  if apt-cache show "$p" >/dev/null 2>&1 && ! installed "$p"; then boot=$p; break; fi
done
if [ -n "$boot" ]; then
  out=$(apt-get install -y "$boot" 2>&1); if ! installed "$boot" && echo "$out" | grep -q 'Hearth:.*kernel, initramfs and boot loader'; then ok "a boot package ($boot) is refused"; else echo "$out" | tail -5; bad "$boot wasn't refused"; fi
else echo "::warning title=writable layer::no boot package to test the refusal with"; fi
if apt-get -s full-upgrade >/tmp/fu.log 2>&1; then ok "apt full-upgrade can run"; else tail -5 /tmp/fu.log; bad "apt-get -s full-upgrade failed"; fi
grep -E '^Inst (linux-image|grub|shim|dracut)' /tmp/fu.log && bad "full-upgrade would change kernel or boot packages" || ok "full-upgrade passes the kernel and boot loader over (pinned)"
up=$(grep '^Inst ' /tmp/fu.log | awk '{print $2}' | while read -r n; do grep -q "^$n	" /usr/share/hearth/apt-layer/image-packages && echo "$n"; done | head -1)
if [ -n "$up" ]; then
  if apt-get install -y --only-upgrade "$up" >/tmp/u.log 2>&1 && listed upgraded "$up"; then ok "upgrading an image package ($up) works and is recorded"; else tail -5 /tmp/u.log; bad "upgrading $up failed or wasn't recorded"; fi
else echo "::notice title=writable layer::no image package has a newer version in the archive right now: upgrades are covered by the unit tests only"; fi

echo "--- 4. a local .deb through hearth-admin install-deb"
d=/tmp/deb/hearth-ci-test; mkdir -p "$d/DEBIAN" "$d/usr/share/applications" "$d/usr/bin"
printf 'Package: hearth-ci-test\nVersion: 1.0\nArchitecture: all\nMaintainer: Hearth CI <ci@example.invalid>\nDescription: Hearth CI test\n' > "$d/DEBIAN/control"
printf '[Desktop Entry]\nType=Application\nName=Hearth CI test\nExec=hearth-ci-test\n' > "$d/usr/share/applications/hearth-ci-test.desktop"
printf '#!/bin/sh\necho hearth-ci-test ok\n' > "$d/usr/bin/hearth-ci-test"; chmod 755 "$d/usr/bin/hearth-ci-test"
dpkg-deb --build "$d" /tmp/hearth-ci-test.deb >/dev/null
if PKEXEC_UID=0 /usr/libexec/nimbus/hearth-admin install-deb /tmp/hearth-ci-test.deb >/tmp/deb.log 2>&1 && hearth-ci-test | grep -q ok; then ok "hearth-admin install-deb installs a .deb"; else tail -10 /tmp/deb.log; bad "hearth-admin install-deb failed"; fi
[ -s /var/lib/hearth/layer/debs/hearth-ci-test.deb ] && ok "a copy of the .deb is kept" || bad "the .deb wasn't kept"
grep -qx hearth-ci-test.desktop /var/lib/hearth/layer/added-apps && ok "its app is in added-apps" || bad "added-apps doesn't list hearth-ci-test.desktop"
out=$(PKEXEC_UID=0 /usr/libexec/nimbus/hearth-admin install-deb /etc/passwd 2>&1); echo "$out" | grep -q '^error:' && ok "install-deb refuses a file that isn't a .deb" || bad "install-deb took /etc/passwd"
# /opt and /usr/local: links into /var in the bootc layout (made at boot by tmpfiles), writable, and a package
# that installs into /opt works.
systemd-tmpfiles --create /usr/lib/tmpfiles.d/bootc-base-dirs.conf /usr/lib/tmpfiles.d/hearth-var.conf >/dev/null 2>&1 || true
mkdir -p /var/opt /var/usrlocal
touch /opt/.hearth-w /usr/local/.hearth-w 2>/dev/null && rm -f /opt/.hearth-w /usr/local/.hearth-w && ok "/opt and /usr/local are writable" || bad "/opt or /usr/local isn't writable ($(readlink /opt), $(readlink /usr/local))"
o=/tmp/deb/hearth-ci-opt; mkdir -p "$o/DEBIAN" "$o/opt/hearth-ci-opt"
printf 'Package: hearth-ci-opt\nVersion: 1.0\nArchitecture: all\nMaintainer: Hearth CI <ci@example.invalid>\nDescription: Hearth CI test (/opt)\n' > "$o/DEBIAN/control"
echo opt-ok > "$o/opt/hearth-ci-opt/check.txt"
dpkg-deb --build "$o" /tmp/hearth-ci-opt.deb >/dev/null
if apt-get install -y /tmp/hearth-ci-opt.deb >/tmp/o.log 2>&1 && grep -q opt-ok /opt/hearth-ci-opt/check.txt; then ok "a package that installs into /opt works"; else tail -8 /tmp/o.log; bad "installing into /opt failed"; fi

echo "--- 5. a system update (new deployment) and the put-back"
# The kept .deb now needs something this system doesn't have: what a package that no longer fits looks like.
sed -i 's/^Version: 1.0/Version: 1.1\nDepends: hearth-ci-missing (>= 9)/' "$d/DEBIAN/control"
dpkg-deb --build "$d" /var/lib/hearth/layer/debs/hearth-ci-test.deb >/dev/null
umount /usr 2>/dev/null || umount -l /usr
HEARTH_APT_LAYER_KEY=ci-deployment-b $L mount || bad "mounting the new deployment's layer failed"
if ! installed hello && [ ! -e /usr/bin/hello ] && installed nano; then ok "the new deployment starts with an empty layer (the image's nano is back)"; else bad "files from the old layer leaked into the new one"; fi
grep -qx ci-deployment-b /var/lib/hearth/layer/pending && ok "the put-back is pending" || bad "no pending put-back"
[ ! -s /var/lib/hearth/layer/added-apps ] && ok "added-apps is empty until the apps are back" || bad "added-apps lists apps that aren't installed"
HEARTH_APT_LAYER_KEY=ci-deployment-b $L reapply >/tmp/re.log 2>&1; cat /tmp/re.log | tail -8
installed hello && hello | grep -q 'Hello, world' && ok "hello is back after the update" || bad "hello wasn't put back"
python3 - <<'EOF' && ok "the package that no longer fits is skipped and reported" || bad "status.json doesn't report the skipped package"
import json, sys
s = json.load(open('/var/lib/hearth/layer/status.json'))
print(s)
sys.exit(0 if 'hello' in s['reinstalled'] and 'nano' in s['reremoved'] and [k['name'] for k in s['skipped']] == ['hearth-ci-test'] and 'hearth-ci-missing' in s['skipped'][0]['reason'] else 1)
EOF
[ ! -e /var/lib/hearth/layer/pending ] && ok "nothing pending afterwards" || bad "still pending"
$L status --json | grep -q '"hearth-ci-test"' && ok "the skipped package stays on the list (retry later)" || bad "the skipped package fell off the list"
! installed nano && ok "nano is removed again on the new version" || bad "nano wasn't removed again"
installed hearth-ci-opt && grep -q opt-ok /opt/hearth-ci-opt/check.txt && ok "the /opt package is back" || bad "the /opt package wasn't put back"
[ -z "$(dpkg --audit 2>&1)" ] && ok "dpkg --audit is clean" || bad "dpkg --audit: $(dpkg --audit 2>&1 | head -3)"
# The image's own dpkg database (the lower layer) never changed.
grep -q '^Package: hello$' /usr/share/hearth/apt-layer/image-packages && bad "hello is in the image list" || ok "the image's package list is untouched"
apt-get remove -y hello >/dev/null 2>&1; listed packages hello && bad "removing hello left it on the list" || ok "removing a package takes it off the list"
apt-get install -y nano >/dev/null 2>&1; listed removed nano && bad "reinstalling nano left it on the removed list" || ok "reinstalling a removed image package takes it off the list"
umount /usr 2>/dev/null || umount -l /usr
echo "::notice title=writable layer::$passed checks passed$([ "$fail" = 0 ] || echo ', some failed (see the errors)')"
exit $fail
