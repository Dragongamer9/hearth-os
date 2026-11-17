"""Tests for a few more of KDE's settings in Hearth Settings (1.14): the wallpaper's picture of the day and a light/dark
pair (hearth-potd, WallShow.qml, SetWallpaperMore.qml), the sub-pixel order of fonts (hearth-lookset, SetFonts.qml),
"Apps follow Hearth" (theme/hearth-theme, theme/apply.sh, SetLookMore.qml), which microphone each app records from
(hearth-sound route, Mixer.qml, SetSoundApps.qml) and Hybrid sleep only where Hibernate works (hearth-powerset,
SetPower.qml). Programs are stand-ins; nothing here touches the real system or the internet."""
import sys
sys.dont_write_bytecode = True
import datetime
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
THEME = ROOT / 'hosts' / 'linux' / 'desktop' / 'theme'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Home(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.env = mock.patch.dict(os.environ, {'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'),
                                                'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
                                                'XDG_CACHE_HOME': os.path.join(self.home, '.cache'),
                                                'XDG_RUNTIME_DIR': self.home})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()


# ------------------------------------------------------------------ picture of the day
BING = json.dumps({'images': [{'url': '/th?id=OHR.Fox_EN-US123_1920x1080.jpg&rf=x', 'title': 'A fox in the snow',
                               'copyright': 'Red fox, Yellowstone (© Someone)', 'startdate': '20261117'}]})
APOD = ('<html><head><title> APOD: 2026 November 17 - The Pillars of Creation\n</title></head><body><center>'
        '<a href="image/2611/pillars_webb.jpg"><img src="image/2611/pillars_webb1024.jpg"></a></center>'
        '<center><b> The Pillars of Creation </b><br><b> Image Credit: </b><a href="x">NASA</a>, ESA &amp; CSA</center></body></html>')
APOD_VIDEO = '<html><title>APOD: 2026 November 18 - A video</title><iframe src="https://www.youtube.com/embed/x"></iframe></html>'
WIKI = json.dumps({'image': {'title': 'File:Lake_Bled_at_dawn.jpg',
                             'image': {'source': 'https://upload.wikimedia.org/wikipedia/commons/a/ab/Lake_Bled_at_dawn.jpg', 'width': 7000, 'height': 4000},
                             'thumbnail': {'source': 'https://upload.wikimedia.org/wikipedia/commons/thumb/a/ab/Lake_Bled_at_dawn.jpg/640px-Lake_Bled_at_dawn.jpg'},
                             'artist': {'text': 'A. Photographer'}, 'description': {'text': 'Lake Bled at dawn'}}})
JPEG = b'\xff\xd8\xff\xe0' + b'0' * 4096


