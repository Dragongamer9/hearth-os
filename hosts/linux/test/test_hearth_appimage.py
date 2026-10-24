"""Tests for hearth-files' AppImage support: telling an AppImage by its bytes, the menu entry it writes (Exec quoted
the Desktop Entry way, pointing at the AppImage in ~/Applications), adding and removing it from the menu, and asking
before making a new one runnable."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import shutil
import stat
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-files'
TYPE2 = b'\x7fELF\x02\x01\x01\x00AI\x02' + b'\x00' * 100


def load(home):
    os.environ['HOME'] = home
    for k in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_DIRS'):
        os.environ.pop(k, None)
    loader = importlib.machinery.SourceFileLoader('hearth_files_ai', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_files_ai', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def unquote_exec(value):
    """Reads an Exec value back the way the spec says: the value's escapes first, then the quoting."""
    out, i = '', 0
    while i < len(value):
        if value[i] == '\\' and i + 1 < len(value):
            out += {'s': ' ', 'n': '\n', 't': '\t', 'r': '\r', '\\': '\\'}.get(value[i + 1], '\\' + value[i + 1])
            i += 2
        else:
            out += value[i]
            i += 1
    # The spec's quoting: inside "...", a backslash escapes " ` $ and itself; words split on spaces outside quotes.
    words, cur, quoted, i, started = [], '', False, 0, False
    while i < len(out):
        c = out[i]
        if quoted and c == '\\' and i + 1 < len(out) and out[i + 1] in '"`$\\':
            cur += out[i + 1]
            i += 2
            continue
        if c == '"':
            quoted, started = not quoted, True
        elif c == ' ' and not quoted:
            if cur or started:
                words.append(cur)
            cur, started = '', False
        else:
            cur += c
        i += 1
    if cur or started:
        words.append(cur)
    return [w.replace('%%', '%') for w in words]


