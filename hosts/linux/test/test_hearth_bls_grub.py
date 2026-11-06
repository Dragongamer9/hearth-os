"""Tests for hearth-bls-grub (Peppermint's GRUB menu from ostree's boot entries) and what 1.1 adds to it: clean names
("Hearth OS 1.1", "... (previous version)"), the Hearth GRUB theme copied to /boot and switched on only when it's
all there (sourced after every entry), and the boot splash preset on the kernel command line. Also
hearth-boot-look's splash/animation logic. With grub-script-check installed (GitHub's Ubuntu runners), GRUB itself
checks the written files."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = REPO / 'os-bases/peppermint/system_files/usr/libexec/nimbus/hearth-bls-grub'
THEME = REPO / 'os-bases/peppermint/system_files/usr/share/hearth/grub-theme'
PLY = REPO / 'os/system_files/usr/share/plymouth/themes'
LOOK = REPO / 'os/system_files/usr/libexec/nimbus/hearth-boot-look'
MOVIES = REPO / 'os/system_files/usr/share/hearth/startup-movies'

ENTRY = """title Hearth OS (Debian 13) (ostree:{n})
version {v}
options root=UUID=abc rootflags=subvol=root rw ostree=/ostree/boot.1/default/{c}/0 quiet splash
linux /ostree/default-{c}/vmlinuz-6.12
initrd /ostree/default-{c}/initramfs-6.12.img
"""


class Fixture:
    def __init__(self, d, versions=('1.1.0', '1.0.0'), theme=True, presets=True):
        self.d = pathlib.Path(d)
        self.own = self.d / 'own'                       # the image the script comes from
        (self.own / 'usr/libexec/nimbus').mkdir(parents=True)
        self.script = self.own / 'usr/libexec/nimbus/hearth-bls-grub'
        shutil.copy(SCRIPT, self.script)
        if theme:
            shutil.copytree(THEME, self.own / 'usr/share/hearth/grub-theme')
            (self.own / 'usr/share/hearth/grub-theme/hearth-regular-18.pf2').write_bytes(b'FILE\x00\x00\x00\x04PFF2')
        if presets:
            for t in sorted(p.name for p in PLY.iterdir() if (p / f'{p.name}.plymouth').is_file()):
                shutil.copytree(PLY / t, self.own / 'usr/share/plymouth/themes' / t)
        self.boot = self.d / 'boot'
        (self.boot / 'loader/entries').mkdir(parents=True)
        self.sysroot = self.d / 'sysroot'
        for i, v in enumerate(versions):
            c = f'csum{i}'
            (self.boot / f'loader/entries/ostree-{i + 1}.conf').write_text(ENTRY.format(n=i, v=len(versions) - i, c=c))
            dep = self.sysroot / f'ostree/deploy/default/deploy/{c}.0'
            (dep / 'usr/share/nimbus').mkdir(parents=True)
            if v:
                (dep / 'usr/share/nimbus/version').write_text(v + '\n')
            link = self.sysroot / f'ostree/boot.1/default/{c}'
            link.mkdir(parents=True)
            os.symlink(f'../../../deploy/default/deploy/{c}.0', link / '0')
        self.splash = self.d / 'boot-splash'
        self.off = self.d / 'no-grub-theme'

    def run(self, **env):
        e = dict(os.environ, HEARTH_SYSROOT=str(self.sysroot), HEARTH_SPLASH_CONF=str(self.splash),
                 HEARTH_GRUB_THEME_OFF=str(self.off), **env)
        return subprocess.run(['sh', str(self.script), '--boot', str(self.boot)], capture_output=True, text=True, env=e)

    @property
    def menu(self):
        return (self.boot / 'grub2/hearth-bls.cfg').read_text()


def grub_check(test, path):
    chk = shutil.which('grub-script-check') or shutil.which('grub2-script-check')
    if not chk:
        return
    r = subprocess.run([chk, str(path)], capture_output=True, text=True)
    test.assertEqual(r.returncode, 0, f'{path}: {r.stdout}{r.stderr}')


class Menu(unittest.TestCase):
    def test_names_and_order(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            r = f.run()
            self.assertEqual(r.returncode, 0, r.stderr)
            titles = [l.split("'")[1] for l in f.menu.splitlines() if l.startswith('menuentry')]
            self.assertEqual(titles, ['Hearth OS 1.1', 'Hearth OS 1.0 (previous version)'])
            self.assertIn('set default=0', f.menu)
            self.assertIn("--id 'ostree-1'", f.menu.split('menuentry')[1])
            grub_check(self, f.boot / 'grub2/hearth-bls.cfg')

    def test_no_version_file_keeps_ostree_title(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d, versions=('', '0.6.63'))
            f.run()
            titles = [l.split("'")[1] for l in f.menu.splitlines() if l.startswith('menuentry')]
            self.assertEqual(titles, ['Hearth OS (Debian 13)', 'Hearth OS 0.6.63 (previous version)'])

    def test_installer_finds_the_root_above_boot(self):
        """An installer's --boot /mnt/target/boot (no HEARTH_SYSROOT, no /sysroot): names from /mnt/target."""
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            shutil.move(str(f.sysroot / 'ostree'), str(f.d / 'ostree'))
            e = dict(os.environ, HEARTH_SPLASH_CONF=str(f.splash), HEARTH_GRUB_THEME_OFF=str(f.off))
            e.pop('HEARTH_SYSROOT', None)
            if os.path.isdir('/sysroot/ostree'):
                self.skipTest('this machine has /sysroot/ostree')
            r = subprocess.run(['sh', str(f.script), '--boot', str(f.boot)], capture_output=True, text=True, env=e)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertIn("menuentry 'Hearth OS 1.1'", f.menu)

    def test_no_entries_keeps_old_menu(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d, versions=())
            (f.boot / 'grub2').mkdir()
            (f.boot / 'grub2/hearth-bls.cfg').write_text('old\n')
            r = f.run()
            self.assertNotEqual(r.returncode, 0)
            self.assertEqual(f.menu, 'old\n')


