# Hearth Settings › System: a picture of every page (hosts/hearth-ui/apps/settings/src/cats/system*.rs). CI has no
# PipeWire, KWin, UPower or logind, so the pages read their answers from a folder of examples (HEARTH_SYSTEM_FIXTURE:
# one file per read, named as system.rs's reader names it) and nothing is run or written. On "KDE Plasma"
# (XDG_CURRENT_DESKTOP=KDE) so the Fedora edition's rows show.
SYSFIX=/tmp/settings-system-fixture
rm -rf "$SYSFIX" && mkdir -p "$SYSFIX"
python3 - "$SYSFIX" <<'PY'
import json, os, sys, time
d = sys.argv[1]
def put(name, v):
    with open(os.path.join(d, name), 'w') as f:
        f.write(v if isinstance(v, str) else json.dumps(v))
now = int(time.time())
def modes(rows):
    return [{'width': w, 'height': h, 'refresh': r, 'current': c, 'preferred': c} for (w, h, r, c) in rows]
put('screens', {'wm': 'kwin', 'outputs': [
    {'name': 'DP-1', 'model': 'LG 27GP850', 'enabled': True, 'scale': 1.25, 'position': {'x': 0, 'y': 0}, 'rotation': 'normal', 'priority': 1,
     'modes': modes([(2560, 1440, 165.0, True), (2560, 1440, 144.0, False), (2560, 1440, 59.95, False), (1920, 1080, 165.0, False), (1920, 1080, 60.0, False), (1280, 720, 60.0, False)]),
     'hdr': {'capable': True, 'on': False}, 'vrr': {'capable': True, 'policy': 'automatic'}, 'wcg': {'capable': True, 'on': False},
     'icc': {'capable': True, 'source': '', 'path': ''}, 'colorPower': {'capable': True, 'value': 'efficiency'}},
    {'name': 'HDMI-A-1', 'model': 'Dell P2419H', 'enabled': True, 'scale': 1.0, 'position': {'x': 2048, 'y': 0}, 'rotation': 'normal', 'priority': 2,
     'modes': modes([(1920, 1080, 60.0, True), (1280, 720, 60.0, False)]), 'hdr': {'capable': False}, 'vrr': {'capable': False}}],
    'tearing': {'capable': True, 'on': False}, 'mirror': {'capable': False, 'why': "KWin 6.3 can't mirror screens yet"}, 'iccProfiles': [],
    'x11Scale': {'self': True}, 'barrier': {'px': 100, 'corner': True}, 'desktops': {'mode': 'all'}, 'autoBrightness': {'capable': False}})
put('gm', {'hdr': False, 'screen': {'name': 'DP-1', 'hdr': True}, 'has': {'hdr': True}})
put('psr', {'supported': False})
put('bright', '')
put('nvidia', 'auto\n')
put('ddc', {'available': False, 'why': "ddcutil isn't installed", 'screens': []})
sink = 'alsa_output.pci-0000_00_1f.3.analog-stereo'
sinks = [{'index': 50, 'name': sink, 'description': 'Speakers (Built-in Audio)', 'properties': {'object.serial': '51', 'object.id': '50'}},
         {'index': 61, 'name': 'bluez_output.AA_BB', 'description': 'WH-1000XM5', 'properties': {'object.serial': '62', 'object.id': '61'}}]
sources = [{'index': 52, 'name': 'alsa_input.pci-0000_00_1f.3.analog-stereo', 'description': 'Microphone (Built-in Audio)', 'properties': {'object.serial': '53', 'object.id': '52'}}]
play = [{'index': 80, 'sink': 50, 'mute': False, 'volume': {'front-left': {'value_percent': '72%'}, 'front-right': {'value_percent': '72%'}}, 'properties': {'application.name': 'Firefox', 'object.id': '80'}},
        {'index': 85, 'sink': 61, 'mute': False, 'volume': {'front-left': {'value_percent': '100%'}}, 'properties': {'application.name': 'Spotify', 'object.id': '85'}}]
