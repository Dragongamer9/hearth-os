"""Tests for Hearth Music's backend (hosts/decky-hearth-music/main.py) and its shipping by hearth-decky. No Steam, no
Decky, no network: a stand-in `decky` module, fake Steam folders and made-up iTunes answers."""
import sys
sys.dont_write_bytecode = True
import asyncio
import base64
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import tempfile
import types
import unittest

HERE = pathlib.Path(__file__).resolve().parents[1]
ROOT = HERE.parents[1]


def load(tmp):
    fake = types.ModuleType('decky')
    fake.DECKY_USER_HOME = os.path.join(tmp, 'home')
    fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(tmp, 'settings')
    fake.logger = types.SimpleNamespace(info=lambda m: None)
    sys.modules['decky'] = fake
    spec = importlib.util.spec_from_file_location('hearth_music_main', HERE / 'main.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def write(p, data=b'x'):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.write(data if isinstance(data, bytes) else data.encode())


def it(track, album, artist='Composer', url='https://audio-ssl.itunes.apple.com/p/a.m4a', n=1):
    return {'trackName': track, 'collectionName': album, 'artistName': artist, 'previewUrl': url, 'trackNumber': n,
            'trackViewUrl': 'https://music.apple.com/us/album/x/1?i=2'}


class Backend(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-music-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.m = load(self.tmp)
        self.m._calls.clear()
        self.steam = os.path.join(self.home, '.local/share/Steam')
        os.makedirs(os.path.join(self.steam, 'steamapps'))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ---- names and matching
    def test_names(self):
        self.assertEqual(self.m.norm('THE WITCHER® 3: Wild Hunt'), 'the witcher 3 wild hunt')
        self.assertEqual(self.m.norm('MainMenu_Theme.ogg'), 'main menu theme ogg')
        self.assertEqual(self.m.title_forms('The Witcher 3: Wild Hunt - Game of the Year Edition'), ['the witcher 3 wild hunt game of the year edition', 'the witcher 3'])
        self.assertEqual(self.m.title_forms('Portal 2'), ['portal 2'])
        self.assertEqual(self.m.title_forms('DOOM: Eternal'), ['doom eternal', 'doom'])

    def test_picks_the_main_theme_from_the_soundtrack(self):
        forms = self.m.title_forms('ELDEN RING')
        res = [it('Godrick the Grafted', 'ELDEN RING (Original Soundtrack)', n=3),
               it('Elden Ring (Main Theme) [Piano Version]', 'Game Piano Covers', 'Piano Guy'),
               it('Elden Ring', 'ELDEN RING (Original Soundtrack)', 'Tsukasa Saitoh', n=1),
               it('Main Theme', 'Elden Ring Tribute', 'The Gamers')]
        m = self.m.best_match(res, forms)
        self.assertEqual((m['track'], m['artist']), ('Elden Ring', 'Tsukasa Saitoh'))
        self.assertTrue(m['link'].startswith('https://music.apple.com/'))

    def test_theme_track_beats_other_tracks(self):
        forms = self.m.title_forms('Stardew Valley')
        res = [it('Spring (It\'s A Big World Outside)', 'Stardew Valley (Original Game Soundtrack)', n=2),
               it('Stardew Valley Overture', 'Stardew Valley (Original Game Soundtrack)', n=1)]
        self.assertEqual(self.m.best_match(res, forms)['track'], 'Stardew Valley Overture')

    def test_no_match_without_the_title_or_with_unsafe_urls(self):
        forms = self.m.title_forms('Hades')
        self.assertIsNone(self.m.best_match([it('Main Theme', 'Some Other Game OST')], forms))
        self.assertIsNone(self.m.best_match([it('Hades', 'Greek Myths Vol. 2', 'Choir')], forms), 'title but no soundtrack or theme: not sure enough')
        self.assertIsNone(self.m.best_match([it('Hades Main Theme', 'Hades OST', url='https://evil.example/a.m4a')], forms))
        self.assertIsNone(self.m.best_match([it('Hades Main Theme', 'Hades OST', url='http://audio.itunes.apple.com/a.m4a')], forms))
        self.assertIsNotNone(self.m.best_match([it('Hades Main Theme', 'Hades OST')], forms))

    def test_lookup_is_cached_including_no_match(self):
        calls = []

        def get(term):
            calls.append(term)
            return [] if 'Nothing' in term else [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')]
        a = self.m.lookup('Celeste', get=get, now=1000)
        self.assertEqual(a['match']['track'], 'Celeste Main Theme')
        self.assertEqual(calls, ['Celeste main theme'], 'a strong match ends the search')
        self.assertEqual(self.m.lookup('Celeste', get=get, now=2000)['match']['track'], 'Celeste Main Theme')
        self.assertEqual(len(calls), 1, 'answered from the cache')
        self.assertIsNone(self.m.lookup('Nothing Here', get=get, now=1000)['match'])
        self.assertEqual(len(calls), 3, 'both searches tried')
        self.m.lookup('Nothing Here', get=get, now=1000 + 86400)
        self.assertEqual(len(calls), 3, '"no match" is kept too')
        self.m.lookup('Nothing Here', get=get, now=1000 + 8 * 86400)
        self.assertEqual(len(calls), 5, 'and asked again after a week')

    def test_errors_are_retried_after_an_hour_and_throttle_says_later(self):
        n = []

        def broken(term):
            n.append(term)
            raise OSError('offline')
        self.assertIsNone(self.m.lookup('Hollow Knight', get=broken, now=0)['match'])
        self.m.lookup('Hollow Knight', get=broken, now=60)
        self.assertEqual(len(n), 1)
        self.m.lookup('Hollow Knight', get=broken, now=4000)
        self.assertEqual(len(n), 2)
        self.assertEqual(self.m.lookup('Hades', get=lambda t: None, now=0), {'later': True})

    def test_rate_limit(self):
        slept = []
        self.assertTrue(self.m.rate_ok(100, sleep=slept.append))
        self.assertTrue(self.m.rate_ok(101, sleep=slept.append))
        self.assertAlmostEqual(slept[0], 2.5)
        for i in range(18):
            self.m.rate_ok(105 + i * 0.1, sleep=lambda s: None)
        self.assertFalse(self.m.rate_ok(140, sleep=lambda s: None), '20 a minute at most')
        self.assertTrue(self.m.rate_ok(200, sleep=lambda s: None))

    # ---- where the song comes from
    def test_order_pick_then_soundtrack_then_apple(self):
        apple = lambda term: [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')]
        s = self.m.find_song('504230', 'Celeste', [], get=apple)
        self.assertEqual((s['source'], s['title'], s['artist']), ('apple', 'Celeste Main Theme', 'Composer'))
        self.assertEqual(s['url'], 'https://audio-ssl.itunes.apple.com/p/a.m4a', 'streamed from Apple\'s address')
        fetched = []
        a = self.m.audio(s['key'], fetch=lambda u: fetched.append(u) or b'\0\0\0 ftypM4A ' + b'0' * 100)
        self.assertEqual(a['mime'], 'audio/mp4')
        self.assertTrue(base64.b64decode(a['data']).startswith(b'\0\0\0 ftyp'))
        self.assertEqual(fetched, [s['url']])
        # a soundtrack you own wins over Apple
        write(os.path.join(self.steam, 'steamapps/music/Celeste Original Soundtrack/02 - Resurrections.mp3'))
        write(os.path.join(self.steam, 'steamapps/music/Celeste Original Soundtrack/01 - Prologue.mp3'))
        s = self.m.find_song('504230', 'Celeste', [], get=apple)
        self.assertEqual((s['source'], s['title']), ('soundtrack', '01 - Prologue'))
        # your pick wins over both; a collection pick applies to its games
        song = os.path.join(self.home, 'Music/mine.ogg')
        write(song)
        self.m.set_pick('collection', 'Platformers', song)
        s = self.m.find_song('504230', 'Celeste', ['Platformers'], get=apple)
        self.assertEqual((s['source'], s['via'], s['name']), ('pick', 'collection', 'Platformers'))
        self.assertNotIn('path', s, 'paths never cross to the frontend')
        with self.assertRaises(ValueError):
            self.m.set_pick('app', '1', os.path.join(self.home, 'notes.txt'))
        self.m.set_pick('collection', 'Platformers', '')
        self.assertEqual(self.m.find_song('504230', 'Celeste', ['Platformers'], get=apple)['source'], 'soundtrack')

    def test_apple_previews_are_never_written_to_disk(self):
        apple = lambda term: [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')]
        before = set()
        for d, _, fs in os.walk(self.tmp):
            before.update(os.path.join(d, f) for f in fs)
        s = self.m.find_song('504230', 'Celeste', [], get=apple)
        audio = b'\0\0\0 ftypM4A ' + b'0' * 100
        self.m.audio(s['key'], fetch=lambda u: audio)
        seen = {}

        def run(cmd, **kw):
            seen['cmd'], seen['input'] = cmd, kw.get('input')
            return types.SimpleNamespace(returncode=0, stdout=b'OggS' + b'1' * 10)
        self.m.shutil.which = lambda n: '/usr/bin/ffmpeg'
        a = self.m.audio(s['key'], transcode=True, run=run, fetch=lambda u: audio)
        self.assertEqual(a['mime'], 'audio/ogg')
        self.assertEqual((seen['cmd'][seen['cmd'].index('-i') + 1], seen['cmd'][-1], seen['input']), ('pipe:0', 'pipe:1', audio), 'piped, not via files')
        self.assertFalse(os.path.exists(os.path.join(self.m.cache_dir(), 'previews')), 'nothing under previews/')
        after = set()
        for d, _, fs in os.walk(self.tmp):
            after.update(os.path.join(d, f) for f in fs)
        self.assertEqual(sorted(os.path.relpath(f, self.tmp) for f in after - before), ['home/.cache/hearth-music/lookups.json'],
                         'only the lookup answer is kept')
        self.assertIsNone(self.m.stream_preview('https://evil.example/a.m4a', fetch=lambda u: audio))

    def test_soundtrack_folder_in_common_and_theme_named_track(self):
        lib = os.path.join(self.tmp, 'drive/SteamLibrary')
        os.makedirs(os.path.join(lib, 'steamapps'))
        write(os.path.join(self.steam, 'steamapps/libraryfolders.vdf'), '"libraryfolders"\n{\n "1"\n {\n  "path" "%s"\n }\n}\n' % lib)
        d = os.path.join(lib, 'steamapps/common/Hades II Soundtrack')
        write(os.path.join(d, 'MP3/01 - Hades II.mp3'))
        write(os.path.join(d, 'MP3/02 - Main Theme.mp3'))
        write(os.path.join(lib, 'steamapps/common/Hades II Toolkit/readme.mp3'))
        p, where = self.m.soundtrack('1145350', 'Hades II')
        self.assertEqual((os.path.basename(p), where), ('02 - Main Theme.mp3', 'soundtrack'))

    def test_game_files_only_when_clearly_the_theme(self):
        write(os.path.join(self.steam, 'steamapps/appmanifest_620.acf'), '"AppState"\n{\n "appid" "620"\n "installdir" "Portal 2"\n}\n')
        g = os.path.join(self.steam, 'steamapps/common/Portal 2')
        write(os.path.join(g, 'sound/music/menu_click.ogg'), b'0' * 400000)
        write(os.path.join(g, 'sound/music/theme.ogg'), b'0' * 400000)          # just "theme": too vague
        write(os.path.join(g, 'sound/ui/MainTheme.ogg'), b'0' * 1000)            # too small for a song
        self.assertIsNone(self.m.soundtrack('620', 'Portal 2'))
        write(os.path.join(g, 'sound/music/MainMenu_Theme.ogg'), b'0' * 400000)
        p, where = self.m.soundtrack('620', 'Portal 2')
        self.assertEqual((os.path.basename(p), where), ('MainMenu_Theme.ogg', 'game'))

    def test_online_off_and_no_match_are_silent(self):
        self.assertEqual(self.m.find_song('1', 'Unknown Game', [], online=False)['source'], None)
        s = self.m.find_song('1', 'Unknown Game', [], get=lambda t: [])
        self.assertEqual((s['source'], s['why']), (None, 'no theme song found'))

    def test_audio_only_for_chosen_keys_and_transcode(self):
        self.assertIsNone(self.m.audio('nope'))
        f = os.path.join(self.home, 'a.m4a')
        write(f, b'm4a')
        k = self.m.keyed(f)
        seen = {}

        def run(cmd, **kw):
            seen['cmd'] = cmd
            write(cmd[-1], b'OggS')
            return types.SimpleNamespace(returncode=0)
        self.m.shutil.which = lambda n: '/usr/bin/ffmpeg'
        a = self.m.audio(k, transcode=True, run=run)
        self.assertEqual(a['mime'], 'audio/ogg')
        self.assertIn('libopus', seen['cmd'])

    def test_settings_are_cleaned(self):
        s = self.m.clean_settings({'volume': 7, 'delay': -3, 'where': 'everywhere', 'enabled': 'yes', 'online': False})
        self.assertEqual(s, dict(self.m.DEFAULTS, volume=1.0, delay=0.0, online=False))
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        run(p.set_settings({'where': 'library', 'volume': 0.5}))
        self.assertEqual(run(p.get_settings())['where'], 'library')

    def test_never_youtube(self):
        src = open(HERE / 'main.py').read().lower() + ''.join(open(f).read().lower() for f in (HERE / 'src').glob('*.ts*'))
        for bad in ('youtube.com', 'youtu.be', 'googlevideo', 'yt-dlp', 'youtube-dl', 'ytdl', 'pytube', 'invidious', 'piped.video', 'pipedapi'):
            self.assertNotIn(bad, src, bad)

    def test_plugin_json_and_version(self):
        with open(HERE / 'plugin.json') as f:
            self.assertEqual(json.load(f)['name'], 'Hearth Music')
        with open(HERE / 'package.json') as f, open(ROOT / 'hosts/decky-hearth-sync/package.json') as g:
            self.assertEqual(json.load(f)['version'], json.load(g)['version'], "the same version as Hearth's other packages")


class Shipping(unittest.TestCase):
    """hearth-decky copies Hearth Music next to Hearth Sync and keeps it up to date."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-music-decky-')
        self.old = dict(os.environ)
        os.environ['HEARTH_DECKY_ROOT'] = os.path.join(self.tmp, 'root')
        loader = importlib.machinery.SourceFileLoader('hearth_decky_m', str(ROOT / 'os/system_files/usr/libexec/nimbus/hearth-decky'))
        spec = importlib.util.spec_from_loader('hearth_decky_m', loader)
        self.d = importlib.util.module_from_spec(spec)
        loader.exec_module(self.d)
        self.home = os.path.join(self.tmp, 'home')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_copied_and_refreshed(self):
        self.assertIn('decky-hearth-music', self.d.PLUGINS)
        src = os.path.join(self.tmp, 'root/usr/share/hearth/decky-hearth-music')
        self.assertFalse(self.d.copy_plugin(self.home, owner_root=False, name='decky-hearth-music'), 'not shipped: skipped')
        write(os.path.join(src, 'plugin.json'), '{"name": "Hearth Music"}')
        write(os.path.join(src, 'dist/index.js'), '// v1')
        self.assertTrue(self.d.plugin_outdated(self.home, 'decky-hearth-music'))
        self.assertTrue(self.d.copy_plugin(self.home, owner_root=False, name='decky-hearth-music'))
        dst = os.path.join(self.home, 'homebrew/plugins/decky-hearth-music')
        self.assertTrue(os.path.isfile(os.path.join(dst, 'plugin.json')))
        self.assertFalse(self.d.plugin_outdated(self.home, 'decky-hearth-music'))
        write(os.path.join(src, 'dist/index.js'), '// v2')
        self.assertTrue(self.d.plugin_outdated(self.home, 'decky-hearth-music'))
        self.assertFalse(os.path.exists(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync')), 'Hearth Sync untouched')


if __name__ == '__main__':
    unittest.main()
