"""Google Chrome on Hearth OS (1.12, the owner's choice): an option next to Hearth Browser (Firefox), for sites that only
let you sign in with Chrome (Xodus Nexus). Chrome is Google's and closed-source, so it can't ship inside Hearth's image:
when you pick it (first-time setup, the one-time "Pick your browser" after updating to 1.12, Settings › Web browser),
Google's own package is downloaded from dl.google.com and checked the way apt checks it, then unpacked into your home
folder (~/.local/opt/google-chrome). Nothing is installed system-wide.

The check: Google's signing key (dl.google.com/linux/linux_signing_key.pub) must have one of the fingerprints below; the
repository's InRelease must be signed by it (gpgv); the Packages list must match the SHA-256 InRelease gives; Chrome's
.deb must match the SHA-256 the Packages list gives. Anything that doesn't match: nothing is installed.
Used by the hearth-browser helper next to this file (`hearth-browser chrome …`)."""
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

REPO = 'https://dl.google.com/linux/chrome/deb'
KEY_URL = 'https://dl.google.com/linux/linux_signing_key.pub'
# Google's Linux package signing key (Google Inc. Linux Package Signing Authority): its primary key's fingerprint.
GOOGLE_FPRS = ('EB4C1BFD4F042F6DDDCCEC917721F63BD38B4796', '4CCA1EAF950CEE4AB83976DCA040830F7FAC5991')
HOSTS = ('dl.google.com', 'dl-ssl.google.com')
PACKAGE = 'google-chrome-stable'
HOME = os.path.expanduser('~')
DATA = os.environ.get('XDG_DATA_HOME') or os.path.join(HOME, '.local', 'share')
ROOT = os.path.join(HOME, '.local', 'opt', 'google-chrome')
DESKTOP_ID = 'hearth-chrome.desktop'
DESKTOP = os.path.join(DATA, 'applications', DESKTOP_ID)
LAUNCHER = os.path.join(HOME, '.local', 'bin', 'google-chrome')
STATE = os.path.join(DATA, 'nimbus', 'chrome.json')
WEEK = 7 * 24 * 3600


class Fail(Exception):
    pass


def _ok(url):
    u = urllib.parse.urlparse(url)
    return u.scheme == 'https' and u.hostname in HOSTS


