# Your account picture in Settings' top-left corner (the owner, Nov 19, on the Ally: it showed the letter "C"
# instead): a stand-in face in ~/.face (where Accounts › Your account puts it), pictured, then taken away again.
python3 - "$HOME/.face" <<'PY' || echo "::warning title=settings face::the stand-in picture wasn't made"
import sys
from PIL import Image, ImageDraw
im = Image.new("RGB", (256, 256), (40, 120, 200))
d = ImageDraw.Draw(im)
d.ellipse((70, 40, 186, 156), fill=(250, 210, 170))
d.rectangle((40, 170, 216, 256), fill=(230, 90, 40))
im.save(sys.argv[1], "PNG")
PY
run settings-face-1-corner hearth-settings 0 "$BIN/hearth-settings-app" --layout a
rm -f "$HOME/.face"