rec = [{'index': 90, 'source': 52, 'volume': {'mono': {'value_percent': '100%'}}, 'properties': {'application.name': 'Discord', 'object.id': '90'}}]
put('snd', '#@vol\nVolume: 0.45\n#@mic\nVolume: 0.80\n#@out\n%s\n#@in\n%s\n#@sinks\n%s\n#@sources\n%s\n' % (sink, sources[0]['name'], json.dumps(sinks), json.dumps(sources)))
put('streams', '#@play\n%s\n#@rec\n%s\n#@sinks\n%s\n#@sources\n%s\n' % (json.dumps(play), json.dumps(rec), json.dumps(sinks), json.dumps(sources)))
put('cards', {'cards': [{'name': 'alsa_card.pci-0000_00_1f.3', 'description': 'Built-in Audio', 'kind': 'analog', 'active': 'output:analog-stereo+input:analog-stereo',
                         'profiles': [{'name': 'output:analog-stereo+input:analog-stereo', 'description': 'Analog Stereo Duplex', 'available': True},
                                      {'name': 'output:analog-surround-51', 'description': 'Analog Surround 5.1 Output', 'available': True}, {'name': 'off', 'description': 'Off', 'available': True}]}]})
put('k-sound', 'overamp=false\ntheme=ocean\nenable=true\n')
put('fdo', 'yes\n')
ch = lambda i, l: {'id': i, 'label': l, 'volume': 0.45}
put('outputs', {'outputs': [{'name': sink, 'description': 'Speakers (Built-in Audio)', 'default': True, 'balance': -20,
                             'channels': [ch('front-left', 'Front left'), ch('front-center', 'Centre'), ch('front-right', 'Front right'), ch('lfe', 'Subwoofer'), ch('rear-left', 'Rear left'), ch('rear-right', 'Rear right')]},
                            {'name': 'bluez_output.AA_BB', 'description': 'WH-1000XM5', 'default': False, 'balance': 0, 'channels': [ch('front-left', 'Left'), ch('front-right', 'Right')]}]})
put('notify', {'apps': {'Discord': {'allow': True, 'sound': True, 'lockScreen': False, 'priority': 'high'}, 'Steam': {'allow': True, 'sound': False, 'lockScreen': False, 'priority': 'normal'}}})
put('desk', {'dnd': False, 'muted': []})
put('k-notify', 'until=\nfullscreen=true\nsharing=true\nmirrored=true\nsounds=\nnow=2026,11,18,20,00,00\napp.org.kde.dolphin.ShowPopups=false\napp.org.kde.dolphin.ShowInHistory=\napp.org.kde.dolphin.ShowPopupsInDndMode=\n')
put('power', {'mode': {'supported': True, 'available': ['power-saver', 'balanced', 'performance'], 'active': 'balanced'}, 'tdp': {'supported': True, 'watts': 15, 'min': 5, 'max': 30}})
put('hw', {'batteries': [{'canLimit': True, 'limit': 80, 'health': 92, 'cycles': 143}], 'saved': {'power.plugged': 'performance', 'power.unplugged': 'power-saver'}})
put('pset', {'lid': {'battery': 'suspend', 'ac': 'suspend', 'docked': 'ignore', 'present': True, 'setBy': []},
             'inhibitors': [{'what': ['idle'], 'who': 'Firefox', 'why': 'Playing video', 'mode': 'block', 'app': 'firefox'},
                            {'what': ['sleep'], 'who': 'Steam', 'why': 'Downloading a game', 'mode': 'block', 'app': 'steam'},
                            {'what': ['sleep'], 'who': 'PowerDevil', 'why': 'Saves its state first', 'mode': 'delay', 'app': 'org_kde_powerdevil'}],
             'hibernate': {'can': True, 'proven': False}, 'hybrid': {'can': True},
             'batteries': [{'path': '/org/freedesktop/UPower/devices/battery_BAT0', 'name': 'BAT0', 'percentage': 76, 'state': 'discharging', 'energyFull': 52.4,
                            'energyDesign': 57.0, 'energyRate': 8.3, 'cycles': 143, 'health': 92, 'vendor': 'ASUSTeK', 'model': 'A32-K55', 'technology': 'lithium-ion'}]})
