"""Red flag 9 (Nov 27, 2026): a move between Hearth editions keeps the whole home folder, nothing put aside is ever
deleted by itself, and Bring back never replaces anything.

What happened: on Nov 24, 2026 the owner's Nvidia PC moved from the Fedora edition (1.17) to Debian. hearth-admin's
move wrote a plan with keepHome, but hearth-peppermint:testing was still the 1.13 image (pointed back at it on Nov 18),
whose hearth-clean-install didn't know keepHome: its first start put nearly his whole home folder (Documents, .config,
.local, Voice Studio's voices...) into ~/.old-hearth-2026-11-24, to be deleted after 30 days by Settings."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import pwd
import re
import subprocess
import tempfile
import types
import unittest
from unittest import mock

REPO = pathlib.Path(__file__).resolve().parents[3]
NIMBUS = REPO / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'
SHELL = REPO / 'hosts' / 'linux' / 'desktop' / 'shell'
OLD_FILES = SHELL / 'hearth-old-files'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


ci = load(NIMBUS / 'hearth-clean-install', 'hearth_clean_install_kh')
ha = load(NIMBUS / 'hearth-admin', 'hearth_admin_kh')


def write(path, text='x'):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def slurp(path):
    with open(path) as f:
        return f.read()


# A home folder like the owner's: hidden settings folders of every kind, Steam, and his own folders.
OWNER_HOME = [
    'Documents/notes.txt', 'Pictures/a.png', 'Videos/v.mp4', 'Music/m.ogg', 'Desktop/x.desktop', 'Downloads/d.zip',
    'Projects/p/main.rs', 'ComfyUI/models/m.safetensors', 'HearthAI/memory.json', 'Saved Games/s.sav',
    'Hearth Storage/s.bin', 'Games/Mine/run.sh', 'Briefcase/b.txt', 'homebrew/plugins/p.py',
    '.config/nimbus/settings.json', '.config/nimbus/desktop.json', '.config/kdeglobals',
    '.local/share/nimbus/voice-studio/voices/ithil.wav', '.local/share/nimbus/library.json',
    '.local/share/Steam/steamapps/common/Hades/Hades.exe', '.local/share/Steam/config/loginusers.vdf',
    '.local/state/nimbus/x.json', '.claude/settings.json', '.codex/config.toml', '.gemini/settings.json',
    '.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/libraryfolders.vdf', '.var/app/org.mozilla.firefox/x',
    '.steam/registry.vdf', '.bashrc', '.ssh/known_hosts', '.mozilla/firefox/p/prefs.js', '.cache/thumbs/t.png',
]


def snapshot(home):
    out = {}
    for root, dirs, files in os.walk(home):
        for n in files:
            p = os.path.join(root, n)
            out[os.path.relpath(p, home)] = slurp(p)
    return out


def edition_move_plan(from_edition, to_edition, user):
    """The plan hearth-admin's move really writes (move_to_new_os, everything around the download mocked)."""
    tmp = tempfile.mkdtemp()
    plan = os.path.join(tmp, 'clean-install.json')
    with mock.patch.object(ha, 'caller', lambda: user), mock.patch.object(ha, 'CLEAN_INSTALL', plan), \
            mock.patch.object(ha.os.path, 'exists', lambda x: True if x == '/run/ostree-booted' else os.path.lexists(x)), \
            mock.patch.object(ha, 'own_edition', lambda: from_edition), mock.patch.object(ha, 'ensure_podman', lambda: None), \
            mock.patch.object(ha, 'drop_cached_images', lambda: 0), mock.patch.object(ha, 'pin_booted', lambda: None), \
            mock.patch.object(ha, 'read_deployments', lambda: []), mock.patch.object(ha, 'back_to_debian', lambda deps: False), \
            mock.patch.object(ha, 'layered', lambda: False), mock.patch.object(ha, 'esp_guard', lambda: None), \
            mock.patch.object(ha, 'bootc_upgrade_with_progress', lambda *a, **k: None), \
            mock.patch.object(ha, 'bootc_status', lambda quiet=False: {}), mock.patch('builtins.print'):
        ha.move_to_new_os(to_edition, False)
    with open(plan) as f:
        return json.load(f)


def first_start(root, plan):
    write(os.path.join(root, 'var/lib/hearth/clean-install.json'), json.dumps(plan))
    ci.ROOT = root
    try:
        return ci.main(['x'])
    finally:
        ci.ROOT = '/'


