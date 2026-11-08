"""Tests for Hearth Music's backend (hosts/decky-hearth-music/main.py) and its shipping by hearth-decky. No Steam, no
Decky, no network: a stand-in `decky` module, fake Steam folders and made-up iTunes and Deezer answers (main._urlopen,
the backend's only way out, is replaced; an unexpected call fails the test)."""
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
import threading
import time
import types
import unittest
import urllib.error
import urllib.parse

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


def it(track, album, artist='Composer', url='https://audio-ssl.itunes.apple.com/p/a.m4a', n=1, tid=111):
    return {'trackId': tid, 'trackName': track, 'collectionName': album, 'artistName': artist, 'previewUrl': url, 'trackNumber': n,
            'trackViewUrl': 'https://music.apple.com/us/album/x/1?i=2'}


DZ_URL = 'https://cdnt-preview.dzcdn.net/api/1/1/a/b/c/0/abc.mp3?hdnea=exp=%d~acl=/api/1/1/a/b/c/0/abc.mp3*~hmac=f00'


def dz(track, album, artist='Composer', tid=3135556, exp=2000000000, url=None):
    """A Deezer search row, as api.deezer.com/search gives it."""
    return {'id': tid, 'title': track, 'title_short': track, 'link': 'https://www.deezer.com/track/%d' % tid,
            'preview': url or DZ_URL % exp, 'artist': {'name': artist}, 'album': {'title': album}}


def dzr(*rows):
    """Deezer rows in the shape _deezer_get hands to lookup (iTunes' shape)."""
    return lambda m: [m.deezer_result(r) for r in rows]


class Resp:
    def __init__(self, data):
        self.data = data if isinstance(data, bytes) else json.dumps(data).encode()

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False

    def read(self, n=-1):
        return self.data


def files_under(d):
    out = set()
    for dd, _, fs in os.walk(d):
        out.update(os.path.join(dd, f) for f in fs)
    return out


