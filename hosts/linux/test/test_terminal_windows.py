"""Hearth's terminal speaks C:\\ (os/system_files/usr/share/hearth/shell/windows-commands.bash, 1.11): run in a real
interactive bash with a pretend home and drive letters."""
import json
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
RC = os.path.join(ROOT, 'os/system_files/usr/share/hearth/shell/windows-commands.bash')
LETTERS = os.path.join(ROOT, 'hosts/linux/desktop/shell/hearth_drive_letters.py')


class Terminal(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.home = os.path.join(self.d, 'home', 'chris')
        os.makedirs(os.path.join(self.home, 'Documents'))
        os.makedirs(os.path.join(self.home, '.config', 'hearth'))
        self.g = os.path.join(self.d, 'games')
        os.makedirs(os.path.join(self.g, 'Steam'))
        json.dump({'drives': {'uuid-g': {'letter': 'G', 'name': 'Games', 'path': self.g}}},
                  open(os.path.join(self.home, '.config', 'hearth', 'drive-letters.json'), 'w'))

    def tearDown(self):
        shutil.rmtree(self.d)

    def sh(self, script):
        env = dict(os.environ, HOME=self.home, USER='chris', PS1='$ ', XDG_CONFIG_HOME=os.path.join(self.home, '.config'))
        pre = 'PS1="$ "; . %s; __hearth_letters=%s; ' % (RC, LETTERS)
        r = subprocess.run(['bash', '--norc', '-c', pre + script], capture_output=True, text=True, env=env, cwd=self.d, timeout=60)
        return r.stdout.strip(), r.stderr.strip()

    def test_cd_to_a_drive_letter_and_back(self):
        out, err = self.sh('cd G:\\\\Steam && pwd')
        self.assertEqual(out, os.path.join(self.g, 'Steam'), err)
        out, _ = self.sh('cd G:/Steam && cd.. && pwd')
        self.assertEqual(out, self.g)

    def test_commands(self):
        out, err = self.sh('md G:\\\\New && copy %s G:\\\\New\\\\x.txt </dev/null && dir G:\\\\New && type G:\\\\New\\\\x.txt' % RC)
        self.assertIn('x.txt', out, err)
        self.assertIn("Hearth's terminal speaks", out)
        out, _ = self.sh('cls >/dev/null 2>&1; ver; where bash')
        self.assertIn('bash', out)

    def test_linux_still_works(self):
        out, _ = self.sh('cd /tmp && pwd && type -t cd && ls -d /tmp')
        self.assertEqual(out.splitlines()[0], '/tmp')
        self.assertIn('function', out)

    def test_off_switch(self):
        env = dict(os.environ, HOME=self.home, HEARTH_WINDOWS_COMMANDS='0')
        r = subprocess.run(['bash', '--norc', '-c', 'PS1="$ "; . %s; type -t dir' % RC], capture_output=True, text=True, env=env)
        self.assertNotEqual(r.stdout.strip(), 'function')

    def test_not_in_scripts(self):
        r = subprocess.run(['bash', '--norc', '-c', 'unset PS1; . %s; type -t copy || echo none' % RC], capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), 'none')


if __name__ == '__main__':
    unittest.main()
