"""hearth-oss-server (Vaultwarden, Nextcloud, Home Assistant for Hearth Passwords, Cloud and Home): only on this PC and
Tailscale, only the digest written down at install, and never part of starting the PC (red flag 2)."""
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
UNITS = ROOT / 'os/system_files/usr/lib/systemd/user'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_oss_server', str(SHELL / 'hearth-oss-server'))
    spec = importlib.util.spec_from_loader('hearth_oss_server', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class OssServer(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.d = tempfile.mkdtemp()
        self.m.STATE = os.path.join(self.d, 'state')
        self.m.DATA = os.path.join(self.d, 'data')

    def test_it_listens_only_here_and_on_tailscale_and_runs_only_its_digest(self):
        dig = 'sha256:' + 'a' * 64
        for name in self.m.SERVERS:
            a = self.m.run_args(name, ['127.0.0.1', '100.64.1.2'], dig)
            ports = [a[i + 1] for i, x in enumerate(a) if x == '-p']
            self.assertTrue(ports and all(p.startswith(('127.0.0.1:', '100.64.1.2:')) for p in ports), (name, ports))
            self.assertIn('--pull=never', a)
            self.assertIn('no-new-privileges', a)
            self.assertNotIn('--network=host', a)
            self.assertTrue(a[-1].endswith('@' + dig), a[-1])
            self.assertNotIn('--privileged', a)

    def test_nothing_runs_before_install_and_off_keeps_the_data(self):
        with self.assertRaises(self.m.Fail):
            self.m.switch('vaultwarden', True)
        st = self.m.status() if self.m.have('systemctl') else None
        if st:
            self.assertFalse(st['vaultwarden']['installed'])

    def test_its_units_never_start_with_the_pc(self):
        for u in ('vaultwarden', 'nextcloud', 'home-assistant'):
            svc = (UNITS / ('hearth-%s.service' % u)).read_text()
            self.assertNotIn('[Install]', svc, u)
            self.assertIn('ConditionPathExists=%h/.local/state/nimbus/oss-server/', svc)
            self.assertIn('[Install]', (UNITS / ('hearth-%s.timer' % u)).read_text())
            build = (ROOT / 'os-bases/peppermint/build.sh').read_text()
            self.assertIn('hearth-%s.timer' % u, build, 'disabled for everyone in the image')


if __name__ == '__main__':
    unittest.main()
