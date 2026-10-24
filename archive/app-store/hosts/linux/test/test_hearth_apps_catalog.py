"""ARCHIVED with Hearth's app store (docs/ARCHIVED.md): hearth-apps' catalog, search, columns and progress tests,
moved out of hosts/linux/test/test_hearth_settings_basics.py. Not run by CI; to bring the store back, move this next to
the other tests again."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import io
import os
import pathlib
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest import mock

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'


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


# ---------------------------------------------------------------------- hearth-apps
APPSTREAM = b'''<?xml version="1.0" encoding="UTF-8"?>
<components version="0.8" origin="flathub">
  <component type="desktop-application">
    <id>org.gimp.GIMP.desktop</id>
    <name>GNU Image Manipulation Program</name>
    <name xml:lang="de">GNU-Bildbearbeitungsprogramm</name>
    <summary>Create images and edit photographs</summary>
    <summary xml:lang="de">Bilder erstellen</summary>
    <description><p>GIMP is an image editor.</p><p xml:lang="de">Deutsch</p><ul><li>Layers</li><li>Filters</li></ul></description>
    <icon type="cached" width="128" height="128">org.gimp.GIMP.png</icon>
    <icon type="remote" width="128" height="128">https://dl.flathub.org/media/gimp.png</icon>
    <categories><category>Graphics</category><category>2DGraphics</category></categories>
    <keywords><keyword>photoshop</keyword></keywords>
    <developer_name>The GIMP team</developer_name>
    <project_license>GPL-3.0+</project_license>
    <url type="homepage">https://www.gimp.org/</url>
    <screenshots><screenshot type="default">
      <image type="source" width="1920" height="1080">https://dl.flathub.org/s/source.png</image>
      <image type="thumbnail" width="624" height="351">https://dl.flathub.org/s/624.png</image>
      <image type="thumbnail" width="112" height="63">https://dl.flathub.org/s/112.png</image>
    </screenshot></screenshots>
    <releases><release version="2.10.38" timestamp="1700000000"/></releases>
    <bundle type="flatpak" runtime="org.gnome.Platform/x86_64/46">app/org.gimp.GIMP/x86_64/stable</bundle>
    <custom><value key="flathub::verification::verified">true</value></custom>
  </component>
  <component type="runtime"><id>org.gnome.Platform</id><name>GNOME</name></component>
  <component type="desktop">
    <id>org.videolan.VLC</id>
    <name>VLC</name>
    <summary>VLC media player</summary>
    <icon type="remote">https://dl.flathub.org/media/vlc.png</icon>
    <categories><category>AudioVideo</category></categories>
  </component>
</components>
'''


class AppsHelper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = mock.patch.dict(os.environ, {'HOME': self.tmp, 'XDG_DATA_HOME': os.path.join(self.tmp, 'data'),
                                                'XDG_CACHE_HOME': os.path.join(self.tmp, 'cache')})
        self.env.start()
        self.h = load(SHELL / 'hearth-apps', 'hearth_apps')

    def tearDown(self):
        self.env.stop()

    def test_appstream(self):
        icon_dir = os.path.join(self.tmp, 'as')
        write(os.path.join(icon_dir, 'icons', '128x128', 'org.gimp.GIMP.png'), 'png')
        apps, details = self.h.parse_appstream(io.BytesIO(APPSTREAM), icon_dir)
        self.assertEqual(sorted(apps), ['org.gimp.GIMP', 'org.videolan.VLC'])
        g = apps['org.gimp.GIMP']
        self.assertEqual(g['name'], 'GNU Image Manipulation Program')
        self.assertEqual(g['summary'], 'Create images and edit photographs')
        self.assertEqual(g['icon'], os.path.join(icon_dir, 'icons', '128x128', 'org.gimp.GIMP.png'))
        self.assertEqual(g['categories'], ['Graphics', '2DGraphics'])
        self.assertEqual(g['developer'], 'The GIMP team')
        self.assertEqual(apps['org.videolan.VLC']['icon'], 'https://dl.flathub.org/media/vlc.png')
        d = details['org.gimp.GIMP']
        self.assertEqual(d['description'], 'GIMP is an image editor.\n\n• Layers\n• Filters')
        self.assertEqual(d['screenshots'], ['https://dl.flathub.org/s/624.png'])
        self.assertEqual((d['version'], d['license'], d['homepage'], d['verified']), ('2.10.38', 'GPL-3.0+', 'https://www.gimp.org/', True))

    def test_catalog_cache_from_users_appstream(self):
        d = os.path.join(self.tmp, 'data', 'flatpak', 'appstream', 'flathub', self.h.arch(), 'active')
        import gzip
        os.makedirs(d)
        with gzip.open(os.path.join(d, 'appstream.xml.gz'), 'wb') as f:
            f.write(APPSTREAM)
        apps, det = self.h.catalog()
        self.assertIn('org.videolan.VLC', apps)
        self.assertTrue(os.path.exists(det))
        apps2, _ = self.h.catalog()                  # from the cache now
        self.assertEqual(apps, apps2)
        r = self.h.search('photo edit', apps)
        self.assertEqual([a['id'] for a in r['apps']], ['org.gimp.GIMP'])
        self.assertEqual(self.h.search('photoshop', apps)['apps'][0]['id'], 'org.gimp.GIMP')
        self.assertEqual(self.h.search('', apps)['apps'], [])

    def test_columns(self):
        text = 'org.gimp.GIMP\tGIMP\t2.10.38\tuser\t320.1 MB\tflathub\ncom.valvesoftware.Steam\tSteam\t1.0.0.81\tsystem\t12 MB\tflathub\n' \
               'Application ID\tName\n\nnot-an-id\tx\n'
        rows = self.h.parse_columns(text, ['id', 'name', 'version', 'installation', 'size', 'origin'])
        self.assertEqual([r['id'] for r in rows], ['org.gimp.GIMP', 'com.valvesoftware.Steam'])
        self.assertEqual(rows[1]['installation'], 'system')
        self.assertEqual(rows[0]['size'], '320.1 MB')
        short = self.h.parse_columns('org.gimp.GIMP\t2.10\n', ['id', 'version', 'origin'])
        self.assertEqual(short[0]['origin'], '')

    def test_progress_lines(self):
        self.assertEqual(self.h.progress_of('Installing 2/3… ████▌     45%  1.2 MB/s  00:12'), (2, 3, 45))
        self.assertEqual(self.h.progress_of('Updating app/org.gimp.GIMP/x86_64/stable'), (None, None, None))
        self.assertEqual(self.h.progress_of('Installing runtime/org.freedesktop.Platform/x86_64/24.08 100%'), (None, None, 100))
        self.assertEqual(self.h.progress_of(' 1/1 Installing… 7%'), (1, 1, 7))

    def test_featured_from_api(self):
        self.assertEqual(self.h.featured_from_api({'hits': [{'app_id': 'org.a.B'}, {'app_id': 'bad'}, {'id': 'org.c.D'}, {'app_id': 'org.a.B'}]}),
                         ['org.a.B', 'org.c.D'])
        self.assertEqual(self.h.featured_from_api(['org.a.B', 7, 'x']), ['org.a.B'])
        self.assertEqual(self.h.featured_from_api(None), [])

    def test_ids_checked(self):
        for bad in ('', 'org', 'org.foo', 'org.foo.Bar;rm', '--user', '../org.a.b', 'org.a.b c'):
            with self.assertRaises(self.h.Fail, msg=bad):
                self.h.check_id(bad)
        self.assertEqual(self.h.check_id('com.valvesoftware.Steam'), 'com.valvesoftware.Steam')

    def test_main_usage(self):
        out = io.StringIO()
        with redirect_stdout(out), self.assertRaises(SystemExit):
            self.h.main(['x', 'remove', 'org.a.B', '--everything'])
        self.assertIn('usage', out.getvalue())


if __name__ == '__main__':
    unittest.main()
