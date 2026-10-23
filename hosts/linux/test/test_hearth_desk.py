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
        os.environ['XDG_DATA_DIRS'] = os.path.join(self.tmp, 'share')   # no installed apps (default pins)
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
        self.assertEqual(self.hd.public(self.hd.load()), {'pins': [], 'desktopIcons': True, 'dnd': False,
                                                          'autoArrange': False, 'sortBy': 'name', 'muted': [], 'recent': []})

    def test_beacon_pinned_for_a_new_user_when_installed(self):
        share = os.path.join(self.tmp, 'share')
        os.environ['XDG_DATA_DIRS'] = share
        os.makedirs(os.path.join(share, 'applications'))
        with open(os.path.join(share, 'applications', 'beacon.desktop'), 'w', encoding='utf-8') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Beacon\nExec=beacon-app %U\n')
        self.assertEqual(self.hd.load()['pins'], ['beacon'])
        # Once the taskbar is saved, it's yours: unpinned stays unpinned.
        s = self.hd.load(); s['pins'] = []; self.hd.save(s)
        self.assertEqual(self.hd.load()['pins'], [])

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
        self.assertEqual(self.hd.launch_argv({'Exec': 'printf 100%%foo'}), ['printf', '100%foo'])
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


class Order(Base):
    def test_pins_order(self):
        for i in ('a', 'b', 'c', 'd'):
            self.hd.pin(i)
        self.assertEqual(self.hd.pins_order(['c', 'a', 'zzz', 'c'])['pins'], ['c', 'a', 'b', 'd'])
        self.assertEqual(self.saved()['pins'], ['c', 'a', 'b', 'd'])
        self.assertEqual(self.hd.pins_order([])['pins'], ['c', 'a', 'b', 'd'])

    def test_sorting_and_auto_arrange(self):
        self.touch('b.txt', 'xx')
        self.touch('a.png', 'x' * 10)
        self.touch('c.mp3', '')
        os.utime(os.path.join(self.desk, 'c.mp3'), (2000000000, 2000000000))
        os.makedirs(os.path.join(self.desk, 'z folder'))
        names = lambda r: [e['name'] for e in r['entries']]
        self.assertEqual(names(self.hd.arrange(3, 3, 'size')), ['z folder', 'a.png', 'b.txt', 'c.mp3'])
        self.assertEqual(names(self.hd.arrange(3, 3, 'type')), ['z folder', 'c.mp3', 'a.png', 'b.txt'])
        self.assertEqual(names(self.hd.arrange(3, 3, 'date'))[:2], ['z folder', 'c.mp3'])
        self.assertEqual(self.saved()['sortBy'], 'date')
        with self.assertRaises(self.hd.Fail):
            self.hd.arrange(3, 3, 'colour')
        # Kept lined up: moving an icon is ignored, the order follows the sort.
        self.hd.set_sort('name')
        self.hd.set_switch('autoArrange', 'on')
        self.hd.place('a.png', 2, 2, 3, 3)
        r = self.hd.icons(3, 3)
        self.assertEqual([(e['name'], e['col'], e['row']) for e in r['entries']][:2], [('z folder', 0, 0), ('a.png', 0, 1)])

    def test_recent_apps(self):
        for i in ('a', 'b', 'c', 'a'):
            self.hd.note_recent(i)
        self.assertEqual(self.hd.load()['recent'], ['a', 'c', 'b'])
        for i in range(12):
            self.hd.note_recent('app%d' % i)
        self.assertEqual(len(self.hd.load()['recent']), 8)
        with self.assertRaises(self.hd.Fail):
            self.hd.note_recent('../x')

    def test_mute_app(self):
        self.assertEqual(self.hd.mute_app('Discord', 'on')['muted'], ['Discord'])
        self.assertEqual(self.hd.mute_app('Discord', 'on')['muted'], ['Discord'])
        self.assertEqual(self.hd.mute_app('Steam', 'on')['muted'], ['Discord', 'Steam'])
        self.assertEqual(self.hd.mute_app('Discord', 'off')['muted'], ['Steam'])
        with self.assertRaises(self.hd.Fail):
            self.hd.mute_app('  ', 'on')


