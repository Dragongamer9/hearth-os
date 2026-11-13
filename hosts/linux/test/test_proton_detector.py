"""The Proton detector (hearth_proton_run.watch, 1.11): a game Hearth picked the Proton for is watched; closing right
away with an error notes that Proton as failed, running a while remembers it as working, and the exit code passes on."""
import importlib.machinery
import importlib.util
import os
import sys
import unittest
from unittest import mock

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
sys.path.insert(0, SHELL)
loader = importlib.machinery.SourceFileLoader('hpr_test', os.path.join(SHELL, 'hearth_proton_run.py'))
spec = importlib.util.spec_from_loader('hpr_test', loader)
hpr = importlib.util.module_from_spec(spec)
loader.exec_module(hpr)


class FakeProc:
    def __init__(self, code):
        self.code = code

    def wait(self):
        return self.code

    def send_signal(self, s):
        pass


class Watch(unittest.TestCase):
    def run_(self, code, secs):
        hp = mock.Mock()
        hp.candidates.return_value = [{'name': 'GE-Proton10-25', 'path': '/x'}, {'name': 'Proton 9.0', 'path': '/y'}]
        t = iter([0.0, secs])
        with mock.patch('subprocess.Popen') as pop:
            r = hpr.watch(['proton', 'run', 'g.exe'], {'PROTONPATH': '/usr/share/steam/compatibilitytools.d/GE-Proton10-25'}, 'gog:7', hp,
                          now=lambda: next(t), popen=lambda cmd, env: FakeProc(code))
        return r, hp, pop

    def test_quick_crash_is_noted_and_told(self):
        r, hp, pop = self.run_(1, 4.0)
        self.assertEqual(r, 1)
        hp.note_failed.assert_called_once_with('gog:7', 'GE-Proton10-25')
        self.assertIn('Proton 9.0', ' '.join(pop.call_args[0][0]))

    def test_a_long_run_is_remembered(self):
        r, hp, _ = self.run_(0, 600.0)
        self.assertEqual(r, 0)
        hp.note_worked.assert_called_once_with('gog:7', 'GE-Proton10-25')
        hp.note_failed.assert_not_called()

    def test_a_quick_clean_exit_isnt_a_failure(self):
        r, hp, _ = self.run_(0, 3.0)
        hp.note_failed.assert_not_called()
        hp.note_worked.assert_not_called()


if __name__ == '__main__':
    unittest.main()
