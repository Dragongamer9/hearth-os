"""Screen power saving (os/system_files/usr/libexec/nimbus/hearth-screen-psr, 1.14; the owner's i3-N305 laptop froze,
logs G2W9-WQTH-FK and F2RL-BJY4-SV): Auto turns Panel Self Refresh off at runtime on Alder Lake-N / Twin Lake built-in
screens only, On and Off are yours, nothing is saved unless you choose, a damaged file is never saved over, the
person is told once, and nothing touches the start options or the boot menu."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[3]
TOOL = REPO / 'os/system_files/usr/libexec/nimbus/hearth-screen-psr'
UNIT = REPO / 'os/system_files/usr/lib/systemd/system/hearth-screen-psr.service'
SHELL = REPO / 'hosts/linux/desktop/shell'


def load(env):
    with mock.patch.dict(os.environ, env):
        loader = importlib.machinery.SourceFileLoader('hearth_screen_psr', str(TOOL))
        spec = importlib.util.spec_from_loader('hearth_screen_psr', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
    return m


def fake_pc(root, device=0x46d0, vendor=0x8086, edp='connected', debug='0', driver='i915'):
    """A /sys and /sys/kernel/debug like the owner's laptop (card0, eDP-1, Intel 0x46d0)."""
    sysr, dbg = os.path.join(root, 'sys'), os.path.join(root, 'debug')
    dev = os.path.join(root, 'devices', 'pci0000:00', '0000:00:02.0')
    os.makedirs(os.path.join(root, 'drivers', driver))
    os.makedirs(dev)
    os.symlink(os.path.join(root, 'drivers', driver), os.path.join(dev, 'driver'))
    with open(os.path.join(dev, 'vendor'), 'w') as f:
        f.write('0x%04x\n' % vendor)
    with open(os.path.join(dev, 'device'), 'w') as f:
        f.write('0x%04x\n' % device)
    card = os.path.join(sysr, 'class', 'drm', 'card0')
    os.makedirs(card)
    os.symlink(dev, os.path.join(card, 'device'))
    if edp:
        os.makedirs(os.path.join(sysr, 'class', 'drm', 'card0-eDP-1'))
        with open(os.path.join(sysr, 'class', 'drm', 'card0-eDP-1', 'status'), 'w') as f:
            f.write(edp + '\n')
    os.makedirs(os.path.join(sysr, 'class', 'drm', 'card0-HDMI-A-1'))
    if debug is not None:
        d = os.path.join(dbg, 'dri', '0000:00:02.0')
        os.makedirs(d)
        os.symlink('0000:00:02.0', os.path.join(dbg, 'dri', '0'))
        with open(os.path.join(d, 'i915_edp_psr_debug'), 'w') as f:
            f.write(debug + '\n')
        with open(os.path.join(d, 'i915_edp_psr_status'), 'w') as f:
            f.write('Sink support: PSR = yes [0x03], Panel Replay = no\nPSR mode: PSR2 enabled\n')
    return sysr, dbg