class NewFiles(Base):
    def test_new_text_file(self):
        self.assertEqual(self.hd.new_file()['name'], 'New text file.txt')
        self.assertEqual(self.hd.new_file()['name'], 'New text file 2.txt')
        self.assertEqual(os.path.getsize(os.path.join(self.desk, 'New text file.txt')), 0)

    def test_templates(self):
        t = os.path.join(self.home, 'Templates')
        os.makedirs(os.path.join(t, 'sub'))
        with open(os.path.join(t, 'Letter.odt'), 'w') as f:
            f.write('LETTER')
        with open(os.path.join(t, '.hidden'), 'w') as f:
            f.write('x')
        r = self.hd.templates()
        self.assertEqual([x['file'] for x in r['templates']], ['Letter.odt'])
        made = self.hd.new_file(os.path.join(t, 'Letter.odt'))
        self.assertEqual(made['name'], 'New Letter.odt')
        with open(made['path']) as f:
            self.assertEqual(f.read(), 'LETTER')
        outside = os.path.join(self.tmp, 'evil.txt')
        with open(outside, 'w') as f:
            f.write('x')
        with self.assertRaises(self.hd.Fail):
            self.hd.new_file(outside)


class Clipboard(Base):
    def test_uri_paths(self):
        text = '# comment\r\nfile:///home/me/My%20File.txt\r\nfile://localhost/tmp/a\nfile://other/x\nhttps://example.com/\n/abs/path\n'
        self.assertEqual(self.hd.uri_paths(text), ['/home/me/My File.txt', '/tmp/a', '/abs/path'])

    def test_file_uri_round_trip(self):
        p = '/home/me/Ünïcode #1 50%.txt'
        self.assertEqual(self.hd.uri_paths(self.hd.file_uri(p)), [p])

    def fake_wl(self, types, payload):
        """wl-copy / wl-paste stand-ins on PATH: wl-copy saves its input, wl-paste answers from files."""
        bin_ = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_, exist_ok=True)
        store = os.path.join(self.tmp, 'clip')
        with open(os.path.join(bin_, 'wl-copy'), 'w') as f:
            f.write('#!/bin/sh\ncat > "%s"\n' % store)
        with open(os.path.join(bin_, 'wl-paste'), 'w') as f:
            f.write('#!/bin/sh\ncase "$*" in *--list-types*) printf "%s\\n" %s ;; *) cat "%s" ;; esac\n'
                    % ('%s', ' '.join(types), store))
        for n in ('wl-copy', 'wl-paste'):
            os.chmod(os.path.join(bin_, n), 0o755)
        with open(store, 'wb') as f:
            f.write(payload)
        os.environ['PATH'] = bin_ + ':/usr/bin:/bin'
        os.environ['XDG_RUNTIME_DIR'] = self.tmp
        return store

    def test_copy_cut_and_read_back(self):
        self.touch('a b.txt')
        store = self.fake_wl(['text/uri-list'], b'')
        p = os.path.join(self.desk, 'a b.txt')
        self.assertEqual(self.hd.clip_copy([p])['count'], 1)
        with open(store) as f:
            self.assertEqual(self.hd.uri_paths(f.read()), [p])
        self.assertEqual(self.hd.clipboard(), {'paths': [p], 'cut': False, 'image': False})
        self.hd.clip_copy([p], cut=True)
        self.assertTrue(self.hd.clipboard()['cut'])
        self.hd.pasted([p])
        self.assertFalse(self.hd.clipboard()['cut'])
        with self.assertRaises(self.hd.Fail):
            self.hd.clip_copy([os.path.join(self.desk, 'missing')])

    def test_gnome_cut_and_pictures(self):
        self.touch('x')
        p = os.path.join(self.desk, 'x')
        self.fake_wl(['x-special/gnome-copied-files'], ('cut\n' + self.hd.file_uri(p)).encode())
        self.assertEqual(self.hd.clipboard(), {'paths': [p], 'cut': True, 'image': False})
        self.fake_wl(['image/png', 'text/html'], b'\x89PNG fake')
        self.assertTrue(self.hd.clipboard()['image'])
        r = self.hd.paste_image()
        self.assertEqual(r['name'], 'Pasted picture.png')
        with open(r['path'], 'rb') as f:
            self.assertEqual(f.read(), b'\x89PNG fake')
        self.fake_wl(['text/plain'], b'hello')
        self.assertEqual(self.hd.clipboard(), {'paths': [], 'cut': False, 'image': False})
        with self.assertRaises(self.hd.Fail):
            self.hd.paste_image()

    def test_without_wl_clipboard(self):
        self.assertEqual(self.hd.clipboard(), {'paths': [], 'cut': False, 'image': False})
        self.touch('x')
        with self.assertRaises(self.hd.Fail):
            self.hd.clip_copy([os.path.join(self.desk, 'x')])


