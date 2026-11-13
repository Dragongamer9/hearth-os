"""System Restore and Reliability history (docs/SYSTEM-RESTORE.md): desktop/shell/hearth_restore.py and
hearth_reliability.py with a fake home, a fake hearth-admin and a fake journal.

Restore points: only the listed settings files are copied and put back (atomically, with .bak, never through a link,
never a file outside the list even when a point says so), damaged copies are never kept or restored, about 10 are
kept, and the point being restored is never pruned. The version part is only Settings › Updates' own
`pkexec hearth-admin rollback` (an argument list), only when the point's version is the one bootc keeps, and a
failure there puts today's settings back. Opening System Restore and Reliability history writes no settings.
"""
import sys
sys.dont_write_bytecode = True
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import time
import unittest

os.environ['XDG_STATE_HOME'] = tempfile.mkdtemp()        # hearth_settings_io's notices stay out of your home
os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)

REPO = pathlib.Path(__file__).resolve().parents[3]
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))
import hearth_restore as hr          # noqa: E402
import hearth_reliability as rel     # noqa: E402
hr.MIN_FREE = 0                      # the test machine's free space doesn't matter here


class Done:
    def __init__(self, code=0, out='', err=''):
        self.returncode, self.stdout, self.stderr = code, out, err


class Home(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ)
        os.environ['HOME'] = self.tmp
        os.environ['XDG_CONFIG_HOME'] = os.path.join(self.tmp, '.config')
        os.environ['XDG_DATA_HOME'] = os.path.join(self.tmp, '.local', 'share')
        os.environ['XDG_CACHE_HOME'] = os.path.join(self.tmp, '.cache')
        os.environ['XDG_STATE_HOME'] = os.path.join(self.tmp, '.local', 'state')
        os.environ['XDG_RUNTIME_DIR'] = os.path.join(self.tmp, 'run')
        info = os.path.join(self.tmp, 'image-info.json')
        self.write(info, json.dumps({'version': '1.8.0'}))
        os.environ['HEARTH_IMAGE_INFO'] = info

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def p(self, rel_):
        return os.path.join(self.tmp, rel_)

    def write(self, path, text):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w', encoding='utf-8') as f:
            f.write(text)

    def read(self, rel_):
        with open(self.p(rel_), encoding='utf-8') as f:
            return f.read()

    def version(self, v):
        self.write(os.environ['HEARTH_IMAGE_INFO'], json.dumps({'version': v}))

    def settings(self):
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['a', 'b'], 'size': 2}))
        self.write(self.p('.config/nimbus/session.conf'), 'HEARTH_START=game\nHEARTH_STEAM_DEFAULT=1\n')
        self.write(self.p('.config/nimbus/osk.json'), json.dumps({'theme': 'dark'}))
        self.write(self.p('.config/nimbus/input/global.json'), json.dumps({'a': 1}))
        self.write(self.p('.local/share/nimbus/desktop.json'), json.dumps({'look': 'steam', 'accent': '#66c0f4'}))
        self.write(self.p('.config/hearth/drive-letters.json'), json.dumps({'C': 'root'}))
        # not settings, or not to be kept:
        self.write(self.p('.config/nimbus/oauth-clients.json'), json.dumps({'secret': 'x'}))
        self.write(self.p('.config/nimbus/osk-words.json'), json.dumps({'hello': 3}))
        self.write(self.p('.config/nimbus/desk.json.bak'), json.dumps({'old': True}))
        self.write(self.p('.config/nimbus/notes.txt'), 'not a setting')
        self.write(self.p('.local/share/nimbus/library.json'), json.dumps({'playtime': 100}))
        self.write(self.p('Documents/mine.json'), json.dumps({'mine': True}))

    def tree(self):
        """Every file under the fake home except the points and the cache, with its hash."""
        out = {}
        for root, _, files in os.walk(self.tmp):
            for n in files:
                full = os.path.join(root, n)
                if '/hearth/restore/' in full or '/.cache/' in full or full.endswith(('settings-events.log', '.lock')):
                    continue
                with open(full, 'rb') as f:
                    out[full] = hashlib.sha256(f.read()).hexdigest()
        return out


