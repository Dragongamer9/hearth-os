"""An update must never sign anyone out of their game accounts. A home folder as Hearth 0.6.3 left it (Epic, GOG and
Xbox signed in to Hearth, the sign-in windows' WebKit cookies, Steam signed in, a desktop keyring) goes through
everything that runs at boot and at sign-in since 0.6.4 (the home folder repair, /etc/hosts, the Flatpak setup, the
clean install's first start, the session's password store and default apps), and every sign-in must come out exactly
as it went in: same bytes, same permissions, still where Hearth looks for it.
(The Rust side, hosts/hearth-core/src/signins.rs, has the matching tests for Hearth itself: missing sign-ins put
back, private backups, the per-store log line.)"""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import os
import pathlib
import re
import stat
import subprocess
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
NIMBUS = REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'
SESSION = REPO / 'hosts' / 'linux' / 'session' / 'nimbus-session'

# What each store's sign-in is, where Hearth (0.6.3 and every version since) keeps it. Fake values only.
SIGNINS = {
    '.local/share/nimbus/legendary/user.json': '{"access_token": "zz-epic-SECRET", "refresh_token": "zz-epic-r-SECRET", "account_id": "a1", "displayName": "Zqx"}',
    '.local/share/nimbus/gog/auth.json': '{"46899977096215655": {"access_token": "zz-gog-SECRET", "refresh_token": "zz-gog-r-SECRET", "expires_in": 3600, "loginTime": 1}}',
    '.local/share/nimbus/gog/profile.json': '{"name": "Zqx", "avatar": null}',
    '.local/share/nimbus/accounts/xbox.bin': 'P1{"refreshToken": "zz-xbox-SECRET", "gamertag": "Zqx"}',
    '.local/share/io.github.dragongamer9.hearth/cookies': '# Netscape HTTP Cookie File\n.gog.com\tTRUE\t/\tTRUE\t1999999999\tgog-al\tzz-SECRET\n',
    '.local/share/nimbus/xcloud-web/cookies': '# Netscape HTTP Cookie File\n.xbox.com\tTRUE\t/\tTRUE\t1999999999\tx\tzz-SECRET\n',
    '.steam/registry.vdf': '"Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "zqxsam" "RememberPassword" "1" } } } } }',
    '.steam/debian-installation/config/loginusers.vdf': '"users" { "76561197960265729" { "AccountName" "zqxsam" "MostRecent" "1" "AllowAutoLogin" "1" } }',
    '.steam/debian-installation/config/config.vdf': '"InstallConfigStore" { "Software" { "Valve" { "Steam" { } } } }',
    '.steam/debian-installation/local.vdf': '"MachineUserConfigStore" { }',
    '.local/share/keyrings/login.keyring': 'GnomeKeyring\n\r\x00\n fake',
    '.local/share/keyrings/default': 'login',
}
PRIVATE = {'.local/share/nimbus/gog/auth.json', '.local/share/nimbus/accounts/xbox.bin', '.local/share/keyrings/login.keyring'}


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def make_home(home):
    for rel, text in SIGNINS.items():
        p = home / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)
        os.chmod(p, 0o600 if rel in PRIVATE else 0o644)
    os.chmod(home / '.local/share/keyrings', 0o700)
    # The rest of a lived-in home folder that the boot and sign-in steps look at.
    (home / '.config').mkdir(exist_ok=True)
    (home / '.config/mimeapps.list').write_text('[Default Applications]\ntext/plain=hearth-gone-app.desktop;\nx-scheme-handler/steam=steam.desktop\n')
    for d in ('.cache/nimbus/apps', '.cache/flatpak', '.local/share/flatpak', '.local/state', '.var/app', '.local/bin',
              '.local/share/applications', '.local/share/icons'):
        (home / d).mkdir(parents=True, exist_ok=True)
    (home / '.cache/nimbus/apps/catalog.json').write_text('{}')


