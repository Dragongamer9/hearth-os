"""Hearth OS version numbers (tools/hearth_version.py, docs/UPDATES.md "Version numbers"): 1.0 is newer than 0.6.63,
1.10 newer than 1.9, "1.0" and "1.0.0" are the same, 1.0.0 shows as "1.0", and ROADMAP headings carry the date."""
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..')
spec = importlib.util.spec_from_file_location('hearth_version', os.path.join(ROOT, 'tools', 'hearth_version.py'))
hv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hv)


class Compare(unittest.TestCase):
    def test_newer(self):
        self.assertTrue(hv.newer('1.0.0', '0.6.63'))
        self.assertTrue(hv.newer('1.0', '0.6.63'))
        self.assertTrue(hv.newer('v1.0.0', 'v0.6.63'))
        self.assertTrue(hv.newer('0.6.0', '0.5.9'))
        self.assertTrue(hv.newer('0.6.63', '0.6.9'))
        self.assertTrue(hv.newer('1.10.0', '1.9.0'))
        self.assertTrue(hv.newer('1.10', '1.9'))
        self.assertTrue(hv.newer('2.0', '1.10.0'))
        self.assertFalse(hv.newer('1.0', '1.0.0'))
        self.assertFalse(hv.newer('1.0.0', '1.0'))
        self.assertFalse(hv.newer('0.6.63', '1.0'))

    def test_show(self):
        for v, want in [('1.0.0', '1.0'), ('1.1.0', '1.1'), ('1.10.0', '1.10'), ('2.0.0', '2.0'), ('v1.0.0', 'v1.0'),
                        ('1.0', '1.0'), ('1.0.0+app.3', '1.0+app.3'), ('1.0.1', '1.0.1'), ('0.6.63', '0.6.63'),
                        ('0.6.0', '0.6.0'), ('', ''), ('Preview 3', 'Preview 3')]:
            self.assertEqual(hv.show(v), want, v)


ROADMAP = """# Roadmap
## Next (unreleased)
- [x] something new
## 1.10 (released 2027-03-01)
- [x] ten
## 1.9 (released 2027-02-01)
- [x] nine
## 1.0 (released 2026-11-02)
- [x] one point oh
## 0.6.63 (released)
- [x] the last 0.x
## 0.5.1 (released)
- [x] older
"""


class Dates(unittest.TestCase):
    def test_date_for(self):
        self.assertEqual(hv.date_for('1.0.0', ROADMAP), '2026-11-02')
        self.assertEqual(hv.date_for('1.0', ROADMAP), '2026-11-02')
        self.assertEqual(hv.date_for('v1.10.0', ROADMAP), '2027-03-01')
        self.assertIsNone(hv.date_for('0.6.63', ROADMAP))
        self.assertIsNone(hv.date_for('1.1.0', ROADMAP))

    def test_cli(self):
        with tempfile.NamedTemporaryFile('w', suffix='.md', delete=False) as f:
            f.write(ROADMAP)
        try:
            run = lambda *a: subprocess.run([sys.executable, os.path.join(ROOT, 'tools', 'hearth_version.py'), *a], capture_output=True, text=True)
            self.assertEqual(run('date', '1.0.0', f.name).stdout.strip(), '2026-11-02')
            self.assertEqual(run('date', '0.6.63', f.name).stdout.strip(), '')
            self.assertEqual(run('show', '1.0.0').stdout.strip(), '1.0')
            self.assertEqual(run('newer', '1.0.0', '0.6.63').returncode, 0)
            self.assertEqual(run('newer', '0.6.63', '1.0.0').returncode, 1)
        finally:
            os.unlink(f.name)

    def test_release_notes_page(self):
        """tools/build-docs.py takes both heading forms, sorts 1.10 above 1.9 above 1.0 above 0.6.63, and shows dates."""
        spec2 = importlib.util.spec_from_file_location('build_docs', os.path.join(ROOT, 'tools', 'build-docs.py'))
        bd = importlib.util.module_from_spec(spec2)
        argv = sys.argv
        sys.argv = ['build-docs.py', '--help-json', os.devnull]   # import without building into site/docs
        try:
            spec2.loader.exec_module(bd)
        except SystemExit:
            pass
        finally:
            sys.argv = argv
        bd.ROADMAP = ROADMAP
        page = bd.releases_body()
        self.assertIn('Hearth OS 1.10 · <time datetime="2027-03-01">March 1, 2027</time> <span class="ver">Latest</span>', page)
        order = [page.index(s) for s in ('Hearth OS 1.10', 'Hearth OS 1.9 ', 'Hearth OS 1.0 ', 'Hearth OS 0.6.63', 'Hearth OS 0.5.1')]
        self.assertEqual(order, sorted(order))
        self.assertIn('Hearth OS 1.0 · <time datetime="2026-11-02">November 2, 2026</time>', page)
        self.assertIn('<summary>Hearth OS 0.6.63 <span class="cnt">', page)   # no date: just the number


if __name__ == '__main__':
    unittest.main()
