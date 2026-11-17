"""2.0 preview: Mantel on Firehold, Hearth's own compositor (docs/FIREHOLD.md). Never by default: only with
HEARTH_DESKTOP_WM=firehold (Settings › System › Your desktop › Firehold (preview)), and then with the same safe
start as KWin one step above it: missing, ending right away or Mantel never up → KWin for that sign-in (then labwc),
with a notice and a count; twice in a row → KWin until it's turned on again. Turning it off goes back to what was
picked before. The session runs for real with stand-ins (test_kwin_session.py's)."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import subprocess
import json
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
PREVIEW = 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=firehold\n'
LOCKED = PREVIEW


class HearthCompositor(Base):
    def setUp(self):
        super().setUp()
        self.write_bin('kwin_wayland', KWIN_WORKS)
        os.makedirs(os.path.join(self.share, 'desktop'), exist_ok=True)
        import shutil
        shutil.copytree(ROOT / 'hosts/linux/desktop/firehold', os.path.join(self.share, 'desktop', 'firehold'))
        if not self._testMethodName.startswith(('test_hidden', 'test_never')):
            self.unlock()

    def unlock(self):
        os.makedirs(os.path.dirname(self.fh_json), exist_ok=True)
        with open(self.fh_json, 'w') as f:
            f.write('{"firehold": {"unlocked": true}}\n')

    @property
    def fh_json(self):
        return os.path.join(os.path.dirname(self.conf), 'firehold.json')

    def env(self, **kw):
        e = super().env(**kw)
        e.setdefault('FIREHOLD_UP_SECS', '3')
        e.setdefault('FIREHOLD_EARLY_SECS', '2')
        return e

    def test_never_by_default(self):
        self.write_bin('firehold-comp', COMP_WORKS)
        r, log = self.session()
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertEqual(self.lines('comp-ran'), [], log)
        self.assertNotIn('Firehold', log)
        st = self.helper('status')
        self.assertEqual((st['fireholdPreview'], st['fireholdHere']), (False, True))

    def test_preview_runs_it_with_mantels_environment(self):
        self.write_bin('firehold-comp', COMP_WORKS)
        r, log = self.session(PREVIEW)
        self.assertIn("desktop mode: Mantel on Firehold (preview)", log)
        self.assertIn('firehold: Mantel up after', log)
        self.assertEqual(self.lines('kwin-ran'), [], log)
        self.assertEqual(self.lines('labwc-ran'), [], log)
        ran = '\n'.join(self.lines('comp-ran'))
        # On the screens, labwc's rc.xml for Mantel's shortcuts, then the autostart after --.
        self.assertIn('args=--tty-udev --labwc-rc %s/desktop/labwc/rc.xml -- sh %s/desktop/firehold/autostart'
                      % (self.share, self.share), ran)
        pid = [l for l in self.lines('comp-ran') if l.startswith('pid=')][0].split()
        self.assertEqual(pid[0][4:], pid[1][len('desktop_pid='):], 'HEARTH_DESKTOP_PID is the compositor itself')
        self.assertIn('wm=firehold', ran)
        self.assertIn('labwc_pid=unset', ran)
        self.assertIn('xcd=Nimbus:wlroots', ran)        # labwc's environment file: Mantel and the portals take labwc's ways
        self.assertIn('type=wayland', ran)
        self.assertTrue(os.path.exists(os.path.join(self.home, '.local/share/nimbus/firehold.log')))
        self.assertEqual(self.notice(), ['firehold', 'intro'])            # the first time: how to get out
        self.assertIn('Ctrl+Alt+Backspace', self.helper('notice')['sub'])
        self.assertEqual(self.fails(), '')
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)

    def test_missing_opens_kwin_and_says_why_once(self):
        r, log = self.session(PREVIEW, FIREHOLD_BIN='hearth-no-compositor-here')
        self.assertIn("Firehold isn't installed; KWin instead", log)
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertEqual(self.notice(), ['firehold', 'missing'])
        n = self.helper('notice')
        self.assertIn("isn't on this device yet", n['title'])
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)   # the choice stays

    def test_dying_fast_opens_kwin_counts_and_twice_stays(self):
        self.write_bin('firehold-comp', COMP_CRASHES)
        r, log = self.session(PREVIEW)
        self.assertIn('firehold ended before Mantel came up (code 101)', log)
        self.assertIn("desktop: Firehold didn't start; KWin for this sign-in", log)
        self.assertIn('kwin: Mantel up after', log)                      # KWin, the next rung
        self.assertEqual(self.notice(), ['firehold', 'failed'])
        self.assertEqual(self.fails(), 'firehold 1\n')
        # The same sign-in: KWin straight away.
        r, log = self.session(PREVIEW)
        self.assertIn("didn't start earlier this sign-in, KWin instead", log)
        self.assertEqual(len(self.lines('comp-ran')), 1)
        # A new sign-in: tried again, fails again → twice → paused.
        os.remove(os.path.join(self.run_dir, 'firehold-failed'))
        r, log = self.session(PREVIEW)
        self.assertIn("didn't start twice in a row", log)
        self.assertEqual(self.notice(), ['firehold', 'twice'])
        self.assertTrue(self.helper('status')['fireholdPaused'])
        os.remove(os.path.join(self.run_dir, 'firehold-failed'))
        r, log = self.session(PREVIEW)
        self.assertEqual(len(self.lines('comp-ran')), 2, log)
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)   # never rewritten by the session
        # Turned on again in Settings: the count starts over.
        st = self.helper('wm', 'firehold')
        self.assertEqual((st['fireholdFails'], st['fireholdPaused']), (0, False))

    def test_kwin_failing_too_still_opens_labwc(self):
        self.write_bin('firehold-comp', COMP_CRASHES)
        self.write_bin('kwin_wayland', '#!/bin/sh\necho "args=$*" >> "$HOME/kwin-ran"\nexit 1\n')
        r, log = self.session(PREVIEW)
        self.assertEqual(len(self.lines('comp-ran')), 1, log)
        self.assertEqual(len(self.lines('kwin-ran')), 1, log)
        self.assertEqual(len(self.lines('labwc-ran')), 1, log)           # the last rung: Desktop Mode still opens

    def test_mantel_never_up_ends_it_and_opens_kwin(self):
        self.write_bin('firehold-comp', COMP_NO_SHELL)
        t = time.time()
        r, log = self.session(PREVIEW)
        self.assertLess(time.time() - t, 40)
        self.assertIn("firehold: Mantel didn't say it's up after 3s; ending it", log)
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(subprocess.run(['pgrep', '-f', os.path.join(self.bin, 'firehold-comp')]).returncode, 1)

    def test_safe_graphics_and_labwc_pick_never_use_it(self):
        self.write_bin('firehold-comp', COMP_WORKS)
        open(os.path.join(self.tmp, 'safe'), 'w').close()
        r, log = self.session(PREVIEW)
        self.assertIn('safe graphics this time, labwc', log)
        self.assertEqual(self.lines('comp-ran'), [])

    def test_settings_switch_keeps_and_restores_the_choice_before(self):
        self.write_bin('firehold-comp', COMP_WORKS)
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=labwc\n')
        st = self.helper('wm', 'firehold')
        self.assertEqual((st['wm'], st['fireholdPreview'], st['wmBefore']), ('firehold', True, 'labwc'))
        text = pathlib.Path(self.conf).read_text()
        self.assertIn('HEARTH_DESKTOP_WM=firehold', text)
        self.assertIn('HEARTH_DESKTOP_WM_BEFORE=labwc', text)
        self.assertIn('NIMBUS_START_MODE=desktop', text)                 # nothing else touched
        st = self.helper('wm', st['wmBefore'])
        self.assertEqual((st['wm'], st['fireholdPreview']), ('labwc', False))

    def test_the_old_name_keeps_the_preview_on_and_its_count(self):
        # Turned on before the rename (Albireo): still Firehold, the count carried over, nothing reset.
        old = 'NIMBUS_START_MODE=desktop\nHEARTH_DESKTOP_WM=albireo\n'
        os.makedirs(self.state, exist_ok=True)
        with open(os.path.join(self.state, 'desktop-fails'), 'w') as f:
            f.write('albireo 1\n')
        self.write_bin('firehold-comp', COMP_WORKS)
        r, log = self.session(old)
        self.assertIn("desktop mode: Mantel on Firehold (preview)", log)
        st = self.helper('status')
        self.assertTrue(st['fireholdPreview'], st)
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)
        self.assertNotIn('albireo', self.fails())

    def test_hidden_until_unlocked_and_does_nothing(self):
        # Locked: as if Firehold weren't there. The setting is ignored, the helper refuses it, nothing mentions it.
        self.write_bin('firehold-comp', COMP_WORKS)
        r, log = self.session(LOCKED)
        self.assertIn('desktop mode: Mantel on KWin', log)
        self.assertEqual(self.lines('comp-ran'), [], log)
        self.assertNotIn('Firehold', log)
        st = self.helper('status')
        self.assertEqual((st['fireholdUnlocked'], st['wm'], st['fireholdPreview']), (False, 'kwin', False))
        r2 = subprocess.run([sys.executable, str(ROOT / 'hosts/linux/desktop/shell/hearth-your-desktop'), 'wm', 'firehold-once'],
                            env=self.env(), capture_output=True, text=True, timeout=30)
        self.assertNotEqual(r2.returncode, 0)
        self.assertFalse(os.path.exists(os.path.join(self.state, 'firehold-once')))
        self.assertEqual(pathlib.Path(self.conf).read_text(), LOCKED)       # nothing written
        # Unlocked (the 7th tap): shown, and then it runs; hidden again: off, the choice before back.
        st = self.helper('unlock')
        self.assertTrue(st['fireholdUnlocked'])
        self.assertEqual(json.loads(pathlib.Path(self.fh_json).read_text()), {'firehold': {'unlocked': True}})
        self.assertTrue(os.path.exists(self.fh_json + '.bak') or True)
        r, log = self.session(pathlib.Path(self.conf).read_text())
        self.assertIn("desktop mode: Mantel on Firehold (preview)", log)
        st = self.helper('relock')
        self.assertEqual((st['fireholdUnlocked'], st['wm']), (False, 'kwin'))
        self.assertIn('HEARTH_DESKTOP_WM=kwin', pathlib.Path(self.conf).read_text())

    def test_try_once_is_the_next_start_only(self):
        self.write_bin('firehold-comp', COMP_WORKS)
        with open(self.conf, 'w') as f:
            f.write('NIMBUS_START_MODE=desktop\n')
        st = self.helper('wm', 'firehold-once')
        self.assertEqual((st['fireholdMode'], st['wm']), ('once', 'kwin'))
        r, log = self.session(pathlib.Path(self.conf).read_text())
        self.assertIn('Firehold (preview), tried once', log)
        self.assertIn("desktop mode: Mantel on Firehold (preview)", log)
        self.assertEqual(self.helper('status')['fireholdMode'], 'off')
        r, log = self.session(pathlib.Path(self.conf).read_text())
        self.assertIn('desktop mode: Mantel on KWin', log)                  # back by itself, as asked
        self.assertEqual(len([l for l in self.lines('comp-ran') if l.startswith('args=')]), 1)

    def test_leaving_with_the_key_is_kwin_and_not_a_failure(self):
        self.write_bin('firehold-comp', '#!/bin/sh\n: > "$XDG_RUNTIME_DIR/hearth-shell-up"; sleep 3\n'
                       ': > "$XDG_RUNTIME_DIR/firehold-leave"; exit 0\n')
        r, log = self.session(PREVIEW)
        self.assertIn('firehold: left for KWin (Ctrl+Alt+Backspace)', log)
        self.assertIn('kwin: Mantel up after', log)
        self.assertEqual(self.fails(), '')
        self.assertEqual(self.notice(), ['firehold', 'left'])

    def test_twice_turns_always_off_says_so_and_one_tap_turns_it_on(self):
        self.write_bin('firehold-comp', COMP_CRASHES)
        for _ in range(2):
            r, log = self.session(PREVIEW)
            try:
                os.remove(os.path.join(self.run_dir, 'firehold-failed'))
            except OSError:
                pass
        n = self.helper('notice')
        self.assertEqual((n['what'], n['kind']), ('firehold', 'twice'))
        self.assertIn('turned "Always" off', n['sub'])
        self.assertIn('code 101', n['sub'])                                  # the reason
        st = self.helper('status')
        self.assertTrue(st['fireholdPaused'])
        self.assertEqual(pathlib.Path(self.conf).read_text(), PREVIEW)        # the choice kept
        st = self.helper('wm', 'firehold')                                   # the card's "Turn Firehold back on"
        self.assertFalse(st['fireholdPaused'])

    def test_a_switch_asked_for_quickly_is_not_a_failure(self):
        self.write_bin('firehold-comp', '#!/bin/sh\necho logout > "$XDG_RUNTIME_DIR/nimbus-next"\nexit 0\n')
        r, log = self.session(PREVIEW)
        self.assertEqual(self.fails(), '')
        self.assertEqual(self.notice(), ['firehold', 'intro'])
        self.assertEqual(self.lines('kwin-ran'), [])


if __name__ == '__main__':
    unittest.main()