class Snapshots(Home):
    def test_only_the_listed_settings_files_are_kept(self):
        self.settings()
        pt = hr.snapshot('manual')
        self.assertEqual(sorted(pt['files']), ['config/hearth/drive-letters.json', 'config/nimbus/desk.json',
                                               'config/nimbus/input/global.json', 'config/nimbus/osk.json',
                                               'config/nimbus/session.conf', 'data/nimbus/desktop.json'])
        d = os.path.join(hr.store_dir(), pt['id'])
        self.assertEqual(os.stat(d).st_mode & 0o777, 0o700)
        self.assertEqual(os.stat(os.path.join(d, 'files', 'config/nimbus/desk.json')).st_mode & 0o777, 0o600)
        self.assertEqual([n for n in os.listdir(hr.store_dir()) if '.tmp' in n], [])     # renamed into place whole
        self.assertEqual(pt['version'], '1.8')
        self.assertEqual(pt['kind'], 'manual')

    def test_a_damaged_settings_file_is_never_kept(self):
        self.settings()
        self.write(self.p('.config/nimbus/osk.json'), '{"theme": "da')
        pt = hr.snapshot('scheduled')
        self.assertNotIn('config/nimbus/osk.json', pt['files'])
        self.assertIn('config/nimbus/osk.json', pt['skipped'])

    def test_links_and_big_files_are_skipped(self):
        self.settings()
        os.symlink(self.p('Documents/mine.json'), self.p('.config/nimbus/link.json'))
        self.write(self.p('.config/nimbus/huge.json'), json.dumps({'x': 'y' * (hr.MAX_FILE + 10)}))
        pt = hr.snapshot('manual')
        self.assertNotIn('config/nimbus/link.json', pt['files'])
        self.assertNotIn('config/nimbus/huge.json', pt['files'])

    def test_allowed_refuses_paths_outside_the_list(self):
        for bad in ('config/../.ssh/id_rsa', 'config/nimbus/../../x.json', '/etc/passwd', 'config//nimbus/x.json',
                    'config/nimbus/sub/x.json', 'data/nimbus/library.json', 'config/nimbus/oauth-clients.json',
                    'config/nimbus/desk.json.bak', 'home/Documents/mine.json', 'config/nimbus/.hidden.json', '',
                    'config/nimbus/x.json\0'):
            self.assertFalse(hr.allowed(bad), bad)
        for good in ('config/nimbus/desk.json', 'config/nimbus/session.conf', 'config/nimbus/display.sh',
                     'data/nimbus/desktop.json', 'config/labwc/rc.xml', 'config/mimeapps.list'):
            self.assertTrue(hr.allowed(good), good)

    def test_prune_keeps_ten_and_the_two_newest_updates(self):
        self.settings()
        t0 = 1_700_000_000
        ids = [hr.snapshot('update', now=t0)['id'], hr.snapshot('update', now=t0 + 60)['id'], hr.snapshot('update', now=t0 + 120)['id']]
        for i in range(12):
            hr.snapshot('scheduled', now=t0 + 1000 + i * 60)
        left = [p['id'] for p in hr.list_points()]
        self.assertEqual(len(left), 12)                  # 10 newest + the two newest updates
        self.assertNotIn(ids[0], left)
        self.assertIn(ids[1], left)
        self.assertIn(ids[2], left)

    def test_the_point_being_restored_is_never_pruned(self):
        self.settings()
        t0 = 1_700_000_000
        old = hr.snapshot('manual', now=t0)['id']
        for i in range(9):
            hr.snapshot('scheduled', now=t0 + 100 + i * 60)
        hr.snapshot('automatic', 'Before restoring', now=t0 + 5000, protect={old})
        self.assertIn(old, [p['id'] for p in hr.list_points()])

    def test_auto_makes_one_point_per_new_version_and_a_weekly_one(self):
        self.settings()
        t0 = 1_700_000_000
        made = hr.auto(now=t0)
        self.assertEqual([(p['kind'], p['title']) for p in made], [('update', 'Hearth 1.8')])
        made = hr.auto(now=t0 + 3600 * 5)
        self.assertEqual([p['kind'] for p in made], ['scheduled'])        # the first weekly one
        self.assertEqual(hr.auto(now=t0 + 86400), [])                    # nothing new
        self.version('1.9.0')
        made = hr.auto(now=t0 + 2 * 86400)
        self.assertEqual([p['title'] for p in made], ['Hearth 1.9'])
        made = hr.auto(now=t0 + 9 * 86400)
        self.assertEqual([p['kind'] for p in made], ['scheduled'])


