#!/bin/bash
# Hearth's Qt Quick renderer (hosts/hearth-qml, Experimental; docs/RENDERERS.md), built and checked inside a Hearth OS
# image's own base (Anatase's Fedora image, or Debian trixie for Peppermint), so it links that image's Qt.
# The repository is at /src, results go to /out. Used by hearth-qml.yml and base-anatase.yml / base-peppermint.yml.
#   hearth-qml-build.sh build          install the build tools, build, run the self-test (pictures in /out/shots)
#   hearth-qml-build.sh runtime-test   in a fresh container: install only what the image installs (runtime_pkgs), and run
#                                      the self-test against the fake core with the binary from /out
#   hearth-qml-build.sh runtime-pkgs   print the packages an image needs for it (build.sh installs the same list)
#   hearth-qml-build.sh image-check    inside a built Hearth OS image: the installed one, against the fake core (must
#                                      pass) and against the image's real core (`nimbus --core-stdio`, reported only)
set -uo pipefail
SRC=${SRC:-/src}
OUT=${OUT:-/out}
VERSION=${HEARTH_VERSION:-dev}
mkdir -p "$OUT"

if command -v dnf5 >/dev/null 2>&1 || command -v dnf >/dev/null 2>&1; then FAMILY=fedora; else FAMILY=debian; fi
DNF=$(command -v dnf5 || command -v dnf || true)

# What Hearth OS installs for the Qt renderer (keep in step with os-bases/*/build.sh).
runtime_pkgs() {
  if [ "$FAMILY" = fedora ]; then
    echo qt6-qtbase qt6-qtbase-gui qt6-qtdeclarative qt6-qtimageformats SDL2
  else
    echo qml6-module-qtquick qml6-module-qtquick-window qml6-module-qtquick-shapes qml6-module-qtquick-layouts qml6-module-qtqml-workerscript \
      qml6-module-qtqml-models qt6-qpa-plugins qt6-image-formats-plugins libsdl2-2.0-0
  fi
}
build_pkgs() {
  if [ "$FAMILY" = fedora ]; then
    echo cmake gcc-c++ make pkgconf-pkg-config qt6-qtbase-devel qt6-qtbase-private-devel qt6-qtdeclarative-devel SDL2-devel \
      python3 python3-fonttools python3-brotli google-noto-sans-fonts dejavu-sans-fonts
  else
    echo cmake g++ make pkg-config qt6-base-dev qt6-base-private-dev qt6-declarative-dev qt6-declarative-private-dev \
      qt6-declarative-dev-tools libsdl2-dev python3 python3-fonttools python3-brotli fonts-noto-core fonts-dejavu-core
  fi
}
pkgs_install() {
  # Packages one by one when the whole list fails (a name that isn't in this release mustn't stop the rest).
  if [ "$FAMILY" = fedora ]; then
    "$DNF" -y install --skip-unavailable "$@" >/tmp/pkg.log 2>&1 || { for p in "$@"; do "$DNF" -y install "$p" >>/tmp/pkg.log 2>&1 || echo "  (no $p here)"; done; }
  else
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq >/tmp/pkg.log 2>&1
    apt-get install -y -qq --no-install-recommends "$@" >>/tmp/pkg.log 2>&1 || { for p in "$@"; do apt-get install -y -qq --no-install-recommends "$p" >>/tmp/pkg.log 2>&1 || echo "  (no $p here)"; done; }
  fi
}

selftest() {   # $1: binary, $2: pictures folder, $3: core (default: the fake one)
  local bin=$1 shots=$2 core=${3:-$SRC/hosts/hearth-qml/test/fake-core.py}
  rm -rf /tmp/qhome && mkdir -p /tmp/qhome "$shots"
  chmod +x "$SRC/hosts/hearth-qml/test/fake-core.py" 2>/dev/null || true
  set +e
  HOME=/tmp/qhome XDG_RUNTIME_DIR=/tmp/qhome QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software HEARTH_QT_SOFTWARE=1 \
    HEARTH_CORE_BIN="$core" timeout 180 "$bin" --self-test --shots "$shots" >"$shots/selftest.log" 2>&1
  local rc=$?
  set -e
  echo "$rc" >"$shots/selftest.rc"
  grep -E "self-test|QML:|error|warning" "$shots/selftest.log" | tail -60
  echo "self-test exit code: $rc"
  return $rc
}

