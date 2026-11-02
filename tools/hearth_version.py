#!/usr/bin/env python3
"""Hearth OS version numbers (docs/UPDATES.md, "Version numbers").

From 1.0 on, Hearth OS is numbered MAJOR.MINOR (1.0, 1.1 … 1.10, then 2.0). Build files need three parts, so the
files say 1.0.0, 1.1.0; people only ever see "1.0", "1.1". 0.x versions (0.6.63) are shown whole, as they were.
Each release's date comes from its heading in docs/ROADMAP.md: "## 1.0 (released 2026-11-02)" (the release step
writes it). Older headings ("## 0.6.63 (released)") have no date, and then only the number is shown.

  hearth_version.py show V            the version as people see it (1.0.0 -> 1.0)
  hearth_version.py date V [ROADMAP]  V's release date (YYYY-MM-DD) from docs/ROADMAP.md, or nothing
  hearth_version.py newer A B         exit 0 when A is newer than B
"""
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
# "## 1.0", "## 1.0 (released)", "## 1.0 (released 2026-11-02)", "## 0.6.63 (released)"
HEADING = re.compile(r'^##\s+v?(\d+(?:\.\d+){1,3})(?:\s*\(released(?:\s+(\d{4}-\d{2}-\d{2}))?\))?\s*$')


def parts(v):
    """The numbers of a version ("v1.0.0-linux.3" -> [1, 0, 0]); missing parts count as 0 when comparing."""
    m = re.match(r'^v?(\d+(?:\.\d+)*)', str(v or '').strip())
    return [int(p) for p in m.group(1).split('.')] if m else [0]


def key(v, n=4):
    p = parts(v)
    return tuple(p + [0] * (n - len(p)))[:max(n, len(p))]


def newer(a, b):
    n = max(len(parts(a)), len(parts(b)), 3)
    return key(a, n) > key(b, n)


def show(v):
    """1.0.0 -> 1.0, 1.10.0 -> 1.10, v2.0.0 -> v2.0, 1.0.0+app.3 -> 1.0+app.3; 0.6.63 and 1.0.1 stay as they are."""
    v = str(v or '').strip()
    m = re.match(r'^(v?)(\d+)\.(\d+)\.0(?=$|[+-])', v)
    if m and int(m.group(2)) >= 1:
        return f'{m.group(1)}{m.group(2)}.{m.group(3)}{v[m.end():]}'
    return v


def releases(text):
    """[(version as written, date or None, released?)] for each version heading, in file order."""
    out = []
    for line in text.splitlines():
        m = HEADING.match(line.strip())
        if m:
            out.append((m.group(1), m.group(2), '(released' in line))
    return out


def date_for(v, text=None):
    """V's release date from docs/ROADMAP.md ("## 1.0 (released 2026-11-02)"), or None."""
    if text is None:
        try:
            text = open(os.path.join(ROOT, 'docs/ROADMAP.md'), encoding='utf-8').read()
        except OSError:
            return None
    want = key(v, 4)
    for ver, date, _ in releases(text):
        if key(ver, 4) == want and date:
            return date
    return None


def main(argv):
    if len(argv) >= 2 and argv[0] == 'show':
        print(show(argv[1]))
        return 0
    if len(argv) >= 2 and argv[0] == 'date':
        text = open(argv[2], encoding='utf-8').read() if len(argv) > 2 else None
        d = date_for(argv[1], text)
        if d:
            print(d)
        return 0
    if len(argv) >= 3 and argv[0] == 'newer':
        return 0 if newer(argv[1], argv[2]) else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
