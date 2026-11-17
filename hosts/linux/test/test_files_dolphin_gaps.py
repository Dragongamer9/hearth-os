"""Hearth Files' parts that Dolphin had and it didn't (1.14): Compare (two files: same or the changed lines; two
folders: what's only on one side and what differs), PDF previews (the first page, pdftoppm, in the shared thumbnail
cache), the terminal panel (F4, FilesTerminal.qml), split view (F9, FilesSplitPane.qml), and the search asking the
file index first."""
import importlib.machinery
import importlib.util
import os
import pathlib
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'


def load(home):
    os.environ['HOME'] = home
    os.environ['XDG_CACHE_HOME'] = os.path.join(home, '.cache')
    loader = importlib.machinery.SourceFileLoader('hearth_files_gaps', str(SHELL / 'hearth-files'))
    spec = importlib.util.spec_from_loader('hearth_files_gaps', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Compare(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.TemporaryDirectory()
        self.h = pathlib.Path(self.tmp.name)
        self.m = load(self.tmp.name)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        self.tmp.cleanup()

    def test_files(self):
        a, b, c = self.h / 'a.txt', self.h / 'b.txt', self.h / 'c.txt'
        a.write_text('one\ntwo\nthree\n')
        b.write_text('one\ntwo\nthree\n')
        c.write_text('one\nTWO\nthree\nfour\n')
        self.assertTrue(self.m.compare(str(a), str(b))['same'])
        r = self.m.compare(str(a), str(c))
        self.assertFalse(r['same'])
        self.assertEqual(r['changed'], 3)
        diff = pathlib.Path(r['diff']).read_text()
        self.assertIn('-two\n', diff)
        self.assertIn('+TWO\n', diff)
        self.assertTrue(r['diff'].startswith(str(self.h / '.cache' / 'hearth' / 'files')))

    def test_binary_and_mixed(self):
        x, y = self.h / 'x.bin', self.h / 'y.bin'
        x.write_bytes(b'\0\1\2')
        y.write_bytes(b'\0\1\3\4')
        r = self.m.compare(str(x), str(y))
        self.assertEqual((r['same'], r['diff'], r['sizes']), (False, '', [3, 4]))
        (self.h / 'd').mkdir()
        with self.assertRaises(self.m.Fail):
            self.m.compare(str(x), str(self.h / 'd'))
        with self.assertRaises(self.m.Fail):
            self.m.compare(str(x), str(self.h / 'gone'))

    def test_folders(self):
        for side, files in (('L', {'same.txt': 'a', 'changed.txt': 'a', 'only-left.txt': 'x', 'sub/deep.txt': '1'}),
                            ('R', {'same.txt': 'a', 'changed.txt': 'b', 'only-right.txt': 'y', 'sub/deep.txt': '2'})):
            for n, t in files.items():
                p = self.h / side / n
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text(t)
        r = self.m.compare(str(self.h / 'L'), str(self.h / 'R'))
        self.assertFalse(r['same'])
        self.assertEqual((r['onlyA'], r['onlyB'], r['differ']), (['only-left.txt'], ['only-right.txt'], ['changed.txt', 'sub/deep.txt']))


class PdfThumbs(unittest.TestCase):
    def test_pdf_first_page(self):
        old = dict(os.environ)
        with tempfile.TemporaryDirectory() as t:
            try:
                bin_ = pathlib.Path(t) / 'bin'
                bin_.mkdir()
                # a stand-in pdftoppm: writes <prefix>.png (with -singlefile)
                (bin_ / 'pdftoppm').write_text('#!/bin/sh\nfor a; do last=$a; done\nprintf "PNG" > "$last.png"\n')
                (bin_ / 'pdftoppm').chmod(0o755)
                os.environ['PATH'] = str(bin_) + ':/usr/bin:/bin'
                m = load(t)
                pdf = pathlib.Path(t) / 'doc.pdf'
                pdf.write_bytes(b'%PDF-1.4\n')
                r = m.thumbs([str(pdf)])
                self.assertEqual(r['made'], [str(pdf)])
                self.assertTrue(os.path.exists(m.thumb_path(str(pdf))))
                self.assertEqual(m.thumbs([str(pdf)])['have'], [str(pdf)])   # made once, then just there
            finally:
                os.environ.clear()
                os.environ.update(old)


class Wiring(unittest.TestCase):
    def test_panels_and_menu(self):
        fm = (SHELL / 'FileManager.qml').read_text()
        for s in ('FilesTerminal {', 'FilesSplitPane {', 'e.key === Qt.Key_F4 && !app.picking', 'e.key === Qt.Key_F9 && !app.picking',
                  'id: "compare", label: "Compare"', 'case "compare": compareTwo(targets()); break;',
                  'onCopyHere: dest => app.transferTo("copy", app.targets(), dest)'):
            self.assertIn(s, fm)
        self.assertIn('Process', (SHELL / 'FilesTerminal.qml').read_text())
        self.assertIn('"list", path', (SHELL / 'FilesSplitPane.qml').read_text())


if __name__ == '__main__':
    unittest.main()
