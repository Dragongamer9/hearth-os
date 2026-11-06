"""Hearth's boot splash themes (Plymouth scripts) can't stop a device from starting: every theme parses, runs for a
while through tools/plymouth_preview.py (a strict stand-in for Plymouth's script plugin: a name never set, a missing
picture or a call to nothing is an error there) at several screen sizes, shows the disk password prompt, and the
themes added after 1.1 (hearth-aurora, hearth-starlight, hearth-orbit) use nothing the first three don't already use
on devices. Also: every list of splash names agrees (hearth-boot-look, hearth-admin, hearth-bls-grub, the image build
and its checks), the default stays hearth, and Settings › Boot & startup's previews are there for every choice."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import struct
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
PLY = REPO / 'os/system_files/usr/share/plymouth/themes'
LOOK = REPO / 'os/system_files/usr/libexec/nimbus/hearth-boot-look'
ADMIN = REPO / 'os/system_files/usr/libexec/nimbus/hearth-admin'
BLS = REPO / 'os-bases/peppermint/system_files/usr/libexec/nimbus/hearth-bls-grub'
BUILD = REPO / 'os-bases/peppermint/build.sh'
IMAGE_CHECKS = REPO / '.github/workflows/base-peppermint.yml'
MOVIES = REPO / 'os/system_files/usr/share/hearth/startup-movies'
PREVIEWS = REPO / 'os/system_files/usr/share/hearth/boot-previews'
ORIGINAL = ('hearth', 'hearth-steam', 'hearth-minimal')
NEW = ('hearth-aurora', 'hearth-starlight', 'hearth-orbit')

sys.path.insert(0, str(REPO / 'tools'))
import plymouth_preview as pp  # noqa: E402

try:
    import PIL  # noqa: F401
    HAVE_PIL = True
except ImportError:
    HAVE_PIL = False


def themes():
    return sorted(p.name for p in PLY.iterdir() if (p / f'{p.name}.script').is_file())


def load_look():
    loader = importlib.machinery.SourceFileLoader('hearth_boot_look_t', str(LOOK))
    spec = importlib.util.spec_from_loader('hearth_boot_look_t', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def tokens(theme):
    src = (PLY / theme / f'{theme}.script').read_text()
    return {(k, t) for k, t, _ in pp.tokenize(src) if k in ('op', 'id') and (k == 'op' or t in pp.KEYWORDS)}


class Syntax(unittest.TestCase):
    def test_every_theme_parses(self):
        self.assertEqual(set(themes()), set(ORIGINAL + NEW))
        for t in themes():
            self.assertEqual(pp.check(str(PLY / t / f'{t}.script')), [], t)

    def test_checker_catches_mistakes(self):
        bad = {
            'missing semicolon': 'a = 1\nb = 2;',
            'unclosed brace': 'fun f() { a = 1;',
            'unclosed string': 'a = "oops;',
            'stray else': 'else a = 1;',
            'assign to a call': 'f() = 2;',
            'stray character': 'a = 1 @ 2;',
        }
        for what, src in bad.items():
            with self.assertRaises(pp.ScriptError, msg=what):
                pp.parse(src)
        pp.parse('fun f(a, b) { if (a < b) return a; else return b; }\nx = f(1, 2); for (i = 0; i < 3; i++) y[i] = i * 2;')

    def test_plymouth_files(self):
        for t in themes():
            conf = (PLY / t / f'{t}.plymouth').read_text()
            self.assertIn('ModuleName=script', conf, t)
            self.assertIn(f'ImageDir=/usr/share/plymouth/themes/{t}\n', conf, t)
            self.assertIn(f'ScriptFile=/usr/share/plymouth/themes/{t}/{t}.script\n', conf, t)
            for img in re.findall(r'Image\("([^"]+)"\)', (PLY / t / f'{t}.script').read_text()):
                self.assertTrue((PLY / t / img).is_file() and (PLY / t / img).stat().st_size > 0, f'{t}: {img}')

    def test_new_themes_use_only_proven_language(self):
        proven = set().union(*(tokens(t) for t in ORIGINAL))
        for t in NEW:
            extra = tokens(t) - proven
            self.assertEqual(extra, set(), f'{t} uses operators or keywords the first three themes never used: {extra}')


@unittest.skipUnless(HAVE_PIL, 'needs Pillow (python3-pil)')
class Runs(unittest.TestCase):
    SIZES = ((1280, 800), (1920, 1080), (3840, 2160), (800, 1280), (640, 480))

    def run_theme(self, t, w, h):
        s = pp.Splash(str(PLY / t), w, h)
        self.assertEqual(s.errors, [], f'{t} at {w}x{h}: {s.errors}')
        if t != 'hearth-minimal':                      # the one preset where nothing moves
            self.assertIn('Refresh', s.callbacks, t)
        s.advance(6.0)
        # Encrypted installs: the password prompt shows, takes bullets (more than fit), and goes away again.
        self.assertTrue(s.password('', 0), f'{t}: no password prompt')
        s.advance(0.5)
        self.assertTrue(s.password('Unlock disk', 60))
        s.advance(0.5)
        self.assertTrue(s.normal())
        self.assertTrue(s.message('Checking the disk'))
        s.advance(1.0)
        self.assertEqual(s.errors, [], f'{t} at {w}x{h}: {s.errors}')
        return s

    def test_every_theme_runs_at_every_size(self):
        for t in themes():
            for w, h in self.SIZES:
                self.run_theme(t, w, h)

    def test_new_themes_use_only_proven_builtins(self):
        proven = set()
        for t in ORIGINAL:
            proven |= self.run_theme(t, 1280, 800).used
        for t in NEW:
            extra = self.run_theme(t, 1280, 800).used - proven
            self.assertEqual(extra, set(), f'{t} uses Plymouth functions the first three themes never used: {extra}')

    def test_mark_shows_and_stays_on_screen(self):
        """After the fade in, the mark is drawn fully on screen (not blank, not off the edge)."""
        for t in themes():
            for w, h in ((1280, 800), (800, 1280)):
                s = pp.Splash(str(PLY / t), w, h)
                s.advance(2.0)
                big = [sp for sp in s.sprites if sp.img is not None and sp.opacity > 0.9 and sp.z >= 10]
                self.assertTrue(big, f'{t}: nothing fully shown after 2 s')
                for sp in big:
                    self.assertTrue(0 <= sp.x and sp.x + sp.img.pil.width <= w and 0 <= sp.y and sp.y + sp.img.pil.height <= h,
                                    f'{t} at {w}x{h}: a sprite is off the screen')
                frame = s.draw()
                small = frame.resize((64, 40)).tobytes()
                self.assertGreater(len({small[i:i + 3] for i in range(0, len(small), 3)}), 20, f'{t}: the frame is blank')

    def test_strict_runner_catches_runtime_mistakes(self):
        with tempfile.TemporaryDirectory() as d:
            th = pathlib.Path(d) / 'broken'
            th.mkdir()
            shutil.copy(PLY / 'hearth' / 'logo.png', th / 'logo.png')
            for src, what in (('a = b + 1;', 'never set'), ('s = Sprite(Image("nope.png"));', 'no such picture'),
                              ('x = Math.Nope(1);', 'Math has no Nope'),
                              ('fun r() { global.q.SetOpacity(1); }\nPlymouth.SetRefreshFunction(r);', 'never set')):
                (th / 'broken.script').write_text(src)
                s = pp.Splash(str(th), 640, 480)
                s.advance(0.1)
                self.assertTrue(any(what in e for e in s.errors), (src, s.errors))


class Lists(unittest.TestCase):
    """Every place that names the splash presets names the same ones; the default stays hearth."""

    def test_names_agree(self):
        look = load_look()
        ids = [s['id'] for s in look.SPLASHES]
        self.assertEqual(sorted(ids), themes())
        self.assertEqual(ids[:3], list(ORIGINAL))          # the first three keep their places
        admin = ADMIN.read_text()
        m = re.search(r"cmd == 'boot-splash' and len\(argv\) == 3 and argv\[2\] in \(([^)]*)\)", admin)
        self.assertTrue(m)
        self.assertEqual(sorted(re.findall(r"'([a-z-]+)'", m.group(1))), sorted(ids))
        bls = BLS.read_text()
        m = re.search(r'^\s+(hearth-steam\|[a-z|-]+)\)', bls, re.M)
        self.assertTrue(m)
        self.assertEqual(sorted(m.group(1).split('|')), sorted(i for i in ids if i != 'hearth'))
        build = BUILD.read_text()
        for t in ids:
            if t != 'hearth':
                self.assertIn(f'/usr/share/plymouth/themes/{t}', build, f'{t} is not put in the initramfs')
        checks = IMAGE_CHECKS.read_text()
        for t in ids:
            self.assertIn(t, checks, f'base-peppermint.yml doesn\'t check {t}')

    def test_default_stays_hearth(self):
        self.assertIn('\nplymouth-set-default-theme hearth\n', BUILD.read_text())
        self.assertEqual(len(re.findall(r'^plymouth-set-default-theme ', BUILD.read_text(), re.M)), 1)
        look = load_look()
        with tempfile.TemporaryDirectory() as d:
            old = look.SPLASH_CONF
            look.SPLASH_CONF = os.path.join(d, 'boot-splash')
            self.addCleanup(setattr, look, 'SPLASH_CONF', old)
            self.assertEqual(look.splash_current(), 'hearth')
            pathlib.Path(look.SPLASH_CONF).write_text('hearth-aurora\n')
            self.assertEqual(look.splash_current(), 'hearth-aurora')
            # An existing choice keeps meaning what it meant.
            for t in ('hearth-steam', 'hearth-minimal'):
                pathlib.Path(look.SPLASH_CONF).write_text(t + '\n')
                self.assertEqual(look.splash_current(), t)

    def test_pairs_point_both_ways(self):
        look = load_look()
        movies = {m['id']: m for m in look.MOVIE_LIST}
        splashes = {s['id']: s for s in look.SPLASHES}
        for s in splashes.values():
            if 'pair' in s:
                self.assertEqual(movies[s['pair']].get('pair'), s['id'])
        for m in movies.values():
            if 'pair' in m:
                self.assertEqual(splashes[m['pair']].get('pair'), m['id'])
        self.assertEqual(sorted(s['id'] for s in splashes.values() if 'pair' in s), sorted(NEW))


def jpeg_size(path):
    with open(path, 'rb') as f:
        data = f.read()
    i = 2
    while i < len(data):
        if data[i] != 0xFF:
            i += 1
            continue
        marker = data[i + 1]
        if marker in (0xC0, 0xC1, 0xC2):
            h, w = struct.unpack('>HH', data[i + 5:i + 9])
            return w, h
        i += 2 + struct.unpack('>H', data[i + 2:i + 4])[0]
    return None


class Previews(unittest.TestCase):
    def setUp(self):
        self.index = json.loads((PREVIEWS / 'index.json').read_text())

    def test_a_sheet_for_every_choice(self):
        look = load_look()
        want = ['splash-' + s['id'] for s in look.SPLASHES] + ['movie-' + m['id'] for m in look.MOVIE_LIST]
        fw, fh, cols = self.index['frameWidth'], self.index['frameHeight'], self.index['columns']
        for name in want:
            info = self.index['sheets'].get(name)
            self.assertTrue(info, name)
            self.assertTrue(30 <= info['frames'] <= 80, name)
            self.assertTrue(0 <= info['poster'] < info['frames'], name)
            size = jpeg_size(PREVIEWS / f'{name}.jpg')
            rows = (info['frames'] + cols - 1) // cols
            self.assertEqual(size, (fw * cols, fh * rows), name)
            self.assertLessEqual(max(size), 4096, f'{name}: too big for some graphics chips')
            self.assertLess((PREVIEWS / f'{name}.jpg').stat().st_size, 600 * 1024, name)

    def test_status_hands_them_over(self):
        saved = {k: os.environ.get(k) for k in ('HEARTH_BOOT_PREVIEWS', 'HEARTH_PLYMOUTH_THEMES', 'HEARTH_MOVIES',
                                                'HEARTH_SPLASH_CONF', 'HOME', 'XDG_CONFIG_HOME', 'HEARTH_DRM')}

        def restore():
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
        self.addCleanup(restore)
        d = tempfile.TemporaryDirectory()
        self.addCleanup(d.cleanup)
        os.environ.update(HEARTH_BOOT_PREVIEWS=str(PREVIEWS), HEARTH_PLYMOUTH_THEMES=str(PLY), HEARTH_MOVIES=str(MOVIES),
                          HEARTH_SPLASH_CONF=os.path.join(d.name, 'boot-splash'), HOME=d.name,
                          HEARTH_DRM=os.path.join(d.name, 'drm'))
        os.environ.pop('XDG_CONFIG_HOME', None)
        look = load_look()
        import io
        import contextlib
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            look.status()
        st = json.loads(out.getvalue())
        self.assertEqual(st['splash']['current'], 'hearth')
        self.assertEqual(st['movie']['current'], 'steam')
        self.assertEqual(len(st['splash']['presets']), 6)
        self.assertEqual(len(st['movie']['list']), 7)
        for x in st['splash']['presets'] + st['movie']['list']:
            self.assertTrue(x['preview'] and os.path.isfile(x['preview']['sheet']), x['id'])
        self.assertEqual(st['preview']['columns'], self.index['columns'])
        # Looking (status) wrote nothing: no choice was made.
        self.assertFalse(os.path.exists(os.path.join(d.name, 'boot-splash')))
        self.assertFalse(os.path.exists(os.path.join(d.name, '.config', 'hearth', 'startup-movie')))

    def test_no_previews_is_fine(self):
        look = load_look()
        old = look.PREVIEWS
        self.addCleanup(setattr, look, 'PREVIEWS', old)
        with tempfile.TemporaryDirectory() as d:
            look.PREVIEWS = d
            self.assertIsNone(look.previews())
            pathlib.Path(d, 'index.json').write_text('{broken')
            self.assertIsNone(look.previews())
            self.assertEqual(look.preview(None, 'splash-hearth'), {})


if __name__ == '__main__':
    unittest.main()