class PictureOfTheDay(Home):
    def setUp(self):
        super().setUp()
        self.p = load(SHELL / 'hearth-potd', 'hearth_potd')
        self.day = datetime.date(2026, 11, 17)
        self.calls = []

    def getter(self, pages):
        def get(url, limit=0):
            self.calls.append(url)
            if url in pages:
                return pages[url]
            if 'image' in url or 'upload.wikimedia' in url or '/th?' in url:
                return JPEG, 'image/jpeg'
            raise self.p.Fail('offline')
        return get

    def test_each_site_is_read(self):
        b = self.p.parse_bing(BING)
        self.assertEqual(b['url'], 'https://www.bing.com/th?id=OHR.Fox_EN-US123_1920x1080.jpg&rf=x')
        self.assertEqual(b['title'], 'A fox in the snow')
        a = self.p.parse_apod(APOD)
        self.assertEqual(a['url'], 'https://apod.nasa.gov/apod/image/2611/pillars_webb.jpg')
        self.assertEqual(a['title'], 'The Pillars of Creation')
        self.assertIn('NASA, ESA & CSA', a['credit'])
        with self.assertRaises(self.p.Fail):
            self.p.parse_apod(APOD_VIDEO)          # a video day: no picture
        w = self.p.parse_wikimedia(WIKI)
        self.assertIn('/3840px-Lake_Bled_at_dawn.jpg', w['url'])      # a 7000 px original comes as a 3840 px copy
        self.assertEqual((w['title'], w['credit']), ('Lake Bled at dawn', 'A. Photographer'))

    def test_pictures_only_from_the_sites_own_hosts(self):
        self.assertTrue(self.p.allowed('bing', 'https://www.bing.com/th?id=x'))
        self.assertFalse(self.p.allowed('bing', 'http://www.bing.com/th?id=x'))
        self.assertFalse(self.p.allowed('apod', 'https://evil.example/image.jpg'))
        self.assertFalse(self.p.allowed('wikimedia', 'https://upload.wikimedia.org.evil.example/x.jpg'))
        with self.assertRaises(self.p.Fail):
            self.p.check_source('../etc')
        bad = json.dumps({'images': [{'url': 'https://evil.example/x.jpg'}]})
        with self.assertRaises(self.p.Fail):
            self.p.fetch('bing', self.day, self.getter({self.p.page_url('bing', self.day): (bad.encode(), 'application/json')}))

    def test_downloaded_once_a_day_and_kept(self):
        g = self.getter({self.p.page_url('bing', self.day): (BING.encode(), 'application/json')})
        r = self.p.fetch('bing', self.day, g)
        self.assertFalse(r['cached'])
        self.assertTrue(r['path'].endswith('bing-2026-11-17.jpg'))
        self.assertTrue(os.path.isfile(r['path']))
        n = len(self.calls)
        again = self.p.fetch('bing', self.day, g)
        self.assertTrue(again['cached'])
        self.assertEqual(len(self.calls), n)        # no second download the same day
        self.assertEqual(self.p.cached('bing')['path'], r['path'])

    def test_old_days_are_tidied_and_only_its_own_files(self):
        cache = self.p.CACHE
        os.makedirs(cache)
        for name in ('bing-2026-11-01.jpg', 'bing-2026-11-01.json', 'bing-2026-11-12.jpg', 'my-photo.jpg'):
            open(os.path.join(cache, name), 'w').close()
        self.p.tidy('bing', self.day)
        self.assertEqual(sorted(os.listdir(cache)), ['bing-2026-11-12.jpg', 'my-photo.jpg'])

    def test_offline_shows_the_newest_kept(self):
        g = self.getter({self.p.page_url('apod', self.day): (APOD.encode(), 'text/html')})
        self.p.fetch('apod', self.day, g)
        with mock.patch.object(self.p, 'get', side_effect=self.p.Fail("couldn't reach apod.nasa.gov")), \
                mock.patch.object(self.p, 'today', return_value=self.day + datetime.timedelta(days=1)):
            out = io.StringIO()
            with redirect_stdout(out):
                self.assertEqual(self.p.main(['fetch', 'apod']), 0)
        r = json.loads(out.getvalue())
        self.assertEqual(r['date'], '2026-11-17')
        self.assertIn("couldn't reach", r['note'])
        out = io.StringIO()
        with redirect_stdout(out):
            self.assertEqual(self.p.main(['cached', 'wikimedia']), 1)
        self.assertTrue(out.getvalue().startswith('error: '))

    def test_not_a_picture_is_refused(self):
        def g(url, limit=0):
            return (BING.encode(), 'application/json') if 'HPImageArchive' in url else (b'<html>blocked</html>', 'text/html')
        with self.assertRaises(self.p.Fail):
            self.p.fetch('bing', self.day, g)
        self.assertEqual(self.p.kept('bing'), [])

    def test_wallshow_order_and_off_by_default(self):
        ws = (SHELL / 'WallShow.qml').read_text()
        self.assertIn('current !== "" ? current : potdPath !== "" ? potdPath : pairWall !== "" ? pairWall : Theme.wall', ws)
        self.assertIn('Theme.dark ? pair.dark : pair.light', ws)
        self.assertIn('readonly property bool potdOn: !!potdCfg.on', ws)
        self.assertIn('"/hearth-potd", "fetch", potdSource]', ws)
        self.assertNotRegex(ws, r'Settings\.wall\w*\s*=[^=]')                  # only read there
        page = (SHELL / 'SetWallpaperMore.qml').read_text()
        self.assertIn('Object.assign({ on: false, source: "bing" }', page)
        self.assertIn('Object.assign({ on: false, light: "", dark: "" }', page)
        for src in ('bing', 'apod', 'wikimedia'):
            self.assertIn('value: "%s"' % src, page)


