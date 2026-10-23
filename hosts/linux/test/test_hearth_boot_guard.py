"""Tests for hearth-boot-guard (going back to the previous version after starts that don't reach Hearth). No real /boot, bootc or GRUB: temporary folders and a fake bootc."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(ROOT / name))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


G = load('hearth-boot-guard')

ENTRY = 'title Hearth OS {v}\nversion {n}\nlinux /ostree/default-{c}/vmlinuz\ninitrd /ostree/default-{c}/initramfs.img\noptions root=UUID=x rw ostree=/ostree/boot.1/default/{c}/0\n'


def make_boot(d, n=2, custom=None):
    os.makedirs(f'{d}/loader/entries')
    os.makedirs(f'{d}/grub2')
    open(f'{d}/grub2/grub.cfg', 'w').write('# bootupd\n')
    if custom is not None:
        open(f'{d}/grub2/custom.cfg', 'w').write(custom)
    for i in range(1, n + 1):
        open(f'{d}/loader/entries/ostree-{i}.conf', 'w').write(ENTRY.format(v=i, n=i, c='c%d' % i))


class Env(unittest.TestCase):
    def test_round_trip_keeps_other_values(self):
        with tempfile.TemporaryDirectory() as d:
            G.env_set(d, saved_entry='x', menu_auto_hide='1')
            G.env_set(d, hearth_tries='1')
            raw = open(f'{d}/grubenv').read()
            self.assertEqual(len(raw.encode()), 1024)
            self.assertTrue(raw.startswith('# GRUB Environment Block\n'))
            self.assertEqual(G.env_read(d), {'saved_entry': 'x', 'menu_auto_hide': '1', 'hearth_tries': '1'})
            G.env_set(d, menu_auto_hide=None)
            self.assertNotIn('menu_auto_hide', G.env_read(d))

    def test_reads_what_grub_editenv_writes(self):
        text = '# GRUB Environment Block\nboot_success=1\nhearth_tries=2\n' + '#' * 900
        self.assertEqual(G.env_parse(text), {'boot_success': '1', 'hearth_tries': '2'})

    def test_too_big_is_refused(self):
        with self.assertRaises(ValueError):
            G.env_format({'x': 'y' * 1100})


class Setup(unittest.TestCase):
    def test_ids_newest_first(self):
        with tempfile.TemporaryDirectory() as d:
            make_boot(d, 2, custom="menuentry 'Windows' {\n  chainloader /EFI/Microsoft/Boot/bootmgfw.efi\n}\n")
            G.setup(d, installer=True)
            cfg = open(f'{d}/grub2/hearth-boot.cfg').read()
            self.assertIn("set hearth_default='ostree-2'", cfg)
            self.assertIn("set hearth_fallback='ostree-1'", cfg)
            self.assertIn('save_env -f "${prefix}/grubenv" hearth_tries', cfg)
            custom = open(f'{d}/grub2/custom.cfg').read()
            self.assertIn('Windows', custom)   # what the installer wrote stays
            self.assertEqual(custom.count('source ${prefix}/hearth-boot.cfg'), 1)
            self.assertTrue(os.path.exists(f'{d}/grub2/grubenv'))
            G.setup(d, installer=True)   # idempotent
            self.assertEqual(open(f'{d}/grub2/custom.cfg').read(), custom)

    def test_one_version_has_no_fallback(self):
        with tempfile.TemporaryDirectory() as d:
            make_boot(d, 1)
            G.setup(d, installer=True)
            cfg = open(f'{d}/grub2/hearth-boot.cfg').read()
            self.assertIn("set hearth_fallback=''", cfg)
            self.assertFalse(G.status(d, cmdline='')['previous'])

    def test_grub_script_shape(self):
        s = G.grub_script('ostree-2', 'ostree-1')
        # Only commands every GRUB has (no increment/decrement), and quotes balanced.
        self.assertNotIn('increment', s)
        lines = [l.strip() for l in s.splitlines()]
        self.assertEqual(sum(l.startswith('if [') for l in lines), sum(l == 'fi' for l in lines))
        self.assertEqual(s.count("'") % 2, 0)

    def test_unsafe_ids_left_out(self):
        with tempfile.TemporaryDirectory() as d:
            make_boot(d, 1)
            open(f"{d}/loader/entries/ostree-2'x.conf", 'w').write(ENTRY.format(v=2, n=2, c='c2'))
            G.setup(d, installer=True)
            self.assertIn("set hearth_default=''", open(f'{d}/grub2/hearth-boot.cfg').read())


class Success(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name + '/boot'
        G.STATE = self.tmp.name + '/state.json'
        self.calls = []
        self.real_run = G.subprocess.run

        def fake(cmd, **kw):
            self.calls.append(cmd)
            if cmd[:2] == ['bootc', 'status']:
                return subprocess.CompletedProcess(cmd, 0, json.dumps({'status': {'rollback': {'image': {
                    'imageDigest': 'sha256:bad', 'version': '0.6.0', 'image': {'image': 'ghcr.io/x/y:latest'}}}}}), '')
            return subprocess.CompletedProcess(cmd, 0, '', '')
        G.subprocess.run = fake

    def tearDown(self):
        G.subprocess.run = self.real_run
        self.tmp.cleanup()

    def test_not_set_up_does_nothing(self):
        make_boot(self.d, 2)
        G.success(self.d, cmdline='ostree=/ostree/boot.1/default/c2/0')
        self.assertFalse(os.path.exists(f'{self.d}/grub2/grubenv'))

    def test_good_start_resets_the_count(self):
        make_boot(self.d, 2)
        G.setup(self.d, installer=True)
        G.env_set(G.grub_dir(self.d), hearth_tries='2')
        G.success(self.d, cmdline='ostree=/ostree/boot.0/default/c2/0')
        self.assertEqual(G.env_read(G.grub_dir(self.d))['hearth_tries'], 'ok')
        self.assertNotIn(['bootc', 'rollback'], self.calls)

    def test_previous_version_after_fallback_becomes_default(self):
        make_boot(self.d, 2)
        G.setup(self.d, installer=True)
        G.env_set(G.grub_dir(self.d), hearth_tries='fallback')
        G.success(self.d, cmdline='quiet ostree=/ostree/boot.0/default/c1/0 rw')
        self.assertEqual(G.env_read(G.grub_dir(self.d))['hearth_tries'], 'ok')
        self.assertIn(['bootc', 'rollback'], self.calls)
        st = json.load(open(G.STATE))
        self.assertEqual(st['bad'][0]['digest'], 'sha256:bad')
        self.assertTrue(st['last']['rolledBack'])

    def test_picked_by_hand_is_not_made_default(self):
        make_boot(self.d, 2)
        G.setup(self.d, installer=True)
        G.env_set(G.grub_dir(self.d), hearth_tries='1')
        G.success(self.d, cmdline='ostree=/ostree/boot.0/default/c1/0')
        self.assertNotIn(['bootc', 'rollback'], self.calls)

    def test_previous_sets_fallback(self):
        make_boot(self.d, 2)
        G.BOOT = self.d
        try:
            self.assertEqual(G.previous(self.d), 0)
        finally:
            G.BOOT = '/boot'
        self.assertEqual(G.env_read(G.grub_dir(self.d))['hearth_tries'], 'fallback')

    def test_status_marks_running(self):
        make_boot(self.d, 2)
        st = G.status(self.d, cmdline='ostree=/ostree/boot.1/default/c1/0')
        self.assertEqual([v['running'] for v in st['versions']], [False, True])
        self.assertTrue(st['previous'])
        self.assertIn('[running now]', G.status_text(st))


if __name__ == '__main__':
    unittest.main()