def snapshot(home):
    out = {}
    for rel in SIGNINS:
        p = home / rel
        st = p.lstat()
        out[rel] = (p.read_bytes(), stat.S_IMODE(st.st_mode), st.st_uid)
    return out


def clean_env(home, **extra):
    env = {k: v for k, v in os.environ.items() if not k.startswith('XDG_')}
    env.update({'HOME': str(home), 'XDG_CURRENT_DESKTOP': 'Nimbus', 'PYTHONDONTWRITEBYTECODE': '1'})
    env.update(extra)
    return env


class UpdateKeepsSignIns(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        self.home = self.root / 'home' / 'player'
        self.home.mkdir(parents=True)
        make_home(self.home)
        self.before = snapshot(self.home)

    def tearDown(self):
        self.tmp.cleanup()

    def assertSignedIn(self, step):
        after = snapshot(self.home)
        for rel, (data, mode, uid) in self.before.items():
            self.assertIn(rel, after, '%s: %s is gone' % (step, rel))
            self.assertEqual(after[rel][0], data, '%s changed %s' % (step, rel))
            self.assertEqual(after[rel][1], mode, '%s changed the permissions of %s' % (step, rel))
            self.assertEqual(after[rel][2], uid, '%s changed the owner of %s' % (step, rel))

    def test_home_folder_repair(self):
        """hearth-home-repair (every boot since 0.6.4): only ownership, only of what root or a deleted account owns."""
        hr = load(NIMBUS / 'hearth-home-repair', 'hr_keep')
        r = hr.Repair(os.getuid(), os.getgid(), log=lambda m: None)
        changed = r.run(str(self.home))
        self.assertEqual(changed, [])
        self.assertSignedIn('hearth-home-repair')
        # It never walks into a sign-in folder: the only folders it goes all the way through are Flatpak's and caches.
        deep = [rel for rel, d in hr.PATHS if d]
        for rel in deep:
            self.assertTrue(rel in ('.local/share/flatpak', '.cache/flatpak', '.cache/nimbus'), rel)
        for rel in SIGNINS:
            self.assertFalse(any(rel.startswith(d + '/') for d in deep), '%s is inside a folder the repair walks' % rel)

    def test_hosts_and_flatpak_setup_and_clean_install(self):
        """hearth-hosts, hearth-flatpak-setup and hearth-clean-install (boot, as root) leave the home folder alone."""
        hosts = self.root / 'hosts'
        hosts.write_text('127.0.0.1\tlocalhost\n')
        subprocess.run([sys.executable, str(NIMBUS / 'hearth-hosts'), '--file', str(hosts), '--hostname', 'player-hearth'],
                       check=True, capture_output=True, env=clean_env(self.home))
        fake_root = self.root / 'sysroot'
        (fake_root / 'var/lib/flatpak').mkdir(parents=True)
        subprocess.run([sys.executable, str(NIMBUS / 'hearth-flatpak-setup'), '--root', str(fake_root)],
                       check=True, capture_output=True, env=clean_env(self.home))
        r = subprocess.run([sys.executable, str(NIMBUS / 'hearth-clean-install'), '--root', str(fake_root)],
                           capture_output=True, text=True, env=clean_env(self.home))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn('nothing to do', r.stdout)
        self.assertSignedIn('the boot services')

    def test_default_apps_tidy_at_sign_in(self):
        """nimbus-session runs hearth-system defaults-tidy at every sign-in (0.6.4): ~/.config/mimeapps.list only."""
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-system'), 'defaults-tidy'], capture_output=True, text=True,
                           env=clean_env(self.home))
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertSignedIn('defaults-tidy')
        # The steam:// link handler (Steam's sign-in and store links) is kept.
        self.assertIn('x-scheme-handler/steam=steam.desktop', (self.home / '.config/mimeapps.list').read_text())

    def test_password_store_keeps_the_keyring(self):
        """nimbus-session's password store: an existing desktop keyring is never replaced or emptied."""
        text = SESSION.read_text()
        m = re.search(r'^password_store\(\) \{\n.*?^\}\n', text, re.S | re.M)
        self.assertIsNotNone(m, 'password_store() is in nimbus-session')
        fakebin = self.root / 'bin'
        fakebin.mkdir()
        daemon = fakebin / 'gnome-keyring-daemon'
        daemon.write_text('#!/bin/sh\necho GNOME_KEYRING_CONTROL=/tmp/none\n')
        daemon.chmod(0o755)
        script = 'have() { command -v "$1" >/dev/null 2>&1; }\n' + m.group(0) + 'password_store\n'
        env = clean_env(self.home, PATH=str(fakebin) + ':' + os.environ.get('PATH', '/usr/bin:/bin'))
        r = subprocess.run(['bash', '-c', script], capture_output=True, text=True, env=env)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse((self.home / '.local/share/keyrings/hearth.keyring').exists(), 'a second keyring was made next to yours')
        self.assertSignedIn('password_store')

    def test_session_environment_keeps_hearths_folders(self):
        """Nothing in the session moves where Hearth or the sign-in windows keep their files: no XDG_DATA_HOME,
        XDG_CONFIG_HOME or HOME of its own (0.6.4 added portal, browser and WebKit variables only)."""
        text = SESSION.read_text()
        for var in ('XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_STATE_HOME', 'HOME'):
            self.assertIsNone(re.search(r'^\s*(export\s+)?%s=' % var, text, re.M), '%s is set in nimbus-session' % var)
            self.assertIsNone(re.search(r'export\s+[^\n]*\b%s=' % var, text), '%s is exported in nimbus-session' % var)

    def test_everything_together(self):
        """All of it, in the order a boot and a sign-in run it."""
        self.test_home_folder_repair()
        self.test_hosts_and_flatpak_setup_and_clean_install()
        self.test_default_apps_tidy_at_sign_in()
        self.test_password_store_keeps_the_keyring()
        self.assertSignedIn('a whole boot and sign-in')


