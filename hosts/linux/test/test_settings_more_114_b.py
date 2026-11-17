"""Tests for the rest of KDE System Settings in Hearth Settings (1.14, docs/SETTINGS-MORE-1.14.md, second part):
receiving files over Bluetooth and renaming this PC, Info Center's temperatures, kernel modules and interrupts, Scan to
PDF, ink and toner levels, the firmware update history, and downloading translations and dictionaries. Programs are
stand-ins; nothing here touches the real system."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
FIX = ROOT / 'hosts' / 'linux' / 'test' / 'fixtures' / 'settings-more'


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
                                                'XDG_CACHE_HOME': os.path.join(self.home, '.cache'),
                                                'XDG_RUNTIME_DIR': self.home, 'LANG': 'en_US.UTF-8'})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()


class Bluetooth(Home):
    def setUp(self):
        super().setUp()
        self.b = load(SHELL / 'hearth-bt-receive', 'hearth_bt_receive')

    def test_this_pc(self):
        st = self.b.parse_show('Controller 00:1A:7D:DA:71:13 (public)\n\tName: hearth\n\tAlias: Living room PC\n\tPowered: yes\n\tDiscoverable: no\n')
        self.assertEqual((st['name'], st['systemName'], st['address'], st['powered']), ('Living room PC', 'hearth', '00:1A:7D:DA:71:13', True))
        self.assertFalse(self.b.parse_show('')['present'])

    def test_rename_is_one_argument_and_checked(self):
        calls = []
        with mock.patch.object(self.b, 'run', side_effect=lambda c, timeout=15: calls.append(c) or (0, '', '')), \
                mock.patch.object(self.b, 'name', return_value={}):
            self.b.rename('My PC; rm -rf ~')
        self.assertEqual(calls[0], ['bluetoothctl', '--timeout', '5', 'system-alias', 'My PC; rm -rf ~'])
        for bad in ('', 'a\nb', 'x' * 249):
            with self.assertRaises(self.b.Fail):
                self.b.rename(bad)

    def test_received_names_stay_in_the_folder(self):
        self.assertEqual(self.b.safe_name('../../.bashrc'), 'bashrc')
        self.assertEqual(self.b.safe_name('/etc/passwd'), 'passwd')
        self.assertEqual(self.b.safe_name(''), 'received-file')
        open(os.path.join(self.home, 'a.jpg'), 'w').close()
        self.assertEqual(os.path.basename(self.b.unique(self.home, 'a.jpg')), 'a (2).jpg')

    def test_watcher_is_off_until_turned_on_and_always_asks(self):
        src = (SHELL / 'BtReceive.qml').read_text()
        self.assertIn('Settings.loaded && Settings.btReceive', src)
        self.assertIn('o.trusted && Settings.btReceiveTrusted', src)
        self.assertIn('if (!ask.answered) br.answer(ask.id, false)', src)   # a notice closed without an answer is a no
        self.assertIn('GLib.timeout_add_seconds(60, lambda: answer(False))', (SHELL / 'hearth-bt-receive').read_text())


class InfoCenter(Home):
    def setUp(self):
        super().setUp()
        self.sys = os.path.join(self.home, 'sys')
        self.proc = os.path.join(self.home, 'proc')
        os.makedirs(self.proc)
        with mock.patch.dict(os.environ, {'HEARTH_INFO_SYS': self.sys, 'HEARTH_INFO_PROC': self.proc}):
            self.i = load(SHELL / 'hearth-info', 'hearth_info_b')

    def w(self, rel, text):
        p = os.path.join(self.sys, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)

    def test_modules_and_interrupts(self):
        with open(os.path.join(self.proc, 'modules'), 'w') as f:
            f.write('snd_hda_intel 61440 3 - Live 0x0\nsnd 135168 12 snd_hda_intel,snd_hwdep, Live 0x0\n')
        self.w('module/snd/parameters/slots', '')
        r = self.i.modules()['items']
        self.assertEqual([x['title'] for x in r], ['snd', 'snd_hda_intel'])
        self.assertEqual(r[0]['sub'], 'Used by snd_hda_intel, snd_hwdep')
        self.assertEqual(r[0]['kind'], '1 settings')
        rows = self.i.parse_interrupts('           CPU0       CPU1\n  0:         38          0   IO-APIC   2-edge      timer\n'
                                       ' 36:     100000      2000   PCI-MSIX-0000:00:02.0 1-edge      nvme0q1\nLOC:    7614533    700000   Local timer interrupts\n')
        self.assertEqual(rows[1], {'irq': '36', 'total': 102000, 'kind': 'PCI-MSIX-0000:00:02.0 1-edge', 'devices': 'nvme0q1', 'cpus': 2})
        self.assertEqual(rows[2]['devices'], 'Local timer interrupts')

    def test_sensors_from_hwmon_and_lm_sensors(self):
        self.w('class/hwmon/hwmon0/name', 'k10temp\n')
        self.w('class/hwmon/hwmon0/temp1_input', '54250\n')
        self.w('class/hwmon/hwmon0/temp1_label', 'Tctl\n')
        self.w('class/hwmon/hwmon0/temp1_crit', '95000\n')
        self.w('class/hwmon/hwmon1/name', 'nct6775\n')
        self.w('class/hwmon/hwmon1/fan2_input', '1180\n')
        chips = self.i.hwmon()
        self.assertEqual(chips[0]['title'], 'Processor (AMD)')
        self.assertEqual(chips[0]['sensors'][0]['text'], '54 °C (critical 95)')
        self.assertEqual(chips[1]['sensors'][0]['text'], '1180 RPM')
        js = {'amdgpu-pci-0300': {'Adapter': 'PCI adapter', 'edge': {'temp1_input': 91.0, 'temp1_crit': 100.0, 'temp1_max': 90.0},
                                  'fan1': {'fan1_input': 1500.0}}}
        c = self.i.parse_sensors_json(js)[0]
        self.assertEqual(c['title'], 'Graphics card (AMD) · PCI adapter')
        self.assertTrue(c['sensors'][0]['hot'])
        with mock.patch.object(self.i, 'run', return_value=(127, '')):
            r = self.i.sensors()
        self.assertTrue(r['live'])
        self.assertEqual(r['sections'][0]['gauges'][0]['label'], 'Tctl')


class Scan(Home):
    def setUp(self):
        super().setUp()
        self.s = load(SHELL / 'hearth-scan', 'hearth_scan')
        self.s.RUN = os.path.join(self.home, 'hearth-scan')
        self.s.LIST = os.path.join(self.s.RUN, 'pages.json')

    def test_options(self):
        o = self.s.parse_options("  Scan mode:\n    --mode Lineart|Gray|Color [Color]\n    --resolution 75|150|300|600dpi [150]\n    --source Flatbed|ADF [Flatbed]\n")
        self.assertEqual(o, {'sources': ['Flatbed', 'ADF'], 'modes': ['Lineart', 'Gray', 'Color'], 'resolutions': [75, 150, 300, 600], 'feeder': True})
        self.assertEqual(self.s.parse_options('    --resolution 50..1200dpi [300]\n')['resolutions'], [75, 100, 150, 200, 300, 600])

    def test_pages_into_one_pdf(self):
        os.makedirs(self.s.RUN)
        for i in (1, 2):
            shutil.copy(FIX / ('scan-page-%d.jpg' % i), os.path.join(self.s.RUN, 'p%d.jpg' % i))
        self.s.save_list([{'file': 'p1.jpg', 'dpi': 75}, {'file': 'p2.jpg', 'dpi': 75}])
        self.assertEqual(self.s.jpeg_size(os.path.join(self.s.RUN, 'p1.jpg')), (620, 877, 1))
        self.s.change('up', '1')
        self.assertEqual([p['file'] for p in self.s.pages()], ['p2.jpg', 'p1.jpg'])
        r = self.s.save('Tax/../return')
        self.assertEqual(r['pages'], 2)
        self.assertEqual(os.path.dirname(r['file']), os.path.join(self.home, 'Documents'))
        with open(r['file'], 'rb') as f:
            data = f.read()
        self.assertTrue(data.startswith(b'%PDF-1.4'))
        self.assertEqual(data.count(b'/Type /Page '), 2)
        self.assertIn(b'/MediaBox [0 0 595.20 841.92]', data)       # 620 px at 75 dpi = A4
        self.assertNotEqual(self.s.save('Tax/../return')['file'], r['file'])   # never over a file that's there

    def test_scanner_arguments_are_checked(self):
        for bad in (('dev', '9999', '', ''), ('dev;x', '150', '', ''), ('dev', '150', 'Color; rm', '')):
            with self.assertRaises(self.s.Fail):
                self.s.check(*bad)


class Ink(unittest.TestCase):
    def test_ipp_round_trip(self):
        ink = load(SHELL / 'hearth-ink', 'hearth_ink')
        fake = load(FIX / 'fake_ipp.py', 'fake_ipp')
        req = ink.request('ipp://localhost/printers/Brother_DCP_L2550')
        self.assertEqual(req[:4], b'\x01\x01\x00\x0b')
        m = ink.markers(ink.parse(fake.answer(req)))
        self.assertEqual([(x['name'], x['level'], x['color']) for x in m][:2], [('Cyan', 64, '00ffff'), ('Magenta', 8, 'ff00ff')])
        self.assertEqual(m[1]['low'], 10)
        src = (SHELL / 'InkWatch.qml').read_text()
        self.assertIn('if (Settings.inkNotify', src)
        self.assertIn('m.level > (m.low || 15) + 10) ch[key] = false', src)   # told again only after a refill


class FirmwareHistory(unittest.TestCase):
    def test_history(self):
        fw = load(SHELL / 'hearth-firmware', 'hearth_firmware_b')
        h = fw.parse_history({'Devices': [
            {'Name': 'Unifying Receiver', 'Vendor': 'Logitech', 'Version': 'RQR12.10', 'UpdateState': 2, 'Modified': 1763300000,
             'Releases': [{'Version': 'RQR12.11', 'Summary': 'Firmware for the receiver'}]},
            {'Name': 'Samsung SSD', 'Version': '5B2QGXA7', 'UpdateState': 3, 'UpdateError': 'device was not in bootloader mode', 'Modified': 1763400000,
             'Releases': [{'Version': '5B2QGXA8'}]}]})['history']
        self.assertEqual([e['name'] for e in h], ['Samsung SSD', 'Unifying Receiver'])
        self.assertEqual((h[0]['state'], h[0]['error'][:6]), ('failed', 'device'))
        self.assertEqual((h[1]['from'], h[1]['to'], h[1]['state']), ('RQR12.10', 'RQR12.11', 'done'))


class LanguagePacks(Home):
    def setUp(self):
        super().setUp()
        self.l = load(SHELL / 'hearth-langpacks', 'hearth_langpacks')
        self.loc = os.path.join(self.home, 'locale')
        os.makedirs(os.path.join(self.loc, 'de', 'LC_MESSAGES'))
        for n in ('dolphin.mo', 'okular.mo'):
            open(os.path.join(self.loc, 'de', 'LC_MESSAGES', n), 'w').close()
        self.l.LOCALE = self.loc
        os.makedirs(os.path.join(self.home, '.config', 'environment.d'))
        with open(os.path.join(self.home, '.config', 'environment.d', '60-hearth-language.conf'), 'w') as f:
            f.write('# x\nLANGUAGE=de_DE:fr\n')

    def test_status_and_dictionaries(self):
        with mock.patch.object(self.l, 'flatpak_extra', return_value=['de']):
            st = self.l.status()
        ids = [x['id'] for x in st['languages']]
        self.assertEqual(ids[:2], ['de_DE', 'fr'])
        de = st['languages'][0]
        self.assertEqual((de['apps'], de['flatpak'], de['dict']), (2, True, 'missing'))
        aff, dic = b'SET UTF-8\n', b'2\nHaus\nBaum\n'
        with mock.patch.object(self.l, 'fetch', side_effect=lambda u: aff if u.endswith('aff') else dic):
            self.l.get_dict('de_DE')
        hd = os.path.join(self.home, '.local', 'share', 'hunspell')
        self.assertTrue(os.path.exists(os.path.join(hd, 'de_DE.dic')))
        self.assertTrue(os.path.islink(os.path.join(self.home, '.config', 'enchant', 'hunspell', 'de_DE.dic')))
        with mock.patch.object(self.l, 'flatpak_extra', return_value=None):
            self.assertEqual(self.l.status()['languages'][0]['dict'], 'downloaded')
            # Only what Hearth downloaded can be removed.
            with self.assertRaises(self.l.Fail):
                self.l.remove_dict('fr')
            self.l.remove_dict('de_DE')
        self.assertFalse(os.path.exists(os.path.join(hd, 'de_DE.dic')))

    def test_not_a_dictionary_is_refused(self):
        with mock.patch.object(self.l, 'fetch', return_value=b'<html>not found</html>'):
            with self.assertRaises(self.l.Fail):
                self.l.get_dict('xx_YY')
        self.assertFalse(os.path.exists(os.path.join(self.home, '.local', 'share', 'hunspell', 'xx_YY.dic')))


class Pages(unittest.TestCase):
    NEW = ['bluetooth-receive', 'scan-pdf', 'ink-levels', 'info-sensors', 'info-modules', 'info-interrupts', 'language-packs', 'firmware-history']
    KEYS = ['btReceive', 'btReceiveFolder', 'btReceiveTrusted', 'inkNotify']

    def test_registered(self):
        reg = (SHELL / 'settings-pages.js').read_text()
        for pid in self.NEW:
            m = re.search(r'\{ id: "%s", file: "([^"]+)"' % re.escape(pid), reg)
            self.assertTrue(m, pid)
            self.assertTrue((SHELL / m.group(1)).exists(), m.group(1))
        idx = (SHELL / 'settings-index.js').read_text()
        for pid in ('bluetooth-receive', 'scan-pdf', 'ink-levels', 'language-packs'):
            self.assertIn('"%s"' % pid, idx)

    def test_settings_keys_saved_one_by_one(self):
        src = (SHELL / 'Settings.qml').read_text()
        for k in self.KEYS:
            self.assertIn('"%s"' % k, src)
            self.assertIn('on%s%sChanged: save("%s")' % (k[0].upper(), k[1:], k), src)
        self.assertIn('property bool btReceive: false', src)        # receiving is off until you turn it on

    def test_watchers_loaded_by_file_name(self):
        shell = (SHELL / 'shell.qml').read_text()
        for f in ('BtReceive.qml', 'InkWatch.qml'):
            self.assertIn('Loader { source: "%s"' % f, shell)

    def test_opening_pages_writes_nothing(self):
        for f in ('SetBtReceive', 'SetScanPdf', 'SetInkLevels', 'SetLanguagePacks', 'SetFirmwareHistory', 'SetInfoCenter'):
            src = (SHELL / (f + '.qml')).read_text()
            for m in re.finditer(r'Component\.onCompleted:([^\n]*(?:\n(?!\s{4}\S)[^\n]*)*)', src):
                self.assertNotRegex(m.group(1), r'Settings\.\w+\s*=[^=]', f)
                self.assertNotRegex(m.group(1), r'"(rename|reset-name|page|feeder|save|clear|remove|get|flatpak-off|remove-dict)"', f)


if __name__ == '__main__':
    unittest.main()
