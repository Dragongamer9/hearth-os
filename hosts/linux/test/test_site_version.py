"""RED FLAG (CLAUDE.md rule 10): every release updates the version on the website. The hearthos.fun "Out now" line in
site/index.html must name the newest released version in docs/ROADMAP.md, so the site never lags a release."""
import os
import re
import unittest

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..')


def newest_released():
    text = open(os.path.join(ROOT, 'docs', 'ROADMAP.md'), encoding='utf-8').read()
    m = re.search(r'^## (\d+(?:\.\d+)+) \(released', text, re.M)
    return m.group(1) if m else None


def site_out_now():
    html = open(os.path.join(ROOT, 'site', 'index.html'), encoding='utf-8').read()
    m = re.search(r'<span class="tag now">Out now</span>\s*Hearth OS ([0-9][0-9.]*)</span>', html)
    return m.group(1) if m else None


def same(a, b):
    trim = lambda v: re.sub(r'(\.0)+$', '', v)
    return trim(a) == trim(b)


class SiteVersion(unittest.TestCase):
    def test_out_now_matches_the_newest_release(self):
        released, shown = newest_released(), site_out_now()
        self.assertIsNotNone(released, 'no "## X (released ...)" heading in docs/ROADMAP.md')
        self.assertIsNotNone(shown, 'no "Out now  Hearth OS X" line in site/index.html')
        self.assertTrue(same(released, shown), f'the website says Hearth OS {shown} but the newest release is {released}: update site/index.html in the release commit')

    def test_same_ignores_trailing_zeros(self):
        self.assertTrue(same('1.1', '1.1.0'))
        self.assertFalse(same('1.1', '1.10'))
        self.assertFalse(same('1.0', '1.1'))


if __name__ == '__main__':
    unittest.main()
