"""The Briefcase in Hearth Files (hosts/linux/desktop/shell/hearth-briefcase, Briefcase.qml; docs/HEARTH-FILES.md).

Statuses (unplugged versus deleted), planning and applying every action, conflicts keeping both, the Trash (home, a
USB stick's .Trash-UID, .briefcase-replaced on a share), copies surviving a crash mid-copy, folders, a bad list kept
aside, carrying a Briefcase, swapping between Briefcases, customizing, locking (with a stand-in gocryptfs where FUSE
can't mount; the real one where it can), auto-lock and auto-sync.
Run: python3 -m unittest discover -s hosts/linux/test -p 'test_hearth_briefcase.py' -v
"""
import importlib.machinery
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..'))
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')
HELPER = os.path.join(SHELL, 'hearth-briefcase')
FAKE = os.path.join(HERE, 'fake_gocryptfs.py')


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_briefcase', HELPER)
    spec = importlib.util.spec_from_loader('hearth_briefcase', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


bc = load()


def write(p, text, mtime=None):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(text)
    if mtime is not None:
        os.utime(p, (mtime, mtime))


def read(p):
    with open(p) as f:
        return f.read()


def age(p, seconds=100):
    t = time.time() - seconds
    os.utime(p, (t, t))


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='bc-test-')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.run_dir = os.path.join(self.tmp, 'run')
        os.makedirs(self.run_dir, mode=0o700)
        bindir = os.path.join(self.tmp, 'bin')
        os.makedirs(bindir)
        os.symlink(FAKE, os.path.join(bindir, 'gocryptfs'))
        os.symlink(FAKE, os.path.join(bindir, 'fake_fusermount'))
        self.argv_log = os.path.join(self.tmp, 'argv.log')
        self.env = {
            'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir,
            'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state'),
            'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
            'HEARTH_BRIEFCASE_DEVICE': 'desk', 'HEARTH_BRIEFCASE_MACHINE': 'm-desk',
            'HEARTH_BRIEFCASE_DRIVES': '{}', 'HEARTH_BRIEFCASE_FAKE_MOUNTS': '1',
            'HEARTH_BRIEFCASE_GOCRYPTFS': os.path.join(bindir, 'gocryptfs'),
            'HEARTH_BRIEFCASE_FUSERMOUNT': os.path.join(bindir, 'fake_fusermount'),
            'HEARTH_BRIEFCASE_NO_GUARD': '1', 'HEARTH_BRIEFCASE_NO_SERVICE': '1', 'HEARTH_BRIEFCASE_NO_NOTIFY': '1',
            'HEARTH_BRIEFCASE_SECRET_TOOL': '/nonexistent', 'FAKE_ARGV_LOG': self.argv_log,
            'FAKE_BUSY': os.path.join(self.tmp, 'busy'),
        }
        self.patch = mock.patch.dict(os.environ, self.env)
        self.patch.start()
        self.docs = os.path.join(self.home, 'Documents')
        os.makedirs(self.docs)
        self.b = bc.new(self.home)['path']
        bc.CANCEL.clear()

    def tearDown(self):
        self.patch.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)

    def status_of(self, name, path=None):
        s = bc.status(path or self.b)
        return next(i for i in s['items'] if i['name'] == name)

    def add_file(self, name='essay.txt', text='first draft\n', path=None):
        src = os.path.join(self.docs, name)
        write(src, text)
        age(src, 1000)
        bc.add(path or self.b, [src])
        return src, os.path.join(path or self.b, name)

    def apply_plan(self, choose=None, path=None):
        p = bc.plan(path or self.b)
        rows = []
        for r in p['rows']:
            r = dict(r)
            if choose and r['kind'] in choose:
                r['action'] = choose[r['kind']]
            rows.append(r)
        return bc.apply(path or self.b, json.dumps({'rows': rows})), p

    def trashed_names(self):
        d = os.path.join(self.home, '.local', 'share', 'Trash', 'files')
        return sorted(os.listdir(d)) if os.path.isdir(d) else []

    def cli(self, *args, stdin='', extra=None):
        env = dict(os.environ, **(extra or {}))
        return subprocess.run([sys.executable, HELPER] + list(args), input=stdin, capture_output=True, text=True,
                              env=env, timeout=60)


