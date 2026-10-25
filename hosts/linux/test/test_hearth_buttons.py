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


def make(resumed=lambda: None):
    out, clock = [], Clock()
    b = hb.Buttons(out.append, offset=clock, resumed=resumed, log=lambda m: None)
    return b, out, clock


def power(out):
    return [o for o in out if o['t'] == 'power']


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
        logged = []
        out, clock = [], Clock()
        b = hb.Buttons(out.append, offset=clock, resumed=lambda: None, log=logged.append)
        clock.slept = 600.0             # was asleep for 10 minutes
        b.key(hb.KEY_POWER, 1, 20.0)
        b.key(hb.KEY_POWER, 0, 20.1)
        # Said once (Desktop Mode then drops labwc's copy of the same press), and written to the journal.
        self.assertEqual(out, [{'t': 'ignored', 'after': 0.0}])
        self.assertEqual(len(logged), 1)
        self.assertIn('ignored', logged[0])
        b.key(hb.KEY_POWER, 1, 25.0)    # later presses work
        b.key(hb.KEY_POWER, 0, 25.1)
        self.assertEqual(power(out), [{'t': 'power', 'press': 'short'}])

    def test_wake_noticed_on_tick_first(self):
        # The main loop's tick can see the jump before the press is read: still ignored.
        b, out, clock = make()
        clock.slept = 690.0
        b.tick(30.0)
        b.key(hb.KEY_POWER, 1, 30.05)
        b.key(hb.KEY_POWER, 0, 30.2)
        self.assertEqual(power(out), [])
        self.assertEqual(out, [{'t': 'ignored', 'after': 0.1}])

    def test_wake_press_released_after_the_grace_isnt_a_press(self):
        b, out, clock = make()
        clock.slept = 600.0
        b.key(hb.KEY_POWER, 1, 20.0)
        for i in range(40):
            b.tick(20.0 + i * 0.1)      # held past the grace and past HOLD
        b.key(hb.KEY_POWER, 0, 24.5)
        self.assertEqual(power(out), [])

    def test_only_the_release_seen(self):
        # The press itself came before the helper looked (or was dropped): the release is ignored, once.
        b, out, clock = make()
        clock.slept = 600.0
        b.key(hb.KEY_POWER, 0, 20.0)
        self.assertEqual(out, [{'t': 'ignored', 'after': 0.0}])

    def test_sleep_hook_says_just_woke(self):
        # Hearth's sleep hook wrote /run/hearth-resumed 0.6 s ago, and the helper didn't see the clocks jump
        # (it was started after waking, or the jump was small): still ignored.
        b, out, _ = make(resumed=lambda: 0.6)
        b.key(hb.KEY_POWER, 1, 5.0)
        b.key(hb.KEY_POWER, 0, 5.1)
        self.assertEqual(out, [{'t': 'ignored', 'after': 0.6}])

    def test_sleep_hook_long_ago(self):
        b, out, _ = make(resumed=lambda: 120.0)
        b.key(hb.KEY_POWER, 1, 5.0)
        b.key(hb.KEY_POWER, 0, 5.1)
        self.assertEqual(out, [{'t': 'power', 'press': 'short'}])

    def test_volume_works_right_after_waking(self):
        b, out, clock = make()
        clock.slept = 600.0
        b.key(hb.KEY_VOLUMEUP, 1, 20.0)
        self.assertEqual(out, [{'t': 'vol', 'd': 1}])


class SinceResume(unittest.TestCase):
    def files(self, resumed, uptime):
        import os
        import tempfile
        d = tempfile.mkdtemp()
        self.addCleanup(__import__('shutil').rmtree, d)
        r, u = os.path.join(d, 'resumed'), os.path.join(d, 'uptime')
        if resumed is not None:
            with open(r, 'w') as f:
                f.write(resumed)
        with open(u, 'w') as f:
            f.write(uptime)
        return r, u

    def test_reads_the_hook_and_uptime(self):
        self.assertAlmostEqual(hb.since_resume(*self.files('1000.50\n', '1001.75 3000.00\n')), 1.25)

    def test_unknown(self):
        self.assertIsNone(hb.since_resume(*self.files(None, '5.0 1.0\n')))      # never slept this boot
        self.assertIsNone(hb.since_resume(*self.files('junk', '5.0 1.0\n')))
        self.assertIsNone(hb.since_resume(*self.files('9.0', '5.0 1.0\n')))     # from the future


