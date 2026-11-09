#!/usr/bin/env python3
"""Makes Hearth Settings' search index (hosts/linux/desktop/shell/settings-index.js) from the page registry
(settings-pages.js) and every page's cards: each card with a plain title (SetRow, SetSwitch, SetChoice, SetSlider,
SetCard, SetLinkCard, SetExpander, SetDropdown) becomes a search entry with its description and the page it's on, so the search
finds single settings with their path ("Go to sleep after · System › Power & battery"). Pages themselves, and their old
names, are searched straight from the registry. A file shown as several pages (About, Boot & startup) marks its parts
with `// @part <name>` comments; a card belongs to the page whose `props.part` is that name.
Run it again after changing a page's cards (hosts/linux/test/test_settings_redesign.py checks it's up to date):

  python3 tools/build-settings-index.py [--check]
"""
import json
import os
import re
import sys

SHELL = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'hosts', 'linux', 'desktop', 'shell')
CARDS = r'\b(SetRow|SetSwitch|SetChoice|SetSlider|SetCard|SetLinkCard|SetExpander|SetDropdown|StartInChoice)\s*\{'
# Pages whose cards repeat a setting whose real home is elsewhere (Home's shortcuts): indexed last, only new titles.
LAST = ('SetHome.qml', 'SetCategory.qml')


def registry():
    """The registry's pages, one per line: {id, file, cat, part}."""
    src = open(os.path.join(SHELL, 'settings-pages.js')).read()
    body = src[src.index('var PAGES = ['):src.index('\n];', src.index('var PAGES = ['))]
    pages = []
    for line in body.splitlines():
        m = re.search(r'\{ id: "([\w-]+)", file: "([\w.]+)", cat: "(\w+)"', line)
        if not m:
            continue
        part = re.search(r'props: \{ part: "(\w+)" \}', line)
        pages.append({'id': m.group(1), 'file': m.group(2), 'cat': m.group(3), 'part': part.group(1) if part else None})
    return pages


def block_end(s, i):
    depth, j, instr = 0, i, None
    while j < len(s):
        c = s[j]
        if instr:
            if c == '\\':
                j += 2
                continue
            if c == instr:
                instr = None
        elif c in '"\'':
            instr = c
        elif c == '/' and s[j + 1:j + 2] == '/':
            j = s.find('\n', j)
            if j < 0:
                return len(s) - 1
            continue
        elif c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return j
        j += 1
    return len(s) - 1


def top_level(body):
    while True:
        t = re.sub(r'\{[^{}]*\}', '', body)
        if t == body:
            return t
        body = t


def literal(top, key):
    m = re.search(r'(?:^|[\s;{])' + key + r':\s*"((?:[^"\\]|\\.)*)"\s*(?=;|\n|\}|$)', top)
    return m.group(1).replace('\\"', '"') if m else ''


def cards(path):
    s = open(path).read()
    parts = [(m.start(), m.group(1)) for m in re.finditer(r'//\s*@part\s+(\w+)', s)]
    out = []
    for m in re.finditer(CARDS, s):
        start = m.end() - 1
        top = top_level(s[start + 1:block_end(s, start)])
        kind = m.group(1)
        title = literal(top, 'title') or ('Start in' if kind == 'StartInChoice' else '')
        if not title or len(title) < 3:
            continue
        sub = literal(top, 'sub') or ('What this PC opens when it turns on or you sign in' if kind == 'StartInChoice' else '')
        icon = literal(top, 'icon') or ('power' if kind == 'StartInChoice' else '')
        part = None
        for pos, name in parts:
            if pos < m.start():
                part = name
        out.append({'t': title, 'd': sub, 'i': icon, 'part': part})
    return out


def build():
    pages = registry()
    entries, seen = [], set()
    files = sorted({p['file'] for p in pages}, key=lambda f: (f in LAST, f))
    for f in files:
        mine = [p for p in pages if p['file'] == f]
        for c in cards(os.path.join(SHELL, f)):
            page = next((p for p in mine if p['part'] == c['part']), None) or (mine[0] if not c['part'] or all(p['part'] is None for p in mine) else None)
            if not page:
                continue
            key = (c['t'].lower(), page['id'])
            if key in seen or (f in LAST and any(e['t'].lower() == c['t'].lower() for e in entries)):
                continue
            seen.add(key)
            e = {'t': c['t'], 'p': page['id']}
            if c['d']:
                e['d'] = c['d']
            if c['i']:
                e['i'] = c['i']
            entries.append(e)
    return ('.pragma library\n// Made by tools/build-settings-index.py from settings-pages.js and the pages\' cards: Hearth Settings\' search.\n'
            '// {t: title, d: description, i: icon, p: the page it\'s on}. Don\'t edit by hand: run the tool again.\n'
            'var ENTRIES = ' + json.dumps(entries, indent=0, ensure_ascii=False) + ';\n')


if __name__ == '__main__':
    out = os.path.join(SHELL, 'settings-index.js')
    if '--check' in sys.argv:
        with open(out) as f:
            sys.exit(0 if f.read() == build() else 1)
    with open(out, 'w') as f:
        f.write(build())
    print(out)