class EditionMoveKeepsHome(unittest.TestCase):
    USER = types.SimpleNamespace(pw_name='chris', pw_uid=1000, pw_dir='/home/chris', pw_gid=1000)

    def setUp(self):
        self.root = tempfile.mkdtemp()
        self.home = os.path.join(self.root, 'home/chris')
        for f in OWNER_HOME:
            write(os.path.join(self.home, f), 'mine:' + f)
        write(os.path.join(self.root, 'etc/hostname'), 'chris-hearth')
        write(os.path.join(self.root, 'usr/etc/hostname'), 'localhost')
        self.before = snapshot(self.home)

    def assert_untouched(self):
        self.assertEqual(snapshot(self.home), self.before, 'every file, hidden settings folders included, is where it was')
        self.assertEqual([n for n in os.listdir(self.home) if n.startswith('.old-hearth-')], [], 'nothing was put aside')

    def test_fedora_to_debian_keeps_every_hidden_settings_folder(self):
        # Fedora → Debian (Switch edition by hand, or auto-move-to-debian: the same move_to_new_os).
        plan = edition_move_plan('anatase', 'peppermint', self.USER)
        self.assertTrue(plan['keepHome'])
        self.assertEqual(plan['putAside'], [])
        write(os.path.join(self.root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'peppermint'}))
        self.assertEqual(first_start(self.root, dict(plan, home='/home/chris', user='chris')), 0)
        self.assert_untouched()
        for hidden in ('.config/nimbus', '.local/share/nimbus/voice-studio/voices', '.claude', '.codex', '.gemini', '.var', '.steam', '.ssh'):
            self.assertTrue(os.path.isdir(os.path.join(self.home, hidden)), hidden)

    def test_debian_to_fedora_keeps_home(self):
        plan = edition_move_plan('peppermint', 'anatase', self.USER)
        write(os.path.join(self.root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'anatase'}))
        self.assertEqual(first_start(self.root, dict(plan, home='/home/chris', user='chris')), 0)
        self.assert_untouched()

    def test_any_plan_from_an_edition_keeps_home_even_without_keep_home(self):
        # A plan from an edition that says nothing about keepHome (or says false) still never touches the home folder.
        write(os.path.join(self.root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'peppermint'}))
        for extra in ({}, {'keepHome': False}, {'keepHome': False, 'putAside': ['Documents', '.config']}):
            plan = dict({'version': 1, 'user': 'chris', 'uid': os.getuid(), 'home': '/home/chris', 'edition': 'peppermint',
                         'fromEdition': 'anatase', 'created': len(str(extra))}, **extra)
            self.assertEqual(first_start(self.root, plan), 0)
            self.assert_untouched()

    def test_an_older_first_start_finds_no_home_folder_to_clean(self):
        # The plan names an account that's never there and a home folder that never exists, so even an older Hearth
        # OS's hearth-clean-install (one that doesn't know keepHome) stops at "no home folder here".
        plan = edition_move_plan('anatase', 'peppermint', self.USER)
        self.assertEqual((plan['user'], plan['home']), (ha.KEPT_HOME_USER, ha.KEPT_HOME_DIR))
        self.assertEqual(plan['account'], {'user': 'chris', 'uid': 1000, 'home': '/home/chris'})
        with self.assertRaises(KeyError):
            pwd.getpwnam(plan['user'])
        self.assertFalse(os.path.exists(plan['home']))
        # What the 1.13 reader's home step does with it: look the account up, else use the plan's home.
        try:
            pw = pwd.getpwnam(plan['user'])
        except KeyError:
            pw = None
        home = pw.pw_dir if pw else plan.get('home')
        self.assertFalse(home and os.path.isdir(home))
        # And this reader's own home step, made to run anyway, finds nothing to move either.
        write(os.path.join(self.root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'peppermint'}))
        ci.ROOT = self.root
        try:
            ci.clean_home(dict(plan, putAside=['Documents', '.config', '.local']), {'stamp': '2026-11-24'})
        finally:
            ci.ROOT = '/'
        self.assert_untouched()

    def test_auto_move_uses_the_same_move(self):
        src = slurp(NIMBUS / 'hearth-admin')
        body = src[src.index('def auto_move_to_debian'):src.index('def write_atomic_json')]
        self.assertIn("move_to_new_os('peppermint')", body)
        self.assertNotIn('clean_install_plan(', body)

    def test_bazzite_moves_put_aside_only_what_was_picked(self):
        # The old Bazzite-based Hearth's plan (no list of picks): nothing goes aside.
        write(os.path.join(self.root, 'usr/share/nimbus/image-info.json'), json.dumps({'base': 'peppermint'}))
        plan = {'version': 1, 'user': 'chris', 'uid': os.getuid(), 'home': '/home/chris', 'edition': 'peppermint', 'created': 1}
        self.assertEqual(first_start(self.root, plan), 0)
        self.assert_untouched()
        # With picks: only those, kept until deleted.
        plan = dict(plan, created=2, fromEdition='bazzite', putAside=['Downloads'])
        self.assertEqual(first_start(self.root, plan), 0)
        olds = [n for n in os.listdir(self.home) if n.startswith('.old-hearth-')]
        self.assertEqual(len(olds), 1)
        self.assertFalse(os.path.exists(os.path.join(self.home, 'Downloads')))
        self.assertTrue(os.path.isfile(os.path.join(self.home, olds[0], 'Downloads/d.zip')))
        man = json.loads(slurp(os.path.join(self.home, olds[0], ci.MANIFEST)))
        self.assertNotIn('deleteAfter', man)
        for f in OWNER_HOME:
            if not f.startswith('Downloads/'):
                self.assertTrue(os.path.isfile(os.path.join(self.home, f)), f)


