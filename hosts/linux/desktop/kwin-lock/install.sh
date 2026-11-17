#!/bin/bash
# Installs Hearth's KWin lock screen so KWin's screen locker can show it (docs/KWIN-INPUT.md), on Debian. Used by
# os-bases/peppermint/build.sh and .github/workflows/kwin-input.yml (the same steps in both), as root.
#   install.sh SOURCE_FOLDER   (this folder: metadata.json, contents/lockscreen/LockScreen.qml)
# 1. KWin's screen locker: Debian's libkscreenlocker6 (kscreenlocker_greet).
# 2. The "Plasma/Shell" package type the greeter reads the lock screen as. KDE ships it only inside
#    plasma-workspace (the whole Plasma desktop, which Hearth doesn't install), so only that one plugin is taken
#    from plasma-workspace's package, and every library it needs must already be there.
# 3. The lock screen itself, as that shell package (and as a look-and-feel package for older lockers).
# Fails (exit 1) when any of it is missing: a lock screen that can't show would be a lockout.
set -euo pipefail
SRC=${1:?install.sh SOURCE_FOLDER}
ID=org.hearth.mantel-lock
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get install -y --no-install-recommends)

pick() { for p in "$@"; do apt-cache show "$p" >/dev/null 2>&1 && { echo "$p"; return 0; }; done; return 1; }
KSL=$(pick kscreenlocker libkscreenlocker6 libkscreenlocker5) || { echo "install.sh: KWin's screen locker isn't in the repositories"; exit 1; }
PLS=$(pick libplasma6 libplasma5 || true)
# The lock screen is plain QtQuick + QtQuick.Layouts (Quickshell needs them too).
"${APT[@]}" "$KSL" ${PLS:+"$PLS"} qml6-module-qtquick qml6-module-qtquick-layouts
GREETER=$(ls /usr/lib/*/libexec/kscreenlocker_greet /usr/libexec/kscreenlocker_greet 2>/dev/null | head -1)
[ -x "$GREETER" ] || { echo "install.sh: kscreenlocker_greet isn't installed ($KSL)"; exit 1; }

if ! ls /usr/lib/*/qt6/plugins/kf6/packagestructure/ 2>/dev/null | grep -qi shell; then
  tmp=$(mktemp -d)
  ( cd "$tmp" && apt-get download plasma-workspace >/dev/null )
  deb=$(ls "$tmp"/plasma-workspace_*.deb | head -1)
  dpkg-deb -x "$deb" "$tmp/x"
  found=$(find "$tmp/x" -path '*kf6/packagestructure/*' -iname '*shell*.so' | head -1)
  [ -n "$found" ] || { echo "install.sh: plasma-workspace has no shell package type"; exit 1; }
  missing=$(ldd "$found" | awk '/not found/{print $1}')
  [ -z "$missing" ] || { echo "install.sh: the shell package type needs libraries that aren't installed: $missing"; exit 1; }
  dest="/${found#"$tmp/x/"}"
  install -Dm644 "$found" "$dest"
  echo "install.sh: Plasma/Shell package type from $(basename "$deb"): $dest"
  rm -rf "$tmp"
fi

for kind in shells look-and-feel; do
  rm -rf "/usr/share/plasma/$kind/$ID"
  mkdir -p "/usr/share/plasma/$kind"
  cp -a "$SRC" "/usr/share/plasma/$kind/$ID"
  rm -f "/usr/share/plasma/$kind/$ID/install.sh"
done
python3 -c 'import json,sys; p=sys.argv[1]; m=json.load(open(p)); m["KPackageStructure"]="Plasma/LookAndFeel"; json.dump(m, open(p,"w"), indent=4)' \
  "/usr/share/plasma/look-and-feel/$ID/metadata.json"
echo "install.sh: $KSL ($GREETER), lock screen $ID installed"
