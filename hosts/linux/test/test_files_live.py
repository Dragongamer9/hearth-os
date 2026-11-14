"""1.13: Hearth Files updates by itself, like File Explorer (hearth-files watch). A local folder is watched with inotify
and a burst comes out as one "changed"; a deleted folder says "gone"; a network share is looked at every few seconds;
drives plugged in or out say "mounts". Files, its Open/Save window and the desktop list again quietly."""
import importlib.machinery
import importlib.util
import io
import os
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from contextlib import redirect_stdout

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
HELPER = os.path.join(SHELL, 'hearth-files')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_files_live', HELPER)
    spec = importlib.util.spec_from_loader('hearth_files_live', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hf = load()


class Watch(unittest.TestCase):
    def test_a_burst_is_one_line_and_a_deleted_folder_says_gone(self):
        with tempfile.TemporaryDirectory() as t:
            d = os.path.join(t, 'open')
            os.mkdir(d)
            p = subprocess.Popen([sys.executable, HELPER, 'watch', d], stdout=subprocess.PIPE, text=True)
            try:
                time.sleep(0.5)
                for i in range(40):
                    with open(os.path.join(d, f'f{i}'), 'w') as f:
                        f.write('x')
                time.sleep(1.6)
                os.rename(os.path.join(d, 'f0'), os.path.join(d, 'renamed'))
                time.sleep(0.9)
                for n in os.listdir(d):
                    os.remove(os.path.join(d, n))
                os.rmdir(d)
                out, _ = p.communicate(timeout=6)
            finally:
                p.kill()
            lines = out.split()
            self.assertEqual(lines[-1], 'gone')
            self.assertGreaterEqual(lines.count('changed'), 2)
            self.assertLessEqual(lines.count('changed'), 6, lines)   # 40 files: a handful of lines, not 40

    def test_a_folder_that_is_not_there(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            hf.watch('/nonexistent/folder', alive=lambda: True)
        self.assertEqual(buf.getvalue().strip(), 'gone')

    def test_a_network_share_is_looked_at_every_few_seconds(self):
        sigs = iter([1, 1, 2, 2, 3])
        buf = io.StringIO()
        with tempfile.TemporaryDirectory() as t, redirect_stdout(buf):
            hf.watch_poll(t, interval=0, sleep=lambda s: None, sig=lambda p: next(sigs), rounds=4)
        self.assertEqual(buf.getvalue().split(), ['changed', 'changed'])

    def test_network_file_systems_are_known(self):
        for fs in ('cifs', 'nfs4', 'fuse.sshfs', 'fuse.gvfsd-fuse'):
            self.assertIn(fs, hf.REMOTE_FS)
        self.assertIn(hf.fs_type('/'), ('', 'overlay', 'ext4', 'btrfs', 'xfs', 'tmpfs', 'composefs', 'zfs', '9p', 'virtiofs', 'fuse.overlayfs'))

    def test_folder_signature_changes_with_the_folder(self):
        with tempfile.TemporaryDirectory() as t:
            a = hf.folder_sig(t)
            open(os.path.join(t, 'x'), 'w').close()
            self.assertNotEqual(a, hf.folder_sig(t))


class Qml(unittest.TestCase):
    def test_files_and_the_desktop_listen(self):
        fm = open(os.path.join(SHELL, 'FileManager.qml'), encoding='utf-8').read()
        self.assertIn('"watch", app.watchPath', fm)
        self.assertIn('"watch", "--mounts"', fm)
        self.assertIn('function refreshQuiet()', fm)
        self.assertIn('if (renaming !== "") { refreshWaiting = true; return; }', fm)   # never while renaming
        self.assertIn('grid.contentY = y1; list.contentY = y2', fm)                  # where you scrolled stays
        desk = open(os.path.join(SHELL, 'Desktop.qml'), encoding='utf-8').read()
        self.assertIn('"watch", root.watchDir', desk)
        self.assertNotIn('command: ["stat", "-c", "%Y", root.dir]', desk)


if __name__ == '__main__':
    unittest.main()
