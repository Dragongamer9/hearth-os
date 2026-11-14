"""RED FLAG (the owner's rule 11): every approved mockup is built completely, exactly as shown.

Each approved mockup in docs/mockups/ is listed in a checklist (docs/mockups/checklists/*.md) with every part of it:
- [x] a part that's built, with its proof: — path/to/file: "text in that file"
- [ ] a part that isn't built yet
A checklist's "Release:" is the update it ships in. Once that version is released in docs/ROADMAP.md, no part may be
open, so a release with a mockup half built fails here (release.yml runs these tests). Every mockup picture must be
in a checklist, and a ticked roadmap item that says "mockup" (1.12 on) must name its checklist, which must be complete.
"""
import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
MOCKS = ROOT / 'docs/mockups'
LISTS = MOCKS / 'checklists'
ITEM = re.compile(r'^- \[( |x)\] (.+?)(?: — ([^\s:]+): (".*"))?\s*$')
PICTURES = ('.png', '.jpg', '.jpeg', '.webp')


def vkey(v):
    return tuple(int(x) for x in re.findall(r'\d+', v))


def released():
    return {m.group(1) for m in re.finditer(r'^## ([\d.]+) \(released', (ROOT / 'docs/ROADMAP.md').read_text(), re.M)}


def checklists():
    out = {}
    for p in sorted(LISTS.glob('*.md')):
        text = p.read_text()
        rel = re.search(r'^Release: *([\d.]+|next)\s*$', text, re.M)
        mocks = re.search(r'^Mockups: *(.+)$', text, re.M)
        items = []
        for n, line in enumerate(text.splitlines(), 1):
            if line.startswith('- ['):
                m = ITEM.match(line)
                items.append((n, line, m))
        out[p.name] = {'path': p, 'release': rel.group(1) if rel else None,
                       'mockups': [x.strip() for x in mocks.group(1).split(',')] if mocks else [], 'items': items}
    return out


class Mockups(unittest.TestCase):
    def test_every_checklist_is_well_formed(self):
        for name, c in checklists().items():
            self.assertTrue(c['release'], '%s: needs a "Release: X" line (the update it ships in)' % name)
            self.assertTrue(c['mockups'], '%s: needs a "Mockups: a.png, b.png" line' % name)
            self.assertTrue(c['items'], '%s: lists no parts' % name)
            for f in c['mockups']:
                self.assertTrue((MOCKS / f).is_file(), '%s: mockup %s isn\'t in docs/mockups' % (name, f))
            for n, line, m in c['items']:
                self.assertIsNotNone(m, '%s:%d: not "- [x] part — file: \\"proof\\"" or "- [ ] part": %s' % (name, n, line))

    def test_built_parts_have_their_proof_in_the_code(self):
        bad = []
        for name, c in checklists().items():
            for n, line, m in c['items']:
                if not m or m.group(1) != 'x':
                    continue
                if not m.group(3):
                    bad.append('%s:%d ticked without a proof: %s' % (name, n, m.group(2)))
                    continue
                f = ROOT / m.group(3)
                proof = json.loads(m.group(4))
                if not f.is_file() or proof not in f.read_text(errors='replace'):
                    bad.append('%s:%d %s: "%s" isn\'t in %s' % (name, n, m.group(2), proof, m.group(3)))
        self.assertEqual(bad, [], '\n'.join(bad))

    def test_a_released_mockup_is_complete(self):
        rel = released()
        bad = []
        for name, c in checklists().items():
            if c['release'] in rel:
                bad += ['%s (released in %s) still misses: %s' % (name, c['release'], m.group(2) if m else line)
                        for n, line, m in c['items'] if not m or m.group(1) != 'x']
        self.assertEqual(bad, [], 'RED FLAG: a mockup must be built completely before its release\n' + '\n'.join(bad))

    def test_every_mockup_picture_has_a_checklist(self):
        listed = {f for c in checklists().values() for f in c['mockups']}
        pics = sorted(str(p.relative_to(MOCKS)) for p in MOCKS.rglob('*') if p.suffix.lower() in PICTURES)
        missing = [p for p in pics if p not in listed]
        self.assertEqual(missing, [], 'mockups without a checklist (docs/mockups/checklists): %s' % missing)

    def test_ticked_roadmap_mockup_items_name_a_complete_checklist(self):
        lists = checklists()
        bad, section = [], None
        for line in (ROOT / 'docs/ROADMAP.md').read_text().splitlines():
            h = re.match(r'^## (?:Planned for )?([\d.]+)', line)
            if h:
                section = h.group(1)
            elif line.startswith('## '):
                section = None
            if not section or vkey(section) < (1, 12) or not line.startswith('- [x]') or 'mockup' not in line.lower():
                continue
            names = re.findall(r'docs/mockups/checklists/([\w.-]+\.md)', line)
            if not names:
                bad.append('%s: a ticked mockup item must name its checklist (docs/mockups/checklists/…): %s' % (section, line[:120]))
            for nm in names:
                c = lists.get(nm)
                if not c:
                    bad.append('%s: no checklist %s' % (section, nm))
                elif any(not m or m.group(1) != 'x' for _, _, m in c['items']):
                    bad.append('%s: ticked, but %s still has open parts' % (section, nm))
        self.assertEqual(bad, [], '\n'.join(bad))


if __name__ == '__main__':
    unittest.main()
