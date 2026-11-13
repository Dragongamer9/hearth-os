"""The app menus' search ranking (hosts/linux/desktop/shell/menu-search.js, used by Launcher.qml for every menu style):
the owner asked for a better search (Nov 12). Runs the library in node with a typical Hearth app list."""
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SHELL = Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
NODE = shutil.which('node')
JS = r'''
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'utf8').replace('.pragma library', '');
const S = new Function(src + '; return { appScore, itemScore, rank, ALIASES };')();
const job = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
const out = {};
for (const q of job.queries) out[q] = S.rank(job.apps, a => S.appScore(q, a, job.uses)).map(a => a.name);
out.__pages = S.rank(job.pages, p => S.itemScore('wifi', p.name, p.words)).map(p => p.name);
console.log(JSON.stringify(out));
'''
APPS = [
    {'id': 'hearth-files', 'name': 'Files', 'genericName': 'File Manager', 'keywords': ['folders', 'explorer'], 'execString': 'hearth-files %U'},
    {'id': 'hearth-terminal', 'name': 'Terminal', 'genericName': 'Terminal emulator', 'keywords': ['shell', 'prompt'], 'execString': 'hearth-terminal'},
    {'id': 'hearth-tasks', 'name': 'Task Manager', 'genericName': 'System monitor', 'keywords': ['processes'], 'execString': 'hearth-tasks'},
    {'id': 'hearth-resmon', 'name': 'Resource Monitor', 'genericName': '', 'keywords': [], 'execString': 'hearth-resmon'},
    {'id': 'hearth-text', 'name': 'Text Editor', 'genericName': 'Text editor', 'keywords': ['notepad'], 'execString': 'hearth-text'},
    {'id': 'hearth-calculator', 'name': 'Calculator', 'genericName': '', 'keywords': [], 'execString': 'hearth-calculator'},
    {'id': 'org.mozilla.firefox', 'name': 'Firefox', 'genericName': 'Web Browser', 'keywords': [], 'execString': '/usr/bin/flatpak run --branch=stable org.mozilla.firefox @@u %u @@'},
    {'id': 'steam', 'name': 'Steam', 'genericName': 'Game store', 'keywords': [], 'execString': 'steam %U'},
    {'id': 'hearth-settings', 'name': 'Settings', 'genericName': 'Control panel', 'keywords': [], 'execString': 'hearth-settings'},
    {'id': 'hearth-capture', 'name': 'Capture', 'genericName': 'Screenshot tool', 'keywords': [], 'execString': 'spectacle'},
    {'id': 'hearth-photos', 'name': 'Photos', 'genericName': 'Image viewer', 'keywords': [], 'execString': 'hearth-photos'},
    {'id': 'org.kde.kate', 'name': 'Café Notes', 'genericName': '', 'keywords': [], 'execString': 'kate'},
]
QUERIES = ['files', 'fil', 'explorer', 'cmd', 'tm', 'task', 'manager', 'tsak', 'calculater', 'notepad', 'konsole',
           'firefox', 'browser', 'cafe', 'xyzzy', 'control panel', 'man task', 'steam', 'snip']


@unittest.skipUnless(NODE, 'node is needed to run menu-search.js')
class Ranking(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        d = tempfile.mkdtemp(prefix='hearth-menu-search-')
        with open(os.path.join(d, 'run.js'), 'w') as f:
            f.write(JS)
        pages = [{'name': 'Wi-Fi', 'words': 'wireless network internet'}, {'name': 'Network & internet', 'words': 'wifi ethernet'},
                 {'name': 'Sound', 'words': 'volume'}]
        with open(os.path.join(d, 'job.json'), 'w') as f:
            json.dump({'apps': APPS, 'queries': QUERIES, 'uses': {'steam': 8}, 'pages': pages}, f)
        r = subprocess.run([NODE, os.path.join(d, 'run.js'), str(SHELL / 'menu-search.js'), os.path.join(d, 'job.json')],
                           capture_output=True, text=True, timeout=60)
        shutil.rmtree(d, ignore_errors=True)
        if r.returncode != 0:
            raise AssertionError(r.stderr)
        cls.r = json.loads(r.stdout)

    def first(self, q):
        return (self.r[q] or [None])[0]

    def test_names(self):
        self.assertEqual(self.first('files'), 'Files')
        self.assertEqual(self.first('fil'), 'Files')
        self.assertEqual(self.first('task'), 'Task Manager')
        self.assertEqual(self.first('manager'), 'Task Manager')       # a word inside the name
        self.assertEqual(self.first('man task'), 'Task Manager')      # words in any order
        self.assertEqual(self.first('steam'), 'Steam')

    def test_initials_typos_and_accents(self):
        self.assertEqual(self.first('tm'), 'Task Manager')
        self.assertEqual(self.first('tsak'), 'Task Manager')          # two letters swapped
        self.assertEqual(self.first('calculater'), 'Calculator')      # one letter wrong
        self.assertEqual(self.first('cafe'), 'Café Notes')

    def test_windows_names_and_programs(self):
        self.assertEqual(self.first('explorer'), 'Files')
        self.assertEqual(self.first('cmd'), 'Terminal')
        self.assertEqual(self.first('notepad'), 'Text Editor')
        self.assertEqual(self.first('control panel'), 'Settings')
        self.assertEqual(self.first('konsole'), 'Terminal')           # an alias for the terminal
        self.assertEqual(self.first('snip'), 'Capture')
        self.assertEqual(self.first('browser'), 'Firefox')

    def test_nothing_unrelated(self):
        self.assertEqual(self.r['xyzzy'], [])
        self.assertNotIn('Steam', self.r['files'])

    def test_settings_pages(self):
        self.assertEqual(self.r['__pages'][:2], ['Wi-Fi', 'Network & internet'])


if __name__ == '__main__':
    unittest.main()
