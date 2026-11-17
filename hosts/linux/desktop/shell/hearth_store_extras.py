"""hearth_store_extras: Settings › App store's 1.14 extras (KDE Discover's), run through hearth-store (it hands itself
over as `hs`, so the same rules apply: plain-words errors, progress lines, one JSON line or "error: ...").

  Update notices      hearth-store check: are there app updates Hearth hasn't told you about yet? StoreWatch.qml
                      asks every few hours and shows a Hearth notice with "Update all" (off-able:
                      ~/.config/nimbus/store.json "notifyUpdates", through hearth_settings_io: atomic, .bak, merged)
  New permissions     each Flatpak update's permissions (the new version's metadata, flatpak remote-info) against the
                      installed one's (flatpak info): what it asks for that it couldn't do before, in plain words.
                      "Update all" leaves those apps out; each waits until you've looked at what it asks for and said yes
                      (hearth-store update APP_ID --allow-new)
  Drivers             like ubuntu-drivers, for Debian: the PC's parts (lspci, the CPU, firmware the kernel couldn't
                      load) and the Debian packages that drive them (Nvidia's driver, firmware-*, microcode), whether
                      each is installed and can be. Installing one goes through hearth-admin driver-install (pkexec,
                      always an administrator's password: 49-hearth-admin.rules) into Peppermint's writable system
                      (hearth-unlock --install-package), which keeps it across updates and keeps the version before
                      startable. Read only otherwise.

Python standard library only.
"""
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.realpath(__file__))
CONFIG = os.environ.get('XDG_CONFIG_HOME') or os.path.join(os.path.expanduser('~'), '.config')
PREFS_FILE = os.path.join(CONFIG, 'nimbus', 'store.json')
PREFS_DEFAULTS = {'notifyUpdates': True}
ADMIN = '/usr/libexec/nimbus/hearth-admin'
# The packages driver-install takes (hearth-admin checks the same list again as root).
# What Hearth installs from Debian: only the firmware packages named in VENDORS and FIRMWARE_FILES below (always with
# re.fullmatch). Microcode and Nvidia's driver come with Hearth OS's own image and updates: shown, never installed here.
DRIVER_PKG = re.compile(r'(firmware-amd-graphics|firmware-intel-graphics|firmware-iwlwifi|firmware-sof-signed|firmware-realtek|firmware-atheros|firmware-mediatek|firmware-brcm80211|firmware-nvidia-graphics|firmware-cirrus|firmware-misc-nonfree)')
REF_RE = re.compile(r'^[A-Za-z_][\w-]*(\.[A-Za-z_][\w-]*){2,}(//[\w.-]+)?$')


def _hsio():
    sys.path.insert(0, HERE)
    import hearth_settings_io
    return hearth_settings_io


# ------------------------------------------------------------------ the store's own choices
def prefs():
    d = dict(PREFS_DEFAULTS)
    if os.path.exists(PREFS_FILE):
        saved = _hsio().load_json(PREFS_FILE, {})
        if isinstance(saved, dict):
            for k in PREFS_DEFAULTS:
                if isinstance(saved.get(k), type(PREFS_DEFAULTS[k])):
                    d[k] = saved[k]
    return d


def prefs_set(hs, key, value):
    if key not in PREFS_DEFAULTS or value not in ('on', 'off'):
        raise hs.Fail('usage: hearth-store prefs-set notifyUpdates on|off')
    _hsio().merge_json(PREFS_FILE, {key: value == 'on'}, who='Settings › App store')
    return {'ok': True, 'prefs': prefs()}


# ------------------------------------------------------------------ permissions
def perms_of(meta):
    """A Flatpak's permissions from its metadata (a keyfile) as a set of "kind:value" ("shared:network",
    "filesystems:home", "session:org.freedesktop.secrets=talk"). Negated ones ("!x11") are left out."""
    out, sect = set(), ''
    for line in (meta or '').splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        if line.startswith('[') and line.endswith(']'):
            sect = line[1:-1]
            continue
        k, eq, v = line.partition('=')
        if not eq:
            continue
        k, v = k.strip(), v.strip()
        if sect == 'Context' and k in ('shared', 'sockets', 'devices', 'filesystems', 'features', 'persistent'):
            for x in v.split(';'):
                x = x.strip()
                if x and not x.startswith('!'):
                    out.add(k + ':' + x)
        elif sect in ('Session Bus Policy', 'System Bus Policy') and v in ('talk', 'own'):
            out.add(('session:' if sect.startswith('Session') else 'system:') + k + '=' + v)
    return out


