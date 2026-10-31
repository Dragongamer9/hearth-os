"""Support Hearth OS (Settings › About and the website): the donation links live in ui/support-links.js and its copy
site/support-links.js, filled in by the owner. They must be the same, https only (or empty: then the page is hidden),
and nothing made up."""
import sys
sys.dont_write_bytecode = True
import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
UI = ROOT / 'ui' / 'support-links.js'
SITE = ROOT / 'site' / 'support-links.js'


def links(path):
    text = path.read_text()
    return re.findall(r"\{\s*id:\s*'([^']+)',\s*name:\s*'([^']+)',\s*note:\s*'([^']*)',\s*url:\s*'([^']*)'\s*\}", text)


class SupportLinks(unittest.TestCase):
    def test_both_copies_are_the_same(self):
        self.assertEqual(UI.read_text(), SITE.read_text(), 'ui/support-links.js and site/support-links.js must be the same')

    def test_links_are_https_or_empty(self):
        found = links(UI)
        self.assertGreaterEqual(len(found), 1)
        for id_, name, note, url in found:
            self.assertTrue(url == '' or re.fullmatch(r'https://[^\s"\'<>\\]{3,2000}', url), f'{id_}: {url!r}')

    def test_pages_hide_it_without_links(self):
        page = (ROOT / 'ui' / 'index.html').read_text()
        self.assertIn('<script src="support-links.js"></script>', page)
        self.assertIn("if(supportLinks().length)", page)
        site = (ROOT / 'site' / 'index.html').read_text()
        self.assertIn('<section id="support" hidden>', site)
        self.assertIn('<script src="support-links.js"></script>', site)


if __name__ == '__main__':
    unittest.main()
