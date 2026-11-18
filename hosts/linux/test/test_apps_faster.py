"""1.15 "Hearth's own apps made faster": the measuring tools (tools/perf-measure.py, tools/perf-breakdown.py,
apps-speed.yml) and the changes that make the shell smaller and its apps open sooner, kept as they are."""
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def read(name):
    with open(os.path.join(SHELL, name), encoding='utf-8') as f:
        return f.read()


class Tools(unittest.TestCase):
    def test_breakdown_finds_the_parts_and_each_variant_stays_balanced(self):
        pb = load('pb', os.path.join(ROOT, 'tools', 'perf-breakdown.py'))
        text = read('shell.qml')
        parts = pb.parts(text)
        names = [p[0] for p in parts]
        for want in ('Wallpaper', 'Desktop', 'Launcher', 'LazyLoader (filesLoader)', 'SettingsApp (settings)', 'Loader Clipboard.qml'):
            self.assertIn(want, names)
        for p in parts:
            v = pb.without(text, p)
            self.assertEqual(sum(pb.braces(l) for l in v.split('\n')), 0, p)
            self.assertNotEqual(v, text)

    def test_braces_ignore_strings_and_comments(self):
        pb = load('pb', os.path.join(ROOT, 'tools', 'perf-breakdown.py'))
        self.assertEqual(pb.braces('Item { text: "}{" // }'), 1)
        self.assertEqual(pb.braces("x: '{'; }"), -1)

    def test_compare_reads_both_files(self):
        pm = load('pm', os.path.join(ROOT, 'tools', 'perf-measure.py'))
        d = tempfile.mkdtemp()
        a, b = os.path.join(d, 'a.json'), os.path.join(d, 'b.json')
        for path, mb, first in ((a, 280.0, 0.4), (b, 200.0, 0.2)):
            with open(path, 'w') as f:
                json.dump({'idle': {'quickshell shell': {'cpu': 1.0, 'mb': mb, 'n': 1}}, 'IPC': {'ipc': 0.15},
                           'Hearth Files (warm)': {'first': first, 'ready': first + 0.1, 'mbBefore': mb, 'mbAfter': mb + 10}}, f)
        table, notes = pm.compare(a, b)
        self.assertIn('280.0 MB | 200.0 MB', table)
        self.assertTrue(any('Hearth Files (warm)' in n for n in notes))
        self.assertIn('An IPC call alone', table)



class Wallpaper(unittest.TestCase):
    def test_moving_wallpapers_are_made_only_while_shown(self):
        w = read('Wallpaper.qml')
        for ident, cond in (('lava', 'root.anim === "lava"'), ('aurora', 'root.anim === "aurora"'), ('tide', 'root.anim === "tide"'),
                            ('ridgesWall', 'root.anim === "ridges"'), ('motes', 'root.anim === "motes"'), ('fireside', '!root.custom && !root.anim')):
            m = re.search(r'Loader \{\s*anchors\.fill: parent\s*active: (.+)\n\s*sourceComponent: Component \{\s*Item \{\s*id: %s\b' % ident, w)
            self.assertIsNotNone(m, ident)
            self.assertEqual(m.group(1).strip(), cond, ident)

    def test_your_picture_is_read_at_the_screens_size(self):
        w = read('Wallpaper.qml')
        for ident in ('chosen', 'jpg', 'png'):
            block = w[w.index('id: %s' % ident):]
            block = block[:block.index('\n    }')]
            self.assertIn('sourceSize: root.wallSize', block, ident)
            self.assertIn('root.sized', block, ident)
        self.assertIn('readonly property size wallSize: Qt.size(Math.ceil(root.width * dpr), Math.ceil(root.height * dpr))', w)