FOLDERS = {'home': 'your home folder', 'host': 'every file on this PC', 'host-os': "the system's programs",
           'host-etc': "the system's settings (/etc)", 'xdg-download': 'your Downloads folder', 'xdg-documents': 'your Documents folder',
           'xdg-pictures': 'your Pictures folder', 'xdg-music': 'your Music folder', 'xdg-videos': 'your Videos folder',
           'xdg-desktop': 'your Desktop folder', 'xdg-config': 'your apps\' settings (~/.config)', 'xdg-data': 'your apps\' data (~/.local/share)',
           'xdg-run': 'running programs\' files', 'xdg-cache': 'your apps\' caches'}
WORDS = {
    'shared:network': 'Use the internet', 'shared:ipc': 'Share memory with other programs',
    'sockets:x11': 'Show windows through X11 (it can see other X11 windows and what you type in them)',
    'sockets:fallback-x11': 'Show windows through X11 when Wayland isn\'t there', 'sockets:wayland': 'Show windows (Wayland)',
    'sockets:pulseaudio': 'Play and record sound', 'sockets:system-bus': 'Talk to every system service',
    'sockets:session-bus': 'Talk to every program you have open', 'sockets:ssh-auth': 'Use your SSH keys',
    'sockets:pcsc': 'Use smart cards', 'sockets:cups': 'Print', 'sockets:gpg-agent': 'Use your GPG keys',
    'sockets:inherit-wayland-socket': 'Show windows (Wayland)',
    'devices:dri': 'Use the graphics card', 'devices:all': 'Use every device (cameras, controllers, USB drives…)',
    'devices:kvm': 'Run virtual machines', 'devices:input': 'Use controllers and other input devices', 'devices:usb': 'Use USB devices',
    'devices:shm': 'Share memory with other programs',
    'features:devel': 'Debugging tools (it can look inside other programs)', 'features:multiarch': 'Run 32-bit programs',
    'features:bluetooth': 'Use Bluetooth', 'features:canbus': 'Use CAN bus devices', 'features:per-app-dev-shm': 'Share memory with its own other copies',
}


def perm_words(p):
    if p in WORDS:
        return WORDS[p]
    kind, _, v = p.partition(':')
    if kind == 'filesystems':
        ro = v.endswith(':ro')
        v = re.sub(r':(ro|rw|create)$', '', v)
        base = v.split('/', 1)
        name = FOLDERS.get(base[0]) or ('the folder ' + v.replace('~', 'your home folder', 1) if v else v)
        if len(base) > 1 and base[0] in FOLDERS:
            name = '“%s” in %s' % (base[1], FOLDERS[base[0]])
        return ('Read ' if ro else 'Read and change ') + name
    if kind in ('session', 'system'):
        name, _, how = v.partition('=')
        if kind == 'system':
            return ('Act as the system service ' if how == 'own' else 'Talk to the system service ') + name
        return ('Take the name ' if how == 'own' else 'Talk to ') + name
    if kind == 'persistent':
        return 'Keep its own folder ~/' + v
    return p


def new_perms(old_meta, new_meta):
    """[{key, label}] that the new version asks for and the installed one doesn't have."""
    old, new = perms_of(old_meta), perms_of(new_meta)
    # "home" covers any folder in it; "host" covers everything: a narrower folder isn't new then.
    def covered(p):
        if not p.startswith('filesystems:'):
            return False
        if 'filesystems:host' in old:
            return True
        v = p[12:]
        return 'filesystems:home' in old and (v.startswith('~') or v.startswith('xdg-') or v.startswith('home/'))
    return [{'key': p, 'label': perm_words(p)} for p in sorted(new - old) if not covered(p)]


def ref(row):
    return row['id'] + ('//' + row['branch'] if row.get('branch') else '')


def with_perms(hs, rows):
    """Adds "newPerms" to each update row (Flatpak's metadata now and in the update)."""
    for row in rows[:40]:
        inst = '--user' if row.get('where') == 'yours' else '--system'
        r_old = hs.run_flatpak(['info', inst, '--show-metadata', ref(row)], timeout=30, check=False)
        r_new = hs.run_flatpak(['remote-info', inst, '--show-metadata', row.get('origin') or 'flathub', ref(row)], timeout=60, check=False)
        row['newPerms'] = new_perms(r_old.stdout, r_new.stdout) if r_old.returncode == 0 and r_new.returncode == 0 else []
        row['permsKnown'] = r_old.returncode == 0 and r_new.returncode == 0
    for row in rows[40:]:
        row['newPerms'], row['permsKnown'] = [], False
    return rows


