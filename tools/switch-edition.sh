#!/bin/sh
# Switch edition on a Hearth OS install whose image has no podman (Peppermint, Ubuntu, CachyOS before 0.3.1):
# podman's packages are unpacked into /run (nothing installed, gone at the next restart) for bootc, then the same
# switch as Settings › Switch edition. Usage: curl -sL <this file> | sh -s ubuntu
set -e
ed="${1:-ubuntu}"
d=/run/user/$(id -u)/hearth-pm
mkdir -p "$d" && cd "$d"
echo "Getting podman (only for this switch)..."
apt-get download podman libsubid5 conmon crun netavark >/dev/null
for f in *.deb; do dpkg -x "$f" root; done
echo "Switching to $ed..."
sudo env PKEXEC_UID="$(id -u)" PATH="$d/root/usr/bin:$PATH" LD_LIBRARY_PATH="$d/root/usr/lib/x86_64-linux-gnu" \
  /usr/libexec/nimbus/hearth-admin move-to-new-os --edition "$ed"