put('canoff', 'yes\n')
put('k-power', 'ac_offon=true\nac_off=600\nac_sleepact=1\nac_sleep=1800\nac_lid=1\nac_button=1\nac_dock=true\nac_prof=performance\nbat_offon=true\nbat_off=300\nbat_sleepact=1\nbat_sleep=900\nbat_lid=1\nbat_button=1\nbat_dock=true\nbat_prof=power-saver\n')
put('upower', 'Device: /org/freedesktop/UPower/devices/battery_BAT0\n  battery\n    percentage: 76%\nDevice: /org/freedesktop/UPower/devices/mouse_hidpp_battery_0\n  model: MX Master 3S\n  mouse\n    percentage: 64%\nDevice: /org/freedesktop/UPower/devices/gaming_input_ps_controller_battery_1\n  model: DualSense Wireless Controller\n  gaming-input\n    percentage: 12%\n')
pts = lambda f: {'points': [[now - 86400 + i * 1800, round(f(i), 1), 2] for i in range(48)]}
put('hist-charge', pts(lambda i: 100 - (i % 24) * 3.2))
put('hist-rate', pts(lambda i: 6 + (i % 7) * 1.3))
G = 1073741824
put('memory', {'memory': {'total': 32 * G, 'inUse': 11 * G, 'available': 21 * G}, 'zram': {'active': True, 'used': 2 * G, 'disksize': 16 * G, 'ratio': 3.1},
               'swaps': [{'path': '/var/swap/swapfile', 'size': 8 * G, 'used': 0}], 'swapfile': {'path': '/var/swap/swapfile', 'exists': True, 'gb': 8, 'hibernate': False},
               'vm': {'swappiness': 180}, 'watermarks': {'low': int(0.025 * 32 * G)}, 'hibernateChecked': 0, 'overridden': {}, 'pressure': {'word': 'none'},
               'settings': {'zram': {'on': True, 'gb': 16, 'algo': 'zstd'}, 'keep': {'mode': 'auto'}, 'swappiness': 180, 'swapfile': {'everyday': True},
                            'guard': {'level': 'gentle', 'never': ['steam'], 'extra': ['obs'], 'checkpoint': []}},
               'recommended': {'zram': {'gb': 16}}, 'limits': {'swapfileMaxGb': 64}, 'algos': ['zstd', 'lz4', 'lzo-rle'], 'testFiles': {},
               'neverLabels': {'steam': 'Steam', 'games': 'Games', 'desktop': 'The desktop'}, 'lastClosed': [{'name': 'chrome', 'mb': 6200, 'at': now - 3600, 'paused': False}]})
put('disk', {'ok': True, 'value': {'drives': [{'name': 'This PC (C:)', 'mount': '/', 'size': 1e12, 'used': 4.1e11, 'free': 5.9e11, 'games': True},
                                              {'name': 'Games (D:)', 'mount': '/run/media/ci/Games', 'size': 2e12, 'used': 1.62e12, 'free': 3.8e11, 'games': True}],
                                   'games': [{'title': 'Cyberpunk 2077', 'store': 'Steam', 'drive': 'Games (D:)', 'size': 7.1e10}, {'title': 'Baldur\'s Gate 3', 'store': 'Steam', 'drive': 'Games (D:)', 'size': 1.5e11},
                                             {'title': 'Hades II', 'store': 'Steam', 'drive': 'This PC (C:)', 'size': 1.2e10}],
                                   'warn': {'on': True, 'gb': 15}, 'low': []}})
put('df', 'Filesystem 1B-blocks Used Mounted on\n/dev/nvme0n1p3 1000000000000 410000000000 /sysroot\n')
put('prefs', {'cdrive': True, 'linuxFolders': False, 'addressBar': 'windows', 'copyPath': 'windows', 'terminalWindows': True})
put('thispc', {'user': 'chris', 'taken': {}, 'drives': [{'key': 'uuid-games', 'letter': 'D', 'name': 'Games', 'mounted': True, 'total': 2e12, 'free': 3.8e11, 'kind': 'drive'},
                                                       {'key': 'uuid-usb', 'letter': 'E', 'name': 'USB stick', 'mounted': False, 'sub': 'Not plugged in', 'kind': 'drive'}], 'network': []})
