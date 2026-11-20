"""1.16, the rest fixes (docs/LABWC-FEDORA.md; the owner's red flag: never freeze or lag, nothing heavy at rest).
Mantel used 13-27 % of a core with nothing open on 1.14 (his laptop and ROG Ally). The measured fix is CI's
(.github/scripts/labwc-fedora-rest.sh); these keep the causes from coming back:
  * no repeating Timer that always runs faster than once a minute (a short list of known ones, each with its reason)
  * the bar asks NetworkManager and BlueZ only when something changed (NetBtWatch.qml), never on a timer
  * hearth-pad and hearth-buttons sleep until a press when nothing is held, and don't reopen every input device
  * the moving wallpaper holds still behind any window and after 30 s without input
"""
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
# Always-on fast timers that stay, and why.
ALLOWED = {
    'Recorder.qml': "the recording's state file can't be watched before it exists (a file read, nothing started)",
    'FireholdFramesComp.qml': "only under Firehold's own compositor (never on labwc, KWin or Plasma)",
    'ServerDash.qml': 'only in Server mode (its own session)',
}


def timers(text):
    for m in re.finditer(r'Timer\s*\{([^{}]*)\}', text):
        yield m.group(1)


class Rest(unittest.TestCase):
    def test_no_fast_timer_runs_all_the_time(self):
        bad = []
        for f in sorted(SHELL.glob('*.qml')):
            if f.name in ALLOWED:
                continue
            for b in timers(f.read_text(encoding='utf-8', errors='replace')):
                if not re.search(r'repeat\s*:\s*true', b) or not re.search(r'running\s*:\s*true\b', b):
                    continue
                iv = re.search(r'interval\s*:\s*([0-9 *]+)\s*(;|$|\n)', b)
                try:
                    ms = eval(iv.group(1)) if iv else 0
                except SyntaxError:
                    ms = 0
                if ms < 60000:
                    bad.append('%s (every %s ms)' % (f.name, ms))
        self.assertEqual(bad, [], 'repeating timers that always run: give them a running: condition or an event instead')

    def test_the_bar_asks_only_when_something_changed(self):
        for name in ('Bar.qml', 'Islands.qml', 'FireholdDesk.qml'):
            t = (SHELL / name).read_text()
            self.assertNotIn('"nmcli", "-t"', t, name)
            self.assertNotIn('bluetoothctl --timeout 3 show', t, name)
            self.assertIn('NetBtWatch.', t, name)
        w = (SHELL / 'NetBtWatch.qml').read_text()
        self.assertIn('nmcli monitor', w)
        self.assertIn('gdbus monitor --system --dest org.bluez', w)
        self.assertEqual(w.count('repeat: true'), 1, 'the only repeat is the 2-minute fallback')
        self.assertIn('interval: 120000\n        repeat: true', w)

    def test_helpers_sleep_until_a_press(self):
        pad = (SHELL / 'hearth-pad').read_text()
        self.assertNotIn('[], [], 0.008)', pad, 'hearth-pad must not wake 125 times a second when nothing is held')
        self.assertIn('wait = 0.008 if busy else', pad)
        self.assertIn('_NOT_PADS', pad)
        btn = (SHELL / 'hearth-buttons').read_text()
        self.assertNotIn('[], [], 0.1)', btn)
        self.assertIn('skipped[path] = nid', btn)
        self.assertIn('b.woke_up(time.monotonic())', btn, 'a press that woke the PC must still be seen as one')

    def test_the_moving_wallpaper_holds_still(self):
        w = (SHELL / 'Wallpaper.qml').read_text()
        self.assertRegex(w, r'readonly property bool moving: [^\n]*!anyOpen[^\n]*!DeskState\.idle')
        self.assertRegex((SHELL / 'IdlePower.qml').read_text(), r'timeout: 30\n\s*respectInhibitors: false\n\s*onIsIdleChanged: DeskState\.idle')


if __name__ == '__main__':
    unittest.main()