class Backend(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-music-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.m = load(self.tmp)
        self.m._calls.clear()
        self.net = []

        def no_network(req, timeout=None):
            self.net.append(getattr(req, 'full_url', req))
            raise AssertionError('no real network in tests: %s' % getattr(req, 'full_url', req))
        self.m._urlopen = no_network
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
        nodz = lambda t: []
        a = self.m.lookup('Celeste', get=get, now=1000, deezer=nodz)
        self.assertEqual(a['match']['track'], 'Celeste Main Theme')
        self.assertEqual(calls, ['Celeste main theme'], 'a strong match ends the search')
        self.assertEqual(self.m.lookup('Celeste', get=get, now=2000, deezer=nodz)['match']['track'], 'Celeste Main Theme')
        self.assertEqual(len(calls), 1, 'answered from the cache')
        self.assertIsNone(self.m.lookup('Nothing Here', get=get, now=1000, deezer=nodz)['match'])
        self.assertEqual(len(calls), 3, 'both searches tried')
        self.m.lookup('Nothing Here', get=get, now=1000 + 86400, deezer=nodz)
        self.assertEqual(len(calls), 3, '"no match" is kept too')
        self.m.lookup('Nothing Here', get=get, now=1000 + 8 * 86400, deezer=nodz)
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
        before = files_under(self.tmp)
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
        after = files_under(self.tmp)
        self.assertEqual(sorted(os.path.relpath(f, self.tmp) for f in after - before),
                         ['home/.cache/hearth-music/.lookups.json.lock', 'home/.cache/hearth-music/lookups.json'],
                         'only the lookup answer is kept')
        for f in after - before:
            with open(f, 'rb') as fh:
                self.assertNotIn(audio, fh.read() or b'-')
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
        self.assertEqual(self.m.find_song('1', 'Unknown Game', [], online=False, deezer=False)['source'], None)
        s = self.m.find_song('1', 'Unknown Game', [], get=lambda t: [], deezer_get=lambda t: [])
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
        self.assertEqual(s, dict(self.m.DEFAULTS, volume=1.0, delay=0.0, online=False, deezer=False),
                         'Apple turned off before Deezer existed: Deezer is off for him too')
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        run(p.set_settings({'where': 'library', 'volume': 0.5}))
        self.assertEqual(run(p.get_settings())['where'], 'library')

    # ---- Deezer, the second preview source
    def test_deezer_only_when_apple_has_no_match(self):
        asked = []
        dzget = lambda t: asked.append(t) or dzr(dz('Celeste Main Theme', 'Celeste (Original Soundtrack)', 'Lena Raine'))(self.m)
        apple = lambda t: [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')]
        self.assertEqual(self.m.lookup('Celeste', get=apple, deezer=dzget, now=0)['match']['source'], 'apple')
        self.assertEqual(asked, [], 'Apple had a good match: Deezer never asked')
        m = self.m.lookup('Hollow Knight', get=lambda t: [it('Hollow Knight', 'Some Piano Covers', 'Piano Guy')], deezer=lambda t: dzr(
            dz('Hollow Knight', 'Hollow Knight (Original Soundtrack)', 'Christopher Larkin', tid=99))(self.m), now=0)['match']
        self.assertEqual((m['source'], m['id'], m['artist']), ('deezer', '99', 'Christopher Larkin'))
        self.assertNotIn('preview', m, "Deezer's addresses expire: only the id is kept")
        raw = open(os.path.join(self.m.cache_dir(), 'lookups.json')).read()
        self.assertNotIn('dzcdn', raw)
        # Apple only asked for later (rate limit): Deezer isn't used instead
        self.assertEqual(self.m.lookup('Hades', get=lambda t: None, deezer=dzget, now=0), {'later': True})
        self.assertEqual(asked, [])
        # Apple turned off, Deezer on: Deezer alone; both off: nothing asked
        self.assertEqual(self.m.lookup('Celeste Two', get=lambda t: self.fail('Apple is off'), deezer=lambda t: dzr(
            dz('Celeste Two Main Theme', 'Celeste Two OST'))(self.m), apple=False, now=0)['match']['source'], 'deezer')
        self.assertEqual(self.m.lookup('Other', get=lambda t: self.fail('off'), deezer=lambda t: self.fail('off'), apple=False, use_deezer=False), {'match': None})

    def test_deezer_follows_the_same_rules(self):
        forms = self.m.title_forms('Hades')
        rows = lambda *r: [self.m.deezer_result(x) for x in r]
        self.assertIsNone(self.m.best_match(rows(dz('Main Theme', 'Some Other Game OST')), forms, 'deezer'))
        self.assertIsNone(self.m.best_match(rows(dz('Hades Main Theme (Cover)', 'Hades Tribute', 'Fans')), forms, 'deezer'))
        self.assertIsNone(self.m.best_match(rows(dz('Hades Main Theme', 'Hades OST', url='https://evil.example/a.mp3')), forms, 'deezer'))
        self.assertIsNone(self.m.best_match(rows(dz('Hades Main Theme', 'Hades OST', url='http://cdnt-preview.dzcdn.net/a.mp3')), forms, 'deezer'))
        self.assertIsNone(self.m.best_match([it('Hades Main Theme', 'Hades OST')], forms, 'deezer'), "an Apple address isn't Deezer's")
        self.assertEqual(self.m.best_match(rows(dz('Hades Main Theme', 'Hades OST')), forms, 'deezer')['source'], 'deezer')

    def test_deezer_preview_resolved_when_it_plays_with_attribution(self):
        self.m.lookup('Hollow Knight', get=lambda t: [], deezer=lambda t: dzr(dz('Hollow Knight Main Theme', 'Hollow Knight OST', tid=42))(self.m), now=time.time())
        asked = []
        track = lambda tid: asked.append(tid) or {'id': 42, 'preview': DZ_URL % (time.time() + 3600)}
        s = self.m.find_song('367520', 'Hollow Knight', [], get=lambda t: self.fail('cached'), deezer_track=track)
        self.assertEqual((s['source'], s['from'], s['title']), ('deezer', 'deezer', 'Hollow Knight Main Theme'))
        self.assertTrue(s['url'].startswith('https://cdnt-preview.dzcdn.net/'))
        self.assertEqual(asked, ['42'])
        self.m.find_song('367520', 'Hollow Knight', [], deezer_track=track)
        self.assertEqual(asked, ['42'], 'kept in memory until shortly before it expires')
        self.m._deezer_urls['42'] = (DZ_URL % 10, 10)
        self.m.find_song('367520', 'Hollow Knight', [], deezer_track=track)
        self.assertEqual(asked, ['42', '42'], 'expired: asked again')
        # can't be had right now: silent for a moment, the kept answer untouched
        self.m._deezer_urls.clear()
        s = self.m.find_song('367520', 'Hollow Knight', [], deezer_track=lambda t: (_ for _ in ()).throw(OSError('offline')))
        self.assertEqual((s['source'], s['later']), (None, True))
        # Deezer turned off afterwards: its kept match isn't played
        self.assertIsNone(self.m.find_song('367520', 'Hollow Knight', [], deezer=False, get=lambda t: [], deezer_track=track)['source'])

    def test_deezer_previews_are_never_written_to_disk(self):
        before = files_under(self.tmp)
        self.m.lookup('Hollow Knight', get=lambda t: [], deezer=lambda t: dzr(dz('Hollow Knight Main Theme', 'Hollow Knight OST', tid=42))(self.m), now=time.time())
        s = self.m.find_song('367520', 'Hollow Knight', [], deezer_track=lambda t: {'preview': DZ_URL % (time.time() + 3600)})
        audio = b'ID3\x03' + b'\xff\xfb' * 200
        a = self.m.audio(s['key'], fetch=lambda u: audio)
        self.assertEqual(a['mime'], 'audio/mpeg')
        self.assertEqual(base64.b64decode(a['data']), audio)
        self.m.shutil.which = lambda n: '/usr/bin/ffmpeg'
        seen = {}

        def run(cmd, **kw):
            seen['cmd'], seen['input'] = cmd, kw.get('input')
            return types.SimpleNamespace(returncode=0, stdout=b'OggS' + b'1' * 10)
        self.assertEqual(self.m.audio(s['key'], transcode=True, run=run, fetch=lambda u: audio)['mime'], 'audio/ogg')
        self.assertEqual((seen['cmd'][seen['cmd'].index('-i') + 1], seen['cmd'][-1], seen['input']), ('pipe:0', 'pipe:1', audio))
        new = files_under(self.tmp) - before
        self.assertEqual(sorted(os.path.relpath(f, self.tmp) for f in new),
                         ['home/.cache/hearth-music/.lookups.json.lock', 'home/.cache/hearth-music/lookups.json'])
        for f in new:
            with open(f, 'rb') as fh:
                data = fh.read()
            self.assertNotIn(audio, data or b'-')
            self.assertNotIn(b'dzcdn', data)

    # ---- rate limits and backing off
    def test_deezer_rate_limit_and_backoff_on_429_403(self):
        slept = []
        self.m.deezer_rate_ok(100, sleep=slept.append)
        self.m.deezer_rate_ok(100.25, sleep=slept.append)
        self.assertAlmostEqual(slept[0], 0.75, msg='one call a second')
        n = []

        def too_many(req, timeout=None):
            n.append(req.full_url)
            raise urllib.error.HTTPError(req.full_url, 429, 'Too Many Requests', {}, None)
        self.m._urlopen = too_many
        self.assertIsNone(self.m._itunes_get('Celeste main theme', now=1000))
        self.assertTrue(self.m.backed_off('apple', 1059))
        self.assertIsNone(self.m._itunes_get('Celeste soundtrack', now=1030))
        self.assertEqual(len(n), 1, 'backed off: not asked again for a minute')
        self.m._calls.clear()
        self.assertIsNone(self.m._itunes_get('x', now=1061))
        self.assertEqual(self.m._backoff['apple'], [1061 + 120, 120], 'doubles')
        self.assertEqual(self.m.lookup('Celeste', get=lambda t: self.m._itunes_get(t, now=1100), now=1100), {'later': True})

        def forbidden(req, timeout=None):
            raise urllib.error.HTTPError(req.full_url, 403, 'Forbidden', {}, None)
        self.m._urlopen = forbidden
        self.assertIsNone(self.m._deezer_get('Hades soundtrack', now=5000))
        self.assertTrue(self.m.backed_off('deezer', 5030))
        self.m._backoff.clear()
        self.m._urlopen = lambda req, timeout=None: Resp({'error': {'type': 'Exception', 'message': 'Quota limit exceeded', 'code': 4}})
        self.assertIsNone(self.m._deezer_get('Hades soundtrack', now=6000))
        self.assertTrue(self.m.backed_off('deezer', 6001), "Deezer's quota answer backs off too")
        self.m._backoff.clear()
        self.m._urlopen = lambda req, timeout=None: Resp({'data': [dz('Hades Main Theme', 'Hades OST')]})
        rows = self.m._deezer_get('Hades soundtrack', now=7000)
        self.assertEqual((rows[0]['trackName'], rows[0]['collectionName'], rows[0]['artistName']), ('Hades Main Theme', 'Hades OST', 'Composer'))
        self.assertNotIn('deezer', self.m._backoff)

    # ---- the library scan
    def manifest(self, appid, name):
        write(os.path.join(self.steam, 'steamapps/appmanifest_%s.acf' % appid),
              '"AppState"\n{\n "appid" "%s"\n "name" "%s"\n "installdir" "%s"\n}\n' % (appid, name, name))

    def test_scan_order(self):
        self.manifest('620', 'Portal 2')
        self.manifest('105600', 'Terraria')
        games = [{'appid': 367520, 'title': 'Hollow Knight'}, {'appid': 3000000001, 'title': 'Epic: Alan Wake 2', 'shortcut': True},
                 {'appid': 504230, 'title': 'Celeste'}, {'appid': 620, 'title': 'Portal 2'}, {'appid': 2999999999, 'title': 'GOG: Celeste'},
                 {'appid': 1, 'title': 'Celeste'}, {'appid': 'x', 'title': 'Bad'}, {'appid': 5, 'title': ''}]
        order = [(r['title'], r['kind']) for r in self.m.scan_order(games)]
        self.assertEqual(order, [('Portal 2', 'steam'), ('Terraria', 'steam'), ('Celeste', 'steam'), ('Hollow Knight', 'steam'),
                                 ('Epic: Alan Wake 2', 'shortcut'), ('GOG: Celeste', 'shortcut')],
                         'installed first, then the rest of the library, then non-Steam shortcuts; one per title')

    def scan_getters(self):
        asked = []

        def apple(t):
            asked.append(t)
            title = t.rsplit(' ', 2)[0] if t.endswith('main theme') else t.rsplit(' ', 1)[0]
            return [it('%s Main Theme' % title, '%s (Original Soundtrack)' % title)] if 'Nomatch' not in t else []
        return asked, apple

    def test_scan_is_resumable_and_skips_what_it_knows(self):
        games = [{'appid': i, 'title': 'Game %s' % n} for i, n in ((10, 'Alpha'), (20, 'Bravo'), (30, 'Charlie'), (40, 'Delta Nomatch'))]
        asked, apple = self.scan_getters()
        prog = []
        stop = lambda: len(prog) > 2 and prog[-1]['done'] >= 2
        st = self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 1000, sleep=lambda s: None,
                                 stopped=stop, progress=prog.append)
        self.assertEqual((st['done'], st['total']), (2, 4))
        self.assertEqual(asked, ['Game Alpha main theme', 'Game Bravo main theme'])
        asked.clear()
        prog.clear()
        st = self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 2000, sleep=lambda s: None, progress=prog.append)
        self.assertEqual(asked, ['Game Charlie main theme', 'Game Delta Nomatch main theme', 'Game Delta Nomatch soundtrack'],
                         'carries on where it stopped')
        self.assertEqual((st['matched'], st['done'], st['total']), (3, 4, 4))
        self.assertEqual(prog[-1]['matched'], 3)
        asked.clear()
        self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 3000, sleep=lambda s: None)
        self.assertEqual(asked, [], 'nothing to ask again')
        self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 1000 + 8 * 86400, sleep=lambda s: None)
        self.assertEqual(len(asked), 5, 'week-old answers refreshed')

    def test_scan_pauses_while_a_game_runs(self):
        asked, apple = self.scan_getters()
        running = [True, True, True]
        slept = []
        prog = []
        self.m.scan_library([{'appid': 10, 'title': 'Game Alpha'}], settings={}, get=lambda t: asked.append('during' if running else t) or apple(t),
                            deezer=lambda t: [], now=lambda: 0, sleep=slept.append, paused=lambda: bool(running and running.pop()),
                            progress=prog.append)
        self.assertEqual(slept, [1, 1, 1])
        self.assertNotIn('during', asked, 'nothing asked while a game runs')
        self.assertIn('game', [p['waiting'] for p in prog])
        self.assertEqual(prog[-1]['matched'], 1)

    def test_scan_waits_when_told_to_slow_down(self):
        n = [0]

        def apple(t):
            n[0] += 1
            return None if n[0] <= 2 else [it('Game Alpha Main Theme', 'Game Alpha OST')]
        slept, prog = [], []
        st = self.m.scan_library([{'appid': 10, 'title': 'Game Alpha'}], settings={}, get=apple, deezer=lambda t: [],
                                 now=lambda: 0, sleep=slept.append, progress=prog.append)
        self.assertEqual(slept, [5, 5])
        self.assertIn('slow', [p['waiting'] for p in prog])
        self.assertEqual(st['matched'], 1)

    def test_scan_keeps_local_soundtracks_without_asking(self):
        write(os.path.join(self.steam, 'steamapps/music/Celeste Original Soundtrack/01 - Prologue.mp3'))
        st = self.m.scan_library([{'appid': 504230, 'title': 'Celeste'}], settings={}, get=lambda t: self.fail('has its own'),
                                 deezer=lambda t: self.fail('has its own'), now=lambda: 0, sleep=lambda s: None)
        self.assertEqual(st['matched'], 1)
        e = self.m.load_json('lookups.json', {}, self.m.cache_dir())['celeste']
        self.assertEqual((e['local'], e['track']), ('soundtrack', '01 - Prologue'))

    def test_network_failure_leaves_the_cache_intact(self):
        apple = lambda t: [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')]
        self.m.lookup('Celeste', get=apple, deezer=lambda t: [], now=0)
        path = os.path.join(self.m.cache_dir(), 'lookups.json')
        before = open(path, 'rb').read()

        def offline(t):
            raise OSError('network unreachable')
        r = self.m.lookup('Celeste', get=offline, now=40 * 86400)
        self.assertEqual(r['match']['track'], 'Celeste Main Theme', 'the kept answer is still used')
        slept = []
        self.m.scan_library([{'appid': 1, 'title': 'Celeste'}], settings={}, get=offline, now=lambda: 40 * 86400,
                            sleep=slept.append, stopped=lambda: len(slept) >= 3)
        self.assertEqual(slept, [5, 10, 20], 'waits longer each time')
        self.assertEqual(open(path, 'rb').read(), before, 'unchanged')
        s = self.m.find_song('1', 'Celeste', [], get=offline, deezer_get=offline)
        self.assertEqual(s['source'], 'apple', 'still plays from the kept answer')

    def test_lookups_are_written_atomically_with_a_bak(self):
        apple = lambda t: [it('%s Main Theme' % t.split(' main')[0].split(' sound')[0], '%s OST' % t.split(' main')[0].split(' sound')[0])]
        self.m.lookup('Celeste', get=apple, deezer=lambda t: [], now=0)
        self.m.lookup('Hades', get=apple, deezer=lambda t: [], now=0)
        d = self.m.cache_dir()
        bak = json.load(open(os.path.join(d, 'lookups.json.bak')))
        self.assertEqual(sorted(bak), ['celeste'], 'the previous version kept')
        self.assertEqual(sorted(json.load(open(os.path.join(d, 'lookups.json')))), ['celeste', 'hades'])
        self.assertEqual([f for f in os.listdir(d) if f.endswith('.tmp')], [], 'no half-written files left')
        with open(os.path.join(d, 'lookups.json'), 'w') as f:
            f.write('{"broken')
        self.assertEqual(sorted(self.m.load_json('lookups.json', {}, d)), ['celeste'], 'a damaged file comes back from the .bak')
        # two writers at once both keep their answer (merge under the lock)
        ts = [threading.Thread(target=self.m.update_cache, args=('lookups.json', lambda c, k=k: c.__setitem__(k, {'at': 1, 'match': None})))
              for k in ('a', 'b', 'c', 'd')]
        [t.start() for t in ts]
        [t.join() for t in ts]
        self.assertTrue({'a', 'b', 'c', 'd'} <= set(self.m.load_json('lookups.json', {}, d)))

    def test_background_scan_through_the_plugin(self):
        self.m.ITUNES_GAP = 0
        self.m.DEEZER_GAP = 0

        def net(req, timeout=None):
            u = req.full_url
            if 'itunes.apple.com' in u:
                return Resp({'results': [it('Celeste Main Theme', 'Celeste (Original Soundtrack)')] if 'Celeste' in u else []})
            if 'api.deezer.com/search' in u:
                return Resp({'data': [dz('Hollow Knight Main Theme', 'Hollow Knight OST')] if 'Hollow' in u else []})
            raise AssertionError(u)
        self.m._urlopen = net
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        # Off Hearth OS the scan never starts (hosts/decky-shared/hearth_only.py), then a stand-in Hearth OS root.
        root = os.path.join(self.tmp, 'root')
        os.makedirs(os.path.join(root, 'etc'))
        open(os.path.join(root, 'etc/os-release'), 'w').write('NAME="SteamOS"\nID=steamos\n')
        old_root = os.environ.get('HEARTH_ONLY_ROOT')
        os.environ['HEARTH_ONLY_ROOT'] = root
        self.addCleanup(lambda: os.environ.pop('HEARTH_ONLY_ROOT', None) if old_root is None else os.environ.__setitem__('HEARTH_ONLY_ROOT', old_root))
        self.assertFalse(run(p.scan_library([{'appid': 504230, 'title': 'Celeste'}])))
        self.assertFalse(self.m._scan['running'])
        self.assertIsNone(self.m._scan['thread'])
        open(os.path.join(root, 'etc/os-release'), 'w').write('NAME="Hearth"\nVARIANT_ID=hearth\n')
        os.makedirs(os.path.join(root, 'usr/libexec/nimbus'))
        self.assertTrue(run(p.scan_library([{'appid': 504230, 'title': 'Celeste'}, {'appid': 367520, 'title': 'Hollow Knight'},
                                            {'appid': 3000000001, 'title': 'Nothing At All', 'shortcut': True}])))
        for _ in range(200):
            if not self.m._scan['running']:
                break
            time.sleep(0.02)
        st = run(p.scan_status())
        self.assertEqual((st['running'], st['matched'], st['total']), (False, 2, 3))
        c = self.m.load_json('lookups.json', {}, self.m.cache_dir())
        self.assertEqual((c['celeste']['match']['source'], c['hollow knight']['match']['source'], c['nothing at all']['match']), ('apple', 'deezer', None))
        self.assertTrue(os.path.isfile(os.path.join(self.m.cache_dir(), 'scan.json')))
        self.assertTrue(run(p.scan_pause(True)))
        self.assertTrue(self.m._scan['paused'])

    # ---- "Choose music…" and your picks
    def test_choose_music_searches_everything_and_the_pick_wins_over_scans(self):
        write(os.path.join(self.steam, 'steamapps/music/Hades Original Soundtrack/01 - No Escape.mp3'))
        write(os.path.join(self.steam, 'steamapps/music/Hades Original Soundtrack/02 - Main Theme.mp3'))
        apple = lambda t: [it('Hades Main Theme', 'Hades OST', tid=5), it('Hades', 'Hades', url='https://evil.example/x.m4a', tid=6)]
        dzget = lambda t: [self.m.deezer_result(dz('In the Blood', 'Hades (Original Soundtrack)', 'Darren Korb', tid=77))]
        r = self.m.search_music('Hades', get=apple, deezer_get=dzget)
        self.assertEqual([(x['from'], x['title']) for x in r['results']],
                         [('soundtrack', '02 - Main Theme'), ('soundtrack', '01 - No Escape'), ('apple', 'Hades Main Theme'), ('deezer', 'In the Blood')])
        self.assertTrue(all('path' not in x and 'preview' not in x for x in r['results']), 'no paths or addresses cross the bridge')
        self.assertEqual(self.m.search_music('Hades', apple=False, deezer=False, get=lambda t: self.fail('off'))['results'][0]['from'], 'soundtrack')
        self.assertEqual(self.m.search_music('Hades', get=lambda t: None, deezer_get=dzget)['later'], ['apple'])
        # the automatic match first, then your pick (a Deezer track) beats it and survives a rescan
        self.m.lookup('Hades', get=apple, deezer=lambda t: [], now=0)
        choice = [x['choice'] for x in r['results'] if x['from'] == 'deezer'][0]
        self.assertFalse(self.m.choose_music('1145360', 'nope'))
        self.assertTrue(self.m.choose_music('1145360', choice))
        picks_path = os.path.join(self.m.settings_dir(), 'picks.json')
        picks = open(picks_path, 'rb').read()
        self.assertNotIn(b'dzcdn', picks)
        track = lambda tid: {'preview': DZ_URL % (time.time() + 3600)}
        s = self.m.find_song('1145360', 'Hades', [], get=lambda t: self.fail('your pick'), deezer_track=track)
        self.assertEqual((s['source'], s['from'], s['title'], s['artist']), ('pick', 'deezer', 'In the Blood', 'Darren Korb'))
        st = self.m.scan_library([{'appid': 1145360, 'title': 'Hades'}], settings={}, get=lambda t: self.fail('pick: never looked up'),
                                 now=lambda: 10 * 86400, sleep=lambda s: None, force=True)
        self.assertEqual(st['matched'], 1)
        self.assertEqual(open(picks_path, 'rb').read(), picks, 'a scan never touches your picks')
        # a soundtrack track chosen the same way
        local = [x['choice'] for x in r['results'] if x['from'] == 'soundtrack'][1]
        self.assertTrue(self.m.choose_music('1145360', local))
        self.assertEqual(self.m.find_song('1145360', 'Hades', [])['title'], '01 - No Escape')
        # an Apple pick while Apple's previews are off stays silent and says why
        self.assertTrue(self.m.choose_music('1145360', 'apple:5'))
        s = self.m.find_song('1145360', 'Hades', [], online=False)
        self.assertEqual((s['source'], s['why']), (None, 'your pick: Apple Music previews are off'))
        self.assertEqual(self.m.find_song('1145360', 'Hades', [])['from'], 'apple')
        with self.assertRaises(ValueError):
            self.m.set_pick('app', '1', {'source': 'youtube', 'id': '1'})

    def test_deezer_switch_defaults_on_and_is_remembered(self):
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        self.assertTrue(run(p.get_settings())['deezer'], 'on by default')
        self.assertFalse(run(p.set_settings({'deezer': False}))['deezer'])
        self.assertFalse(run(p.get_settings())['deezer'], 'remembered')
        run(p.set_settings({'volume': 0.6}))
        run(p.set_settings({'online': True}))
        self.assertFalse(run(p.get_settings())['deezer'], 'other changes never turn it back on')
        path = os.path.join(self.m.settings_dir(), 'settings.json')
        self.assertEqual(json.load(open(path)), {'deezer': False, 'volume': 0.6, 'online': True}, 'only the keys changed are written')
        self.assertTrue(os.path.isfile(path + '.bak'))
        with open(path, 'w') as f:
            f.write('{broken')
        self.assertFalse(run(p.get_settings())['deezer'], 'a damaged file comes back from the .bak, not the defaults')
        self.assertEqual(json.load(open(path)).get('deezer'), False)
        self.assertTrue(run(p.set_settings({'deezer': True}))['deezer'])

    def test_youtube_only_through_the_official_embed_and_api(self):
        """YouTube only through its official embed (the mini player) and the Data API's search: never a downloader,
        Invidious/Piped, YouTube's internal API, its media servers, or anything that saves or extracts media."""
        import re
        files = [HERE / 'main.py'] + sorted((HERE / 'src').glob('*.ts*'))
        src = ''.join(open(f).read().lower() for f in files)
        for bad in ('yt-dlp', 'yt_dlp', 'youtube-dl', 'youtube_dl', 'ytdl', 'pytube', 'pafy', 'youtubei', 'googlevideo',
                    'videoplayback', 'get_video_info', 'ytinitialplayerresponse', 'player_response', 'invidious',
                    'piped.video', 'pipedapi', 'piped.kavin', 'newpipe', 'ytmusicapi', 'iframe_api', 'adblock'):
            self.assertNotIn(bad, src, bad)
        hosts = set()
        for f in files:
            for h in re.findall(r'https?://([a-z0-9.-]+)', open(f).read().lower()):
                if 'youtu' in h or 'google' in h:
                    hosts.add(h)
        self.assertLessEqual(hosts, {'www.youtube-nocookie.com', 'www.youtube.com', 'www.googleapis.com'}, hosts)
        # The only YouTube address the backend ever asks is the Data API; the embed is the frontend's.
        main = open(HERE / 'main.py').read()
        self.assertIn("YT_API = 'https://www.googleapis.com/youtube/v3'", main)
        self.assertNotIn('youtube-nocookie.com/embed', main.replace('www.youtube-nocookie.com/embed/<id>', ''))
        video = open(HERE / 'src/video.tsx').read()
        self.assertIn('https://www.youtube-nocookie.com', open(HERE / 'src/model.ts').read())
        for bad in ('fetch(', 'XMLHttpRequest', 'new Audio', 'MediaRecorder', 'captureStream', 'createMediaElementSource', 'localStorage', 'caches.'):
            self.assertNotIn(bad, video, bad)

    def test_plugin_json_and_version(self):
        with open(HERE / 'plugin.json') as f:
            self.assertEqual(json.load(f)['name'], 'Hearth Music')
        with open(HERE / 'package.json') as f, open(ROOT / 'hosts/decky-hearth-sync/package.json') as g:
            self.assertEqual(json.load(f)['version'], json.load(g)['version'], "the same version as Hearth's other packages")


class Matching(unittest.TestCase):
    """Matcher 2: the game's own soundtrack or nothing (made-up answers in Apple's and Deezer's real shapes)."""
    setUp = Backend.setUp
    tearDown = Backend.tearDown

    def best(self, title, rows, source='apple'):
        m = self.m.best_match(rows, self.m.match_forms(title), source)
        return m and (m['track'], m['album'])

    def test_generic_titles_need_the_games_own_soundtrack(self):
        control = [it('Control', 'Badlands (Deluxe Edition)', 'Halsey', tid=1),
                   it('Control (Main Theme)', 'Mind Control', 'DJ Someone', tid=2),
                   it('Control', 'Golden Wings', 'Zoe Wees', tid=3)]
        self.assertIsNone(self.best('Control Ultimate Edition', control), 'pop songs called "Control" are not the game')
        self.assertEqual(self.best('Control Ultimate Edition', control + [it('Control Theme', 'CONTROL (Original Soundtrack)', 'Petri Alanko', n=1)]),
                         ('Control Theme', 'CONTROL (Original Soundtrack)'))
        prey = [it('Prey', 'Prey (Original Motion Picture Soundtrack)', 'Sarah Schachner'),
                it('Prey Main Theme', 'Prey (Music from the Hulu Film)', 'Sarah Schachner'),
                it('Prey', 'Prey', 'The Neighbourhood')]
        self.assertIsNone(self.best('Prey', prey), "the film's score is not the game's")
        self.assertEqual(self.best('Prey', prey + [it('Mind Game', 'Prey (Original Game Soundtrack)', 'Mick Gordon', n=2)])[1],
                         'Prey (Original Game Soundtrack)')
        inside = [it('Inside Out Theme', 'Inside Out (Original Motion Picture Soundtrack)', 'Michael Giacchino'),
                  it('Bundle of Joy', 'Inside Out (Original Score)', 'Michael Giacchino'),
                  it('Inside (Main Theme)', 'INSIDE (The Songs)', 'Bo Burnham'),
                  it('Inside', 'Inside - Single', 'Chris Stapleton')]
        self.assertIsNone(self.best('INSIDE', inside), '"Inside Out" and other "Inside"s are not INSIDE')
        self.assertEqual(self.best('INSIDE', inside + [it('Inside Main Theme', 'INSIDE (Original Soundtrack)', 'Martin Stig Andersen')]),
                         ('Inside Main Theme', 'INSIDE (Original Soundtrack)'))

    def test_inside_never_plays_fallout_4(self):
        """The owner's report: INSIDE (Playdead, 304430) played Fallout 4's music. Matcher 1 took any soundtrack album
        with "inside" anywhere in a track name ("Inside the Vault" on Fallout 4's soundtrack scored 5, enough)."""
        fallout = [it('Inside the Vault', 'Fallout 4 (Original Game Soundtrack)', 'Inon Zur', tid=41, n=7),
                   it('Main Title', 'Fallout 4 (Original Game Soundtrack)', 'Inon Zur', tid=42, n=1),
                   it('Inside Vault 111', 'Fallout 4: Vault-Tec Edition (Music from the Game)', 'Inon Zur', tid=43)]
        self.assertIsNone(self.best('INSIDE', fallout), "Fallout 4's music isn't INSIDE's")
        dzrows = [self.m.deezer_result(dz('Inside the Vault', 'Fallout 4 (Original Game Soundtrack)', 'Inon Zur', tid=51))]
        self.assertIsNone(self.best('INSIDE', dzrows, 'deezer'), 'nor through Deezer')
        real = it('Inside', 'INSIDE (Original Soundtrack)', 'Martin Stig Andersen', tid=44, n=1)
        self.assertEqual(self.best('INSIDE', fallout + [real]), ('Inside', 'INSIDE (Original Soundtrack)'))
        # The wrong answer kept on the device from 1.2: never played (not even offline), looked up again once.
        stale = {'inside': {'at': time.time(), 'tried': ['apple'], 'match': {
            'source': 'apple', 'id': '41', 'track': 'Inside the Vault', 'artist': 'Inon Zur', 'album': 'Fallout 4 (Original Game Soundtrack)',
            'link': '', 'score': 5, 'preview': 'https://audio-ssl.itunes.apple.com/p/fallout.m4a'}}}
        self.m.save_cache('lookups.json', stale)
        offline = lambda t: (_ for _ in ()).throw(OSError('offline'))
        self.assertIsNone(self.m.find_song('304430', 'INSIDE', [], get=offline, deezer_get=offline)['source'])
        self.m.save_cache('lookups.json', stale)
        asked = []
        s = self.m.find_song('304430', 'INSIDE', [], get=lambda t: asked.append(t) or fallout + [real])
        self.assertEqual((s['source'], s['artist'], s['album']), ('apple', 'Martin Stig Andersen', 'INSIDE (Original Soundtrack)'))
        self.assertEqual(asked, ['INSIDE main theme'])
        self.m.find_song('304430', 'INSIDE', [], get=lambda t: self.fail('checked once, then kept'))
        self.assertEqual(self.m.load_json('lookups.json', {}, self.m.cache_dir())['inside']['v'], self.m.MATCHER)

    def test_same_name_songs_sequels_and_covers_are_left_out(self):
        self.assertIsNone(self.best('Celeste', [it('Celeste', 'Celeste', 'Ezra Vine'), it('Celeste', 'Not Your Muse', 'Celeste')]))
        self.assertIsNone(self.best('Hades', [it('No Escape', 'Hades II (Original Soundtrack)', 'Darren Korb')]), "Hades II's music isn't Hades'")
        self.assertIsNone(self.best('Hollow Knight: Silksong', [it('Enter Hallownest', 'Hollow Knight (Original Soundtrack)', 'Christopher Larkin')]))
        self.assertIsNone(self.best('DOOM Eternal', [it('At Doom\'s Gate', 'DOOM (Original Game Soundtrack)', 'Mick Gordon')]))
        self.assertIsNone(self.best('Celeste', [it('Celeste Main Theme', 'Video Game Themes Vol. 3', 'The Video Game Players')]), 'a compilation')
        self.assertIsNone(self.best('Hollow Knight', [it('Hollow Knight Main Theme', 'Hollow Knight (Piano Collections)', 'Piano Guy')]))
        self.assertIsNone(self.best('ELDEN RING', [it('Elden Ring Theme (Epic Version)', 'Elden Ring (Original Soundtrack)')]))
        # a two-word title's clearly named theme is enough when the album names the game
        self.assertEqual(self.best('Hollow Knight', [it('Hollow Knight Main Theme', 'Hollow Knight Music', 'Christopher Larkin')])[0],
                         'Hollow Knight Main Theme')
        # but never for a one-word title
        self.assertIsNone(self.best('Celeste', [it('Celeste Main Theme', 'Celeste Music', 'Someone')]))
        # the theme of the game's own album wins over its other tracks
        self.assertEqual(self.best('Hades', [it('In the Blood', 'Hades (Original Soundtrack)', n=12), it('No Escape', 'Hades (Original Soundtrack)', n=1),
                                             it('Hades Main Theme', 'Hades (Original Soundtrack)', n=30)])[0], 'Hades Main Theme')

    def test_steam_names_with_marks_editions_and_years(self):
        self.assertEqual(self.m.clean_title('The Witcher® 3: Wild Hunt - Game of the Year Edition'), 'The Witcher 3: Wild Hunt')
        self.assertEqual(self.m.clean_title('DARK SOULS™ III: Deluxe Edition'), 'DARK SOULS III')
        self.assertEqual(self.m.clean_title('Tomb Raider (2013)'), 'Tomb Raider')
        self.assertEqual(self.m.clean_title('The Elder Scrolls V: Skyrim Special Edition'), 'The Elder Scrolls V: Skyrim')
        self.assertEqual(self.m.clean_title('Dark Souls Remastered'), 'Dark Souls')
        self.assertEqual(self.m.clean_title('Epic: Alan Wake 2'), 'Alan Wake 2', "a store's name on a shortcut")
        self.assertEqual(self.m.clean_title('Hades (GOG)'), 'Hades')
        self.assertEqual(self.m.clean_title('Prey (2017)'), 'Prey')
        self.assertEqual(self.best('The Witcher® 3: Wild Hunt - Game of the Year Edition',
                                   [it('The Witcher 3: Wild Hunt', 'The Witcher 3: Wild Hunt (Original Game Soundtrack)', 'Marcin Przybyłowicz')])[1],
                         'The Witcher 3: Wild Hunt (Original Game Soundtrack)')
        self.assertTrue(self.best('DARK SOULS™ III: Deluxe Edition', [it('Dark Souls III', 'Dark Souls 3 (Original Soundtrack)', 'Motoi Sakuraba')]))
        self.assertTrue(self.best("Sid Meier's Civilization® VI", [it('Sogno di Volare', 'Civilization VI (Original Game Soundtrack)', 'Christopher Tin')]))
        asked = []
        self.m.lookup('Tomb Raider (2013)', get=lambda t: asked.append(t) or [], deezer=lambda t: [], now=0)
        self.assertEqual(asked, ['Tomb Raider main theme', 'Tomb Raider soundtrack'], 'searched without the year')
        asked.clear()
        self.m.lookup('DARK SOULS™ III: Deluxe Edition', get=lambda t: asked.append(t) or [], deezer=lambda t: [], now=0)
        self.assertEqual(asked[0], 'DARK SOULS III main theme')

    def test_deezer_fallback_follows_the_same_strict_rules(self):
        rows = lambda *r: [self.m.deezer_result(x) for x in r]
        self.assertIsNone(self.best('Control', rows(dz('Control', 'Badlands', 'Halsey')), 'deezer'))
        self.assertIsNone(self.best('Prey', rows(dz('Prey', 'Prey (Original Motion Picture Soundtrack)')), 'deezer'))
        piano = dz('Hollow Knight', 'Hollow Knight (Original Soundtrack)', 'Christopher Larkin')
        piano['title_version'] = '(Piano Version)'
        self.assertIsNone(self.best('Hollow Knight', rows(piano), 'deezer'), "Deezer's title_version is checked too")
        # Apple only has wrong songs: Deezer's right one is used; Deezer's only wrong ones: silence
        m = self.m.lookup('Control', get=lambda t: [it('Control', 'Badlands', 'Halsey')],
                          deezer=lambda t: dzr(dz('Control', 'Badlands', 'Halsey', tid=5), dz('Control Theme', 'Control (Original Soundtrack)', 'Petri Alanko', tid=6))(self.m), now=0)
        self.assertEqual((m['match']['source'], m['match']['id']), ('deezer', '6'))
        m = self.m.lookup('Prey', get=lambda t: [it('Prey', 'Prey', 'The Neighbourhood')],
                          deezer=lambda t: dzr(dz('Prey', 'Prey (Original Motion Picture Soundtrack)', tid=7), dz('Prey', 'Wolves', 'Someone', tid=8))(self.m), now=0)
        self.assertIsNone(m['match'], 'nothing rather than a wrong song')

    def test_old_matches_are_checked_again_once_and_picks_are_kept(self):
        d = self.m.cache_dir()
        old = {'control': {'at': 1000, 'tried': ['apple'], 'match': {'source': 'apple', 'id': '1', 'track': 'Control', 'artist': 'Halsey',
                                                                      'album': 'Badlands', 'link': '', 'score': 5,
                                                                      'preview': 'https://audio-ssl.itunes.apple.com/p/halsey.m4a'}},
               'celeste': {'at': 1000, 'tried': ['apple'], 'match': {'source': 'apple', 'id': '2', 'track': 'Celeste Main Theme', 'artist': 'Lena Raine',
                                                                      'album': 'Celeste (Original Soundtrack)', 'link': '', 'score': 13,
                                                                      'preview': 'https://audio-ssl.itunes.apple.com/p/celeste.m4a'}}}
        self.m.save_cache('lookups.json', old)
        # The owner chose a song himself for INSIDE ("Choose music…"), even one the matcher would never pick.
        self.m.set_pick('app', '304430', {'source': 'apple', 'id': '9', 'track': 'Inside', 'artist': 'Chris Stapleton', 'album': 'Inside - Single',
                                          'preview': 'https://audio-ssl.itunes.apple.com/p/mine.m4a'})
        picks_path = os.path.join(self.m.settings_dir(), 'picks.json')
        picks = open(picks_path, 'rb').read()
        offline = lambda t: (_ for _ in ()).throw(OSError('offline'))
        # offline: the old wrong song is never played; the old right one still is
        self.assertEqual(self.m.find_song('870780', 'Control', [], get=offline, deezer_get=offline)['source'], None)
        self.assertEqual(self.m.find_song('504230', 'Celeste', [], get=offline, deezer_get=offline)['source'], 'apple')
        asked = []

        def apple(t):
            asked.append(t)
            if t.startswith('Control'):
                return [it('Control', 'Badlands', 'Halsey'), it('Control Theme', 'CONTROL (Original Soundtrack)', 'Petri Alanko', tid=3)]
            return [it('Celeste Main Theme', 'Celeste (Original Soundtrack)', 'Lena Raine', tid=2)]
        games = [{'appid': 870780, 'title': 'Control Ultimate Edition'}, {'appid': 504230, 'title': 'Celeste'}, {'appid': 304430, 'title': 'INSIDE'}]
        old['control ultimate edition'] = old.pop('control')
        self.m.save_cache('lookups.json', old)
        self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 2000, sleep=lambda s: None)
        self.assertEqual(sorted(asked), ['Celeste main theme', 'Control main theme'], 'old answers looked up again (fresh or not); your pick never')
        c = self.m.load_json('lookups.json', {}, d)
        self.assertEqual((c['control ultimate edition']['match']['artist'], c['control ultimate edition']['v']), ('Petri Alanko', self.m.MATCHER))
        self.assertEqual(c['celeste']['match']['id'], '2')
        asked.clear()
        self.m.scan_library(games, settings={}, get=apple, deezer=lambda t: [], now=lambda: 3000, sleep=lambda s: None)
        self.assertEqual(asked, [], 'only once')
        self.assertEqual(open(picks_path, 'rb').read(), picks, 'your picks are never changed')
        s = self.m.find_song('304430', 'INSIDE', [], get=lambda t: self.fail('your pick'))
        self.assertEqual((s['source'], s['artist']), ('pick', 'Chris Stapleton'), 'your pick plays, whatever the matcher thinks')
        self.assertTrue(os.path.isfile(os.path.join(d, 'lookups.json.bak')), 'written with a .bak')


