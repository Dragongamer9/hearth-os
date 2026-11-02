"""Screen modes for Game Mode and Steam Game Mode (docs/STEAM-GAME-MODE.md "Screen modes"): hearth-screen's check of a
chosen mode against the screen's real modes, the ROG Ally's built-in screen as a gamescope display script, and
nimbus-session's check before gamescope starts (run for real between its >>> and <<< marks)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
HELPER = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-screen'
SESSION = ROOT / 'hosts/linux/session/nimbus-session'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_screen', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_screen', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hs = load()


def mode(w, h, r, preferred=False, **t):
    d = {'w': w, 'h': h, 'refresh': r, 'exact': float(r), 'preferred': preferred, 'clock': 0, 'htotal': 0,
         'vsync_start': 0, 'vsync_end': 0, 'vtotal': 0}
    d.update(t)
    return d


# The owner's 4K TV on the RTX 5070 (2026-11-02 log): 2560 × 1440 listed once (one refresh rate), 1920 × 1080 five times.
TV = {'card': 'card0', 'name': 'HDMI-A-1', 'internal': False, 'edid': {'vendor': 'GSM', 'product': 0x1234, 'bytes': 256},
      'modes': [mode(3840, 2160, 60, True), mode(4096, 2160, 120), mode(3840, 2160, 120), mode(2560, 1440, 60),
                mode(1920, 1080, 120), mode(1920, 1080, 60), mode(1280, 720, 60)]}
# The ROG Ally's panel: 1920 × 1080 at 120 Hz (timings that give gamescope's own Ally table, within a few lines).
ALLY_BASE = mode(1920, 1080, 120, True, clock=274560, htotal=2000, vsync_start=1134, vsync_end=1138, vtotal=1144)
ALLY = {'card': 'card0', 'name': 'eDP-1', 'internal': True, 'edid': {'vendor': 'XYZ', 'product': 0x0A1B, 'bytes': 128},
        'modes': [ALLY_BASE, mode(1920, 1080, 60), mode(1280, 800, 120)]}


class Check(unittest.TestCase):
    def test_reads_gamescopes_mode_options(self):
        self.assertEqual(hs.mode_args(['--hdr-enabled', '-W', '2560', '-H', '1440', '-r', '120']), (2560, 1440, 120))
        self.assertEqual(hs.mode_args(['--output-width=1920', '--output-height', '1080', '--nested-refresh=59.94']), (1920, 1080, 60))
        self.assertEqual(hs.mode_args(['--adaptive-sync']), (None, None, None))

    def test_a_mode_the_tv_doesnt_list_is_caught(self):
        code, msg = hs.check([TV], ['-W', '2560', '-H', '1440', '-r', '120'])
        self.assertEqual(code, 1)
        self.assertIn('only at 60 Hz', msg)

    def test_listed_modes_pass(self):
        self.assertEqual(hs.check([TV], ['-W', '1920', '-H', '1080', '-r', '120'])[0], 0)
        self.assertEqual(hs.check([TV], ['-W', '2560', '-H', '1440'])[0], 0)       # no -r: any rate of that size
        self.assertEqual(hs.check([TV], ['--adaptive-sync'])[0], 0)                 # nothing chosen

    def test_gamescopes_blocklisted_size_counts_as_missing(self):
        self.assertEqual(hs.check([TV], ['-W', '4096', '-H', '2160', '-r', '120'])[0], 1)

    def test_what_cant_be_told_is_left_to_gamescope(self):
        self.assertEqual(hs.check([], ['-W', '1920', '-H', '1080'])[0], 2)
        self.assertEqual(hs.check([TV, ALLY], ['-W', '1920', '-H', '1080', '-r', '144'])[0], 2)
        self.assertEqual(hs.check([ALLY], ['-W', '1920', '-H', '1080', '-r', '90'])[0], 2)  # built-in: gamescope's rates
        self.assertEqual(hs.check([ALLY], ['-W', '2560', '-H', '1600'])[0], 1)              # ...but not a size it lacks

    def test_edid_vendor_and_product(self):
        edid = bytearray(128)
        edid[:8] = b'\x00\xff\xff\xff\xff\xff\xff\x00'
        edid[8], edid[9] = 0x1E, 0x6D      # "GSM"
        edid[10], edid[11] = 0x34, 0x12
        self.assertEqual(hs.edid_id(bytes(edid)), ('GSM', 0x1234))
        self.assertIsNone(hs.edid_id(b'\x00' * 128))
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'edid')
            pathlib.Path(p).write_bytes(bytes(edid))
            self.assertEqual(hs.read_edid(p)['vendor'], 'GSM')
            self.assertIsNone(hs.read_edid(os.path.join(d, 'missing')))


class AllyPanel(unittest.TestCase):
    # gamescope's asus.rogally.lcd.lua table (48..120 Hz), for comparison.
    GAMESCOPE = [1771, 1720, 1655, 1600, 1549, 1499, 1455, 1405, 1361, 1320, 1279, 1224, 1200, 1162, 1120, 1088, 1055,
                 1022, 991, 958, 930, 900, 871, 845, 817, 794, 762, 740, 715, 690, 668, 647, 626, 605, 585, 566, 546,
                 526, 507, 488, 470, 452, 435, 419, 402, 387, 371, 355, 341, 326, 310, 297, 284, 269, 255, 244, 229,
                 217, 204, 194, 181, 172, 158, 147, 136, 123, 113, 104, 93, 83, 74, 64, 54]

    def test_front_porches_follow_gamescopes_ally_table(self):
        v = hs.front_porches(ALLY_BASE, 48, 120)
        self.assertEqual(sorted(v), list(range(48, 121)))
        self.assertEqual(v[120], 54)                                    # the panel's own mode, unchanged
        self.assertLessEqual(max(abs(v[r] - self.GAMESCOPE[r - 48]) for r in v), 15)

    def test_script_lists_only_real_rates_and_matches_only_this_panel(self):
        text = hs.panel_script(ALLY)
        self.assertIn('display.vendor == "XYZ" and display.product == 0x0A1B', text)
        self.assertIn('for r = 48, 120 do', text)
        self.assertIn('[48] = ', text)
        self.assertNotIn('[121]', text)
        self.assertIn('return 4000', text)          # gamescope's own scripts (5000) win when they know the panel
        self.assertIn('base_mode.hdisplay ~= 1920', text)

    def test_no_script_when_gamescope_knows_the_panel_or_its_not_built_in(self):
        known = dict(ALLY, edid={'vendor': 'BOE', 'product': 0x0C33, 'bytes': 128})
        self.assertIsNone(hs.panel_script(known))
        self.assertIsNone(hs.panel_script(TV))
        self.assertIsNone(hs.panel_script(dict(ALLY, edid=None)))
        self.assertIsNone(hs.panel_script(dict(ALLY, modes=[mode(1920, 1080, 120, True)])))   # no timings

    def test_written_once_and_removed_when_not_needed(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(hs.write_panel_script([ALLY], d, True), 'written')
            self.assertEqual(hs.write_panel_script([ALLY], d, True), 'unchanged')
            self.assertEqual(os.listdir(d), [hs.SCRIPT])
            self.assertEqual(hs.write_panel_script([ALLY], d, False), 'removed')     # not an Ally
            self.assertEqual(os.listdir(d), [])
            self.assertEqual(hs.write_panel_script([TV], d, True), 'not needed')

    def test_ally_models(self):
        with tempfile.TemporaryDirectory() as d:
            for name, want in (('ROG Ally RC71L_RC71L', True), ('ROG Ally X RC72LA_RC72LA', True),
                               ('ROG Xbox Ally RC73YA_RC73YA', True), ('ROG Flow Z13 GZ302EA', False)):
                pathlib.Path(d, 'sys_vendor').write_text('ASUSTeK COMPUTER INC.\n')
                pathlib.Path(d, 'product_name').write_text(name + '\n')
                self.assertEqual(hs.is_ally(d), want, name)


class SessionPrecheck(unittest.TestCase):
    def run_block(self, helper_body, conf_args, pending=False):
        src = SESSION.read_text()
        block = re.search(r'# >>> mode precheck\n(.*?)# <<< mode precheck', src, re.S).group(1)
        regex = re.search(r"^MODE_ARGS_RE='[^']*'$", src, re.M).group(0)
        has = re.search(r'^has_mode_args\(\) \{.*\}$', src, re.M).group(0)
        note = re.search(r'^note_reverted\(\) \{\n.*?^\}\n', src, re.M | re.S).group(0)
        with tempfile.TemporaryDirectory() as d:
            conf = os.path.join(d, 'nimbus', 'session.conf')
            os.makedirs(os.path.dirname(conf))
            text = 'NIMBUS_GAMESCOPE_ARGS="%s"\n' % conf_args
            pathlib.Path(conf).write_text(text)
            if pending:
                pathlib.Path(d, 'nimbus', 'display-pending.json').write_text('{}')
            helper = os.path.join(d, 'helper')
            pathlib.Path(helper).write_text('import sys\nopen(sys.argv[0] + ".ran", "w").write(" ".join(sys.argv[1:]))\n' + helper_body)
            script = '\n'.join([regex, has, 'have() { command -v "$1" >/dev/null 2>&1; }', 'log() { echo "$*" >>"%s/log"; }' % d,
                                'SESSION_CONF=%s' % conf, note, 'HEARTH_SCREEN_HELPER=%s' % helper, block,
                                'NIMBUS_GAMESCOPE_ARGS="%s"' % conf_args,
                                'if mode_not_listed "$NIMBUS_GAMESCOPE_ARGS"; then echo SKIP; else echo KEEP; fi'])
            out = subprocess.run(['sh', '-c', script], capture_output=True, text=True, timeout=30).stdout.strip()
            rev = os.path.join(d, 'nimbus', 'display-reverted.json')
            return (out, pathlib.Path(conf).read_text() == text, json.loads(pathlib.Path(rev).read_text()) if os.path.exists(rev) else None,
                    os.path.exists(helper + '.ran'), pathlib.Path(d, 'log').read_text() if os.path.exists(os.path.join(d, 'log')) else '')

    def test_a_mode_the_screen_lacks_skips_this_start_and_keeps_your_setting(self):
        out, kept, rev, ran, log = self.run_block('print("HDMI-A-1 doesn\'t have 2560x1440@120"); sys.exit(1)', '--hdr-enabled -W 2560 -H 1440 -r 120')
        self.assertEqual(out, 'SKIP')
        self.assertTrue(kept, 'session.conf is never changed by the check')
        self.assertEqual(rev['args'], '--hdr-enabled -W 2560 -H 1440 -r 120')    # Game Mode says so, with Try again
        self.assertIn("doesn't have 2560x1440@120", log)

    def test_listed_or_unknown_modes_go_to_gamescope_as_before(self):
        for code in (0, 2, 3):
            out, kept, rev, ran, _ = self.run_block('sys.exit(%d)' % code, '-W 1920 -H 1080 -r 120')
            self.assertEqual((out, kept, rev), ('KEEP', True, None), code)

    def test_a_mode_on_trial_is_really_tried(self):
        out, kept, rev, ran, _ = self.run_block('sys.exit(1)', '-W 2560 -H 1440 -r 120', pending=True)
        self.assertEqual((out, rev, ran), ('KEEP', None, False))

    def test_nothing_to_check_without_mode_options(self):
        out, kept, rev, ran, _ = self.run_block('sys.exit(1)', '--adaptive-sync')
        self.assertEqual((out, rev, ran), ('KEEP', None, False))

    def test_steam_game_mode_checks_and_retries_once_at_the_screens_own_mode(self):
        src = SESSION.read_text()
        steam = re.search(r'^run_steam_mode\(\) \{\n.*?^\}\n', src, re.M | re.S).group(0)
        self.assertIn('mode_not_listed "$NIMBUS_GAMESCOPE_ARGS"', steam)
        self.assertIn('strip_mode_args "$NIMBUS_GAMESCOPE_ARGS"', steam)
        self.assertIn('[ ! -s "$NEXT" ]', steam)          # a Switch you asked for is never retried
        game = re.search(r'^run_game\(\) \{\n.*?^\}\n', src, re.M | re.S).group(0)
        self.assertIn('mode_not_listed "$NIMBUS_GAMESCOPE_ARGS"', game)
        self.assertLess(game.index('mode_not_listed'), game.index('gamescope $full'))


if __name__ == '__main__':
    unittest.main()
