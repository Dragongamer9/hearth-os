"""Hearth Classic's backend (hosts/decky-hearth-look/main.py): settings off by default and remembered safely, the
Hearth OS gate, the load mark, the home data and pictures. No Steam, no Decky (a stand-in module), no network."""
import asyncio
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import types
import unittest
from urllib.parse import unquote as urllib_unquote

HERE = pathlib.Path(__file__).resolve().parent
PLUGIN = HERE.parent
ROOT = PLUGIN.parents[1]


def system(root, hearth=True):
    os.makedirs(os.path.join(root, 'etc'), exist_ok=True)
    with open(os.path.join(root, 'etc/os-release'), 'w') as f:
        f.write('NAME="Hearth"\nVARIANT_ID=hearth-debian\nPRETTY_NAME="Hearth OS"\n' if hearth else 'NAME="SteamOS"\nID=steamos\n')
    if hearth:
        os.makedirs(os.path.join(root, 'usr/libexec/nimbus'), exist_ok=True)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-look-')
        self.sys_root = os.path.join(self.tmp, 'root')
        system(self.sys_root, True)
        os.environ['HEARTH_ONLY_ROOT'] = self.sys_root
        self.logs = []
        fake = types.ModuleType('decky')
        fake.DECKY_USER_HOME = os.path.join(self.tmp, 'home')
        fake.DECKY_PLUGIN_SETTINGS_DIR = os.path.join(self.tmp, 'settings', 'decky-hearth-look')
        fake.DECKY_PLUGIN_RUNTIME_DIR = os.path.join(self.tmp, 'data', 'decky-hearth-look')
        fake.DECKY_PLUGIN_DIR = str(PLUGIN)
        fake.logger = types.SimpleNamespace(info=self.logs.append, error=self.logs.append, warning=self.logs.append)
        sys.modules['decky'] = fake
        sys.modules.pop('hearth_only', None)
        spec = importlib.util.spec_from_file_location('look_main', PLUGIN / 'main.py')
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)
        self.m.HOME_CACHE.update(at=0.0, result=None)
        self.sfile = os.path.join(self.tmp, 'settings', 'decky-hearth-look', 'settings.json')

    def tearDown(self):
        os.environ.pop('HEARTH_ONLY_ROOT', None)
        sys.modules.pop('decky', None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_(self, coro):
        return asyncio.run(coro)


class Settings(Base):
    def test_default_is_off_and_nothing_is_written(self):
        p = self.m.Plugin()
        s = self.run_(p.get_settings())
        self.assertEqual(s['look'], 'steam')          # Steam's look: Hearth Classic off
        self.assertFalse(s['sounds'])                  # sounds off by default
        self.run_(p._main())
        self.assertFalse(os.path.exists(self.sfile))   # starting never writes settings
        self.run_(p._unload())

    def test_a_change_is_remembered(self):
        p = self.m.Plugin()
        r = self.run_(p.set_settings({'look': 'classic'}))
        self.assertTrue(r['ok'])
        self.assertEqual(r['settings']['look'], 'classic')
        self.run_(p.set_settings({'sounds': True}))
        # A fresh load (a restart) reads the same.
        self.setUp_again()
        s = self.run_(self.m.Plugin().get_settings())
        self.assertEqual((s['look'], s['sounds'], s['home'], s['wallpaper']), ('classic', True, True, True))
        self.assertTrue(os.path.exists(self.sfile + '.bak'))   # the previous good copy is kept

    def setUp_again(self):
        sys.modules.pop('hearth_only', None)
        spec = importlib.util.spec_from_file_location('look_main2', PLUGIN / 'main.py')
        self.m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.m)

    def test_bad_values_and_unknown_keys_are_refused_or_ignored(self):
        p = self.m.Plugin()
        self.assertFalse(self.run_(p.set_settings({'look': 'something'}))['ok'])
        self.assertFalse(self.run_(p.set_settings({'sounds': 'yes'}))['ok'])
        self.assertFalse(self.run_(p.set_settings('look=classic'))['ok'])
        self.assertFalse(os.path.exists(self.sfile))
        r = self.run_(p.set_settings({'look': 'classic', 'evil': 1}))
        self.assertNotIn('evil', json.load(open(self.sfile)))
        self.assertEqual(r['settings']['look'], 'classic')

    def test_merge_on_write_keeps_keys_a_newer_version_wrote(self):
        os.makedirs(os.path.dirname(self.sfile), exist_ok=True)
        json.dump({'look': 'classic', 'future': {'x': 1}}, open(self.sfile, 'w'))
        self.run_(self.m.Plugin().set_settings({'sounds': True}))
        saved = json.load(open(self.sfile))
        self.assertEqual(saved['future'], {'x': 1})
        self.assertEqual((saved['look'], saved['sounds']), ('classic', True))

    def test_damaged_file_is_put_back_from_bak(self):
        p = self.m.Plugin()
        self.run_(p.set_settings({'look': 'classic'}))
        self.run_(p.set_settings({'wallpaper': False}))   # .bak now holds look=classic, wallpaper on
        open(self.sfile, 'w').write('{broken')
        s = self.run_(p.get_settings())
        self.assertEqual(s['look'], 'classic')
        self.assertEqual(json.load(open(self.sfile))['look'], 'classic')   # put back

    def test_damaged_file_without_bak_is_never_replaced_with_defaults(self):
        os.makedirs(os.path.dirname(self.sfile), exist_ok=True)
        open(self.sfile, 'w').write('{"look": "classic",')   # cut off
        p = self.m.Plugin()
        self.assertEqual(self.run_(p.get_settings())['look'], 'steam')   # read as off for now...
        self.run_(p._main())
        self.assertEqual(open(self.sfile).read(), '{"look": "classic",')   # ...but the file is left alone
        self.run_(p._unload())
        # Only the person's own change writes, and the damaged file is kept beside it.
        self.run_(p.set_settings({'look': 'classic'}))
        self.assertEqual(json.load(open(self.sfile))['look'], 'classic')
        self.assertEqual(open(self.sfile + '.damaged').read(), '{"look": "classic",')
        self.assertFalse(os.path.exists(self.sfile + '.bak'))   # a damaged file never becomes the .bak

    def test_menu_bars_default_on_and_remembered_off(self):
        p = self.m.Plugin()
        s = self.run_(p.get_settings())
        self.assertTrue(s['menu'] and s['menuTop'])
        self.assertFalse(os.path.exists(self.sfile))   # the defaults are never written
        self.run_(p.set_settings({'look': 'classic', 'menuTop': False}))
        self.assertFalse(self.run_(p.set_settings({'menu': 'off'}))['ok'])
        s = self.run_(p.get_settings())
        self.assertEqual((s['menu'], s['menuTop']), (True, False))
        self.assertNotIn('menu', json.load(open(self.sfile)))   # only what the person changed is written

    def test_writes_are_atomic(self):
        p = self.m.Plugin()
        self.run_(p.set_settings({'look': 'classic'}))
        d = os.path.dirname(self.sfile)
        self.assertEqual([f for f in os.listdir(d) if f.endswith('.tmp')], [])


