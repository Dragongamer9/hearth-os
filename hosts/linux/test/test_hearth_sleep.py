"""Sleep like SteamOS on Hearth's own editions (Peppermint, Anatase): the power button suspends (logind outside
Hearth, Hearth's own handling inside its session), the ROG Xbox Ally's controller doesn't wake the device, and the
log report's Sleep section is a valid shell script."""
import sys
sys.dont_write_bytecode = True
import configparser
import pathlib
import re
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
EDITIONS = ('peppermint', 'anatase')
LOGIND = 'usr/lib/systemd/logind.conf.d/50-hearth-power.conf'
WAKEUP = 'usr/lib/udev/rules.d/50-hearth-handheld-wakeup.rules'


def edition_file(edition, rel):
    return ROOT / 'os-bases' / edition / 'system_files' / rel


def logind(edition):
    c = configparser.ConfigParser(strict=True, interpolation=None)
    c.optionxform = str
    c.read_string(edition_file(edition, LOGIND).read_text())
    return c


class Logind(unittest.TestCase):
    def test_power_button_sleeps_outside_hearth(self):
        for e in EDITIONS:
            c = logind(e)
            self.assertEqual(c.sections(), ['Login'], e)
            self.assertEqual(c['Login']['HandlePowerKey'], 'suspend', e)
            self.assertEqual(c['Login']['HandleSuspendKey'], 'suspend', e)
            self.assertEqual(c['Login']['HandleLidSwitch'], 'suspend', e)
            # Never turned off by a tap, and no long-press action that could fight Hearth's hold → power menu.
            self.assertNotIn('HandlePowerKeyLongPress', c['Login'], e)
            self.assertNotIn('poweroff', ''.join(c['Login'].values()), e)

    def test_editions_agree(self):
        for rel in (LOGIND, WAKEUP):
            self.assertEqual(edition_file('peppermint', rel).read_text(), edition_file('anatase', rel).read_text(), rel)

    def test_hearth_takes_the_button_while_its_session_runs(self):
        # Without the inhibitor logind would act on the same press as Hearth: two handlers.
        s = (ROOT / 'hosts/linux/session/nimbus-session').read_text()
        self.assertRegex(s, r'systemd-inhibit --what=handle-power-key')
        self.assertIn('--mode=block', s)

    def test_no_other_handler_in_the_images(self):
        # Steam's powerbuttond is turned off for every session in both editions (Hearth handles the button).
        for e in EDITIONS:
            b = (ROOT / 'os-bases' / e / 'build.sh').read_text()
            self.assertIn('systemctl --global disable steamos-powerbuttond.service', b, e)


class Wakeup(unittest.TestCase):
    def test_ally_controller_is_not_a_wake_source(self):
        for e in EDITIONS:
            rules = [l for l in edition_file(e, WAKEUP).read_text().splitlines() if l.strip() and not l.startswith('#')]
            self.assertEqual(len(rules), 1, e)
            r = rules[0]
            for part in ('ACTION=="add"', 'SUBSYSTEM=="usb"', 'ATTR{idVendor}=="0b05"', 'ATTR{idProduct}=="1b4c"',
                         'TEST=="power/wakeup"', 'ATTR{power/wakeup}="disabled"'):
                self.assertIn(part, r, e)

    def test_rule_syntax(self):
        udevadm = shutil.which('udevadm')
        if not udevadm or subprocess.run([udevadm, 'verify', '--help'], capture_output=True).returncode != 0:
            self.skipTest('udevadm verify not available')
        for e in EDITIONS:
            r = subprocess.run([udevadm, 'verify', '--no-style', str(edition_file(e, WAKEUP))], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)


def sleep_script():
    src = (ROOT / 'hosts/hearth-core/src/logs.rs').read_text()
    m = re.search(r'const SLEEP_SH: &str = r##"(.*?)"##;', src, re.S)
    assert m, 'SLEEP_SH not found in logs.rs'
    return m.group(1)


class SleepSection(unittest.TestCase):
    def test_valid_shell(self):
        sh = shutil.which('dash') or shutil.which('sh')
        r = subprocess.run([sh, '-n', '-c', '{ ' + sleep_script() + ' }'], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_runs_anywhere(self):
        # Without sysfs, logind or the journal (a container) it still prints its headings and ends well.
        sh = shutil.which('sh')
        r = subprocess.run([sh, '-c', '{ ' + sleep_script() + ' } 2>&1'], capture_output=True, text=True, timeout=60)
        self.assertEqual(r.returncode, 0, r.stdout)
        for heading in ('sleep states:', '--- who handles the power button', '--- inhibitors', '--- what can wake the device',
                        '--- sleeps this boot', '--- how the previous boot ended'):
            self.assertIn(heading, r.stdout)

    def test_what_it_reports(self):
        s = sleep_script()
        for k in ('/sys/power/mem_sleep', 'total_hw_sleep', 'last_hw_sleep', 'pm_wakeup_irq', 'HandlePowerKey',
                  'systemd-inhibit --list', '/proc/acpi/wakeup', 'power/wakeup', 'amd_pmc', 'PM: suspend (entry|exit)',
                  'journalctl -b -1 -k', 'fan*_input', 'gamepadui'):
            self.assertIn(k, s)
        self.assertTrue(s.rstrip().endswith('\ntrue'))

    def test_who_asked_and_the_kernel_lines_arent_empty(self):
        # -k together with -u matched kernel lines from those units only: none, so that part came out empty.
        s = sleep_script()
        for line in s.splitlines():
            if not line.lstrip().startswith('#') and 'journalctl' in line and re.search(r'(^|\s)-k(\s|$)', line):
                self.assertNotIn(' -u ', line, line)
        # Who asked for each sleep (Hearth Game Mode, Desktop Mode, hearth-buttons, the sleep hook) and Handheld
        # Daemon's own presses, in one timeline with the kernel's and logind's lines.
        for k in ('-t hearth-sleep', 'Executing (short|long) press', '/run/hearth-resumed', 'sort -s -k1,1'):
            self.assertIn(k, s)
        # Steam's gamepad UI is looked for in Steam's own command line (pgrep -f matched this script itself).
        self.assertNotIn("pgrep -f -- '-gamepadui'", s)


if __name__ == '__main__':
    unittest.main()
