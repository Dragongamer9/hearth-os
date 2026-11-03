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
import tempfile
import types
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
DECKY = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-decky'
SHARED = ROOT / 'hosts/decky-shared'
PLUGINS = ('decky-hearth-sync', 'decky-hearth-link', 'decky-hearth-music')
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
        for n in PLUGINS:
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
                corrupt=None, sums_wrong=None, broken=None):
        """A made-up plugins release: its API entry and the downloads behind it, served by self.get."""
        tag = tag or 'plugins-v' + version
        files, entries, sums = {}, [], []
        for n, v in plugins.items():
            data = make_zip(n, plugin_files(n, v, broken=(n == broken)))
            sha = hashlib.sha256(data).hexdigest()
            entries.append({'id': n, 'version': v, 'archive': n + '.zip', 'sha256': sha, 'size': len(data),
                            'min_hearth_os': min_hearth, 'min_decky': min_decky})
            sums.append('%s  %s.zip' % ('0' * 64 if n == sums_wrong else sha, n))
            files[n + '.zip'] = data + (b'tampered' if n == corrupt else b'')
        files['plugins.json'] = json.dumps({'format': 1, 'version': version, 'plugins': entries}).encode()
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
        r = self.release('1.1.1', {'decky-hearth-sync': '1.1.1', 'decky-hearth-music': '1.1.1'})
        data = self.served[[a['browser_download_url'] for a in r['assets'] if a['name'] == 'plugins.json'][0]]
        m = self.m.parse_manifest(data, '1.1.1')
        self.assertEqual([p['id'] for p in m['plugins']], ['decky-hearth-sync', 'decky-hearth-music'])
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
        self.serve(self.release('1.1.1', {'decky-hearth-sync': '1.1.1', 'decky-hearth-link': '1.1.0', 'decky-hearth-music': '1.0.9'}))
        out, got = self.m.plugins_check(self.home, self.get)
        self.assertEqual(out['available'], {'decky-hearth-sync': '1.1.1'})
        self.assertIn('not newer', out['skipped']['decky-hearth-link'])
        self.assertIn('not newer', out['skipped']['decky-hearth-music'])
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
        for name, folder in (('Hearth Sync', 'decky-hearth-sync'), ('Hearth Link', 'decky-hearth-link'), ('Hearth Music', 'decky-hearth-music')):
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