class Review(unittest.TestCase):
    """ "Scan all games", the review list and fixing a game from it."""
    setUp = Backend.setUp
    tearDown = Backend.tearDown
    manifest = Backend.manifest

    def library(self):
        for appid, name in (('504230', 'Celeste'), ('870780', 'Control Ultimate Edition'), ('304430', 'INSIDE'), ('1145360', 'Hades')):
            self.manifest(appid, name)
        return [{'appid': 504230, 'title': 'Celeste'}, {'appid': 870780, 'title': 'Control Ultimate Edition'},
                {'appid': 304430, 'title': 'INSIDE'}, {'appid': 1145360, 'title': 'Hades'},
                {'appid': 3000000005, 'title': 'Epic: Alan Wake 2', 'shortcut': True},
                {'appid': 620, 'title': 'Portal 2'}]   # in the library, not installed

    def apple(self, asked):
        def get(t):
            asked.append(t)
            if t.startswith('Celeste'):
                return [it('Celeste Main Theme', 'Celeste (Original Soundtrack)', 'Lena Raine', tid=2)]
            if t.startswith('Control'):
                return [it('Control', 'Badlands', 'Halsey'), it('Control Theme', 'CONTROL (Original Soundtrack)', 'Petri Alanko', tid=3)]
            if t.startswith('Alan Wake'):
                return [it('Herald of Darkness', 'Alan Wake 2 (Original Soundtrack)', 'Old Gods of Asgard', tid=4)]
            return []
        return get

    def test_scan_all_rechecks_every_installed_game_and_never_touches_picks(self):
        games = self.library()
        # an automatic answer from minutes ago (current matcher), and the owner's own picks
        self.m.save_cache('lookups.json', {'celeste': {'at': 1000, 'v': self.m.MATCHER, 'tried': ['apple', 'deezer'], 'match': None}})
        self.m.set_pick('app', '1145360', {'source': 'deezer', 'id': '77', 'track': 'In the Blood', 'artist': 'Darren Korb', 'album': 'Hades (Original Soundtrack)'})
        self.assertTrue(self.m.no_music('304430'))
        picks_path = os.path.join(self.m.settings_dir(), 'picks.json')
        picks = open(picks_path, 'rb').read()
        asked = []
        self.m.scan_library(games, settings={}, get=self.apple(asked), deezer=lambda t: [], now=lambda: 1100, sleep=lambda s: None)
        self.assertNotIn('Celeste main theme', asked, 'the ordinary scan trusts a recent answer')
        asked.clear()
        prog = []
        st = self.m.scan_library(games, settings={}, get=self.apple(asked), deezer=lambda t: [], now=lambda: 1200, sleep=lambda s: None,
                                 recheck=True, progress=prog.append)
        self.assertEqual(st['total'], 5, 'installed games and the shortcut; Portal 2 is not installed')
        self.assertIn('Celeste main theme', asked, 'Scan all games ignores the kept answers')
        self.assertFalse([t for t in asked if t.startswith(('Hades', 'INSIDE'))], 'your picks are never looked up')
        self.assertIn('Celeste', [p['current'] for p in prog])
        self.assertEqual(open(picks_path, 'rb').read(), picks, 'nor changed')
        # stopped part way ("Cancel"): what it found is kept
        asked.clear()
        stop = lambda: len(asked) >= 1
        st = self.m.scan_library(games, settings={}, get=self.apple(asked), deezer=lambda t: [], now=lambda: 1300, sleep=lambda s: None,
                                 recheck=True, stopped=stop)
        self.assertLess(st['done'], st['total'])

    def test_review_list_groups_and_fixing_a_game(self):
        games = self.library()
        asked = []
        self.m.set_pick('app', '1145360', {'source': 'deezer', 'id': '77', 'track': 'In the Blood', 'artist': 'Darren Korb', 'album': 'Hades (Original Soundtrack)'})
        self.m.scan_library(games, settings={}, get=self.apple(asked), deezer=lambda t: [], now=lambda: 1000, sleep=lambda s: None, recheck=True)
        rows = {r['title']: r for r in self.m.review(games)}
        self.assertEqual(sorted(rows), ['Celeste', 'Control Ultimate Edition', 'Epic: Alan Wake 2', 'Hades', 'INSIDE'])
        self.assertEqual((rows['Celeste']['group'], rows['Celeste']['song']['from'], rows['Celeste']['song']['artist']), ('auto', 'apple', 'Lena Raine'))
        self.assertEqual(rows['Control Ultimate Edition']['song']['album'], 'CONTROL (Original Soundtrack)')
        self.assertEqual((rows['Control Ultimate Edition']['query']), 'Control', 'the search box starts with the cleaned title')
        self.assertEqual((rows['INSIDE']['group'], rows['INSIDE']['checked']), ('none', True))
        self.assertEqual((rows['Hades']['group'], rows['Hades']['song']['from']), ('pick', 'deezer'))
        self.assertEqual(rows['Epic: Alan Wake 2']['group'], 'auto')
        # "No music for this game": a pick, silent, kept by scans
        self.assertTrue(self.m.no_music('504230'))
        s = self.m.find_song('504230', 'Celeste', [], get=lambda t: self.fail('your pick: silent'))
        self.assertEqual((s['source'], s['pick']), (None, 'none'))
        self.assertEqual({r['title']: r for r in self.m.review(games)}['Celeste']['song']['from'], 'none')
        picks_path = os.path.join(self.m.settings_dir(), 'picks.json')
        self.assertTrue(os.path.isfile(picks_path + '.bak'), 'picks saved with a .bak')
        self.m.scan_library(games, settings={}, get=self.apple([]), deezer=lambda t: [], now=lambda: 2000, sleep=lambda s: None, recheck=True)
        self.assertEqual(json.load(open(picks_path))['app:504230'], {'source': 'none'})
        # "Back to automatic"
        self.m.set_pick('app', '504230', '')
        self.assertEqual({r['title']: r for r in self.m.review(games)}['Celeste']['group'], 'auto')
        self.assertEqual(self.m.find_song('504230', 'Celeste', [], get=self.apple([]))['source'], 'apple')

    def test_choices_can_be_heard_before_choosing(self):
        write(os.path.join(self.steam, 'steamapps/music/Hades Original Soundtrack/02 - Main Theme.mp3'))
        apple = lambda t: [it('Hades Main Theme', 'Hades (Original Soundtrack)', tid=5)]
        dzget = lambda t: [self.m.deezer_result(dz('In the Blood', 'Hades (Original Soundtrack)', 'Darren Korb', tid=77))]
        r = self.m.search_music('Hades', get=apple, deezer_get=dzget)
        by = {x['from']: x['choice'] for x in r['results']}
        a = self.m.choice_preview(by['apple'])
        self.assertEqual(a['url'], 'https://audio-ssl.itunes.apple.com/p/a.m4a')
        d = self.m.choice_preview(by['deezer'], deezer_get=lambda tid: {'preview': DZ_URL % (time.time() + 3600)})
        self.assertTrue(d['url'].startswith('https://cdnt-preview.dzcdn.net/'))
        loc = self.m.choice_preview(by['soundtrack'])
        self.assertNotIn('url', loc)
        self.assertTrue(self.m._keys[loc['key']].endswith('02 - Main Theme.mp3'))
        self.assertIsNone(self.m.choice_preview('nope'))
        self.assertFalse(os.path.exists(os.path.join(self.m.settings_dir(), 'picks.json')), 'hearing one picks nothing')

    def test_a_kept_answer_plays_while_the_service_asks_to_wait(self):
        self.m.save_cache('lookups.json', {'celeste': {'at': 0, 'tried': ['apple'], 'match': {
            'source': 'apple', 'id': '2', 'track': 'Celeste Main Theme', 'artist': 'Lena Raine', 'album': 'Celeste (Original Soundtrack)',
            'link': '', 'preview': 'https://audio-ssl.itunes.apple.com/p/celeste.m4a'}}})
        s = self.m.find_song('504230', 'Celeste', [], get=lambda t: None)
        self.assertEqual(s['source'], 'apple', 'an old answer that fits today\'s rules plays while matcher 2 waits to check it')
        self.m.save_cache('lookups.json', {'inside': {'at': 0, 'tried': ['apple'], 'match': {
            'source': 'apple', 'id': '41', 'track': 'Inside the Vault', 'artist': 'Inon Zur', 'album': 'Fallout 4 (Original Game Soundtrack)',
            'link': '', 'preview': 'https://audio-ssl.itunes.apple.com/p/f.m4a'}}})
        self.assertIsNone(self.m.find_song('304430', 'INSIDE', [], get=lambda t: None)['source'], 'a wrong old answer never')

    def test_scan_all_through_the_plugin_reports_progress_and_cancels(self):
        self.assertTrue(self.m.cancel_scan())
        self.m._scan.update(running=True, all=True, total=40, done=12, current='Celeste')
        st = self.m.scan_status()
        self.assertEqual((st['all'], st['done'], st['total'], st['current']), (True, 12, 40, 'Celeste'))
        self.m.cancel_scan()
        self.assertTrue(self.m._scan['stop'])
        self.m._scan.update(running=False, all=False)
        st = self.m.scan_status()
        self.assertTrue(st['cancelled'])
        self.assertEqual(st['done'], 12, 'how far it got')


