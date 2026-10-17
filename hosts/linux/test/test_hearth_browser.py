"""Tests for desktop/shell/hearth-browser (Aloha Browser install), with a fake package."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-browser'


def load(home):
    os.environ['HOME'] = home
    os.environ.pop('XDG_DATA_HOME', None)
    os.environ['XDG_CONFIG_HOME'] = os.path.join(home, '.config')
    loader = importlib.machinery.SourceFileLoader('hearth_browser', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_browser', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Install(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.home = tempfile.mkdtemp()
        os.environ['PATH'] = '/nonexistent'          # no xdg-utils here: the mimeapps.list fallback is used
        self.hb = load(self.home)

        def fake_download(dest):
            open(dest, 'w').close()

        def fake_unpack(rpm, dest):
            app = os.path.join(dest, 'opt', 'aloha')
            os.makedirs(app)
            os.makedirs(os.path.join(dest, 'usr', 'share', 'applications'))
            exe = os.path.join(app, 'aloha')
            with open(exe, 'w') as f:
                f.write('#!/bin/sh\necho "Aloha 4.9.1.0"\n')
            os.chmod(exe, 0o755)
            open(os.path.join(app, 'chrome-sandbox'), 'w').close()
            with open(os.path.join(dest, 'usr', 'share', 'applications', 'aloha-browser.desktop'), 'w') as f:
                f.write('[Desktop Entry]\nName=Aloha\nExec=/opt/aloha/aloha %U\nIcon=aloha\nType=Application\n')

        self.hb.download = fake_download
        self.hb.unpack = fake_unpack

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)

    def test_install_status_remove(self):
        os.environ['PATH'] = '/usr/bin:/bin'
        self.hb.install(stamp='v1')
        os.environ['PATH'] = '/nonexistent'
        st = self.hb.status()
        self.assertTrue(st['installed'])
        self.assertEqual(st['version'], '4.9.1.0')
        desktop = open(self.hb.DESKTOP).read()
        self.assertIn('Exec=' + self.hb.LAUNCHER + ' %U', desktop)
        self.assertFalse(os.path.exists(os.path.join(self.hb.ROOT, 'opt', 'aloha', 'chrome-sandbox')))
        mime = open(os.path.join(self.home, '.config', 'mimeapps.list')).read()
        self.assertIn('x-scheme-handler/https=hearth-aloha.desktop', mime)
        self.hb.remove()
        st = self.hb.status()
        self.assertFalse(st['installed'])
        self.assertTrue(st['removed'])


if __name__ == '__main__':
    unittest.main()
