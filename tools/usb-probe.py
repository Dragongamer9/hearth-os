#!/usr/bin/env python3
"""Temporary probe (CI): what the official download indexes say, and the keys that sign them."""
import base64, json, os, re, subprocess, sys, tempfile

def note(title, msg):
    msg = msg.replace('%', '%25').replace('\r', '').replace('\n', '%0A')
    print(f'::notice title={title}::{msg}', flush=True)

def get(url, maxb=4_000_000):
    r = subprocess.run(['curl', '-fsSL', '--max-time', '60', '-A', 'HearthDisks-probe', url], capture_output=True)
    if r.returncode:
        return None
    return r.stdout[:maxb]

def head(url):
    r = subprocess.run(['curl', '-fsSIL', '--max-time', '60', '-o', '/dev/null', '-w', '%{http_code} %{size_download} %{url_effective}', url], capture_output=True, text=True)
    return r.stdout.strip() or ('fail ' + r.stderr.strip()[:200])

def hrefs(html):
    return re.findall(r'href="([^"?]+)"', html.decode('utf-8', 'replace'))

GNUPG = tempfile.mkdtemp()
def gpg(*a, inp=None):
    return subprocess.run(['gpg', '--homedir', GNUPG, '--batch'] + list(a), input=inp, capture_output=True)

def issuer(sig_bytes):
    r = gpg('--list-packets', inp=sig_bytes)
    out = r.stdout.decode('utf-8', 'replace') + r.stderr.decode('utf-8', 'replace')
    fp = re.findall(r'issuer fpr v\d ([0-9A-F]{40})', out)
    kid = re.findall(r'keyid ([0-9A-F]{16})', out)
    return (fp or [''])[0], (kid or [''])[0]

seen = set()
def key(fpr, src):
    if not fpr or fpr in seen:
        return
    seen.add(fpr)
    data = get(src)
    if not data:
        note('KEYFAIL', f'{fpr} {src}')
        return
    r = subprocess.run(['gpg', '--show-keys', '--with-colons', '--with-fingerprint'], input=data, capture_output=True)
    note('KEYINFO ' + fpr[-16:], src + '\n' + r.stdout.decode('utf-8', 'replace')[:3000])
    b = base64.b64encode(data).decode()
    for i in range(0, len(b), 4000):
        note(f'KEY {fpr} {i // 4000}', b[i:i + 4000])

def sigprobe(name, sig_url):
    s = get(sig_url)
    if not s:
        note('SIG ' + name, 'no signature at ' + sig_url)
        return
    fp, kid = issuer(s)
    note('SIG ' + name, f'{sig_url}\nissuer fpr={fp} keyid={kid}')
    if fp:
        key(fp, f'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x{fp}')

which = sys.argv[1] if len(sys.argv) > 1 else 'all'

if which in ('all', 'a'):
    j = get('https://fedoraproject.org/releases.json')
    if j:
        rel = json.loads(j)
        vs = sorted({r['version'] for r in rel})
        ws = [r for r in rel if r.get('subvariant') in ('Workstation', 'KDE') and r.get('arch') == 'x86_64' and r['link'].endswith('.iso')]
        note('fedora', 'versions ' + ' '.join(vs) + '\n' + '\n'.join(f"{r['version']} {r['variant']} {r['subvariant']} {r['link']} {r.get('sha256','')[:12]} {r.get('size')}" for r in ws[-8:]) + '\nkeys ' + ','.join(sorted(rel[0].keys())))
        if ws:
            top = max(ws, key=lambda r: (r['version'].replace(' Beta', '').isdigit(), r['version']))
            d = top['link'].rsplit('/', 1)[0] + '/'
            lst = get(d)
            cs = [h for h in hrefs(lst or b'') if 'CHECKSUM' in h]
            note('fedora dir', d + '\n' + ' '.join(hrefs(lst or b'')[:30]))
            if cs:
                c = get(d + cs[0])
                note('fedora checksum', (c or b'').decode()[:1500])
                if c:
                    fp, kid = issuer(c)
                    note('SIG fedora', f'issuer {fp} {kid}')
    fk = get('https://fedoraproject.org/fedora.gpg')
    if fk:
        r = subprocess.run(['gpg', '--show-keys', '--with-colons'], input=fk, capture_output=True)
        uids = [l for l in r.stdout.decode().splitlines() if l.startswith(('fpr', 'uid'))]
        note('fedora.gpg', f'{len(fk)} bytes\n' + '\n'.join(uids[-40:]))
        b = base64.b64encode(fk).decode()
        for i in range(0, len(b), 4000):
            note(f'KEY fedora.gpg {i // 4000}', b[i:i + 4000])