class Changes(Home):
    def test_changes_list_settings_by_key_and_files_made_since(self):
        self.settings()
        pt = hr.snapshot('manual')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['a'], 'size': 2, 'new': 1}))
        self.write(self.p('.config/nimbus/session.conf'), 'HEARTH_START=desktop\nHEARTH_STEAM_DEFAULT=1\n')
        self.write(self.p('.config/nimbus/taskmgr.json'), json.dumps({'speed': 'high'}))
        ch, added = hr.changes(pt)
        got = {(c['file'], c['key']) for c in ch}
        self.assertEqual(got, {('nimbus/desk.json', 'pins'), ('nimbus/desk.json', 'new'), ('nimbus/session.conf', 'HEARTH_START')})
        self.assertEqual(added, ['nimbus/taskmgr.json'])

    def test_looking_writes_nothing(self):
        self.settings()
        pt = hr.snapshot('manual')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': []}))
        before = self.tree()
        hr.changes(pt)
        with unittest.mock.patch.object(hr, 'read_status', return_value=None):
            hr.overview()
        self.assertEqual(self.tree(), before)


class Restoring(Home):
    def test_settings_go_back_atomically_with_a_bak_and_files_made_since_stay(self):
        self.settings()
        pt = hr.snapshot('manual')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': []}))
        self.write(self.p('.config/nimbus/taskmgr.json'), json.dumps({'speed': 'high'}))
        os.remove(self.p('.config/nimbus/osk.json'))
        written = hr.restore_settings(pt)
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json')), {'pins': ['a', 'b'], 'size': 2})
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json.bak')), {'pins': []})     # today's kept
        self.assertEqual(json.loads(self.read('.config/nimbus/osk.json')), {'theme': 'dark'})
        self.assertEqual(json.loads(self.read('.config/nimbus/taskmgr.json')), {'speed': 'high'})  # untouched
        self.assertEqual(sorted(written), ['config/nimbus/desk.json', 'config/nimbus/osk.json'])
        self.assertEqual([n for n in os.listdir(self.p('.config/nimbus')) if n.endswith('.tmp')], [])

    def test_a_point_never_writes_outside_the_list(self):
        self.settings()
        pt = hr.snapshot('manual')
        d = os.path.join(hr.store_dir(), pt['id'])
        evil = {'config/../.bashrc': b'rm -rf ~\n', 'config/nimbus/sub/x.json': b'{}', 'data/nimbus/library.json': b'{"playtime": 0}'}
        for k, data in evil.items():
            path = os.path.normpath(os.path.join(d, 'files', k))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'wb') as f:
                f.write(data)
            pt['files'][k] = hashlib.sha256(data).hexdigest()
        with open(os.path.join(d, 'point.json'), 'w') as f:
            json.dump(pt, f)
        before = self.tree()
        hr.restore_settings(hr.get_point(pt['id']))
        self.assertEqual(self.tree(), before)
        self.assertFalse(os.path.exists(self.p('.bashrc')))
        self.assertEqual(json.loads(self.read('.local/share/nimbus/library.json')), {'playtime': 100})

    def test_a_tampered_or_damaged_copy_is_never_put_back(self):
        self.settings()
        pt = hr.snapshot('manual')
        f = os.path.join(hr.store_dir(), pt['id'], 'files', 'config/nimbus/desk.json')
        with open(f, 'w') as fh:
            fh.write('{"pins": [')                                  # cut short: the hash no longer matches
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['now']}))
        hr.restore_settings(pt)
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json')), {'pins': ['now']})

    def test_a_link_in_place_of_a_settings_file_is_left_alone(self):
        self.settings()
        pt = hr.snapshot('manual')
        os.remove(self.p('.config/nimbus/desk.json'))
        os.symlink(self.p('Documents/mine.json'), self.p('.config/nimbus/desk.json'))
        hr.restore_settings(pt)
        self.assertEqual(json.loads(self.read('Documents/mine.json')), {'mine': True})


def status(booted='1.9.0', rollback='1.8.0', staged=None, queued=False):
    return {'booted': {'version': booted}, 'rollback': {'version': rollback} if rollback else None,
            'staged': {'version': staged} if staged else None, 'rollbackQueued': queued, 'available': None}


