#!/bin/bash
# Hearth's writable layer (docs/APT-LAYER.md): the image side. Used by the Peppermint edition (and wired into the
# archived Ubuntu one). Run by os-bases/<edition>/build.sh after the last package is installed and before dpkg's
# database moves to /usr/lib/sysimage/dpkg (system_files/ is already copied in by then).
#   - the list of the image's packages (to tell your changes from the image), the packages Hearth's own tools
#     need (never removed), and apt pins holding the kernel and boot loader at the image's versions (so
#     `apt upgrade` passes them over)
#   - apt's and dpkg's hooks (only now: they'd otherwise run for every package the build installs)
#   - update-initramfs as a no-op (the initramfs comes with the image; a package's trigger can't rebuild it)
#   - .deb files open with Hearth's installer (hearth-install-deb.desktop)
#   - the two services
set -euxo pipefail
D=/usr/share/hearth/apt-layer
chmod 755 /usr/libexec/nimbus/hearth-apt-layer /usr/libexec/nimbus/hearth-deb-open

/usr/libexec/nimbus/hearth-apt-layer image-setup
test -s "$D/image-packages" && test -s "$D/image-pins.pref" && grep -qx apt "$D/protected" && grep -qx python3 "$D/protected"

install -Dm644 "$D/apt.conf" /etc/apt/apt.conf.d/80-hearth-apt-layer
install -Dm644 "$D/dpkg.cfg" /etc/dpkg/dpkg.cfg.d/hearth-apt-layer
mkdir -p /etc/apt/preferences.d
ln -sfn "$D/image-pins.pref" /etc/apt/preferences.d/zz-hearth-image
# apt must still read its configuration with the hooks and pins in place.
apt-config dump >/dev/null
apt-cache policy bash | sed -n 1,4p

U=/usr/sbin/update-initramfs
if [ -e "$U" ] && ! dpkg-divert --list "$U" | grep -q .; then
  dpkg-divert --local --rename --divert "$U.hearth-image" --add "$U"
  install -m755 "$D/update-initramfs" "$U"
fi

f=/usr/share/applications/mimeapps.list
[ -f "$f" ] || printf '[Default Applications]\n' > "$f"
grep -q '^\[Default Applications\]' "$f" || printf '\n[Default Applications]\n' >> "$f"
sed -i -e '/^application\/vnd\.debian\.binary-package=/d' -e '/^application\/x-deb=/d' "$f"
sed -i '/^\[Default Applications\]/a application/vnd.debian.binary-package=hearth-install-deb.desktop\napplication/x-deb=hearth-install-deb.desktop' "$f"
command -v update-desktop-database >/dev/null && update-desktop-database /usr/share/applications || true

systemctl enable hearth-apt-layer.service hearth-apt-layer-sync.service
