#!/bin/bash
# Installs Hearth's KWin lock screen so KWin's screen locker can show it (docs/KWIN-INPUT.md), on Debian. Used by
# os-bases/peppermint/build.sh and .github/workflows/kwin-input.yml (the same steps in both), as root.
#   install.sh SOURCE_FOLDER   (this folder: metadata.json, contents/lockscreen/LockScreen.qml)
# 1. KWin's screen locker: Debian's libkscreenlocker6 (kscreenlocker_greet).
# 2. The "Plasma/Shell" package type the greeter reads the lock screen as (Hearth's own on Debian trixie, which ships
#    none: kwin-helpers/shell-package).
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
PFW=""
# The lock screen is plain QtQuick + QtQuick.Layouts (Quickshell needs them too).
# qt6-wayland and layer-shell-qt: the greeter is a Wayland client of KWin shown as a layer surface (without either it
# can't start, and KWin stays on its emergency screen).
LSQ=$(pick layer-shell-qt liblayershellqtinterface6 || true)
"${APT[@]}" "$KSL" ${PLS:+"$PLS"} ${PFW:+"$PFW"} ${LSQ:+"$LSQ"} qml6-module-qtquick qml6-module-qtquick-layouts qt6-wayland
for f in /usr/lib/*/qt6/plugins/wayland-shell-integration/liblayer-shell.so /usr/lib64/qt6/plugins/wayland-shell-integration/liblayer-shell.so; do
  [ -f "$f" ] && LAYER_OK=1
done
[ -n "${LAYER_OK:-}" ] || { echo "install.sh: Qt's layer-shell integration (layer-shell-qt) isn't installed ($LSQ)"; exit 1; }
GREETER=$( (ls /usr/lib/*/libexec/kscreenlocker_greet /usr/libexec/kscreenlocker_greet 2>/dev/null || true) | head -1)
[ -x "$GREETER" ] || { echo "install.sh: kscreenlocker_greet isn't installed ($KSL)"; exit 1; }

# The Plasma/Shell package type: newer libplasma has it (plasma_shell); Debian trixie's doesn't, so Hearth's own
# (plasma_shell.so, built by the image's builder from kwin-helpers/shell-package; HEARTH_SHELL_PACKAGE_SO
# points elsewhere for CI) goes in KDE's package-type folder.
have_shell_type() { for f in /usr/lib/*/qt6/plugins/kf6/packagestructure/plasma_shell.so /usr/lib64/qt6/plugins/kf6/packagestructure/plasma_shell.so; do [ -f "$f" ] && return 0; done; return 1; }
if ! have_shell_type; then
  so=${HEARTH_SHELL_PACKAGE_SO:-/usr/lib/hearth/plasma_shell.so}
  [ -f "$so" ] || { echo "install.sh: no Plasma/Shell package type here and no $so"; exit 1; }
  qtp=$( (ls -d /usr/lib/*/qt6/plugins 2>/dev/null || true) | head -1)
  [ -n "$qtp" ] || { echo "install.sh: Qt's plugin folder isn't there"; exit 1; }
  missing=$(ldd "$so" | awk '/not found/{print $1}')
  [ -z "$missing" ] || { echo "install.sh: $so needs libraries that aren't installed: $missing"; exit 1; }
  # KPackage finds a type by its file name: "Plasma/Shell" is plasma_shell.so.
  install -Dm644 "$so" "$qtp/kf6/packagestructure/plasma_shell.so"
  echo "install.sh: Hearth's Plasma/Shell package type: $qtp/kf6/packagestructure/plasma_shell.so"
fi
have_shell_type || { echo "install.sh: the Plasma/Shell package type still isn't found"; exit 1; }

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
