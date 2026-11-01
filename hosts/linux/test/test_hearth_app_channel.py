"""Hearth app updates (docs/APP-UPDATES.md): the start-up choice between a Hearth app update and the built-in Hearth,
end to end, with the real program.

With HEARTH_BIN set to a built `hearth` (release.yml's app-bundle jobs, rust.yml): a throwaway signing key is made here
(ssh-keygen, never kept), bundles are packed with tools/app-bundle.py and signed with it, installed into a temporary
home with `hearth --app-install`, and Hearth is started the way the session's watchdog starts it (`--session`, with
NIMBUS_SESSION=1) to see which program runs (`--app-which`). Every fallback path is played: a bad signature, another
edition or ABI, an older or same version, a changed file, a bundle that crashes, one that never brings its screen up,
one that can't be started, the force flags, Off, Desktop Mode windows and extra multi-seat players before the bundle
was good, and the system catching up. Without HEARTH_BIN only the packing and ABI parts run."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
TOOL = REPO / 'tools' / 'app-bundle.py'
BIN = os.environ.get('HEARTH_BIN', '')
HAVE_TOOLS = all(shutil.which(t) for t in ('tar', 'zstd'))
CAN_RUN = bool(BIN) and os.path.isfile(BIN) and HAVE_TOOLS and bool(shutil.which('ssh-keygen'))
BUILTIN = '0.6.60'
EDITION = os.environ.get('HEARTH_EDITION', 'anatase')


def run(cmd, env=None, timeout=60, check=False):
    r = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout)
    if check and r.returncode != 0:
        raise AssertionError(f'{cmd} failed ({r.returncode}): {r.stdout} {r.stderr}')
    return r


class Packing(unittest.TestCase):
    def test_abi_tag_is_stable_and_follows_the_image_files(self):
        sys.path.insert(0, str(REPO / 'tools'))
        import importlib.util
        spec = importlib.util.spec_from_file_location('app_bundle', TOOL)
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
        tag = m.abi_tag()
        self.assertRegex(tag, r'^\d+-[0-9a-f]{16}$')
        self.assertEqual(tag, m.abi_tag())
        with tempfile.TemporaryDirectory() as t:
            for rel in m.ABI_FILES:
                p = pathlib.Path(t, rel)
                p.parent.mkdir(parents=True, exist_ok=True)
                src = REPO / rel
                if src.is_file():
                    shutil.copy(src, p)
            shutil.copytree(REPO / 'os/system_files/usr/share/polkit-1', pathlib.Path(t, 'os/system_files/usr/share/polkit-1'))
            self.assertEqual(m.abi_tag(t), tag, 'the same files give the same tag')
            with open(pathlib.Path(t, 'os/system_files/usr/libexec/nimbus/hearth-admin'), 'a') as f:
                f.write('\n# a new verb\n')
            self.assertNotEqual(m.abi_tag(t), tag, 'a changed root helper is a new ABI')

    @unittest.skipUnless(HAVE_TOOLS, 'tar and zstd')
    def test_pack_lists_every_file_and_drops_the_setuid_helper(self):
        with tempfile.TemporaryDirectory() as t:
            src = pathlib.Path(t, 'src')
            (src / 'electron').mkdir(parents=True)
            (src / 'nimbus').write_text('#!/bin/sh\n')
            (src / 'nimbus').chmod(0o755)
            (src / 'electron/chrome-sandbox').write_text('x')
            (src / 'electron/resources.pak').write_text('pak')
            os.symlink('resources.pak', src / 'electron/link.pak')
            out = pathlib.Path(t, 'b')
            run([sys.executable, str(TOOL), 'pack', '--src', str(src), '--version', '0.6.61', '--edition', 'anatase', '--out', str(out)], check=True)
            man = json.loads(pathlib.Path(str(out) + '.json').read_text())
            paths = {f['path']: f for f in man['files']}
            self.assertEqual(set(paths), {'nimbus', 'electron/resources.pak', 'electron/link.pak'})
            self.assertEqual(paths['nimbus']['mode'], 0o755)
            self.assertEqual(paths['electron/link.pak']['link'], 'resources.pak')
            self.assertEqual(man['size'], os.path.getsize(str(out) + '.tar.zst'))
            self.assertRegex(man['abi'], r'^\d+-[0-9a-f]{16}$')

    @unittest.skipUnless(HAVE_TOOLS, 'tar and zstd')
    def test_pack_refuses_links_out_of_the_bundle(self):
        with tempfile.TemporaryDirectory() as t:
            src = pathlib.Path(t, 'src')
            src.mkdir()
            (src / 'nimbus').write_text('#!/bin/sh\n')
            (src / 'nimbus').chmod(0o755)
            os.symlink('/etc/passwd', src / 'evil')
            r = run([sys.executable, str(TOOL), 'pack', '--src', str(src), '--version', '1', '--edition', 'anatase', '--out', str(pathlib.Path(t, 'b'))])
            self.assertNotEqual(r.returncode, 0)


@unittest.skipUnless(CAN_RUN, 'needs HEARTH_BIN (a built hearth), ssh-keygen, tar and zstd')
class StartUpChoice(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.dir = pathlib.Path(self.t.name)
        self.home = self.dir / 'home'
        self.home.mkdir()
        self.run_dir = self.dir / 'run'
        self.run_dir.mkdir()
        self.image = self.dir / 'image'
        self.image.mkdir()
        self.key = self.dir / 'key'
        run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-C', 'throwaway', '-f', str(self.key)], check=True)
        self.abi = '1-0123456789abcdef'
        (self.image / 'version').write_text(BUILTIN + '\n')
        (self.image / 'app-abi').write_text(self.abi + '\n')
        (self.image / 'image-info.json').write_text(json.dumps({'base': EDITION}))
        (self.image / 'app-signing.pub').write_text(pathlib.Path(str(self.key) + '.pub').read_text())
        self.cmdline = self.dir / 'cmdline'
        self.cmdline.write_text('quiet splash\n')
        self.root = self.home / '.local/share/hearth/app'
        self.n = 0

    def tearDown(self):
        self.t.cleanup()

    def env(self, **extra):
        e = {'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'HOME': str(self.home), 'XDG_RUNTIME_DIR': str(self.run_dir),
             'HEARTH_APP_IMAGE_DIR': str(self.image), 'HEARTH_CMDLINE': str(self.cmdline), 'NIMBUS_SESSION': '1',
             'HEARTH_RENDERER': 'webkit', 'HEARTH_APP_UI_TIMEOUT': '600'}
        e.update({k: v for k, v in extra.items() if v is not None})
        return e

    def bundle(self, version, program=None, edition=EDITION, abi=None, min_image='', key=None, extra=None):
        """Packs, signs and returns (archive, manifest, signature)."""
        self.n += 1
        src = self.dir / f'src{self.n}'
        src.mkdir()
        if program is None:
            shutil.copy(BIN, src / 'nimbus')
        else:
            (src / 'nimbus').write_text(program)
        (src / 'nimbus').chmod(0o755)
        (src / 'electron').mkdir()
        (src / 'electron/resources.txt').write_text('screens ' + version)
        for rel, body in (extra or {}).items():
            (src / rel).write_text(body)
        out = self.dir / f'b{self.n}'
        run([sys.executable, str(TOOL), 'pack', '--src', str(src), '--version', version, '--edition', edition, '--abi', abi or self.abi,
             '--min-image', min_image, '--level', '3', '--out', str(out)], check=True)
        man = pathlib.Path(str(out) + '.json')
        run(['ssh-keygen', '-Y', 'sign', '-q', '-n', 'hearth-app', '-f', str(key or self.key), str(man)], check=True)
        sig = pathlib.Path(str(man) + '.sig')
        # The real ssh-keygen agrees with itself (and so with what release.yml publishes).
        allowed = self.dir / 'allowed'
        allowed.write_text('hearth ' + pathlib.Path(str(key or self.key) + '.pub').read_text())
        with open(man, 'rb') as f:
            subprocess.run(['ssh-keygen', '-Y', 'verify', '-f', str(allowed), '-I', 'hearth', '-n', 'hearth-app', '-s', str(sig)], stdin=f, check=True, capture_output=True)
        return pathlib.Path(str(out) + '.tar.zst'), man, sig

    def install(self, b, ok=True):
        r = run([BIN, '--app-install', str(b[0]), str(b[1]), str(b[2])], env=self.env())
        if ok:
            self.assertEqual(r.returncode, 0, f'install failed: {r.stdout} {r.stderr}')
        else:
            self.assertNotEqual(r.returncode, 0, f'install should have failed: {r.stdout}')
        return r

    def start(self, *args, **env):
        """Starts Hearth as the session's watchdog does (or with other args); returns (code, which JSON or None)."""
        a = list(args) if args else ['--session', '--app-which']
        r = run([BIN] + a, env=self.env(**env), timeout=60)
        which = None
        for line in r.stdout.splitlines():
            if line.startswith('{'):
                which = json.loads(line)
        return r.returncode, which

    def assert_bundle(self, version, *args, **env):
        # Game Mode's start by default: the bundle says its screen came up and marks itself good (as a real start does
        # a minute after its screen comes up), so the count doesn't run on between checks.
        code, w = self.start(*(args or ('--session', '--app-mark-good')), **env)
        self.assertEqual(code, 0)
        self.assertIsNotNone(w, 'no answer')
        self.assertEqual(w['bundle'], version, f'expected the bundle {version}, got {w}')
        self.assertTrue(w['exe'].startswith(str(self.root)), w)

    def assert_builtin(self, *args, **env):
        code, w = self.start(*args, **env)
        self.assertEqual(code, 0, f'the built-in Hearth should have answered (code {code})')
        self.assertIsNotNone(w)
        self.assertIsNone(w['bundle'], f'expected the built-in Hearth, got {w}')
        self.assertEqual(os.path.realpath(w['exe']), os.path.realpath(BIN))

    def status(self):
        return json.loads(run([BIN, '--app-status'], env=self.env(), check=True).stdout)

    # ---------------------------------------------------------------------------------------------- the good path

    def test_a_checked_bundle_is_picked_and_switching_keeps_the_previous(self):
        self.assert_builtin()   # nothing installed
        self.install(self.bundle('0.6.61'))
        self.assertEqual(os.readlink(self.root / 'current'), '0.6.61')
        self.assert_bundle('0.6.61')
        self.install(self.bundle('0.6.62'))
        self.assertEqual(os.readlink(self.root / 'current'), '0.6.62')
        self.assertEqual(os.readlink(self.root / 'previous'), '0.6.61')
        self.assert_bundle('0.6.62')
        self.install(self.bundle('0.6.63'))
        self.assertFalse((self.root / '0.6.61').exists(), 'only current and previous are kept')
        s = self.status()
        self.assertEqual(s['bundle'], '0.6.63')
        self.assertEqual(s['previous'], '0.6.62')
        self.assertEqual(s['builtin'], BUILTIN)

    def test_good_marks_open_it_to_windows_and_extra_players(self):
        self.install(self.bundle('0.6.61'))
        self.assert_builtin('--windowed', '--app-which')
        self.assert_builtin('--session', '--app-which', HEARTH_SEAT='2')
        code, _ = self.start('--session', '--app-mark-good')
        self.assertEqual(code, 0)
        self.assertIn('0.6.61', self.status()['good'])
        self.assert_bundle('0.6.61', '--windowed', '--app-which')
        self.assert_bundle('0.6.61', '--session', '--app-which', HEARTH_SEAT='2')

    # ------------------------------------------------------------------------------------- refused at install

    def test_install_refuses_what_doesnt_check_out(self):
        other = self.dir / 'other'
        run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(other)], check=True)
        self.install(self.bundle('0.6.61', key=other), ok=False)
        self.install(self.bundle('0.6.61', edition='nope'), ok=False)
        self.install(self.bundle('0.6.61', abi='2-ffffffffffffffff'), ok=False)
        self.install(self.bundle(BUILTIN), ok=False)
        self.install(self.bundle('0.6.59'), ok=False)
        self.install(self.bundle('0.6.61', min_image='0.6.61'), ok=False)
        a, m, s = self.bundle('0.6.61')
        data = bytearray(a.read_bytes())
        data[len(data) // 2] ^= 0xFF
        a.write_bytes(bytes(data))
        self.install((a, m, s), ok=False)
        a, m, s = self.bundle('0.6.61')
        m.write_text(m.read_text().replace('"0.6.61"', '"0.6.69"'))
        self.install((a, m, s), ok=False)
        self.assertFalse((self.root / 'current').exists())
        self.assert_builtin()

    # ------------------------------------------------------------------------------------- refused at start

    def test_a_changed_bundle_falls_back(self):
        self.install(self.bundle('0.6.61'))
        prog = self.root / '0.6.61' / 'nimbus'
        data = bytearray(prog.read_bytes())
        data[-1] ^= 1
        prog.write_bytes(bytes(data))
        self.assert_builtin()
        self.install(self.bundle('0.6.62'))
        (self.root / '0.6.62/manifest.json').write_text((self.root / '0.6.62/manifest.json').read_text().replace(EDITION, EDITION + 'x'))
        self.assert_builtin()
        self.install(self.bundle('0.6.63'))
        (self.root / '0.6.63/manifest.json.sig').unlink()
        self.assert_builtin()

    def test_the_system_catching_up_wins(self):
        self.install(self.bundle('0.6.61'))
        self.assert_bundle('0.6.61')
        (self.image / 'version').write_text('0.6.61\n')
        self.assert_builtin()
        (self.image / 'version').write_text('0.6.62\n')
        self.assert_builtin()

    def test_another_abi_or_no_key_in_the_image_falls_back(self):
        self.install(self.bundle('0.6.61'))
        (self.image / 'app-abi').write_text('2-0123456789abcdef\n')
        self.assert_builtin()
        (self.image / 'app-abi').write_text(self.abi + '\n')
        (self.image / 'app-signing.pub').write_text('# placeholder: no key yet\n')
        self.assert_builtin()
        (self.image / 'app-signing.pub').write_text(pathlib.Path(str(self.key) + '.pub').read_text())
        (self.image / 'app-abi').unlink()
        self.assert_builtin()

    def test_force_flags_and_off(self):
        self.install(self.bundle('0.6.61'))
        self.assert_builtin(HEARTH_APP='builtin')
        self.cmdline.write_text('quiet hearth.app=builtin splash\n')
        self.assert_builtin()
        self.cmdline.write_text('quiet\n')
        flag = self.home / '.config/nimbus/app-builtin'
        flag.parent.mkdir(parents=True, exist_ok=True)
        flag.write_text('')
        self.assert_builtin()
        flag.unlink()
        (self.run_dir / 'hearth-app-builtin').write_text('')
        self.assert_builtin()
        (self.run_dir / 'hearth-app-builtin').unlink()
        (self.home / '.config/nimbus/app-updates').write_text('off')
        self.assert_builtin()
        (self.home / '.config/nimbus/app-updates').write_text('ask')
        self.assert_bundle('0.6.61')

    def test_qt_renderer_uses_the_built_in(self):
        self.install(self.bundle('0.6.61'))
        self.assert_builtin(HEARTH_RENDERER='qt', HEARTH_QT_APP=BIN)

    def test_use_the_built_in_version(self):
        self.install(self.bundle('0.6.61'))
        self.assertEqual(run([BIN, '--app-use-builtin'], env=self.env()).returncode, 0)
        self.assert_builtin()
        self.install(self.bundle('0.6.61'), ok=False)   # set aside: not taken again
        self.install(self.bundle('0.6.62'))             # a newer one is
        self.assert_bundle('0.6.62')

    # --------------------------------------------------------------------------------- the start counter

    def test_a_bundle_that_crashes_twice_is_set_aside(self):
        self.install(self.bundle('0.6.61', program='#!/bin/sh\necho crashing >&2\nexit 3\n'))
        self.assertEqual(self.start()[0], 3, 'first start: the bundle runs (and crashes)')
        self.assertEqual(self.start()[0], 3, 'second start: tried once more')
        self.assert_builtin()   # third start: set aside, built-in
        s = self.status()
        self.assertIn('0.6.61', s['bad'])
        self.assertIn('in a row', s['lastFallback'])
        self.assert_builtin()   # and stays so

    def test_a_bundle_whose_screen_never_comes_up_is_stopped_and_set_aside(self):
        self.install(self.bundle('0.6.61', program='#!/bin/sh\nsleep 120\n'))
        code, _ = self.start(HEARTH_APP_UI_TIMEOUT='2')
        self.assertEqual(code, 1, 'stopped after the timeout, counted as failed')
        code, _ = self.start(HEARTH_APP_UI_TIMEOUT='2')
        self.assertEqual(code, 1)
        self.assert_builtin(HEARTH_APP_UI_TIMEOUT='2')
        self.assertIn('0.6.61', self.status()['bad'])

    def test_a_screen_that_comes_up_keeps_the_bundle(self):
        self.install(self.bundle('0.6.61'))
        code, w = self.start('--session', '--app-ui-up', HEARTH_APP_UI_TIMEOUT='1')
        self.assertEqual(code, 0, 'its screen came up in time: not stopped')
        self.assertEqual(w['bundle'], '0.6.61')

    def test_a_good_start_resets_the_count(self):
        self.install(self.bundle('0.6.61'))
        for _ in range(4):
            self.assertEqual(self.start('--session', '--app-mark-good')[0], 0)
        self.assert_bundle('0.6.61')
        self.assertEqual(self.status()['bad'], {})

    def test_a_bundle_that_cant_start_falls_back_at_once(self):
        self.install(self.bundle('0.6.61', program='#!/nonexistent/interpreter\n'))
        self.assert_builtin()
        self.assertIn('0.6.61', self.status()['bad'])

    def test_a_crashing_bundle_never_leaves_the_built_in_unstartable(self):
        # Whatever the bundle does, the built-in Hearth answers by the third start, every time.
        for prog in ('#!/bin/sh\nexit 1\n', '#!/bin/sh\nkill -SEGV $$\n', '#!/bin/sh\nexit 0\n'):
            shutil.rmtree(self.root, ignore_errors=True)
            self.install(self.bundle('0.6.61', program=prog))
            self.start()
            self.start()
            self.assert_builtin()


if __name__ == '__main__':
    unittest.main()
