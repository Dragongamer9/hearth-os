"""Hearth's Decky plugin updates (docs/PLUGIN-UPDATES.md): hearth-decky's plugins-* commands and refresh_plugins, and
the plugins' shared Hearth OS check (hosts/decky-shared/hearth_only.py). Everything in temporary folders with made-up
releases: no network, no Decky, no Steam, nothing of the real system is touched."""
import sys
sys.dont_write_bytecode = True
import asyncio
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import types
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
DECKY = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-decky'
SHARED = ROOT / 'hosts/decky-shared'
PLUGINS = ('decky-hearth-sync', 'decky-hearth-link')
# A third built-in plugin for these tests only (Hearth Music was the third until it was retired for ThemeDeck).
THIRD = 'decky-hearth-third'
DL = 'https://github.com/Dragongamer9/hearth-os/releases/download/plugins-v'

HEARTH_OS_RELEASE = '''PRETTY_NAME="Hearth OS (Debian 13)"
NAME="Hearth"
VERSION_ID="13"
ID=debian
VARIANT="Debian edition"
VARIANT_ID=hearth-peppermint
ID_LIKE=debian
'''
STEAMOS_RELEASE = '''NAME="SteamOS"
PRETTY_NAME="SteamOS"
ID=steamos
ID_LIKE=arch
VARIANT_ID=steamdeck
VERSION_ID=3.6.19
'''
DEBIAN_RELEASE = '''PRETTY_NAME="Debian GNU/Linux 13 (trixie)"
NAME="Debian GNU/Linux"
VERSION_ID="13"
ID=debian
'''


def load_decky():
    loader = importlib.machinery.SourceFileLoader('hearth_decky_pu', str(DECKY))
    spec = importlib.util.spec_from_loader('hearth_decky_pu', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def load_shared():
    spec = importlib.util.spec_from_file_location('hearth_only_t', SHARED / 'hearth_only.py')
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def plugin_files(name, version, broken=False):
    files = {'plugin.json': json.dumps({'name': name, 'author': 'Dragongamer9'}),
             'package.json': json.dumps({'name': name, 'version': version}),
             'main.py': '# %s %s\n' % (name, version), 'dist/index.js': '// %s %s\n' % (name, version)}
    if broken:
        del files['dist/index.js']
    return files


def make_zip(name, files, extra=None):
    b = io.BytesIO()
    with zipfile.ZipFile(b, 'w') as z:
        for rel, text in files.items():
            z.writestr('%s/%s' % (name, rel), text)
        for zi, data in (extra or []):
            z.writestr(zi, data)
    return b.getvalue()


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-plugin-updates-')
        self.root = os.path.join(self.tmp, 'root')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.old = dict(os.environ)
        os.environ['HEARTH_DECKY_ROOT'] = self.root
        os.environ['HEARTH_DECKY_TEST_HOME'] = self.home
        os.environ['HEARTH_SETTINGS_IO_DIR'] = str(ROOT / 'hosts/linux/desktop/shell')
        os.environ.pop('XDG_STATE_HOME', None)
        self.m = load_decky()
        self.w('usr/share/nimbus/version', '1.1.0\n')
        self.w('usr/lib/hearth-decky/version', 'v3.3.1\n')
        self.w('usr/lib/hearth-decky/PluginLoader', 'x')
        self.w('usr/lib/systemd/system/hearth-decky@.service', '[Service]\n')
        self.w('etc/os-release', HEARTH_OS_RELEASE)
        for n in PLUGINS + (THIRD,):
            self.ship(n, '1.1.0')
        self.fetched = []

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def w(self, rel, text, base=None):
        p = os.path.join(base or self.root, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def ship(self, name, version):
        d = os.path.join(self.root, 'usr/share/hearth', name)
        shutil.rmtree(d, ignore_errors=True)
        for rel, text in plugin_files(name, version).items():
            self.w(rel, text, d)

    def release(self, version, plugins, tag=None, draft=False, prerelease=False, min_hearth='1.1.0', min_decky='3.3.1',
                corrupt=None, sums_wrong=None, broken=None, retired=None, about=None):
        """A made-up plugins release: its API entry and the downloads behind it, served by self.get."""
        tag = tag or 'plugins-v' + version
        files, entries, sums = {}, [], []
        for n, v in plugins.items():
            data = make_zip(n, plugin_files(n, v, broken=(n == broken)))
            sha = hashlib.sha256(data).hexdigest()
            entries.append(dict({'id': n, 'version': v, 'archive': n + '.zip', 'sha256': sha, 'size': len(data),
                                 'min_hearth_os': min_hearth, 'min_decky': min_decky}, **((about or {}).get(n) or {})))
            sums.append('%s  %s.zip' % ('0' * 64 if n == sums_wrong else sha, n))
            files[n + '.zip'] = data + (b'tampered' if n == corrupt else b'')
        man = {'format': 1, 'version': version, 'plugins': entries}
        if retired is not None:
            man['retired'] = retired
        files['plugins.json'] = json.dumps(man).encode()
        files['SHA256SUMS'] = ('\n'.join(sums) + '\n').encode()
        base = DL + version + '/'
        self.served = getattr(self, 'served', {})
        for k, v in files.items():
            self.served[base + k] = v
        return {'tag_name': tag, 'draft': draft, 'prerelease': prerelease,
                'assets': [{'name': k, 'browser_download_url': base + k} for k in files]}

    def serve(self, *releases):
        self.served = getattr(self, 'served', {})
        self.served[self.m.PLUGIN_RELEASES] = json.dumps(list(releases)).encode()

    def get(self, url, limit=None):
        self.fetched.append(url)
        if url not in self.served:
            raise OSError('404 %s' % url)
        return self.served[url]

    def installed(self, name):
        return self.m.folder_version(os.path.join(self.home, 'homebrew/plugins', name))

    def prepare(self):
        out = {}
        self.m.refresh_plugins(self.home, out)
        return out

    def snapshot(self):
        snap = {}
        for base, dirs, files in os.walk(self.home):
            for f in files:
                p = os.path.join(base, f)
                snap[os.path.relpath(p, self.home)] = open(p, 'rb').read()
        return snap


class Manifest(Base):
    def test_parses_a_good_manifest(self):
        r = self.release('1.1.1', {'decky-hearth-sync': '1.1.1', THIRD: '1.1.1'})
        data = self.served[[a['browser_download_url'] for a in r['assets'] if a['name'] == 'plugins.json'][0]]
        m = self.m.parse_manifest(data, '1.1.1')
        self.assertEqual([p['id'] for p in m['plugins']], ['decky-hearth-sync', THIRD])
        self.assertEqual(m['plugins'][0]['min_decky'], '3.3.1')

    def test_refuses_bad_manifests(self):
        good = {'id': 'decky-hearth-sync', 'version': '1.1.1', 'archive': 'decky-hearth-sync.zip', 'sha256': 'a' * 64}
        bad = [b'not json', json.dumps({'format': 2, 'version': '1', 'plugins': []}),
               json.dumps({'format': 1, 'version': '1.1.1', 'plugins': [dict(good, id='evil-plugin')]}),
               json.dumps({'format': 1, 'version': '1.1.1', 'plugins': [dict(good, archive='../x.zip')]}),
               json.dumps({'format': 1, 'version': '1.1.1', 'plugins': [dict(good, sha256='xyz')]}),
               json.dumps({'format': 1, 'version': '1.1.1', 'plugins': [dict(good, version='../1')]}),
               json.dumps({'format': 1, 'version': '1.1.1', 'plugins': [good, good]})]
        for b in bad:
            with self.assertRaises(ValueError):
                self.m.parse_manifest(b, '1.1.1')
        with self.assertRaises(ValueError):   # a manifest from another release
            self.m.parse_manifest(json.dumps({'format': 1, 'version': '1.1.2', 'plugins': [good]}), '1.1.1')

    def test_versions_compare_by_number(self):
        self.assertTrue(self.m.newer('1.10', '1.9'))
        self.assertTrue(self.m.newer('1.1.0.1', '1.1.0'))
        self.assertFalse(self.m.newer('1.1', '1.1.0'))
        self.assertTrue(self.m.newer('v3.3.1', '3.3.0'))

    def test_no_drafts_or_prereleases_and_only_plugins_tags(self):
        rels = [self.release('1.3.0', {'decky-hearth-sync': '1.3.0'}, draft=True),
                self.release('1.2.5', {'decky-hearth-sync': '1.2.5'}, prerelease=True),
                self.release('9.9.9', {'decky-hearth-sync': '9.9.9'}, tag='v9.9.9'),
                self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}),
                self.release('1.1.2', {'decky-hearth-sync': '1.1.2'})]
        self.assertEqual(self.m.pick_plugins_release(rels)['version'], '1.1.2')
        rels[4]['assets'][0]['browser_download_url'] = 'https://evil.example/x'   # only this repo's own downloads
        rels[4]['assets'] = [a for a in rels[4]['assets'] if a['name'] != 'plugins.json'] + [
            {'name': 'plugins.json', 'browser_download_url': 'https://evil.example/plugins.json'}]
        self.assertEqual(self.m.pick_plugins_release(rels)['version'], '1.1.1')
        self.assertIsNone(self.m.pick_plugins_release({'message': 'rate limited'}))


