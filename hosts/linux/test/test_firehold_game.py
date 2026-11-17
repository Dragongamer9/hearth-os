"""Firehold's game rows (docs/FIREHOLD.md, "Games: the Guide's rows"): hearth-firehold-game, which the Guide runs.
Each game's choices go into ~/.config/hearth/firehold-games.json only through hearth_settings_io (atomic, .bak, merge
under the lock); `status` (what opening the Guide runs) writes nothing; a value that isn't one of the Guide's is
refused without touching the file. A stand-in compositor answers on the control socket like firehold-comp."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import socket
import subprocess
import tempfile
import threading
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
HELPER = ROOT / 'hosts/linux/desktop/shell/hearth-firehold-game'


class FakeComp(threading.Thread):
    """Answers status / reload like firehold-comp, with a fullscreen game called 'ember'."""

    def __init__(self, path):
        super().__init__(daemon=True)
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.bind(path)
        self.sock.listen(8)
        self.got = []

    def run(self):
        while True:
            try:
                c, _ = self.sock.accept()
            except OSError:
                return
            line = c.makefile().readline().strip()
            self.got.append(line)
            game = {'appId': 'ember', 'frameLimit': 60, 'scaling': 'quality', 'renderSize': [2560, 1440]}
            c.sendall((json.dumps({'ok': True, 'backend': 'nested', 'game': game}) + '\n').encode())
            c.close()


class Helper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.run_dir = os.path.join(self.tmp, 'run')
        os.makedirs(self.run_dir)
        self.cfg = os.path.join(self.tmp, 'config')
        self.file = os.path.join(self.cfg, 'hearth', 'firehold-games.json')
        self.env = dict(os.environ, HOME=self.tmp, XDG_CONFIG_HOME=self.cfg, XDG_RUNTIME_DIR=self.run_dir,
                        XDG_STATE_HOME=os.path.join(self.tmp, 'state'), WAYLAND_DISPLAY='wayland-9')
        self.env.pop('FIREHOLD_GAMES', None)

    def run_helper(self, *args):
        p = subprocess.run([sys.executable, str(HELPER), *args], env=self.env, capture_output=True, text=True,
                           timeout=20)
        return p.returncode, p.stdout.strip().splitlines()[-1] if p.stdout.strip() else ''

    def fake(self):
        f = FakeComp(os.path.join(self.run_dir, 'firehold-game-wayland-9.sock'))
        f.start()
        return f

    def test_status_without_firehold_writes_nothing(self):
        rc, out = self.run_helper('status')
        self.assertEqual(rc, 0)
        self.assertEqual(json.loads(out)['running'], False)
        self.assertFalse(os.path.exists(os.path.join(self.cfg, 'hearth')))

    def test_status_labels_and_nothing_written(self):
        self.fake()
        rc, out = self.run_helper('status')
        st = json.loads(out)
        self.assertTrue(st['running'])
        self.assertEqual(st['game']['frameLimitLabel'], '60')
        self.assertEqual(st['game']['scalingLabel'], 'Quality · 1440p')
        self.assertFalse(os.path.exists(self.file), 'opening the Guide must write nothing')

    def test_set_for_the_fullscreen_game_and_reload(self):
        f = self.fake()
        rc, out = self.run_helper('set', 'frameLimit', '30')
        self.assertEqual(rc, 0, out)
        data = json.load(open(self.file))
        self.assertEqual(data, {'games': {'ember': {'frameLimit': 30}}})
        self.assertIn('reload', f.got)
        # Merge: another game's choice and another key stay.
        self.run_helper('set', 'scaling', 'balanced', 'other.game')
        self.run_helper('set', 'vrr', 'off')
        data = json.load(open(self.file))
        self.assertEqual(data['games']['ember'], {'frameLimit': 30, 'vrr': False})
        self.assertEqual(data['games']['other.game'], {'scaling': 'balanced'})
        self.assertTrue(os.path.exists(self.file + '.bak'), 'the last good copy is kept')

    def test_step_cycles_like_the_guide(self):
        self.fake()
        self.run_helper('set', 'frameLimit', '120', 'ember')
        self.run_helper('step', 'frameLimit', 'up')
        self.assertEqual(json.load(open(self.file))['games']['ember']['frameLimit'], 'screen')
        self.run_helper('step', 'frameLimit', 'up')
        self.assertEqual(json.load(open(self.file))['games']['ember']['frameLimit'], 'off')
        self.run_helper('step', 'frameLimit', 'down')
        self.assertEqual(json.load(open(self.file))['games']['ember']['frameLimit'], 'screen')
        self.run_helper('step', 'scaling', 'up')
        self.assertEqual(json.load(open(self.file))['games']['ember']['scaling'], 'quality')
        self.run_helper('step', 'direct', 'up')
        self.assertIs(json.load(open(self.file))['games']['ember']['direct'], False)

    def test_bad_values_change_nothing(self):
        self.fake()
        self.run_helper('set', 'frameLimit', '60', 'ember')
        before = open(self.file).read()
        for args in (('set', 'frameLimit', '33'), ('set', 'scaling', 'ultra'), ('set', 'vrr', 'maybe'),
                     ('set', 'colour', 'red'), ('step', 'colour', 'up')):
            rc, out = self.run_helper(*args)
            self.assertEqual(rc, 1, args)
            self.assertTrue(out.startswith('error: '), out)
        self.assertEqual(open(self.file).read(), before)

    def test_no_game_no_write(self):
        rc, out = self.run_helper('set', 'frameLimit', '60')
        self.assertEqual(rc, 1)
        self.assertIn('no fullscreen game', out)
        self.assertFalse(os.path.exists(self.file))

    def test_damaged_file_is_put_back_not_replaced(self):
        self.fake()
        self.run_helper('set', 'frameLimit', '40', 'ember')
        self.run_helper('set', 'scaling', 'quality', 'ember')   # .bak now has frameLimit 40
        with open(self.file, 'w') as f:
            f.write('{"games": ')   # cut short (a power cut)
        self.run_helper('set', 'vrr', 'off', 'ember')
        g = json.load(open(self.file))['games']['ember']
        self.assertEqual(g.get('frameLimit'), 40, 'the choices before were put back from .bak, not reset')
        self.assertIs(g['vrr'], False)


if __name__ == '__main__':
    unittest.main()
