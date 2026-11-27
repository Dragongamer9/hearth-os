# Store apps (the owner, Nov 27: "Forge isn't a built in app also it shows it's installed when it's not"): on a PC on
# Hearth OS 1.18 whose image doesn't have Hearth Forge, the Store shows it as a Hearth app to Get (not built in, not
# installed); once got (its Store copy in the home folder, as hearth-part leaves it), it's installed with Open and
# Uninstall. Both looks (the mouse one and the controller one). Pretend numbers, nothing released.
# Sourced by hearth-apps-rust-test.sh inside its headless screen.
SF=/tmp/store-fixture
[ -f "$SF/catalog.json" ] || python3 "$ROOT/.github/scripts/hearth-store-fixture.py" "$SF" "$ROOT/os/system_files/usr/share/hearth/store/catalog.json" || echo "::error title=store fixture::it couldn't be made"
if ! type store_home >/dev/null 2>&1; then
  STORE_LOOK="$HOME/.local/share/nimbus/desktop.json"
  store_home() { # NAME [WORLD]
    export HOME_BEFORE_STORE="${HOME_BEFORE_STORE:-$HOME}"
    export HOME="$SF/$1" XDG_DATA_HOME="$SF/$1/data" XDG_CONFIG_HOME="$SF/$1/config" XDG_CACHE_HOME="$SF/$1/cache"
    export HEARTH_DESKTOP_JSON="$STORE_LOOK" HEARTH_STORE_FIXTURE="$SF/${2:-.}" HEARTH_STORE_CATALOG="$SF/catalog.json"
  }
  store_done() {
    export HOME="$HOME_BEFORE_STORE"
    unset XDG_DATA_HOME XDG_CONFIG_HOME XDG_CACHE_HOME HEARTH_DESKTOP_JSON HEARTH_STORE_FIXTURE HEARTH_STORE_CATALOG HOME_BEFORE_STORE
  }
fi
GW="$SF/get-world"
rm -rf "$GW"
mkdir -p "$GW"
for f in releases.json fuse; do [ -e "$SF/$f" ] && cp -a "$SF/$f" "$GW/"; done
echo '{"hearth": "1.18", "edition": "Debian edition"}' > "$GW/this-pc.json"
# Hearth Forge isn't in this image (catalog.rs in_image reads this in the pretend world)
echo hearth-forge-app > "$GW/not-in-image"
python3 - "$ROOT/os/system_files/usr/share/hearth/store/catalog.json" "$GW" <<'PY'
import json, os, sys
d = json.load(open(sys.argv[1])); w = sys.argv[2]
for a in d["apps"]:
    if a["id"] == "forge":
        a["kind"], a["inImage"] = "app", False
        a["part"] = {"id": "hearth-forge-app", "version": "1.18.1", "os": "1.18", "path": "hearth-forge-app", "sha256": "0" * 64, "size": 11 * 1024 * 1024,
                     "url": "https://github.com/Dragongamer9/hearth-os/releases/download/part-hearth-forge-app-1.18.1/hearth-forge-app-1.18.1.tar.gz",
                     "notes": ["Make a model from a photo or a few words, on your own Nvidia PC", "Send it to Hearth 3D or your printer's slicer"]}
json.dump(d, open(os.path.join(w, "catalog.json"), "w"), indent=1)
# After Get: its Store copy as hearth-part leaves it
v = os.path.join(w, "parts", "hearth-forge-app", "1.18.1")
os.makedirs(v, exist_ok=True)
open(os.path.join(v, "hearth-forge-app"), "w").write("#!/bin/sh\n")
os.chmod(os.path.join(v, "hearth-forge-app"), 0o755)
json.dump({"id": "hearth-forge-app", "version": "1.18.1", "os": "1.18", "path": "hearth-forge-app", "installed": 1795564800},
          open(os.path.join(w, "parts", "hearth-forge-app", "part.json"), "w"))
os.makedirs(os.path.join(w, "state"), exist_ok=True)
os.makedirs(os.path.join(w, "none"), exist_ok=True)
PY
store_home home-g get-world
export HEARTH_STORE_CATALOG="$GW/catalog.json" HEARTH_PARTS="$GW/none" HEARTH_PARTS_STATE="$GW/state"
run store-get-1-forge-before hearth-store 0 "$BIN/hearth-store-app" --page apps --app forge
run store-get-2-forge-before-controller hearth-store 0 "$BIN/hearth-store-app" --controller --page apps --app forge
run store-get-5-parts-without-forge hearth-store 0 "$BIN/hearth-store-app" --page parts
export HEARTH_PARTS="$GW/parts"
run store-get-3-forge-after hearth-store 0 "$BIN/hearth-store-app" --page apps --app forge
run store-get-4-forge-after-controller hearth-store 0 "$BIN/hearth-store-app" --controller --page apps --app forge
run store-get-6-library-after hearth-store 0 "$BIN/hearth-store-app" --page library
run store-get-7-uninstall hearth-store 0 "$BIN/hearth-store-app" --page apps --app forge --show uninstall:forge
unset HEARTH_PARTS HEARTH_PARTS_STATE
store_done
