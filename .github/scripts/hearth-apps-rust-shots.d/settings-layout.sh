# Hearth Settings › Personalization › Desktop layout (cats/pers/layout.rs; mockup docs/mockups/desktop-layouts board
# 6): the ready-made layouts from the repo (NIMBUS_SHARE), Plasma-style put in place first as Settings does it (the
# library's apply: desktop.json and bar.json merged, with Undo), then the page with the main bar picked, and with a
# panel added on the left and picked. Sourced by hearth-apps-rust-test.sh (run()).
S="$BIN/hearth-settings-app"
export NIMBUS_SHARE="$ROOT/hosts/linux"
mkdir -p "$HOME/.local/share/nimbus" "$HOME/.config/nimbus"
[ -f "$HOME/.local/share/nimbus/desktop.json" ] || printf '{"look":"hearth","mode":"dark"}\n' >"$HOME/.local/share/nimbus/desktop.json"
python3 - "$HOME" <<'PY'
import json, sys, os
h = sys.argv[1]
d = os.path.join(h, ".local/share/nimbus/desktop.json")
o = json.load(open(d))
o.update({"layout": "plasma", "barLayout": "one", "barEdge": "bottom", "dockFloat": "off", "appMenu": "start", "barStyle": "bar", "windowButtons": "icon:iconify,max,close"})
json.dump(o, open(d, "w"))
b = os.path.join(h, ".config/nimbus/bar.json")
json.dump({"items": {"start": ["menu"], "end": ["tray", "network", "volume", "battery", "notifications", "showdesktop"], "hidden": []}}, open(b, "w"))
PY
run settings-layout-01-plasma hearth-settings 0 "$S" --page desktop-layout
