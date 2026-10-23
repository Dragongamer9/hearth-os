#!/usr/bin/env python3
"""Sample documents for checking Hearth Documents (CI, and the page in a browser): a PDF with text, a table of
contents and a link; an EPUB e-book with three chapters; a CBZ comic book. Plain Python, nothing to install.

    python3 samples.py OUT_DIR
"""
import os
import struct
import sys
import zipfile
import zlib

WORDS = ('Hearth OS keeps your games, files and documents close at hand. The quick brown fox jumps over the lazy '
         'dog near the fireside, where the embers glow orange and the flame dances. ').split()


def lines(n, seed):
    out, w = [], seed
    for _ in range(n):
        line = []
        while len(' '.join(line)) < 70:
            line.append(WORDS[w % len(WORDS)])
            w += 7
        out.append(' '.join(line))
    return out


TITLES = ['Getting started', 'Games', 'Files', 'Desktop Mode', 'Controllers', 'A wide page', 'Updates', 'Settings',
          'Handhelds', 'Networking', 'Troubleshooting', 'Index']


def pdf(path, pages=12):
    """A PDF with real text (searchable, selectable), a table of contents with a nested entry, a link, and one
    landscape page."""
    def esc(s):
        return s.replace('\\', '\\\\').replace('(', '\\(').replace(')', '\\)')
    # Object numbers, worked out first.
    FONT, BOLD = 1, 2
    content = [3 + i for i in range(pages)]
    page = [3 + pages + i for i in range(pages)]
    PAGES = 3 + 2 * pages
    LINK = PAGES + 1
    OUTLINE = LINK + 1
    item = [OUTLINE + 1 + i for i in range(pages)]
    kid = [OUTLINE + 1 + pages, OUTLINE + 2 + pages]
    INFO = kid[1] + 1
    CATALOG = INFO + 1
    objs = {FONT: b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>', BOLD: b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>'}
    for i in range(pages):
        wide = i == 5
        w, h = (842, 595) if wide else (595, 842)
        text = [f'BT /F2 26 Tf 60 {h - 90} Td (Chapter {i + 1}: {esc(TITLES[i])}) Tj ET']
        y = h - 140
        for k, line in enumerate(lines(16 if wide else 28, i * 13)):
            if i == 2 and k == 3:
                line = 'Searching for the word fireside should find this line on page three.'
            text.append(f'BT /F1 12 Tf 60 {y} Td ({esc(line)}) Tj ET')
            y -= 22
        if i == 0:
            text.append('BT /F1 12 Tf 60 80 Td (Visit https://github.com/Dragongamer9/hearth-os for more.) Tj ET')
        stream = '\n'.join(text).encode()
        objs[content[i]] = b'<< /Length %d >>\nstream\n' % len(stream) + stream + b'\nendstream'
        annots = f' /Annots [{LINK} 0 R]' if i == 0 else ''
        objs[page[i]] = (f'<< /Type /Page /Parent {PAGES} 0 R /MediaBox [0 0 {w} {h}] /Resources << /Font << /F1 {FONT} 0 R /F2 {BOLD} 0 R >> >> '
                         f'/Contents {content[i]} 0 R{annots} >>').encode()
    objs[PAGES] = f'<< /Type /Pages /Kids [{" ".join(f"{p} 0 R" for p in page)}] /Count {pages} >>'.encode()
    objs[LINK] = b'<< /Type /Annot /Subtype /Link /Rect [60 75 420 92] /Border [0 0 0] /A << /S /URI /URI (https://github.com/Dragongamer9/hearth-os) >> >>'
    objs[OUTLINE] = f'<< /Type /Outlines /First {item[0]} 0 R /Last {item[-1]} 0 R /Count {pages} >>'.encode()
    for i in range(pages):
        parts = [f'/Title (Chapter {i + 1}: {TITLES[i]})', f'/Parent {OUTLINE} 0 R', f'/Dest [{page[i]} 0 R /Fit]']
        if i > 0:
            parts.append(f'/Prev {item[i - 1]} 0 R')
        if i < pages - 1:
            parts.append(f'/Next {item[i + 1]} 0 R')
        if i == 2:
            parts.append(f'/First {kid[0]} 0 R /Last {kid[1]} 0 R /Count -2')
        objs[item[i]] = ('<< ' + ' '.join(parts) + ' >>').encode()
    objs[kid[0]] = f'<< /Title (Copying files) /Parent {item[2]} 0 R /Next {kid[1]} 0 R /Dest [{page[2]} 0 R /XYZ 0 600 0] >>'.encode()
    objs[kid[1]] = f'<< /Title (USB drives) /Parent {item[2]} 0 R /Prev {kid[0]} 0 R /Dest [{page[2]} 0 R /XYZ 0 300 0] >>'.encode()
    objs[INFO] = b'<< /Title (Hearth OS Guide) /Author (Hearth) >>'
    objs[CATALOG] = f'<< /Type /Catalog /Pages {PAGES} 0 R /Outlines {OUTLINE} 0 R /PageMode /UseOutlines >>'.encode()
    out = bytearray(b'%PDF-1.7\n%\xe2\xe3\xcf\xd3\n')
    offsets = []
    for n in range(1, CATALOG + 1):
        offsets.append(len(out))
        out += b'%d 0 obj\n' % n + objs[n] + b'\nendobj\n'
    xref = len(out)
    out += b'xref\n0 %d\n0000000000 65535 f \n' % (CATALOG + 1)
    for o in offsets:
        out += b'%010d 00000 n \n' % o
    out += b'trailer\n<< /Size %d /Root %d 0 R /Info %d 0 R >>\nstartxref\n%d\n%%%%EOF\n' % (CATALOG + 1, CATALOG, INFO, xref)
    open(path, 'wb').write(out)