class Shortcut(Base):
    def test_app_on_the_desktop(self):
        share = os.path.join(self.tmp, 'share')
        os.environ['XDG_DATA_DIRS'] = share
        os.makedirs(os.path.join(share, 'applications', 'kde4'))
        with open(os.path.join(share, 'applications', 'firefox.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\n')
        with open(os.path.join(share, 'applications', 'kde4', 'old.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Old\nExec=old\n')
        r = self.hd.desktop_shortcut('firefox')
        self.assertEqual(r['name'], 'firefox.desktop')
        self.assertEqual(self.hd.desktop_shortcut('firefox.desktop')['name'], 'firefox 2.desktop')
        self.assertEqual(self.hd.desktop_shortcut('kde4-old')['name'], 'old.desktop')
        e = [x for x in self.hd.icons(4, 4)['entries'] if x['name'] == 'firefox.desktop'][0]
        self.assertEqual((e['label'], e.get('launcher')), ('Firefox', True))
        with self.assertRaises(self.hd.Fail):
            self.hd.desktop_shortcut('missing')
        with self.assertRaises(self.hd.Fail):
            self.hd.desktop_shortcut('../etc/passwd')


class PlaceMany(Base):
    def grid(self, r):
        return {e['name']: (e['col'], e['row']) for e in r['entries']}

    def test_move_a_selection_together(self):
        for n in ('a', 'b', 'c', 'd'):
            self.touch(n)
        self.hd.icons(3, 3)                       # a(0,0) b(0,1) c(0,2) d(1,0)
        g = self.grid(self.hd.place_many([('a', 1, 0), ('b', 1, 1)], 3, 3))
        self.assertEqual((g['a'], g['b']), ((1, 0), (1, 1)))
        self.assertEqual(g['d'], (0, 0))          # d was in the way: it takes a's old place
        self.assertEqual(len(set(g.values())), 4)

    def test_same_target_twice_keeps_one(self):
        for n in ('a', 'b'):
            self.touch(n)
        self.hd.icons(3, 3)
        g = self.grid(self.hd.place_many([('a', 2, 2), ('b', 2, 2)], 3, 3))
        self.assertEqual(g['a'], (2, 2))
        self.assertEqual(g['b'], (0, 1))

    def test_cli_and_same_device(self):
        self.touch('a')
        r = self.run_cli('place-many', '3', '3', 'a', '2', '1')
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(self.grid(json.loads(r.stdout))['a'], (2, 1))
        self.assertEqual(self.run_cli('place-many', '3', '3', 'a', '2').returncode, 2)
        self.assertTrue(self.hd.same_device(self.desk, [os.path.join(self.desk, 'a')])['move'])
        self.assertFalse(self.hd.same_device(self.desk, ['/nonexistent'])['move'])


if __name__ == '__main__':
    unittest.main()
