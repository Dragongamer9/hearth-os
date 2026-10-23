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

    def test_move_across_bind_mount_copies(self):
        # Same device number but rename says EXDEV (the same drive mounted twice): copied, then removed.
        import errno
        from unittest import mock
        f = os.path.join(self.home, 'a.txt')
        write(f, 'abc')
        d = os.path.join(self.home, 'dest')
        os.mkdir(d)
        with mock.patch.object(self.hf.os, 'rename', side_effect=OSError(errno.EXDEV, 'Invalid cross-device link')):
            r = self.hf.transfer([f], d, move=True)
        self.assertTrue(r['ok'])
        with open(os.path.join(d, 'a.txt')) as fh:
            self.assertEqual(fh.read(), 'abc')
        self.assertFalse(os.path.exists(f))

    def test_copy_links_to_drive_without_links(self):
        # FAT32/exFAT can't hold symlinks: the file a link points to is copied instead, a folder link left out.
        import errno
        from unittest import mock
        src = os.path.join(self.home, 'game')
        write(os.path.join(src, 'data.bin'), 'data')
        os.symlink('data.bin', os.path.join(src, 'link.bin'))
        os.mkdir(os.path.join(src, 'sub'))
        os.symlink('sub', os.path.join(src, 'sublink'))
        d = os.path.join(self.media, 'USB')
        os.mkdir(d)
        with mock.patch.object(self.hf.os, 'symlink', side_effect=OSError(errno.EPERM, 'Operation not permitted')):
            r = self.hf.transfer([src], d, move=False)
        self.assertTrue(r['ok'])
        out = os.path.join(d, 'game')
        self.assertFalse(os.path.islink(os.path.join(out, 'link.bin')))
        with open(os.path.join(out, 'link.bin')) as fh:
            self.assertEqual(fh.read(), 'data')
        self.assertFalse(os.path.lexists(os.path.join(out, 'sublink')))

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

    def test_expand_exec_names_with_percent_signs(self):
        # A file name with "%u" or "%%" in it is put in as it is, never read again as a field code.
        odd = '/home/me/100%% done %u.txt'
        self.assertEqual(self.hf.expand_exec('edit --file=%f', odd), ['edit', '--file=' + odd])
        self.assertEqual(self.hf.expand_exec('printf 50%%%f', '/x'), ['printf', '50%/x'])
        self.assertEqual(self.hf.expand_exec('app --title=%c %f', odd, {'Name': 'A %f B'}), ['app', '--title=A %f B', odd])


