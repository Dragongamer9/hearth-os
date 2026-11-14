"""Main drive almost full (1.11): the desktop's pop-up, the 2 GB emergency reserve freed when the drive is nearly
full (hearth-diskguard, never failing the start-up) and system updates that stop before filling the drive."""
import importlib.machinery
import importlib.util
import os
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts/linux/desktop/shell')
GUARD = os.path.join(ROOT, 'os/system_files/usr/libexec/nimbus/hearth-diskguard')
GB = 1000 ** 3


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, path)
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


dg = load(GUARD, 'hearth_diskguard')


class Reserve(unittest.TestCase):
    def test_decide(self):
        self.assertEqual(dg.decide(1 * GB, 2 * GB), 'release')
        self.assertEqual(dg.decide(5 * GB, 2 * GB), 'keep')
        self.assertEqual(dg.decide(31 * GB, 0), 'keep')          # 31 - 2 isn't over 30
        self.assertEqual(dg.decide(40 * GB, 0), 'make')
        self.assertEqual(dg.decide(1 * GB, 0), 'keep')

    def test_make_and_release(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'sub', 'space-reserve')
            old = dg.RESERVE_BYTES
            dg.RESERVE_BYTES = 1 << 20
            try:
                self.assertTrue(dg.make(p))
            finally:
                dg.RESERVE_BYTES = old
            self.assertGreaterEqual(os.path.getsize(p), 1 << 20)
            self.assertFalse(os.path.exists(p + '.new'))
            self.assertTrue(dg.release(p))
            self.assertFalse(dg.release(p))

    def test_status_json(self):
        s = dg.status()
        for k in ('size', 'free', 'pct', 'reserve', 'freeGb'):
            self.assertIn(k, s)

    def test_service_never_blocks_the_start(self):
        unit = open(os.path.join(ROOT, 'os/system_files/usr/lib/systemd/system/hearth-diskguard.service')).read()
        self.assertIn('ExecStart=-/usr/bin/python3 /usr/libexec/nimbus/hearth-diskguard boot', unit)
        self.assertIn('TimeoutStartSec=20', unit)
        self.assertNotIn('Requires=', unit.replace('RequiresMountsFor', ''))
        src = open(GUARD).read()
        self.assertIn('except Exception as e:      # never stops the PC from starting', src)

    def test_enabled_and_allowed(self):
        build = open(os.path.join(ROOT, 'os-bases/peppermint/build.sh')).read()
        self.assertIn('hearth-diskguard.service', build)
        rules = open(os.path.join(ROOT, 'os/system_files/usr/share/polkit-1/rules.d/49-hearth-memory.rules')).read()
        self.assertIn('/usr/libexec/nimbus/hearth-diskguard', rules)


class Popup(unittest.TestCase):
    QML = open(os.path.join(SHELL, 'SpaceWatch.qml')).read()

    def test_the_message(self):
        self.assertIn("If the drive Hearth OS starts from fills up, your PC might not start properly.", self.QML)
        self.assertIn('dialog.confirm(', self.QML)
        self.assertIn('"Free up space"', self.QML)

    def test_levels(self):
        self.assertIn('if (free < 3e9) return 2;', self.QML)
        self.assertIn('if ((warnPct > 0 && pct >= warnPct) || free < 10e9) return 1;', self.QML)
        self.assertIn('property int warnPct: 90', self.QML)          # Warn when C: is over (Task Manager › Disk, 1.12)

    def test_loaded_by_the_shell(self):
        shell = open(os.path.join(SHELL, 'shell.qml')).read()
        self.assertIn('Loader { source: "SpaceWatch.qml"', shell)
        self.assertIn('settings.open("storage")', shell)


class HearthDisks(unittest.TestCase):
    def test_disk_management_warns_too(self):
        ui = open(os.path.join(ROOT, 'hosts/hearth-disks/ui/index.html')).read()
        self.assertIn('id="fullbar"', ui)
        self.assertIn("starts from fills up, your PC might not start properly.", ui)
        self.assertIn("full >= .9 || sp.free < 10e9", ui)


class Update(unittest.TestCase):
    def test_update_stops_when_too_full(self):
        src = open(os.path.join(ROOT, 'os/system_files/usr/libexec/nimbus/hearth-admin')).read()
        self.assertIn('UPDATE_ROOM = 5 * 1000 ** 3', src)
        upd = src[src.index('def update():'):]
        self.assertLess(upd.index('room < UPDATE_ROOM'), upd.index('ensure_podman()'))


if __name__ == '__main__':
    unittest.main()
