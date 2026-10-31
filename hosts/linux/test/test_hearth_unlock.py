"""Tests for hearth-unlock (Hearth OS on Peppermint is writable: each version's own layer over /usr, your packages put
back after an update, a safe start when your changes stop Hearth from starting, the lock switch), hearth-admin's
unlock/deb commands, Hearth Recovery's "without your system changes" and the .deb installer going native.
Nothing here mounts anything or runs apt: mount, apt and systemctl are replaced; the record, the layers and the
decisions are the real code."""
import sys
sys.dont_write_bytecode = True
import contextlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
UNLOCK = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'bin' / 'hearth-unlock'
ADMIN = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'
RECOVERY = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-recovery'
DEBBOX = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-debbox'
UNITS = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system'
CONF = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'share' / 'hearth' / 'unlock'
GENERATOR = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system-generators' / 'hearth-etc-guard'
A = 'a' * 64
B = 'b' * 64


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-unlock-test-')
        self.state = os.path.join(self.tmp, 'state')
        self.lockfile = os.path.join(self.tmp, 'etc', 'system-locked')
        self.baseline = os.path.join(self.tmp, 'baseline.list')
        self.cmdline = os.path.join(self.tmp, 'cmdline')
        self.mountinfo = os.path.join(self.tmp, 'mountinfo')
        with open(self.baseline, 'w') as f:
            f.write('bash\nsteam-installer\nxfce4\n')
        with open(self.cmdline, 'w') as f:
            f.write('root=UUID=x quiet\n')
        open(self.mountinfo, 'w').close()
        os.environ.update(HEARTH_UNLOCK_STATE=self.state, HEARTH_UNLOCK_LOCKFILE=self.lockfile,
                          HEARTH_UNLOCK_BASELINE=self.baseline, HEARTH_UNLOCK_CMDLINE=self.cmdline,
                          HEARTH_UNLOCK_MOUNTINFO=self.mountinfo, HEARTH_UNLOCK_ID=A, HEARTH_UNLOCK_USR='/usr',
                          HEARTH_UNLOCK_IMAGE_USR=os.path.join(self.tmp, 'run', 'usr-image'),
                          HEARTH_UNLOCK_STOP=os.path.join(self.tmp, 'run', 'unlock-stop'))
        self.mount_fails = False
        self.u = load(UNLOCK, 'hearth_unlock')
        self.calls = []
        self.mounted = False

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_run(self, cmd, timeout=600, env=None, input=None):
        self.calls.append(list(cmd))
        if cmd[:3] == ['mount', '-t', 'overlay']:
            if self.mount_fails:
                return 32, 'mount: wrong fs type'
            self.mounted = True
        return 0, ''

    def boot(self, lid=None):
        if lid:
            os.environ['HEARTH_UNLOCK_ID'] = lid
        self.mounted = False
        with mock.patch.object(self.u, 'run', side_effect=self.fake_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted), \
                contextlib.redirect_stderr(io.StringIO()):
            self.u.cmd_boot()
        return self.mounted

    def touch_upper(self, lid, name='bin/htop'):
        p = os.path.join(self.state, 'layers', lid, 'upper', name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        open(p, 'w').close()

    def good(self, lid=A):
        """What hearth-unlock-good.service does once Hearth has been up long enough."""
        with mock.patch.object(self.u, 'up_now', return_value=True), mock.patch.object(self.u, 'UP_SECONDS', 0), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted), \
                mock.patch.object(self.u.time, 'sleep'):
            self.u.cmd_watch(limit=5)


class Parsing(Base):
    def test_commit_from_the_boot_link(self):
        self.assertEqual(self.u.deploy_from_link(f'/sysroot/ostree/deploy/hearth/deploy/{A}.0'), A)
        self.assertIsNone(self.u.deploy_from_link('/sysroot/ostree/boot.1/hearth/x/0'))

    def test_ostree_status(self):
        st = (f'  hearth {B}.0 (staged)\n    origin: ostree-unverified-registry:x\n'
              f'* hearth {A}.1\n    Version: 0.6.58\n  hearth {"c" * 64}.0 (rollback)\n')
        self.assertEqual(self.u.booted_from_status(st), A)
        self.assertEqual(self.u.deployed_from_status(st), {A, B, 'c' * 64})
        self.assertIsNone(self.u.booted_from_status('nothing'))

    def test_overlay_seen_in_mountinfo(self):
        mine = '1234 22 0:55 / /usr rw,relatime shared:1 - overlay hearth-usr rw,lowerdir=/usr,upperdir=/x'
        other = '99 1 0:30 / /usr ro,relatime - overlay composefs ro,lowerdir=/y'
        self.assertTrue(self.u.overlay_on(mine + '\n'))
        self.assertFalse(self.u.overlay_on(other))
        self.assertFalse(self.u.overlay_on(mine.replace(' /usr ', ' /usr/lib ')))

    def test_mount_command(self):
        cmd = self.u.mount_cmd(A)
        self.assertEqual(cmd[:4], ['mount', '-t', 'overlay', 'hearth-usr'])
        self.assertEqual(cmd[-1], '/usr')
        self.assertIn(f'upperdir={self.state}/layers/{A}/upper', cmd[5])
        self.assertIn(f'workdir={self.state}/layers/{A}/work', cmd[5])
        self.assertTrue(cmd[5].startswith('lowerdir=/usr,'))

    def test_installed_list(self):
        got = self.u.parse_installed('htop\thtop\tii \nfoo\tfoo:i386\tii \ngone\tgone\trc \n')
        self.assertEqual(got, {'htop', 'foo', 'foo:i386'})

    def test_apt_progress(self):
        self.assertEqual(self.u.apt_status_line('pmstatus:htop:50:Installing htop', 10, 90)[1], 'Installing htop')
        self.assertIsNone(self.u.apt_status_line('Reading package lists...', 10, 90))

    def test_removes_only(self):
        self.assertTrue(self.u.removes_only('Remv xfce4 [4.20]\n', 'xfce4'))
        self.assertFalse(self.u.removes_only('Remv xfce4 [4.20]\nRemv xfce4-panel [4.20]\n', 'xfce4'))
        self.assertFalse(self.u.removes_only('', 'xfce4'))


