"""labwc's rc.xml files parse, and no If/ForEach sits inside another action's then/else/none.

labwc (0.8.3 in Debian trixie, and later) refuses a nested If or ForEach: it logs "action 'If' cannot be a child
action" and drops the whole branch, so the keybind does nothing. Chains are written as a flat list of If actions."""
import os
import unittest
import xml.etree.ElementTree as ET

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..')
RC_FILES = [
    os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'labwc', 'rc.xml'),
    os.path.join(ROOT, 'os', 'recovery', 'src', 'labwc', 'rc.xml'),
]
CONDITIONAL = {'if', 'foreach'}
BRANCHES = {'then', 'else', 'none'}


def nested_conditionals(root):
    """Every If/ForEach found inside the then/else/none of another action, as (outer, inner) names."""
    found = []
    for action in root.iter('action'):
        for branch in action:
            if branch.tag.lower() not in BRANCHES:
                continue
            for child in branch.iter('action'):
                name = (child.get('name') or '').lower()
                if name in CONDITIONAL:
                    found.append((action.get('name'), child.get('name')))
    return found


def keybind(root, key):
    for kb in root.iter('keybind'):
        if kb.get('key') == key:
            return kb
    return None


class LabwcRcTest(unittest.TestCase):
    def test_files_parse_and_have_no_nested_if(self):
        for path in RC_FILES:
            if not os.path.exists(path):
                continue
            root = ET.parse(path).getroot()
            bad = nested_conditionals(root)
            if bad:
                print('::error file=%s::labwc refuses a nested If/ForEach (%s)' % (os.path.relpath(path, ROOT), bad))
            self.assertEqual(bad, [], path)

    def test_detects_nesting(self):
        root = ET.fromstring('<k><keybind key="x"><action name="If"><query tiled="left"/><else>'
                             '<action name="If"><query tiled="right"/></action></else></action></keybind></k>')
        self.assertEqual(nested_conditionals(root), [('If', 'If')])

    def test_super_up_down_steps(self):
        root = ET.parse(RC_FILES[0]).getroot()
        for key, fallback, steps in (
            ('W-Up', 'Maximize', [('tiled', 'left'), ('tiled', 'right'),
                                  ('tiled_region', 'bottom-left'), ('tiled_region', 'bottom-right')]),
            ('W-Down', 'Iconify', [('tiled', 'left'), ('tiled', 'right'), ('tiled_region', 'top-left'),
                                   ('tiled_region', 'top-right'), ('maximized', 'both')]),
        ):
            kb = keybind(root, key)
            self.assertIsNotNone(kb, key)
            actions = [a for a in kb if a.tag == 'action']
            self.assertTrue(all(a.get('name') == 'If' for a in actions), key)
            # The fallback comes first and fires only when none of the later steps' states match.
            first = actions[0]
            queries = [next(iter(q.attrib.items())) for q in first.findall('query')]
            self.assertEqual(sorted(queries), sorted(steps), key)
            self.assertEqual([a.get('name') for a in first.find('else')], [fallback], key)
            # Then one step per state, in the same order.
            later = [next(iter(a.find('query').attrib.items())) for a in actions[1:]]
            self.assertEqual(later, steps, key)
            for a in actions[1:]:
                self.assertEqual(len(a.find('then')), 1, key)


if __name__ == '__main__':
    unittest.main()
