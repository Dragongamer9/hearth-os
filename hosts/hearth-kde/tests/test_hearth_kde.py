"""Hearth's KDE apps, the image-side pieces (hosts/hearth-kde): menu entries, the old apps' "(classic)" names, Hearth
Storage in KDE's Places, the hearth-files-kde starter and install.sh's list. Run: python3 -m unittest discover -s
hosts/hearth-kde/tests"""
import configparser
import importlib.util
import json
import os
import pathlib
import stat
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

KDE = pathlib.Path(__file__).resolve().parent.parent
REPO = KDE.parent.parent
spec = importlib.util.spec_from_file_location('hearth_kde', KDE / 'hearth_kde.py')
hk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hk)

SERVER = '0123456789abcdef' * 4
STOCK_DOLPHIN = """[Desktop Entry]
Type=Application
Exec=dolphin %u
Icon=system-file-manager
Terminal=false
InitialPreference=10
Name=Dolphin
Name[de]=Dolphin
GenericName=File Manager
GenericName[de]=Dateiverwaltung
MimeType=inode/directory;
Categories=Qt;KDE;System;FileTools;FileManager;
Actions=new-window;

[Desktop Action new-window]
Name=Open a New Window
Name[de]=Neues Fenster
Exec=dolphin --new-window
"""


def parse(text):
    cp = configparser.RawConfigParser(strict=False, interpolation=None)
    cp.optionxform = str
    cp.read_string(text)
    return cp


class MenuEntries(unittest.TestCase):
    def test_rebrand_keeps_kde_entry_for_app_id(self):
        out = hk.rebrand_text(STOCK_DOLPHIN, 'Files', 'hearth-files', '/usr/bin/hearth-files-kde', 'dolphin')
        cp = parse(out)
        e = cp['Desktop Entry']
        self.assertEqual(e['Name'], 'Files')
        self.assertEqual(e['Icon'], 'hearth-files')
        self.assertEqual(e['NoDisplay'], 'true')
        self.assertNotIn('Name[de]', e)
        self.assertNotIn('GenericName[de]', e)
        self.assertEqual(e['GenericName'], 'File Manager')
        self.assertEqual(e['Exec'], '/usr/bin/hearth-files-kde %u')
        self.assertEqual(e['MimeType'], 'inode/directory;')
        a = cp['Desktop Action new-window']
        self.assertEqual(a['Exec'], '/usr/bin/hearth-files-kde --new-window')
        self.assertEqual(a['Name[de]'], 'Neues Fenster')   # only the main entry's names are Hearth's
        # Again: the same.
        self.assertEqual(hk.rebrand_text(out, 'Files', 'hearth-files', '/usr/bin/hearth-files-kde', 'hearth-files-kde').count('NoDisplay'), 1)

    def test_rebrand_without_exec_leaves_program(self):
        out = hk.rebrand_text(STOCK_DOLPHIN.replace('dolphin', 'konsole'), 'Terminal', 'hearth-terminal')
        self.assertIn('Exec=konsole %u', out)

    def test_classic(self):
        text = '[Desktop Entry]\nName=Terminal\nExec=hearth-terminal\n\n[Desktop Action new-window]\nName=New window\n'
        out = hk.classic_text(text)
        self.assertIn('Name=Terminal (classic)', out)
        self.assertIn('Name=New window\n', out)
        self.assertEqual(hk.classic_text(out), out)

    def test_hearth_entries(self):
        for f in sorted((KDE / 'apps').glob('*.desktop')):
            e = parse(f.read_text())['Desktop Entry']
            self.assertEqual(e['Type'], 'Application', f.name)
            self.assertTrue(f.name.startswith('hearth-'), f.name)    # hearth-system only ranks hearth-* entries
            self.assertTrue(e['Icon'].startswith('hearth-'), f.name)
            self.assertTrue((REPO / 'branding/icons/apps' / (e['Icon'] + '.svg')).exists(), e['Icon'])
            self.assertIn('X-Hearth-Based-On', e, f.name)              # KDE credited: which KDE app it is
        term = parse((KDE / 'apps/hearth-terminal-kde.desktop').read_text())['Desktop Entry']
        self.assertIn('x-scheme-handler/terminal', term['MimeType'])
        self.assertIn('TerminalEmulator', term['Categories'])
        # Kate opens what Hearth Text opens, so it can take over every type.
        old = parse((REPO / 'os/system_files/usr/share/applications/hearth-text.desktop').read_text())['Desktop Entry']
        new = parse((KDE / 'apps/hearth-text-kde.desktop').read_text())['Desktop Entry']
        self.assertEqual(set(old['MimeType'].split(';')), set(new['MimeType'].split(';')))
        files = parse((KDE / 'apps/hearth-files-kde.desktop').read_text())['Desktop Entry']
        self.assertEqual(files['StartupWMClass'], 'org.kde.dolphin')
        self.assertEqual(files['TryExec'], '/usr/lib/hearth-files/bin/dolphin')

    def test_install_list_matches_entries(self):
        text = (KDE / 'install.sh').read_text()
        block = text.split("<<'LIST'\n", 1)[1].split('\nLIST', 1)[0]
        rows = [l.split() for l in block.splitlines() if l.strip()]
        self.assertEqual(len(rows), 6)
        for entry, prog, stock, name, icon, old in rows:
            self.assertTrue((KDE / 'apps' / entry).exists(), entry)
            self.assertTrue(stock.startswith('org.kde.'), stock)
            self.assertEqual(parse((KDE / 'apps' / entry).read_text())['Desktop Entry']['Icon'], icon)
            if old != '-':
                self.assertTrue(old.startswith('hearth-'))

    def test_hearth_system_ranks_kde_apps(self):
        sys_src = (REPO / 'hosts/linux/desktop/shell/hearth-system').read_text()
        rank = sys_src.split('HEARTH_RANK = [', 1)[1].split(']', 1)[0]
        ids = [x.strip().strip("'") for x in rank.replace('\n', ' ').split(',') if x.strip()]
        # The KDE-based terminal and text editor first; Hearth Files (Dolphin) after the old one in Hearth Classic's
        # list (Game Mode and Classic keep their built-in Files), first in every other desktop's (it has no OnlyShowIn).
        self.assertLess(ids.index('hearth-terminal-kde.desktop'), ids.index('hearth-terminal.desktop'))
        self.assertLess(ids.index('hearth-text-kde.desktop'), ids.index('hearth-text.desktop'))
        self.assertGreater(ids.index('hearth-files-kde.desktop'), ids.index('hearth-files.desktop'))
        self.assertLess(ids.index('hearth-documents.desktop'), ids.index('hearth-reader.desktop'))


