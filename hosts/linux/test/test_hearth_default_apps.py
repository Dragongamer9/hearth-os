"""Tests for Hearth's default apps (desktop/shell/hearth-system): reading and writing mimeapps.list without losing
anything else in it, Settings › Default apps (list, set, reset) against a made-up system, the sign-in tidy, the
image's lists (defaults-system), and the real thing: every Hearth app's .desktop file in the repository (os/system_files,
both bases' build.sh, Hearth Office's install.sh) has a default for every type it opens, with no dangling app ids."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import re
import shlex
import shutil
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
BASES = ['anatase', 'peppermint']


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def read(path):
    with open(path) as f:
        return f.read()


def entry(name, mimes='', extra=''):
    return '[Desktop Entry]\nType=Application\nName=%s\nExec=%s %%U\n%s%s' % (
        name, name.lower().replace(' ', '-'), 'MimeType=%s\n' % mimes if mimes else '', extra)


class MimeappsText(unittest.TestCase):
    """Only the lines for the types being changed change: comments, blank lines, other types and sections stay."""

    def setUp(self):
        self.s = load(SHELL / 'hearth-system', 'hearth_system_defaults_text')

    OLD = ('# my own notes\n[Default Applications]\n# browsers\nx-scheme-handler/https=firefox.desktop;\n'
           'text/plain=gedit.desktop;\nimage/png=old.desktop\n\n[Added Associations]\nimage/png=old.desktop;eog.desktop;\n'
           'application/x-foo=foo.desktop;\n\n[Removed Associations]\nimage/png=eog.desktop;x.desktop;\n\n[Something Else]\nkey=value\n')

    def test_set_keeps_everything_else(self):
        out = self.s.mimeapps_with(self.OLD, ['image/png', 'image/jpeg'], 'eog.desktop')
        self.assertTrue(out.startswith('# my own notes\n[Default Applications]\n# browsers\n'))
        for keep in ('x-scheme-handler/https=firefox.desktop;', 'text/plain=gedit.desktop;', 'application/x-foo=foo.desktop;',
                     '[Something Else]\nkey=value'):
            self.assertIn(keep, out)
        self.assertIn('image/png=eog.desktop;\n', out)
        self.assertIn('image/jpeg=eog.desktop;\n', out)
        self.assertNotIn('image/png=old.desktop\n', out)
        self.assertIn('image/png=eog.desktop;old.desktop;', out)          # added association: the choice first
        self.assertIn('[Removed Associations]\nimage/png=x.desktop;', out)  # no longer "removed"
        self.assertEqual(out.count('[Default Applications]'), 1)
        # The same choice again changes nothing.
        self.assertEqual(self.s.mimeapps_with(out, ['image/png', 'image/jpeg'], 'eog.desktop'), out)

    def test_set_on_nothing(self):
        self.assertEqual(self.s.mimeapps_with('', ['text/plain'], 'a.desktop'),
                         '[Default Applications]\ntext/plain=a.desktop;\n\n[Added Associations]\ntext/plain=a.desktop;\n')

    def test_without(self):
        out = self.s.mimeapps_without(self.OLD, ['image/png', 'x-scheme-handler/https'])
        self.assertNotIn('image/png=old.desktop\n', out)
        self.assertNotIn('firefox', out)
        self.assertIn('# browsers\ntext/plain=gedit.desktop;', out)
        self.assertIn('[Added Associations]\nimage/png=old.desktop;eog.desktop;', out)   # only the defaults go
        self.assertIn('# my own notes', out)
        self.assertEqual(self.s.mimeapps_without('', ['a/b']), '')

    def test_duplicate_lines_and_sections(self):
        text = '[Default Applications]\na/b=x.desktop\na/b=y.desktop\n[Default Applications]\nc/d=z.desktop\n'
        out = self.s.mimeapps_with(text, ['a/b'], 'n.desktop')
        self.assertEqual(out.count('a/b=n.desktop;'), 2)                  # one default line and one added association
        self.assertNotIn('a/b=y.desktop', out)
        self.assertIn('c/d=z.desktop', out)
        self.assertEqual(self.s.ml_read(out, 'c/d'), ['z.desktop'])

    def test_tidy_only_takes_out_hearth_apps_that_are_gone(self):
        text = ('[Default Applications]\ntext/plain=hearth-old-editor.desktop;\nimage/png=hearth-gone.desktop;org.gnome.Loupe.desktop;\n'
                'video/mp4=org.videolan.VLC.desktop;\naudio/mpeg=hearth-player.desktop;\n# kept\n'
                '[Added Associations]\nimage/png=nimbus-viewer.desktop;\n')
        out = self.s.mimeapps_tidy(text, {'hearth-player.desktop', 'org.gnome.Loupe.desktop'})
        self.assertNotIn('text/plain', out)                                # no app left: the line goes
        self.assertIn('image/png=org.gnome.Loupe.desktop;', out)
        self.assertIn('video/mp4=org.videolan.VLC.desktop;', out)         # not Hearth's: never touched, even not installed
        self.assertIn('audio/mpeg=hearth-player.desktop;', out)
        self.assertIn('# kept', out)
        self.assertNotIn('nimbus-viewer', out)

    def test_merge_system_list(self):
        base = '[Default Applications]\nimage/jpeg=org.gnome.Loupe.desktop\napplication/x-foo=foo.desktop\ntext/plain=hearth-gone.desktop;\n'
        out = self.s.mimeapps_merge(base, {'image/jpeg': 'hearth-photos.desktop'}, {'hearth-photos.desktop', 'foo.desktop'})
        self.assertIn('image/jpeg=hearth-photos.desktop;', out)
        self.assertNotIn('Loupe', out)
        self.assertIn('application/x-foo=foo.desktop', out)
        self.assertNotIn('hearth-gone', out)
        self.assertTrue(out.startswith('# Hearth'))
        self.assertEqual(self.s.mimeapps_merge(out, {'image/jpeg': 'hearth-photos.desktop'}, {'hearth-photos.desktop', 'foo.desktop'}), out)


class Settings(unittest.TestCase):
    """Settings › Default apps against a made-up system: Hearth's apps and the base's (Firefox, Loupe, Kate, foot)."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.data = os.path.join(self.tmp, 'usr')
        self.etc = os.path.join(self.tmp, 'etc')
        self.home = os.path.join(self.tmp, 'home')
        apps = os.path.join(self.data, 'applications')
        write(os.path.join(apps, 'hearth-photos.desktop'), entry('Photos', 'image/jpeg;image/png;image/heic;'))
        write(os.path.join(apps, 'hearth-paint.desktop'), entry('Paint', 'image/png;image/jpeg;'))
        write(os.path.join(apps, 'org.gnome.Loupe.desktop'), entry('Image Viewer', 'image/jpeg;image/png;'))
        write(os.path.join(apps, 'hearth-text.desktop'), entry('Text Editor', 'text/plain;text/x-python;'))
        write(os.path.join(apps, 'org.kde.kate.desktop'), entry('Kate', 'text/plain;'))
        write(os.path.join(apps, 'hearth-browser.desktop'), entry('Hearth Browser', 'text/html;x-scheme-handler/http;x-scheme-handler/https;'))
        write(os.path.join(apps, 'firefox.desktop'), entry('Firefox', 'text/html;x-scheme-handler/http;x-scheme-handler/https;'))
        write(os.path.join(apps, 'hearth-terminal.desktop'), entry('Terminal', 'x-scheme-handler/terminal;', 'Categories=System;TerminalEmulator;\n'))
        write(os.path.join(apps, 'foot.desktop'), entry('Foot', '', 'Categories=System;TerminalEmulator;\n'))
        write(os.path.join(apps, 'hearth-files.desktop'), entry('Files', 'inode/directory;application/zip;', 'OnlyShowIn=Nimbus;\n'))
        write(os.path.join(apps, 'hidden.desktop'), entry('Hidden', 'image/png;', 'NoDisplay=true\n'))
        write(os.path.join(apps, 'broken.desktop'), entry('Broken', 'image/png;', 'TryExec=/nonexistent/broken\n'))
        self.env = mock.patch.dict(os.environ, {
            'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'), 'XDG_DATA_HOME': os.path.join(self.home, '.local/share'),
            'XDG_DATA_DIRS': self.data, 'XDG_CONFIG_DIRS': os.path.join(self.etc, 'xdg'),
            'XDG_CURRENT_DESKTOP': 'Nimbus', 'PATH': '/nonexistent'})
        self.env.start()
        self.addCleanup(self.env.stop)
        self.s = load(SHELL / 'hearth-system', 'hearth_system_defaults_settings')
        self.hf = self.s.load_files_helper()
        # The image's lists, as defaults-system makes them.
        hearth = self.s.hearth_apps(apps, self.hf)
        nimbus, generic = self.s.system_defaults(hearth)
        write(os.path.join(self.etc, 'xdg', 'nimbus-mimeapps.list'), self.s.mimeapps_merge('', nimbus))
        write(os.path.join(self.etc, 'xdg', 'nimbus-xdg-terminals.list'), 'hearth-terminal.desktop\nfoot.desktop\n')

    def kinds(self):
        return {k['kind']: k for k in self.s.defaults(self.hf)['kinds']}

    def user_list(self):
        return read(os.path.join(self.home, '.config', 'mimeapps.list'))

    def test_hearth_apps_are_the_defaults(self):
        k = self.kinds()
        for kind, app in (('image', 'hearth-photos.desktop'), ('text', 'hearth-text.desktop'), ('browser', 'hearth-browser.desktop'),
                          ('terminal', 'hearth-terminal.desktop'), ('files', 'hearth-files.desktop'), ('archives', 'hearth-files.desktop')):
            self.assertEqual(k[kind]['current'], app, kind)
            self.assertTrue(k[kind]['isHearth'], kind)
            self.assertFalse(k[kind]['yours'], kind)
        self.assertEqual([o['id'] for o in k['image']['options']],
                         ['hearth-photos.desktop', 'org.gnome.Loupe.desktop', 'hearth-paint.desktop'])   # Hearth's default, then by name
        self.assertTrue(all(o['hearth'] == (o['id'] == 'hearth-photos.desktop') for o in k['image']['options']))
        self.assertEqual([o['id'] for o in k['terminal']['options']], ['hearth-terminal.desktop', 'foot.desktop'])
        self.assertIsNone(k['email']['current'])
        self.assertEqual(k['email']['options'], [])
        self.assertIsNone(k['email']['hearth'])
        self.assertEqual(k['music']['hearth'], None)                       # Hearth Player isn't in this made-up system

    def test_pick_and_reset(self):
        write(os.path.join(self.home, '.config', 'mimeapps.list'), '# mine\n[Default Applications]\napplication/x-foo=foo.desktop;\n')
        write(os.path.join(self.home, '.config', 'nimbus-mimeapps.list'), '[Default Applications]\nimage/png=hearth-paint.desktop;\nvideo/mp4=vlc.desktop;\n')
        r = self.s.default_set('image', 'org.gnome.Loupe.desktop', self.hf)
        self.assertEqual(r['types'], ['image/jpeg', 'image/png'])         # what Loupe opens: HEIC stays with Photos
        k = self.kinds()['image']
        self.assertEqual((k['current'], k['yours'], k['isHearth']), ('org.gnome.Loupe.desktop', True, False))
        text = self.user_list()
        self.assertIn('# mine', text)
        self.assertIn('application/x-foo=foo.desktop;', text)
        self.assertIn('image/jpeg=org.gnome.Loupe.desktop;', text)
        self.assertNotIn('image/heic', text)
        desk = read(os.path.join(self.home, '.config', 'nimbus-mimeapps.list'))
        self.assertNotIn('image/png', desk)                                 # it came before ~/.config/mimeapps.list
        self.assertIn('video/mp4=vlc.desktop;', desk)
        self.s.default_reset('image', self.hf)
        k = self.kinds()['image']
        self.assertEqual((k['current'], k['yours']), ('hearth-photos.desktop', False))
        self.assertIn('application/x-foo=foo.desktop;', self.user_list())
        self.assertNotIn('image/jpeg=', self.user_list().split('[Added Associations]')[0])

    def test_every_type_for_text_and_the_browser(self):
        r = self.s.default_set('text', 'org.kde.kate.desktop', self.hf)
        self.assertIn('text/x-python', r['types'])                         # all text, not only what Kate lists
        self.assertIn('application/json', r['types'])
        self.s.default_set('browser', 'firefox.desktop', self.hf)
        k = self.kinds()
        self.assertEqual(k['text']['current'], 'org.kde.kate.desktop')
        self.assertEqual(k['browser']['current'], 'firefox.desktop')
        self.assertIn('application/xhtml+xml=firefox.desktop;', self.user_list())
        self.s.default_reset(None, self.hf)
        k = self.kinds()
        self.assertEqual(k['text']['current'], 'hearth-text.desktop')
        self.assertEqual(k['browser']['current'], 'hearth-browser.desktop')
        self.assertFalse(any(x['yours'] for x in k.values()))

    def test_terminal(self):
        self.s.default_set('terminal', 'foot.desktop', self.hf)
        mine = os.path.join(self.home, '.config', 'nimbus-xdg-terminals.list')
        self.assertEqual(read(mine), 'foot.desktop\n')
        self.assertIn('x-scheme-handler/terminal=foot.desktop;', self.user_list())
        k = self.kinds()['terminal']
        self.assertEqual((k['current'], k['yours']), ('foot.desktop', True))
        self.s.default_reset('terminal', self.hf)
        self.assertFalse(os.path.exists(mine))
        self.assertEqual(self.kinds()['terminal']['current'], 'hearth-terminal.desktop')

    def test_refused(self):
        for kind, app in (('image', 'hidden.desktop.x'), ('image', '../x.desktop'), ('image', 'missing.desktop'),
                          ('image', 'broken.desktop'), ('nope', 'org.gnome.Loupe.desktop'), ('image', '')):
            with self.assertRaises(self.s.Fail, msg=(kind, app)):
                self.s.default_set(kind, app, self.hf)
        with self.assertRaises(self.s.Fail):
            self.s.default_reset('nope', self.hf)

    def test_tidy_at_sign_in(self):
        write(os.path.join(self.home, '.config', 'mimeapps.list'),
              '[Default Applications]\nimage/jpeg=hearth-aloha.desktop;\ntext/plain=org.kde.kate.desktop;\n'
              'x-scheme-handler/https=hearth-browser.desktop;\n')
        write(os.path.join(self.home, '.config', 'nimbus-xdg-terminals.list'), 'hearth-old-terminal.desktop\n')
        r = self.s.defaults_tidy(self.hf)
        self.assertEqual(len(r['changed']), 2)
        text = self.user_list()
        self.assertNotIn('hearth-aloha', text)                              # an older Hearth's, not installed any more
        self.assertIn('text/plain=org.kde.kate.desktop;', text)             # your own choice stays
        self.assertIn('x-scheme-handler/https=hearth-browser.desktop;', text)
        self.assertEqual(self.kinds()['image']['current'], 'hearth-photos.desktop')
        self.assertEqual(self.kinds()['terminal']['current'], 'hearth-terminal.desktop')
        self.assertEqual(self.s.defaults_tidy(self.hf)['changed'], [])