class Picking(Base):
    def test_only_newer_than_both_installed_and_builtin(self):
        self.prepare()   # the image's 1.1.0 copies in place
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1', 'decky-hearth-link': '1.1.0', THIRD: '1.0.9'}))
        out, got = self.m.plugins_check(self.home, self.get)
        self.assertEqual(out['available'], {'decky-hearth-sync': '1.1.1'})
        self.assertIn('not newer', out['skipped']['decky-hearth-link'])
        self.assertIn('not newer', out['skipped'][THIRD])
        # A built-in version newer than the installed one also counts: an image with 1.2.0 needs nothing from 1.1.1.
        self.ship('decky-hearth-sync', '1.2.0')
        out, _ = self.m.plugins_check(self.home, self.get)
        self.assertEqual(out['available'], {})

    def test_minimum_versions(self):
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}, min_hearth='1.2.0'))
        out, _ = self.m.plugins_check(self.home, self.get)
        self.assertEqual(out['available'], {})
        self.assertIn('needs Hearth OS 1.2.0', out['skipped']['decky-hearth-sync'])
        self.served.clear()
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}, min_decky='3.4.0'))
        out, _ = self.m.plugins_check(self.home, self.get)
        self.assertIn('needs Decky Loader 3.4.0', out['skipped']['decky-hearth-sync'])
        self.assertIsNone(self.m.fits({'min_hearth_os': '1.1.0', 'min_decky': '3.3.1'}))


