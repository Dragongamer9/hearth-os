"""ThemeDeck in Hearth's Steam Game Mode (docs/THEMEDECK.md): hearth-decky copying the image's unmodified ThemeDeck
into Decky's plugins folder, retiring the built-in Hearth Music (docs/archive/HEARTH-MUSIC.md), and carrying the songs
you picked in Hearth Music over to ThemeDeck once, by the settings rules (docs/SETTINGS-PERSISTENCE.md). Everything in
temporary folders: no Decky, no Steam, nothing of the real system is touched."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import tempfile
import types
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DECKY = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-decky'
BUILD = ROOT / 'os-bases/peppermint/build.sh'
WORKFLOW = ROOT / '.github/workflows/base-peppermint.yml'

TD_FILES = {'plugin.json': json.dumps({'name': 'ThemeDeck', 'author': 'Your Name', 'flags': [], 'api_version': 1}, indent=2),
            'package.json': json.dumps({'name': 'themedeck', 'version': '4.0.0', 'license': 'BSD-3-Clause'}, indent=2),
            'main.py': '# ThemeDeck backend 4.0.0\n', 'dist/index.js': '// ThemeDeck 4.0.0\n',
            'LICENSE': 'BSD 3-Clause License\n\nCopyright (c) 2025, BrenticusMaximus\n', 'README.md': '# ThemeDeck\n'}


def load_decky():
    loader = importlib.machinery.SourceFileLoader('hearth_decky_td', str(DECKY))
    spec = importlib.util.spec_from_loader('hearth_decky_td', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def tree(d):
    out = {}
    for base, _, files in os.walk(d):
        for f in files:
            p = os.path.join(base, f)
            out[os.path.relpath(p, d)] = hashlib.sha256(open(p, 'rb').read()).hexdigest()
    return out


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-themedeck-')
        self.root = os.path.join(self.tmp, 'root')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.old = dict(os.environ)
        os.environ['HEARTH_DECKY_ROOT'] = self.root
        os.environ['HEARTH_DECKY_TEST_HOME'] = self.home
        os.environ['HEARTH_SETTINGS_IO_DIR'] = str(ROOT / 'hosts/linux/desktop/shell')
        os.environ['XDG_STATE_HOME'] = os.path.join(self.tmp, 'state')   # the helper's settings-events.log
        self.m = load_decky()
        if os.geteuid() == 0:
            # Run as root (a developer's container): "as the person" is this process; CI runs as a normal user.
            self.m.as_user = lambda pw, fn: (fn(), True)[1]
        self.w('usr/share/nimbus/version', '1.9.0\n')
        self.w('etc/os-release', 'NAME="Hearth"\nVARIANT_ID=hearth-peppermint\n')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def w(self, rel, text, base=None):
        p = os.path.join(base or self.root, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text if isinstance(text, str) else json.dumps(text))
        return p

    def ship_themedeck(self, version='4.0.0'):
        d = os.path.join(self.root, 'usr/share/hearth/decky-third-party/ThemeDeck')
        shutil.rmtree(d, ignore_errors=True)
        for rel, text in TD_FILES.items():
            self.w(rel, text.replace('4.0.0', version), d)
        return d

    def td(self, *rel):
        return os.path.join(self.home, 'homebrew/plugins/ThemeDeck', *rel)

    def refresh(self):
        out = {}
        self.m.refresh_plugins(self.home, out)
        return out


class ThemeDeckCopy(Base):
    def test_added_unmodified_with_its_licence_then_left_as_it_is(self):
        src = self.ship_themedeck()
        out = self.refresh()
        self.assertEqual(out['ThemeDeck'], 'added')
        got = tree(self.td())
        mark = got.pop(self.m.THIRD_PARTY_MARK)
        self.assertTrue(mark)
        self.assertEqual(got, tree(src), 'every file exactly as shipped (LICENSE too); only Hearth\'s mark beside them')
        self.assertIn('BSD 3-Clause License', open(self.td('LICENSE')).read())
        self.assertEqual(json.load(open(self.td(self.m.THIRD_PARTY_MARK)))['version'], '4.0.0')
        self.assertEqual(self.refresh()['ThemeDeck'], 'up to date')
        # Decky's store folder name, so ThemeDeck's settings are ~/homebrew/settings/ThemeDeck either way.
        self.assertEqual(self.m.THEMEDECK, 'ThemeDeck')

    def test_a_hearth_os_update_brings_the_new_version(self):
        self.ship_themedeck('4.0.0')
        self.refresh()
        self.ship_themedeck('4.1.0')
        self.assertEqual(self.refresh()['ThemeDeck'], 'updated')
        self.assertIn('4.1.0', open(self.td('package.json')).read())

    def test_your_own_themedeck_is_never_replaced_or_removed(self):
        self.ship_themedeck('4.0.0')
        for rel, text in TD_FILES.items():
            self.w(rel, text.replace('4.0.0', '3.9.0'), os.path.join(self.home, 'homebrew/plugins/ThemeDeck'))
        before = tree(self.td())
        self.assertEqual(self.refresh()['ThemeDeck'], 'yours, left alone')
        self.assertEqual(tree(self.td()), before)
        self.assertFalse(self.m.remove_themedeck(self.home))
        self.assertEqual(tree(self.td()), before)

    def test_a_link_is_never_followed(self):
        self.ship_themedeck()
        elsewhere = os.path.join(self.tmp, 'elsewhere')
        os.makedirs(elsewhere)
        os.makedirs(os.path.join(self.home, 'homebrew/plugins'))
        os.symlink(elsewhere, self.td())
        self.assertEqual(self.refresh()['ThemeDeck'], 'yours, left alone')
        self.assertEqual(os.listdir(elsewhere), [])

    def test_not_shipped_nothing_happens(self):
        out = self.refresh()
        self.assertNotIn('ThemeDeck', out)
        self.assertFalse(os.path.exists(self.td()))

    def test_hearths_copy_goes_with_remove(self):
        self.ship_themedeck()
        self.refresh()
        self.assertTrue(self.m.remove_themedeck(self.home))
        self.assertFalse(os.path.exists(self.td()))

    def test_a_problem_never_stops_the_rest(self):
        self.ship_themedeck()

        def boom(home):
            raise OSError('disk full')
        self.m.refresh_themedeck = boom
        out = self.refresh()
        self.assertIn('couldn\'t copy', out['ThemeDeck'])


class HearthMusicRetired(Base):
    def hearth_music(self, author='Dragongamer9', marker=False):
        d = os.path.join(self.home, 'homebrew/plugins/decky-hearth-music')
        self.w('plugin.json', {'name': 'Hearth Music', 'author': author}, d)
        self.w('package.json', {'version': '1.8'}, d)
        self.w('main.py', '#\n', d)
        if marker:
            self.w(self.m.MARKER, {'version': '1.8.0.1'}, d)
        return d

    def test_hearths_copy_goes_its_settings_stay(self):
        d = self.hearth_music()
        settings = self.w('homebrew/settings/decky-hearth-music/settings.json', {'volume': 0.5}, self.home)
        out = self.refresh()
        self.assertFalse(os.path.exists(d))
        self.assertEqual(out['decky-hearth-music'], 'removed (retired)')
        self.assertEqual(json.load(open(settings)), {'volume': 0.5})
        self.assertNotIn('decky-hearth-music', self.refresh())   # gone, nothing more to do

    def test_a_downloaded_copy_goes_and_is_never_put_back(self):
        d = self.hearth_music(marker=True)
        store = os.path.join(self.home, '.local/share/hearth/decky-plugins/decky-hearth-music')
        self.w('plugin.json', {'name': 'Hearth Music', 'author': 'Dragongamer9'}, store)
        self.w(self.m.MARKER, {'version': '9.9.9'}, store)
        self.refresh()
        self.assertFalse(os.path.exists(d))
        self.assertIsNone(self.m.downloaded_ok(self.home, 'decky-hearth-music'))
        self.refresh()
        self.assertFalse(os.path.exists(d))

    def test_someone_elses_folder_of_that_name_is_left_alone(self):
        d = self.hearth_music(author='someone')
        self.refresh()
        self.assertTrue(os.path.isfile(os.path.join(d, 'plugin.json')))

    def test_an_image_that_still_ships_it_keeps_it(self):
        # Retired by a plugins release while the image still has it: the built-in copy stays (the old rule).
        src = os.path.join(self.root, 'usr/share/hearth/decky-hearth-music')
        self.w('plugin.json', {'name': 'Hearth Music', 'author': 'Dragongamer9'}, src)
        self.w('package.json', {'version': '1.8'}, src)
        self.refresh()
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-music/plugin.json')))


class CarryOver(Base):
    def setUp(self):
        super().setUp()
        self.music = os.path.join(self.home, 'Music')
        self.song = self.w('celeste.ogg', 'ogg', self.music)
        self.song2 = self.w('hades.mp3', 'mp3', self.music)
        self.picks = os.path.join(self.home, 'homebrew/settings/decky-hearth-music/picks.json')
        self.tracks = os.path.join(self.home, 'homebrew/settings/ThemeDeck/tracks.json')
        self.record = os.path.join(self.home, '.local/state/nimbus/themedeck-from-hearth-music.json')

    def write_picks(self, picks):
        self.w(self.picks, picks)
        return hashlib.sha256(open(self.picks, 'rb').read()).hexdigest()

    def all_picks(self):
        return {'app:504230': self.song, 'app:1145360': self.song2, 'app:620': os.path.join(self.music, 'gone.ogg'),
                'app:570': {'source': 'apple', 'id': '123', 'track': 't', 'preview': 'https://audio.itunes.apple.com/x.m4a'},
                'app:440': {'source': 'deezer', 'id': '456'}, 'app:730': {'source': 'none'},
                'collection:Favorites': self.song, 'app:not-a-number': self.song}

    def test_your_music_files_carry_over_streamed_previews_dont(self):
        h = self.write_picks(self.all_picks())
        rec = self.m.music_to_themedeck(self.home, now=1000)
        t = json.load(open(self.tracks))
        self.assertEqual(sorted(t), ['1145360', '504230'])
        self.assertEqual(t['504230'], {'app_id': 504230, 'assigned_at': 1000.0, 'path': os.path.realpath(self.song),
                                       'filename': 'celeste.ogg', 'volume': 1.0, 'start_offset': 0.0, 'loop': True})
        self.assertEqual((rec['carried'], rec['kept']), (2, 0))
        self.assertEqual(rec['skipped'], {'file gone': 1, 'streamed preview': 2, 'no music': 1, 'collection': 1, 'other': 1})
        self.assertEqual(hashlib.sha256(open(self.picks, 'rb').read()).hexdigest(), h, 'Hearth Music\'s file is only read')
        self.assertTrue(json.load(open(self.record))['done'])

    def test_merges_and_never_overwrites_what_themedeck_has(self):
        self.write_picks({'app:504230': self.song, 'app:1145360': self.song2})
        mine = {'504230': {'app_id': 504230, 'path': '/home/me/mine.flac', 'filename': 'mine.flac', 'volume': 0.4},
                '__global__': {'path': '/home/me/menu.ogg'}, '__store__': {'path': '/home/me/store.ogg'}}
        self.w(self.tracks, mine)
        before = open(self.tracks, 'rb').read()
        rec = self.m.music_to_themedeck(self.home, now=5)
        t = json.load(open(self.tracks))
        self.assertEqual(t['504230'], mine['504230'], 'a game you already gave a song in ThemeDeck keeps it')
        self.assertEqual((t['__global__'], t['__store__']), (mine['__global__'], mine['__store__']))
        self.assertEqual(t['1145360']['path'], os.path.realpath(self.song2))
        self.assertEqual((rec['carried'], rec['kept']), (1, 1))
        self.assertEqual(open(self.tracks + '.bak', 'rb').read(), before, 'the file before is kept as .bak')

    def test_runs_once(self):
        self.write_picks({'app:504230': self.song})
        self.m.music_to_themedeck(self.home)
        os.remove(self.tracks)   # you removed the song in ThemeDeck afterwards
        self.write_picks({'app:504230': self.song, 'app:1145360': self.song2})
        self.assertIsNone(self.m.music_to_themedeck(self.home))
        self.assertFalse(os.path.exists(self.tracks), 'never brought back')

    def test_a_damaged_tracks_file_is_never_written_over(self):
        self.write_picks({'app:504230': self.song})
        self.w(self.tracks, '{"504230": {"path": ')
        before = open(self.tracks, 'rb').read()
        rec = self.m.music_to_themedeck(self.home)
        self.assertFalse(rec['done'])
        self.assertEqual(open(self.tracks, 'rb').read(), before)
        self.assertFalse(os.path.exists(self.tracks + '.bak'))
        self.assertFalse(os.path.exists(self.record), 'tried again next time')
        self.w(self.tracks, {})   # ThemeDeck wrote a good file meanwhile
        self.assertEqual(self.m.music_to_themedeck(self.home)['carried'], 1)

    def test_a_damaged_picks_file_uses_its_bak_read_only(self):
        self.w(self.picks, '{"app:5')
        self.w(self.picks + '.bak', {'app:504230': self.song})
        rec = self.m.music_to_themedeck(self.home)
        self.assertEqual(rec['carried'], 1)
        self.assertEqual(open(self.picks).read(), '{"app:5', 'Hearth Music\'s file is left as it is')
        os.remove(self.record)
        os.remove(self.picks + '.bak')
        rec = self.m.music_to_themedeck(self.home)
        self.assertEqual(rec['carried'], 0)
        self.assertIn('couldn\'t be read', rec['note'])

    def test_nothing_from_hearth_music_nothing_written(self):
        pw = types.SimpleNamespace(pw_uid=os.getuid(), pw_gid=os.getgid(), pw_name='me', pw_dir=self.home)
        self.assertIsNone(self.m.carry_music_picks(pw, self.home))
        self.assertFalse(os.path.exists(self.record))
        self.assertFalse(os.path.exists(os.path.dirname(self.tracks)))

    def test_prepare_carries_over_then_retires_and_never_blocks_decky(self):
        pw = types.SimpleNamespace(pw_uid=os.getuid(), pw_gid=os.getgid(), pw_name='me', pw_dir=self.home)
        self.m.person = lambda name: pw
        self.w('usr/lib/hearth-decky/PluginLoader', 'x')
        self.w('usr/lib/systemd/system/hearth-decky@.service', '[Service]\n')
        self.ship_themedeck()
        self.write_picks({'app:504230': self.song})
        self.w('homebrew/plugins/decky-hearth-music/plugin.json', {'name': 'Hearth Music', 'author': 'Dragongamer9'}, self.home)
        out = self.m.prepare('me')
        self.assertEqual(out['musicPicks'], 'carried 1, kept 0 of yours, skipped 0')
        self.assertEqual(out['ThemeDeck'], 'added')
        self.assertEqual(out['decky-hearth-music'], 'removed (retired)')
        self.assertIn('504230', json.load(open(self.tracks)))
        self.assertNotIn('musicPicks', self.m.prepare('me'), 'once')
        # Whatever goes wrong in the carry-over, Decky still starts.
        os.remove(self.record)

        def boom(pw, home):
            raise RuntimeError('unexpected')
        self.m.carry_music_picks = boom
        out = self.m.prepare('me')
        self.assertTrue(out['ok'])
        self.assertIn('couldn\'t', out['musicPicks'])


class Image(unittest.TestCase):
    def test_build_pins_themedeck_and_fails_on_a_mismatch(self):
        b = BUILD.read_text()
        self.assertIn('THEMEDECK_VERSION=v4.0.0', b)
        self.assertIn('THEMEDECK_SHA256=9a2bd31bf4a9daeb2a2c3ba1e3b94fab4259dac27166a2098405580e3063b9c0', b)
        self.assertIn('https://github.com/BrenticusMaximus/ThemeDeck/releases/download/$THEMEDECK_VERSION/ThemeDeck-$THEMEDECK_VERSION.zip', b)
        self.assertIn('echo "$THEMEDECK_SHA256  /tmp/ThemeDeck.zip" | sha256sum -c -', b)
        self.assertIn('set -ouex pipefail', b)   # so the check above fails the build
        self.assertIn('/usr/share/hearth/decky-third-party/ThemeDeck', b)
        self.assertIn('LICENSE', b)
        self.assertNotIn('decky-hearth-music', b)

    def test_no_youtube_tooling_is_preinstalled(self):
        b = BUILD.read_text()
        for never in ('yt-dlp', 'deno '):
            self.assertNotIn(never, b.replace('Its YouTube support (yt-dlp, Deno)', ''), never)
        decky = DECKY.read_text()
        self.assertNotIn('yt-dlp', decky)
        self.assertIn('! command -v yt-dlp && ! command -v deno', WORKFLOW.read_text())

    def test_workflow_ships_no_hearth_music_and_checks_themedeck(self):
        wf = WORKFLOW.read_text()
        self.assertNotIn('hearth-decky-music', wf)
        self.assertIn('ThemeDeck is built in, unmodified', wf)

    def test_docs_credit_themedeck(self):
        d = (ROOT / 'docs/THEMEDECK.md').read_text()
        for want in ('BrenticusMaximus', 'BSD-3-Clause', 'Install YouTube support', 'v4.0.0',
                     '9a2bd31bf4a9daeb2a2c3ba1e3b94fab4259dac27166a2098405580e3063b9c0'):
            self.assertIn(want, d)


if __name__ == '__main__':
    unittest.main()
