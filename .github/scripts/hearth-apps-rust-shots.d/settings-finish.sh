# Hearth Settings' last parts (docs/mockups/checklists/settings-rust.md, feature/settings-rust-finish), pictured:
#   App store with the apps' icons and screenshots as pictures (pretend answers from Flathub, pictures made here)
#   Gaming › Startup animation and System › Boot splash with each one's picture, the card the focus is on and the
#     preview playing (the image's real preview sheets, os/system_files/usr/share/hearth/boot-previews)
#   System › Startup with Server mode's schedule to change on the page
#   Display › Identify outside KDE Plasma (sway here): each screen's number over the page
#   Gaming › Controller (the pointer speed reaches KDE Plasma's controller mouse) and the categories with nothing
#     "Coming next"
# Sourced by hearth-apps-rust-test.sh. Opening these pages writes nothing: checked at the end.
FIN=/tmp/settings-finish
rm -rf "$FIN" && mkdir -p "$FIN/fx" "$FIN/sys" "$FIN/pics" "$FIN/movies"
FIN_PREV="$ROOT/os/system_files/usr/share/hearth/boot-previews"
for m in flame sweep reveal embers aurora starlight orbit; do : > "$FIN/movies/hearth-$m-16x9.webm"; done
python3 - "$FIN" "$ROOT" <<'PY' || echo "::error title=settings-finish::the pretend answers couldn't be made"
import json, os, subprocess, sys
fin, root = sys.argv[1], sys.argv[2]
def put(d, name, v):
    with open(os.path.join(fin, d, name), 'w') as f:
        f.write(v if isinstance(v, str) else json.dumps(v))
# ---- App store: icons and screenshots drawn here (Flathub's would be https:// addresses hearth-store saves)
from PIL import Image, ImageDraw
apps = [('org.kde.krita', 'Krita', 'Digital painting, creative freedom', 'KDE', (230, 80, 120)),
        ('org.blender.Blender', 'Blender', 'Free and open 3D creation suite', 'Blender Foundation', (240, 140, 40)),
        ('org.gimp.GIMP', 'GNU Image Manipulation Program', 'Create images and edit photographs', 'The GIMP team', (120, 110, 90)),
        ('com.obsproject.Studio', 'OBS Studio', 'Live streaming and video recording', 'OBS Project', (60, 60, 70)),
        ('org.videolan.VLC', 'VLC', 'Play anything', 'VideoLAN', (245, 130, 20)),
        ('net.lutris.Lutris', 'Lutris', 'Video game preservation platform', 'Lutris Team', (240, 160, 60)),
        ('com.discordapp.Discord', 'Discord', 'Talk, video chat and hang out', 'Discord Inc.', (88, 101, 242)),
        ('org.libreoffice.LibreOffice', 'LibreOffice', 'The LibreOffice productivity suite', 'The Document Foundation', (30, 150, 90))]
def icon(path, col, letter):
    im = Image.new('RGBA', (128, 128), (0, 0, 0, 0)); d = ImageDraw.Draw(im)
    d.rounded_rectangle([8, 8, 120, 120], 28, fill=col + (255,))
    d.ellipse([34, 34, 94, 94], fill=(255, 255, 255, 235))
    d.text((56, 54), letter, fill=col + (255,))
    im.save(path)
hits = []
for i, (aid, name, summ, dev, col) in enumerate(apps):
    p = os.path.join(fin, 'pics', aid + '.png'); icon(p, col, name[0])
    hits.append({'id': aid, 'name': name, 'summary': summ, 'icon': p, 'developer': dev, 'installs': 1000 - i, 'verified': i % 2 == 0})
cats = [('AudioVideo', 'Audio & video', 'film'), ('Development', 'Developer tools', 'kbd'), ('Education', 'Education', 'bulb'), ('Game', 'Games', 'pad'),
        ('Graphics', 'Graphics & photos', 'image'), ('Network', 'Internet', 'globe'), ('Office', 'Office', 'doc'), ('Science', 'Science', 'sparkle'),
        ('System', 'System', 'gear'), ('Utility', 'Utilities', 'box')]
