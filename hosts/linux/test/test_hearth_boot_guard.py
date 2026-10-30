"""Tests for hearth-boot-guard (automatic fallback to the previous version): the GRUB environment block, the menu
snippet, custom.cfg, the ostree status check, and a whole failed-update story with GRUB's logic played out.
With grub-script-check / grub-editenv installed (GitHub's Ubuntu runners), GRUB itself checks the files too."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

PATH = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-boot-guard'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_boot_guard', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_boot_guard', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


G = load()


def grub_play(env):
    """What GRUB does with the snippet (the counting part), as a function of the saved environment."""
    tries = env.get('hearth_tries', '')
    out = dict(env)
    default = 0
    if tries == '2':
        default, out['hearth_fellback'], out['hearth_tries'] = 1, '1', '0'
    elif tries == '1':
        out['hearth_tries'] = '2'
    else:
        out['hearth_tries'] = '1'
    return default, out


class EnvBlock(unittest.TestCase):
    def test_size_and_round_trip(self):
        data = G.render_env({'saved_entry': 'x', 'hearth_tries': '1'})
        self.assertEqual(len(data), 1024)
        self.assertTrue(data.startswith(b'# GRUB Environment Block\n'))
        self.assertEqual(G.parse_env(data), {'saved_entry': 'x', 'hearth_tries': '1'})

    def test_keeps_other_variables(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'grubenv')
            G.write_env(p, {'menu_auto_hide': '1', 'boot_success': '1'})
            G.write_env(p, {'hearth_tries': '2', 'boot_success': None})
            self.assertEqual(G.read_env(p), {'menu_auto_hide': '1', 'hearth_tries': '2'})
            self.assertEqual(os.path.getsize(p), 1024)

    def test_grub_editenv_reads_it(self):
        ed = shutil.which('grub-editenv') or shutil.which('grub2-editenv')
        if not ed:
            self.skipTest('no grub-editenv')
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'grubenv')
            G.write_env(p, {'hearth_tries': '1'})
            self.assertIn('hearth_tries=1', subprocess.run([ed, p, 'list'], capture_output=True, text=True).stdout)
            subprocess.run([ed, p, 'set', 'other=5'], check=True)
            self.assertEqual(G.read_env(p).get('other'), '5')


class Snippet(unittest.TestCase):
    def test_off_without_previous_version(self):
        s = G.snippet(has_prev=False, can_save=True)
        self.assertNotIn('set default', s)
        self.assertNotIn('fallback', s.split('\n', 2)[2])

    def test_off_switch(self):
        self.assertNotIn('set default', G.snippet(True, True, off=True))

    def test_no_counting_where_grub_cannot_save(self):
        s = G.snippet(True, can_save=False)
        self.assertIn('set fallback=1', s)
        self.assertNotIn('save_env', s)

    def test_counting(self):
        s = G.snippet(True, True)
        self.assertIn('set default=1', s)
        self.assertIn('save_env hearth_tries hearth_fellback', s)

    def test_grub_parses_it(self):
        chk = shutil.which('grub-script-check') or shutil.which('grub2-script-check')
        if not chk:
            self.skipTest('no grub-script-check')
        for args in ((True, True), (True, False), (False, True)):
            with tempfile.NamedTemporaryFile('w', suffix='.cfg', delete=False) as f:
                f.write(G.snippet(*args))
            try:
                r = subprocess.run([chk, f.name], capture_output=True, text=True)
                self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            finally:
                os.unlink(f.name)
        with tempfile.NamedTemporaryFile('w', suffix='.cfg', delete=False) as f:
            f.write(G.custom_with_block("menuentry 'Windows' {\n  chainloader /x\n}\n"))
        try:
            self.assertEqual(subprocess.run([chk, f.name]).returncode, 0)
        finally:
            os.unlink(f.name)


class Custom(unittest.TestCase):
    def test_keeps_windows_entries_and_adds_once(self):
        win = "menuentry 'Windows Boot Manager' {\n  chainloader /EFI/Microsoft/Boot/bootmgfw.efi\n}\n"
        once = G.custom_with_block(win)
        self.assertIn(win, once)
        self.assertEqual(G.custom_with_block(once), once)
        self.assertEqual(once.count(G.BEGIN), 1)

    def test_empty(self):
        self.assertIn('source $prefix/hearth-fallback.cfg', G.custom_with_block(''))


class Status(unittest.TestCase):
    H = 'a' * 64

    def test_booted_default(self):
        st = f'* default {self.H}.0\n    Version: 0.6.57\n  default {"b" * 64}.0 (rollback)\n'
        self.assertTrue(G.booted_is_default(st))

    def test_booted_previous(self):
        st = f'  default {self.H}.0 (staged)\n* default {"b" * 64}.1\n'
        self.assertFalse(G.booted_is_default(st))

    def test_unknown(self):
        self.assertIsNone(G.booted_is_default('error: not an ostree system'))


class Story(unittest.TestCase):
    def test_broken_update_goes_back_on_third_start(self):
        env = {}
        d, env = grub_play(env)          # 1st start of the new version: fails (never "good")
        self.assertEqual(d, 0)
        d, env = grub_play(env)          # 2nd: fails
        self.assertEqual(d, 0)
        d, env = grub_play(env)          # 3rd: the previous version
        self.assertEqual((d, env['hearth_fellback'], env['hearth_tries']), (1, '1', '0'))

    def test_good_start_resets(self):
        env = {}
        _, env = grub_play(env)
        env.update({'hearth_tries': '0', 'hearth_fellback': '0'})   # what `good` writes
        for _ in range(5):
            d, env = grub_play(env)
            self.assertEqual(d, 0)
            env['hearth_tries'] = '0'

    def test_never_stuck_on_the_old_version(self):
        # A start that never says "good" (a bug, or the previous version without this tool): after going back once,
        # the new version is tried again; the device is never kept on the old one by the count.
        env, seen = {}, []
        for _ in range(6):
            d, env = grub_play(env)
            seen.append(d)
        self.assertEqual(seen, [0, 0, 1, 0, 0, 1])

    def test_write_with_a_fake_boot(self):
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(f'{d}/grub2'); os.makedirs(f'{d}/loader/entries')
            open(f'{d}/grub2/grub.cfg', 'w').close()
            open(f'{d}/loader/entries/ostree-1.conf', 'w').close()
            G.BOOT, G.OFF = d, f'{d}/no-guard'
            G.fstype = lambda p: 'ext4'
            G.cmd_write()
            self.assertNotIn('set default', open(f'{d}/grub2/hearth-fallback.cfg').read())
            self.assertEqual(os.path.getsize(f'{d}/grub2/grubenv'), 1024)
            open(f'{d}/loader/entries/ostree-2.conf', 'w').close()
            G.cmd_write()
            self.assertIn('set default=1', open(f'{d}/grub2/hearth-fallback.cfg').read())
            self.assertIn('hearth-fallback.cfg', open(f'{d}/grub2/custom.cfg').read())
            G.write_env(f'{d}/grub2/grubenv', {'hearth_tries': '2'})
            G.cmd_good()
            self.assertEqual(G.read_env(f'{d}/grub2/grubenv')['hearth_tries'], '0')


if __name__ == '__main__':
    unittest.main()
