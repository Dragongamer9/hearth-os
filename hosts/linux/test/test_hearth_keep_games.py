"""Tests for Quick Resume through restarts: hearth-admin hibernate-reboot (one hibernation that restarts the PC, with
the setting removed again) and the sleep hook that removes it when the PC comes back. Nothing here hibernates:
systemctl and systemd-run are stand-ins, and the files live in a temporary folder."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout

REPO = pathlib.Path(__file__).resolve().parents[3]
ADMIN = REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'
HOOK = REPO / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system-sleep' / 'hearth-restart-keep'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_admin_keep', str(ADMIN))
    spec = importlib.util.spec_from_loader('hearth_admin_keep', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class HibernateReboot(unittest.TestCase):
    def setUp(self):
        self.ha = load()
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.ha.POWER_DISK = os.path.join(self.tmp, 'disk')
        self.ha.RESTART_KEEP_DROPIN = os.path.join(self.tmp, 'run', 'sleep.conf.d', '50-hearth-restart-keep.conf')
        with open(self.ha.POWER_DISK, 'w') as f:
            f.write('[platform] shutdown reboot suspend test_resume\n')
        os.environ['PKEXEC_UID'] = str(os.getuid())
        self.addCleanup(os.environ.pop, 'PKEXEC_UID', None)
        self.calls = []
        self.hibernate_code = 0
        self.dropin_at_hibernate = None
        real = subprocess.run

        def fake(args, **kw):
            self.calls.append(list(args))
            if args[:2] == ['systemctl', 'hibernate']:
                if os.path.exists(self.ha.RESTART_KEEP_DROPIN):
                    with open(self.ha.RESTART_KEEP_DROPIN) as f:
                        self.dropin_at_hibernate = f.read()
                return subprocess.CompletedProcess(args, self.hibernate_code, '', 'Call to Hibernate failed: Sleep verb "hibernate" not supported' if self.hibernate_code else '')
            if args[0] in ('systemctl', 'systemd-run'):
                return subprocess.CompletedProcess(args, 0, '', '')
            return real(args, **kw)
        self.ha.subprocess.run = fake
        self.addCleanup(setattr, self.ha.subprocess, 'run', real)

    def run_it(self):
        out = io.StringIO()
        code = 0
        with redirect_stdout(out):
            try:
                self.ha.hibernate_reboot()
            except SystemExit as e:
                code = e.code
        return code, out.getvalue()

    def test_asks_for_one_hibernation_that_restarts(self):
        code, out = self.run_it()
        self.assertEqual(code, 0, out)
        self.assertIn('"ok": true', out)
        self.assertIn('[Sleep]', self.dropin_at_hibernate)
        self.assertIn('HibernateMode=reboot', self.dropin_at_hibernate)
        # The safety net is set before the hibernation, on the running-time clock, and removes only that file.
        timer = next(c for c in self.calls if c[0] == 'systemd-run')
        self.assertLess(self.calls.index(timer), self.calls.index(['systemctl', 'hibernate']))
        self.assertIn('--on-active=2min', timer)
        self.assertEqual(timer[-3:], ['/usr/bin/rm', '-f', self.ha.RESTART_KEEP_DROPIN])

    def test_a_failed_hibernation_leaves_turning_off_as_it_was(self):
        self.hibernate_code = 1
        code, out = self.run_it()
        self.assertEqual(code, 1)
        self.assertIn("error: couldn't restart with games kept", out)
        self.assertIn('not supported', out)
        self.assertFalse(os.path.exists(self.ha.RESTART_KEEP_DROPIN))

    def test_refused_when_the_kernel_cant_restart_after_hibernating(self):
        with open(self.ha.POWER_DISK, 'w') as f:
            f.write('[platform] shutdown\n')
        code, out = self.run_it()
        self.assertEqual(code, 1)
        self.assertIn("can't restart with games kept", out)
        self.assertFalse(os.path.exists(self.ha.RESTART_KEEP_DROPIN))
        self.assertNotIn(['systemctl', 'hibernate'], self.calls)

    def test_only_through_pkexec(self):
        os.environ.pop('PKEXEC_UID', None)
        code, out = self.run_it()
        self.assertEqual(code, 1)
        self.assertFalse(os.path.exists(self.ha.RESTART_KEEP_DROPIN))

    def test_disk_modes(self):
        self.assertEqual(self.ha.disk_modes('[platform] shutdown reboot suspend test_resume'), ['platform', 'shutdown', 'reboot', 'suspend', 'test_resume'])
        self.assertEqual(self.ha.disk_modes(''), [])

    def test_main_takes_no_extra_arguments(self):
        self.assertIn("cmd == 'hibernate-reboot' and len(argv) == 2", ADMIN.read_text())


class SleepHook(unittest.TestCase):
    """The hook removes the setting after the PC comes back (post), not before it goes (pre)."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.dropin = os.path.join(self.tmp, '50-hearth-restart-keep.conf')
        text = HOOK.read_text().replace('/run/systemd/sleep.conf.d/50-hearth-restart-keep.conf', self.dropin)
        self.hook = os.path.join(self.tmp, 'hook')
        with open(self.hook, 'w') as f:
            f.write(text)
        bindir = os.path.join(self.tmp, 'bin')
        os.makedirs(bindir)
        with open(os.path.join(bindir, 'systemctl'), 'w') as f:
            f.write('#!/bin/sh\nexit 0\n')
        os.chmod(os.path.join(bindir, 'systemctl'), 0o755)
        self.env = dict(os.environ, PATH=bindir + os.pathsep + os.environ.get('PATH', ''))

    def hook_run(self, *args):
        with open(self.dropin, 'w') as f:
            f.write('[Sleep]\nHibernateMode=reboot\n')
        return subprocess.run(['sh', self.hook, *args], env=self.env, capture_output=True, text=True)

    def test_pre_keeps_it(self):
        self.assertEqual(self.hook_run('pre', 'hibernate').returncode, 0)
        self.assertTrue(os.path.exists(self.dropin))

    def test_post_removes_it(self):
        self.assertEqual(self.hook_run('post', 'hibernate').returncode, 0)
        self.assertFalse(os.path.exists(self.dropin))

    def test_hook_is_executable_in_the_repository(self):
        self.assertTrue(os.access(HOOK, os.X_OK))


if __name__ == '__main__':
    unittest.main()