class SystemLists(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.s = load(SHELL / 'hearth-system', 'hearth_system_defaults_system')
        self.hf = self.s.load_files_helper()

    def test_rank_and_desktops(self):
        apps = os.path.join(self.tmp, 'usr/share/applications')
        write(os.path.join(apps, 'hearth-photos.desktop'), entry('Photos', 'image/png;image/heic;'))
        write(os.path.join(apps, 'hearth-paint.desktop'), entry('Paint', 'image/png;image/bmp;'))
        write(os.path.join(apps, 'hearth-text.desktop'), entry('Text', 'text/plain;text/markdown;text/csv;'))
        write(os.path.join(apps, 'hearth-write.desktop'), entry('Write', 'text/plain;text/markdown;application/rtf;'))
        write(os.path.join(apps, 'hearth-sheets.desktop'), entry('Sheets', 'text/csv;'))
        write(os.path.join(apps, 'hearth-files.desktop'), entry('Files', 'inode/directory;application/zip;', 'OnlyShowIn=Nimbus;\n'))
        write(os.path.join(apps, 'hearth-hidden.desktop'), entry('Gone', 'image/png;', 'Hidden=true\n'))
        write(os.path.join(apps, 'org.gnome.Loupe.desktop'), entry('Loupe', 'image/png;'))
        write(os.path.join(apps, 'mimeapps.list'), '[Default Applications]\nimage/png=org.gnome.Loupe.desktop;\nx/base=base.desktop;\n')
        write(os.path.join(self.tmp, 'etc/xdg/nimbus-mimeapps.list'), '[Default Applications]\ntext/plain=hearth-old.desktop;\n')
        r = self.s.defaults_system(self.tmp, self.hf)
        self.assertNotIn('hearth-hidden.desktop', r['apps'])
        nim = read(os.path.join(self.tmp, 'etc/xdg/nimbus-mimeapps.list'))
        gen = read(os.path.join(apps, 'mimeapps.list'))
        want = {'image/png': 'hearth-photos.desktop', 'image/bmp': 'hearth-paint.desktop', 'text/plain': 'hearth-text.desktop',
                'text/markdown': 'hearth-text.desktop', 'text/csv': 'hearth-sheets.desktop', 'application/rtf': 'hearth-write.desktop',
                'inode/directory': 'hearth-files.desktop', 'application/zip': 'hearth-files.desktop'}
        for m, d in want.items():
            self.assertEqual(self.s.ml_read(nim, m), [d], m)
        self.assertNotIn('hearth-old', nim)
        self.assertEqual(self.s.ml_read(gen, 'image/png'), ['hearth-photos.desktop'])
        self.assertEqual(self.s.ml_read(gen, 'inode/directory'), ['hearth-files.desktop'])   # folders: every desktop too
        self.assertEqual(self.s.ml_read(gen, 'application/zip'), [])         # archives only in Hearth's desktop
        self.assertEqual(self.s.ml_read(gen, 'x/base'), ['base.desktop'])
        self.assertEqual(nim.count('image/png='), 1)


class FilesOpensArchives(unittest.TestCase):
    """Hearth Files is the default app for archives: opening one in it offers to unpack it (open-plan kind archive),
    unless you picked another app for them."""

    def test_open_plan(self):
        hf = load(SHELL / 'hearth-files', 'hearth_files_archives')
        tmp = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, tmp)
        z = os.path.join(tmp, 'photos.zip')
        with open(z, 'wb') as f:
            f.write(b'PK\x03\x04' + b'\0' * 40)
        hf.sniff_mime = lambda p: 'application/zip'
        for app, kind in ((None, 'archive'), ({'id': 'hearth-files.desktop'}, 'archive'), ({'id': 'org.kde.ark.desktop'}, 'default')):
            hf.default_app = lambda m, app=app: app
            self.assertEqual(hf.open_plan(z)['kind'], kind, app)
        cbz = os.path.join(tmp, 'comic.cbz')
        shutil.copy(z, cbz)
        hf.sniff_mime = lambda p: 'application/vnd.comicbook+zip'
        hf.default_app = lambda m: {'id': 'hearth-documents.desktop'}
        self.assertEqual(hf.open_plan(cbz)['kind'], 'default')