if which in ('all', 'b'):
    for base, flt in (('https://releases.ubuntu.com/', None), ('https://cdimage.ubuntu.com/kubuntu/releases/', None)):
        lst = get(base)
        dirs = sorted({h.strip('/') for h in hrefs(lst or b'') if re.fullmatch(r'\d\d\.\d\d(\.\d)?/', h)})
        note('ubuntu index ' + base, ' '.join(dirs))
        for v in dirs[-3:]:
            d = base + v + '/' + ('release/' if 'kubuntu' in base else '')
            s = get(d + 'SHA256SUMS')
            note('ubuntu sums ' + d, (s or b'none').decode()[:800])
        if dirs:
            d = base + dirs[-1] + '/' + ('release/' if 'kubuntu' in base else '')
            sigprobe('ubuntu ' + base, d + 'SHA256SUMS.gpg')
    for d in ('https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/', 'https://cdimage.debian.org/debian-cd/current-live/amd64/iso-hybrid/'):
        s = get(d + 'SHA256SUMS')
        note('debian ' + d, (s or b'none').decode()[:1500])
        sigprobe('debian ' + d, d + 'SHA256SUMS.sign')
    lst = get('https://mirrors.kernel.org/linuxmint/stable/')
    dirs = sorted({h.strip('/') for h in hrefs(lst or b'') if re.fullmatch(r'\d+(\.\d+)?/', h)}, key=lambda v: [int(x) for x in v.split('.')])
    note('mint index', ' '.join(dirs))
    if dirs:
        d = f'https://mirrors.kernel.org/linuxmint/stable/{dirs[-1]}/'
        note('mint sums', (get(d + 'sha256sum.txt') or b'none').decode()[:800])
        sigprobe('mint', d + 'sha256sum.txt.gpg')

if which in ('all', 'c'):
    for ed in ('intel', 'nvidia'):
        note('pop ' + ed, (get(f'https://api.pop-os.org/builds/24.04/{ed}') or b'none').decode()[:1000])
    j = get('https://archlinux.org/releng/releases/json/')
    if j:
        a = json.loads(j)
        r0 = [r for r in a['releases'] if r.get('available')][0]
        note('arch', json.dumps({k: v for k, v in r0.items() if k not in ('info',)})[:2500] + '\nkeys ' + ' '.join(a.keys()))
        d = f"https://geo.mirror.pkgbuild.com/iso/{r0['version']}/"
        note('arch dir', d + ' ' + ' '.join(hrefs(get(d) or b'')[:30]))
        sigprobe('arch', d + f"archlinux-{r0['version']}-x86_64.iso.sig")
        note('arch latest', head('https://geo.mirror.pkgbuild.com/iso/latest/archlinux-x86_64.iso'))
    lst = get('https://mirror.cachyos.org/ISO/desktop/')
    dirs = sorted({h.strip('/') for h in hrefs(lst or b'') if re.fullmatch(r'\d{6}/', h)})
    note('cachy index', ' '.join(dirs))
    if dirs:
        v = dirs[-1]
        for host in ('https://mirror.cachyos.org', 'https://cdn77.cachyos.org'):
            d = f'{host}/ISO/desktop/{v}/'
            note('cachy dir ' + host, ' '.join(hrefs(get(d) or b'')[:20]) + '\n' + head(d + f'cachyos-desktop-linux-{v}.iso'))
        note('cachy sha', (get(f'https://mirror.cachyos.org/ISO/desktop/{v}/cachyos-desktop-linux-{v}.iso.sha256') or b'none').decode()[:300])
        sigprobe('cachy', f'https://mirror.cachyos.org/ISO/desktop/{v}/cachyos-desktop-linux-{v}.iso.sig')
    key('882DCFE48E2051D48E2562ABF3B607488DB35A47', 'https://keys.openpgp.org/vks/v1/by-fingerprint/882DCFE48E2051D48E2562ABF3B607488DB35A47')

if which in ('all', 'd'):
    for iso in ('bazzite-stable-amd64.iso', 'bazzite-gnome-stable-amd64.iso', 'bazzite-deck-stable-amd64.iso'):
        note('bazzite ' + iso, head('https://download.bazzite.gg/' + iso) + '\n' + (get('https://download.bazzite.gg/' + iso + '-CHECKSUM') or b'none').decode()[:300])
    for f in ('openSUSE-Tumbleweed-DVD-x86_64-Current.iso', 'openSUSE-Tumbleweed-NET-x86_64-Current.iso', 'openSUSE-Tumbleweed-KDE-Live-x86_64-Current.iso'):
        b = 'https://download.opensuse.org/tumbleweed/iso/'
        note('suse ' + f, head(b + f) + '\n' + (get(b + f + '.sha256') or b'none').decode()[:600] + '\nasc: ' + head(b + f + '.sha256.asc'))
    sigprobe('suse asc', 'https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Current.iso.sha256.asc')
    s = get('https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-DVD-x86_64-Current.iso.sha256')
    if s and b'BEGIN PGP' in s:
        fp, kid = issuer(s)
        note('SIG suse inline', f'{fp} {kid}')
        key(fp, f'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x{fp}')
    key('repomd', 'https://download.opensuse.org/tumbleweed/repo/oss/repodata/repomd.xml.key')
    j = get('https://gitlab.manjaro.org/web/iso-info/-/raw/master/file-info.json')
    if j:
        m = json.loads(j)
        note('manjaro', json.dumps(m.get('official', {}))[:3000])
        try:
            img = m['official']['plasma']
            note('manjaro plasma', json.dumps(img) + '\n' + head(img['image']) + '\n' + (get(img['image'] + '.sha256') or b'none').decode()[:300] + '\n' + (get(img['image'] + '.sha512') or b'none').decode()[:300])
            sigprobe('manjaro', img.get('signature') or img['image'] + '.sig')
        except Exception as e:
            note('manjaro err', str(e))
    for f in ('hearth-os-live.iso', 'hearth-os-installer.iso', 'hearth-os-peppermint.iso'):
        b = 'https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/'
        note('hearth ' + f, head(b + f) + '\n' + (get(b + f + '.sha256') or b'none').decode()[:300])
    for u in ('https://www.microsoft.com/en-us/software-download/windows11', 'https://www.microsoft.com/en-us/software-download/windows10ISO'):
        note('ms ' + u, head(u))