class Statuses(Base):
    def test_new_briefcase_is_a_folder_with_its_list(self):
        self.assertTrue(os.path.isfile(os.path.join(self.b, '.briefcase.json')))
        self.assertEqual(os.path.basename(self.b), 'Briefcase')
        self.assertIn(os.path.realpath(self.b), [b['path'] for b in bc.list_known()['briefcases']])
        again = bc.new(self.home, 'Briefcase', ensure=True)
        self.assertFalse(again['made'])
        other = bc.new(self.home, 'Briefcase')['path']
        self.assertNotEqual(other, self.b)

    def test_up_to_date_and_newer_on_each_side(self):
        src, cp = self.add_file()
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')
        write(cp, 'edited in the briefcase\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'newer-b')
        bc.apply(self.b, json.dumps({'rows': bc.plan(self.b)['rows']}))
        self.assertEqual(read(src), 'edited in the briefcase\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')
        write(src, 'edited at home\n')
        st = self.status_of('essay.txt')
        self.assertEqual(st['status'], 'newer-o')
        self.assertEqual(st['label'], 'Needs updating: newer at original')

    def test_changed_in_both(self):
        src, cp = self.add_file()
        write(cp, 'briefcase side\n')
        write(src, 'home side!\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'conflict')

    def test_same_change_on_both_sides_is_up_to_date(self):
        src, cp = self.add_file()
        write(cp, 'same\n')
        write(src, 'same\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')

    def test_unplugged_drive_is_away_not_orphan_and_found_under_new_mount(self):
        usb = os.path.join(self.tmp, 'media', 'STICK')
        write(os.path.join(usb, 'notes', 'a.txt'), 'on the stick\n')
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = json.dumps({'1234-ABCD': {'path': usb, 'label': 'STICK'}})
        bc.add(self.b, [os.path.join(usb, 'notes', 'a.txt')])
        rec = bc.open_bc(self.b).items()['a.txt']
        self.assertEqual(rec['origin']['kind'], 'drive')
        self.assertEqual(rec['origin']['rel'], 'notes/a.txt')
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = '{}'
        st = self.status_of('a.txt')
        self.assertEqual(st['status'], 'away')
        self.assertEqual(st['label'], 'Plug in “STICK”')
        moved = os.path.join(self.tmp, 'media2', 'STICK1')
        os.makedirs(os.path.dirname(moved))
        shutil.move(usb, moved)
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = json.dumps({'1234-ABCD': {'path': moved, 'label': 'STICK'}})
        self.assertEqual(self.status_of('a.txt')['status'], 'ok')
        os.unlink(os.path.join(moved, 'notes', 'a.txt'))
        st = self.status_of('a.txt')
        self.assertEqual((st['status'], st['reason']), ('orphan', 'deleted'))

    def test_offline_share_is_away(self):
        share = os.path.join(self.run_dir, 'gvfs', 'smb-share:server=study.local,share=docs')
        write(os.path.join(share, 'plan.txt'), 'shared\n')
        bc.add(self.b, [os.path.join(share, 'plan.txt')])
        rec = bc.open_bc(self.b).items()['plan.txt']
        self.assertEqual(rec['origin']['kind'], 'share')
        self.assertTrue(rec['origin']['url'].startswith('smb://study.local/docs/'))
        shutil.rmtree(os.path.join(self.run_dir, 'gvfs'))
        st = self.status_of('plan.txt')
        self.assertEqual((st['status'], st['label']), ('away', 'study is offline'))

    def test_original_on_another_device_is_away(self):
        self.add_file()
        os.environ['HEARTH_BRIEFCASE_MACHINE'] = 'm-laptop'
        st = self.status_of('essay.txt')
        self.assertEqual(st['status'], 'away')
        self.assertIn('desk', st['label'])

    def test_deleted_original_is_orphan_and_split(self):
        src, cp = self.add_file()
        bc.split(self.b, ['essay.txt'])
        st = self.status_of('essay.txt')
        self.assertEqual((st['status'], st['reason']), ('orphan', 'split'))
        src2, _ = self.add_file('other.txt')
        os.unlink(src2)
        self.assertEqual(self.status_of('other.txt')['reason'], 'deleted')

    def test_find_original(self):
        src, _ = self.add_file()
        f = bc.find(self.b, 'essay.txt')
        self.assertEqual(f['folder'], os.path.realpath(self.docs))
        self.assertEqual(f['select'], 'essay.txt')

    def test_removed_from_briefcase_is_forgotten(self):
        src, cp = self.add_file()
        os.unlink(cp)
        bc.status(self.b)
        self.assertNotIn('essay.txt', bc.open_bc(self.b).items())
        self.assertTrue(os.path.exists(src))


class Updating(Base):
    def test_replace_original_trashes_old_version(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        res, plan = self.apply_plan()
        self.assertEqual(plan['rows'][0]['action'], 'to-original')
        self.assertEqual(read(src), 'new\n')
        self.assertEqual(len(res['trashed']), 1)
        self.assertEqual(read(res['trashed'][0]), 'first draft\n')
        info = os.path.join(self.home, '.local', 'share', 'Trash', 'info', os.path.basename(res['trashed'][0]) + '.trashinfo')
        self.assertIn('Path=' + os.path.realpath(src).replace(' ', '%20'), read(info))

    def test_replace_in_briefcase(self):
        src, cp = self.add_file()
        write(src, 'from home\n')
        res, plan = self.apply_plan()
        self.assertEqual(plan['rows'][0]['action'], 'to-briefcase')
        self.assertEqual(read(cp), 'from home\n')
        self.assertEqual(read(res['trashed'][0]), 'first draft\n')

    def test_skip_changes_nothing(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        self.apply_plan({'newer-b': 'skip'})
        self.assertEqual(read(src), 'first draft\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'newer-b')

    def test_conflict_defaults_to_keep_both(self):
        src, cp = self.add_file()
        write(cp, 'briefcase side\n')
        write(src, 'home side\n')
        res, plan = self.apply_plan()
        self.assertEqual(plan['rows'][0]['kind'], 'conflict')
        self.assertEqual(plan['rows'][0]['action'], 'keep-both')
        self.assertEqual(read(cp), 'briefcase side\n')
        self.assertEqual(read(src), 'home side\n')
        kept = [n for n in os.listdir(self.b) if n.startswith('essay (from desk ')]
        self.assertEqual(len(kept), 1)
        self.assertEqual(read(os.path.join(self.b, kept[0])), 'home side\n')
        self.assertTrue(kept[0].endswith(').txt'))
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')

    def test_conflict_can_be_decided_by_hand(self):
        src, cp = self.add_file()
        write(cp, 'briefcase side\n')
        write(src, 'home side\n')
        res, _ = self.apply_plan({'conflict': 'to-original'})
        self.assertEqual(read(src), 'briefcase side\n')
        self.assertEqual(read(res['trashed'][0]), 'home side\n')

    def test_changed_since_the_list_was_made_is_skipped(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        p = bc.plan(self.b)
        write(cp, 'newer still, typed after the list\n')
        res = bc.apply(self.b, json.dumps({'rows': p['rows']}))
        self.assertEqual(read(src), 'first draft\n')
        self.assertIn('changed since', res['skipped'][0]['why'])

    def test_choice_not_offered_is_refused(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        rows = bc.plan(self.b)['rows']
        rows[0]['action'] = 'delete-original'
        res = bc.apply(self.b, json.dumps({'rows': rows}))
        self.assertTrue(os.path.exists(src))
        self.assertTrue(res['skipped'])

    def test_folder_new_files_and_deletes(self):
        proj = os.path.join(self.docs, 'Project')
        write(os.path.join(proj, 'a.txt'), 'a\n')
        write(os.path.join(proj, 'sub', 'b.txt'), 'b\n')
        write(os.path.join(proj, 'gone.txt'), 'g\n')
        for f in ('a.txt', 'sub/b.txt', 'gone.txt'):
            age(os.path.join(proj, f), 1000)
        bc.add(self.b, [proj])
        bp = os.path.join(self.b, 'Project')
        self.assertTrue(os.path.exists(os.path.join(bp, 'sub', 'b.txt')))
        write(os.path.join(bp, 'new-in-briefcase.txt'), 'nb\n')
        write(os.path.join(proj, 'sub', 'new-at-home.txt'), 'nh\n')
        os.unlink(os.path.join(proj, 'gone.txt'))
        self.assertEqual(self.status_of('Project')['status'], 'both')
        p = bc.plan(self.b)
        kinds = {r['rel']: (r['kind'], r['action']) for r in p['rows']}
        self.assertEqual(kinds['new-in-briefcase.txt'], ('new-b', 'to-original'))
        self.assertEqual(kinds['sub/new-at-home.txt'], ('new-o', 'to-briefcase'))
        self.assertEqual(kinds['gone.txt'], ('deleted-o', 'skip'))     # "Delete here too?" never by itself
        bc.apply(self.b, json.dumps({'rows': p['rows']}))
        self.assertTrue(os.path.exists(os.path.join(proj, 'new-in-briefcase.txt')))
        self.assertTrue(os.path.exists(os.path.join(bp, 'sub', 'new-at-home.txt')))
        self.assertTrue(os.path.exists(os.path.join(bp, 'gone.txt')))
        rows = bc.plan(self.b)['rows']
        self.assertEqual([r['rel'] for r in rows], ['gone.txt'])
        rows[0]['action'] = 'delete-briefcase'
        res = bc.apply(self.b, json.dumps({'rows': rows}))
        self.assertFalse(os.path.exists(os.path.join(bp, 'gone.txt')))
        self.assertEqual(read(res['trashed'][0]), 'g\n')
        self.assertEqual(self.status_of('Project')['status'], 'ok')

    def test_usb_stick_uses_its_own_trash(self):
        usb = os.path.join(self.tmp, 'media', 'STICK')
        write(os.path.join(usb, 'doc.txt'), 'old\n')
        age(os.path.join(usb, 'doc.txt'), 1000)
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = json.dumps({'U1': {'path': usb, 'label': 'STICK'}})
        bc.add(self.b, [os.path.join(usb, 'doc.txt')])
        write(os.path.join(self.b, 'doc.txt'), 'new\n')
        real_same = bc.same_device
        with mock.patch.object(bc, 'same_device', lambda a, b: False if a.startswith(usb) else real_same(a, b)), \
                mock.patch.object(bc, 'mount_point', lambda p: usb if p.startswith(usb) else '/'):
            res, _ = self.apply_plan()
        t = os.path.join(usb, '.Trash-%d' % os.getuid())
        self.assertEqual(read(os.path.join(t, 'files', 'doc.txt')), 'old\n')
        self.assertIn('Path=doc.txt', read(os.path.join(t, 'info', 'doc.txt.trashinfo')))   # relative to the stick
        self.assertEqual(read(os.path.join(usb, 'doc.txt')), 'new\n')

    def test_share_without_trash_keeps_replaced_copy_in_briefcase(self):
        share = os.path.join(self.run_dir, 'gvfs', 'smb-share:server=study,share=docs')
        write(os.path.join(share, 'plan.txt'), 'v1\n')
        age(os.path.join(share, 'plan.txt'), 1000)
        bc.add(self.b, [os.path.join(share, 'plan.txt')])
        write(os.path.join(self.b, 'plan.txt'), 'v2\n')
        res, _ = self.apply_plan()
        self.assertEqual(read(os.path.join(share, 'plan.txt')), 'v2\n')
        self.assertEqual(len(res['replaced']), 1)
        self.assertIn(os.path.join(self.b, '.briefcase-replaced'), res['replaced'][0])
        self.assertEqual(read(res['replaced'][0]), 'v1\n')

    def test_crash_mid_copy_keeps_the_old_file(self):
        src = os.path.join(self.docs, 'big.bin')
        with open(src, 'wb') as f:
            f.write(b'A' * (3 * 1024 * 1024))
        age(src, 1000)
        bc.add(self.b, [src])
        with open(os.path.join(self.b, 'big.bin'), 'wb') as f:
            f.write(b'B' * (3 * 1024 * 1024))
        rows = bc.plan(self.b)['rows']
        # A real crash: the helper process dies (os._exit) after 1 MiB of the copy, like a power cut.
        r = self.cli('apply', self.b, json.dumps({'rows': rows}), extra={'HEARTH_BRIEFCASE_CRASH_AFTER': str(1024 * 1024)})
        self.assertEqual(r.returncode, 9)
        with open(src, 'rb') as f:
            self.assertEqual(f.read(), b'A' * (3 * 1024 * 1024))                # untouched
        self.assertTrue([n for n in os.listdir(self.docs) if n.startswith('.briefcase-tmp-')])
        self.assertEqual(self.trashed_names(), [])
        self.assertEqual(self.status_of('big.bin')['status'], 'newer-b')
        bc.apply(self.b, json.dumps({'rows': bc.plan(self.b)['rows']}))
        with open(src, 'rb') as f:
            self.assertEqual(f.read(), b'B' * (3 * 1024 * 1024))
        self.assertFalse([n for n in os.listdir(self.docs) if n.startswith('.briefcase-tmp-')])

    def test_error_mid_copy_cleans_up(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        with mock.patch.object(bc, '_write_chunk', side_effect=OSError(28, 'No space left on device')):
            with self.assertRaises(OSError):
                self.apply_plan()
        self.assertEqual(read(src), 'first draft\n')
        self.assertFalse([n for n in os.listdir(self.docs) if n.startswith('.briefcase-tmp-')])

    def test_cancel_stops_cleanly(self):
        src, cp = self.add_file()
        write(cp, 'new\n')
        bc.CANCEL.set()
        res, _ = self.apply_plan()
        bc.CANCEL.clear()
        self.assertTrue(res['cancelled'])
        self.assertEqual(read(src), 'first draft\n')


class BadList(Base):
    def test_bad_list_is_kept_aside_never_overwritten(self):
        self.add_file()
        p = os.path.join(self.b, '.briefcase.json')
        for f in (p + '.bak',):
            if os.path.exists(f):
                os.unlink(f)
        write(p, '{"items": {broken')
        s = bc.status(self.b)
        self.assertTrue(s['damaged'])
        self.assertEqual(read(p), '{"items": {broken')
        broken = [n for n in os.listdir(self.b) if n.startswith('.briefcase.json.broken-')]
        self.assertEqual(len(broken), 1)
        with self.assertRaises(bc.Fail) as cm:
            bc.add(self.b, [os.path.join(self.docs, 'essay.txt')])
        self.assertEqual(cm.exception.code, 'damaged')
        with self.assertRaises(bc.Fail):
            bc.customize(self.b, json.dumps({'color': 'red'}))
        bc.status(self.b)
        self.assertEqual(len([n for n in os.listdir(self.b) if n.startswith('.briefcase.json.broken-')]), 1)
        self.assertEqual(read(p), '{"items": {broken')
        r = bc.reset_list(self.b)
        self.assertTrue(r['reset'])
        self.assertEqual(read(r['kept']), '{"items": {broken')
        self.assertFalse(bc.status(self.b)['damaged'])
        self.assertTrue(os.path.exists(os.path.join(self.b, 'essay.txt')))      # the files themselves untouched

    def test_bad_list_restored_from_bak(self):
        self.add_file()
        self.add_file('two.txt')           # the second write leaves a good .bak
        p = os.path.join(self.b, '.briefcase.json')
        write(p, 'garbage')
        s = bc.status(self.b)
        self.assertFalse(s['damaged'])
        self.assertIn('essay.txt', [i['name'] for i in s['items']])


class Carry(Base):
    def test_send_keeps_list_and_customization(self):
        self.add_file()
        bc.customize(self.b, json.dumps({'color': 'green', 'icon': 'satchel', 'badge': 'S', 'note': 'School work'}))
        usb = os.path.join(self.tmp, 'media', 'STICK')
        os.makedirs(usb)
        out = bc.send(self.b, usb)
        self.assertTrue(os.path.isfile(os.path.join(out['path'], '.briefcase.json')))
        s = bc.status(out['path'])
        self.assertEqual(s['look']['icon'], 'satchel')
        self.assertEqual(s['look']['note'], 'School work')
        self.assertEqual(self.status_of('essay.txt', out['path'])['status'], 'ok')
        # On another Hearth device the original is shown as on "desk", never as an orphan.
        os.environ['HEARTH_BRIEFCASE_MACHINE'] = 'm-laptop'
        self.assertEqual(self.status_of('essay.txt', out['path'])['status'], 'away')


class Swapping(Base):
    def setUp(self):
        super().setUp()
        self.b2 = bc.new(self.home, 'School')['path']

    def test_copy_and_move_keep_the_link(self):
        src, cp = self.add_file()
        bc.transfer(self.b, self.b2, ['essay.txt'], move=False)
        self.assertEqual(self.status_of('essay.txt', self.b2)['status'], 'ok')
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')
        write(src, 'changed at home\n')
        self.assertEqual(self.status_of('essay.txt', self.b2)['status'], 'newer-o')
        b3 = bc.new(self.home, 'Work')['path']
        bc.transfer(self.b, b3, ['essay.txt'], move=True)
        self.assertNotIn('essay.txt', bc.open_bc(self.b).items())
        self.assertFalse(os.path.exists(cp))
        self.assertIn('essay.txt', self.trashed_names())          # the source's copy went to the Trash
        self.assertEqual(self.status_of('essay.txt', b3)['status'], 'newer-o')

    def test_swap(self):
        self.add_file('a.txt', 'A\n')
        self.add_file('b.txt', 'B\n', path=self.b2)
        bc.swap(self.b, 'a.txt', self.b2, 'b.txt')
        self.assertEqual(sorted(bc.open_bc(self.b).items()), ['b.txt'])
        self.assertEqual(sorted(bc.open_bc(self.b2).items()), ['a.txt'])
        self.assertEqual(self.status_of('a.txt', self.b2)['status'], 'ok')
        self.assertEqual(read(os.path.join(self.b, 'b.txt')), 'B\n')

    def test_crash_mid_move_loses_nothing(self):
        src, cp = self.add_file()
        with mock.patch.object(bc, 'remove_to_trash', side_effect=OSError('power cut')):
            with self.assertRaises(OSError):
                bc.transfer(self.b, self.b2, ['essay.txt'], move=True)
        self.assertTrue(os.path.exists(os.path.join(self.b2, 'essay.txt')))
        self.assertTrue(os.path.exists(cp))

    def test_locked_needs_unlocking_first(self):
        self.add_file()
        with mock.patch.object(bc, 'read_password_lines', return_value=['pw-School-1']):
            bc.lock_setup(self.b2)
        with self.assertRaises(bc.Fail) as cm:
            bc.transfer(self.b, self.b2, ['essay.txt'], move=False)
        self.assertEqual(cm.exception.code, 'locked')
        self.assertEqual(cm.exception.extra['path'], os.path.realpath(self.b2))
        with mock.patch.object(bc, 'read_password_lines', return_value=['pw-School-1']):
            bc.unlock(self.b2)
        bc.transfer(self.b, self.b2, ['essay.txt'], move=False)
        inner = bc.open_bc(self.b2)
        self.assertIn('essay.txt', inner.items())
        self.assertTrue(os.path.realpath(os.path.join(inner.content, 'essay.txt')).startswith(
            os.path.realpath(os.path.join(self.b2, '.briefcase-vault'))))     # stored inside the encrypted folder


class Customize(Base):
    def test_customization_persists_and_validates(self):
        out = bc.customize(self.b, json.dumps({'color': 'blue', 'icon': 'backpack', 'badge': '🎒', 'note': 'Trips',
                                               'view': 'details', 'sort': 'status', 'showInNav': False,
                                               'autolock': {'minutes': 30, 'onSleep': False}}))
        look = bc.status(self.b)['look']
        self.assertEqual((look['color'], look['icon'], look['badge'], look['view']), ('blue', 'backpack', '🎒', 'details'))
        self.assertEqual(look['autolock'], {'minutes': 30, 'onSleep': False, 'onEject': True})
        self.assertFalse(look['showInNav'])
        for bad in ({'icon': 'spaceship'}, {'color': 'mauve'}, {'badge': 'TOOLONG'}, {'autolock': {'minutes': -3}}):
            with self.assertRaises(bc.Fail):
                bc.customize(self.b, json.dumps(bad))
        self.assertEqual(bc.status(self.b)['look']['icon'], 'backpack')
        self.assertTrue(os.path.exists(os.path.join(self.b, '.briefcase.json.bak')))
        self.assertEqual(out['path'], self.b)

    def test_rename(self):
        self.add_file()
        out = bc.customize(self.b, json.dumps({'name': 'School'}))
        self.assertEqual(os.path.basename(out['path']), 'School')
        self.assertEqual(bc.status(out['path'])['name'], 'School')
        self.assertIn(os.path.realpath(out['path']), bc.registry())

    def test_customizing_never_touches_files(self):
        src, cp = self.add_file()
        before = os.stat(cp).st_mtime_ns
        bc.customize(self.b, json.dumps({'color': 'red', 'note': 'x'}))
        self.assertEqual(os.stat(cp).st_mtime_ns, before)
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')


class LockBase(Base):
    PW = 'correct horse 9!'

    def lock_setup(self, path=None):
        with mock.patch.object(bc, 'read_password_lines', return_value=[self.PW]):
            return bc.lock_setup(path or self.b)

    def unlock(self, pw=None, path=None):
        with mock.patch.object(bc, 'read_password_lines', return_value=[pw or self.PW]):
            return bc.unlock(path or self.b)


class Locking(LockBase):

    def test_lock_unlock_round_trip(self):
        src, cp = self.add_file()
        res = self.lock_setup()
        self.assertEqual(res['trashedPlain'], ['essay.txt'])
        self.assertFalse(os.path.exists(cp))
        self.assertIn('essay.txt', self.trashed_names())         # the plain copy went to the Trash, not deleted
        self.assertTrue(os.path.isdir(os.path.join(self.b, '.briefcase-vault')))
        s = bc.status(self.b)
        self.assertTrue(s['locked'])
        self.assertFalse(s['unlocked'])
        self.assertEqual(s['items'], [])                             # "Locked" without reading what's inside
        self.assertEqual(s['look']['autolock']['minutes'], 15)
        raw = read(os.path.join(self.b, '.briefcase.json'))
        self.assertNotIn('essay', raw)
        self.assertFalse(os.path.exists(os.path.join(self.b, '.briefcase.json.bak')))
        u = self.unlock()
        self.assertTrue(os.path.exists(os.path.join(u['content'], 'essay.txt')))
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')
        bc.lock(self.b)
        self.assertFalse(bc.status(self.b)['unlocked'])

    def test_wrong_password_is_refused(self):
        self.add_file()
        self.lock_setup()
        with self.assertRaises(bc.Fail) as cm:
            self.unlock('wrong')
        self.assertEqual(cm.exception.code, 'badpass')
        self.assertEqual(str(cm.exception), "That password isn't right")
        self.assertFalse(bc.status(self.b)['unlocked'])

    def test_update_all_on_locked_asks_to_unlock(self):
        self.add_file()
        self.lock_setup()
        with self.assertRaises(bc.Fail) as cm:
            bc.plan(self.b)
        self.assertEqual(cm.exception.code, 'locked')

    def test_remove_lock_round_trip(self):
        src, cp = self.add_file()
        self.lock_setup()
        self.unlock()
        bc.lock_remove(self.b)
        s = bc.status(self.b)
        self.assertFalse(s['locked'])
        self.assertEqual(read(cp), 'first draft\n')
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')
        self.assertFalse(os.path.exists(os.path.join(self.b, '.briefcase-vault')))
        self.assertIn('.briefcase-vault', self.trashed_names())

    def test_change_password(self):
        self.add_file()
        self.lock_setup()
        with mock.patch.object(bc, 'read_password_lines', return_value=[self.PW, 'new pass']):
            bc.passwd(self.b)
        with self.assertRaises(bc.Fail):
            self.unlock()
        self.unlock('new pass')

    def test_crash_during_conversion_never_loses_plain_files(self):
        src, cp = self.add_file()
        with mock.patch.object(bc, '_verify_same', side_effect=bc.Fail('power cut', 'verify')):
            with self.assertRaises(bc.Fail):
                self.lock_setup()
        self.assertEqual(read(cp), 'first draft\n')
        self.assertFalse(bc.status(self.b)['locked'])
        # A crash after the encrypted copy was checked and put in place, before the list was switched:
        self.lock_setup_until_rename()
        self.assertTrue(os.path.exists(cp))
        s = bc.status(self.b)                         # opening it finishes the lock
        self.assertTrue(s['locked'])
        self.assertFalse(os.path.exists(cp))
        self.assertIn('essay.txt', self.trashed_names())
        self.unlock()
        self.assertEqual(self.status_of('essay.txt')['status'], 'ok')

    def lock_setup_until_rename(self):
        real = bc._stub_from
        with mock.patch.object(bc, '_stub_from', side_effect=RuntimeError('power cut')):
            with self.assertRaises(RuntimeError):
                self.lock_setup()
        bc._stub_from = real

    def test_crash_during_remove_lock_keeps_encrypted_copy(self):
        self.add_file()
        self.lock_setup()
        self.unlock()
        with mock.patch.object(bc, '_verify_same', side_effect=bc.Fail('power cut', 'verify')):
            with self.assertRaises(bc.Fail):
                bc.lock_remove(self.b)
        self.assertTrue(os.path.isdir(os.path.join(self.b, '.briefcase-vault')))
        self.assertTrue(bc.status(self.b)['locked'])
        self.assertFalse(os.path.exists(os.path.join(self.b, '.briefcase-unlocking')))

    def test_password_never_in_files_logs_or_argv(self):
        src, cp = self.add_file()
        secret = 'Zebra-Secret-77'
        r = self.cli('lock-setup', self.b, stdin=secret + '\n')
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        r = self.cli('unlock', self.b, stdin='nope\n')
        self.assertEqual(json.loads(r.stdout.strip().splitlines()[-1])['code'], 'badpass')
        r = self.cli('unlock', self.b, stdin=secret + '\n')
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertNotIn(secret, r.stdout + r.stderr)
        self.assertNotIn(secret, read(self.argv_log))
        for dp, dns, fns in os.walk(self.tmp):
            for f in fns:
                p = os.path.join(dp, f)
                if os.path.islink(p):
                    continue
                with open(p, 'rb') as fh:
                    self.assertNotIn(secret.encode(), fh.read(), p)
        self.assertNotIn('--password', read(HELPER).split('def main')[1])

    def test_remember_uses_keyring_only_when_asked(self):
        self.add_file()
        store = os.path.join(self.tmp, 'keyring')
        st = os.path.join(self.tmp, 'bin', 'secret-tool')
        write(st, '#!/bin/sh\nmkdir -p %s\ncase "$1" in store) cat > %s/"$5";; lookup) cat %s/"$3" 2>/dev/null || exit 1;; '
                  'clear) rm -f %s/"$3";; esac\n' % (store, store, store, store))
        os.chmod(st, 0o755)
        os.environ['HEARTH_BRIEFCASE_SECRET_TOOL'] = st
        self.lock_setup()
        self.assertFalse(os.path.exists(store))      # off by default
        with mock.patch.object(bc, 'read_password_lines', return_value=[self.PW]):
            bc.unlock(self.b, remember=True)
        bc.lock(self.b)
        bc.unlock(self.b, use_keyring=True)
        self.assertTrue(bc.status(self.b)['unlocked'])

    def test_busy_lock_is_reported(self):
        self.add_file()
        self.lock_setup()
        self.unlock()
        write(os.environ['FAKE_BUSY'], '')
        with self.assertRaises(bc.Fail) as cm:
            bc.lock(self.b)
        self.assertEqual(cm.exception.code, 'busy')


class AutoLock(LockBase):
    def test_decision(self):
        d = bc.autolock_decision
        self.assertEqual(d(1000, 1000, 0, False), ('never', None))
        self.assertEqual(d(1000, 400, 15, True)[0], 'open')            # an open file keeps it unlocked
        self.assertEqual(d(1000, 900, 15, False), ('wait', 800))
        self.assertEqual(d(1000, 120, 15, False)[0], 'warn')          # a minute before: "will lock in 1 minute"
        self.assertEqual(d(1000, 120, 15, False, warned=130)[0], 'wait')
        self.assertEqual(d(1000, 100, 15, False), ('lock', 0))

    def test_guard_locks_when_idle_and_open_files_defer(self):
        self.add_file()
        self.lock_setup()
        bc.customize(self.b, json.dumps({'autolock': {'minutes': 5}}))
        self.unlock()
        mp = bc.open_bc(self.b).mp
        t0 = time.time()
        with mock.patch.object(bc, 'newest_mtime', return_value=0), mock.patch.object(bc, 'notify_lock_soon'):
            with mock.patch.object(bc, 'open_files', return_value=['123']):
                bc.guard_tick(t0 + 3600)
            self.assertTrue(bc.status(self.b)['unlocked'])
            with mock.patch.object(bc, 'open_files', return_value=[]):
                bc.guard_tick(t0 + 3600 + 120)
                self.assertTrue(bc.is_mounted(mp))
                bc.guard_tick(t0 + 3600 + 301)
        self.assertFalse(bc.status(self.b)['unlocked'])

    def test_never_keeps_it_open(self):
        self.add_file()
        self.lock_setup()
        bc.customize(self.b, json.dumps({'autolock': {'minutes': 0}}))
        self.unlock()
        with mock.patch.object(bc, 'newest_mtime', return_value=0), mock.patch.object(bc, 'open_files', return_value=[]):
            bc.guard_tick(time.time() + 10 ** 6)
        self.assertTrue(bc.status(self.b)['unlocked'])
        self.assertEqual(bc.status(self.b)['look']['autolock']['minutes'], 0)

    def test_sleep_and_eject(self):
        self.add_file()
        self.lock_setup()
        self.unlock()
        self.assertTrue(bc.sleep_line("/org/freedesktop/login1: org.freedesktop.login1.Manager.PrepareForSleep (true,)"))
        self.assertFalse(bc.sleep_line("PrepareForSleep (false,)"))
        bc.customize(self.b, json.dumps({'autolock': {'onSleep': False}}))
        bc.lock_all(reason='sleep')
        self.assertTrue(bc.status(self.b)['unlocked'])
        bc.lock_all(under_path=os.path.join(self.tmp, 'elsewhere'), reason='eject')
        self.assertTrue(bc.status(self.b)['unlocked'])
        bc.lock_all(under_path=self.home, reason='eject')
        self.assertFalse(bc.status(self.b)['unlocked'])

    def test_eject_and_sleep_are_wired(self):
        files = read(os.path.join(SHELL, 'hearth-files'))
        self.assertIn("'lock-all', '--under'", files)
        self.assertIn("'--reason', 'eject'", files)


class AutoSync(Base):
    def test_default_on_only_for_new_and_never_overwritten(self):
        raw = json.loads(read(os.path.join(self.b, '.briefcase.json')))
        self.assertEqual(raw['look']['autosync']['mode'], 'instant')
        bc.customize(self.b, json.dumps({'autosync': {'mode': 'off'}}))
        bc.customize(self.b, json.dumps({'color': 'red'}))
        bc.status(self.b)
        self.assertEqual(bc.status(self.b)['look']['autosync']['mode'], 'off')
        # A list made earlier on this branch with no auto-sync key: Right away, filled in when read, never saved.
        old = bc.new(self.home, 'Old')['path']
        p = os.path.join(old, '.briefcase.json')
        d = json.loads(read(p))
        del d['look']['autosync']
        write(p, json.dumps(d))
        before = read(p)
        self.assertEqual(bc.status(old)['look']['autosync']['mode'], 'instant')
        self.assertEqual(read(p), before)

    def test_one_sided_changes_sync_after_settling(self):
        src, cp = self.add_file()
        write(cp, 'edit\n')
        t = time.time()
        r = bc.autosync_one(bc.open_bc(self.b), t)
        self.assertEqual((r['synced'], r['waiting']), (0, 1))          # just saved: wait ~3 seconds
        self.assertEqual(read(src), 'first draft\n')
        with mock.patch.object(bc, 'open_for_writing', return_value=True):
            r = bc.autosync_one(bc.open_bc(self.b), t + 10)
            self.assertEqual(r['waiting'], 1)                           # still being written by an app
        r = bc.autosync_one(bc.open_bc(self.b), t + 10)
        self.assertEqual(r['synced'], 1)
        self.assertEqual(read(src), 'edit\n')
        self.assertIn('essay.txt', self.trashed_names())               # the replaced version is in the Trash
        self.assertEqual(bc.sync_state()[self.b]['last'], t + 10)
        write(src, 'home edit\n')
        bc.autosync_one(bc.open_bc(self.b), t + 20)
        self.assertEqual(read(cp), 'home edit\n')

    def test_both_changed_waits_and_tells_once(self):
        src, cp = self.add_file()
        write(cp, 'b\n')
        write(src, 'o\n')
        with mock.patch.object(bc, 'notify_needs') as nn:
            r = bc.autosync_one(bc.open_bc(self.b), time.time() + 10)
            bc.autosync_one(bc.open_bc(self.b), time.time() + 20)
        self.assertEqual(r['needs'], 1)
        self.assertEqual(nn.call_count, 1)
        self.assertEqual(bc.needs_text(2, 'School'), '2 files in “School” need you')
        self.assertEqual((read(cp), read(src)), ('b\n', 'o\n'))
        self.assertEqual(bc.sync_state()[self.b]['needs'], 1)

    def test_deletions_wait(self):
        proj = os.path.join(self.docs, 'P')
        write(os.path.join(proj, 'x.txt'), 'x\n')
        bc.add(self.b, [proj])
        os.unlink(os.path.join(proj, 'x.txt'))
        r = bc.autosync_one(bc.open_bc(self.b), time.time() + 10)
        self.assertEqual(r['needs'], 1)
        self.assertTrue(os.path.exists(os.path.join(self.b, 'P', 'x.txt')))

    def test_unplugged_original_syncs_when_back(self):
        usb = os.path.join(self.tmp, 'media', 'STICK')
        write(os.path.join(usb, 'd.txt'), 'v1\n')
        drives = json.dumps({'U1': {'path': usb, 'label': 'STICK'}})
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = drives
        bc.add(self.b, [os.path.join(usb, 'd.txt')])
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = '{}'
        write(os.path.join(self.b, 'd.txt'), 'v2\n')
        r = bc.autosync_one(bc.open_bc(self.b), time.time() + 10)
        self.assertEqual((r['away'], r['synced']), (1, 0))
        os.environ['HEARTH_BRIEFCASE_DRIVES'] = drives
        r = bc.autosync_one(bc.open_bc(self.b), time.time() + 10)
        self.assertEqual(r['synced'], 1)
        self.assertEqual(read(os.path.join(usb, 'd.txt')), 'v2\n')

    def test_locked_waits_and_onlock_syncs_before_locking(self):
        src, cp = self.add_file()
        with mock.patch.object(bc, 'read_password_lines', return_value=['pw']):
            bc.lock_setup(self.b)
        r = bc.autosync_one(bc.open_bc(self.b), time.time() + 10)
        self.assertTrue(r['locked'])
        bc.customize(self.b, json.dumps({'autosync': {'mode': 'onlock'}}))
        with mock.patch.object(bc, 'read_password_lines', return_value=['pw']):
            u = bc.unlock(self.b)
        write(os.path.join(u['content'], 'essay.txt'), 'edited while unlocked\n')
        out = bc.lock(self.b)
        self.assertEqual(out['synced']['synced'], 1)
        self.assertEqual(read(src), 'edited while unlocked\n')

    def test_service_only_with_an_autosync_briefcase(self):
        bc.customize(self.b, json.dumps({'autosync': {'mode': 'off'}}))
        self.assertFalse(bc.autosync_needed())
        r = self.cli('autosync-needed')
        self.assertEqual(r.returncode, 1)
        b2 = bc.new(self.home, 'Synced')['path']
        self.assertTrue(bc.autosync_needed())
        self.assertEqual(bc.autosync_daemon(max_loops=1)['ok'], True)
        unit = read(os.path.join(ROOT, 'os', 'system_files', 'usr', 'lib', 'systemd', 'user', 'hearth-briefcase-sync.service'))
        self.assertIn('ExecCondition=', unit)
        self.assertIn('autosync-needed', unit)
        self.assertIn('Restart=on-failure', unit)
        self.assertIn('StartLimitBurst=', unit)
        self.assertNotIn('WantedBy=', unit)                             # never part of sign-in or Game Mode starting
        auto = read(os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'labwc', 'autostart'))
        self.assertIn('hearth-briefcase-sync.service', auto)
        line = next(l for l in auto.splitlines() if 'hearth-briefcase-sync' in l and 'systemctl' in l)
        self.assertIn('--no-block', line)
        self.assertTrue(line.rstrip().endswith('&'))
        bc.customize(b2, json.dumps({'autosync': {'mode': 'off'}}))
        self.assertEqual(bc.autosync_daemon(max_loops=3)['ok'], True)   # exits at once: nothing wants it

    def test_interval_mode(self):
        bc.customize(self.b, json.dumps({'autosync': {'mode': 'interval', 'minutes': 5}}))
        b = bc.open_bc(self.b)
        self.assertTrue(bc.autosync_due(b, 1000, {}))
        self.assertFalse(bc.autosync_due(b, 1000, {'checked': 900}))
        self.assertTrue(bc.autosync_due(b, 1301, {'checked': 1000}))


class FilesWiring(unittest.TestCase):
    def test_files_shows_briefcases(self):
        files = read(os.path.join(SHELL, 'hearth-files'))
        self.assertIn("'kind': 'briefcase'", files)
        self.assertIn('briefcase_places', files)
        qml = read(os.path.join(SHELL, 'FileManager.qml'))
        self.assertIn('Briefcase {', qml)
        self.assertTrue(os.path.exists(os.path.join(SHELL, 'Briefcase.qml')))
        for icon in ('briefcase', 'satchel', 'suitcase', 'backpack', 'box'):
            self.assertTrue(os.path.exists(os.path.join(SHELL, 'icons', 'bc-%s-light.svg' % icon)), icon)
        b = read(os.path.join(SHELL, 'Briefcase.qml')) + read(os.path.join(SHELL, 'BriefcaseDialog.qml'))
        for words in ('Update All', 'Split from original', 'Find original', 'Send Briefcase to', 'Customize',
                      'Lock this Briefcase', "If you forget this password, the files inside can't be recovered. Hearth can't reset it.",
                      'Remember on this device', 'Move to Briefcase', 'Copy to Briefcase', 'Swap with',
                      'Locks in', 'Synced', 'Auto-sync', 'stdinEnabled'):
            self.assertIn(words, b, words)
        self.assertNotIn('HEARTH_INPUT', b)            # the password goes on stdin, not the environment

    def test_gocryptfs_in_image(self):
        build = read(os.path.join(ROOT, 'os-bases', 'peppermint', 'build.sh'))
        self.assertIn('gocryptfs', build)
        self.assertIn("command -v gocryptfs", build)


@unittest.skipUnless(shutil.which('gocryptfs') and os.path.exists('/dev/fuse') and
                     (shutil.which('fusermount3') or shutil.which('fusermount')), 'no FUSE / gocryptfs here')
class RealGocryptfs(Base):
    def setUp(self):
        super().setUp()
        for k in ('HEARTH_BRIEFCASE_FAKE_MOUNTS', 'HEARTH_BRIEFCASE_GOCRYPTFS', 'HEARTH_BRIEFCASE_FUSERMOUNT'):
            os.environ.pop(k, None)

    def test_real_round_trip(self):
        src, cp = self.add_file()
        try:
            with mock.patch.object(bc, 'read_password_lines', return_value=['real pass 1']):
                bc.lock_setup(self.b)
        except bc.Fail as e:
            if e.code in ('mountfail', 'initfail'):
                self.skipTest('FUSE refused here: %s' % e)
            raise
        self.assertTrue(os.path.exists(os.path.join(self.b, '.briefcase-vault', 'gocryptfs.conf')))
        with mock.patch.object(bc, 'read_password_lines', return_value=['bad']):
            with self.assertRaises(bc.Fail):
                bc.unlock(self.b)
        with mock.patch.object(bc, 'read_password_lines', return_value=['real pass 1']):
            u = bc.unlock(self.b)
        self.assertEqual(read(os.path.join(u['content'], 'essay.txt')), 'first draft\n')
        bc.lock(self.b)
        print('::notice title=Briefcase::the real gocryptfs lock round trip ran (FUSE available)')


def tearDownModule():
    if not (shutil.which('gocryptfs') and os.path.exists('/dev/fuse')):
        print('::notice title=Briefcase::FUSE or gocryptfs not available here: locking was tested with the stand-in '
              'gocryptfs (fake_gocryptfs.py) only')


if __name__ == '__main__':
    unittest.main()