class Record(Base):
    def rec(self, **kw):
        r = {'manual': [], 'removed': [], 'debs': {}, 'updated': ''}
        r.update(kw)
        return r

    def test_an_install_is_kept(self):
        base = {'bash', 'xfce4'}
        out = self.u.merge_record(self.rec(), base, base | {'htop'}, base, base | {'htop', 'libhtop'})
        self.assertEqual(out['manual'], ['htop'])

    def test_a_removal_is_dropped(self):
        base = {'bash'}
        out = self.u.merge_record(self.rec(manual=['htop', 'code'], debs={'code': 'code.deb'}), base | {'htop', 'code'},
                                  base | {'htop'}, base, base | {'htop'})
        self.assertEqual(out['manual'], ['htop'])
        self.assertEqual(out['debs'], {})

    def test_image_package_removed_then_back(self):
        base = {'bash', 'xfce4'}
        out = self.u.merge_record(self.rec(), base, {'bash'}, base, {'bash'})
        self.assertEqual(out['removed'], ['xfce4'])
        out = self.u.merge_record(out, {'bash'}, base, base, base)
        self.assertEqual(out['removed'], [])

    def test_image_package_marked_auto_is_not_removed(self):
        base = {'bash', 'xfce4'}
        out = self.u.merge_record(self.rec(), base, {'bash'}, base, {'bash', 'xfce4'})
        self.assertEqual(out['removed'], [])

    def test_installing_on_a_new_version_before_the_reapply_keeps_the_rest(self):
        # The update started clean; you install one thing before your packages are back: the others stay recorded.
        base = {'bash'}
        out = self.u.merge_record(self.rec(manual=['htop', 'code']), base, base | {'vlc'}, base, base | {'vlc'})
        self.assertEqual(out['manual'], ['code', 'htop', 'vlc'])

    def test_record_command_uses_the_image_when_the_layer_is_new(self):
        os.makedirs(os.path.join(self.state, 'layers', A))
        with mock.patch.object(self.u, 'mounted', return_value=True), \
                mock.patch.object(self.u, 'lock_free', return_value=True), \
                mock.patch.object(self.u, 'apt_manual', return_value={'bash', 'steam-installer', 'xfce4', 'htop'}), \
                mock.patch.object(self.u, 'installed_set', return_value={'bash', 'steam-installer', 'xfce4', 'htop'}):
            self.u.cmd_record()
        self.assertEqual(self.u.load_record()['manual'], ['htop'])
        self.assertEqual(sorted(self.u.load_layer(A)['seen']), ['bash', 'htop', 'steam-installer', 'xfce4'])

    def test_record_never_puts_back_an_older_reapply_state(self):
        # The record runs in the background after each dpkg run, while the reapply writes its result (seen in CI).
        self.u.save_layer(A, {'reapply': {'state': 'running'}})
        real_load = self.u.load_layer

        def slow_load(lid):
            got = real_load(lid)
            self.u.save_layer(A, dict(real_load(lid), reapply={'state': 'done'}))   # the reapply finishes meanwhile
            return got
        with mock.patch.object(self.u, 'mounted', return_value=True), mock.patch.object(self.u, 'lock_free', return_value=True), \
                mock.patch.object(self.u, 'apt_manual', return_value={'bash', 'htop'}), \
                mock.patch.object(self.u, 'installed_set', return_value={'bash', 'htop'}), \
                mock.patch.object(self.u, 'load_layer', side_effect=slow_load):
            self.u.cmd_record()
        self.assertEqual(real_load(A)['reapply']['state'], 'done')
        self.assertEqual(real_load(A)['seen'], ['bash', 'htop'])

    def test_record_does_nothing_when_locked(self):
        with mock.patch.object(self.u, 'mounted', return_value=False), \
                mock.patch.object(self.u, 'apt_manual', side_effect=AssertionError('must not run')):
            self.assertEqual(self.u.cmd_record(), 0)

    def test_hooks_never_fail(self):
        with mock.patch.object(self.u.os.path, 'exists', side_effect=OSError('boom')), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(self.u.cmd_record_later(), 0)
        with mock.patch.object(self.u, 'mounted', return_value=False), mock.patch.object(sys, 'stdin', io.StringIO('/x.deb\n')):
            self.assertEqual(self.u.cmd_stash_debs(), 0)

    @unittest.skipUnless(shutil.which('dpkg-deb'), 'needs dpkg-deb')
    def test_a_deb_from_a_file_is_kept(self):
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
        import make_test_deb
        deb = os.path.join(self.tmp, 'dl', 'hearth-hello_1.0_all.deb')
        os.makedirs(os.path.dirname(deb))
        make_test_deb.make(deb)
        self.assertEqual(self.u.stash_deb(deb), 'hearth-hello')
        self.assertTrue(os.path.isfile(os.path.join(self.state, 'debs', 'hearth-hello.deb')))
        self.assertEqual(self.u.load_record()['debs'], {'hearth-hello': 'hearth-hello.deb'})
        # apt's own downloads are not copied
        self.assertIsNone(self.u.stash_deb('/var/cache/apt/archives/htop_3_amd64.deb'))


