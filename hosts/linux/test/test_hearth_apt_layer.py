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


class ImageChanges(unittest.TestCase):
    IMAGE = {'bash': '5.2-1', 'nano': '8.4-1', 'vlc-bin': '3.0.20', 'tzdata': '2025a'}

    def test_finds_removed_and_upgraded_image_packages(self):
        status = {'bash': {'version': '5.2-1'}, 'vlc-bin': {'version': '3.0.21'}, 'tzdata': {'version': '2025a'}}
        removed, changed = L.image_changes(status, self.IMAGE)
        self.assertEqual(removed, {'nano'})
        self.assertEqual(changed, {'vlc-bin': '3.0.21'})

    def test_records_and_drops_removals_and_upgrades(self):
        s = L.empty_state()
        s, _, _ = L.update_list(s, CSUM, set(), removed={'nano'}, changed={'vlc-bin': '3.0.21'}, image=self.IMAGE,
                                now=5)
        self.assertEqual(s['removed'], {'nano': {'image': '8.4-1', 'added': 5}})
        self.assertEqual(s['upgraded']['vlc-bin']['version'], '3.0.21')
        # Reinstalling nano and going back to the image's vlc-bin drop both.
        s, _, _ = L.update_list(s, CSUM, set(), removed=set(), changed={}, image=self.IMAGE, now=6)
        self.assertEqual((s['removed'], s['upgraded']), ({}, {}))

    def test_a_new_deployment_keeps_what_isnt_reapplied_yet(self):
        s = L.empty_state()
        s, _, _ = L.update_list(s, CSUM, set(), removed={'nano'}, changed={'vlc-bin': '3.0.21'}, image=self.IMAGE)
        # The new image: nano is installed again (nothing re-removed yet), vlc-bin at the image's version.
        s, _, _ = L.update_list(s, CSUM2, set(), removed=set(), changed={}, image=self.IMAGE)
        self.assertIn('nano', s['removed'])
        self.assertIn('vlc-bin', s['upgraded'])

    def test_reads_version_1_state(self):
        s = {'version': 1, 'packages': {'hello': {}}, 'removed': {}, 'upgraded': {}, 'seen': {CSUM: ['hello']}}
        s, added, gone = L.update_list(s, CSUM, {'hello'})
        self.assertEqual((added, gone), ([], []))


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

    def test_removing_system_packages_is_fine_except_boot_and_protected(self):
        plan = L.parse_hook_v3(HOOK_REMOVE + 'linux-image-amd64 7.0 amd64 none > - - none **REMOVE**\n')
        image = dict(self.IMAGE, hello='2.10-3', **{'linux-image-amd64': '7.0'})
        p = L.check_v3(plan, image, 'amd64', protected={'bash', 'libc6', 'libc6:i386'})
        self.assertEqual(len(p), 3)
        self.assertIn('bash stays', p[0])
        self.assertIn('libc6:i386 stays', p[1])
        self.assertIn('kernel', p[2])
        # hello (an image package nothing of Hearth's needs) is removed without a word.
        self.assertFalse(any('hello' in x for x in p))
        self.assertEqual(L.check_v3(L.parse_hook_v3(HOOK_REMOVE), image, 'amd64'), [])

    def test_only_boot_debs_are_refused(self):
        f = lambda n, v, a='amd64': {'Package': n, 'Version': v, 'Architecture': a}
        debs = [('/var/cache/apt/archives/bash_5.3-1_amd64.deb', f('bash', '5.3-1')),            # an upgrade
                ('/var/cache/apt/archives/bash_5.2-1_amd64.deb', f('bash', '5.2-1')),            # a reinstall
                ('/var/cache/apt/archives/linux-image-7.1.0-1-generic.deb', f('linux-image-7.1.0-1-generic', '7.1')),
                ('/var/cache/apt/archives/grub-efi-amd64-signed.deb', f('grub-efi-amd64-signed', '2')),
                ('/home/sam/Downloads/discord.deb', f('discord', '0.0.90')),
                ('/home/sam/Downloads/bash.deb', f('bash', '9')),
                ('/var/cache/apt/archives/vlc_3.0_amd64.deb', f('vlc', '3.0'))]
        problems, keep = L.check_debs(debs, self.IMAGE, 'amd64')
        self.assertEqual(len(problems), 2)
        self.assertIn('linux-image-7.1.0-1-generic', problems[0])
        self.assertIn('grub-efi-amd64-signed', problems[1])
        self.assertEqual(keep, [('discord', '/home/sam/Downloads/discord.deb')])   # only your own local files

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

    def test_removals(self):
        todo, gone = L.plan_removals({'nano': {}, 'oldpkg': {}, 'vim': {}}, {'nano': '1', 'vim': '2'}, {'nano'})
        self.assertEqual(todo, ['nano'])        # vim isn't installed on the new system: nothing to do
        self.assertEqual(gone, ['oldpkg'])      # the new image doesn't have it any more

    def test_removal_that_takes_more_with_it_means_the_image_needs_it(self):
        sim = 'NOTE: This is only a simulation!\nRemv libfoo1 [1.0]\nRemv foo-tools [1.0]\nRemv nano [8]\n'
        self.assertEqual(L.removal_extras(sim, {'nano', 'libfoo1'}), ['foo-tools'])
        self.assertEqual(L.removal_extras('Remv nano [8]\n', {'nano'}), [])

    def test_upgrades(self):
        up = {'vlc-bin': {'version': '3.1'}, 'mesa': {'version': '25.1'}, 'gone': {'version': '1.0'},
              'old': {'version': '2.0'}}
        image = {'vlc-bin': '3.0', 'mesa': '25.2', 'gone-not': '1', 'old': '1.0'}
        present = {'vlc-bin', 'mesa', 'old'}
        jobs, skipped, caught, dropped = L.plan_upgrades(up, image, present, {'vlc-bin': '3.2', 'old': '1.0'}, True,
                                                          self.newer)
        self.assertEqual(jobs, ['vlc-bin'])                  # the image is still older: upgraded again
        self.assertEqual(caught, ['mesa'])                   # the image caught up: dropped
        self.assertEqual([d[0] for d in dropped], ['gone'])  # not in the new image
        self.assertEqual(skipped[0][0], 'old')               # wanted, but the sources don't have a newer one
        _, skipped, _, _ = L.plan_upgrades({'vlc-bin': {'version': '3.1'}}, image, present, {}, False, self.newer)
        self.assertIn('no internet', skipped[0][1])

    def test_summary(self):
        t, b = L.summary({'reinstalled': ['a', 'b'], 'reremoved': ['nano'], 'skipped': []})
        self.assertEqual(t, 'Your changes are back')
        self.assertIn('3 put back', b)
        t, b = L.summary({'reinstalled': ['a'], 'skipped': [{'name': 'vlc', 'reason': 'x'}]})
        self.assertIn("weren't put back", t)
        self.assertIn('1 put back on the updated system, 1 skipped: vlc', b)


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
    def test_pins_hold_only_boot_packages(self):
        text = L.pins_text({'bash': '5.2-1', 'linux-image-amd64': '7.0', 'grub-efi-amd64-signed': '2',
                            'shim-signed': '1', 'shim-signed:i386': '2'})
        self.assertNotIn('Package: bash', text)
        self.assertIn('Package: linux-image-amd64\nPin: version 7.0\nPin-Priority: 1001', text)
        self.assertIn('Package: shim-signed:i386\nPin: version 2', text)

    def test_protected_from_apt_cache_depends(self):
        out = 'python3\n  Depends: python3-minimal\n  PreDepends: <libc6>\npython3-minimal\n  Depends: libc6\n' \
              '<virtual-thing>\nlibc6\nnot-installed\n'
        self.assertEqual(L.protected_from_depends(out, {'python3': '1', 'python3-minimal': '1', 'libc6': '2'}),
                         {'python3', 'python3-minimal', 'libc6'})

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
