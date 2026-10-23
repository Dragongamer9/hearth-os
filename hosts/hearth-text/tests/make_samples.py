#!/usr/bin/python3
"""Sample files for trying Hearth Text (the CI run and tests/page_check.py): a shell script, Python, an INI file
with Windows line endings, Latin-1 and UTF-16 text, and a 60 MB log.  make_samples.py DIR [--no-big]"""
import os
import sys

SAMPLES = {
    'sample.sh': '''#!/bin/bash
# Back up the games folder, keeping the last 5 copies.
set -eu
name="${1:-games}"
dest="$HOME/Backups/$name-$(date +%F)"
for f in "$HOME"/Games/*; do
  [ -d "$f" ] && echo "copying $f" || continue
  cp -a "$f" "$dest/"   # keeps times and permissions
done
ls -1d "$HOME"/Backups/"$name"-* | head -n -5 | xargs -r rm -rf
exit 0
''',
    'app.py': '''#!/usr/bin/env python3
"""Lists the biggest files in a folder."""
import os, sys


def biggest(folder: str, count: int = 10) -> list[tuple[int, str]]:
    sizes = []
    for root, _dirs, names in os.walk(folder):
        for name in names:
            path = os.path.join(root, name)
            try:
                sizes.append((os.path.getsize(path), path))
            except OSError:
                pass  # gone, or not ours
    return sorted(sizes, reverse=True)[:count]


if __name__ == "__main__":
    for size, path in biggest(sys.argv[1] if len(sys.argv) > 1 else "."):
        print(f"{size / 1e6:8.1f} MB  {path}")
''',
    'config.ini': '''; Hearth sample settings
[Desktop Entry]
Name=Sample App
Exec=sample-app %U
Icon=hearth
Terminal=false

[display]
width = 1280
height = 800
fullscreen = true
''',
    'app.json': '{\n  "name": "hearth-text",\n  "version": 1,\n  "tags": ["editor", "linux"],\n  "ok": true,\n  "size": null\n}\n',
    'notes.md': '# Notes\n\nSome **bold** and *italic* text, `code` and a [link](https://example.org).\n\n- one\n- two\n',
}


def main():
    d = sys.argv[1]
    os.makedirs(d, exist_ok=True)
    for name, text in SAMPLES.items():
        with open(os.path.join(d, name), 'w', encoding='utf-8') as f:
            f.write(text)
    with open(os.path.join(d, 'crlf.ini'), 'wb') as f:
        f.write(b'[section]\r\nkey = value\r\nother = 2\r\n')
    with open(os.path.join(d, 'latin1.txt'), 'wb') as f:
        f.write('Café, crème brûlée and jalapeño (this file is Latin-1)\n'.encode('latin-1'))
    with open(os.path.join(d, 'utf16.txt'), 'wb') as f:
        f.write('﻿UTF-16 text with a byte-order mark: ✓ ü ß\r\n'.encode('utf-16-le'))
    with open(os.path.join(d, 'program.bin'), 'wb') as f:
        f.write(b'\x7fELF\x02\x01\x01\x00' + bytes(range(256)) * 4)
    if '--no-big' not in sys.argv:
        with open(os.path.join(d, 'huge.log'), 'w') as f:
            i = 0
            while f.tell() < 60 * 1024 * 1024:
                f.write('2026-10-22 12:%02d:%02d INFO [worker-%d] request %d took %d ms\n' % (i // 60 % 60, i % 60, i % 8, i, i % 997))
                if i % 5000 == 4999:
                    f.write('2026-10-22 12:%02d:%02d WARN [worker-%d] slow disk "sda"\n' % (i // 60 % 60, i % 60, i % 8))
                i += 1
            f.write('2026-10-22 23:59:59 ERROR the very last line\n')


if __name__ == '__main__':
    main()
