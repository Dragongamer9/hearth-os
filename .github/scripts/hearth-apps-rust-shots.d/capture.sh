# Capture (hosts/hearth-ui/apps/capture; docs/mockups/checklists/capture-rust.md): every page, the editor with each
# panel, the area picker, the countdown, the scrolling capture's pill and the capture dot. Sourced by
# hearth-apps-rust-test.sh inside its headless screen (sway): there the helper takes real pictures with grim, so
# capture-16/17 are real screenshots; the others use a fixture picture made here.
CAPF=/tmp/capture-fixture
mkdir -p "$CAPF"
export HEARTH_CAPTURE_HELPER="$HEARTH_SHELL_DIR/hearth-capture-helper"

# The first, measured at rest: Take, nothing saved yet (opening it must write nothing).
run capture-1-take hearth-capture 20 "$BIN/hearth-capture-app"

# Fixtures: a picture like a page (text, a QR-ish block, colour), and three saved screenshots with their recent list.
python3 - "$CAPF" "$HOME" <<'PY'
import json, os, sys, time
from PIL import Image, ImageDraw, ImageFont
out, home = sys.argv[1], sys.argv[2]
def font(size, bold=False):
    for f in ('/usr/share/fonts/google-noto-vf/NotoSans[wght].ttf', '/usr/share/fonts/google-noto/NotoSans-%s.ttf' % ('Bold' if bold else 'Regular'),
              '/usr/share/fonts/dejavu-sans-fonts/DejaVuSans%s.ttf' % ('-Bold' if bold else '')):
        if os.path.exists(f):
            return ImageFont.truetype(f, size)
    return ImageFont.load_default()
def page(w, h, title, bg, accent):
    im = Image.new('RGB', (w, h), bg)
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, w, 64], fill=accent)
    d.text((32, 16), 'Hearth  ·  ' + title, fill='white', font=font(26, True))
    d.text((48, 120), title, fill='#1b1f24', font=font(48, True))
    for i, line in enumerate(['Your screenshot, marked up and shared.', 'Arrows, boxes, a pen, a highlighter and text;',
                              'blur and pixelate hide what nobody should see;', 'numbered steps show the way.']):
        d.text((48, 210 + i * 44), line, fill='#333a44', font=font(28))
    d.rounded_rectangle([w - 520, 140, w - 60, 460], radius=18, fill='#ffffff', outline='#c9cfd6', width=2)
    for y in range(8):
        for x in range(8):
            if (x * 3 + y * 5) % 7 < 3:
                d.rectangle([w - 480 + x * 30, 170 + y * 30, w - 454 + x * 30, 196 + y * 30], fill='#1b1f24')
    d.text((w - 220, 200), 'Card number', fill='#5b6470', font=font(22))
    d.text((w - 220, 236), '4921 5583', fill='#1b1f24', font=font(28, True))
    d.rectangle([48, h - 220, w - 60, h - 60], fill='#e8eef6')
    d.text((72, h - 190), 'Click Save to keep your changes', fill='#1b1f24', font=font(30))
    return im
page(1600, 900, 'Share me with your phone', '#f6ede4', '#c2410c').save(os.path.join(out, 'shot.png'))
shots = os.path.join(home, 'Pictures', 'Screenshots')
os.makedirs(shots, exist_ok=True)
items, now = [], int(time.time())
for i, (t, bg, ac) in enumerate([('Game night', '#e6eefb', '#2563eb'), ('Settings', '#eef6ea', '#15803d'), ('Photos', '#f7e9f1', '#be185d')]):
    p = os.path.join(shots, 'Screenshot 2026-11-18 2%d-1%d-0%d.png' % (i, i, i))
    page(1600, 900, t, bg, ac).save(p)
    items.append({'path': p, 'time': now - [40, 600, 7200][i], 'width': 1600, 'height': 900})
st = os.path.join(home, '.local', 'state', 'nimbus')
os.makedirs(st, exist_ok=True)
json.dump({'items': items}, open(os.path.join(st, 'capture-history.json'), 'w'))
PY
SHOT="$CAPF/shot.png"

