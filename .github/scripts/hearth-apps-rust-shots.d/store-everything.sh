# Every Hearth part in the Hearth Store, and the system's updates (1.18; mockups docs/mockups/store-everything/store-1…3,
# checklist docs/mockups/checklists/store-everything.md): Hearth parts, Library › Hearth OS and Library while updating,
# on a pretend world (this PC on Hearth OS 1.18 on Debian 13, a system update from Nov 23 out, Mantel, the home server
# helper and Hearth Sync with updates waiting). The numbers here are pretend ones for the pictures, nothing released.
# Sourced by hearth-apps-rust-test.sh inside its headless screen.
SF=/tmp/store-fixture
[ -f "$SF/catalog.json" ] || python3 "$ROOT/.github/scripts/hearth-store-fixture.py" "$SF" "$ROOT/os/system_files/usr/share/hearth/store/catalog.json" || echo "::error title=store fixture::it couldn't be made"
# (store.sh's helpers, when this file runs on its own)
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
EW="$SF/everything-world"
rm -rf "$EW"
mkdir -p "$EW"
for f in releases.json fuse; do [ -e "$SF/$f" ] && cp -a "$SF/$f" "$EW/"; done
echo '{"hearth": "1.18", "edition": "Debian edition"}' > "$EW/this-pc.json"
echo 'Debian 13' > "$EW/system-base"
python3 - "$ROOT/os/system_files/usr/share/hearth/store/catalog.json" "$EW" <<'PY'
import json, os, sys
d = json.load(open(sys.argv[1])); w = sys.argv[2]
def part(pid, v, notes, path=None, system=False):
    p = {"id": pid, "version": v, "os": "1.18", "path": path or pid, "sha256": "0" * 64, "size": 3 * 1024 * 1024, "notes": notes,
         "url": "https://github.com/Dragongamer9/hearth-os/releases/download/part-%s-%s/%s-%s.tar.gz" % (pid, v, pid, v)}
    if system:
        p["system"] = True
    return p
for a in d["apps"]:
    if a["id"] == "mantel":
        a["part"] = part("mantel", "1.18.1", ["Searching the app menu: a result always opens on the first click."], "shell")
    if a["id"] == "home-server-helper":
        a["part"] = part("hearth-home-server-helper", "1.18.1", ["fixes Ollama on Debian"], system=True)
d["system"] = {"date": "2026-11-23", "packages": 43, "fixes": "security fixes for the kernel, Mesa and Firefox", "size": 214000000,
               "changes": ["linux-image 6.12.57 → 6.12.58", "mesa 25.0.7 → 25.0.8", "firefox-esr 140.5 → 140.6", "network-manager 1.52.1 → 1.52.2",
                           "openssl 3.5.1 → 3.5.4", "libc6 2.41-12 → 2.41-13"], "next": {"version": "", "out": False}}
json.dump(d, open(os.path.join(w, "catalog.json"), "w"), indent=1)
status = {"booted": {"version": "1.18", "timestamp": "2026-11-21T04:00:00Z"}, "staged": None,
          "available": {"version": "1.18", "timestamp": "2026-11-23T04:00:00Z"}, "rollback": {"version": "1.17", "timestamp": "2026-11-19T04:00:00Z"}}
json.dump(status, open(os.path.join(w, "system-status.json"), "w"))
json.dump({"ok": True, "plugins": {"decky-hearth-sync": {"installed": "1.18.0", "builtin": "1.18.0"}, "decky-hearth-link": {"installed": None, "builtin": "1.18.0"},
           "decky-hearth-music": {"installed": None, "builtin": "1.18.0"}}, "available": {"decky-hearth-sync": "1.18.1"}}, open(os.path.join(w, "decky-status.json"), "w"))
json.dump({}, open(os.path.join(w, "system-parts.json"), "w"))
json.dump({"done": [{"what": "Mantel 1.18.0 (test)", "at": 1795392000}, {"what": "Hearth Store 1.18.0", "at": 1795390000}, {"what": "Debian updates · Nov 21", "at": 1795219200}]},
          open(os.path.join(w, "history.json"), "w"))
# Library while updating (store-3): the system update downloading, and one from before ready for the restart.
u = os.path.join(w, "updating"); os.makedirs(u, exist_ok=True)
for f in ("catalog.json", "decky-status.json", "system-parts.json", "history.json", "this-pc.json", "system-base"):
    with open(os.path.join(w, f), "rb") as s, open(os.path.join(u, f), "wb") as o:
        o.write(s.read())
status["staged"] = {"version": "1.18", "timestamp": "2026-11-23T04:00:00Z"}
json.dump(status, open(os.path.join(u, "system-status.json"), "w"))
PY
for f in releases.json fuse; do [ -e "$EW/$f" ] && cp -a "$EW/$f" "$EW/updating/"; done

store_home home-a everything-world
export HEARTH_STORE_CATALOG="$EW/catalog.json"
run store-e1-hearth-parts hearth-store 0 "$BIN/hearth-store-app" --page parts
run store-e2-hearth-os hearth-store 0 "$BIN/hearth-store-app" --page system
store_done
store_home home-a everything-world/updating
export HEARTH_STORE_CATALOG="$EW/catalog.json"
run store-e3-updating hearth-store 0 "$BIN/hearth-store-app" --page library --demo-system 148:214 --demo-done mantel,home-server-helper
store_done

# A Decky plugin's test version (red flag 13): a system part from the list's tests, with Get test versions on.
python3 - "$EW/catalog.json" <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p))
part = {"id": "decky-hearth-sync", "version": "1.18.1", "os": "1.18", "path": "decky-hearth-sync", "system": True, "sha256": "0" * 64, "size": 400000,
        "url": "https://github.com/Dragongamer9/hearth-os/releases/download/part-decky-hearth-sync-1.18.1/decky-hearth-sync-1.18.1.tar.gz",
        "notes": ["Your non-Steam games keep their artwork after Steam updates"], "missing": ["Collections sync"]}
d["tests"] = [{"id": "test-decky-hearth-sync", "name": "Hearth Sync (Decky) (test)", "short": "Hearth Sync (Decky)", "kind": "builtin", "icon": "HS",
               "from": "#5b9dff", "to": "#2563eb", "card": "in Steam's Game Mode", "group": "gamemode", "part": part}]
json.dump(d, open(p, "w"), indent=1)
PY
store_home home-d everything-world
export HEARTH_STORE_CATALOG="$EW/catalog.json"
mkdir -p "$XDG_CONFIG_HOME/nimbus"
echo '{"testVersions": true}' > "$XDG_CONFIG_HOME/nimbus/hearth-store.json"
run store-e4-decky-test hearth-store 0 "$BIN/hearth-store-app" --page tests --app test-decky-hearth-sync
rm -f "$XDG_CONFIG_HOME/nimbus/hearth-store.json"
store_done
