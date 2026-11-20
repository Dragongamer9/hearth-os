#!/usr/bin/python3
"""Every KDE System Settings / Info Center module (KCM) in this Fedora with Plasma installed, for Hearth Settings'
"Build them all into Hearth Settings" (docs/mockups/checklists/settings-kde-all.md). Writes kcms.json (each module's
id, name, description, category, package, file) and kcfg.tar.gz (each module's settings schema, /usr/share/config.kcfg),
and posts both as gzip+base64 notices: "INV-PARTS what n", then "INV what i" (kde-kcm-inventory.yml, 9 a step).
  kde-kcm-inventory.py collect | kde-kcm-inventory.py post WHAT FIRST LAST"""
import base64, glob, gzip, io, json, os, re, shutil, subprocess, sys, tarfile

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
    # What the last parts write and call (dig.json): the exact names, read from the image's own programs.
    dig = {}
    def strs(path, pat, n=300):
        try:
            out_s = subprocess.run(['strings', '-n', '4', path], capture_output=True, text=True, timeout=60).stdout.splitlines()
        except Exception as e:
            return [str(e)]
        return [x for x in out_s if re.search(pat, x)][:n]
    def sh(cmd):
        try:
            r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=60)
            return (r.stdout + r.stderr).splitlines()[:300]
        except Exception as e:
            return [str(e)]
    dig['kscreen-doctor --help'] = sh('kscreen-doctor --help')
    dig['kscreen-doctor strings'] = strs(shutil.which('kscreen-doctor') or '/usr/bin/kscreen-doctor', r'(?i)brightness|hdr|wcg|gamut|override|iccprofile|sdr')
    for pat, rx in (('/usr/lib*/libexec/kf6/kunifiedpush-distributor', r'(?i)gotify|nextpush|ntfy|url|token|password|user|login|wallet|keychain'),
                    ('/usr/libexec/kunifiedpush-distributor', r'(?i)gotify|nextpush|ntfy|url|token|password|user|login|wallet|keychain'),
                    ('/usr/bin/krdpserver', r'(?i)keychain|wallet|KRDP|password|Users'),
                    ('/usr/lib64/qt6/plugins/plasma/kcms/systemsettings/kcm_krdpserver.so', r'(?i)keychain|wallet|KRDP|password|Users|service'),
                    ('/usr/lib64/qt6/plugins/plasma/kcms/systemsettings/kcm_keys.so', r'(?i)net\.local|X-KDE|desktop|Exec|khotkeys|_launch|command'),
                    ('/usr/lib64/qt6/plugins/plasma/kcms/systemsettings/kcm_activities.so', r'(?i)OTR|isOTR|Features|Private|icon|shortcut|switch-to'),
                    ('/usr/bin/kactivitymanagerd', r'(?i)OTR|Features|SetValue|GetValue|switch-to|ActivityManager'),
                    ('/usr/libexec/kactivitymanagerd', r'(?i)OTR|Features|SetValue|GetValue|switch-to|ActivityManager'),
                    ('/usr/lib64/qt6/plugins/plasma/kcms/systemsettings_qwidgets/kcm_solid_actions.so', r'(?i)solid/actions|Hidden|X-KDE|desktop')):
        for f in glob.glob(pat):
            dig[f] = strs(f, rx)
    for f in glob.glob('/usr/share/knsrcfiles/*.knsrc'):
        if any(k in f for k in ('lookandfeel', 'color', 'plasma-themes', 'aurorae', 'window-decorations', 'plasma_themes', 'kwin')):
            dig[f] = open(f, errors='replace').read()[:1500].splitlines()
    dig['knsrcfiles'] = sorted(os.path.basename(x) for x in glob.glob('/usr/share/knsrcfiles/*'))
    dig['kwallet-query --help'] = sh('kwallet-query --help')
    dig['which'] = sh('which kwallet-query kpackagetool6 knewstuff-dialog6 kscreen-doctor qdbus6 kglobalaccel6 2>&1; ls /usr/libexec/kf6 | head -40')
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