class Starting(Base):
    def test_unlocked_by_default(self):
        self.assertTrue(self.boot())
        self.assertTrue(any(c[0] == 'mount' for c in self.calls))
        self.assertIn(['ldconfig', '-X'], self.calls)

    def test_locked_starts_clean(self):
        os.makedirs(os.path.dirname(self.lockfile))
        open(self.lockfile, 'w').close()
        self.assertFalse(self.boot())

    def test_kernel_option_starts_clean(self):
        with open(self.cmdline, 'w') as f:
            f.write('quiet hearth.clean\n')
        self.assertFalse(self.boot())

    def test_decisions(self):
        d = self.u.boot_decision
        self.assertEqual(d(True, False, False, False, False, 0), 'mount')
        self.assertEqual(d(True, False, False, False, False, 5), 'mount')   # an empty layer can't be the problem
        self.assertEqual(d(True, False, False, False, True, 0), 'mount')
        self.assertEqual(d(True, False, False, False, True, 1), 'safe-start')
        self.assertEqual(d(True, True, False, False, True, 1), 'locked')
        self.assertEqual(d(True, False, True, False, True, 0), 'clean')
        self.assertEqual(d(True, False, False, True, True, 0), 'suspended')
        self.assertEqual(d(False, False, False, False, False, 0), 'unknown')

    def test_a_bad_change_gets_a_safe_start_then_can_come_back(self):
        self.touch_upper(A)
        self.assertTrue(self.boot())                  # start with your changes...
        self.assertEqual(self.u.load_layer(A)['pending'], 1)
        self.assertFalse(self.boot())                 # ...never reached Hearth: the next one is without them
        layer = self.u.load_layer(A)
        self.assertTrue(layer['suspended'])
        self.assertEqual(self.u.status()['notice']['kind'], 'safe-start')
        self.assertFalse(self.boot())                 # and stays so until you turn them back on
        with mock.patch.object(self.u.os, 'geteuid', return_value=0), \
                mock.patch.object(self.u, 'run', side_effect=self.fake_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted):
            self.u.cmd_unlock()
        self.assertFalse(self.u.load_layer(A)['suspended'])
        self.assertIsNone(self.u.status()['notice'])
        self.assertTrue(self.boot())

    def test_a_good_start_resets_the_count(self):
        self.touch_upper(A)
        self.assertTrue(self.boot())
        self.good()
        self.assertEqual(self.u.load_layer(A)['pending'], 0)
        self.assertTrue(self.boot())                  # no safe start

    def test_a_clean_start_does_not_clear_the_count(self):
        self.touch_upper(A)
        self.boot()
        self.mounted = False
        self.good()                                    # Hearth up, but this start had no layer
        self.assertEqual(self.u.load_layer(A)['pending'], 1)

    def test_an_update_starts_clean_and_puts_packages_back_while_the_old_version_keeps_its_own(self):
        self.u.save_record({'manual': ['htop'], 'removed': [], 'debs': {}})
        self.touch_upper(A, 'bin/htop')
        self.boot(A)
        self.good()
        self.boot(B)                                   # the new version: its own, new, empty layer
        self.assertFalse(self.u.has_changes(B))
        self.assertEqual(self.u.load_layer(B)['reapply']['state'], 'pending')
        self.assertTrue(os.path.exists(os.path.join(self.state, 'layers', A, 'upper', 'bin', 'htop')))
        self.assertEqual(self.u.load_layer(A)['pending'], 0)
        self.assertTrue(self.boot(A))                  # rollback / automatic fallback: the old one, as it was

    def test_boot_never_raises(self):
        with mock.patch.object(self.u, 'layer_id', side_effect=RuntimeError('x')), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(self.u.cmd_boot(), 0)

    def test_old_layers_are_tidied_but_never_on_an_empty_answer(self):
        for lid in (A, B):
            os.makedirs(os.path.join(self.state, 'layers', lid, 'upper'))
        self.assertEqual(self.u.prune_layers(set(), A, A), [])
        self.assertEqual(self.u.prune_layers({A}, None, A), [])        # ostree didn't say which one is running
        self.assertEqual(self.u.prune_layers({B}, A, A), [])           # the running one isn't listed: odd answer
        self.assertEqual(self.u.prune_layers({A}, A, B), [])           # not the one running here
        self.assertTrue(os.path.isdir(os.path.join(self.state, 'layers', B)))
        self.assertEqual(self.u.prune_layers({A}, A, A), [B])
        self.assertTrue(os.path.isdir(os.path.join(self.state, 'layers', A)))

    def test_units_the_layer_brought(self):
        upper = os.path.join(self.tmp, 'upper')
        os.makedirs(os.path.join(upper, 'lib', 'systemd', 'system'))
        open(os.path.join(upper, 'lib', 'systemd', 'system', 'tailscaled.service'), 'w').close()
        etc = os.path.join(self.tmp, 'etc-systemd')
        os.makedirs(os.path.join(etc, 'multi-user.target.wants'))
        for u in ('tailscaled.service', 'sshd.service'):
            os.symlink('/usr/lib/systemd/system/' + u, os.path.join(etc, 'multi-user.target.wants', u))
        self.assertEqual(self.u.wanted_units(upper, etc), ['tailscaled.service'])


