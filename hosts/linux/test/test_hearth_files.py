"""Tests for desktop/shell/hearth-files (Hearth Files' helper), in a throwaway home folder."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import io
import importlib.util
import json
import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import time
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-files'


def load(home):
    os.environ['HOME'] = home
    for k in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_DIRS', 'XDG_CONFIG_DIRS', 'XDG_CURRENT_DESKTOP'):
        os.environ.pop(k, None)
    loader = importlib.machinery.SourceFileLoader('hearth_files', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_files', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def write(path, data=b'x'):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb') as f:
        f.write(data if isinstance(data, bytes) else data.encode())


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-files-test-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        os.environ['PATH'] = '/nonexistent'      # no gio here: Hearth's own Trash code is used
        self.hf = load(self.home)
        self.media = os.path.join(self.tmp, 'media')
        os.makedirs(self.media)
        self.hf.media_bases = lambda: [self.media]
        quiet = contextlib.redirect_stdout(io.StringIO())     # transfer() prints its progress lines
        quiet.__enter__()
        self.addCleanup(quiet.__exit__, None, None, None)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_cli(self, *args, env=None):
        e = dict(os.environ, PATH='/usr/bin:/bin', PYTHONDONTWRITEBYTECODE='1')
        e.update(env or {})
        return subprocess.run([sys.executable, str(PATH), *args], capture_output=True, text=True, env=e, timeout=60)


class Listing(Base):
    def test_list_hidden_and_unicode(self):
        d = os.path.join(self.home, 'My Stuff')
        write(os.path.join(d, 'Fête été.png'), b'\x89PNG')
        write(os.path.join(d, 'notes.txt'), 'hello')
        write(os.path.join(d, '.secret'), 'shh')
        os.mkdir(os.path.join(d, 'Sub folder'))
        os.symlink('/nonexistent/target', os.path.join(d, 'broken link'))
        r = self.hf.list_dir(d)
        names = {e['name']: e for e in r['entries']}
        self.assertNotIn('.secret', names)
        self.assertTrue(names['Sub folder']['dir'])
        self.assertEqual(names['notes.txt']['size'], 5)
        self.assertEqual(names['Fête été.png']['mime'], 'image/png')
        self.assertTrue(names['broken link']['isLink'])
        self.assertTrue(r['writable'])
        self.assertIn('.secret', {e['name'] for e in self.hf.list_dir(d, hidden=True)['entries']})

    def test_list_file_selects_it(self):
        write(os.path.join(self.home, 'a b.txt'))
        r = self.hf.list_dir(os.path.join(self.home, 'a b.txt'))
        self.assertEqual(r['path'], self.home)
        self.assertEqual(r['select'], 'a b.txt')

    def test_list_missing_and_outside_is_read_only(self):
        with self.assertRaises(self.hf.Fail):
            self.hf.list_dir(os.path.join(self.home, 'nope'))
        r = self.hf.list_dir('/usr')
        self.assertFalse(r['writable'])

    def test_cli_json(self):
        write(os.path.join(self.home, 'x.txt'))
        r = self.run_cli('list', self.home)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(json.loads(r.stdout)['entries'][0]['name'], 'x.txt')
        r = self.run_cli('mkdir', '/usr/hearth-files-test')
        self.assertEqual(r.returncode, 1)
        self.assertTrue(r.stdout.startswith('error: '))


class Places(Base):
    def test_places(self):
        for name in ('Desktop', 'Downloads', 'Pictures/Screenshots/Hearth', 'Games', 'Bilder'):
            os.makedirs(os.path.join(self.home, name))
        write(os.path.join(self.home, '.config', 'user-dirs.dirs'),
              'XDG_PICTURES_DIR="$HOME/Bilder"\nXDG_DOWNLOAD_DIR="$HOME/Downloads"\n')
        os.makedirs(os.path.join(self.home, 'Bilder', 'Screenshots'))
        steam = os.path.join(self.home, '.local', 'share', 'Steam')
        os.makedirs(os.path.join(steam, 'steamapps', 'common'))
        sd = os.path.join(self.media, 'SD Card')
        os.makedirs(os.path.join(sd, 'SteamLibrary', 'steamapps', 'common'))
        write(os.path.join(steam, 'steamapps', 'libraryfolders.vdf'),
              '"libraryfolders"\n{\n "0"\n {\n  "path"  "%s"\n }\n "1"\n {\n  "path"  "%s"\n }\n}\n'
              % (steam, os.path.join(sd, 'SteamLibrary')))
        p = self.hf.places()['places']
        by = {x['id']: x for x in p}
        self.assertEqual(by['home']['path'], self.home)
        self.assertEqual(by['pictures']['path'], os.path.join(self.home, 'Bilder'))
        self.assertIn('screenshots', by)
        self.assertNotIn('documents', by)                    # missing folders are left out
        self.assertIn('games', by)
        steam_labels = sorted(x['label'] for x in p if x['id'].startswith('steam:'))
        self.assertEqual(steam_labels, ['Steam games', 'Steam games (SD Card)'])
        self.assertEqual(by['drive:' + sd]['section'], 'drives')
        self.assertEqual(p[-1]['id'], 'trash')


class Changes(Base):
    def test_mkdir_rename(self):
        d = self.hf.mkdir(os.path.join(self.home, 'Nouveau dossier'))['path']
        self.assertTrue(os.path.isdir(d))
        with self.assertRaises(self.hf.Fail):
            self.hf.mkdir(d)
        r = self.hf.rename(d, 'Ünïcödé name')['path']
        self.assertTrue(os.path.isdir(r))
        write(os.path.join(self.home, 'taken'))
        with self.assertRaises(self.hf.Fail):
            self.hf.rename(r, 'taken')
        for bad in ('', 'a/b', '..'):
            with self.assertRaises(self.hf.Fail):
                self.hf.rename(r, bad)

    def test_refuses_outside(self):
        with self.assertRaises(self.hf.Fail):
            self.hf.rename('/etc/hostname', 'x')
        with self.assertRaises(self.hf.Fail):
            self.hf.delete(['/usr/bin/env'])
        with self.assertRaises(self.hf.Fail):
            self.hf.delete([self.home])                       # never the home folder itself
        with self.assertRaises(self.hf.Fail):
            self.hf.transfer([os.path.join(self.home)], '/usr', move=False)

    def test_delete_never_follows_symlinks(self):
        keep = os.path.join(self.home, 'keep')
        write(os.path.join(keep, 'precious.txt'))
        link = os.path.join(self.home, 'link to keep')
        os.symlink(keep, link)
        d = os.path.join(self.home, 'folder')
        os.makedirs(d)
        os.symlink(keep, os.path.join(d, 'inner link'))
        self.hf.delete([link, d])
        self.assertFalse(os.path.lexists(link))
        self.assertFalse(os.path.exists(d))
        self.assertTrue(os.path.exists(os.path.join(keep, 'precious.txt')))

    def test_trash_and_restore(self):
        f = os.path.join(self.home, 'Docs', 'my file.txt')
        write(f, 'data')
        self.hf.trash([f])
        self.assertFalse(os.path.exists(f))
        t = os.path.join(self.home, '.local', 'share', 'Trash')
        self.assertTrue(os.path.exists(os.path.join(t, 'files', 'my file.txt')))
        info = open(os.path.join(t, 'info', 'my file.txt.trashinfo')).read()
        self.assertIn('Path=' + f.replace(' ', '%20'), info)
        self.assertIn('DeletionDate=', info)
        # A second one with the same name gets its own slot.
        write(f, 'again')
        self.hf.trash([f])
        self.assertEqual(len(os.listdir(os.path.join(t, 'files'))), 2)
        listing = self.hf.list_dir(os.path.join(t, 'files'))
        self.assertTrue(listing['trash'])
        self.assertTrue(all(e['origPath'] == f for e in listing['entries']))
        self.assertEqual(self.hf.places()['places'][-1]['count'], 2)
        self.hf.restore([os.path.join(t, 'files', 'my file.txt')])
        self.assertEqual(open(f).read(), 'data')
        self.hf.empty_trash()
        self.assertEqual(os.listdir(os.path.join(t, 'files')), [])


class Transfers(Base):
    def test_copy_with_progress(self):
        src = os.path.join(self.home, 'Game Folder')
        write(os.path.join(src, 'data', 'big.bin'), os.urandom(3 * 1024 * 1024))
        write(os.path.join(src, 'readme é.txt'), 'hi')
        os.symlink('readme é.txt', os.path.join(src, 'link'))
        dest = os.path.join(self.home, 'SD')
        os.makedirs(dest)
        r = self.run_cli('copy', src, dest, env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        lines = r.stdout.strip().splitlines()
        prog = [json.loads(l[len('progress: '):]) for l in lines if l.startswith('progress: ')]
        self.assertTrue(prog)
        self.assertEqual(prog[-1]['done'], prog[-1]['total'])
        final = json.loads(lines[-1])
        self.assertTrue(final['ok'])
        out = os.path.join(dest, 'Game Folder')
        self.assertEqual(final['items'], [out])
        self.assertEqual(os.path.getsize(os.path.join(out, 'data', 'big.bin')), 3 * 1024 * 1024)
        self.assertTrue(os.path.islink(os.path.join(out, 'link')))
        self.assertTrue(os.path.exists(src))
        # Copying again doesn't overwrite: "Game Folder (2)".
        self.assertEqual(self.hf.transfer([src], dest, move=False)['items'], [out + ' (2)'])

    def test_copy_into_itself_refused(self):
        src = os.path.join(self.home, 'a')
        os.makedirs(os.path.join(src, 'b'))
        with self.assertRaises(self.hf.Fail):
            self.hf.transfer([src], os.path.join(src, 'b'), move=False)

    def test_move_same_drive_is_a_rename(self):
        f = os.path.join(self.home, 'x.txt')
        write(f, 'x')
        ino = os.stat(f).st_ino
        d = os.path.join(self.home, 'dest')
        os.mkdir(d)
        r = self.hf.transfer([f], d, move=True)
        self.assertEqual(os.stat(r['items'][0]).st_ino, ino)
        self.assertFalse(os.path.exists(f))

    def test_move_other_drive_copies_then_deletes(self):
        src = os.path.join(self.home, 'folder')
        write(os.path.join(src, 'f.txt'), 'abc')
        d = os.path.join(self.home, 'dest')
        os.mkdir(d)
        self.hf.same_device = lambda a, b: False
        r = self.hf.transfer([src], d, move=True)
        self.assertEqual(open(os.path.join(r['items'][0], 'f.txt')).read(), 'abc')
        self.assertFalse(os.path.exists(src))

    def test_cancel_removes_partial_copy(self):
        src = os.path.join(self.home, 'big.bin')
        write(src, b'\0' * (24 * 1024 * 1024))
        dest = os.path.join(self.home, 'dest')
        os.mkdir(dest)
        e = dict(os.environ, PATH='/usr/bin:/bin', PYTHONDONTWRITEBYTECODE='1', HEARTH_FILES_SLOW='0.2')
        p = subprocess.Popen([sys.executable, str(PATH), 'copy', src, dest], stdout=subprocess.PIPE, text=True, env=e)
        p.stdout.readline()                                    # the first progress line
        time.sleep(0.3)
        p.send_signal(signal.SIGTERM)
        out, _ = p.communicate(timeout=30)
        final = json.loads(out.strip().splitlines()[-1])
        self.assertTrue(final['cancelled'])
        self.assertEqual(os.listdir(dest), [])
        self.assertTrue(os.path.exists(src))


class Details(Base):
    def test_info_size(self):
        d = os.path.join(self.home, 'folder')
        write(os.path.join(d, 'a'), b'1' * 100)
        write(os.path.join(d, 'sub', 'b'), b'2' * 50)
        i = self.hf.info(d)
        self.assertEqual(i['size'], 150)
        self.assertEqual(i['files'], 2)
        self.assertEqual(i['folders'], 1)
        self.assertTrue(i['dir'])

    def test_apps_and_exec(self):
        apps = os.path.join(self.home, '.local', 'share', 'applications')
        write(os.path.join(apps, 'viewer.desktop'),
              '[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer --open %F\nIcon=viewer\nMimeType=image/png;\n')
        write(os.path.join(apps, 'painter.desktop'),
              '[Desktop Entry]\nType=Application\nName=Painter\nExec=painter %u\nMimeType=image/*;\n')
        write(os.path.join(apps, 'hidden.desktop'),
              '[Desktop Entry]\nType=Application\nName=Hidden\nExec=x\nNoDisplay=true\nMimeType=image/png;\n')
        write(os.path.join(apps, 'text.desktop'),
              '[Desktop Entry]\nType=Application\nName=Text\nExec=text %f\nMimeType=text/plain;\n')
        write(os.path.join(self.home, '.config', 'mimeapps.list'), '[Default Applications]\nimage/png=painter.desktop\n')
        self.hf.data_dirs = lambda: [os.path.join(self.home, '.local', 'share')]   # not the system's apps
        pic = os.path.join(self.home, 'a picture.png')
        write(pic, b'\x89PNG')
        r = self.hf.apps(pic)
        self.assertEqual(r['mime'], 'image/png')
        self.assertEqual([a['id'] for a in r['apps']], ['painter.desktop', 'viewer.desktop'])
        self.assertTrue(r['apps'][0]['default'])
        self.assertEqual(self.hf.expand_exec('viewer --open %F', pic), ['viewer', '--open', pic])
        self.assertEqual(self.hf.expand_exec('painter %u', pic), ['painter', 'file://' + pic.replace(' ', '%20')])
        self.assertEqual(self.hf.expand_exec('"/opt/my app/run"', pic), ['/opt/my app/run', pic])
        self.assertEqual(self.hf.expand_exec('flatpak run --file-forwarding org.x.Y @@ %F @@', pic)[-3:], ['@@', pic, '@@'])


if __name__ == '__main__':
    unittest.main()