case "${1:-build}" in
  runtime-pkgs) runtime_pkgs ;;
  build)
    echo "== $FAMILY: build tools"
    # shellcheck disable=SC2046
    pkgs_install $(build_pkgs) $(runtime_pkgs)
    cmake --version | head -1
    qtv=$(ls -d /usr/lib*/cmake/Qt6Quick /usr/lib/*/cmake/Qt6Quick 2>/dev/null | head -1)
    echo "Qt6Quick cmake: ${qtv:-missing}"
    echo "== Anybody for headings (site/fonts, SIL OFL): a static cut"
    font=""
    if python3 - "$SRC/site/fonts/anybody.woff2" /tmp/anybody-hearth.ttf <<'PY'
import sys
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
f = TTFont(sys.argv[1]); f.flavor = None
axes = {a.axisTag: a for a in f['fvar'].axes} if 'fvar' in f else {}
loc = {}
if 'wght' in axes: loc['wght'] = max(axes['wght'].minValue, min(axes['wght'].maxValue, 800))
if 'wdth' in axes: loc['wdth'] = max(axes['wdth'].minValue, min(axes['wdth'].maxValue, 112))
try:
    s = instancer.instantiateVariableFont(f, loc, updateFontNames=True)
except Exception:
    s = instancer.instantiateVariableFont(f, loc)
s.flavor = None
s.save(sys.argv[2])
print('made', sys.argv[2], loc)
PY
    then font=/tmp/anybody-hearth.ttf; else echo "::warning title=hearth-qml::couldn't make Anybody's static cut; headings use the body face"; fi
    echo "== build"
    set -e
    rm -rf /tmp/qbuild
    cmake -S "$SRC/hosts/hearth-qml" -B /tmp/qbuild -DCMAKE_BUILD_TYPE=Release -DHEARTH_QML_VERSION="$VERSION" -DHEARTH_FONT="$font" 2>&1 | tail -25
    cmake --build /tmp/qbuild -j"$(nproc)" 2>&1 | grep -vE '^\[ *[0-9]+%\] (Building|Automatic|Running|Generating)' | tail -60
    test -x /tmp/qbuild/hearth-qml
    strip /tmp/qbuild/hearth-qml || true
    mkdir -p "$OUT/hearth-qml"
    cp /tmp/qbuild/hearth-qml "$OUT/hearth-qml/"
    cp "$SRC/site/fonts/OFL.txt" "$OUT/hearth-qml/ANYBODY-OFL.txt" 2>/dev/null || true
    ldd /tmp/qbuild/hearth-qml | grep -E "Qt6|SDL" | sed 's/^/  /'
    set +e
    echo "== qmllint (advice only)"
    lint=$(ls /usr/lib*/qt6/bin/qmllint /usr/lib/qt6/bin/qmllint /usr/lib/*/qt6/bin/qmllint /usr/bin/qmllint6 2>/dev/null | head -1)
    if [ -n "$lint" ]; then
      (cd /tmp/qbuild && "$lint" -I /tmp/qbuild "$SRC"/hosts/hearth-qml/qml/*.qml >"$OUT/qmllint.log" 2>&1)
      echo "qmllint: $(grep -c -i 'warning' "$OUT/qmllint.log") warnings (in qmllint.log)"
    fi
    echo "== self-test (offscreen, against the fake core)"
    selftest "$OUT/hearth-qml/hearth-qml" "$OUT/shots"
    exit $?
    ;;
  runtime-test)
    echo "== $FAMILY: only what the image installs"
    # shellcheck disable=SC2046
    pkgs_install $(runtime_pkgs) python3 $( [ "$FAMILY" = fedora ] && echo google-noto-sans-fonts || echo fonts-noto-core )
    missing=$(ldd "$OUT/hearth-qml/hearth-qml" | grep "not found")
    if [ -n "$missing" ]; then echo "missing libraries: $missing"; exit 4; fi
    selftest "$OUT/hearth-qml/hearth-qml" "$OUT/shots-runtime"
    exit $?
    ;;
  image-check)
    bin=/usr/lib/hearth-qml/hearth-qml
    test -x "$bin" || { echo "::error title=Qt renderer::/usr/lib/hearth-qml/hearth-qml isn't in the image"; exit 1; }
    missing=$(ldd "$bin" | grep "not found")
    [ -n "$missing" ] && { echo "::error title=Qt renderer::libraries missing in the image: $(echo $missing | tr '\n' ' ')"; exit 1; }
    rc=0
    if selftest "$bin" "$OUT/image-fake"; then echo "::notice title=Qt renderer in the image::starts and draws every screen (self-test against the fake core): $(grep -m1 'starting: Qt' "$OUT/image-fake/selftest.log" | sed 's/.*starting: //')"
    else echo "::error title=Qt renderer in the image::the self-test against the fake core failed: $(grep -E 'FAILED|QML:' "$OUT/image-fake/selftest.log" | head -8 | cut -c1-200 | tr '\n' ' ')"; rc=1; fi
    if selftest "$bin" "$OUT/image-real" /usr/lib/nimbus/nimbus; then echo "::notice title=Qt renderer in the image::also works against this image's own core (nimbus --core-stdio, an empty library)"
    else echo "::warning title=Qt renderer in the image::against this image's own core: $(grep -E 'FAILED|QML:|core' "$OUT/image-real/selftest.log" | head -8 | cut -c1-200 | tr '\n' ' ')"; fi
    exit $rc
    ;;
  *) echo "usage: $0 build|runtime-test|runtime-pkgs|image-check"; exit 2 ;;
esac
