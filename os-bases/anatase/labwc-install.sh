#!/bin/bash
# What Mantel (Hearth Classic) and Firehold on labwc need on the Fedora edition (1.16; docs/LABWC-FEDORA.md): labwc,
# the Qt modules Quickshell's QML loads, and every program Mantel's Debian build installs for its desktop
# (os-bases/peppermint/build.sh "Desktop Mode: labwc + Quickshell" and "KDE pieces Mantel ... use"), by Fedora's names.
# Quickshell itself is built separately (quickshell-build.sh) and copied in before this runs.
#   labwc-install.sh           install (build.sh, and .github/workflows/labwc-fedora.yml's test container)
#   labwc-install.sh --check   only check: labwc, Quickshell and its libraries; exit 1 if anything required is missing
# Required: labwc and Quickshell able to start (every library found). Everything else is optional: a missing one is
# listed (::warning) and Mantel shows that part as unavailable, as on Debian.
set -uo pipefail
REQUIRED=(labwc xorg-x11-server-Xwayland qt6-qtbase-gui qt6-qtdeclarative qt6-qtwayland qt6-qtsvg jemalloc)
OPTIONAL=(qt6-qt5compat qt6-qtimageformats qt6-qtmultimedia qt6ct
  grim slurp wlr-randr wlsunset wlopm wl-clipboard cliphist wtype foot xdotool
  xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk libnotify
  python3-evdev python3-gobject python3-dbus
  breeze-cursor-theme breeze-icon-theme adwaita-icon-theme hicolor-icon-theme rsms-inter-fonts google-noto-sans-fonts
  gnome-keyring libsecret gvfs gvfs-smb gvfs-fuse gocryptfs fuse3 poppler-utils
  tesseract tesseract-langpack-eng zbar mate-polkit)
check() {
  bad=""
  command -v labwc >/dev/null || bad="$bad labwc"
  if command -v quickshell >/dev/null; then
    miss=$(ldd "$(command -v quickshell)" 2>/dev/null | awk '/not found/ {print $1}' | tr '\n' ' ')
    [ -z "$miss" ] || bad="$bad quickshell-libraries($miss)"
    built=$(cat /usr/share/hearth/quickshell-qt 2>/dev/null); have=$(rpm -q --qf '%{VERSION}-%{RELEASE}' qt6-qtbase 2>/dev/null)
    if [ -n "$built" ] && [ "$built" != "$have" ]; then bad="$bad quickshell-built-for-qt-$built-not-$have"; fi
  else
    bad="$bad quickshell"
  fi
  if [ -n "$bad" ]; then echo "::error title=labwc on Fedora::missing or broken:$bad"; return 1; fi
  echo "::notice title=labwc on Fedora::labwc $(labwc --version 2>/dev/null | head -1 | awk '{print $NF}'), Quickshell $(cat /usr/share/hearth/quickshell-ref 2>/dev/null) on Qt $(rpm -q --qf '%{VERSION}' qt6-qtbase)"
}
if [ "${1:-}" = --check ]; then check; exit $?; fi
dnf5 -y install --setopt=install_weak_deps=False "${REQUIRED[@]}" || { echo "::error title=labwc on Fedora::couldn't install ${REQUIRED[*]}"; exit 1; }
dnf5 -y install --skip-unavailable --skip-broken --setopt=install_weak_deps=False "${OPTIONAL[@]}" >/tmp/labwc-optional.log 2>&1 || true
gone=""
for p in "${OPTIONAL[@]}"; do rpm -q "$p" >/dev/null 2>&1 || gone="$gone $p"; done
[ -z "$gone" ] || echo "::warning title=labwc on Fedora::not in Fedora's repositories here (Mantel shows those parts as unavailable):$gone"
echo "$gone" | tr ' ' '\n' | sed '/^$/d' >/tmp/labwc-missing-packages.txt
check
