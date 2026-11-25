"""Sort my Downloads in Mantel's Hearth Files (desktop/shell/hearth_files_sortdl.py through hearth-files,
FilesSortDownloads.qml): the same rules as the Rust Files' sortdl.rs (its tests mirrored here), only moves (never
deletes or overwrites), one Undo puts everything back, unknown names asked of the AI (names only) with the rules'
place when there's no answer, and the mockups' parts (docs/mockups/1.17-sort-downloads) in Mantel's Files."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
HF = SHELL / 'hearth-files'
RUST = ROOT / 'hosts' / 'hearth-ui' / 'apps' / 'files' / 'src' / 'sortdl.rs'
sys.path.insert(0, str(SHELL))
import hearth_files_sortdl as sd  # noqa: E402


class Home(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp(prefix='hearth-sortdl-')
        self.dl = os.path.join(self.home, 'Downloads')
        os.makedirs(self.dl)
        self.env = dict(os.environ, HOME=self.home, XDG_CONFIG_HOME=os.path.join(self.home, '.config'))
        self._old = {k: os.environ.get(k) for k in ('HOME', 'XDG_CONFIG_HOME')}
        os.environ['HOME'] = self.home
        os.environ['XDG_CONFIG_HOME'] = os.path.join(self.home, '.config')

    def tearDown(self):
        for k, v in self._old.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        shutil.rmtree(self.home, ignore_errors=True)

    def put(self, name, body='x'):
        p = os.path.join(self.dl, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(body)
        return p

    def hf(self, *args, path=None):
        env = dict(self.env)
        if path:
            env['PATH'] = path + os.pathsep + env.get('PATH', '')
        r = subprocess.run([sys.executable, str(HF)] + list(args), capture_output=True, text=True, env=env, timeout=60)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return json.loads(r.stdout.strip().splitlines()[-1])


class Rules(Home):
    """sortdl.rs's tests, the same here."""

    def test_copies_are_found(self):
        self.assertEqual(sd.original_of('AppImage(2).zip'), 'AppImage.zip')
        self.assertEqual(sd.original_of('photo (1).jpg'), 'photo.jpg')
        self.assertIsNone(sd.original_of('notes.txt'))
        self.assertIsNone(sd.original_of('(1).txt'))
        self.assertIsNone(sd.original_of('v(1.2).txt'))

    def test_places_match_the_rust_files(self):
        d = self.dl
        want = {
            'hearth-ai-command-center_1.2.AppImage': ('Hearth/AI Command Center', 0),
            'console-remote-play.deb': ('Hearth/Console Remote Play', 0),
            'capture-card-tool.zip': ('Hearth/Capture Card', 0),
            'hearth-os-anatase.iso': ('Hearth/Hearth OS', 0),
            'hearth-recovery.img': ('Hearth/Hearth OS', 0),
            'ithil-notes.txt': ('Hearth/Other', 0),
            'disk.iso': ('Installers/ISOs', 1),
            'Krita.AppImage': ('Installers', 1),
            'steam-installer.bin': ('Installers', 1),
            'mods.tar.zst': ('Installers/Archives', 1),
            'shot.PNG': ('Media/Images', 2),
            'clip.mkv': ('Media/Videos', 2),
            'song.flac': ('Media/Sound', 2),
            'fix.sh': ('Documents/Scripts', 3),
            'budget.ods': ('Documents', 3),
            'mystery.xyz': ('Documents/Unsorted', 3),
        }
        for name, w in want.items():
            self.assertEqual(sd.place(d, name, False), w, name)
        self.assertEqual(sd.place(d, 'Some folder', True), ('Documents/Folders', 3))
        # the same lists of endings and names as sortdl.rs
        rs = RUST.read_text()
        for x in ('.flatpakref', '.tar.zst', '.avif', '.opus', '"Hearth/Console Remote Play"', '"_to_delete/Old copies"'):
            self.assertIn(x.strip('"'), rs)
            self.assertIn(x.strip('"'), (SHELL / 'hearth_files_sortdl.py').read_text())
        self.assertEqual(list(sd.TOP), ['Hearth', 'Installers', 'Media', 'Documents', '_to_delete'])
        for c in sd.COLOURS:
            self.assertIn('0x' + c[1:3], rs)

    def test_sorts_only_moves_and_undo_puts_back(self):
        os.makedirs(os.path.join(self.dl, 'Media/Images'))
        for n, b in [('a.png', 'x'), ('a (1).png', 'x'), ('b.AppImage', 'y'), ('hearth-os-anatase.iso', 'z'), ('notes.pdf', 'n'),
                     ('Media/Images/c.png', 'c'), ('c.png', 'other')]:
            self.put(n, b)
        p = sd.plan(self.dl)
        self.assertTrue(any(m['from'].endswith('a (1).png') and m['group'] == 4 for m in p['moves']))
        self.assertTrue(any(m['from'].endswith('hearth-os-anatase.iso') and m['to'].endswith('Hearth/Hearth OS') for m in p['moves']))
        self.assertFalse(any(m['from'].endswith('/Media') for m in p['moves']))
        r = sd.apply(p)
        self.assertEqual(r['skipped'], 1)  # c.png: one is already in Media/Images
        with open(os.path.join(self.dl, 'Media/Images/c.png')) as f:
            self.assertEqual(f.read(), 'c')  # never overwritten
        self.assertTrue(os.path.exists(os.path.join(self.dl, 'Installers/b.AppImage')))
        self.assertTrue(os.path.exists(os.path.join(self.dl, '_to_delete/Old copies/a (1).png')))
        self.assertEqual(sd.swap(r['done'], True)['moved'], len(r['done']))
        for n in ('b.AppImage', 'a (1).png', 'c.png', 'a.png', 'notes.pdf'):
            self.assertTrue(os.path.exists(os.path.join(self.dl, n)), n)

    def test_a_copy_of_another_size_is_not_old(self):
        self.put('photo.jpg', 'one')
        self.put('photo (1).jpg', 'different')
        self.assertEqual(sd.place(self.dl, 'photo (1).jpg', False), ('Media/Images', 2))

    def test_hidden_and_the_five_folders_stay(self):
        self.put('.hidden.zip')
        for t in sd.TOP:
            os.makedirs(os.path.join(self.dl, t), exist_ok=True)
        self.assertEqual(sd.plan(self.dl)['moves'], [])


