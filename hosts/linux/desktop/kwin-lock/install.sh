#!/bin/bash
# Installs Hearth's KWin lock screen so KWin's screen locker can show it (docs/KWIN-INPUT.md), on Debian. Used by
# os-bases/peppermint/build.sh and .github/workflows/kwin-input.yml (the same steps in both), as root.
#   install.sh SOURCE_FOLDER   (this folder: metadata.json, contents/lockscreen/LockScreen.qml)
# 1. KWin's screen locker: Debian's libkscreenlocker6 (kscreenlocker_greet).
# 2. The "Plasma/Shell" package type the greeter reads the lock screen as: libplasma's plugins (Debian's
#    plasma-framework; not the Plasma desktop). If this Debian has it elsewhere, it's looked for in plasma-workspace's
#    package and only that one plugin taken, with every library it needs already there.
# 3. The lock screen itself, as that shell package.
# Fails (exit 1) when any of it is missing: a lock screen that can't show would be a lockout.
set -Eeuo pipefail
trap 'echo "install.sh: failed at line $LINENO: $BASH_COMMAND" >&2' ERR
SRC=${1:?install.sh SOURCE_FOLDER}
ID=org.hearth.mantel-lock
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get install -y --no-install-recommends)

pick() { for p in "$@"; do apt-cache show "$p" >/dev/null 2>&1 && { echo "$p"; return 0; }; done; return 1; }
KSL=$(pick kscreenlocker libkscreenlocker6 libkscreenlocker5) || { echo "install.sh: KWin's screen locker isn't in the repositories"; exit 1; }
PLS=$(pick libplasma6 libplasma5 || true)
# plasma-framework: libplasma's plugins, among them the Plasma/Shell package type (not the Plasma desktop).
PFW=$(pick plasma-framework || true)
# The lock screen is plain QtQuick + QtQuick.Layouts (Quickshell needs them too).
"${APT[@]}" "$KSL" ${PLS:+"$PLS"} ${PFW:+"$PFW"} qml6-module-qtquick qml6-module-qtquick-layouts
GREETER=$( (ls /usr/lib/*/libexec/kscreenlocker_greet /usr/libexec/kscreenlocker_greet 2>/dev/null || true) | head -1)
[ -x "$GREETER" ] || { echo "install.sh: kscreenlocker_greet isn't installed ($KSL)"; exit 1; }

# The package types already there (from what's installed), else the one plugin from whichever Debian package has it.
have_shell_type() { find /usr/lib/*/qt6/plugins/kf6/packagestructure/ -iname '*shell*' 2>/dev/null | grep -q .; }
if ! have_shell_type; then
  tmp=$(mktemp -d)
  seen=""
  for pkg in libplasma6 plasma-framework libplasmaquick6 plasma-workspace plasma-desktop; do
    apt-cache show "$pkg" >/dev/null 2>&1 || continue
    ( cd "$tmp" && apt-get download "$pkg" >/dev/null 2>&1 ) || continue
    deb=$(ls "$tmp"/"$pkg"_*.deb 2>/dev/null | head -1 || true)
    [ -n "$deb" ] || continue
    rm -rf "$tmp/x" && dpkg-deb -x "$deb" "$tmp/x"
    seen="$seen $pkg:[$(find "$tmp/x" -ipath '*packagestructure*' -name '*.so' -printf '%f ' 2>/dev/null || true)]"
    found=$(find "$tmp/x" -ipath '*packagestructure*' -iname '*shell*.so' -print -quit)
    [ -n "$found" ] || continue
    missing=$(ldd "$found" | awk '/not found/{print $1}')
    [ -z "$missing" ] || { echo "install.sh: the shell package type ($pkg) needs libraries that aren't installed: $missing"; exit 1; }
    dest="/${found#"$tmp/x/"}"
    install -Dm644 "$found" "$dest"
    echo "install.sh: Plasma/Shell package type from $(basename "$deb"): $dest"
    break
  done
  rm -rf "$tmp"
  have_shell_type || { echo "install.sh: no Debian package here has the Plasma/Shell package type;$seen"; exit 1; }
fi

# KWin's locker checks the password with PAM's "kde" service: Debian's common-auth if no package brought one.
if [ ! -e /etc/pam.d/kde ]; then
  printf '# KWin screen locker (written by Hearth: kwin-lock/install.sh)\n@include common-auth\n@include common-account\n' > /etc/pam.d/kde
  echo "install.sh: /etc/pam.d/kde written (common-auth)"
fi

rm -rf "/usr/share/plasma/shells/$ID"
mkdir -p /usr/share/plasma/shells
cp -a "$SRC" "/usr/share/plasma/shells/$ID"
rm -f "/usr/share/plasma/shells/$ID/install.sh"
echo "install.sh: $KSL ($GREETER), lock screen $ID installed"