# ---------------------------------------------------------------------- the repository's own Hearth apps
HEREDOC = re.compile(r'cat > "?(?:\$D)?/usr/share/applications/(hearth-[A-Za-z0-9-]+)\.desktop"? <<\'?DESKTOP\'?\n(.*?)\nDESKTOP\n', re.S)
ASSIGN = re.compile(r'^\s*([A-Z_][A-Z0-9_]*)=(?:"([^"]*)"|\'([^\']*)\')\s*$', re.M)


def expand(text, variables):
    return re.sub(r'\$\{([A-Z_][A-Z0-9_]*)\}|\$([A-Z_][A-Z0-9_]*)', lambda m: variables.get(m.group(1) or m.group(2), ''), text)


def repo_desktop_entries(base):
    """{id: (MimeType, OnlyShowIn)} for every Hearth app an image of this base gets."""
    out = {}
    for f in sorted((ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'applications').glob('hearth-*.desktop')):
        fields = dict(l.split('=', 1) for l in f.read_text().split('\n[', 1)[0].splitlines() if '=' in l and not l.startswith('#'))
        out[f.name] = (fields.get('MimeType', ''), fields.get('OnlyShowIn', ''))
    for script in (ROOT / 'os-bases' / base / 'build.sh', ROOT / 'hosts' / 'hearth-office' / 'linux' / 'install.sh'):
        text = script.read_text()
        variables = {m.group(1): m.group(2) if m.group(2) is not None else m.group(3) for m in ASSIGN.finditer(text)}
        for m in HEREDOC.finditer(text):
            body = expand(m.group(2), variables)
            fields = dict(l.split('=', 1) for l in body.split('\n[', 1)[0].splitlines() if '=' in l)
            out[m.group(1) + '.desktop'] = (fields.get('MimeType', ''), fields.get('OnlyShowIn', ''))
        for line in text.splitlines():
            line = line.strip()
            if line.startswith('hearth_time_entry ') and '()' not in line:
                args = shlex.split(line)[1:]
                out['hearth-%s.desktop' % args[0]] = (args[5] if len(args) > 5 else '', '')
    return out


