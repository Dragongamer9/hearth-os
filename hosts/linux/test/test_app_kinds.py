"""Tests for one app of each kind per desktop (os/system_files/usr/libexec/hearth-app-kinds and its table
os/system_files/usr/share/hearth/app-kinds.json, docs/APP-KINDS.md): the table, the menu-entry copies, each desktop's
default apps, and the check that stops the image build."""
import copy
import importlib.machinery
import importlib.util
import json
import os
import shutil
import tempfile
import unittest

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
TOOL = os.path.join(REPO, 'os/system_files/usr/libexec/hearth-app-kinds')
TABLE = os.path.join(REPO, 'os/system_files/usr/share/hearth/app-kinds.json')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_app_kinds', TOOL)
    spec = importlib.util.spec_from_loader('hearth_app_kinds', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ak = load()


def entry(name, cats='', mimes='', tryexec='', extra=''):
    out = '[Desktop Entry]\nType=Application\nName=%s\nExec=%s %%U\n' % (name, name.lower().replace(' ', '-'))
    if cats:
        out += 'Categories=%s;\n' % cats
    if mimes:
        out += 'MimeType=%s;\n' % mimes
    if tryexec:
        out += 'TryExec=%s\n' % tryexec
    return out + extra


TEXT = 'text/plain;text/markdown;application/json'
PDF = 'application/pdf;application/epub+zip'
IMAGES = 'image/jpeg;image/png'
MEDIA = 'video/mp4;audio/mpeg;audio/flac'
APPS = {
    'hearth-files.desktop': entry('Files', 'System;FileManager', 'inode/directory;x-directory/normal', extra='OnlyShowIn=Nimbus;\n'),
    'hearth-files-kde.desktop': entry('Files', 'Qt;KDE;FileManager', 'inode/directory;x-directory/normal', '/usr/lib/hearth-files/bin/dolphin'),
    'org.kde.dolphin.desktop': entry('Files', 'Qt;KDE;FileManager', 'inode/directory', 'dolphin', 'NoDisplay=true\n'),
    'hearth-terminal.desktop': entry('Terminal', 'System;TerminalEmulator', 'x-scheme-handler/terminal', 'hearth-terminal'),
    'hearth-terminal-kde.desktop': entry('Terminal', 'KDE;TerminalEmulator', 'x-scheme-handler/terminal', 'konsole'),
    'org.kde.konsole.desktop': entry('Konsole', 'KDE;TerminalEmulator', '', 'konsole', 'NoDisplay=true\n'),
    'footclient.desktop': entry('Foot Client', 'System;TerminalEmulator'),
    'hearth-text.desktop': entry('Text Editor', 'Utility;TextEditor', TEXT, 'hearth-text'),
    'hearth-text-kde.desktop': entry('Text Editor', 'KDE;TextEditor', TEXT + ';text/csv', 'kate'),
    'org.kde.kwrite.desktop': entry('KWrite', 'KDE;TextEditor', TEXT, 'kwrite'),
    'hearth-calculator.desktop': entry('Calculator', 'Utility;Calculator', tryexec='hearth-calculator'),
    'hearth-calculator-kde.desktop': entry('Calculator', 'KDE;Utility;Calculator', tryexec='kcalc'),
    'hearth-documents.desktop': entry('Documents', 'Office;Viewer', PDF, '/usr/bin/hearth-documents'),
    'hearth-reader.desktop': entry('Reader', 'KDE;Office;Viewer', PDF, 'okular'),
    'hearth-capture.desktop': entry('Capture', 'KDE;Utility', tryexec='spectacle',
                                    extra='Actions=area;\n\n[Desktop Action area]\nName=Capture an area\nExec=spectacle --region\n'),
    'hearth-tasks.desktop': entry('Task Manager', 'System;Monitor', extra='OnlyShowIn=Nimbus;\n'),
    'hearth-photos.desktop': entry('Photos', 'Graphics;Viewer', IMAGES, '/usr/bin/hearth-photos'),
    'org.kde.gwenview.desktop': entry('Gwenview', 'KDE;Graphics;Viewer', IMAGES),
    'hearth-player.desktop': entry('Hearth Player', 'AudioVideo;Player', MEDIA, 'hearth-player'),
    'org.kde.elisa.desktop': entry('Elisa', 'KDE;AudioVideo;Player', 'audio/mpeg'),
    'hearth-paint.desktop': entry('Paint', 'Graphics', 'image/png'),
    'org.example.Files.desktop': entry('Other Files', 'System;FileManager', 'inode/directory'),
    'hearth-settings.desktop': entry('Hearth Settings', 'Settings'),
}
PROGRAMS = ['usr/bin/labwc', 'usr/bin/startplasma-wayland', 'usr/lib/hearth-files/bin/dolphin', 'usr/bin/dolphin',
            'usr/bin/hearth-terminal', 'usr/bin/konsole', 'usr/bin/hearth-text', 'usr/bin/kate', 'usr/bin/kwrite',
            'usr/bin/hearth-calculator', 'usr/bin/kcalc', 'usr/bin/hearth-documents', 'usr/bin/okular',
            'usr/bin/spectacle', 'usr/bin/hearth-photos', 'usr/bin/hearth-player']


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(text)


def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


class Base(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()
        self.table = ak.load_table(TABLE)
        for did, text in APPS.items():
            write(os.path.join(self.root, 'usr/share/applications', did), text)
        for p in PROGRAMS:
            write(os.path.join(self.root, p), '')
        write(os.path.join(self.root, 'etc/xdg/nimbus-mimeapps.list'),
              "# Hearth's default apps (hearth-system defaults-system)\n[Default Applications]\n"
              "text/plain=hearth-text-kde.desktop;\napplication/vnd.debian.binary-package=hearth-install-package.desktop;\n")

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def apply(self, table=None, refresh=False):
        return ak.apply(self.root, refresh=refresh, table=table or self.table)

    def shown(self, desktop, names=None):
        eff = ak.effective(self.root)
        names = names or self.table['desktops'][desktop]['names']
        return {i for i, f in eff.items() if ak.usable(f, self.root) and ak.shown_in(f, names)}

    def default(self, desktop, mime):
        return ak.mimeapps_lookup(self.root, self.table['desktops'][desktop]['names'], mime,
                                  ak.effective(self.root))


class TableTest(unittest.TestCase):
    def test_real_table_is_valid(self):
        t = ak.load_table(TABLE)
        self.assertEqual(ak.validate(t), [])
        self.assertEqual(set(t['desktops']), {'Nimbus', 'KDE', 'GNOME', 'COSMIC', 'Deepin'})
        self.assertEqual(t['desktops']['Nimbus']['names'], ['Nimbus'])   # what nimbus-session sets
        self.assertEqual(t['desktops']['KDE']['names'], ['KDE'])

    def test_session_sets_these_names(self):
        s = read(os.path.join(REPO, 'hosts/linux/session/nimbus-session'))
        self.assertIn('XDG_CURRENT_DESKTOP=Nimbus', s)
        self.assertIn('XDG_CURRENT_DESKTOP=KDE', s)
        self.assertIn('/usr/share/hearth/app-kinds', s)              # the copies come first in XDG_DATA_DIRS

    def test_mistakes_are_found(self):
        t = ak.load_table(TABLE)
        bad = copy.deepcopy(t)
        bad['kinds']['music']['mimes'] = ['audio/*', 'text/plain']    # text/plain is the text editor's
        bad['kinds']['files']['apps']['Plasma'] = ['x.desktop']        # no such desktop
        bad['kinds']['calculator']['others'] = ['not an id']
        errs = ak.validate(bad)
        self.assertTrue(any('text-editor' in e and 'music' in e for e in errs), errs)
        self.assertTrue(any('Plasma' in e for e in errs), errs)
        self.assertTrue(any('not an id' in e for e in errs), errs)

    def test_patterns(self):
        self.assertTrue(ak.mime_match('image/*', 'image/png'))
        self.assertFalse(ak.mime_match('image/*', 'video/mp4'))
        self.assertTrue(ak.mime_match('text/plain', 'text/plain'))


class ApplyTest(Base):
    def test_each_desktop_shows_one_of_each_kind(self):
        rep = self.apply()
        self.assertTrue(rep['ok'], rep['problems'])
        nim, kde, gnome = self.shown('Nimbus'), self.shown('KDE'), self.shown('GNOME')
        # Hearth Classic: Hearth's own (classic) apps.
        for i in ('hearth-files.desktop', 'hearth-terminal.desktop', 'hearth-text.desktop', 'hearth-calculator.desktop',
                  'hearth-documents.desktop', 'hearth-tasks.desktop'):
            self.assertIn(i, nim)
        for i in ('hearth-files-kde.desktop', 'hearth-terminal-kde.desktop', 'hearth-text-kde.desktop', 'hearth-reader.desktop',
                  'hearth-capture.desktop', 'hearth-calculator-kde.desktop', 'org.kde.kwrite.desktop', 'footclient.desktop',
                  'org.example.Files.desktop'):
            self.assertNotIn(i, nim)
        # Hearth Plasma: the KDE-based ones, stock KDE duplicates hidden.
        for i in ('hearth-files-kde.desktop', 'hearth-terminal-kde.desktop', 'hearth-text-kde.desktop', 'hearth-reader.desktop',
                  'hearth-capture.desktop', 'hearth-calculator-kde.desktop'):
            self.assertIn(i, kde)
        for i in ('hearth-files.desktop', 'hearth-terminal.desktop', 'hearth-text.desktop', 'hearth-documents.desktop',
                  'hearth-calculator.desktop', 'org.kde.dolphin.desktop', 'org.kde.konsole.desktop', 'org.kde.kwrite.desktop',
                  'org.kde.gwenview.desktop', 'org.kde.elisa.desktop', 'hearth-tasks.desktop', 'org.example.Files.desktop'):
            self.assertNotIn(i, kde)
        # Apps with no other of their kind: everywhere.
        for d in (nim, kde, gnome):
            for i in ('hearth-photos.desktop', 'hearth-player.desktop', 'hearth-paint.desktop', 'hearth-settings.desktop'):
                self.assertIn(i, d)
        # A desktop that hasn't registered its file manager yet keeps the one it has.
        self.assertIn('org.example.Files.desktop', gnome)
        self.assertNotIn('hearth-files.desktop', gnome)
        self.assertEqual(rep['desktops']['Nimbus']['apps']['files'], 'hearth-files.desktop')
        self.assertEqual(rep['desktops']['KDE']['apps']['text-editor'], 'hearth-text-kde.desktop')

    def test_package_files_are_not_edited(self):
        self.apply()
        for did, text in APPS.items():
            self.assertEqual(read(os.path.join(self.root, 'usr/share/applications', did)), text)
        copy_ = read(os.path.join(self.root, 'usr/share/hearth/app-kinds/applications/hearth-capture.desktop'))
        self.assertIn('OnlyShowIn=KDE;', copy_)
        self.assertIn('[Desktop Action area]', copy_)                # actions kept
        self.assertIn('Exec=spectacle --region', copy_)
        f = ak.parse_desktop(read(os.path.join(self.root, 'usr/share/hearth/app-kinds/applications/hearth-photos.desktop')))
        self.assertEqual(f.get('OnlyShowIn'), None)                  # everywhere: unknown desktops (XFCE...) see it too
        g = ak.parse_desktop(read(os.path.join(self.root, 'usr/share/hearth/app-kinds/applications/org.kde.gwenview.desktop')))
        self.assertEqual(set(g['NotShowIn'].strip(';').split(';')), {'Nimbus', 'KDE', 'GNOME', 'COSMIC', 'Deepin', 'DDE'})

    def test_default_apps_follow_the_desktop(self):
        self.apply()
        self.assertEqual(self.default('Nimbus', 'text/plain'), 'hearth-text.desktop')
        self.assertEqual(self.default('KDE', 'text/plain'), 'hearth-text-kde.desktop')
        self.assertEqual(self.default('Nimbus', 'inode/directory'), 'hearth-files.desktop')
        self.assertEqual(self.default('KDE', 'inode/directory'), 'hearth-files-kde.desktop')
        self.assertEqual(self.default('Nimbus', 'application/pdf'), 'hearth-documents.desktop')
        self.assertEqual(self.default('KDE', 'application/pdf'), 'hearth-reader.desktop')
        self.assertEqual(self.default('KDE', 'x-scheme-handler/terminal'), 'hearth-terminal-kde.desktop')
        self.assertEqual(self.default('KDE', 'image/png'), 'hearth-photos.desktop')
        self.assertEqual(self.default('Deepin', 'audio/flac'), 'hearth-player.desktop')
        nim = read(os.path.join(self.root, 'etc/xdg/nimbus-mimeapps.list'))
        self.assertIn("# Hearth's default apps (hearth-system defaults-system)", nim)     # what was there stays
        self.assertIn('application/vnd.debian.binary-package=hearth-install-package.desktop;', nim)
        self.assertEqual(nim.count('text/plain='), 1)
        self.assertTrue(os.path.exists(os.path.join(self.root, 'etc/xdg/dde-mimeapps.list')))
        self.assertEqual(read(os.path.join(self.root, 'etc/xdg/kde-xdg-terminals.list')).splitlines()[-1],
                         'hearth-terminal-kde.desktop')

    def test_runs_twice_the_same(self):
        self.apply()
        a = {p: read(os.path.join(self.root, 'etc/xdg', p)) for p in os.listdir(os.path.join(self.root, 'etc/xdg'))}
        self.apply()
        b = {p: read(os.path.join(self.root, 'etc/xdg', p)) for p in os.listdir(os.path.join(self.root, 'etc/xdg'))}
        self.assertEqual(a, b)

    def test_fallback_when_hearth_files_kde_did_not_build(self):
        os.unlink(os.path.join(self.root, 'usr/lib/hearth-files/bin/dolphin'))
        rep = self.apply()
        self.assertTrue(rep['ok'], rep['problems'])
        kde = self.shown('KDE')
        self.assertIn('org.kde.dolphin.desktop', kde)                # KDE's own entry shown instead (NoDisplay taken out)
        self.assertNotIn('hearth-files-kde.desktop', kde)
        self.assertEqual(self.default('KDE', 'inode/directory'), 'org.kde.dolphin.desktop')

    def test_none_where_one_is_needed_fails(self):
        for p in ('usr/bin/hearth-text', 'usr/bin/kate', 'usr/bin/kwrite'):
            os.unlink(os.path.join(self.root, p))
        rep = self.apply()
        self.assertFalse(rep['ok'])
        self.assertTrue(any('Nimbus shows no text-editor' in p for p in rep['problems']), rep['problems'])

    def test_a_desktop_not_installed_needs_nothing(self):
        os.unlink(os.path.join(self.root, 'usr/bin/startplasma-wayland'))
        for p in ('usr/bin/kate', 'usr/bin/hearth-text'):
            os.unlink(os.path.join(self.root, p))
        rep = self.apply()
        self.assertFalse(any(p.startswith('KDE ') for p in rep['problems']), rep['problems'])

    def test_two_shown_fails_the_check(self):
        self.apply()
        # Something put the classic Text Editor back into Plasma: the check (the image check) says so.
        write(os.path.join(self.root, 'usr/share/hearth/app-kinds/applications/hearth-text.desktop'), APPS['hearth-text.desktop'])
        rep = ak.check(self.root, self.table)
        self.assertFalse(rep['ok'])
        self.assertTrue(any('KDE shows 2 apps of kind text-editor' in p for p in rep['problems']), rep['problems'])

    def test_main_exit_codes(self):
        shutil.copy(TABLE, os.path.join(self.root, 'tab.json'))
        os.makedirs(os.path.join(self.root, 'usr/share/hearth'), exist_ok=True)
        shutil.copy(TABLE, os.path.join(self.root, 'usr/share/hearth/app-kinds.json'))
        self.assertEqual(ak.main(['x', 'apply', '--root', self.root]), 0)
        self.assertEqual(ak.main(['x', 'check', '--root', self.root]), 0)
        for p in ('usr/bin/hearth-text', 'usr/bin/kate', 'usr/bin/kwrite'):
            os.unlink(os.path.join(self.root, p))
        self.assertEqual(ak.main(['x', 'apply', '--root', self.root]), 1)
        self.assertEqual(ak.main(['x', 'apply', '--refresh', '--root', self.root]), 0)   # the apt hook never fails apt

    def test_hidden_lists_for_the_launcher(self):
        self.apply()
        old = os.environ.pop('XDG_DATA_DIRS', None)
        try:
            hid = ak.hidden(['Nimbus', 'wlroots'], self.root)
        finally:
            if old is not None:
                os.environ['XDG_DATA_DIRS'] = old
        self.assertIn('hearth-files-kde.desktop', hid)
        self.assertIn('org.kde.kwrite.desktop', hid)
        self.assertNotIn('hearth-files.desktop', hid)
        self.assertNotIn('hearth-photos.desktop', hid)

    def test_refresh_leaves_apps_you_install_alone(self):
        self.apply()
        write(os.path.join(self.root, 'usr/share/applications/thunar.desktop'), entry('Thunar', 'System;FileManager', 'inode/directory'))
        self.apply(refresh=True)
        self.assertIn('thunar.desktop', self.shown('Nimbus'))
        self.assertNotIn('hearth-files-kde.desktop', self.shown('Nimbus'))   # the copies are still written

    def test_a_desktop_registers_its_apps(self):
        """What docs/APP-KINDS.md tells a new desktop to do: its native apps in each kind's "apps"."""
        t = copy.deepcopy(self.table)
        t['kinds']['files']['apps']['GNOME'] = ['org.example.Files.desktop']
        t['kinds']['image-viewer']['apps']['GNOME'] = ['org.gnome.Loupe.desktop']
        write(os.path.join(self.root, 'usr/share/applications/org.gnome.Loupe.desktop'), entry('Image Viewer', 'Graphics;Viewer', IMAGES))
        write(os.path.join(self.root, 'usr/bin/gnome-shell'), '')
        rep = self.apply(t)
        self.assertTrue(rep['ok'], rep['problems'])
        gnome, nim, kde = self.shown('GNOME'), self.shown('Nimbus'), self.shown('KDE')
        self.assertIn('org.gnome.Loupe.desktop', gnome)
        self.assertNotIn('hearth-photos.desktop', gnome)              # the native one replaces Hearth's there
        self.assertIn('hearth-photos.desktop', nim)
        self.assertIn('hearth-photos.desktop', kde)
        self.assertNotIn('org.gnome.Loupe.desktop', nim)
        self.assertNotIn('org.gnome.Loupe.desktop', kde)
        self.assertIn('org.example.Files.desktop', gnome)
        self.assertNotIn('org.example.Files.desktop', kde)
        self.assertEqual(self.default('GNOME', 'image/png'), 'org.gnome.Loupe.desktop')
        self.assertEqual(self.default('GNOME', 'inode/directory'), 'org.example.Files.desktop')
        self.assertEqual(self.default('KDE', 'image/png'), 'hearth-photos.desktop')

    def test_with_keys(self):
        t = '[Desktop Entry]\nName=A\nOnlyShowIn=Nimbus;\nNoDisplay=true\n\n[Desktop Action x]\nName=X\nOnlyShowIn=keep\n'
        out = ak.with_keys(t, {'OnlyShowIn': 'KDE;', 'NoDisplay': None, 'NotShowIn': None})
        self.assertEqual(out, '[Desktop Entry]\nName=A\nOnlyShowIn=KDE;\n\n[Desktop Action x]\nName=X\nOnlyShowIn=keep\n')


if __name__ == '__main__':
    unittest.main()