def update_apps(hs, ids, allow_new=False):
    """Update all (no IDS): every app whose update asks for nothing new; the others wait for you. IDS: those apps;
    one that asks for new permissions only with ALLOW_NEW (you looked at them and said yes)."""
    rows = with_perms(hs, hs.updates()['flatpaks'])
    for a in ids:
        if not REF_RE.match(a):
            raise hs.Fail("That isn't an app id")
    if ids:
        pick = [r for r in rows if r['id'] in ids]
        asking = [r for r in pick if r['newPerms']]
        if asking and not allow_new:
            r = asking[0]
            raise hs.Fail('%s asks for new permissions (%s): look at them first' % (r.get('name') or r['id'], '; '.join(p['label'] for p in r['newPerms'])))
        if not pick:
            pick = [{'id': a, 'where': w, 'branch': ''} for a in ids for w in ('yours', 'system')]
    else:
        pick = [r for r in rows if not r['newPerms']]
    waiting = [{'id': r['id'], 'name': r.get('name') or r['id'], 'newPerms': r['newPerms']} for r in rows if r['newPerms'] and r not in pick]
    done = []
    for who, inst in (('yours', '--user'), ('system', '--system')):
        refs = [ref(r) for r in pick if r.get('where') == who]
        if not refs:
            continue
        if not ids and not waiting:
            refs = []                                # nothing to leave out: flatpak update everything (runtimes too)
        try:
            hs.stream_flatpak(['update', '-y', '--noninteractive', inst] + refs,
                              'Updating ' + (', '.join(r.split('//')[0] for r in refs[:3]) if refs else 'your apps' if who == 'yours' else "the system's apps"))
            done += [r.split('//')[0] for r in refs] or [who]
        except hs.Fail as e:
            if who == 'system' and ('permission' in str(e).lower() or 'not allowed' in str(e).lower()):
                continue
            if ids and 'not installed' in str(e).lower():
                continue
            raise
    return {'ok': True, 'updated': done, 'waiting': waiting}


def check(hs):
    """For StoreWatch.qml: {notify, count, names, waiting, fresh}. fresh: this set of updates hasn't been told yet
    (remembered in the store's cache folder: what was told isn't a setting)."""
    p = prefs()
    if not p['notifyUpdates']:
        return {'notify': False, 'count': 0, 'names': [], 'waiting': 0, 'fresh': False}
    rows = with_perms(hs, hs.updates()['flatpaks'])
    sig = ','.join(sorted('%s=%s' % (r['id'], r.get('version') or r.get('size') or '') for r in rows))
    seen_file = os.path.join(hs.CACHE, 'told-updates.txt')
    try:
        with open(seen_file) as f:
            told = f.read().strip()
    except OSError:
        told = None
    fresh = bool(rows) and sig != told
    if fresh:
        os.makedirs(hs.CACHE, mode=0o700, exist_ok=True)
        with open(seen_file + '.tmp', 'w') as f:
            f.write(sig)
        os.replace(seen_file + '.tmp', seen_file)
    return {'notify': True, 'count': len(rows), 'names': [r.get('name') or r['id'] for r in rows][:6],
            'waiting': sum(1 for r in rows if r['newPerms']), 'fresh': fresh}


# ------------------------------------------------------------------ drivers
# A PCI vendor (and class) → the Debian packages that drive it. Classes: 03 display, 02 network, 04 multimedia, 0d wireless.
VENDORS = {
    '10de': [('03', 'nvidia', 'Nvidia graphics')],
    '1002': [('03', 'firmware-amd-graphics', 'AMD graphics')],
    '8086': [('03', 'firmware-intel-graphics', 'Intel graphics'), ('02', 'firmware-iwlwifi', 'Intel Wi-Fi and Bluetooth'),
             ('0d', 'firmware-iwlwifi', 'Intel Wi-Fi and Bluetooth'), ('04', 'firmware-sof-signed', 'Intel sound')],
    '10ec': [('02', 'firmware-realtek', 'Realtek network')],
    '168c': [('02', 'firmware-atheros', 'Qualcomm Atheros Wi-Fi')],
    '17cb': [('02', 'firmware-atheros', 'Qualcomm Wi-Fi')],
    '14c3': [('02', 'firmware-mediatek', 'MediaTek Wi-Fi')],
    '14e4': [('02', 'firmware-brcm80211', 'Broadcom Wi-Fi')],
}
# Firmware the kernel asked for and couldn't load ("Direct firmware load for X failed") → its package.
FIRMWARE_FILES = [(r'^iwlwifi|^intel/ibt', 'firmware-iwlwifi'), (r'^(rtl_nic|rtl_bt|rtw88|rtw89|rtlwifi)/', 'firmware-realtek'),
                  (r'^amdgpu/|^radeon/', 'firmware-amd-graphics'), (r'^(i915|xe)/', 'firmware-intel-graphics'),
                  (r'^(ath9k_htc|ath10k|ath11k|ath12k|qca)/', 'firmware-atheros'), (r'^(mediatek|mt76)', 'firmware-mediatek'),
                  (r'^brcm/', 'firmware-brcm80211'), (r'^nvidia/', 'firmware-nvidia-graphics'), (r'^intel/(sof|avs)', 'firmware-sof-signed'),
                  (r'^cirrus/', 'firmware-cirrus'), (r'', 'firmware-misc-nonfree')]