put('fx', 'hearth-store-home.json', {'categories': [{'id': a, 'label': b, 'icon': c} for a, b, c in cats], 'popular': hits[:5], 'trending': hits[3:], 'new': hits[1:4], 'cached': False})
shots = []
for k, (bg, acc) in enumerate([((32, 34, 40), (230, 80, 120)), ((240, 240, 244), (60, 120, 220)), ((24, 26, 30), (120, 200, 120))]):
    im = Image.new('RGB', (1248, 702), bg); d = ImageDraw.Draw(im)
    d.rectangle([0, 0, 1248, 44], fill=tuple(min(255, c + 25) for c in bg))
    d.rectangle([0, 44, 70, 702], fill=tuple(min(255, c + 12) for c in bg))
    for j in range(8):
        d.rounded_rectangle([14, 64 + j * 60, 56, 106 + j * 60], 8, fill=acc if j == k else tuple(min(255, c + 40) for c in bg))
    d.rounded_rectangle([110, 80, 1000, 640], 12, fill=(250, 250, 250) if k != 1 else (255, 255, 255))
    d.ellipse([300 + k * 80, 200, 700 + k * 80, 560], fill=acc)
    d.rectangle([1030, 80, 1220, 640], fill=tuple(min(255, c + 20) for c in bg))
    d.text((20, 14), 'Krita  ·  screenshot %d' % (k + 1), fill=(200, 200, 200) if k != 1 else (40, 40, 40))
    p = os.path.join(fin, 'pics', 'krita-shot-%d.png' % (k + 1)); im.save(p)
    shots.append({'url': p, 'caption': 'Screenshot %d' % (k + 1)})
put('fx', 'hearth-store-app-org.kde.krita.json', {'id': 'org.kde.krita', 'name': 'Krita', 'developer': 'KDE', 'summary': 'Digital painting, creative freedom',
    'icon': hits[0]['icon'], 'verified': True, 'description': 'Krita is a professional free and open source painting program, made by artists who want to see affordable art tools for everyone.\n• Brushes, layers and filters\n• Comics, textures and concept art',
    'screenshots': shots, 'license': 'GPL-3.0-or-later', 'homepage': 'https://krita.org', 'size': 189000000, 'ratings': {'total': 412, 'average': 4.6},
    'reviews': [{'stars': 5, 'summary': 'The best for drawing', 'who': 'Sam', 'text': 'Works great with my tablet.'}], 'installed': [], 'cached': False})
# ---- Boot splash and Startup animation: the real helper's answer with the image's preview sheets
env = dict(os.environ, HEARTH_BOOT_PREVIEWS=os.path.join(root, 'os/system_files/usr/share/hearth/boot-previews'), HEARTH_MOVIES=os.path.join(fin, 'movies'))
out = subprocess.run(['python3', os.path.join(root, 'os/system_files/usr/libexec/nimbus/hearth-boot-look'), 'status'], capture_output=True, text=True, env=env, timeout=30).stdout
boot = json.loads(out.strip().splitlines()[-1])
# As on an installed PC (CI has no boot menu to change): the splashes can be changed and are there.
boot['splash']['canChange'] = True
for s in boot['splash']['presets']:
    s['available'] = True
boot['splash']['current'] = 'hearth'
n = sum(1 for s in boot['splash']['presets'] if s.get('preview', {}).get('sheet'))
print('::notice title=settings-finish boot previews::%d splashes with a sheet, %d animations, geometry %s' % (n, len(boot['movie']['list']), boot.get('preview')))
put('sys', 'boot', boot)
# ---- System: Startup with Server mode on and a schedule; Display's screens
put('sys', 'server', {'settings': {'on': True, 'schedule': {'on': True, 'weekday': [{'at': '00:00', 'mode': 'server'}, {'at': '07:00', 'mode': 'desktop'}, {'at': '18:00', 'mode': 'game'}],
                                                            'weekend': [{'at': '02:00', 'mode': 'server'}, {'at': '10:30', 'mode': 'game'}]}}, 'startIn': 'game'})
put('sys', 'k-session', 'login=restorePreviousLogout\nconfirm=true\n')
def modes(rows):
    return [{'width': w, 'height': h, 'refresh': r, 'current': c, 'preferred': c} for (w, h, r, c) in rows]
put('sys', 'screens', {'wm': 'labwc', 'outputs': [
    {'name': 'HEADLESS-1', 'model': 'LG 27GP850', 'enabled': True, 'scale': 1.0, 'position': {'x': 0, 'y': 0}, 'rotation': 'normal',
     'modes': modes([(1280, 800, 60.0, True)]), 'hdr': {'capable': False}, 'vrr': {'capable': False}}]})