FAKE_KEY = 'AIzaTESTfakeKEY0000000000000000000000'   # a made-up key: tests never see a real one
VID = 'dQw4w9WgXcQ'


def yv(vid, title, channel='Some Channel', desc=''):
    """A video as the backend keeps search results ({'id', 'title', 'channel', 'description'})."""
    return {'id': vid, 'title': title, 'channel': channel, 'description': desc}


def yt_item(vid, title, channel='Some Channel', desc=''):
    """A search.list item in the Data API's own shape (HTML entities as YouTube sends them)."""
    return {'kind': 'youtube#searchResult', 'id': {'kind': 'youtube#video', 'videoId': vid},
            'snippet': {'title': title, 'channelTitle': channel, 'description': desc}}


def http_error(url, code, reason=''):
    import io
    body = json.dumps({'error': {'code': code, 'message': reason, 'errors': [{'reason': reason}]}}).encode()
    return urllib.error.HTTPError(url, code, reason, {}, io.BytesIO(body))


class YouTube(unittest.TestCase):
    """The YouTube mini player's backend: the person's key, link parsing, the official search with made-up answers,
    scoring by the plugin's own rules, quota, the cache, the order of sources and the settings."""
    setUp = Backend.setUp
    tearDown = Backend.tearDown

    def api(self, answers):
        """Stands in for googleapis.com: each request is recorded; ANSWERS (a function of the request's address) gives a
        dict to answer or an exception to raise. Anything not to www.googleapis.com/youtube/v3 fails the test."""
        self.asked = []

        def fake(req, timeout=None):
            url = req.full_url
            self.asked.append(url)
            u = urllib.parse.urlparse(url)
            assert u.scheme == 'https' and u.hostname == 'www.googleapis.com' and u.path.startswith('/youtube/v3/'), url
            a = answers(url)
            if isinstance(a, Exception):
                raise a
            return Resp(a)
        self.m._urlopen = fake
        self.m._yt_calls.clear()

    def test_links_are_parsed(self):
        ok = ['https://www.youtube.com/watch?v=%s' % VID, 'youtube.com/watch?feature=share&v=%s&t=42' % VID,
              'https://youtu.be/%s?si=abc' % VID, 'https://m.youtube.com/watch?v=%s' % VID,
              'https://www.youtube.com/shorts/%s' % VID, 'https://music.youtube.com/watch?v=%s&list=x' % VID,
              'https://www.youtube-nocookie.com/embed/%s' % VID, '  https://youtube.com/live/%s  ' % VID]
        for link in ok:
            self.assertEqual(self.m.youtube_id(link), VID, link)
        for bad in ('', 'hello', 'https://www.youtube.com/watch?v=short', 'https://evil.example/watch?v=%s' % VID,
                    'https://youtube.com.evil.example/watch?v=%s' % VID, 'https://www.youtube.com/channel/UC123',
                    'javascript:alert(1)//youtu.be/%s' % VID):
            self.assertIsNone(self.m.youtube_id(bad), bad)
        self.assertEqual(self.m.youtube_watch(VID), 'https://www.youtube.com/watch?v=%s' % VID)
        self.assertEqual(self.m.youtube_watch('bad'), '')

    def test_key_is_kept_0600_never_logged_never_handed_out(self):
        lines = []
        self.m.decky.logger = types.SimpleNamespace(info=lambda msg: lines.append(msg))
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        self.assertFalse(run(p.youtube_status())['key'])
        st = run(p.set_youtube_key(FAKE_KEY))
        self.assertTrue(st['saved'] and st['key'])
        path = self.m.youtube_key_file()
        self.assertEqual(os.stat(path).st_mode & 0o777, 0o600)
        self.assertEqual(os.path.dirname(path), self.m.settings_dir(), "only the plugin's own settings folder")
        run(p.set_youtube_key(FAKE_KEY[:-1] + 'X'))
        self.assertEqual(os.stat(path + '.bak').st_mode & 0o777, 0o600, 'the .bak too')
        self.assertEqual(self.m.youtube_key(), FAKE_KEY[:-1] + 'X')
        self.assertFalse(run(p.set_youtube_key('not a key!'))['saved'], 'not shaped like a key: nothing changes')
        self.assertEqual(self.m.youtube_key(), FAKE_KEY[:-1] + 'X')
        # Never in settings.json (Hearth Link syncs keys of that file), never in what the frontend gets.
        run(p.set_settings({'volume': 0.5}))
        everything = json.dumps([run(p.get_settings()), run(p.youtube_status()), run(p.set_youtube_key(FAKE_KEY))])
        self.assertNotIn('AIza', everything)
        self.assertNotIn('AIza', open(os.path.join(self.m.settings_dir(), 'settings.json')).read())
        # Never in the log, whatever reaches it.
        self.api(lambda url: http_error(url, 500, 'backendError'))
        with self.assertRaises(ValueError):
            self.m._youtube_get('Hades main theme OST', budget=self.m.YT_DAILY)
        run(p.report('play failed: https://www.googleapis.com/youtube/v3/search?q=x&key=%s' % FAKE_KEY))
        self.m.log('oops %s' % FAKE_KEY)
        self.assertTrue(lines)
        self.assertFalse([ln for ln in lines if FAKE_KEY in ln or 'AIza' in ln], lines)
        self.assertTrue(self.asked and all('key=' in u for u in self.asked), 'the key goes only to googleapis.com')
        # Forgotten: the file and its .bak are gone.
        self.assertTrue(run(p.set_youtube_key(''))['saved'])
        self.assertFalse(os.path.exists(path) or os.path.exists(path + '.bak'))
        self.assertFalse(run(p.youtube_status())['key'])

    def test_search_asks_only_the_official_api_for_embeddable_videos(self):
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: {'items': [yt_item(VID, 'Hades - Main Theme &amp; Title &#39;OST&#39;', 'Supergiant Games')]})
        res = self.m._youtube_get('Hades main theme OST', now=1_800_000_000)
        self.assertEqual(res[0], {'id': VID, 'title': "Hades - Main Theme & Title 'OST'", 'channel': 'Supergiant Games', 'description': ''})
        q = urllib.parse.parse_qs(urllib.parse.urlparse(self.asked[0]).query)
        self.assertEqual(urllib.parse.urlparse(self.asked[0]).path, '/youtube/v3/search')
        self.assertEqual((q['part'], q['type'], q['videoEmbeddable'], q['q'], q['key']),
                         (['snippet'], ['video'], ['true'], ['Hades main theme OST'], [FAKE_KEY]))
        self.assertEqual(self.m.youtube_status(now=1_800_000_000)['used'], 100, 'a search costs 100 units')

    def test_scoring_uses_the_plugins_own_rules(self):
        f = self.m.match_forms
        sc = lambda title, v: self.m.youtube_score(v, f(title))
        ok = [('Hades', yv(VID, 'Hades OST - Main Theme', 'Supergiant Games')),
              ('Hades', yv(VID, 'Hades - Main Theme', 'Darren Korb - Topic')),
              ('Hollow Knight', yv(VID, 'Hollow Knight - Main Theme')),
              ('Hollow Knight', yv(VID, 'Hollow Knight OST - Main Theme')),
              ('Hollow Knight', yv(VID, 'Hollow Knight (Original Soundtrack) - Dirtmouth', 'Christopher Larkin')),
              ('The Witcher® 3: Wild Hunt - Game of the Year Edition', yv(VID, 'The Witcher 3: Wild Hunt OST - Main Theme')),
              ('Hades', yv(VID, 'No Escape', 'Darren Korb - Topic', 'Provided to YouTube by Supergiant Games No Escape · Darren Korb Hades (Original Soundtrack) ℗ 2020')),
              ('Celeste', yv(VID, 'Celeste Original Soundtrack - First Steps', 'Lena Raine'))]
        for title, v in ok:
            self.assertGreaterEqual(sc(title, v), self.m.YT_MIN, (title, v['title']))
        bad = [('Hades', yv(VID, 'Hades II - Main Theme')),                                  # the sequel
               ('Hollow Knight', yv(VID, 'Hollow Knight Silksong OST - Main Theme')),       # another game
               ('Control Ultimate Edition', yv(VID, 'Halsey - Control')),                   # a pop song
               ('Control Ultimate Edition', yv(VID, 'Control - Main Theme')),               # one word, no soundtrack word
               ('Hades', yv(VID, 'Hades Main Theme (Piano Cover)')),                        # BAD
               ('Hades', yv(VID, 'Hades - Main Theme | Epic Version')),
               ('Hades', yv(VID, 'Hades OST Main Theme - 10 Hours')),                       # YT_BAD
               ('Hades', yv(VID, 'Hades - Official Launch Trailer')),
               ('Hades', yv(VID, 'Hades Gameplay Walkthrough Part 1')),
               ('Hades', yv(VID, 'Hades')),                                                 # only the title
               ('Hades', yv(VID, 'Top 10 Video Game Themes - Hades Main Theme')),          # COMPILATION
               ('Inside', yv(VID, 'Inside Out (Original Motion Picture Soundtrack) - Bundle of Joy')),   # FILM
               ('Hades', yv(VID, 'Hades Theme', 'The Piano Guys')),
               ('Hades', yv(VID, 'No Escape', 'Darren Korb - Topic', 'Provided to YouTube by X No Escape · Darren Korb Hades II (Original Soundtrack)')),
               ('Hades', yv('bad id', 'Hades - Main Theme'))]
        for title, v in bad:
            self.assertEqual(sc(title, v), 0, (title, v['title'], v['channel']))
        # Official channels win over others of the same video title.
        best = self.m.youtube_best([yv('aaaaaaaaaaa', 'Hades OST - Main Theme', 'GameMusicFan'),
                                    yv('bbbbbbbbbbb', 'Hades OST - Main Theme', 'Supergiant Games - Topic')], f('Hades'))
        self.assertEqual(best['id'], 'bbbbbbbbbbb')
        self.assertIsNone(self.m.youtube_best([yv(VID, 'Hades II - Main Theme')], f('Hades')))

    def test_order_of_sources_youtube_last(self):
        self.m.set_youtube_key(FAKE_KEY)
        calls = []

        def yt(term):
            calls.append(term)
            return [yv(VID, 'Hades OST - Main Theme', 'Supergiant Games')]
        apple_hit = lambda t: [it('Hades', 'Hades (Original Soundtrack)', 'Darren Korb')]
        nothing = lambda t: []
        s = self.m.find_song('1145360', 'Hades', [], get=apple_hit, deezer_get=nothing, youtube=True, yt_get=yt)
        self.assertEqual((s['source'], calls), ('apple', []), 'Apple first: YouTube never asked')
        os.remove(os.path.join(self.m.cache_dir(), 'lookups.json'))
        s = self.m.find_song('1145360', 'Hades', [], get=nothing, deezer_get=lambda t: dzr(dz('Hades', 'Hades (Original Soundtrack)'))(self.m),
                             youtube=True, yt_get=yt, deezer_track=lambda tid: {'preview': DZ_URL % 2000000000})
        self.assertEqual((s['source'], calls), ('deezer', []), 'then Deezer')
        os.remove(os.path.join(self.m.cache_dir(), 'lookups.json'))
        s = self.m.find_song('1145360', 'Hades', [], get=nothing, deezer_get=nothing, youtube=True, yt_get=yt)
        self.assertEqual((s['source'], s['youtube'], s['key'], s['from']), ('youtube', VID, 'yt:' + VID, 'youtube'))
        self.assertEqual(calls, ['Hades main theme OST'], 'one search when it found the theme')
        self.assertNotIn('url', s, 'no media address: YouTube\'s own player plays it')
        self.assertEqual(s['link'], 'https://www.youtube.com/watch?v=%s' % VID)
        # The game's soundtrack and a pick win over it.
        write(os.path.join(self.steam, 'steamapps/music/Hades Soundtrack/01 Main Theme.ogg'), b'o' * 10)
        self.assertEqual(self.m.find_song('1145360', 'Hades', [], youtube=True, yt_get=yt)['source'], 'soundtrack')
        shutil.rmtree(os.path.join(self.steam, 'steamapps/music'))
        mine = os.path.join(self.home, 'Music/mine.mp3')
        write(mine, b'm')
        self.m.set_pick('app', '1145360', mine)
        self.assertEqual(self.m.find_song('1145360', 'Hades', [], youtube=True, yt_get=yt)['source'], 'pick')
        self.m.set_pick('app', '1145360', '')
        # The switch off: the kept video isn't played, and nothing is searched.
        n = len(calls)
        s = self.m.find_song('1145360', 'Hades', [], online=False, deezer=False, youtube=False, yt_get=yt)
        self.assertIsNone(s['source'])
        self.assertEqual(len(calls), n)

    def test_video_ids_are_cached_and_no_media_is_ever_written(self):
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: {'items': [yt_item(VID, 'Hades OST - Main Theme', 'Supergiant Games')]})
        nothing = lambda t: []
        s = self.m.find_song('1145360', 'Hades', [], get=nothing, deezer_get=nothing, youtube=True)
        self.assertEqual(s['youtube'], VID)
        self.assertEqual(len(self.asked), 1)
        kept = json.load(open(os.path.join(self.m.cache_dir(), 'lookups.json')))['hades']
        self.assertEqual((kept['match']['source'], kept['match']['id'], kept['tried']), ('youtube', VID, ['apple', 'deezer', 'youtube']))
        again = self.m.find_song('1145360', 'Hades', [], get=None, deezer_get=None, youtube=True)
        self.assertEqual((again['youtube'], len(self.asked)), (VID, 1), 'from lookups.json: no second search')
        self.assertIsNone(self.m.audio('yt:' + VID), 'nothing of YouTube passes through the backend')
        made = files_under(self.tmp)
        self.assertFalse([f for f in made if not f.endswith(('.json', '.bak', '.lock')) and '/home/' not in f.replace(self.home, '/home/')], made)
        self.assertFalse([f for f in made if f.endswith(('.mp4', '.webm', '.m4a', '.ogg', '.opus', '.mp3'))])
        # Without a key the kept video still plays (an id is not media), and nothing is searched.
        self.m.set_youtube_key('')
        self.assertEqual(self.m.find_song('1145360', 'Hades', [], youtube=True)['youtube'], VID)
        self.assertEqual(len(self.asked), 1)

    def test_quota_stops_youtube_for_the_day_and_the_scan_goes_on(self):
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: http_error(url, 403, 'quotaExceeded'))
        apple = []
        nothing = lambda t: (apple.append(t), [])[1]
        r = self.m.lookup('Hades', get=nothing, deezer=nothing, youtube=self.m.youtube_search_now(), youtube_on=True)
        self.assertEqual(r, {'match': None})
        self.assertEqual(self.m.youtube_ready(), 'quota', 'stopped until midnight Pacific time')
        self.assertIsNone(self.m.youtube_search_now())
        st = self.m._yt_state()
        self.assertGreater(st['until'], time.time())
        self.assertLess(st['until'], time.time() + 86400 + 3600)
        kept = json.load(open(os.path.join(self.m.cache_dir(), 'lookups.json')))['hades']
        self.assertEqual(kept['tried'], ['apple', 'deezer'], 'YouTube not counted as asked: asked another day')
        # The scan meanwhile: it doesn't wait for YouTube, it goes on through the library.
        n = len(self.asked)
        done = self.m.scan_library([{'appid': '1145360', 'title': 'Hades'}, {'appid': '504230', 'title': 'Celeste'}],
                                   settings={'youtube': True}, get=nothing, deezer=nothing, sleep=lambda s: None)
        self.assertEqual((done['done'], len(self.asked)), (2, n))
        # The next day: YouTube only, Apple and Deezer aren't asked again for their recent "no match".
        self.m._yt_state_update(lambda s: s.update(until=0))
        self.api(lambda url: {'items': [yt_item(VID, 'Hades OST - Main Theme', 'Supergiant Games')]})
        before = len(apple)
        done = self.m.scan_library([{'appid': '1145360', 'title': 'Hades'}], settings={'youtube': True}, get=nothing,
                                   deezer=nothing, sleep=lambda s: None)
        self.assertEqual((done['matched'], len(apple)), (1, before))
        self.assertEqual(len(self.asked), 1)

    def test_automatic_searches_leave_quota_for_the_panel(self):
        self.m.set_youtube_key(FAKE_KEY)
        now = time.time()
        day, _ = self.m._pacific_day(now)
        self.m._yt_state_update(lambda s: s.update(day=day, units=self.m.YT_AUTO_BUDGET - 50))
        self.assertEqual(self.m.youtube_ready(now=now), 'quota', 'the scan and highlighted games stop at 8,000')
        self.assertEqual(self.m.youtube_ready(self.m.YT_DAILY, now=now), '', '"Choose music..." still can')
        self.m._yt_state_update(lambda s: s.update(day='2000-01-01'))
        self.assertEqual(self.m.youtube_ready(now=now), '', 'a new day starts at 0')

    def test_a_refused_key_stops_until_a_new_one(self):
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: http_error(url, 400, 'keyInvalid'))
        self.assertIsNone(self.m._youtube_get('x'))
        self.assertEqual(self.m.youtube_status()['state'], 'bad_key')
        self.assertIsNone(self.m.youtube_search_now())
        self.m.set_youtube_key(FAKE_KEY[:-1] + 'Y')
        self.assertEqual(self.m.youtube_status()['state'], 'ok')
        self.api(lambda url: http_error(url, 429, 'rateLimitExceeded'))
        self.assertIsNone(self.m._youtube_get('x'))
        self.assertEqual(self.m.youtube_ready(), 'slow', 'asked to slow down: backs off like Apple and Deezer')

    def test_a_pasted_link_works_without_a_key_and_stays_the_pick(self):
        r = self.m.search_music('https://youtu.be/%s' % VID, youtube=True)
        self.assertEqual([(c['choice'], c['from']) for c in r['results']], [('youtube:' + VID, 'youtube')])
        self.assertEqual(self.net, [], 'no key: nothing asked')
        self.assertEqual(self.m.choice_preview('youtube:' + VID), {'key': 'yt:' + VID, 'youtube': VID})
        self.assertTrue(self.m.choose_music('1145360', 'youtube:' + VID))
        pick = json.load(open(os.path.join(self.m.settings_dir(), 'picks.json')))['app:1145360']
        self.assertEqual((pick['source'], pick['id'], pick['link']), ('youtube', VID, 'https://www.youtube.com/watch?v=%s' % VID))
        apple = lambda t: [it('Hades', 'Hades (Original Soundtrack)', 'Darren Korb')]
        s = self.m.find_song('1145360', 'Hades', [], get=apple, youtube=True)
        self.assertEqual((s['source'], s['youtube'], s['from']), ('pick', VID, 'youtube'), 'picks always win')
        self.m.scan_library([{'appid': '1145360', 'title': 'Hades'}], settings={}, get=apple, deezer=lambda t: [], sleep=lambda s: None, recheck=True)
        self.assertEqual(json.load(open(os.path.join(self.m.settings_dir(), 'picks.json')))['app:1145360']['id'], VID, 'no scan changes it')
        s = self.m.find_song('1145360', 'Hades', [], get=apple, youtube=False)
        self.assertEqual((s['source'], s['why']), (None, 'your pick: the YouTube mini player is off'))
        self.assertEqual(self.m.review([{'appid': '1145360', 'title': 'Hades', 'shortcut': True}])[0]['song']['from'], 'youtube')
        # The mini player off: a link isn't taken.
        self.assertEqual(self.m.search_music('https://youtu.be/%s' % VID, youtube=False)['results'], [])
        # With a key: its names asked for (1 unit), and a video its owner keeps on YouTube is refused.
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: {'items': [{'id': VID, 'snippet': {'title': 'Hades OST', 'channelTitle': 'X'}, 'status': {'embeddable': False}}]})
        r = self.m.search_music('youtube.com/watch?v=%s' % VID, youtube=True)
        self.assertEqual(r['results'], [])
        self.assertIn('outside YouTube', r['note'])
        self.assertEqual(urllib.parse.urlparse(self.asked[0]).path, '/youtube/v3/videos')

    def test_choose_music_searches_youtube_with_a_key(self):
        self.m.set_youtube_key(FAKE_KEY)
        self.api(lambda url: {'items': [yt_item('aaaaaaaaaaa', 'Hades II - Main Theme'), yt_item('bbbbbbbbbbb', 'Hades OST - Main Theme', 'Supergiant Games')]})
        r = self.m.search_music('Hades', apple=False, deezer=False, youtube=True)
        self.assertEqual([c['choice'] for c in r['results']], ['youtube:bbbbbbbbbbb', 'youtube:aaaaaaaaaaa'], 'the surest first')
        self.api(lambda url: http_error(url, 403, 'quotaExceeded'))
        self.m._yt_state_update(lambda s: s.pop('until', None))
        self.assertEqual(self.m.search_music('Celeste', apple=False, deezer=False, youtube=True)['later'], ['youtube'])

    def test_kept_videos_must_still_fit_and_old_answers_stay(self):
        good = {'source': 'youtube', 'id': VID, 'track': 'Hades OST - Main Theme', 'artist': 'Supergiant Games', 'album': ''}
        self.assertTrue(self.m.still_fits(good, 'Hades'))
        self.assertFalse(self.m.still_fits(dict(good, track='Hades II OST - Main Theme'), 'Hades'))
        # Apple's and Deezer's kept answers are untouched by the mini player (the matcher's version is the same).
        self.assertEqual(self.m.MATCHER, 2)

    def test_the_new_setting_never_changes_existing_ones(self):
        p = self.m.Plugin()
        run = lambda c: asyncio.new_event_loop().run_until_complete(c)
        path = os.path.join(self.m.settings_dir(), 'settings.json')
        write(path, json.dumps({'enabled': False, 'online': False, 'volume': 0.8, 'someday': 1}))
        before = open(path).read()
        got = run(p.get_settings())
        self.assertTrue(got['youtube'], 'on by default')
        self.assertEqual((got['enabled'], got['online'], got['deezer'], got['volume']), (False, False, False, 0.8))
        self.assertEqual(open(path).read(), before, 'reading never writes')
        run(p.set_settings({'youtube': False}))
        self.assertEqual(json.load(open(path)), {'enabled': False, 'online': False, 'volume': 0.8, 'someday': 1, 'youtube': False},
                         'merge-on-write: only the key changed')
        run(p.set_settings({'volume': 0.5}))
        self.assertFalse(run(p.get_settings())['youtube'], 'remembered')
        self.assertTrue(os.path.isfile(path + '.bak'))
        with open(path, 'w') as f:
            f.write('{broken')
        self.assertFalse(run(p.get_settings())['youtube'], 'a damaged file comes back from the .bak')


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
