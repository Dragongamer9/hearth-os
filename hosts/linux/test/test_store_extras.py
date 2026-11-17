"""1.14's App store extras (hearth_store_extras.py through hearth-store) and recent files per activity
(hearth_files_extras.py's activity_*):
  - update notices: `check` tells a set of updates once, nothing when it's off; the choice merges into store.json
  - new permissions: read from the installed and the new metadata, in plain words; Update all leaves those apps out,
    and one of them updates only with --allow-new
  - drivers: Nvidia, Wi-Fi, sound, network and microcode found from lspci, the CPU and the kernel's firmware
    failures; only real driver packages install, and only through hearth-admin
  - recent files: each activity's own list, a shared one, and "Don't remember" forgetting only what was opened on it
"""
import contextlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
FAKE = pathlib.Path(__file__).resolve().parent / 'fake_flatpak_store.py'
FIXTURE = pathlib.Path(__file__).resolve().parent / 'drivers_fixture.json'
NODE = shutil.which('node')

OLD = '[Application]\nname=org.kde.krita\n\n[Context]\nshared=ipc;\nsockets=wayland;fallback-x11;pulseaudio;\ndevices=dri;\nfilesystems=xdg-pictures;\n'
NEW = OLD.replace('shared=ipc;', 'shared=ipc;network;').replace('filesystems=xdg-pictures;', 'filesystems=xdg-pictures;home;!host;') + \
    '\n[Session Bus Policy]\norg.freedesktop.secrets=talk\n'
STATE = {'updates': {'user': [['org.kde.krita', 'Krita', '5.3', 'stable', 'flathub', '120 MB'],
                              ['org.gnome.Calculator', 'Calculator', '48', 'stable', 'flathub', '3 MB']],
                     'system': [['com.valvesoftware.Steam', 'Steam', '1.0.0.82', 'stable', 'flathub', '9 MB']]},
         'meta': {'org.kde.krita': {'old': OLD, 'new': NEW}},
         'remotes': {'user': [['flathub', 'Flathub', 'https://dl.flathub.org/repo/', '']], 'system': []}}


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = pathlib.Path(self.tmp.name)
        self.t = t
        self.state = t / 'state.json'
        self.state.write_text(json.dumps(STATE))
        self.log = t / 'log'
        self.env = {'HEARTH_FLATPAK': str(FAKE), 'FAKE_STATE': str(self.state), 'FAKE_LOG': str(self.log),
                    'XDG_CACHE_HOME': str(t / 'cache'), 'XDG_CONFIG_HOME': str(t / 'config'), 'XDG_DATA_HOME': str(t / 'data'),
                    'HOME': str(t)}
        os.chmod(FAKE, 0o755)

    def tearDown(self):
        self.tmp.cleanup()

    def run_h(self, *args, helper='hearth-store', extra=None):
        r = subprocess.run([sys.executable, str(SHELL / helper)] + list(args), env=dict(os.environ, **self.env, **(extra or {})),
                           capture_output=True, text=True, timeout=60)
        last = (r.stdout.strip().splitlines() or [''])[-1]
        try:
            return r.returncode, json.loads(last), r.stdout
        except ValueError:
            return r.returncode, last, r.stdout + r.stderr


class Permissions(unittest.TestCase):
    def setUp(self):
        self.x = load(SHELL / 'hearth_store_extras.py', 'hse_perm')

    def test_parse_skips_negated_and_reads_bus_names(self):
        p = self.x.perms_of(NEW)
        self.assertIn('shared:network', p)
        self.assertIn('filesystems:home', p)
        self.assertIn('session:org.freedesktop.secrets=talk', p)
        self.assertNotIn('filesystems:!host', p)
        self.assertNotIn('filesystems:host', p)

    def test_only_what_is_new_in_plain_words(self):
        got = self.x.new_perms(OLD, NEW)
        self.assertEqual([g['key'] for g in got], ['filesystems:home', 'session:org.freedesktop.secrets=talk', 'shared:network'])
        words = [g['label'] for g in got]
        self.assertIn('Use the internet', words)
        self.assertIn('Read and change your home folder', words)
        self.assertIn('Talk to org.freedesktop.secrets', words)
        self.assertEqual(self.x.new_perms(OLD, OLD), [])

    def test_a_folder_inside_home_is_not_new_when_home_was_allowed(self):
        old = '[Context]\nfilesystems=home;\n'
        self.assertEqual(self.x.new_perms(old, '[Context]\nfilesystems=home;xdg-download;~/Games:ro;\n'), [])
        self.assertEqual(self.x.perm_words('filesystems:xdg-download:ro'), 'Read your Downloads folder')
        self.assertEqual(self.x.perm_words('devices:all'), 'Use every device (cameras, controllers, USB drives…)')


