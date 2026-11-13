"""Hearth (C:) with Windows folder names in Hearth Files (docs/HEARTH-FILES.md "Hearth (C:) the Windows way"): the
path mapping both ways (drive-paths.js with node, and hearth_drive_letters.py's copy of it), the view's pages built
from Installed apps' list, writes into the view's own folders refused (and real folders reached through it working
as always), D:, E: and the old C: forms unchanged with the view off, Windows apps' C: untouched, and the settings never
written by opening Files or Settings (only the key you change, merged). Each test has its own throwaway home."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import io
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
import hearth_drive_letters as hdl  # noqa: E402

NODE = shutil.which('node')
HOME = '/var/home/chris'
APPS = [
    {'id': 'flatpak:org.mozilla.firefox', 'appId': 'org.mozilla.firefox', 'name': 'Firefox', 'kind': 'flatpak', 'size': 312000000,
     'path': '/var/lib/flatpak/app/org.mozilla.firefox'},
    {'id': 'flatpak:com.discordapp.Discord', 'appId': 'com.discordapp.Discord', 'name': 'Discord', 'kind': 'flatpak', 'size': 410000000,
     'path': '/var/lib/flatpak/app/com.discordapp.Discord'},
    {'id': 'hearth:hearth-paint.desktop', 'name': 'Hearth Paint', 'kind': 'hearth', 'part': True},
    {'id': 'recomp:soh', 'name': 'Ship of Harkinian', 'kind': 'recomp', 'size': 140000000, 'path': HOME + '/Games/Recomps/Ship of Harkinian'},
    {'id': 'gog:1', 'name': 'Witcher', 'kind': 'games', 'publisher': 'GOG', 'size': 50, 'path': HOME + '/Games/GOG/Witcher'},
    {'id': 'epic:1', 'name': 'Alan Wake', 'kind': 'games', 'publisher': 'Epic', 'size': 20, 'path': HOME + '/Games/Epic/Alan Wake'},
    {'id': 'winapp:7zip', 'appId': '7zip', 'name': '7-Zip', 'kind': 'winapp', 'size': 6000000, 'dir': HOME + '/Games/Windows apps/7-Zip'},
    {'id': 'steam:440', 'name': 'TF2', 'kind': 'steam', 'size': 1000, 'drive': '/'},
]
DATA = {'apps': APPS, 'hearth': '/usr/share/nimbus', 'steam': HOME + '/.local/share/Steam', 'linux': ['usr', 'etc', 'var', 'opt'],
        'flatpak': ['/var/lib/flatpak/app'], 'showLinux': False}

JS = r'''
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'utf8').replace(/^\.pragma library\s*$/m, '');
const m = {};
new Function('exports', src + '\nfor (const k of ["crumbsFor","toWindows","fromWindows","cPage","cResolve","cEntries","cExtras","cRefusal","cAppList","isCPath"]) exports[k] = eval(k);')(m);
const job = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
const data = job.data;
const base = { home: job.home, user: 'chris',
  drives: [{ letter: 'C', path: '/', label: 'Hearth (C:)' }, { letter: 'D', path: '/run/media/chris/Games', label: 'Games (D:)' },
           { letter: 'E', path: '/run/media/chris/USB', label: 'USB stick (E:)' }],
  folders: [{ label: 'Documents', path: job.home + '/Documents' }] };
const ctx = on => Object.assign({}, base, { c: { on: on, users: '/var/home', apps: m.cAppList(data, null).map(a => ({ name: a.name, path: a.path, x86: a.x86, noMap: !!a.noMap })) } });
const out = { on: {}, off: {}, back: {}, backOff: {}, crumbs: {}, pages: {}, extras: {}, refuse: {}, page: {} };
for (const p of job.paths) { out.on[p] = m.toWindows(p, ctx(true)); out.off[p] = m.toWindows(p, ctx(false));
  const c = m.crumbsFor(p, ctx(true), 'hearth:thispc'); out.crumbs[p] = c ? c.map(x => x.label + '@' + x.path) : null; out.page[p] = m.cPage(p, ctx(true)); }
for (const t of job.typed) { out.back[t] = m.fromWindows(t, ctx(true)); out.backOff[t] = m.fromWindows(t, ctx(false)); }
for (const pg of ['root', 'pf', 'x86', 'appdata']) out.pages[pg] = m.cEntries(pg, ctx(true), data);
out.pages.rootLinux = m.cEntries('root', ctx(true), Object.assign({}, data, { showLinux: true }));
out.extras.home = m.cExtras(job.home, ctx(true), ['Documents']);
out.extras.local = m.cExtras(job.home + '/.local/share', ctx(true), []);
out.extras.off = m.cExtras(job.home, ctx(false), []);
out.extras.other = m.cExtras(job.home + '/Documents', ctx(true), []);
for (const p of job.refuse) out.refuse[p] = m.cRefusal(p, ctx(true));
out.temp = m.cResolve('hearth:c/Users/chris/AppData/Local/Temp', ctx(true));
console.log(JSON.stringify(out));
'''

PATHS = ['/usr/share/fonts', '/var/lib/flatpak/app/org.mozilla.firefox/current', HOME + '/.config/hearth', HOME + '/.local/share/x',
         HOME + '/.cache/thumbs', HOME + '/Documents/a b.txt', HOME + '/Games', '/var/home', '/var/home/alex', '/etc', '/',
         'hearth:c', 'hearth:c/Program Files', 'hearth:c/Program Files (x86)', 'hearth:c/Users/chris/AppData',
         HOME + '/Games/Windows apps/7-Zip/prefix', HOME + '/.local/share/Steam/steamapps', '/run/media/chris/Games/SteamLibrary',
         '/run/media/chris/USB/photo.jpg', '/var/lib/systemd']
TYPED = ['C:\\', 'c:/', 'C:\\Program Files', 'C:\\Program Files\\Firefox', 'c:\\program files\\firefox\\current\\x',
         'C:\\Program Files\\Hearth', 'C:\\Program Files\\Hearth Games', 'C:\\Program Files (x86)\\Steam\\steamapps',
         'C:\\Program Files (x86)\\7-Zip', 'C:\\Program Files\\Nope', 'C:\\Users', 'C:\\Users\\chris', 'C:\\Users\\Chris\\Documents',
         'C:\\Users\\chris\\AppData', 'C:\\Users\\chris\\AppData\\Roaming', 'C:\\Users\\chris\\AppData\\Roaming\\hearth',
         'C:\\Users\\chris\\AppData\\Local', 'C:\\Users\\chris\\AppData\\Local\\Temp', 'C:\\Users\\chris\\AppData\\Local\\Temp\\x',
         'C:\\Users\\alex\\Music', 'C:\\Windows', 'C:\\Windows\\share', 'C:\\ProgramData\\flatpak', 'C:\\etc\\hosts',
         'D:\\SteamLibrary', 'E:', 'X:\\nope', '/etc', 'Documents']
REFUSE = ['hearth:c', 'hearth:c/Program Files', 'hearth:c/Program Files (x86)', 'hearth:c/Users/chris/AppData', '/usr', '/usr/share',
          '/var/lib', '/var/lib/flatpak', HOME + '/Documents', HOME + '/.config', HOME + '/Games/Recomps/Ship of Harkinian',
          '/run/media/chris/Games', '/var/home', '/tmp']


@unittest.skipUnless(NODE, 'node is needed to run drive-paths.js')
class Mapping(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        d = tempfile.mkdtemp(prefix='hearth-cdrive-js-')
        with open(os.path.join(d, 'run.js'), 'w') as f:
            f.write(JS)
        with open(os.path.join(d, 'job.json'), 'w') as f:
            json.dump({'home': HOME, 'paths': PATHS, 'typed': TYPED, 'refuse': REFUSE, 'data': DATA}, f)
        r = subprocess.run([NODE, os.path.join(d, 'run.js'), str(SHELL / 'drive-paths.js'), os.path.join(d, 'job.json')],
                           capture_output=True, text=True, timeout=60)
        shutil.rmtree(d, ignore_errors=True)
        if r.returncode != 0:
            raise AssertionError(r.stderr)
        cls.r = json.loads(r.stdout)

    def test_real_paths_to_c_forms(self):
        on = self.r['on']
        self.assertEqual(on['/usr/share/fonts'], 'C:\\Windows\\share\\fonts')
        self.assertEqual(on['/var/lib/flatpak/app/org.mozilla.firefox/current'], 'C:\\Program Files\\Firefox\\current')
        self.assertEqual(on[HOME + '/.config/hearth'], 'C:\\Users\\chris\\AppData\\Roaming\\hearth')
        self.assertEqual(on[HOME + '/.local/share/x'], 'C:\\Users\\chris\\AppData\\Local\\x')
        self.assertEqual(on[HOME + '/.cache/thumbs'], 'C:\\Users\\chris\\AppData\\Local\\Temp\\thumbs')
        self.assertEqual(on[HOME + '/Documents/a b.txt'], 'C:\\Users\\chris\\Documents\\a b.txt')
        self.assertEqual(on['/var/home'], 'C:\\Users')
        self.assertEqual(on['/var/home/alex'], 'C:\\Users\\alex')
        self.assertEqual(on['/etc'], 'C:\\etc')
        self.assertEqual(on['/'], 'C:\\')
        self.assertEqual(on['/var/lib/systemd'], 'C:\\ProgramData\\systemd')
        self.assertEqual(on[HOME + '/Games/Windows apps/7-Zip/prefix'], 'C:\\Program Files (x86)\\7-Zip\\prefix')
        self.assertEqual(on[HOME + '/.local/share/Steam/steamapps'], 'C:\\Program Files (x86)\\Steam\\steamapps')
        self.assertEqual(on['hearth:c'], 'C:\\')
        self.assertEqual(on['hearth:c/Program Files (x86)'], 'C:\\Program Files (x86)')
        self.assertEqual(on['hearth:c/Users/chris/AppData'], 'C:\\Users\\chris\\AppData')

    def test_c_forms_to_real_paths(self):
        b = self.r['back']
        want = {'C:\\': 'hearth:c', 'c:/': 'hearth:c', 'C:\\Program Files': 'hearth:c/Program Files',
                'C:\\Program Files\\Firefox': '/var/lib/flatpak/app/org.mozilla.firefox',
                'c:\\program files\\firefox\\current\\x': '/var/lib/flatpak/app/org.mozilla.firefox/current/x',
                'C:\\Program Files\\Hearth': '/usr/share/nimbus', 'C:\\Program Files\\Hearth Games': HOME + '/Games',
                'C:\\Program Files (x86)\\Steam\\steamapps': HOME + '/.local/share/Steam/steamapps',
                'C:\\Program Files (x86)\\7-Zip': HOME + '/Games/Windows apps/7-Zip',
                'C:\\Users': '/var/home', 'C:\\Users\\chris': HOME, 'C:\\Users\\Chris\\Documents': HOME + '/Documents',
                'C:\\Users\\chris\\AppData': 'hearth:c/Users/chris/AppData', 'C:\\Users\\chris\\AppData\\Roaming': HOME + '/.config',
                'C:\\Users\\chris\\AppData\\Roaming\\hearth': HOME + '/.config/hearth',
                'C:\\Users\\chris\\AppData\\Local': HOME + '/.local/share', 'C:\\Users\\chris\\AppData\\Local\\Temp': HOME + '/.cache',
                'C:\\Users\\chris\\AppData\\Local\\Temp\\x': HOME + '/.cache/x', 'C:\\Users\\alex\\Music': '/var/home/alex/Music',
                'C:\\Windows': '/usr', 'C:\\Windows\\share': '/usr/share', 'C:\\ProgramData\\flatpak': '/var/lib/flatpak',
                'C:\\etc\\hosts': '/etc/hosts', 'D:\\SteamLibrary': '/run/media/chris/Games/SteamLibrary', 'E:': '/run/media/chris/USB'}
        for t, p in want.items():
            self.assertEqual(b[t], {'path': p}, t)
        self.assertIn('error', b['C:\\Program Files\\Nope'])
        self.assertIn('error', b['X:\\nope'])
        self.assertIsNone(b['/etc'])
        self.assertIsNone(b['Documents'])

    def test_round_trip(self):
        for p in PATHS:
            if p.startswith('/') and p != '/':
                w = self.r['on'][p]
                self.assertIn(w, TYPED + [w])
        # Every real path's C:\ form leads back to it.
        job_back = {}
        for p in ('/usr/share/fonts', HOME + '/.cache/thumbs', HOME + '/.config/hearth', '/var/home/alex', '/etc'):
            job_back[p] = self.r['on'][p]
        typed = set(TYPED)
        for p, w in job_back.items():
            if w in typed:
                self.assertEqual(self.r['back'][w]['path'], p)

    def test_off_is_exactly_as_before(self):
        off, b = self.r['off'], self.r['backOff']
        self.assertEqual(off['/usr/share/fonts'], 'C:\\usr\\share\\fonts')
        self.assertEqual(off[HOME + '/.config/hearth'], 'C:\\Users\\chris\\.config\\hearth')
        self.assertEqual(off['/var/lib/flatpak/app/org.mozilla.firefox/current'], 'C:\\var\\lib\\flatpak\\app\\org.mozilla.firefox\\current')
        self.assertEqual(off['/run/media/chris/Games/SteamLibrary'], 'D:\\SteamLibrary')
        self.assertEqual(b['C:\\'], {'path': '/'})
        self.assertEqual(b['C:\\Windows'], {'path': '/Windows'})
        self.assertEqual(b['C:\\Users\\chris'], {'path': HOME})
        self.assertEqual(b['D:\\SteamLibrary'], {'path': '/run/media/chris/Games/SteamLibrary'})
        self.assertEqual(b['E:'], {'path': '/run/media/chris/USB'})
        # D: and E: are the same with the view on or off.
        for t in ('D:\\SteamLibrary', 'E:', 'X:\\nope'):
            self.assertEqual(self.r['back'][t], b[t])
        self.assertEqual(self.r['on']['/run/media/chris/USB/photo.jpg'], 'E:\\photo.jpg')

    def test_crumbs(self):
        c = self.r['crumbs']
        self.assertEqual(c['/var/lib/flatpak/app/org.mozilla.firefox/current'],
                         ['This PC@hearth:thispc', 'Hearth (C:)@hearth:c', 'Program Files@hearth:c/Program Files',
                          'Firefox@/var/lib/flatpak/app/org.mozilla.firefox', 'current@/var/lib/flatpak/app/org.mozilla.firefox/current'])
        self.assertEqual(c[HOME + '/.config/hearth'][2:],
                         ['Users@/var/home', 'chris@' + HOME, 'AppData@hearth:c/Users/chris/AppData', 'Roaming@' + HOME + '/.config',
                          'hearth@' + HOME + '/.config/hearth'])
        self.assertEqual(c['/usr/share/fonts'][2:], ['Windows@/usr', 'share@/usr/share', 'fonts@/usr/share/fonts'])
        self.assertEqual(c['hearth:c'], ['This PC@hearth:thispc', 'Hearth (C:)@hearth:c'])
        self.assertIsNone(c[HOME + '/Games'])                                  # the rest of home: Files' own steps
        self.assertEqual(self.r['on'][HOME + '/Games'], 'C:\\Users\\chris\\Games')   # (not Hearth Games: that's two folders)
        self.assertEqual(c[HOME + '/Documents/a b.txt'][1], 'Documents@' + HOME + '/Documents')
        self.assertEqual(c['/run/media/chris/Games/SteamLibrary'][1], 'Games (D:)@/run/media/chris/Games')
        self.assertEqual(self.r['page']['hearth:c'], 'root')
        self.assertEqual(self.r['page']['hearth:c/Program Files'], 'pf')
        self.assertEqual(self.r['page']['hearth:c/Program Files (x86)'], 'x86')
        self.assertEqual(self.r['page']['hearth:c/Users/chris/AppData'], 'appdata')
        self.assertEqual(self.r['page']['/usr/share/fonts'], '')
        self.assertEqual(self.r['temp'], {'path': HOME + '/.cache'})

    def test_pages(self):
        pg = self.r['pages']
        root = [(e['name'], e['path']) for e in pg['root']]
        self.assertEqual(root, [('Program Files', 'hearth:c/Program Files'), ('Program Files (x86)', 'hearth:c/Program Files (x86)'),
                                ('ProgramData', '/var/lib'), ('Users', '/var/home'), ('Windows', '/usr')])
        by = {e['name']: e['cv'] for e in pg['root']}
        self.assertTrue(by['Windows']['lock'] and by['Windows']['look'])
        self.assertTrue(by['ProgramData']['look'])
        self.assertFalse(by['Users'].get('look'))
        self.assertTrue(all(e['dir'] for e in pg['root']))
        # The Linux folders, off by default, under their own heading when switched on.
        lin = pg['rootLinux'][5:]
        self.assertEqual([e['name'] for e in lin], ['etc', 'opt', 'usr', 'var'])
        self.assertEqual(lin[0]['cv']['group'], 'THE LINUX FOLDERS')
        self.assertTrue(all(e['cv']['dim'] for e in lin))
        # Program Files: Hearth (look only) first, every Flatpak, Hearth Games once, each Recomp. No Steam game or Windows app.
        pf = [e['name'] for e in pg['pf']]
        self.assertEqual(pf, ['Hearth', 'Discord', 'Firefox', 'Hearth Games', 'Ship of Harkinian'])
        pfb = {e['name']: e for e in pg['pf']}
        self.assertEqual(pfb['Firefox']['path'], '/var/lib/flatpak/app/org.mozilla.firefox')
        self.assertEqual(pfb['Firefox']['cv']['win'], 'C:\\Program Files\\Firefox')
        self.assertEqual(pfb['Firefox']['cv']['app']['appId'], 'org.mozilla.firefox')   # Modify / Uninstall reuse Installed apps
        self.assertTrue(pfb['Hearth']['cv']['lock'])
        self.assertEqual(pfb['Hearth Games']['path'], HOME + '/Games')
        self.assertEqual(pfb['Hearth Games']['cv']['type'], 'Folder · GOG and Epic')
        self.assertEqual(pfb['Hearth Games']['cv']['size'], 70)
        # Program Files (x86): Windows apps and Steam.
        x86 = {e['name']: e for e in pg['x86']}
        self.assertEqual(sorted(x86), ['7-Zip', 'Steam'])
        self.assertEqual(x86['Steam']['path'], HOME + '/.local/share/Steam')
        self.assertEqual(x86['Steam']['cv']['size'], 1000)
        self.assertEqual(x86['7-Zip']['cv']['type'], 'Windows app')
        # AppData: Local and Roaming, the real folders.
        self.assertEqual([(e['name'], e['path']) for e in pg['appdata']], [('Local', HOME + '/.local/share'), ('Roaming', HOME + '/.config')])

    def test_extras(self):
        x = self.r['extras']
        self.assertEqual([(e['name'], e['path']) for e in x['home']], [('AppData', 'hearth:c/Users/chris/AppData')])
        self.assertEqual([(e['name'], e['path']) for e in x['local']], [('Temp', 'hearth:c/Users/chris/AppData/Local/Temp')])
        self.assertEqual(x['off'], [])
        self.assertEqual(x['other'], [])

    def test_writes_into_the_view_refused(self):
        f = self.r['refuse']
        for p in ('hearth:c', 'hearth:c/Program Files', 'hearth:c/Program Files (x86)', 'hearth:c/Users/chris/AppData', '/usr', '/usr/share',
                  '/var/lib', '/var/lib/flatpak'):
            self.assertTrue(f[p], p)
        self.assertIn('install it', f['hearth:c/Program Files'])
        self.assertIn('Program Files (x86)', f['hearth:c/Program Files (x86)'])
        self.assertIn('look only', f['/usr'])
        # Real folders reached through the view work as always.
        for p in (HOME + '/Documents', HOME + '/.config', HOME + '/Games/Recomps/Ship of Harkinian', '/run/media/chris/Games', '/var/home', '/tmp'):
            self.assertEqual(f[p], '', p)


class PythonMapping(unittest.TestCase):
    """hearth_drive_letters.py's copy of the mapping (hearth-files' backstop) says the same as drive-paths.js."""
    def setUp(self):
        apps = [{'name': 'Firefox', 'path': '/var/lib/flatpak/app/org.mozilla.firefox'},
                {'name': '7-Zip', 'path': HOME + '/Games/Windows apps/7-Zip', 'x86': True}]
        self.ctx = hdl.c_context(apps, HOME, 'chris')

    def test_both_ways(self):
        f = lambda t: hdl.from_c(t, self.ctx)
        self.assertEqual(f('C:\\'), {'path': 'hearth:c'})
        self.assertEqual(f('C:\\Program Files\\firefox\\x'), {'path': '/var/lib/flatpak/app/org.mozilla.firefox/x'})
        self.assertEqual(f('C:\\Program Files (x86)\\7-Zip'), {'path': HOME + '/Games/Windows apps/7-Zip'})
        self.assertIn('error', f('C:\\Program Files\\7-Zip'))                     # it's in (x86)
        self.assertEqual(f('C:\\Users\\chris\\AppData\\Local\\Temp\\a'), {'path': HOME + '/.cache/a'})
        self.assertEqual(f('C:\\Users\\chris\\AppData\\Roaming'), {'path': HOME + '/.config'})
        self.assertEqual(f('C:\\Users\\chris\\AppData'), {'path': 'hearth:c/Users/chris/AppData'})
        self.assertEqual(f('C:/Windows/../ProgramData'), {'path': '/var/lib'})
        self.assertIsNone(f('D:\\x'))
        t = lambda p: hdl.to_c(p, self.ctx)
        self.assertEqual(t('/usr/bin'), 'C:\\Windows\\bin')
        self.assertEqual(t(HOME + '/.local/share/x'), 'C:\\Users\\chris\\AppData\\Local\\x')
        self.assertEqual(t('/var/lib/flatpak/app/org.mozilla.firefox'), 'C:\\Program Files\\Firefox')
        self.assertEqual(t('/var/home'), 'C:\\Users')
        for p in ('/usr/bin', HOME + '/.cache/a', HOME + '/Documents', '/var/lib/x', '/var/lib/flatpak/app/org.mozilla.firefox/y'):
            self.assertEqual(hdl.from_c(t(p), self.ctx), {'path': p}, p)
        self.assertEqual(hdl.c_page('hearth:c/Program Files', self.ctx), 'pf')
        self.assertEqual(hdl.c_page('hearth:c/Users/chris/AppData/Local/Temp', self.ctx), '')

    def test_refusals(self):
        self.assertIn('install it', hdl.view_refusal('hearth:c/Program Files', self.ctx))
        self.assertIn('Hearth (C:)', hdl.view_refusal('hearth:c', self.ctx))
        self.assertIn('look only', hdl.view_refusal('/usr/share'))
        self.assertIn('ProgramData', hdl.view_refusal('/var/lib/flatpak'))
        self.assertEqual(hdl.view_refusal(HOME + '/Documents'), '')

    def test_wine_prefix_keeps_its_own_c(self):
        """Windows apps are unchanged: map_prefix never touches c: (the prefix's own drive) or z:."""
        home = tempfile.mkdtemp(prefix='hearth-cdrive-wine-')
        old = dict(os.environ)
        try:
            os.environ['HOME'] = home
            os.environ.pop('XDG_CONFIG_HOME', None)
            p = os.path.join(home, 'prefix')
            os.makedirs(os.path.join(p, 'dosdevices'))
            os.makedirs(os.path.join(p, 'drive_c', 'users'))
            os.symlink('../drive_c', os.path.join(p, 'dosdevices', 'c:'))
            os.symlink('/', os.path.join(p, 'dosdevices', 'z:'))
            hdl.map_prefix(p)
            self.assertEqual(os.readlink(os.path.join(p, 'dosdevices', 'c:')), '../drive_c')
            self.assertEqual(os.readlink(os.path.join(p, 'dosdevices', 'z:')), '/')
        finally:
            os.environ.clear()
            os.environ.update(old)
            shutil.rmtree(home, ignore_errors=True)
        src = (SHELL / 'hearth_drive_letters.py').read_text()
        body = src[src.index('def map_prefix'):src.index('# ---------------------------------------------------------------- Hearth (C:)')]
        self.assertNotIn('from_c', body)
        self.assertNotIn('C_ROOT', body)


def load_files(home):
    os.environ['HOME'] = home
    for k in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_DIRS', 'XDG_CONFIG_DIRS', 'XDG_CACHE_HOME'):
        os.environ.pop(k, None)
    os.environ['XDG_STATE_HOME'] = os.path.join(home, '.local', 'state')
    loader = importlib.machinery.SourceFileLoader('hearth_files_cdrive', str(SHELL / 'hearth-files'))
    spec = importlib.util.spec_from_loader('hearth_files_cdrive', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Helper(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.home = tempfile.mkdtemp(prefix='hearth-cdrive-')
        self.hf = load_files(self.home)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.home, ignore_errors=True)

    def files(self):
        out = {}
        for root, dirs, names in os.walk(self.home):
            for n in names:
                p = os.path.join(root, n)
                with open(p, 'rb') as f:
                    out[os.path.relpath(p, self.home)] = f.read()
        return out

    def test_copy_into_view_refused(self):
        src = os.path.join(self.home, 'a.txt')
        with open(src, 'w') as f:
            f.write('x')
        for dest in ('hearth:c/Program Files', 'hearth:c/Program Files (x86)', 'hearth:c', 'hearth:c/Users/x/AppData'):
            with self.assertRaises(self.hf.Fail) as cm, contextlib.redirect_stdout(io.StringIO()):
                self.hf.transfer([src], dest, False)
            self.assertNotIn('only changes things in your home', str(cm.exception), dest)
            with self.assertRaises(self.hf.Fail):
                self.hf.mkdir(dest + '/new')
        with self.assertRaises(self.hf.Fail) as cm:
            self.hf.check_writable('/usr/share/x')
        self.assertIn('look only', str(cm.exception))
        with self.assertRaises(self.hf.Fail):
            self.hf.trash(['hearth:c/Users/x/AppData'])
        # A real folder reached through the view (Users\you\Documents) works as always.
        docs = os.path.join(self.home, 'Documents')
        os.makedirs(docs)
        with contextlib.redirect_stdout(io.StringIO()):
            r = self.hf.transfer([src], docs, False)
        self.assertTrue(os.path.isfile(os.path.join(docs, 'a.txt')), r)

    def test_installed_apps_have_real_folders(self):
        src = (SHELL / 'hearth-files').read_text()
        fl = src[src.index("out.append({'id': 'flatpak:' + aid"):][:400]
        self.assertIn("'path': os.path.join(inst, 'app', aid)", fl)
        gm = src[src.index("out.append({'id': gid, 'name': g['title']"):][:300]
        self.assertIn("'path': g.get('path')", gm)

    def test_opening_writes_nothing(self):
        """Opening Files (prefs, cdrive) and Settings › Drives & folders (prefs, this-pc) writes no setting."""
        cfg = os.path.join(self.home, '.config', 'nimbus')
        before = self.files()
        self.assertEqual(self.hf.prefs_get(), {})
        r = self.hf.cdrive()
        self.assertEqual(r['home'], self.home)
        self.assertIn('linux', r)
        self.assertEqual(self.files(), before)
        self.assertFalse(os.path.exists(os.path.join(cfg, 'files.json')))
        # With a file there: read, never rewritten.
        os.makedirs(cfg, exist_ok=True)
        with open(os.path.join(cfg, 'files.json'), 'w') as f:
            json.dump({'view': 'details', 'startPage': 'thispc'}, f)
        before = self.files()
        self.hf.prefs_get()
        self.hf.cdrive()
        self.assertEqual(self.files(), before)

    def test_only_the_changed_key_is_merged(self):
        cfg = os.path.join(self.home, '.config', 'nimbus')
        os.makedirs(cfg)
        p = os.path.join(cfg, 'files.json')
        with open(p, 'w') as f:
            json.dump({'view': 'details', 'showDevices': False}, f)
        self.hf.prefs_set(json.dumps({'linuxFolders': True}))
        with open(p) as f:
            got = json.load(f)
        self.assertEqual(got, {'view': 'details', 'showDevices': False, 'linuxFolders': True})
        self.assertTrue(os.path.exists(p + '.bak'))
        self.assertNotIn('cdrive', got)                                       # defaults are never written

    def test_damaged_file_not_replaced_with_defaults(self):
        cfg = os.path.join(self.home, '.config', 'nimbus')
        os.makedirs(cfg)
        p = os.path.join(cfg, 'files.json')
        with open(p, 'w') as f:
            json.dump({'view': 'tiles', 'copyPath': 'linux'}, f)
        self.hf.prefs_set(json.dumps({'addressBar': 'linux'}))              # makes the .bak
        with open(p, 'w') as f:
            f.write('{"view": "til')                                         # cut short
        self.hf.prefs_set(json.dumps({'linuxFolders': True}))
        with open(p) as f:
            got = json.load(f)
        self.assertEqual(got.get('copyPath'), 'linux')                       # put back from .bak, then merged
        self.assertEqual(got.get('linuxFolders'), True)


class Qml(unittest.TestCase):
    """The window and Settings: new pages behind Loaders, and the switches written only when changed."""
    def setUp(self):
        self.q = (SHELL / 'FileManager.qml').read_text()

    def fn(self, name):
        i = self.q.index('function ' + name + '(')
        j = self.q.find('\n    }\n', i)
        return self.q[i:j]

    def test_loaders(self):
        self.assertIn('setSource("FilesCDrive.qml"', self.q)
        self.assertIn('setSource("FilesCProps.qml"', self.q)
        self.assertIsNone(re.search(r'^\s*FilesCDrive\s*\{', self.q, re.M), 'FilesCDrive.qml must load through a Loader')
        self.assertIsNone(re.search(r'^\s*FilesCProps\s*\{', self.q, re.M))
        for f in ('FilesCDrive.qml', 'FilesCProps.qml', 'SetDrives.qml'):
            self.assertTrue((SHELL / f).exists(), f)

    def test_open_writes_nothing(self):
        for name in ('open', 'loadPrefs', 'refreshCPrefs', 'loadCPage', 'loadCData', 'showCProps', 'openCMenu'):
            self.assertNotIn('prefs-set', self.fn(name), name)
        # setFilesPref is the one writer of the view's keys: one key, and only when it really changes.
        sf = self.fn('setFilesPref')
        self.assertIn('if (now === v) return;', sf)
        self.assertIn('call(["prefs-set", JSON.stringify(o)]', sf)
        for k in ('cdrive', 'linuxFolders', 'addressBar', 'copyPath'):
            for m in re.finditer(r'prefs-set[^\n]*' + k, self.q):
                self.fail('files.json %s written outside setFilesPref: %s' % (k, m.group(0)))
        self.assertIn('readonly property bool cdriveOn: prefs.cdrive !== false', self.q)    # on by default, never saved
        self.assertIn('readonly property bool cLinux: prefs.linuxFolders === true', self.q)  # off by default

    def test_menu_and_view(self):
        for needle in ('"Go to the real folder"', '"Open in new tab"', '"Copy Linux path"', '"Show the Linux folders too"',
                       '"Show Hearth (C:) with Windows folder names"', '"Address bar: Windows names"', '"Address bar: Linux paths"',
                       '"Copy path: C:\\\\ form"', '"Copy path: Linux path"', 'appModify(e.cv.app)', 'appUninstall(e.cv.app)',
                       'cRefusalFor(dest)', 'function hearthc(sub: string): void'):
            self.assertIn(needle, self.q)

    def test_settings_page(self):
        s = (SHELL / 'SetDrives.qml').read_text()
        reg = (SHELL / 'settings-pages.js').read_text()
        self.assertIn('{ id: "drives", file: "SetDrives.qml", cat: "system", label: "Drives & folders"', reg)
        for needle in ('Show Hearth (C:) with Windows folder names', 'Show system folders', 'The address bar shows', 'Copy path gives',
                       '"Always C:"', 'value: "Change letter…"', '"letter-set"'):
            self.assertIn(needle, s)
        # Opening it reads only; save() writes one key, only from a switch or a choice, only when it changed.
        onc = s[s.index('Component.onCompleted'):].split('\n')[0]
        self.assertNotIn('save(', onc)
        self.assertNotIn('prefs-set', s[s.index('function load()'):s.index('function save(')])
        self.assertIn('if (!page.loaded || page[k] === v) return;', s)
        self.assertEqual(s.count('"prefs-set"'), 1)
        for m in re.finditer(r'page\.save\(', s):
            line = s[s.rfind('\n', 0, m.start()):s.find('\n', m.start())]
            self.assertRegex(line, r'onToggled|onChosen', line)


if __name__ == '__main__':
    unittest.main()
