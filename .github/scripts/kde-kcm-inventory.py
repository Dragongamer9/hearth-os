#!/usr/bin/python3
"""Every KDE System Settings / Info Center module (KCM) in this Fedora with Plasma installed, for Hearth Settings'
"Build them all into Hearth Settings" (docs/mockups/checklists/settings-kde-all.md). Writes kcms.json (each module's
id, name, description, category, package, file) and kcfg.tar.gz (each module's settings schema, /usr/share/config.kcfg),
and posts both as gzip+base64 notices: "INV-PARTS what n", then "INV what i" (kde-kcm-inventory.yml, 9 a step).
  kde-kcm-inventory.py collect | kde-kcm-inventory.py post WHAT FIRST LAST"""
import base64, glob, gzip, io, json, os, re, subprocess, sys, tarfile

def meta(path):
    """The plugin's JSON from its .qtmetadata (Qt 6: CBOR after "QTMETADATA !" and a 4-byte header)."""
    import cbor2
    b = open(path, 'rb').read()
    i = b.find(b'QTMETADATA !')
    if i < 0:
        return {}
    for off in range(12, 40):
        try:
            v = cbor2.CBORDecoder(io.BytesIO(b[i + off:])).decode()
        except Exception:
            continue
        if isinstance(v, dict):
            for k in (3, 'MetaData'):
                if isinstance(v.get(k), dict):
                    return v[k]
    return {}

def rpm(path):
    r = subprocess.run(['rpm', '-qf', '--qf', '%{NAME}', path], capture_output=True, text=True, timeout=20)
    return r.stdout.strip()

def collect():
    out = []
    for so in sorted(glob.glob('/usr/lib64/qt6/plugins/**/*.so', recursive=True)):
        if '/kcms/' not in so and '/kcm_' not in so and 'kcms' not in so:
            continue
        m = meta(so)
        kp = m.get('KPlugin', {}) if isinstance(m, dict) else {}
        out.append({'id': os.path.basename(so)[:-3], 'dir': os.path.dirname(so).split('plugins/')[-1],
                    'name': kp.get('Name', ''), 'desc': kp.get('Description', ''),
                    'cat': m.get('X-KDE-System-Settings-Parent-Category', '') or m.get('X-KDE-KInfoCenter-Category', ''),
                    'keywords': (m.get('X-KDE-Keywords', '') or '')[:200], 'formfactors': kp.get('FormFactors', ''),
                    'pkg': rpm(so), 'show': m.get('X-KDE-OnlyShowOnQtPlatforms', '')})
    for d in sorted(glob.glob('/usr/share/applications/kcm_*.desktop')):
        out.append({'id': os.path.basename(d)[:-8], 'dir': 'applications', 'name': '', 'desc': '', 'cat': '', 'pkg': rpm(d),
                    'desktop': open(d, errors='replace').read()[:600]})
    cats = {}
    for c in glob.glob('/usr/share/systemsettings/categories/*.desktop'):
        cats[os.path.basename(c)] = open(c, errors='replace').read()[:400]
    lst = subprocess.run(['kcmshell6', '--list'], capture_output=True, text=True, timeout=60,
                         env=dict(os.environ, QT_QPA_PLATFORM='offscreen')).stdout
    ver = subprocess.run(['rpm', '-q', 'plasma-workspace', 'plasma-desktop', 'kwin', 'plasma-systemsettings', 'kinfocenter'],
                         capture_output=True, text=True).stdout
    json.dump({'kcms': out, 'categories': cats, 'kcmshell6_list': lst, 'versions': ver}, open('kcms.json', 'w'), indent=1)
    # What a few KCMs without a kcfg write (their config files, groups and keys show in their strings).
    dig = {}
    for so in glob.glob('/usr/lib64/qt6/plugins/plasma/kcms/systemsettings*/*.so'):
        if any(k in so for k in ('push_notifications', 'fontinst', 'kcm_keys', 'colord', 'kwin_scripts')):
            out_s = subprocess.run(['strings', '-n', '4', so], capture_output=True, text=True, timeout=60).stdout.splitlines()
            keep = [x for x in out_s if re.search(r'(rc$|\.conf|^\[|Url|Type|Provider|Distributor|kglobalaccel|setForeign|QKeySequence|colormgr|org\.freedesktop|fontconfig|fonts|Enabled$|kpackage)', x)]
            dig[os.path.basename(so)] = keep[:400]
    for f in glob.glob('/usr/share/applications/*kunifiedpush*') + glob.glob('/usr/share/dbus-1/services/*UnifiedPush*') + glob.glob('/etc/xdg/*push*'):
        dig[f] = open(f, errors='replace').read()[:2000].splitlines()
    json.dump(dig, open('dig.json', 'w'), indent=0)
    with tarfile.open('kcfg.tar.gz', 'w:gz') as t:
        for f in sorted(glob.glob('/usr/share/config.kcfg/*.kcfg')):
            t.add(f, arcname=os.path.basename(f))
    print('::notice title=KCMs::%d modules, %d categories; %s' % (len(out), len(cats), ver.replace('\n', ' ')))

def post(what, first, last):
    data = open(what, 'rb').read()
    if not what.endswith('.gz'):
        data = gzip.compress(data, 9)
    s = base64.b64encode(data).decode()
    parts = [s[i:i + 3800] for i in range(0, len(s), 3800)]
    if first == 1:
        print('::notice title=INV-PARTS %s %d::%d bytes' % (what, len(parts), len(data)))
    for i in range(first, min(last, len(parts)) + 1):
        print('::notice title=INV %s %d::%s' % (what, i, parts[i - 1]))

if __name__ == '__main__':
    if sys.argv[1] == 'collect':
        collect()
    else:
        post(sys.argv[2], int(sys.argv[3]), int(sys.argv[4]))
