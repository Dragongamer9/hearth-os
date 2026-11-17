"""Tests for Hearth Files' 1.14 extras (desktop/shell/hearth_files_extras.py through hearth-files, and
os/system_files/usr/libexec/nimbus/hearth-files-admin), in a throwaway home folder: tags (user.xdg.tags) and finding by
tag, git status badges, disc images (a stand-in udisksctl), phones and cameras (gio's listing), apps' right-click
actions (KDE service menus), sharing a folder (a stand-in Home server helper), recent files' privacy, permissions for a
whole folder, and Act as administrator's checks (never what starts the PC)."""
import sys
sys.dont_write_bytecode = True
import json
import os
import pathlib
import shutil
import stat
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
HF = SHELL / 'hearth-files'
ADMIN = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-files-admin'
sys.path.insert(0, str(SHELL))
import hearth_files_extras as hx  # noqa: E402


def xattrs_work(d):
    p = os.path.join(d, '.probe')
    open(p, 'w').close()
    try:
        os.setxattr(p, 'user.hearth', b'1')
        return True
    except OSError:
        return False
    finally:
        os.unlink(p)


class Extras(unittest.TestCase):
    def setUp(self):
        self.tmp = os.path.realpath(tempfile.mkdtemp(prefix='hearth-files-extras-'))
        self.home = os.path.join(self.tmp, 'home')
        self.bin = os.path.join(self.tmp, 'bin')
        for d in (self.home, self.bin, os.path.join(self.home, 'Documents')):
            os.makedirs(d, exist_ok=True)
        self.env = dict(os.environ, HOME=self.home, PATH=self.bin + ':/usr/bin:/bin', PYTHONDONTWRITEBYTECODE='1',
                        XDG_CONFIG_HOME=os.path.join(self.home, '.config'), XDG_DATA_HOME=os.path.join(self.home, '.local', 'share'),
                        XDG_CACHE_HOME=os.path.join(self.home, '.cache'), XDG_DATA_DIRS=os.path.join(self.tmp, 'sys-share') + ':/usr/share',
                        XDG_RUNTIME_DIR=os.path.join(self.tmp, 'run'), HEARTH_FILES_NO_DISCOVERY='1')
        for k in ('WAYLAND_DISPLAY', 'DISPLAY'):
            self.env.pop(k, None)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def hf(self, *args, ok=True):
        p = subprocess.run([sys.executable, str(HF)] + list(args), capture_output=True, text=True, timeout=60, env=self.env)
        lines = [l for l in p.stdout.strip().splitlines() if l.strip()]
        last = lines[-1] if lines else ''
        if ok:
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            return json.loads(last)
        self.assertNotEqual(p.returncode, 0, p.stdout)
        self.assertTrue(last.startswith('error: '), last)
        return last[7:]

    def fake(self, name, text):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + text)
        os.chmod(p, 0o755)

    # ---- tags
    def test_tags_set_list_find_and_in_listings(self):
        if not xattrs_work(self.home):
            self.skipTest('this drive keeps no extended attributes')
        a = os.path.join(self.home, 'Documents', 'taxes.pdf')
        b = os.path.join(self.home, 'Documents', 'notes.txt')
        for p in (a, b):
            open(p, 'w').close()
        r = self.hf('tag-set', '--add', 'Work', '--add', 'Taxes', a, b)
        self.assertEqual(r['tags'][a], ['Work', 'Taxes'])
        # Dolphin's own format: the same attribute, comma separated.
        self.assertEqual(os.getxattr(a, 'user.xdg.tags'), b'Work,Taxes')
        self.hf('tag-set', '--remove', 'Taxes', b)
        self.assertEqual(self.hf('tags', a, b)['tags'], {a: ['Work', 'Taxes'], b: ['Work']})
        ls = self.hf('list', os.path.join(self.home, 'Documents'))
        self.assertEqual(next(e for e in ls['entries'] if e['name'] == 'taxes.pdf')['tags'], ['Work', 'Taxes'])
        self.assertNotIn('tags', next(e for e in ls['entries'] if e['name'] == 'notes.txt') if False else {})
        self.assertEqual({t['name']: t['count'] for t in self.hf('tag-list')['tags']}, {'Work': 2, 'Taxes': 1})
        p = subprocess.run([sys.executable, str(HF), 'tag-find', 'Taxes'], capture_output=True, text=True, timeout=60, env=self.env)
        found = [json.loads(l[7:]) for l in p.stdout.splitlines() if l.startswith('found: ')]
        self.assertEqual([f['path'] for f in found], [a])
        self.assertEqual(found[0]['where'], 'Documents')
        self.assertIn('commas', self.hf('tag-set', '--add', 'a,b', a, ok=False))
        # Outside where you may change files: refused.
        self.hf('tag-set', '--add', 'X', '/etc/hostname', ok=False)
        self.hf('tag-set', '--remove', 'Work', a)
        self.hf('tag-set', '--remove', 'Taxes', a)
        with self.assertRaises(OSError):
            os.getxattr(a, 'user.xdg.tags')

    # ---- git
    def test_git_status_badges(self):
        if not shutil.which('git'):
            self.skipTest('git missing')
        repo = os.path.join(self.home, 'code')
        os.makedirs(os.path.join(repo, 'src'))
        g = lambda *a: subprocess.run(['git', '-C', repo] + list(a), capture_output=True, text=True, check=True,
                                      env=dict(self.env, GIT_AUTHOR_NAME='t', GIT_AUTHOR_EMAIL='t@t', GIT_COMMITTER_NAME='t', GIT_COMMITTER_EMAIL='t@t'))
        g('init', '-q', '-b', 'main')
        for n, t in (('README.md', 'a'), ('src/main.rs', 'fn main(){}'), ('.gitignore', 'build/\n')):
            with open(os.path.join(repo, n), 'w') as f:
                f.write(t)
        g('add', '.')
        g('commit', '-q', '-m', 'first')
        with open(os.path.join(repo, 'README.md'), 'a') as f:
            f.write('changed')
        open(os.path.join(repo, 'new.txt'), 'w').close()
        os.makedirs(os.path.join(repo, 'build'))
        open(os.path.join(repo, 'build', 'out.o'), 'w').close()
        r = self.hf('git-status', repo)
        self.assertEqual(r['branch'], 'main')
        st = r['status']
        self.assertEqual(st['README.md'], 'modified')
        self.assertEqual(st['new.txt'], 'untracked')
        self.assertEqual(st['src'], 'clean')
        self.assertEqual(st['build'], 'ignored')
        # Inside a folder of the repo: its own items.
        with open(os.path.join(repo, 'src', 'main.rs'), 'a') as f:
            f.write('//')
        self.assertEqual(self.hf('git-status', os.path.join(repo, 'src'))['status'], {'main.rs': 'modified'})
        self.assertEqual(self.hf('git-status', repo)['status']['src'], 'modified', 'a folder shows the worst inside it')
        self.assertIsNone(self.hf('git-status', os.path.join(self.home, 'Documents'))['repo'])

    def test_porcelain_parsing(self):
        raw = ' M a.txt\0?? b.txt\0R  new.txt\0old.txt\0UU c.txt\0!! d/\0A  e.txt\0'
        self.assertEqual(hx.parse_porcelain(raw), [('modified', 'a.txt'), ('untracked', 'b.txt'), ('modified', 'new.txt'),
                                                   ('conflict', 'c.txt'), ('ignored', 'd/'), ('added', 'e.txt')])

    # ---- disc images
    def test_iso_mount_and_close_through_udisks(self):
        iso = os.path.join(self.home, 'Debian.iso')
        open(iso, 'wb').write(b'\0' * 4096)
        log = os.path.join(self.tmp, 'udisks.log')
        mnt = os.path.join(self.tmp, 'media', 'DEBIAN')
        self.fake('udisksctl', 'echo "$*" >> %s\ncase "$1" in\n loop-setup) echo "Mapped file $4 as /dev/loop7." ;;\n mount) mkdir -p %s; echo "Mounted /dev/loop7 at %s" ;;\nesac\n' % (log, mnt, mnt))
        self.fake('losetup', 'exit 0\n')
        r = self.hf('iso-mount', iso)
        self.assertEqual((r['path'], r['device']), (mnt, '/dev/loop7'))
        said = open(log).read()
        self.assertIn('loop-setup -r -f ' + iso, said, 'read only')
        self.assertIn('--no-user-interaction', said)
        self.assertIn('Only disc images', self.hf('iso-mount', os.path.join(self.home, 'Documents'), ok=False))
        self.assertEqual(self.hf('iso-unmount', iso), {'ok': True, 'closed': False})

    # ---- phones and cameras
    def test_phones_and_cameras_from_gio(self):
        out = '''Drive(0): Samsung SSD
Volume(0): Pixel 8
  Type: GProxyVolume (GProxyVolumeMonitorMTP)
  activation_root=mtp://Google_Pixel_8_3A2B/
Volume(1): Canon EOS R10
  Type: GProxyVolume (GProxyVolumeMonitorGPhoto2)
  activation_root=gphoto2://Canon_Inc._Canon_Digital_Camera/
Mount(0): Pixel 8 -> mtp://Google_Pixel_8_3A2B/
  Type: GProxyShadowMount
Volume(2): Data
  activation_root=file:///run/media/me/Data
'''
        devs = hx.parse_gio_mounts(out)
        self.assertEqual([(d['label'], d['kind'], d['mounted']) for d in devs], [('Pixel 8', 'phone', True), ('Canon EOS R10', 'camera', False)])
        gv = os.path.join(self.tmp, 'run', 'gvfs', 'mtp:host=Google_Pixel_8_3A2B')
        os.makedirs(gv)
        self.fake('gio', 'case "$*" in\n "mount -li") cat <<EOF\n%sEOF\n ;;\n *) echo "$*" >> %s/gio.log ;;\nesac\n' % (out, self.tmp))
        r = self.hf('portable')
        self.assertEqual(r['devices'][0]['path'], gv)
        self.assertIsNone(r['devices'][1]['path'])
        self.assertEqual(self.hf('portable-mount', 'mtp://Google_Pixel_8_3A2B/')['path'], gv)
        self.assertIn("isn't a phone", self.hf('portable-mount', 'smb://server/share', ok=False))

    # ---- service menus
    def test_service_menus_kde_style(self):
        user = os.path.join(self.home, '.local', 'share', 'kio', 'servicemenus')
        sysd = os.path.join(self.tmp, 'sys-share', 'kio', 'servicemenus')
        for d in (user, sysd):
            os.makedirs(d)
        mark = os.path.join(self.tmp, 'ran')
        with open(os.path.join(sysd, 'images.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Service\nMimeType=image/*;\nActions=small;both\nX-KDE-Submenu=Pictures\n\n'
                    '[Desktop Action small]\nName=Make smaller\nName[de]=Kleiner\nIcon=image\nExec=sh -c \'echo "$0" >> %s\' %%f\n\n'
                    '[Desktop Action both]\nName=Put together\nExec=sh -c \'echo "$*" >> %s\' x %%F\n' % (mark, mark))
        with open(os.path.join(user, 'mine.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Service\nMimeType=all/allfiles;\nActions=a\n[Desktop Action a]\nName=Mine\nExec=true %f\n')
        pics = [os.path.join(self.home, n) for n in ('a.png', 'b.png')]
        for p in pics:
            open(p, 'wb').write(b'\x89PNG\r\n\x1a\n')
        acts = [a for a in self.hf('service-menus', *pics)['actions'] if a['id'].startswith(('images.desktop', 'mine.desktop'))]
        self.assertEqual([(a['label'], a['submenu']) for a in acts], [('Make smaller', 'Pictures'), ('Put together', 'Pictures')])
        # One you added yourself counts once it's marked as a program (as KDE does).
        os.chmod(os.path.join(user, 'mine.desktop'), 0o755)
        self.assertIn('Mine', [a['label'] for a in self.hf('service-menus', *pics)['actions']])
        self.assertFalse([a for a in self.hf('service-menus', os.path.join(self.home, 'Documents'))['actions'] if a['id'].startswith(('images', 'mine'))], 'not for folders')
        self.hf('service-run', 'images.desktop#small', *pics)
        self.hf('service-run', 'images.desktop#both', *pics)
        deadline = time.time() + 10
        while time.time() < deadline and (not os.path.exists(mark) or len(open(mark).read().splitlines()) < 3):
            time.sleep(0.1)
        lines = open(mark).read().splitlines()
        self.assertEqual(sorted(lines[:3]), sorted([pics[0], pics[1], ' '.join(pics)]), '%f once per file, %F all at once')
        self.assertIn("isn't there", self.hf('service-run', '../../etc.desktop#x', pics[0], ok=False))

    # ---- sharing a folder
    def test_share_this_folder_through_home_server(self):
        state = os.path.join(self.tmp, 'shares.json')
        json.dump([], open(state, 'w'))
        fake = os.path.join(self.bin, 'hearth-home-server')
        with open(fake, 'w') as f:
            f.write('''import json, sys, os
st = %r
shares = json.load(open(st))
a = sys.argv[1:]
if a == ['status']:
    for s in shares: s['how'] = {'windows': '\\\\\\\\pc\\\\' + s['name'], 'mac': 'smb://pc/' + s['name']}
    print(json.dumps({'shares': shares, 'tools': {'samba': True}}))
elif a[0] == 'share-add':
    shares.append({'id': 'abc123', 'name': os.path.basename(a[1]), 'path': a[1], 'write': False}); json.dump(shares, open(st, 'w')); print('{}')
elif a[0] == 'share-set':
    shares[0]['write'] = a[3] == 'on'; json.dump(shares, open(st, 'w')); print('{}')
elif a[0] == 'share-remove':
    json.dump([], open(st, 'w')); print('{}')
''' % state)
        self.env['HEARTH_HOME_SERVER_BIN'] = fake
        docs = os.path.join(self.home, 'Documents')
        self.assertFalse(self.hf('sharing', docs)['shared'])
        r = self.hf('share', docs, 'on')
        self.assertTrue(r['shared'])
        self.assertEqual(r['share']['how']['mac'], 'smb://pc/Documents')
        self.assertTrue(self.hf('share', docs, 'write-on')['share']['write'])
        self.assertFalse(self.hf('share', docs, 'off')['shared'])
        self.assertIn('Share it first', self.hf('share', docs, 'write-on', ok=False))

    # ---- recent files
    def xbel(self, items):
        d = os.path.join(self.home, '.local', 'share')
        os.makedirs(d, exist_ok=True)
        body = ''
        for p, when, app in items:
            body += ('  <bookmark href="file://%s" added="%s" modified="%s" visited="%s">\n    <info><metadata owner="http://freedesktop.org">'
                     '<bookmark:applications><bookmark:application name="%s" exec="&apos;%s %%u&apos;" modified="%s" count="1"/>'
                     '</bookmark:applications></metadata></info>\n  </bookmark>\n') % (p, when, when, when, app, app.lower(), when)
        with open(os.path.join(d, 'recently-used.xbel'), 'w') as f:
            f.write('<?xml version="1.0" encoding="UTF-8"?>\n<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks">\n'
                    + body + '</xbel>\n')

    def test_recent_files_privacy(self):
        now = time.time()
        iso = lambda t: time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(t))
        files = {}
        for n in ('old.txt', 'today.txt', 'secret.txt', 'game.txt'):
            files[n] = os.path.join(self.home, 'Documents', n)
            open(files[n], 'w').close()
        os.makedirs(os.path.join(self.home, 'Private'))
        files['secret.txt'] = os.path.join(self.home, 'Private', 'secret.txt')
        open(files['secret.txt'], 'w').close()
        self.xbel([(files['old.txt'], iso(now - 40 * 86400), 'Text'), (files['today.txt'], iso(now - 600), 'Text'),
                   (files['secret.txt'], iso(now - 300), 'Text'), (files['game.txt'], iso(now - 200), 'Steam')])
        names = lambda: [e['name'] for e in self.hf('recent')['recent']]
        self.assertEqual(names(), ['game.txt', 'secret.txt', 'today.txt', 'old.txt'])
        r = self.hf('recent-prefs')
        self.assertEqual(r['prefs'], {'remember': True, 'keepDays': 0, 'leaveOut': []})
        self.assertEqual(sorted(r['apps']), ['Steam', 'Text'])
        self.assertFalse(os.path.exists(os.path.join(self.home, '.config', 'nimbus', 'recent.json')), 'reading changes nothing')
        r = self.hf('recent-prefs-set', json.dumps({'keepDays': 30, 'leaveOut': [os.path.join(self.home, 'Private'), 'Steam'], 'evil': 1}))
        self.assertEqual(r['forgot'], 3)
        self.assertEqual(names(), ['today.txt'])
        saved = json.load(open(os.path.join(self.home, '.config', 'nimbus', 'recent.json')))
        self.assertNotIn('evil', saved)
        self.assertTrue(os.path.exists(os.path.join(self.home, '.local', 'share', 'recently-used.xbel.bak')))
        # The launcher's list (hearth-appmenu) follows the same choice.
        p = subprocess.run([sys.executable, str(SHELL / 'hearth-appmenu'), 'recent'], capture_output=True, text=True, timeout=30, env=self.env)
        self.assertEqual([x['name'] for x in json.loads(p.stdout.strip().splitlines()[-1])], ['today.txt'])
        self.assertEqual(self.hf('recent-forget', 'hour')['forgot'], 1)
        self.assertEqual(names(), [])
        self.hf('recent-prefs-set', json.dumps({'remember': False}))
        self.xbel([(files['today.txt'], iso(now - 60), 'Text')])
        self.assertEqual(names(), [], 'off: none shown')
        self.assertIn('Nothing to change', self.hf('recent-prefs-set', json.dumps({'keepDays': 3}), ok=False))

    def test_xbel_both_shapes(self):
        text = ('<xbel><bookmark href="file:///a%20b.txt" visited="2026-11-01T10:00:00Z"/>'
                '<bookmark href="file:///c.txt" visited="2026-11-02T10:00:00Z"><info><bookmark:application name="Kate"/></info></bookmark></xbel>')
        items = hx.xbel_items(text)
        self.assertEqual([(p, apps) for _, _, p, apps in items], [('/a b.txt', []), ('/c.txt', ['Kate'])])

    # ---- permissions
    def test_chmod_tree_and_groups(self):
        d = os.path.join(self.home, 'proj')
        os.makedirs(os.path.join(d, 'sub'))
        for n, m in (('run.sh', 0o755), ('doc.txt', 0o644), ('sub/x.txt', 0o600)):
            open(os.path.join(d, n), 'w').close()
            os.chmod(os.path.join(d, n), m)
        self.hf('chmod-tree', d, '750')
        m = lambda n: stat.S_IMODE(os.stat(os.path.join(d, n)).st_mode)
        self.assertEqual((m(''), m('sub'), m('run.sh'), m('doc.txt'), m('sub/x.txt')), (0o750, 0o750, 0o750, 0o640, 0o640))
        self.assertIsInstance(self.hf('groups')['groups'], list)
        self.assertIn("isn't a permission", self.hf('chmod-tree', d, '9z', ok=False))


class AdminChecks(unittest.TestCase):
    """hearth-files-admin, run as you in a throwaway folder (pkexec runs it as root on the PC): every path is checked
    and the parts that start the PC are never changed."""

    def setUp(self):
        self.tmp = os.path.realpath(tempfile.mkdtemp(prefix='hearth-files-admin-'))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def admin(self, *args, ok=True):
        p = subprocess.run([sys.executable, str(ADMIN)] + list(args), capture_output=True, text=True, timeout=30,
                           env=dict({k: v for k, v in os.environ.items()}, PKEXEC_UID=str(os.getuid())))
        last = (p.stdout.strip().splitlines() or [''])[-1]
        if ok:
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            return json.loads(last)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertTrue(last.startswith('error: '), last)
        return last[7:]

    def test_never_what_starts_the_pc(self):
        for p in ('/boot/grub/grub.cfg', '/boot', '/efi/EFI', '/etc/fstab', '/etc/kernel/cmdline', '/etc/default/grub', '/usr/bin/bash',
                  '/sysroot/ostree', '/proc/1', '/etc/sudoers', '/etc/pam.d/login', '/etc/shadow'):
            why = self.admin('files-delete', p, ok=False)
            self.assertTrue('starts this PC' in why or 'top-level' in why.lower(), p + ': ' + why)
        self.assertIn('top-level', self.admin('files-rename', '/etc', 'etc2', ok=False).lower())
        self.assertIn('plainly', self.admin('files-delete', self.tmp + '/../x', ok=False))
        self.assertIn('full path', self.admin('files-mkdir', 'relative', ok=False))

    def test_links_and_changes(self):
        d = os.path.join(self.tmp, 'cfg')
        self.assertEqual(self.admin('files-mkdir', d)['path'], d)
        f = self.admin('files-new-file', d + '/app.conf')['path']
        self.assertEqual(self.admin('files-rename', f, 'main.conf')['path'], d + '/main.conf')
        self.admin('files-chmod', d + '/main.conf', '600')
        self.assertEqual(stat.S_IMODE(os.stat(d + '/main.conf').st_mode), 0o600)
        out = os.path.join(self.tmp, 'out')
        os.mkdir(out)
        r = self.admin('files-copy', d + '/main.conf', out)
        self.assertEqual(r['items'], [out + '/main.conf'])
        r = self.admin('files-copy', d + '/main.conf', out)
        self.assertEqual(r['items'], [out + '/main (2).conf'], 'a name already there: both kept')
        self.admin('files-move', out + '/main (2).conf', d)
        self.assertTrue(os.path.exists(d + '/main (2).conf'))
        os.symlink(d, os.path.join(self.tmp, 'link'))
        self.assertIn('link', self.admin('files-delete', os.path.join(self.tmp, 'link', 'main.conf'), ok=False))
        self.assertEqual(self.admin('files-delete', d)['deleted'], [d])
        self.assertFalse(os.path.exists(d))

    def test_the_polkit_rules(self):
        """Its own action keeps the password a few minutes for this program only; nothing shared keeps it, and making
        an account an administrator always asks."""
        pol = (ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'actions' / 'fun.hearthos.files-admin.policy').read_text()
        self.assertIn('<annotate key="org.freedesktop.policykit.exec.path">/usr/libexec/nimbus/hearth-files-admin</annotate>', pol)
        self.assertIn('<allow_active>auth_admin_keep</allow_active>', pol)
        self.assertIn('<allow_any>auth_admin</allow_any>', pol)
        rules_dir = ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'rules.d'
        for f in rules_dir.iterdir():
            self.assertNotIn('AUTH_ADMIN_KEEP', f.read_text(), f.name + ' keeps a password for a shared action')
        rules = (rules_dir / '49-hearth-admin.rules').read_text()
        yes = rules.index('polkit.Result.YES')
        i = rules.index('files-\\S+')
        self.assertIn('Result.NO', rules[i:i + 200])
        j = rules.index('user-type(')
        self.assertIn('Result.AUTH_ADMIN;', rules[j:j + 200])
        self.assertLess(max(i, j), yes, 'decided before the rule that lets Settings through without a password')
        src = (ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-admin').read_text()
        self.assertNotIn("'files-chown'", src, 'hearth-admin no longer runs Files changes')
        for q in ('FilesExtras.qml', 'FileProperties.qml'):
            self.assertIn('"/usr/libexec/nimbus/hearth-files-admin"', (SHELL / q).read_text())


class AdminFakeRoot(unittest.TestCase):
    """The security review's cases, against hearth-files-admin itself with its refused list moved into a throwaway
    "root": nothing it refuses can be replaced, made, renamed onto, changed or reached through a link."""

    def setUp(self):
        import importlib.machinery
        import importlib.util
        self.r = os.path.realpath(tempfile.mkdtemp(prefix='hearth-fake-root-'))
        self.addCleanup(shutil.rmtree, self.r, True)
        self.addCleanup(os.environ.pop, 'PKEXEC_UID', None)
        os.environ['PKEXEC_UID'] = str(os.getuid())        # as pkexec runs it (the helper refuses root without it)
        loader = importlib.machinery.SourceFileLoader('hearth_files_admin', str(ADMIN))
        spec = importlib.util.spec_from_loader('hearth_files_admin', loader)
        self.m = importlib.util.module_from_spec(spec)
        loader.exec_module(self.m)
        R = self.r
        self.m.REFUSED = tuple(R + x for x in ('/boot', '/etc/fstab', '/etc/pam.d', '/etc/sudoers', '/etc/sudoers.d', '/etc/default/grub',
                                                '/etc/shadow', '/etc/polkit-1', '/usr'))
        self.m.ALLOWED_UNDER_REFUSED = (R + '/usr/local',)
        for d in ('/etc/pam.d', '/etc/default', '/boot', '/opt', '/var/data', '/usr/local', '/src/pam.d', '/src/sudoers.d', '/src/grub'):
            os.makedirs(R + d, exist_ok=True)
        for f, t in (('/etc/pam.d/login', 'auth required pam_unix.so'), ('/etc/fstab', 'UUID=x / ext4'), ('/boot/vmlinuz', 'kernel'),
                     ('/src/fstab', 'evil'), ('/src/pam.d/login', 'auth sufficient pam_permit.so'), ('/src/grub', 'evil'),
                     ('/src/sudoers.d/x', 'ALL ALL=(ALL) NOPASSWD: ALL'), ('/etc/app.conf', 'ok')):
            if os.path.isdir(R + f):
                continue
            with open(R + f, 'w') as fh:
                fh.write(t)
        os.rmdir(R + '/src/grub') if os.path.isdir(R + '/src/grub') else None
        with open(R + '/src/grub', 'w') as fh:
            fh.write('evil')

    def run_m(self, *args):
        import contextlib
        import io
        with contextlib.redirect_stdout(io.StringIO()):
            return self.m.main(['hearth-files-admin'] + list(args))

    def refused(self, *args):
        with self.assertRaises(self.m.Fail) as c:
            self.run_m(*args)
        return str(c.exception)

    def intact(self):
        R = self.r
        self.assertEqual(open(R + '/etc/pam.d/login').read(), 'auth required pam_unix.so')
        self.assertEqual(open(R + '/etc/fstab').read(), 'UUID=x / ext4')
        self.assertEqual(open(R + '/boot/vmlinuz').read(), 'kernel')
        self.assertFalse(os.path.exists(R + '/etc/sudoers.d'))
        self.assertFalse(os.path.exists(R + '/etc/default/grub'))

    def test_copy_or_move_onto_refused_names(self):
        R = self.r
        for verb in ('files-copy', 'files-move'):
            for how in ('replace', 'rename', 'skip'):
                self.refused(verb, R + '/src/pam.d', R + '/etc', '--default', how) if how == 'replace' else None
            self.assertIn('keeps it safe', self.refused(verb, R + '/src/pam.d', R + '/etc', '--default', 'replace'))
            self.refused(verb, R + '/src/pam.d', R + '/etc', '--decide', json.dumps({'pam.d': 'replace'}))
            self.refused(verb, R + '/src/fstab', R + '/etc', '--default', 'replace')
            self.refused(verb, R + '/src/sudoers.d', R + '/etc')          # not there yet: still refused
            self.refused(verb, R + '/src/grub', R + '/etc/default')
            self.intact()
        # "Keep both" makes a new name, which is fine; nothing refused was touched.
        r = self.run_m('files-copy', R + '/src/fstab', R + '/etc', '--default', 'rename')
        self.assertEqual(r['items'], [R + '/etc/fstab (2)'])
        self.intact()

    def test_symlinked_destination_and_parents(self):
        R = self.r
        os.symlink(R + '/boot', R + '/opt/x')                   # a destination that is a link to a refused place
        self.refused('files-copy', R + '/src/fstab', R + '/opt/x')
        os.symlink(R + '/var/data', R + '/opt/plain')             # a link to an allowed place is refused too
        self.assertIn('link', self.refused('files-copy', R + '/src/fstab', R + '/opt/plain'))
        os.symlink(R + '/etc', R + '/opt/etc')                  # a link in the parents
        self.refused('files-mkdir', R + '/opt/etc/pam.d/new')
        self.refused('files-new-file', R + '/opt/etc/new.conf')
        self.refused('files-delete', R + '/opt/etc/app.conf')
        self.refused('files-chmod', R + '/opt/etc/app.conf', '777')
        os.symlink(R + '/etc/fstab', R + '/var/data/fs')          # a link that leads into a refused file
        self.refused('files-chmod', R + '/var/data/fs', '666')
        self.refused('files-delete', R + '/var/data/fs')
        self.intact()

    def test_more_refused_places_and_folders_anyone_can_change(self):
        src = ADMIN.read_text()
        for x in ('/var/lib', '/etc/security', '/etc/login.defs', '/etc/nsswitch.conf', '/etc/systemd', '/etc/NetworkManager', '/etc/ssh'):
            self.assertIn("'%s'" % x, src)
        R = self.r
        os.makedirs(R + '/shared')
        os.chmod(R + '/shared', 0o1777)                         # like /tmp
        with open(R + '/shared/f', 'w') as fh:
            fh.write('x')
        for args in (('files-new-file', R + '/shared/new'), ('files-delete', R + '/shared/f'), ('files-chmod', R + '/shared/f', '600'),
                     ('files-copy', R + '/src/fstab', R + '/shared')):
            self.assertIn('anyone can change', self.refused(*args))
        self.assertTrue(os.path.exists(R + '/shared/f'))

    def test_portal_cast_only_in_checks(self):
        env = {k: v for k, v in os.environ.items() if k != 'HEARTH_CHECKS'}
        for args in (('outputs',), ('shot', os.path.join(self.r, 'x.png')), ('stream', 'output', 'X', '1')):
            p = subprocess.run([sys.executable, str(SHELL / 'hearth-portal'), 'cast'] + list(args), capture_output=True, text=True, timeout=30, env=env)
            self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
            self.assertIn("only for Hearth's checks", p.stdout)
        self.assertFalse(os.path.exists(os.path.join(self.r, 'x.png')))
        self.assertIn("os.environ['HEARTH_CHECKS'] = '1'", (ROOT / 'hosts' / 'linux' / 'test' / 'kwin_e2e.py').read_text())

    def test_open_dir_never_follows_a_swapped_link(self):
        R = self.r
        fd = self.m.open_dir(R + '/var/data')
        os.close(fd)
        os.symlink(R + '/boot', R + '/var/swap')
        with self.assertRaises(self.m.Fail):
            self.m.open_dir(R + '/var/swap')

    def test_every_operation_refuses_refused_paths(self):
        R = self.r
        self.refused('files-rename', R + '/etc/app.conf', 'fstab')          # rename onto a refused name
        self.refused('files-rename', R + '/etc/app.conf', 'sudoers.d')
        self.refused('files-rename', R + '/etc/pam.d', 'pam.old')           # rename a refused one away
        self.refused('files-mkdir', R + '/etc/sudoers.d')
        self.refused('files-mkdir', R + '/usr/lib/x')
        self.refused('files-new-file', R + '/etc/default/grub')
        self.refused('files-delete', R + '/etc/app.conf', R + '/etc/pam.d/login')   # one refused: none deleted
        self.assertTrue(os.path.exists(R + '/etc/app.conf'))
        self.refused('files-chmod', R + '/boot/vmlinuz', '777')
        self.refused('files-chown', R + '/etc/fstab', 'root', 'root')
        self.refused('files-move', R + '/etc/pam.d', R + '/var/data')
        self.intact()
        # What isn't refused still works, through the folder fds.
        self.assertEqual(self.run_m('files-mkdir', R + '/usr/local/tool')['path'], R + '/usr/local/tool')
        self.run_m('files-chmod', R + '/etc/app.conf', '640')
        self.assertEqual(stat.S_IMODE(os.stat(R + '/etc/app.conf').st_mode), 0o640)
        self.assertEqual(self.run_m('files-copy', R + '/src/pam.d', R + '/var/data')['items'], [R + '/var/data/pam.d'])
        self.assertEqual(open(R + '/var/data/pam.d/login').read(), 'auth sufficient pam_permit.so')
        self.run_m('files-move', R + '/var/data/pam.d', R + '/opt')
        self.assertTrue(os.path.isdir(R + '/opt/pam.d') and not os.path.exists(R + '/var/data/pam.d'))
        self.run_m('files-delete', R + '/opt/pam.d')
        self.assertFalse(os.path.exists(R + '/opt/pam.d'))

if __name__ == '__main__':
    unittest.main(verbosity=2)