put('sys', 'gm', {'hdr': False, 'has': {'hdr': False}})
put('sys', 'psr', {'supported': False})
put('sys', 'bright', '')
put('sys', 'nvidia', '')
put('sys', 'ddc', {'available': False, 'why': "ddcutil isn't installed", 'screens': []})
PY
chmod -R a+rX "$FIN"
fin_sum() { cat "$HOME/.local/share/nimbus/desktop.json" "$HOME/.config/nimbus/session.conf" "$HOME/.config/nimbus/store.json" "$HOME/.config/hearth/startup-movie" 2>/dev/null | md5sum; }
look steam dark
FIN_BEFORE=$(fin_sum)
S="$BIN/hearth-settings-app"
# App store: icons in the lists, the icon and the screenshots on an app's page (measured at rest: nothing runs).
run settings-fin-01-store hearth-settings 20 env HEARTH_SETTINGS_FIXTURES="$FIN/fx" "$S" --page store
run settings-fin-02-store-app hearth-settings 0 env HEARTH_SETTINGS_FIXTURES="$FIN/fx" "$S" --page store:app-org_kde_krita
run settings-fin-03-store-app-b hearth-settings 0 env HEARTH_SETTINGS_FIXTURES="$FIN/fx" "$S" --layout b --page store:app-org_kde_krita --focus Screenshots
# Startup animation: the cards' pictures, the one the focus is on playing, the preview playing.
fin_anim() { # NAME ARGS...
  n=$1; shift
  run "$n" hearth-settings 0 env HEARTH_LIBEXEC_DIR="$ROOT/os/system_files/usr/libexec/nimbus" HEARTH_MOVIES="$FIN/movies" HEARTH_BOOT_PREVIEWS="$FIN_PREV" "$S" "$@"
}
fin_anim settings-fin-04-animation --page startup-animation
fin_anim settings-fin-05-animation-focus --page startup-animation --focus Aurora
fin_anim settings-fin-06-animation-preview --page startup-animation:preview-aurora
# Boot splash (System): the same, from the answer above (measured at rest: no picture plays by itself).
run settings-fin-07-boot hearth-settings 20 env HEARTH_SYSTEM_FIXTURE="$FIN/sys" XDG_CURRENT_DESKTOP=KDE "$S" --page boot
run settings-fin-08-boot-focus hearth-settings 0 env HEARTH_SYSTEM_FIXTURE="$FIN/sys" XDG_CURRENT_DESKTOP=KDE "$S" --page boot --focus Aurora
run settings-fin-09-boot-preview hearth-settings 0 env HEARTH_SYSTEM_FIXTURE="$FIN/sys" XDG_CURRENT_DESKTOP=KDE "$S" --page boot:preview-hearth-starlight
# Startup: the schedule, each day's track and its changes; the focus on one (← → move, A mode, X remove).
run settings-fin-10-startup-schedule hearth-settings 0 env HEARTH_SYSTEM_FIXTURE="$FIN/sys" XDG_CURRENT_DESKTOP=KDE "$S" --page startup --focus "From 6 PM"
run settings-fin-11-startup-schedule-b hearth-settings 0 env HEARTH_SYSTEM_FIXTURE="$FIN/sys" XDG_CURRENT_DESKTOP=KDE "$S" --layout b --page startup --focus "From 10:30 AM"
# Display › Identify outside KDE Plasma: the page, then each screen's number over it (as the Identify button starts it).
"$S" --page display >/tmp/fin-identify-backdrop.log 2>&1 &
fin_bd=$!
for i in $(seq 1 300); do mapped hearth-settings && break; sleep 0.02; done
sleep 1.5
"$S" --identify HEADLESS-1=1 --for 6000 >/tmp/fin-identify.log 2>&1 &
fin_id=$!
sleep 2
grim "$ROOT/shots/settings-fin-12-identify.png" 2>/tmp/grim.log || echo "::warning title=settings-fin-12-identify::grim: $(cat /tmp/grim.log)"
kill -0 $fin_id 2>/dev/null || echo "::error title=settings-fin-12-identify::it ended early: $(tail -c 800 /tmp/fin-identify.log | tr '\n' '|')"
echo "SHOT settings-fin-12-identify firstframe=overlay" >> "$ROOT/results.txt"
sleep 4.5
kill -0 $fin_id 2>/dev/null && { echo "::error title=settings-fin-12-identify::still showing after its 6 s"; kill $fin_id; }
wait $fin_id 2>/dev/null
kill $fin_bd 2>/dev/null; wait $fin_bd 2>/dev/null
"$S" --identify 'bad name=x' >/tmp/fin-identify-bad.log 2>&1; rc=$?
[ "$rc" = 2 ] || echo "::error title=settings-finish identify::a wrong screen list gave $rc, not 2"
HEARTH_SYSTEM_FIXTURE="$FIN/sys" run settings-fin-13-display hearth-settings 0 "$S" --page display --focus Identify
sleep 0.3
# Controller (the pointer speed's line) and two categories with every page built (nothing "Coming next").
run settings-fin-14-controller hearth-settings 0 "$S" --page controller
run settings-fin-15-apps hearth-settings 0 "$S" --page apps-home
run settings-fin-16-system hearth-settings 0 "$S" --page system
# Opening these pages wrote nothing (red flag 9).
[ "$(fin_sum)" = "$FIN_BEFORE" ] || echo "::error title=settings-finish::opening a page changed desktop.json, session.conf, store.json or the startup animation's choice"
