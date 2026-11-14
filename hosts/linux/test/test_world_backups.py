"""1.13: Server mode › World backups for the game servers: nightly and before every update, kept for a set number of
days (the newest always stays), Roll back (today's world kept first), the server stopped cleanly and started again."""
import importlib.machinery
import importlib.util
import os
import tempfile
import time
import unittest

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_server_apps_worlds', os.path.join(SHELL, 'hearth-server-apps'))
    spec = importlib.util.spec_from_loader('hearth_server_apps_worlds', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


sa = load()


class Worlds(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.saved = (sa.DATA_ROOT, sa.SETTINGS, sa.STATE, sa.UNIT_DIR, sa.cmd)
        sa.DATA_ROOT = os.path.join(self.t.name, 'Server apps')
        sa.SETTINGS = os.path.join(self.t.name, 'server-apps.json')
        sa.STATE = os.path.join(self.t.name, 'state.json')
        sa.UNIT_DIR = os.path.join(self.t.name, 'units')
        self.calls = []
        self.running = True
        real = self.saved[4]

        def fake(args, timeout=60, env=None):
            self.calls.append(args)
            if args[:2] == ['systemctl', '--user'] and args[2] == 'is-active':
                return 0, 'active\n' if self.running else 'inactive\n'
            if args[:2] == ['systemctl', '--user'] and args[2] in ('stop', 'start'):
                self.running = args[2] == 'start'
                return 0, ''
            if args[:2] == ['podman', 'unshare']:
                return real(args[2:], timeout, env)        # tar / find as ourselves in the test
            if args[:2] == ['systemctl', '--user']:
                return 0, ''
            return real(args, timeout, env)
        sa.cmd = fake
        world = os.path.join(sa.DATA_ROOT, 'Minecraft', 'world')
        os.makedirs(os.path.join(world, 'world'))
        with open(os.path.join(world, 'world', 'level.dat'), 'w') as f:
            f.write('day 1')

    def tearDown(self):
        sa.DATA_ROOT, sa.SETTINGS, sa.STATE, sa.UNIT_DIR, sa.cmd = self.saved
        self.t.cleanup()

    def level(self):
        return open(os.path.join(sa.DATA_ROOT, 'Minecraft', 'world', 'world', 'level.dat')).read()

    def test_backup_stops_cleanly_and_starts_again(self):
        r = sa.world_backup('minecraft', 'nightly', now=time.mktime((2026, 11, 14, 4, 30, 0, 0, 0, -1)))
        self.assertEqual(r['file'], '2026-11-14 0430.tar.gz')
        verbs = [c[2] for c in self.calls if c[:2] == ['systemctl', '--user']]
        self.assertEqual(verbs[:3], ['is-active', 'stop', 'start'])
        self.assertTrue(self.running)
        b = sa.world_list('minecraft')
        self.assertEqual((b[0]['why'], len(b)), ('nightly', 1))

    def test_roll_back_keeps_today_first(self):
        old = sa.world_backup('minecraft', 'nightly', now=time.time() - 86400)['file']
        os.utime(os.path.join(sa.world_dir('minecraft'), old), (time.time() - 86400, time.time() - 86400))
        with open(os.path.join(sa.DATA_ROOT, 'Minecraft', 'world', 'world', 'level.dat'), 'w') as f:
            f.write('day 2, griefed')
        sa.world_restore('minecraft', old)
        self.assertEqual(self.level(), 'day 1')
        whys = [b['why'] for b in sa.world_list('minecraft')]
        self.assertIn('before rollback', whys)
        self.assertTrue(self.running)
        with self.assertRaises(sa.Fail):
            sa.world_restore('minecraft', '../../etc.tar.gz')

    def test_kept_for_days_and_the_newest_always_stays(self):
        d = sa.world_dir('minecraft')
        os.makedirs(d, exist_ok=True)
        now = time.time()
        for days, n in ((20, '2026-10-25 0430.tar.gz'), (10, '2026-11-04 0430.tar.gz'), (1, '2026-11-13 0430.tar.gz')):
            p = os.path.join(d, n)
            open(p, 'w').close()
            os.utime(p, (now - days * 86400, now - days * 86400))
        self.assertEqual(sa.world_prune('minecraft', 7, now), 2)
        self.assertEqual([b['file'] for b in sa.world_list('minecraft')], ['2026-11-13 0430.tar.gz'])
        for n in os.listdir(d):
            os.utime(os.path.join(d, n), (now - 50 * 86400, now - 50 * 86400))
        self.assertEqual(sa.world_prune('minecraft', 7, now), 0, 'the newest stays whatever its age')

    def test_settings_and_the_nightly_run(self):
        self.assertEqual(sa.settings()['worlds'], {'on': True, 'at': '04:30', 'keep': 7})
        v = sa.worlds_set('keep', '14')
        self.assertEqual(v['settings']['keep'], 14)
        with self.assertRaises(sa.Fail):
            sa.worlds_set('at', '25:00')
        sa.worlds_set('at', '03:30')
        timer = open(os.path.join(sa.UNIT_DIR, 'hearth-server-worlds.timer')).read()
        self.assertIn('OnCalendar=*-*-* 03:30:00', timer)
        sa.players_on = lambda aid: False
        r = sa.worlds_run(now=time.time())
        self.assertEqual(r['done'], ['minecraft'])
        sa.worlds_set('on', 'off')
        self.assertTrue(sa.worlds_run(now=time.time())['off'])

    def test_before_every_update(self):
        src = open(os.path.join(SHELL, 'hearth-server-apps'), encoding='utf-8').read()
        self.assertIn("if update and aid in WORLDS:", src)
        self.assertIn("world_backup(aid, 'before update')", src)
        qml = open(os.path.join(SHELL, 'SetHomeApps.qml'), encoding='utf-8').read()
        self.assertIn('"Roll back to last night"', qml)
        self.assertIn('"Press again to roll back"', qml)


if __name__ == '__main__':
    unittest.main()
