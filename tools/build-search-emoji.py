#!/usr/bin/python3
"""Makes hosts/linux/desktop/shell/search/emoji-data.js, the emoji the app menus' search finds (search/emoji.js), from
Hearth's Emoji & symbols data (hosts/hearth-emoji/ui/emoji-data.js): each emoji, its name and its search words, without
kaomoji, letters, numbers and punctuation. Run again when that file changes; hosts/linux/test/test_search_plugins.py
checks the two match.
  python3 tools/build-search-emoji.py
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SRC = ROOT / 'hosts' / 'hearth-emoji' / 'ui' / 'emoji-data.js'
OUT = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'search' / 'emoji-data.js'
SKIP = {'kaomoji', 'letters', 'numbers', 'punctuation'}


def rows(src=SRC):
    s = src.read_text(encoding='utf-8')
    data = json.loads(s[s.index('['):s.rindex(']') + 1])
    return [[d['e'], d['n'].lower(), ' '.join(d.get('k') or [])] for d in data if d.get('g') not in SKIP]


def text(src=SRC):
    body = ',\n'.join(json.dumps(r, ensure_ascii=False) for r in rows(src))
    return ('.pragma library\n// Made by tools/build-search-emoji.py from hosts/hearth-emoji/ui/emoji-data.js: don\'t edit by hand.\n'
            '// [emoji, name, search words]\nvar EMOJI = [\n' + body + '\n];\n')


if __name__ == '__main__':
    OUT.write_text(text(), encoding='utf-8')
    print('%s: %d emoji' % (OUT.relative_to(ROOT), len(rows())))
    sys.exit(0)
