"""hearth-steam-warm (1.14, faster switching): Steam's program files read into memory ahead of Game Mode."""
import importlib.machinery, importlib.util, os, shutil, tempfile, unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
PATH = os.path.join(HERE, '..', 'desktop', 'shell', 'hearth-steam-warm')
SESSION = os.path.join(HERE, '..', 'session', 'nimbus-session')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_steam_warm', PATH)
    spec = importlib.util.spec_from_loader('hearth_steam_warm', loader)
    m = importlib.util.module_from_spec(spec); loader.exec_module(m); return m


class Warm(unittest.TestCase):
    def setUp(self):
        self.w = load()
        self.home = tempfile.mkdtemp()
        self.root = os.path.join(self.home, '.steam', 'debian-installation')
        for d, n, size in (('ubuntu12_32', 'steam', 10), ('ubuntu12_32', 'libsteam.so', 20), ('ubuntu12_64', 'notes.txt', 5),
                           ('steamrt64', 'big.so', 30)):
            os.makedirs(os.path.join(self.root, d), exist_ok=True)
            p = os.path.join(self.root, d, n)
            with open(p, 'wb') as f:
                f.write(b'x' * size)
            if n == 'steam':
                os.chmod(p, 0o755)

    def tearDown(self):
        shutil.rmtree(self.home, ignore_errors=True)

    def test_finds_steam_and_only_its_programs(self):
        self.assertEqual(self.w.steam_root(self.home), os.path.realpath(self.root))
        paths, total = self.w.files(self.root)
        names = sorted(os.path.basename(p) for p in paths)
        self.assertEqual(names, ['big.so', 'libsteam.so', 'steam'])
        self.assertEqual(total, 60)

    def test_cap(self):
        with mock.patch.object(self.w, 'CAP_MB', 0):
            self.assertEqual(self.w.files(self.root), ([], 0))

    def test_skips_with_little_memory_or_no_steam(self):
        with mock.patch.object(self.w, 'mem_available_mb', return_value=1000), mock.patch('builtins.print') as p:
            self.w.main([])
        self.assertIn('skipped', p.call_args[0][0])
        self.assertIsNone(self.w.steam_root(tempfile.gettempdir() + '/no-such-home'))

    def test_warm_only_reads(self):
        paths, _ = self.w.files(self.root)
        before = {p: os.stat(p).st_mtime for p in paths}
        self.assertEqual(self.w.warm(paths, pause=0), len(paths))
        self.assertEqual(before, {p: os.stat(p).st_mtime for p in paths})

    def test_session_starts_it_once_low_priority_and_closes_steam_behind_for_the_desktop(self):
        s = open(SESSION).read()
        self.assertIn('nice -n 19 ionice -c 3 python3 "$SHARE/desktop/shell/hearth-steam-warm"', s)
        self.assertIn('[ ! -e "$RUN/hearth-steam-warm" ]', s)
        self.assertIn('settle "$mode" "$to"', s)
        self.assertIn('desktop|server) log "closing Steam left from $1 mode (in the background)"; steam_close_behind', s)
        self.assertIn('*) log "closing Steam left from $1 mode"; steam_close_now', s)


if __name__ == '__main__':
    unittest.main()
