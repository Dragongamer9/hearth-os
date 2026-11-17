"""Hearth's own password prompt (1.14, PolkitPrompt.qml: Mantel is Desktop Mode's polkit agent) and the RED FLAG
that password prompts always work: hearth-polkit-fallback starts the classic agent when Mantel's isn't registered
within a few seconds, and again at once when the shell stops; the same agent list as labwc/autostart and
nimbus-watchdog; the password only ever goes to polkit; polkit-actions.js's plain words (with node)."""
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts/linux/desktop/shell'
FALLBACK = SHELL / 'hearth-polkit-fallback'
NODE = shutil.which('node')


def agents(text):
    m = re.search(r'for agent in (.*?); do', text, re.S)
    return [a for a in re.split(r'[\s\\]+', m.group(1)) if a] if m else []


def code(s):
    return re.sub(r'(?m)(^|\s)//.*$', '', s)


def wait_for(cond, secs=8.0):
    end = time.time() + secs
    while time.time() < end:
        if cond():
            return True
        time.sleep(0.1)
    return cond()


class Fallback(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.mkdtemp()
        self.run_dir = os.path.join(self.t, 'run')
        os.makedirs(self.run_dir)
        self.started = os.path.join(self.t, 'started')
        # A stand-in classic agent: notes each start and keeps running until it's stopped.
        self.agent = os.path.join(self.t, 'fake-agent')
        with open(self.agent, 'w') as f:
            f.write('#!/bin/sh\necho $$ >> "%s"\nexec sleep 600\n' % self.started)
        os.chmod(self.agent, 0o755)
        self.procs = []

    def tearDown(self):
        for p in self.procs:
            try:
                p.send_signal(signal.SIGTERM)
                p.wait(5)
            except Exception:  # noqa: BLE001
                p.kill()
        subprocess.run(['pkill', '-f', self.t], capture_output=True)
        shutil.rmtree(self.t, ignore_errors=True)

    def starts(self):
        return pathlib.Path(self.started).read_text().split() if os.path.exists(self.started) else []

    def fallback(self, session_pid='', **env):
        e = dict(os.environ, XDG_RUNTIME_DIR=self.run_dir, HEARTH_POLKIT_AGENTS=self.agent, HEARTH_POLKIT_WAIT='1',
                 HEARTH_POLKIT_POLL='0.3', **env)
        if 'HEARTH_POLKIT' not in env:
            e.pop('HEARTH_POLKIT', None)
        p = subprocess.Popen(['sh', str(FALLBACK)] + ([str(session_pid)] if session_pid else []), env=e,
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.procs.append(p)
        return p

    def fake_quickshell(self):
        """A running process named quickshell (as Mantel's shell is), and the marker naming it."""
        link = os.path.join(self.t, 'quickshell')
        os.symlink(shutil.which('sleep'), link)
        qs = subprocess.Popen([link, '600'])
        self.procs.append(qs)
        with open(os.path.join(self.run_dir, 'hearth-polkit-mantel'), 'w') as f:
            f.write('%d\n' % qs.pid)
        return qs

    def test_no_mantel_agent_starts_the_classic_one(self):
        self.fallback()
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 6))
        time.sleep(1.2)
        self.assertEqual(len(self.starts()), 1)         # once, not again and again

    def test_mantel_registered_leaves_it_off_until_the_shell_stops(self):
        qs = self.fake_quickshell()
        self.fallback()
        time.sleep(2.5)
        self.assertEqual(self.starts(), [])
        qs.kill()
        qs.wait()
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 4))   # the shell crashed: prompts still work

    def test_a_stale_marker_doesnt_count(self):
        with open(os.path.join(self.run_dir, 'hearth-polkit-mantel'), 'w') as f:
            f.write('%d\n' % os.getpid())                 # running, but not quickshell
        self.fallback()
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 6))

    def test_a_crashed_classic_agent_is_started_again(self):
        self.fallback()
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 6))
        os.kill(int(self.starts()[0]), signal.SIGKILL)
        self.assertTrue(wait_for(lambda: len(self.starts()) == 2, 4))

    def test_session_end_stops_it(self):
        session = subprocess.Popen(['sleep', '600'])
        self.procs.append(session)
        p = self.fallback(session.pid)
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 6))
        agent_pid = int(self.starts()[0])
        session.kill()
        session.wait()
        p.wait(5)
        self.assertTrue(wait_for(lambda: not os.path.exists('/proc/%d' % agent_pid) or
                                 pathlib.Path('/proc/%d/stat' % agent_pid).read_text().split()[2] == 'Z', 4))

    def test_classic_switch(self):
        self.fake_quickshell()
        self.fallback(HEARTH_POLKIT='classic')
        self.assertTrue(wait_for(lambda: len(self.starts()) == 1, 3))

    def test_same_agent_list_everywhere(self):
        auto = agents((ROOT / 'hosts/linux/desktop/labwc/autostart').read_text())
        mine = agents(FALLBACK.read_text())
        self.assertTrue(mine)
        self.assertEqual(mine, auto)
        self.assertEqual(mine, agents((ROOT / 'hosts/linux/session/nimbus-watchdog').read_text()))

    def test_posix_sh(self):
        self.assertEqual(subprocess.run(['sh', '-n', str(FALLBACK)]).returncode, 0)
        self.assertTrue(os.access(FALLBACK, os.X_OK))