class Places(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.status = {'servers': [{'id': SERVER, 'name': 'Den PC', 'shares': [{'id': 'ab12', 'name': 'Movies'}, {'id': 'zz!', 'name': 'bad'}]},
                                   {'id': 'nope', 'name': 'x'}]}

    def tearDown(self):
        self.tmp.cleanup()

    def hrefs(self, text):
        return [b.get('href') for b in ET.fromstring(text).findall('bookmark')]

    def test_wanted(self):
        w = hk.storage_places(self.status)
        self.assertEqual([x[0] for x in w], ['hearth:/', 'hearth:/%s/ab12' % SERVER])
        self.assertEqual(w[1][1], 'Movies on Den PC')

    def test_new_file_gets_defaults_and_storage(self):
        text, added = hk.sync_places_xml('', hk.storage_places(self.status), self.home)
        h = self.hrefs(text)
        self.assertEqual(h[0], 'file://' + self.home)
        self.assertIn('trash:/', h)
        self.assertIn('hearth:/', h)
        self.assertTrue(text.startswith('<?xml'))
        self.assertIn('xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"', text)
        self.assertEqual(added, {'hearth-storage', 'hearth-storage/%s/ab12' % SERVER})

    def test_yours_kept_ours_updated_removed_stays_removed(self):
        mine = ('<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE xbel>\n<xbel xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks">'
                '<bookmark href="file:///home/me/Games"><title>Games</title></bookmark></xbel>')
        text, added = hk.sync_places_xml(mine, hk.storage_places(self.status), self.home)
        self.assertEqual(self.hrefs(text), ['file:///home/me/Games', 'hearth:/', 'hearth:/%s/ab12' % SERVER])
        # Nothing to do the second time.
        again, added2 = hk.sync_places_xml(text, hk.storage_places(self.status), self.home, added)
        self.assertIsNone(again)
        # The share renamed on the server: the title follows.
        self.status['servers'][0]['shares'][0]['name'] = 'Films'
        renamed, _ = hk.sync_places_xml(text, hk.storage_places(self.status), self.home, added)
        self.assertIn('Films on Den PC', renamed)
        # You remove Hearth Storage: it isn't put back.
        root = ET.fromstring(text)
        for b in root.findall('bookmark'):
            if b.get('href') == 'hearth:/':
                root.remove(b)
        removed = ET.tostring(root, encoding='unicode')
        back, _ = hk.sync_places_xml(removed, hk.storage_places(self.status), self.home, added)
        self.assertNotIn('hearth:/', self.hrefs(back) if back else self.hrefs(removed))
        # The share is no longer shared: its place goes, yours stays.
        self.status['servers'][0]['shares'] = []
        gone, added3 = hk.sync_places_xml(text, hk.storage_places(self.status), self.home, added)
        self.assertEqual(self.hrefs(gone), ['file:///home/me/Games', 'hearth:/'])
        self.assertNotIn('hearth-storage/%s/ab12' % SERVER, added3)

    def test_unreadable_file_left_alone(self):
        self.assertIsNone(hk.sync_places_xml('<xbel><bookmark', hk.storage_places(self.status), self.home)[0])

    def fake_link(self, answer):
        b = os.path.join(self.home, 'hearth-link')
        with open(b, 'w') as f:
            f.write('#!/bin/sh\necho \'%s\'\n' % json.dumps(answer))
        os.chmod(b, 0o755)
        return b

    def test_sync_places_end_to_end(self):
        env = dict(os.environ, HOME=self.home, XDG_DATA_HOME=os.path.join(self.home, 'share'), XDG_CONFIG_HOME=os.path.join(self.home, 'cfg'),
                   HEARTH_LINK_BIN=self.fake_link(self.status), HEARTH_KDE_LIB=str(KDE), HEARTH_FILES_BIN='/nonexistent')
        r = subprocess.run([sys.executable, str(KDE / 'hearth-files-kde'), '--sync-places'], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        text = open(os.path.join(self.home, 'share/user-places.xbel')).read()
        self.assertIn('hearth:/%s/ab12' % SERVER, text)
        state = json.load(open(os.path.join(self.home, 'cfg/nimbus/hearth-files-kde.json')))
        self.assertIn('hearth-storage', state['added'])

    def test_no_hearth_link_no_places(self):
        env = dict(os.environ, HOME=self.home, XDG_DATA_HOME=os.path.join(self.home, 'share'), HEARTH_LINK_BIN='/nonexistent',
                   HEARTH_KDE_LIB=str(KDE))
        r = subprocess.run([sys.executable, str(KDE / 'hearth-files-kde'), '--sync-places'], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0)
        self.assertFalse(os.path.exists(os.path.join(self.home, 'share/user-places.xbel')))

    def test_starter_falls_back(self):
        # No Hearth Files and no other file manager here: it says so instead of failing silently.
        env = dict(os.environ, HOME=self.home, HEARTH_LINK_BIN='/nonexistent', HEARTH_KDE_LIB=str(KDE), HEARTH_FILES_BIN='/nonexistent',
                   PATH='/nonexistent')
        src = (KDE / 'hearth-files-kde').read_text().replace("'/usr/bin/hearth-filemanager', '/usr/bin/dolphin'", "'/nonexistent/a', '/nonexistent/b'")
        p = os.path.join(self.home, 'starter')
        open(p, 'w').write(src)
        r = subprocess.run([sys.executable, p], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 1)
        self.assertIn('no file manager', r.stderr)


class Config(unittest.TestCase):
    def test_color_scheme_everywhere(self):
        for rc in ('konsolerc', 'katerc', 'okularrc', 'kcalcrc', 'spectaclerc'):
            self.assertEqual(parse((KDE / 'config' / rc).read_text())['UiSettings']['ColorScheme'], 'Hearth', rc)
        self.assertEqual(parse((KDE / 'config/konsolerc').read_text())['Desktop Entry']['DefaultProfile'], 'Hearth.profile')
        prof = parse((KDE / 'config/Hearth.profile').read_text())
        self.assertEqual(prof['Appearance']['ColorScheme'], 'Hearth')
        cs = parse((KDE / 'config/Hearth.colorscheme').read_text())
        for i in range(8):
            self.assertIn('Color%d' % i, cs)
        self.assertTrue((REPO / 'hosts/linux/desktop/theme/Hearth.colors').exists())

    def test_dolphin_pin_and_patches(self):
        rows = [l.split() for l in (KDE / 'dolphin/versions.txt').read_text().splitlines() if l.strip() and not l.startswith('#')]
        self.assertEqual(sorted(r[0] for r in rows), ['anatase', 'peppermint'])
        for base, ver, sha in rows:
            self.assertTrue(ver == 'discover' or (len(sha) == 64 and all(c in '0123456789abcdef' for c in sha)), base)
        patches = sorted((KDE / 'dolphin/patches').glob('*.patch'))
        self.assertTrue(patches)
        for p in patches:
            t = p.read_text()
            self.assertIn('--- a/', t)
            self.assertNotIn('Co-authored-by', t)


if __name__ == '__main__':
    unittest.main()