def epub(path):
    chapters = [('The fireside', 18), ('Embers in the dark', 26), ('Morning light', 14)]
    z = zipfile.ZipFile(path, 'w')
    z.writestr(zipfile.ZipInfo('mimetype'), 'application/epub+zip', compress_type=zipfile.ZIP_STORED)
    z.writestr('META-INF/container.xml', '<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">'
               '<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>')
    items, spine, nav = [], [], []
    for i, (title, paras) in enumerate(chapters):
        body = ''.join(f'<p>{" ".join(lines(4, i * 31 + k))}</p>' for k in range(paras))
        if i == 1:
            body = '<p>The word lantern appears only here, in the second chapter.</p>' + body
        z.writestr(f'OEBPS/ch{i + 1}.xhtml', f'<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>{title}</title></head>'
                   f'<body><h1>Chapter {i + 1}: {title}</h1>{body}</body></html>')
        items.append(f'<item id="ch{i + 1}" href="ch{i + 1}.xhtml" media-type="application/xhtml+xml"/>')
        spine.append(f'<itemref idref="ch{i + 1}"/>')
        nav.append(f'<li><a href="ch{i + 1}.xhtml">Chapter {i + 1}: {title}</a></li>')
    z.writestr('OEBPS/nav.xhtml', '<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">'
               f'<head><title>Contents</title></head><body><nav epub:type="toc"><ol>{"".join(nav)}</ol></nav></body></html>')
    z.writestr('OEBPS/content.opf', '<?xml version="1.0" encoding="utf-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">'
               '<metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="id">hearth-sample-book</dc:identifier><dc:title>A Hearth Story</dc:title>'
               '<dc:creator>Hearth</dc:creator><dc:language>en</dc:language><meta property="dcterms:modified">2026-01-01T00:00:00Z</meta></metadata>'
               f'<manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>{"".join(items)}</manifest>'
               f'<spine>{"".join(spine)}</spine></package>')
    z.close()


def png(w, h, rgb, band):
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        c = band if (y // 40) % 2 else rgb
        raw += bytes(c) * w
    chunk = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(bytes(raw), 6)) + chunk(b'IEND', b'')


def cbz(path):
    z = zipfile.ZipFile(path, 'w')
    colors = [((255, 138, 61), (42, 18, 6)), ((255, 209, 102), (60, 40, 10)), ((111, 208, 140), (20, 50, 30)), ((74, 163, 255), (10, 30, 60))]
    for i, (a, b) in enumerate(colors):
        z.writestr(f'Comic/page{i + 1:02d}.png', png(600, 900, a, b))
    z.close()


if __name__ == '__main__':
    out = sys.argv[1] if len(sys.argv) > 1 else '.'
    os.makedirs(out, exist_ok=True)
    pdf(os.path.join(out, 'Hearth Guide.pdf'))
    epub(os.path.join(out, 'A Hearth Story.epub'))
    cbz(os.path.join(out, 'Ember Comics.cbz'))
    print('samples in', out)
