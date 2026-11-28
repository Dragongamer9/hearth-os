"""Windows & desktops on labwc (hearth-system windows / windows-set, Settings › Personalization › Windows & desktops;
the owner, Nov 28, 2026: every Settings button has to work, and KWin's pages can't on labwc)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest
import xml.dom.minidom

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
RC = (ROOT / 'hosts/linux/desktop/labwc/rc.xml').read_text()


def load(tmp):
    os.environ['XDG_CONFIG_HOME'] = tmp
    os.environ['XDG_RUNTIME_DIR'] = os.path.join(tmp, 'run')
    os.environ.pop('LABWC_PID', None)
    loader = importlib.machinery.SourceFileLoader('hsys_%d' % id(tmp), str(SHELL / 'hearth-system'))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Windows(unittest.TestCase):
    def test_nothing_changed_keeps_hearths_rc(self):
        with tempfile.TemporaryDirectory() as t:
            self.assertEqual(load(t).rc_with_windows(RC, {}), RC)

    def test_each_setting_lands_in_rc_and_stays_valid_xml(self):
        with tempfile.TemporaryDirectory() as t:
            out = load(t).rc_with_windows(RC, {'desktops': 2, 'followMouse': True, 'raiseOnFocus': True, 'snapEdges': False,
                                                'topMaximize': False, 'adaptiveSync': 'fullscreen', 'allowTearing': True})
            xml.dom.minidom.parseString(out)
            self.assertIn('<desktops number="2">', out)
            self.assertEqual(out.count('<name>Desktop'), 2)
            self.assertIn('<followMouse>yes</followMouse><raiseOnFocus>yes</raiseOnFocus>', out)
            self.assertIn('<range inner="0" outer="0" />', out)
            self.assertIn('<topMaximize>no</topMaximize>', out)
            self.assertRegex(out, r'(?s)<core>.*<adaptiveSync>fullscreen</adaptiveSync>.*</core>')
            self.assertIn('<allowTearing>yes</allowTearing>', out)
            # twice is the same as once
            m = load(t)
            w = {'desktops': 3, 'adaptiveSync': 'yes'}
            self.assertEqual(m.rc_with_windows(m.rc_with_windows(RC, w), w), m.rc_with_windows(RC, w))

    def test_set_merges_and_odd_values_are_dropped(self):
        with tempfile.TemporaryDirectory() as t:
            m = load(t)
            m.windows_set(json.dumps({'desktops': 3}))
            m.windows_set(json.dumps({'followMouse': True, 'desktops': 'lots', 'adaptiveSync': 'sometimes'}))
            saved = json.loads(pathlib.Path(t, 'nimbus/windows.json').read_text())
            self.assertEqual(saved, {'desktops': 3, 'followMouse': True})
            self.assertEqual(m.windows_status()['desktops'], 3)


if __name__ == '__main__':
    unittest.main()