class Updates(Base):
    def test_updates_carry_new_permissions(self):
        code, out, _ = self.run_h('updates')
        self.assertEqual(code, 0, out)
        by = {r['id']: r for r in out['flatpaks']}
        self.assertEqual(len(by['org.kde.krita']['newPerms']), 3)
        self.assertEqual(by['org.gnome.Calculator']['newPerms'], [])

    def test_update_all_leaves_out_an_app_that_asks_for_more(self):
        code, out, _ = self.run_h('update')
        self.assertEqual(code, 0, out)
        self.assertEqual([w['id'] for w in out['waiting']], ['org.kde.krita'])
        st = json.loads(self.state.read_text())
        self.assertEqual([r[0] for r in st['updates']['user']], ['org.kde.krita'])     # Calculator updated, Krita waits
        self.assertEqual(st['updates']['system'], [])                                   # nothing waits there: all of it
        calls = self.log.read_text().splitlines()
        self.assertTrue(any(c.startswith('update -y --noninteractive --user org.gnome.Calculator//stable') for c in calls), calls)
        self.assertFalse(any(c.startswith('update') and 'krita' in c for c in calls), calls)

    def test_one_app_with_new_permissions_needs_your_yes(self):
        code, out, _ = self.run_h('update', 'org.kde.krita')
        self.assertEqual(code, 1)
        self.assertIn('new permissions', out)
        self.assertIn('Use the internet', out)
        code, out, _ = self.run_h('update', 'org.kde.krita', '--allow-new')
        self.assertEqual(code, 0, out)
        st = json.loads(self.state.read_text())
        self.assertNotIn('org.kde.krita', [r[0] for r in st['updates']['user']])

    def test_check_tells_once_and_not_when_off(self):
        code, out, _ = self.run_h('check')
        self.assertEqual(code, 0, out)
        self.assertTrue(out['fresh'])
        self.assertEqual(out['count'], 3)
        self.assertEqual(out['waiting'], 1)
        _, again, _ = self.run_h('check')
        self.assertFalse(again['fresh'])                      # the same updates: told already
        cfg = self.t / 'config' / 'nimbus' / 'store.json'
        cfg.parent.mkdir(parents=True, exist_ok=True)
        cfg.write_text(json.dumps({'other': 1}))
        code, out, _ = self.run_h('prefs-set', 'notifyUpdates', 'off')
        self.assertEqual(code, 0, out)
        self.assertEqual(json.loads(cfg.read_text()), {'other': 1, 'notifyUpdates': False})   # merged, nothing else lost
        self.assertTrue(os.path.exists(str(cfg) + '.bak'))
        _, off, _ = self.run_h('check')
        self.assertFalse(off['notify'])
        self.assertEqual(self.run_h('prefs')[1]['prefs'], {'notifyUpdates': False})
        code, out, _ = self.run_h('prefs-set', 'notifyUpdates', 'maybe')
        self.assertEqual(code, 1)

    def test_a_damaged_store_json_is_not_overwritten_with_defaults_by_reading(self):
        cfg = self.t / 'config' / 'nimbus' / 'store.json'
        cfg.parent.mkdir(parents=True, exist_ok=True)
        cfg.write_text('{broken')
        _, out, _ = self.run_h('prefs')
        self.assertEqual(out['prefs'], {'notifyUpdates': True})
        self.assertFalse(cfg.exists() and cfg.read_text() == json.dumps({'notifyUpdates': True}))


