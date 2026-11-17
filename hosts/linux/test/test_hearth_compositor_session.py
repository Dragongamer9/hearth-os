"""2.0 preview: Mantel on Hearth's own compositor (docs/COMPOSITOR.md). Never by default: only with
HEARTH_DESKTOP_WM=hearth (Settings › System › Your desktop › Hearth compositor (preview)), and then with the same safe
start as KWin one step above it: missing, ending right away or Mantel never up → KWin for that sign-in (then labwc),
with a notice and a count; twice in a row → KWin until it's turned on again. Turning it off goes back to what was
picked before. The session runs for real with stand-ins (test_kwin_session.py's)."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import subprocess
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_kwin_session import Base, KWIN_WORKS  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[3]

COMP_WORKS = '''#!/bin/sh
{ echo "args=$*"; echo "pid=$$ desktop_pid=$HEARTH_DESKTOP_PID wm=$HEARTH_WM labwc_pid=${LABWC_PID-unset}";
  echo "xcd=$XDG_CURRENT_DESKTOP"; echo "type=$XDG_SESSION_TYPE"; } >> "$HOME/comp-ran"
: > "$XDG_RUNTIME_DIR/hearth-shell-up"
sleep 3
exit 0
'''
COMP_CRASHES = '#!/bin/sh\necho "args=$*" >> "$HOME/comp-ran"\nexit 101\n'
COMP_NO_SHELL = '#!/bin/sh\necho "args=$*" >> "$HOME/comp-ran"\nexec sleep 60\n'
PREVIEW = 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=hearth\n'


class HearthCompositor(Base):
    def setUp(self):
        super().setUp()
        self.write_bin('kwin_wayland', KWIN_WORKS)
        os.makedirs(os.path.join(self.share, 'desktop'), exist_ok=True)
        import shutil
        shutil.copytree(ROOT / 'hosts/linux/desktop/hearth-compositor', os.path.join(self.share, 'desktop', 'hearth-compositor'))

    def env(self, **kw):
        e = super().env(**kw)
        e.setdefault('HEARTH_COMP_UP_SECS', '3')
        e.setdefault('HEARTH_COMP_EARLY_SECS', '2')
        return e

    def test_never_by_default(self):
        self.write_bin('hearth-compositor', COMP_WORKS)
        r, log = self.session()
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertEqual(self.lines('comp-ran'), [], log)
        self.assertNotIn("Hearth's compositor", log)
        st = self.helper('status')
        self.assertEqual((st['hearthPreview'], st['hearthHere']), (False, True))

    def test_preview_runs_it_with_mantels_environment(self):
        self.write_bin('hearth-compositor', COMP_WORKS)
        r, log = self.session(PREVIEW)
        self.assertIn("desktop mode: Mantel on Hearth's compositor (preview)", log)
        self.assertIn('hearth-compositor: Mantel up after', log)
        self.assertEqual(self.lines('kwin-ran'), [], log)
        self.assertEqual(self.lines('labwc-ran'), [], log)
        ran = '\n'.join(self.lines('comp-ran'))
        # On the screens, labwc's rc.xml for Mantel's shortcuts, then the autostart after --.
        self.assertIn('args=--tty-udev --labwc-rc %s/desktop/labwc/rc.xml -- sh %s/desktop/hearth-compositor/autostart'
                      % (self.share, self.share), ran)
        pid = [l for l in self.lines('comp-ran') if l.startswith('pid=')][0].split()
        self.assertEqual(pid[0][4:], pid[1][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is the compositor itself')
        self.assertIn('wm=hearth', ran)
        self.assertIn('labwc_pid=unset', ran)
        self.assertIn('xcd=Nimbus:wlroots', ran)        # labwc's environment file: Mantel and the portals take labwc's ways
        self.assertIn('type=wayland', ran)
        self.assertTrue(os.path.exists(os.path.join(self.home, '.local/share/nimbus/hearth-compositor.log')))
        self.assertEqual(self.notice(), [])
        self.assertEqual(self.fails(), '')
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)

    def test_missing_opens_kwin_and_says_why_once(self):
        r, log = self.session(PREVIEW, HEARTH_COMPOSITOR_BIN='hearth-no-compositor-here')
        self.assertIn("Hearth's compositor (hearth-compositor) isn't installed; KWin instead", log)
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertEqual(self.notice(), ['hearth', 'missing'])
        n = self.helper('notice')
        self.assertIn("isn't on this device yet", n['title'])
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)   # the choice stays

    def test_dying_fast_opens_kwin_counts_and_twice_stays(self):
        self.write_bin('hearth-compositor', COMP_CRASHES)
        r, log = self.session(PREVIEW)
        self.assertIn('hearth-compositor ended before Mantel came up (code 101)', log)
        self.assertIn("desktop: Hearth's compositor didn't start; KWin for this sign-in", log)
        self.assertIn('kwin: Mantel up after', log)                      # KWin, the next rung
        self.assertEqual(self.notice(), ['hearth', 'failed'])
        self.assertEqual(self.fails(), 'hearth 1\n')
        # The same sign-in: KWin straight away.
        r, log = self.session(PREVIEW)
        self.assertIn("didn't start earlier this sign-in, KWin instead", log)
        self.assertEqual(len(self.lines('comp-ran')), 1)
        # A new sign-in: tried again, fails again → twice → paused.
        os.remove(os.path.join(self.run_dir, 'hearth-comp-failed'))
        r, log = self.session(PREVIEW)
        self.assertIn("didn't start twice in a row", log)
        self.assertEqual(self.notice(), ['hearth', 'twice'])
        self.assertTrue(self.helper('status')['hearthPaused'])
        os.remove(os.path.join(self.run_dir, 'hearth-comp-failed'))
        r, log = self.session(PREVIEW)
        self.assertEqual(len(self.lines('comp-ran')), 2, log)
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)   # never rewritten by the session
        # Turned on again in Settings: the count starts over.
        st = self.helper('wm', 'hearth')
        self.assertEqual((st['hearthFails'], st['hearthPaused']), (0, False))

    def test_kwin_failing_too_still_opens_labwc(self):
        self.write_bin('hearth-compositor', COMP_CRASHES)
        self.write_bin('kwin_wayland', '#!/bin/sh\necho "args=$*" >> "$HOME/kwin-ran"\nexit 1\n')
        r, log = self.session(PREVIEW)
        self.assertEqual(len(self.lines('comp-ran')), 1, log)
        self.assertEqual(len(self.lines('kwin-ran')), 1, log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)           # the last rung: Desktop Mode still opens

    def test_mantel_never_up_ends_it_and_opens_kwin(self):
        self.write_bin('hearth-compositor', COMP_NO_SHELL)
        t = time.time()
        r, log = self.session(PREVIEW)
        self.assertLess(time.time() - t, 40)
        self.assertIn("hearth-compositor: Mantel didn't say it's up after 3s; ending it", log)
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(subprocess.run(['pgrep', '-f', os.path.join(self.bin, 'hearth-compositor')]).returncode, 1)

    def test_safe_graphics_and_labwc_pick_never_use_it(self):
        self.write_bin('hearth-compositor', COMP_WORKS)
        open(os.path.join(self.tmp, 'safe'), 'w').close()
        r, log = self.session(PREVIEW)
        self.assertIn('safe graphics this time, labwc', log)
        self.assertEqual(self.lines('comp-ran'), [])

    def test_settings_switch_keeps_and_restores_the_choice_before(self):
        self.write_bin('hearth-compositor', COMP_WORKS)
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=labwc\n')
        st = self.helper('wm', 'hearth')
        self.assertEqual((st['wm'], st['hearthPreview'], st['wmBefore']), ('hearth', True, 'labwc'))
        text = pathlib.Path(self.conf).read_text()
        self.assertIn('HEARTH_DESKTOP_WM=hearth', text)
        self.assertIn('HEARTH_DESKTOP_WM_BEFORE=labwc', text)
        self.assertIn('NIMBUS_START_MODE=desktop', text)                 # nothing else touched
        st = self.helper('wm', st['wmBefore'])
        self.assertEqual((st['wm'], st['hearthPreview']), ('labwc', False))

    def test_a_switch_asked_for_quickly_is_not_a_failure(self):
        self.write_bin('hearth-compositor', '#!/bin/sh\necho logout > "$XDG_RUNTIME_DIR/nimbus-next"\nexit 0\n')
        r, log = self.session(PREVIEW)
        self.assertEqual(self.fails(), '')
        self.assertEqual(self.notice(), [])
        self.assertEqual(self.lines('kwin-ran'), [])


if __name__ == '__main__':
    unittest.main()