class SleepCommand(unittest.TestCase):
    """`hearth-buttons sleep WHO`: Desktop Mode's way to sleep."""

    def run_it(self, since):
        logged, ran = [], []
        code = hb.sleep('Desktop Mode: power button', since=lambda: since, run=lambda *a: ran.append(a), log=logged.append)
        return code, logged, ran

    def test_sleeps_and_says_who(self):
        code, logged, ran = self.run_it(None)
        self.assertEqual(code, 0)
        self.assertEqual(ran, [('systemctl', ['systemctl', 'suspend'])])
        self.assertEqual(logged, ['Desktop Mode: power button → sleep'])
        code, _, ran = self.run_it(30.0)
        self.assertEqual((code, len(ran)), (0, 1))

    def test_refused_just_after_waking(self):
        # The press that woke the device reached labwc too: it can't put it back to sleep.
        code, logged, ran = self.run_it(0.8)
        self.assertEqual(code, 3)
        self.assertEqual(ran, [])
        self.assertIn('ignored', logged[0])
        self.assertIn('0.8 s', logged[0])


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




class Problems(unittest.TestCase):
    """Only the devices Hearth's udev rule is meant to open count as "no permission" problems."""

    def check(self, props):
        import os
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            node = os.path.join(d, 'event3')
            open(node, 'w').close()
            rdev = os.stat(node).st_rdev
            with open(os.path.join(d, 'c%d:%d' % (os.major(rdev), os.minor(rdev))), 'w') as f:
                f.write('I:123\n' + ''.join('E:%s=%s\n' % kv for kv in props.items()))
            return hb.meant_to_be_readable(node, d)

    def test_keyboards_mice_and_controllers_arent_problems(self):
        self.assertFalse(self.check({'ID_INPUT': '1', 'ID_INPUT_KEY': '1', 'ID_INPUT_KEYBOARD': '1'}))
        self.assertFalse(self.check({'ID_INPUT': '1', 'ID_INPUT_MOUSE': '1'}))
        self.assertFalse(self.check({'ID_INPUT': '1', 'ID_INPUT_KEY': '1', 'ID_INPUT_JOYSTICK': '1'}))

    def test_power_button_is(self):
        self.assertTrue(self.check({'ID_INPUT': '1', 'ID_INPUT_KEY': '1'}))
        self.assertTrue(hb.meant_to_be_readable('/nonexistent/event9', '/nonexistent'))


class OneHandler(unittest.TestCase):
    """Desktop Mode: one handler for the power button, and every sleep through `hearth-buttons sleep`."""
    SHELL = PATH.parent

    def test_labwc_copy_dropped_when_the_helper_has_the_button(self):
        q = (self.SHELL / 'Buttons.qml').read_text()
        self.assertIn('if (helperHasPower)', q)
        self.assertIn('root.helperHasPower = m.power === true', q)
        # An ignored wake press counts as the helper's press: labwc's copy of it is dropped too.
        self.assertRegex(q, r'm\.t === "ignored"\) \{ root\.lastHelperPower = Date\.now\(\)')

    def test_no_direct_suspend_in_desktop_mode(self):
        import re
        for f in sorted(self.SHELL.glob('*.qml')):
            self.assertNotRegex(f.read_text(), r'"systemctl",\s*"suspend"', f.name)
        rc = (self.SHELL.parent / 'labwc' / 'rc.xml').read_text()
        sleep_key = [l for l in rc.splitlines() if 'XF86Sleep' in l][0]
        self.assertIn('hearth-buttons sleep', sleep_key)
        self.assertTrue(re.search(r'XF86PowerOff.*call buttons power', rc))

    def test_sleep_hook(self):
        import os
        import subprocess
        import tempfile
        hook = PATH.parents[4] / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system-sleep' / 'hearth-resumed'
        self.assertTrue(os.access(hook, os.X_OK))
        with tempfile.TemporaryDirectory() as d:
            target = os.path.join(d, 'hearth-resumed')
            text = hook.read_text().replace('/run/hearth-resumed', target)
            subprocess.run(['sh', '-c', text, 'hook', 'pre', 'suspend'], check=True)
            self.assertFalse(os.path.exists(target))
            subprocess.run(['sh', '-c', text, 'hook', 'post', 'suspend'], check=True)
            with open(target) as f:
                at = float(f.read())
            with open('/proc/uptime') as f:
                now = float(f.read().split()[0])
            self.assertTrue(0 <= now - at < 5)
            self.assertIsNotNone(hb.since_resume(target))


if __name__ == '__main__':
    unittest.main()
