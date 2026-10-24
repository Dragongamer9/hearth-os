"""Tests for os/system_files/usr/libexec/nimbus/hearth-flatpak-setup: a never-initialised /var/lib/flatpak/repo
(Peppermint up to 0.6.0: "opening repo: openat(config)") moved aside when empty and given its config back when it
has objects, and when Bazaar gets installed."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import importlib.util
import io
import os
import pathlib
import shutil
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]


def load():
    path = REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-flatpak-setup'
    loader = importlib.machinery.SourceFileLoader('hearth_flatpak_setup', str(path))
    spec = importlib.util.spec_from_loader('hearth_flatpak_setup', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


fs = load()


class Repo(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-flatpak-test-')
        self.repo = os.path.join(self.tmp, 'var/lib/flatpak/repo')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def quiet(self, fn, *a):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            return fn(*a), out.getvalue()

    def test_states(self):
        self.assertEqual(fs.repo_state(self.repo), 'missing')
        os.makedirs(os.path.join(self.repo, 'objects', 'ab'))
        os.makedirs(os.path.join(self.repo, 'tmp'))
        self.assertEqual(fs.repo_state(self.repo), 'empty-no-config')
        open(os.path.join(self.repo, 'objects', 'ab', 'cdef.file'), 'w').close()
        self.assertEqual(fs.repo_state(self.repo), 'objects-no-config')
        open(os.path.join(self.repo, 'config'), 'w').close()
        self.assertEqual(fs.repo_state(self.repo), 'ok')

    def test_empty_repo_moved_aside(self):
        os.makedirs(os.path.join(self.repo, 'objects', 'ab'))     # what tmpfiles.d made at boot
        r, out = self.quiet(fs.fix_repo, self.repo, 1234)
        self.assertEqual(r, 'moved-aside')
        self.assertFalse(os.path.exists(self.repo))
        self.assertTrue(os.path.isdir(self.repo + '.broken-1234'))
        self.assertIn('never initialised', out)

    def test_repo_with_objects_kept_and_config_written(self):
        os.makedirs(os.path.join(self.repo, 'objects', 'ab'))
        open(os.path.join(self.repo, 'objects', 'ab', 'cdef.file'), 'w').close()
        r, out = self.quiet(fs.fix_repo, self.repo)
        self.assertEqual(r, 'config-written')
        self.assertIn('WARNING', out)
        self.assertIn('mode=bare-user-only', pathlib.Path(self.repo, 'config').read_text())
        self.assertTrue(os.path.exists(os.path.join(self.repo, 'objects', 'ab', 'cdef.file')))
        self.assertEqual(self.quiet(fs.fix_repo, self.repo)[0], 'ok')

    def test_main_with_a_fake_root(self):
        os.makedirs(os.path.join(self.repo, 'objects'))
        r, out = self.quiet(fs.main, ['x', '--root', self.tmp])
        self.assertEqual(r, 0)
        self.assertIn('moved-aside', out)


class BazaarPlan(unittest.TestCase):
    def test_plan(self):
        self.assertEqual(fs.bazaar_plan(True, False, False), 'skip-packaged')
        self.assertEqual(fs.bazaar_plan(False, True, True), 'override')
        self.assertEqual(fs.bazaar_plan(False, False, True), 'skip-removed')
        self.assertEqual(fs.bazaar_plan(False, False, False), 'install')


if __name__ == '__main__':
    unittest.main()
