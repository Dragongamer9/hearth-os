"""Tests for more of KDE System Settings in Hearth Settings (1.14, docs/SETTINGS-MORE-1.14.md): Sound (balance, speaker
test, push to talk), Power (lid, apps keeping it awake, hibernate, battery history), Autostart, File types, the App
menu editor, Look (fonts, frame tint), Region (languages, spell check, fcitx5), App permissions, Bluetooth, the print
queue and scanners, USB devices, Info Center, and that every new page is registered and every new setting is saved
only by its own key. Programs are stand-ins; nothing here touches the real system."""
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
import wave
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
THEME = ROOT / 'hosts' / 'linux' / 'desktop' / 'theme'
NIMBUS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus'


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Home(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.home = self.tmp.name
        self.env = mock.patch.dict(os.environ, {'HOME': self.home, 'XDG_CONFIG_HOME': os.path.join(self.home, '.config'),
                                                'XDG_DATA_HOME': os.path.join(self.home, '.local', 'share'),
                                                'XDG_STATE_HOME': os.path.join(self.home, '.local', 'state'),
                                                'XDG_RUNTIME_DIR': self.home})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def read(self, *p):
        with open(os.path.join(self.home, *p)) as f:
            return f.read()


SINKS = [{'name': 'alsa_output.pci.analog-stereo', 'description': 'Speakers', 'channel_map': 'front-left,front-right',
          'volume': {'front-left': {'value': 65536}, 'front-right': {'value': 32768}}, 'mute': False},
         {'name': 'alsa_output.surround', 'description': 'Surround', 'channel_map': 'front-left,front-right,front-center,lfe,rear-left,rear-right',
          'volume': {c: {'value': 52428} for c in ('front-left', 'front-right', 'front-center', 'lfe', 'rear-left', 'rear-right')}},
         {'name': 'bad name; rm -rf', 'channel_map': 'mono'}]


class Sound(Home):
    def setUp(self):
        super().setUp()
        self.s = load(SHELL / 'hearth-sound', 'hearth_sound_more')

    def test_outputs_and_their_balance(self):
        outs = self.s.parse_outputs(SINKS, 'alsa_output.surround')
        self.assertEqual([o['name'] for o in outs], ['alsa_output.pci.analog-stereo', 'alsa_output.surround'])
        self.assertEqual(outs[0]['balance'], -50)           # the right side at half: towards the left
        self.assertEqual(outs[1]['balance'], 0)
        self.assertTrue(outs[1]['default'])
        self.assertEqual([c['label'] for c in outs[1]['channels']][:4], ['Front left', 'Front right', 'Centre', 'Subwoofer'])

    def test_balance_keeps_the_loudest_level_and_leaves_centre_alone(self):
        ch = self.s.parse_outputs(SINKS)[1]['channels']
        lv = self.s.balance_levels(ch, 40)
        self.assertAlmostEqual(lv[0], 0.8 * 0.6, places=3)  # left quieter
        self.assertAlmostEqual(lv[1], 0.8, places=3)        # right keeps the top
        self.assertAlmostEqual(lv[2], 0.8, places=3)        # centre unchanged
        self.assertEqual(self.s.balance_levels(ch, -100)[1], 0.0)

    def test_balance_runs_pactl_with_a_list(self):
        calls = []
        with mock.patch.object(self.s, 'outputs', return_value={'outputs': self.s.parse_outputs(SINKS)}), \
                mock.patch.object(self.s, 'run', side_effect=lambda c, timeout=20: (calls.append(c), (0, '', ''))[1]):
            self.s.balance('alsa_output.pci.analog-stereo', '0')
            with self.assertRaises(self.s.Fail):
                self.s.balance('alsa_output.pci.analog-stereo', '300')
            with self.assertRaises(self.s.Fail):
                self.s.balance('x;y', '0')
        self.assertEqual(calls, [['pactl', 'set-sink-volume', 'alsa_output.pci.analog-stereo', '65536', '65536']])

    def test_speaker_tone_is_only_in_its_channel(self):
        p = os.path.join(self.home, 't.wav')
        self.s.tone(p, 2, 1, seconds=0.1)
        with wave.open(p) as w:
            self.assertEqual(w.getnchannels(), 2)
            import array
            a = array.array('h', w.readframes(w.getnframes()))
        self.assertTrue(all(x == 0 for x in a[0::2]))
        self.assertTrue(any(x != 0 for x in a[1::2]))


class PushToTalk(unittest.TestCase):
    def test_keys_and_only_under_kwin(self):
        p = load(SHELL / 'hearth-ptt', 'hearth_ptt')
        self.assertIn('f13', p.KEYS)
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            with self.assertRaises(p.Fail):
                p.run('f13')
        with self.assertRaises(p.Fail):
            p.run('rm -rf')
        src = (SHELL / 'PushToTalk.qml').read_text()
        self.assertIn('Quickshell.env("HEARTH_WM") === "kwin"', src)
        self.assertIn('fails < 3', src)


class Power(Home):
    def setUp(self):
        super().setUp()
        self.p = load(SHELL / 'hearth-powerset', 'hearth_powerset')

    def test_lid_reads_logind_files_in_order(self):
        etc = os.path.join(self.home, 'etc')
        os.makedirs(os.path.join(etc, 'systemd', 'logind.conf.d'))
        with open(os.path.join(etc, 'systemd', 'logind.conf'), 'w') as f:
            f.write('[Login]\n#HandleLidSwitch=suspend\nHandleLidSwitchDocked=suspend\n')
        with open(os.path.join(etc, 'systemd', 'logind.conf.d', '60-hearth-lid.conf'), 'w') as f:
            f.write('[Login]\nHandleLidSwitch=lock\n')
        with mock.patch('glob.glob', side_effect=lambda pat: [os.path.join(etc, 'systemd', 'logind.conf.d', '60-hearth-lid.conf')] if pat.startswith(etc) else []):
            l = self.p.lid(etc)
        self.assertEqual((l['battery'], l['ac'], l['docked']), ('lock', 'lock', 'suspend'))
        self.assertEqual(l['setBy'], ['logind.conf'])

    def test_inhibitors_skip_system_ones(self):
        js = {'type': 'a(ssssuu)', 'data': [[['sleep:idle', 'Firefox', 'Playing video', 'block', 1000, 4242],
                                             ['shutdown:sleep', 'NetworkManager', 'NM', 'delay', 0, 10],
                                             ['handle-power-key', 'Hearth', 'keys', 'block', 1000, 5]]]}
        out = self.p.parse_inhibitors(js)
        self.assertEqual([i['who'] for i in out], ['Firefox'])
        self.assertEqual(out[0]['what'], ['sleep', 'idle'])

    def test_hibernate_is_offered_only_when_everything_allows_it(self):
        sysd, proc = os.path.join(self.home, 'sys'), os.path.join(self.home, 'proc')
        os.makedirs(os.path.join(sysd, 'power'))
        os.makedirs(os.path.join(sysd, 'kernel', 'security'))
        os.makedirs(proc)

        def put(path, text):
            with open(path, 'w') as f:
                f.write(text)
        put(os.path.join(proc, 'meminfo'), 'MemTotal: 16000000 kB\nMemAvailable: 8000000 kB\n')
        put(os.path.join(sysd, 'power', 'state'), 'freeze mem\n')
        self.assertIn('firmware', self.p.hibernate_check(sysd, proc, can='yes')['why'])
        put(os.path.join(sysd, 'power', 'state'), 'freeze mem disk\n')
        put(os.path.join(sysd, 'kernel', 'security', 'lockdown'), 'none [integrity] confidentiality\n')
        self.assertIn('Secure Boot', self.p.hibernate_check(sysd, proc, can='yes')['why'])
        put(os.path.join(sysd, 'kernel', 'security', 'lockdown'), '[none] integrity confidentiality\n')
        put(os.path.join(proc, 'swaps'), 'Filename Type Size Used Priority\n/dev/zram0 partition 8000000 0 100\n')
        self.assertIn('swap file', self.p.hibernate_check(sysd, proc, can='yes')['why'])      # zram doesn't count
        put(os.path.join(proc, 'swaps'), 'Filename Type Size Used Priority\n/var/swap/hearth-hibernate file 4000000 0 -2\n')
        self.assertIn('too small', self.p.hibernate_check(sysd, proc, can='yes')['why'])
        put(os.path.join(proc, 'swaps'), 'Filename Type Size Used Priority\n/var/swap/hearth-hibernate file 17000000 0 -2\n')
        self.assertFalse(self.p.hibernate_check(sysd, proc, can='na')['can'])
        self.assertTrue(self.p.hibernate_check(sysd, proc, can='yes')['can'])

    def test_battery_history_and_upower(self):
        pts = self.p.parse_history('{"type":"a(udu)","data":[[[1700000600,55.0,2],[1700000000,60.5,2],[0,1,1]]]}')
        self.assertEqual(pts, [[1700000000, 60.5, 2], [1700000600, 55.0, 2]])
        d = self.p.parse_upower('  native-path:          BAT0\n  energy-full:          52.0 Wh\n  energy-full-design:   57 Wh\n  charge-cycles:        212\n')
        self.assertEqual((self.p.num(d['energy-full']), d['charge-cycles']), (52.0, '212'))
        with self.assertRaises(self.p.Fail):
            self.p.history('/org/freedesktop/UPower/devices/battery_BAT0; rm', 'charge', '24')
        with self.assertRaises(self.p.Fail):
            self.p.history('/org/freedesktop/UPower/devices/battery_BAT0', 'charge', '9999')


class LidAdmin(Home):
    def setUp(self):
        super().setUp()
        self.a = load(NIMBUS / 'hearth-sysset-admin', 'hearth_sysset_admin_more')

    def test_lid_file_only_and_hibernate_only_when_it_can(self):
        d = os.path.join(self.home, 'logind.conf.d')
        r = self.a.set_lid('suspend', 'ignore', 'ignore', directory=d, hib=False)
        self.assertTrue(r['changed'])
        text = open(os.path.join(d, '60-hearth-lid.conf')).read()
        self.assertIn('HandleLidSwitchExternalPower=ignore', text)
        self.assertEqual(os.listdir(d), ['60-hearth-lid.conf'])
        self.assertFalse(self.a.set_lid('suspend', 'ignore', 'ignore', directory=d, hib=False)['changed'])
        with self.assertRaises(ValueError):
            self.a.set_lid('hibernate', 'ignore', 'ignore', directory=d, hib=False)
        with self.assertRaises(ValueError):
            self.a.set_lid('reboot', 'ignore', 'ignore', directory=d, hib=True)
        self.a.set_lid('lock', 'lock', 'ignore', directory=d, hib=False)
        self.assertTrue(os.path.exists(os.path.join(d, '60-hearth-lid.conf.bak')))
        src = (NIMBUS / 'hearth-sysset-admin').read_text()
        self.assertIn("'reload', 'systemd-logind'", src)
        self.assertNotIn("'restart', 'systemd-logind'", src)
        self.assertIn("'lid'", (NIMBUS / 'hearth-admin').read_text())


class Autostart(Home):
    def setUp(self):
        super().setUp()
        self.a = load(SHELL / 'hearth-autostart', 'hearth_autostart')
        self.t = self.a.tasks()
        self.sysd = os.path.join(self.home, 'xdg', 'autostart')
        os.makedirs(self.sysd)
        self.user = os.path.join(self.home, '.config', 'autostart')
        self.patch = mock.patch.object(self.t, 'autostart_dirs', return_value=(self.user, [self.sysd]))
        self.patch.start()

    def tearDown(self):
        self.patch.stop()
        super().tearDown()

    def test_a_command_survives_the_round_trip(self):
        cmd = 'notify-send "Hi $USER" 100% `date` \\o/'
        r = self.a.add_command({'name': 'Say hi', 'command': cmd}, self.t)
        text = open(os.path.join(self.user, r['file'])).read()
        self.assertIn('X-Hearth-Autostart=true', text)
        argv = self.t.autostart_argv(self.t.parse_desktop(text)['Exec'])
        self.assertEqual(argv, ['sh', '-c', cmd])
        with self.assertRaises(self.a.Fail):
            self.a.add_command({'name': 'x', 'command': 'a\nb'}, self.t)
        items = self.a.items(self.t)['items']
        self.assertEqual(items[0]['name'], 'Say hi')
        self.assertTrue(items[0]['yours'] and items[0]['added'])
        self.a.remove(r['file'], self.t)
        self.assertEqual(self.a.items(self.t)['items'], [])

    def test_a_system_one_is_never_deleted(self):
        with open(os.path.join(self.sysd, 'sync.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Sync\nExec=syncthing\n')
        with self.assertRaises(self.a.Fail):
            self.a.remove('sync.desktop', self.t)
        self.a.set_on('sync.desktop', 'off', self.t)
        self.assertTrue(os.path.exists(os.path.join(self.sysd, 'sync.desktop')))
        self.assertIn('Hidden=true', open(os.path.join(self.user, 'sync.desktop')).read())


class FileTypes(Home):
    def setUp(self):
        super().setUp()
        self.m = load(SHELL / 'hearth-mime', 'hearth_mime')
        apps = os.path.join(self.home, '.local', 'share', 'applications')
        os.makedirs(apps)
        for name, mt in (('viewer', 'image/png;image/jpeg;'), ('paint', 'image/png;')):
            with open(os.path.join(apps, name + '.desktop'), 'w') as f:
                f.write('[Desktop Entry]\nType=Application\nName=%s\nExec=%s %%f\nMimeType=%s\n' % (name.title(), name, mt))
        self.dd = mock.patch.dict(os.environ, {'XDG_DATA_DIRS': os.path.join(self.home, 'none'), 'XDG_CONFIG_DIRS': os.path.join(self.home, 'none')})
        self.dd.start()

    def tearDown(self):
        self.dd.stop()
        super().tearDown()

    def test_types_set_and_reset(self):
        t = {x['mime']: x for x in self.m.types()['types']}
        self.assertEqual(sorted(t), ['image/jpeg', 'image/png'])
        self.assertEqual([a['id'] for a in t['image/png']['apps']], ['paint.desktop', 'viewer.desktop'])
        ml = os.path.join(self.home, '.config', 'mimeapps.list')
        os.makedirs(os.path.dirname(ml))
        with open(ml, 'w') as f:
            f.write('# mine\n[Default Applications]\ntext/plain=gedit.desktop\n')
        self.m.set_default('image/png', 'viewer.desktop')
        text = open(ml).read()
        self.assertIn('# mine', text)
        self.assertIn('text/plain=gedit.desktop', text)
        self.assertIn('image/png=viewer.desktop', text)
        t = {x['mime']: x for x in self.m.types()['types']}
        self.assertTrue(t['image/png']['yours'])
        self.m.reset('image/png')
        self.assertNotIn('image/png=', open(ml).read().split('[Added Associations]')[0])   # your default gone
        self.assertFalse({x['mime']: x for x in self.m.types()['types']}['image/png']['yours'])
        with self.assertRaises(self.m.Fail):
            self.m.set_default('image/png', 'missing.desktop')
        with self.assertRaises(self.m.Fail):
            self.m.set_default('../x', 'viewer.desktop')

    def test_own_endings_file(self):
        with mock.patch.object(self.m, 'write_globs', side_effect=lambda t: open(self.m.user_xml(), 'w').write(self.m.xml_for(t)) if os.makedirs(os.path.dirname(self.m.user_xml()), exist_ok=True) is None else None):
            self.m.add_glob('image/png', '*.pngx')
            self.assertEqual(self.m.own_globs(), {'image/png': ['*.pngx']})
            with self.assertRaises(self.m.Fail):
                self.m.add_glob('image/png', '*.a b')
            self.m.remove_glob('image/png', '*.pngx')
            self.assertEqual(self.m.own_globs(), {})


class MenuEditor(Home):
    def setUp(self):
        super().setUp()
        self.m = load(SHELL / 'hearth-menu', 'hearth_menu')
        self.sysapps = os.path.join(self.home, 'usr', 'applications')
        os.makedirs(self.sysapps)
        with open(os.path.join(self.sysapps, 'calc.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Calculator\nName[de]=Rechner\nExec=calc\nCategories=Utility;Math;\n\n[Desktop Action new]\nName=New\nExec=calc --new\n')
        self.dd = mock.patch.dict(os.environ, {'XDG_DATA_DIRS': os.path.join(self.home, 'usr')})
        self.dd.start()

    def tearDown(self):
        self.dd.stop()
        super().tearDown()

    def test_save_makes_your_copy_and_reset_puts_it_back(self):
        self.m.save('calc.desktop', {'name': 'Sums', 'hidden': True, 'category': 'Office'})
        mine = self.read('.local', 'share', 'applications', 'calc.desktop')
        self.assertIn('Name=Sums', mine)
        self.assertNotIn('Name[de]', mine)                    # the new name in every language
        self.assertIn('NoDisplay=true', mine)
        self.assertIn('Categories=Office;Math;', mine)
        self.assertIn('[Desktop Action new]', mine)           # its actions kept
        self.assertIn('X-Hearth-Edited=true', mine)
        self.assertIn('Name=Calculator', open(os.path.join(self.sysapps, 'calc.desktop')).read())
        a = {x['id']: x for x in self.m.listing()['apps']}['calc.desktop']
        self.assertTrue(a['yours'] and a['hidden'])
        self.m.reset('calc.desktop')
        self.assertFalse(os.path.exists(os.path.join(self.home, '.local', 'share', 'applications', 'calc.desktop')))
        with self.assertRaises(self.m.Fail):
            self.m.reset('calc.desktop')

    def test_your_own_entry_stays_yours(self):
        apps = os.path.join(self.home, '.local', 'share', 'applications')
        os.makedirs(apps)
        with open(os.path.join(apps, 'mine.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=Mine\nExec=mine\n')
        self.m.save('mine.desktop', {'name': 'Still mine'})
        text = open(os.path.join(apps, 'mine.desktop')).read()
        self.assertIn('Name=Still mine', text)
        self.assertNotIn('X-Hearth-Edited', text)
        with self.assertRaises(self.m.Fail):
            self.m.reset('mine.desktop')
        r = self.m.new({'name': 'Backup', 'exec': 'sh ~/b.sh'})
        self.assertTrue(r['id'].startswith('hearth-menu-backup'))
        with self.assertRaises(self.m.Fail):
            self.m.new({'name': 'x'})
        with self.assertRaises(self.m.Fail):
            self.m.save('calc.desktop', {'exec': 'a\nb'})


class LookAndFonts(Home):
    def test_small_and_title_fonts(self):
        ls = load(SHELL / 'hearth-lookset', 'hearth_lookset_more')
        c = ls.clean({'smallFont': 'Inter', 'smallSize': 7.5, 'titleSize': 11})
        self.assertEqual(c, {'smallFont': 'Inter', 'smallSize': 7.5, 'titleSize': 11})
        with self.assertRaises(ls.Fail):
            ls.clean({'titleFont': 'Bad"Name'})
        e = ls.effective({'titleFont': 'Inter', 'titleSize': 11})
        cmds = ls.gsettings_cmds(e, {'titleFont': 'Inter', 'titleSize': 11})
        self.assertIn(['org.gnome.desktop.wm.preferences', 'titlebar-font', 'Inter Bold 11'], cmds)

    def test_frame_tint(self):
        sys.path.insert(0, str(THEME))
        try:
            import hearth_aurorae as ha
        finally:
            sys.path.remove(str(THEME))
        text = 'window.active.title.bg.color: #202020\nwindow.active.label.text.color: #ffffff\nwindow.inactive.border.color: #101010\n'
        self.assertEqual(ha.tint_themerc(text, '#ff8a3d', 0), text)
        out = ha.tint_themerc(text, '#ff8a3d', 100)
        self.assertNotIn('#202020', out)
        self.assertIn('window.active.label.text.color: #ffffff', out)   # only frame keys change
        self.assertEqual(ha.tint_colors({'window.active.title.bg.color': '#000000'}, '#ffffff', 100)['window.active.title.bg.color'], '#8c8c8c')
        self.assertEqual(ha.tint_amount('x'), 0)
        self.assertEqual(ha.tint_amount(400), 100)
        src = (THEME / 'hearth-theme').read_text()
        self.assertIn("get('frameTint', 0)", src)


class Region(Home):
    def setUp(self):
        super().setUp()
        self.r = load(SHELL / 'hearth-region', 'hearth_region_more')
        cfg = os.path.join(self.home, '.config')
        for k, v in (('LANG_ENV', os.path.join(cfg, 'environment.d', '60-hearth-language.conf')), ('LOCALERC', os.path.join(cfg, 'plasma-localerc')),
                     ('SONNET', os.path.join(cfg, 'KDE', 'Sonnet.conf')), ('KWINRC', os.path.join(cfg, 'kwinrc')),
                     ('IM_ENV', os.path.join(cfg, 'environment.d', '60-hearth-input-method.conf'))):
            setattr(self.r, k, v)

    def test_languages_in_order(self):
        self.r.languages_set({'languages': ['de_DE', 'fr', 'de_DE', 'en_US']})
        self.assertEqual(self.r.languages()['languages'], ['de_DE', 'fr', 'en_US'])
        self.assertIn('LANGUAGE=de_DE:fr:en_US', self.read('.config', 'plasma-localerc'))
        with self.assertRaises(self.r.Fail):
            self.r.languages_set({'languages': ['de;rm']})
        self.r.languages_set({'languages': []})
        self.assertFalse(os.path.exists(self.r.LANG_ENV))
        self.assertNotIn('LANGUAGE=', self.read('.config', 'plasma-localerc'))

    def test_spell_keeps_other_lines(self):
        os.makedirs(os.path.dirname(self.r.SONNET))
        with open(self.r.SONNET, 'w') as f:
            f.write('[General]\ndefaultClient=Hunspell\n\n[Other]\nx=1\n')
        st = self.r.spell_set({'backgroundCheckerEnabled': False, 'defaultLanguage': 'en_GB', 'ignore': ['Hearth', 'KWin']})
        text = open(self.r.SONNET).read()
        self.assertIn('defaultClient=Hunspell', text)
        self.assertIn('[Other]\nx=1', text)
        self.assertIn('ignore_en_GB=Hearth,KWin', text)
        self.assertFalse(st['settings']['backgroundCheckerEnabled'])
        self.assertEqual(st['settings']['ignore'], ['Hearth', 'KWin'])
        with self.assertRaises(self.r.Fail):
            self.r.spell_set({'ignore': ['two words']})

    def test_fcitx_on_and_off_only_touch_its_key(self):
        os.makedirs(os.path.dirname(self.r.KWINRC), exist_ok=True)
        with open(self.r.KWINRC, 'w') as f:
            f.write('[Wayland]\nVirtualKeyboardEnabled=true\n\n[Windows]\nFocusPolicy=ClickToFocus\n')
        with mock.patch.object(self.r, 'fcitx_desktop', return_value=''):
            with self.assertRaises(self.r.Fail):
                self.r.im_set('on')
        with mock.patch.object(self.r, 'fcitx_desktop', return_value='/usr/share/applications/org.fcitx.Fcitx5.desktop'), \
                mock.patch.object(self.r, 'flatpak_has', return_value=False):
            self.assertTrue(self.r.im_set('on')['on'])
            self.assertIn('InputMethod=/usr/share/applications/org.fcitx.Fcitx5.desktop', open(self.r.KWINRC).read())
            self.assertIn('XMODIFIERS=@im=fcitx', open(self.r.IM_ENV).read())
            self.assertFalse(self.r.im_set('off')['on'])
        text = open(self.r.KWINRC).read()
        self.assertNotIn('InputMethod', text)
        self.assertIn('VirtualKeyboardEnabled=true', text)
        self.assertIn('FocusPolicy=ClickToFocus', text)
        self.assertFalse(os.path.exists(self.r.IM_ENV))


class FlatpakPerms(unittest.TestCase):
    def test_permissions_from_metadata_and_your_overrides(self):
        f = load(SHELL / 'hearth-flatpak-perms', 'hearth_flatpak_perms')
        meta = '[Application]\nname=org.example.App\n\n[Context]\nshared=network;ipc;\nsockets=x11;pulseaudio;\nfilesystems=xdg-download:ro;home;\nfeatures=bluetooth;\n'
        over = '[Context]\nshared=!network;\nfilesystems=host;\n'
        p = {x['key']: x for x in f.perms_of(meta, over)}
        self.assertTrue(p['network']['app'] and not p['network']['now'] and p['network']['changed'])
        self.assertTrue(p['host']['now'] and p['host']['changed'])
        self.assertTrue(p['xdg-download']['now'] and not p['xdg-download']['changed'])
        self.assertTrue(p['bluetooth']['app'])
        self.assertFalse(p['devices']['now'])
        with self.assertRaises(f.Fail):
            f.check_app('not-an-app; rm')
        self.assertEqual([a['id'] for a in f.parse_list('org.a.B\tB\tflathub\nbad\tX\norg.a.B\tB\tflathub\n')], ['org.a.B'])


class DevicesAndInfo(Home):
    def test_bluetooth_info(self):
        b = load(SHELL / 'hearth-bt', 'hearth_bt')
        d = b.parse_info('Device AA:BB:CC:DD:EE:FF (public)\n\tName: Pixel 8\n\tAlias: Pixel 8\n\tIcon: phone\n\tPaired: yes\n\tTrusted: no\n'
                         '\tBlocked: no\n\tConnected: yes\n\tUUID: OBEX Object Push          (00001105-0000-1000-8000-00805f9b34fb)\n\tBattery Percentage: 0x5a (90)\n')
        self.assertEqual((d['name'], d['battery'], d['canSend'], d['trusted'], d['connected']), ('Pixel 8', 90, True, False, True))
        with self.assertRaises(b.Fail):
            b.change('trust', 'AA:BB; rm')
        with self.assertRaises(b.Fail):
            b.send('AA:BB:CC:DD:EE:FF', os.path.join(self.home, 'missing.txt'))

    def test_print_queue_and_scanners(self):
        q = load(SHELL / 'hearth-printq', 'hearth_printq')
        jobs = q.parse_jobs('HP_LaserJet-12   alice   10240   Mon 16 Nov 2026 10:02:03 PM\nnoise\n', 'waiting')
        self.assertEqual((jobs[0]['id'], jobs[0]['printer'], jobs[0]['num'], jobs[0]['size']), ('HP_LaserJet-12', 'HP_LaserJet', 12, 10240))
        self.assertEqual(q.parse_titles('Rank    Owner   Job     File(s)                         Total Size\nactive  alice   12      report.pdf                      10240 bytes\n'), {12: ('active', 'report.pdf')})
        pr = q.parse_printers('printer HP_LaserJet now printing HP_LaserJet-12.  enabled since Mon\nprinter Brother is idle.  enabled since Mon\n')
        self.assertEqual([(p['name'], p['state']) for p in pr], [('HP_LaserJet', 'printing'), ('Brother', 'idle')])
        sc = q.parse_scanners("device `airscan:e0:HP OfficeJet' is a eSCL HP OfficeJet ip=192.168.1.20\ndevice `bad;dev' is x\n")
        self.assertEqual(sc, [{'device': 'airscan:e0:HP OfficeJet', 'name': 'eSCL HP OfficeJet ip=192.168.1.20'}])
        with self.assertRaises(q.Fail):
            q.job_cmd('cancel', 'x; rm')

    def test_usb_devices_from_sysfs(self):
        u = load(SHELL / 'hearth-usbdev', 'hearth_usbdev')
        root = os.path.join(self.home, 'sys', 'bus', 'usb', 'devices')

        def dev(name, vals, ifaces=()):
            d = os.path.join(root, name)
            os.makedirs(d)
            for k, v in vals.items():
                open(os.path.join(d, k), 'w').write(v + '\n')
            for i, (c, p) in enumerate(ifaces):
                ii = os.path.join(d, '%s:1.%d' % (name, i))
                os.makedirs(ii)
                open(os.path.join(ii, 'bInterfaceClass'), 'w').write(c)
                open(os.path.join(ii, 'bInterfaceProtocol'), 'w').write(p)
        dev('usb1', {'idVendor': '1d6b', 'idProduct': '0002', 'bDeviceClass': '09'})
        dev('1-1', {'idVendor': '046d', 'idProduct': 'c52b', 'product': 'USB Receiver', 'manufacturer': 'Logitech', 'bDeviceClass': '00', 'speed': '12'}, [('03', '01'), ('03', '02')])
        dev('1-2', {'idVendor': '0781', 'idProduct': '5581', 'product': 'Ultra', 'bDeviceClass': '00', 'authorized': '0'}, [('08', '50')])
        ds = u.devices(os.path.join(self.home, 'sys'))
        self.assertEqual([(d['name'], d['kind'], d['authorized']) for d in ds], [('USB Receiver', 'Keyboard', True), ('Ultra', 'Storage', False)])
        g = u.parse_guard('3: allow id 046d:c52b serial "" name "USB Receiver" hash "x"\n4: block id 0781:5581 serial "1" name "Ultra"\n')
        self.assertEqual([(x['id'], x['target']) for x in g], [('3', 'allow'), ('4', 'block')])
        with self.assertRaises(u.Fail):
            with mock.patch('shutil.which', return_value='/usr/bin/usbguard'):
                u.guard_cmd('allow', '4; rm')
        src = (SHELL / 'hearth-usbdev').read_text()
        self.assertNotIn("'block-device', '-p'", src)       # never blocks for good

    def test_info_center_parsers(self):
        i = load(SHELL / 'hearth-info', 'hearth_info')
        l = i.parse_lscpu({'lscpu': [{'field': 'Model name:', 'data': 'Ryzen 7'}, {'field': 'CPU(s):', 'data': '16', 'children': [{'field': 'L2 cache:', 'data': '8 MiB'}]}]})
        self.assertEqual((l['Model name'], l['CPU(s)'], l['L2 cache']), ('Ryzen 7', '16', '8 MiB'))
        v = i.parse_vmm('Slot:\t01:00.0\nClass:\tVGA compatible controller [0300]\nDevice:\tNavi 31 [744c]\nDriver:\tamdgpu\n\nSlot:\t00:1f.3\nClass:\tAudio device [0403]\n')
        self.assertEqual([(x['Slot'], x.get('Driver')) for x in v], [('01:00.0', 'amdgpu'), ('00:1f.3', None)])
        self.assertEqual(i.clean_nn('Navi 31 [744c]'), 'Navi 31')
        vk = i.parse_vulkan('Devices:\n========\nGPU0:\n\tapiVersion         = 1.3.280\n\tdeviceName         = AMD Radeon RX 7900\n\tdriverName         = radv\n')
        self.assertEqual(vk, [{'apiVersion': '1.3.280', 'deviceName': 'AMD Radeon RX 7900', 'driverName': 'radv'}])
        gl = i.parse_gl('OpenGL vendor string: AMD\nOpenGL renderer string: AMD Radeon RX 7900 (radeonsi)\nOpenGL core profile version string: 4.6\n')
        self.assertEqual(gl['OpenGL renderer string'], 'AMD Radeon RX 7900 (radeonsi)')


class Pages(unittest.TestCase):
    """Every new page is registered with a file that's there, and every new setting is only saved by its own key."""
    NEW = ['sound-speakers', 'sound-apps', 'push-to-talk', 'power-profiles', 'power-awake', 'battery', 'languages', 'spellcheck',
           'input-methods', 'info-cpu', 'info-gpu', 'info-memory', 'info-graphics', 'info-pci', 'info-usb', 'info-energy',
           'print-queue', 'scanners', 'usb-devices', 'bluetooth-devices', 'wallpaper-more', 'my-looks', 'look-more', 'file-types',
           'autostart', 'menu-editor', 'app-permissions']
    KEYS = ['dockScrollVolume', 'pttOn', 'pttKey', 'screenOffBattery', 'sleepBattery', 'wallScreens', 'wallShow', 'frameTint',
            'signinSplash', 'removable', 'usbNotify', 'firmwareNotify', 'welcomeAfterUpdates', 'oomNotice']

    def test_registered(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        for pid in self.NEW:
            m = re.search(r'\{ id: "%s", file: "([^"]+)"' % re.escape(pid), reg)
            self.assertTrue(m, pid)
            self.assertTrue((SHELL / m.group(1)).exists(), m.group(1))
        idx = (SHELL / 'settings-index.js').read_text()
        for pid in ('autostart', 'menu-editor', 'push-to-talk', 'info-cpu'):
            self.assertIn('"%s"' % pid, idx)

    def test_settings_keys_saved_one_by_one(self):
        src = (SHELL / 'Settings.qml').read_text()
        for k in self.KEYS:
            self.assertIn('"%s"' % k, src)
            self.assertIn('on%s%sChanged: save("%s")' % (k[0].upper(), k[1:], k), src)

    def test_watchers_are_loaded_by_file_name_and_cant_hold_the_desktop_up(self):
        shell = (SHELL / 'shell.qml').read_text()
        for f in ('PushToTalk.qml', 'FirmwareWatch.qml', 'OomWatch.qml', 'DeviceWatch.qml', 'Welcome.qml', 'SigninSplash.qml'):
            self.assertIn('Loader { source: "%s"' % f, shell)
        splash = (SHELL / 'SigninSplash.qml').read_text()
        self.assertIn('mask: Region {}', splash)                         # clicks go through
        self.assertIn('WlrKeyboardFocus.None', splash)
        self.assertIn('interval: 2500', splash)
        self.assertIn('if (!Settings.loaded || !early || Settings.signinSplash === ""', splash)
        dw = (SHELL / 'DeviceWatch.qml').read_text()
        self.assertIn('started', dw)                                      # nothing for what's there at sign-in

    def test_opening_pages_writes_nothing(self):
        # A page's Component.onCompleted only reads (the helpers' read commands), never a Settings key.
        for f in ('SetSoundSpeakers', 'SetSoundApps', 'SetPushToTalk', 'SetPowerProfiles', 'SetPowerAwake', 'SetBattery', 'SetAutostart',
                  'SetFileTypes', 'SetMenuEditor', 'SetWallpaperMore', 'SetMyLooks', 'SetLookMore', 'SetLanguages', 'SetSpellcheck',
                  'SetInputMethods', 'SetAppPermissions', 'SetBtDevices', 'SetPrintQueue', 'SetScanners', 'SetUsbDevices', 'SetInfoCenter'):
            src = (SHELL / (f + '.qml')).read_text()
            for m in re.finditer(r'Component\.onCompleted:([^\n]*)', src):
                self.assertNotRegex(m.group(1), r'Settings\.\w+\s*=[^=]', f)
                self.assertNotRegex(m.group(1), r'"(set|save|add-|remove|languages-set|spell-set|im-set|trust|block|cancel)', f)


if __name__ == '__main__':
    unittest.main()
