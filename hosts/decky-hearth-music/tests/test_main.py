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

    def test_never_youtube(self):
        src = open(HERE / 'main.py').read().lower() + ''.join(open(f).read().lower() for f in (HERE / 'src').glob('*.ts*'))
        for bad in ('youtube.com', 'ytimg', 'youtubei', 'youtu.be', 'googlevideo', 'yt-dlp', 'youtube-dl', 'ytdl', 'pytube', 'invidious', 'piped.video', 'pipedapi'):
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