class Versions(Home):
    def test_plan(self):
        pt = {'version': '1.8'}
        self.assertEqual(hr.plan(pt, status())['action'], 'rollback')
        self.assertEqual(hr.plan(pt, status(queued=True))['action'], 'queued')
        self.assertEqual(hr.plan(pt, status(staged='2.0.0'))['action'], 'blocked')
        self.assertEqual(hr.plan({'version': '1.9'}, status())['action'], 'none')
        self.assertEqual(hr.plan({'version': '1.7'}, status())['action'], 'missing')
        self.assertIn('no longer kept', hr.plan({'version': '1.7'}, status())['why'])
        self.assertEqual(hr.plan(pt, status(rollback=None))['action'], 'missing')
        self.assertEqual(hr.plan(pt, None)['action'], 'unknown')
        self.assertEqual(hr.plan({'version': ''}, status())['action'], 'missing')
        # the same version on both: nothing to switch
        self.assertEqual(hr.plan({'version': '1.9'}, status(rollback='1.9.0'))['action'], 'none')

    def test_commands_are_argument_lists_through_the_existing_helper(self):
        self.assertEqual(hr.rollback_command(), ['pkexec', '/usr/libexec/nimbus/hearth-admin', 'rollback'])
        self.assertEqual(hr.status_command(), ['pkexec', '/usr/libexec/nimbus/hearth-admin', 'update-status'])
        for c in (hr.rollback_command(), hr.status_command()):
            self.assertIsInstance(c, list)
            self.assertFalse(any(x in ('sh', 'bash', '-c') for x in c))
        src = (SHELL / 'hearth_restore.py').read_text()
        self.assertNotIn('shell=True', src)
        # bootc itself is never run from here: only hearth-admin's own commands.
        self.assertFalse(re.search(r"\[\s*'bootc'", src))
        self.assertNotIn("'switch'", src)

    def fake(self, rollback_code=0, rollback_out=None, st=None):
        calls = []
        admin = os.path.join(self.tmp, 'hearth-admin')
        self.write(admin, '#!/bin/sh\n')
        os.chmod(admin, 0o755)

        def run(cmd, **kw):
            calls.append(cmd)
            self.assertIsInstance(cmd, list)
            self.assertNotIn('shell', kw)
            if cmd[-1] == 'update-status':
                return Done(0, json.dumps(st or status()) + '\n')
            if cmd[-1] == 'rollback':
                return Done(rollback_code, rollback_out if rollback_out is not None else 'log: ok\n' + json.dumps(status(queued=True)) + '\n')
            raise AssertionError(cmd)
        return run, calls, admin

    def test_restore_with_the_version(self):
        self.settings()
        pt = hr.snapshot('update', now=1_700_000_000)            # made on 1.8
        self.version('1.9.0')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': []}))
        run, calls, admin = self.fake()
        r = hr.apply(pt['id'], run=run, admin=admin, now=1_700_100_000)
        self.assertTrue(r['ok'] and r['restart'])
        self.assertEqual(calls[-1], hr.rollback_command())
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json'))['pins'], ['a', 'b'])
        safety = [p for p in hr.list_points() if p['id'] == r['safety']][0]
        self.assertTrue(safety['title'].startswith('Before restoring'))
        self.assertEqual(json.loads(hr.point_file(safety, 'config/nimbus/desk.json')), {'pins': []})

    def test_a_failed_rollback_puts_todays_settings_back(self):
        self.settings()
        pt = hr.snapshot('update', now=1_700_000_000)
        self.version('1.9.0')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['today']}))
        run, calls, admin = self.fake(rollback_code=1, rollback_out='error: there is no previous version to go back to\n')
        with self.assertRaises(hr.RestoreError) as e:
            hr.apply(pt['id'], run=run, admin=admin, now=1_700_100_000)
        self.assertIn('Nothing was changed', str(e.exception))
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json')), {'pins': ['today']})

    def test_rollback_that_didnt_queue_counts_as_failed(self):
        self.settings()
        pt = hr.snapshot('update', now=1_700_000_000)
        self.version('1.9.0')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['today']}))
        run, calls, admin = self.fake(rollback_out=json.dumps(status(queued=False)) + '\n')
        with self.assertRaises(hr.RestoreError):
            hr.apply(pt['id'], run=run, admin=admin, now=1_700_100_000)
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json')), {'pins': ['today']})

    def test_blocked_changes_nothing_and_settings_only_never_runs_anything(self):
        self.settings()
        pt = hr.snapshot('update', now=1_700_000_000)
        self.version('1.9.0')
        self.write(self.p('.config/nimbus/desk.json'), json.dumps({'pins': ['today']}))
        run, calls, admin = self.fake(st=status(staged='2.0.0'))
        before = self.tree()
        with self.assertRaises(hr.RestoreError):
            hr.apply(pt['id'], run=run, admin=admin)
        self.assertEqual(self.tree(), before)
        calls.clear()
        r = hr.apply(pt['id'], settings_only=True, run=run, admin=admin, now=1_700_100_000)
        self.assertEqual(calls, [])
        self.assertFalse(r['restart'])
        self.assertEqual(json.loads(self.read('.config/nimbus/desk.json'))['pins'], ['a', 'b'])

    def test_a_version_no_longer_kept_restores_settings_only(self):
        self.settings()
        pt = hr.snapshot('update', now=1_700_000_000)
        self.version('2.0.0')
        run, calls, admin = self.fake(st=status(booted='2.0.0', rollback='1.9.0'))
        r = hr.apply(pt['id'], run=run, admin=admin, now=1_700_100_000)
        self.assertEqual(r['plan']['action'], 'missing')
        self.assertFalse(r['restart'])
        self.assertNotIn(hr.rollback_command(), calls)


