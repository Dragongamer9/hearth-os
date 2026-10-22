"""Tests for desktop/shell/hearth-games: the Games section of Mantel's launcher (Hearth's installed games, a copy kept
for next time, games started through Hearth)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import stat
import tempfile
import time
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
PATH = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-games'
RS = ROOT / 'hosts' / 'hearth-core' / 'src' / 'desktop_games.rs'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_games', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_games', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class HearthGames(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-games-test-')
        os.environ['HOME'] = os.path.join(self.tmp, 'home')
        os.environ['XDG_CACHE_HOME'] = os.path.join(self.tmp, 'cache')
        os.environ['XDG_RUNTIME_DIR'] = os.path.join(self.tmp, 'run')
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        self.m = load()

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_hearth(self, out, code=0):
        """A stand-in for the Hearth app that prints `out` for --list-games."""
        exe = os.path.join(self.tmp, 'nimbus')
        with open(exe, 'w') as f:
            f.write('#!/bin/sh\n[ "$1" = --list-games ] || exit 3\ncat <<\'EOF\'\n%s\nEOF\nexit %d\n' % (out, code))
        os.chmod(exe, 0o755)
        os.environ['NIMBUS_APP'] = exe
        return exe

    def test_ids_match_the_rust_rules(self):
        for ok in ('steam:570', 'manual:3f2a', 'beacon:bfme2', 'emu:snes:Super Mario World.sfc'):
            self.assertTrue(self.m.valid_id(ok), ok)
        for bad in ('', '570', 'steam:\n570', 'steam:"x', 'a\\b:c', None, 5, 'steam:' + '9' * 300):
            self.assertFalse(self.m.valid_id(bad), repr(bad))
        # The Rust side checks the same things.
        rs = RS.read_text()
        self.assertIn("id.len() <= 200 && id.contains(':')", rs)

    def test_list_asks_hearth_and_keeps_a_copy(self):
        games = [{'id': 'steam:570', 'title': 'Dota 2', 'source': 'steam', 'cover': '/home/a/.steam/grid/570p.png', 'wide': 'https://cdn/x.jpg'},
                 {'id': 'manual:1', 'title': 'alpha', 'source': 'manual', 'cover': None},
                 {'id': 'bad', 'title': 'no colon'}, 'junk']
        self.fake_hearth('some log line\n' + json.dumps(games))
        out = self.m.list_games()
        self.assertEqual([g['id'] for g in out], ['manual:1', 'steam:570'])   # A to Z, odd entries dropped
        self.assertEqual(out[1]['cover'], 'file:///home/a/.steam/grid/570p.png')
        self.assertEqual(out[1]['wide'], 'https://cdn/x.jpg')
        self.assertEqual(out[0]['cover'], '')
        self.assertEqual(self.m.read_cache(), out)
        # Hearth failing: the copy from last time.
        self.fake_hearth('oops', code=1)
        self.assertEqual(self.m.list_games(), out)
        self.fake_hearth('not json')
        self.assertEqual(self.m.list_games(), out)

    def test_no_hearth_no_copy(self):
        os.environ['NIMBUS_APP'] = os.path.join(self.tmp, 'missing')
        with mock.patch.object(self.m, 'app', return_value=None):
            self.assertEqual(self.m.list_games(), [])
            self.assertEqual(self.m.read_cache(), [])

    def test_pictures(self):
        self.assertEqual(self.m.picture('/a b/c.png'), 'file:///a%20b/c.png')
        self.assertEqual(self.m.picture('https://x/y.jpg'), 'https://x/y.jpg')
        self.assertEqual(self.m.picture('nimbus-art://cache/x'), '')
        self.assertEqual(self.m.picture(None), '')

    def test_play_hands_over_to_an_open_hearth(self):
        with mock.patch.object(self.m, 'running_hearth', return_value=1234):
            self.assertEqual(self.m.play('steam:570'), {'ok': True, 'how': 'handed'})
        with open(os.path.join(os.environ['XDG_RUNTIME_DIR'], 'hearth-play-route.json')) as f:
            r = json.load(f)
        self.assertEqual(r['id'], 'steam:570')
        self.assertLess(abs(r['at'] - time.time()), 5)

    def test_play_opens_hearth(self):
        exe = self.fake_hearth('[]')
        with mock.patch.object(self.m, 'running_hearth', return_value=None), mock.patch.object(self.m.subprocess, 'Popen') as p:
            self.assertEqual(self.m.play('manual:1'), {'ok': True, 'how': 'opened'})
        self.assertEqual(p.call_args[0][0], [exe, '--windowed', '--play', 'manual:1'])
        with self.assertRaises(ValueError):
            self.m.play('rm -rf')

    def test_finds_a_windowed_hearth(self):
        proc = os.path.join(self.tmp, 'proc')
        for pid, argv in (('10', ['/usr/bin/nimbus', '--windowed']), ('11', ['/usr/bin/nimbus']), ('12', ['bash'])):
            os.makedirs(os.path.join(proc, pid))
            with open(os.path.join(proc, pid, 'cmdline'), 'wb') as f:
                f.write(b'\0'.join(a.encode() for a in argv) + b'\0')
        self.assertEqual(self.m.running_hearth(proc), 10)
        shutil.rmtree(os.path.join(proc, '10'))
        self.assertIsNone(self.m.running_hearth(proc))

    def test_command_line(self):
        self.fake_hearth(json.dumps([{'id': 'steam:1', 'title': 'One'}]))
        with mock.patch('builtins.print') as pr:
            self.assertEqual(self.m.main(['hearth-games', 'list']), 0)
        self.assertEqual(json.loads(pr.call_args[0][0])[0]['id'], 'steam:1')
        with mock.patch('builtins.print') as pr:
            self.assertEqual(self.m.main(['hearth-games', 'play', 'nope']), 1)
        self.assertTrue(pr.call_args[0][0].startswith('error: '))
        with mock.patch('sys.stderr'):
            self.assertEqual(self.m.main(['hearth-games']), 2)


if __name__ == '__main__':
    unittest.main()