class Properties(Base):
    def test_props_file_folder_link(self):
        f = os.path.join(self.home, 'docs', 'report.txt')
        write(f, 'hello world')
        os.symlink(f, os.path.join(self.home, 'link to report'))
        i = self.hf.props([f])
        self.assertEqual(i['name'], 'report.txt')
        self.assertEqual(i['size'], 11)
        self.assertFalse(i['dir'])
        self.assertTrue(i['mine'])
        self.assertTrue(i['canRename'])
        self.assertEqual(i['mode'], os.stat(f).st_mode & 0o7777)
        self.assertEqual(i['icon'], 'text-plain')
        self.assertIn('free', i)
        d = self.hf.props([os.path.dirname(f)])
        self.assertTrue(d['dir'])
        self.assertEqual(d['icon'], 'folder')
        ln = self.hf.props([os.path.join(self.home, 'link to report')])
        self.assertTrue(ln['isLink'])
        self.assertEqual(ln['target'], f)
        self.assertFalse(ln['mine'])                         # a link's permissions are its target's

    def test_props_several(self):
        a, b = os.path.join(self.home, 'a.txt'), os.path.join(self.home, 'b.txt')
        write(a, 'aa')
        write(b, 'bbb')
        os.mkdir(os.path.join(self.home, 'sub'))
        r = self.hf.props([a, b, os.path.join(self.home, 'sub')])
        self.assertTrue(r['multi'])
        self.assertEqual((r['count'], r['files'], r['folders']), (3, 2, 1))
        self.assertEqual(r['parent'], self.home)

    def test_image_sizes(self):
        import struct
        import zlib
        png = os.path.join(self.home, 'p.png')
        ihdr = struct.pack('>IIBBBBB', 640, 480, 8, 2, 0, 0, 0)
        write(png, b'\x89PNG\r\n\x1a\n' + struct.pack('>I', 13) + b'IHDR' + ihdr + struct.pack('>I', zlib.crc32(b'IHDR' + ihdr)))
        self.assertEqual(self.hf.image_size(png), (640, 480))
        gif = os.path.join(self.home, 'g.gif')
        write(gif, b'GIF89a' + struct.pack('<HH', 32, 16) + b'\0' * 10)
        self.assertEqual(self.hf.image_size(gif), (32, 16))
        jpg = os.path.join(self.home, 'j.jpg')
        write(jpg, b'\xff\xd8' + b'\xff\xe0' + struct.pack('>H', 16) + b'JFIF\0' + b'\0' * 9
              + b'\xff\xc0' + struct.pack('>HBHH', 17, 8, 300, 400) + b'\0' * 10)
        self.assertEqual(self.hf.image_size(jpg), (400, 300))
        self.assertEqual(self.hf.props([png])['width'], 640)
        self.assertIsNone(self.hf.image_size(os.path.join(self.home, 'nope.png')))

    def test_count_live(self):
        d = os.path.join(self.home, 'big')
        for i in range(30):
            write(os.path.join(d, 'sub%d' % (i % 3), 'f%d' % i), b'x' * 10)
        r = self.run_cli('count', d, env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout)
        final = json.loads(r.stdout.strip().splitlines()[-1])
        self.assertEqual((final['size'], final['files'], final['folders']), (300, 30, 3))
        self.assertTrue(r.stdout.startswith('progress: '))
        self.assertEqual(self.hf.count([d, os.path.join(d, 'sub0', 'f0')])['files'], 31)

    def test_checksums(self):
        import hashlib
        f = os.path.join(self.home, 'data.bin')
        data = os.urandom(5 * 1024 * 1024 + 7)
        write(f, data)
        for algo, h in (('md5', hashlib.md5), ('sha1', hashlib.sha1), ('sha256', hashlib.sha256)):
            self.assertEqual(self.hf.checksum(algo, f)['sum'], h(data).hexdigest())
        self.assertEqual(self.hf.checksum('SHA-256', f)['sum'], hashlib.sha256(data).hexdigest())
        r = self.run_cli('checksum', 'sha1', f, env={'HOME': self.home})
        self.assertEqual(json.loads(r.stdout.strip().splitlines()[-1])['sum'], hashlib.sha1(data).hexdigest())
        with self.assertRaises(self.hf.Fail):
            self.hf.checksum('crc99', f)
        with self.assertRaises(self.hf.Fail):
            self.hf.checksum('md5', self.home)

    def test_chmod(self):
        f = os.path.join(self.home, 'run.sh')
        write(f, '#!/bin/sh\n')
        os.chmod(f, 0o644)
        self.assertEqual(self.hf.chmod(f, '755')['mode'], 0o755)
        self.assertEqual(os.stat(f).st_mode & 0o777, 0o755)
        self.hf.chmod(f, '600')
        self.assertEqual(os.stat(f).st_mode & 0o777, 0o600)
        with self.assertRaises(self.hf.Fail):
            self.hf.chmod(f, '9z')
        with self.assertRaises(self.hf.Fail):
            self.hf.chmod('/usr/bin/env', '777')
        self.assertTrue(self.hf.props([f])['mode'] == 0o600)

    def test_set_default_app(self):
        apps = os.path.join(self.home, '.local', 'share', 'applications')
        write(os.path.join(apps, 'viewer.desktop'), '[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer %F\nMimeType=image/png;\n')
        write(os.path.join(apps, 'painter.desktop'), '[Desktop Entry]\nType=Application\nName=Painter\nExec=painter %u\nMimeType=image/png;\n')
        cfg = os.path.join(self.home, '.config', 'mimeapps.list')
        write(cfg, '[Default Applications]\nimage/png=painter.desktop\ntext/plain=gedit.desktop\n\n[Added Associations]\nimage/png=painter.desktop;\n')
        self.hf.data_dirs = lambda: [os.path.join(self.home, '.local', 'share')]
        pic = os.path.join(self.home, 'x.png')
        write(pic, b'\x89PNG')
        self.hf.set_default(pic, 'viewer.desktop')
        with open(cfg) as f:
            text = f.read()
        self.assertIn('image/png=viewer.desktop;', text)
        self.assertIn('text/plain=gedit.desktop', text)              # the rest is kept
        self.assertIn('image/png=viewer.desktop;painter.desktop;', text)
        self.assertEqual(self.hf.apps(pic)['default'], 'viewer.desktop')
        self.assertEqual(self.hf.props([pic])['app']['name'], 'Viewer')
        with self.assertRaises(self.hf.Fail):
            self.hf.set_default(pic, 'nothing.desktop')