def get(url, limit=8 << 20, dest=None, progress=None):
    """https from Google's download site only (redirects too): bytes, or the SHA-256 of the file written to dest."""
    class Only(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            if not _ok(newurl):
                raise Fail('a download tried to go somewhere else (%s): stopped' % urllib.parse.urlparse(newurl).hostname)
            return super().redirect_request(req, fp, code, msg, headers, newurl)
    if not _ok(url):
        raise Fail('not an allowed address')
    h = hashlib.sha256()
    got, chunks = 0, []
    try:
        with urllib.request.build_opener(Only).open(urllib.request.Request(url, headers={'User-Agent': 'Hearth-OS'}), timeout=60) as r:
            total = int(r.headers.get('Content-Length') or 0)
            f = open(dest, 'wb') if dest else None
            try:
                while True:
                    c = r.read(1 << 20)
                    if not c:
                        break
                    got += len(c)
                    if got > limit:
                        raise Fail('the download was bigger than expected')
                    h.update(c)
                    (f.write(c) if f else chunks.append(c))
                    if progress and total:
                        progress(got, total)
            finally:
                if f:
                    f.close()
    except (OSError, urllib.error.URLError) as e:
        raise Fail('couldn\'t reach dl.google.com (are you online?): %s' % e)
    return h.hexdigest() if dest else b''.join(chunks)


def run(args, timeout=120, inp=None):
    """(exit code, output). Tests replace this."""
    try:
        p = subprocess.run(args, capture_output=True, timeout=timeout, input=inp)
        return p.returncode, (p.stdout or b'').decode('utf-8', 'replace') + (p.stderr or b'').decode('utf-8', 'replace')
    except (OSError, subprocess.SubprocessError) as e:
        return 127, str(e)


def key_fingerprints(key_path):
    code, out = run(['gpg', '--batch', '--with-colons', '--show-keys', key_path], 30)
    if code != 0:
        raise Fail('Google\'s signing key couldn\'t be read (is gpg on this PC?)')
    fprs, primary = [], False
    for line in out.splitlines():
        f = line.split(':')
        if f[0] == 'pub':
            primary = True
        elif f[0] == 'sub':
            primary = False
        elif f[0] == 'fpr' and primary and len(f) > 9:
            fprs.append(f[9].upper())
    return fprs


def parse_release_sha(release_text, path):
    """The SHA-256 InRelease gives for a file (main/binary-amd64/Packages), or ''."""
    sec = False
    for line in release_text.splitlines():
        if re.match(r'^SHA256:', line):
            sec = True
            continue
        if sec:
            if not line.startswith(' '):
                break
            f = line.split()
            if len(f) == 3 and f[2] == path and re.fullmatch(r'[0-9a-f]{64}', f[0]):
                return f[0]
    return ''


def parse_packages(text, name=PACKAGE):
    """The newest stanza for a package in a Packages list: {version, filename, sha256, size}."""
    best = None
    for stanza in text.split('\n\n'):
        d = {}
        for line in stanza.splitlines():
            m = re.match(r'^([A-Za-z0-9-]+):\s*(.*)$', line)
            if m:
                d[m.group(1)] = m.group(2).strip()
        if d.get('Package') == name and d.get('Filename') and re.fullmatch(r'[0-9a-f]{64}', d.get('SHA256', '')):
            item = {'version': d.get('Version', ''), 'filename': d['Filename'], 'sha256': d['SHA256'], 'size': int(d.get('Size') or 0)}
            if not best or _vkey(item['version']) > _vkey(best['version']):
                best = item
    return best


def _vkey(v):
    return [int(x) for x in re.findall(r'\d+', v or '')]


def newest(fetch=None):
    """Chrome's newest stable release, checked: {version, url, sha256, size}."""
    fetch = fetch or get
    with tempfile.TemporaryDirectory() as t:
        kp = os.path.join(t, 'key.pub')
        with open(kp, 'wb') as f:
            f.write(fetch(KEY_URL, 1 << 20))
        fprs = key_fingerprints(kp)
        if not any(x in GOOGLE_FPRS for x in fprs):
            raise Fail('dl.google.com gave a signing key that isn\'t Google\'s known one: not installed')
        ring = os.path.join(t, 'google.gpg')
        code, out = run(['gpg', '--batch', '--yes', '--dearmor', '-o', ring, kp], 30)
        if code != 0:
            raise Fail('Google\'s signing key couldn\'t be prepared: ' + out.strip()[-120:])
        ir = os.path.join(t, 'InRelease')
        with open(ir, 'wb') as f:
            f.write(fetch(REPO + '/dists/stable/InRelease', 1 << 20))
        code, out = run(['gpgv', '--keyring', ring, ir], 30)
        if code != 0:
            raise Fail('Google\'s package list isn\'t signed by Google\'s key: not installed')
        release = open(ir, encoding='utf-8', errors='replace').read()
    want = parse_release_sha(release, 'main/binary-amd64/Packages')
    if not want:
        raise Fail('Google\'s signed list doesn\'t give a checksum for its packages')
    pk = fetch(REPO + '/dists/stable/main/binary-amd64/Packages', 8 << 20)
    if hashlib.sha256(pk).hexdigest() != want:
        raise Fail('Google\'s package list doesn\'t match its signed checksum: not installed')
    p = parse_packages(pk.decode('utf-8', 'replace'))
    if not p:
        raise Fail('Google\'s package list has no Chrome for this PC')
    return {'version': p['version'], 'url': REPO + '/' + p['filename'].lstrip('/'), 'sha256': p['sha256'], 'size': p['size']}


def load_state():
    try:
        with open(STATE) as f:
            d = json.load(f)
        return d if isinstance(d, dict) else {}
    except (OSError, ValueError):
        return {}


def save_state(st):
    os.makedirs(os.path.dirname(STATE), exist_ok=True)
    with open(STATE + '.tmp', 'w') as f:
        json.dump(st, f, indent=2)
    os.replace(STATE + '.tmp', STATE)


def program(root=None):
    return os.path.join(root or os.path.join(ROOT, 'current'), 'opt', 'google', 'chrome', 'google-chrome')


def install(progress=lambda pct, what: None, rel=None):
    """Download, check and unpack the newest Chrome beside the one you have, then swap them; its menu entry and launcher."""
    if os.uname().machine != 'x86_64':
        raise Fail('Google Chrome for Linux is only made for x86-64 PCs')
    if not shutil.which('dpkg-deb'):
        raise Fail('dpkg-deb isn\'t on this PC')
    progress(3, 'Checking Google\'s signed package list')
    rel = rel or newest()
    os.makedirs(ROOT, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix='.download-', dir=ROOT)
    try:
        deb = os.path.join(tmp, 'chrome.deb')
        got = get(rel['url'], max(rel['size'] * 2, 64 << 20) if rel['size'] else 400 << 20, dest=deb,
                  progress=lambda g, t: progress(5 + int(80 * g / t), 'Downloading Google Chrome'))
        if got != rel['sha256']:
            raise Fail('Chrome\'s download doesn\'t match Google\'s signed checksum: not installed')
        progress(88, 'Unpacking')
        x = os.path.join(tmp, 'x')
        code, out = run(['dpkg-deb', '-x', deb, x], 600)
        if code != 0 or not os.path.isfile(program(x)):
            raise Fail('Chrome couldn\'t be unpacked: ' + out.strip()[-160:])
        dest = os.path.join(ROOT, re.sub(r'[^\w.+-]', '_', rel['version'])[:60] or 'chrome')
        if os.path.isdir(dest):
            shutil.rmtree(dest)
        os.rename(x, dest)
        link = os.path.join(ROOT, 'current')
        prev = os.path.realpath(link) if os.path.lexists(link) else ''
        if os.path.lexists(link + '.new'):
            os.remove(link + '.new')
        os.symlink(os.path.basename(dest), link + '.new')
        os.replace(link + '.new', link)
        for d in glob.glob(os.path.join(ROOT, '*')):
            if os.path.isdir(d) and not os.path.islink(d) and os.path.realpath(d) not in (os.path.realpath(dest), prev) and not os.path.basename(d).startswith('.'):
                shutil.rmtree(d, ignore_errors=True)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    write_entry()
    st = load_state()
    st.update({'version': rel['version'], 'installedAt': int(time.time()), 'checkedAt': int(time.time()), 'removed': False})
    save_state(st)
    progress(100, 'Done')
    return rel['version']


def write_entry():
    os.makedirs(os.path.dirname(LAUNCHER), exist_ok=True)
    # Wayland when there is one (Desktop Mode), else X11; no setuid sandbox in a home folder: Chrome uses its namespace one.
    with open(LAUNCHER + '.tmp', 'w') as f:
        f.write('#!/bin/sh\n# Google Chrome from your home folder (Hearth OS, hearth-browser chrome)\n'
                'exec "%s" --ozone-platform-hint=auto "$@"\n' % program())
    os.chmod(LAUNCHER + '.tmp', 0o755)
    os.replace(LAUNCHER + '.tmp', LAUNCHER)
    icon = next((p for p in (os.path.join(ROOT, 'current', 'opt', 'google', 'chrome', 'product_logo_%d.png' % s) for s in (256, 128, 64)) if os.path.isfile(p)), 'web-browser')
    os.makedirs(os.path.dirname(DESKTOP), exist_ok=True)
    with open(DESKTOP + '.tmp', 'w') as f:
        f.write('[Desktop Entry]\nType=Application\nName=Google Chrome\nGenericName=Web Browser\n'
                'Comment=Google\'s browser, from your home folder\nExec=%s %%U\nIcon=%s\nTerminal=false\n'
                'Categories=Network;WebBrowser;\nStartupWMClass=google-chrome\n'
                'MimeType=text/html;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;\n'
                'Actions=new-window;incognito;\n\n[Desktop Action new-window]\nName=New window\nExec=%s\n\n'
                '[Desktop Action incognito]\nName=New incognito window\nExec=%s --incognito\n' % (LAUNCHER, icon, LAUNCHER, LAUNCHER))
    os.replace(DESKTOP + '.tmp', DESKTOP)


def installed():
    return os.path.isfile(program()) and os.path.exists(LAUNCHER) and os.path.exists(DESKTOP)


def update():
    """A newer Chrome when Google has one (looked for once a week by `ensure`, or when you press Update)."""
    st = load_state()
    rel = newest()
    st['checkedAt'] = int(time.time())
    save_state(st)
    if installed() and rel['version'] == st.get('version'):
        return False
    install(rel=rel)
    return True


def remove():
    shutil.rmtree(ROOT, ignore_errors=True)
    for f in (DESKTOP, LAUNCHER):
        try:
            os.remove(f)
        except OSError:
            pass
    st = load_state()
    st.update({'removed': True})
    st.pop('version', None)
    save_state(st)


def status():
    st = load_state()
    return {'installed': installed(), 'version': st.get('version') if installed() else None, 'checkedAt': st.get('checkedAt'),
            'installedAt': st.get('installedAt'), 'name': 'Google Chrome', 'desktop': DESKTOP_ID,
            'note': 'Must have for Xodus Nexus'}
