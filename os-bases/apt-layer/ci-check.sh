#!/bin/bash
# CI (base-ubuntu.yml, base-peppermint.yml): proves the added-apps layer works in the built image, inside a
# privileged container (podman run --privileged, with a real directory mounted on /var/lib/hearth: an overlay's
# upper layer can't be on the container's own overlay). What it does, as on a booted system:
#   1. mounts the layer on the image's /usr (hearth-apt-layer mount, the boot service's command)
#   2. apt install of a small package works, and it's on the list
#   3. the guard rails: removing or reinstalling a system package, and a boot package, are refused; apt
#      full-upgrade wouldn't touch system packages
#   4. a local .deb through hearth-admin install-deb is installed, kept, and its app is in added-apps
#   5. a simulated system update (a new deployment: an empty layer) and the put-back: the package comes back,
#      a package that no longer fits is skipped and reported, the system's own packages are untouched
# Not covered here (no booted machine): systemd ordering at boot, the deployment key from the real ostree= boot
# option, notifications, polkit's password prompt, the .deb double-click.
set -u
fail=0
ok() { echo "ok: $1"; }
bad() { echo "::error title=added-apps layer::$1"; fail=1; }
L=/usr/libexec/nimbus/hearth-apt-layer
export DEBIAN_FRONTEND=noninteractive

# What tmpfiles does at boot (podman run doesn't): /var/lib/dpkg -> /usr/lib/sysimage/dpkg, apt's folders.
mkdir -p /var/lib && ln -sfn /usr/lib/sysimage/dpkg /var/lib/dpkg
systemd-tmpfiles --create /usr/lib/tmpfiles.d/hearth-apt-layer.conf >/dev/null 2>&1 || true
mkdir -p /var/lib/apt/lists /var/cache/apt/archives /var/log/apt /var/cache/debconf

echo "--- image side"
[ -s /usr/share/hearth/apt-layer/image-packages ] && ok "the image's package list" || bad "no /usr/share/hearth/apt-layer/image-packages"
[ -L /etc/apt/preferences.d/zz-hearth-image ] && grep -q 'Pin-Priority: 1001' /etc/apt/preferences.d/zz-hearth-image && ok "system packages are pinned" || bad "the image pins are missing"
grep -q hearth-apt-layer /etc/apt/apt.conf.d/80-hearth-apt-layer && grep -q 'hearth-apt-layer record' /etc/dpkg/dpkg.cfg.d/hearth-apt-layer && ok "apt's and dpkg's hooks" || bad "apt/dpkg hooks are missing"
[ -L /etc/systemd/system/sysinit.target.wants/hearth-apt-layer.service ] && [ -L /etc/systemd/system/multi-user.target.wants/hearth-apt-layer-sync.service ] && ok "the services are enabled" || bad "the services aren't enabled"
grep -q 'application/vnd.debian.binary-package=hearth-install-deb.desktop' /usr/share/applications/mimeapps.list && ok ".deb files open with Hearth's installer" || bad "no .deb handler"
systemd-analyze verify /usr/lib/systemd/system/hearth-apt-layer.service /usr/lib/systemd/system/hearth-apt-layer-sync.service 2>&1 | grep -v -e 'ostree-remount' -e 'Failed to prepare filename' -e 'not found' | grep -i 'error\|invalid\|unknown' && echo "::warning title=added-apps layer::systemd-analyze verify complained (above)" || ok "systemd accepts the units"

echo "--- 1. mount"
HEARTH_APT_LAYER_KEY=ci-deployment-a $L mount || bad "hearth-apt-layer mount failed"
[ "$(findmnt -n -o FSTYPE /usr)" = overlay ] && ok "/usr has the layer on it" || bad "/usr isn't an overlay"
if touch /usr/.hearth-write-test 2>/dev/null; then rm -f /usr/.hearth-write-test; ok "/usr is writable"; else bad "/usr is still read-only"; fi
[ -s /run/hearth/apt-layer/active.json ] && ok "the hooks see the layer as active" || bad "no active.json"

echo "--- 2. apt install"
apt-get update -q >/tmp/update.log 2>&1 || { tail -5 /tmp/update.log; bad "apt-get update failed"; }
if apt-get install -y hello >/tmp/i.log 2>&1 && hello | grep -q 'Hello, world'; then ok "apt install hello works"; else tail -15 /tmp/i.log; bad "apt install hello failed"; fi
grep -q 'kept through restarts' /tmp/i.log && ok "apt says it's kept" || echo "::warning title=added-apps layer::apt didn't print Hearth's line"
$L status --json | grep -q '"hello"' && ok "hello is on the list" || bad "hello isn't on the list"
[ -s /var/lib/hearth/layer/usr/ci-deployment-a/upper/bin/hello ] && ok "its files are in the layer (not the image)" || bad "hello isn't in the layer's upper dir"

