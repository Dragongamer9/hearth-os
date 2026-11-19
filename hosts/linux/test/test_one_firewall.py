"""The Fedora edition ships ONE firewall (firewalld); a device that already ran ufw keeps it as it was
(os-bases/anatase/system_files/usr/libexec/hearth-firewall-choice, its unit and the two drop-ins)."""
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
AF = ROOT / 'os-bases' / 'anatase' / 'system_files'
CHOICE = AF / 'usr' / 'libexec' / 'hearth-firewall-choice'


class Choice(unittest.TestCase):
    def run_choice(self, ufw_conf, ufw_enabled, booted_before, chosen=None):
        with tempfile.TemporaryDirectory() as t:
            t = pathlib.Path(t)
            state = t / 'state'
            state.mkdir()
            if booted_before:
                (state / 'boot-good.json').write_text('{}')
            if chosen:
                (state / 'firewall-chosen').write_text(chosen)
            conf = t / 'ufw.conf'
            if ufw_conf is not None:
                conf.write_text(ufw_conf)
            bin_ = t / 'bin'
            bin_.mkdir()
            (bin_ / 'systemctl').write_text('#!/bin/sh\necho %s\n' % ('enabled' if ufw_enabled else 'disabled'))
            (bin_ / 'systemctl').chmod(0o755)
            env = dict(os.environ, HEARTH_FWC_STATE=str(state), HEARTH_FWC_UFW_CONF=str(conf), PATH='%s:%s' % (bin_, os.environ['PATH']))
            r = subprocess.run(['sh', str(CHOICE)], env=env, capture_output=True, text=True, timeout=10)
            self.assertEqual(r.returncode, 0, r.stderr)
            return (state / 'firewall-chosen').read_text().strip(), (state / 'firewall-ufw').exists()

    def test_existing_device_with_ufw_on_keeps_ufw(self):
        self.assertEqual(self.run_choice('ENABLED=yes\n', True, True), ('ufw', True))

    def test_new_install_gets_firewalld_only(self):
        self.assertEqual(self.run_choice('ENABLED=yes\n', True, False), ('firewalld', False))
        self.assertEqual(self.run_choice(None, False, False), ('firewalld', False))

    def test_existing_device_without_ufw_on_uses_firewalld(self):
        self.assertEqual(self.run_choice('ENABLED=no\n', True, True), ('firewalld', False))
        self.assertEqual(self.run_choice('ENABLED=yes\n', False, True), ('firewalld', False))

    def test_decided_once(self):
        # Already decided (firewalld): a later start never flips it, whatever ufw looks like then.
        self.assertEqual(self.run_choice('ENABLED=yes\n', True, True, chosen='firewalld'), ('firewalld', False))


class Units(unittest.TestCase):
    def test_units_and_dropins(self):
        unit = (AF / 'usr/lib/systemd/system/hearth-firewall-choice.service').read_text()
        self.assertIn('DefaultDependencies=no', unit)
        self.assertIn('Before=firewalld.service ufw.service', unit)
        self.assertIn('TimeoutStartSec=', unit)
        ufw = (AF / 'usr/lib/systemd/system/ufw.service.d/hearth-one-firewall.conf').read_text()
        fwd = (AF / 'usr/lib/systemd/system/firewalld.service.d/hearth-one-firewall.conf').read_text()
        self.assertIn('ConditionPathExists=/var/lib/hearth/firewall-ufw', ufw)
        self.assertIn('ConditionPathExists=!/var/lib/hearth/firewall-ufw', fwd)
        for d in (ufw, fwd):
            self.assertIn('After=hearth-firewall-choice.service', d)
        build = (ROOT / 'os-bases/anatase/build.sh').read_text()
        self.assertIn('systemctl enable firewalld.service', build)
        self.assertNotIn('systemctl disable ufw', build)     # that would take ufw from devices that kept it
        self.assertIn('UFW_KEEP_MARK', (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-firewall').read_text())


if __name__ == '__main__':
    unittest.main()