class FilesOpen(unittest.TestCase):
    def test_warm_open_reads_quietly_and_once(self):
        f = read('FileManager.qml')
        op = f[f.index('    function open(p) {'):f.index('    function close() {')]
        self.assertIn('const warmOpen = !!ui && prefsLoaded && !picking;', op)
        self.assertIn('else if (warmOpen && !onHome && !onSpecial && !listing.error) refreshQuiet();', op)
        self.assertIn('if (warmOpen) Qt.callLater(refreshOpened);', op)
        # A cold open still reads all three on their own, as before.
        self.assertRegex(op, r'else \{\s*if \(prefsLoaded\) refreshCPrefs\(\);\s*refreshPlaces\(\);\s*loadTemplates\(\);')
        # Falls back to the three calls when the combined one fails.
        self.assertIn('if (!r || err) { refreshCPrefs(); refreshPlaces(); loadTemplates(); return; }', f)
        self.assertIn('if (recentBusy) return;', f)
        self.assertIn('running: app.shown || app.warm', f)

    def test_opened_writes_nothing_places_doesnt(self):
        # `opened` is prefs + places + templates in one call: it may keep only what `places` alone already keeps
        # (the drives' letters, state in ~/.config/hearth/drive-letters.json), never a setting.
        def run(cmd):
            h = tempfile.mkdtemp()
            env = dict(os.environ, HOME=h, XDG_CONFIG_HOME=os.path.join(h, '.config'), XDG_DATA_HOME=os.path.join(h, '.local', 'share'))
            r = subprocess.run([sys.executable, os.path.join(SHELL, 'hearth-files'), cmd], capture_output=True, text=True, env=env, timeout=60)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            made = set(os.path.relpath(os.path.join(dp, n), h) for dp, ds, fs in os.walk(h) for n in fs)
            return json.loads(r.stdout), made
        d, made = run('opened')
        _, by_places = run('places')
        self.assertIn('places', d['places'])
        self.assertIn('templates', d['templates'])
        self.assertIsInstance(d['prefs'], dict)
        self.assertTrue(made <= by_places | {p + '.bak' for p in by_places}, made - by_places)


class SettingsWarm(unittest.TestCase):
    def test_window_kept_but_page_let_go(self):
        a = read('SettingsApp.qml')
        self.assertIn('active: app.shown || app.warm', a)
        self.assertIn('visible: app.shown', a)
        self.assertIn('interval: 180000', a)
        # Kept only with a frame that can let its page go; the plain fallback list goes at once.
        self.assertIn('if (f && typeof f.park === "function") { f.park(); warm = true; warmTimer.restart(); }', a)
        self.assertIn('else warm = false;', a)
        for name in ('SettingsFrame.qml', 'SettingsFrameB.qml'):
            f = read(name)
            self.assertIn('function park() { loader.source = "";', f, name)
            self.assertIn('function unpark() { if (String(loader.source) !== "") return;', f, name)
            load = f[f.index('    function load() {'):]
            self.assertIn('if (!frame.app.shown) return;', load.split('\n')[1], name)



class MadeWhenNeeded(unittest.TestCase):
    def test_cards_make_their_window_only_while_shown(self):
        for name in ('LiveWelcome.qml', 'LeaveConfirm.qml', 'DesktopNotice.qml'):
            q = read(name)
            self.assertRegex(q, r'\nScope \{\n    id: root\n', name)
            self.assertIn('    LazyLoader {\n        active: root.shown\n        PanelWindow {\n            visible: shown', q, name)
            self.assertNotIn('\nPanelWindow {', q, name)
        # Log out? still says it can ask (Leave.qml logs out at once only when it can't).
        self.assertIn('Component.onCompleted: Leave.canAsk = true', read('LeaveConfirm.qml'))

    def test_files_made_on_first_use(self):
        s = read('shell.qml')
        self.assertIn('LazyLoader { id: filesLoader; FileManager {} }', s)
        self.assertNotIn('FileManager { id: files }', s)
        self.assertIn('FilesIpc { get: () => files.fm(); peek: () => filesLoader.item }', s)
        proxy = s[s.index('        id: files\n'):s.index('    FilesIpc {')]
        for fn in ('function open(p)', 'function close()', 'function toggle()', 'function properties(paths)', 'readonly property bool shown'):
            self.assertIn(fn, proxy)
        # Closing Files that was never opened doesn't make it.
        self.assertIn('function close() { if (filesLoader.item) filesLoader.item.close(); }', proxy)
        # Everything the shell's other parts call on `files` is on the stand-in.
        used = set()
        for f in os.listdir(SHELL):
            if f.endswith('.qml') and f not in ('FileManager.qml', 'picker.qml') and not f.startswith('Files'):
                used |= set(re.findall(r'\b(?:filesApp|wt\.files|files)\.(open|close|toggle|properties|shown|fm)\b', read(f)))
                for m in re.findall(r'\b(?:filesApp|wt\.files)\.(\w+)', read(f)):
                    used.add(m)
        self.assertTrue(used <= {'open', 'close', 'toggle', 'properties', 'shown', 'fm'}, used)


if __name__ == '__main__':
    unittest.main()
