"""Tests for hearth-apt-layer (os-bases/apt-layer): the list of added packages, the guard rails apt's hooks
apply, which deployment is booted, what's put back after an update and how. No apt, dpkg or mounts."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import tempfile
import unittest

PATH = (pathlib.Path(__file__).resolve().parents[3] / 'os-bases' / 'apt-layer' / 'system_files' / 'usr' / 'libexec'
        / 'nimbus' / 'hearth-apt-layer')
CSUM = 'a' * 64
CSUM2 = 'b' * 64


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_apt_layer', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_apt_layer', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


L = load()

STATUS = """Package: bash
Status: install ok installed
Architecture: amd64
Version: 5.2-1

Package: libc6
Status: install ok installed
Architecture: i386
Multi-Arch: same
Version: 2.41-1

Package: libc6
Status: install ok installed
Architecture: amd64
Multi-Arch: same
Version: 2.41-1

Package: tzdata
Status: install ok installed
Architecture: all
Version: 2025a
Description: time zones
 with a long description
 .
 over lines

Package: gone
Status: deinstall ok config-files
Architecture: amd64
Version: 1.0

Package: hello
Status: install ok installed
Architecture: amd64
Version: 2.10-3

Package: libhello
Status: install ok unpacked
Architecture: amd64
Version: 2.10-3
"""

EXT = """Package: libhello
Architecture: amd64
Auto-Installed: 1

Package: hello
Architecture: amd64
Auto-Installed: 0
"""

# What apt really sends (captured with apt 2.8): installing hearth-ta, which pulls in hearth-tb, then removing.
HOOK_INSTALL = """VERSION 3
APT::Architecture=amd64
Dir=/

hearth-tb - - none < 1.0 all none /var/cache/apt/archives/hearth-tb_1.0_all.deb
hearth-ta - - none < 1.0 all none /var/cache/apt/archives/hearth-ta_1.0_all.deb
hearth-tb - - none < 1.0 all none **CONFIGURE**
hearth-ta - - none < 1.0 all none **CONFIGURE**
"""
HOOK_REMOVE = """VERSION 3
APT::Architecture=amd64

