#!/usr/bin/python3
"""Makes ui/emoji-data.js, the emoji picker's list: every emoji (Unicode's emoji-test.txt) with its name and search
words (CLDR's English annotations), whether it takes a skin tone, then symbols and special characters (named from
Python's own Unicode tables) and kaomoji. Run again when Unicode adds emoji:

  python3 build_data.py emoji-test.txt cldr/common/annotations/en.xml [cldr/common/annotationsDerived/en.xml]

(emoji-test.txt: https://unicode.org/Public/emoji/<version>/; the CLDR files: github.com/unicode-org/cldr.)
"""
import json
import os
import re
import sys
import unicodedata
import xml.etree.ElementTree as ET

TONES = ['\U0001F3FB', '\U0001F3FC', '\U0001F3FD', '\U0001F3FE', '\U0001F3FF']
GROUPS = {'Smileys & Emotion': 'smileys', 'People & Body': 'people', 'Animals & Nature': 'nature', 'Food & Drink': 'food',
          'Travel & Places': 'travel', 'Activities': 'activities', 'Objects': 'objects', 'Symbols': 'symbols', 'Flags': 'flags'}


def annotations(*paths):
    words = {}
    for p in paths:
        if not p or not os.path.exists(p):
            continue
        for a in ET.parse(p).getroot().iter('annotation'):
            if a.get('type') == 'tts':
                continue
            cp = a.get('cp')
            words.setdefault(cp, [])
            for w in (a.text or '').split('|'):
                w = w.strip().lower()
                if w and w not in words[cp]:
                    words[cp].append(w)
    return words


def with_tone(e, tone):
    """The usual way a skin tone goes on: right after the first character (its emoji-style marker dropped)."""
    first, rest = e[0], e[1:]
    if rest.startswith('️'):
        rest = rest[1:]
    return first + tone + rest


def emoji(test_path, words):
    out, group = [], None
    variants = {}
    line_re = re.compile(r'^([0-9A-F ]+?)\s*;\s*fully-qualified\s*#\s*(\S+)\s+E[\d.]+\s+(.+)$')
    for line in open(test_path, encoding='utf-8'):
        line = line.rstrip('\n')
        if line.startswith('# group:'):
            group = GROUPS.get(line.split(':', 1)[1].strip())
            continue
        m = line_re.match(line)
        if not m or not group:
            continue
        chars = ''.join(chr(int(c, 16)) for c in m.group(1).split())
        name = m.group(3)
        if any(t in chars for t in TONES):
            base = name.split(':')[0]
            variants.setdefault(base, []).append(chars)
            continue
        kw = [w for w in words.get(chars, words.get(chars.replace('️', ''), [])) if w != name]
        out.append({'e': chars, 'n': name, 'g': group, 'k': kw[:8]})
    for item in out:
        v = variants.get(item['n'], [])
        single = [x for x in v if sum(x.count(t) for t in TONES) == 1]
        if len(single) >= 5 and all(with_tone(item['e'], t) in single for t in TONES):
            item['s'] = 1
    return out


def named(cps, cat):
    out = []
    for cp in cps:
        c = chr(cp)
        try:
            n = unicodedata.name(c).lower()
        except ValueError:
            continue
        if unicodedata.category(c) in ('Cn', 'Co', 'Cs', 'Cc', 'Mn'):
            continue
        out.append({'e': c, 'n': n, 'g': cat})
    return out


def r(a, b):
    return range(a, b + 1)


SYMBOLS = [
    ('punctuation', [0x2013, 0x2014, 0x2026, 0xAB, 0xBB, 0x2018, 0x2019, 0x201C, 0x201D, 0x201E, 0x2039, 0x203A, 0x2022, 0xB7,
                     0xA7, 0xB6, 0x2020, 0x2021, 0x2030, 0x2032, 0x2033, 0xA1, 0xBF, 0x203D, 0x2E2E, 0x2042, 0x2016, 0x2043, 0x25E6]),
    ('currency', [0x24, 0xA2, 0xA3, 0xA4, 0xA5] + list(r(0x20A0, 0x20C0))),
    ('maths', [0xB1, 0xD7, 0xF7, 0xB0, 0xB5, 0xAC, 0x2030] + list(r(0x2200, 0x22FF)) + list(r(0x2308, 0x230B))),
    ('arrows', list(r(0x2190, 0x21FF)) + list(r(0x27F0, 0x27FF)) + list(r(0x2B00, 0x2B11))),
    ('letters', list(r(0xC0, 0xFF)) + list(r(0x100, 0x17F)) + list(r(0x391, 0x3A9)) + list(r(0x3B1, 0x3C9))),
    ('numbers', [0xB9, 0xB2, 0xB3] + list(r(0x2070, 0x209C)) + [0xBC, 0xBD, 0xBE] + list(r(0x2150, 0x218B)) + list(r(0x2460, 0x2473)) + list(r(0x2776, 0x2793))),
    ('marks', [0xA9, 0xAE, 0x2122, 0x2120, 0x2117, 0x2103, 0x2109, 0x2116, 0x2113, 0x212E] +
              list(r(0x2713, 0x2718)) + list(r(0x2605, 0x2606)) + list(r(0x2660, 0x2667)) + list(r(0x2669, 0x266F)) +
              [0x2610, 0x2611, 0x2612, 0x263A, 0x2639, 0x2640, 0x2642, 0x26A2, 0x26A3, 0x26A4, 0x2318, 0x2325, 0x2326, 0x232B, 0x21E7, 0x23CE, 0x2387, 0x238B]),
    ('shapes', list(r(0x25A0, 0x25FF)) + list(r(0x2500, 0x257F)) + list(r(0x2580, 0x259F))),
]

