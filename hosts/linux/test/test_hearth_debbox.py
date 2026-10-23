"""Tests for desktop/shell/hearth-debbox (.deb files through Hearth's Debian box): the distrobox/apt commands it
builds, reading a .deb's control file without dpkg, the progress it reports, and a whole install/remove with
stand-in distrobox and podman commands (no containers are made)."""
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

HERE = pathlib.Path(__file__).resolve().parent
PATH = HERE.parent / 'desktop' / 'shell' / 'hearth-debbox'
sys.path.insert(0, str(HERE))
import make_test_deb  # noqa: E402


def load(home):
    os.environ['HOME'] = home
    os.environ.pop('XDG_DATA_HOME', None)
    loader = importlib.machinery.SourceFileLoader('hearth_debbox', str(PATH))
    spec = importlib.util.spec_from_loader('hearth_debbox', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Base(unittest.TestCase):
    def setUp(self):
        self.old = dict(os.environ)
        self.tmp = tempfile.mkdtemp(prefix='hearth-debbox-test-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.d = load(self.home)

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)


class Commands(Base):
    def test_create(self):
        self.assertEqual(self.d.create_cmd(), ['distrobox', 'create', '--name', 'hearth-debian', '--image',
                                               'docker.io/library/debian:trixie', '--yes', '--no-entry'])
        self.assertEqual(self.d.create_cmd(nvidia=True)[-1], '--nvidia')

    def test_nvidia_detected_from_the_driver(self):
        root = os.path.join(self.tmp, 'root')
        self.assertFalse(self.d.has_nvidia(root))
        os.makedirs(os.path.join(root, 'proc/driver/nvidia'))
        open(os.path.join(root, 'proc/driver/nvidia/version'), 'w').close()
        self.assertTrue(self.d.has_nvidia(root))

    def test_enter(self):
        self.assertEqual(self.d.enter_cmd(['true']), ['distrobox', 'enter', '--name', 'hearth-debian', '--no-tty', '--', 'true'])
        self.assertEqual(self.d.enter_cmd(['sudo', 'apt', 'install', 'foo'], tty=True),
                         ['distrobox', 'enter', '--name', 'hearth-debian', '--', 'sudo', 'apt', 'install', 'foo'])

    def test_apt_install_takes_the_file_as_one_argument(self):
        cmd = self.d.apt_install_cmd('/home/me/Downloads/My App; rm -rf.deb')
        self.assertEqual(cmd[-1], '/home/me/Downloads/My App; rm -rf.deb')
        self.assertEqual(cmd[:7], ['podman', 'exec', '--user', 'root', '--env', 'DEBIAN_FRONTEND=noninteractive', 'hearth-debian'])
        self.assertIn('APT::Status-Fd=1', cmd)
        self.assertIn('-y', cmd)
        self.assertEqual(cmd[7:9], ['apt-get', 'install'])
        with self.assertRaises(self.d.Fail):
            self.d.apt_install_cmd('relative.deb')   # apt would look it up as a package name

    def test_apt_remove_checks_the_name(self):
        self.assertEqual(self.d.apt_remove_cmd('hearth-hello')[-2:], ['--', 'hearth-hello'])
        for bad in ('', '-y', 'foo bar', 'Foo', '../x', 'a;b', 'x' * 200):
            with self.assertRaises(self.d.Fail, msg=bad):
                self.d.apt_remove_cmd(bad)

    def test_export(self):
        self.assertEqual(self.d.export_cmd('/usr/share/applications/code.desktop')[-5:],
                         ['distrobox-export', '--app', '/usr/share/applications/code.desktop', '--export-label', 'none'])
        self.assertEqual(self.d.export_cmd('/usr/share/applications/code.desktop', label=False)[-3:],
                         ['distrobox-export', '--app', '/usr/share/applications/code.desktop'])
        self.assertEqual(self.d.export_cmd('/usr/share/applications/code.desktop', delete=True)[-1], '--delete')
        for bad in ('/etc/passwd', 'code.desktop', '/usr/share/applications/../x.desktop/', '/usr/share/applications/a/b.desktop'):
            with self.assertRaises(self.d.Fail, msg=bad):
                self.d.export_cmd(bad)

    def test_paths_the_box_sees(self):
        f = os.path.join(self.home, 'Downloads', 'a.deb')
        os.makedirs(os.path.dirname(f))
        open(f, 'w').close()
        self.assertEqual(self.d.box_path(f, self.home), os.path.realpath(f))
        self.assertEqual(self.d.box_path('/media/usb/a.deb', self.home), '/run/host/media/usb/a.deb')
        self.assertEqual(self.d.exported_path('/usr/share/applications/code.desktop', '/h/apps'), '/h/apps/hearth-debian-code.desktop')

    def test_apt_progress(self):
        self.assertEqual(self.d.apt_progress('dlstatus:1:0.0:Retrieving file 1 of 3', 10, 90), (10, 'Downloading Retrieving file 1 of 3'))
        self.assertEqual(self.d.apt_progress('dlstatus:1:100:done', 10, 90), (50, 'Downloading done'))
        self.assertEqual(self.d.apt_progress('pmstatus:code:50.0:Installing code (amd64)', 10, 90), (70, 'Installing code (amd64)'))
        self.assertEqual(self.d.apt_progress('pmstatus:code:100:Installed code', 10, 90)[0], 90)
        self.assertIsNone(self.d.apt_progress('Setting up code (1.0) ...'))

    def test_exec_line(self):
        self.assertEqual(self.d.exec_argv('[Desktop Entry]\nExec=/usr/bin/distrobox-enter -n hearth-debian -- code %F\n'),
                         ['/usr/bin/distrobox-enter', '-n', 'hearth-debian', '--', 'code'])
        with self.assertRaises(self.d.Fail):
            self.d.exec_argv('[Desktop Entry]\nName=x\n')

    def test_app_ids_checked(self):
        for bad in ('code.desktop', 'hearth-debian-../x.desktop', 'hearth-debian-a/b.desktop', 'hearth-debian-.desktop', ''):
            with self.assertRaises(self.d.Fail, msg=bad):
                self.d.open_app(bad)
            with self.assertRaises(self.d.Fail, msg=bad):
                self.d.owner(bad)


class ReadingDebs(Base):
    def test_control_from_our_own_test_package(self):
        for comp in ('gz', 'xz', 'none'):
            f = make_test_deb.make(os.path.join(self.tmp, 'h-%s.deb' % comp), control_compression=comp)
            with open(f, 'rb') as fh:
                c = self.d.control_from_deb_bytes(fh.read())
            self.assertEqual(c['Package'], 'hearth-hello', comp)
            self.assertEqual(c['Version'], '1.0-1')
            self.assertEqual(c['Depends'], 'libc6 (>= 2.34), coreutils | busybox')
            self.assertEqual(c['Description'].splitlines()[0], 'Hello from Hearth\'s Debian box')
            self.assertIn('Nothing else.', c['Description'])

    @unittest.skipUnless(shutil.which('dpkg-deb'), 'dpkg-deb not installed')
    def test_same_as_dpkg_deb(self):
        f = make_test_deb.make(os.path.join(self.tmp, 'h.deb'))
        with open(f, 'rb') as fh:
            ours = self.d.control_from_deb_bytes(fh.read())
        theirs = self.d.parse_control(subprocess.run(['dpkg-deb', '-f', f], capture_output=True, text=True).stdout)
        self.assertEqual(ours, theirs)

    def test_not_a_deb(self):
        with self.assertRaises(self.d.Fail):
            self.d.control_from_deb_bytes(b'PK\x03\x04 a zip file')
        with self.assertRaises(self.d.Fail):
            self.d.control_from_deb_bytes(b'!<arch>\ndebian-binary   0           0     0     100644  4         X\n2.0\n')

    def test_control_parsing(self):
        c = self.d.parse_control('Package: foo\nDescription: short\n first line\n .\n second\nDepends: a\n\nPackage: other\n')
        self.assertEqual(c['Package'], 'foo')
        self.assertEqual(c['Description'], 'short\nfirst line\n\nsecond')
        self.assertEqual(c['Depends'], 'a')

    def test_info(self):
        info = self.d.package_info({'Package': 'foo', 'Version': '2.0', 'Architecture': 'all', 'Installed-Size': '2048',
                                    'Depends': 'libc6 (>= 2.34), libgtk-3-0t64 | libgtk-3-0 [amd64], zlib1g',
                                    'Pre-Depends': 'dpkg (>= 1.19)', 'Description': 'A thing\n Long text.'}, 3 * 1024 * 1024)
        self.assertEqual(info['depends'], ['dpkg', 'libc6', 'libgtk-3-0t64 or libgtk-3-0', 'zlib1g'])
        self.assertEqual((info['summary'], info['description']), ('A thing', 'Long text.'))
        self.assertEqual((info['installedSizeText'], info['fileSizeText']), ('2.0 MB', '3.0 MB'))
        self.assertTrue(info['installable'])
        self.assertEqual(info['warnings'], [])

    def test_wrong_kind_of_pc_or_a_driver(self):
        other = 'arm64' if os.uname().machine == 'x86_64' else 'amd64'
        info = self.d.package_info({'Package': 'foo', 'Architecture': other})
        self.assertFalse(info['installable'])
        self.assertIn(other, info['warnings'][0])
        info = self.d.package_info({'Package': 'foo-dkms', 'Architecture': 'all', 'Depends': 'dkms'})
        self.assertTrue(info['installable'])
        self.assertIn('driver', info['warnings'][0])

    def test_inspect(self):
        f = make_test_deb.make(os.path.join(self.home, 'h.deb'))
        self.d.box_exists = lambda: False
        info = self.d.inspect(f)
        self.assertEqual((info['package'], info['version'], info['installed']), ('hearth-hello', '1.0-1', False))
        self.assertIn('Debian box', info['limits'])
        with self.assertRaises(self.d.Fail):
            self.d.inspect(os.path.join(self.home, 'missing.deb'))


class Desktops(Base):
    def test_parse(self):
        text = ('@@/usr/share/applications/code.desktop\n[Desktop Entry]\nName=Visual Studio Code\nName[de]=X\nIcon=vscode\n'
                '[Desktop Action new]\nName=New\n@@/usr/share/applications/code-url.desktop\n[Desktop Entry]\nName=URL handler\n'
                'NoDisplay=true\n@@/usr/share/applications/link.desktop\nType=Link\n@@/etc/evil.desktop\nName=x\n')
        d = self.d.parse_desktops(text)
        self.assertEqual([x['desktop'] for x in d], ['/usr/share/applications/code.desktop', '/usr/share/applications/code-url.desktop',
                                                     '/usr/share/applications/link.desktop'])
        self.assertEqual((d[0]['name'], d[0]['icon'], d[0]['hidden']), ('Visual Studio Code', 'vscode', False))
        self.assertTrue(d[1]['hidden'])
        self.assertTrue(d[2]['hidden'])

    def test_our_exports_only(self):
        apps = os.path.join(self.tmp, 'apps')
        os.makedirs(apps)
        for name, text in (('hearth-debian-code.desktop', 'Exec=/usr/bin/distrobox-enter -n hearth-debian -- code\n'),
                           ('hearth-debian-mine.desktop', 'Exec=mine\n'),        # not made by distrobox
                           ('other-code.desktop', 'Exec=distrobox-enter -n hearth-debian -- x\n')):
            with open(os.path.join(apps, name), 'w') as f:
                f.write('[Desktop Entry]\n' + text)
        os.symlink('/etc/passwd', os.path.join(apps, 'hearth-debian-link.desktop'))
        self.assertEqual([os.path.basename(x) for x in self.d.our_exports(apps)], ['hearth-debian-code.desktop'])


FAKE_DISTROBOX = r'''#!/usr/bin/python3
import json, os, sys
log = os.environ['FAKE_LOG']
with open(log, 'a') as f:
    f.write(json.dumps(sys.argv[1:]) + '\n')
a = sys.argv[1:]
state = os.environ['FAKE_STATE']
if a[0] == 'create':
    open(state, 'w').close()
    print('Trying to pull docker.io/library/debian:trixie...')
    print('Copying blob 1234 done')
    sys.exit(0)
if a[0] == 'enter':
    cmd = a[a.index('--') + 1:]
    if cmd == ['true']:
        print(' Installing basic packages...  [ OK ]')
        sys.exit(0)
    if 'apt-get' in cmd and 'install' in cmd:
        deb = cmd[-1]
        if not os.path.exists(deb):
            print('E: Unsupported file ' + deb + ' given on commandline')
            sys.exit(100)
        print('dlstatus:1:50:Retrieving file 1 of 2')
        print('pmstatus:hearth-hello:20:Preparing hearth-hello')
        print('pmstatus:hearth-hello:100:Installed hearth-hello')
        sys.exit(0)
    if 'apt-get' in cmd:
        sys.exit(0)
    if cmd[:2] == ['sh', '-c']:
        print('@@/usr/share/applications/hearth-hello.desktop')
        print('[Desktop Entry]')
        print('Name=Hearth Hello')
        print('Icon=utilities-terminal')
        sys.exit(0)
    if cmd[0] == 'distrobox-export':
        name = os.path.basename(cmd[2])
        target = os.path.join(os.environ['HOME'], '.local/share/applications', 'hearth-debian-' + name)
        if '--delete' in cmd:
            if os.path.exists(target):
                os.remove(target)
        else:
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with open(target, 'w') as f:
                f.write('[Desktop Entry]\nName=Hearth Hello\nExec=/usr/bin/distrobox-enter -n hearth-debian -- hearth-hello\n')
        sys.exit(0)
sys.exit(1)
'''
FAKE_PODMAN = r'''#!/bin/sh
echo "$*" >> "$FAKE_LOG.podman"
if [ "$1" = exec ]; then
  shift 6
  exec python3 -c 'import json, os, sys; open(os.environ["FAKE_LOG"], "a").write(json.dumps(["root-exec"] + sys.argv[1:]) + "\n")
a = sys.argv[1:]
if "install" in a:
    if not os.path.exists(a[-1]):
        print("E: Unsupported file " + a[-1] + " given on commandline"); sys.exit(100)
    print("dlstatus:1:50:Retrieving file 1 of 2"); print("pmstatus:hearth-hello:20:Preparing hearth-hello"); print("pmstatus:hearth-hello:100:Installed hearth-hello")
' "$@"
fi
case "$1 $2" in
  "container exists") [ -e "$FAKE_STATE" ] ;;
  "rm --force") rm -f "$FAKE_STATE" ;;
  "ps --all") echo "350MB (virtual 480MB)" ;;
  *) exit 0 ;;
esac
'''


class WholeInstall(Base):
    """install/remove/list/remove-box with stand-in distrobox and podman (they record what they were asked)."""
    def setUp(self):
        super().setUp()
        bin_dir = os.path.join(self.tmp, 'bin')
        os.makedirs(bin_dir)
        for name, text in (('distrobox', FAKE_DISTROBOX), ('podman', FAKE_PODMAN)):
            with open(os.path.join(bin_dir, name), 'w') as f:
                f.write(text)
            os.chmod(os.path.join(bin_dir, name), 0o755)
        os.environ['PATH'] = bin_dir + os.pathsep + os.environ.get('PATH', '')
        os.environ['FAKE_LOG'] = os.path.join(self.tmp, 'log')
        os.environ['FAKE_STATE'] = os.path.join(self.tmp, 'box-made')
        self.d.has_subids = lambda *a: True
        self.d.has_nvidia = lambda *a: False

    def calls(self):
        with open(os.environ['FAKE_LOG']) as f:
            return [json.loads(l) for l in f]

    def quiet(self, fn, *a):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            r = fn(*a)
        return r, out.getvalue()

    def test_install_list_owner_remove(self):
        deb = make_test_deb.make(os.path.join(self.home, 'Downloads-hello.deb'))
        r, out = self.quiet(self.d.install, deb)
        self.assertEqual(r['package'], 'hearth-hello')
        self.assertEqual([a['id'] for a in r['apps']], ['hearth-debian-hearth-hello.desktop'])
        self.assertIn('progress: 2 Setting up Hearth\'s Debian box', out)
        self.assertIn('progress: 100 Installed', out)
        calls = self.calls()
        self.assertEqual(calls[0][:3], ['create', '--name', 'hearth-debian'])
        self.assertIn(['enter', '--name', 'hearth-debian', '--no-tty', '--', 'true'], calls)
        inst = next(c for c in calls if c[0] == 'root-exec' and 'install' in c)
        self.assertEqual(inst[-1], os.path.realpath(deb))     # the same path in the box (home is shared)
        exp = next(c for c in calls if 'distrobox-export' in c)
        self.assertEqual(exp[-4:], ['--app', '/usr/share/applications/hearth-hello.desktop', '--export-label', 'none'])
        exported = os.path.join(self.home, '.local/share/applications/hearth-debian-hearth-hello.desktop')
        self.assertTrue(os.path.exists(exported))
        self.assertEqual(self.d.list_packages()['packages'][0]['package'], 'hearth-hello')
        self.assertEqual(self.d.owner('hearth-debian-hearth-hello.desktop')['package'], 'hearth-hello')
        self.assertTrue(self.d.inspect(deb)['installed'])
        st = self.d.status()
        self.assertEqual((st['box'], st['count'], st['size']), (True, 1, '350MB (virtual 480MB)'))
        r, _ = self.quiet(self.d.remove, 'hearth-hello')
        self.assertFalse(os.path.exists(exported))
        self.assertEqual(self.d.list_packages()['packages'], [])
        rm = [c for c in self.calls() if c[0] == 'root-exec' and 'remove' in c]
        self.assertEqual(rm[0][-2:], ['--', 'hearth-hello'])

    def test_a_file_outside_home_is_copied_in(self):
        outside = os.path.join(self.tmp, 'usb')
        os.makedirs(outside)
        deb = make_test_deb.make(os.path.join(outside, 'hello.deb'))
        open(os.environ['FAKE_STATE'], 'w').close()
        self.quiet(self.d.install, deb)
        inst = next(c for c in self.calls() if c[0] == 'root-exec' and 'install' in c)
        self.assertTrue(inst[-1].startswith(os.path.realpath(self.home)), inst[-1])
        self.assertFalse(os.path.exists(inst[-1]))         # removed again afterwards

    def test_remove_box(self):
        deb = make_test_deb.make(os.path.join(self.home, 'hello.deb'))
        self.quiet(self.d.install, deb)
        self.quiet(self.d.remove_box)
        self.assertFalse(os.path.exists(os.environ['FAKE_STATE']))
        self.assertEqual(self.d.our_exports(), [])
        self.assertEqual(self.d.load_records(), {})
        with open(os.environ['FAKE_LOG'] + '.podman') as f:
            self.assertIn('rm --force hearth-debian', f.read())

    def test_no_subids_says_how_to_fix(self):
        self.d.has_subids = lambda *a: False
        with self.assertRaises(self.d.Fail) as e:
            self.quiet(self.d.create)
        self.assertIn('home-repair', str(e.exception))


if __name__ == '__main__':
    unittest.main()
