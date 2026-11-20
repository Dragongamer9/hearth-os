"""Hearth Settings › Controller's pointer speed reaches the controller mouse on KDE Plasma (hearth-pad --plasma under
hearth-pad-host, and --standalone): no shell sends speed {value} there, so hearth-pad reads desktop.json's pointerSpeed
at the start and again when the file changes (one look every 2 s), and never writes it."""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'
loader = importlib.machinery.SourceFileLoader('hearth_pad_speed', str(SHELL / 'hearth-pad'))
spec = importlib.util.spec_from_loader('hearth_pad_speed', loader)
hp = importlib.util.module_from_spec(spec)
loader.exec_module(hp)


class PointerSpeed(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.tmp.name, 'desktop.json')

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, v, mtime):
        with open(self.path, 'w') as f:
            json.dump(v, f)
        os.utime(self.path, (mtime, mtime))

    def test_read_and_kept_in_range(self):
        self.write({'look': 'steam', 'pointerSpeed': 1.6}, 1000)
        self.assertEqual(hp.saved_pointer_speed(self.path), 1.6)
        self.write({'pointerSpeed': 9}, 1001)
        self.assertEqual(hp.saved_pointer_speed(self.path), 3.0)
        for bad in ({'pointerSpeed': 'fast'}, {'pointerSpeed': True}, {}, [1, 2]):
            self.write(bad, 1002)
            self.assertIsNone(hp.saved_pointer_speed(self.path), bad)
        with open(self.path, 'w') as f:
            f.write('{broken')
        self.assertIsNone(hp.saved_pointer_speed(self.path))
        self.assertIsNone(hp.saved_pointer_speed(os.path.join(self.tmp.name, 'none.json')))

    def test_watch_reads_at_start_and_after_a_change_only(self):
        self.write({'pointerSpeed': 0.8}, 1000)
        w = hp.SpeedWatch(self.path, every=2.0)
        self.assertEqual(w.poll(10.0), 0.8)        # at the start
        self.assertIsNone(w.poll(10.5))            # not looked at again yet
        self.assertIsNone(w.poll(12.1))            # looked at: unchanged
        self.write({'pointerSpeed': 1.4}, 1005)
        self.assertIsNone(w.poll(13.0))            # within the 2 s
        self.assertEqual(w.poll(14.2), 1.4)        # the change
        with open(self.path) as f:
            self.assertEqual(json.load(f), {'pointerSpeed': 1.4})   # never written

    def test_plasma_and_standalone_watch_it(self):
        src = (SHELL / 'hearth-pad').read_text()
        self.assertIn("speed_watch = SpeedWatch() if (plasma or standalone) and not cosmic else None", src)
        self.assertIn('pad.speed = sp', src)


if __name__ == '__main__':
    unittest.main()