class Drivers(Base):
    def test_found_from_the_pc(self):
        code, out, _ = self.run_h('drivers', extra={'HEARTH_DRIVERS_FIXTURE': str(FIXTURE)})
        self.assertEqual(code, 0, out)
        by = {r['package']: r for r in out['drivers']}
        self.assertEqual(by['nvidia-open']['installed'], '580.95.05-1')
        self.assertTrue(by['nvidia-open']['inUse'])
        self.assertNotIn('nvidia-driver', by)                    # Debian's 550 isn't offered next to NVIDIA's own
        for p in ('firmware-iwlwifi', 'firmware-intel-graphics', 'firmware-sof-signed', 'intel-microcode', 'firmware-nvidia-graphics'):
            self.assertTrue(by[p]['installed'], p)
        self.assertEqual(by['firmware-realtek']['installed'], '')
        self.assertEqual(by['firmware-realtek']['available'], '20250410-2')
        self.assertIn('rtl_nic/rtl8125b-2.fw', ' '.join(by['firmware-realtek']['why']))
        self.assertIn('firmware-mediatek', by)
        self.assertEqual(out['missing'], 2)
        self.assertNotIn('amd64-microcode', by)
        # The processor's microcode and Nvidia's driver come with Hearth OS's image: shown, never offered to install.
        for p in ('nvidia-open', 'intel-microcode'):
            self.assertTrue(by[p]['infoOnly'], p)
            self.assertEqual(by[p]['available'], '', p)
        self.assertEqual(by['intel-microcode']['note'], "Comes with Hearth OS's updates")
        self.assertIn("Hearth OS's own image carries it", by['nvidia-open']['note'])

    def test_only_the_firmware_it_offers_is_ever_installed(self):
        x = load(SHELL / 'hearth_store_extras.py', 'hse_pkgs')
        offered = {p for v in x.VENDORS.values() for _, p, _ in v if p != 'nvidia'} | {p for _, p in x.FIRMWARE_FILES}
        for p in offered:
            self.assertTrue(x.DRIVER_PKG.fullmatch(p), p)
        for bad in ('nvidia-open', 'nvidia-driver', 'intel-microcode', 'amd64-microcode', 'firmware-anything',
                    'firmware-realtek;rm', 'firmware-realtek\n', 'linux-image-amd64'):
            self.assertIsNone(x.DRIVER_PKG.fullmatch(bad), bad)
        admin = (ROOT / 'os/system_files/usr/libexec/nimbus/hearth-admin').read_text()
        self.assertIn("DRIVER_PKG = re.compile(r'(%s)')" % x.DRIVER_PKG.pattern[1:-1], admin)   # the same list there
        self.assertIn('DRIVER_PKG.fullmatch(argv[2])', admin)

    def test_install_takes_only_drivers_this_pc_needs_and_goes_through_hearth_admin(self):
        fake = self.t / 'pkexec'
        fake.write_text('#!/bin/sh\necho "$@" >> "%s"\necho "progress: 50 Installing"\necho \'{"ok": true}\'\n' % (self.t / 'pk.log'))
        fake.chmod(0o755)
        extra = {'HEARTH_DRIVERS_FIXTURE': str(FIXTURE), 'HEARTH_PKEXEC': str(fake)}
        code, out, _ = self.run_h('driver-install', 'bash', extra=extra)
        self.assertEqual(code, 1)
        for p in ('nvidia-open', 'intel-microcode'):                                   # shown, never installed here
            code, out, _ = self.run_h('driver-install', p, extra=extra)
            self.assertEqual(code, 1, p)
        code, out, _ = self.run_h('driver-install', 'firmware-atheros', extra=extra)     # a driver, but not this PC's
        self.assertEqual(code, 1)
        self.assertIn("doesn't need", out)
        self.assertFalse((self.t / 'pk.log').exists())
        code, out, text = self.run_h('driver-install', 'firmware-realtek', extra=extra)
        self.assertEqual(code, 0, out)
        self.assertIn('progress: 50 Installing', text)
        self.assertEqual((self.t / 'pk.log').read_text().split(), ['/usr/libexec/nimbus/hearth-admin', 'driver-install', 'firmware-realtek'])

    def test_lspci_both_forms(self):
        x = load(SHELL / 'hearth_store_extras.py', 'hse_pci')
        a = x.pci_devices('01:00.0 "0300" "10de" "2786" -ra1 "1043" "8932"')
        b = x.pci_devices('01:00.0 "VGA compatible controller [0300]" "NVIDIA Corporation [10de]" "AD104 [2786]" -ra1 "ASUS [1043]" "x [8932]"')
        self.assertEqual([d[:3] for d in a], [('0300', '10de', '2786')])
        self.assertEqual([d[:3] for d in b], [('0300', '10de', '2786')])


