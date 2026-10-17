"""Tests for desktop/shell/hearth-buttons (power and volume buttons)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import pathlib
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-buttons'
loader = importlib.machinery.SourceFileLoader('hearth_buttons', str(PATH))
spec = importlib.util.spec_from_loader('hearth_buttons', loader)
hb = importlib.util.module_from_spec(spec)
loader.exec_module(hb)


class Clock:
    def __init__(self):
        self.slept = 0.0

    def __call__(self):
        return self.slept


def make():
    out, clock = [], Clock()
    return hb.Buttons(out.append, offset=clock), out, clock


class Power(unittest.TestCase):
    def test_short_press(self):
        b, out, _ = make()
        b.key(hb.KEY_POWER, 1, 10.0)
        b.tick(10.2)
        b.key(hb.KEY_POWER, 0, 10.3)
        self.assertEqual(out, [{'t': 'power', 'press': 'short'}])

    def test_long_press_sent_while_held_once(self):
        b, out, _ = make()
        b.key(hb.KEY_POWER, 1, 10.0)
        for i in range(30):
            b.tick(10.0 + i * 0.1)
        b.key(hb.KEY_POWER, 0, 13.1)
        self.assertEqual(out, [{'t': 'power', 'press': 'long'}])

    def test_press_and_release_together(self):
        b, out, _ = make()
        b.key(hb.KEY_POWER, 1, 5.0)
        b.key(hb.KEY_POWER, 0, 5.0)
        self.assertEqual(out, [{'t': 'power', 'press': 'short'}])

    def test_wake_press_ignored(self):
        b, out, clock = make()
        clock.slept = 600.0             # was asleep for 10 minutes
        b.key(hb.KEY_POWER, 1, 20.0)
        b.key(hb.KEY_POWER, 0, 20.1)
        self.assertEqual(out, [])
        b.key(hb.KEY_POWER, 1, 25.0)    # later presses work
        b.key(hb.KEY_POWER, 0, 25.1)
        self.assertEqual(out, [{'t': 'power', 'press': 'short'}])


class Volume(unittest.TestCase):
    def test_steps_and_repeat(self):
        b, out, _ = make()
        b.key(hb.KEY_VOLUMEUP, 1, 1.0)
        b.key(hb.KEY_VOLUMEUP, 2, 1.05)      # too soon
        b.key(hb.KEY_VOLUMEUP, 2, 1.2)
        b.key(hb.KEY_VOLUMEUP, 0, 1.3)
        b.key(hb.KEY_VOLUMEDOWN, 1, 2.0)
        b.key(hb.KEY_MUTE, 1, 3.0)
        b.key(hb.KEY_MUTE, 0, 3.1)
        self.assertEqual(out, [{'t': 'vol', 'd': 1}, {'t': 'vol', 'd': 1}, {'t': 'vol', 'd': -1}, {'t': 'mute'}])

    def test_controllers_not_watched(self):
        self.assertTrue(hb.wanted({1: [hb.KEY_POWER]}))
        self.assertTrue(hb.wanted({1: [hb.KEY_VOLUMEUP, 30]}))
        self.assertFalse(hb.wanted({1: [hb.KEY_VOLUMEUP, 0x130, 0x131]}))
        self.assertFalse(hb.wanted({1: [30, 31]}))


if __name__ == '__main__':
    unittest.main()
