"""Hearth Eras' backend (hosts/decky-hearth-eras/main.py): the settings off by default and remembered safely, the Hearth
OS gate, the load mark, the skins and pictures, the home data, the log and sending it; plus the plugin's rules (the
Classic era's code is Hearth Classic's, unchanged; no one else's names). No Steam, no Decky (a stand-in), no network."""
import asyncio
import importlib.util
import io
import json
import os
import pathlib
import re
import shutil
import ssl
import subprocess
import sys
import tempfile
import types
import unittest
import urllib.error

HERE = pathlib.Path(__file__).resolve().parent
PLUGIN = HERE.parent
ROOT = PLUGIN.parents[1]
LOOK = ROOT / 'hosts/decky-hearth-look'


def system(root, hearth=True):
    os.makedirs(os.path.join(root, 'etc'), exist_ok=True)
    with open(os.path.join(root, 'etc/os-release'), 'w') as f:
        f.write('NAME="Hearth"\nVARIANT_ID=hearth-debian\nPRETTY_NAME="Hearth OS"\n' if hearth else 'NAME="SteamOS"\nID=steamos\n')
    if hearth:
        os.makedirs(os.path.join(root, 'usr/libexec/nimbus'), exist_ok=True)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-eras-')
        self.sys_root = os.path.join(self.tmp, 'root')
        system(self.sys_root, True)
        os.environ['HEARTH_ONLY_ROOT'] = self.sys_root
        self.logs = []
        fake = types.ModuleType('decky')
        fake.DECKY_USER_HOME = os.path.join(self.tmp, 'home')
        fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(self.tmp, 'settings', 'decky-hearth-eras')
        fake.DECKY_PLUGIN_RUNTIME_DIR = os.path.join(self.tmp, 'data', 'decky-hearth-eras')
        fake.DECKY_PLUGIN_DIR = str(PLUGIN)
        fake.logger = types.SimpleNamespace(info=self.logs.append, error=self.logs.append, warning=self.logs.append)
        sys.modules['decky'] = fake
        sys.modules.pop('hearth_only', None)
        spec = importlib.util.spec_from_file_location('eras_main', PLUGIN / 'main.py')
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)
        self.m.HOME_CACHE.update(at=0.0, result=None)
        self.sfile = os.path.join(self.tmp, 'settings', 'decky-hearth-eras', 'settings.json')

    def tearDown(self):
        os.environ.pop('HEARTH_ONLY_ROOT', None)
        sys.modules.pop('decky', None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_(self, coro):
        return asyncio.run(coro)


class Settings(Base):
    def test_default_is_steam_and_nothing_is_written(self):
        p = self.m.Plugin()
        s = self.run_(p.get_settings())
        self.assertEqual(s['era'], 'steam')   # Steam's look: off
        for k in ('sounds', 'quickSwitch', 'loading', 'keyboard', 'pages'):
            self.assertFalse(s[k], k)          # every extra off
        self.assertEqual(s['presets'], {})
        self.run_(p._main())
        self.run_(p.get_settings())
        self.assertFalse(os.path.exists(self.sfile))   # starting and reading never write
        self.run_(p._unload())

    def test_a_change_is_remembered_and_only_the_change_is_written(self):
        p = self.m.Plugin()
        r = self.run_(p.set_settings({'era': 'crossbar'}))
        self.assertTrue(r['ok'])
        self.assertEqual(r['settings']['era'], 'crossbar')
        self.assertEqual(json.load(open(self.sfile)), {'era': 'crossbar'})   # the defaults never go in the file
        self.run_(p.set_settings({'sounds': True}))
        self.run_(p.set_settings({'presets': {'crossbar': 2}}))
        self.run_(p.set_settings({'presets': {'blades': 1}}))   # merges: crossbar's stays
        s = self.run_(p.get_settings())
        self.assertEqual((s['era'], s['sounds'], s['presets']), ('crossbar', True, {'crossbar': 2, 'blades': 1}))
        # Every era can be picked, and Steam again.
        for e in self.m.ERAS:
            self.assertEqual(self.run_(p.set_settings({'era': e}))['settings']['era'], e)

    def test_bad_values_and_unknown_keys_are_refused(self):
        p = self.m.Plugin()
        for bad in ({'era': 'other'}, {'era': 3}, {'sounds': 'yes'}, {'presets': {'steam': 1}}, {'presets': {'blades': 3}},
                    {'presets': {'blades': True}}, {'presets': []}, {'evil': 1}, {}, 'era'):
            self.assertFalse(self.run_(p.set_settings(bad))['ok'], bad)
        self.assertFalse(os.path.exists(self.sfile))
        # Rubbish in the file reads as the defaults for those keys.
        os.makedirs(os.path.dirname(self.sfile))
        json.dump({'era': 'nope', 'sounds': 1, 'presets': {'tiles': 9, 'pocket': 1, 'x': 0}}, open(self.sfile, 'w'))
        s = self.m.read_settings()[0]
        self.assertEqual((s['era'], s['sounds'], s['presets']), ('steam', False, {'pocket': 1}))

    def test_merge_on_write_keeps_keys_a_newer_version_wrote(self):
        os.makedirs(os.path.dirname(self.sfile))
        json.dump({'era': 'tiles', 'future': {'x': 1}}, open(self.sfile, 'w'))
        self.run_(self.m.Plugin().set_settings({'keyboard': True}))
        self.assertEqual(json.load(open(self.sfile)), {'era': 'tiles', 'future': {'x': 1}, 'keyboard': True})
        self.assertEqual(json.load(open(self.sfile + '.bak')), {'era': 'tiles', 'future': {'x': 1}})

    def test_damaged_file_is_put_back_from_bak(self):
        os.makedirs(os.path.dirname(self.sfile))
        open(self.sfile, 'w').write('{"era": "pock')
        json.dump({'era': 'pocket'}, open(self.sfile + '.bak', 'w'))
        s, state = self.m.read_settings()
        self.assertEqual((s['era'], state), ('pocket', 'restored'))
        self.assertEqual(json.load(open(self.sfile)), {'era': 'pocket'})

    def test_damaged_file_without_bak_is_never_replaced_with_defaults(self):
        os.makedirs(os.path.dirname(self.sfile))
        open(self.sfile, 'w').write('{"era": "blad')
        s, state = self.m.read_settings()
        self.assertEqual((s['era'], state), ('steam', 'damaged'))
        self.run_(self.m.Plugin().get_settings())
        self.assertEqual(open(self.sfile).read(), '{"era": "blad')   # reading never overwrites it
        self.run_(self.m.Plugin().set_settings({'era': 'blades'}))   # the person's change: the damaged copy is kept
        self.assertEqual(open(self.sfile + '.damaged').read(), '{"era": "blad')
        self.assertEqual(json.load(open(self.sfile)), {'era': 'blades'})

    def test_writes_are_atomic(self):
        self.run_(self.m.Plugin().set_settings({'era': 'channels'}))
        self.run_(self.m.Plugin().set_settings({'era': 'tiles'}))
        left = [f for f in os.listdir(os.path.dirname(self.sfile)) if f.endswith('.tmp')]
        self.assertEqual(left, [])
        self.assertEqual(json.load(open(self.sfile + '.bak')), {'era': 'channels'})

    def test_eras_match_the_frontend(self):
        ts = (PLUGIN / 'src/model.ts').read_text()
        for e in self.m.ERAS:
            self.assertIn('id: "%s"' % e, ts)
        self.assertIn('MAX_PRESETS = %d' % self.m.PRESETS, ts)
        for k in self.m.EXTRAS:
            self.assertIn('"%s"' % k, ts)


class Gate(Base):
    def test_off_hearth_os_nothing_starts_and_calls_refuse(self):
        system(self.sys_root, False)
        shutil.rmtree(os.path.join(self.sys_root, 'usr'), ignore_errors=True)
        p = self.m.Plugin()
        r = self.run_(p.hearth_os())
        self.assertFalse(r['ok'])
        self.assertIn('hearthos.fun', r['message'])
        self.run_(p._main())
        self.assertFalse(hasattr(p, '_loaded'))
        self.assertFalse(self.run_(p.set_settings({'era': 'tiles'}))['ok'])
        self.assertFalse(os.path.exists(self.sfile))
        self.assertIsNone(self.run_(p.brand()))
        self.assertFalse(self.run_(p.home())['ok'])
        self.assertFalse(self.run_(p.send_log('x', 'y'))['ok'])
        self.assertEqual(self.run_(p.art('/etc/passwd')), '')
        self.assertEqual(len([line for line in self.logs if 'features stay off' in line]), 1)

    def test_on_hearth_os(self):
        self.assertTrue(self.run_(self.m.Plugin().hearth_os())['ok'])


class LoadMark(Base):
    def test_the_load_mark_is_written(self):
        self.run_(self.m.hearth_only.mark_loaded_later(sys.modules['decky'], delay=0))
        mark = os.path.join(self.tmp, 'data', 'decky-hearth-eras', 'hearth-loaded.json')
        self.assertEqual(json.load(open(mark))['version'], json.load(open(PLUGIN / 'package.json'))['version'])

    def test_main_starts_the_load_mark_task(self):
        p = self.m.Plugin()

        async def go():
            await p._main()
            self.assertTrue(hasattr(p, '_loaded'))
            await p._unload()
        self.run_(go())

    def test_follows_the_template(self):
        src = (PLUGIN / 'main.py').read_text()
        self.assertIn('import hearth_only', src)
        self.assertIn('hearth_only.mark_loaded_later(', src)
        self.assertIn('hosts/decky-shared/hearth_only.py', (PLUGIN / 'package.sh').read_text())
        self.assertEqual((PLUGIN / 'src/hearth-only.tsx').read_text(), (ROOT / 'hosts/decky-shared/hearth-only.tsx').read_text())
        self.assertIn('<HearthOnly', (PLUGIN / 'src/index.tsx').read_text())
        pkg = json.load(open(PLUGIN / 'package.json'))
        self.assertEqual((pkg['name'], pkg['version']), ('decky-hearth-eras', '0.0.0'))   # the release sets the owner's number
        self.assertEqual(json.load(open(PLUGIN / 'plugin.json'))['name'], 'Hearth Eras')


class Skins(Base):
    def test_brand_has_every_skin_and_hearths_pictures(self):
        b = self.m.brand()
        self.assertIn('@section keyboard', b['base'])
        for e in self.m.SKINNED:
            self.assertTrue(b['eras'][e].strip(), e)
        self.assertEqual(b['eras']['classic'], (LOOK / 'defaults/hearth-classic.css').read_text())
        self.assertTrue(b['logo'].startswith('data:image/svg+xml;base64,'))

    def test_packed_layout(self):
        tmp = pathlib.Path(self.tmp) / 'packed'
        (tmp / 'eras').mkdir(parents=True)
        (tmp / 'assets').mkdir()
        for f in (PLUGIN / 'defaults/eras').glob('*.css'):
            shutil.copy(f, tmp / 'eras' / f.name)
        (tmp / 'hearth-classic.css').write_text('/* classic */')
        shutil.copy(ROOT / 'site/img/logo.svg', tmp / 'assets/logo.svg')
        b = self.m.brand(str(tmp))
        self.assertEqual(b['eras']['classic'], '/* classic */')
        self.assertTrue(b['eras']['pocket'])
        sh = (PLUGIN / 'package.sh').read_text()
        for want in ('defaults/eras/*.css', 'hosts/decky-hearth-look/defaults/hearth-classic.css', 'assets/logo.svg', 'assets/wallpaper.jpg'):
            self.assertIn(want, sh)

    def test_one_rule_per_line_and_no_menu_window_reshaping(self):
        for f in (PLUGIN / 'defaults/eras').glob('*.css'):
            css = f.read_text()
            self.assertEqual(css.count('/* @section'), css.count('/* @end */'), f.name)
            for line in css.splitlines():
                if '{{' in line:
                    self.assertRegex(line, r'\{\{[A-Za-z0-9_]+:[A-Za-z0-9_]+\}\}')
                    self.assertLessEqual(re.sub(r'\{\{[^}]*\}\}', '', line).count('{'), 1, line)   # one rule per line
            # Eras never touches Steam's menu window (Hearth Classic's lesson: Steam keeps it 241x644).
            for bad in ('MainNavMenuContainer', 'hearth-bar', 'ResizeTo', 'MoveTo'):
                self.assertNotIn(bad, css)
        src = ''.join(p.read_text() for p in (PLUGIN / 'src').glob('*.ts*'))
        for bad in ('ResizeTo', 'MoveTo', 'resizeTo', 'SetBounds', 'startMenuBar', 'MainNavMenuContainer'):
            self.assertNotIn(bad, src)


class HomeAndPictures(Base):
    def test_home_layout_and_hearth_sync_shortcuts(self):
        sync = os.path.join(self.tmp, 'settings', 'decky-hearth-sync')
        os.makedirs(sync)
        json.dump({'epic:a': 3000000001, 'gog:1': 'nope'}, open(os.path.join(sync, 'shortcuts.json'), 'w'))
        calls = []

        def run(cmd, **kw):
            calls.append(cmd)
            return subprocess.CompletedProcess(cmd, 0, stdout='noise\n{"hero": null, "rows": []}\n', stderr='')
        r = self.m.home_layout(run=run, now=1000, finder=lambda n: '/usr/bin/nimbus')
        self.assertTrue(r['ok'])
        self.assertEqual(r['shortcuts'], {'epic:a': 3000000001})
        self.assertEqual(calls, [['/usr/bin/nimbus', '--steam-mode-data', 'home']])
        self.m.home_layout(run=run, now=1030, finder=lambda n: '/usr/bin/nimbus')
        self.assertEqual(len(calls), 1)   # at most once a minute
        self.m.HOME_CACHE.update(at=0.0, result=None)
        self.assertFalse(self.m.home_layout(now=1, finder=lambda n: None)['ok'])

    def test_pictures_only_from_hearth_and_steam_folders(self):
        h = os.path.join(self.tmp, 'home')
        d = os.path.join(h, '.local', 'share', 'Steam', 'appcache', 'librarycache')
        os.makedirs(d)
        open(os.path.join(d, '1_hero.png'), 'wb').write(b'\x89PNG\r\n\x1a\n' + b'0' * 20)
        self.assertTrue(self.m.read_art(os.path.join(d, '1_hero.png'), h).startswith('data:image/png;base64,'))
        self.assertEqual(self.m.read_art('/etc/passwd', h), '')
        self.assertEqual(self.m.read_art(os.path.join(d, '..', '..', '..', '..', '..', '..', 'x'), h), '')


class Logs(Base):
    def curl_run(self, calls, stdout='{"code":"CURL-CODE-01"}\n200', rc=0, stderr=''):
        def run(args, **kw):
            calls.append((args, kw))
            return subprocess.CompletedProcess(args, rc, stdout=stdout, stderr=stderr)
        return run

    def test_diagnostics_go_to_hearths_log_folder(self):
        r = self.run_(self.m.Plugin().diagnostics('Hearth Eras: on\nEra: tiles'))
        self.assertTrue(r['ok'])
        self.assertEqual(r['path'], '~/.local/share/nimbus/hearth-eras.log')
        self.assertIn('Era: tiles', open(self.m.diag_log_path()).read())

    def test_send_by_hand_falls_back_to_curl_and_has_no_personal_details(self):
        def ssl_fail(req, timeout=60, **kw):
            raise urllib.error.URLError(ssl.SSLCertVerificationError(1, 'certificate verify failed'))
        calls = []
        r = self.m.send_now('by hand', 'Era: tiles', opener=ssl_fail, now=0, run=self.curl_run(calls), finder=lambda n: '/usr/bin/curl', ctx=ssl.create_default_context())
        self.assertEqual((r['ok'], r['code'], r['via']), (True, 'CURL-CODE-01', 'curl'))
        self.assertIn('X-Hearth-Kind: peppermint', calls[0][0])
        self.assertEqual(self.run_(self.m.Plugin().last_sent())['code'], 'CURL-CODE-01')
        b = self.m.send_body('test', 'Era: tiles', '1.3', home_dir=os.path.join(self.tmp, 'home'), names=set(), now=0)
        self.assertTrue(b.startswith('Hearth logs (Hearth Eras diagnostics only)'))
        t = self.m.redact('token=abc123 76561198012345678 a.b@c.de', os.path.join(self.tmp, 'home'), set())
        for bad in ('abc123', '76561198012345678', 'a.b@c.de'):
            self.assertNotIn(bad, t)

    def test_direct_send_says_who_it_is(self):
        sent = []

        class R:
            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False

            def read(self, n):
                return b'{"code":"DIRECT-01"}'

        def op(req, timeout=60, **kw):
            sent.append(req)
            return R()
        r = self.m.deliver('Hearth logs x', 'n', '1.3', op, None, None, ctx=ssl.create_default_context())
        self.assertEqual((r['ok'], r['via']), (True, 'direct'))
        self.assertTrue(sent[0].get_header('User-agent').startswith('hearth-eras/1.3'))

    def test_daily_limit(self):
        self.m.write_send_state({'days': {__import__('time').strftime('%Y-%m-%d', __import__('time').localtime(0)): self.m.MANUAL_PER_DAY}})
        self.assertEqual(self.m.send_now('x', 'y', now=0)['skipped'], 'the limit for today is reached')


class Rules(unittest.TestCase):
    def test_the_classic_era_reuses_hearth_classics_code_unchanged(self):
        for f in ('model.ts', 'home.tsx'):
            self.assertEqual((PLUGIN / 'src/classic' / f).read_text(), (LOOK / 'src' / f).read_text(), f)

    def test_no_console_makers_names(self):
        # Hearth's own designs: no console maker's or console's names anywhere in Eras' own code, skins or texts
        # (src/classic/ is Hearth Classic's code, unchanged; its store row names come from Hearth Sync).
        words = r'\b(xbox|playstation|nintendo|sony|microsoft|wii|xmb|psp|vita|gamecube|dreamcast|sega|ps[1-5]|switch\s*os)\b'
        files = [PLUGIN / 'main.py', PLUGIN / 'plugin.json', PLUGIN / 'package.json', *(PLUGIN / 'defaults/eras').glob('*.css'),
                 *(p for p in (PLUGIN / 'src').glob('*.ts*'))]
        for f in files:
            text = re.sub(r'base64,[A-Za-z0-9+/=]+', '', f.read_text())
            self.assertIsNone(re.search(words, text, re.I), '%s: %s' % (f.name, (re.search(words, text, re.I) or [''])[0]))

    def test_no_recordings_or_fonts_shipped(self):
        for p in PLUGIN.rglob('*'):
            if 'node_modules' in p.parts:
                continue
            self.assertNotIn(p.suffix.lower(), ('.wav', '.mp3', '.ogg', '.flac', '.ttf', '.otf', '.woff', '.woff2'), p)


if __name__ == '__main__':
    unittest.main()