def xbel(items):
    out = ['<?xml version="1.0" encoding="UTF-8"?>', '<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks">']
    for path, when in items:
        out.append('  <bookmark href="file://%s" added="%s" modified="%s" visited="%s">' % (path, when, when, when))
        out.append('    <info><metadata owner="http://freedesktop.org"><bookmark:applications><bookmark:application name="Text" exec="hearth-text %%u" modified="%s" count="1"/></bookmark:applications></metadata></info>' % when)
        out.append('  </bookmark>')
    out.append('</xbel>')
    return '\n'.join(out) + '\n'


class RecentPerActivity(Base):
    """Files opened at 10:00 on Main, 11:00 on Work, 12:00 on Private (Don't remember)."""
    T = {'main': 1794477600, 'work': 1794481200, 'private': 1794484800}     # 2026-11-12 10:00, 11:00, 12:00 UTC

    def setUp(self):
        super().setUp()
        self.home = self.t
        for n in ('main.txt', 'work.txt', 'private.txt'):
            (self.home / n).write_text(n)
        self.data = self.t / 'data'
        self.data.mkdir()
        (self.data / 'recently-used.xbel').write_text(xbel([
            (str(self.home / 'main.txt'), '2026-11-12T10:00:00Z'), (str(self.home / 'work.txt'), '2026-11-12T11:00:00Z'),
            (str(self.home / 'private.txt'), '2026-11-12T12:00:00Z')]))
        cfg = self.t / 'config' / 'nimbus'
        cfg.mkdir(parents=True)
        self.acts = cfg / 'activities.json'
        self.set_acts('work', {'work': 'own', 'private': 'own'})
        (self.data / 'hearth').mkdir()
        (self.data / 'hearth' / 'activity-history.json').write_text(json.dumps({'spans': [
            [self.T['work'] - 60, 'work'], [self.T['private'] - 60, 'private'], [self.T['private'] + 600, 'work']]}))

    def set_acts(self, current, modes):
        d = {'current': current, 'act:default': {'name': 'Main'}}
        for a, m in modes.items():
            d['act:' + a] = {'name': a.title(), 'recent': m}
        self.acts.write_text(json.dumps(d))

    def names(self):
        code, out, _ = self.run_h('recent', helper='hearth-files')
        self.assertEqual(code, 0, out)
        return sorted(e['name'] for e in out['recent'])

    def test_each_activity_its_own_list(self):
        self.assertEqual(self.names(), ['work.txt'])
        self.set_acts('default', {'work': 'own', 'private': 'own'})
        self.assertEqual(self.names(), ['main.txt'])                  # before the first switch: Main's
        self.set_acts('work', {'work': 'all', 'private': 'own'})
        self.assertEqual(self.names(), ['main.txt', 'private.txt', 'work.txt'])

    def test_the_app_menu_follows_too(self):
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-appmenu'), 'recent', '20'], env=dict(os.environ, **self.env),
                           capture_output=True, text=True, timeout=60)
        self.assertEqual([x['name'] for x in json.loads(r.stdout.strip().splitlines()[-1])], ['work.txt'])

    def test_dont_remember_forgets_only_what_was_opened_on_it(self):
        self.set_acts('work', {'work': 'own', 'private': 'off'})
        code, out, _ = self.run_h('activity-tidy', helper='hearth-files')
        self.assertEqual((code, out['forgot']), (0, 1), out)
        text = (self.data / 'recently-used.xbel').read_text()
        self.assertNotIn('private.txt', text)
        self.assertIn('main.txt', text)
        self.assertIn('work.txt', text)
        self.assertTrue((self.data / 'recently-used.xbel.bak').exists())
        self.assertTrue((self.home / 'private.txt').exists())        # only the list: the file stays

    def test_mark_notes_a_switch_once_and_forget_one_activity(self):
        code, out, _ = self.run_h('activity-mark', 'default', helper='hearth-files')
        self.assertEqual(code, 0, out)
        self.run_h('activity-mark', 'default', helper='hearth-files')
        spans = json.loads((self.data / 'hearth' / 'activity-history.json').read_text())['spans']
        self.assertEqual([s[1] for s in spans], ['work', 'private', 'work', 'default'])
        self.assertTrue((self.data / 'hearth' / 'activity-history.json.bak').exists())
        code, out, _ = self.run_h('recent-forget', 'activity:work', helper='hearth-files')
        self.assertEqual((code, out['forgot']), (0, 1), out)
        code, out, _ = self.run_h('activity-mark', '../x', helper='hearth-files')
        self.assertNotEqual(code, 0)

    def test_privacy_choices_never_forget_other_activities_files(self):
        # Changing the global Recent files choice keeps every activity's files (lists filter; only "off" forgets).
        code, out, _ = self.run_h('recent-prefs-set', json.dumps({'keepDays': 0}), helper='hearth-files')
        self.assertEqual((code, out['forgot']), (0, 0), out)
        code, out, _ = self.run_h('recent-prefs', helper='hearth-files')
        by = {a['id']: a for a in out['activities']}
        self.assertEqual((by['default']['files'], by['work']['files'], by['private']['files']), (1, 1, 1))
        self.assertEqual(by['work']['name'], 'Work')

    def test_one_activity_shows_everything_as_before(self):
        self.acts.write_text(json.dumps({'current': 'default'}))
        self.assertEqual(self.names(), ['main.txt', 'private.txt', 'work.txt'])
        self.acts.write_text('{broken')
        self.assertEqual(self.names(), ['main.txt', 'private.txt', 'work.txt'])