class Conflicts(Base):
    def setUp(self):
        super().setUp()
        self.src = os.path.join(self.home, 'src')
        self.dest = os.path.join(self.home, 'dest')
        write(os.path.join(self.src, 'same.txt'), 'new')
        write(os.path.join(self.src, 'other.txt'), 'other')
        write(os.path.join(self.src, 'Folder', 'a.txt'), 'new a')
        write(os.path.join(self.src, 'Folder', 'b.txt'), 'b')
        write(os.path.join(self.dest, 'same.txt'), 'old')
        write(os.path.join(self.dest, 'Folder', 'a.txt'), 'old a')
        write(os.path.join(self.dest, 'Folder', 'c.txt'), 'c')
        self.items = [os.path.join(self.src, n) for n in ('same.txt', 'other.txt', 'Folder')]

    def read(self, *parts):
        with open(os.path.join(self.dest, *parts)) as f:
            return f.read()

    def test_lists_clashes(self):
        c = self.hf.conflicts(self.items, self.dest)['conflicts']
        self.assertEqual(sorted(x['name'] for x in c), ['Folder', 'same.txt'])
        f = next(x for x in c if x['name'] == 'Folder')
        self.assertTrue(f['srcDir'] and f['targetDir'])
        # Pasting into the folder they're already in is never a clash (it makes "name (2)").
        self.assertEqual(self.hf.conflicts(self.items, self.src)['conflicts'], [])

    def test_keep_both_by_default(self):
        self.hf.transfer(self.items, self.dest, False)
        self.assertEqual(self.read('same.txt'), 'old')
        self.assertEqual(self.read('same (2).txt'), 'new')
        self.assertTrue(os.path.isdir(os.path.join(self.dest, 'Folder (2)')))

    def test_replace_and_merge(self):
        r = self.hf.transfer(self.items, self.dest, False, {'same.txt': 'replace', 'Folder': 'replace'})
        self.assertTrue(r['ok'])
        self.assertEqual(self.read('same.txt'), 'new')
        self.assertEqual(self.read('Folder', 'a.txt'), 'new a')      # replaced inside
        self.assertEqual(self.read('Folder', 'b.txt'), 'b')          # added
        self.assertEqual(self.read('Folder', 'c.txt'), 'c')          # kept
        self.assertFalse(os.path.exists(os.path.join(self.dest, 'same (2).txt')))
        self.assertEqual([n for n in os.listdir(os.path.join(self.dest, 'Folder')) if 'hearth-part' in n], [])

    def test_skip_all(self):
        r = self.hf.transfer(self.items, self.dest, False, {}, 'skip')
        self.assertEqual(len(r['skipped']), 2)
        self.assertEqual(self.read('same.txt'), 'old')
        self.assertEqual(self.read('other.txt'), 'other')
        self.assertFalse(os.path.exists(os.path.join(self.dest, 'Folder', 'b.txt')))

    def test_move_replace(self):
        self.hf.transfer(self.items, self.dest, True, {'same.txt': 'replace', 'Folder': 'replace', 'other.txt': 'skip'})
        self.assertEqual(self.read('same.txt'), 'new')
        self.assertEqual(self.read('Folder', 'a.txt'), 'new a')
        self.assertEqual(self.read('Folder', 'c.txt'), 'c')
        self.assertEqual(os.listdir(self.src), [])                   # moved ones are gone from where they were
        self.assertEqual(self.read('other.txt'), 'other')            # nothing clashed: 'skip' doesn't apply

    def test_cli_decide(self):
        r = self.run_cli('copy', *self.items, self.dest, '--decide', json.dumps({'same.txt': 'skip'}), '--default', 'replace',
                         env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(self.read('same.txt'), 'old')
        self.assertEqual(self.read('Folder', 'a.txt'), 'new a')


class TrashRestore(Base):
    def test_restore_where_it_was_even_if_taken(self):
        f = os.path.join(self.home, 'Docs', 'letter.txt')
        write(f, 'first')
        self.hf.trash([f])
        write(f, 'second')                                      # something new with that name meanwhile
        trashed = os.path.join(self.hf.home_trash(), 'files', 'letter.txt')
        info = os.path.join(self.hf.home_trash(), 'info', 'letter.txt.trashinfo')
        self.assertTrue(os.path.exists(info))
        r = self.hf.restore([trashed])
        self.assertEqual(r['restored'], [os.path.join(self.home, 'Docs', 'letter (2).txt')])
        self.assertFalse(os.path.exists(info))
        with open(f) as fh:
            self.assertEqual(fh.read(), 'second')

    def test_restore_remakes_missing_folder(self):
        d = os.path.join(self.home, 'Gone')
        f = os.path.join(d, 'x.txt')
        write(f, 'x')
        self.hf.trash([f])
        os.rmdir(d)
        self.hf.restore([os.path.join(self.hf.home_trash(), 'files', 'x.txt')])
        self.assertTrue(os.path.exists(f))

    def test_trash_listing_and_empty(self):
        f = os.path.join(self.home, 'a.txt')
        write(f)
        self.hf.trash([f])
        r = self.hf.list_dir(os.path.join(self.hf.home_trash(), 'files'))
        self.assertTrue(r['trash'])
        self.assertEqual(r['entries'][0]['origPath'], f)
        self.hf.empty_trash()
        self.assertEqual(os.listdir(os.path.join(self.hf.home_trash(), 'files')), [])


class Archives(Base):
    def make_tree(self):
        src = os.path.join(self.home, 'Project')
        write(os.path.join(src, 'readme.txt'), 'hello')
        write(os.path.join(src, 'sub', 'data.bin'), os.urandom(3000))
        write(os.path.join(src, 'run.sh'), '#!/bin/sh\n')
        os.chmod(os.path.join(src, 'run.sh'), 0o755)
        return src

    def test_zip_round_trip_here(self):
        src = self.make_tree()
        z = os.path.join(self.home, 'Project.zip')
        r = self.hf.compress('zip', z, [src])
        self.assertTrue(r['ok'] and os.path.isfile(z))
        shutil.rmtree(src)
        r = self.hf.extract(z, '--here', here=True)
        self.assertEqual(r['path'], src)                        # one folder inside: straight in, no extra level
        with open(os.path.join(src, 'readme.txt')) as f:
            self.assertEqual(f.read(), 'hello')
        self.assertEqual(os.stat(os.path.join(src, 'run.sh')).st_mode & 0o111, 0o111)
        r = self.hf.extract(z, '--here', here=True)            # again: the folder's there, so "Project (2)"
        self.assertEqual(r['path'], os.path.join(self.home, 'Project (2)'))

    def test_targz_several_into_named_folder(self):
        src = self.make_tree()
        t = os.path.join(self.home, 'bits.tar.gz')
        self.hf.compress('tar.gz', t, [os.path.join(src, 'readme.txt'), os.path.join(src, 'sub')])
        r = self.hf.extract(t, '--here', here=True)            # several things inside: a folder named after it
        self.assertEqual(r['path'], os.path.join(self.home, 'bits'))
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'bits', 'sub', 'data.bin')))
        out = os.path.join(self.home, 'Elsewhere', 'Deep')
        r = self.hf.extract(t, out)
        self.assertTrue(os.path.isfile(os.path.join(out, 'readme.txt')))

    def test_refuses_paths_outside(self):
        import zipfile
        z = os.path.join(self.home, 'evil.zip')
        with zipfile.ZipFile(z, 'w') as zf:
            zf.writestr('../../escaped.txt', 'no')
            zf.writestr('/abs.txt', 'no')
            zf.writestr('ok.txt', 'yes')
        out = os.path.join(self.home, 'out')
        self.hf.extract(z, out)
        self.assertEqual(sorted(os.listdir(out)), ['ok.txt'])
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'escaped.txt')))

    def test_compress_refuses_existing_and_cli_progress(self):
        src = self.make_tree()
        z = os.path.join(self.home, 'p.zip')
        r = self.run_cli('compress', 'zip', z, src, env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertTrue(r.stdout.startswith('progress: '))
        with self.assertRaises(self.hf.Fail):
            self.hf.compress('zip', z, [src])
        with self.assertRaises(self.hf.Fail):
            self.hf.compress('rar', os.path.join(self.home, 'x.rar'), [src])

    def test_cancel_compress_leaves_nothing(self):
        src = os.path.join(self.home, 'big')
        write(os.path.join(src, 'zeros'), b'\0' * (24 * 1024 * 1024))
        e = dict(os.environ, PATH='/usr/bin:/bin', PYTHONDONTWRITEBYTECODE='1', HEARTH_FILES_SLOW='0.2', HOME=self.home)
        z = os.path.join(self.home, 'big.zip')
        p = subprocess.Popen([sys.executable, str(PATH), 'compress', 'zip', z, src], stdout=subprocess.PIPE, text=True, env=e)
        p.stdout.readline()
        time.sleep(0.3)
        p.send_signal(signal.SIGTERM)
        out, _ = p.communicate(timeout=30)
        self.assertTrue(json.loads(out.strip().splitlines()[-1])['cancelled'])
        self.assertEqual(sorted(os.listdir(self.home)), ['big'])


class Search(Base):
    def test_search_streams_matches(self):
        write(os.path.join(self.home, 'Music', 'Holiday song.mp3'))
        write(os.path.join(self.home, 'Pictures', '2024', 'holiday.jpg'))
        write(os.path.join(self.home, '.hidden', 'holiday.txt'))
        write(os.path.join(self.home, 'other.txt'))
        r = self.run_cli('search', self.home, 'HOLIDAY', env={'HOME': self.home})
        self.assertEqual(r.returncode, 0, r.stdout)
        found = [json.loads(l[7:]) for l in r.stdout.splitlines() if l.startswith('found: ')]
        self.assertEqual(sorted(f['name'] for f in found), ['Holiday song.mp3', 'holiday.jpg'])
        self.assertEqual(next(f for f in found if f['name'] == 'holiday.jpg')['where'], 'Pictures/2024')
        self.assertEqual(json.loads(r.stdout.splitlines()[-1])['count'], 2)
        r = self.run_cli('search', self.home, 'holiday', '--hidden', env={'HOME': self.home})
        self.assertEqual(json.loads(r.stdout.splitlines()[-1])['count'], 3)
        r = self.run_cli('search', self.home, '*.txt', env={'HOME': self.home})
        self.assertEqual(json.loads(r.stdout.splitlines()[-1])['count'], 1)


class Extras(Base):
    def test_bookmarks_in_places(self):
        d = os.path.join(self.home, 'Projects', 'Hearth OS')
        os.makedirs(d)
        self.hf.bookmark_add(d)
        self.hf.bookmark_add(os.path.join(self.home, 'Projects'), 'All projects')
        with open(self.hf.bookmarks_file()) as f:
            self.assertIn('Hearth%20OS', f.read())
        p = [x for x in self.hf.places()['places'] if x['section'] == 'bookmarks']
        self.assertEqual([x['label'] for x in p], ['Hearth OS', 'All projects'])
        self.hf.bookmark_remove(d)
        self.assertEqual([x['path'] for x in self.hf.places()['places'] if x['section'] == 'bookmarks'],
                         [os.path.join(self.home, 'Projects')])
        with self.assertRaises(self.hf.Fail):
            self.hf.bookmark_add(os.path.join(self.home, 'nope'))

    def test_prefs(self):
        self.assertEqual(self.hf.prefs_get(), {})
        self.hf.prefs_set('{"view": "details", "folders": {"/a": {"view": "grid"}}}')
        self.hf.prefs_set('{"hidden": true}')
        self.assertEqual(self.hf.prefs_get(), {'view': 'details', 'folders': {'/a': {'view': 'grid'}}, 'hidden': True})
        with self.assertRaises(self.hf.Fail):
            self.hf.prefs_set('[1]')

    def test_new_file_and_templates(self):
        f = self.hf.new_file(os.path.join(self.home, 'empty.txt'))['path']
        self.assertEqual(os.path.getsize(f), 0)
        with self.assertRaises(self.hf.Fail):
            self.hf.new_file(f)
        write(os.path.join(self.home, 'Templates', 'Letter.odt'), 'template')
        write(os.path.join(self.home, 'Templates', 'Code', 'script.sh'), '#!/bin/sh')
        t = self.hf.templates()['templates']
        self.assertEqual([x['name'] for x in t], ['Letter', 'Code › script'])
        r = self.hf.from_template(t[0]['path'], self.home)
        self.assertEqual(r['path'], os.path.join(self.home, 'Letter.odt'))
        r = self.hf.from_template(t[0]['path'], self.home)
        self.assertEqual(r['path'], os.path.join(self.home, 'Letter (2).odt'))

    def test_batch_rename(self):
        paths = []
        for n in ('b.jpg', 'a.jpg', 'c.png'):
            write(os.path.join(self.home, n))
            paths.append(os.path.join(self.home, n))
        r = self.hf.batch_rename('Holiday ##', paths)
        self.assertEqual([os.path.basename(p) for p in r['paths']], ['Holiday 01.jpg', 'Holiday 02.jpg', 'Holiday 03.png'])
        self.assertEqual(sorted(os.listdir(self.home)), ['Holiday 01.jpg', 'Holiday 02.jpg', 'Holiday 03.png'])
        write(os.path.join(self.home, 'Trip 2.jpg'))
        with self.assertRaises(self.hf.Fail):
            self.hf.batch_rename('Trip #', [os.path.join(self.home, 'Holiday 01.jpg'), os.path.join(self.home, 'Holiday 02.jpg')])

    def test_peek(self):
        t = os.path.join(self.home, 'n.txt')
        write(t, 'line one\nline two')
        self.assertEqual(self.hf.peek(t)['text'], 'line one\nline two')
        b = os.path.join(self.home, 'b.bin')
        write(b, b'\0\1\2')
        with self.assertRaises(self.hf.Fail):
            self.hf.peek(b)


if __name__ == '__main__':
    unittest.main()
