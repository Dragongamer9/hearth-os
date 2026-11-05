"""Tests for Settings › Home server, stage 2 (docs/HOME-SERVER.md): people (Samba-only accounts, per-share access,
their own folders, passwords only through stdin, nothing of theirs deleted), backups (restic: targets never on the
system drive, passwords and keys only in root's 0600 files, schedules, Light mode, restore into a new folder only),
backups from your other Hearth devices, alerts, power (never sleeping while Game Mode runs, a share is open or a
backup runs), the UPS, Game Mode's Quick Access tile, everything off by default and settings written only when
changed. The system's commands are replaced by recorders (test_hearth_home_server.Rec)."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import pwd
import stat
import tempfile
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_hearth_home_server import Base, Rec, hh, hs, USER, ROOT, SHELL, LIBEXEC, UNITS  # noqa: E402

DECKY = ROOT / 'hosts' / 'decky-hearth-link'


class FakePw:
    def __init__(self, name, uid=1500, gecos=hh.PERSON_GECOS, shell='/usr/sbin/nologin', home='/nonexistent'):
        self.pw_name, self.pw_uid, self.pw_gid, self.pw_gecos, self.pw_shell, self.pw_dir = name, uid, uid, gecos, shell, home


class Stage2(Base):
    def setUp(self):
        super().setUp()
        self.accounts = {}
        hh.getpw = lambda n: self.accounts.get(n)
        hh.hearth_people = lambda: {n for n, p in self.accounts.items() if p.pw_gecos == hh.PERSON_GECOS}
        hh.hostname = lambda: 'nvidia-pc'
        self.mnt = os.path.join(self.d, 'mnt', 'big')
        os.makedirs(self.mnt)
        hh.drive_bases = lambda user: [os.path.join(self.d, 'mnt')]
        self.envs = []
        orig = self.rec.run

        def run(cmd, stdin=None, timeout=60):
            if cmd[:2] == ['systemctl', 'is-active']:
                self.rec.calls.append(list(cmd))
                return 3, 'inactive'
            return orig(cmd, stdin, timeout)
        self.rec.run = run
        hh.run = run
        if hh.hsh:
            hh.hsh.run = run

        def run_env(cmd, env, timeout=60):
            self.envs.append((list(cmd), dict(env)))
            self.rec.calls.append(list(cmd))
            return self.restic_answer(cmd)
        hh.run_env = run_env

    def restic_answer(self, cmd):
        if 'snapshots' in cmd:
            return 0, json.dumps([{'id': 'a' * 64, 'short_id': 'aaaaaaaa', 'time': '2026-11-05T03:00:12Z', 'paths': [os.path.join(self.home, 'Games')], 'hostname': 'nvidia-pc'}])
        if 'backup' in cmd:
            return 0, '{"message_type":"summary","total_files_processed":12,"total_bytes_processed":3400,"data_added":100,"snapshot_id":"abcdef0123456789"}\n'
        return 0, ''


# ---------------------------------------------------------------------------------------------------- people

class People(Stage2):
    def test_names_are_checked_strictly(self):
        for bad in ('', 'a', 'Sam', '1sam', 'sam smith', 'sam;rm', 'sam$', '../x', 'x' * 25, 'root', 'admin', 'nobody', 'guest', USER, 'sam\n'):
            self.assertIsNotNone(hh.person_problem(bad, USER), bad)
        self.assertIsNone(hh.person_problem('sam', USER))
        self.accounts['mia'] = FakePw('mia', gecos='Mia', shell='/bin/bash')
        self.assertIn('already an account', hh.person_problem('mia', USER), "someone else's account is never taken over")

    def test_a_person_is_a_samba_only_account_and_the_password_goes_only_through_stdin(self):
        r = hh.person_add({'name': 'sam', 'password': 'correct horse'})
        self.assertTrue(r['made'])
        add = self.rec.ran('useradd')[0]
        for a in ('--system', '--no-create-home', '/usr/sbin/nologin', hh.PERSON_GECOS):
            self.assertIn(a, add)
        smb = self.rec.ran('smbpasswd', '-a', '-s', 'sam')
        self.assertTrue(smb)
        for c in self.rec.calls:
            self.assertNotIn('correct horse', ' '.join(c), 'never on a command line')
        with self.assertRaises(hh.Fail):
            hh.person_add({'name': 'kid', 'password': 'short'})
        with self.assertRaises(hh.Fail):
            hh.person_add({'name': 'kid', 'password': 'bad\x07password'})

    def test_removing_a_person_never_deletes_files_or_someone_elses_account(self):
        self.accounts['sam'] = FakePw('sam')
        r = hh.person_remove('sam')
        self.assertTrue(r['filesKept'])
        self.assertEqual(self.rec.ran('userdel'), [['userdel', 'sam']], 'never userdel -r')
        self.accounts['mia'] = FakePw('mia', gecos='Mia', shell='/bin/bash')
        with self.assertRaises(hh.Fail):
            hh.person_remove('mia')
        with self.assertRaises(hh.Fail):
            hh.person_remove(USER)
        self.assertFalse(self.rec.ran('userdel', 'mia'))

    def test_share_access_per_person(self):
        good, _ = hh.check_shares([self.share('Games', 'Games', write=True, people={'sam': 'write', 'kid': 'read', 'ghost': 'write'}),
                                   self.share('Videos', 'Videos', people={'sam': 'write'})], USER, self.home, set(), people={'sam', 'kid'})
        text = hh.conf_text(good, USER, False)
        games = text.split('[Games]')[1].split('[Videos]')[0]
        videos = text.split('[Videos]')[1]
        self.assertIn('valid users = %s kid sam' % USER, games)
        self.assertIn('read list = kid', games)
        self.assertNotIn('ghost', text, 'an account Hearth didn\'t make is left out')
        self.assertIn('valid users = %s sam' % USER, videos)
        self.assertIn('read only = yes', videos, 'write on a read-only share is read')
        self.assertIn('force user = %s' % USER, games, 'files stay yours')
        with self.assertRaises(hh.Fail):
            hh.check_shares([self.share('Games', 'Games', people={'sam': 'admin'})], USER, self.home, set(), people={'sam'})

    def test_own_folders_are_their_own_share(self):
        self.accounts['sam'] = FakePw('sam')
        os.makedirs(os.path.join(self.home, 'People', 'sam'))
        f = hh.check_people([{'name': 'sam', 'folder': os.path.join(self.home, 'People', 'sam')}, {'name': 'kid', 'folder': '/etc'}], USER, self.home, {'sam'})
        self.assertEqual([x['name'] for x in f], ['sam'])
        text = hh.conf_text([], USER, False, f)
        self.assertIn('[sam]', text)
        self.assertIn('valid users = %s sam' % USER, text)

    def test_the_mantel_side_keeps_no_password(self):
        d = tempfile.mkdtemp()
        hs.SETTINGS = os.path.join(d, 'home-server.json')
        calls = []
        saved = hs.root
        hs.root = lambda args, stdin='': calls.append((args, stdin)) or {'ok': True}
        try:
            hs.person_add('sam', 'correct horse')
            self.assertEqual(calls[0][0], ['person-add'])
            self.assertIn('correct horse', calls[0][1], 'to the root helper through stdin')
            text = open(hs.SETTINGS).read()
            self.assertNotIn('correct horse', text)
            self.assertEqual([p['name'] for p in json.loads(text)['people']], ['sam'])
            with self.assertRaises(hs.Fail):
                hs.person_add('Sam!', 'correct horse')
        finally:
            hs.root = saved


# ---------------------------------------------------------------------------------------------------- backups

class Backups(Stage2):
    def job(self, **kw):
        j = {'id': 'abcdef012345', 'name': 'Games', 'paths': [os.path.join(self.home, 'Games')],
             'target': {'kind': 'drive', 'path': os.path.join(self.mnt, 'Hearth backups')}, 'every': 'daily', 'at': '03:00'}
        j.update(kw)
        return j

    def test_a_backup_never_goes_to_the_system_drive_home_or_into_itself(self):
        self.assertTrue(hh.target_problem(os.path.join(self.mnt, 'Hearth backups'), USER, self.home, {'nvme0n1'})[0])
        for bad in (os.path.join(self.home, 'Backups'), '/etc/backup', '/', os.path.join(self.mnt, '.hidden', 'x'), 'relative', self.mnt + '/a;b'):
            self.assertIsNone(hh.target_problem(bad, USER, self.home, {'nvme0n1'})[0], bad)
        hh.mount_source = lambda p: '/dev/nvme0n1p3'
        self.assertIn('system drive', hh.target_problem(os.path.join(self.mnt, 'x'), USER, self.home, {'nvme0n1'})[1])
        hh.mount_source = lambda p: '/dev/sda1'
        os.makedirs(os.path.join(self.mnt, 'Media'))
        hh.drive_bases = lambda user: [os.path.join(self.d, 'mnt')]
        with self.assertRaises(hh.Fail):
            hh.check_backups([self.job(paths=[os.path.join(self.mnt, 'Media')], target={'kind': 'drive', 'path': os.path.join(self.mnt, 'Media', 'bk')})], USER, self.home, {'nvme0n1'})

    def test_jobs_are_checked(self):
        good = hh.check_backups([self.job()], USER, self.home, {'nvme0n1'})
        self.assertEqual(good[0]['keep'], hh.BACKUP_KEEP_DEFAULT)
        for bad in ({'every': 'minutely'}, {'at': '25:00'}, {'target': {'kind': 'ftp'}}, {'id': '../x'}, {'name': 'a;b'},
                    {'target': {'kind': 'hearth', 'peer': 'rog;ally', 'user': 'ally-1234'}}, {'target': {'kind': 'cloud', 'path': '../up'}}):
            with self.assertRaises(hh.Fail, msg=str(bad)):
                hh.check_backups([self.job(**bad)], USER, self.home, {'nvme0n1'})
        h = hh.check_backups([self.job(target={'kind': 'hearth', 'peer': 'rog-ally.tail4f2a.ts.net', 'user': 'ally-3f9a'})], USER, self.home, set())
        self.assertEqual(hh.repo_of(h[0]), 'rest:http://rog-ally.tail4f2a.ts.net:8095/ally-3f9a/')
        c = hh.check_backups([self.job(target={'kind': 'cloud', 'path': 'my-bucket/hearth'})], USER, self.home, set())
        self.assertEqual(hh.repo_of(c[0]), 'rclone:hearth-abcdef012345:my-bucket/hearth')

    def test_secrets_go_to_root_s_0600_files_only(self):
        hh.backup_secret('abcdef012345', {'password': 'correct horse', 'rest': {'user': 'ally-3f9a', 'password': 'ABCD-EFGH-JKLM-NPQR'},
                                          'cloud': {'kind': 'b2', 'access': 'keyid123', 'secret': 'K00/applicationkey'}})
        for f in (hh.secret_path('abcdef012345', 'pass'), hh.secret_path('abcdef012345', 'rest'), hh.RCLONE_CONF):
            self.assertEqual(stat.S_IMODE(os.stat(f).st_mode), 0o600, f)
        self.assertEqual(stat.S_IMODE(os.stat(hh.BACKUP_DIR).st_mode), 0o700)
        conf = open(hh.RCLONE_CONF).read()
        self.assertIn('[hearth-abcdef012345]', conf)
        self.assertIn('type = b2', conf)
        with self.assertRaises(hh.Fail):
            hh.backup_secret('abcdef012345', {'cloud': {'kind': 's3', 'access': 'has space', 'secret': 'x' * 10}})
        with self.assertRaises(hh.Fail):
            hh.backup_secret('abcdef012345', {'rest': {'user': 'Bad User', 'password': 'x'}})

    def test_your_own_rclone_remotes_are_left_alone(self):
        text = '[mine]\ntype = drive\ntoken = {"x":1}\n\n[hearth-abcdef012345]\ntype = b2\naccount = old\nkey = old\n'
        out = hh.rclone_conf_with(text, 'hearth-abcdef012345', {'kind': 's3', 'access': 'AKIA1234', 'secret': 'secretsecret', 'endpoint': 's3.eu.example.com', 'region': 'eu-1'})
        self.assertIn('[mine]\ntype = drive\ntoken = {"x":1}', out)
        self.assertEqual(out.count('[hearth-abcdef012345]'), 1)
        self.assertNotIn('account = old', out)
        self.assertIn('endpoint = s3.eu.example.com', out)
        self.assertNotIn('hearth-abcdef012345', hh.rclone_strip(out, 'hearth-abcdef012345'))
        self.assertIn('[mine]', hh.rclone_strip(out, 'hearth-abcdef012345'))

    def test_when_a_backup_is_due(self):
        j = hh.check_backups([self.job()], USER, self.home, set())[0]
        lt = time.localtime()
        now = time.mktime((lt.tm_year, lt.tm_mon, lt.tm_mday, 10, 0, 0, 0, 0, -1))
        self.assertTrue(hh.backup_due(j, {}, now), 'never ran')
        self.assertFalse(hh.backup_due(j, {'lastOk': now - 3600}, now), 'ran at 9 today, after 3:00')
        self.assertTrue(hh.backup_due(j, {'lastOk': now - 10 * 3600}, now), 'last ran at midnight, before 3:00')
        self.assertFalse(hh.backup_due(j, {'last': {'ok': False, 'at': now - 600}}, now), 'failed 10 minutes ago: waits an hour')
        self.assertTrue(hh.backup_due(j, {'last': {'ok': False, 'at': now - 4000}}, now))
        self.assertTrue(hh.backup_due(j, {'lastOk': now, 'pending': True}, now), 'Back up now that waited')
        h = dict(j, every='hourly')
        self.assertFalse(hh.backup_due(h, {'lastOk': now - 1800}, now))
        self.assertTrue(hh.backup_due(h, {'lastOk': now - 3500}, now))
        w = dict(j, every='weekly', day=lt.tm_wday)
        self.assertFalse(hh.backup_due(w, {'lastOk': now - 2 * 3600}, now))
        self.assertTrue(hh.backup_due(w, {'lastOk': now - 8 * 86400}, now))

    def plan(self, **kw):
        p = {'user': USER, 'lightMode': True, 'backups': hh.check_backups([self.job(**kw)], USER, self.home, set())}
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        open(hh.PLAN, 'w').write(json.dumps(p))
        hh.backup_secret('abcdef012345', {'password': 'correct horse'})

    def test_a_run_backs_up_forgets_old_ones_and_never_puts_the_password_on_a_command_line(self):
        self.plan()
        r = hh.backup_run(now=time.time())
        self.assertEqual(r['ran'], [{'id': 'abcdef012345', 'ok': True}])
        cmds = [c for c, e in self.envs]
        self.assertTrue(any('backup' in c and os.path.join(self.home, 'Games') in c for c in cmds))
        forget = next(c for c in cmds if 'forget' in c)
        self.assertEqual(forget[forget.index('--keep-daily') + 1], '7')
        self.assertIn('--prune', forget)
        for c, e in self.envs:
            self.assertNotIn('correct horse', ' '.join(c))
            self.assertEqual(e['RESTIC_PASSWORD_FILE'], hh.secret_path('abcdef012345', 'pass'))
        st = json.load(open(hh.BACKUP_STATE))['jobs']['abcdef012345']
        self.assertTrue(st['last']['ok'])
        self.assertEqual(st['last']['files'], 12)
        self.assertEqual(st['count'], 1)
        self.assertNotIn('correct horse', open(hh.BACKUP_STATE).read())
        self.envs.clear()
        hh.backup_run(now=time.time() + 60)
        self.assertFalse(any('backup' in c for c, e in self.envs), 'not due again a minute later')

    def test_light_mode_waits_while_you_play_and_back_up_now_waits_too(self):
        self.plan()
        hh.game_running = lambda: True
        r = hh.backup_run(only='abcdef012345', now_flag=True, now=time.time())
        self.assertIn('deferred', r)
        self.assertFalse(self.envs, 'nothing ran while you play')
        st = json.load(open(hh.BACKUP_STATE))['jobs']['abcdef012345']
        self.assertTrue(st['pending'])
        self.assertIn('Light mode', st['waiting'])
        hh.light_resumed = lambda now=None: True
        r = hh.backup_run(now=time.time())
        self.assertEqual(r['ran'][0]['ok'], True, 'Let paused jobs run now')
        hh.light_resumed = lambda now=None: False
        hh.user_settings = lambda home: {'lightMode': False}
        self.accounts[USER] = FakePw(USER, uid=os.getuid(), gecos='', shell='/bin/bash', home=self.home)
        self.assertFalse(hh.wait_for_game(json.load(open(hh.PLAN))), 'Light mode turned off in Quick Access is read live from your settings')

    def test_an_unplugged_backup_drive_never_gets_a_backup_on_the_system_drive(self):
        self.plan()
        gone = os.path.join(self.d, 'mnt', 'usb', 'Hearth backups')
        os.makedirs(os.path.dirname(gone))           # the empty mount folder an unplugged drive leaves
        hh.mount_source = lambda p: '/dev/sda1' if p.startswith(self.mnt) else '/dev/nvme0n1p3'
        jobs = hh.check_backups([self.job(target={'kind': 'drive', 'path': gone})], USER, self.home)
        self.assertEqual(jobs[0]['target']['path'], gone, 'apply keeps it (the drive comes back)')
        r = hh.backup_one(jobs[0])
        self.assertFalse(r['ok'])
        self.assertIn('plugged in', r['error'])
        self.assertFalse(os.path.exists(gone), 'nothing written onto the system drive')
        self.assertFalse(self.envs, 'restic never ran')

    def test_a_failed_backup_is_kept_with_a_clean_error(self):
        self.plan()
        self.restic_answer = lambda cmd: (1, 'Fatal: unable to open repository at rest:http://ally-3f9a:SECRET@peer:8095/x/: connection refused') if 'backup' in cmd else (0, '')
        r = hh.backup_run(now=time.time())
        self.assertFalse(r['ok'])
        st = json.load(open(hh.BACKUP_STATE))['jobs']['abcdef012345']
        self.assertFalse(st['last']['ok'])
        self.assertNotIn('SECRET', st['last']['error'])

    def test_restore_goes_into_a_new_folder_and_never_overwrites(self):
        self.plan()
        self.accounts[USER] = FakePw(USER, uid=os.getuid(), gecos='', shell='/bin/bash', home=self.home)
        r = hh.backup_restore({'id': 'abcdef012345', 'snapshot': 'a' * 64, 'path': os.path.join(self.home, 'Games')})
        self.assertTrue(r['dest'].startswith(os.path.join(self.home, 'Restored from backups') + '/'))
        self.assertTrue(os.path.isdir(r['dest']))
        self.assertEqual(os.listdir(r['dest']), [])
        unit = self.rec.ran('systemd-run')[0]
        self.assertIn('restore-run', unit)
        r2 = hh.backup_restore({'id': 'abcdef012345', 'snapshot': 'a' * 64, 'path': os.path.join(self.home, 'Games')})
        self.assertNotEqual(r['dest'], r2['dest'], 'a second restore gets its own folder')
        with self.assertRaises(hh.Fail):
            hh.backup_restore({'id': 'abcdef012345', 'snapshot': 'a' * 64, 'path': '/etc'})
        with self.assertRaises(hh.Fail):
            hh.backup_restore({'id': 'abcdef012345', 'snapshot': '; rm -rf', 'path': os.path.join(self.home, 'Games')})
        res = hh.restore_run('abcdef012345', 'a' * 64, r['dest'], os.path.join(self.home, 'Games'))
        cmd = next(c for c, e in self.envs if 'restore' in c)
        self.assertEqual(cmd[cmd.index('--overwrite') + 1], 'never')
        self.assertEqual(cmd[cmd.index('--target') + 1], r['dest'])
        self.assertTrue(res['ok'])
        os.makedirs(os.path.join(self.home, 'Games', 'x'))
        with self.assertRaises(hh.Fail):
            hh.restore_run('abcdef012345', 'a' * 64, os.path.join(self.home, 'Games'), os.path.join(self.home, 'Games'))

    def test_the_mantel_side_keeps_no_password_or_key(self):
        d = tempfile.mkdtemp()
        hs.SETTINGS = os.path.join(d, 'home-server.json')
        home = tempfile.mkdtemp()
        os.makedirs(os.path.join(home, 'Games'))
        saved_me, saved_root, saved_mod = hs.me, hs.root, hs.hh
        hs.me = lambda: (USER, home)
        hs.hh = hh
        calls = []
        hs.root = lambda args, stdin='': calls.append((args, stdin)) or {'ok': True}
        try:
            s = hs.share_add(os.path.join(home, 'Games'))
            j = hs.backup_add({'name': 'Cloud', 'shares': [s['id']], 'target': {'kind': 'cloud', 'provider': 'b2', 'path': 'bucket/hearth'},
                               'password': 'correct horse', 'cloud': {'access': 'keyid123', 'secret': 'K00secretkey'}})
            text = open(hs.SETTINGS).read()
            for secret in ('correct horse', 'keyid123', 'K00secretkey'):
                self.assertNotIn(secret, text)
            self.assertEqual(calls[0][0], ['backup-secret', j['id']])
            self.assertIn('K00secretkey', calls[0][1])
            j2 = hs.backup_add({'name': 'Ally', 'shares': [s['id']], 'target': {'kind': 'hearth', 'peer': 'rog-ally'},
                                'password': 'correct horse', 'code': 'ally-3f9a:ABCD-EFGH-JKLM-NPQR'})
            self.assertEqual(j2['target'], {'kind': 'hearth', 'peer': 'rog-ally', 'user': 'ally-3f9a'})
            self.assertNotIn('ABCD-EFGH', open(hs.SETTINGS).read())
            with self.assertRaises(hs.Fail):
                hs.backup_add({'name': 'X', 'shares': [s['id']], 'target': {'kind': 'hearth', 'peer': 'rog-ally'}, 'password': 'correct horse', 'code': 'nope'})
            with self.assertRaises(hs.Fail):
                hs.backup_add({'name': 'X', 'shares': [s['id']], 'target': {'kind': 'cloud', 'path': 'b'}, 'password': 'short'})
            p = hs.apply_payload(hs.load())
            self.assertEqual(p['backups'][0]['paths'], [os.path.join(home, 'Games')])
            self.assertNotIn('correct horse', json.dumps(p))
            self.assertTrue(hs.backup_set(j['id'], 'every', 'weekly'))
            self.assertFalse(hs.backup_set(j['id'], 'every', 'weekly'), 'the same: not written')
            self.assertTrue(hs.backup_set(j['id'], 'keepDaily', '14'))
            self.assertEqual(hs.job_of(j['id'])['keep']['daily'], 14)
            r = hs.backup_remove(j['id'])
            self.assertTrue(r['backupKept'])
            self.assertEqual(calls[-1][0], ['backup-forget', j['id']])
        finally:
            hs.me, hs.root, hs.hh = saved_me, saved_root, saved_mod


class Receive(Stage2):
    def test_codes_are_shown_once_and_only_their_hash_kept(self):
        r = hh.receive_add('rog-ally')
        user, pw = r['code'].split(':', 1)
        self.assertEqual(user, r['user'])
        ht = open(os.path.join(hh.RECEIVE_DIR, 'htpasswd')).read()
        self.assertIn(user + ':{SHA}', ht)
        self.assertNotIn(pw, ht)
        hh.receive_remove(user)
        self.assertNotIn(user, open(os.path.join(hh.RECEIVE_DIR, 'htpasswd')).read())
        with self.assertRaises(hh.Fail):
            hh.receive_add('Bad Name;')

    def test_it_listens_only_on_tailscale_and_is_off_by_default(self):
        self.assertEqual(hh.receive_apply({'on': False}, USER, self.home)['receive'], 'off')
        self.assertFalse(self.rec.ran('systemctl', 'enable', '--now', hh.RECEIVE_TIMER))
        self.rec.tools |= {'rclone', 'tailscale'}
        orig = self.rec.run
        self.rec.run = lambda cmd, stdin=None, timeout=60: (0, '100.84.12.7\n') if cmd[:2] == ['tailscale', 'ip'] else orig(cmd, stdin, timeout)
        hh.run = self.rec.run
        r = hh.receive_apply({'on': True, 'path': os.path.join(self.home, 'Games')}, USER, self.home)
        self.assertEqual(r['address'], '100.84.12.7:8095')
        drop = open(hh.RECEIVE_DROPIN).read()
        self.assertIn('--addr 100.84.12.7:8095', drop)
        self.assertIn('--private-repos', drop)
        self.assertIn('User=%s' % USER, drop)
        with self.assertRaises(hh.Fail):
            hh.receive_apply({'on': True, 'path': '/etc'}, USER, self.home)


# ---------------------------------------------------------------------------------------------------- alerts

class Alerts(Stage2):
    def write_plan(self, **kw):
        p = {'user': USER, 'alerts': hh.clean_alerts(dict({'on': True}, **kw)), 'backups': [], 'shares': []}
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        open(hh.PLAN, 'w').write(json.dumps(p))
        return p

    def test_a_failing_drive_and_a_failed_backup_are_found_and_told_once_a_day(self):
        self.write_plan()
        saved = (hh.drives, hh.notify)
        hh.drives = lambda: {'drives': [{'model': 'WD Red 8TB', 'token': 't1', 'smart': {'passed': False, 'temp': 40}},
                                        {'model': 'Samsung 990', 'token': 't2', 'smart': {'passed': True, 'temp': 70}}]}
        told = []
        hh.notify = lambda user, text, urgent=False: told.append(text) or True
        try:
            r = hh.alerts_run(now=1000000)
            kinds = sorted(a['key'] for a in r['active'])
            self.assertEqual(kinds, ['hot:t2', 'smart:t1'])
            self.assertEqual(len(told), 2)
            hh.alerts_run(now=1000000 + 3600)
            self.assertEqual(len(told), 2, 'not again within a day')
            hh.alerts_run(now=1000000 + 25 * 3600)
            self.assertEqual(len(told), 4)
        finally:
            hh.drives, hh.notify = saved

    def test_off_finds_nothing_and_taildrop_only_when_chosen(self):
        self.write_plan(on=False)
        self.assertEqual(hh.alerts_run(now=1)['active'], [])
        self.assertEqual(hh.clean_alerts({})['on'], False, 'off until you turn it on')
        self.assertEqual(hh.clean_alerts({'peers': ['phone', 'bad;peer']})['peers'], ['phone'])
        self.assertEqual(hh.taildrop_alert([], 'x', 1), [])

    def test_a_degraded_mirror(self):
        self.rec.run_answers = {}
        orig = self.rec.run

        def run(cmd, stdin=None, timeout=60):
            if cmd[:3] == ['btrfs', 'filesystem', 'show']:
                return 0, 'Label: big\n\tdevid 1 size 4TiB path /dev/sda1\n\t*** Some devices missing\n'
            return orig(cmd, stdin, timeout)
        hh.run = run
        self.assertIn('missing', hh.mirror_problem('/var/mnt/big'))


# ---------------------------------------------------------------------------------------------------- power

class Power(Stage2):
    def write_plan(self, **kw):
        p = {'user': USER, 'power': hh.clean_power(kw)}
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        open(hh.PLAN, 'w').write(json.dumps(p))

    def test_hdparm_values_and_only_spinning_data_drives(self):
        self.assertEqual([hh.hdparm_value(m) for m in (0, 10, 20, 30, 60)], [0, 120, 240, 241, 242])
        hh.lsblk = lambda: [{'name': 'nvme0n1', 'path': '/dev/nvme0n1', 'type': 'disk', 'rota': False},
                            {'name': 'sda', 'path': '/dev/sda', 'type': 'disk', 'rota': True},
                            {'name': 'sdb', 'path': '/dev/sdb', 'type': 'disk', 'rota': False},
                            {'name': 'sdc', 'path': '/dev/sdc', 'type': 'disk', 'rota': True, 'rm': True}]
        hh.system_disks = lambda: {'nvme0n1'}
        self.assertEqual(hh.spin_disks(), ['/dev/sda'])
        hh.system_disks = lambda: {'sda'}
        self.assertEqual(hh.spin_disks(), [], 'never the system drive')

    def test_never_sleeps_while_playing_sharing_or_backing_up(self):
        self.write_plan(sleepIdle=30)
        hh.someone_using = lambda: False
        for busy in ('game', 'smb', 'job'):
            hh.game_running = lambda: busy == 'game'
            hh.smb_sessions = lambda: 1 if busy == 'smb' else 0
            hh.jobs_running = lambda: busy == 'job'
            self.rec.calls.clear()
            for t in range(0, 3 * 3600, 300):
                hh.power_run(now=1000000 + t)
            self.assertFalse(self.rec.ran('systemctl', 'suspend'), busy)

    def test_sleeps_when_nobody_needs_it_and_respects_inhibitors(self):
        self.write_plan(sleepIdle=30)
        hh.game_running = lambda: False
        hh.smb_sessions = lambda: 0
        hh.jobs_running = lambda: False
        hh.someone_using = lambda: False
        hh.power_run(now=1000000)
        hh.power_run(now=1000000 + 20 * 60)
        self.assertFalse(self.rec.ran('systemctl', 'suspend'))
        hh.power_run(now=1000000 + 31 * 60)
        self.assertEqual(self.rec.ran('systemctl', 'suspend'), [['systemctl', 'suspend', '--check-inhibitors=yes']])
        hh.someone_using = lambda: True
        self.rec.calls.clear()
        for t in range(0, 3 * 3600, 300):
            hh.power_run(now=2000000 + t)
        self.assertFalse(self.rec.ran('systemctl', 'suspend'), 'someone is using it')

    def test_off_by_default_does_nothing(self):
        self.write_plan()
        hh.game_running = lambda: False
        hh.smb_sessions = lambda: 0
        hh.jobs_running = lambda: False
        hh.someone_using = lambda: False
        for t in range(0, 5 * 3600, 300):
            hh.power_run(now=1000000 + t)
        self.assertFalse(self.rec.ran('systemctl', 'suspend'))
        self.assertFalse(self.rec.ran('hdparm'))
        self.assertEqual(hh.clean_power({}), hh.POWER_DEFAULT)

    def test_keep_awake_while_used(self):
        self.write_plan(keepAwake=True)
        hh.game_running = lambda: False
        hh.smb_sessions = lambda: 2
        hh.jobs_running = lambda: False
        hh.someone_using = lambda: False
        hh.power_run(now=1000000)
        run = self.rec.ran('systemd-run')[0]
        self.assertIn('systemd-inhibit', run)
        self.assertIn('--what=sleep', run)

    def test_the_schedule_wakes_with_a_timer_and_sleeps_once_a_night(self):
        self.write_plan(schedule=True, sleepAt='01:00', wakeAt='07:00')
        hh.game_running = lambda: False
        hh.smb_sessions = lambda: 0
        hh.jobs_running = lambda: False
        hh.someone_using = lambda: True
        lt = time.localtime()
        night = time.mktime((lt.tm_year, lt.tm_mon, lt.tm_mday, 1, 30, 0, 0, 0, -1))
        hh.power_run(now=night)
        hh.power_run(now=night + 600)
        self.assertEqual(len(self.rec.ran('systemctl', 'suspend')), 1, 'once a night (waking it with Wake-on-LAN keeps it up)')
        self.assertEqual(hh.wake_apply(hh.clean_power({'schedule': True, 'wakeAt': '06:45'})), 'on')
        self.assertIn('OnCalendar=*-*-* 06:45:00', open(hh.WAKE_DROPIN).read())
        self.assertIn('WakeSystem=true', open(hh.WAKE_DROPIN).read())
        hh.wake_apply(hh.clean_power({}))
        self.assertFalse(os.path.exists(hh.WAKE_DROPIN))


class Ups(Stage2):
    def test_hand_made_nut_is_left_alone_and_hearth_s_own_comes_and_goes(self):
        os.makedirs(hh.NUT_DIR)
        self.rec.tools.add('upsmon')
        open(os.path.join(hh.NUT_DIR, 'ups.conf'), 'w').write('[myups]\n driver = usbhid-ups\n')
        self.assertEqual(hh.ups_apply(True)['ups'], 'by-hand')
        self.assertFalse(self.rec.ran('systemctl', 'enable', '--now', 'nut-server.service'))
        os.remove(os.path.join(hh.NUT_DIR, 'ups.conf'))
        self.assertEqual(hh.ups_apply(True)['ups'], 'on')
        self.assertIn('MODE=standalone', open(os.path.join(hh.NUT_DIR, 'nut.conf')).read())
        self.assertIn('LISTEN 127.0.0.1', open(os.path.join(hh.NUT_DIR, 'upsd.conf')).read(), 'only this PC')
        self.assertTrue(self.rec.ran('systemctl', 'enable', '--now', 'nut-server.service'))
        self.assertEqual(hh.ups_apply(False)['ups'], 'off')
        self.assertTrue(self.rec.ran('systemctl', 'disable', '--now', 'nut-monitor.service'))
        self.assertIn('MODE=none', open(os.path.join(hh.NUT_DIR, 'nut.conf')).read())


# ---------------------------------------------------------------------------------------------------- apply, off by default

class ApplyStage2(Stage2):
    def test_apply_turns_on_only_the_timers_something_needs(self):
        data = {'shares': [self.share('Games', 'Games', write=False, recycle=False)], 'people': [], 'backups': [], 'alerts': {}, 'power': {}}
        hh.apply(data)
        for t in (hh.BACKUP_TIMER, hh.ALERT_TIMER, hh.POWER_TIMER, hh.WAKE_TIMER, hh.RECEIVE_TIMER, hh.SNAP_TIMER):
            self.assertFalse(self.rec.ran('systemctl', 'enable', '--now', t), t)
        plan = json.load(open(hh.PLAN))
        self.assertEqual(plan['alerts']['on'], False)
        self.assertEqual(plan['power'], hh.POWER_DEFAULT)
        hh.backup_secret('abcdef012345', {'password': 'correct horse'})
        data['backups'] = [{'id': 'abcdef012345', 'name': 'Games', 'paths': [os.path.join(self.home, 'Games')],
                            'target': {'kind': 'drive', 'path': os.path.join(self.mnt, 'Hearth backups')}}]
        data['alerts'] = {'on': True}
        data['power'] = {'spinDown': 20}
        hh.apply(data)
        for t in (hh.BACKUP_TIMER, hh.ALERT_TIMER, hh.POWER_TIMER):
            self.assertTrue(self.rec.ran('systemctl', 'enable', '--now', t), t)
        self.assertNotIn('correct horse', open(hh.PLAN).read())

    def test_a_person_s_folder_name_never_clashes_with_a_share(self):
        self.accounts['games'] = FakePw('games')
        os.makedirs(os.path.join(self.home, 'People', 'games'))
        with self.assertRaises(hh.Fail):
            hh.apply({'shares': [self.share('Games', 'Games')], 'people': [{'name': 'games', 'folder': os.path.join(self.home, 'People', 'games')}]})


class OffByDefault2(unittest.TestCase):
    def test_defaults(self):
        d = hs.DEFAULTS
        self.assertEqual((d['people'], d['backups'], d['syncs']), ([], [], []))
        self.assertIs(d['alerts']['on'], False)
        self.assertIs(d['receive']['on'], False)
        p = d['power']
        self.assertEqual((p['sleepIdle'], p['keepAwake'], p['spinDown'], p['schedule'], p['ups']), (0, False, 0, False, False))

    def test_the_image_turns_every_new_service_off_and_checks_it(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        block = b.split('# ---- Hearth home server')[1].split('# ---- end of Hearth home server')[0]
        for pkg in ('restic', 'rclone', 'syncthing', 'hdparm', 'nut-client', 'nut-server'):
            self.assertIn(pkg, block)
        for u in ('hearth-home-server-backup.timer', 'hearth-home-server-alerts.timer', 'hearth-home-server-power.timer',
                  'hearth-home-server-wake.timer', 'hearth-home-server-receive.timer', 'nut-server', 'nut-monitor', 'syncthing'):
            self.assertIn(u, block, u)
        self.assertNotIn('systemctl enable', block)
        wf = (ROOT / '.github' / 'workflows' / 'base-peppermint.yml').read_text()
        for u in ('for n in backup alerts power wake receive', 'hearth-home-server-\\$n.timer', 'nut-server', 'syncthing.service', 'restic', 'rclone'):
            self.assertIn(u, wf, u)

    def test_units_never_hold_up_starting(self):
        for n in ('backup', 'alerts', 'power', 'wake', 'receive'):
            svc = (UNITS / 'system' / ('hearth-home-server-%s.service' % n)).read_text()
            timer = (UNITS / 'system' / ('hearth-home-server-%s.timer' % n)).read_text()
            self.assertNotIn('[Install]', svc, n + ': only its timer starts it')
            for text in (svc, timer):
                self.assertNotIn('Requires=', text)
                self.assertNotIn('network-online', text, 'never waits for the network at start')
                for target in ('multi-user.target', 'graphical.target', 'default.target', 'nimbus', 'gamescope', 'steam'):
                    self.assertNotIn('WantedBy=' + target, text)
            self.assertIn('WantedBy=timers.target', timer)
        backup = (UNITS / 'system' / 'hearth-home-server-backup.service').read_text()
        self.assertIn('systemd-inhibit --what=sleep', backup)
        self.assertIn('IOSchedulingClass=idle', backup)
        for wants in UNITS.glob('**/*.wants'):
            for n in os.listdir(wants):
                self.assertNotIn('hearth-home-server', n)


# ---------------------------------------------------------------------------------------------------- settings, Quick Access

class Settings2(unittest.TestCase):
    def setUp(self):
        d = tempfile.mkdtemp()
        hs.SETTINGS = os.path.join(d, 'home-server.json')
        hs.SEEN = os.path.join(d, 'seen.json')
        os.environ['XDG_RUNTIME_DIR'] = tempfile.mkdtemp()

    def test_a_nested_setting_writes_only_that_key(self):
        self.assertFalse(hs.set_key('alertsOn', 'off'), 'the default: not written')
        self.assertFalse(os.path.exists(hs.SETTINGS))
        self.assertTrue(hs.set_key('alertsOn', 'on'))
        self.assertEqual(json.load(open(hs.SETTINGS)), {'alerts': {'on': True}})
        self.assertTrue(hs.set_key('sleepIdle', '60'))
        self.assertTrue(hs.set_key('wakeAt', '06:30'))
        self.assertEqual(json.load(open(hs.SETTINGS)), {'alerts': {'on': True}, 'power': {'sleepIdle': 60, 'wakeAt': '06:30'}})
        for k, v in (('sleepIdle', '45'), ('wakeAt', '25:00'), ('alertFullPct', '50'), ('spinDown', 'x'), ('ups', 'yes')):
            with self.assertRaises(hs.Fail, msg=k):
                hs.set_key(k, v)
        self.assertEqual(hs.load()['alerts']['smart'], True, 'defaults filled in when read')

    def test_resume_is_a_runtime_flag_not_a_setting(self):
        hs.resume(True)
        self.assertTrue(hs.resumed())
        self.assertFalse(os.path.exists(hs.SETTINGS))
        hs.resume(False)
        self.assertFalse(hs.resumed())

    def test_quick_is_short_and_saves_nothing(self):
        q = hs.quick()
        for k in ('available', 'host', 'lightMode', 'playing', 'paused', 'shares', 'usage', 'backups', 'alerts'):
            self.assertIn(k, q)
        self.assertFalse(os.path.exists(hs.SETTINGS))
        self.assertTrue(hs.main(['x', 'quick-set', 'lightMode', 'off']) == 0)
        self.assertEqual(json.load(open(hs.SETTINGS)), {'lightMode': False})

    def test_the_seen_ups_never_changes_a_setting(self):
        hs.mark_seen('ups', 'CyberPower CP1500')
        self.assertFalse(os.path.exists(hs.SETTINGS))


class QuickAccess(unittest.TestCase):
    def test_the_hearth_link_panel_asks_only_a_few_commands_and_never_blocks(self):
        src = (DECKY / 'main.py').read_text()
        block = src.split('# ---- Home server')[1].split('# ---- end Home server')[0]
        self.assertIn("HOME_SERVER_CALLS = ('quick', 'quick-set', 'resume')", block)
        self.assertIn('timeout=', block)
        self.assertNotIn('pkexec', block, 'Game Mode never asks for a password')
        self.assertNotIn('shell=True', block)
        ui = (DECKY / 'src' / 'index.tsx').read_text()
        self.assertIn('HomeServer', ui)
        self.assertIn('class Guard', ui, 'its own error boundary: a problem there never breaks the panel')

    def test_game_mode_s_start_is_not_touched(self):
        for p in (ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session',):
            self.assertNotIn('home-server', p.read_text())


class Wiring2(unittest.TestCase):
    def test_pages_are_registered(self):
        s = (SHELL / 'SettingsApp.qml').read_text()
        for pid, f in (('hs-backups', 'SetHomeBackups.qml'), ('hs-people', 'SetHomePeople.qml'), ('hs-alerts', 'SetHomeAlerts.qml')):
            self.assertIn('"%s": "%s"' % (pid, f), s)
            self.assertIn('"%s": "Home server"' % pid, s)
            qml = (SHELL / f).read_text()
            self.assertIn('helper: "hearth-home-server"', qml)
            self.assertNotIn('FileView', qml)
        setup = (SHELL / 'SetHomeSetup.qml').read_text()
        self.assertIn('hs-backups', setup, 'the setup steps point at Backups & sync')

    def test_the_root_helper_never_uses_a_shell(self):
        src = (LIBEXEC / 'hearth-home-server-helper').read_text()
        self.assertNotIn('shell=True', src)
        self.assertNotIn('os.system(', src)
        self.assertNotRegex(src, r"\['(ba)?sh', '-c'")


if __name__ == '__main__':
    unittest.main()