@unittest.skipUnless(NODE, 'node is needed for the activities rules')
class ActivityRecentRule(unittest.TestCase):
    def test_recent_mode_defaults_to_own(self):
        src = (SHELL / 'activities.js').read_text().replace('.pragma library', '')
        body = 'console.log(JSON.stringify(parse({"act:w": {name: "W", recent: "off"}, "act:g": {name: "G", recent: "weird"}}).list.map(function (a) { return [a.id, a.recent]; })));'
        r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr)
        got = dict(json.loads(r.stdout.strip().splitlines()[-1]))
        self.assertEqual(got, {'default': 'own', 'w': 'off', 'g': 'own'})


class Wiring(unittest.TestCase):
    """The pages use what the helpers give, and the notice is loaded on its own (a problem there loses only it)."""

    def test_pages_and_shell(self):
        shell = (SHELL / 'shell.qml').read_text()
        self.assertIn('Loader { source: "StoreWatch.qml"', shell)
        store = (SHELL / 'SetStore.qml').read_text()
        for s in ('"prefs-set", "notifyUpdates"', '"--allow-new"', 'source: "StoreDrivers.qml"', 'a === "drivers"'):
            self.assertIn(s, store)
        watch = (SHELL / 'StoreWatch.qml').read_text()
        self.assertIn('--action=update=Update all', watch)
        self.assertIn('["python3", sw.helper, "update"]', watch)       # Update all: never one asking for new permissions
        self.assertNotIn('--allow-new', watch)
        sw = (SHELL / 'ActivitySwitcher.qml').read_text()
        self.assertIn('Activities.setRecent(sw.sel.id, A.RECENT[Math.min(sw.editCol, A.RECENT.length - 1)])', sw)
        rf = (SHELL / 'SetRecentFiles.qml').read_text()
        self.assertIn('"activity:" + modelData.id', rf)
        act = (SHELL / 'Activities.qml').read_text()
        self.assertIn('"activity-mark", current', act)


if __name__ == '__main__':
    unittest.main()
