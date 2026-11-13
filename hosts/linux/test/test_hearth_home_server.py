"""Tests for Settings › Home server, stage 1 (docs/HOME-SERVER.md): the root helper's share config and its checks
(never /, /etc or the system drive), the "Tailscale only" firewall rules, services off by default, snapshot scheduling
and retention, the double-confirmed format (dry run), settings written only when changed, and the hide-until-seen
rule for Command Center and hardware items. The system's commands are replaced by recorders."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import pwd
import re
import tempfile
import time
import unittest

os.environ['XDG_STATE_HOME'] = tempfile.mkdtemp()
os.environ['XDG_CONFIG_HOME'] = tempfile.mkdtemp()
os.environ.pop('DBUS_SESSION_BUS_ADDRESS', None)

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIBEXEC = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
UNITS = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hh = load(LIBEXEC / 'hearth-home-server-helper', 'hearth_home_server_helper_t')
os.environ['HEARTH_HOME_SERVER_ROOT'] = str(LIBEXEC / 'hearth-home-server-helper')
hs = load(SHELL / 'hearth-home-server', 'hearth_home_server_t')
USER = pwd.getpwuid(os.getuid()).pw_name


class Rec:
    """Stands in for the system's commands and remembers them."""

    def __init__(self, tools=('smbd', 'smbpasswd', 'testparm', 'nft', 'btrfs', 'tailscaled', 'smartctl'), enabled=()):
        self.calls = []
        self.tools = set(tools)
        self.enabled = set(enabled)

    def run(self, cmd, stdin=None, timeout=60):
        self.calls.append(list(cmd))
        if cmd[:2] == ['systemctl', 'cat']:
            return (0, '') if cmd[2] in ('smbd.service', 'tailscaled.service') else (1, '')
        if cmd[:2] == ['systemctl', 'is-enabled']:
            return (0, 'enabled') if cmd[2] in self.enabled else (1, 'disabled')
        if cmd[:2] == ['systemctl', 'enable'] or cmd[:3] == ['systemctl', 'enable', '--now']:
            self.enabled.add(cmd[-1])
        if cmd[:2] == ['systemctl', 'disable']:
            self.enabled.discard(cmd[-1])
        return 0, ''

    def which(self, name):
        return '/usr/bin/' + name if name in self.tools else None

    def ran(self, *prefix):
        return [c for c in self.calls if c[:len(prefix)] == list(prefix)]