# ------------------------------------------------------------------ fonts: sub-pixel order
class SubPixel(Home):
    def setUp(self):
        super().setUp()
        self.ls = load(SHELL / 'hearth-lookset', 'hearth_lookset_bits')
        self.ls.FONTCONF = os.path.join(self.home, '.config', 'fontconfig', 'conf.d', '90-hearth-fonts.conf')
        self.ls.CURSOR_ENV = os.path.join(self.home, '.config', 'environment.d', '90-hearth-cursor.conf')
        self.ls.SAVED = os.path.join(self.home, '.config', 'nimbus', 'fonts-cursor-icons.json')

    def test_checked(self):
        self.assertEqual(self.ls.clean({'subpixel': 'bgr'}), {'subpixel': 'bgr'})
        self.assertEqual(self.ls.clean({'subpixel': None}), {'subpixel': None})
        for bad in ('rgba', 'BGR', 1, '"; rm'):
            with self.assertRaises(self.ls.Fail):
                self.ls.clean({'subpixel': bad})

    def test_handed_on_only_with_subpixel_smoothing(self):
        s = {'antialias': 'rgba', 'subpixel': 'vbgr'}
        fc = self.ls.fontconfig_text(self.ls.effective(s), s)
        self.assertIn('<const>vbgr</const>', fc)
        s2 = {'antialias': 'grayscale', 'subpixel': 'vbgr'}
        self.assertIn('<edit name="rgba" mode="assign"><const>none</const></edit>', self.ls.fontconfig_text(self.ls.effective(s2), s2))
        self.assertIn(['org.gnome.desktop.interface', 'font-rgba-order', 'vbgr'], self.ls.gsettings_cmds(self.ls.effective(s), s))
        self.assertIsNone(self.ls.fontconfig_text(self.ls.effective({}), {}))       # nothing chosen: no file

    def test_kde_apps_only_while_kdeglobals_is_hearths(self):
        kg = os.path.join(self.home, '.config', 'kdeglobals')
        os.makedirs(os.path.dirname(kg))
        with open(kg, 'w') as f:
            f.write(self.ls.KDE_MARK + ' (Settings > Appearance)\n[General]\nfont=Noto Sans,10\n[KScreen]\nScaleFactor=1\n')
        with mock.patch.object(self.ls.shutil, 'which', return_value=None):
            self.ls.apply({'antialias': 'rgba', 'subpixel': 'bgr'})
        text = open(kg).read()
        self.assertIn('XftSubPixel=bgr', text)
        self.assertIn('ScaleFactor=1', text)
        with open(kg, 'w') as f:
            f.write('[General]\nXftSubPixel=rgb\n')              # your own kdeglobals: left alone
        with mock.patch.object(self.ls.shutil, 'which', return_value=None):
            self.ls.apply({'antialias': 'rgba', 'subpixel': 'bgr'})
        self.assertEqual(open(kg).read(), '[General]\nXftSubPixel=rgb\n')

    def test_page(self):
        src = (SHELL / 'SetFonts.qml').read_text()
        self.assertIn('page.pick("subpixel", v === "rgb" ? "" : v)', src)
        self.assertIn('enabled: (page.e.antialias || "grayscale") === "rgba"', src)