class Through_hearth_files(Home):
    def test_state_plan_apply_undo_redo(self):
        self.put('x.deb')
        self.put('y.mp3')
        self.put('who-knows.qqq')
        s = self.hf('sort-state', self.dl)
        self.assertTrue(s['here'])
        self.assertEqual(s['loose'], 3)
        self.assertFalse(self.hf('sort-state', self.home)['here'])
        p = self.hf('sort-plan', self.dl)
        self.assertEqual(p['unknown'], ['who-knows.qqq'])
        self.assertEqual(p['counts'], [0, 1, 1, 1, 0])
        r = self.hf('sort-apply', json.dumps(p))
        self.assertEqual(len(r['done']), 3)
        self.assertEqual(r['folders'], 3)
        s = self.hf('sort-state', self.dl)
        self.assertEqual(s['loose'], 0)
        self.assertEqual(s['folders'], {'Installers': 1, 'Media': 1, 'Documents': 1})
        self.assertEqual(self.hf('sort-undo', json.dumps(r['done']))['moved'], 3)
        self.assertEqual(sorted(os.listdir(self.dl)), sorted(['Documents', 'Installers', 'Media', 'who-knows.qqq', 'x.deb', 'y.mp3']))
        self.assertEqual(self.hf('sort-redo', json.dumps(r['done']))['moved'], 3)

    def test_xdg_download_dir_is_where(self):
        other = os.path.join(self.home, 'Stuff')
        os.makedirs(other)
        os.makedirs(os.path.join(self.home, '.config'))
        with open(os.path.join(self.home, '.config', 'user-dirs.dirs'), 'w') as f:
            f.write('XDG_DOWNLOAD_DIR="$HOME/Stuff"\n')
        self.assertTrue(self.hf('sort-state', other)['here'])
        self.assertFalse(self.hf('sort-state', self.dl)['here'])
        # never somewhere else than Downloads
        self.put('a.zip')
        r = self.hf('sort-plan', self.dl)
        self.assertEqual(r['moves'], [])
        self.assertIn('error', r)

    def test_a_plan_outside_the_five_folders_moves_nothing(self):
        f = self.put('a.zip')
        bad = {'dir': self.dl, 'moves': [{'from': f, 'to': os.path.join(self.home, 'elsewhere'), 'group': 1},
                                         {'from': f, 'to': os.path.join(self.dl, 'Mine'), 'group': 1}]}
        r = self.hf('sort-apply', json.dumps(bad))
        self.assertEqual(r['done'], [])
        self.assertTrue(os.path.exists(f))
        # undo never moves something that wasn't sorted
        self.assertEqual(self.hf('sort-undo', json.dumps([[os.path.join(self.home, 'x'), f]]))['moved'], 0)
        self.assertTrue(os.path.exists(f))

    def test_the_ai_places_unknown_names_only_and_only_where_it_may(self):
        self.put('who-knows.qqq')
        self.put('setup-thing.abc')
        self.put('b.deb')
        bin_ = os.path.join(self.home, 'bin')
        os.makedirs(bin_)
        got = os.path.join(self.home, 'asked.json')
        with open(os.path.join(bin_, 'hearth-layout'), 'w') as f:
            f.write('#!/bin/sh\nprintf "%s" "$2" > ' + json.dumps(got) + '\n'
                    'echo \'{"ok":true,"places":{"who-knows.qqq":"Media/Sound","setup-thing.abc":"/etc"}}\'\n')
        os.chmod(os.path.join(bin_, 'hearth-layout'), 0o755)
        p = self.hf('sort-plan', self.dl)
        a = self.hf('sort-ai', json.dumps(p), path=bin_)
        with open(got) as f:
            self.assertEqual(json.loads(f.read()), {'names': ['setup-thing.abc', 'who-knows.qqq']})  # names only
        to = {os.path.basename(m['from']): m['to'] for m in a['moves']}
        self.assertTrue(to['who-knows.qqq'].endswith('/Media/Sound'))
        self.assertTrue(to['setup-thing.abc'].endswith('/Documents/Unsorted'))  # outside what it may give: the rules' place
        self.assertTrue(to['b.deb'].endswith('/Installers'))
        # no AI: the plan as it was
        empty = os.path.join(self.home, 'empty')
        os.makedirs(empty)
        env_path = empty
        r = subprocess.run([sys.executable, str(HF), 'sort-ai', json.dumps(p)], capture_output=True, text=True,
                           env=dict(self.env, PATH=env_path), timeout=60)
        b = json.loads(r.stdout.strip().splitlines()[-1])
        self.assertEqual([m['to'] for m in b['moves']], [m['to'] for m in p['moves']])