class Psr(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.t)
        self.settings = os.path.join(self.t, 'etc', 'screen-power.json')
        self.run_file = os.path.join(self.t, 'run', 'screen-psr.json')
        self.state = os.path.join(self.t, 'state')

    def tool(self, **pc):
        sysr, dbg = fake_pc(os.path.join(self.t, 'pc'), **pc)
        self.dbgfile = os.path.join(dbg, 'dri', '0000:00:02.0', 'i915_edp_psr_debug')
        self.env = {'HEARTH_PSR_SETTINGS': self.settings, 'HEARTH_PSR_RUN': self.run_file, 'HEARTH_PSR_SYS': sysr,
                    'HEARTH_PSR_DEBUGFS': dbg, 'HEARTH_SETTINGS_IO_DIR': str(SHELL), 'XDG_STATE_HOME': self.state}
        return load(self.env)

    def debug_value(self):
        return pathlib.Path(self.dbgfile).read_text().strip()

    def test_auto_turns_it_off_on_the_owners_laptop_only(self):
        m = self.tool()
        self.assertEqual([s['device'] for s in m.intel_screens()], ['0x46d0'])
        self.assertEqual(len(m.switches()), 1)                       # dri/0 and dri/<pci> are one switch
        st = m.apply(wait=0)
        self.assertEqual((st['on'], st['by'], st['applied']), (False, 'auto', True))
        self.assertEqual(self.debug_value(), '1')                    # I915_PSR_DEBUG_DISABLE
        self.assertFalse(os.path.exists(self.settings))              # Auto saves nothing
        self.assertIn('PSR mode: PSR2 enabled', st['status'])
        s = m.status()
        self.assertTrue(s['supported'] and s['knownStall'])
        self.assertEqual(s['choice'], 'auto')
        self.assertIn('Off by itself', s['line'])

    def test_other_intel_laptops_keep_the_drivers_choice(self):
        m = self.tool(device=0xa7a0)                                 # Raptor Lake-P
        st = m.apply(wait=0)
        self.assertEqual((st['on'], st['by']), (True, 'auto'))
        self.assertEqual(self.debug_value(), '0')
        self.assertIn('On:', m.status()['line'])

    def test_not_intel_or_no_built_in_screen(self):
        m = self.tool(vendor=0x1002)
        self.assertEqual(m.intel_screens(), [])
        self.assertFalse(m.status()['supported'])
        self.assertEqual(m.apply(wait=0)['why'], 'no Intel built-in screen')
        shutil.rmtree(os.path.join(self.t, 'pc'))
        m = self.tool(edp=None)
        self.assertFalse(m.status()['supported'])

    def test_on_and_off_are_yours_and_saved_merged(self):
        m = self.tool()
        os.makedirs(os.path.dirname(self.settings))
        with open(self.settings, 'w') as f:
            json.dump({'other': 1}, f)
        st = m.set_choice('on')
        self.assertEqual(json.loads(pathlib.Path(self.settings).read_text()), {'other': 1, 'psr': 'on'})   # merged, the rest kept
        self.assertTrue(os.path.exists(self.settings + '.bak'))
        self.assertEqual((st['choice'], st['wanted'], st['by']), ('on', True, 'you'))
        self.assertEqual(self.debug_value(), '0')
        m.set_choice('off')
        self.assertEqual(self.debug_value(), '1')
        self.assertIn('Off:', m.status()['line'])
        m.set_choice('auto')
        self.assertEqual(json.loads(pathlib.Path(self.settings).read_text())['psr'], 'auto')
        with self.assertRaises(SystemExit):
            m.set_choice('maybe')

    def test_other_debug_bits_are_kept_and_on_only_lifts_an_off(self):
        m = self.tool(debug='0x10')                                  # IRQ debugging someone turned on
        m.apply(wait=0)
        self.assertEqual(self.debug_value(), str(0x11))
        m.set_choice('on')
        self.assertEqual(self.debug_value(), str(0x10))
        with open(self.dbgfile, 'w') as f:
            f.write('3\n')                                           # forced PSR1 by someone: left alone by On
        m.set_choice('on')
        self.assertEqual(self.debug_value(), '3')

    def test_a_damaged_file_is_never_saved_over(self):
        m = self.tool()
        os.makedirs(os.path.dirname(self.settings))
        with open(self.settings, 'w') as f:
            f.write('{"psr": "of')
        self.assertIn('could not be read', m.status()['problem'])
        with self.assertRaises(SystemExit):
            m.set_choice('on')
        self.assertEqual(pathlib.Path(self.settings).read_text(), '{"psr": "of')
        st = m.apply(wait=0)                                         # Auto meanwhile; the file stays as it is
        self.assertEqual((st['on'], st['by']), (False, 'auto'))
        self.assertEqual(pathlib.Path(self.settings).read_text(), '{"psr": "of')

    def test_no_switch_says_so_and_names_the_start_option_without_using_it(self):
        m = self.tool(debug=None)
        st = m.apply(wait=0)
        self.assertFalse(st['applied'])
        self.assertIn('i915.enable_psr=0', st['problem'])
        self.assertIn("Couldn't change it here", m.status()['line'])

    def test_told_once_and_only_when_auto_turned_it_off(self):
        m = self.tool()
        self.assertFalse(m.notice())                                 # nothing done yet
        m.apply(wait=0)
        with mock.patch.dict(os.environ, {'XDG_STATE_HOME': self.state}), mock.patch('shutil.which', return_value=None):
            self.assertTrue(m.notice())
            self.assertFalse(m.notice())                             # once
        log = pathlib.Path(os.path.join(self.state, 'nimbus', 'settings-events.log')).read_text()
        self.assertIn('Settings › Display › Screen power saving', log)
        m.set_choice('off')                                          # yours: no notice about it
        self.assertEqual(json.loads(pathlib.Path(self.run_file).read_text())['by'], 'you')

    def test_command_line(self):
        self.tool()
        env = dict(os.environ, **self.env)
        r = subprocess.run([sys.executable, str(TOOL), 'status'], capture_output=True, text=True, env=env, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(json.loads(r.stdout)['supported'])
        r = subprocess.run([sys.executable, str(TOOL), 'bogus'], capture_output=True, text=True, env=env, timeout=30)
        self.assertEqual(r.returncode, 2)


class StartSafety(unittest.TestCase):
    """Red flag 2: nothing waits for it, a failure is ignored, and boot files are never touched."""

    def test_the_service(self):
        u = UNIT.read_text()
        self.assertIn('Type=simple', u)
        self.assertIn('ExecStart=-/usr/bin/python3 /usr/libexec/nimbus/hearth-screen-psr apply', u)
        self.assertIn('RuntimeMaxSec=', u)
        for bad in ('Before=', 'RequiredBy=', 'Requires=', 'display-manager', 'sddm', 'graphical.target'):
            self.assertNotIn(bad, u.split('[Unit]')[1])
        build = (REPO / 'os-bases/peppermint/build.sh').read_text()
        self.assertIn('systemctl enable hearth-screen-psr.service', build)
        self.assertIn('/usr/libexec/nimbus/hearth-screen-psr', (REPO / 'os/system_files/usr/share/polkit-1/rules.d/49-hearth-memory.rules').read_text())

    def test_never_the_start_options_or_the_boot_menu(self):
        src = TOOL.read_text()
        code = '\n'.join(l for l in src.splitlines() if not l.lstrip().startswith('#'))
        code = code.split('"""', 2)[2]                               # past the docstring
        for bad in ('grub', 'kargs', 'rpm-ostree', 'bootc', '/boot', 'cmdline', 'modprobe.d', 'kernelopts'):
            self.assertNotIn(bad, code)


class SettingsPage(unittest.TestCase):
    def test_display_page_row(self):
        q = (SHELL / 'SetDisplay.qml').read_text()
        self.assertIn('title: "Screen power saving"', q)
        self.assertIn('{ value: "auto", label: "Auto" }, { value: "on", label: "On" }, { value: "off", label: "Off" }', q)
        self.assertIn('psrJob.run(["status"]', q)                    # opening the page only reads
        self.assertIn('SetJob { id: psrSet; helper: page.psrBin; admin: true }', q)
        self.assertIn('power: psrHead', q)

    def test_shell_tells_once(self):
        q = (SHELL / 'shell.qml').read_text()
        self.assertIn('hearth-screen-psr notice', q)


class ShellQuiet(unittest.TestCase):
    """The shell's steady CPU on the owner's laptop (~13%): the dock's attention pulse ran for any badge, and the
    moving wallpaper drew all night."""

    def test_dock_pulse_only_when_urgent(self):
        q = (SHELL / 'Dock.qml').read_text()
        self.assertIn('running: db.visible && !!db.info && db.info.urgent === true && !Settings.reduceMotion', q)

    def test_wallpaper_still_when_idle(self):
        self.assertIn('&& !DeskState.idle', (SHELL / 'Wallpaper.qml').read_text())
        self.assertIn('onIsIdleChanged: DeskState.idle = isIdle', (SHELL / 'IdlePower.qml').read_text())
        self.assertIn('property bool idle: false', (SHELL / 'DeskState.qml').read_text())


if __name__ == '__main__':
    unittest.main()