class NeverDeletedByItself(unittest.TestCase):
    def test_hearth_core_never_deletes_by_itself(self):
        src = slurp(REPO / 'hosts' / 'hearth-core' / 'src' / 'newos.rs')
        status = src[src.index('pub fn old_files_status'):src.index('fn checked_dir')]
        self.assertNotIn('remove_dir_all', status)
        self.assertNotIn('expired', src[src.index('pub fn list_in'):src.index('fn checked_dir')])
        # The only deleting is "Delete now" (oldfiles.delete, pressed by the person).
        self.assertEqual(src.count('remove_dir_all'), 1)
        self.assertIn('pub fn delete(name: &str)', src)

    def test_clean_install_sets_no_date(self):
        src = slurp(NIMBUS / 'hearth-clean-install')
        self.assertNotIn('deleteAfter', src)
        self.assertNotIn('KEEP_DAYS', src)

    def test_nothing_else_deletes_old_folders(self):
        # No timer, service or helper anywhere deletes ~/.old-hearth-* (only hearth-old-files delete NAME --yes and
        # hearth-core's Delete now, both pressed by the person).
        pat = re.compile(r'(rm\s+-r|rmtree|remove_dir_all|unlink)[^\n]{0,120}old-hearth|old-hearth[^\n]{0,120}(rm\s+-r|rmtree|remove_dir_all)')
        hits = []
        for base in ('os', 'hosts/linux/desktop', 'hosts/hearth-core/src', 'hosts/hearth-ui', 'os-bases'):
            for root, dirs, files in os.walk(REPO / base):
                dirs[:] = [d for d in dirs if d not in ('target', 'node_modules', 'vendor', '.git')]
                for n in files:
                    p = os.path.join(root, n)
                    try:
                        if os.path.getsize(p) > 3_000_000:
                            continue
                        with open(p, errors='ignore') as f:
                            if pat.search(f.read()):
                                hits.append(os.path.relpath(p, REPO))
                    except OSError:
                        pass
        self.assertEqual(hits, [])

    def test_old_files_helper_needs_yes_and_ignores_old_dates(self):
        home = tempfile.mkdtemp()
        old = os.path.join(home, '.old-hearth-2026-11-24')
        write(os.path.join(old, 'Documents/a.txt'))
        write(os.path.join(old, '.hearth-old-files.json'), json.dumps({'created': 1, 'deleteAfter': 2, 'moved': ['Documents']}))
        env = dict(os.environ, HEARTH_HOME=home, XDG_STATE_HOME=os.path.join(home, '.local/state'))
        r = subprocess.run([sys.executable, str(OLD_FILES), 'status'], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        st = json.loads(r.stdout)
        self.assertEqual([(f['name'], f['back'], f['ask']) for f in st['folders']], [('.old-hearth-2026-11-24', 1, True)])
        self.assertTrue(os.path.isdir(old), 'long past its old date and still there')
        r = subprocess.run([sys.executable, str(OLD_FILES), 'delete', '.old-hearth-2026-11-24'], env=env, capture_output=True, text=True)
        self.assertNotEqual(r.returncode, 0)
        self.assertTrue(os.path.isdir(old), 'never deleted without --yes')
        for bad in ('..', '.old-hearth-x', '../.old-hearth-2026-11-24', 'Documents'):
            r = subprocess.run([sys.executable, str(OLD_FILES), 'delete', bad, '--yes'], env=env, capture_output=True, text=True)
            self.assertNotEqual(r.returncode, 0, bad)
        # Keep: stops asking; the folder stays.
        r = subprocess.run([sys.executable, str(OLD_FILES), 'keep', '.old-hearth-2026-11-24'], env=env, capture_output=True, text=True)
        self.assertEqual(json.loads(r.stdout)['folders'][0]['ask'], False)
        self.assertTrue(os.path.isdir(old))
        r = subprocess.run([sys.executable, str(OLD_FILES), 'delete', '.old-hearth-2026-11-24', '--yes'], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertFalse(os.path.exists(old), 'gone only when the person said so')

    def test_the_notice_never_deletes(self):
        q = slurp(SHELL / 'OldFilesWatch.qml')
        self.assertNotIn('"delete"', q.replace('k === "delete"', ''))
        self.assertIn('ow.settings.open("storage")', q)
        self.assertIn('OldFilesWatch.qml', slurp(SHELL / 'shell.qml'))
        rs = slurp(REPO / 'hosts/hearth-ui/apps/settings/src/cats/system_more.rs')
        # Settings' Delete now asks once more before it runs.
        i = rs.index('"delete".into()')
        self.assertIn('if asking', rs[rs.rindex('fn old_files', 0, i):i])

    def test_the_update_image_never_goes_back_before_1_18(self):
        wf = slurp(REPO / '.github/workflows/image-point.yml')
        self.assertIn('-lt 18', wf)


class BringBack(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp()
        self.old = os.path.join(self.home, '.old-hearth-2026-11-24')
        for f in OWNER_HOME:
            write(os.path.join(self.old, f), 'old:' + f)
        moved = sorted({f.split('/')[0] for f in OWNER_HOME})
        write(os.path.join(self.old, '.hearth-old-files.json'), json.dumps({'created': 1, 'moved': moved}))
        # What the new system has now: a fresh desktop.json, a new Documents file, Steam set up again.
        write(os.path.join(self.home, '.config/nimbus/desktop.json'), 'new')
        write(os.path.join(self.home, 'Documents/notes.txt'), 'new notes')
        write(os.path.join(self.home, 'Documents/today.txt'), 'today')
        write(os.path.join(self.home, '.local/share/Steam/config/loginusers.vdf'), 'new sign-in')
        self.outside = tempfile.mkdtemp()
        os.symlink(self.outside, os.path.join(self.home, 'Music'))
        self.env = dict(os.environ, HEARTH_HOME=self.home, XDG_STATE_HOME=os.path.join(self.home, '.local/state'))

    def run_it(self, *a):
        r = subprocess.run([sys.executable, str(OLD_FILES)] + list(a), env=self.env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return json.loads(r.stdout)

    def test_never_overwrites_and_links(self):
        old_before = snapshot(self.old)
        st = self.run_it('status')
        self.assertGreater(st['folders'][0]['back'], 0)
        r = self.run_it('bring-back', '.old-hearth-2026-11-24')
        self.assertEqual(r['failed'], [])
        # Never replaced: what's in the home folder now stays exactly as it is.
        self.assertEqual(slurp(os.path.join(self.home, '.config/nimbus/desktop.json')), 'new')
        self.assertEqual(slurp(os.path.join(self.home, 'Documents/notes.txt')), 'new notes')
        self.assertEqual(slurp(os.path.join(self.home, 'Documents/today.txt')), 'today')
        self.assertEqual(slurp(os.path.join(self.home, '.local/share/Steam/config/loginusers.vdf')), 'new sign-in')
        # What was missing is back, as hard links (no extra space).
        for f in ('.config/nimbus/settings.json', '.local/share/nimbus/voice-studio/voices/ithil.wav', '.claude/settings.json',
                  'Pictures/a.png', 'Saved Games/s.sav', 'Hearth Storage/s.bin', '.ssh/known_hosts'):
            self.assertEqual(slurp(os.path.join(self.home, f)), 'old:' + f, f)
            self.assertEqual(os.stat(os.path.join(self.home, f)).st_ino, os.stat(os.path.join(self.old, f)).st_ino, f)
        # Steam's own folders and caches stay out.
        for f in ('.steam/registry.vdf', '.cache/thumbs/t.png', '.local/share/Steam/steamapps/common/Hades/Hades.exe',
                  '.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/libraryfolders.vdf'):
            self.assertFalse(os.path.exists(os.path.join(self.home, f)), f)
        self.assertTrue(os.path.isfile(os.path.join(self.home, '.var/app/org.mozilla.firefox/x')))
        # A link in the home folder that points elsewhere now: nothing written through it.
        self.assertEqual(os.listdir(self.outside), [])
        # The old folder is exactly as it was (only its own record of what came back added).
        after = snapshot(self.old)
        after.pop('.hearth-brought-back.json')
        self.assertEqual(after, old_before)
        # Again: nothing more to do, nothing replaced.
        r2 = self.run_it('bring-back', '.old-hearth-2026-11-24')
        self.assertEqual((r2['linked'], r2['copied']), (0, 0))
        self.assertEqual(self.run_it('status')['folders'][0]['ask'], False)


if __name__ == '__main__':
    unittest.main()