KAOMOJI = [
    ('(＾▽＾)', 'happy'), ('(◕‿◕)', 'happy smile'), ('٩(◕‿◕)۶', 'joy cheer'), ('(ﾉ◕ヮ◕)ﾉ*:･ﾟ✧', 'excited sparkle'),
    ('＼(＾▽＾)／', 'yay cheer'), ('(*^‿^*)', 'blush happy'), ('(≧◡≦)', 'cute happy'), ('(ᵔᴥᵔ)', 'bear cute'),
    ('ʕ•ᴥ•ʔ', 'bear'), ('(=^･ω･^=)', 'cat'), ('(^・ω・^ )', 'cat'), ('U・ᴥ・U', 'dog'),
    ('(╥﹏╥)', 'cry sad'), ('(ಥ﹏ಥ)', 'cry sad'), ('(._.)', 'sad quiet'), ('(｡•́︿•̀｡)', 'sad pout'),
    ('(╯°□°)╯︵ ┻━┻', 'table flip angry'), ('┬─┬ノ( º _ ºノ)', 'table put back calm'), ('(ノಠ益ಠ)ノ彡┻━┻', 'rage table flip'),
    ('ಠ_ಠ', 'disapproval look'), ('(¬_¬)', 'suspicious side eye'), ('(￣ヘ￣)', 'annoyed grumpy'), ('(︶︹︶)', 'grumpy'),
    ('¯\\_(ツ)_/¯', 'shrug whatever'), ('(・_・;)', 'nervous sweat'), ('(⊙_⊙)', 'shocked stare'), ('(°ロ°)', 'shocked surprised'),
    ('(￣▽￣)ノ', 'hi wave hello'), ('(^_^)/', 'wave hi'), ('(｡･∀･)ﾉﾞ', 'hello wave'), ('(￣o￣) zzZZ', 'sleep tired'),
    ('(づ｡◕‿‿◕｡)づ', 'hug'), ('(っ˘̩╭╮˘̩)っ', 'hug sad'), ('(♡μ_μ)', 'love shy'), ('(♥ω♥*)', 'love heart eyes'),
    ('(´｡• ω •｡`) ♡', 'love'), ('( ˘ ³˘)♥', 'kiss love'), ('(˘▽˘>ԅ( ˘⌣˘)', 'kiss'), ('(ง •̀_•́)ง', 'fight determined'),
    ('ᕦ(ò_óˇ)ᕤ', 'strong flex'), ('(☞ﾟヮﾟ)☞', 'point you'), ('☜(ﾟヮﾟ☜)', 'point'), ('(⌐■_■)', 'cool sunglasses'),
    ('( •_•)>⌐■-■', 'deal with it sunglasses'), ('(^_−)☆', 'wink'), ('(￣ω￣)', 'smug'), ('(◔_◔)', 'eye roll'),
    ('(｀・ω・´)', 'serious'), ('( ͡° ͜ʖ ͡°)', 'lenny face'), ('(☉_☉)', 'stare'), ('(＞﹏＜)', 'pain ugh'),
    ('(/ω＼)', 'embarrassed hide'), ('(*/ω＼*)', 'shy hide'), ('(ﾉ´ヮ`)ﾉ*: ･ﾟ', 'magic sparkle'), ('♪(┌・。・)┌', 'dance music'),
    ('ヾ(⌐■_■)ノ♪', 'dance cool'), ('(っ˘ڡ˘ς)', 'yummy food'), ('(￣﹃￣)', 'drool hungry'), ('(✿◠‿◠)', 'flower happy'),
    ('(°▽°)', 'surprised happy'), ('m(_ _)m', 'sorry bow'), ('(シ_ _)シ', 'bow sorry'), ('<(_ _)>', 'bow thanks'),
]


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        sys.exit(2)
    words = annotations(*argv[2:4])
    items = emoji(argv[1], words)
    for cat, cps in SYMBOLS:
        seen = set()
        for it in named(cps, cat):
            if it['e'] not in seen:
                seen.add(it['e'])
                kw = words.get(it['e'], [])
                if kw:
                    it['k'] = [w for w in kw if w != it['n']][:6]
                items.append(it)
    items += [{'e': e, 'n': n, 'g': 'kaomoji'} for e, n in KAOMOJI]
    for it in items:
        if not it.get('k'):
            it.pop('k', None)
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, '..', 'ui', 'emoji-data.js')
    with open(out, 'w', encoding='utf-8') as f:
        f.write('// Made by data/build_data.py from Unicode %s and CLDR: don\'t edit by hand.\n' % (re.search(r'Version: ([\d.]+)', open(argv[1], encoding='utf-8').read(400)) or ['', '?'])[1])
        f.write('window.EMOJI_DATA = ')
        json.dump(items, f, ensure_ascii=False, separators=(',', ':'))
        f.write(';\n')
    print('%d entries -> %s (%d KB)' % (len(items), out, os.path.getsize(out) // 1024))


if __name__ == '__main__':
    main(sys.argv)
