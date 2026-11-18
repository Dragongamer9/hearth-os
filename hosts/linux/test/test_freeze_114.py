"""1.14's freeze (the owner's laptop and ROG Xbox Ally, Nov 17): Mantel at 70-90% of a core because a menu closed with
words still in its search kept working the search out again (SearchSources' refresh Timer was read and started inside
MenuSearch.qml's result binding: a binding loop that re-ran it for ever). And Hearth's own apps missing from Mantel's
menu under KWin (XDG_CURRENT_DESKTOP Hearth:Nimbus:KDE against the app-kinds copies' NotShowIn=KDE;GNOME;)."""
import sys
sys.dont_write_bytecode = True
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
KINDS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'hearth-app-kinds'


class SearchNeverSpins(unittest.TestCase):
    def test_lookup_starts_no_timer_inside_a_binding(self):
        t = (SHELL / 'SearchSources.qml').read_text()
        body = t[t.index('function lookup('):]
        body = body[:body.index('\n    }\n')]
        self.assertNotRegex(body, r'\.running\b|\.start\(\)', 'lookup() runs inside the menus\' result bindings')
        self.assertIn('Qt.callLater(root.refresh)', body)
        self.assertNotIn('refreshSoon', t)

    def test_menu_search_only_while_on_screen(self):
        t = (SHELL / 'MenuSearch.qml').read_text()
        m = re.search(r'readonly property var result: ([^\n]+)', t)
        self.assertIsNotNone(m)
        self.assertIn('ms.visible', m.group(1))
        self.assertIn('query.trim() !== ""', m.group(1))

    def test_launcher_query_reaches_the_classic_menu(self):
        self.assertIn('styleLoader.item.setQuery(t)', (SHELL / 'Launcher.qml').read_text())
        self.assertIn('function setQuery(t)', (SHELL / 'MenuClassic.qml').read_text())


class HearthAppsUnderKWin(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix='hearth-kinds-')
        apps = os.path.join(self.root, 'usr/share/applications')
        over = os.path.join(self.root, 'usr/share/hearth/app-kinds/applications')
        os.makedirs(apps); os.makedirs(over)
        for did, extra in (('hearth-settings', 'NotShowIn=KDE;GNOME;\n'), ('hearth-logs', 'NotShowIn=KDE;GNOME;\n'),
                           ('org.kde.dolphin', 'OnlyShowIn=KDE;\n'), ('firefox', '')):
            base = '[Desktop Entry]\nType=Application\nName=%s\nExec=true\n' % did
            pathlib.Path(apps, did + '.desktop').write_text(base)
            if extra:
                pathlib.Path(over, did + '.desktop').write_text(base + extra)
        tdir = os.path.join(self.root, 'usr/share/hearth')
        pathlib.Path(tdir, 'app-kinds.json').write_text((ROOT / 'os/system_files/usr/share/hearth/app-kinds.json').read_text())

    def hidden(self, names):
        env = dict(os.environ, XDG_DATA_DIRS='', XDG_CURRENT_DESKTOP=names)
        r = subprocess.run([sys.executable, str(KINDS), '--root', self.root, 'hidden'], env=env, capture_output=True, text=True, timeout=60)
        self.assertEqual(r.returncode, 0, r.stderr)
        return set(r.stdout.split())

    def test_mantel_on_kwin_shows_what_it_shows_on_labwc(self):
        self.assertEqual(self.hidden('Hearth:Nimbus:KDE'), self.hidden('Nimbus:wlroots'))
        self.assertNotIn('hearth-settings', self.hidden('Hearth:Nimbus:KDE'))
        self.assertNotIn('hearth-logs', self.hidden('Hearth:Nimbus:KDE'))
        self.assertIn('org.kde.dolphin', self.hidden('Hearth:Nimbus:KDE'))     # KDE's own stays KDE's, as on labwc

    def test_other_desktops_are_as_before(self):
        self.assertIn('hearth-settings', self.hidden('KDE'))
        self.assertNotIn('org.kde.dolphin', self.hidden('KDE'))
        self.assertNotIn('firefox', self.hidden('KDE'))


class SlowLockScreenTest(unittest.TestCase):
    """The laptop (Nov 17): KWin's greeter said nothing in its 8 s on a cold first start. A run still going when its time
    is up is told as slow and never kept, so the next check runs it again."""
    def test_slow_isnt_kept_as_broken(self):
        d = tempfile.mkdtemp(prefix='hearth-lock-')
        pkg = os.path.join(d, 'pkg')
        os.makedirs(os.path.join(pkg, 'contents', 'lockscreen'))
        pathlib.Path(pkg, 'contents', 'lockscreen', 'LockScreen.qml').write_text('')
        pathlib.Path(pkg, 'metadata.json').write_text('{"X-Plasma-APIVersion": "2"}')
        qt = os.path.join(d, 'qt')
        for f in ('platforms/libqwayland-generic.so', 'wayland-shell-integration/liblayer-shell.so'):
            os.makedirs(os.path.dirname(os.path.join(qt, f)), exist_ok=True)
            pathlib.Path(qt, f).write_text('')
        g = os.path.join(d, 'greeter')
        pathlib.Path(g).write_text('#!/bin/sh\nexec sleep 30\n')
        os.chmod(g, 0o755)
        run = os.path.join(d, 'run')
        os.makedirs(run)
        env = dict(os.environ, HEARTH_LOCK_PKG=pkg, HEARTH_LOCK_GREETER=g, HEARTH_LOCK_QT_PLUGINS=qt, XDG_RUNTIME_DIR=run,
                   HEARTH_LOCK_PROBE_TIME='1', PLASMA_DEFAULT_SHELL='org.hearth.mantel-lock')
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-lockcheck'), '--as-kwin'], env=env, capture_output=True, text=True, timeout=60)
        self.assertEqual(r.returncode, 1)
        self.assertIn('still starting after 1s', r.stdout)
        self.assertFalse(os.path.exists(os.path.join(run, 'hearth', 'lockcheck.json')))


if __name__ == '__main__':
    unittest.main()
