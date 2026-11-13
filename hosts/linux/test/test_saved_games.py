"""Saved games (hosts/linux/desktop/shell/hearth-saves, 1.11) with a stand-in Ludusavi: what's found, backed up and
put back, and that a backup always goes to your Saved Games folder."""
import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')
FAKE = r'''#!/usr/bin/python3
import json, sys, os
a = sys.argv[1:]
open(os.environ['LOG'], 'a').write(' '.join(a) + '\n')
games = {"Hades II": {"decision": "Processed", "files": {"/h/a.sav": {"bytes": 2048}, "/h/b.sav": {"bytes": 1024}}},
         "Celeste": {"decision": "Processed", "files": {"/h/c.sav": {"bytes": 512}}}}
if a[0] == 'restore':
    games = {"Celeste": games["Celeste"], "Old Game": {"files": {"/x": {"bytes": 10}}}}
print(json.dumps({"overall": {"processedGames": len(games), "processedBytes": 3584}, "games": games}))
'''


class Saves(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.d, 'bin'))
        f = os.path.join(self.d, 'bin', 'ludusavi')
        open(f, 'w').write(FAKE)
        os.chmod(f, 0o755)
        self.env = dict(os.environ, HOME=self.d, PATH=os.path.join(self.d, 'bin') + ':' + os.environ['PATH'], LOG=os.path.join(self.d, 'log'))

    def tearDown(self):
        shutil.rmtree(self.d)

    def run_(self, *a):
        r = subprocess.run(['python3', os.path.join(SHELL, 'hearth-saves')] + list(a), capture_output=True, text=True, env=self.env, timeout=60)
        return json.loads(r.stdout.strip().splitlines()[-1])

    def test_status_before_and_after_a_backup(self):
        s = self.run_('status')
        self.assertTrue(s['ludusavi'])
        self.assertEqual([g['name'] for g in s['games']], ['Celeste', 'Hades II'])
        self.assertFalse(any(g['backedUp'] for g in s['games']))
        b = self.run_('backup')
        self.assertEqual(b['folder'], os.path.join(self.d, 'Saved Games', 'Backups'))
        s = self.run_('status')
        names = {g['name']: g for g in s['games']}
        self.assertTrue(names['Celeste']['backedUp'])
        self.assertFalse(names['Old Game']['here'])           # only in the backup: can be put back
        log = open(self.env['LOG']).read()
        self.assertIn('backup --api --force --path ' + os.path.join(self.d, 'Saved Games', 'Backups'), log)
        self.assertIn('backup --preview --api --force', log)

    def test_without_ludusavi(self):
        os.remove(os.path.join(self.d, 'bin', 'ludusavi'))
        self.env['PATH'] = os.path.join(self.d, 'bin')
        r = subprocess.run(['/usr/bin/python3', os.path.join(SHELL, 'hearth-saves'), 'status'], capture_output=True, text=True, env=self.env)
        self.assertFalse(json.loads(r.stdout)['ludusavi'])


if __name__ == '__main__':
    unittest.main()