class Theme(unittest.TestCase):
    def test_copied_and_sourced_last(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            f.run()
            g = f.boot / 'grub2'
            self.assertTrue((g / 'themes/hearth/theme.txt').is_file())
            self.assertTrue((g / 'themes/hearth/background.png').is_file())
            cfg = (g / 'hearth-theme.cfg').read_text()
            lines = [l for l in f.menu.splitlines() if l.strip()]
            self.assertEqual(lines[-1], 'if [ -f "${prefix}/hearth-theme.cfg" ]; then source "${prefix}/hearth-theme.cfg"; fi')
            self.assertLess(max(i for i, l in enumerate(lines) if l == '}'), len(lines) - 1)
            # Only switches when every piece is there: each step guarded.
            for step in ('insmod gfxterm', 'insmod gfxmenu', 'insmod png', 'loadfont', 'terminal_output gfxterm', 'set theme='):
                self.assertIn(step, cfg)
            self.assertLess(cfg.index('insmod png'), cfg.index('set theme='))
            self.assertNotIn('timeout', cfg)
            self.assertNotIn('default', cfg)
            grub_check(self, g / 'hearth-theme.cfg')

    def test_off_switch_removes_it(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            f.run()
            f.off.write_text('')
            f.run()
            g = f.boot / 'grub2'
            self.assertFalse((g / 'themes/hearth').exists())
            self.assertFalse((g / 'hearth-theme.cfg').exists())
            self.assertIn("menuentry 'Hearth OS 1.1'", f.menu)

    def test_no_theme_in_image(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d, theme=False)
            r = f.run()
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertFalse((f.boot / 'grub2/hearth-theme.cfg').exists())

    def test_broken_copy_on_boot_is_put_back(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            f.run()
            t = f.boot / 'grub2/themes/hearth/theme.txt'
            t.write_text('garbage {{{')
            f.run()
            self.assertEqual(t.read_text(), (THEME / 'theme.txt').read_text())
            (f.boot / 'grub2/themes/hearth/background.png').unlink()
            f.run()
            self.assertTrue((f.boot / 'grub2/themes/hearth/background.png').is_file())

    def test_theme_has_its_files(self):
        txt = (THEME / 'theme.txt').read_text()
        for name in ('background.png', 'logo.png', 'select_c.png', 'menu_c.png'):
            self.assertIn(name.split('_')[0], txt)
            self.assertTrue((THEME / name).is_file(), name)
        for k in ('nw', 'n', 'ne', 'w', 'c', 'e', 'sw', 's', 'se'):
            self.assertTrue((THEME / f'select_{k}.png').is_file())


class Splash(unittest.TestCase):
    def kernel_lines(self, f):
        return [l for l in f.menu.splitlines() if l.strip().startswith('linux ')]

    def test_default_adds_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            f.run()
            self.assertTrue(all('plymouth.splash' not in l for l in self.kernel_lines(f)))
            f.splash.write_text('hearth\n')
            f.run()
            self.assertTrue(all('plymouth.splash' not in l for l in self.kernel_lines(f)))

    def test_preset_on_every_entry(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            f.splash.write_text('hearth-steam\n')
            f.run()
            ls = self.kernel_lines(f)
            self.assertEqual(len(ls), 2)
            self.assertTrue(all(l.endswith(' plymouth.splash=hearth-steam') for l in ls), ls)
            # The presets added after 1.1 the same way.
            for t in ('hearth-aurora', 'hearth-starlight', 'hearth-orbit'):
                f.splash.write_text(t + '\n')
                f.run()
                ls = self.kernel_lines(f)
                self.assertTrue(all(l.endswith(' plymouth.splash=' + t) for l in ls), ls)

    def test_unknown_or_missing_preset_adds_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d)
            for v in ('evil; reboot', 'hearth-steam rd.break', 'nope', 'hearth-aurora-x', 'hearth-orbit init=/bin/sh'):
                f.splash.write_text(v + '\n')
                f.run()
                self.assertTrue(all('plymouth.splash' not in l and 'rd.break' not in l for l in self.kernel_lines(f)), v)
        with tempfile.TemporaryDirectory() as d:
            f = Fixture(d, presets=False)
            f.splash.write_text('hearth-minimal\n')
            f.run()
            self.assertTrue(all('plymouth.splash' not in l for l in self.kernel_lines(f)))


class Presets(unittest.TestCase):
    def test_each_theme_complete(self):
        for t in ('hearth', 'hearth-steam', 'hearth-minimal', 'hearth-aurora', 'hearth-starlight', 'hearth-orbit'):
            conf = (PLY / t / f'{t}.plymouth').read_text()
            self.assertIn(f'ScriptFile=/usr/share/plymouth/themes/{t}/{t}.script', conf)
            self.assertIn(f'ImageDir=/usr/share/plymouth/themes/{t}', conf)
            script = (PLY / t / f'{t}.script').read_text()
            # Encrypted installs: the disk password prompt in every preset.
            self.assertIn('Plymouth.SetDisplayPasswordFunction', script)
            for img in __import__('re').findall(r'Image\("([^"]+)"\)', script):
                self.assertTrue((PLY / t / img).is_file(), f'{t}: {img}')


def load_look():
    loader = importlib.machinery.SourceFileLoader('hearth_boot_look', str(LOOK))
    spec = importlib.util.spec_from_loader('hearth_boot_look', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class StartupMovie(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = pathlib.Path(self.tmp.name)
        self.home = self.d / 'home'
        (self.home / '.steam/root/config').mkdir(parents=True)
        self.drm = self.d / 'drm'
        (self.drm / 'card0-eDP-1').mkdir(parents=True)
        (self.drm / 'card0-eDP-1/status').write_text('connected\n')
        (self.drm / 'card0-eDP-1/modes').write_text('800x1280\n')
        # The environment goes back as it was afterwards (other tests in the same run read it).
        saved = {k: os.environ.get(k) for k in ('HOME', 'HEARTH_DRM', 'HEARTH_MOVIES', 'XDG_CONFIG_HOME')}

        def restore():
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
        self.addCleanup(restore)
        os.environ.update(HOME=str(self.home), HEARTH_DRM=str(self.drm), HEARTH_MOVIES=str(MOVIES))
        os.environ.pop('XDG_CONFIG_HOME', None)
        self.m = load_look()
        self.movies = self.home / '.steam/root/config/uioverrides/movies'

    def tearDown(self):
        self.tmp.cleanup()

    def test_portrait_handheld_is_16x10(self):
        self.assertEqual(self.m.screen_aspect(), '16x10')

    def test_choice_applied_and_all_listed(self):
        with open(os.devnull, 'w') as null:
            old = sys.stdout; sys.stdout = null
            try:
                self.m.movie_set('flame')
            finally:
                sys.stdout = old
        self.assertEqual((self.movies / 'deck_startup.webm').read_bytes(), (MOVIES / 'hearth-flame-16x10.webm').read_bytes())
        for mid in self.m.MOVIE_IDS:
            self.assertTrue((self.movies / f'hearth-{mid}.webm').is_file())
        # Back to Steam's own: our deck_startup.webm goes, the listed ones stay.
        self.m.CHOICE and pathlib.Path(self.m.CHOICE).write_text('steam\n')
        self.m.movie_apply()
        self.assertFalse((self.movies / 'deck_startup.webm').exists())
        self.assertTrue((self.movies / 'hearth-flame.webm').exists())

    def test_someone_elses_file_is_left_alone(self):
        self.movies.mkdir(parents=True)
        (self.movies / 'deck_startup.webm').write_bytes(b'my own movie')
        pathlib.Path(self.m.CHOICE).parent.mkdir(parents=True, exist_ok=True)
        pathlib.Path(self.m.CHOICE).write_text('sweep\n')
        r = self.m.movie_apply()
        self.assertEqual((self.movies / 'deck_startup.webm').read_bytes(), b'my own movie')
        self.assertFalse(r['steam'][0]['applied'])
        pathlib.Path(self.m.CHOICE).write_text('steam\n')
        self.m.movie_apply()
        self.assertEqual((self.movies / 'deck_startup.webm').read_bytes(), b'my own movie')

    def test_no_steam_yet_does_nothing(self):
        shutil.rmtree(self.home / '.steam')
        r = self.m.movie_apply()
        self.assertEqual(r['steam'], [])
        self.assertFalse((self.home / '.steam').exists())

    def test_movies_are_vp9_webm_in_both_aspects(self):
        self.assertEqual(len(self.m.MOVIE_IDS), 7)
        for mid in self.m.MOVIE_IDS:
            for a in ('16x9', '16x10'):
                p = MOVIES / f'hearth-{mid}-{a}.webm'
                self.assertTrue(p.is_file(), p)
                self.assertEqual(p.read_bytes()[:4], b'\x1a\x45\xdf\xa3', p)   # EBML (Matroska/WebM)
                self.assertIn(b'V_VP9', p.read_bytes()[:4096], p)


if __name__ == '__main__':
    unittest.main()
