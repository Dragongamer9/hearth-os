#!/usr/bin/python3
"""Smaller updates (1.13): every file in the image gets a component, so chunkah (the rechunk step in
base-peppermint.yml) puts files that change together into the same layer, and a device downloads only the layers
that really changed. Run once, at the very end of build.sh (after the bootc layout).

Without it every file landed in chunkah's one "unclaimed" component, whose layer changed with every build (its
timestamps were the build's), so every update was the whole 2-3 GB image.

  - A file from a Debian package: "deb/<source package>" (a source package's binary packages change together),
    from dpkg's file lists. The kernel, Nvidia, Mesa and firmware are marked as changing weekly, the rest monthly.
  - Anything else (Hearth's own apps, what the builder compiled, generated files): "hearth/<folder>", by where it
    is: /usr/lib/hearth-office, /usr/share/nimbus/desktop, /usr/lib/hhd ... so each Hearth app is its own piece
    and a new Hearth Files doesn't make the Electron app download again. Marked as changing weekly.

Only regular files are marked (Linux doesn't allow user.* attributes on symbolic links, and a mark on a folder
would cover everything under it). Prints a one-line summary; never fails the build (a file it can't mark just
stays unclaimed, which only makes updates bigger).

  components.py [ROOT [DPKG_ADMINDIR]]     defaults: / and /usr/lib/sysimage/dpkg
"""
import collections
import json
import os
import re
import subprocess
import sys

XATTR = 'user.component'
INTERVAL = 'user.update-interval'
# Sources whose new versions come often (the kernel from backports, Nvidia's own repository, Mesa, firmware).
WEEKLY = re.compile(r'^(linux(-signed)?(-amd64)?|linux-.*|nvidia-.*|mesa|firmware-.*|libdrm|intel-.*|amd64-microcode|'
                    r'steam.*|gamescope|wine.*|tailscale|cuda-.*)$')
# Folders whose sub-folders are each their own piece (Hearth's own trees).
DEEP = {('usr', 'share', 'nimbus'), ('usr', 'share', 'hearth'), ('usr', 'lib', 'nimbus'), ('usr', 'lib', 'hearth'),
        ('usr', 'libexec', 'nimbus'), ('usr', 'lib', 'opt')}
SKIP_TOP = {'proc', 'sys', 'dev', 'run', 'tmp', 'var', 'sysroot', 'ostree', 'boot', 'ctx', 'hearth-debs'}  # ctx, hearth-debs: build-only mounts


def packages(admindir):
    """{package (with :arch when dpkg names it so): source package} from dpkg's database."""
    r = subprocess.run(['dpkg-query', '--admindir=' + admindir, '-W', '-f', '${binary:Package}\t${source:Package}\t${Package}\n'],
                       capture_output=True, text=True)
    out = {}
    for line in r.stdout.splitlines():
        f = line.split('\t')
        if len(f) == 3 and f[0]:
            out[f[0]] = f[1] or f[2]
    return out


def where(root, path):
    """Where a path from dpkg's list is in the finished image: /opt moved to /usr/lib/opt, and /bin, /lib ... are
    links into /usr (merged /usr): the folder is followed, never the file itself (a link stays a link)."""
    if path.startswith('/opt/'):
        path = '/usr/lib/opt/' + path[5:]
    d, b = os.path.split(path)
    real = os.path.realpath(os.path.join(root, d.lstrip('/')))
    return os.path.join(real, b)


def owners(root, admindir, pkgs):
    """{absolute file path in root: source package} for every regular file dpkg says a package put there."""
    info = os.path.join(admindir, 'info')
    out = {}
    for name in sorted(pkgs):
        lst = os.path.join(info, name + '.list')
        if not os.path.isfile(lst):
            base = name.split(':')[0]
            lst = os.path.join(info, base + '.list')
            if not os.path.isfile(lst):
                continue
        try:
            with open(lst, encoding='utf-8', errors='replace') as f:
                lines = f.read().splitlines()
        except OSError:
            continue
        for p in lines:
            if not p.startswith('/') or p == '/.':
                continue
            full = where(root, p)
            if full not in out and os.path.isfile(full) and not os.path.islink(full):
                out[full] = pkgs[name]
    return out


def local_name(rel):
    """The piece for a file no package owns, by its folder (rel: path inside the image, no leading /)."""
    parts = rel.split('/')
    if len(parts) <= 2:
        return 'hearth/' + parts[0]
    depth = 4 if tuple(parts[:3]) in DEEP and len(parts) > 4 else 3
    return 'hearth/' + '/'.join(parts[:min(depth, len(parts) - 1)])


def interval_for(comp):
    if comp.startswith('deb/'):
        return 'weekly' if WEEKLY.match(comp[4:]) else 'monthly'
    return 'weekly'


def walk(root):
    for dirpath, dirs, files in os.walk(root):
        rel = os.path.relpath(dirpath, root)
        if rel == '.':
            dirs[:] = sorted(d for d in dirs if d not in SKIP_TOP)
        else:
            dirs.sort()
        for f in sorted(files):
            p = os.path.join(dirpath, f)
            if os.path.isfile(p) and not os.path.islink(p):
                yield p


def assign(root, owned):
    """{path: component} for every regular file under root."""
    out = {}
    for p in walk(root):
        src = owned.get(p)
        out[p] = ('deb/' + src) if src else local_name(os.path.relpath(p, root))
    return out


def mark(comps):
    """Set the attributes; one update-interval per component (on its first file). Returns (marked, failed)."""
    done, failed, seen = 0, 0, set()
    for p in sorted(comps):
        c = comps[p]
        try:
            os.setxattr(p, XATTR, c.encode(), follow_symlinks=False)
            if c not in seen:
                os.setxattr(p, INTERVAL, interval_for(c).encode(), follow_symlinks=False)
                seen.add(c)
            done += 1
        except OSError:
            failed += 1
    return done, failed


def summary(comps, root):
    sizes = collections.Counter()
    for p, c in comps.items():
        try:
            sizes[c] += os.path.getsize(p)
        except OSError:
            pass
    big = [{'component': c, 'mb': round(s / 1e6)} for c, s in sizes.most_common(12)]
    return {'files': len(comps), 'components': len(sizes),
            'deb': sum(1 for c in sizes if c.startswith('deb/')), 'hearth': sum(1 for c in sizes if c.startswith('hearth/')),
            'biggest': big}


def main(argv):
    root = argv[1] if len(argv) > 1 else '/'
    admindir = argv[2] if len(argv) > 2 else os.path.join(root, 'usr/lib/sysimage/dpkg')
    root = os.path.realpath(root)
    pkgs = packages(admindir)
    comps = assign(root, owners(root, admindir, pkgs))
    done, failed = mark(comps)
    s = summary(comps, root)
    s.update({'marked': done, 'failed': failed, 'packages': len(pkgs)})
    print('components: ' + json.dumps(s))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