class Reapply(Base):
    def test_plan(self):
        rec = {'manual': ['htop', 'code', 'gone-pkg', 'bash'], 'removed': ['xfce4', 'never-there'], 'debs': {'code': 'code.deb'}}
        rm, repo, files, missing = self.u.plan_reapply(rec, {'bash', 'xfce4'}, lambda p: p == 'htop', lambda d: True)
        self.assertEqual(rm, ['xfce4'])
        self.assertEqual(repo, ['htop'])
        self.assertEqual(files, [os.path.join(self.state, 'debs', 'code.deb')])
        self.assertEqual(missing, ['gone-pkg'])

    def run_reapply(self, results):
        calls = []

        def fake(cmd, timeout=600, env=None, input=None):
            calls.append(cmd)
            for key, val in results.items():
                if key(cmd):
                    return val
            return 0, ''
        with mock.patch.object(self.u, 'run', side_effect=fake), \
                mock.patch.object(self.u, 'installed_set', return_value={'bash'}), \
                mock.patch.object(self.u, 'has_candidate', return_value=True), \
                mock.patch.object(self.u, 'cmd_record'), mock.patch.object(self.u.time, 'sleep'), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            out = self.u.reapply(A, say=lambda *a: None)
        return out, calls

    def test_puts_packages_back(self):
        self.u.save_record({'manual': ['htop', 'vlc'], 'removed': [], 'debs': {}})
        out, calls = self.run_reapply({})
        self.assertEqual(out['state'], 'done')
        self.assertIn(['apt-get', 'install', '-y'] + self.u.DPKG_OPTS + ['htop', 'vlc'], calls)
        self.assertIsNone(self.u.status()['notice'])

    def test_one_bad_package_does_not_stop_the_others_and_shows_a_notice(self):
        self.u.save_record({'manual': ['htop', 'broken'], 'removed': [], 'debs': {}})
        bad = lambda c: c[:2] == ['apt-get', 'install'] and 'broken' in c
        out, calls = self.run_reapply({bad: (100, 'E: broken is broken')})
        self.assertEqual(out['state'], 'failed')
        self.assertEqual(out['failed'], ['broken'])
        self.assertIn(['apt-get', 'install', '-y'] + self.u.DPKG_OPTS + ['htop'], calls)
        n = self.u.status()['notice']
        self.assertEqual(n['kind'], 'reapply-failed')
        self.assertIn('broken', n['text'])

    def test_offline_tries_again_next_start(self):
        self.u.save_record({'manual': ['htop'], 'removed': [], 'debs': {}})
        upd = lambda c: c[:2] == ['apt-get', 'update']
        real = os.path.isdir
        with mock.patch.object(self.u.os.path, 'isdir', side_effect=lambda p: False if p == '/var/lib/apt/lists' else real(p)):
            out, calls = self.run_reapply({upd: (100, 'Temporary failure resolving')})
        self.assertEqual(out['state'], 'pending')
        self.assertFalse(any(c[:2] == ['apt-get', 'install'] for c in calls))
        self.assertEqual(self.u.status()['notice']['kind'], 'reapply-offline')

    def test_removing_an_image_package_only_when_nothing_else_goes(self):
        self.u.save_record({'manual': [], 'removed': ['bash'], 'debs': {}})
        sim = lambda c: c[:3] == ['apt-get', '-s', 'remove']
        out, calls = self.run_reapply({sim: (0, 'Remv bash [5]\nRemv everything-else [1]\n')})
        self.assertFalse(any(c[:3] == ['apt-get', 'remove', '-y'] for c in calls))
        out, calls = self.run_reapply({sim: (0, 'Remv bash [5]\n')})
        self.assertTrue(any(c[:3] == ['apt-get', 'remove', '-y'] for c in calls))

    def test_after_start_runs_only_when_pending_and_writable(self):
        with mock.patch.object(self.u, 'run', return_value=(0, '')), mock.patch.object(self.u, 'mounted', return_value=True), \
                mock.patch.object(self.u, 'reapply') as r:
            self.u.save_layer(A, {'reapply': {'state': 'done'}})
            self.u.cmd_after_start()
            r.assert_not_called()
            self.u.save_layer(A, {'reapply': {'state': 'pending'}})
            self.u.cmd_after_start()
            r.assert_called_once()
        with mock.patch.object(self.u, 'run', return_value=(0, '')), mock.patch.object(self.u, 'mounted', return_value=False), \
                mock.patch.object(self.u, 'reapply') as r:
            self.u.cmd_after_start()
            r.assert_not_called()


