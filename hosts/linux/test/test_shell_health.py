"""The desktop shell's self check (hosts/linux/desktop/shell/hearth-shell-health, ShellHealth.qml, 1.11): a slow leak
or a shell stuck busy asks for a refresh; normal use never does; the refresh waits until nobody is looking."""
import importlib.machinery
import importlib.util
import os
import re
import unittest

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
loader = importlib.machinery.SourceFileLoader('hearth_shell_health', os.path.join(SHELL, 'hearth-shell-health'))
spec = importlib.util.spec_from_loader('hearth_shell_health', loader)
hh = importlib.util.module_from_spec(spec)
loader.exec_module(hh)
MB = 1024 * 1024


def rec(at, rss, cpu, start=100.0, uptime=None):
    return {'at': at, 'start': start, 'uptime': uptime if uptime is not None else at, 'rss': rss * MB, 'cpu': cpu}


class Judge(unittest.TestCase):
    def test_normal_use_never_refreshes(self):
        hist = [rec(t, 440, 3.0) for t in range(600, 90000, 600)]
        self.assertFalse(hh.judge(hist, rec(90600, 480, 4.0))[0])

    def test_the_1_10_leak_refreshes(self):
        hist = [rec(t, 440 + t // 100, 5.0) for t in range(600, 90000, 600)]
        self.assertTrue(hh.judge(hist, rec(90000, 1515, 5.0))[0])     # 1.5 GB after 25 hours

    def test_stuck_busy_refreshes(self):
        hist = [rec(t, 450, 95.0) for t in range(600, 3600, 600)]
        self.assertTrue(hh.judge(hist, rec(3600, 450, 99.0))[0])

    def test_a_new_shell_starts_counting_again(self):
        hist = [rec(t, 1400, 90.0, start=5.0) for t in range(600, 3000, 600)]
        self.assertFalse(hh.judge(hist, rec(3000, 450, 2.0, start=2900.0, uptime=100))[0])


class Qml(unittest.TestCase):
    def test_refreshes_only_while_nobody_is_looking(self):
        q = open(os.path.join(SHELL, 'ShellHealth.qml')).read()
        self.assertIn('DeskState.screenOff || DeskState.locked', q)
        self.assertRegex(q, r'if \(inUse \|\| DeskState.overlayShown\) return;')
        shell = open(os.path.join(SHELL, 'shell.qml')).read()
        self.assertIn('item.inUse = Qt.binding(() => settings.shown || tasks.shown || files.shown)', shell)

    def test_every_call_process_is_destroyed_even_when_the_callback_throws(self):
        for f in ('DeskState.qml', 'FileManager.qml', 'FileProperties.qml'):
            q = open(os.path.join(SHELL, f)).read()
            self.assertNotRegex(q, r'\n\s*if \((proc|procIn)\.cb\) \1\.cb\(res, err\);\n\s*\1\.destroy\(\);', f)
            self.assertRegex(q, r'try \{ if \((proc|procIn)\.cb\) \1\.cb\(res, err\); \} finally \{ \1\.destroy\(\); \}', f)


if __name__ == '__main__':
    unittest.main()