# ------------------------------------------------------------------ Reliability history
def jl(**kw):
    return json.dumps(kw)


class Journal(Home):
    def test_crashes(self):
        ev = rel.crash_events([{'__REALTIME_TIMESTAMP': '1700000000000000', 'COREDUMP_COMM': 'steamwebhelper',
                                'COREDUMP_EXE': '/home/x/.steam/steamwebhelper', 'COREDUMP_SIGNAL_NAME': 'SIGSEGV',
                                'MESSAGE': 'Process 12 (steamwebhelper) of user 1000 dumped core.'},
                               {'COREDUMP_COMM': 'no-time'}])
        self.assertEqual(len(ev), 1)
        self.assertEqual(ev[0]['t'], 'Steam closed unexpectedly')
        self.assertEqual(ev[0]['cat'], 'app')
        self.assertIn('SIGSEGV', ev[0]['s'])
        self.assertEqual(ev[0]['time'], 1700000000)

    def test_failed_units_only_hearths(self):
        ev = rel.unit_events([{'__REALTIME_TIMESTAMP': '1700000000000000', 'UNIT': 'hearth-boot-guard.service', 'UNIT_RESULT': 'exit-code'},
                              {'__REALTIME_TIMESTAMP': '1700000000000000', 'UNIT': 'cups.service'},
                              {'__REALTIME_TIMESTAMP': '1700000000000000', 'USER_UNIT': 'nimbus-session.service'}])
        self.assertEqual([e['s'].split()[0] for e in ev], ['hearth-boot-guard.service', 'nimbus-session.service'])

    def test_network_drops_are_modest(self):
        def nm(t, msg):
            return {'__REALTIME_TIMESTAMP': str(int(t * 1e6)), 'MESSAGE': msg}
        t = 1_700_000_000
        ev = rel.network_events([
            nm(t, "<info> device (wlan0): state change: activated -> deactivating (reason 'supplicant-disconnect', sys-iface-state: 'managed')"),
            nm(t + 40, "<info> device (wlan0): state change: secondaries -> activated (reason 'none', sys-iface-state: 'managed')"),
            nm(t + 600, "<info> device (wlan0): state change: activated -> disconnected (reason 'supplicant-disconnect', sys-iface-state: 'managed')"),
            nm(t + 610, "<info> device (wlan0): state change: secondaries -> activated (reason 'none', sys-iface-state: 'managed')"),
            nm(t + 9000, "<info> device (wlan0): state change: activated -> deactivating (reason 'sleeping', sys-iface-state: 'managed')"),
            nm(t + 9100, "<info> device (wlan0): state change: secondaries -> activated (reason 'none', sys-iface-state: 'managed')"),
        ])
        self.assertEqual(len(ev), 1)                     # two drops in the hour are one warning; sleep isn't one
        self.assertEqual(ev[0]['t'], 'Wi-Fi dropped 2 times')
        self.assertEqual(ev[0]['cat'], 'warn')

    def test_session_log_fallbacks(self):
        text = ('2026-11-08 21:15:02 start: steam game mode: Steam ended right away 3 times; Desktop Mode instead\n'
                '2026-11-08 21:16:00 desktop: plasma asked for on the login screen; Mantel is the only desktop now\n'
                '2026-11-08 21:17:00 started Steam\n'
                "2026-11-08 21:18:00 gamescope couldn't start with the chosen screen mode (-W 2560); back to the screen's own mode\n")
        ev = rel.session_events(text)
        self.assertEqual(len(ev), 2)
        self.assertEqual(ev[0]['t'], "Steam game mode didn't start, Desktop Mode opened instead")
        self.assertEqual(ev[0]['s'], 'Steam ended right away 3 times')
        self.assertTrue(all(e['cat'] == 'hearth' for e in ev))

    def test_reason_files_skip_what_the_log_already_has(self):
        f = os.path.join(self.tmp, 'run', 'hearth-game-mode-failed')
        self.write(f, 'gamescope stopped 3 times in 2 minutes\n')
        t = os.stat(f).st_mtime
        self.assertEqual(rel.reason_events([f], near=[t + 30]), [])
        ev = rel.reason_events([f, os.path.join(self.tmp, 'missing')])
        self.assertEqual(ev[0]['t'], "Hearth Game Mode didn't start")

    def test_restore_points_are_updates_and_changes(self):
        ev = rel.restore_events([{'id': 'a', 'created': 10, 'kind': 'update', 'title': 'Hearth 1.9'},
                                 {'id': 'b', 'created': 5, 'kind': 'update', 'title': 'Before updating to Hearth 1.9'},
                                 {'id': 'c', 'created': 6, 'kind': 'automatic', 'title': 'Before restoring to Nov 8, 21:14'}])
        self.assertEqual([e['t'] for e in ev], ['Hearth updated to 1.9', 'Restore point made', 'System Restore was used'])
        self.assertTrue(all(e['cat'] == 'info' for e in ev))

    def test_score_dips_and_recovers(self):
        s = rel.score_days([{}, {'app': 1}, {'hearth': 1, 'app': 1}, {}, {}, {}])
        self.assertEqual(s[0], 10)
        self.assertLess(s[2], s[1])
        self.assertGreater(s[5], s[3])
        self.assertTrue(all(1 <= x <= 10 for x in s))

    def test_summary_reads_the_journal_once_then_only_today(self):
        now = time.mktime(time.strptime('2026-11-08 22:00:00', '%Y-%m-%d %H:%M:%S'))
        crash = jl(__REALTIME_TIMESTAMP=str(int((now - 2 * 86400) * 1e6)), COREDUMP_COMM='steam', COREDUMP_SIGNAL_NAME='SIGABRT')
        calls = []

        def run(cmd, **kw):
            calls.append(cmd)
            self.assertIsInstance(cmd, list)
            if any(c == 'MESSAGE_ID=' + rel.COREDUMP_ID for c in cmd):
                return Done(0, crash + '\n')
            return Done(0, '')
        src = dict(points=[], session_text='', reasons=[])
        s = rel.summary(14, now=now, run=run, **src)
        self.assertEqual(len(s['rows']), 14)
        self.assertEqual(s['rows'][-1]['date'], '2026-11-08')
        self.assertEqual(s['rows'][-3]['counts']['app'], 1)
        self.assertLess(s['rows'][-3]['score'], 10)
        first_since = [c for c in calls[0] if c.startswith('--since=@')][0]
        self.assertEqual(int(first_since[9:]), int(rel.day_start('2026-10-26')))
        self.assertTrue(os.path.exists(rel.cache_path()))
        calls.clear()
        s2 = rel.summary(14, now=now, run=run, **src)
        self.assertEqual(int([c for c in calls[0] if c.startswith('--since=@')][0][9:]), int(rel.day_start('2026-11-08')))
        self.assertEqual(s2['rows'][-3]['counts']['app'], 1)                      # from the cache
        w = rel.summary(14, weeks=True, now=now, run=run, **src)
        self.assertEqual(len(w['rows']), 14)
        self.assertEqual(w['unit'], 'week')

    def test_summary_writes_no_settings(self):
        self.settings()
        before = self.tree()
        rel.summary(14, run=lambda cmd, **kw: Done(0, ''), points=[], session_text='', reasons=[])
        self.assertEqual(self.tree(), before)

    def test_grep_falls_back_without_pcre(self):
        calls = []

        def run(cmd, **kw):
            calls.append(cmd)
            return Done(1, '') if any(c.startswith('--grep') for c in cmd) else Done(0, jl(MESSAGE='x') + '\n')
        self.assertEqual(len(rel.journal(['SYSLOG_IDENTIFIER=NetworkManager'], 0, run, extra=['--grep=state change: '])), 1)
        self.assertEqual(len(calls), 2)