class Switch(Base):
    def test_lock_and_unlock(self):
        with mock.patch.object(self.u.os, 'geteuid', return_value=0), \
                mock.patch.object(self.u, 'run', side_effect=self.fake_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted):
            self.mounted = True
            s = self.u.cmd_lock()
            self.assertTrue(os.path.exists(self.lockfile))
            self.assertEqual(s['state'], 'locks-at-restart')
            self.assertFalse(self.boot())
            with mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted):
                self.assertEqual(self.u.status()['state'], 'locked')
            s = self.u.cmd_unlock()
            self.assertFalse(os.path.exists(self.lockfile))
            self.assertEqual(s['state'], 'unlocked')

    def test_unlocking_after_an_update_while_locked_puts_packages_back(self):
        self.u.save_record({'manual': ['htop'], 'removed': [], 'debs': {}})
        os.makedirs(os.path.dirname(self.lockfile))
        open(self.lockfile, 'w').close()
        self.boot(B)
        with mock.patch.object(self.u.os, 'geteuid', return_value=0), \
                mock.patch.object(self.u, 'run', side_effect=self.fake_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted):
            self.u.cmd_unlock()
        self.assertIn(['systemctl', 'start', '--no-block', 'hearth-unlock-reapply.service'], self.calls)

    def test_needs_root(self):
        with mock.patch.object(self.u.os, 'geteuid', return_value=1000):
            with self.assertRaises(self.u.Fail):
                self.u.cmd_lock()

    def test_status_json_from_the_command_line(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), mock.patch.object(self.u, 'mounted', return_value=True):
            self.assertEqual(self.u.main(['hearth-unlock', '--status', '--json']), 0)
        s = json.loads(out.getvalue())
        self.assertEqual(s['state'], 'unlocked')
        self.assertTrue(s['available'])

    def test_install_deb_refused_when_locked(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), mock.patch.object(self.u.os, 'geteuid', return_value=0), \
                mock.patch.object(self.u, 'mounted', return_value=False):
            self.assertEqual(self.u.main(['hearth-unlock', '--install-deb', '/tmp/x.deb']), 1)
        self.assertIn('locked', out.getvalue())