# ------------------------------------------------------------------ apps follow Hearth
class AppsFollow(Home):
    def setUp(self):
        super().setUp()
        self.t = load(THEME / 'hearth-theme', 'hearth_theme_bits')

    def test_on_unless_turned_off(self):
        self.assertTrue(self.t.apps_follow({}))
        self.assertTrue(self.t.apps_follow(None))
        self.assertTrue(self.t.apps_follow({'appsFollow': 'no'}))
        self.assertTrue(self.t.apps_follow({'appsFollow': True}))
        self.assertFalse(self.t.apps_follow({'appsFollow': False}))
        self.assertIn('ColorScheme=BreezeLight', self.t.kde_own_scheme('light'))
        self.assertIn('ColorScheme=BreezeDark', self.t.kde_own_scheme('dark'))

    def run_apply(self):
        """theme/apply.sh in this home with only the programs it needs (no gsettings or flatpak here)."""
        bindir = os.path.join(self.home, 'bin')
        os.makedirs(bindir, exist_ok=True)
        for p in ('python3', 'mkdir', 'cat', 'head', 'grep', 'cp', 'sed', 'sh', 'printf'):
            w = shutil.which(p)
            if w and not os.path.exists(os.path.join(bindir, p)):
                os.symlink(w, os.path.join(bindir, p))
        env = dict(os.environ, PATH=bindir, NIMBUS_SHARE=str(ROOT / 'hosts' / 'linux'))
        env.pop('HEARTH_WM', None)
        r = subprocess.run(['sh', str(THEME / 'apply.sh')], env=env, capture_output=True, text=True, timeout=60)
        self.assertEqual(r.returncode, 0, r.stderr)

    def desktop(self, d):
        p = os.path.join(self.home, '.local', 'share', 'nimbus', 'desktop.json')
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            json.dump(d, f)

    def test_off_and_on_again(self):
        conf = os.path.join(self.home, '.config')
        self.desktop({'look': 'hearth', 'mode': 'dark', 'appsFollow': False})
        self.run_apply()
        qt = open(os.path.join(conf, 'qt6ct', 'qt6ct.conf')).read()
        self.assertIn('custom_palette=false', qt)
        self.assertNotIn('color_scheme_path', qt)
        gtk = open(os.path.join(conf, 'gtk-4.0', 'gtk.css')).read()
        self.assertTrue(gtk.startswith('/* Written by Hearth; delete this line'))     # still Hearth's: on again rewrites it
        self.assertNotIn('@define-color', gtk)
        kg = open(os.path.join(conf, 'kdeglobals')).read() if os.path.exists(os.path.join(conf, 'kdeglobals')) else ''
        self.assertNotIn('ColorScheme=Hearth', kg)
        # On again: Hearth's colours are back in the same files.
        self.desktop({'look': 'hearth', 'mode': 'dark', 'appsFollow': True})
        self.run_apply()
        self.assertIn('custom_palette=true', open(os.path.join(conf, 'qt6ct', 'qt6ct.conf')).read())
        self.assertIn('@define-color', open(os.path.join(conf, 'gtk-4.0', 'gtk.css')).read())
        self.assertIn('ColorScheme=Hearth', open(os.path.join(conf, 'kdeglobals')).read())
        # Off again: kdeglobals (Hearth's) gets KDE's own scheme, the rest of it stays.
        self.desktop({'look': 'hearth', 'mode': 'dark', 'appsFollow': False})
        self.run_apply()
        self.assertIn('ColorScheme=BreezeDark', open(os.path.join(conf, 'kdeglobals')).read())

    def test_your_own_files_stay_yours(self):
        conf = os.path.join(self.home, '.config')
        os.makedirs(os.path.join(conf, 'gtk-4.0'))
        with open(os.path.join(conf, 'gtk-4.0', 'gtk.css'), 'w') as f:
            f.write('/* mine */\n')
        self.desktop({'appsFollow': False})
        self.run_apply()
        self.assertEqual(open(os.path.join(conf, 'gtk-4.0', 'gtk.css')).read(), '/* mine */\n')

    def test_saved_by_its_own_key_and_reapplied(self):
        st = (SHELL / 'Settings.qml').read_text()
        for k in ('wallPotd', 'wallPair', 'appsFollow'):
            self.assertIn('"%s"' % k, st)
            self.assertIn('on%s%sChanged: save("%s")' % (k[0].upper(), k[1:], k), st)
        self.assertIn('property bool appsFollow: true', st)
        self.assertIn('Settings.appsFollow', (SHELL / 'Theme.qml').read_text())
        self.assertIn("('HEARTH_APPS_FOLLOW', '1' if follow else '0')", (THEME / 'hearth-theme').read_text())
        self.assertIn('onToggled: v => Settings.appsFollow = v', (SHELL / 'SetLookMore.qml').read_text())


# ------------------------------------------------------------------ per-app microphone
class AppMicrophone(Home):
    def setUp(self):
        super().setUp()
        self.s = load(SHELL / 'hearth-sound', 'hearth_sound_bits')

    def test_route_is_argument_lists(self):
        self.assertEqual(self.s.route_cmds('57', '42', ['101', '102']),
                         [['pw-metadata', '101', 'target.object', '57'], ['pw-metadata', '101', 'target.node', '42'],
                          ['pw-metadata', '102', 'target.object', '57'], ['pw-metadata', '102', 'target.node', '42']])
        self.assertEqual(self.s.route_cmds('-1', '-1', ['101']),
                         [['pw-metadata', '-d', '101', 'target.object'], ['pw-metadata', '-d', '101', 'target.node']])
        for bad in (('57; rm', '42', ['1']), ('57', '42', []), ('57', '-1', ['1']), ('57', '42', ['-1']), ('57', '42', ['1 2'])):
            with self.assertRaises(self.s.Fail):
                self.s.route_cmds(*bad)

    def test_route_runs_each_and_stops_at_a_refusal(self):
        ran = []

        def run(cmd, timeout=20):
            ran.append(cmd)
            return (1, '', 'no such object') if cmd[1] == '102' else (0, '', '')
        with mock.patch.object(self.s.shutil, 'which', return_value='/usr/bin/pw-metadata'), mock.patch.object(self.s, 'run', side_effect=run):
            self.assertTrue(self.s.route('57', '42', ['101'])['ok'])
            with self.assertRaises(self.s.Fail):
                self.s.route('57', '42', ['102', '103'])
        self.assertNotIn(['pw-metadata', '103', 'target.object', '57'], ran)

    def test_mixer_and_page(self):
        mx = (SHELL / 'Mixer.qml').read_text()
        self.assertIn('n.audio && n.isStream && n.isSink', mx)               # apps recording
        self.assertIn('n.audio && !n.isSink && !n.isStream', mx)             # microphones
        self.assertIn('"/hearth-sound", "route", serial', mx)
        fx = json.loads((ROOT / 'hosts' / 'linux' / 'test' / 'fixtures' / 'mixer.json').read_text())
        self.assertTrue(fx['recording'] and fx['inputs'])
        page = (SHELL / 'SetSoundApps.qml').read_text()
        self.assertIn('page.mixer.takeFrom(rec.modelData', page)
        self.assertIn('{ value: "", label: "The default microphone" }', page)