run capture-2-take-recent hearth-capture 0 "$BIN/hearth-capture-app"
run capture-3-recent hearth-capture 0 "$BIN/hearth-capture-app" --page recent
run capture-4-edit hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --sample
run capture-5-edit-share hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --sample --show share
run capture-6-edit-saveas hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --sample --show saveas
run capture-7-edit-text hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --show text
run capture-8-edit-qr hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --show qr
run capture-9-edit-crop-tool hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --crop 120,90,1100,600 --tool crop
run capture-10-edit-cropped hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --crop 120,90,1100,600 --sample
run capture-11-picker hearth-capture 0 "$BIN/hearth-capture-app" --pick "$SHOT" --sel 300,180,560,320
run capture-12-picker-start hearth-capture 0 "$BIN/hearth-capture-app" --pick "$SHOT"
run capture-13-countdown hearth-capture-pill 0 "$BIN/hearth-capture-app" --take all --delay 10
run capture-14-scroll-pill hearth-capture-pill 0 "$BIN/hearth-capture-app" --scroll-pill
# The capture dot while a recording runs and someone is connected (the two helpers' runtime files).
mkdir -p "$XDG_RUNTIME_DIR/hearth-record" "$XDG_RUNTIME_DIR/hearth-vnc"
printf '{"state": "recording", "since": %s}\n' "$(( $(date +%s) - 83 ))" > "$XDG_RUNTIME_DIR/hearth-record/state.json"
printf '{"clients": [{"name": "Pixel 8", "ip": "192.168.1.20"}]}\n' > "$XDG_RUNTIME_DIR/hearth-vnc/clients.json"
run capture-15-indicator hearth-capture-dot 0 "$BIN/hearth-capture-app" --indicator
rm -f "$XDG_RUNTIME_DIR/hearth-record/state.json" "$XDG_RUNTIME_DIR/hearth-vnc/clients.json"
# Real pictures through Mantel's helper (grim here): this screen, with another app on it, into the editor; and an
# area, which opens the picker over a still picture of the screen.
"$BIN/hearth-task-manager" --page performance:cpu >/tmp/capture-bg.log 2>&1 &
bgpid=$!
sleep 2
run capture-16-real-screen hearth-capture 0 "$BIN/hearth-capture-app" screen
run capture-17-real-area hearth-capture 0 "$BIN/hearth-capture-app" area
kill $bgpid 2>/dev/null; wait $bgpid 2>/dev/null
# Save and copy: a new file in Pictures › Screenshots, full size, with the markup, and on the recent list.
before=$(ls "$HOME/Pictures/Screenshots" | wc -l)
run capture-18-edit-saved hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --sample --save
python3 - "$HOME" "$before" <<'PY'
import json, os, sys
from PIL import Image
home, before = sys.argv[1], int(sys.argv[2])
d = os.path.join(home, 'Pictures', 'Screenshots')
names = sorted(os.listdir(d), key=lambda n: os.path.getmtime(os.path.join(d, n)))
if len(names) <= before:
    print('::error title=capture-18-edit-saved::Save and copy made no new file in Pictures/Screenshots (%s)' % names)
    sys.exit(0)
p = os.path.join(d, names[-1])
im = Image.open(p).convert('RGB')
# The sample's red box starts at 8 % across, 12 % down: its edge is red at full size.
x, y = int(1600 * 0.08), int(900 * 0.25)
r, g, b = max((im.getpixel((x + dx, y)) for dx in range(-4, 5)), key=lambda c: c[0] - c[1])
hist = json.load(open(os.path.join(home, '.local', 'state', 'nimbus', 'capture-history.json')))
on_list = any(i.get('path') == p for i in hist.get('items', []))
ok = im.size == (1600, 900) and r > 200 and g < 120 and on_list
print('::%s title=capture-18-edit-saved::%s %dx%d, the box edge %s, on the recent list: %s' % ('notice' if ok else 'error', names[-1], im.size[0], im.size[1], (r, g, b), on_list))
PY
# Opening Capture wrote nothing of its choices (red flag 9): no capture.json, as nothing was changed.
[ -e "$HOME/.config/nimbus/capture.json" ] && echo "::error title=capture::opening Capture wrote capture.json: $(cat "$HOME/.config/nimbus/capture.json")"
# Hearth's own look, light.
look hearth light
run capture-19-light-take hearth-capture 0 "$BIN/hearth-capture-app"
run capture-20-light-edit hearth-capture 0 "$BIN/hearth-capture-app" --edit "$SHOT" --sample
look steam dark
unset HEARTH_CAPTURE_HELPER