class BootSafety(Base):
    """The boot-safety review: an interrupted dpkg, a failed mount, the image's /usr for the boot writers, the stop."""

    def write_status(self, lid, status_line, journal=False):
        d = os.path.join(self.state, 'layers', lid, 'upper', 'lib', 'sysimage', 'dpkg')
        os.makedirs(os.path.join(d, 'updates'), exist_ok=True)
        with open(os.path.join(d, 'status'), 'w') as f:
            f.write(f'Package: sl\n{status_line}\nVersion: 5\n\nPackage: bash\nStatus: install ok installed\n')
        if journal:
            with open(os.path.join(d, 'updates', '0001'), 'w') as f:
                f.write('Package: sl\nStatus: install ok half-configured\n')

    def test_interrupted_dpkg_is_seen(self):
        up = os.path.join(self.state, 'layers', A, 'upper')
        self.assertFalse(self.u.dpkg_interrupted(up))
        self.write_status(A, 'Status: install ok installed')
        self.assertFalse(self.u.dpkg_interrupted(up))
        for st in ('Status: install ok half-installed', 'Status: install ok unpacked', 'Status: install reinstreq half-installed',
                   'Status: install ok half-configured', 'Status: install ok triggers-awaited'):
            self.write_status(A, st)
            self.assertTrue(self.u.dpkg_interrupted(up), st)
        self.write_status(A, 'Status: install ok installed', journal=True)
        self.assertTrue(self.u.dpkg_interrupted(up))

    def test_interrupted_dpkg_starts_without_the_layer_once_then_repairs(self):
        self.touch_upper(A)
        self.write_status(A, 'Status: install ok half-configured')
        self.assertFalse(self.boot())
        self.assertTrue(self.u.load_layer(A)['repair'])
        self.assertFalse(self.u.load_layer(A).get('suspended'))    # this start only
        self.assertEqual(self.u.status()['notice']['kind'], 'dpkg-interrupted')
        self.assertEqual(self.u.status()['state'], 'repairing')

        def fixing_run(cmd, timeout=600, env=None, input=None):
            r = self.fake_run(cmd, timeout, env, input)
            if cmd[:2] == ['dpkg', '--configure']:
                self.write_status(A, 'Status: install ok installed')
            return r
        with mock.patch.object(self.u, 'run', side_effect=fixing_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted), \
                mock.patch.object(self.u, 'reapply'), contextlib.redirect_stderr(io.StringIO()):
            self.u.cmd_after_start()
        self.assertIn(['dpkg', '--configure', '-a', '--force-confdef', '--force-confold'], self.calls)
        self.assertFalse(self.u.load_layer(A)['repair'])
        self.assertIsNone(self.u.status()['notice'])
        self.assertTrue(self.boot())

    def test_repair_that_fails_keeps_starting_without_the_layer(self):
        self.touch_upper(A)
        self.write_status(A, 'Status: install reinstreq half-installed')
        self.boot()
        with mock.patch.object(self.u, 'run', side_effect=self.fake_run), \
                mock.patch.object(self.u, 'mounted', side_effect=lambda: self.mounted), \
                mock.patch.object(self.u, 'reapply'), contextlib.redirect_stderr(io.StringIO()):
            self.u.cmd_after_start()
        self.assertTrue(self.u.load_layer(A)['repair'])
        self.assertFalse(self.boot())

    def test_a_failed_mount_is_no_reason_for_a_safe_start(self):
        self.touch_upper(A)
        self.mount_fails = True
        self.assertFalse(self.boot())
        self.assertEqual(self.u.load_layer(A).get('pending', 0), 0)
        self.mount_fails = False
        self.assertTrue(self.boot())

    def test_image_usr_kept_before_the_layer_goes_over(self):
        self.boot()
        img = os.environ['HEARTH_UNLOCK_IMAGE_USR']
        binds = [c for c in self.calls if c[0] == 'mount' and img in c]
        self.assertEqual(binds[0], ['mount', '--bind', '/usr', img])
        self.assertIn(['mount', '--make-private', img], binds)     # the layer over /usr mustn't reach it
        first_overlay = next(i for i, c in enumerate(self.calls) if c[:3] == ['mount', '-t', 'overlay'])
        self.assertLess(self.calls.index(binds[-1]), first_overlay)

    def test_image_usr_never_bound_from_the_layer(self):
        with open(self.mountinfo, 'w') as f:
            f.write('1234 22 0:55 / /usr rw - overlay hearth-usr rw,lowerdir=/usr\n')
        with mock.patch.object(self.u, 'run', side_effect=self.fake_run):
            self.assertFalse(self.u.bind_image_usr())
        self.assertFalse(any(c[:2] == ['mount', '--bind'] for c in self.calls))

    def test_reapply_stops_for_a_shutdown_and_waits_for_the_next_start(self):
        self.u.save_record({'manual': ['htop'], 'removed': [], 'debs': {}})
        os.makedirs(os.path.dirname(os.environ['HEARTH_UNLOCK_STOP']), exist_ok=True)
        open(os.environ['HEARTH_UNLOCK_STOP'], 'w').close()
        with mock.patch.object(self.u, 'run', side_effect=self.fake_run), contextlib.redirect_stderr(io.StringIO()):
            r = self.u.reapply(A, say=lambda *a: None)
        self.assertEqual(r['state'], 'pending')
        self.assertFalse(any(c[:2] == ['apt-get', 'install'] for c in self.calls))

    def test_stop_waits_for_dpkg(self):
        busy = iter([0, 0, 1, 1])   # dpkg running twice, then gone
        with mock.patch.object(self.u, 'run', side_effect=lambda cmd, **k: (next(busy, 1), '')), \
                mock.patch.object(self.u, 'lock_free', return_value=True), mock.patch.object(self.u.time, 'sleep') as sl:
            self.u.cmd_stop_wait(limit=60)
        self.assertTrue(os.path.exists(os.environ['HEARTH_UNLOCK_STOP']))
        self.assertGreaterEqual(sl.call_count, 1)