echo "--- 3. guard rails"
out=$(apt-get remove -y nano 2>&1); if dpkg -s nano >/dev/null 2>&1 && echo "$out" | grep -q 'Hearth:.*part of Hearth OS'; then ok "removing a system package is refused"; else echo "$out" | tail -5; bad "removing nano wasn't refused"; fi
out=$(apt-get install -y --reinstall nano 2>&1); if echo "$out" | grep -q "Hearth:.*can't be reinstalled"; then ok "reinstalling a system package is refused"; else echo "$out" | tail -5; bad "reinstalling nano wasn't refused"; fi
[ ! -e /var/lib/hearth/layer/usr/ci-deployment-a/upper/bin/nano ] && ok "nano isn't in the layer" || bad "nano's files went into the layer"
boot=""; for p in dracut-network dracut-squash dracut-live linux-image-virtual linux-image-cloud-amd64; do
  if apt-cache show "$p" >/dev/null 2>&1 && ! dpkg -s "$p" >/dev/null 2>&1; then boot=$p; break; fi
done
if [ -n "$boot" ]; then
  out=$(apt-get install -y "$boot" 2>&1); if ! dpkg -s "$boot" >/dev/null 2>&1 && echo "$out" | grep -q 'Hearth:.*kernel, initramfs and boot loader'; then ok "a boot package ($boot) is refused"; else echo "$out" | tail -5; bad "$boot wasn't refused"; fi
else echo "::warning title=added-apps layer::no boot package to test the refusal with"; fi
n=$(apt-get -s full-upgrade 2>/dev/null | grep -c '^Inst' || true)
[ "$n" = 0 ] && ok "apt full-upgrade wouldn't upgrade system packages" || { apt-get -s full-upgrade | grep '^Inst' | head -5; bad "apt full-upgrade would change $n packages"; }

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

echo "--- 5. a system update (new deployment) and the put-back"
# The kept .deb now needs something this system doesn't have: what a package that no longer fits looks like.
sed -i 's/^Version: 1.0/Version: 1.1\nDepends: hearth-ci-missing (>= 9)/' "$d/DEBIAN/control"
dpkg-deb --build "$d" /var/lib/hearth/layer/debs/hearth-ci-test.deb >/dev/null
umount /usr 2>/dev/null || umount -l /usr
HEARTH_APT_LAYER_KEY=ci-deployment-b $L mount || bad "mounting the new deployment's layer failed"
if ! dpkg -s hello >/dev/null 2>&1 && [ ! -e /usr/bin/hello ]; then ok "the new deployment starts with an empty layer"; else bad "files from the old layer leaked into the new one"; fi
grep -qx ci-deployment-b /var/lib/hearth/layer/pending && ok "the put-back is pending" || bad "no pending put-back"
[ ! -s /var/lib/hearth/layer/added-apps ] && ok "added-apps is empty until the apps are back" || bad "added-apps lists apps that aren't installed"
HEARTH_APT_LAYER_KEY=ci-deployment-b $L reapply >/tmp/re.log 2>&1; cat /tmp/re.log | tail -8
dpkg -s hello >/dev/null 2>&1 && hello | grep -q 'Hello, world' && ok "hello is back after the update" || bad "hello wasn't put back"
python3 - <<'EOF' && ok "the package that no longer fits is skipped and reported" || bad "status.json doesn't report the skipped package"
import json, sys
s = json.load(open('/var/lib/hearth/layer/status.json'))
print(s)
sys.exit(0 if 'hello' in s['reinstalled'] and [k['name'] for k in s['skipped']] == ['hearth-ci-test'] and 'hearth-ci-missing' in s['skipped'][0]['reason'] else 1)
EOF
[ ! -e /var/lib/hearth/layer/pending ] && ok "nothing pending afterwards" || bad "still pending"
$L status --json | grep -q '"hearth-ci-test"' && ok "the skipped package stays on the list (retry later)" || bad "the skipped package fell off the list"
dpkg -s nano >/dev/null 2>&1 && [ -z "$(dpkg --audit 2>&1)" ] && ok "the system's packages are intact (dpkg --audit is clean)" || bad "dpkg --audit: $(dpkg --audit 2>&1 | head -3)"
# The image's own dpkg database (the lower layer) never changed.
grep -q '^Package: hello$' /usr/share/hearth/apt-layer/image-packages && bad "hello is in the image list" || ok "the image's package list is untouched"
apt-get remove -y hello >/dev/null 2>&1; $L status --json | grep -q '"hello"' && bad "removing hello left it on the list" || ok "removing a package takes it off the list"
umount /usr 2>/dev/null || umount -l /usr
exit $fail