if which == 'e':
    for u in ('https://gitlab.manjaro.org/web/iso-info/-/raw/master/file-info.json', 'https://manjaro.org/products/download/x86', 'https://download.manjaro.org/kde/', 'https://manjaro.org/download/'):
        b = get(u)
        note('manjaro src ' + u, 'none' if b is None else (str(len(b)) + ' ' + ' '.join(h for h in hrefs(b) if 'iso' in h or 'download.manjaro' in h)[:3000]))
    r = subprocess.run(['curl', '-sSL', '--max-time', '60', '-o', '/dev/null', '-w', '%{http_code}', 'https://gitlab.manjaro.org/web/iso-info/-/raw/master/file-info.json'], capture_output=True, text=True)
    note('manjaro gitlab status', r.stdout + ' ' + r.stderr[:300])

def minimal(fpr, src):
    data = get(src)
    if not data:
        note('KEYFAIL', f'{fpr} {src}'); return
    h = tempfile.mkdtemp()
    subprocess.run(['gpg', '--homedir', h, '--batch', '--import'], input=data, capture_output=True)
    r = subprocess.run(['gpg', '--homedir', h, '--batch', '--armor', '--export-options', 'export-minimal', '--export', fpr], capture_output=True)
    b = base64.b64encode(r.stdout).decode()
    for i in range(0, len(b), 3900):
        note(f'MKEY {fpr} {i // 3900}', b[i:i + 3900])

if which == 'k1':
    minimal('843938DF228D22F7B3742BC0D94AA3F0EFE21092', 'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x843938DF228D22F7B3742BC0D94AA3F0EFE21092')
    minimal('DF9B9C49EAA9298432589D76DA87E80D6294BE9B', 'https://keyring.debian.org/pks/lookup?op=get&search=0xDF9B9C49EAA9298432589D76DA87E80D6294BE9B')
if which == 'k2':
    minimal('27DEB15644C6B3CF3BD7D291300F846BA25BAE09', 'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x27DEB15644C6B3CF3BD7D291300F846BA25BAE09')
    minimal('3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C', 'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x3E80CA1A8B89F69CBA57D98A76A5EF9054449A5C')
if which == 'k3':
    minimal('882DCFE48E2051D48E2562ABF3B607488DB35A47', 'https://keys.openpgp.org/vks/v1/by-fingerprint/882DCFE48E2051D48E2562ABF3B607488DB35A47')
    page = get('https://manjaro.org/products/download/x86') or b''
    isos = sorted(set(re.findall(rb'https://download\.manjaro\.org/[a-z0-9]+/[^"\' ]+?\.iso', page)))
    for u in isos[:3]:
        u = u.decode()
        note('manjaro ' + u.rsplit('/', 1)[1], '\n'.join(e + ' ' + head(u + e) for e in ('', '.sha256', '.sha512', '.sha1', '.sig', '.torrent')) + '\n' + (get(u + '.sha256') or b'none').decode()[:200])
        if u.endswith('.iso') and 'kde' in u:
            sigprobe('manjaro', u + '.sig')
            s = get(u + '.sig')
            if s:
                fp, kid = issuer(s)
                if fp: minimal(fp, f'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x{fp}')

if which == 'k4':
    minimal('DF9B9C49EAA9298432589D76DA87E80D6294BE9B', 'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0xDF9B9C49EAA9298432589D76DA87E80D6294BE9B')
    page = get('https://manjaro.org/products/download/x86') or b''
    isos = sorted(set(re.findall(rb'https://download\.manjaro\.org/kde/[^"\' ]+?\.iso', page)))
    note('manjaro kde', ' '.join(i.decode() for i in isos))
    for u in isos[:1]:
        u = u.decode()
        s = get(u + '.sig')
        if s:
            fp, kid = issuer(s)
            note('SIG manjaro kde', f'{fp} {kid}')
            if fp: minimal(fp, f'https://keyserver.ubuntu.com/pks/lookup?op=get&options=mr&search=0x{fp}')
        note('manjaro sha', (get(u + '.sha256') or b'none').decode()[:200])
