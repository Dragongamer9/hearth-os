"""Tests for desktop/shell/hearth-portal: what an app's OpenFile / SaveFile / SaveFiles call (xdg-desktop-portal's
FileChooser) becomes for the picker, what the picker's answer becomes for the app, which mode/display the window opens
on, and the D-Bus interfaces it offers (FileChooser and FileManager1)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest
import xml.etree.ElementTree as ET

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-portal'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_portal', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_portal', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hp = load()
TABLES = ({'image/png': ['*.png'], 'image/jpeg': ['*.jpg', '*.jpeg'], 'text/plain': ['*.txt'],
           'text/x-python': ['*.py'], 'text/markdown': ['*.md']},
          {'text/plain': {'text/x-python', 'text/markdown'}})


class Request(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.docs = os.path.join(self.home, 'Documents')
        os.makedirs(self.docs)

    def tearDown(self):
        self.tmp.cleanup()

    def req(self, method='OpenFile', options=None, title='', remembered=None, app='org.example.App'):
        return hp.build_request(method, app, 'x11:1a', title, options or {}, tables=TABLES, home=self.home,
                                remembered=remembered)

    def test_open_defaults(self):
        r = self.req()
        self.assertEqual((r['kind'], r['title'], r['accept'], r['multiple'], r['directory']), ('open', 'Open File', 'Open', False, False))
        self.assertEqual((r['folder'], r['filters'], r['filter'], r['name'], r['files']), (self.home, [], -1, '', []))
        self.assertEqual((r['app'], r['parent']), ('org.example.App', 'x11:1a'))

    def test_open_options(self):
        folder = self.docs.encode() + b'\0'
        r = self.req(options={'multiple': True, 'accept_label': '_Upload', 'current_folder': folder, 'modal': True}, title='Upload photos')
        self.assertEqual((r['title'], r['accept'], r['multiple'], r['folder']), ('Upload photos', 'Upload', True, self.docs))
        # ay can also arrive as a list of numbers.
        r = self.req(options={'current_folder': list(folder)})
        self.assertEqual(r['folder'], self.docs)
        # A folder that isn't there: the one remembered for this app, else Home.
        r = self.req(options={'current_folder': b'/nowhere/at/all\0'}, remembered={'org.example.App': self.docs})
        self.assertEqual(r['folder'], self.docs)
        r = self.req(options={'current_folder': b'/nowhere/at/all\0'}, remembered={'other': self.docs})
        self.assertEqual(r['folder'], self.home)

    def test_directory_mode(self):
        r = self.req(options={'directory': True, 'filters': [('Images', [(0, '*.png')])]})
        self.assertEqual((r['directory'], r['title'], r['accept'], r['filters'], r['filter']), (True, 'Choose a Folder', 'Choose', [], -1))

    def test_filters(self):
        filters = [('Pictures', [(1, 'image/png'), (1, 'image/jpeg'), (0, '*.webp')]), ('Text', [(1, 'text/plain')]), ('All', [(0, '*')])]
        r = self.req(options={'filters': filters})
        self.assertEqual(r['filter'], 0)
        self.assertEqual(r['filters'][0], {'name': 'Pictures', 'globs': ['*.png', '*.jpg', '*.jpeg', '*.webp'], 'mimes': ['image/png', 'image/jpeg']})
        # Kinds of text/plain count as text, as in GTK's filters.
        self.assertEqual(sorted(r['filters'][1]['globs']), ['*.md', '*.py', '*.txt'])
        self.assertEqual(r['filters'][2]['globs'], ['*'])
        # current_filter picks one of them...
        r = self.req(options={'filters': filters, 'current_filter': ('Text', [(1, 'text/plain')])})
        self.assertEqual(r['filter'], 1)
        # ...or is added when it isn't one of them.
        r = self.req(options={'filters': filters, 'current_filter': ('Python', [(0, '*.py')])})
        self.assertEqual((r['filter'], len(r['filters']), r['filters'][3]['name']), (3, 4, 'Python'))
        self.assertEqual(r['raw'][3], ['Python', [[0, '*.py']]])
        # Only a current filter: that's the filter.
        r = self.req(options={'current_filter': ('Python', [(0, '*.py')])})
        self.assertEqual((r['filter'], [f['name'] for f in r['filters']]), (0, ['Python']))
        # Wildcard types.
        r = self.req(options={'filters': [('Any picture', [(1, 'image/*')])]})
        self.assertEqual(sorted(r['filters'][0]['globs']), ['*.jpeg', '*.jpg', '*.png'])

    def test_save(self):
        r = self.req('SaveFile', {'current_name': 'Report.odt', 'current_folder': self.docs.encode() + b'\0'})
        self.assertEqual((r['kind'], r['title'], r['accept'], r['name'], r['folder']), ('save', 'Save File', 'Save', 'Report.odt', self.docs))
        # Saving a file that's already somewhere: its folder and its name.
        existing = os.path.join(self.docs, 'notes.txt')
        r = self.req('SaveFile', {'current_file': existing.encode() + b'\0', 'current_name': 'ignored.txt'})
        self.assertEqual((r['folder'], r['name']), (self.docs, 'notes.txt'))
        self.assertFalse(r['multiple'])

    def test_save_files(self):
        r = self.req('SaveFiles', {'files': [b'/tmp/a.png\0', b'b.png\0'], 'current_folder': self.docs.encode() + b'\0'})
        self.assertEqual((r['kind'], r['files'], r['folder'], r['name'], r['accept']), ('savefiles', ['a.png', 'b.png'], self.docs, '', 'Save Here'))

    def test_choices(self):
        r = self.req(options={'choices': [('encoding', 'Encoding', [('utf8', 'Unicode'), ('latin1', 'Western')], 'latin1'),
                                          ('reencode', 'Re-encode', [], 'true'),
                                          ('other', 'Other', [('a', 'A'), ('b', 'B')], ''),
                                          ('flag', 'Flag', [], '')]})
        self.assertEqual(r['choices'][0], {'id': 'encoding', 'label': 'Encoding', 'options': [['utf8', 'Unicode'], ['latin1', 'Western']], 'value': 'latin1'})
        self.assertEqual(r['choices'][1]['value'], 'true')
        self.assertEqual(r['choices'][2]['value'], 'a')          # nothing chosen: the first option
        self.assertEqual(r['choices'][3]['value'], 'false')      # a checkbox without a value: off

    def test_labels(self):
        self.assertEqual(hp.plain_label('_Open'), 'Open')
        self.assertEqual(hp.plain_label('Save __as'), 'Save _as')
        self.assertEqual(hp.plain_label(''), '')

    def test_paths(self):
        self.assertEqual(hp.decode_path(b'/a b/c\0'), '/a b/c')
        self.assertEqual(hp.decode_path([47, 120, 0]), '/x')
        self.assertEqual(hp.decode_path(None), '')
        self.assertEqual(hp.decode_path('/s\0'), '/s')


class Results(unittest.TestCase):
    def req(self, method='OpenFile', options=None):
        return hp.build_request(method, 'app', '', '', options or {}, tables=TABLES, home='/tmp')

    def test_cancel_and_failure(self):
        r = self.req()
        self.assertEqual(hp.build_results(r, {'cancel': True}), (1, {}))
        self.assertEqual(hp.build_results(r, {'paths': []}), (1, {}))
        self.assertEqual(hp.build_results(r, None), (2, {}))
        self.assertEqual(hp.build_results(r, {'paths': ['relative.txt']}), (1, {}))

    def test_open(self):
        r = self.req()
        code, out = hp.build_results(r, {'paths': ['/home/me/My File #1.txt', '/home/me/b.txt'], 'filter': -1, 'choices': {}})
        self.assertEqual(code, 0)
        self.assertEqual(out, {'uris': ('as', ['file:///home/me/My%20File%20%231.txt'])})   # one file unless multiple
        r = self.req(options={'multiple': True})
        code, out = hp.build_results(r, {'paths': ['/a/ü.png', '/a/b.png']})
        self.assertEqual(out['uris'], ('as', ['file:///a/%C3%BC.png', 'file:///a/b.png']))

    def test_filter_and_choices(self):
        filters = [('Pictures', [(1, 'image/png')]), ('All', [(0, '*')])]
        r = self.req(options={'filters': filters, 'choices': [('enc', 'Encoding', [('u', 'U'), ('l', 'L')], 'u'), ('chk', 'Check', [], 'false')]})
        code, out = hp.build_results(r, {'paths': ['/x/a.png'], 'filter': 1, 'choices': {'enc': 'l'}})
        self.assertEqual(code, 0)
        self.assertEqual(out['current_filter'], ('(sa(us))', ('All', [(0, '*')])))
        self.assertEqual(out['choices'], ('a(ss)', [('enc', 'l'), ('chk', 'false')]))
        # An out-of-range filter is left out.
        code, out = hp.build_results(r, {'paths': ['/x/a.png'], 'filter': 7})
        self.assertNotIn('current_filter', out)

    def test_save_files(self):
        r = self.req('SaveFiles', {'files': [b'a.png\0', b'b c.png\0']})
        code, out = hp.build_results(r, {'paths': ['/home/me/Pictures']})
        self.assertEqual((code, out['uris']), (0, ('as', ['file:///home/me/Pictures/a.png', 'file:///home/me/Pictures/b%20c.png'])))

    def test_save(self):
        r = self.req('SaveFile', {'current_name': 'x.txt'})
        code, out = hp.build_results(r, {'paths': ['/home/me/x.txt', '/home/me/y.txt']})
        self.assertEqual(out['uris'], ('as', ['file:///home/me/x.txt']))

    def test_remember_folder(self):
        r = self.req()
        self.assertEqual(hp.remember_folder(r, {'paths': ['/a/b/c.txt']}, {'x': '/q'}), {'x': '/q', 'app': '/a/b'})
        r2 = self.req(options={'directory': True})
        self.assertEqual(hp.remember_folder(r2, {'paths': ['/a/b']}, {}), {'app': '/a/b'})
        self.assertEqual(hp.remember_folder(r, {'cancel': True}, {'x': '/q'}), {'x': '/q'})
        many = {str(i): '/f' for i in range(150)}
        self.assertEqual(len(hp.remember_folder(r, {'paths': ['/z/1']}, many)), 100)


class Mode(unittest.TestCase):
    def test_parse_env(self):
        e = hp.parse_env("DISPLAY=:1\nWAYLAND_DISPLAY=\nXDG_CURRENT_DESKTOP=Nimbus:wlroots\nODD=$'a\\nb'\nbad line\n")
        self.assertEqual(e, {'DISPLAY': ':1', 'WAYLAND_DISPLAY': '', 'XDG_CURRENT_DESKTOP': 'Nimbus:wlroots', 'ODD': 'a\nb'})

    def test_game_mode(self):
        env, game = hp.session_env({'PATH': '/bin', 'WAYLAND_DISPLAY': 'wayland-1', 'QT_QPA_PLATFORMTHEME': 'xdgdesktopportal'},
                                   'DISPLAY=:0\nWAYLAND_DISPLAY=\nXDG_CURRENT_DESKTOP=Nimbus\n')
        self.assertTrue(game)
        self.assertEqual((env['DISPLAY'], env['QT_QPA_PLATFORM']), (':0', 'xcb'))
        self.assertNotIn('WAYLAND_DISPLAY', env)
        self.assertNotIn('QT_QPA_PLATFORMTHEME', env)    # the picker never asks the portal for its own dialogs

    def test_desktop_mode(self):
        env, game = hp.session_env({'PATH': '/bin', 'DISPLAY': ':5'}, 'DISPLAY=:1\nWAYLAND_DISPLAY=wayland-1\n')
        self.assertFalse(game)
        self.assertEqual((env['WAYLAND_DISPLAY'], env['DISPLAY'], env['QT_QPA_PLATFORM']), ('wayland-1', ':1', 'wayland;xcb'))

    def test_no_activation_env(self):
        # Without systemd's environment (or with no display in it), the process's own is used.
        env, game = hp.session_env({'WAYLAND_DISPLAY': 'wayland-0'}, '')
        self.assertFalse(game)
        self.assertEqual(env['WAYLAND_DISPLAY'], 'wayland-0')

    def test_in_hearth(self):
        self.assertTrue(hp.in_hearth({'XDG_CURRENT_DESKTOP': 'Nimbus:wlroots'}))
        self.assertTrue(hp.in_hearth({'XDG_CURRENT_DESKTOP': 'Nimbus'}))
        self.assertFalse(hp.in_hearth({'XDG_CURRENT_DESKTOP': 'KDE'}))
        self.assertFalse(hp.in_hearth({}))

    def test_other_file_managers(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(hp.other_file_managers(d)[0], ['dolphin', d])
            f = os.path.join(d, 'a.txt')
            self.assertEqual(hp.other_file_managers(f)[0], ['dolphin', '--select', f])

    def test_ids(self):
        self.assertEqual(hp.parse_ids('123\n456\n'), [123, 456])
        self.assertEqual(hp.parse_ids('Defaulting to search window name\n'), [])


class MimeTables(unittest.TestCase):
    def test_reads_shared_mime_info(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, 'mime'))
            with open(os.path.join(d, 'mime', 'globs2'), 'w') as f:
                f.write('# comment\n50:text/plain:*.txt\n50:text/x-python:*.py\n50:image/png:*.png:cs\n')
            with open(os.path.join(d, 'mime', 'subclasses'), 'w') as f:
                f.write('text/x-python text/plain\n')
            tables = hp.mime_tables([d])
            self.assertEqual(tables[0]['image/png'], ['*.png'])
            self.assertEqual(hp.mime_globs('text/plain', tables), ['*.txt', '*.py'])
            self.assertEqual(hp.mime_globs('application/none', tables), [])


class FileManager1(unittest.TestCase):
    def test_uri_path(self):
        self.assertEqual(hp.uri_path('file:///home/me/My%20Files'), '/home/me/My Files')
        self.assertEqual(hp.uri_path('file://localhost/tmp/x'), '/tmp/x')
        self.assertEqual(hp.uri_path('/plain/path'), '/plain/path')
        self.assertIsNone(hp.uri_path('https://example.com/x'))
        self.assertIsNone(hp.uri_path('file://otherhost/x'))

    def test_targets(self):
        self.assertEqual(hp.fm_targets('ShowFolders', ['file:///a', 'file:///b']), [('/a', False)])
        self.assertEqual(hp.fm_targets('ShowItems', ['file:///a/b.txt']), [('/a/b.txt', False)])
        self.assertEqual(hp.fm_targets('ShowItemProperties', ['file:///a/b.txt']), [('/a/b.txt', True)])
        self.assertEqual(hp.fm_targets('ShowItems', ['https://x']), [])


class Interfaces(unittest.TestCase):
    def methods(self, xml, iface):
        root = ET.fromstring(xml)
        i = next(x for x in root.findall('interface') if x.get('name') == iface)
        return {m.get('name'): [(a.get('type'), a.get('direction')) for a in m.findall('arg')] for m in i.findall('method')}, i

    def test_filechooser(self):
        ms, iface = self.methods(hp.FILECHOOSER_XML, 'org.freedesktop.impl.portal.FileChooser')
        sig = [('o', 'in'), ('s', 'in'), ('s', 'in'), ('s', 'in'), ('a{sv}', 'in'), ('u', 'out'), ('a{sv}', 'out')]
        self.assertEqual(ms, {'OpenFile': sig, 'SaveFile': sig, 'SaveFiles': sig})
        self.assertEqual([(p.get('name'), p.get('type')) for p in iface.findall('property')], [('version', 'u')])
        req, _ = self.methods(hp.FILECHOOSER_XML, 'org.freedesktop.impl.portal.Request')
        self.assertEqual(req, {'Close': []})
        self.assertGreaterEqual(hp.FC_VERSION, 3)      # xdg-desktop-portal only passes "directory" to version 3+

    def test_filemanager1(self):
        ms, _ = self.methods(hp.FILEMANAGER_XML, 'org.freedesktop.FileManager1')
        self.assertEqual(ms, {m: [('as', 'in'), ('s', 'in')] for m in ('ShowFolders', 'ShowItems', 'ShowItemProperties')})

    def test_installed_files_agree(self):
        root = PATH.parent.parent.parent.parent.parent / 'os' / 'system_files' / 'usr' / 'share'
        portal = (root / 'xdg-desktop-portal' / 'portals' / 'hearth.portal').read_text()
        self.assertIn('DBusName=%s' % hp.BUS_NAME, portal)
        self.assertIn('Interfaces=org.freedesktop.impl.portal.FileChooser;', portal)
        svc = (root / 'dbus-1' / 'services' / (hp.BUS_NAME + '.service')).read_text()
        self.assertIn('Name=%s' % hp.BUS_NAME, svc)
        self.assertIn('hearth-portal filechooser', svc)
        fm = (root / 'dbus-1' / 'services' / 'org.freedesktop.FileManager1.hearth.service').read_text()
        self.assertIn('Name=org.freedesktop.FileManager1', fm)
        self.assertIn('hearth-portal filemanager1', fm)
        self.assertTrue((PATH.parent / 'picker.qml').is_file())


if __name__ == '__main__':
    unittest.main()
