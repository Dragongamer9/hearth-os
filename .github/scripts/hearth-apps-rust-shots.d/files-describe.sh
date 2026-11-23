# Describe it in Hearth Files (desktop-layouts board 11; hosts/hearth-layout app.rs, hosts/hearth-ui src/describe.rs,
# apps/files ldescribe.rs): its own Downloads with pictures, old files and an installer. Sourced by hearth-apps-rust-test.sh.
H="$HOME"
mkdir -p "$H/Downloads/Installers" "$H/Downloads/Pictures" "$H/Downloads/Videos" "$H/Downloads/Documents"
python3 - "$H/Downloads" <<'PY' || true
import os, sys, time
from PIL import Image
d = sys.argv[1]
for i, c in enumerate([(200, 80, 110), (60, 110, 200), (90, 160, 90)]):
    Image.new("RGB", (640, 360), c).save(os.path.join(d, f"console-{i}.png"))
old = time.time() - 60 * 86400
for n in ["mod.zip", "notes-2025.txt", "driver.run"]:
    p = os.path.join(d, n); open(p, "wb").write(b"x" * 2048); os.utime(p, (old, old))
PY

# Describe it in Hearth Files (desktop-layouts board 11): Downloads with the sidebar icons only, the preview on the
# right and bigger, and the new counts on its bar marked NEW, tried (nothing kept), the box at the bottom with its
# chips. The AI's answer is CI's canned one (HEARTH_DESCRIBE_FAKE: what an AI would say), checked by hearth-layout.
cat > /tmp/describe-files.json <<'JSON'
{"kind": "app", "layout": {"sidebar": "icons", "preview": "right-big"},
 "features": [{"format": 1, "id": "downloads-counts", "name": "Counts in Downloads", "part": "files:downloads",
               "shows": [{"source": "folder-pictures", "label": "Pictures here"}]},
              {"format": 1, "id": "downloads-old", "name": "Old files", "part": "files:downloads",
               "shows": [{"source": "folder-old", "label": "Older than a month"}], "click": {"action": "select-old"}}],
 "changes": ["The sidebar shows icons only", "The preview is on the right, bigger", "Downloads counts its pictures and old files"]}
JSON
SHOT_WAIT=4 run files-describe-it hearth-files 15 env PATH="$BIN:$PATH" HEARTH_DESCRIBE_FAKE=/tmp/describe-files.json "$BIN/hearth-files-app" "$H/Downloads" \
  --describe "Sidebar icons only, the preview on the right and bigger, and in Downloads add buttons that count my pictures and old files"

# The whole app rearranged (the owner, Nov 22: "they need to be able to edit the top bar as well, basicly the enditre
# app"): no tabs, the address and search alone on top, Back, Forward, Up, New, Sort and View in a bar at the bottom,
# Share and Rename hidden, icons only on the buttons; each changed bar outlined.
cat > /tmp/describe-files-2.json <<'JSON'
{"kind": "app", "layout": {"tabs": "hidden", "toolbar": "compact",
  "zones": {"top": ["address", "spacer", "search"], "bottom": ["back", "forward", "up", "sep", "new", "sort", "view", "spacer", "details"], "hidden": ["share", "rename", "refresh"]}},
 "changes": ["No tabs", "The address and search alone at the top", "Back, Forward, Up, New, Sort and View at the bottom", "Share and Rename hidden"]}
JSON
SHOT_WAIT=4 run files-describe-it-2-whole-app hearth-files 0 env PATH="$BIN:$PATH" HEARTH_DESCRIBE_FAKE=/tmp/describe-files-2.json "$BIN/hearth-files-app" "$H/Downloads" \
  --describe "No tabs, just the address and search on top, and put back, forward, up, new, sort and view in a bar at the bottom; hide share and rename"

# The window's own bar too (the owner, Nov 22: "even the top bar were the minimise maximise and the close button, The
# entire APP"): macOS-style, the buttons as coloured dots on the left, then the folder's name; the tabs gone.
cat > /tmp/describe-files-3.json <<'JSON'
{"kind": "app", "layout": {"tabs": "hidden", "winbuttons": "dots",
  "zones": {"titlebar": ["close", "minimize", "maximize", "spacer", "title", "spacer", "search"], "top": ["back", "forward", "address"]}},
 "changes": ["The window's buttons on the left as coloured dots", "The folder's name in the middle", "Search up in the window's bar"]}
JSON
SHOT_WAIT=4 run files-describe-it-3-window-bar hearth-files 0 env PATH="$BIN:$PATH" HEARTH_DESCRIBE_FAKE=/tmp/describe-files-3.json "$BIN/hearth-files-app" "$H/Downloads" \
  --describe "Make the window bar like a Mac: the close, minimise and maximise dots on the left, the folder name in the middle and search up there too"
