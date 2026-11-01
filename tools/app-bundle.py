#!/usr/bin/env python3
"""Hearth app updates (docs/APP-UPDATES.md): the app ABI tag and the app bundle.

  app-bundle.py abi
      The image's app ABI tag: os/app-abi's number plus a fingerprint of the image-only files the app talks to (the
      root helper, its polkit rules, the session scripts). base-anatase.yml / base-peppermint.yml write it into the
      image (/usr/share/nimbus/app-abi); release.yml puts the same tag in each bundle's manifest. A bundle only runs on
      an image with exactly its tag, so a bundle never meets image-side files it wasn't built with.

  app-bundle.py pack --src DIR --version V --edition E [--arch x86_64] [--abi TAG] [--min-image V] [--commit SHA] --out PREFIX
      Packs DIR (nimbus, electron/...) into PREFIX.tar.zst and writes PREFIX.json, the manifest (every file's SHA-256,
      size and mode; the archive's SHA-256 and size). release.yml signs PREFIX.json (ssh-keygen -Y sign -n hearth-app).

Only the standard library, tar and zstd. Never signs anything itself and never reads a key.
"""
import argparse
import hashlib
import json
import os
import stat
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))

# Image-only files the app depends on (they're never in a bundle). A change to any of them makes a new ABI tag.
ABI_FILES = [
    'os/app-abi',
    'os/system_files/usr/libexec/nimbus/hearth-admin',
    'hosts/linux/session/nimbus-session',
    'hosts/linux/session/nimbus-mode',
    'hosts/linux/session/nimbus-watchdog',
]
ABI_DIRS = ['os/system_files/usr/share/polkit-1']


def abi_tag(root=ROOT):
    files = list(ABI_FILES)
    for d in ABI_DIRS:
        for base, _, names in os.walk(os.path.join(root, d)):
            for n in names:
                files.append(os.path.relpath(os.path.join(base, n), root))
    h = hashlib.sha256()
    for rel in sorted(set(files)):
        p = os.path.join(root, rel)
        if not os.path.isfile(p):
            continue
        h.update(rel.encode() + b'\0')
        with open(p, 'rb') as f:
            h.update(hashlib.sha256(f.read()).digest())
    number = '0'
    try:
        for line in open(os.path.join(root, 'os/app-abi')):
            line = line.strip()
            if line and not line.startswith('#'):
                number = line.split()[0]
                break
    except OSError:
        pass
    return f'{number}-{h.hexdigest()[:16]}'


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def safe_rel(p):
    parts = p.split('/')
    return p and not p.startswith('/') and all(x not in ('', '.', '..') for x in parts)


def manifest_files(src):
    out = []
    for base, dirs, names in os.walk(src):
        dirs.sort()
        for n in sorted(names):
            full = os.path.join(base, n)
            rel = os.path.relpath(full, src)
            if not safe_rel(rel):
                raise SystemExit(f'unsafe path in the bundle: {rel}')
            st = os.lstat(full)
            if stat.S_ISLNK(st.st_mode):
                target = os.readlink(full)
                if not safe_rel(target):
                    raise SystemExit(f'{rel} links outside the bundle ({target})')
                out.append({'path': rel, 'link': target})
            elif stat.S_ISREG(st.st_mode):
                out.append({'path': rel, 'sha256': sha256(full), 'size': st.st_size, 'mode': 0o755 if st.st_mode & 0o111 else 0o644})
            else:
                raise SystemExit(f'{rel} is not a file')
    return out


def pack(a):
    src = os.path.abspath(a.src)
    if not os.path.isfile(os.path.join(src, 'nimbus')) or not os.access(os.path.join(src, 'nimbus'), os.X_OK):
        raise SystemExit('the bundle needs its program (nimbus) at the top')
    # Chromium's setuid sandbox helper can't work from a home folder (Electron refuses to start with one that isn't
    # setuid root): the bundle's Electron uses the user-namespace sandbox instead, and Hearth checks it can.
    sandbox = os.path.join(src, 'electron', 'chrome-sandbox')
    if os.path.lexists(sandbox):
        os.remove(sandbox)
    files = manifest_files(src)
    archive = a.out + '.tar.zst'
    tarf = a.out + '.tar'
    subprocess.run(['tar', '-C', src, '--sort=name', '--owner=0', '--group=0', '--numeric-owner', '--mtime=@0', '-cf', tarf, '.'], check=True)
    subprocess.run(['zstd', '-q', '-f', f'-{a.level}', '-T0', '--rm', '-o', archive, tarf], check=True)
    m = {
        'format': 1,
        'name': 'hearth-app',
        'version': a.version,
        'edition': a.edition,
        'arch': a.arch,
        'abi': a.abi or abi_tag(),
        'min_image_version': a.min_image,
        'max_image_version': '',
        'commit': a.commit,
        'archive': os.path.basename(archive),
        'sha256': sha256(archive),
        'size': os.path.getsize(archive),
        'files': files,
    }
    with open(a.out + '.json', 'w') as f:
        json.dump(m, f, indent=1)
        f.write('\n')
    print(f'{archive}: {m["size"]} bytes, {len(files)} files, ABI {m["abi"]}')


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest='cmd', required=True)
    sub.add_parser('abi')
    k = sub.add_parser('pack')
    k.add_argument('--src', required=True)
    k.add_argument('--version', required=True)
    k.add_argument('--edition', required=True)
    k.add_argument('--arch', default='x86_64')
    k.add_argument('--abi', default='')
    k.add_argument('--min-image', default='')
    k.add_argument('--commit', default='')
    k.add_argument('--level', default='10')
    k.add_argument('--out', required=True)
    a = p.parse_args()
    if a.cmd == 'abi':
        print(abi_tag())
    else:
        pack(a)


if __name__ == '__main__':
    sys.exit(main())
