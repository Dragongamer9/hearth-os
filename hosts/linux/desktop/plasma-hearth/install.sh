#!/bin/bash
# Hearth's panel on KDE Plasma (Hearth Classic on Plasma; docs/mockups/checklists/plasma-hearth-bar.md): builds
# Plasma's packages from Mantel's own files, so the bar, dock and app menus are Debian's, line for line.
#   install.sh [ROOT]      ROOT defaults to /. Used by os-bases/anatase/build.sh and .github/workflows/plasma-hearth.yml.
# Mantel's files (hosts/linux/desktop/shell) are copied with only these changes (transform.py): the Quickshell
# imports and the layer-shell lines are taken out, and Qt.resolvedUrl("x") points at Mantel's own folder (its
# helpers and pictures stay there: one copy). shim/ gives what Quickshell gave, with Plasma's own pieces.
# Installs org.hearthos.panel (the bar and the dock) and org.hearthos.appmenu (the Hearth app menu for any panel),
# the helper, and the .path unit that tells the panel when Hearth's files changed. It never changes anyone's panel:
# Hearth's panel is put in only when you pick it (hearth-plasma-panel apply). Never fails the image.
set -u
SRC=$(cd "$(dirname "$0")" && pwd)
SHELL_SRC=$(cd "$SRC/../shell" && pwd)
ROOT=${1:-/}
P="$ROOT/usr/share/plasma/plasmoids"
ok=1
mkdir -p "$P" "$ROOT/usr/lib/systemd/user" "$ROOT/usr/bin"

# Mantel's files the packages are made of (and what those need).
MANTEL="Theme.qml subthemes.js Settings.qml SettingsWriter.qml DeskState.qml Leave.qml BarLayout.qml widget-tools.js
  NetSpeed.qml NotifyRules.qml Frame.qml Icon.qml icon-data.js MenuHints.qml MenuSearch.qml MenuTile.qml MenuCentred.qml
  MenuClassic.qml MenuTiles.qml MenuStart.qml MenuStyles.qml PopMenu.qml Islands.qml Dock.qml TaskStrip.qml Launcher.qml
  DeskSwitcher.qml MediaPill.qml PerfIsland.qml Stats.qml SearchSources.qml WinPreviewTip.qml menu-nav.js menu-search.js
  QuickAccess.qml PhoneStatus.qml ClockPanel.qml MediaCard.qml StatsCard.qml calendars.js"

build() {   # build ID NAME DESCRIPTION
  local id=$1 dir="$P/$1"
  rm -rf "$dir"
  mkdir -p "$dir/contents/ui" "$dir/contents/config"
  for f in $MANTEL; do
    if [ -f "$SHELL_SRC/$f" ]; then cp "$SHELL_SRC/$f" "$dir/contents/ui/$f"; else echo "plasma-hearth: $f isn't in Mantel's folder"; ok=0; fi
  done
  cp -a "$SHELL_SRC/search" "$dir/contents/ui/search"
  cp "$SRC"/shim/*.qml "$dir/contents/ui/"
  cp "$SRC"/package/*.qml "$dir/contents/ui/"
  cp "$SRC/package/config/main.xml" "$dir/contents/config/main.xml"
  python3 "$SRC/transform.py" "$dir/contents/ui" || ok=0
  python3 - "$SRC/package/metadata.json" "$dir/metadata.json" "$id" "$2" "$3" <<'PY' || ok=0
import json, sys
m = json.load(open(sys.argv[1]))
m['KPlugin']['Id'], m['KPlugin']['Name'], m['KPlugin']['Description'] = sys.argv[3], sys.argv[4], sys.argv[5]
json.dump(m, open(sys.argv[2], 'w'), indent=4)
PY
}
build org.hearthos.panel "Hearth's panel" "Hearth's bar and dock from Debian's Desktop Mode: the islands, your apps, the app menus and Game Mode, in every Hearth look"
build org.hearthos.appmenu "Hearth's app menu" "Hearth's app menus (Hearth, Centred, Tiles, Start screen, Classic) from the Hearth flame, in any panel"

mkdir -p "$ROOT/usr/share/nimbus/desktop/plasma-hearth"
if [ "$(cd "$ROOT/usr/share/nimbus/desktop/plasma-hearth" && pwd)" != "$SRC" ]; then
  install -m755 "$SRC/hearth-plasma-panel" "$ROOT/usr/share/nimbus/desktop/plasma-hearth/hearth-plasma-panel"
else
  chmod 755 "$SRC/hearth-plasma-panel"
fi
ln -sfn /usr/share/nimbus/desktop/plasma-hearth/hearth-plasma-panel "$ROOT/usr/bin/hearth-plasma-panel"
install -m644 "$SRC/hearth-plasma-panel.path" "$SRC/hearth-plasma-panel.service" "$ROOT/usr/lib/systemd/user/"
mkdir -p "$ROOT/usr/lib/systemd/user/plasma-workspace.target.wants"
ln -sfn ../hearth-plasma-panel.path "$ROOT/usr/lib/systemd/user/plasma-workspace.target.wants/hearth-plasma-panel.path"
[ "$ok" = 1 ] && echo "plasma-hearth: installed" || echo "plasma-hearth: installed with gaps (see above)"
exit 0
