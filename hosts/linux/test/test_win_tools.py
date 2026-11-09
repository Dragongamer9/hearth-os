"""Windows' tools in Mantel (docs/DEVICE-MANAGER.md): Device Manager's helper (desktop/shell/hearth-devmgr) grouping a
made-up PC's devices from sample sysfs / proc files and a sample kernel log, the problems it finds (no driver, missing
firmware, a USB device that didn't start) and the ones it mustn't (firmware a driver found an older copy of, optional
files), Disable only for radios; the Run box's map (win-commands.json, run-map.js run with node): Windows commands,
%folders%, C:\\ paths, apps that aren't installed; and the wiring: Super+R / Super+X in labwc, the Loader, nothing
written when something opens."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import struct
import subprocess
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
SHELL = HERE.parent / 'desktop' / 'shell'
LABWC = HERE.parent / 'desktop' / 'labwc'
loader = importlib.machinery.SourceFileLoader('hearth_devmgr', str(SHELL / 'hearth-devmgr'))
spec = importlib.util.spec_from_loader('hearth_devmgr', loader)
hd = importlib.util.module_from_spec(spec)
loader.exec_module(hd)

NODE = shutil.which('node')


def write(path, text=''):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    mode = 'wb' if isinstance(text, bytes) else 'w'
    with open(path, mode) as f:
        f.write(text)


def link(target, name):
    os.makedirs(os.path.dirname(name), exist_ok=True)
    os.symlink(target, name)


def edid(maker, name):
    code = ((ord(maker[0]) - 64) << 10) | ((ord(maker[1]) - 64) << 5) | (ord(maker[2]) - 64)
    b = bytearray(128)
    b[0:8] = b'\x00\xff\xff\xff\xff\xff\xff\x00'
    b[8], b[9] = code >> 8, code & 0xff
    desc = b'\x00\x00\x00\xfc\x00' + (name.encode() + b'\n').ljust(13, b' ')
    b[54:72] = desc
    return bytes(b)


class FakePC:
    """A gaming PC: an NVIDIA card with a TV on HDMI, Intel Wi-Fi (an older firmware found), a Realtek Bluetooth
    adapter missing its firmware, an Ethernet chip with no driver, an Xbox controller, a laptop keyboard, an NVMe drive
    Hearth runs from, a phone, a USB gadget nobody drives, and a USB port where something didn't start."""

    def __init__(self, root):
        self.r = root
        R = lambda p: root + p
        dev = R('/sys/devices/pci0000:00')
        write(R('/proc/sys/kernel/osrelease'), '6.12.0-hearth\n')
        write(R('/proc/sys/kernel/hostname'), 'chris-hearth\n')
        write(R('/lib/modules/6.12.0-hearth/modules.dep'),
              'updates/dkms/nvidia.ko.zst:\nkernel/drivers/bluetooth/btusb.ko.zst: kernel/drivers/bluetooth/btrtl.ko.zst\n'
              'kernel/drivers/net/wireless/intel/iwlwifi/iwlwifi.ko.zst:\nkernel/drivers/usb/host/xhci-pci.ko.zst:\n'
              'kernel/drivers/input/joystick/xpad.ko.zst:\nkernel/sound/pci/hda/snd-hda-intel.ko.zst:\n')
        write(R('/lib/modules/6.12.0-hearth/modules.builtin'), 'kernel/drivers/input/keyboard/atkbd.ko\nkernel/drivers/nvme/host/nvme.ko\n')
        write(R('/usr/share/misc/pci.ids'), '# pci.ids\n10de  NVIDIA Corporation\n\t2c05  GB205 [GeForce RTX 5070]\n'
              '8086  Intel Corporation\n\t7af0  Alder Lake-S PCH CNVi WiFi\n\t7ae0  Alder Lake-S PCH USB 3.2 xHCI\n'
              '10ec  Realtek Semiconductor Co., Ltd.\n\t8126  RTL8126 5GbE Controller\nC 00  Unclassified device\n')
        for drv, mod in (('xhci_hcd', 'xhci_pci'), ('iwlwifi', 'iwlwifi'), ('nvidia', 'nvidia'), ('snd_hda_intel', 'snd_hda_intel'),
                         ('nvme', '')):
            os.makedirs(R('/sys/bus/pci/drivers/' + drv), exist_ok=True)
            if mod:
                os.makedirs(R('/sys/module/' + mod), exist_ok=True)
                link(R('/sys/module/' + mod), R('/sys/bus/pci/drivers/%s/module' % drv))
        for drv, mod in (('btusb', 'btusb'), ('xpad', 'xpad')):
            os.makedirs(R('/sys/bus/usb/drivers/' + drv))
            os.makedirs(R('/sys/module/' + mod), exist_ok=True)
            link(R('/sys/module/' + mod), R('/sys/bus/usb/drivers/%s/module' % drv))
        os.makedirs(R('/sys/bus/serio/drivers/atkbd'))
        write(R('/sys/module/nvidia/version'), '580.95.05\n')

        def pci(slot, cls, ven, did, drv=None):
            d = '%s/%s' % (dev, slot)
            write(d + '/class', cls + '\n')
            write(d + '/vendor', ven + '\n')
            write(d + '/device', did + '\n')
            if drv:
                link(R('/sys/bus/pci/drivers/' + drv), d + '/driver')
            link(d, R('/sys/bus/pci/devices/' + slot))
            return d

        xhci = pci('0000:00:14.0', '0x0c0330', '0x8086', '0x7ae0', 'xhci_hcd')
        wifi = pci('0000:00:14.3', '0x028000', '0x8086', '0x7af0', 'iwlwifi')
        gpu = pci('0000:01:00.0', '0x030000', '0x10de', '0x2c05', 'nvidia')
        hda = pci('0000:01:00.1', '0x040300', '0x10de', '0x22e8', 'snd_hda_intel')
        nvme = pci('0000:02:00.0', '0x010802', '0x144d', '0xa80c', 'nvme')
        pci('0000:03:00.0', '0x020000', '0x10ec', '0x8126')                 # no driver
        pci('0000:00:1f.4', '0x0c0500', '0x8086', '0x7aa3')                 # SMBus, no driver: normal, not listed
        # Wi-Fi: wlan0, its radio (rfkill0)
        write(wifi + '/net/wlan0/wireless/.keep')
        link(wifi, wifi + '/net/wlan0/device')
        phy = wifi + '/ieee80211/phy0'
        link(phy, wifi + '/net/wlan0/phy80211')
        self.rfkill(phy + '/rfkill0', 'wlan', 'phy0')
        link(wifi + '/net/wlan0', R('/sys/class/net/wlan0'))
        os.makedirs(R('/sys/devices/virtual/net/lo'))
        link(R('/sys/devices/virtual/net/lo'), R('/sys/class/net/lo'))
        # The TV on HDMI
        conn = gpu + '/drm/card1/card1-HDMI-A-1'
        write(conn + '/status', 'connected\n')
        write(conn + '/edid', edid('TCL', '55C805'))
        link(conn, R('/sys/class/drm/card1-HDMI-A-1'))
        write(gpu + '/drm/card1/card1-DP-1/status', 'disconnected\n')
        link(gpu + '/drm/card1/card1-DP-1', R('/sys/class/drm/card1-DP-1'))
        # Sound: the card's HDMI audio
        write(hda + '/sound/card1/id', 'NVidia\n')
        link(hda, hda + '/sound/card1/device')
        link(hda + '/sound/card1', R('/sys/class/sound/card1'))
        write(R('/proc/asound/cards'), ' 1 [NVidia         ]: HDA-Intel - HDA NVidia\n                      HDA NVidia at 0xf6080000 irq 17\n')
        # USB
        usb1 = xhci + '/usb1'

        def usb(port, ven, prod, name='', maker='', cls='00', ifaces=(), speed='480', removable='removable'):
            d = '%s/%s' % (usb1, port)
            write(d + '/idVendor', ven + '\n')
            write(d + '/idProduct', prod + '\n')
            write(d + '/bDeviceClass', cls + '\n')
            write(d + '/speed', speed + '\n')
            write(d + '/removable', removable + '\n')
            if name:
                write(d + '/product', name + '\n')
            if maker:
                write(d + '/manufacturer', maker + '\n')
            for n, (icls, sub, proto, drv) in enumerate(ifaces):
                i = '%s/%s:1.%d' % (d, port, n)
                write(i + '/bInterfaceClass', icls + '\n')
                write(i + '/bInterfaceSubClass', sub + '\n')
                write(i + '/bInterfaceProtocol', proto + '\n')
                if drv:
                    link(R('/sys/bus/usb/drivers/' + drv), i + '/driver')
                link(i, R('/sys/bus/usb/devices/%s:1.%d' % (port, n)))
            link(d, R('/sys/bus/usb/devices/' + port))
            return d

        os.makedirs(usb1)
        link(usb1, R('/sys/bus/usb/devices/usb1'))
        usb('1-1', '05e3', '0610', 'USB2.1 Hub', cls='09', ifaces=[('09', '00', '00', None)])
        bt = usb('1-4', '0bda', 'b85b', 'Bluetooth Radio', 'Realtek', 'e0', [('e0', '01', '01', 'btusb'), ('e0', '01', '01', 'btusb')],
                 removable='fixed')
        hci = bt + '/1-4:1.0/bluetooth/hci0'
        os.makedirs(hci)
        link(hci, R('/sys/class/bluetooth/hci0'))
        self.rfkill(hci + '/rfkill1', 'bluetooth', 'hci0')
        pad = usb('1-2', '045e', '0b12', 'Controller', 'Microsoft', 'ff', [('ff', '47', 'd0', 'xpad')], speed='12')
        os.makedirs(pad + '/1-2:1.0/input/input20')
        usb('1-3', '1234', '5678', 'Gadget', 'Acme', '00', [('03', '00', '00', None)])
        usb('1-5', '18d1', '4ee1', 'Pixel 8', 'Google', '00', [('06', '01', '01', None)])
        # The NVMe drive Hearth runs from (/sysroot on its second partition)
        ctrl = nvme + '/nvme/nvme0'
        write(ctrl + '/model', 'Samsung SSD 990 PRO 2TB                 \n')
        link(nvme, ctrl + '/device')
        disk = ctrl + '/nvme0n1'
        write(disk + '/size', str(2000398934016 // 512) + '\n')
        link(ctrl, disk + '/device')
        write(disk + '/nvme0n1p2/partition', '2\n')
        link(disk, R('/sys/block/nvme0n1'))
        link(disk + '/nvme0n1p2', R('/sys/class/block/nvme0n1p2'))
        os.makedirs(R('/sys/devices/virtual/block/loop0'))
        write(R('/sys/devices/virtual/block/loop0/size'), '1000\n')
        link(R('/sys/devices/virtual/block/loop0'), R('/sys/block/loop0'))
        write(R('/dev/nvme0n1p2'))
        write(R('/proc/mounts'), 'composefs / overlay ro 0 0\n/dev/nvme0n1p2 /sysroot ext4 rw 0 0\n/dev/nvme0n1p2 /var ext4 rw 0 0\n')
        # Input: the controller (and Hearth's own virtual pad, the power button, the keyboard)
        kb = R('/sys/devices/platform/i8042/serio0')
        os.makedirs(kb + '/input/input3')
        link(R('/sys/bus/serio/drivers/atkbd'), kb + '/driver')
        low = (1 << 30) | (1 << 57) | (1 << 1)
        write(R('/proc/bus/input/devices'), '\n'.join([
            'I: Bus=0019 Vendor=0000 Product=0001 Version=0000', 'N: Name="Power Button"', 'P: Phys=PNP0C0C/button/input0',
            'S: Sysfs=/devices/LNXSYSTM:00/input/input0', 'H: Handlers=kbd event0', 'B: EV=3', 'B: KEY=10000000000000 0', '',
            'I: Bus=0011 Vendor=0001 Product=0001 Version=ab41', 'N: Name="AT Translated Set 2 keyboard"', 'P: Phys=isa0060/serio0/input0',
            'S: Sysfs=/devices/platform/i8042/serio0/input/input3', 'H: Handlers=sysrq kbd leds event3', 'B: EV=120013',
            'B: KEY=402000000 3803078f800d001 feffffdfffefffff %x' % low, '',
            'I: Bus=0003 Vendor=045e Product=0b12 Version=0110', 'N: Name="Microsoft X-Box Series S|X Controller"',
            'P: Phys=usb-0000:00:14.0-2/input0', 'S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/input/input20',
            'H: Handlers=event20 js0', 'B: EV=20000b', '',
            'I: Bus=0003 Vendor=28de Product=11ff Version=0001', 'N: Name="Hearth virtual controller"', 'P: Phys=',
            'S: Sysfs=/devices/virtual/input/input30', 'H: Handlers=event30 js1', 'B: EV=b', '']))
        write(R('/proc/cpuinfo'), ''.join('processor\t: %d\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 7 7800X3D 8-Core Processor\n\n' % i for i in range(16)))
        write(R('/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver'), 'amd-pstate-epp\n')
        # Firmware: iwlwifi tried -89 and found -86; the Bluetooth file isn't there.
        write(R('/lib/firmware/iwlwifi-so-a0-gf-a0-86.ucode.zst'), 'x')
        write(R('/lib/firmware/rtl_bt/rtl8852bu_config.bin'), 'x')
        self.log = R('/kernel.log')
        write(self.log, '\n'.join([
            '[    1.000000] Linux version 6.12.0-hearth',
            '[    2.100000] iwlwifi 0000:00:14.3: Direct firmware load for iwlwifi-so-a0-gf-a0-89.ucode failed with error -2',
            '[    2.100100] iwlwifi 0000:00:14.3: Direct firmware load for iwlwifi-so-a0-gf-a0.pnvm failed with error -2',
            '[    2.300000] Bluetooth: hci0: RTL: firmware file rtl_bt/rtl8852bu_fw.bin not found',
            '[    2.300100] bluetooth hci0: Direct firmware load for rtl_bt/rtl8852bu_fw.bin failed with error -2',
            '[    3.000000] usb 1-7: device descriptor read/64, error -71',
            '[    3.100000] usb 1-7: device not accepting address 9, error -71',
            '[    3.200000] cfg80211: failed to load regulatory.db',
            '']))
        write(R('/dev/rfkill'), b'')

    def rfkill(self, path, kind, name):
        write(path + '/type', kind + '\n')
        write(path + '/name', name + '\n')
        write(path + '/soft', '0\n')
        write(path + '/hard', '0\n')
        link(os.path.dirname(path), path + '/device')
        link(path, self.r + '/sys/class/rfkill/' + os.path.basename(path))


class DeviceManagerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp()
        cls.pc = FakePC(cls.tmp)
        hd.ROOT = cls.tmp
        os.environ['HEARTH_DEVMGR_LOG'] = cls.pc.log
        cls.data = hd.Scan().run()
        cls.by_name = {d['name']: d for g in cls.data['groups'] for d in g['devices']}
        cls.groups = {g['id']: g for g in cls.data['groups']}

    @classmethod
    def tearDownClass(cls):
        hd.ROOT = ''
        os.environ.pop('HEARTH_DEVMGR_LOG', None)
        shutil.rmtree(cls.tmp, ignore_errors=True)

    def names(self, gid):
        return [d['name'] for d in self.groups[gid]['devices']]

    def dev(self, part):
        hits = [d for n, d in self.by_name.items() if part in n]
        self.assertEqual(len(hits), 1, (part, list(self.by_name)))
        return hits[0]

    def test_groups_in_windows_order(self):
        labels = [g['label'] for g in self.data['groups']]
        self.assertEqual(labels, ['Audio', 'Bluetooth', 'Cameras', 'Controllers', 'Disk drives', 'Display adapters',
                                  'Keyboards and mice', 'Monitors', 'Network adapters', 'Processors', 'USB controllers', 'Other devices'])
        self.assertEqual(self.data['host'], 'chris-hearth')
        self.assertTrue(self.data['kernelLog'])

    def test_each_device_lands_in_its_group(self):
        self.assertEqual(self.names('display'), ['NVIDIA GeForce RTX 5070 · driver 580'])
        self.assertEqual(self.names('monitors'), ['55C805'])
        self.assertEqual(self.dev('55C805')['maker'], 'TCL')
        self.assertIn('HDMI-A-1', self.dev('55C805')['where'])
        self.assertEqual(self.names('bluetooth'), ['Realtek Bluetooth Radio'])
        self.assertEqual(self.names('controllers'), ['Microsoft X-Box Series S|X Controller'])
        self.assertEqual(self.names('input'), ['AT Translated Set 2 keyboard'])       # not the power button, not Hearth's pad
        self.assertEqual(self.names('disks'), ['Samsung SSD 990 PRO 2TB 2 TB'])       # not loop0
        self.assertEqual(self.names('cpu'), ['AMD Ryzen 7 7800X3D 8-Core Processor'])
        self.assertEqual(self.dev('Ryzen')['threads'], 16)
        self.assertEqual(self.names('usb'), ['Intel Alder Lake-S PCH USB 3.2 xHCI'])
        self.assertEqual(self.names('audio'), ['HDA NVidia'])
        net = self.names('net')
        self.assertIn('Intel Alder Lake-S PCH CNVi WiFi', net)
        self.assertIn('Realtek RTL8126 5GbE Controller', net)
        self.assertEqual(self.groups['cameras']['devices'], [])
        # Other devices: the gadget nobody drives, the phone (apps talk to it), the port that failed; never the hub.
        other = self.names('other')
        self.assertIn('Acme Gadget', other)
        self.assertIn('Google Pixel 8', other)
        self.assertIn('Unknown USB device (port 1-7)', other)
        self.assertFalse(any('Hub' in n for g in self.data['groups'] for n in [d['name'] for d in g['devices']]))

    def test_drivers_and_where_they_come_from(self):
        bt = self.dev('Bluetooth Radio')
        self.assertEqual((bt['driver'], bt['driverNote']), ('btusb', 'built into Hearth'))
        self.assertEqual(bt['maker'], 'Realtek')
        self.assertIn('USB 2 port 1-4', bt['where'])
        self.assertIn('inside the PC', bt['where'])
        self.assertEqual(bt['ids'], 'USB 0bda:b85b')
        gpu = self.dev('RTX 5070')
        self.assertEqual(gpu['driverNote'], "NVIDIA's driver, added by Hearth")
        self.assertTrue(gpu['nvidia'])
        self.assertEqual(gpu['driverVersion'], '580.95.05')
        kb = self.dev('AT Translated')
        self.assertEqual((kb['driver'], kb['driverNote']), ('atkbd', "built into Hearth's kernel"))
        self.assertEqual(kb['where'], 'Built in (PS/2)')
        pad = self.dev('X-Box')
        self.assertEqual(pad['driver'], 'xpad')
        self.assertIn('USB 1 port 1-2', pad['where'])
        self.assertEqual(self.dev('Ryzen')['driver'], 'amd-pstate-epp')
        self.assertEqual(self.dev('Pixel 8')['driver'], 'none needed')
        self.assertIsNone(self.dev('Pixel 8')['problem'])

    def test_problems(self):
        bt = self.dev('Bluetooth Radio')
        self.assertEqual(bt['problem']['kind'], 'firmware')
        self.assertEqual(bt['firmware'], 'rtl8852bu_fw.bin · not found')
        self.assertEqual(bt['firmwareFiles'], ['rtl_bt/rtl8852bu_fw.bin'])
        self.assertEqual(self.dev('RTL8126')['problem']['kind'], 'driver')
        self.assertEqual(self.dev('Acme Gadget')['problem']['kind'], 'driver')
        self.assertEqual(self.dev('port 1-7')['problem']['kind'], 'error')
        self.assertIn('device descriptor read', self.dev('port 1-7')['problem']['text'])
        # Wi-Fi tried a newer firmware and found an older one; the .pnvm and regulatory.db are optional.
        wifi = self.dev('CNVi WiFi')
        self.assertIsNone(wifi['problem'])
        self.assertEqual(wifi['firmware'], '')
        self.assertEqual(self.data['problems'], 4)
        self.assertEqual(self.groups['bluetooth']['problems'], 1)
        # Problems first in their group
        self.assertEqual(self.groups['net']['devices'][0]['name'], 'Realtek RTL8126 5GbE Controller')

    def test_parse_log_alone(self):
        r = hd.parse_log(['[ 1.0] r8169 0000:03:00.0: probe with driver r8169 failed with error -22',
                          'amdgpu 0000:05:00.0: firmware: failed to load amdgpu/gc_11_0_1_mes.bin (-2)',
                          'iwlwifi 0000:00:14.3: Direct firmware load for iwlwifi-so-a0-gf-a0-89.ucode failed with error -2'])
        self.assertEqual(r['probe'], [('0000:03:00.0', 'r8169', '-22')])
        self.assertEqual(r['firmware'], [('0000:05:00.0', 'amdgpu', 'amdgpu/gc_11_0_1_mes.bin')])

    def test_only_radios_can_be_disabled(self):
        can = sorted(d['name'] for g in self.data['groups'] for d in g['devices'] if d['canDisable'])
        self.assertEqual(can, ['Intel Alder Lake-S PCH CNVi WiFi', 'Realtek Bluetooth Radio'])
        disk = self.dev('990 PRO')
        self.assertTrue(disk['holdsSystem'])
        self.assertFalse(disk['canDisable'])
        self.assertTrue(self.dev('RTX 5070')['inUse'])
        self.assertTrue(self.dev('55C805')['inUse'])

    def test_ids_are_stable(self):
        again = hd.Scan().run()
        self.assertEqual([d['id'] for g in again['groups'] for d in g['devices']],
                         [d['id'] for g in self.data['groups'] for d in g['devices']])
        ids = [d['id'] for g in again['groups'] for d in g['devices']]
        self.assertEqual(len(ids), len(set(ids)))

    def test_disable_writes_one_rfkill_change_and_nothing_else(self):
        rf = self.tmp + '/dev/rfkill'
        open(rf, 'wb').close()
        bt = self.dev('Bluetooth Radio')
        r = hd.set_radio(bt['id'], True)
        self.assertTrue(r['disabled'])
        with open(rf, 'rb') as f:
            self.assertEqual(f.read(), struct.pack('IBBBB', 1, 0, 2, 1, 0))
        open(rf, 'wb').close()
        hd.set_radio(bt['id'], False)
        with open(rf, 'rb') as f:
            self.assertEqual(f.read(), struct.pack('IBBBB', 1, 0, 2, 0, 0))
        with self.assertRaises(ValueError):
            hd.set_radio(self.dev('990 PRO')['id'], True)
        with self.assertRaises(ValueError):
            hd.set_radio(self.dev('RTX 5070')['id'], True)

    def test_list_runs_as_a_program(self):
        env = dict(os.environ, HEARTH_DEVMGR_ROOT=self.tmp, HEARTH_DEVMGR_LOG=self.pc.log)
        r = subprocess.run([sys.executable, str(SHELL / 'hearth-devmgr'), 'list'], capture_output=True, text=True, env=env, timeout=30)
        self.assertEqual(r.returncode, 0, r.stderr + r.stdout)
        d = json.loads(r.stdout)
        self.assertEqual(d['problems'], 4)

    def test_edid_and_sizes(self):
        self.assertEqual(hd.edid_name(edid('SAM', 'Odyssey G7')), ('Odyssey G7', 'Samsung'))
        self.assertEqual(hd.edid_name(b'junk'), ('', ''))
        self.assertEqual(hd.human_size(2000398934016), '2 TB')
        self.assertEqual(hd.human_size(512110190592), '512 GB')
        self.assertEqual(hd.human_size(1500000000000), '1.5 TB')

    def test_the_helper_is_read_only(self):
        src = (SHELL / 'hearth-devmgr').read_text()
        # The only write is the rfkill change; no settings file, no service.
        self.assertEqual(src.count("'wb'"), 1)
        self.assertIn("open(P('/dev/rfkill'), 'wb'", src)
        self.assertNotRegex(src, r"open\([^\n]*'(w|a|w\+|r\+)'")
        self.assertNotIn('systemctl', src)
        self.assertNotIn('hearth_settings_io', src)
        self.assertTrue(os.access(SHELL / 'hearth-devmgr', os.X_OK))


JS = r'''
const fs = require('fs');
const load = (f, names) => { const src = fs.readFileSync(f, 'utf8').replace(/^\.pragma library\s*$/m, ''); const m = {};
  new Function('exports', src + '\n' + names.map(n => 'exports.' + n + ' = ' + n + ';').join(''))(m); return m; };
const rm = load(process.argv[2], ['resolve', 'suggest', 'remember', 'winxItems', 'needed']);
const dp = load(process.argv[3], ['fromWindows']);
const data = JSON.parse(fs.readFileSync(process.argv[4], 'utf8'));
const have = JSON.parse(process.argv[5]);
const ctx = { home: '/home/me', user: 'me', drives: [{ letter: 'C', path: '/' }, { letter: 'D', path: '/run/media/me/Games' }] };
const env = { home: '/home/me', installed: n => have.indexOf(n) >= 0, fromWindows: t => dp.fromWindows(t, ctx),
              apps: [{ id: 'org.mozilla.firefox.desktop', name: 'Firefox' }, { id: 'org.kde.kcalc.desktop', name: 'KCalc' }] };
const out = { resolve: {}, suggest: {}, winx: rm.winxItems(data, env.installed).map(x => x.sep ? '-' : x.label), needed: rm.needed(data) };
for (const t of JSON.parse(process.argv[6])) out.resolve[t] = rm.resolve(t, data, env);
for (const t of ['dev', 'disk', '%', 'fire', '']) out.suggest[t] = rm.suggest(t, data, env, ['notepad', 'D:\\Games'], 6);
out.remember = rm.remember(['taskmgr', 'calc', 'Notepad'], 'notepad', 3);
console.log(JSON.stringify(out));
'''


@unittest.skipUnless(NODE, 'node is needed to run run-map.js')
class RunMapTest(unittest.TestCase):
    TYPED = ['taskmgr', 'TaskMgr.exe', 'diskmgmt.msc', 'devmgmt.msc', 'control', 'ms-settings:', 'ms-settings:display',
             'ms-settings:nothing-like-this', 'notepad', 'cmd', 'powershell', 'explorer', 'calc', 'msconfig', 'rstrui',
             'perfmon /rel', 'resmon', 'perfmon', 'appwiz.cpl', '%appdata%', '%LocalAppData%', '%temp%', '%userprofile%', '%appdata%\\discord',
             '%nope%', 'C:\\', 'C:\\Users\\me\\Documents', 'c:/Users/ME/Downloads', 'D:\\SteamLibrary', 'E:\\', '~/Music',
             '/etc', '\\\\gaming-pc\\Games', 'https://hearthos.fun', 'www.example.com', 'firefox', 'kcalc', 'htop -t', '  ', '???']

    def run_js(self, have):
        with tempfile.NamedTemporaryFile('w', suffix='.js', delete=False) as f:
            f.write(JS)
        try:
            r = subprocess.run([NODE, f.name, str(SHELL / 'run-map.js'), str(SHELL / 'drive-paths.js'), str(SHELL / 'win-commands.json'),
                                json.dumps(have), json.dumps(self.TYPED)], capture_output=True, text=True, timeout=30)
        finally:
            os.unlink(f.name)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout)

    @classmethod
    def setUpClass(cls):
        cls.data = json.loads((SHELL / 'win-commands.json').read_text())

    def test_windows_commands(self):
        r = self.run_js(['hearth-disks', 'hearth-text', 'hearth-calculator'])['resolve']
        self.assertEqual(r['taskmgr'], {'label': 'Task Manager', 'kind': 'tasks', 'page': ''})
        self.assertEqual(r['TaskMgr.exe']['kind'], 'tasks')
        self.assertEqual(r['msconfig']['page'], 'startup')
        self.assertEqual(r['diskmgmt.msc']['argv'], ['hearth-disks'])
        self.assertEqual(r['devmgmt.msc']['kind'], 'devices')
        self.assertEqual((r['control']['kind'], r['control']['page']), ('settings', ''))
        self.assertEqual((r['ms-settings:']['kind'], r['ms-settings:']['page']), ('settings', ''))
        self.assertEqual(r['ms-settings:display']['page'], 'display')
        self.assertEqual(r['ms-settings:nothing-like-this']['page'], '')
        self.assertEqual(r['notepad']['argv'], ['hearth-text'])
        self.assertEqual(r['cmd']['kind'], 'terminal')
        self.assertEqual(r['powershell']['kind'], 'terminal')
        self.assertEqual((r['explorer']['kind'], r['explorer']['path']), ('files', ''))
        self.assertEqual(r['calc']['argv'], ['hearth-calculator'])
        self.assertEqual((r['appwiz.cpl']['kind'], r['appwiz.cpl']['path']), ('files', 'hearth:apps'))

    def test_apps_that_arent_installed_say_so(self):
        r = self.run_js([])['resolve']
        for t in ('rstrui', 'perfmon /rel', 'diskmgmt.msc', 'notepad', 'calc'):
            self.assertEqual(r[t]['kind'], 'missing', t)
        r = self.run_js(['hearth-restore', 'hearth-reliability'])['resolve']
        self.assertEqual(r['rstrui']['argv'], ['hearth-restore'])
        self.assertEqual(r['perfmon /rel']['argv'], ['hearth-reliability'])
        self.assertEqual(self.run_js([])['resolve']['resmon']['kind'], 'missing')
        self.assertEqual(self.run_js(['hearth-resmon'])['resolve']['resmon']['argv'], ['hearth-resmon'])
        self.assertEqual(self.run_js([])['resolve']['perfmon']['page'], 'performance')
        self.assertIn('Resource Monitor', self.run_js(['hearth-resmon'])['winx'])

    def test_folders_paths_and_websites(self):
        r = self.run_js([])['resolve']
        self.assertEqual(r['%appdata%']['path'], '/home/me/.config')
        self.assertEqual(r['%LocalAppData%']['path'], '/home/me/.local/share')
        self.assertEqual(r['%temp%']['path'], '/home/me/.cache')
        self.assertEqual(r['%userprofile%']['path'], '/home/me')
        self.assertEqual(r['%appdata%\\discord']['path'], '/home/me/.config/discord')
        self.assertEqual(r['%nope%']['kind'], 'error')
        self.assertEqual(r['C:\\']['path'], '/')
        self.assertEqual(r['C:\\Users\\me\\Documents']['path'], '/home/me/Documents')
        self.assertEqual(r['c:/Users/ME/Downloads']['path'], '/home/me/Downloads')
        self.assertEqual(r['D:\\SteamLibrary']['path'], '/run/media/me/Games/SteamLibrary')
        self.assertEqual(r['E:\\']['kind'], 'error')
        self.assertIn('no drive E:', r['E:\\']['text'])
        self.assertEqual(r['~/Music']['path'], '/home/me/Music')
        self.assertEqual(r['/etc']['path'], '/etc')
        self.assertEqual(r['\\\\gaming-pc\\Games']['path'], 'smb://gaming-pc/Games')
        self.assertEqual(r['https://hearthos.fun']['url'], 'https://hearthos.fun')
        self.assertEqual(r['www.example.com']['url'], 'https://www.example.com')
        self.assertEqual(r['firefox'], {'kind': 'app', 'id': 'org.mozilla.firefox.desktop', 'label': 'Firefox'})
        self.assertEqual(r['kcalc']['kind'], 'app')
        self.assertEqual(r['htop -t'], {'kind': 'command', 'argv': ['htop', '-t'], 'label': 'htop -t'})
        self.assertIsNone(r['  '])
        self.assertEqual(r['???']['kind'], 'error')

    def test_suggestions(self):
        out = self.run_js(['hearth-disks'])
        s = out['suggest']
        self.assertEqual(s['dev'][0], {'cmd': 'devmgmt.msc', 'opens': 'Device Manager'})
        self.assertIn({'cmd': 'diskmgmt.msc', 'opens': 'Disk Management (Hearth Disks)'}, s['disk'])
        self.assertTrue(all(x['cmd'].startswith('%') for x in s['%']))
        self.assertIn({'cmd': 'Firefox', 'opens': 'App'}, s['fire'])
        # Empty: the recent entries (notepad isn't installed here, so it isn't offered)
        self.assertEqual([x['cmd'] for x in s['']], ['D:\\Games'])
        self.assertTrue(all(len(v) <= 6 for v in s.values()))
        self.assertEqual(out['remember'], ['notepad', 'taskmgr', 'calc'])
        # No suggestion for an app that isn't installed
        self.assertFalse(any(x['cmd'] == 'rstrui' for x in self.run_js([])['suggest'].get('r', [])))

    def test_winx_hides_what_isnt_installed(self):
        out = self.run_js([])
        self.assertEqual(out['winx'], ['Installed apps', 'Power options', 'System', 'Device Manager', 'Network connections', '-',
                                       'Terminal', 'Task Manager', 'Settings', 'Files', 'Search', 'Run', '-', 'Shut down or sign out', 'Desktop'])
        full = self.run_js(['hearth-reliability', 'hearth-disks'])['winx']
        self.assertEqual(full[:8], ['Installed apps', 'Power options', 'Reliability history', 'System', 'Device Manager',
                                    'Network connections', 'Disk Management', '-'])
        self.assertEqual(out['needed'], ['hearth-calculator', 'hearth-disks', 'hearth-reliability', 'hearth-resmon', 'hearth-restore', 'hearth-text', 'hearth-tools'])

    def test_the_map_file(self):
        keys = [x['key'] for x in self.data['winx'] if not x.get('sep')]
        self.assertEqual(len(keys), len(set(keys)), keys)
        for it in self.data['winx']:
            if not it.get('sep'):
                self.assertIn(it['key'].lower(), it['label'].lower(), it)          # the underlined letter is in the label
        names = [n for c in self.data['commands'] for n in c['names']]
        self.assertEqual(len(names), len(set(names)))
        self.assertTrue(all(n == n.lower() and not n.endswith('.exe') for n in names))
        pages = set(re.findall(r'\{ id: "([a-z0-9-]+)"', (SHELL / 'settings-pages.js').read_text()))
        used = {c['do']['page'] for c in self.data['commands'] + [w for w in self.data['winx'] if not w.get('sep')]
                if c['do']['kind'] == 'settings' and c['do']['page']} | set(self.data['msSettings'].values())
        self.assertTrue(used <= pages, used - pages)
        kinds = {c['do']['kind'] for c in self.data['commands'] + [w for w in self.data['winx'] if not w.get('sep')]}
        act = (SHELL / 'WinTools.qml').read_text()
        for k in kinds - {'power'}:
            self.assertIn('case "%s":' % k, act, k)


class WiringTest(unittest.TestCase):
    def test_labwc_binds_super_r_and_super_x_once(self):
        rc = (LABWC / 'rc.xml').read_text()
        keys = re.findall(r'<keybind key="([^"]+)"', rc)
        dupes = {k for k in keys if keys.count(k) > 1}
        self.assertEqual(dupes, set())
        self.assertRegex(rc, r'<keybind key="W-r"><action name="Execute" command="[^"]*call run toggle')
        self.assertRegex(rc, r'<keybind key="W-x"><action name="Execute" command="[^"]*call winx toggle')
        self.assertRegex(rc, r'<keybind key="W-s"><action name="Execute" command="[^"]*call launcher open')   # search stays

    def test_loaded_through_a_loader(self):
        sh = (SHELL / 'shell.qml').read_text()
        self.assertIn('source: "WinTools.qml"', sh)
        self.assertNotRegex(sh, r'^\s*WinTools\s*\{', 'WinTools must load by file name, so a problem in it never stops the shell')
        wt = (SHELL / 'WinTools.qml').read_text()
        self.assertIn('source: "DeviceManagerView.qml"', wt)
        self.assertIn('source: "RunView.qml"', wt)
        self.assertEqual(wt.count('LazyLoader {'), 3)             # nothing exists until it's opened
        self.assertNotIn('Timer', wt)                             # and nothing runs by itself
        self.assertNotIn('running: true', wt)

    def test_pages_dont_hide_items_data(self):
        # An Item's default property is `data` (its children): a page declaring its own `data` reads its children
        # instead (the first CI run: "Cannot read property '0' of undefined" in DeviceManagerView.qml).
        for n in ('DeviceManagerView.qml', 'RunView.qml'):
            self.assertNotRegex((SHELL / n).read_text(), r'property\s+var\s+data\b', n)

    def test_opening_writes_nothing(self):
        wt = (SHELL / 'WinTools.qml').read_text()
        # The only write: the recent entries, through SettingsWriter, from remember() (called after something opened).
        self.assertEqual(re.findall(r'\w+\.merge\(', wt), ['historyWriter.merge('])
        fn = wt[wt.index('function remember('):]
        fn = fn[:fn.index('\n    }\n')]
        self.assertIn('historyWriter.merge({ recent: next })', fn)
        self.assertNotIn('remember(', wt[:wt.index('function remember(')])
        rv = (SHELL / 'RunView.qml').read_text()
        self.assertEqual(rv.count('wt.remember(entry)'), 2)      # after opening it (directly, or a program once found)
        dv = (SHELL / 'DeviceManagerView.qml').read_text()
        self.assertNotIn('SettingsWriter', dv)
        self.assertNotIn('merge(', dv)
        self.assertIn('/nimbus/run-history.json', wt)
        self.assertIn('.local/state', wt)

    def test_controller_reaches_them(self):
        pad = (SHELL / 'Pad.qml').read_text()
        self.assertIn('SettingsNav.winxShown || (active !== null && (active.title === "Device Manager" || active.title === "Run"))', pad)
        wt = (SHELL / 'WinTools.qml').read_text()
        self.assertIn('title: "Device Manager"', wt)
        self.assertIn('title: "Run"', wt)
        self.assertIn('property: "winxShown"', wt)
        for f in ('Islands.qml', 'Dock.qml', 'Bar.qml'):
            self.assertIn('SettingsNav.winTools.winxFrom(', (SHELL / f).read_text(), f)

    def test_no_environment_objects_and_no_direct_suspend(self):
        for f in ('WinTools.qml', 'RunView.qml', 'DeviceManagerView.qml'):
            t = (SHELL / f).read_text()
            self.assertIsNone(re.search(r'^\s*environment\s*:', t, re.M), f)
            self.assertNotRegex(t, r'"systemctl",\s*"suspend"', f)

    def test_shortcuts_page_names_them(self):
        sysm = (SHELL / 'hearth-system').read_text()
        self.assertIn("(r'call run toggle'", sysm)
        self.assertIn("(r'call winx toggle'", sysm)


if __name__ == '__main__':
    unittest.main()