put('deskchoice', '#@fp\nplasma\n#@has\n/usr/bin/hearth-firehold-plasma\n')
put('server', {'settings': {'on': False, 'schedule': {'on': False, 'weekday': [], 'weekend': []}}, 'startIn': 'game'})
put('k-session', 'login=restorePreviousLogout\nconfirm=true\n')
put('boot', {'splash': {'presets': [{'id': 'hearth', 'label': 'Hearth', 'about': 'The flame on black, with a thin line while it starts', 'pair': 'hearth'},
                                    {'id': 'ember', 'label': 'Ember', 'about': 'Warm embers rising, for the TV'}, {'id': 'steamos', 'label': 'Plain', 'about': "Hearth's name only"}],
                        'current': 'hearth'}, 'movie': {'list': [{'id': 'hearth', 'label': 'Hearth'}], 'current': 'steam', 'steamReady': True}})
put('vnc', {'on': True, 'running': True, 'reach': 'tailscale', 'viewOnly': False, 'password': True, 'keyring': 'unlocked', 'input': 'portal',
            'addresses': {'tailscale': {'name': 'chris-hearth.tail1234.ts.net', 'ip': '100.64.0.5'}, 'home': ['192.168.1.20']}, 'port': 5900,
            'fingerprint': '5A:1F:9C:22:7B:40:EE:03', 'clients': [{'id': 1, 'name': "Chris's iPad", 'ip': '100.64.0.9', 'via': 'tailscale', 'since': now - 720, 'viewOnly': False}], 'tips': []})
put('time', {'timezone': 'America/Chicago', 'ntp': True, 'synced': True, 'canNtp': True, 'available': True, 'now': '2026-11-18T20:15:00-06:00', 'admin': True})
zones = [('America/Chicago', 'America', 'Chicago', '-06:00'), ('America/New_York', 'America', 'New York', '-05:00'), ('Europe/London', 'Europe', 'London', '+00:00'),
         ('Europe/Berlin', 'Europe', 'Berlin', '+01:00'), ('Asia/Tokyo', 'Asia', 'Tokyo', '+09:00'), ('Australia/Sydney', 'Australia', 'Sydney', '+11:00')]
put('zones', {'zones': [{'zone': z, 'region': r, 'city': c, 'offset': o} for z, r, c, o in zones]})
put('k-clock', 'clock=0\n')
with open(os.path.join(d, 'bar-json'), 'w') as f:
    json.dump({'clockZones': ['Asia/Tokyo'], 'clockHolidays': 'us'}, f)
loc = [('en_US.UTF-8', 'English (United States)'), ('en_GB.UTF-8', 'English (United Kingdom)'), ('de_DE.UTF-8', 'German (Germany)'), ('fr_FR.UTF-8', 'French (France)'), ('ja_JP.UTF-8', 'Japanese (Japan)')]
put('region', {'lang': 'en_US.UTF-8', 'formats': {}, 'locales': [{'id': i, 'name': n} for i, n in loc],
               'look': {'date': 'Wednesday, November 18, 2026', 'time': '8:15 PM', 'number': '1,234.56', 'money': '$1,234.56', 'metric': False, 'paper': 'Letter'}})
put('languages', {'languages': ['en_US', 'de_DE']})
put('spell', {'dictionaries': [{'id': 'en_US', 'name': 'English (United States)'}, {'id': 'de_DE', 'name': 'German'}],
              'settings': {'defaultLanguage': 'en_US', 'backgroundCheckerEnabled': True, 'checkerEnabledByDefault': False, 'autodetect': True, 'skipUppercase': True, 'skipRunTogether': True, 'ignore': ['Hearth', 'Ithil', 'Firehold']}})
put('langpacks', {'languages': [{'id': 'en_US', 'name': 'English (United States)', 'apps': 412, 'flatpak': True, 'dict': 'system'},
                                {'id': 'de_DE', 'name': 'German (Germany)', 'apps': 398, 'flatpak': False, 'dict': 'missing'}], 'flatpak': True})
