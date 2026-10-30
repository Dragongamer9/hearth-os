"""Tests for hearth-admin recovery-next-start (Settings › Updates & Recovery › Restart into Recovery, the Dragonfire
look): it runs hearth-recovery's own --next-start and nothing else, says so as JSON, and fails in plain words when
Hearth Recovery is missing or couldn't be set. Nothing here touches a real boot menu: hearth-recovery is replaced."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import importlib.util
import io
import json
import pathlib
import subprocess
import unittest
from unittest import mock

ADMIN = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'
TOOL = '/usr/libexec/nimbus/hearth-recovery'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_admin_recovery', str(ADMIN))
    spec = importlib.util.spec_from_loader('hearth_admin_recovery', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class RecoveryNextStart(unittest.TestCase):
    def setUp(self):
        self.ha = load()

    def run_main(self, argv, exists=True, rc=0, err=''):
        calls = []

        def fake_run(cmd, **kw):
            calls.append(cmd)
            return subprocess.CompletedProcess(cmd, rc, '', err)
        out = io.StringIO()
        with mock.patch.object(self.ha.os, 'geteuid', return_value=0), \
                mock.patch.object(self.ha.os.path, 'exists', side_effect=lambda p: exists if p == TOOL else False), \
                mock.patch.object(self.ha.subprocess, 'run', side_effect=fake_run), contextlib.redirect_stdout(out):
            try:
                self.ha.main(argv)
                code = 0
            except SystemExit as e:
                code = e.code
        return code, out.getvalue(), calls

    def test_runs_hearth_recovery_once(self):
        code, out, calls = self.run_main(['hearth-admin', 'recovery-next-start'])
        self.assertEqual(code, 0)
        self.assertEqual(calls, [[TOOL, '--next-start']])
        self.assertEqual(json.loads(out.strip().splitlines()[-1]), {'ok': True})

    def test_missing_recovery_says_so(self):
        code, out, calls = self.run_main(['hearth-admin', 'recovery-next-start'], exists=False)
        self.assertEqual(code, 1)
        self.assertEqual(calls, [])
        self.assertIn("doesn't have Hearth Recovery", out)

    def test_failure_passes_the_reason_on(self):
        code, out, _ = self.run_main(['hearth-admin', 'recovery-next-start'], rc=1, err='No GRUB menu found\n')
        self.assertEqual(code, 1)
        self.assertIn('error: No GRUB menu found', out)

    def test_takes_no_arguments(self):
        code, _, calls = self.run_main(['hearth-admin', 'recovery-next-start', '--show'])
        self.assertNotEqual(code, 0)
        self.assertEqual(calls, [])


if __name__ == '__main__':
    unittest.main()
