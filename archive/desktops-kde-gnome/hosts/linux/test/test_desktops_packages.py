"""KDE Plasma / GNOME downloads (1.14, the owner's laptop, Nov 16: "Unable to locate package spectacle" stopped the
whole of KDE): Debian's name for KDE's screenshot tool, and an extra app that isn't in the package lists is left out
instead of stopping the desktop; the desktop itself must all be there."""
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest
from unittest import mock

HELPER = pathlib.Path(__file__).resolve().parents[3] / 'os/system_files/usr/libexec/nimbus/hearth-desktops-helper'


def load(cache):
    with mock.patch.dict(os.environ, {'HEARTH_DESKTOPS_APT_CACHE': cache}):
        loader = importlib.machinery.SourceFileLoader('hdh', str(HELPER))
        spec = importlib.util.spec_from_loader('hdh', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
    return m


class Packages(unittest.TestCase):
    def test_debian_names(self):
        h = load('')
        self.assertIn('kde-spectacle', h.DESKTOPS['kde']['packages'])
        self.assertNotIn('spectacle', h.DESKTOPS['kde']['packages'])

    def test_missing_extra_is_left_out_missing_core_stops(self):
        with tempfile.TemporaryDirectory() as d:
            fake = os.path.join(d, 'apt-cache')
            pathlib.Path(fake).write_text('#!/bin/sh\ncase "$2" in kate|nope) echo "  Candidate: (none)";; *) echo "  Candidate: 1.0";; esac\n')
            os.chmod(fake, 0o755)
            h = load(fake)
            pk = h.wanted_packages(h.DESKTOPS['kde'])
            self.assertNotIn('kate', pk)
            self.assertIn('kde-plasma-desktop', pk)
            self.assertIn('gdm3-', h.wanted_packages(h.DESKTOPS['gnome']))
            with self.assertRaises(h.Fail):
                h.wanted_packages({'label': 'Test', 'core': 1, 'packages': ['nope', 'dolphin']})


if __name__ == '__main__':
    unittest.main()