# ------------------------------------------------------------------ how it's wired
class Wiring(unittest.TestCase):
    def read(self, rel_):
        return (REPO / rel_).read_text(encoding='utf-8')

    def test_shell_loads_both_separately_and_pad_gives_them_the_controller(self):
        shell = self.read('hosts/linux/desktop/shell/shell.qml')
        self.assertIn('Loader { source: "Restore.qml" }', shell)
        self.assertIn('Loader { source: "Reliability.qml" }', shell)
        pad = self.read('hosts/linux/desktop/shell/Pad.qml')
        self.assertIn('"System Restore"', pad)
        self.assertIn('"Reliability history"', pad)
        self.assertIn('target: "restore"', self.read('hosts/linux/desktop/shell/Restore.qml'))
        self.assertIn('target: "reliability"', self.read('hosts/linux/desktop/shell/Reliability.qml'))

    def test_settings_updates_makes_a_point_before_updating_and_opens_both(self):
        q = self.read('hosts/linux/desktop/shell/SetUpdates.qml')
        start = q[q.index('function start(cmd)'):]
        start = start[:start.index('admin.running = true')]
        self.assertLess(start.index('restorePoint("update"'), start.index('admin.command'))
        self.assertIn('Quickshell.execDetached(["python3", restoreHelper', q)        # can't hold up the update
        self.assertIn('"call", "restore", "open"', q)
        self.assertIn('"call", "reliability", "open"', q)

    def test_the_views_never_run_bootc_or_write_settings(self):
        for f in ('RestoreView.qml', 'ReliabilityView.qml', 'Restore.qml', 'Reliability.qml'):
            q = self.read('hosts/linux/desktop/shell/' + f)
            self.assertNotIn('"bootc"', q, f)
            self.assertNotIn('libexec/nimbus/hearth-admin', q, f)
            self.assertNotIn('"pkexec"', q, f)
            self.assertNotIn('SettingsWriter', q, f)
            self.assertNotIn('setText(', q, f)
        rv = self.read('hosts/linux/desktop/shell/ReliabilityView.qml')
        self.assertIn('"hearth-send-logs"', rv)
        self.assertNotIn('"apply"', rv)

    def test_launchers_menu_entries_and_a_user_timer(self):
        for n in ('restore', 'reliability'):
            b = REPO / 'os/system_files/usr/bin' / ('hearth-' + n)
            self.assertTrue(os.access(b, os.X_OK), b)
            self.assertIn('call %s open' % n, b.read_text())
            d = (REPO / 'os/system_files/usr/share/applications' / ('hearth-%s.desktop' % n)).read_text()
            self.assertIn('Exec=/usr/bin/hearth-' + n, d)
            self.assertIn('OnlyShowIn=Nimbus;', d)
            self.assertTrue((REPO / 'branding/icons/apps' / ('hearth-%s.svg' % n)).exists())
        t = self.read('os/system_files/usr/lib/systemd/user/hearth-restore-points.timer')
        self.assertIn('WantedBy=timers.target', t)
        s = self.read('os/system_files/usr/lib/systemd/user/hearth-restore-points.service')
        self.assertIn('Type=oneshot', s)
        self.assertNotIn('WantedBy', s)                   # never part of starting the session
        self.assertTrue(os.path.islink(REPO / 'os/system_files/usr/lib/systemd/user/timers.target.wants/hearth-restore-points.timer'))


import unittest.mock  # noqa: E402

if __name__ == '__main__':
    unittest.main()
