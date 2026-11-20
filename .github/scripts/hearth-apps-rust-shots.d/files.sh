# Hearth Files and Hearth's file chooser (hosts/hearth-ui/apps/files; docs/mockups/checklists/files-rust.md): every
# view and dialog, pictured against the approved mockups (docs/mockups/next-update/files-*.png, cdrive-1, cdrive-3).
# The home folder is made like the mockups' (Desktop … Videos, swaptest.py, swapfill.py, ten pictures, a download or
# three, recent files). Sourced by hearth-apps-rust-test.sh inside its headless screen: run NAME APP_ID REST BINARY ARGS…
H="$HOME"
mkdir -p "$H/Desktop" "$H/Documents" "$H/Downloads/Telegram" "$H/Music" "$H/Pictures" "$H/Videos" \
  "$H/Saved Games/Ember Trails" "$H/Saved Games/Pixel Rally" "$H/Saved Games/Quad Mech" "$H/Projects/hearth-os/docs" "$H/Templates"
head -c 1900 /dev/zero | tr '\0' '#' > "$H/swaptest.py"
head -c 2100 /dev/zero | tr '\0' '#' > "$H/swapfill.py"
head -c 49000 /dev/zero > "$H/Desktop/Budget.ods"
head -c 217000 /dev/zero > "$H/Downloads/invoice-0412.pdf"
truncate -s 4400000000 "$H/Downloads/hearth-os-1.10.0-peppermint.iso" 2>/dev/null || true
cp "$H/swaptest.py" "$H/Downloads/swaptest.py"
printf '# Roadmap\n' > "$H/Projects/hearth-os/docs/ROADMAP.md"
python3 - "$H" <<'PY' || echo "::warning title=files pictures::the sample pictures weren't made"
import os, sys, time
from PIL import Image
h = sys.argv[1]
pics = [("mountains.jpg", (88, 70, 170)), ("fox.png", (40, 110, 90)), ("hearth-wall-04.jpg", (160, 90, 40)), ("screenshot 04-01.png", (40, 80, 150)),
        ("aurora.jpg", (130, 40, 110)), ("forest.webp", (50, 110, 40)), ("sunset.jpg", (150, 120, 40)), ("lake.jpg", (40, 100, 120)),
        ("dragon.png", (120, 40, 40)), ("city-night.jpg", (60, 60, 120))]
for name, c in pics:
    w, hh = (3840, 2160) if name == "hearth-wall-04.jpg" else (960, 540)
    small = Image.new("RGB", (16, 16))
    px = small.load()
    for yy in range(16):
        f = yy / 15
        row = tuple(int(c[i] * (1 - f) + 24 * f) for i in range(3))
        for xx in range(16):
            g = xx / 15
            px[xx, yy] = tuple(int(row[i] * (1 - 0.35 * g)) for i in range(3))
    im = small.resize((w, hh), Image.BILINEAR)
    p = os.path.join(h, "Pictures", name)
    if name.endswith(".jpg"):
        ex = Image.Exif()
        ex[36867] = "2026:11:10 23:55:00"
        ex[306] = "2026:11:10 23:55:00"
        im.save(p, "JPEG", quality=80, exif=ex)
    elif name.endswith(".webp"):
        im.save(p, "WEBP", quality=70)
    else:
        im.save(p, "PNG")
im = Image.new("RGB", (800, 500), (30, 30, 40))
im.save(os.path.join(h, "Downloads", "task-manager-memory.png"))
# The dates the mockups show (this year).
y = time.localtime().tm_year
def at(path, mon, day, hh, mm):
    t = time.mktime((y, mon, day, hh, mm, 0, 0, 0, -1))
    os.utime(os.path.join(h, path), (t, t))
for p, d in [("Desktop", (11, 11, 21, 40)), ("Documents", (11, 12, 2, 10)), ("Downloads", (11, 12, 4, 12)), ("Music", (11, 2, 18, 3)),
             ("Pictures", (11, 10, 23, 55)), ("Saved Games", (11, 11, 20, 1)), ("Videos", (11, 8, 14, 20)), ("swaptest.py", (11, 11, 21, 11)),
             ("swapfill.py", (11, 11, 21, 17)), ("Downloads/Telegram", (11, 11, 21, 40)), ("Downloads/invoice-0412.pdf", (11, 8, 14, 2)),
             ("Downloads/hearth-os-1.10.0-peppermint.iso", (11, 9, 4, 12)), ("Downloads/swaptest.py", (11, 11, 21, 11))]:
    try:
        at(p, *d)
    except OSError:
        pass
# Recent files (recently-used.xbel), as on the Home page mockup.
now = time.time()
def iso(t):
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(t))
items = [("Projects/hearth-os/docs/ROADMAP.md", now - 600), ("Downloads/task-manager-memory.png", now - 900), ("swapfill.py", now - 86400 - 3600)]
xb = ['<?xml version="1.0" encoding="UTF-8"?>', '<xbel version="1.0">']
for p, t in items:
    xb.append('<bookmark href="file://%s/%s" added="%s" modified="%s" visited="%s"></bookmark>' % (h, p.replace(" ", "%20"), iso(t), iso(t), iso(t)))