FW_FAIL = re.compile(r'(?:Direct firmware load for|firmware: failed to load) (\S+?)(?: failed|\s|$)')


def _run(cmd, timeout=30):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=dict(os.environ, LC_ALL='C'))
        return r.returncode, r.stdout
    except (OSError, subprocess.TimeoutExpired):
        return 127, ''


def facts():
    """What drivers() looks at: {lspci, cpu, kernlog, installed: {pkg: version}, candidates: {pkg: version or ""},
    nvidiaLoaded}. HEARTH_DRIVERS_FIXTURE (a JSON file with the same keys) stands in for the PC in tests and CI."""
    fx = os.environ.get('HEARTH_DRIVERS_FIXTURE')
    if fx:
        with open(fx) as f:
            return json.load(f)
    _, lspci = _run(['lspci', '-n', '-mm'])
    try:
        cpu = open('/proc/cpuinfo').read(20000)
    except OSError:
        cpu = ''
    _, klog = _run(['journalctl', '-k', '-b', '-q', '--no-pager', '-o', 'cat', '--grep', 'firmware'], timeout=20)
    return {'lspci': lspci, 'cpu': cpu, 'kernlog': klog, 'installed': None, 'candidates': None,
            'nvidiaLoaded': os.path.exists('/proc/driver/nvidia')}


def pkg_state(f, pkgs):
    """{pkg: (installed version or "", candidate version or "")}."""
    out = {}
    if f.get('installed') is not None:
        for p in pkgs:
            out[p] = ((f['installed'] or {}).get(p, ''), (f.get('candidates') or {}).get(p, ''))
        return out
    for p in pkgs:
        code, o = _run(['dpkg-query', '-W', '-f', '${db:Status-Abbrev}\t${Version}', p])
        st, _, ver = o.partition('\t')
        inst = ver.strip() if code == 0 and st[1:2] == 'i' else ''
        code, o = _run(['apt-cache', 'policy', p])
        m = re.search(r'Candidate:\s*(\S+)', o or '')
        out[p] = (inst, m.group(1) if m and m.group(1) != '(none)' else '')
    return out


def pci_devices(text):
    """[(class, vendor, device, name)] from `lspci -n -mm` ("00:02.0 "0300" "8086" "46a6" ...") or `lspci -nn -mm`."""
    out = []
    for line in (text or '').splitlines():
        q = re.findall(r'"([^"]*)"', line)
        if len(q) >= 3:
            cls, ven, dev = (re.search(r'\[?([0-9a-fA-F]{4})\]?$', x.strip()) for x in q[:3])
            if cls and ven and dev:
                out.append((cls.group(1).lower(), ven.group(1).lower(), dev.group(1).lower(), q[1]))
    return out


