"""One C: (1.11): Steam games' Windows setups get your own folders, Saved Games included, the careful way."""
import importlib.machinery
import importlib.util
import os
import shutil
import tempfile
import unittest
from unittest import mock

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
loader = importlib.machinery.SourceFileLoader('hdl_onec', os.path.join(SHELL, 'hearth_drive_letters.py'))
spec = importlib.util.spec_from_loader('hdl_onec', loader)
hdl = importlib.util.module_from_spec(spec)
loader.exec_module(hdl)


class OneC(unittest.TestCase):
    def setUp(self):
        self.h = tempfile.mkdtemp()
        for d in ('Documents', 'Desktop'):
            os.makedirs(os.path.join(self.h, d))
        self.pfx = os.path.join(self.h, '.local/share/Steam/steamapps/compatdata/570/pfx')
        u = os.path.join(self.pfx, 'drive_c/users/steamuser')
        os.makedirs(os.path.join(u, 'Documents'))                        # empty: becomes yours
        os.makedirs(os.path.join(u, 'Saved Games', 'Game'))               # has a save: left alone
        open(os.path.join(u, 'Saved Games', 'Game', 'slot1.sav'), 'w').write('x')
        self.p = mock.patch.multiple(hdl, home=lambda: self.h, user_dirs=lambda: {}, load=lambda path=None: {})
        self.p.start()

    def tearDown(self):
        self.p.stop()
        shutil.rmtree(self.h)

    def test_steam_prefix_found_and_mapped(self):
        self.assertEqual(hdl.steam_prefixes(self.h), [self.pfx])
        r = hdl.map_steam()
        u = os.path.join(self.pfx, 'drive_c/users/steamuser')
        self.assertEqual(os.readlink(os.path.join(u, 'Documents')), os.path.join(self.h, 'Documents'))
        self.assertFalse(os.path.islink(os.path.join(u, 'Saved Games')))        # a save was there: nothing moved
        self.assertTrue(os.path.isfile(os.path.join(u, 'Saved Games', 'Game', 'slot1.sav')))
        self.assertTrue(os.path.isdir(os.path.join(self.h, 'Saved Games')))
        self.assertEqual(r['prefixes'], 1)

    def test_dry_run_changes_nothing(self):
        hdl.map_steam(dry=True)
        self.assertFalse(os.path.islink(os.path.join(self.pfx, 'drive_c/users/steamuser/Documents')))


if __name__ == '__main__':
    unittest.main()