class Downloading(Base):
    def test_sha256_mismatch_is_refused(self):
        self.prepare()
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}, corrupt='decky-hearth-sync'))
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertFalse(out['ok'])
        self.assertIn('SHA-256 is wrong', out['error'])
        self.assertFalse(os.path.exists(os.path.join(self.m.store_dir(self.home), 'decky-hearth-sync')))
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')
        self.assertIn('SHA-256', self.m.state(self.home)['lastError'])

    def test_sha256sums_must_agree(self):
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}, sums_wrong='decky-hearth-sync'))
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertIn('SHA256SUMS and plugins.json disagree', out['error'])

    def test_incomplete_plugin_is_refused(self):
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}, broken='decky-hearth-sync'))
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertIn('dist/index.js is missing', out['error'])

    def test_unsafe_archives_are_refused(self):
        stage = os.path.join(self.tmp, 'stage')
        os.makedirs(stage)
        evil = make_zip('decky-hearth-sync', plugin_files('decky-hearth-sync', '1.1.1'), extra=[('../escape.txt', 'x')])
        with self.assertRaises(ValueError):
            self.m.unpack(evil, 'decky-hearth-sync', stage)
        link = zipfile.ZipInfo('decky-hearth-sync/main.py')
        link.external_attr = (0o120777 << 16)
        evil = make_zip('decky-hearth-sync', {}, extra=[(link, '/etc/shadow')])
        with self.assertRaises(ValueError):
            self.m.unpack(evil, 'decky-hearth-sync', stage)
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'escape.txt')))

    def test_network_failure_leaves_everything_unchanged(self):
        self.prepare()
        before = self.snapshot()

        def offline(url, limit=None):
            raise OSError('Network is unreachable')
        out = self.m.plugins_update(None, self.home, offline)
        self.assertFalse(out['ok'])
        after = self.snapshot()
        st = json.loads(after.pop('.local/state/nimbus/plugin-updates.json'))
        for snap in (before, after):
            for k in [k for k in snap if k.startswith('.local/state/nimbus/')]:
                snap.pop(k)
        self.assertEqual(before, after)   # no plugin, download or setting changed
        self.assertIn('Network is unreachable', st['lastError'])
        self.assertNotIn('lastOk', st)


