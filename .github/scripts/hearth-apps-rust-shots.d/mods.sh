# The Mod Manager (hosts/hearth-ui/apps/mods; checklist docs/mockups/checklists/mod-manager.md, boards
# docs/mockups/mod-manager): each board's picture, on a pretend PC (mods-fixture.py: a real Hearth Mod Organizer instance
# for Skyrim SE with clashing mods, plugins with masters, profiles, snapshots, a Crash Logger log and saves) at the
# boards' size (1920x1080). Sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its.
MW=/tmp/mods-world
rm -rf "$MW"; mkdir -p "$MW"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/mods-fixture.py" "$MW/home" >/dev/null || echo "::error title=mods fixture::the pretend PC couldn't be made"
MH="$MW/home"
MM="$BIN/hearth-mods"
mm() { # NAME ARGS…: pictured in that world
  local n=$1
  shift
  run "$n" hearth-mods 0 env HOME="$MH" XDG_CONFIG_HOME="$MH/.config" XDG_DATA_HOME="$MH/.local/share" HEARTH_DESKTOP_JSON="$MH/.local/share/nimbus/desktop.json" "$@"
}
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1
before=$(cd "$MH/.local/share/Steam/steamapps/common" && find . -type f -exec md5sum {} + | sort | md5sum)

# What the organizer reads from the pretend instance (the command line the old Hearth Mods had, unchanged)
sum=$(env HOME="$MH" XDG_CONFIG_HOME="$MH/.config" XDG_DATA_HOME="$MH/.local/share" "$MM" --organizer skyrimse summary 2>&1)
echo "::notice title=mods organizer::$(echo "$sum" | python3 -c 'import json,sys
try:
  s=json.load(sys.stdin); print("mods %d · plugins %d · profiles %d · warnings %s" % (len(s.get("mods",[])), len(s.get("plugins",[])), len(s.get("profiles",[])), s.get("warnings")))
except Exception as e: print("not JSON:", str(e)[:200])' 2>&1 | cut -c1-900)"
# The FOMOD installer, read by the organizer (fomod: unpacked and read without installing), as the window keeps it open
env HOME="$MH" XDG_CONFIG_HOME="$MH/.config" XDG_DATA_HOME="$MH/.local/share" "$MM" --organizer skyrimse fomod "$MH/Downloads/Lux - Lighting" > "$MH/.local/share/nimbus/mods/manager/fomod-open.json" 2>/tmp/mods-fomod.err \
  || echo "::warning title=mods fomod::$(head -c 600 /tmp/mods-fomod.err) $(head -c 600 "$MH/.local/share/nimbus/mods/manager/fomod-open.json")"

# The devices' notes on the pretend home server, as the Mod Manager reads them
echo "::notice title=mods devices::$(ls "$MH/HomeServer/Games/Mods/.hearth/devices" 2>&1 | tr '\n' ' ')"

# LOOT's masterlist to pin (its commit and SHA-256), and Thunderstore's search, as CI sees them
for r in skyrimse fallout4; do
  c=$(curl -fsS -m 20 "https://api.github.com/repos/loot/$r/commits/v0.26" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])' 2>/dev/null)
  [ -n "$c" ] || c=$(curl -fsS -m 20 "https://api.github.com/repos/loot/$r/branches" 2>/dev/null | python3 -c 'import json,sys; b=json.load(sys.stdin); print(" ".join(x["name"]+"="+x["commit"]["sha"] for x in b)[:600])' 2>/dev/null)
  sha=$(curl -fsSL -m 30 "https://raw.githubusercontent.com/loot/$r/${c%% *}/masterlist.yaml" 2>/dev/null | sha256sum | cut -c1-64)
  echo "::notice title=mods loot $r::commit $c sha256 $sha"
done
echo "::notice title=mods thunderstore::$(curl -sS -m 20 -w ' HTTP %{http_code}' 'https://thunderstore.io/api/cyberstorm/listing/valheim/?q=map&ordering=most-downloaded' 2>&1 | head -c 700)"

SHOT_WAIT=6 mm mods-01-game "$MM" --game steam:489830 --mod "Noble Skyrim" --sort
mm mods-02-install "$MM" --game steam:489830 --page downloads --step 1
SHOT_WAIT=8 mm mods-03-find "$MM" --game steam:489830 --page find --find map
mm mods-04-updates "$MM" --game steam:489830 --page updates --snap 2
mm mods-05-crash "$MM" --game steam:489830 --page crashes
mm mods-05b-crash-proton "$MM" --game steam:489830 --page crashes --crash-tab 1
mm mods-05c-crash-what-ran "$MM" --game steam:489830 --page crashes --crash-tab 2
mm mods-06-describe "$MM" --game steam:489830 --page describe
mm mods-07-game-mode "$MM" --game steam:489830 --mod "Noble Skyrim" --fullscreen --lift
mm mods-08-every-game "$MM" --page every
mm mods-10-accounts "$MM" --page accounts
mm mods-11-modlists "$MM" --page modlists
SHOT_WAIT=4.5 mm mods-09-editing env HEARTH_ARRANGE_OPEN=mods:game HEARTH_STUDIO_STAGE=mods:none "$MM" --game steam:489830

# This PC's own note was written to the home server for the other devices (sync.rs)
[ -f "$MH/HomeServer/Games/Mods/.hearth/devices/nvidia-pc.json" ] && echo "::notice title=mods sync::$(head -c 700 "$MH/HomeServer/Games/Mods/.hearth/devices/nvidia-pc.json" | tr '\n' ' ')" || echo "::warning title=mods sync::this PC's note wasn't written to the home server"

# The game folder is never touched, and opening it wrote no setting (red flag 9)
after=$(cd "$MH/.local/share/Steam/steamapps/common" && find . -type f -exec md5sum {} + | sort | md5sum)
[ "$before" = "$after" ] || echo "::error title=mods::opening the Mod Manager changed the game's own folder"
[ -e "$MH/.config/nimbus/mods.json" ] && echo "::error title=mods::opening it wrote a setting: $(head -c 300 "$MH/.config/nimbus/mods.json")"
for n in 01-game 02-install 05-crash; do
  grep -iE "panicked|error" "/tmp/mods-$n.log" | head -3 | while read -r l; do echo "::warning title=mods $n log::$(echo "$l" | cut -c1-400)"; done
done
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
