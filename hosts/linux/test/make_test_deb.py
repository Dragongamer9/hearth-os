#!/usr/bin/python3
"""Makes a tiny .deb without dpkg (an ar archive with control.tar and data.tar), for the tests and the image
checks (base-*.yml): package hearth-hello, a script /usr/bin/hearth-hello that prints "hello from the Debian box",
and its app menu entry /usr/share/applications/hearth-hello.desktop.

  python3 make_test_deb.py OUT.deb [--control-compression gz|xz|none] [--package NAME] [--arch amd64|all|...]
"""
import gzip
import io
import lzma
import sys
import tarfile
import time

CONTROL = """Package: {package}
Version: 1.0-1
Architecture: {arch}
Maintainer: Hearth Tests <tests@example.invalid>
Installed-Size: 12
Depends: libc6 (>= 2.34), coreutils | busybox
Section: utils
Priority: optional
Homepage: https://example.invalid/hearth-hello
Description: Hello from Hearth's Debian box
 A tiny package used by Hearth's tests: one command and one app menu entry.
 .
 Nothing else.
"""
SCRIPT = '#!/bin/sh\necho "hello from the Debian box"\n'
DESKTOP = """[Desktop Entry]
Type=Application
Name=Hearth Hello
Comment=Says hello
Exec=hearth-hello
Icon=utilities-terminal
Terminal=true
Categories=Utility;
"""


def tar_bytes(files, compression):
    """files: [(name, bytes, mode)] (folders made for them) -> a tar, compressed."""
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode='w', format=tarfile.GNU_FORMAT) as t:
        dirs = set()
        for name, _, _ in files:
            parts = name.split('/')[1:-1]
            for i in range(1, len(parts) + 1):
                dirs.add('./' + '/'.join(parts[:i]))
        for d in sorted(dirs):
            ti = tarfile.TarInfo(d)
            ti.type, ti.mode, ti.mtime = tarfile.DIRTYPE, 0o755, int(time.time())
            t.addfile(ti)
        for name, data, mode in files:
            ti = tarfile.TarInfo(name)
            ti.size, ti.mode, ti.mtime = len(data), mode, int(time.time())
            t.addfile(ti, io.BytesIO(data))
    raw = buf.getvalue()
    if compression == 'gz':
        return gzip.compress(raw)
    if compression == 'xz':
        return lzma.compress(raw)
    return raw


def ar_member(name, data):
    head = '%-16s%-12d%-6d%-6d%-8s%-10d`\n' % (name, int(time.time()), 0, 0, '100644', len(data))
    out = head.encode('ascii') + data
    if len(data) % 2:
        out += b'\n'
    return out


def make(path, package='hearth-hello', arch='all', control_compression='gz'):
    ext = {'gz': '.gz', 'xz': '.xz', 'none': ''}[control_compression]
    control = tar_bytes([('./control', CONTROL.format(package=package, arch=arch).encode(), 0o644)], control_compression)
    data = tar_bytes([('./usr/bin/' + package, SCRIPT.encode(), 0o755),
                      ('./usr/share/applications/' + package + '.desktop', DESKTOP.encode(), 0o644)], 'xz')
    with open(path, 'wb') as f:
        f.write(b'!<arch>\n')
        f.write(ar_member('debian-binary', b'2.0\n'))
        f.write(ar_member('control.tar' + ext, control))
        f.write(ar_member('data.tar.xz', data))
    return path


if __name__ == '__main__':
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(2)
    opts = {'package': 'hearth-hello', 'arch': 'all', 'control_compression': 'gz'}
    for flag, key in (('--package', 'package'), ('--arch', 'arch'), ('--control-compression', 'control_compression')):
        if flag in args:
            opts[key] = args[args.index(flag) + 1]
    make(args[0], **opts)
    print(args[0])