class Applying(Base):
    def update_to(self, version, **kw):
        self.served = {}
        self.serve(self.release(version, {'decky-hearth-sync': version}, **kw))
        return self.m.plugins_update(None, self.home, self.get)

    def test_atomic_swap_and_prev(self):
        self.prepare()
        out = self.update_to('1.1.1')
        self.assertEqual(out['downloaded'], {'decky-hearth-sync': '1.1.1'})
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')   # put in place only by Decky's own start
        r = self.prepare()
        self.assertEqual(r['plugin'], 'updated (downloaded 1.1.1)')
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')
        prev = os.path.join(self.home, 'homebrew/hearth-prev/decky-hearth-sync.prev')
        self.assertEqual(self.m.folder_version(prev), '1.1.0')
        plugins = os.listdir(os.path.join(self.home, 'homebrew/plugins'))
        self.assertFalse([p for p in plugins if p.endswith('.new') or p.endswith('.prev')], plugins)   # Decky loads only one copy
        v = self.m.versions(self.home, 'decky-hearth-sync')
        self.assertEqual((v['source'], v['installed'], v['builtin']), ('downloaded', '1.1.1', '1.1.0'))
        st = self.m.state(self.home)
        self.assertEqual(st['plugins']['decky-hearth-sync']['source'], 'downloaded')
        self.assertEqual(self.prepare().get('plugin'), None)   # nothing to do the next time

    def test_refresh_keeps_a_newer_download_and_a_newer_image_wins(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()
        self.mark('1.1.1')   # it loaded
        for _ in range(3):
            self.prepare()
            self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')   # the image's 1.1.0 never replaces it
        self.ship('decky-hearth-sync', '1.2.0')   # a Hearth OS update with a newer one
        self.prepare()
        self.assertEqual(self.installed('decky-hearth-sync'), '1.2.0')
        self.assertEqual(self.m.versions(self.home, 'decky-hearth-sync')['source'], 'built-in')

    def test_a_changed_download_is_not_used(self):
        self.prepare()
        self.update_to('1.1.1')
        with open(os.path.join(self.m.store_dir(self.home), 'decky-hearth-sync/main.py'), 'a') as f:
            f.write('import os; os.system("bad")\n')
        r = self.prepare()
        self.assertIn('changed since it was checked', r.get('pluginUpdate', ''))
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')

    def mark(self, version):
        self.w('homebrew/data/decky-hearth-sync/hearth-loaded.json', json.dumps({'version': version, 'at': 4102444800}), self.home)

    def test_rollback_when_it_never_loads(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()   # start 1: on trial
        self.assertIn('decky-hearth-sync', self.m.trials(self.home))
        self.prepare()   # start 2: still no sign
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')
        r = self.prepare()   # start 3: back to the copy before
        self.assertIn('didn\'t load', r['trial']['decky-hearth-sync'])
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')
        st = self.m.state(self.home)
        self.assertEqual(st['bad']['decky-hearth-sync'], ['1.1.1'])
        self.assertIn('didn\'t load', st['lastError'])
        self.assertEqual(st['rollback']['decky-hearth-sync']['version'], '1.1.1')
        self.assertFalse(os.path.exists(os.path.join(self.m.store_dir(self.home), 'decky-hearth-sync')))
        self.prepare()
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')   # stays back
        out = self.update_to('1.1.1')   # never picked again
        self.assertIn('didn\'t load before', out['skipped']['decky-hearth-sync'])

    def test_watcher_rolls_back_in_the_first_run_and_restarts_only_decky(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()
        restarts = []
        self.m.restart_decky = lambda user: restarts.append(user) or True
        self.assertTrue(self.m.watch_trial(self.home, 'you', 1234, sleep=lambda s: None, alive=lambda pid: True))
        self.assertEqual(restarts, ['you'])
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')
        # Decky stopped meanwhile (Steam Game Mode ended): nothing is restarted, the next start decides.
        self.update_to('1.1.2')
        self.prepare()
        self.assertFalse(self.m.watch_trial(self.home, 'you', 1234, sleep=lambda s: None, alive=lambda pid: False))
        self.assertEqual(restarts, ['you'])
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.2')

    def test_rollback_goes_to_the_download_before(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()
        self.mark('1.1.1')
        self.prepare()   # confirmed
        self.assertEqual(self.m.trials(self.home), {})
        self.update_to('1.1.2')
        self.prepare()
        self.m.check_trials(self.home, {}, final=True)
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')
        self.prepare()
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')   # kept: the download before came back too

    def test_a_loaded_plugin_is_confirmed(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()
        self.mark('1.1.1')
        r = self.prepare()
        self.assertEqual(r['trial']['decky-hearth-sync'], 'loaded')
        for _ in range(3):
            self.prepare()
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.1')

    def test_use_builtin_restores_the_image_versions(self):
        self.prepare()
        self.update_to('1.1.1')
        self.prepare()
        out = self.m.plugins_builtin(None, self.home)
        self.assertEqual(out['removed'], {'decky-hearth-sync': '1.1.1'})
        self.prepare()
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')
        self.assertEqual(self.m.versions(self.home, 'decky-hearth-sync')['source'], 'built-in')
        # Automatic updates don't fetch that version again; a newer release still comes.
        self.m.set_auto(self.home, True)
        self.served = {}
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1'}))
        out = self.m.plugins_auto(None, self.home, self.get, now=10 ** 10)
        self.assertEqual(out.get('downloaded'), {})
        self.served = {}
        self.serve(self.release('1.1.2', {'decky-hearth-sync': '1.1.2'}))
        out = self.m.plugins_auto(None, self.home, self.get, now=10 ** 10 + 10 ** 6)
        self.assertEqual(out.get('downloaded'), {'decky-hearth-sync': '1.1.2'})

    def test_applies_now_only_when_decky_runs(self):
        self.prepare()
        restarts = []
        self.m.restart_decky = lambda user: restarts.append(user) or True
        self.m.decky_active = lambda user: False
        out = self.update_to('1.1.1')
        self.assertEqual(out['applies'], 'at the next Steam Game Mode start')
        self.m.decky_active = lambda user: True
        self.served = {}
        self.serve(self.release('1.1.2', {'decky-hearth-sync': '1.1.2'}))
        self.m.plugins_update('you', self.home, self.get)
        self.assertEqual(restarts, ['you'])
        # Automatic, a game running: waits for the next start.
        self.m.game_running = lambda: True
        self.m.set_auto(self.home, True)
        self.served = {}
        self.serve(self.release('1.1.3', {'decky-hearth-sync': '1.1.3'}))
        out = self.m.plugins_auto('you', self.home, self.get, now=10 ** 10)
        self.assertIn('a game is running', out['applies'])
        self.assertEqual(restarts, ['you'])


NEW = 'decky-hearth-lights'
ABOUT = {NEW: {'name': 'Hearth Lights', 'description': 'RGB lighting that follows your game.'}}


class NewPlugins(Base):
    """Future plugins arrive through the plugins channel (docs/PLUGIN-UPDATES.md, "Adding a new plugin")."""

    def offer(self, version, plugins, **kw):
        self.served = {}
        self.serve(self.release(version, plugins, about=ABOUT, **kw))

    def mark_new(self, version):
        self.w('homebrew/data/%s/hearth-loaded.json' % NEW, json.dumps({'version': version, 'at': 4102444800}), self.home)

    def test_valid_ids_only(self):
        for good in ('decky-hearth-lights', 'decky-hearth-a', 'decky-hearth-x1-y2'):
            self.assertTrue(self.m.valid_id(good), good)
        for bad in ('decky-hearth', 'decky-hearth-', 'decky-hearth-Lights', 'decky-hearth-../x', 'evil', 'decky-hearth-a/b',
                    'decky-hearth-lights\n', 'decky-hearth-' + 'a' * 42, '-decky-hearth-a', 'decky-hearth--x', None, 3):
            self.assertFalse(self.m.valid_id(bad), bad)
        good = {'id': NEW, 'version': '1.0', 'archive': NEW + '.zip', 'sha256': 'a' * 64, 'name': 'Hearth Lights', 'description': 'x'}
        m = self.m.parse_manifest(json.dumps({'format': 1, 'version': '1.2.0.0', 'plugins': [good]}), '1.2.0.0')
        self.assertEqual((m['plugins'][0]['name'], m['plugins'][0]['description'], m['retired']), ('Hearth Lights', 'x', []))
        for bad in (dict(good, id='decky-hearth-../evil', archive='decky-hearth-../evil.zip'), dict(good, id='decky-other', archive='decky-other.zip'),
                    dict(good, name=5)):
            with self.assertRaises(ValueError):
                self.m.parse_manifest(json.dumps({'format': 1, 'version': '1.2.0.0', 'plugins': [bad]}), '1.2.0.0')
        for retired in (['../x'], 'decky-hearth-sync', [NEW]):
            with self.assertRaises(ValueError):
                self.m.parse_manifest(json.dumps({'format': 1, 'version': '1.2.0.0', 'plugins': [good], 'retired': retired}), '1.2.0.0')

    def test_unknown_valid_id_is_offered_as_new_and_box_off_installs_nothing(self):
        self.prepare()
        self.offer('1.2.0.0', {NEW: '1.0.0', 'decky-hearth-sync': '1.2.0.0'})
        self.assertFalse(self.m.install_new_on(self.home))
        self.assertFalse(os.path.exists(self.m.settings_path(self.home)))   # the default is never written
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertEqual(out['downloaded'], {'decky-hearth-sync': '1.2.0.0'})   # updates still come
        self.assertEqual(out['new'], {NEW: {'name': 'Hearth Lights', 'description': 'RGB lighting that follows your game.', 'version': '1.0.0'}})
        self.assertNotIn(NEW, out['available'])
        self.prepare()
        self.assertIsNone(self.installed(NEW))
        st = self.m.plugins_status(self.home)
        self.assertIn(NEW, st['new'])
        self.assertFalse(st['installNew'])
        self.assertNotIn(NEW, st['plugins'])

    def test_box_ticked_installs_a_new_plugin(self):
        self.prepare()
        self.assertTrue(self.m.set_install_new(self.home, True))
        self.offer('1.2.0.0', {NEW: '1.0.0'})
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertEqual(out['downloaded'], {NEW: '1.0.0'})
        r = self.prepare()
        self.assertEqual(self.installed(NEW), '1.0.0')
        self.assertIn('downloaded 1.0.0', r[NEW])
        self.assertIn(NEW, self.m.trials(self.home))   # load-checked like the others
        st = self.m.plugins_status(self.home)
        self.assertEqual(st['plugins'][NEW]['source'], 'downloaded')
        self.assertEqual(st['plugins'][NEW]['name'], NEW)   # plugin.json's name
        self.assertEqual(st['new'], {})
        self.mark_new('1.0.0')
        self.assertEqual(self.prepare()['trial'][NEW], 'loaded')
        # It updates like any other plugin from now on, the box no longer matters for it.
        self.m.set_install_new(self.home, False)
        self.offer('1.2.0.1', {NEW: '1.0.1'})
        self.assertEqual(self.m.plugins_update(None, self.home, self.get)['downloaded'], {NEW: '1.0.1'})
        self.prepare()
        self.assertEqual(self.installed(NEW), '1.0.1')
        self.assertFalse(self.m.install_new_on(self.home))   # never changed by itself

    def test_automatic_only_with_the_box(self):
        self.prepare()
        self.m.set_auto(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0'})
        out = self.m.plugins_auto(None, self.home, self.get, now=10 ** 10)
        self.assertEqual(out['downloaded'], {})
        self.assertIn(NEW, out['new'])
        self.m.set_install_new(self.home, True)
        out = self.m.plugins_auto(None, self.home, self.get, now=10 ** 10 + 10 ** 6)
        self.assertEqual(out['downloaded'], {NEW: '1.0.0'})
        self.m.set_auto(self.home, False)   # the box alone never downloads anything
        self.offer('1.2.0.1', {NEW: '1.0.1'})
        self.assertIn('off', self.m.plugins_auto(None, self.home, self.get, now=10 ** 11)['skipped'])

    def test_a_failed_load_removes_a_new_plugin(self):
        self.prepare()
        self.m.set_install_new(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0'})
        self.m.plugins_update(None, self.home, self.get)
        self.prepare()
        self.assertEqual(self.installed(NEW), '1.0.0')
        r = {}
        self.m.check_trials(self.home, r, final=True)
        self.assertIn('removed', r['trial'][NEW])
        self.assertFalse(os.path.exists(os.path.join(self.home, 'homebrew/plugins', NEW)))
        st = self.m.state(self.home)
        self.assertEqual(st['bad'][NEW], ['1.0.0'])
        self.assertNotIn(NEW, st.get('extra') or {})
        self.prepare()
        self.assertIsNone(self.installed(NEW))   # stays gone
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertIn('didn\'t load before', out['skipped'][NEW])

    def test_use_builtin_removes_new_plugins(self):
        self.prepare()
        self.m.set_install_new(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0'})
        self.m.plugins_update(None, self.home, self.get)
        self.prepare()
        self.assertEqual(self.m.plugins_builtin(None, self.home)['removed'], {NEW: '1.0.0'})
        self.prepare()
        self.assertIsNone(self.installed(NEW))
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')

    def test_missing_from_a_manifest_is_never_removed(self):
        self.prepare()
        self.m.set_install_new(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0', 'decky-hearth-sync': '1.2.0.0'})
        self.m.plugins_update(None, self.home, self.get)
        self.prepare()
        self.offer('1.2.0.1', {THIRD: '1.2.0.1'})
        self.m.plugins_update(None, self.home, self.get)
        self.prepare()
        self.assertEqual((self.installed(NEW), self.installed('decky-hearth-sync'), self.installed(THIRD)),
                         ('1.0.0', '1.2.0.0', '1.2.0.1'))

    def test_retired_removes_only_downloaded_copies(self):
        self.prepare()
        self.m.set_install_new(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0', 'decky-hearth-sync': '1.2.0.0'})
        self.m.plugins_update(None, self.home, self.get)
        self.prepare()
        # Retired: the new plugin (downloaded only) and Hearth Sync's download (the image still ships Hearth Sync) and
        # Hearth Link (only the built-in copy, still shipped).
        self.offer('1.2.0.1', {THIRD: '1.2.0.1'}, retired=[NEW, 'decky-hearth-sync', 'decky-hearth-link'])
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertEqual(sorted(out['retired']), [NEW, 'decky-hearth-sync'])
        self.prepare()
        self.assertIsNone(self.installed(NEW))
        self.assertEqual(self.installed('decky-hearth-sync'), '1.1.0')   # the built-in one came back
        self.assertEqual(self.m.versions(self.home, 'decky-hearth-sync')['source'], 'built-in')
        self.assertEqual(self.installed('decky-hearth-link'), '1.1.0')   # built in and still shipped: stays
        self.assertEqual(self.installed(THIRD), '1.2.0.1')
        # An image that no longer ships a retired built-in plugin: then it goes.
        shutil.rmtree(os.path.join(self.root, 'usr/share/hearth/decky-hearth-link'))
        self.prepare()
        self.assertIsNone(self.installed('decky-hearth-link'))
        # A plugin of a Hearth name that isn't Hearth's own is never touched.
        self.w('homebrew/plugins/decky-hearth-other/plugin.json', json.dumps({'name': 'x', 'author': 'someone'}), self.home)
        self.offer('1.2.0.2', {'decky-hearth-other': '9.0'}, retired=[])
        out = self.m.plugins_update(None, self.home, self.get)
        self.assertIn('isn\'t from Hearth', out['skipped']['decky-hearth-other'])
        self.prepare()
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-other/plugin.json')))

    def test_refresh_handles_downloaded_only_plugins(self):
        self.prepare()
        self.m.set_install_new(self.home, True)
        self.offer('1.2.0.0', {NEW: '1.0.0'})
        self.m.plugins_update(None, self.home, self.get)
        self.assertIn(NEW, self.m.plugin_names(self.home))
        # A fresh Decky folder (e.g. emptied): the next start puts the downloaded-only plugin back.
        shutil.rmtree(os.path.join(self.home, 'homebrew/plugins', NEW), ignore_errors=True)
        self.prepare()
        self.assertEqual(self.installed(NEW), '1.0.0')
        self.mark_new('1.0.0')
        self.prepare()
        self.m.note_installed(self.home)
        self.assertIn(NEW, self.m.state(self.home)['plugins'])
        self.assertIn(NEW, self.m.plugins_status(self.home)['plugins'])

    def test_root_swap_rejects_names_outside_the_pattern(self):
        src = os.path.join(self.tmp, 'src')
        for rel, text in plugin_files('x', '1.0').items():
            self.w(rel, text, src)
        for bad in ('../evil', 'decky-hearth-../../etc', 'decky-other', 'decky-hearth-X', '/abs'):
            with self.assertRaises(ValueError):
                self.m.swap_in(self.home, bad, src, {'tree': self.m.tree_hash(src)})
            with self.assertRaises(ValueError):
                self.m.remove_installed(self.home, bad)
            with self.assertRaises(ValueError):
                self.m.roll_back(self.home, bad, '1.0', 'test')
            self.assertIsNone(self.m.downloaded_ok(self.home, bad))
        # A state file naming something else is ignored.
        self.w('.local/state/nimbus/plugin-updates.json', json.dumps({'extra': {'../evil': '1', 'decky-other': '1'}, 'retired': ['../x']}), self.home)
        self.assertEqual(self.m.plugin_names(self.home), list(PLUGINS) + [THIRD])
        self.prepare()
        self.assertFalse(os.path.exists(os.path.join(self.home, 'homebrew/evil')))
        # A planted link for Decky's plugins folder is refused.
        shutil.rmtree(os.path.join(self.home, 'homebrew/plugins'))
        os.symlink(self.tmp, os.path.join(self.home, 'homebrew/plugins'))
        with self.assertRaises(ValueError):
            self.m.swap_in(self.home, NEW, src, {'tree': self.m.tree_hash(src)})

    def test_box_saved_with_the_settings_helpers(self):
        p = self.w('.config/nimbus/plugin-updates.json', json.dumps({'auto': True}), self.home)
        self.assertTrue(self.m.set_install_new(self.home, True))
        self.assertEqual(json.load(open(p)), {'auto': True, 'installNew': True})
        self.assertTrue(os.path.exists(p + '.bak'))
        qml = (ROOT / 'hosts/linux/desktop/shell/SetUpdates.qml').read_text()
        self.assertIn('Also install new Hearth plugins', qml)
        self.assertIn('New from Hearth', qml)
        self.assertIn('installNew', qml)


class PublishWorkflow(unittest.TestCase):
    """decky-plugins-release.yml builds every hosts/decky-hearth-*/ plugin (tools/decky_plugins.py finds them)."""

    def test_discovery_finds_every_plugin_folder(self):
        spec = importlib.util.spec_from_file_location('decky_plugins_t', ROOT / 'tools/decky_plugins.py')
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
        found = [p['id'] for p in m.discover(ROOT)]
        want = sorted(d.name for d in (ROOT / 'hosts').iterdir()
                      if d.name.startswith('decky-hearth-') and (d / 'package.json').is_file() and (d / 'package.sh').is_file()
                      and d.name not in m.RETIRED)
        self.assertEqual(found, want)
        self.assertTrue(set(PLUGINS) <= set(found))
        for p in m.discover(ROOT):
            self.assertTrue(p['name'] and p['description'], p['id'])
        self.assertEqual(m.problems(ROOT), [])   # each has the shared Hearth OS check and the load mark
        # A made-up repo: discovery takes only valid folders with package.json and package.sh, never archive/.
        tmp = pathlib.Path(tempfile.mkdtemp(prefix='discover-'))
        try:
            for n, files in (('decky-hearth-a', ('package.json', 'package.sh', 'plugin.json')), ('decky-hearth-b', ('package.json',)),
                             ('decky-shared', ('package.json', 'package.sh')), ('decky-hearth-Bad', ('package.json', 'package.sh'))):
                (tmp / 'hosts' / n).mkdir(parents=True)
                for f in files:
                    (tmp / 'hosts' / n / f).write_text('{"name": "A", "publish": {"description": "d"}}' if f.endswith('.json') else '')
            (tmp / 'archive/hosts/decky-hearth-old').mkdir(parents=True)
            self.assertEqual([p['id'] for p in m.discover(tmp)], ['decky-hearth-a'])
            self.assertTrue(m.problems(tmp))   # no shared helper, no load mark: the CI check fails
            # A plugin made from the template (with the shared frontend helper copied in) passes the check.
            (tmp / 'hosts/decky-shared').mkdir(parents=True, exist_ok=True)
            shutil.copy(SHARED / 'hearth-only.tsx', tmp / 'hosts/decky-shared/hearth-only.tsx')
            shutil.rmtree(tmp / 'hosts/decky-hearth-a')
            shutil.copytree(SHARED / 'template', tmp / 'hosts/decky-hearth-a')
            shutil.copy(SHARED / 'hearth-only.tsx', tmp / 'hosts/decky-hearth-a/src/hearth-only.tsx')
            self.assertEqual(m.problems(tmp), [])
            self.assertIn('hosts/decky-shared/hearth_only.py', (SHARED / 'template/package.sh').read_text())
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        wf = (ROOT / '.github/workflows/decky-plugins-release.yml').read_text()
        self.assertIn('tools/decky_plugins.py', wf)
        self.assertNotIn('for p in sync link music', wf)


class RetiredPlugins(unittest.TestCase):
    """Retiring a plugin (docs/PLUGIN-UPDATES.md): Hearth Classic (decky-hearth-look) keeps its code in the repository
    (Hearth Eras' Classic era reuses it) but is never built into a plugins release, and every release lists it under
    "retired", which devices accept and act on (their downloaded copy goes, it's no longer offered)."""

    def setUp(self):
        spec = importlib.util.spec_from_file_location('decky_plugins_r', ROOT / 'tools/decky_plugins.py')
        self.dp = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.dp)
        loader = importlib.machinery.SourceFileLoader("hd_r", str(DECKY))
        spec = importlib.util.spec_from_loader('hd_r', loader)
        self.hd = importlib.util.module_from_spec(spec)
        loader.exec_module(self.hd)

    def test_hearth_classic_is_retired_but_kept(self):
        self.assertIn('decky-hearth-look', self.dp.RETIRED)
        self.assertTrue((ROOT / 'hosts/decky-hearth-look/src/home.tsx').is_file())   # the code stays
        ids = [p['id'] for p in self.dp.discover(ROOT)]
        self.assertNotIn('decky-hearth-look', ids)   # never built, checked or released
        self.assertIn('decky-hearth-eras', ids)      # its successor is
        for r in self.dp.RETIRED:
            self.assertTrue(self.dp.PLUGIN_ID.fullmatch(r), r)

    def test_hearth_music_is_retired_archived_and_retired_by_the_image(self):
        # Hearth Music (ThemeDeck replaced it, docs/THEMEDECK.md): code archived, never released, and the image's
        # hearth-decky retires its built-in copy by itself (RETIRED_BUILTIN), never shipping it again.
        self.assertIn('decky-hearth-music', self.dp.RETIRED)
        self.assertFalse((ROOT / 'hosts/decky-hearth-music').exists())
        self.assertTrue((ROOT / 'archive/hosts/decky-hearth-music/main.py').is_file())
        self.assertNotIn('decky-hearth-music', [p['id'] for p in self.dp.discover(ROOT)])
        self.assertIn('decky-hearth-music', self.hd.RETIRED_BUILTIN)
        self.assertNotIn('decky-hearth-music', self.hd.PLUGINS)
        self.assertIn('decky-hearth-music', self.hd.retired_ids({}))
        self.assertIn('decky-hearth-music', (ROOT / 'docs/ARCHIVED.md').read_text())

    def test_manifest_lists_retired_and_devices_accept_it(self):
        tmp = pathlib.Path(tempfile.mkdtemp(prefix='retired-'))
        try:
            for p in self.dp.discover(ROOT):
                with zipfile.ZipFile(tmp / (p['id'] + '.zip'), 'w') as z:
                    z.writestr(p['id'] + '/package.json', json.dumps({'version': '9.9.9'}))
            self.assertFalse((tmp / 'decky-hearth-look.zip').exists())
            m = self.dp.manifest(tmp, '9.9.9', '1.2', '3.0', ROOT)
            self.assertEqual(m['retired'], ['decky-hearth-look', 'decky-hearth-music'])
            self.assertNotIn('decky-hearth-look', [e['id'] for e in m['plugins']])
            self.assertNotIn('decky-hearth-music', [e['id'] for e in m['plugins']])
            parsed = self.hd.parse_manifest((tmp / 'plugins.json').read_bytes(), '9.9.9')
            self.assertEqual(parsed['retired'], ['decky-hearth-look', 'decky-hearth-music'])
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        self.assertEqual(subprocess.run([sys.executable, str(ROOT / 'tools/decky_plugins.py'), 'retired'], capture_output=True,
                                        text=True).stdout.split(), list(self.dp.RETIRED))

    def test_release_workflow_checks_retired(self):
        wf = (ROOT / '.github/workflows/decky-plugins-release.yml').read_text()
        self.assertIn("man['retired']", wf)
        self.assertIn("'tools/decky_plugins.py', 'retired'", wf)


class AutoSwitch(Base):
    def test_defaults_off_and_persists(self):
        self.assertFalse(self.m.auto_on(self.home))
        self.assertFalse(os.path.exists(self.m.settings_path(self.home)))   # the default is never written
        out = self.m.plugins_auto(None, self.home, self.get)
        self.assertIn('off', out['skipped'])
        self.assertEqual(self.fetched, [])   # off: nothing is even checked
        # Other keys in the file stay; the switch is merged with .bak (hearth_settings_io).
        p = self.w('.config/nimbus/plugin-updates.json', json.dumps({'other': 1}), self.home)
        self.assertTrue(self.m.set_auto(self.home, True))
        self.assertEqual(json.load(open(p)), {'other': 1, 'auto': True})
        self.assertTrue(os.path.exists(p + '.bak'))
        self.m = load_decky()
        self.assertTrue(self.m.auto_on(self.home))
        # Checks, downloads and rollbacks never change it.
        self.m.plugins_update(None, self.home, self.get)
        self.m.plugins_builtin(None, self.home)
        self.assertTrue(self.m.auto_on(self.home))
        self.m.set_auto(self.home, False)
        self.assertFalse(self.m.auto_on(self.home))

    def test_mantel_saves_it_with_the_settings_writer(self):
        qml = (ROOT / 'hosts/linux/desktop/shell/SetUpdates.qml').read_text()
        self.assertIn('SettingsWriter', qml)
        self.assertIn('/.config/nimbus/plugin-updates.json', qml)
        self.assertIn('Update plugins automatically', qml)
        self.assertIn('Use the built-in plugins', qml)


class HearthOnly(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-only-')
        self.old = dict(os.environ)
        os.environ['HEARTH_ONLY_ROOT'] = self.tmp
        self.h = load_shared()

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def system(self, release, hearth_files=True):
        os.makedirs(os.path.join(self.tmp, 'etc'), exist_ok=True)
        open(os.path.join(self.tmp, 'etc/os-release'), 'w').write(release)
        if hearth_files:
            os.makedirs(os.path.join(self.tmp, 'usr/libexec/nimbus'), exist_ok=True)

    def test_passes_on_hearth_os(self):
        self.system(HEARTH_OS_RELEASE)
        self.assertEqual(self.h.check(), (True, ''))

    def test_fails_elsewhere(self):
        for rel in (STEAMOS_RELEASE, DEBIAN_RELEASE):
            self.system(rel)
            ok, why = self.h.check()
            self.assertFalse(ok)
            self.assertIn('not Hearth OS', why)
        self.system(HEARTH_OS_RELEASE, hearth_files=False)   # the name alone isn't enough
        shutil.rmtree(os.path.join(self.tmp, 'usr'), ignore_errors=True)
        self.assertFalse(self.h.check()[0])
        shutil.rmtree(os.path.join(self.tmp, 'etc'))
        self.assertEqual(self.h.check(), (False, 'no os-release'))

    def test_hearth_decky_uses_the_same_rule(self):
        root = os.path.join(self.tmp, 'r')
        os.environ['HEARTH_DECKY_ROOT'] = root
        m = load_decky()
        for rel, want in ((HEARTH_OS_RELEASE, True), (STEAMOS_RELEASE, False), (DEBIAN_RELEASE, False)):
            os.makedirs(os.path.join(root, 'etc'), exist_ok=True)
            os.makedirs(os.path.join(root, 'usr/libexec/nimbus'), exist_ok=True)
            open(os.path.join(root, 'etc/os-release'), 'w').write(rel)
            self.assertEqual(m.on_hearth_os(), want)
        os.environ['HEARTH_DECKY_TEST_HOME'] = self.tmp
        with self.assertRaises(ValueError):   # the update channel refuses anywhere but Hearth OS
            m.me()

    def test_each_plugin_starts_nothing_off_hearth_and_logs_once(self):
        self.system(STEAMOS_RELEASE)
        for name, folder in (('Hearth Sync', 'decky-hearth-sync'), ('Hearth Link', 'decky-hearth-link')):
            logs = []
            fake = types.ModuleType('decky')
            fake.DECKY_USER_HOME = os.path.join(self.tmp, 'home')
            fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(self.tmp, 'settings', folder)
            fake.DECKY_PLUGIN_RUNTIME_DIR = os.path.join(self.tmp, 'data', folder)
            fake.DECKY_PLUGIN_DIR = str(ROOT / 'hosts' / folder)
            fake.logger = types.SimpleNamespace(info=logs.append, error=logs.append, warning=logs.append)
            sys.modules['decky'] = fake
            sys.modules.pop('hearth_only', None)
            spec = importlib.util.spec_from_file_location('main_' + folder.replace('-', '_'), ROOT / 'hosts' / folder / 'main.py')
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            p = mod.Plugin()

            async def go():
                r = await p.hearth_os()
                await p.hearth_os()
                await p._main()
                return r
            r = asyncio.run(go())
            self.assertFalse(r['ok'], name)
            self.assertIn('hearthos.fun', r['message'])
            self.assertFalse(hasattr(p, '_sync') or hasattr(p, '_loaded'), name)   # nothing started
            self.assertEqual(len([l for l in logs if 'features stay off' in l]), 1, (name, logs))
        sys.modules.pop('decky', None)

    def test_load_mark(self):
        self.system(HEARTH_OS_RELEASE)
        d = os.path.join(self.tmp, 'data')
        self.assertTrue(self.h.write_loaded(d, '1.1.1', now=5))
        self.assertEqual(json.load(open(os.path.join(d, 'hearth-loaded.json'))), {'version': '1.1.1', 'at': 5})

    def test_frontend_copies_match_and_panel_invites(self):
        canon = (SHARED / 'hearth-only.tsx').read_text()
        for n in PLUGINS:
            self.assertEqual((ROOT / 'hosts' / n / 'src/hearth-only.tsx').read_text(), canon, n)
            self.assertIn('hearth_only.py', (ROOT / 'hosts' / n / 'package.sh').read_text(), n)
            self.assertIn('<HearthOnly', (ROOT / 'hosts' / n / 'src/index.tsx').read_text(), n)
        for want in ('Get Hearth OS', 'Free and open source', 'NavigateToExternalWeb', 'https://hearthos.fun', 'Steam Game Mode, like SteamOS'):
            self.assertIn(want, canon)
        py = (SHARED / 'hearth_only.py').read_text()
        for never in ('urlopen', 'socket', 'machine-id', 'uuid'):
            self.assertNotIn(never, py)   # a check, not DRM: nothing phones home


if __name__ == '__main__':
    unittest.main()