class In_Mantels_Files(unittest.TestCase):
    """The mockups' parts in FileManager.qml / FilesSortDownloads.qml."""

    @classmethod
    def setUpClass(cls):
        cls.fm = (SHELL / 'FileManager.qml').read_text()
        cls.sq = (SHELL / 'FilesSortDownloads.qml').read_text() + (SHELL / 'FilesSortSheet.qml').read_text()

    def test_button_on_the_command_bar_only_in_downloads_accent_when_loose(self):
        self.assertIn('t: "sortdl"', self.fm)
        self.assertIn('✦  Sort my Downloads', self.fm)
        self.assertIn('sortDl.here', self.fm)
        self.assertIn('primary: cmd.modelData.t === "sortdl" && sortDl.loose > 0', self.fm)

    def test_status_bar(self):
        self.assertIn('loose in Downloads', self.fm)
        self.assertIn('· nothing loose', self.fm)

    def test_the_sheet(self):
        for t in ('Sort my Downloads', 'go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back.',
                  'AppImages, .rpm, .deb and Flatpaks · disc images in ISOs · archives in Archives', 'kept for you to empty yourself',
                  'Show every file', 'Hide the files', 'Cancel', '"Sort " + ', 'sort-ai', 'sort-apply'):
            self.assertIn(t, self.sq, t)

    def test_the_note_and_undo(self):
        self.assertIn('✓ Sorted ', self.sq)
        self.assertIn('Ctrl+Z also puts them back', self.sq)
        self.assertIn('kind: "sortdl"', self.sq)
        self.assertIn('k === "sortdl"', self.fm)
        self.assertIn('sort-undo', self.fm)

    def test_folders_in_their_colours(self):
        self.assertIn('sortDl.colourOf', self.fm)
        for c in sd.COLOURS:
            self.assertIn(c, self.sq)


if __name__ == '__main__':
    unittest.main()