class RepositoryApps(unittest.TestCase):
    def setUp(self):
        self.s = load(SHELL / 'hearth-system', 'hearth_system_defaults_repo')
        self.hf = self.s.load_files_helper()

    def test_found_the_apps(self):
        for base in BASES:
            got = repo_desktop_entries(base)
            for d in ('hearth-text.desktop', 'hearth-player.desktop', 'hearth-photos.desktop', 'hearth-documents.desktop',
                      'hearth-browser.desktop', 'hearth-files.desktop', 'hearth-write.desktop', 'hearth-sheets.desktop',
                      'hearth-slides.desktop', 'hearth-fonts.desktop', 'hearth-contacts.desktop', 'hearth-maps.desktop',
                      'hearth-calendar.desktop', 'hearth-terminal.desktop', 'hearth-paint.desktop'):
                self.assertIn(d, got, (base, d))
            self.assertIn('text/calendar', got['hearth-calendar.desktop'][0])
            self.assertIn('application/zip', got['hearth-files.desktop'][0])

    def test_every_type_has_a_hearth_default(self):
        for base in BASES:
            with self.subTest(base=base):
                tmp = tempfile.mkdtemp()
                self.addCleanup(shutil.rmtree, tmp)
                apps = os.path.join(tmp, 'usr/share/applications')
                entries = repo_desktop_entries(base)
                for did, (mimes, only) in entries.items():
                    write(os.path.join(apps, did), entry(did[:-8], mimes, 'OnlyShowIn=%s\n' % only if only else ''))
                self.s.defaults_system(tmp, self.hf)
                nim = read(os.path.join(tmp, 'etc/xdg/nimbus-mimeapps.list'))
                gen = read(os.path.join(apps, 'mimeapps.list'))
                for did, (mimes, only) in entries.items():
                    for m in [t for t in mimes.split(';') if t and '*' not in t]:
                        got = self.s.ml_read(nim, m)
                        self.assertEqual(len(got), 1, (base, did, m))
                        self.assertTrue(got[0].startswith('hearth-'), (base, did, m, got))
                        if not only or m in self.s.EVERY_DESKTOP_TOO:
                            self.assertEqual(len(self.s.ml_read(gen, m)), 1, (base, did, m))
                # No dangling ids: every app in the lists is one of the image's.
                for text in (nim, gen):
                    for line in text.splitlines():
                        if '=' in line and not line.startswith('#'):
                            for did in [x for x in line.split('=', 1)[1].split(';') if x]:
                                self.assertIn(did, entries, (base, line))
                # Settings › Default apps' Hearth app for each kind is in the image and the default for its main type.
                for c in self.s.CATEGORIES:
                    if c['hearth']:
                        self.assertIn(c['hearth'], entries, (base, c['kind']))
                        self.assertEqual(self.s.ml_read(nim, c['primary']), [c['hearth']], (base, c['kind']))
                # The choices Hearth makes between its own apps.
                for m, d in (('text/plain', 'hearth-text.desktop'), ('text/markdown', 'hearth-text.desktop'),
                             ('text/csv', 'hearth-sheets.desktop'), ('image/png', 'hearth-photos.desktop'),
                             ('application/epub+zip', 'hearth-documents.desktop')):
                    self.assertEqual(self.s.ml_read(nim, m), [d], (base, m))

    def test_build_scripts_only_write_the_lists_through_hearth_system(self):
        for base in BASES:
            text = (ROOT / 'os-bases' / base / 'build.sh').read_text()
            for line in text.splitlines():
                if 'mimeapps.list' in line and not line.lstrip().startswith('#'):
                    self.assertTrue('hearth-system' in line or line.lstrip().startswith('echo'), (base, line))
            self.assertIn('hearth-system defaults-system', text)
        self.assertNotIn('mimeapps.list"', (ROOT / 'hosts' / 'hearth-office' / 'linux' / 'install.sh').read_text())


if __name__ == '__main__':
    unittest.main()