class HearthOnly(Base):
    def test_off_hearth_os_nothing_starts_and_calls_refuse(self):
        system(self.sys_root, False)
        shutil.rmtree(os.path.join(self.sys_root, 'usr'), ignore_errors=True)
        p = self.m.Plugin()
        r = self.run_(p.hearth_os())
        self.assertFalse(r['ok'])
        self.assertIn('hearthos.fun', r['message'])
        self.run_(p._main())
        self.assertFalse(hasattr(p, '_loaded'))
        self.assertFalse(self.run_(p.set_settings({'look': 'classic'}))['ok'])
        self.assertFalse(os.path.exists(self.sfile))
        self.assertIsNone(self.run_(p.brand()))
        self.assertFalse(self.run_(p.home())['ok'])
        self.assertEqual(self.run_(p.art('/etc/passwd')), '')
        self.assertEqual(len([l for l in self.logs if 'features stay off' in l]), 1)

    def test_on_hearth_os(self):
        self.assertTrue(self.run_(self.m.Plugin().hearth_os())['ok'])


class LoadMark(Base):
    def test_the_load_mark_is_written_on_hearth_os(self):
        self.run_(self.m.hearth_only.mark_loaded_later(sys.modules['decky'], delay=0))
        mark = os.path.join(self.tmp, 'data', 'decky-hearth-look', 'hearth-loaded.json')
        want = json.load(open(PLUGIN / 'package.json'))['version']
        self.assertEqual(json.load(open(mark))['version'], want)

    def test_main_starts_the_load_mark_task(self):
        p = self.m.Plugin()

        async def go():
            await p._main()
            self.assertTrue(hasattr(p, '_loaded'))
            await p._unload()
        self.run_(go())

    def test_main_py_follows_the_template(self):
        src = (PLUGIN / 'main.py').read_text()
        self.assertIn('import hearth_only', src)
        self.assertIn('hearth_only.mark_loaded_later(', src)
        self.assertIn('hosts/decky-shared/hearth_only.py', (PLUGIN / 'package.sh').read_text())
        self.assertEqual((PLUGIN / 'src/hearth-only.tsx').read_text(), (ROOT / 'hosts/decky-shared/hearth-only.tsx').read_text())