xb.append('</xbel>')
os.makedirs(os.path.join(h, ".local/share"), exist_ok=True)
open(os.path.join(h, ".local/share/recently-used.xbel"), "w").write("\n".join(xb))
PY
# Opening Files writes nothing of its own (red flag 9): files.json only appears when something is changed.
rm -f "$H/.config/nimbus/files.json"

# The file manager: measured at rest on the first (CPU, memory, first frame), then each view and dialog.
run files-1-details hearth-files 20 "$BIN/hearth-files-app" "$H" --view details
[ -e "$H/.config/nimbus/files.json" ] && echo "::error title=files::opening Hearth Files wrote files.json: $(head -c 300 "$H/.config/nimbus/files.json")"
# Opened with nothing asked (the menu's Files) while the C: view is on (the default): This PC, like File Explorer
# (the owner, Nov 19); no Gallery and no separate Home server in the left menu (Network has them).
run files-0-first-window hearth-files 0 "$BIN/hearth-files-app"
run files-2-home hearth-files 0 "$BIN/hearth-files-app" --page home
run files-3-thispc hearth-files 0 "$BIN/hearth-files-app" --page thispc
run files-4-pictures-pane hearth-files 0 "$BIN/hearth-files-app" "$H/Pictures" --view large --show tabs,pane,select
run files-5-right-click hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show menu
run files-6-view-menu hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show view
run files-7-properties hearth-files 0 "$BIN/hearth-files-app" "$H/Desktop" --view details --show props
run files-8-cdrive hearth-files 0 "$BIN/hearth-files-app" --page c
run files-9-program-files hearth-files 0 "$BIN/hearth-files-app" --page "hearth:c/Program Files"
run files-10-xlarge hearth-files 0 "$BIN/hearth-files-app" "$H/Pictures" --view xlarge
run files-11-medium hearth-files 0 "$BIN/hearth-files-app" "$H" --view grid
run files-12-tiles hearth-files 0 "$BIN/hearth-files-app" "$H" --view tiles
run files-13-list hearth-files 0 "$BIN/hearth-files-app" "$H" --view compact
run files-14-rename hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show rename
run files-15-downloads hearth-files 0 "$BIN/hearth-files-app" "$H/Downloads" --view details
# The Open / Save window (mockup cdrive-1), from a request like hearth-portal's.
cat > /tmp/files-save.json <<JSON
{"kind": "save", "title": "Save As – Firefox", "accept": "Save", "folder": "$H/Downloads", "name": "recipe-notes.txt", "filters": [], "choices": []}
JSON
cat > /tmp/files-open.json <<JSON
{"kind": "open", "title": "Open – GIMP", "accept": "Open", "multiple": true, "folder": "$H/Pictures", "filters": [{"name": "Pictures", "globs": ["*.jpg", "*.png", "*.webp"], "mimes": []}, {"name": "All files", "globs": ["*"], "mimes": []}], "choices": []}
JSON
run chooser-1-save hearth-file-chooser 20 "$BIN/hearth-file-chooser" /tmp/files-save.json
run chooser-2-open hearth-file-chooser 0 "$BIN/hearth-file-chooser" /tmp/files-open.json
[ -e "$H/.config/nimbus/files.json" ] && echo "::error title=files::the pictures' runs wrote files.json: $(head -c 300 "$H/.config/nimbus/files.json")"
look hearth light
run files-16-hearth-light hearth-files 0 "$BIN/hearth-files-app" "$H" --view details
look steam dark

# ---------------------------------------------------------------- Mantel's Files beyond the mockups (made after the
# mockups' pictures, so those stay as the mockups show them): the wider left menu, This PC with a nearly full drive and
# network locations, the terminal panel, split view, tags and git badges, Act as administrator, Sharing, the Briefcase,
# shared folders, drag and drop, and the Open / Save window's keyboard in Game Mode.
FX="$ROOT/.github/scripts/hearth-apps-rust-shots.d/files-fixture.py"
SD="$HEARTH_SHELL_DIR"
run files-17-wide-nav hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show widenav
run files-18-thispc-full hearth-files 0 env HEARTH_FILES_HELPER="$FX" "$BIN/hearth-files-app" --page thispc
run files-19-terminal hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show term
run files-20-split hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show split,select
# A git repository (one change, one new file, one added) with tagged files (user.xdg.tags, as Dolphin keeps them).
G="$H/Projects/hearth-os"
printf 'echo build\n' > "$G/build.sh"; printf '# Hearth OS\n' > "$G/README.md"; mkdir -p "$G/os"; printf 'x\n' > "$G/os/kernel.conf"
( cd "$G" && git init -q && git add docs/ROADMAP.md build.sh && git -c user.name=CI -c user.email=ci@invalid commit -qm start \
  && printf '## 1.16\n' >> docs/ROADMAP.md && git add os/kernel.conf ) >/dev/null 2>&1 || echo "::warning title=files git::the sample repository wasn't made"
