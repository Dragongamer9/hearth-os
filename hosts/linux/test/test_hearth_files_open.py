"""Tests for what opening a file does in Hearth Files (hearth-files open-plan, run, make-runnable, open-text,
desktop-launch, open-choice): scripts and programs ask, .desktop files show what they run, and text-like files
without an app open in Hearth Text."""
import sys
sys.dont_write_bytecode = True
import json
import os
import stat
import time

from test_hearth_files import Base, write


class OpenPlan(Base):
    def setUp(self):
        super().setUp()
        self.spawned = []
        self.hf.spawn = lambda argv, cwd=None: self.spawned.append((argv, cwd))
        self.have = {'hearth-text', 'hearth-terminal', 'python3', 'bash', 'sh', 'xdg-open'}
        # hf.shutil is the real shutil module: put its which() back afterwards, for the other tests.
        real = self.hf.shutil.which
        self.addCleanup(setattr, self.hf.shutil, 'which', real)
        self.hf.shutil.which = lambda n, *a, **k: '/usr/bin/' + n if n in self.have else None

    def p(self, name, data=b'', mode=0o644):
        path = os.path.join(self.home, name)
        write(path, data)
        os.chmod(path, mode)
        return path

    def test_scripts(self):
        sh = self.p('backup.sh', 'echo hi\n')
        r = self.hf.open_plan(sh)
        self.assertEqual((r['kind'], r['executable'], r['canChmod'], r['text'], r['interpreter']), ('script', False, True, True, 'bash'))
        nox = self.p('tool', '#!/usr/bin/env python3\nprint(1)\n', 0o755)
        r = self.hf.open_plan(nox)
        self.assertEqual((r['kind'], r['executable'], r['canChmod'], r['interpreter']), ('script', True, False, 'python3'))
        self.assertEqual(self.hf.open_plan(self.p('a.py', 'print(1)'))['interpreter'], 'python3')

    def test_programs_desktop_and_others(self):
        elf = self.p('game', b'\x7fELF\x02\x01\x01\x00' + b'\0' * 64, 0o755)
        self.assertEqual(self.hf.open_plan(elf)['kind'], 'program')
        ai = self.p('App.AppImage', b'\x7fELF\x02\x01\x01\x00AI\x02' + b'\0' * 64, 0o755)
        self.assertEqual(self.hf.open_plan(ai)['kind'], 'default')
        d = self.p('steam.desktop', '[Desktop Entry]\nType=Application\nName=Steam\nExec=steam -silent %U\nTerminal=false\n')
        r = self.hf.open_plan(d)
        self.assertEqual((r['kind'], r['desktop']['name'], r['desktop']['exec'], r['desktop']['runnable']), ('desktop', 'Steam', 'steam -silent %U', True))
        conf = self.p('.gamerc', 'volume=3\n')
        self.assertEqual(self.hf.open_plan(conf)['kind'], 'text')
        log = self.p('weird.data', 'plain words\n')
        self.assertEqual(self.hf.open_plan(log)['kind'], 'text')
        blob = self.p('blob.dat', b'\x00\x01\x02\x03')
        self.assertEqual(self.hf.open_plan(blob)['kind'], 'binary')
        utf16 = self.p('notes.unknown', '﻿hello'.encode('utf-16-le'))
        self.assertEqual(self.hf.open_plan(utf16)['kind'], 'text')
        self.assertEqual(self.hf.open_plan(self.home)['kind'], 'default')
        with self.assertRaises(self.hf.Fail):
            self.hf.open_plan(os.path.join(self.home, 'missing'))

    def test_make_runnable(self):
        sh = self.p('run.sh', 'echo\n', 0o644)
        self.hf.make_runnable(sh)
        self.assertEqual(stat.S_IMODE(os.stat(sh).st_mode), 0o755)
        priv = self.p('mine.sh', 'echo\n', 0o600)
        self.hf.make_runnable(priv)
        self.assertEqual(stat.S_IMODE(os.stat(priv).st_mode), 0o700)
        with self.assertRaises(self.hf.Fail):
            self.hf.make_runnable('/etc/hostname')

    def test_run(self):
        sh = self.p('go.sh', 'echo hi\n', 0o644)
        with self.assertRaises(self.hf.Fail) as e:
            self.hf.run_file(sh)
        self.assertTrue(str(e.exception).startswith('confirm:'))
        os.chmod(sh, 0o755)
        self.hf.run_file(sh)
        self.assertEqual(self.spawned[-1], (['bash', sh], self.home))
        self.hf.run_file(sh, in_terminal=True)
        argv, cwd = self.spawned[-1]
        self.assertEqual(argv[:4], ['hearth-terminal', '--cwd', self.home, '-e'])
        self.assertEqual(argv[4:6], ['sh', '-c'])
        self.assertIn('read -r', argv[6])                 # stays open at the end
        self.assertEqual(argv[-2:], ['bash', sh])
        bang = self.p('tool', '#!/bin/sh\necho\n', 0o755)
        self.hf.run_file(bang)
        self.assertEqual(self.spawned[-1][0], [bang])
        py = self.p('x.py', 'print(1)\n', 0o755)
        self.hf.run_file(py)
        self.assertEqual(self.spawned[-1][0], ['python3', py])
        self.have = {'foot'}
        self.hf.run_file(bang, in_terminal=True)
        self.assertEqual(self.spawned[-1][0][:3], ['foot', '-D', self.home])
        self.have = set()
        with self.assertRaises(self.hf.Fail):
            self.hf.run_file(bang, in_terminal=True)

    def test_open_text(self):
        t = self.p('notes', 'hi\n')
        self.hf.open_text(t, readonly=True)
        self.assertEqual(self.spawned[-1][0], ['hearth-text', '--readonly', '--', t])
        # open_path's fallback when no app opens the type: text goes to Hearth Text, anything else says so.
        self.assertEqual(self.hf.open_unknown(t)['app'], 'hearth-text.desktop')
        with self.assertRaises(self.hf.Fail):
            self.hf.open_unknown(self.p('b.bin', b'\0\0'))
        self.have = set()
        with self.assertRaises(self.hf.Fail):
            self.hf.open_text(t)

    def test_desktop_launch(self):
        d = self.p('app.desktop', '[Desktop Entry]\nType=Application\nName=My App\nExec=myapp --file %f --name=%c 100%%\nPath=/tmp\n')
        self.hf.desktop_launch(d)
        self.assertEqual(self.spawned[-1], (['myapp', '--file', '--name=', '100%'], '/tmp'))
        t = self.p('top.desktop', '[Desktop Entry]\nType=Application\nName=Top\nExec=htop\nTerminal=true\n')
        self.hf.desktop_launch(t)
        self.assertEqual(self.spawned[-1][0], ['hearth-terminal', '--cwd', self.home, '-e', 'htop'])
        link = self.p('site.desktop', '[Desktop Entry]\nType=Link\nName=Site\nURL=https://example.org\n')
        self.hf.desktop_launch(link)
        self.assertEqual(self.spawned[-1][0], ['xdg-open', 'https://example.org'])
        missing = self.p('gone.desktop', '[Desktop Entry]\nType=Application\nName=Gone\nExec=gone\nTryExec=definitely-not-installed\n')
        with self.assertRaises(self.hf.Fail):
            self.hf.desktop_launch(missing)

    def test_remembered_choice(self):
        sh = self.p('again.sh', 'echo 1\n', 0o755)
        self.assertIsNone(self.hf.open_plan(sh)['remembered'])
        self.hf.open_choice(sh, 'terminal')
        self.assertEqual(self.hf.open_plan(sh)['remembered'], 'terminal')
        with self.assertRaises(self.hf.Fail):
            self.hf.open_choice(sh, 'format-the-disk')
        # The file changed: ask again.
        time.sleep(1.1)
        write(sh, 'echo 2 changed\n')
        self.assertIsNone(self.hf.open_plan(sh)['remembered'])
        self.hf.open_choice(sh, 'run')
        self.hf.open_choice(sh)
        self.assertIsNone(self.hf.open_plan(sh)['remembered'])

    def test_cli(self):
        sh = self.p('cli.sh', '#!/bin/sh\necho\n')
        r = self.run_cli('open-plan', sh)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(json.loads(r.stdout)['kind'], 'script')
        r = self.run_cli('run', sh)
        self.assertEqual(r.returncode, 1)
        self.assertIn('confirm:', r.stdout)
        r = self.run_cli('make-runnable', sh, env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertTrue(os.access(sh, os.X_OK))


if __name__ == '__main__':
    import unittest
    unittest.main()