class RustSafetyNetIsWiredIn(unittest.TestCase):
    """The Rust host's safety net (signins.rs) runs before anything else and before every change to sign-ins."""
    CORE = REPO / 'hosts' / 'hearth-core' / 'src'

    def test_called_where_sign_ins_change(self):
        bridge = (self.CORE / 'bridge.rs').read_text()
        new = bridge[bridge.index('pub fn new(version: &str) -> Bridge {'):]
        self.assertLess(new.index('crate::signins::at_start(version)'), new.index('Store::open('), 'at_start runs first')
        self.assertIn('crate::signins::backup("before switching profiles")', bridge)
        self.assertIn('crate::signins::backup("before restoring a Hearth backup")', bridge)
        stores = (self.CORE / 'stores.rs').read_text()
        so = stores[stores.index('pub fn sign_out(s: &str)'):]
        self.assertLess(so.index('crate::signins::backup('), so.index('epic::sign_out()'))
        self.assertIn('crate::signins::mark_signed_out(s)', so)
        profiles = (self.CORE / 'profiles.rs').read_text()
        ss = profiles[profiles.index('pub fn set_steam_account'):]
        self.assertLess(ss.index('backup_with_steam'), ss.index('write_atomic'))

    def test_backups_stay_on_the_device(self):
        backup = (self.CORE / 'backup.rs').read_text()
        self.assertRegex(backup, r'SKIP: \[&str; \d+\] = \[[^\]]*"signin-backup"')
        sync = (REPO / 'hosts' / 'hearth-link' / 'src' / 'sync.rs').read_text()
        self.assertNotIn('signin-backup', sync)
        signins = (self.CORE / 'signins.rs').read_text()
        # Hearth never touches Steam's saved sign-in itself.
        code = signins[:signins.index('#[cfg(test)]')]
        for f in ('local.vdf', 'config.vdf', 'ConnectCache', 'steamclient'):
            self.assertNotIn('"%s' % f, code)
            self.assertNotIn("join(\"config/%s" % f, code)


if __name__ == '__main__':
    unittest.main()
