#!/bin/bash
# Firehold on labwc's Couch (1.16): the approved Couch's pages are Firehold on Plasma's own files
# (desktop/firehold-plasma/couch/contents/ui, one source). Quickshell only loads QML files that are really in its shell
# folder (it lists them when it starts; links aren't followed), so they're copied into desktop/shell/fhcouch when the
# image is built, with firehold.js, leaving out the files only Plasma can load.
#   install-couch.sh [DESKTOP]      DESKTOP defaults to /usr/share/nimbus/desktop
# Never fails the image: without them, Firehold on labwc shows its own Couch (FireholdCouch.qml).
set -u
D=${1:-/usr/share/nimbus/desktop}
UI="$D/firehold-plasma/couch/contents/ui"
OUT="$D/shell/fhcouch"
[ -d "$UI" ] || { echo "install-couch: no $UI; Firehold on labwc keeps its own Couch"; exit 0; }
rm -rf "$OUT"; mkdir -p "$OUT"
for f in "$UI"/*.qml "$UI"/*.js; do
  case "$(basename "$f")" in
    main.qml|PlasmaHost.qml|PlasmaMedia.qml|KIcon.qml|Thumb.qml|GuideLayer.qml|GuideWindow.qml|Guide.qml) continue ;;
  esac
  cp "$f" "$OUT/" || echo "install-couch: couldn't copy $f"
done
cp "$D/shell/firehold.js" "$OUT/firehold.js" || echo "install-couch: couldn't copy firehold.js"
# Each page names its neighbours by type (T, Card, Home...): a qmldir saying which file each one is, so the QML engine
# doesn't have to find them by looking in the folder (under Quickshell some were missed: "T is not a type").
for f in "$OUT"/*.qml; do n=$(basename "$f" .qml); echo "$n 1.0 $n.qml"; done >"$OUT/qmldir"
# Firehold's Quick Settings, notification cards and list (mockups 07-10): Firehold on Plasma's own files
# (desktop/plasma-hearth/package), written in Mantel's own words; next to Mantel's (they use its DeskState, Settings,
# NotifyRules...), with Quickshell's imports put back (on Plasma they come from its shims instead).
PKG="$D/plasma-hearth/package"
if [ -d "$PKG" ]; then
  for f in FireholdQuick FireholdCards FireholdHistory FhButton FhSegs FhSlider FhState FhTile; do
    [ -f "$PKG/$f.qml" ] || { echo "install-couch: no $f.qml"; continue; }
    awk 'BEGIN { done = 0 } { print } !done && /^import QtQuick$/ { print "import Quickshell"; print "import Quickshell.Io"; print "import Quickshell.Wayland"; done = 1 }' \
      "$PKG/$f.qml" >"$D/shell/$f.qml" || echo "install-couch: couldn't copy $f.qml"
  done
  # (a singleton under Quickshell is a Singleton, not an Item)
  [ -f "$D/shell/FhState.qml" ] && sed -i '0,/^Item {$/s//Singleton {/' "$D/shell/FhState.qml"
  cp "$PKG/fh.js" "$D/shell/fh.js" || echo "install-couch: couldn't copy fh.js"
fi
echo "install-couch: $(ls "$OUT" | wc -l) files in $OUT"
exit 0