class AppImages(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-appimage-test-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(self.home, 'Downloads'))
        self.f = load(self.home)
        self.f.fuse2_available = lambda: True

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake(self, name='Cool App-1.2-x86_64.AppImage', data=TYPE2, mode=0o644):
        p = os.path.join(self.home, 'Downloads', name)
        with open(p, 'wb') as fh:
            fh.write(data)
        os.chmod(p, mode)
        return p

    def test_type_by_bytes(self):
        self.assertEqual(self.f.appimage_type(self.fake()), 2)
        self.assertEqual(self.f.appimage_type(self.fake('a.AppImage', b'\x7fELF\x02\x01\x01\x00AI\x01xx')), 1)
        self.assertEqual(self.f.appimage_type(self.fake('fake.AppImage', b'#!/bin/sh\necho hi\n')), 0)
        self.assertEqual(self.f.appimage_type(os.path.join(self.home, 'nope')), 0)

    def test_exec_quoting(self):
        for path in ('/home/me/Applications/Simple.AppImage', '/home/me/Applications/My App.AppImage',
                     '/home/me/Apps/a"b$c`d\\e.AppImage', '/home/me/100%/x.AppImage', "/home/me/it's.AppImage"):
            self.assertEqual(unquote_exec(self.f.exec_quote(path)), [path], path)
        self.assertEqual(self.f.exec_quote('/a/b.AppImage'), '/a/b.AppImage')
        self.assertEqual(self.f.exec_quote('/a/My App.AppImage'), '"/a/My App.AppImage"')

    def test_entry(self):
        text = self.f.build_appimage_entry('/home/me/Applications/My App.AppImage',
                                           {'Name': 'My App', 'Exec': 'AppRun --flag %U', 'Comment': 'Does\nthings',
                                            'Categories': 'Utility;', 'Actions': 'new;', 'TryExec': 'AppRun'},
                                           '/home/me/.local/share/icons/hearth-appimages/x.png')
        lines = dict(l.split('=', 1) for l in text.splitlines()[1:])
        self.assertEqual(text.splitlines()[0], '[Desktop Entry]')
        self.assertEqual(lines['Name'], 'My App')
        self.assertEqual(unquote_exec(lines['Exec']), ['/home/me/Applications/My App.AppImage', '%U'])
        self.assertEqual(lines['TryExec'], '/home/me/Applications/My App.AppImage')
        self.assertEqual(lines['Comment'], 'Does things')
        self.assertEqual(lines['X-Hearth-AppImage'], '/home/me/Applications/My App.AppImage')
        self.assertNotIn('Actions', lines)
        no_fuse = self.f.build_appimage_entry('/a/b.AppImage', {}, '', fuse_ok=False)
        self.assertIn('Exec=env APPIMAGE_EXTRACT_AND_RUN=1 /a/b.AppImage\n', no_fuse)
        self.assertIn('Name=b\n', no_fuse)
        self.assertIn('Icon=application-x-executable\n', no_fuse)

    def test_ids(self):
        a = self.f.appimage_id('/x/Cool App-1.2-x86_64.AppImage')
        self.assertRegex(a, r'^appimage-Cool-App-1\.2-[0-9a-f]{8}$')
        self.assertNotEqual(a, self.f.appimage_id('/y/Cool App-1.2-x86_64.AppImage'))

    def test_asks_before_making_it_runnable(self):
        p = self.fake()
        with self.assertRaises(self.f.Fail) as e:
            self.f.appimage_check(p, False)
        self.assertTrue(str(e.exception).startswith('confirm:'))
        self.f.appimage_check(p, True)
        self.assertTrue(os.stat(p).st_mode & stat.S_IXUSR)
        with self.assertRaises(self.f.Fail):
            self.f.appimage_check(self.fake('fake.AppImage', b'not one'), True)

    def test_add_and_remove(self):
        p = self.fake()
        icon = os.path.join(self.tmp, 'icon.png')
        with open(icon, 'wb') as fh:
            fh.write(b'\x89PNG\r\n\x1a\n' + b'0' * 20)
        parts_tmp = tempfile.mkdtemp(dir=self.tmp)
        self.f.appimage_parts = lambda app: ({'Name': 'Cool App', 'Exec': 'cool %F', 'Categories': 'Game;'}, icon, parts_tmp)
        r = self.f.appimage_menu_add(p, True)
        dest = os.path.join(os.path.realpath(self.home), 'Applications', 'Cool App-1.2-x86_64.AppImage')
        self.assertEqual(r['path'], dest)
        self.assertTrue(r['moved'])
        self.assertFalse(os.path.exists(p))                 # moved, not copied
        self.assertTrue(os.access(dest, os.X_OK))
        self.assertFalse(os.path.exists(parts_tmp))         # the temporary folder is cleaned up
        entry = pathlib.Path(r['desktop']).read_text()
        self.assertTrue(r['desktop'].startswith(os.path.join(self.home, '.local', 'share', 'applications', 'appimage-Cool-App-1.2-')))
        self.assertIn('Name=Cool App\n', entry)
        self.assertIn('Categories=Game;\n', entry)
        self.assertIn('Exec="%s" %%F\n' % dest, entry)
        self.assertTrue(os.path.exists(r['icon']) and r['icon'].endswith('.png'))
        self.assertIn(dest, self.f.appimage_menu_entries())
        listing = self.f.list_dir(os.path.dirname(dest))
        e = next(x for x in listing['entries'] if x['name'].endswith('.AppImage'))
        self.assertTrue(e['appimage'] and e['inMenu'])
        # Added again: the same entry, not a second one.
        self.f.appimage_parts = lambda app: ({'Name': 'Cool App'}, '', tempfile.mkdtemp(dir=self.tmp))
        self.f.appimage_menu_add(dest)
        self.assertEqual(len(self.f.appimage_menu_entries()), 1)
        self.assertEqual(self.f.appimage_menu_remove(dest)['removed'], 1)
        self.assertEqual(self.f.appimage_menu_entries(), {})
        self.assertTrue(os.path.exists(dest))              # the AppImage stays
        with self.assertRaises(self.f.Fail):
            self.f.appimage_menu_remove(dest)

    def test_info(self):
        p = self.fake()
        i = self.f.appimage_info(p)
        self.assertEqual((i['appimage'], i['type'], i['executable'], i['inMenu']), (True, 2, False, False))


if __name__ == '__main__':
    unittest.main()