class HomeData(Base):
    def fake_run(self, out, calls):
        def run(cmd, **kw):
            calls.append(cmd)
            return subprocess.CompletedProcess(cmd, 0, stdout=out, stderr='')
        return run

    def test_home_layout_and_hearth_sync_shortcuts(self):
        sync = os.path.join(self.tmp, 'settings', 'decky-hearth-sync')
        os.makedirs(sync)
        json.dump({'epic:fortnite': 3000000001, 'gog:1': 'nope', 'bad': True}, open(os.path.join(sync, 'shortcuts.json'), 'w'))
        calls = []
        home = {'hero': {'id': 'steam:620', 'title': 'Portal 2'}, 'continue': [], 'favorites': [], 'rows': [], 'added': [], 'count': 1}
        r = self.m.home_layout(run=self.fake_run('noise\n' + json.dumps(home) + '\n', calls), now=1000, finder=lambda n: '/usr/bin/nimbus')
        self.assertTrue(r['ok'])
        self.assertEqual(r['home']['hero']['id'], 'steam:620')
        self.assertEqual(r['shortcuts'], {'epic:fortnite': 3000000001})
        self.assertEqual(calls, [['/usr/bin/nimbus', '--steam-mode-data', 'home']])
        # Asked at most once a minute.
        self.m.home_layout(run=self.fake_run('{}', calls), now=1030, finder=lambda n: '/usr/bin/nimbus')
        self.assertEqual(len(calls), 1)
        self.m.home_layout(run=self.fake_run(json.dumps(home), calls), now=1100, finder=lambda n: '/usr/bin/nimbus')
        self.assertEqual(len(calls), 2)

    def test_home_errors(self):
        self.assertFalse(self.m.home_layout(now=1, finder=lambda n: None)['ok'])
        self.m.HOME_CACHE.update(at=0.0, result=None)

        def boom(cmd, **kw):
            raise subprocess.TimeoutExpired(cmd, 20)
        r = self.m.home_layout(run=boom, now=1, finder=lambda n: '/usr/bin/nimbus')
        self.assertFalse(r['ok'])
        self.m.HOME_CACHE.update(at=0.0, result=None)
        r = self.m.home_layout(run=self.fake_run('{"error":"nope"}', []), now=1, finder=lambda n: '/usr/bin/nimbus')
        self.assertEqual(r, {'ok': False, 'error': 'nope', 'shortcuts': {}})

    def test_pictures_only_from_hearth_and_steam_folders(self):
        h = os.path.join(self.tmp, 'home')
        cache = os.path.join(h, '.local', 'share', 'nimbus', 'art-cache')
        os.makedirs(cache)
        png = b'\x89PNG\r\n\x1a\n' + b'0' * 20
        open(os.path.join(cache, 'a.png'), 'wb').write(png)
        open(os.path.join(h, 'secret.png'), 'wb').write(png)
        self.assertTrue(self.m.read_art(os.path.join(cache, 'a.png')).startswith('data:image/png;base64,'))
        self.assertEqual(self.m.read_art(os.path.join(h, 'secret.png')), '')
        self.assertEqual(self.m.read_art(os.path.join(cache, '..', '..', '..', '..', 'secret.png')), '')
        open(os.path.join(cache, 'b.png'), 'wb').write(b'not a picture')
        self.assertEqual(self.m.read_art(os.path.join(cache, 'b.png')), '')

    def test_brand_has_the_skin_and_pictures(self):
        b = self.m.brand(str(PLUGIN))
        self.assertIn('{{gamepadTabbedPageClasses:TabHeaderRowWrapper}}', b['css'])
        self.assertIn('#3fd0b8', b['css'])
        tmp = os.path.join(self.tmp, 'packed')
        os.makedirs(os.path.join(tmp, 'assets'))
        shutil.copy(ROOT / 'site/img/logo.svg', os.path.join(tmp, 'assets', 'logo.svg'))
        shutil.copy(ROOT / 'os/branding/wallpapers/hearth-embers.jpg', os.path.join(tmp, 'assets', 'wallpaper.jpg'))
        shutil.copy(PLUGIN / 'defaults/hearth-classic.css', os.path.join(tmp, 'hearth-classic.css'))
        b = self.m.brand(tmp)
        self.assertTrue(b['logo'].startswith('data:image/svg+xml;base64,'))
        self.assertTrue(b['wallpaper'].startswith('data:image/jpeg;base64,'))

    def test_diagnostics_go_to_hearths_log_folder(self):
        p = self.m.Plugin()
        r = self.run_(p.diagnostics('Menu: found\nMenu bar: bar'))
        self.assertTrue(r['ok'])
        self.assertEqual(r['path'], '~/.local/share/nimbus/hearth-classic.log')
        f = os.path.join(self.tmp, 'home', '.local', 'share', 'nimbus', 'hearth-classic.log')
        with open(f) as fh:
            text = fh.read()
        self.assertIn('Menu bar: bar', text)
        self.assertTrue(any('diagnostics: Menu: found' in l for l in self.logs))
        # Kept small: past the limit, the newer half stays.
        with open(f, 'a') as fh:
            fh.write('x' * (self.m.DIAG_LIMIT + 10) + '\n')
        self.m.write_diag('after')
        self.assertLess(os.path.getsize(f), self.m.DIAG_LIMIT)
        with open(f) as fh:
            self.assertTrue(fh.read().rstrip().endswith('after'))
        # Off Hearth OS nothing is written.
        system(self.sys_root, False)
        shutil.rmtree(os.path.join(self.sys_root, 'usr'), ignore_errors=True)
        self.assertFalse(self.run_(p.diagnostics('x'))['ok'])

    def fake_opener(self, sent, reply=b'{"code":"ABCD-EFGH-JK"}'):
        import io

        class R(io.BytesIO):
            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False

        def open_(req, timeout=60, **kw):
            sent.append(req)
            return R(reply)
        return open_

    def test_auto_send_is_off_by_default_and_never_written(self):
        p = self.m.Plugin()
        self.assertFalse(self.run_(p.get_settings())['autoSend'])
        sent = []
        r = self.m.send_now('mirror', 'the mirror is used', 'diag', True, opener=self.fake_opener(sent))
        self.assertEqual(r['skipped'], 'automatic sending is off')
        self.assertEqual(sent, [])
        self.assertFalse(os.path.exists(self.sfile))

    def test_auto_send_once_per_reason_and_a_day_limit(self):
        self.m.SENT_THIS_START.clear()
        self.run_(self.m.Plugin().set_settings({'look': 'classic', 'autoSend': True}))
        sent = []
        op = self.fake_opener(sent)
        r = self.m.send_now('mirror', 'the mirror is used', 'Menu: found', True, opener=op, now=1000)
        self.assertTrue(r['ok'])
        self.assertEqual(r['code'], 'ABCD-EFGH-JK')
        self.assertTrue(self.m.send_now('mirror', 'again', 'x', True, opener=op, now=1001)['skipped'].startswith('already'))
        for i in range(20):
            self.m.send_now('reason-%d' % i, 'r', 'x', True, opener=op, now=1002)
        self.assertEqual(len(sent), self.m.AUTO_PER_DAY)
        self.assertTrue(self.m.send_now('one-more', 'r', 'x', True, opener=op, now=1003)['skipped'].startswith('the limit'))
        # By hand still works (its own limit), and the last send is remembered for the panel.
        self.assertTrue(self.m.send_now('manual', 'by hand', 'x', False, opener=op, now=1004)['ok'])
        last = self.run_(self.m.Plugin().last_sent())
        self.assertEqual((last['code'], last['auto']), ('ABCD-EFGH-JK', False))
        # Hearth's Send logs service: its address, its headers, a body it takes.
        req = sent[0]
        self.assertEqual(req.full_url, 'https://hearth-counter.game305-cl.workers.dev/v1/logs')
        self.assertEqual(req.get_header('X-hearth-kind'), 'peppermint')
        self.assertTrue(req.data.decode().startswith('Hearth logs'))
        self.assertIn('the%20mirror is used'.replace('%20', ' '), urllib_unquote(req.get_header('X-hearth-note')))

    def test_ca_bundle_selection(self):
        import ssl
        real = next((f for f in self.m.CA_FILES if os.path.isfile(f)), None)
        if real:
            ctx, where = self.m.ssl_context()
            self.assertEqual(where, real)   # Debian's bundle first, then Fedora's
            self.assertEqual(ctx.verify_mode, ssl.CERT_REQUIRED)
            self.assertTrue(ctx.check_hostname)
            # A missing or broken first one: the next one.
            broken = os.path.join(self.tmp, 'broken.crt')
            with open(broken, 'w') as f:
                f.write('not a certificate')
            self.assertEqual(self.m.ssl_context((os.path.join(self.tmp, 'missing.crt'), broken, real))[1], real)
        ctx, where = self.m.ssl_context(())
        self.assertTrue(where.startswith('Python default'))
        self.assertEqual(ctx.verify_mode, ssl.CERT_REQUIRED)

    def curl_run(self, calls, stdout='{"code":"CURL-CODE-01"}\n200', rc=0, stderr=''):
        def run(args, **kw):
            calls.append((args, kw))
            body_file = [a for a in args if a.startswith('@')][0][1:]
            with open(body_file) as f:
                calls.append(('body', f.read()))
            return subprocess.CompletedProcess(args, rc, stdout=stdout, stderr=stderr)
        return run

    def test_falls_back_to_curl_when_the_direct_send_fails(self):
        import ssl
        import urllib.error
        import io

        def ssl_fail(req, timeout=60, **kw):
            raise urllib.error.URLError(ssl.SSLCertVerificationError(1, '[SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed: unable to get local issuer certificate'))

        def http_fail(req, timeout=60, **kw):
            raise urllib.error.HTTPError(req.full_url, 400, 'Bad Request', {}, io.BytesIO(b'{"error":"not a Hearth log"}'))
        for opener, want in ((ssl_fail, 'certificate verify failed'), (http_fail, 'HTTP 400: not a Hearth log')):
            calls = []
            r = self.m.deliver('Hearth logs x', 'Hearth Classic: test', '1.2.5', opener, self.curl_run(calls), lambda n: '/usr/bin/curl', ctx=ssl.create_default_context())
            self.assertEqual((r['ok'], r['code'], r['via']), (True, 'CURL-CODE-01', 'curl'))
            args, kw = calls[0]
            self.assertEqual(args[0], '/usr/bin/curl')
            self.assertIn('https://hearth-counter.game305-cl.workers.dev/v1/logs', args)
            self.assertIn('X-Hearth-Kind: peppermint', args)
            self.assertFalse(any(a.startswith('User-Agent:') for a in args))   # curl's own, as Hearth's sender
            self.assertNotIn('LD_LIBRARY_PATH', kw['env'])   # Decky's bundled libraries never reach curl
            self.assertEqual(calls[1], ('body', 'Hearth logs x'))
            with open(self.m.diag_log_path()) as f:
                self.assertIn(want, f.read())
        # Both ways fail: the panel gets both reasons.
        calls = []
        r = self.m.deliver('Hearth logs x', 'n', '1.2.5', ssl_fail, self.curl_run(calls, stdout='{"error":"busy, try again later"}\n429'), lambda n: '/usr/bin/curl', ctx=ssl.create_default_context())
        self.assertFalse(r['ok'])
        self.assertIn('direct: [SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed', r['error'])
        self.assertIn('through curl: HTTP 429: busy, try again later', r['error'])
        r = self.m.deliver('Hearth logs x', 'n', '1.2.5', ssl_fail, self.curl_run([], rc=6, stderr='curl: (6) Could not resolve host'), lambda n: '/usr/bin/curl', ctx=ssl.create_default_context())
        self.assertIn('through curl: curl: (6) Could not resolve host', r['error'])
        r = self.m.deliver('Hearth logs x', 'n', '1.2.5', ssl_fail, self.curl_run([]), lambda n: None, ctx=ssl.create_default_context())
        self.assertIn("curl isn't installed", r['error'])

    def test_the_direct_send_uses_the_ca_context(self):
        sent, seen = [], {}

        def op(req, timeout=60, **kw):
            seen.update(kw)
            return self.fake_opener(sent)(req, timeout)
        import ssl
        ctx = ssl.create_default_context()
        r = self.m.deliver('Hearth logs x', 'n', '1.2.5', op, None, None, ctx=ctx)
        self.assertEqual((r['ok'], r['via']), (True, 'direct'))
        self.assertIs(seen['context'], ctx)
        self.assertTrue(sent[0].get_header('User-agent').startswith('hearth-classic/1.2.5'))

    def test_what_is_sent_has_no_personal_details(self):
        h = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(h, '.local', 'share', 'Steam', 'config'), exist_ok=True)
        with open(os.path.join(h, '.local', 'share', 'Steam', 'config', 'loginusers.vdf'), 'w') as f:
            f.write('"users" { "7656119800000001" { "AccountName" "dragonacct" "PersonaName" "Dragon Slayer" } }')
        t = self.m.redact('at %s/x by Dragon Slayer (dragonacct) id 76561198012345678 https://store.example.com/app?id=1&t=2 '
                          'token=abc123 mail a.b@c.de' % h, h)
        for bad in (h, 'Dragon Slayer', 'dragonacct', '76561198012345678', 'id=1', 'abc123', 'a.b@c.de'):
            self.assertNotIn(bad, t)
        self.assertIn('https://store.example.com/app?[query left out]', t)
        b = self.m.send_body('test', 'Menu: found', '1.2.4', home_dir=h, names={'Dragon Slayer'}, now=0)
        self.assertTrue(b.startswith('Hearth logs (Hearth Classic diagnostics only)'))
        self.assertIn('Reason: test', b)

    def test_no_other_trade_dress(self):
        import re
        css = re.sub(r'base64,[a-z0-9+/=]+', '', (PLUGIN / 'defaults/hearth-classic.css').read_text().lower())
        for word in ('xbox', 'playstation', 'nintendo', 'switch', 'dashboard'):
            self.assertNotIn(word, css)


if __name__ == '__main__':
    unittest.main()