put('im', {'installed': True, 'flatpak': True, 'on': False, 'kwin': True})
put('clip', {'available': True})
put('k-clip', 'max=25\nkeep=true\nignoresel=true\nsync=false\n')
put('baloo', 'Baloo File Indexer is running\nIndexer state: Idle\nTotal files indexed: 48,213\nCurrent size of index is 212.45 MiB\n')
put('k-search', 'indexing=true\nbasic=false\nfolders=/home/ci/\nexclude=/home/ci/Private\nrun.krunner_killrunner=false\n')
put('about', 'os=Hearth OS (Fedora edition)\nver=45\nhver=1.14.0\nhdate=2026-11-16\nhost=chris-hearth\nmodel=ROG Strix G16\nvendor=ASUSTeK COMPUTER INC.\ncpu=AMD Ryzen 9 7945HX with Radeon Graphics\ncores=32\nmem=32 GB\ngpu=NVIDIA Corporation AD106M [GeForce RTX 4070 Max-Q / Mobile]\ndisk=590G free of 931G\nkernel=6.17.8-300.fc45.x86_64\nplasma=plasmashell 6.5.2\nuser=chris\n')
put('updates', {'booted': {'version': '1.14.0', 'image': 'ghcr.io/dragongamer9/hearth-os:anatase'}})
rows = lambda *r: [list(x) for x in r]
put('info-cpu', {'sections': [{'title': 'AMD Ryzen 9 7945HX', 'rows': rows(('Cores', '16 (32 threads)'), ('Speed now', '3.1 GHz (up to 5.4 GHz)'), ('Cache', 'L2 16 MB · L3 64 MB'), ('Features', 'AVX-512, AES, SHA, virtualization')),
                              'bars': [2900 + (i * 137) % 2400 for i in range(32)], 'top': 5400}]})
put('info-gpu', {'sections': [{'title': 'GeForce RTX 4070 Laptop', 'rows': rows(('Driver', 'nvidia 580.95'), ('Memory', '8 GB GDDR6'), ('PCI', '01:00.0'))},
                              {'title': 'Radeon 610M', 'rows': rows(('Driver', 'amdgpu'), ('Memory', 'Shared'), ('PCI', '07:00.0'))}]})
put('info-memory', {'sections': [{'title': 'Memory', 'meter': [11 * G, 32 * G], 'rows': rows(('Total', '32 GB'), ('In use', '11 GB'), ('Cached', '9.2 GB'))},
                                 {'title': 'Swap', 'rows': rows(('Compressed (zram)', '2 GB of 16 GB'), ('Drive swap file', '0 of 8 GB'))}]})
put('info-graphics', {'sections': [{'title': 'Vulkan', 'rows': rows(('Version', '1.4.321'), ('Devices', 'GeForce RTX 4070 Laptop, Radeon 610M'))}, {'title': 'OpenGL', 'rows': rows(('Renderer', 'NVIDIA GeForce RTX 4070 Laptop GPU'), ('Version', '4.6.0 NVIDIA 580.95'))}]})
items = lambda l: {'items': [dict(zip(('id', 'kind', 'title', 'sub', 'value'), x)) for x in l]}
put('info-pci', items([('00:00.0', 'Host bridge', 'Raphael Root Complex', 'AMD', ''), ('01:00.0', 'VGA compatible controller', 'GeForce RTX 4070 Laptop', 'NVIDIA', 'nvidia'),
                       ('02:00.0', 'Non-Volatile memory controller', 'PC801 NVMe SSD', 'SK hynix', 'nvme'), ('03:00.0', 'Network controller', 'MT7922 Wi-Fi 6E', 'MediaTek', 'mt7921e'), ('07:00.6', 'Audio device', 'Family 17h HD Audio', 'AMD', 'snd_hda_intel')]))
put('info-usb', items([('1-3', 'Mouse', 'MX Master 3S receiver', 'Logitech', '12 Mbit/s'), ('1-4', 'Bluetooth', 'Wireless Bluetooth', 'MediaTek', '480 Mbit/s'), ('3-1', 'Camera', 'HD Webcam', 'Chicony', '480 Mbit/s')]))
put('info-sensors', {'sections': [{'title': 'Temperatures', 'gauges': [{'label': 'Processor (Tctl)', 'value': 64, 'top': 95, 'text': 'k10temp (Tctl)'}, {'label': 'Graphics card', 'value': 57, 'top': 87, 'text': 'nvidia'},
                                                                       {'label': 'SSD', 'value': 41, 'top': 80, 'text': 'nvme (Composite)'}]}, {'title': 'Fans', 'rows': rows(('CPU fan', '2,950 RPM'), ('GPU fan', '3,100 RPM'))}]})
