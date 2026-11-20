#!/bin/bash
# Firehold on Plasma (a test, docs/FIREHOLD.md "Firehold on Plasma"): puts its Plasma packages where Plasma finds them.
#   install.sh [ROOT]      ROOT defaults to /. Used by os-bases/anatase/build.sh and .github/workflows/firehold-plasma.yml.
# Needs Plasma installed first (the Desk is a copy of Plasma's own desktop shell, made here). Never fails the image:
# what can't be put in place is reported and skipped, and Desktop Mode is plain KDE Plasma without it.
set -u
SRC=$(cd "$(dirname "$0")" && pwd)
ROOT=${1:-/}
P="$ROOT/usr/share/plasma"
ok=1
mkdir -p "$P/shells" "$P/plasmoids" "$ROOT/usr/share/applications" "$ROOT/etc/xdg/autostart" "$ROOT/usr/bin"
rm -rf "$P/shells/org.hearthos.firehold" "$P/plasmoids/org.hearthos.firehold.couch" "$P/plasmoids/org.hearthos.firehold.posture" \
  "$P/plasmoids/org.hearthos.firehold.app" "$P/shells/org.hearthos.firehold.desk"
cp -a "$SRC/shell" "$P/shells/org.hearthos.firehold"
cp -a "$SRC/couch" "$P/plasmoids/org.hearthos.firehold.couch"
# The Couch's rules are Mantel's Firehold's (one copy: desktop/shell/firehold.js).
cp "$SRC/../shell/firehold.js" "$P/plasmoids/org.hearthos.firehold.couch/contents/ui/firehold.js" || ok=0
cp -a "$SRC/posture" "$P/plasmoids/org.hearthos.firehold.posture"
# The Desk's mode pill shows a Couch or Away screen's page over the Desk, and its Modes popup uses the Couch's parts:
# the Couch's own files, copied next to it (one source: couch/contents/ui).
mkdir -p "$P/plasmoids/org.hearthos.firehold.posture/contents/ui/couch"
cp "$SRC"/couch/contents/ui/*.qml "$SRC"/couch/contents/ui/*.js "$P/plasmoids/org.hearthos.firehold.posture/contents/ui/couch/" || ok=0
cp "$SRC/../shell/firehold.js" "$P/plasmoids/org.hearthos.firehold.posture/contents/ui/couch/firehold.js" || ok=0
rm -f "$P/plasmoids/org.hearthos.firehold.posture/contents/ui/couch/main.qml"
cp -a "$SRC/app" "$P/plasmoids/org.hearthos.firehold.app"
# The app menus' card is the Couch's too (MenuCard.qml, shared with Firehold on labwc): the Couch's files next to it.
mkdir -p "$P/plasmoids/org.hearthos.firehold.app/contents/ui/couch"
cp "$SRC"/couch/contents/ui/*.qml "$SRC"/couch/contents/ui/*.js "$P/plasmoids/org.hearthos.firehold.app/contents/ui/couch/" || ok=0
cp "$SRC/../shell/firehold.js" "$P/plasmoids/org.hearthos.firehold.app/contents/ui/couch/firehold.js" || ok=0
rm -f "$P/plasmoids/org.hearthos.firehold.app/contents/ui/couch/main.qml"
# The Desk: Plasma's own desktop shell under Firehold's name, with Firehold's first layout and none of the desktop
# shell's update scripts (they're for plain Plasma's own panels).
if [ -f "$P/shells/org.kde.plasma.desktop/metadata.json" ]; then
  cp -a "$P/shells/org.kde.plasma.desktop" "$P/shells/org.hearthos.firehold.desk"
  rm -rf "$P/shells/org.hearthos.firehold.desk/contents/updates"
  cp "$SRC/desk/layout.js" "$P/shells/org.hearthos.firehold.desk/contents/layout.js"
  python3 - "$P/shells/org.kde.plasma.desktop/metadata.json" "$SRC/desk/metadata.json" "$P/shells/org.hearthos.firehold.desk/metadata.json" <<'PY' || ok=0
import json, sys
kde, ours, out = (json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), sys.argv[3])
kde.setdefault('KPlugin', {})
for k in ('Id', 'Name', 'Description', 'Authors', 'Version'):
    kde['KPlugin'][k] = ours['KPlugin'][k]
for k in list(kde['KPlugin']):
    if k.startswith('Name[') or k.startswith('Description['):
        del kde['KPlugin'][k]
json.dump(kde, open(out, 'w'), indent=4)
PY
else
  echo "firehold-plasma: Plasma's desktop shell isn't installed: no Firehold Desk (the Desk is plain KDE Plasma)"
  ok=0
fi
for f in "$SRC"/apps/org.hearthos.firehold.*.desktop; do install -m644 "$f" "$ROOT/usr/share/applications/$(basename "$f")"; done
install -m644 "$SRC/apps/hearth-firehold-plasma.autostart.desktop" "$ROOT/etc/xdg/autostart/hearth-firehold-plasma.desktop"
chmod 755 "$SRC/hearth-firehold-plasma"
ln -sfn "${HELPER_PATH:-/usr/share/nimbus/desktop/firehold-plasma/hearth-firehold-plasma}" "$ROOT/usr/bin/hearth-firehold-plasma"
[ "$ok" = 1 ] && echo "firehold-plasma: installed" || echo "firehold-plasma: installed with gaps (see above)"
exit 0
