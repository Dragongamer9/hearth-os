"""Tests for hearth-admin write-file (Hearth Text's Edit as administrator), in a throwaway folder: it replaces an
existing regular file at a plain path and keeps its permissions, and refuses links, devices, folders, new files,
hard-linked files and paths with "..". (polkit's side is test/polkit-rules.test.js.)"""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import importlib.util
import io
import os
import pathlib
import shutil
import stat
import tempfile
import unittest

ADMIN = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_admin_write', str(ADMIN))
    spec = importlib.util.spec_from_loader('hearth_admin_write', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class WriteFile(unittest.TestCase):
    def setUp(self):
        self.ha = load()
        self.d = os.path.realpath(tempfile.mkdtemp(prefix='hearth-admin-write-'))
        self.addCleanup(shutil.rmtree, self.d, True)

    def write(self, target, data):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            try:
                self.ha.write_file(target, data)
                code = 0
            except SystemExit as e:
                code = e.code
        return code, out.getvalue()

    def test_replaces_and_keeps_permissions(self):
        f = os.path.join(self.d, 'fstab')
        with open(f, 'w') as h:
            h.write('old\n')
        os.chmod(f, 0o640)
        code, out = self.write(f, b'new text\n')
        self.assertEqual(code, 0, out)
        with open(f, 'rb') as h:
            self.assertEqual(h.read(), b'new text\n')
        self.assertEqual(stat.S_IMODE(os.stat(f).st_mode), 0o640)
        self.assertEqual(sorted(os.listdir(self.d)), ['fstab'])      # nothing left beside it

    def test_refusals(self):
        f = os.path.join(self.d, 'real.conf')
        with open(f, 'w') as h:
            h.write('x')
        link = os.path.join(self.d, 'link.conf')
        os.symlink(f, link)
        hard = os.path.join(self.d, 'hard.conf')
        h2 = os.path.join(self.d, 'h2.conf')
        with open(hard, 'w') as h:
            h.write('x')
        os.link(hard, h2)
        sub = os.path.join(self.d, 'sub')
        os.mkdir(sub)
        for target, why in ((link, 'link'), ('relative/path', 'full path'), (self.d + '/sub/../real.conf', 'plainly'),
                            (self.d + '//real.conf', 'plainly'), (os.path.join(self.d, 'new.conf'), 'already exist'),
                            (sub, 'regular files'), (hard, 'hard links'), ('/proc/self/environ', 'not saved this way'),
                            ('/dev/null', 'not saved this way')):
            code, out = self.write(target, b'y')
            self.assertEqual(code, 1, target)
            self.assertIn(why, out, target)
        with open(f) as h:
            self.assertEqual(h.read(), 'x')
        code, out = self.write(f, b'z' * (self.ha.WRITE_MAX + 1))
        self.assertIn('too big', out)

    def test_needs_pkexec(self):
        out = io.StringIO()
        env = os.environ.pop('PKEXEC_UID', None)
        try:
            with contextlib.redirect_stdout(out), self.assertRaises(SystemExit):
                self.ha.caller()
        finally:
            if env is not None:
                os.environ['PKEXEC_UID'] = env
        self.assertIn('pkexec', out.getvalue())


if __name__ == '__main__':
    unittest.main()