# ------------------------------------------------------------------ hybrid sleep
class HybridSleep(Home):
    def setUp(self):
        super().setUp()
        self.p = load(SHELL / 'hearth-powerset', 'hearth_powerset_bits')
        self.sysd = os.path.join(self.home, 'sys')
        os.makedirs(os.path.join(self.sysd, 'power'))

    def state(self, text):
        with open(os.path.join(self.sysd, 'power', 'state'), 'w') as f:
            f.write(text)

    def test_only_where_hibernate_works(self):
        self.state('freeze mem disk\n')
        self.assertFalse(self.p.hybrid_check({'can': False}, self.sysd, can='yes')['can'])
        self.assertIn('Hibernate', self.p.hybrid_check({'can': False}, self.sysd, can='yes')['why'])
        self.assertFalse(self.p.hybrid_check(None, self.sysd, can='yes')['can'])
        self.assertFalse(self.p.hybrid_check({'can': True}, self.sysd, can='na')['can'])
        self.assertTrue(self.p.hybrid_check({'can': True}, self.sysd, can='yes')['can'])
        self.state('disk\n')
        self.assertFalse(self.p.hybrid_check({'can': True}, self.sysd, can='yes')['can'])

    def test_status_and_the_tests_preview(self):
        with mock.patch.object(self.p, 'lid', return_value={}), mock.patch.object(self.p, 'has_lid', return_value=False), \
                mock.patch.object(self.p, 'inhibitors', return_value=[]), mock.patch.object(self.p, 'batteries', return_value=[]), \
                mock.patch.object(self.p, 'hibernate_check', return_value={'can': False, 'why': 'no'}):
            r = self.p.status()
            self.assertFalse(r['hybrid']['can'])
            with open(os.path.join(self.home, 'hearth-powerset-preview.json'), 'w') as f:
                json.dump({'hibernate': {'can': True, 'why': ''}, 'hybrid': {'can': True, 'why': ''}}, f)
            # Outside Hearth's checks a stray file changes nothing.
            with mock.patch.dict(os.environ, {'HEARTH_CHECKS': ''}):
                r = self.p.status()
            self.assertFalse(r['hybrid']['can'])
            with mock.patch.dict(os.environ, {'HEARTH_CHECKS': '1'}):
                r = self.p.status()
            self.assertTrue(r['hibernate']['can'] and r['hybrid']['can'])

    def test_page_asks_first_and_only_with_hibernate(self):
        src = (SHELL / 'SetPower.qml').read_text()
        self.assertIn('visible: !!page.hib && page.hib.can && !!page.hybrid && page.hybrid.can', src)
        self.assertIn('if (page.hib.proven) Quickshell.execDetached(["systemctl", "hybrid-sleep"]); else { page.askWhat = "hybrid-sleep"; page.askHibernate = true; }', src)
        # The resume hook already marks a hybrid sleep that came back (hibernate-proven).
        hook = (ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system-sleep' / 'hearth-resumed').read_text()
        self.assertIn('hybrid-sleep', hook)


class Pages(unittest.TestCase):
    def test_opening_pages_writes_nothing(self):
        for f in ('SetWallpaperMore', 'SetLookMore', 'SetSoundApps', 'SetFonts', 'SetPower'):
            src = (SHELL / (f + '.qml')).read_text()
            for m in re.finditer(r'Component\.onCompleted:([^\n]*)', src):
                self.assertNotRegex(m.group(1), r'Settings\.\w+\s*=[^=]', f)
                self.assertNotRegex(m.group(1), r'"(set|route|fetch)"', f)

    def test_search_finds_them(self):
        idx = (SHELL / 'settings-index.js').read_text()
        for t in ('Picture of the day', 'Apps follow Hearth', 'Sub-pixel order', 'Hybrid sleep'):
            self.assertIn('"%s"' % t, idx)


if __name__ == '__main__':
    unittest.main()
