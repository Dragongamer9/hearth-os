"""1.15 (Anatase): every Hearth look in palettes.json is a Plasma Global Theme "Hearth OS · <name>", generated from its
palette at build time (os-bases/anatase/system_files/usr/libexec/nimbus/hearth-plasma-looks), and Hearth's Settings
and System Settings stay in step without anything changing by itself."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import pathlib
import re
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
TOOL = REPO / 'os-bases/anatase/system_files/usr/libexec/nimbus/hearth-plasma-looks'
THEME = REPO / 'hosts/linux/desktop/theme'
LOGO = REPO / 'branding/icons/apps/hearth.svg'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_plasma_looks', str(TOOL))
    m = importlib.util.module_from_spec(importlib.util.spec_from_loader(loader.name, loader))
    loader.exec_module(m)
    return m


PL = load()
PAL = json.loads((THEME / 'palettes.json').read_text())
LOOKS = [k for k, v in PAL.items() if not k.startswith('_') and isinstance(v, dict) and isinstance(v.get('dark'), dict)]


class Build(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = pathlib.Path(cls.tmp.name)
        cls.made = PL.build(str(cls.root), str(THEME), str(LOGO))
        cls.share = cls.root / 'usr/share'

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_the_eight_looks(self):
        self.assertEqual(set(LOOKS), {'hearth', 'steam', 'gaming', 'zen', 'glass', 'synthwave', 'cabin', 'console'})

    def test_every_palette_has_a_global_theme(self):
        for look in LOOKS:
            d = self.share / 'plasma/look-and-feel' / ('org.hearthos.' + look)
            meta = json.loads((d / 'metadata.json').read_text())
            self.assertEqual(meta['KPlugin']['Id'], 'org.hearthos.' + look)
            self.assertEqual(meta['KPlugin']['Name'], 'Hearth OS · ' + PAL[look]['label'])
            self.assertEqual(meta['X-Hearth-Palette'], look)
            self.assertEqual(meta['KPackageStructure'], 'Plasma/LookAndFeel')
            for f in ('contents/defaults', 'contents/splash/Splash.qml', 'contents/splash/images/hearth.svg',
                      'contents/previews/preview.png', 'contents/previews/fullscreenpreview.png'):
                self.assertTrue((d / f).is_file(), '%s: %s' % (look, f))

    def test_every_global_theme_maps_to_a_palette(self):
        for d in (self.share / 'plasma/look-and-feel').iterdir():
            meta = json.loads((d / 'metadata.json').read_text())
            self.assertIn(meta['X-Hearth-Palette'], LOOKS, d.name)
            self.assertEqual(d.name, 'org.hearthos.' + meta['X-Hearth-Palette'])
        for d in (self.share / 'plasma/desktoptheme').iterdir():
            self.assertIn(re.sub(r'-light$', '', d.name[len('hearth-'):]), LOOKS, d.name)
        for f in (self.share / 'color-schemes').iterdir():
            self.assertIn(f.name.split('-')[1], LOOKS, f.name)

    def test_defaults_point_at_its_own_pieces(self):
        for look in LOOKS:
            text = (self.share / 'plasma/look-and-feel' / ('org.hearthos.' + look) / 'contents/defaults').read_text()
            self.assertIn('ColorScheme=HearthOS-%s-Dark\n' % look, text)
            self.assertIn('[plasmarc][Theme]\nname=hearth-%s\n' % look, text)
            self.assertIn('library=org.kde.breeze\n', text)       # Breeze frames in the look's colours, no Aurorae
            self.assertNotIn('aurorae', text.lower())
            self.assertIn('Theme=org.hearthos.%s\n' % look, text)  # its splash
            wall = re.search(r'(?m)^Image=(Hearth-[a-z0-9-]+)$', text).group(1)
            self.assertTrue((REPO / 'os/branding/wallpapers' / (wall.replace('Hearth-', 'hearth-') + '.jpg')).is_file(), wall)
            for mode in ('Dark', 'Light'):
                self.assertTrue((self.share / 'color-schemes' / ('HearthOS-%s-%s.colors' % (look, mode))).is_file())

    def test_colours_come_from_the_palette(self):
        def rgb(h):
            return ','.join(str(int(h[i:i + 2], 16)) for i in (1, 3, 5))
        for look in LOOKS:
            for mode in ('dark', 'light'):
                text = (self.share / 'color-schemes' / PL.scheme_file(look, mode)).with_suffix('.colors').read_text()
                p = PAL[look][mode]
                self.assertIn('Name=Hearth OS · %s (%s)' % (PAL[look]['label'], mode.capitalize()), text)
                self.assertIn('DecorationFocus=' + rgb(p['ember']), text, '%s %s accent' % (look, mode))
                self.assertIn('[WM]', text)
                style = (self.share / 'plasma/desktoptheme' / PL.style_id(look, mode) / 'colors').read_text()
                self.assertEqual(style, text)

    def test_sub_themes_are_colour_schemes(self):
        for look in LOOKS:
            subs = [s['id'] for s in PL.load_theme(str(THEME)).sub_list(PAL, look)][1:]
            for sid in subs:
                for mode in ('dark', 'light'):
                    self.assertTrue((self.share / 'color-schemes' / (PL.scheme_file(look, mode, sid) + '.colors')).is_file())

    def test_build_lists_every_look(self):
        self.assertEqual(sorted(m['look'] for m in self.made), sorted(LOOKS))


class Sync(unittest.TestCase):
    W = {'look': 'hearth', 'mode': 'dark', 'accent': '', 'sub': ''}

    def test_want_defaults_to_hearth(self):
        self.assertEqual(PL.want_of(PAL, {}), self.W)
        self.assertEqual(PL.want_of(PAL, {'look': 'nope', 'mode': 'odd'}), self.W)
        self.assertEqual(PL.want_of(PAL, {'look': 'zen', 'mode': 'light', 'accent': '#AABBCC'})['accent'], '#aabbcc')

    def test_kde_choice(self):
        self.assertFalse(PL.kde_of('')['chose'])
        self.assertFalse(PL.kde_of('[KDE]\nLookAndFeelPackage=org.hearthos.zen\n')['chose'])
        self.assertTrue(PL.kde_of('[KDE]\nLookAndFeelPackage=org.kde.breezedark.desktop\n')['chose'])
        self.assertTrue(PL.kde_of('[General]\nColorScheme=BreezeLight\n')['chose'])
        self.assertFalse(PL.kde_of('# Written by Hearth (Settings)\n[General]\nColorScheme=Nordic\n')['chose'])
        self.assertFalse(PL.kde_of('[General]\nColorScheme=HearthOS-zen-Dark\n')['chose'])

    def test_first_time(self):
        self.assertEqual(PL.decide(None, self.W, {'lnf': None, 'chose': False}), 'apply')
        self.assertEqual(PL.decide(None, self.W, {'lnf': 'org.kde.breeze.desktop', 'chose': True}), 'ask')

    def test_nothing_changes_by_itself(self):
        st = {'applied': self.W, 'kde': 'org.kde.breezedark.desktop', 'asked': 'x'}
        self.assertIsNone(PL.decide(st, self.W, {'lnf': 'org.kde.breezedark.desktop', 'chose': True}))
        # another (non-Hearth) Global Theme picked in System Settings: left alone
        self.assertIsNone(PL.decide(st, self.W, {'lnf': 'org.kde.breeze.desktop', 'chose': True}))

    def test_hearth_settings_pick_applies(self):
        st = {'applied': self.W, 'kde': 'org.hearthos.hearth'}
        self.assertEqual(PL.decide(st, dict(self.W, look='zen'), {'lnf': 'org.hearthos.hearth', 'chose': False}), 'apply')

    def test_system_settings_pick_is_saved(self):
        st = {'applied': self.W, 'kde': 'org.hearthos.hearth'}
        self.assertEqual(PL.decide(st, self.W, {'lnf': 'org.hearthos.cabin', 'chose': False}), 'save')
        self.assertEqual(PL.look_of('org.hearthos.cabin'), 'cabin')
        self.assertIsNone(PL.look_of('org.kde.breeze.desktop'))

    def test_wallpaper_follows_only_hearths(self):
        self.assertTrue(PL.hearth_wallpaper(''))
        self.assertTrue(PL.hearth_wallpaper('file:///usr/share/wallpapers/Hearth-fireside/contents/images/3840x2160.jpg'))
        self.assertFalse(PL.hearth_wallpaper('file:///home/a/Pictures/me.jpg'))


class Wiring(unittest.TestCase):
    def test_build_runs_it(self):
        b = (REPO / 'os-bases/anatase/build.sh').read_text()
        self.assertIn('hearth-plasma-looks build', b)
        self.assertIn("'LookAndFeelPackage': 'org.hearthos.hearth'", b)
        self.assertIn('systemctl --global enable hearth-plasma-looks.path', b)
        self.assertNotIn('Engine=none', b)

    def test_units(self):
        sf = REPO / 'os-bases/anatase/system_files'
        p = (sf / 'usr/lib/systemd/user/hearth-plasma-looks.path').read_text()
        self.assertIn('PathChanged=%h/.local/share/nimbus/desktop.json', p)
        self.assertIn('WantedBy=plasma-workspace.target', p)
        a = (sf / 'etc/xdg/autostart/hearth-plasma-looks.desktop').read_text()
        self.assertIn('OnlyShowIn=KDE;', a)
        self.assertIn('sync --wait', a)


if __name__ == '__main__':
    unittest.main()