class EtcGuard(unittest.TestCase):
    """The generator that lifts /etc settings pointing into the layer on starts without it."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-etc-guard-')
        self.etc = os.path.join(self.tmp, 'etc')
        self.usr = os.path.join(self.tmp, 'usr')
        self.early = os.path.join(self.tmp, 'early')
        self.x11 = os.path.join(self.tmp, 'default-display-manager')
        os.makedirs(self.etc)
        os.makedirs(self.usr)
        for u in ('sddm.service', 'graphical.target', 'getty@.service'):
            open(os.path.join(self.usr, u), 'w').close()
        with open(self.x11, 'w') as f:
            f.write('/usr/bin/sddm\n')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def gen(self):
        env = dict(os.environ, HEARTH_ETC_UNITS=self.etc, HEARTH_USR_UNITS=self.usr, HEARTH_X11_DM=self.x11)
        r = subprocess.run(['sh', str(GENERATOR), os.path.join(self.tmp, 'n'), self.early, os.path.join(self.tmp, 'l')],
                           env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        return r

    def test_nothing_to_do_writes_nothing(self):
        os.symlink(os.path.join(self.usr, 'sddm.service'), os.path.join(self.etc, 'display-manager.service'))
        self.gen()
        self.assertFalse(os.path.exists(self.early) and os.listdir(self.early))

    def test_a_display_manager_from_the_layer_falls_back_to_hearths(self):
        os.symlink('/usr/lib/systemd/system/gdm3.service', os.path.join(self.etc, 'display-manager.service'))
        with open(self.x11, 'w') as f:
            f.write('/usr/sbin/gdm3\n')
        self.gen()
        self.assertEqual(os.readlink(os.path.join(self.early, 'display-manager.service')), os.path.join(self.usr, 'sddm.service'))
        with open(os.path.join(self.early, 'sddm.service.d', '50-hearth-etc-guard.conf')) as f:
            self.assertIn('ExecStartPre=\n', f.read())

    def test_default_target_and_units_and_drop_ins(self):
        os.symlink('/usr/lib/systemd/system/kodi.target', os.path.join(self.etc, 'default.target'))
        with open(os.path.join(self.etc, 'getty@.service'), 'w') as f:
            f.write('[Service]\nExecStart=-/usr/sbin/layer-agetty %I\n')
        os.makedirs(os.path.join(self.etc, 'sddm.service.d'))
        with open(os.path.join(self.etc, 'sddm.service.d', 'x.conf'), 'w') as f:
            f.write('[Service]\nExecStart=\nExecStart=/usr/bin/not-here\n')
        with open(os.path.join(self.etc, 'sddm.service.d', 'fine.conf'), 'w') as f:
            f.write('[Service]\nExecStartPre=/bin/true\nEnvironment=A=1\n')
        self.gen()
        self.assertEqual(os.readlink(os.path.join(self.early, 'default.target')), os.path.join(self.usr, 'graphical.target'))
        self.assertEqual(os.readlink(os.path.join(self.early, 'getty@.service')), os.path.join(self.usr, 'getty@.service'))
        self.assertEqual(os.path.getsize(os.path.join(self.early, 'sddm.service.d', 'x.conf')), 0)
        self.assertFalse(os.path.exists(os.path.join(self.early, 'sddm.service.d', 'fine.conf')))

    def test_masked_units_stay_masked(self):
        os.symlink('/dev/null', os.path.join(self.etc, 'getty@.service'))
        self.gen()
        self.assertFalse(os.path.lexists(os.path.join(self.early, 'getty@.service')))

class Files(unittest.TestCase):
    def test_units(self):
        boot = (UNITS / 'hearth-unlock.service').read_text()
        self.assertIn('DefaultDependencies=no', boot)
        self.assertIn('Before=sysinit.target', boot)
        self.assertIn('ExecStart=-/usr/bin/hearth-unlock --boot', boot)   # a failure never fails the start
        self.assertIn('TimeoutStartSec=', boot)
        self.assertNotIn('Requires=', boot)
        for u in ('hearth-unlock-units', 'hearth-unlock-good', 'hearth-unlock-reapply'):
            t = (UNITS / (u + '.service')).read_text()
            self.assertIn('Type=simple', t, u)   # nothing in the start waits for these
            self.assertIn('ExecStart=-', t, u)
        re_ = (UNITS / 'hearth-unlock-reapply.service').read_text()
        self.assertNotIn('network-online', re_)
        self.assertIn('ExecStop=-/usr/bin/hearth-unlock --stop-wait', re_)
        self.assertIn('TimeoutStopSec=', re_)
        self.assertIn('TimeoutStartSec=120', boot)

    def test_boot_writers_run_from_the_image(self):
        for u in ('hearth-bls-grub', 'hearth-bls-grub-refresh', 'hearth-boot-guard', 'hearth-boot-guard-refresh',
                  'hearth-boot-good', 'ostree-finalize-staged', 'bootc-fetch-apply-updates', 'hearth-recovery'):
            t = (UNITS / (u + '.service.d') / '20-hearth-image-usr.conf').read_text()
            self.assertIn('BindReadOnlyPaths=-/run/hearth/usr-image:/usr', t, u)
            self.assertIn('After=hearth-unlock.service', t, u)
        ld = (UNITS / 'hearth-unlock-ldcache.service').read_text()
        self.assertIn('After=hearth-unlock.service ostree-finalize-staged.service', ld)   # stops before finalizing
        self.assertIn('BindReadOnlyPaths=-/run/hearth/usr-image:/usr', ld)
        self.assertTrue(os.access(GENERATOR, os.X_OK))

    def test_hooks_and_pin(self):
        apt = (CONF / 'apt.conf').read_text()
        self.assertIn('Dir::State::extended_states "/usr/lib/sysimage/apt/extended_states";', apt)
        self.assertIn('--stash-debs || cat >/dev/null || true', apt)
        self.assertIn('--record-later || true', (CONF / 'dpkg.cfg').read_text())
        pin = (CONF / 'boot-chain.pref').read_text()
        for p in ('grub*', 'shim*', 'linux-image*', 'bootc', 'bootupd', 'dracut', 'ostree', 'dkms', '*-dkms',
                  'amd64-microcode', 'intel-microcode', 'fwupd*', 'mokutil', 'memtest86+'):
            self.assertIn(' ' + p, ' ' + pin.split('Package:', 1)[1].split('\n', 1)[0] + ' ')
        self.assertIn('Pin-Priority: -1', pin)

    def test_build_enables_and_installs_them(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('systemctl enable hearth-unlock.service hearth-unlock-units.service hearth-unlock-good.service hearth-unlock-reapply.service', b)
        self.assertIn('/etc/apt/apt.conf.d/50-hearth-unlock', b)
        self.assertIn('/etc/dpkg/dpkg.cfg.d/hearth-unlock', b)
        self.assertIn('/etc/apt/preferences.d/hearth-boot-chain', b)
        self.assertIn('/etc/apt/preferences.d/hearth-image-core', b)
        self.assertIn('shared/default-x-display-manager select sddm', b)
        self.assertIn('C /var/cache/debconf', b)
        for f in ('update-initramfs', 'update-grub', 'grub-mkconfig'):
            self.assertIn('/usr/sbin/' + f, b)
        self.assertIn('rm -f /etc/kernel/postinst.d/*', b)
        self.assertIn('hearth-unlock-ldcache.service', b)
        # the hooks go in only after the last apt run of the build
        self.assertGreater(b.index('/etc/dpkg/dpkg.cfg.d/hearth-unlock'), b.rindex('apt-get clean'))


class Admin(unittest.TestCase):
    def setUp(self):
        self.ha = load(ADMIN, 'hearth_admin_unlock')

    def main(self, argv, have=True):
        out, execd = io.StringIO(), []
        with mock.patch.object(self.ha.os, 'geteuid', return_value=0), \
                mock.patch.object(self.ha.os, 'access', side_effect=lambda p, m: have if p == self.ha.UNLOCK else os.access(p, m)), \
                mock.patch.object(self.ha.os, 'execv', side_effect=lambda p, a: execd.append(a) or sys.exit(0)), \
                mock.patch.dict(os.environ, {'PKEXEC_UID': str(os.getuid())}), contextlib.redirect_stdout(out):
            try:
                self.ha.main(argv)
                code = 0
            except SystemExit as e:
                code = e.code
        return code, out.getvalue(), execd

    def test_switch_goes_to_hearth_unlock(self):
        for act, flag in (('on', '--unlock'), ('off', '--lock'), ('resume', '--resume'), ('dismiss', '--dismiss'),
                          ('reapply', '--reapply-later')):
            code, _, ex = self.main(['hearth-admin', 'unlock', act])
            self.assertEqual(ex, [[self.ha.UNLOCK, flag, '--json']], act)

    def test_unknown_action_refused(self):
        code, out, ex = self.main(['hearth-admin', 'unlock', '--install-deb'])
        self.assertEqual(ex, [])
        self.assertNotEqual(code, 0)

    def test_deb_commands(self):
        _, _, ex = self.main(['hearth-admin', 'deb-install', '/home/me/a.deb'])
        self.assertEqual(ex, [[self.ha.UNLOCK, '--install-deb', '/home/me/a.deb']])
        _, _, ex = self.main(['hearth-admin', 'deb-remove', 'a'])
        self.assertEqual(ex, [[self.ha.UNLOCK, '--remove-package', 'a']])

    def test_anatase_is_always_locked(self):
        code, out, ex = self.main(['hearth-admin', 'unlock', 'on'], have=False)
        self.assertEqual(code, 1)
        self.assertIn('Anatase', out)
        self.assertEqual(ex, [])


class RecoveryItem(unittest.TestCase):
    def test_menu_offers_it_only_when_the_layer_is_on(self):
        r = load(RECOVERY, 'hearth_recovery_unlock')
        mi = '1 2 0:5 / /usr rw - overlay hearth-usr rw,lowerdir=/usr\n'
        with mock.patch.object(r.os, 'access', return_value=True), mock.patch('builtins.open', mock.mock_open(read_data=mi)):
            self.assertTrue(r.unlock_on())
        with mock.patch.object(r.os, 'access', return_value=False):
            self.assertFalse(r.unlock_on())


class NativeDebs(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-debnative-test-')
        os.environ['HOME'] = self.tmp
        os.environ.pop('XDG_DATA_HOME', None)
        self.fake = os.path.join(self.tmp, 'hearth-unlock')

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_unlock(self, writable):
        with open(self.fake, 'w') as f:
            f.write('#!/bin/sh\necho \'{"available": true, "writable": %s}\'\n' % ('true' if writable else 'false'))
        os.chmod(self.fake, 0o755)
        os.environ['HEARTH_UNLOCK_BIN'] = self.fake
        return load(DEBBOX, 'hearth_debbox_native')

    def test_native_when_unlocked(self):
        self.assertTrue(self.fake_unlock(True).native_ready())

    def test_box_when_locked_or_missing(self):
        self.assertFalse(self.fake_unlock(False).native_ready())
        os.environ['HEARTH_UNLOCK_BIN'] = os.path.join(self.tmp, 'nothing')
        self.assertFalse(load(DEBBOX, 'hearth_debbox_none').native_ready())

    def test_install_goes_through_hearth_admin(self):
        d = self.fake_unlock(True)
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
        import make_test_deb
        deb = os.path.join(self.tmp, 'hearth-hello_1.0_all.deb')
        make_test_deb.make(deb)
        info = d.inspect(deb)
        self.assertTrue(info['native'])
        self.assertEqual(info['limits'], d.NATIVE_LIMITS)
        seen = []

        def fake_stream(cmd, on_line, timeout=1800):
            seen.append(cmd)
            on_line('progress: 50 Installing')
            return 0, '{"ok": true, "package": "hearth-hello"}\n'
        with mock.patch.object(d, 'stream', side_effect=fake_stream), mock.patch.object(d, 'native_apps', return_value=[]), \
                contextlib.redirect_stdout(io.StringIO()):
            out = d.install(deb)
        self.assertEqual(seen[0], ['pkexec', '/usr/libexec/nimbus/hearth-admin', 'deb-install', deb])
        self.assertTrue(out['native'])
        self.assertTrue(d.load_records()['hearth-hello']['native'])


if __name__ == '__main__':
    unittest.main()
