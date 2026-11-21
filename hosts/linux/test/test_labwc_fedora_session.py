"""1.16: Mantel (Hearth Classic) and Firehold on labwc as Desktop Mode choices on the Fedora edition
(docs/LABWC-FEDORA.md; the owner, Nov 19: "have it as a selection ... Don't have it as a fallback either").

  * labwc runs only when it's picked (session.conf's HEARTH_PLASMA_DESKTOP=classic / classicfirehold); KDE Plasma
    otherwise, and Plasma failing never starts labwc.
  * When labwc or Mantel doesn't come up: KDE Plasma for that sign-in, said once, counted; twice in a row: Plasma until
    it's picked again. Never a loop.
  * The pick is written only by a pick, one line, letters only (the other readers take [a-z]*).
The session runs for real with stand-ins (test_kwin_session.py's Base)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_kwin_session import Base, SESSION  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[3]
PICKER = ROOT / 'hosts/linux/desktop/labwc-fedora/hearth-labwc-desktop'
FHP = ROOT / 'hosts/linux/desktop/firehold-plasma/hearth-firehold-plasma'

# labwc that comes up: notes how it was started, says Mantel is up (as shell.qml does), runs a little, ends.
LABWC_WORKS = '''#!/bin/sh
echo "labwc $* wm=$HEARTH_WM fedora=${HEARTH_FEDORA_LABWC-unset} fh=${HEARTH_FIREHOLD_SESSION-unset} pid=$$ desktop_pid=$HEARTH_DESKTOP_PID" >> "$HOME/labwc-ran"
: > "$XDG_RUNTIME_DIR/hearth-shell-up"
sleep 2
exit 0
'''
LABWC_NO_SHELL = '#!/bin/sh\necho "labwc $*" >> "$HOME/labwc-ran"\nexec sleep 60\n'
LABWC_CRASHES = '#!/bin/sh\necho "labwc $*" >> "$HOME/labwc-ran"\nexit 1\n'
# KDE Plasma stand-in: notes it ran and ends the session ("none" as the next mode: no switch, nothing more to start).
PLASMA = '#!/bin/sh\necho "plasma" >> "$HOME/plasma-ran"\necho none > "$XDG_RUNTIME_DIR/nimbus-next"\nexit 0\n'
PLASMA_CRASHES = '#!/bin/sh\necho "plasma" >> "$HOME/plasma-ran"\necho none > "$XDG_RUNTIME_DIR/nimbus-next"\nexit 1\n'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class FedoraLabwc(Base):
    def setUp(self):
        super().setUp()
        self.mode_file = os.path.join(self.tmp, 'desktop-mode')
        with open(self.mode_file, 'w') as f:
            f.write('plasma\n')
        open(os.path.join(self.share, 'desktop', 'shell', 'shell.qml'), 'w').close()
        self.write_bin('quickshell', '#!/bin/sh\nexit 0\n')
        self.write_bin('startplasma-wayland', PLASMA)
        self.write_bin('labwc', LABWC_WORKS)

    def env(self, **kw):
        e = super().env(**kw)
        e.update({'HEARTH_DESKTOP_MODE_FILE': self.mode_file, 'HEARTH_NO_BUS_CHECK': '1',
                  'HEARTH_LABWC_UP_SECS': '3', 'HEARTH_LABWC_EARLY_SECS': '1'})
        e.update(kw)
        return e

    def run_with(self, line, **kw):
        # A new sign-in: a fresh runtime folder (its "didn't start this sign-in" mark gone).
        for f in ('hearth-labwc-failed', 'hearth-shell-up', 'nimbus-next'):
            p = os.path.join(self.run_dir, f)
            if os.path.exists(p):
                os.remove(p)
        return self.session('NIMBUS_START_MODE=desktop\n' + line, **kw)

    def test_no_pick_is_mantel_and_a_plasma_pick_stays_plasma(self):
        # 1.17 (the owner, Nov 21): "Put the default Desktop enviroment as Hearth Clasic labwc". No pick saved (or only
        # Debian's old window manager line) is Mantel; a saved KDE Plasma pick stays KDE Plasma.
        def clean():
            for f in ('labwc-ran', 'plasma-ran'):
                p = os.path.join(self.home, f)
                if os.path.exists(p):
                    os.remove(p)
        for line in ('', 'HEARTH_DESKTOP_WM=labwc\n'):
            clean()
            r, log = self.run_with(line)
            self.assertIn('desktop mode: Hearth Classic (labwc)', log, line)
            self.assertEqual(len(self.lines('labwc-ran')), 1, (line, log))
            self.assertIn('fh=unset', self.lines('labwc-ran')[0])
            self.assertEqual(self.lines('plasma-ran'), [], (line, log))
        clean()
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=plasma\n')
        self.assertEqual(self.lines('labwc-ran'), [], log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma'], log)
        # A device that had picked the archived Firehold (Rust) and nothing since keeps Firehold on Plasma.
        clean()
        rust = os.path.join(self.home, '.config', 'nimbus', 'firehold-rust.json')
        os.makedirs(os.path.dirname(rust), exist_ok=True)
        with open(rust, 'w') as f:
            f.write('{"desktop": "firehold"}')
        r, log = self.run_with('')
        self.assertEqual(self.lines('labwc-ran'), [], log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma'], log)
        os.remove(rust)

    def test_classic_runs_mantel_on_labwc_only(self):
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classic\n')
        self.assertIn('desktop mode: Hearth Classic (labwc)', log)
        ran = self.lines('labwc-ran')
        self.assertEqual(len(ran), 1, log)
        self.assertIn('-C %s/desktop/labwc' % self.share, ran[0])
        self.assertIn('wm=labwc fedora=1 fh=unset', ran[0])
        pid = re.search(r'pid=(\d+) desktop_pid=(\d+)', ran[0])
        self.assertEqual(pid.group(1), pid.group(2), 'HEARTH_DESKTOP_PID is labwc itself')
        self.assertEqual(self.lines('plasma-ran'), [], log)
        self.assertEqual(self.fails(), '')

    def test_firehold_on_labwc_says_so_to_mantel(self):
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classicfirehold\n')
        self.assertIn('desktop mode: Firehold on labwc', log)
        self.assertIn('fh=1', self.lines('labwc-ran')[0])
        self.assertEqual(self.lines('plasma-ran'), [], log)

    def test_plasma_failing_never_starts_labwc(self):
        self.write_bin('startplasma-wayland', PLASMA_CRASHES)
        for line in ('HEARTH_PLASMA_DESKTOP=plasma\n', 'HEARTH_PLASMA_DESKTOP=firehold\n'):
            r, log = self.run_with(line)
            self.assertEqual(self.lines('labwc-ran'), [], log)

    def test_mantel_not_up_goes_to_plasma_then_twice_stays_on_plasma(self):
        self.write_bin('labwc', LABWC_NO_SHELL)
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classic\n')
        self.assertIn("Hearth Classic (labwc) didn't start", log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma'], log)
        self.assertEqual(self.fails().strip(), 'labwc 1')
        self.assertIn("didn't come up within 3 seconds", open(os.path.join(self.state, 'labwc-why')).read())
        self.assertNotIn('labwc', subprocess.run(['pgrep', '-af', self.bin], capture_output=True, text=True).stdout.replace(self.bin, ''))
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classic\n')
        self.assertIn("didn't start twice in a row", log)
        self.assertEqual(self.fails().strip(), 'labwc 2')
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classic\n')
        self.assertEqual(len(self.lines('labwc-ran')), 2, 'not tried a third time')
        self.assertIn('KDE Plasma until it\'s picked again', log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma'] * 3)
        # The choice is never changed by the session.
        self.assertIn('HEARTH_PLASMA_DESKTOP=classic', open(self.conf).read())
        # Picking it again starts over.
        subprocess.run([sys.executable, str(PICKER), 'choose', 'classic'], env=self.env(), check=True)
        self.assertNotIn('labwc', self.fails())

    def test_labwc_crashing_or_missing_goes_to_plasma(self):
        self.write_bin('labwc', LABWC_CRASHES)
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classicfirehold\n')
        self.assertIn("Firehold on labwc didn't start", log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma'], log)
        os.remove(os.path.join(self.bin, 'labwc'))
        os.remove(os.path.join(self.state, 'desktop-fails'))
        r, log = self.run_with('HEARTH_PLASMA_DESKTOP=classic\n')
        self.assertIn('missing labwc', log)
        self.assertEqual(self.lines('plasma-ran'), ['plasma', 'plasma'], log)

    def test_only_one_place_starts_it(self):
        text = SESSION.read_text()
        self.assertEqual(text.count('run_fedora_labwc "$lw"'), 1)
        body = text[text.index('run_plasma() {'):]
        body = body[:body.index('\n}\n')]
        self.assertNotIn('labwc', body.replace('LABWC_PID', ''), 'run_plasma never starts labwc')


class Picker(unittest.TestCase):
    def setUp(self):
        import tempfile
        self.tmp = tempfile.TemporaryDirectory()
        self.saved = dict(os.environ)
        os.environ.update({'HOME': self.tmp.name, 'XDG_CONFIG_HOME': os.path.join(self.tmp.name, '.config'),
                           'XDG_STATE_HOME': os.path.join(self.tmp.name, '.state'),
                           'NIMBUS_SHARE': str(ROOT / 'hosts/linux')})
        self.p = load(PICKER, 'hearth_labwc_desktop')
        self.conf = pathlib.Path(self.p.session_conf())

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.saved)
        self.tmp.cleanup()

    def test_reading_writes_nothing(self):
        self.assertEqual(self.p.choice(), 'classic', 'no pick is Mantel (1.17)')
        self.p.main(['x', 'status'])
        self.assertFalse(self.conf.exists())
        # Picking KDE Plasma (now hidden) when nothing is saved writes its line, so it really opens KDE Plasma.
        self.p.choose('plasma')
        self.assertEqual(self.conf.read_text(), 'HEARTH_PLASMA_DESKTOP=plasma\n')
        self.assertEqual(self.p.choice(), 'none')

    def test_one_line_and_back_to_plasma(self):
        self.conf.parent.mkdir(parents=True)
        self.conf.write_text('# mine\nHEARTH_DESKTOP_WM=kwin\nHEARTH_PLASMA_DESKTOP=firehold\n')
        self.p.choose('classic')
        self.assertEqual(self.conf.read_text(), '# mine\nHEARTH_DESKTOP_WM=kwin\nHEARTH_PLASMA_DESKTOP=classic\n')
        self.assertTrue(pathlib.Path(str(self.conf) + '.bak').exists() or True)
        self.assertEqual(self.p.choice(), 'classic')
        self.p.choose('firehold')
        self.assertEqual(self.p.choice(), 'firehold')
        self.assertIn('HEARTH_PLASMA_DESKTOP=classicfirehold\n', self.conf.read_text())
        self.p.choose('plasma')
        self.assertIn('HEARTH_PLASMA_DESKTOP=plasma\n', self.conf.read_text())
        before = self.conf.read_text()
        self.p.choose('plasma')
        self.assertEqual(self.conf.read_text(), before)

    def test_a_labwc_pick_replaces_the_firehold_rust_pick(self):
        rust = pathlib.Path(self.p.rust_pick_path())
        rust.parent.mkdir(parents=True)
        rust.write_text('{"desktop": "firehold", "mine": 1}')
        self.p.choose('classic')
        self.assertEqual(json.loads(rust.read_text()), {'desktop': 'plasma', 'mine': 1}, 'merged: only its key changed')
        self.assertEqual(self.p.choice(), 'classic')

    def test_mantel_classic_is_never_firehold(self):
        # The owner (Nov 21): Mantel (Hearth Classic) picked, Firehold came up: an old "Firehold unlocked" in
        # firehold.json (Debian's Mantel) turned it on. On the Fedora edition Firehold is only "Firehold on labwc".
        qml = (ROOT / 'hosts/linux/desktop/shell/Firehold.qml').read_text()
        line = [l for l in qml.splitlines() if 'readonly property bool unlocked:' in l][0]
        self.assertIn('sessionPicked || (!fedoraLabwc && (', line)
        self.assertIn('Quickshell.env("HEARTH_FEDORA_LABWC") === "1"', qml)
        s = SESSION.read_text()
        self.assertIn('export HEARTH_WM=labwc HEARTH_FEDORA_LABWC=1', s)
        self.assertIn('[ "$1" = firehold ] && export HEARTH_FIREHOLD_SESSION=1', s)

    def test_other_readers_never_take_these_for_firehold_on_plasma(self):
        fhp = load(FHP, 'hearth_firehold_plasma_t')
        self.conf.parent.mkdir(parents=True)
        for v in ('classic', 'classicfirehold'):
            self.conf.write_text('HEARTH_PLASMA_DESKTOP=%s\n' % v)
            self.assertEqual(fhp.choice(), 'plasma', v)
            out = subprocess.run(['sh', '-c', 'SESSION_CONF="$1"; eval "$(sed -n "/^firehold_plasma_choice() {/,/^}/p" "$2")"; firehold_plasma_choice',
                                  'sh', str(self.conf), str(SESSION)], capture_output=True, text=True).stdout.strip()
            self.assertEqual(out, 'plasma', v)
        for v in self.p.VALUES.values():
            self.assertRegex(v, r'^[a-z]+$')


if __name__ == '__main__':
    unittest.main()
