"""Tests for the move to the new Hearth OS: hearth-admin's side (the plan it leaves, the Flatpak lists, the boot
options' fallbacks) and hearth-clean-install (os/system_files, enabled in every new edition), the new system's first start: what stays in a
made-up home folder and what's moved aside, /etc back to its defaults, and carrying on after an interruption."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import sqlite3
import tempfile
import types
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ci = load(REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-clean-install', 'hearth_clean_install')
ha = load(REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin', 'hearth_admin')


def write(path, text=''):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def slurp(path):
    with open(path) as f:
        return f.read()


def jload(path):
    return json.loads(slurp(path))


def fake_home(home):
    """A home folder after a while on the Bazzite-based Hearth."""
    files = [
        'Documents/taxes.pdf', 'Downloads/thing.zip', 'Pictures/cat.png', '.bashrc', '.config/kdeglobals',
        '.config/heroic/config.json', '.mozilla/firefox/profile/x', '.var/app/org.mozilla.firefox/data',
        # Steam: games, saves, and the rest of Steam (the client itself, logs, its sign-in)
        '.local/share/Steam/steamapps/common/Hades/Hades.exe', '.local/share/Steam/steamapps/compatdata/1145360/pfx/user.reg',
        '.local/share/Steam/steamapps/libraryfolders.vdf', '.local/share/Steam/userdata/123/remote/save.dat',
        '.local/share/Steam/config/loginusers.vdf', '.local/share/Steam/ubuntu12_32/steamclient.so', '.local/share/Steam/logs/a.txt',
        # Hearth: Epic and GOG games in ~/Games, their prefixes, records, and the rest of its data
        'Games/Epic/Fortnite/x.exe', 'Games/GOG/Witcher/goggame-1.info', 'Games/Lutris/Doom/doom.exe', 'Games/Mine/run.sh', 'Games/notes.txt',
        '.local/share/nimbus/prefixes/epic-abc/drive_c/x', '.local/share/nimbus/legendary/metadata/abc.json',
        '.local/share/nimbus/legendary/user.json', '.local/share/nimbus/art/1.jpg', '.config/nimbus/controller.json',
        # Heroic and Lutris
        'Games/Heroic/Prefixes/default/Celeste/pfx', 'OtherGames/Celeste/Celeste.exe', 'LutrisGames/quake/quake.exe',
        '.local/share/lutris/pga.db.placeholder',
        # a game added by hand straight in Downloads
        'Downloads/Game.AppImage',
        # Steam library on another drive: not in home at all
    ]
    for f in files:
        write(os.path.join(home, f), 'x')
    os.symlink(os.path.join(home, '.local/share/Steam'), os.path.join(home, '.local/share/steam-link'))
    os.makedirs(os.path.join(home, '.steam'))
    os.symlink(os.path.join(home, '.local/share/Steam'), os.path.join(home, '.steam/steam'))
    write(os.path.join(home, '.local/share/nimbus/legendary/installed.json'),
          json.dumps({'abc': {'install_path': os.path.join(home, 'Games/Epic/Fortnite'), 'title': 'Fortnite'},
                      'far': {'install_path': '/run/media/sd/Epic/Far', 'title': 'Far'}}))
    write(os.path.join(home, '.local/share/nimbus/library.json'),
          json.dumps({'favorites': ['steam:1'], 'stats': {'x': 1}, 'gogInstalls': {'1': os.path.join(home, 'Games/GOG/Witcher')},
                      'manual': [{'id': 'manual:1', 'path': os.path.join(home, 'Games/Mine/run.sh')},
                                 {'id': 'manual:2', 'path': os.path.join(home, 'Downloads/Game.AppImage')},
                                 {'id': 'manual:3', 'path': '/usr/bin/true'}]}))
    write(os.path.join(home, '.config/heroic/legendaryConfig/legendary/installed.json'),
          json.dumps({'cel': {'install_path': os.path.join(home, 'OtherGames/Celeste')}}))
    db = sqlite3.connect(os.path.join(home, '.local/share/lutris/pga.db'))
    db.execute('CREATE TABLE games (slug TEXT, directory TEXT, installed INTEGER)')
    db.execute('INSERT INTO games VALUES (?, ?, 1)', ('quake', os.path.join(home, 'LutrisGames/quake')))
    db.execute('INSERT INTO games VALUES (?, ?, 0)', ('gone', os.path.join(home, 'Documents')))
    db.commit()
    db.close()


class HomePlan(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp()
        fake_home(self.home)

    def test_keeps_only_games(self):
        keep = ci.keep_list(self.home)
        self.assertEqual(keep, sorted([
            '.local/share/Steam/steamapps', '.local/share/Steam/userdata',
            '.local/share/nimbus/prefixes', '.local/share/nimbus/legendary/installed.json', '.local/share/nimbus/legendary/metadata',
            '.local/share/nimbus/library.json',
            'Games/Epic', 'Games/GOG', 'Games/Lutris', 'Games/Heroic', 'Games/Mine',
            'Downloads/Game.AppImage', 'OtherGames/Celeste', 'LutrisGames/quake', '.steam/steam',
        ]))

    def test_moves_the_rest_in_big_pieces(self):
        todo = ci.plan_home(self.home, ci.keep_list(self.home))
        for whole in ('Documents', 'Pictures', '.bashrc', '.config', '.mozilla', '.var', 'Games/notes.txt',
                      '.local/share/Steam/config', '.local/share/Steam/ubuntu12_32', '.local/share/nimbus/art',
                      '.local/share/nimbus/legendary/user.json', '.local/share/lutris', '.local/share/steam-link', 'Downloads/thing.zip'):
            self.assertIn(whole, todo)
        for kept in ('Games', 'Games/Epic', '.local/share/Steam', '.steam', '.steam/steam', '.local/share/Steam/steamapps', 'Downloads', 'Downloads/Game.AppImage',
                     'OtherGames/Celeste', '.local', '.local/share'):
            self.assertNotIn(kept, todo)
        # Nothing is both kept and moved, and nothing inside a moved folder is listed again.
        self.assertFalse(any(a != b and b.startswith(a + '/') for a in todo for b in todo))

    def test_runs_and_carries_on(self):
        root = tempfile.mkdtemp()
        home = os.path.join(root, 'var/home/sam')
        os.makedirs(os.path.dirname(home))
        os.rename(self.home, home)
        # Paths in the records point at the old place: write them again for this one.
        fake_home_records = jload(os.path.join(home, '.local/share/nimbus/library.json'))
        fake_home_records['gogInstalls'] = {'1': os.path.join(home, 'Games/GOG/Witcher')}
        fake_home_records['manual'] = [{'id': 'manual:1', 'path': os.path.join(home, 'Games/Mine/run.sh')}]
        write(os.path.join(home, '.local/share/nimbus/library.json'), json.dumps(fake_home_records))
        write(os.path.join(root, 'var/lib/hearth/clean-install.json'), json.dumps({'version': 1, 'user': 'sam', 'uid': os.getuid(), 'home': '/var/home/sam', 'from': 'Hearth OS (based on Bazzite)'}))
        write(os.path.join(root, 'etc/hostname'), 'deck')
        write(os.path.join(root, 'usr/etc/hostname'), 'localhost')
        write(os.path.join(root, 'usr/etc/os-release'), 'NAME=Hearth')
        # On the old (Bazzite-based) system, started again from the boot menu: nothing happens.
        ci.ROOT = root
        try:
            self.assertEqual(ci.main(['x']), 0)
        finally:
            ci.ROOT = '/'
        self.assertTrue(os.path.isdir(os.path.join(home, 'Documents')))
        self.assertTrue(os.path.exists(os.path.join(root, ci.PLAN)))
        write(os.path.join(root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'anatase'}))
        # Interrupted after /etc and /var: only the home step is left.
        write(os.path.join(root, 'var/lib/hearth/clean-install-state.json'), json.dumps({'stamp': '2026-10-19', 'done': ['etc', 'var']}))
        ci.ROOT = root
        try:
            self.assertEqual(ci.main(['x']), 0)
        finally:
            ci.ROOT = '/'
        old = os.path.join(home, '.old-hearth-2026-10-19')
        self.assertTrue(os.path.isfile(os.path.join(old, 'Documents/taxes.pdf')))
        self.assertTrue(os.path.isfile(os.path.join(home, '.local/share/Steam/steamapps/common/Hades/Hades.exe')))
        self.assertTrue(os.path.isfile(os.path.join(home, '.local/share/Steam/userdata/123/remote/save.dat')))
        self.assertFalse(os.path.exists(os.path.join(home, '.local/share/Steam/config')))
        self.assertTrue(os.path.isfile(os.path.join(old, '.local/share/Steam/config/loginusers.vdf')))
        self.assertFalse(os.path.exists(os.path.join(home, 'Documents')))
        # /etc wasn't touched (that step was done already)
        self.assertEqual(slurp(os.path.join(root, 'etc/hostname')), 'deck')
        man = jload(os.path.join(old, ci.MANIFEST))
        self.assertIn('Documents', man['moved'])
        self.assertEqual(man['deleteAfter'] - man['created'], 30 * 86400)
        # library.json trimmed to what finds the games; the whole one kept in the old folder.
        self.assertEqual(sorted(jload(os.path.join(home, '.local/share/nimbus/library.json'))), ['gogInstalls', 'manual'])
        self.assertIn('favorites', jload(os.path.join(old, '.local/share/nimbus/library.json')))
        self.assertTrue(os.path.exists(os.path.join(root, ci.DONE)))
        self.assertFalse(os.path.exists(os.path.join(root, ci.PLAN)))
        # Run again (no plan): nothing happens.
        ci.ROOT = root
        try:
            self.assertEqual(ci.main(['x']), 0)
        finally:
            ci.ROOT = '/'


class Etc(unittest.TestCase):
    def test_defaults_except_what_keeps_you_in(self):
        root = tempfile.mkdtemp()
        etc, usr = os.path.join(root, 'etc'), os.path.join(root, 'usr/etc')
        for rel, a, b in (('passwd', 'sam:x:1000', 'root:x:0'), ('hostname', 'deck', 'localhost'), ('sddm.conf.d/zz.conf', 'x', None),
                          ('hearth/no-hhd', '', None), ('profile', 'mine', 'default'), ('os-release', 'same', 'same'),
                          ('NetworkManager/system-connections/home.nmconnection', 'psk', None), ('ssh/ssh_host_ed25519_key', 'k', None),
                          ('ssh/sshd_config', 'mine', 'default'), ('plasmalogin.conf', None, '[Autologin]')):
            if a is not None:
                write(os.path.join(etc, rel), a)
            if b is not None:
                write(os.path.join(usr, rel), b)
        os.makedirs(os.path.join(etc, 'systemd/system/multi-user.target.wants'))
        os.symlink('/usr/lib/systemd/system/bazzite-thing.service', os.path.join(etc, 'systemd/system/multi-user.target.wants/bazzite-thing.service'))
        restore, add, aside = ci.plan_etc(etc, usr)
        self.assertEqual(sorted(restore), ['profile', 'ssh/sshd_config'])
        self.assertEqual(add, ['plasmalogin.conf'])
        self.assertEqual(sorted(aside), ['hearth/no-hhd', 'sddm.conf.d/zz.conf', 'systemd/system/multi-user.target.wants/bazzite-thing.service'])
        ci.ROOT = root
        try:
            ci.reset_etc('2026-10-19')
        finally:
            ci.ROOT = '/'
        self.assertEqual(slurp(os.path.join(etc, 'profile')), 'default')
        self.assertEqual(slurp(os.path.join(etc, 'passwd')), 'sam:x:1000')
        self.assertEqual(slurp(os.path.join(etc, 'hostname')), 'deck')
        self.assertTrue(os.path.exists(os.path.join(etc, 'NetworkManager/system-connections/home.nmconnection')))
        self.assertFalse(os.path.exists(os.path.join(etc, 'sddm.conf.d')))
        backup = os.path.join(root, 'var/lib/hearth/old-etc-2026-10-19')
        self.assertEqual(slurp(os.path.join(backup, 'profile')), 'mine')
        self.assertTrue(os.path.exists(os.path.join(backup, 'sddm.conf.d/zz.conf')))
        self.assertEqual(slurp(os.path.join(etc, 'plasmalogin.conf')), '[Autologin]')


class OtherEditions(unittest.TestCase):
    OLD = {
        'passwd': 'root:x:0:0:root:/root:/bin/bash\nsystemd-network:x:192:192::/:/sbin/nologin\nsam:x:1000:1000:Sam:/var/home/sam:/bin/bash\n',
        'group': 'root:x:0:\nwheel:x:10:sam\nvideo:x:39:sam\ngamemode:x:977:sam\nsam:x:1000:\n',
        'shadow': 'root:!locked::0:99999:7:::\nsam:$6$hash:19000:0:99999:7:::\n',
        'gshadow': 'wheel:::sam\nsam:!::\n',
    }
    NEW = {
        'passwd': 'root:x:0:0:root:/root:/bin/bash\nsddm:x:110:115::/var/lib/sddm:/usr/sbin/nologin\n',
        'group': 'root:x:0:\nwheel:x:27:\nvideo:x:44:\nsddm:x:115:\n',
        'shadow': 'root:*:20000:0:99999:7:::\nsddm:!:20000::::::\n',
        'gshadow': 'root:*::\nwheel:!::\nvideo:!::\n',
    }

    def system(self):
        root = tempfile.mkdtemp()
        for name, text in self.OLD.items():
            write(os.path.join(root, 'etc', name), text)
        for name, text in self.NEW.items():
            write(os.path.join(root, 'usr/etc', name), text)
        write(os.path.join(root, 'etc/ld.so.cache'), 'fedora')
        write(os.path.join(root, 'usr/etc/ld.so.cache'), 'debian')
        write(os.path.join(root, 'etc/hostname'), 'deck')
        write(os.path.join(root, 'etc/selinux/config'), 'SELINUX=enforcing')
        return root

    def test_accounts_merge(self):
        root = self.system()
        m = ci.merge_accounts(os.path.join(root, 'etc'), os.path.join(root, 'usr/etc'))
        self.assertEqual(m['passwd'], self.NEW['passwd'] + 'sam:x:1000:1000:Sam:/var/home/sam:/bin/bash\n')
        self.assertEqual(m['group'], 'root:x:0:\nwheel:x:27:sam\nvideo:x:44:sam\nsddm:x:115:\nsam:x:1000:\n')
        self.assertEqual(m['shadow'], self.NEW['shadow'] + 'sam:$6$hash:19000:0:99999:7:::\n')
        self.assertEqual(m['gshadow'], 'root:*::\nwheel:!::sam\nvideo:!::sam\nsam:!::\n')
        # No account files of its own: nothing to merge (the old ones are kept).
        self.assertIsNone(ci.merge_accounts(os.path.join(root, 'etc'), os.path.join(root, 'nothing')))

    def test_reset_on_ubuntu(self):
        root = self.system()
        write(os.path.join(root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'ubuntu'}))
        ci.ROOT = root
        try:
            ci.reset_etc('2026-10-19')
        finally:
            ci.ROOT = '/'
        etc = os.path.join(root, 'etc')
        self.assertIn('sam:x:1000', slurp(os.path.join(etc, 'passwd')))
        self.assertIn('sddm:x:110', slurp(os.path.join(etc, 'passwd')))
        self.assertNotIn('systemd-network', slurp(os.path.join(etc, 'passwd')))
        self.assertIn('wheel:x:27:sam', slurp(os.path.join(etc, 'group')))
        self.assertIn('sam:$6$hash', slurp(os.path.join(etc, 'shadow')))
        self.assertEqual(slurp(os.path.join(etc, 'ld.so.cache')), 'debian')
        self.assertEqual(slurp(os.path.join(etc, 'hostname')), 'deck')
        self.assertFalse(os.path.exists(os.path.join(etc, 'selinux')))
        backup = os.path.join(root, 'var/lib/hearth/old-etc-2026-10-19')
        self.assertIn('systemd-network', slurp(os.path.join(backup, 'passwd')))

    def test_anatase_keeps_accounts(self):
        root = self.system()
        write(os.path.join(root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'anatase'}))
        ci.ROOT = root
        try:
            ci.reset_etc('2026-10-19')
        finally:
            ci.ROOT = '/'
        self.assertEqual(slurp(os.path.join(root, 'etc/passwd')), self.OLD['passwd'])
        self.assertEqual(slurp(os.path.join(root, 'etc/ld.so.cache')), 'fedora')


class SwitchEditions(unittest.TestCase):
    def test_which_moves_rebuild_the_accounts(self):
        self.assertFalse(ci.foreign_accounts('anatase', 'bazzite'), 'Bazzite to Anatase: both Fedora')
        self.assertFalse(ci.foreign_accounts('anatase', None))
        self.assertTrue(ci.foreign_accounts('ubuntu', 'bazzite'))
        self.assertTrue(ci.foreign_accounts('anatase', 'ubuntu'), 'Ubuntu to Anatase: Ubuntu\'s accounts don\'t fit Fedora')
        self.assertTrue(ci.foreign_accounts('peppermint', 'cachyos'))
        self.assertTrue(ci.foreign_accounts('cachyos', 'anatase'))


class Admin(unittest.TestCase):
    def test_never_on_another_edition(self):
        # A plan for CachyOS, but Anatase started (the old system from the boot menu, or a switch that never
        # finished): nothing is touched and the plan stays for the system it's for.
        root = tempfile.mkdtemp()
        write(os.path.join(root, 'var/lib/hearth/clean-install.json'), json.dumps({'version': 1, 'user': 'sam', 'uid': os.getuid(), 'home': '/var/home/sam', 'edition': 'cachyos'}))
        write(os.path.join(root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'anatase'}))
        write(os.path.join(root, 'var/home/sam/Documents/taxes.pdf'), 'x')
        write(os.path.join(root, 'etc/hostname'), 'deck')
        ci.ROOT = root
        try:
            self.assertEqual(ci.main(['x']), 0)
        finally:
            ci.ROOT = '/'
        self.assertTrue(os.path.isfile(os.path.join(root, 'var/home/sam/Documents/taxes.pdf')))
        self.assertTrue(os.path.exists(os.path.join(root, ci.PLAN)))
        self.assertFalse(os.path.exists(os.path.join(root, ci.STATE)))

    def test_switch_writes_the_plan_only_when_it_worked(self):
        from unittest import mock
        tmp = tempfile.mkdtemp()
        plan = os.path.join(tmp, 'clean-install.json')
        write(plan, '{"edition": "ubuntu"}')   # left from a switch that never finished
        user = types.SimpleNamespace(pw_name='sam', pw_uid=1000, pw_dir='/var/home/sam', pw_gid=1000)
        calls = []

        def fails(*a, **k):
            calls.append(a)
            raise ha.UpgradeFailed('error: Switching: pinging container registry ghcr.io: Get "https://ghcr.io/v2/": dial tcp: lookup ghcr.io: Temporary failure in name resolution')
        with mock.patch.object(ha, 'caller', lambda: user), mock.patch.object(ha, 'CLEAN_INSTALL', plan), \
                mock.patch.object(ha.os.path, 'exists', lambda x: True if x == '/run/ostree-booted' else os.path.lexists(x)), \
                mock.patch.object(ha, 'own_edition', lambda: 'anatase'), mock.patch.object(ha, 'ensure_podman', lambda: None), \
                mock.patch.object(ha, 'layered', lambda: False), mock.patch.object(ha, 'bootc_upgrade_with_progress', fails), \
                mock.patch.object(ha.time, 'sleep', lambda s: None), mock.patch('builtins.print') as out:
            with self.assertRaises(SystemExit):
                ha.move_to_new_os('ubuntu', False)
        self.assertEqual(len(calls), 3)   # tried again twice after the dropped connection
        self.assertFalse(os.path.exists(plan))
        said = [c[0][0] for c in out.call_args_list if c[0] and str(c[0][0]).startswith('error: ')]
        self.assertEqual(said, ["error: couldn't get the new Hearth OS: the internet connection dropped while downloading. Check the connection and try again"])
        ok = lambda *a, **k: None
        with mock.patch.object(ha, 'caller', lambda: user), mock.patch.object(ha, 'CLEAN_INSTALL', plan), \
                mock.patch.object(ha.os.path, 'exists', lambda x: True if x == '/run/ostree-booted' else os.path.lexists(x)), \
                mock.patch.object(ha, 'own_edition', lambda: 'anatase'), mock.patch.object(ha, 'ensure_podman', lambda: None), \
                mock.patch.object(ha, 'layered', lambda: False), mock.patch.object(ha, 'bootc_upgrade_with_progress', ok), \
                mock.patch.object(ha, 'bootc_status', lambda: {}), mock.patch('builtins.print'):
            ha.move_to_new_os('ubuntu', False)
        self.assertEqual(jload(plan)['edition'], 'ubuntu')

    def test_editions(self):
        self.assertEqual(ha.new_os_image('anatase'), 'ghcr.io/dragongamer9/hearth-os:latest')
        self.assertEqual(ha.new_os_image('anatase', True), 'ghcr.io/dragongamer9/hearth-os:testing')
        for e in ('ubuntu', 'cachyos', 'peppermint'):
            self.assertEqual(ha.new_os_image(e), 'ghcr.io/dragongamer9/hearth-%s:testing' % e)
            self.assertEqual(ha.new_os_image(e, True), 'ghcr.io/dragongamer9/hearth-%s:testing' % e)
        self.assertIsNone(ha.new_os_image('ghcr.io/evil/image:latest'))
        self.assertEqual(ha.move_args([]), ('anatase', False))
        self.assertEqual(ha.move_args(['--testing']), ('anatase', True))
        self.assertEqual(ha.move_args(['--edition', 'cachyos']), ('cachyos', False))
        self.assertEqual(ha.move_args(['--testing', '--edition', 'ubuntu']), ('ubuntu', True))
        for bad in (['--edition'], ['--edition', 'fedora'], ['--edition', 'ghcr.io/x/y:z'], ['--testing', '--testing'],
                    ['--edition', 'ubuntu', '--edition', 'cachyos'], ['--image', 'x'], ['--edition', '--testing']):
            self.assertIsNone(ha.move_args(bad), bad)

    def test_x86_64_v3(self):
        v3 = 'processor\t: 0\nflags\t\t: fpu sse2 avx2 bmi1 bmi2 fma movbe\n\nprocessor\t: 1\nflags\t\t: avx2 bmi2 fma movbe\n'
        self.assertTrue(ha.x86_64_v3(v3))
        self.assertFalse(ha.x86_64_v3('processor\t: 0\nflags\t\t: fpu sse2 avx2 bmi2 fma\n'))
        self.assertFalse(ha.x86_64_v3(''))

    def test_flatpak_lists(self):
        text = 'org.mozilla.firefox\n# comment\napp/org.kde.okular/x86_64/stable\nruntime/org.freedesktop.Platform.VulkanLayer.MangoHud/x86_64/25.08\n' \
               'org.freedesktop.Platform.VulkanLayer.vkBasalt//23.08\nnot-an-id\norg.mozilla.firefox\n'
        self.assertEqual(ha.flatpak_ids(text), ['org.mozilla.firefox', 'org.kde.okular', 'org.freedesktop.Platform.VulkanLayer.vkBasalt'])
        d = tempfile.mkdtemp()
        write(os.path.join(d, 'bazzite/flatpak/install'), 'org.mozilla.firefox\napp/org.kde.haruna/x86_64/stable\n')
        self.assertEqual(ha.bazzite_flatpaks([os.path.join(d, '**/flatpak*/*install*')]), ['org.mozilla.firefox', 'org.kde.haruna'])

    def test_plan(self):
        user = types.SimpleNamespace(pw_name='sam', pw_uid=1000, pw_dir='/var/home/sam')
        p = ha.clean_install_plan(user, ['org.mozilla.firefox'], 'ghcr.io/dragongamer9/hearth-os:latest', now=1700000000)
        self.assertEqual((p['user'], p['uid'], p['home'], p['created'], p['oldFilesDays']), ('sam', 1000, '/var/home/sam', 1700000000, 30))
        self.assertIn('steam-library', p['keep'])
        self.assertEqual(p['edition'], 'anatase')
        self.assertEqual(ha.clean_install_plan(user, [], 'x', now=1, edition='ubuntu')['edition'], 'ubuntu')
        self.assertEqual(p['removeFlatpaks'], ['org.mozilla.firefox'])
        self.assertEqual(p['fromEdition'], 'bazzite')
        self.assertEqual(ha.clean_install_plan(user, [], 'x', now=1, edition='cachyos', from_edition='anatase')['fromEdition'], 'anatase')

    def test_own_edition(self):
        d = tempfile.mkdtemp()
        f = os.path.join(d, 'image-info.json')
        self.assertIsNone(ha.own_edition(f), 'the Bazzite-based Hearth has none')
        write(f, '{"base": "ubuntu"}')
        self.assertEqual(ha.own_edition(f), 'ubuntu')
        write(f, '{"base": "something"}')
        self.assertIsNone(ha.own_edition(f))

    def test_boot_options_fall_back_to_ostree(self):
        old, want = ['resume=UUID=a', 'resume_offset=1'], ['resume=UUID=b', 'resume_offset=2']
        cmds = ha.kargs_commands(old, want, rpm_ostree=False, ostree=True)
        self.assertEqual(cmds, [['ostree', 'admin', 'instutil', 'set-kargs', '--merge', '--replace=resume=UUID=b', '--replace=resume_offset=2']])
        cmds = ha.kargs_commands([], want, rpm_ostree=True, ostree=True)
        self.assertEqual(cmds[0][:2], ['rpm-ostree', 'kargs'])
        self.assertEqual(cmds[-1], ['ostree', 'admin', 'kargs', 'edit-in-place', '--append-if-missing=resume=UUID=b', '--append-if-missing=resume_offset=2'])
        self.assertEqual(ha.kargs_commands(old, want, rpm_ostree=False, ostree=False), [])


if __name__ == '__main__':
    unittest.main()