class Prompt(unittest.TestCase):
    def test_loaded_by_file_name_with_an_off_switch(self):
        s = code((SHELL / 'shell.qml').read_text())
        self.assertRegex(s, r'Loader \{ active: Quickshell\.env\("HEARTH_POLKIT"\) !== "classic"; source: "PolkitPrompt\.qml"')
        self.assertNotIn('Quickshell.Services.Polkit', s)    # the module is only imported by PolkitPrompt.qml

    def test_password_only_to_polkit(self):
        q = code((SHELL / 'PolkitPrompt.qml').read_text())
        self.assertIn('import Quickshell.Services.Polkit', q)
        self.assertIn('PolkitAgent { id: agent }', q)
        submit = q.split('function submit()')[1].split('function cancel()')[0]
        self.assertIn('field.text = "";', submit)            # the field is emptied as it's sent
        self.assertIn('flow.submit(v)', submit)
        # Never printed, kept or handed to anything else.
        for line in q.splitlines():
            if 'console.' in line or 'Process' in line or 'command' in line or 'writer' in line:
                self.assertNotIn('field.text', line)
                self.assertNotIn('submit(', line)
        self.assertIn('TextInput.Password', q)
        self.assertIn('Qt.ImhSensitiveData', q)

    def test_marker_and_controller(self):
        q = code((SHELL / 'PolkitPrompt.qml').read_text())
        self.assertIn('/hearth-polkit-mantel"', q)
        self.assertIn('echo \\"$PPID\\"', q)                 # quickshell's PID (sh's parent)
        self.assertIn('target: SettingsNav; property: "polkitShown"', q)
        self.assertIn('WlrKeyboardFocus.Exclusive', q)
        self.assertIn('cancelAuthenticationRequest()', q)
        self.assertIn('target: "polkit"', q)
        self.assertIn('SettingsNav.polkitShown', (SHELL / 'Pad.qml').read_text())
        self.assertIn('SettingsNav.polkitShown', (SHELL / 'ShellHealth.qml').read_text())   # no refresh mid-prompt


@unittest.skipUnless(NODE, 'node is needed for polkit-actions.js')
class Words(unittest.TestCase):
    def js(self, body):
        src = (SHELL / 'polkit-actions.js').read_text().replace('.pragma library', '')
        r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout.strip().splitlines()[-1])

    def test_words(self):
        r = self.js('''console.log(JSON.stringify([describe("org.freedesktop.policykit.exec"), describe("org.freedesktop.udisks2.filesystem-mount-system"),
            describe("org.freedesktop.Flatpak.app-install"), describe("com.example.whatever"), describe(""),
            program("Authentication is needed to run `/usr/bin/hearth-admin update-status' as the super user"), program("Hi"),
            personLabel({string: "unix-user:dragon", displayName: "Dragon"}), personLabel({string: "unix-group:wheel", isGroup: true}),
            userName({string: "unix-user:dragon"}), initial("Someone in wheel"), initial("")]))''')
        self.assertEqual(r[0], {'who': 'An app', 'what': 'Run something as an administrator'})
        self.assertEqual(r[1], {'who': 'Disks', 'what': 'Open a drive'})
        self.assertEqual(r[2]['who'], 'Bazaar')
        self.assertEqual(r[3], {'who': 'An app', 'what': 'Administrator access'})
        self.assertEqual(r[4], r[3])
        self.assertEqual(r[5], 'hearth-admin')
        self.assertEqual(r[6], '')
        self.assertEqual(r[7], 'Dragon')
        self.assertEqual(r[8], 'Someone in wheel')
        self.assertEqual(r[9], 'dragon')
        self.assertEqual((r[10], r[11]), ('W', '?'))


if __name__ == '__main__':
    unittest.main()
