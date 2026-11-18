"""1.15 "Hearth's own apps made faster": the measuring tools (tools/perf-measure.py, tools/perf-breakdown.py,
apps-speed.yml) and the changes that make the shell smaller and its apps open sooner, kept as they are."""
import importlib.util
import json
import os
import re
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def read(name):
    return open(os.path.join(SHELL, name), encoding='utf-8').read()


class Tools(unittest.TestCase):
    def test_breakdown_finds_the_parts_and_each_variant_stays_balanced(self):
        pb = load('pb', os.path.join(ROOT, 'tools', 'perf-breakdown.py'))
        text = read('shell.qml')
        parts = pb.parts(text)
        names = [p[0] for p in parts]
        for want in ('Wallpaper', 'Desktop', 'Launcher', 'FileManager (files)', 'SettingsApp (settings)', 'Loader Clipboard.qml'):
            self.assertIn(want, names)
        for p in parts:
            v = pb.without(text, p)
            self.assertEqual(sum(pb.braces(l) for l in v.split('\n')), 0, p)
            self.assertNotEqual(v, text)

    def test_braces_ignore_strings_and_comments(self):
        pb = load('pb', os.path.join(ROOT, 'tools', 'perf-breakdown.py'))
        self.assertEqual(pb.braces('Item { text: "}{" // }'), 1)
        self.assertEqual(pb.braces("x: '{'; }"), -1)

    def test_compare_reads_both_files(self):
        pm = load('pm', os.path.join(ROOT, 'tools', 'perf-measure.py'))
        d = tempfile.mkdtemp()
        a, b = os.path.join(d, 'a.json'), os.path.join(d, 'b.json')
        json.dump({'idle': {'quickshell shell': {'cpu': 1.0, 'mb': 280.0, 'n': 1}}, 'IPC': {'ipc': 0.15},
                   'Hearth Files (warm)': {'first': 0.4, 'ready': 0.5, 'mbBefore': 280, 'mbAfter': 300}}, open(a, 'w'))
        json.dump({'idle': {'quickshell shell': {'cpu': 0.5, 'mb': 200.0, 'n': 1}}, 'IPC': {'ipc': 0.15},
                   'Hearth Files (warm)': {'first': 0.2, 'ready': 0.3, 'mbBefore': 200, 'mbAfter': 210}}, open(b, 'w'))
        table, notes = pm.compare(a, b)
        self.assertIn('280.0 MB | 200.0 MB', table)
        self.assertTrue(any('Hearth Files (warm)' in n for n in notes))
        self.assertIn('An IPC call alone', table)


if __name__ == '__main__':
    unittest.main()