put('info-modules', items([('nvidia', 'module', 'nvidia', 'used by nvidia_modeset, nvidia_uvm', '104 MB'), ('amdgpu', 'module', 'amdgpu', 'used by 3', '14 MB'), ('mt7921e', 'module', 'mt7921e', '', '1 MB')]))
put('info-interrupts', items([('0', 'IR-IO-APIC', 'timer', '', '42'), ('9', 'IR-IO-APIC', 'acpi', '', '12,904'), ('120', 'IR-PCI-MSIX', 'nvme0q0', '', '88,120')]))
put('live', '')
# Storage › Files a move put aside (red flag 9): the owner's Nov 24 folder, not answered yet.
put('oldf', {'folders': [{'name': '.old-hearth-2026-11-24', 'date': '2026-11-24', 'from': 'Hearth OS, Fedora edition', 'items': 31, 'back': 27,
                          'ask': True, 'kept': False, 'brought': 0}]})
put('hsearch', {'prefs': {'indexing': True}, 'plugins': []})
PY
mkdir -p "$HOME/.config/nimbus"
cp "$SYSFIX/bar-json" "$HOME/.config/nimbus/bar.json"
export HEARTH_SYSTEM_FIXTURE="$SYSFIX" XDG_CURRENT_DESKTOP=KDE
for page in display sound sound-speakers sound-apps push-to-talk sounds notifications power power-profiles power-awake battery memory storage \
            drives your-desktop startup boot remote-desktop datetime region languages spellcheck language-packs input-methods clipboard search \
            about info-cpu info-gpu info-memory info-graphics info-pci info-usb info-sensors info-modules info-interrupts info-energy install; do
  run "settings-sys-$page" hearth-settings 0 "$BIN/hearth-settings-app" --page "$page"
done
# The pages' other states: a display change waiting for Keep, over-amplification's warning, Hibernate's first-time
# question, a chooser's list, a notification app opened, Memory with a change not saved yet.
HEARTH_SYSTEM_SHOW=keep run settings-sys-display-keep hearth-settings 0 "$BIN/hearth-settings-app" --page display
HEARTH_SYSTEM_SHOW=flag:askoveramp run settings-sys-sound-overamp hearth-settings 0 "$BIN/hearth-settings-app" --page sound
HEARTH_SYSTEM_SHOW=text:askhib=hibernate run settings-sys-power-hibernate hearth-settings 0 "$BIN/hearth-settings-app" --page power
HEARTH_SYSTEM_SHOW=open:lang run settings-sys-region-chooser hearth-settings 0 "$BIN/hearth-settings-app" --page region
HEARTH_SYSTEM_SHOW=open:app:Discord run settings-sys-notifications-app hearth-settings 0 "$BIN/hearth-settings-app" --page notifications
HEARTH_SYSTEM_SHOW=text:oldfdel=.old-hearth-2026-11-24 run settings-sys-storage-oldfiles-delete hearth-settings 0 "$BIN/hearth-settings-app" --page storage
# The long pages' lower parts: the screen made three times as tall (the window fills it), pictured in two more
# pieces of 1280x800 (NAME-2, NAME-3), then the screen put back.
tall() { # tall NAME PAGE
  name=$1
  "$BIN/hearth-settings-app" --page "$2" >"/tmp/$name.log" 2>&1 &
  pid=$!
  for i in $(seq 1 600); do mapped hearth-settings && break; kill -0 $pid 2>/dev/null || break; sleep 0.005; done
  sleep 2.5
  grim -g "0,800 1280x800" "$ROOT/shots/$name-2.png" 2>/dev/null
  grim -g "0,1600 1280x800" "$ROOT/shots/$name-3.png" 2>/dev/null
  kill -0 $pid 2>/dev/null || echo "::error title=$name::it ended by itself: $(tail -c 1500 /tmp/$name.log | tr '\n' '|')"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.3
}
if swaymsg output HEADLESS-1 resolution 1280x2400 >/dev/null 2>&1; then
  sleep 0.5
  for page in display sound notifications power power-profiles battery memory storage drives startup remote-desktop datetime region search sounds about; do
    tall "settings-sys-$page" "$page"
  done
  HEARTH_SYSTEM_SHOW=open:app:Discord tall settings-sys-notifications-app notifications
  swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
  sleep 0.5
else
  echo "::warning title=settings-system::the screen couldn't be made taller for the long pages' pictures"
fi
unset HEARTH_SYSTEM_FIXTURE XDG_CURRENT_DESKTOP
rm -f "$HOME/.config/nimbus/bar.json"
