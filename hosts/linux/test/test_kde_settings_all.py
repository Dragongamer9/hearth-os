"""Tests for Hearth Settings' pages for every KDE System Settings module (hearth-kde-settings, kde-settings.json,
cats/kde.rs; docs/mockups/checklists/settings-kde-all.md): only the keys a page lists can be changed, only the keys
changed are written (the rest of the file kept, a .bak made), a file that can't be read is never written over, opening
(get) writes nothing, KDE's tools only run with an installed choice, and the spec, the Rust pages and the checklist
agree. Without KDE (no session bus, no plasma-apply-*)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
SPEC = SHELL / 'kde-settings.json'
RUST = ROOT / 'hosts' / 'hearth-ui' / 'apps' / 'settings' / 'src' / 'cats' / 'kde.rs'
CHECKLIST = ROOT / 'docs' / 'mockups' / 'checklists' / 'settings-kde-all.md'
sys.path.insert(0, str(SHELL))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class KdeSettings(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = pathlib.Path(self.tmp.name)
        self.cfg = d / 'cfg'
        self.cfg.mkdir()
        env = {'XDG_CONFIG_HOME': str(self.cfg), 'XDG_CONFIG_DIRS': str(d / 'sys'), 'XDG_RUNTIME_DIR': str(d),
               'XDG_STATE_HOME': str(d / 'state'), 'HOME': str(d), 'DBUS_SESSION_BUS_ADDRESS': ''}
        self.env = mock.patch.dict(os.environ, env)
        self.env.start()
        os.environ.pop('HEARTH_KDE_FIXTURE', None)
        self.h = load(SHELL / 'hearth-kde-settings', 'hearth_kde_settings_t')

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def test_only_listed_keys_change_and_the_rest_of_the_file_stays(self):
        p = self.cfg / 'kdeglobals'
        p.write_text('[KDE]\nSingleClick=true\n# mine\nName[de]=Hallo\n\n[Colors:View]\nBackgroundNormal=1,2,3\n')
        out = self.h.set_file('kdeglobals', json.dumps({'KDE': {'DoubleClickInterval': '600'}}))
        self.assertTrue(out['changed'])
        text = p.read_text()
        self.assertIn('DoubleClickInterval=600', text)
        for keep in ('SingleClick=true', '# mine', 'Name[de]=Hallo', '[Colors:View]', 'BackgroundNormal=1,2,3'):
            self.assertIn(keep, text)
        self.assertTrue((self.cfg / 'kdeglobals.bak').exists(), 'the last good copy is kept')
        with self.assertRaises(self.h.Fail):
            self.h.set_file('kdeglobals', json.dumps({'KDE': {'NotAPageSetting': '1'}}))
        with self.assertRaises(self.h.Fail):
            self.h.set_file('passwd', json.dumps({'x': {'y': 1}}))
        with self.assertRaises(self.h.Fail):
            self.h.set_file('kdeglobals', json.dumps({'KDE': {'SingleClick': {'nested': 1}}}))

    def test_a_choices_other_keys_are_allowed_with_it(self):
        allow = self.h.allowlist()
        self.assertIn('ButtonsOnLeft', allow['kwinrc']['org.kde.kdecoration2'])
        self.assertIn('theme', allow['kwinrc']['org.kde.kdecoration2'], 'the frame theme writes library and theme')
        # A choice KDE applies with its own tool isn't written directly.
        self.assertNotIn('ColorScheme', allow.get('kdeglobals', {}).get('General', set()))
        self.assertNotIn('LookAndFeelPackage', allow.get('kdeglobals', {}).get('KDE', set()))

    def test_a_file_that_cant_be_read_is_never_written_over(self):
        p = self.cfg / 'kwinrc'
        bad = b'[org.kde.kdecoration2\nlibrary=x\x00'
        p.write_bytes(bad)
        with self.assertRaises(self.h.Fail):
            self.h.set_file('kwinrc', json.dumps({'org.kde.kdecoration2': {'ShowToolTips': False}}))
        self.assertEqual(p.read_bytes(), bad)
        got = self.h.get('kde-decorations')
        self.assertIn('kwinrc', got['bad'])

    def test_opening_every_page_writes_nothing(self):
        (self.cfg / 'kscreenlockerrc').write_text('[Daemon]\nTimeout=10\n')
        before = sorted((f.name, f.read_bytes()) for f in self.cfg.iterdir())
        for p in json.loads(SPEC.read_text())['pages']:
            out = self.h.get(p['id'])
            self.assertEqual(out['page'], p['id'])
        after = sorted((f.name, f.read_bytes()) for f in self.cfg.iterdir())
        self.assertEqual(before, after)

    def test_kdes_tools_run_only_for_an_installed_choice(self):
        with mock.patch.object(self.h, 'list_kind', return_value=[['BreezeDark', 'Breeze Dark']]), mock.patch.object(self.h, 'run') as run, \
                mock.patch.object(self.h.shutil, 'which', return_value='/usr/bin/plasma-apply-colorscheme'):
            run.return_value = (0, '', '')
            with self.assertRaises(self.h.Fail):
                self.h.apply_tool('colorscheme', 'NotInstalled; rm -rf /')
            run.assert_not_called()
            self.h.apply_tool('colorscheme', 'BreezeDark')
            self.assertEqual(run.call_args[0][0], ['plasma-apply-colorscheme', 'BreezeDark'])
        with self.assertRaises(self.h.Fail):
            self.h.apply_tool('sh', 'x')

    def test_locked_keys_arent_changed(self):
        sysd = pathlib.Path(os.environ['XDG_CONFIG_DIRS'])
        sysd.mkdir()
        (sysd / 'kscreenlockerrc').write_text('[Daemon]\nAutolock[$i]=false\n')
        with self.assertRaises(self.h.Fail) as e:
            self.h.set_file('kscreenlockerrc', json.dumps({'Daemon': {'Autolock': True}}))
        self.assertIn('locked', str(e.exception))
        self.assertFalse((self.cfg / 'kscreenlockerrc').exists())

    def test_a_password_goes_only_to_the_pretend_wallet(self):
        import io
        secret = 'Sw0rdfish-not-a-real-one'
        calls = []

        def fake_wallet(method, sig, args):
            calls.append((method, args))
            return {'open': 7, 'hasFolder': False, 'createFolder': True, 'writePassword': 0, 'close': 0, 'networkWallet': 'kdewallet'}.get(method)
        with mock.patch.object(self.h, 'wallet_call', side_effect=fake_wallet), mock.patch.object(self.h.sys, 'stdin', io.StringIO(secret + '\n')):
            out = self.h.rdp_user_add('dad')
        self.assertEqual(out['users'], ['dad'])
        self.assertIn(('writePassword', (7, 'KRDP', 'dad', secret, 'Hearth Settings')), calls)
        # Nowhere on disk: not the config, not Hearth's log, not a backup.
        for root, _, files in os.walk(self.tmp.name):
            for f in files:
                with open(os.path.join(root, f), 'rb') as fh:
                    self.assertNotIn(secret.encode(), fh.read(), 'the password was written to %s' % f)
        # Not from the environment or the command line: only stdin.
        with mock.patch.dict(os.environ, {'HEARTH_INPUT': secret}), mock.patch.object(self.h.sys, 'stdin', io.StringIO('')):
            with self.assertRaises(self.h.Fail):
                self.h.rdp_user_add('mum')

    def test_your_commands_and_device_actions(self):
        with mock.patch.object(self.h, 'set_keys') as keys:
            out = self.h.command_set(json.dumps({'name': 'Notes', 'exec': 'kate ~/notes.txt', 'keys': 'Meta+N'}))
        self.assertEqual([c['name'] for c in out], ['Notes'])
        keys.assert_called_once()
        self.assertEqual(keys.call_args[0][1], 'Meta+N')
        f = out[0]['file']
        self.assertTrue(f.startswith('net.local.kate'))
        with self.assertRaises(self.h.Fail):
            self.h.command_set(json.dumps({'name': 'x', 'exec': 'y', 'keys': 'Hyper+Q'}))
        with mock.patch.object(self.h, 'set_keys'):
            self.assertEqual(self.h.command_remove(f), [])
        # A system device action changed as your copy; the system's file untouched; reset brings it back.
        sysd = pathlib.Path(self.tmp.name) / 'sysdata' / 'solid' / 'actions'
        sysd.mkdir(parents=True)
        orig = '[Desktop Entry]\nX-KDE-Solid-Predicate=IS Camera\nActions=get;\n\n[Desktop Action get]\nName=Get photos\nExec=gwenview_importer %f\n'
        (sysd / 'cam.desktop').write_text(orig)
        with mock.patch.dict(os.environ, {'XDG_DATA_DIRS': str(sysd.parent.parent), 'XDG_DATA_HOME': str(pathlib.Path(self.tmp.name) / 'data')}):
            out = self.h.device_action_edit(json.dumps({'file': 'cam.desktop', 'action': 'get', 'name': 'Copy photos', 'exec': 'rsync -a %f ~/Pictures'}))
            self.assertEqual(out[0]['name'], 'Copy photos')
            self.assertTrue(out[0]['changed'])
            self.assertEqual((sysd / 'cam.desktop').read_text(), orig)
            out = self.h.device_action_reset('cam.desktop')
            self.assertEqual(out[0]['name'], 'Get photos')

    def test_store_downloads_stay_in_their_folder(self):
        import zipfile
        d = pathlib.Path(self.tmp.name)
        z = d / 'bad.zip'
        with zipfile.ZipFile(z, 'w') as zf:
            zf.writestr('../../evil.txt', 'x')
        with self.assertRaises(self.h.Fail):
            self.h.safe_extract(str(z), str(d / 'out'))
        self.assertFalse((d / 'evil.txt').exists())
        with self.assertRaises(self.h.Fail):
            self.h.store_install('lookandfeel', '1; rm -rf ~')

    def test_the_spec_the_rust_pages_and_the_checklist_agree(self):
        spec = json.loads(SPEC.read_text())
        ids = [p['id'] for p in spec['pages']]
        rust = RUST.read_text()
        m = re.search(r'pub const PAGES: &\[&str\] = &\[(.*?)\];', rust, re.S)
        self.assertEqual(re.findall(r'"([a-z0-9-]+)"', m.group(1)), ids)
        pages_list = (ROOT / 'hosts/hearth-ui/apps/settings/src/pages_list.rs').read_text()
        js = (SHELL / 'settings-pages.js').read_text()
        for p in spec['pages']:
            self.assertIn('id: "%s", cat: "%s", label: %s' % (p['id'], p['cat'], json.dumps(p['label'], ensure_ascii=False)), pages_list,
                          'run tools/settings-registry-rs.py')
            self.assertNotIn('id: "%s"' % p['id'], js, "the KDE pages are the Rust app's, not Mantel's")
            for k in ('cat', 'label', 'desc', 'words', 'kcm', 'rows'):
                self.assertIn(k, p, '%s has no %s' % (p['id'], k))
        checklist = CHECKLIST.read_text()
        self.assertIn('Release: 1.16', checklist)
        for p in spec['pages']:
            for kcm in p['kcm']:
                self.assertIn(kcm.split(' ')[0], checklist, '%s (%s) is in the checklist' % (kcm, p['id']))


if __name__ == '__main__':
    unittest.main()