def drivers(hs=None):
    f = facts()
    found = {}      # package -> {why: [...]}

    def want(pkg, why):
        e = found.setdefault(pkg, {'why': []})
        if why not in e['why']:
            e['why'].append(why)
    nvidia = False
    for cls, ven, dev, _ in pci_devices(f.get('lspci')):
        for c, pkg, why in VENDORS.get(ven, []):
            if cls.startswith(c):
                if pkg == 'nvidia':
                    nvidia = True
                else:
                    want(pkg, why)
    for m in FW_FAIL.finditer(f.get('kernlog') or ''):
        name = m.group(1)
        for pat, pkg in FIRMWARE_FILES:
            if re.search(pat, name):
                want(pkg, 'Firmware the kernel asked for: ' + name)
                break
    cpu = f.get('cpu') or ''
    ucode = 'intel-microcode' if 'GenuineIntel' in cpu else 'amd64-microcode' if 'AuthenticAMD' in cpu else ''
    nv_pkgs = ['nvidia-open', 'nvidia-driver', 'firmware-nvidia-graphics'] if nvidia else []
    st = pkg_state(f, list(found) + nv_pkgs + ([ucode] if ucode else []))
    rows = []
    if ucode:
        # Information only: the processor's fixes come with Hearth OS's updates (never installed from here).
        rows.append({'package': ucode, 'device': ('Intel' if ucode.startswith('intel') else 'AMD') + ' processor',
                     'why': ['Security and stability fixes for the processor'], 'installed': st[ucode][0], 'available': '',
                     'inUse': bool(st[ucode][0]), 'recommended': False, 'infoOnly': True,
                     'note': "Comes with Hearth OS's updates"})
    if nvidia:
        # NVIDIA's own driver (Hearth OS's image carries it on PCs with an Nvidia card); Debian's nvidia-driver only
        # when it's what's installed already (Debian's 550 doesn't build for the newer kernel Hearth uses).
        have = next((p for p in ('nvidia-open', 'nvidia-driver') if st[p][0]), '')
        pkg = have or 'nvidia-open'
        # Information only: Hearth OS's own image carries NVIDIA's driver (never installed from here).
        rows.append({'package': pkg, 'device': 'Nvidia graphics', 'why': ['An Nvidia graphics card'], 'installed': st[pkg][0],
                     'available': '', 'inUse': bool(f.get('nvidiaLoaded')), 'recommended': False, 'infoOnly': True,
                     'note': "Hearth OS's own image carries it (Settings › Updates)"})
        if st['firmware-nvidia-graphics'][0] or st['firmware-nvidia-graphics'][1]:
            found.setdefault('firmware-nvidia-graphics', {'why': ['Nvidia graphics (its GSP firmware)']})
    for pkg, e in sorted(found.items()):
        inst, cand = st.get(pkg, ('', ''))
        rows.append({'package': pkg, 'device': e['why'][0], 'why': e['why'], 'installed': inst, 'available': cand,
                     'inUse': bool(inst), 'recommended': True, 'note': '' if inst or cand else "Not in this system's package sources"})
    can, why = can_install()
    return {'drivers': rows, 'canInstall': can, 'cantWhy': why,
            'missing': sum(1 for r in rows if not r.get('infoOnly') and not r['installed'] and r['available'])}


def can_install():
    """Drivers install on Peppermint's writable system (hearth-unlock); elsewhere they're the image's."""
    if os.environ.get('HEARTH_DRIVERS_FIXTURE'):
        return True, ''
    if not os.access('/usr/bin/hearth-unlock', os.X_OK) or not os.access(ADMIN, os.X_OK):
        return False, "This edition's system is an image: drivers come with Hearth OS's updates"
    code, out = _run(['/usr/bin/hearth-unlock', '--status', '--json'])
    try:
        s = json.loads((out.strip().splitlines() or ['{}'])[-1])
    except ValueError:
        s = {}
    if not s.get('writable'):
        return False, 'The system is locked (Settings › Updates › Writable system): unlock it to install drivers'
    return True, ''


def driver_install(hs, pkg):
    if not DRIVER_PKG.fullmatch(pkg or ''):
        raise hs.Fail("That isn't one of the drivers Hearth installs")
    known = [r['package'] for r in drivers()['drivers'] if not r.get('infoOnly')]
    if pkg not in known:
        raise hs.Fail('This PC doesn\'t need %s' % pkg)
    can, why = can_install()
    if not can:
        raise hs.Fail(why)
    hs.progress(2, 'Installing %s (asks for an administrator\'s password)' % pkg)
    exe = os.environ.get('HEARTH_PKEXEC') or 'pkexec'
    p = subprocess.Popen([exe, ADMIN, 'driver-install', pkg], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    last = ''
    for line in p.stdout:
        line = line.rstrip()
        if line.startswith('progress: '):
            print(line, flush=True)
        elif line:
            last = line
    code = p.wait()
    if code in (126, 127):                          # pkexec: the password box was closed, or the password was wrong
        raise hs.Fail('Not installed: the password wasn\'t given')
    if code != 0:
        raise hs.Fail('Couldn\'t install %s (%s)' % (pkg, re.sub(r'^error:\s*', '', last)[:200] or 'apt-get failed'))
    return {'ok': True, 'installed': pkg, 'restart': True}   # firmware takes effect after a restart


def run(hs, c, rest):
    if c == 'prefs' and not rest:
        return {'prefs': prefs()}
    if c == 'prefs-set' and len(rest) == 2:
        return prefs_set(hs, rest[0], rest[1])
    if c == 'check' and not rest:
        return check(hs)
    if c == 'drivers' and not rest:
        return drivers(hs)
    if c == 'driver-install' and len(rest) == 1:
        return driver_install(hs, rest[0])
    raise hs.Fail('Unknown command')


COMMANDS = ('prefs', 'prefs-set', 'check', 'drivers', 'driver-install')
