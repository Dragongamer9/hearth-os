"""Hearth's typing as KWin's input method on the Fedora edition (os-bases/anatase: hearth-kwin-im-launch,
hearth-kwin-im-default, the kwinim build stage). Red flags: an update must never leave the desktop without a way to
start (KWin and Plasma come up, with KDE's keyboard when Hearth's typing can't run), and nobody's own KDE setting is
ever written (only the system's /etc/xdg/kwinrc gets the default, merged)."""
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
LIBEXEC = ROOT / 'os-bases' / 'anatase' / 'system_files' / 'usr' / 'libexec' / 'hearth'
LAUNCH = LIBEXEC / 'hearth-kwin-im-launch'
DEFAULT = LIBEXEC / 'hearth-kwin-im-default'


class Launcher(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-kwin-im-')
        self.run_dir = os.path.join(self.tmp, 'run')
        os.makedirs(self.run_dir, mode=0o700)
        self.kde_ran = os.path.join(self.tmp, 'kde-ran')
        kde = os.path.join(self.tmp, 'plasma-keyboard')
        with open(kde, 'w') as f:
            f.write('#!/bin/sh\necho "$*" > %s\n' % self.kde_ran)
        os.chmod(kde, 0o755)
        self.entry = os.path.join(self.tmp, 'org.kde.plasma.keyboard.desktop')
        with open(self.entry, 'w') as f:
            f.write('[Desktop Entry]\nName=Plasma Keyboard\nExec=%s %%u\nX-KDE-Wayland-VirtualKeyboard=true\n' % kde)
        self.im = os.path.join(self.tmp, 'hearth-kwin-im')

    def fake_im(self, body):
        with open(self.im, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(self.im, 0o755)

    def launch(self, quick='15'):
        env = {'PATH': '/usr/bin:/bin', 'HOME': self.tmp, 'XDG_RUNTIME_DIR': self.run_dir, 'HEARTH_KWIN_IM': self.im,
               'HEARTH_KDE_IM_ENTRY': self.entry, 'HEARTH_KWIN_IM_QUICK': quick}
        return subprocess.run(['sh', str(LAUNCH)], env=env, capture_output=True, text=True, timeout=30)

    def mark(self):
        return os.path.join(self.run_dir, 'hearth', 'kwin-im-failed')

    def test_missing_runs_kdes_keyboard(self):
        r = self.launch()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(os.path.exists(self.kde_ran), r.stderr)
        self.assertEqual(pathlib.Path(self.kde_ran).read_text().strip(), '')   # %u dropped

    def test_crash_at_once_then_kde_next_time(self):
        # Crashed at once (or no input-method protocol: exit 2): marked, and it ends like a crash so KWin starts it
        # again; the next start is KDE's keyboard.
        for body in ('kill -SEGV $$', 'exit 2'):
            if os.path.exists(self.mark()):
                os.remove(self.mark())
            self.fake_im(body)
            r = self.launch()
            self.assertEqual(r.returncode, -9, (body, r.returncode, r.stderr))
            self.assertTrue(os.path.exists(self.mark()), body)
            self.assertFalse(os.path.exists(self.kde_ran))
            r = self.launch()
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertTrue(os.path.exists(self.kde_ran), body)
            os.remove(self.kde_ran)

    def test_running_one_is_stopped_with_it(self):
        self.fake_im('trap "echo stopped > %s/stopped; exit 0" TERM; while :; do sleep 0.1; done' % self.tmp)
        env = {'PATH': '/usr/bin:/bin', 'HOME': self.tmp, 'XDG_RUNTIME_DIR': self.run_dir, 'HEARTH_KWIN_IM': self.im,
               'HEARTH_KDE_IM_ENTRY': self.entry}
        p = subprocess.Popen(['sh', str(LAUNCH)], env=env)
        import time
        time.sleep(1.0)
        p.terminate()
        self.assertEqual(p.wait(10), 0)
        time.sleep(0.3)
        self.assertTrue(os.path.exists(os.path.join(self.tmp, 'stopped')))
        self.assertFalse(os.path.exists(self.mark()))

    def test_no_kde_keyboard_either_ends_quietly(self):
        os.remove(self.entry)
        r = self.launch()
        self.assertEqual(r.returncode, 0)

    def test_writes_no_settings(self):
        self.fake_im('exit 2')
        self.launch()
        self.launch()
        made = [os.path.relpath(os.path.join(d, f), self.tmp) for d, _, fs in os.walk(self.tmp) for f in fs]
        self.assertFalse([m for m in made if m.startswith('.config')], made)


class SystemDefault(unittest.TestCase):
    def run_on(self, text):
        d = tempfile.mkdtemp(prefix='hearth-kwinrc-')
        p = os.path.join(d, 'kwinrc')
        if text is not None:
            with open(p, 'w') as f:
                f.write(text)
        r = subprocess.run(['python3', str(DEFAULT), p], capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        return pathlib.Path(p).read_text(), p

    def test_new_file(self):
        out, _ = self.run_on(None)
        self.assertIn('[Wayland]\nInputMethod=/usr/share/applications/hearth-kwin-im.desktop\nVirtualKeyboardEnabled=true\n', out)

    def test_merged_keeps_everything_else(self):
        out, p = self.run_on('[Plugins]\nblurEnabled=true\n\n[Wayland]\nFoo=1\n\n[Xwayland]\nScale=1\n')
        self.assertIn('[Plugins]\nblurEnabled=true\n', out)
        self.assertIn('[Wayland]\nFoo=1\nInputMethod=/usr/share/applications/hearth-kwin-im.desktop\nVirtualKeyboardEnabled=true\n', out)
        self.assertIn('[Xwayland]\nScale=1', out)
        self.assertTrue(os.path.exists(p + '.bak'))

    def test_an_input_method_already_there_is_left(self):
        before = '[Wayland]\nInputMethod[$e]=/usr/share/applications/org.kde.plasma.keyboard.desktop\nVirtualKeyboardEnabled=false\n'
        out, _ = self.run_on(before)
        self.assertEqual(out, before)

    def test_only_the_system_file_in_the_image_build(self):
        build = (ROOT / 'os-bases' / 'anatase' / 'build.sh').read_text()
        self.assertIn('hearth-kwin-im-default /etc/xdg/kwinrc', build)
        self.assertNotIn('.config/kwinrc', DEFAULT.read_text().split('"""', 2)[2])
        self.assertIn('Exec=/usr/libexec/hearth/hearth-kwin-im-launch', build)
        cf = (ROOT / 'os-bases' / 'anatase' / 'Containerfile').read_text()
        self.assertIn('FROM ${BASE_IMAGE} AS kwinim', cf)
        self.assertIn("|| echo \"hearth-kwin-im wasn't built", cf)


if __name__ == '__main__':
    unittest.main()
