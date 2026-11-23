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
