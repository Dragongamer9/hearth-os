#!/usr/bin/env python3
"""Stand-in files for the Hearth Files screenshots (Desktop Mode test): a few documents, pictures (real PNGs, so
thumbnails show), a folder, and recently-used.xbel so the Home page has recent files. Usage: files_redesign_sample.py HOME"""
import os
import struct
import sys
import time
import urllib.parse
import zlib


def png(path, w, h, a, b):
    """A w×h PNG fading from colour a to colour b (top-left to bottom-right)."""
    rows = []
    for y in range(h):
        row = bytearray([0])
        for x in range(w):
            t = (x / w + y / h) / 2
            row += bytes(int(a[i] + (b[i] - a[i]) * t) for i in range(3))
        rows.append(bytes(row))
    raw = zlib.compress(b''.join(rows), 9)

    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)
    with open(path, 'wb') as f:
        f.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0))
                + chunk(b'IDAT', raw) + chunk(b'IEND', b''))


def main(home):
    docs, pics, dl = (os.path.join(home, d) for d in ('Documents', 'Pictures', 'Downloads'))
    for d in (docs, pics, dl, os.path.join(docs, 'Reports'), os.path.join(docs, 'Recipes'), os.path.join(home, 'Music'),
              os.path.join(home, 'Videos'), os.path.join(home, '.local/share')):
        os.makedirs(d, exist_ok=True)
    with open(os.path.join(docs, 'notes.txt'), 'w') as f:
        f.write('Things to do this weekend\n')
    with open(os.path.join(docs, 'budget.ods'), 'wb') as f:
        f.write(b'PK\x03\x04' + os.urandom(18000))
    with open(os.path.join(docs, 'trip-plan.pdf'), 'wb') as f:
        f.write(b'%PDF-1.4\n' + os.urandom(52000))
    png(os.path.join(docs, 'photo-1.png'), 320, 240, (255, 140, 60), (120, 40, 160))
    png(os.path.join(docs, 'photo-2.png'), 320, 240, (40, 170, 200), (20, 60, 120))
    for i, (a, b) in enumerate([((250, 200, 80), (220, 80, 60)), ((60, 200, 120), (20, 90, 80)),
                                ((200, 120, 240), (60, 40, 120)), ((240, 240, 240), (90, 110, 140))]):
        png(os.path.join(pics, 'holiday-%d.png' % (i + 1)), 320, 240, a, b)
    with open(os.path.join(dl, 'mods.zip'), 'wb') as f:
        f.write(b'PK\x05\x06' + bytes(18))
    now = time.time()
    recent = [os.path.join(docs, 'trip-plan.pdf'), os.path.join(pics, 'holiday-2.png'), os.path.join(docs, 'notes.txt'),
              os.path.join(docs, 'budget.ods'), os.path.join(dl, 'mods.zip'), os.path.join(pics, 'holiday-1.png')]
    marks = []
    for i, p in enumerate(recent):
        when = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now - i * 5400))
        marks.append('  <bookmark href="file://%s" added="%s" modified="%s" visited="%s"/>'
                     % (urllib.parse.quote(p), when, when, when))
    with open(os.path.join(home, '.local/share/recently-used.xbel'), 'w') as f:
        f.write('<?xml version="1.0" encoding="UTF-8"?>\n<xbel version="1.0">\n' + '\n'.join(marks) + '\n</xbel>\n')


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser('~'))
