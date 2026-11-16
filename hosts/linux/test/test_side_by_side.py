"""Game Mode beside the desktop (1.14, docs/SIDE-BY-SIDE.md): off by default, the switch is one session.conf line,
nothing starts by itself, and with it off nimbus-mode does the usual switch."""
import os
import pathlib
import subprocess
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
HELPER = REPO / 'os/system_files/usr/libexec/nimbus/hearth-side-by-side'
UNIT = REPO / 'os/system_files/usr/lib/systemd/system/hearth-side-game@.service'
IO = REPO / 'hosts/linux/desktop/shell/hearth_settings_io.py'


class SideBySide(unittest.TestCase):
    def env(self, d):
        return {**os.environ, 'HOME': d, 'XDG_CONFIG_HOME': d + '/cfg', 'XDG_DATA_HOME': d + '/data', 'XDG_RUNTIME_DIR': d,
                'HEARTH_SETTINGS_IO': str(IO), 'PATH': '/usr/bin:/bin'}

    def test_off_by_default_and_the_switch_is_one_line(self):
        with tempfile.TemporaryDirectory() as d:
            e = self.env(d)
            conf = pathlib.Path(d, 'cfg/nimbus/session.conf')
            conf.parent.mkdir(parents=True)
            conf.write_text('# mine\nHEARTH_GAME_MODE_CHOICE=steam\n')
            st = subprocess.run(['sh', str(HELPER), 'status'], env=e, capture_output=True, text=True).stdout
            self.assertIn('"on": false', st)
            self.assertEqual(subprocess.run(['sh', str(HELPER), 'game'], env=e).returncode, 1, 'off: game does nothing')
            subprocess.run(['sh', str(HELPER), 'on'], env=e, check=True)
            self.assertEqual(conf.read_text(), '# mine\nHEARTH_GAME_MODE_CHOICE=steam\nHEARTH_SIDE_BY_SIDE=1\n')
            self.assertIn('"on": true', subprocess.run(['sh', str(HELPER), 'status'], env=e, capture_output=True, text=True).stdout)
            subprocess.run(['sh', str(HELPER), 'off'], env=e, check=True)
            self.assertIn('HEARTH_SIDE_BY_SIDE=0', conf.read_text())
            self.assertIn('# mine', conf.read_text())

    def test_the_unit_never_starts_by_itself(self):
        u = UNIT.read_text()
        self.assertNotRegex(u, r'(?m)^\[Install\]')
        self.assertIn('HEARTH_SIDE=1', u)
        self.assertIn('Restart=no', u)

    def test_with_it_off_nimbus_mode_switches_as_before(self):
        with tempfile.TemporaryDirectory() as d:
            e = {**self.env(d), 'LABWC_PID': '999999', 'HEARTH_SIDE_HELPER': str(HELPER)}
            fake = pathlib.Path(d, 'bin'); fake.mkdir()
            (fake / 'labwc').write_text('#!/bin/sh\nexit 0\n'); (fake / 'labwc').chmod(0o755)
            e['PATH'] = '%s:/usr/bin:/bin' % fake
            subprocess.run(['sh', str(REPO / 'hosts/linux/session/nimbus-mode'), 'steam'], env=e, check=True, timeout=20)
            self.assertEqual(pathlib.Path(d, 'nimbus-next').read_text().strip(), 'steam')


if __name__ == '__main__':
    unittest.main()
