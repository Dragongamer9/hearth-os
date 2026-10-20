"""Tests for desktop/shell/hearth-desk (taskbar pins, desktop icon places), in a throwaway home folder."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-desk'


def load(home):
    os.environ['HOME'] = home
    for k in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_DIRS', 'XDG_CONFIG_DIRS'):
        os.environ.pop(k, None)
    loader = importlib.machinery.SourceFileLoader('hearth_desk', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_desk', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-desk-test-')
        self.home = os.path.join(self.tmp, 'home')
        self.desk = os.path.join(self.home, 'Desktop')
        os.makedirs(self.desk)
        os.environ['PATH'] = '/nonexistent'
        self.hd = load(self.home)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def saved(self):
        with open(os.path.join(self.home, '.config', 'nimbus', 'desk.json'), encoding='utf-8') as f:
            return json.load(f)

    def touch(self, name, data='x'):
        with open(os.path.join(self.desk, name), 'w', encoding='utf-8') as f:
            f.write(data)

    def run_cli(self, *args):
        e = dict(os.environ, PATH='/usr/bin:/bin', PYTHONDONTWRITEBYTECODE='1')
        return subprocess.run([sys.executable, str(PATH), *args], capture_output=True, text=True, env=e, timeout=60)


class Pins(Base):
    def test_defaults_without_a_file(self):
        self.assertEqual(self.hd.public(self.hd.load()), {'pins': [], 'desktopIcons': True, 'dnd': False})

    def test_pin_unpin_kept_in_order(self):
        self.hd.pin('firefox.desktop')
        self.hd.pin('org.gnome.Nautilus.desktop')
        self.hd.pin('firefox.desktop')                  # twice: still once
        self.assertEqual(self.saved()['pins'], ['firefox.desktop', 'org.gnome.Nautilus.desktop'])
        self.hd.unpin('firefox.desktop')
        self.hd.unpin('not-pinned')
        self.assertEqual(self.hd.load()['pins'], ['org.gnome.Nautilus.desktop'])

    def test_bad_ids_refused(self):
        for bad in ('', '../evil', ' spaced', 'a\0b'):
            with self.assertRaises(self.hd.Fail):
                self.hd.pin(bad)

    def test_full_taskbar(self):
        for i in range(self.hd.MAX_PINS):
            self.hd.pin('app%d' % i)
        with self.assertRaises(self.hd.Fail):
            self.hd.pin('one-more')

    def test_broken_or_odd_file_is_cleaned(self):
        p = os.path.join(self.home, '.config', 'nimbus', 'desk.json')
        os.makedirs(os.path.dirname(p))
        with open(p, 'w') as f:
            f.write('{"pins": ["ok", "ok", 3, "bad/id"], "dnd": "yes", "desktopIcons": false, "icons": {"a": [1, 2], "b": "x", "c": [-1, 0]}}')
        s = self.hd.load()
        self.assertEqual(s['pins'], ['ok'])
        self.assertFalse(s['dnd'])                      # not a bool: default
        self.assertFalse(s['desktopIcons'])
        self.assertEqual(s['icons'], {'a': [1, 2]})
        with open(p, 'w') as f:
            f.write('not json')
        self.assertEqual(self.hd.load()['pins'], [])

    def test_switches(self):
        self.hd.set_switch('dnd', 'on')
        self.hd.set_switch('desktopIcons', 'off')
        self.assertTrue(self.saved()['dnd'])
        self.assertFalse(self.saved()['desktopIcons'])
        with self.assertRaises(self.hd.Fail):
            self.hd.set_switch('other', 'on')
        with self.assertRaises(self.hd.Fail):
            self.hd.set_switch('dnd', 'maybe')

    def test_cli(self):
        r = self.run_cli('pin', 'foot.desktop')
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(json.loads(r.stdout)['pins'], ['foot.desktop'])
        r = self.run_cli('pin', '../x')
        self.assertEqual(r.returncode, 1)
        self.assertTrue(r.stdout.startswith('error: '))
        self.assertEqual(self.run_cli('nonsense').returncode, 2)


class Layout(Base):
    def test_layout_keeps_saved_and_fills_down_columns(self):
        placed, fresh = self.hd.layout(['a', 'b', 'c', 'd'], {'c': [2, 1]}, 3, 2)
        self.assertEqual(placed['c'], (2, 1))
        self.assertEqual([placed[n] for n in 'abd'], [(0, 0), (0, 1), (1, 0)])
        self.assertEqual(fresh, ['a', 'b', 'd'])

    def test_clash_and_outside_grid(self):
        placed, fresh = self.hd.layout(['a', 'b', 'c'], {'a': [0, 0], 'b': [0, 0], 'c': [9, 9]}, 2, 2)
        self.assertEqual(placed['a'], (0, 0))
        self.assertNotEqual(placed['b'], (0, 0))
        self.assertTrue(placed['c'][0] < 2 and placed['c'][1] < 2)
        self.assertEqual(fresh, [])                     # they all had a saved place
        self.assertEqual(len(set(placed.values())), 3)

    def test_more_icons_than_places(self):
        placed, _ = self.hd.layout(['a', 'b', 'c'], {}, 1, 2)
        self.assertEqual(len(set(placed.values())), 3)
        self.assertEqual(placed['c'], (1, 0))


class Icons(Base):
    def test_lists_desktop_and_remembers_places(self):
        self.touch('notes.txt', 'hi')
        os.mkdir(os.path.join(self.desk, 'Projects'))
        self.touch('.hidden')
        r = self.hd.icons(4, 3)
        self.assertTrue(r['exists'])
        names = [e['name'] for e in r['entries']]
        self.assertEqual(names, ['Projects', 'notes.txt'])   # folders first, no hidden files
        self.assertEqual((r['entries'][0]['col'], r['entries'][0]['row']), (0, 0))
        self.assertEqual(self.saved()['icons'], {'Projects': [0, 0], 'notes.txt': [0, 1]})
        # A new file takes a free place; the old ones stay put.
        self.touch('a.txt')
        r = self.hd.icons(4, 3)
        cellof = {e['name']: (e['col'], e['row']) for e in r['entries']}
        self.assertEqual(cellof['Projects'], (0, 0))
        self.assertEqual(cellof['notes.txt'], (0, 1))
        self.assertEqual(cellof['a.txt'], (0, 2))
        # A removed file is forgotten.
        os.remove(os.path.join(self.desk, 'notes.txt'))
        self.hd.icons(4, 3)
        self.assertNotIn('notes.txt', self.saved()['icons'])

    def test_place_moves_and_swaps(self):
        self.touch('a')
        self.touch('b')
        self.hd.icons(3, 3)
        r = self.hd.place('a', 2, 2, 3, 3)
        cellof = {e['name']: (e['col'], e['row']) for e in r['entries']}
        self.assertEqual(cellof['a'], (2, 2))
        r = self.hd.place('b', 2, 2, 3, 3)           # onto a: they swap
        cellof = {e['name']: (e['col'], e['row']) for e in r['entries']}
        self.assertEqual(cellof['b'], (2, 2))
        self.assertEqual(cellof['a'], (0, 1))
        r = self.hd.place('a', 50, 50, 3, 3)         # off the grid: the nearest place
        cellof = {e['name']: (e['col'], e['row']) for e in r['entries']}
        self.assertEqual(cellof['a'], (2, 2))
        with self.assertRaises(self.hd.Fail):
            self.hd.place('missing', 0, 0, 3, 3)

    def test_small_screen_keeps_big_screen_places(self):
        self.touch('a')
        self.hd.icons(10, 10)
        self.hd.place('a', 8, 8, 10, 10)
        r = self.hd.icons(3, 3)
        self.assertTrue(r['entries'][0]['col'] < 3)
        self.assertEqual(self.saved()['icons']['a'], [8, 8])

    def test_arrange(self):
        for n in ('c', 'a', 'b'):
            self.touch(n)
        self.hd.icons(3, 3)
        self.hd.place('c', 2, 2, 3, 3)
        r = self.hd.arrange(3, 3)
        self.assertEqual([(e['name'], e['col'], e['row']) for e in r['entries']], [('a', 0, 0), ('b', 0, 1), ('c', 0, 2)])

    def test_launchers_show_their_name_and_icon(self):
        self.touch('steam.desktop', '[Desktop Entry]\nType=Application\nName=Steam\nIcon=steam\nExec=steam %U\n')
        e = self.hd.icons(3, 3)['entries'][0]
        self.assertEqual((e['label'], e['icon'], e.get('launcher')), ('Steam', 'steam', True))

    def test_no_desktop_folder(self):
        shutil.rmtree(self.desk)
        r = self.hd.icons(3, 3)
        self.assertFalse(r['exists'])
        self.assertEqual(r['entries'], [])

    def test_desktop_set_to_home_is_ignored(self):
        cfg = os.path.join(self.home, '.config')
        os.makedirs(cfg, exist_ok=True)
        with open(os.path.join(cfg, 'user-dirs.dirs'), 'w') as f:
            f.write('XDG_DESKTOP_DIR="$HOME/"\n')
        self.assertIsNone(self.hd.desktop_dir())
        self.assertFalse(self.hd.icons(3, 3)['exists'])
        with self.assertRaises(self.hd.Fail):
            self.hd.new_folder()

    def test_custom_desktop_dir(self):
        cfg = os.path.join(self.home, '.config')
        os.makedirs(cfg, exist_ok=True)
        with open(os.path.join(cfg, 'user-dirs.dirs'), 'w') as f:
            f.write('XDG_DESKTOP_DIR="$HOME/Schreibtisch"\n')
        self.assertEqual(self.hd.desktop_dir(), os.path.join(self.home, 'Schreibtisch'))

    def test_new_folder_unique_names(self):
        self.assertEqual(self.hd.new_folder()['name'], 'New folder')
        self.assertEqual(self.hd.new_folder()['name'], 'New folder 2')
        self.assertEqual(self.hd.new_folder('Games')['name'], 'Games')
        self.assertTrue(os.path.isdir(os.path.join(self.desk, 'New folder 2')))
        with self.assertRaises(Exception):
            self.hd.new_folder('a/b')

    def test_launch_argv(self):
        self.assertEqual(self.hd.launch_argv({'Exec': 'steam %U -silent'}), ['steam', '-silent'])
        self.assertEqual(self.hd.launch_argv({'Exec': 'app --icon-arg %i', 'Icon': 'x'}), ['app', '--icon-arg', '--icon', 'x'])
        self.assertEqual(self.hd.launch_argv({'Exec': '"/opt/My App/run" --pct=100%%'}), ['/opt/My App/run', '--pct=100%'])
        with self.assertRaises(self.hd.Fail):
            self.hd.launch_argv({'Exec': '%U'})
        with self.assertRaises(self.hd.Fail):
            self.hd.launch_argv({})

    def test_icons_cli(self):
        self.touch('x.txt')
        r = self.run_cli('icons', '5', '4')
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out['entries'][0]['name'], 'x.txt')
        self.assertEqual(self.run_cli('icons', 'x', '4').returncode, 1)


if __name__ == '__main__':
    unittest.main()