class Base(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.home = os.path.join(self.d, 'home', USER)
        for p in ('Videos', 'Games', 'Work files', '.ssh', 'Media/.secret'):
            os.makedirs(os.path.join(self.home, p))
        self.etc = os.path.join(self.d, 'etc')
        os.makedirs(os.path.join(self.etc, 'samba'))
        with open(os.path.join(self.etc, 'samba', 'smb.conf'), 'w') as f:
            f.write('[global]\n   workgroup = WORKGROUP\n')
        self.saved = {}
        for k, v in (('SMB_CONF', 'samba/smb.conf'), ('OUR_CONF', 'samba/hearth-home-server.conf'), ('STORAGE_CONF', 'samba/hearth-storage.conf'),
                     ('STARTED', 'samba/.hearth-home-server-started'), ('PLAN', 'hearth/home-server.json'), ('NFT_FILE', 'hearth/home-server.nft'),
                     ('SMBD_DROPIN', 'systemd/system/smbd.service.d/hearth-home-server.conf'), ('EXIT_SYSCTL', 'sysctl.d/99-hearth-exit-node.conf'),
                     ('FSTAB', 'fstab')):
            self.saved[k] = getattr(hh, k)
            setattr(hh, k, os.path.join(self.etc, v))
        self.rec = Rec()
        self.saved_fns = {k: getattr(hh, k) for k in ('run', 'which', 'mount_source', 'disks_of', 'system_disks', 'mount_info', 'is_subvol_root',
                                                      'game_running', 'lsblk', 'smb_unit', 'caller', 'drive_bases', 'user_settings',
                                                      'light_resumed', 'getpw', 'hearth_people', 'run_env', 'hostname',
                                                      'smb_sessions', 'jobs_running', 'someone_using')}
        hh.user_settings = lambda home: {}
        hh.light_resumed = lambda now=None: False
        for k, v in (('BACKUP_DIR', 'hearth/backup'), ('BACKUP_STATE', 'state/home-server-backups.json'), ('BACKUP_CACHE', 'cache/restic'),
                     ('RCLONE_CONF', 'hearth/backup/rclone.conf'), ('RECEIVE_DIR', 'hearth/backup-receiver'),
                     ('RECEIVE_DROPIN', 'systemd/system/hearth-home-server-receive.service.d/hearth.conf'),
                     ('ALERT_STATE', 'state/home-server-alerts.json'), ('POWER_STATE', 'run/home-server-power.json'),
                     ('WAKE_DROPIN', 'systemd/system/hearth-home-server-wake.timer.d/hearth.conf'), ('NUT_DIR', 'nut'),
                     ('NUT_STARTED', 'nut/.hearth-started')):
            self.saved[k] = getattr(hh, k)
            setattr(hh, k, os.path.join(self.etc, v))
        hh.run = self.rec.run
        hh.which = self.rec.which
        if hh.hsh:
            self.saved_hsh = (hh.hsh.run, hh.hsh.which, hh.hsh.STARTED)
            hh.hsh.run = self.rec.run
            hh.hsh.which = self.rec.which
            hh.hsh.STARTED = os.path.join(self.etc, 'samba', '.hearth-storage-started')
        hh.smb_unit = lambda: 'smbd'
        hh.caller = lambda: (USER, self.home)
        hh.system_disks = lambda: {'nvme0n1'}
        hh.mount_source = lambda p: '/dev/sda1' if p.startswith('/mnt/big') or p.startswith(os.path.join(self.d, 'mnt')) else '/dev/nvme0n1p3'
        hh.disks_of = lambda dev: {'sda'} if 'sda' in dev else ({'nvme0n1'} if 'nvme0n1' in dev else set())
        os.environ['PKEXEC_UID'] = str(os.getuid())

    def tearDown(self):
        for k, v in self.saved.items():
            setattr(hh, k, v)
        for k, v in self.saved_fns.items():
            setattr(hh, k, v)
        if hh.hsh:
            hh.hsh.run, hh.hsh.which, hh.hsh.STARTED = self.saved_hsh
        os.environ.pop('PKEXEC_UID', None)

    def share(self, name, rel, **kw):
        s = {'id': 'a1b2c3' + str(len(name)), 'name': name, 'path': os.path.join(self.home, rel), 'write': False, 'reach': 'home', 'recycle': True}
        s.update(kw)
        return s


# ---------------------------------------------------------------------------------------------------- shares

class ShareConfig(Base):
    def test_good_shares_make_a_hardened_samba_config(self):
        good, missing = hh.check_shares([self.share('Games', 'Games', write=True), self.share('Videos', 'Videos')], USER, self.home, set())
        self.assertEqual([s['name'] for s in good], ['Games', 'Videos'])
        text = hh.conf_text(good, USER, False)
        for line in ('server min protocol = SMB3', 'map to guest = Never', 'restrict anonymous = 2', 'usershare allow guests = no'):
            self.assertIn(line, text)
        self.assertIn('[Games]', text)
        self.assertIn('   valid users = %s' % USER, text)
        self.assertIn('   guest ok = no', text)
        self.assertIn('   veto files = /.*/', text)
        games = text.split('[Games]')[1].split('[Videos]')[0]
        videos = text.split('[Videos]')[1]
        self.assertIn('read only = no', games)
        self.assertIn('vfs objects = recycle', games)
        self.assertIn('read only = yes', videos)
        self.assertNotIn('vfs objects = recycle', videos, 'a read-only share has nothing to recycle')
        self.assertNotIn('guest ok = yes', text)
        self.assertNotIn('hosts allow', text, 'home network shares are not limited to Tailscale')

    def test_bad_names_are_refused(self):
        for bad in ('global', 'Homes', 'a;b', 'x%u', '', ' lead', '../up', 'a' * 41, 'name"q', 'ipc$', 'with\nnewline'):
            with self.assertRaises(hh.Fail, msg=bad):
                hh.check_shares([self.share(bad, 'Games')], USER, self.home, set())

    def test_root_etc_system_and_hidden_paths_are_refused(self):
        bad = ['/', '/etc', '/etc/samba', '/usr', '/boot', '/sysroot', '/ostree/repo', '/root', '/proc', self.home, self.home + '/',
               os.path.join(self.home, '.ssh'), os.path.join(self.home, 'Media', '.secret'), os.path.join(self.home, 'Games', '..', '.ssh'),
               os.path.join(self.home, 'Games/../.ssh'), self.home + '/Games;rm -rf ~', self.home + '/$(reboot)', self.home + '/a`b`',
               self.home + '/x%u', 'relative/path', self.d]
        for p in bad:
            real, why = hh.path_problem(p, USER, self.home, set())
            self.assertIsNone(real, p)
            self.assertTrue(why, p)
        self.assertEqual(hh.path_problem(os.path.join(self.home, 'Work files'), USER, self.home, set())[0], os.path.realpath(os.path.join(self.home, 'Work files')))

    def test_a_link_out_of_home_is_judged_by_where_it_points(self):
        link = os.path.join(self.home, 'Videos', 'etc-link')
        os.symlink('/etc', link)
        self.assertIsNone(hh.path_problem(link, USER, self.home, set())[0])

    def test_the_system_drive_is_never_shared_outside_home(self):
        # A drive's folder: refused when that drive is the system's, fine on another drive.
        mnt = os.path.join(self.d, 'mnt')
        os.makedirs(os.path.join(mnt, 'Movies'))
        hh.drive_bases = lambda user: [mnt]
        hh.mount_source = lambda p: '/dev/nvme0n1p3'
        real, why = hh.path_problem(os.path.join(mnt, 'Movies'), USER, self.home, {'nvme0n1'})
        self.assertIsNone(real)
        self.assertIn('system drive', why)
        hh.mount_source = lambda p: '/dev/sda1'
        real, why = hh.path_problem(os.path.join(mnt, 'Movies'), USER, self.home, {'nvme0n1'})
        self.assertEqual(real, os.path.realpath(os.path.join(mnt, 'Movies')), why)
        self.assertIsNone(hh.path_problem(mnt, USER, self.home, {'nvme0n1'})[0], 'a whole drive base is never shared')
        # Folders in your Home on the system drive are fine.
        hh.mount_source = lambda p: '/dev/nvme0n1p3'
        self.assertIsNotNone(hh.path_problem(os.path.join(self.home, 'Games'), USER, self.home, {'nvme0n1'})[0])

    def test_a_missing_drive_is_left_out_not_refused(self):
        good, missing = hh.check_shares([self.share('Games', 'Games'), self.share('Old', 'Unplugged')], USER, self.home, set())
        self.assertEqual([s['name'] for s in good], ['Games'])
        self.assertEqual(missing, ['Old'])

    def test_names_never_clash_with_hearth_storage(self):
        with self.assertRaises(hh.Fail) as e:
            hh.check_shares([self.share('Games', 'Games')], USER, self.home, set(), taken={'games'})
        self.assertIn('Hearth Storage', str(e.exception))

    def test_apply_writes_the_config_includes_it_and_starts_samba_only_with_a_share(self):
        res = hh.apply({'shares': [self.share('Games', 'Games')]})
        self.assertTrue(res['ok'])
        self.assertIn('[Games]', open(hh.OUR_CONF).read())
        smb = open(hh.SMB_CONF).read()
        self.assertEqual(smb.count(hh.INCLUDE), 1)
        self.assertIn('workgroup = WORKGROUP', smb, 'the rest of smb.conf stays')
        self.assertTrue(self.rec.ran('systemctl', 'enable', '--now', 'smbd'))
        self.assertTrue(os.path.exists(hh.STARTED))
        # Again: the include line stays once.
        hh.apply({'shares': [self.share('Games', 'Games')]})
        self.assertEqual(open(hh.SMB_CONF).read().count(hh.INCLUDE), 1)
        # Everything off: Samba stopped again (Hearth started it), the include gone, the files untouched.
        self.rec.calls.clear()
        hh.apply({'shares': []})
        self.assertFalse(os.path.exists(hh.OUR_CONF))
        self.assertNotIn(hh.INCLUDE, open(hh.SMB_CONF).read())
        self.assertTrue(self.rec.ran('systemctl', 'disable', '--now', 'smbd'))
        self.assertTrue(os.path.isdir(os.path.join(self.home, 'Games')), 'removing a share never deletes its folder')

    def test_a_share_samba_refuses_puts_the_old_config_back(self):
        hh.apply({'shares': [self.share('Games', 'Games')]})
        before = open(hh.OUR_CONF).read()
        orig = self.rec.run
        self.rec.run = lambda cmd, stdin=None, timeout=60: (1, 'bad') if cmd[0] == 'testparm' else orig(cmd, stdin, timeout)
        hh.run = self.rec.run
        with self.assertRaises(hh.Fail):
            hh.apply({'shares': [self.share('Videos', 'Videos')]})
        self.assertEqual(open(hh.OUR_CONF).read(), before)

    def test_a_share_turned_off_is_not_in_samba(self):
        hh.apply({'shares': [self.share('Games', 'Games'), self.share('Videos', 'Videos', on=False)]})
        text = open(hh.OUR_CONF).read()
        self.assertIn('[Games]', text)
        self.assertNotIn('[Videos]', text)

    def test_the_password_goes_only_through_stdin(self):
        if not hh.hsh:
            self.skipTest("Hearth Storage's helper isn't next to this one")
        seen = []
        hh.hsh.run = lambda cmd, stdin=None: (seen.append((cmd, stdin)), (0, ''))[1]
        hh.hsh.smb_password('correct horse\n')
        cmd, stdin = seen[0]
        self.assertNotIn('correct horse', ' '.join(cmd))
        self.assertIn('correct horse', stdin)


# ---------------------------------------------------------------------------------------------------- Tailscale only

class TailscaleOnly(Base):
    def test_nft_rules_let_samba_in_only_through_tailscale(self):
        text = hh.nft_text(hh.SMB_PORTS_TCP, hh.SMB_PORTS_UDP)
        self.assertIn('table inet hearth_home_server', text)
        self.assertIn('delete table inet hearth_home_server', text, 'loading it again replaces the old rules')
        self.assertIn('type filter hook input priority -10; policy accept;', text)
        accept = text.index('iifname { "lo", "tailscale0" } accept')
        drop = text.index('tcp dport { 139, 445 } counter drop')
        self.assertLess(accept, drop, 'Tailscale is let in before everything else is dropped')
        self.assertIn('udp dport { 137, 138 } counter drop', text)
        self.assertNotIn('policy drop', text, 'only these ports, never the whole PC')

    def test_when_the_lan_is_blocked(self):
        home, ts = {'reach': 'home'}, {'reach': 'tailscale'}
        self.assertTrue(hh.lan_blocked([home], True, False), 'Tailscale only for everything')
        self.assertTrue(hh.lan_blocked([ts, ts], False, False), 'every share Tailscale only')
        self.assertFalse(hh.lan_blocked([ts, home], False, False))
        self.assertFalse(hh.lan_blocked([ts], False, True), "Hearth Storage's Windows sharing is for this network")
        self.assertTrue(hh.lan_blocked([ts], True, True))
        self.assertFalse(hh.lan_blocked([], False, False))

    def test_tailscale_only_shares_get_samba_s_own_address_check_too(self):
        good, _ = hh.check_shares([self.share('Work', 'Work files', reach='tailscale'), self.share('Games', 'Games')], USER, self.home, set())
        text = hh.conf_text(good, USER, False)
        work = text.split('[Work]')[1].split('[Games]')[0]
        self.assertIn('hosts allow = 100.64.0.0/10 fd7a:115c:a1e0::/48', work)
        self.assertIn('hosts deny = 0.0.0.0/0 ::/0', work)
        self.assertNotIn('hosts allow', text.split('[Games]')[1])
        everything = hh.conf_text(good, USER, True)
        self.assertEqual(everything.count('hosts allow = 100.64.0.0/10'), 2)

    def test_apply_loads_the_rules_and_keeps_them_for_samba_s_next_start(self):
        hh.apply({'shares': [self.share('Work', 'Work files')], 'tsOnly': True})
        self.assertTrue(os.path.exists(hh.NFT_FILE))
        self.assertTrue(self.rec.ran('nft', '-f', hh.NFT_FILE))
        drop = open(hh.SMBD_DROPIN).read()
        self.assertIn('ExecStartPre=-/usr/sbin/nft -f', drop, "a firewall that can't load never stops Samba")
        self.assertNotIn('Requires=', drop)
        # Off again: the table deleted, the files gone.
        self.rec.calls.clear()
        hh.apply({'shares': [self.share('Work', 'Work files')], 'tsOnly': False})
        self.assertFalse(os.path.exists(hh.NFT_FILE))
        self.assertFalse(os.path.exists(hh.SMBD_DROPIN))
        self.assertTrue(self.rec.ran('nft', 'delete', 'table', 'inet', 'hearth_home_server'))


# ---------------------------------------------------------------------------------------------------- off by default

class OffByDefault(Base):
    def test_settings_start_with_nothing_on(self):
        d = hs.DEFAULTS
        self.assertEqual(d['shares'], [])
        for k in ('tsOnly', 'tsBoot', 'exitNode', 'taildropReceive', 'wakeFromAway', 'snapshots', 'showHidden', 'setupDone'):
            self.assertIs(d[k], False, k)

    def test_the_image_turns_every_new_service_off_and_checks_it(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        block = b.split('# ---- Hearth home server')[1].split('# ---- end of Hearth home server')[0]
        self.assertIn('for u in smbd nmbd samba-ad-dc tailscaled smartmontools smartd nftables ', block)
        self.assertIn('systemctl disable "$u.service"', block)
        self.assertIn('hearth-home-server-snapshots.timer', block.split('HS_TIMERS=')[1].split('\n')[0])
        self.assertIn('for t in $HS_TIMERS; do systemctl disable "$t"', block)
        self.assertIn('systemctl --global disable hearth-taildrop.service', block)
        self.assertIn('is on in the image', block, 'the build fails if one is left on')
        self.assertIn('TS_FPR=2596A99EAAB33821893C0A79458CA832957F5868', block, "Tailscale's key is checked")
        self.assertIn('signed-by=/usr/share/keyrings/tailscale-archive-keyring.gpg', block)
        self.assertNotIn('systemctl enable', block)
        self.assertNotIn('bootloader', block.lower().replace('boot menu', ''))

    def test_units_never_hold_up_starting(self):
        svc = (UNITS / 'system' / 'hearth-home-server-snapshots.service').read_text()
        timer = (UNITS / 'system' / 'hearth-home-server-snapshots.timer').read_text()
        drop = (UNITS / 'user' / 'hearth-taildrop.service').read_text()
        for text in (svc, timer, drop):
            self.assertNotIn('Requires=', text)
            self.assertNotIn('RequiredBy=', text)
            self.assertNotIn('Before=graphical', text)
            for target in ('multi-user.target', 'graphical.target', 'nimbus', 'gamescope', 'steam'):
                self.assertNotIn('WantedBy=' + target, text)
        self.assertNotIn('[Install]', svc, 'only the timer starts it')
        self.assertIn('TimeoutStartSec=', svc)
        self.assertIn('IOSchedulingClass=idle', svc)
        self.assertIn('WantedBy=timers.target', timer)
        self.assertIn('Persistent=false', timer, 'no catch-up run right at start')
        self.assertIn('Restart=on-failure', drop)
        # Nothing ships them enabled.
        for wants in UNITS.glob('**/*.wants'):
            for n in os.listdir(wants):
                self.assertNotIn('hearth-home-server', n)
                self.assertNotIn('hearth-taildrop', n)

    def test_the_session_and_boot_files_are_not_touched(self):
        for p in (ROOT / 'hosts' / 'linux' / 'session' / 'nimbus-session',):
            self.assertNotIn('home-server', p.read_text())
        self.assertNotIn('grub', (LIBEXEC / 'hearth-home-server-helper').read_text().lower())

    def test_tailscale_at_boot_is_enabled_or_disabled_never_stopped(self):
        self.assertEqual(hh.tailscaled_boot(True), 'on')
        self.assertTrue(self.rec.ran('systemctl', 'enable', 'tailscaled'))
        self.assertFalse(self.rec.ran('systemctl', 'enable', '--now', 'tailscaled'))
        hh.tailscaled_boot(False)
        self.assertTrue(self.rec.ran('systemctl', 'disable', 'tailscaled'))
        self.assertFalse(self.rec.ran('systemctl', 'stop', 'tailscaled'))

    def test_polkit_lets_only_the_person_at_the_device_run_it(self):
        rules = (ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'rules.d' / '49-hearth-storage.rules').read_text()
        self.assertIn('/usr/libexec/nimbus/hearth-home-server-helper', rules)
        self.assertIn('subject.local && subject.active && subject.isInGroup("wheel")', rules)


# ---------------------------------------------------------------------------------------------------- snapshots

def name_at(kind, t):
    return hh.snap_name(kind, t)


class Snapshots(Base):
    def test_due_by_kind(self):
        now = time.mktime((2026, 11, 5, 20, 0, 0, 0, 0, -1))
        keep = {'hourly': 24, 'daily': 14, 'weekly': 8}
        self.assertTrue(hh.due('hourly', [], now, keep))
        self.assertFalse(hh.due('hourly', [name_at('hourly', now - 1800)], now, keep))
        self.assertTrue(hh.due('hourly', [name_at('hourly', now - 3600)], now, keep))
        self.assertTrue(hh.due('hourly', [name_at('hourly', now - 3500)], now, keep), 'a timer a few minutes early still counts')
        self.assertFalse(hh.due('daily', [name_at('daily', now - 3 * 3600)], now, keep))
        self.assertTrue(hh.due('daily', [name_at('daily', now - 86400)], now, keep))
        self.assertFalse(hh.due('weekly', [name_at('weekly', now - 3 * 86400)], now, keep))
        self.assertFalse(hh.due('hourly', [], now, dict(keep, hourly=0)), 'keep 0: that kind is off')
        self.assertTrue(hh.due('hourly', [name_at('daily', now - 600)], now, keep), 'a daily one is not an hourly one')

    def test_retention_drops_the_oldest_beyond_each_count_and_only_hearth_s(self):
        now = time.mktime((2026, 11, 5, 20, 0, 0, 0, 0, -1))
        names = [name_at('hourly', now - i * 3600) for i in range(30)] + [name_at('daily', now - i * 86400) for i in range(3)]
        names += ['my-own-snapshot', 'hourly-oops']
        drop = hh.to_prune(names, {'hourly': 24, 'daily': 14, 'weekly': 8})
        self.assertEqual(len(drop), 6)
        self.assertEqual(set(drop), {name_at('hourly', now - i * 3600) for i in range(24, 30)}, 'the oldest six go')
        self.assertNotIn('my-own-snapshot', drop)
        self.assertNotIn('hourly-oops', drop)
        self.assertEqual(hh.to_prune(names, {'hourly': 0, 'daily': 0, 'weekly': 0}).count('my-own-snapshot'), 0)

    def test_snapshot_availability(self):
        hh.mount_info = lambda p: {'target': self.home, 'fstype': 'ext4', 'source': '/dev/sda1'}
        info = hh.snap_info(os.path.join(self.home, 'Games'))
        self.assertFalse(info['ok'])
        self.assertIn("aren't available", info['why'])
        hh.mount_info = lambda p: {'target': os.path.dirname(self.home), 'fstype': 'btrfs', 'source': '/dev/nvme0n1p3'}
        hh.is_subvol_root = lambda p: p == os.path.dirname(self.home)
        info = hh.snap_info(os.path.join(self.home, 'Games'))
        self.assertTrue(info['ok'])
        self.assertEqual(info['subvol'], os.path.dirname(self.home))
        self.assertEqual(info['rel'], os.path.join(USER, 'Games'))
        hh.mount_info = lambda p: {'target': '/', 'fstype': 'btrfs', 'source': '/dev/nvme0n1p3'}
        hh.is_subvol_root = lambda p: p == '/'
        self.assertFalse(hh.snap_info('/var/lib')['ok'], "the system's own volume")

    def test_the_timer_takes_what_is_due_drops_old_ones_and_waits_while_you_play(self):
        vol = os.path.join(self.d, 'vol')
        os.makedirs(os.path.join(vol, 'Games'))
        now = time.mktime((2026, 11, 5, 20, 0, 0, 0, 0, -1))
        store = os.path.join(vol, hh.SNAP_DIR)
        os.makedirs(store)
        for i in range(1, 26):
            os.makedirs(os.path.join(store, name_at('hourly', now - i * 3600)))
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        with open(hh.PLAN, 'w') as f:
            json.dump({'lightMode': True, 'snapshots': True, 'keep': {'hourly': 24, 'daily': 14, 'weekly': 8},
                       'shares': [{'name': 'Games', 'path': os.path.join(vol, 'Games'), 'snapshots': True}]}, f)
        hh.mount_info = lambda p: {'target': vol, 'fstype': 'btrfs', 'source': '/dev/sda1'}
        hh.is_subvol_root = lambda p: p == vol
        hh.game_running = lambda: True
        res = hh.snapshot_run(now)
        self.assertIn('deferred', res)
        self.assertFalse(self.rec.ran('btrfs'), 'nothing runs while you play with Light mode on')
        hh.game_running = lambda: False
        res = hh.snapshot_run(now)
        snaps = [c for c in self.rec.ran('btrfs', 'subvolume', 'snapshot', '-r')]
        self.assertEqual(sorted(os.path.basename(c[-1]).split('-')[0] for c in snaps), ['daily', 'hourly', 'weekly'])
        self.assertTrue(all(c[4] == vol for c in snaps))
        dels = [os.path.basename(c[-1]) for c in self.rec.ran('btrfs', 'subvolume', 'delete')]
        # 25 old + 1 new hourly, keep 24: the two oldest go.
        self.assertEqual(sorted(dels), sorted([name_at('hourly', now - 25 * 3600), name_at('hourly', now - 24 * 3600)]))
        # Light mode off: it runs even while you play.
        with open(hh.PLAN) as f:
            plan = json.load(f)
        plan['lightMode'] = False
        with open(hh.PLAN, 'w') as f:
            json.dump(plan, f)
        hh.game_running = lambda: True
        self.assertNotIn('deferred', hh.snapshot_run(now + 3600))

    def test_old_recycle_bin_files_are_emptied_and_nothing_else(self):
        share = os.path.join(self.home, 'Games')
        rb = os.path.join(share, hh.RECYCLE_DIR, 'sub')
        os.makedirs(rb)
        old, new, keep = os.path.join(rb, 'old.txt'), os.path.join(rb, 'new.txt'), os.path.join(share, 'game.sav')
        for p in (old, new, keep):
            open(p, 'w').close()
        now = time.time()
        os.utime(old, (now - 40 * 86400, now - 40 * 86400))
        os.utime(keep, (now - 400 * 86400, now - 400 * 86400))
        n = hh.recycle_empty({'shares': [{'path': share, 'recycle': True}]}, now)
        self.assertEqual(n, 1)
        self.assertFalse(os.path.exists(old))
        self.assertTrue(os.path.exists(new))
        self.assertTrue(os.path.exists(keep), "files outside the recycle bin are never touched")

    def test_keep_counts_are_checked(self):
        self.assertEqual(hh.clean_keep({'hourly': 9999, 'daily': -3, 'weekly': 'x'}), {'hourly': 168, 'daily': 0, 'weekly': 8})
        self.assertEqual(hh.clean_keep(None), hh.KEEP_DEFAULT)


# ---------------------------------------------------------------------------------------------------- formatting

DISK = {'name': 'sdb', 'path': '/dev/sdb', 'size': 8001563222016, 'type': 'disk', 'model': 'Seagate BarraCuda', 'serial': 'ZA1234',
        'wwn': '0x5000c500', 'children': []}
SYSDISK = {'name': 'nvme0n1', 'path': '/dev/nvme0n1', 'size': 1000204886016, 'type': 'disk', 'model': 'Samsung 990', 'serial': 'S1',
           'children': [{'name': 'nvme0n1p3', 'mountpoints': ['/sysroot']}]}


class Format(Base):
    def setUp(self):
        super().setUp()
        hh.lsblk = lambda: [json.loads(json.dumps(DISK)), json.loads(json.dumps(SYSDISK))]

    def req(self, **kw):
        p = hh.format_plan('/dev/sdb', 'storage', 'Big', '', USER, sys_disks={'nvme0n1'})
        r = {'device': '/dev/sdb', 'use': 'storage', 'label': 'Big', 'token': p['token'], 'confirm1': True, 'confirm2': p['confirm'], 'dryRun': True}
        r.update(kw)
        return r

    def test_the_plan_names_the_drive_and_its_size(self):
        p = hh.format_plan('/dev/sdb', 'storage', 'Big', '', USER, sys_disks={'nvme0n1'})
        self.assertTrue(p['canFormat'])
        self.assertEqual(p['confirm'], 'Seagate BarraCuda · 8 TB')
        self.assertEqual(p['steps'][0], ['wipefs', '-a', '/dev/sdb'])
        self.assertIn(['mkfs.btrfs', '-f', '-L', 'Big', '/dev/sdb1'], p['steps'])

    def test_the_system_drive_can_never_be_formatted(self):
        p = hh.format_plan('/dev/nvme0n1', 'storage', 'X', '', USER, sys_disks={'nvme0n1'})
        self.assertFalse(p['canFormat'])
        self.assertIn('system drive', p['why'])
        self.assertEqual(p['steps'], [])

    def test_a_drive_in_use_is_refused(self):
        busy = dict(DISK, children=[{'name': 'sdb1', 'mountpoints': ['/run/media/x/Stuff']}])
        hh.lsblk = lambda: [busy]
        p = hh.format_plan('/dev/sdb', 'storage', 'X', '', USER, sys_disks=set())
        self.assertFalse(p['canFormat'])

    def test_both_confirmations_naming_the_drive_are_needed_and_a_dry_run_changes_nothing(self):
        hh.system_disks = lambda: {'nvme0n1'}
        for bad in ({'confirm1': False}, {'confirm2': 'Seagate'}, {'confirm2': 'Samsung 990 · 1 TB'}, {'token': 'x'}, {'confirm1': 'yes'}):
            with self.assertRaises(hh.Fail, msg=bad):
                hh.format_drive(self.req(**bad))
        res = hh.format_drive(self.req())
        self.assertTrue(res['dryRun'])
        self.assertEqual(self.rec.calls, [], 'a dry run runs nothing')
        self.assertFalse(os.path.exists(hh.FSTAB))

    def test_a_swapped_drive_at_the_same_path_doesn_t_match(self):
        hh.system_disks = lambda: {'nvme0n1'}
        r = self.req()
        hh.lsblk = lambda: [dict(DISK, serial='OTHER')]
        with self.assertRaises(hh.Fail):
            hh.format_drive(r)

    def test_bad_devices_and_labels_are_refused(self):
        for dev in ('/dev/sdb1', '/dev/../etc', 'sdb', '/dev/sdb; rm', '/dev/mapper/x'):
            with self.assertRaises(hh.Fail):
                hh.find_disk(dev)
        with self.assertRaises(hh.Fail):
            hh.format_plan('/dev/sdb', 'storage', 'bad label!', '', USER, sys_disks=set())

    def test_new_drives_never_stop_the_pc_starting(self):
        line = hh.fstab_line('1234abcd-0000-1111-2222-333344445555', '/var/mnt/Big')
        self.assertIn('nofail', line)
        self.assertIn('x-systemd.device-timeout=10s', line)
        with self.assertRaises(hh.Fail):
            hh.fstab_line('$(reboot)', '/var/mnt/Big')
        with self.assertRaises(hh.Fail):
            hh.fstab_line('1234abcd-0000', '/')

    def test_mirroring_needs_a_hearth_btrfs_drive_never_the_system_one(self):
        hh.mount_info = lambda p: {'target': '/var/mnt/Big', 'fstype': 'btrfs', 'source': '/dev/sda1'}
        p = hh.format_plan('/dev/sdb', 'mirror', '', '/var/mnt/Big', USER, sys_disks={'nvme0n1'})
        self.assertTrue(p['canFormat'], p['why'])
        self.assertIn('-dconvert=raid1', p['steps'][-1])
        self.assertIn('--property=IOSchedulingClass=idle', p['steps'][-1])
        hh.mount_info = lambda p: {'target': '/var/mnt/Big', 'fstype': 'btrfs', 'source': '/dev/nvme0n1p3'}
        self.assertFalse(hh.format_plan('/dev/sdb', 'mirror', '', '/var/mnt/Big', USER, sys_disks={'nvme0n1'})['canFormat'])
        self.assertFalse(hh.format_plan('/dev/sdb', 'mirror', '', '/var/home', USER, sys_disks={'nvme0n1'})['canFormat'])


# ---------------------------------------------------------------------------------------------------- settings

class Settings(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        hs.SETTINGS = os.path.join(self.d, 'nimbus', 'home-server.json')
        hs.APPLIED = os.path.join(self.d, 'state', 'applied.json')
        hs.SEEN = os.path.join(self.d, 'state', 'seen.json')

    def test_nothing_is_written_until_something_changes(self):
        self.assertEqual(hs.load()['lightMode'], True)
        self.assertFalse(os.path.exists(hs.SETTINGS), 'reading never writes')
        self.assertFalse(hs.set_key('lightMode', 'on'), 'the default is what you have: nothing to save')
        self.assertFalse(hs.set_key('keepHourly', '24'))
        self.assertFalse(os.path.exists(hs.SETTINGS))
        self.assertTrue(hs.set_key('lightMode', 'off'))
        self.assertEqual(json.load(open(hs.SETTINGS)), {'lightMode': False}, 'only the changed key, never the defaults')
        m = os.stat(hs.SETTINGS).st_mtime_ns
        time.sleep(0.01)
        self.assertFalse(hs.set_key('lightMode', 'off'))
        self.assertEqual(os.stat(hs.SETTINGS).st_mtime_ns, m, 'the same value again: not written')
        self.assertTrue(hs.set_key('keepDaily', '30'))
        self.assertEqual(json.load(open(hs.SETTINGS))['keep'], {'hourly': 24, 'daily': 30, 'weekly': 8})

    def test_bad_values_are_refused_and_nothing_saved(self):
        for k, v in (('lightMode', 'yes'), ('keepHourly', '999'), ('ccDays', '0'), ('ccDays', 'x'), ('nope', 'on'), ('keepDaily', '-1')):
            with self.assertRaises(hs.Fail):
                hs.set_key(k, v)
        self.assertFalse(os.path.exists(hs.SETTINGS))

    def test_a_damaged_file_never_gets_the_defaults_saved_over_it(self):
        os.makedirs(os.path.dirname(hs.SETTINGS))
        with open(hs.SETTINGS, 'w') as f:
            f.write('{"shares": [{"id": "abc123"')
        self.assertEqual(hs.load()['shares'], [], 'defaults in memory')
        self.assertEqual(open(hs.SETTINGS).read(), '{"shares": [{"id": "abc123"', 'the damaged file is left alone by reading')
        hs.set_key('tsBoot', 'on')
        kept = [n for n in os.listdir(os.path.dirname(hs.SETTINGS)) if n.startswith('home-server.json.broken-')]
        self.assertGreaterEqual(len(kept), 1, 'the damaged copy is kept')
        self.assertEqual(json.load(open(hs.SETTINGS)), {'tsBoot': True})

    def test_shares_are_added_changed_and_removed_with_a_lock_and_files_kept(self):
        home = tempfile.mkdtemp()
        os.makedirs(os.path.join(home, 'Movies (TV)'))
        saved = hs.me
        hs.me = lambda: (USER, home)
        try:
            s = hs.share_add(os.path.join(home, 'Movies (TV)'))
            self.assertEqual(s['name'], 'Movies TV')
            self.assertEqual((s['on'], s['write'], s['reach'], s['recycle'], s['snapshots']), (True, False, 'home', True, False), 'read only, home network')
            self.assertEqual(s['users'], [USER])
            with self.assertRaises(hs.Fail):
                hs.share_add(os.path.join(home, 'Movies (TV)'))
            os.makedirs(os.path.join(home, 'Tom & Jerry'))
            with self.assertRaises(hs.Fail):
                hs.share_add(os.path.join(home, 'Tom & Jerry'))
            with self.assertRaises(hs.Fail):
                hs.share_add('/etc')
            self.assertTrue(hs.share_set(s['id'], 'write', 'on'))
            self.assertFalse(hs.share_set(s['id'], 'write', 'on'), 'already so: not written')
            with self.assertRaises(hs.Fail):
                hs.share_set(s['id'], 'reach', 'internet')
            gone = hs.share_remove(s['id'])
            self.assertEqual(gone['id'], s['id'])
            self.assertTrue(os.path.isdir(os.path.join(home, 'Movies (TV)')))
            self.assertEqual(json.load(open(hs.SETTINGS))['shares'], [])
        finally:
            hs.me = saved

    def test_the_apply_payload_carries_only_shares_that_are_on(self):
        d = hs.load()
        d['shares'] = [{'id': 'aaaaaa', 'name': 'A', 'path': '/x', 'on': True}, {'id': 'bbbbbb', 'name': 'B', 'path': '/y', 'on': False}]
        p = hs.apply_payload(d)
        self.assertEqual([s['name'] for s in p['shares']], ['A'])
        self.assertEqual(set(p), {'shares', 'tsOnly', 'tsBoot', 'snapshots', 'keep', 'lightMode', 'people', 'backups', 'receive', 'alerts', 'power',
                                  'media', 'ai', 'guard', 'pictures', 'voices', 'studio', 'serverWeb'})
        self.assertNotIn('password', json.dumps(p))

    def test_the_settings_file_is_on_the_persistence_check_list(self):
        t = (ROOT / 'hosts' / 'linux' / 'test' / 'test_settings_persistence.py').read_text()
        self.assertIn("'home-server.json'", t)


# ---------------------------------------------------------------------------------------------------- hidden until seen

class HiddenUntilSeen(unittest.TestCase):
    NOW = 1_800_000_000_000

    def data(self, **kw):
        d = dict(hs.DEFAULTS)
        d.update(kw)
        return d

    def test_the_rule(self):
        self.assertTrue(hs.hidden_until_seen(0, self.NOW, 30), 'never seen')
        self.assertFalse(hs.hidden_until_seen(self.NOW - 29 * hs.DAY_MS, self.NOW, 30))
        self.assertTrue(hs.hidden_until_seen(self.NOW - 31 * hs.DAY_MS, self.NOW, 30))
        self.assertFalse(hs.hidden_until_seen(self.NOW - 31 * hs.DAY_MS, self.NOW, 90), 'the days are yours to set')

    def test_command_center_items_show_only_when_seen_and_show_hidden_is_display_only(self):
        items = [{'id': 'ai', 'needs': 'commandCenter'}, {'id': 'shares'}, {'id': 'cams', 'needs': 'camera'}, {'id': 'ups', 'needs': 'ups'}]
        seen = {'commandCenter': {'at': self.NOW - 2 * hs.DAY_MS, 'device': 'Laptop'}}
        shown, hidden = hs.visible_items(items, self.data(), self.NOW, seen)
        self.assertEqual([i['id'] for i in shown], ['ai', 'shares'])
        self.assertEqual(hidden, 2, 'no camera or UPS found yet')
        shown, hidden = hs.visible_items(items, self.data(), self.NOW, {})
        self.assertEqual([i['id'] for i in shown], ['shares'])
        self.assertEqual(hidden, 3)
        shown, hidden = hs.visible_items(items, self.data(showHidden=True), self.NOW, {})
        self.assertEqual(len(shown), 4, 'Show hidden items shows them')
        self.assertEqual(hidden, 3, 'and they are still counted as hidden: nothing was turned on')
        st = hs.seen_state(self.data(ccDays=1), self.NOW, seen)
        self.assertTrue(st['commandCenter']['hidden'], 'seen 2 days ago, 1-day window')
        self.assertEqual(st['commandCenter']['device'], 'Laptop')

    def test_hiding_never_changes_a_setting(self):
        d = tempfile.mkdtemp()
        hs.SETTINGS = os.path.join(d, 'home-server.json')
        hs.SEEN = os.path.join(d, 'seen.json')
        hs.seen_state(hs.load(), self.NOW)
        hs.visible_items([{'id': 'ai', 'needs': 'commandCenter'}], hs.load(), self.NOW)
        self.assertFalse(os.path.exists(hs.SETTINGS))
        hs.mark_seen('commandCenter', 'Laptop')
        self.assertFalse(os.path.exists(hs.SETTINGS), 'seeing a device is state, not a setting')
        self.assertIn('commandCenter', json.load(open(hs.SEEN)))
        with self.assertRaises(hs.Fail):
            hs.mark_seen('toaster')


class Wiring(unittest.TestCase):
    def test_settings_pages_are_registered(self):
        s = (SHELL / 'settings-pages.js').read_text()   # Hearth Settings' page registry (the redesign)
        for pid, f in (('hs-overview', 'SetHomeServer.qml'), ('hs-shares', 'SetHomeShares.qml'), ('hs-drives', 'SetHomeDrives.qml'),
                       ('hs-tailscale', 'SetHomeTailscale.qml'), ('hs-setup', 'SetHomeSetup.qml')):
            # Home server is its own category; Tailscale's one home is Network & internet, with a link card in Home server.
            self.assertIn('{ id: "%s", file: "%s", cat: "%s"' % (pid, f, 'net' if pid == 'hs-tailscale' else 'hs'), s)
            self.assertTrue((SHELL / f).exists(), f)
            qml = (SHELL / f).read_text()
            self.assertIn('helper: "hearth-home-server"', qml)
            self.assertNotIn('FileView', qml, 'settings go through hearth-home-server only')
        self.assertIn('hs: [{ page: "hs-tailscale"', s)

    def test_the_user_helper_never_logs_the_sign_in_link(self):
        src = (SHELL / 'hearth-home-server').read_text()
        login = src.split('def ts_login')[1].split('\ndef ')[0]
        self.assertIn('stdout=subprocess.DEVNULL, stderr=errf', login)
        self.assertIn('0o600', login)                       # what Tailscale says stays private
        self.assertIn('os.remove(err_path)', login)         # and is removed at once
        self.assertIn("re.sub(r'https?://", login)          # a refusal shown to you never carries the link
        self.assertNotIn('log_event', login)
        self.assertNotIn('authkey', src.lower())

    def test_the_root_helper_never_uses_a_shell(self):
        src = (LIBEXEC / 'hearth-home-server-helper').read_text()
        self.assertNotIn('shell=True', src)
        self.assertNotIn('os.system(', src)
        self.assertNotRegex(src, r"\['(ba)?sh', '-c'")


if __name__ == '__main__':
    unittest.main()
