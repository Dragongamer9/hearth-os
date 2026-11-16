"""hearth-gamemode-display (1.14, Settings › Gaming › Performance and Display's HDR row): Game Mode's options in
session.conf's NIMBUS_GAMESCOPE_ARGS line only, every other option and line kept (red flag 9)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest
from unittest import mock

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
loader = importlib.machinery.SourceFileLoader('gmd', str(SHELL / 'hearth-gamemode-display'))
spec = importlib.util.spec_from_loader('gmd', loader)
gmd = importlib.util.module_from_spec(spec)
loader.exec_module(gmd)


def edid(hdr):
    base = bytearray(128)
    base[0:8] = b'\x00\xff\xff\xff\xff\xff\xff\x00'
    base[54 + 3] = 0xFC
    base[54 + 5:54 + 5 + 9] = b'LG 27GP85'
    base[54:57] = b'\0\0\0'
    cta = bytearray(128)
    cta[0], cta[1], cta[2] = 0x02, 3, 8
    if hdr:
        cta[4:8] = bytes([0xE3, 0x06, 0x05, 0x01])   # extended tag 6: HDR static metadata
    return bytes(base + cta)


class Options(unittest.TestCase):
    def test_parse_and_build_keep_other_options(self):
        got, rest = gmd.parse(['--adaptive-sync', '-W', '2560', '-H', '1440', '-r', '165', '--framerate-limit=60', '--hdr-enabled', '--mangoapp'])
        self.assertEqual(got, {'w': 2560, 'h': 1440, 'r': 165, 'limit': 60, 'hdr': True})
        self.assertEqual(rest, ['--adaptive-sync', '--mangoapp'])
        has = {'limit': True, 'hdr': True}
        self.assertEqual(gmd.build(got, rest, has), '--adaptive-sync --mangoapp -W 2560 -H 1440 -r 165 --framerate-limit 60 --hdr-enabled')
        # A gamescope without them: not written.
        self.assertEqual(gmd.build(got, rest, {'limit': False, 'hdr': False}), '--adaptive-sync --mangoapp -W 2560 -H 1440 -r 165')

    def test_hdr_from_the_screens_edid(self):
        self.assertTrue(gmd.edid_hdr(edid(True)))
        self.assertFalse(gmd.edid_hdr(edid(False)))
        self.assertEqual(gmd.edid_name(edid(True)), 'LG 27GP85')

    def test_set_changes_only_its_own_line_and_options(self):
        with tempfile.TemporaryDirectory() as d:
            conf = os.path.join(d, 'nimbus', 'session.conf')
            os.makedirs(os.path.dirname(conf))
            before = '# mine\nHEARTH_GAME_MODE_CHOICE=steam\nNIMBUS_GAMESCOPE_ARGS="--adaptive-sync -W 1920 -H 1080"\nNIMBUS_START_MODE=desktop\n'
            pathlib.Path(conf).write_text(before)
            with mock.patch.object(gmd, 'CONF', conf), mock.patch.object(gmd, 'gamescope_has', return_value={'limit': True, 'hdr': True}), \
                    mock.patch.object(gmd.io, 'log_event'):
                gmd.set_({'limit': 60, 'hdr': True})
                after = pathlib.Path(conf).read_text()
                self.assertIn('HEARTH_GAME_MODE_CHOICE=steam\n', after)
                self.assertIn('NIMBUS_START_MODE=desktop\n', after)
                self.assertIn('# mine\n', after)
                self.assertIn('NIMBUS_GAMESCOPE_ARGS="--adaptive-sync -W 1920 -H 1080 --framerate-limit 60 --hdr-enabled"', after)
                self.assertEqual(pathlib.Path(conf + '.bak').read_text(), before)
                gmd.set_({'w': 0, 'h': 0, 'limit': 0, 'hdr': False})
                self.assertIn('NIMBUS_GAMESCOPE_ARGS="--adaptive-sync"', pathlib.Path(conf).read_text())
            with mock.patch.object(gmd, 'CONF', conf), mock.patch.object(gmd, 'gamescope_has', return_value={'limit': False, 'hdr': False}):
                with self.assertRaises(SystemExit):
                    gmd.set_({'hdr': True})

    def test_the_session_takes_a_frame_limit_out_with_the_mode_when_gamescope_cant_start(self):
        src = (SHELL.parents[1] / 'session' / 'nimbus-session').read_text()
        regex = re.search(r"^MODE_ARGS_RE='([^']*)'$", src, re.M).group(1)
        out = subprocess.run(['sed', '-E', 's/%s/\\1/g' % regex], input=' --hdr-enabled -W 2560 -H 1440 -r 120 --framerate-limit 60\n',
                             capture_output=True, text=True).stdout
        self.assertEqual(out.split(), ['--hdr-enabled'])


if __name__ == '__main__':
    unittest.main()