bash 5.2-1 amd64 none > - - none **REMOVE**
libc6 2.41-1 i386 same > - - none **REMOVE**
hello 2.10-3 amd64 none > - - none **REMOVE**
"""


class Status(unittest.TestCase):
    def test_reads_installed_packages_with_arch_ids(self):
        st = L.parse_status(STATUS, 'amd64')
        self.assertEqual(set(st), {'bash', 'libc6', 'libc6:i386', 'tzdata', 'hello', 'libhello'})
        self.assertEqual(st['tzdata']['version'], '2025a')
        self.assertNotIn('gone', st)   # only its settings are left

    def test_auto_installed(self):
        self.assertEqual(L.parse_extended_states(EXT, 'amd64'), {'libhello'})

    def test_user_packages_leave_out_the_image_and_dependencies(self):
        st = L.parse_status(STATUS, 'amd64')
        image = {'bash': '5.2-1', 'libc6': '2.41-1', 'libc6:i386': '2.41-1', 'tzdata': '2025a'}
        self.assertEqual(L.user_packages(st, image, {'libhello'}), {'hello'})

    def test_ids(self):
        self.assertEqual(L.pkg_id('foo', 'amd64', 'amd64'), 'foo')
        self.assertEqual(L.pkg_id('foo', 'all', 'amd64'), 'foo')
        self.assertEqual(L.pkg_id('foo:i386', 'i386', 'amd64'), 'foo:i386')

    def test_image_list_round_trip(self):
        st = L.parse_status(STATUS, 'amd64')
        back = L.parse_image_list(L.image_list_text(st))
        self.assertEqual(back['libc6:i386'], '2.41-1')
        self.assertEqual(len(back), len(st))


class TheList(unittest.TestCase):
    def test_install_adds_and_remove_takes_off(self):
        s = L.empty_state()
        s, added, removed = L.update_list(s, CSUM, {'hello'}, now=5)
        self.assertEqual((added, removed), (['hello'], []))
        self.assertIn('hello', s['packages'])
        s, added, removed = L.update_list(s, CSUM, set(), now=6)
        self.assertEqual((added, removed), ([], ['hello']))
        self.assertNotIn('hello', s['packages'])

    def test_a_new_deployment_doesnt_lose_what_isnt_back_yet(self):
        s = L.empty_state()
        s, _, _ = L.update_list(s, CSUM, {'hello', 'vlc'}, now=5)
        # After the update: the first package is back, the other one still on its way (or skipped).
        s, added, removed = L.update_list(s, CSUM2, {'hello'}, now=6)
        self.assertEqual(removed, [])
        self.assertEqual(set(s['packages']), {'hello', 'vlc'})
        # Removing it on the new version takes it off.
        s, _, removed = L.update_list(s, CSUM2, set(), now=7)
        self.assertEqual(removed, ['hello'])
        self.assertEqual(set(s['packages']), {'vlc'})

    def test_keeps_the_version_and_first_added_time(self):
        st = L.parse_status(STATUS, 'amd64')
        s, _, _ = L.update_list(L.empty_state(), CSUM, {'hello'}, st, now=5)
        s, _, _ = L.update_list(s, CSUM2, {'hello'}, st, now=9)
        self.assertEqual(s['packages']['hello'], {'added': 5, 'version': '2.10-3'})


class Guards(unittest.TestCase):
    IMAGE = {'bash': '5.2-1', 'libc6': '2.41-1', 'libc6:i386': '2.41-1'}

    def test_parses_apts_protocol_3(self):
        a = L.parse_hook_v3(HOOK_INSTALL)
        self.assertEqual(len(a), 4)
        self.assertEqual(a[0]['name'], 'hearth-tb')
        self.assertEqual(a[0]['action'], '/var/cache/apt/archives/hearth-tb_1.0_all.deb')
        self.assertEqual(a[2]['action'], '**CONFIGURE**')
        self.assertEqual(L.parse_hook_v3('VERSION 2\n\nx - < 1 /a.deb\n'), [])

    def test_installing_new_packages_is_fine(self):
        self.assertEqual(L.check_v3(L.parse_hook_v3(HOOK_INSTALL), self.IMAGE, 'amd64'), [])

    def test_removing_system_packages_is_refused(self):
        p = L.check_v3(L.parse_hook_v3(HOOK_REMOVE), self.IMAGE, 'amd64')
        self.assertEqual(len(p), 2)
        self.assertIn('bash', p[0])
        self.assertIn('libc6:i386', p[1])

    def test_debs_replacing_system_packages_are_refused(self):
        f = lambda n, v, a='amd64': {'Package': n, 'Version': v, 'Architecture': a}
        debs = [('/var/cache/apt/archives/bash_5.3-1_amd64.deb', f('bash', '5.3-1')),
                ('/var/cache/apt/archives/bash_5.2-1_amd64.deb', f('bash', '5.2-1')),
                ('/var/cache/apt/archives/linux-image-7.1.0-1-generic.deb', f('linux-image-7.1.0-1-generic', '7.1')),
                ('/home/sam/Downloads/discord.deb', f('discord', '0.0.90')),
                ('/var/cache/apt/archives/vlc_3.0_amd64.deb', f('vlc', '3.0'))]
        problems, keep = L.check_debs(debs, self.IMAGE, 'amd64')
        self.assertEqual(len(problems), 3)
        self.assertIn("would replace the system's bash 5.2-1", problems[0])   # an upgrade
        self.assertIn("can't be reinstalled", problems[1])                    # a reinstall
        self.assertIn('kernel', problems[2])
        self.assertEqual(keep, [('discord', '/home/sam/Downloads/discord.deb')])   # only local files are kept

    def test_boot_packages(self):
        for n in ('linux-image-7.0.0-12-generic', 'linux-image-amd64', 'linux-modules-extra-7.0.0-1-generic',
                  'grub-efi-amd64-signed', 'shim-signed', 'dracut', 'dracut-network', 'initramfs-tools',
                  'systemd-boot', 'bootc', 'ostree', 'linux-generic-hwe-26.04'):
            self.assertTrue(L.is_boot_package(n), n)
        for n in ('linux-headers-amd64', 'linux-firmware', 'linux-tools-common', 'shimmer', 'grubby-ish', 'vlc',
                  'steam-installer', 'firmware-misc-nonfree'):
            self.assertFalse(L.is_boot_package(n), n)   # (shimmer, grubby-ish: names that only start the same)

    def test_reasons_from_apt(self):
        out = ("Reading package lists...\nSome packages could not be installed.\nThe following packages have unmet "
               "dependencies:\n vlc : Depends: libvlc5 (= 3.0.21) but 3.0.20 is to be installed\nE: Unable to correct "
               "problems, you have held broken packages.\n")
        self.assertIn('libvlc5', L.apt_reason(out))
        self.assertEqual(L.apt_reason('Hearth: bash is part of Hearth OS\nE: Sub-process returned an error code'),
                         'bash is part of Hearth OS')
        self.assertEqual(L.apt_reason('E: Unable to locate package nope\n'), 'Unable to locate package nope')


class PutBack(unittest.TestCase):
    def newer(self, a, b):
        return tuple(int(x) for x in a.split('.')) > tuple(int(x) for x in b.split('.'))

    def test_plans_repo_and_kept_debs(self):
        wanted = {'vlc': {}, 'discord': {}, 'old-tool': {}, 'hello': {}, 'tzdata': {}, 'linux-image-x': {},
                  'mytool': {}}
        kept = {'discord': ('/k/discord.deb', '0.90'), 'mytool': ('/k/mytool.deb', '1.0')}
        cands = {'vlc': '3.0', 'discord': None, 'old-tool': None, 'mytool': '2.0'}
        jobs, skipped, included, already = L.plan_reapply(wanted, {'hello'}, {'tzdata': '2025a'}, kept, cands, True,
                                                          self.newer)
        self.assertEqual(dict(jobs), {'discord': '/k/discord.deb', 'mytool': 'mytool', 'vlc': 'vlc'})
        self.assertEqual([s[0] for s in skipped], ['linux-image-x', 'old-tool'])
        self.assertEqual(included, ['tzdata'])   # the new image has it now
        self.assertEqual(already, ['hello'])

    def test_kept_deb_newer_than_the_repo_wins(self):
        jobs, _, _, _ = L.plan_reapply({'mytool': {}}, set(), {}, {'mytool': ('/k/m.deb', '3.0')},
                                       {'mytool': '2.0'}, True, self.newer)
        self.assertEqual(jobs, [('mytool', '/k/m.deb')])

    def test_offline_uses_kept_debs_and_waits_for_the_rest(self):
        jobs, skipped, _, _ = L.plan_reapply({'vlc': {}, 'discord': {}}, set(), {},
                                             {'discord': ('/k/d.deb', '1')}, {'vlc': '3.0'}, False, self.newer)
        self.assertEqual(jobs, [('discord', '/k/d.deb')])
        self.assertEqual(skipped[0][0], 'vlc')
        self.assertIn('no internet', skipped[0][1])

    def test_summary(self):
        t, b = L.summary({'reinstalled': ['a', 'b'], 'skipped': []})
        self.assertEqual(t, 'Your added apps are back')
        t, b = L.summary({'reinstalled': ['a'], 'skipped': [{'name': 'vlc', 'reason': 'x'}]})
        self.assertIn("weren't put back", t)
        self.assertIn('1 re-installed, 1 skipped: vlc', b)


class Deployment(unittest.TestCase):
    def test_from_the_boot_option(self):
        target = '/sysroot/ostree/deploy/default/deploy/%s.0' % CSUM
        res = lambda p: target if p.startswith('/sysroot/ostree/boot.1') else p
        self.assertEqual(L.key_from_cmdline('BOOT_IMAGE=x ostree=/ostree/boot.1/default/ff/0 rw', res), CSUM)
        self.assertIsNone(L.key_from_cmdline('BOOT_IMAGE=x quiet', res))

    def test_from_ostree_admin_status(self):
        text = ('  default %s.0 (staged)\n* default %s.1\n    Version: 0.6\n  default %s.0 (rollback)\n'
                % (CSUM2, CSUM, CSUM2))
        self.assertEqual(L.key_from_admin_status(text), CSUM)

    def test_deployments_on_the_drive(self):
        with tempfile.TemporaryDirectory() as d:
            dep = os.path.join(d, 'default', 'deploy')
            os.makedirs(os.path.join(dep, CSUM + '.0'))
            os.makedirs(os.path.join(dep, CSUM2 + '.1'))
            open(os.path.join(dep, CSUM + '.0.origin'), 'w').close()
            self.assertEqual(L.deployment_keys(d), {CSUM, CSUM2})
            self.assertIsNone(L.deployment_keys(os.path.join(d, 'nope')))


class ImageAndApps(unittest.TestCase):
    def test_pins(self):
        text = L.pins_text({'bash': '5.2-1', 'libc6': '2.41-1', 'libc6:i386': '2.41-1', 'odd': '1', 'odd:i386': '2'})
        self.assertIn('Package: bash\nPin: version 5.2-1\nPin-Priority: 1001', text)
        self.assertEqual(text.count('Package: libc6\n'), 1)   # one stanza for both architectures
        self.assertIn('Package: odd:i386\nPin: version 2', text)

    def test_desktop_ids(self):
        lst = ('/.\n/usr\n/usr/share/applications\n/usr/share/applications/discord.desktop\n'
               '/usr/share/applications/kde4/foo.desktop\n/usr/share/discord/discord.png\n')
        self.assertEqual(L.desktop_ids(lst), ['discord.desktop', 'kde4-foo.desktop'])

    def test_added_apps_file(self):
        with tempfile.TemporaryDirectory() as d:
            info = os.path.join(d, 'info')
            os.makedirs(info)
            with open(os.path.join(info, 'discord.list'), 'w') as f:
                f.write('/usr/share/applications/discord.desktop\n')
            with open(os.path.join(info, 'libfoo:i386.list'), 'w') as f:
                f.write('/usr/lib/i386-linux-gnu/libfoo.so.1\n')
            old = (L.DPKG_STATUS, L.ADDED_APPS)
            L.DPKG_STATUS, L.ADDED_APPS = os.path.join(d, 'status'), os.path.join(d, 'layer', 'added-apps')
            try:
                state = {'packages': {'discord': {}, 'libfoo:i386': {}, 'notinstalled': {}}}
                status = {'discord': {'name': 'discord', 'arch': 'amd64', 'version': '1'},
                          'libfoo:i386': {'name': 'libfoo', 'arch': 'i386', 'version': '1'}}
                L.write_added_apps(state, status)
                self.assertEqual(L.read(L.ADDED_APPS), 'discord.desktop\n')
                self.assertEqual(os.stat(L.ADDED_APPS).st_mode & 0o777, 0o644)
            finally:
                L.DPKG_STATUS, L.ADDED_APPS = old


ADMIN = pathlib.Path(__file__).resolve().parents[3] / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin'


def load_admin():
    loader = importlib.machinery.SourceFileLoader('hearth_admin_apt', str(ADMIN))
    spec = importlib.util.spec_from_loader('hearth_admin_apt', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class AdminInstallDeb(unittest.TestCase):
    def test_only_real_deb_files(self):
        A = load_admin()
        with tempfile.TemporaryDirectory() as d:
            deb = os.path.join(d, 'app.deb')
            with open(deb, 'wb') as f:
                f.write(b'!<arch>\n')
            os.symlink(deb, os.path.join(d, 'link.deb'))
            os.symlink('/etc/passwd', os.path.join(d, 'sneaky.deb'))
            os.mkdir(os.path.join(d, 'dir.deb'))
            open(os.path.join(d, 'empty.deb'), 'w').close()
            self.assertEqual(A.valid_deb_path(deb), os.path.realpath(deb))
            self.assertEqual(A.valid_deb_path(os.path.join(d, 'link.deb')), os.path.realpath(deb))
            for bad in ('app.deb', os.path.join(d, 'sneaky.deb'), os.path.join(d, 'dir.deb'),
                        os.path.join(d, 'empty.deb'), deb + '\n', '/etc/passwd', None):
                self.assertIsNone(A.valid_deb_path(bad), bad)


if __name__ == '__main__':
    unittest.main()
