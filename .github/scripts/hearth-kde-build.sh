#!/bin/bash
# Hearth's KDE apps (hosts/hearth-kde, docs/KDE-APPS.md), built and tested inside a Hearth OS image's own base
# (Anatase's Fedora image, or Debian trixie for Peppermint), so they link exactly that image's Qt and KDE Frameworks.
# The repository is at /src (read-only), results go to /out. Used by kde-apps.yml and base-anatase.yml /
# base-peppermint.yml:
#   hearth-kde-build.sh anatase|peppermint
# Makes /out/hearth-kde.tar (unpacked over / by hosts/hearth-kde/install.sh in the image build):
#   usr/lib/hearth-files/...            Hearth Files: Dolphin (versions.txt) + patches/ + dolphin/hearth, its own prefix
#   <Qt plugin dir>/kf6/kio/hearth.so   kio-hearth, hearth:/ for every KDE app
#   usr/share/hearth-kde/build.json     what was built, from which release, and whether its tests passed
# Failures are ::error:: annotations (the end of the failing step's output), since the full log isn't readable.
# kio-hearth's tests failing, or Hearth Files' patch not applying / not building / not starting: that part is left
# out of the tar (the image then keeps the old Hearth Files) and this script exits 1.
set -uo pipefail
BASE=${1:?anatase or peppermint}
SRC=${SRC:-/src}
OUT=${OUT:-/out}
KDE="$SRC/hosts/hearth-kde"
STAGE=/tmp/stage
mkdir -p "$OUT" "$STAGE"
rc=0
note() { echo "::notice title=$1::$2"; }
fail_with_log() { # title, log file
  local msg
  msg=$(grep -v -e '^\s*$' "$2" 2>/dev/null | tail -40 | cut -c1-300 | sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g')
  echo "::error title=$1 ($BASE)::$msg"
}

if command -v dnf5 >/dev/null 2>&1 || command -v dnf >/dev/null 2>&1; then FAMILY=fedora; else FAMILY=debian; fi
DNF=$(command -v dnf5 || command -v dnf || true)

# ------------------------------------------------------------------ build tools, and Dolphin's own build dependencies
if [ "$FAMILY" = fedora ]; then
  $DNF -y install --skip-unavailable cmake ninja-build gcc-c++ make extra-cmake-modules qt6-qtbase-devel kf6-kio-devel \
    kf6-kcoreaddons-devel dbus-daemon dbus-tools python3 xz curl tar patch findutils file binutils dolphin >/tmp/deps.log 2>&1 \
    || { fail_with_log "build tools" /tmp/deps.log; exit 1; }
  # Dolphin's build dependencies as Fedora's own package states them; the list below when the source repos can't be had.
  if ! $DNF -y builddep --enablerepo='*-source' dolphin >>/tmp/deps.log 2>&1 && ! $DNF -y builddep dolphin >>/tmp/deps.log 2>&1; then
    $DNF -y install --skip-unavailable kf6-kbookmarks-devel kf6-kcmutils-devel kf6-kcompletion-devel kf6-kconfig-devel \
      kf6-kcrash-devel kf6-kdbusaddons-devel kf6-kdoctools-devel kf6-kfilemetadata-devel kf6-ki18n-devel kf6-kiconthemes-devel \
      kf6-kitemviews-devel kf6-knewstuff-devel kf6-knotifications-devel kf6-kparts-devel kf6-solid-devel kf6-ktextwidgets-devel \
      kf6-kwidgetsaddons-devel kf6-kwindowsystem-devel kf6-kxmlgui-devel kf6-kcolorscheme-devel kf6-kguiaddons-devel \
      kf6-kservice-devel kf6-baloo-devel baloo-widgets-devel kuserfeedback-devel packagekitqt6-devel phonon-qt6-devel \
      qt6-qtmultimedia-devel plasma-activities-devel zlib-devel >>/tmp/deps.log 2>&1 || true
  fi
  DISTRO_DOLPHIN=$(rpm -q --qf '%{VERSION}' dolphin 2>/dev/null)
  KF_VERSION=$(rpm -q --qf '%{VERSION}' kf6-kio-core 2>/dev/null || rpm -q --qf '%{VERSION}' kf6-kio 2>/dev/null)
else
  export DEBIAN_FRONTEND=noninteractive
  # Debian's source lists too, for `apt-get build-dep dolphin`.
  for f in /etc/apt/sources.list.d/*.sources; do
    [ -f "$f" ] && sed -i -e 's/^Types: deb$/Types: deb deb-src/' "$f"
  done
  { apt-get update -qq && apt-get install -y -qq --no-install-recommends build-essential cmake ninja-build extra-cmake-modules \
      qt6-base-dev libkf6kio-dev libkf6coreaddons-dev dbus python3 xz-utils curl ca-certificates patch file binutils dolphin \
    && apt-get build-dep -y -qq dolphin; } >/tmp/deps.log 2>&1 || { fail_with_log "build tools" /tmp/deps.log; exit 1; }
  DISTRO_DOLPHIN=$(dpkg-query -W -f '${Version}' dolphin 2>/dev/null | sed -e 's/^[0-9]*://' -e 's/-[^-]*$//' -e 's/+.*//')
  KF_VERSION=$(dpkg-query -W -f '${Version}' libkf6kiocore6 2>/dev/null | sed -e 's/-[^-]*$//')
fi
QT_VERSION=$(qtpaths6 --qt-version 2>/dev/null || qmake6 -query QT_VERSION 2>/dev/null || echo '?')
note "KDE in $BASE" "Dolphin packaged: ${DISTRO_DOLPHIN:-none}; KDE Frameworks (KIO): ${KF_VERSION:-?}; Qt: $QT_VERSION"
DBUS_RUN=""
command -v dbus-run-session >/dev/null && DBUS_RUN="dbus-run-session --"

# ------------------------------------------------------------------ kio-hearth: hearth:/ for every KDE app
KIO_OK=false
KIO_TESTS=false
if { cmake -S "$KDE/kio-hearth" -B /tmp/kio-build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr \
      && cmake --build /tmp/kio-build; } >/tmp/kio.log 2>&1; then
  if (cd /tmp/kio-build && HOME=/tmp/kio-home XDG_RUNTIME_DIR=/tmp/kio-run $DBUS_RUN ctest --output-on-failure --timeout 120) >/tmp/kio-test.log 2>&1; then
    KIO_TESTS=true
    note "kio-hearth ($BASE)" "built; its tests passed ($(grep -oE '[0-9]+% tests passed.*' /tmp/kio-test.log | tail -1))"
  else
    fail_with_log "kio-hearth tests" /tmp/kio-test.log
    rc=1
  fi
  if [ "$KIO_TESTS" = true ] && DESTDIR="$STAGE" cmake --install /tmp/kio-build >>/tmp/kio.log 2>&1; then
    KIO_OK=true
  fi
else
  fail_with_log "kio-hearth build" /tmp/kio.log
  rc=1
fi
[ "$KIO_OK" = true ] || rm -rf "$STAGE/usr/lib64/qt6/plugins/kf6/kio/hearth.so" "$STAGE"/usr/lib/*/qt6/plugins/kf6/kio/hearth.so

# ------------------------------------------------------------------ Hearth Files: Dolphin + Hearth's patches
FILES_OK=false
read -r PIN_VER PIN_SHA < <(awk -v b="$BASE" '$1 == b {print $2, $3}' "$KDE/dolphin/versions.txt")
VER=$PIN_VER
PINNED=true
if [ -z "$VER" ] || [ "$VER" = discover ]; then VER=$DISTRO_DOLPHIN; PINNED=false; fi
if [ -n "$VER" ]; then
  mkdir -p /tmp/dolphin && cd /tmp/dolphin || exit 1
  url="https://download.kde.org/stable/release-service/$VER/src/dolphin-$VER.tar.xz"
  if curl -fsSL --retry 3 -o dolphin.tar.xz "$url" >/tmp/dl.log 2>&1; then
    sha=$(sha256sum dolphin.tar.xz | cut -d' ' -f1)
    if [ "$PINNED" = true ] && [ "$sha" != "$PIN_SHA" ]; then
      echo "::error title=Hearth Files ($BASE)::dolphin-$VER.tar.xz has sha256 $sha, versions.txt pins $PIN_SHA"
      rc=1
    else
      [ "$PINNED" = true ] || echo "::warning title=Hearth Files ($BASE)::not pinned yet: pin '$BASE $VER $sha' in hosts/hearth-kde/dolphin/versions.txt"
      tar -xf dolphin.tar.xz
      cd "dolphin-$VER" || exit 1
      ok=true
      for p in "$KDE"/dolphin/patches/*.patch; do
        if ! patch -p1 --forward --no-backup-if-mismatch -i "$p" >/tmp/patch.log 2>&1; then
          {
            cat /tmp/patch.log
            echo "--- where it should go in src/main.cpp:"
            grep -n -e 'KAboutData' -e '^#include' src/main.cpp | head -30
          } >/tmp/patch-fail.log
          fail_with_log "Hearth Files patch $(basename "$p")" /tmp/patch-fail.log
          ok=false
          rc=1
          break
        fi
      done
      if [ "$ok" = true ]; then
        # Its own prefix: the base's Dolphin package (installed for the libraries it needs) stays untouched.
        if { cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF \
              -DCMAKE_INSTALL_PREFIX=/usr/lib/hearth-files -DKDE_INSTALL_LIBDIR=lib -DCMAKE_INSTALL_RPATH=/usr/lib/hearth-files/lib \
              -DCMAKE_PROJECT_INCLUDE="$KDE/dolphin/hearth/hearth.cmake" \
            && cmake --build build && DESTDIR="$STAGE" cmake --install build; } >/tmp/dolphin.log 2>&1; then
          d="$STAGE/usr/lib/hearth-files"
          # Only the program, its libraries and plugins: the menu entry, D-Bus services, icons and translations are
          # the base's own Dolphin package's (install.sh writes Hearth's menu entry), headers aren't needed.
          rm -rf "$d/include" "$d/lib/cmake" "$d/share"
          find "$d" -type f \( -name '*.so*' -o -path '*/bin/*' \) -exec sh -c 'file -b "$1" | grep -q ELF && strip --strip-unneeded "$1"' _ {} \; 2>/dev/null
          # It starts, and it's Hearth Files (the patch's description in --help), and it runs for a few seconds
          # on an offscreen screen without crashing.
          export QT_QPA_PLATFORM=offscreen HOME=/tmp/files-home XDG_RUNTIME_DIR=/tmp/files-run LD_LIBRARY_PATH="$d/lib"
          mkdir -p /tmp/files-home /tmp/files-run && chmod 700 /tmp/files-run
          help=$("$d/bin/dolphin" --help 2>&1)
          if echo "$help" | grep -q "Hearth OS's file manager"; then
            timeout 10 $DBUS_RUN "$d/bin/dolphin" /tmp >/tmp/dolphin-run.log 2>&1
            code=$?
            if [ "$code" = 124 ] || [ "$code" = 0 ]; then
              FILES_OK=true
              note "Hearth Files ($BASE)" "Dolphin $VER with Hearth's patches built and started ($(du -sh "$d" | cut -f1))"
            else
              fail_with_log "Hearth Files didn't keep running (exit $code)" /tmp/dolphin-run.log
              rc=1
            fi
          else
            echo "$help" >/tmp/dolphin-help.log
            fail_with_log "Hearth Files --help doesn't say it's Hearth Files" /tmp/dolphin-help.log
            rc=1
          fi
          unset QT_QPA_PLATFORM LD_LIBRARY_PATH
        else
          fail_with_log "Hearth Files build" /tmp/dolphin.log
          rc=1
        fi
      fi
    fi
  else
    fail_with_log "Hearth Files download ($url)" /tmp/dl.log
    rc=1
  fi
else
  echo "::error title=Hearth Files ($BASE)::no Dolphin version (not pinned, and the base doesn't package Dolphin)"
  rc=1
fi
# Only a pinned, checked release goes into an image.
[ "$FILES_OK" = true ] && [ "$PINNED" = true ] || rm -rf "$STAGE/usr/lib/hearth-files"
[ "$FILES_OK" = true ] && [ "$PINNED" = false ] && echo "::warning title=Hearth Files ($BASE)::built and started, but left out of the tar until versions.txt pins it"

mkdir -p "$STAGE/usr/share/hearth-kde"
cat >"$STAGE/usr/share/hearth-kde/build.json" <<JSON
{"base": "$BASE", "dolphin": "$VER", "pinned": $PINNED, "files": $([ -d "$STAGE/usr/lib/hearth-files" ] && echo true || echo false), "kio": $KIO_OK, "kioTests": $KIO_TESTS, "kf": "${KF_VERSION:-}", "qt": "$QT_VERSION"}
JSON
cat "$STAGE/usr/share/hearth-kde/build.json"
(cd "$STAGE" && tar -cf "$OUT/hearth-kde.tar" usr)
ls -l "$OUT/hearth-kde.tar"
exit $rc