for t in "Work:$G/README.md" "Important:$G/build.sh" "Work:$G/build.sh" "Taxes:$H/Downloads/invoice-0412.pdf"; do
  python3 "$SD/hearth-files" tag-set --add "${t%%:*}" "${t#*:}" >/dev/null 2>&1 || echo "::warning title=files tags::couldn't tag ${t#*:}"
done
run files-21-tags-git hearth-files 0 "$BIN/hearth-files-app" "$G" --view details --show pane
run files-22-tags-git-large hearth-files 0 "$BIN/hearth-files-app" "$G" --view large
run files-23-tag-menu hearth-files 0 "$BIN/hearth-files-app" "$G" --view details --show tagmenu
run files-24-admin hearth-files 0 "$BIN/hearth-files-app" /usr/share --view details --show admin
run files-25-sharing hearth-files 0 env HEARTH_FILES_HELPER="$FX" "$BIN/hearth-files-app" "$H" --view details --show sharing
# A Briefcase in Documents holding three reports: one up to date, one newer at the original, one newer in the Briefcase,
# and a file of its own that isn't linked to anything.
R="$H/Documents/Reports"; B="$H/Documents/Work Briefcase"
mkdir -p "$R"; printf 'Q3\n' > "$R/report-q3.txt"; printf 'notes\n' > "$R/notes.txt"; printf '# Plan\n' > "$R/plan.md"
python3 "$SD/hearth-briefcase" new "$H/Documents" "Work Briefcase" >/dev/null 2>&1 \
  && python3 "$SD/hearth-briefcase" add "$B" "$R/report-q3.txt" "$R/notes.txt" "$R/plan.md" >/dev/null 2>&1 \
  || echo "::warning title=files briefcase::the sample Briefcase wasn't made"
sleep 1.2
printf 'more notes\n' >> "$R/notes.txt"; printf 'step two\n' >> "$B/plan.md"; printf 'scratch\n' > "$B/scratch.txt"
run files-26-briefcase hearth-files 0 "$BIN/hearth-files-app" "$B" --view details
run files-27-bc-update hearth-files 0 "$BIN/hearth-files-app" "$B" --view details --show bc-update-all
run files-28-bc-unlock hearth-files 0 "$BIN/hearth-files-app" "$B" --view details --show bc-password
run files-29-bc-setlock hearth-files 0 "$BIN/hearth-files-app" "$B" --view details --show bc-setlock
run files-30-bc-customize hearth-files 0 "$BIN/hearth-files-app" "$B" --view details --show bc-customize
run files-31-bc-menu hearth-files 0 "$BIN/hearth-files-app" "$B" --view details --show menu-bc
run files-32-bc-addto hearth-files 0 "$BIN/hearth-files-app" "$R" --view details --show bc-addto
run files-33-bc-icons hearth-files 0 "$BIN/hearth-files-app" "$H/Documents" --view large
run files-34-netsign hearth-files 0 env HEARTH_FILES_HELPER="$FX" "$BIN/hearth-files-app" "$H" --view details --show netsign
run files-35-drag hearth-files 0 "$BIN/hearth-files-app" "$H" --view details --show drag
# Game Mode's Open / Save window: Hearth's keyboard inside it (hearth-portal sets HEARTH_PICKER_PAD=1 over a game).
run chooser-3-keyboard hearth-file-chooser 0 env HEARTH_PICKER_PAD=1 "$BIN/hearth-file-chooser" /tmp/files-save.json --show osk
# hearth-portal itself opening it where there's no Quickshell (the Fedora edition): its own run_picker, as an app's
# Save request reaches it over D-Bus (test_hearth_portal.py checks the D-Bus side and Plasma's kde-portals.conf).
: > /tmp/files-rust.ok
cat > /tmp/portal-pick.py <<'PY'
import importlib.machinery, importlib.util, json, os, signal, sys
signal.signal(signal.SIGTERM, lambda *a: sys.exit(0))
path = os.path.join(os.environ['HEARTH_SHELL_DIR'], 'hearth-portal')
loader = importlib.machinery.SourceFileLoader('hearth_portal', path)
spec = importlib.util.spec_from_loader('hearth_portal', loader)
hp = importlib.util.module_from_spec(spec)
loader.exec_module(hp)
req = json.load(open('/tmp/files-save.json'))
req['title'] = 'Save As – Text Editor'
print(json.dumps(hp.run_picker(req, env=dict(os.environ), game=False)))
PY
run chooser-4-portal hearth-file-chooser 0 env HEARTH_FILES_RUST_OK=/tmp/files-rust.ok PATH="$BIN:$PATH" python3 /tmp/portal-pick.py
