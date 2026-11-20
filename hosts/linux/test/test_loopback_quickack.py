"""Quick ACKs on the loopback (1.16, the Fedora edition; docs/BASE-ANATASE.md): hearth-loopback-quickack sets quickack
on lo's local routes (127.0.0.0/8, 127.0.0.1 and ::1 where it's there) with a stand-in `ip`, never twice, and always
exits 0 (it can't hold up starting the PC); its unit runs after network.target, once, and NetworkManager's dispatcher
runs it again when lo comes back up."""
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
AF = ROOT / 'os-bases/anatase/system_files'
SCRIPT = AF / 'usr/libexec/nimbus/hearth-loopback-quickack'
UNIT = AF / 'usr/lib/systemd/system/hearth-loopback-quickack.service'
HOOK = AF / 'usr/lib/NetworkManager/dispatcher.d/90-hearth-loopback-quickack'

IP = '''#!/bin/sh
echo "$*" >> "$IP_LOG"
case "$*" in
  "route show table local exact 127.0.0.0/8 dev lo") echo "${IP_NET-local 127.0.0.0/8 proto kernel scope host src 127.0.0.1}" ;;
  "route show table local exact 127.0.0.1 dev lo") echo "${IP_ONE-local 127.0.0.1 proto kernel scope host src 127.0.0.1}" ;;
  "-6 route show table local ::1 dev lo") echo "${IP_SIX-}" ;;
  *change*) [ -n "$IP_FAIL" ] && exit 2 ;;
esac
exit 0
'''


class Quickack(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-quickack-')
        self.ip = os.path.join(self.tmp, 'ip')
        with open(self.ip, 'w') as f:
            f.write(IP)
        os.chmod(self.ip, 0o755)
        self.log = os.path.join(self.tmp, 'calls')

    def run_it(self, **env):
        e = dict(os.environ, HEARTH_IP=self.ip, IP_LOG=self.log, **env)
        r = subprocess.run(['sh', str(SCRIPT)], env=e, capture_output=True, text=True, timeout=30)
        calls = open(self.log).read().splitlines() if os.path.exists(self.log) else []
        return r, [c for c in calls if 'change' in c]

    def test_sets_quickack_on_both_ipv4_routes(self):
        r, ch = self.run_it()
        self.assertEqual(r.returncode, 0)
        self.assertEqual(ch, ['route change local 127.0.0.0/8 dev lo proto kernel scope host src 127.0.0.1 table local quickack 1',
                              'route change local 127.0.0.1 dev lo proto kernel scope host src 127.0.0.1 table local quickack 1'])

    def test_ipv6_only_where_it_is_there(self):
        _, ch = self.run_it(IP_SIX='local ::1 dev lo proto kernel metric 0 pref medium')
        self.assertIn('-6 route change local ::1 dev lo proto kernel metric 0 table local quickack 1', ch)
        os.remove(self.log)
        _, ch = self.run_it()
        self.assertFalse([c for c in ch if c.startswith('-6')])

    def test_never_twice_and_never_on_a_missing_route(self):
        _, ch = self.run_it(IP_NET='local 127.0.0.0/8 proto kernel scope host src 127.0.0.1 quickack 1', IP_ONE='')
        self.assertEqual(ch, [])

    def test_failing_or_no_ip_still_exits_0(self):
        r, _ = self.run_it(IP_FAIL='1')
        self.assertEqual(r.returncode, 0)
        self.assertIn("couldn't set quickack", r.stderr)
        r = subprocess.run(['sh', str(SCRIPT)], env=dict(os.environ, HEARTH_IP='/nonexistent/ip'), capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0)

    def test_the_unit_never_holds_up_starting(self):
        u = UNIT.read_text()
        for line in ('After=network.target', 'Type=oneshot', 'RemainAfterExit=yes', 'ExecStart=-/usr/libexec/nimbus/hearth-loopback-quickack', 'TimeoutStartSec=15', 'WantedBy=multi-user.target'):
            self.assertIn(line, u)
        self.assertNotIn('Before=', u)
        self.assertIn('systemctl enable hearth-loopback-quickack.service', (ROOT / 'os-bases/anatase/build.sh').read_text())

    def test_networkmanager_runs_it_again_when_lo_comes_up(self):
        h = HOOK.read_text()
        self.assertIn('up|connectivity-change|reapply', h)
        self.assertIn('timeout 10 /usr/libexec/nimbus/hearth-loopback-quickack', h)
        r = subprocess.run(['sh', str(HOOK), 'lo', 'down'], capture_output=True, timeout=10)
        self.assertEqual(r.returncode, 0)
        for p in (SCRIPT, HOOK):
            self.assertTrue(os.access(p, os.X_OK), p)


if __name__ == '__main__':
    unittest.main()
