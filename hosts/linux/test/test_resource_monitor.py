"""Resource Monitor (docs/RESOURCE-MONITOR.md): "Your PC"'s detection from sample sysfs / DMI / lspci / hwmon trees of four
machines (the owner's ATX desktop, dad's FM2A88X desktop, a laptop, a ROG Ally), the drawing chosen for each, the tips
that are true from the data, the root helper's output (only the memory and slot tables, never serial numbers), the
sampler's extras, and that opening Resource Monitor writes no setting."""
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import re
import struct
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
FIX = pathlib.Path(__file__).resolve().parent / 'fixtures' / 'resmon'
HWINFO = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-hwinfo'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


rm = load('hearth_resmon', SHELL / 'hearth-resmon')
hw = load('hearth_hwinfo', HWINFO)


def machine(name):
    """(pc, view) for a fixture, the way `hearth-resmon pc --fixture` makes them."""
    fx = json.loads((FIX / (name + '.json')).read_text())
    with tempfile.TemporaryDirectory() as root:
        rm.materialize(fx, root)
        pc = rm.detect(rm.Machine(root, cmds=fx.get('cmds') or {}, hwinfo=fx.get('hwinfo') or {}), False, False)
    return pc, rm.build_view(pc)


def parts(view):
    return {p['id']: p for p in view['parts']}


class Detection(unittest.TestCase):
    def test_owners_atx_desktop(self):
        pc, v = machine('owner-atx')
        self.assertIsNone(pc.get('problems'))
        self.assertEqual(pc['family'], 'desktop')
        self.assertEqual(pc['cpu']['name'], 'Intel Core i7-14700F')
        self.assertEqual((pc['cpu']['cores'], pc['cpu']['threads']), (20, 28))
        self.assertEqual(pc['cpu']['socket'], 'LGA1700')
        self.assertIs(pc['cpu']['igpu'], False, 'an F model has no graphics')
        self.assertEqual(pc['chipset'], 'B760')
        self.assertEqual(pc['formFactor'], 'ATX')
        g = pc['gpus'][0]
        self.assertEqual((g['name'], g['link'], g['via'], g['integrated']), ('NVIDIA GeForce RTX 5070', 'PCIe 5.0 x16', 'cpu', False))
        self.assertEqual(next(o for o in g['outputs'] if o['connected']), {'port': 'HDMI', 'connected': True, 'monitor': 'TCL TV'}, 'the TV from its EDID')
        d = {x['name']: x for x in pc['drives']}
        self.assertEqual((d['nvme0n1']['link'], d['nvme0n1']['via'], d['nvme0n1']['system']), ('PCIe 4.0 x4', 'cpu', True))
        self.assertEqual((d['sda']['kind'], d['sda']['ataPort'], d['sda']['via']), ('hdd', 3, 'chipset'))
        slots = pc['memory']['slots']
        self.assertEqual([(s['locator'], s['size'] // 2 ** 30, s['channel'], s['empty']) for s in slots],
                         [('DIMM_A1', 0, 'A', True), ('DIMM_A2', 16, 'A', False), ('DIMM_B1', 0, 'B', True), ('DIMM_B2', 16, 'B', False)])
        self.assertEqual((slots[1]['type'], slots[1]['configured'], slots[1]['maker'], slots[1]['part']), ('DDR5', 5600, 'Kingston', 'KF556C40-16'))
        net = {n['name']: n for n in pc['net']}
        self.assertEqual((net['wlo1']['kind'], net['wlo1']['via'], net['enp4s0']['via']), ('wifi', 'chipset', 'chipset'))
        self.assertTrue(net['wlo1']['mac'].endswith('xx:xx:xx'), 'the MAC is shortened')
        self.assertEqual(pc['audio'], ['Realtek ALC897'], "the graphics card's HDMI sound isn't the board's sound chip")
        self.assertEqual(pc['sensors']['chip'], 'nct6798')
        self.assertEqual(pc['sensors']['cmos'], 3.04)
        # the board: 4 USB 3 + 3 USB 2 at the back (USB 3 ports counted once, the Bluetooth port inside left out), with
        # the mouse, keyboard and Xbox adapter on the ports they're really in
        p = parts(v)
        rear = [x for x in v['parts'] if re.fullmatch(r'usb\d', x['id'])]
        self.assertEqual([x['style'] for x in rear], ['ioUsb3'] * 4 + ['ioUsb2'] * 3)
        self.assertEqual([x['dev'] for x in rear[:4]], ['Logitech G502 HERO Gaming Mouse', 'Holtek USB Keyboard', 'Microsoft Xbox Wireless Adapter for Windows', 'Nothing plugged in'])
        self.assertEqual([p['dimm%d' % i]['status'] for i in range(4)], ['empty', 'on', 'empty', 'on'])
        self.assertIn('PCIe 5.0 x16 (full speed)', dict(p['pcie0']['specs'])['Link'])
        self.assertEqual(p['pcie0']['lanes'], 10, 'as many lines as lanes (16, drawn as up to 10)')
        self.assertEqual(p['m2_0']['hub'], 'cpu')
        self.assertEqual(p['m2_1']['status'], 'empty')
        self.assertEqual(p['sata3']['status'], 'on')
        self.assertEqual(p['io_disp']['status'], 'empty')
        self.assertEqual(v['layout']['template'], 'atx')
        self.assertIn('Intel B760', v['sub'])

    def test_dads_fm2a88x_desktop(self):
        pc, v = machine('dad-fm2')
        self.assertEqual(pc['cpu']['name'], 'AMD A10-6800K APU')
        self.assertEqual((pc['cpu']['socket'], pc['cpu']['family']), ('FM2', 'Richland'))
        self.assertTrue(pc['cpu']['igpu'])
        self.assertEqual(pc['chipset'], 'A88X')
        self.assertEqual(pc['dmi']['sys_vendor'], '', '"To Be Filled By O.E.M." is no name')
        gs = {g['name']: g for g in pc['gpus']}
        self.assertTrue(gs['AMD Radeon HD 8670D']['integrated'])
        r9 = gs['AMD Radeon R9 290/390']
        self.assertEqual((r9['integrated'], r9['link'], r9['vram']), (False, 'PCIe 2.0 x16', 8589934592))
        self.assertEqual([(s['type'], s['size'] // 2 ** 30, s['configured']) for s in pc['memory']['slots']], [('DDR3', 8, 1600)] * 4)
        self.assertEqual(pc['drives'][0]['kind'], 'hdd')
        self.assertTrue(pc['drives'][0]['system'])
        self.assertEqual(pc['optical'][0]['model'], 'DVD-RW DRU-870S')
        self.assertEqual(len(pc['sata']), 8)
        p = parts(v)
        self.assertEqual(p['dimm0']['style'], 'ddr3')
        self.assertEqual(p['sata2']['dev'], 'DVD-RW DRU-870S · no disc')
        self.assertTrue(p['pcie0']['dev'].endswith('R9 290/390'))
        self.assertEqual(p['vrm1']['style'], 'heatRed', "ASRock's red heatsinks on FM2")
        tips = [t['id'] for t in v['tips']]
        self.assertEqual(tips, ['hdd-system', 'm2-empty', 'cmos-low', 'no-wifi'])

    def test_laptop(self):
        pc, v = machine('laptop')
        self.assertEqual(pc['family'], 'laptop')
        self.assertEqual(v['layout'], {'family': 'laptop', 'template': 'laptop', 'why': 'The chassis says laptop'})
        self.assertTrue(pc['memory']['soldered'])
        self.assertEqual(pc['gpus'][0]['integrated'], True)
        self.assertEqual(pc['battery']['health'], 91)
        self.assertEqual(pc['battery']['cycles'], 212)
        p = parts(v)
        self.assertNotIn('dgpu', p, 'the Radeon 610M is built in')
        self.assertEqual([p['ram%d' % i]['style'] for i in range(1, 5)], ['lpddr'] * 4)
        self.assertEqual(p['batt']['text'], 'BATTERY · 87%')
        self.assertEqual(p['usb0']['dev'], 'Microsoft Xbox Wireless Controller', "the camera inside isn't a port")
        self.assertEqual(v['name'], 'LENOVO IdeaPad Slim 3 15AMN8')
        self.assertIn('8 GB LPDDR5 soldered', v['sub'])
        self.assertEqual(v['tips'], [])

    def test_rog_ally(self):
        pc, v = machine('rog-ally')
        self.assertEqual((pc['family'], pc['handheld']), ('handheld', 'rog-ally'), 'by its DMI name, though the chassis says desktop')
        self.assertEqual(v['layout']['template'], 'handheld:rog-ally')
        self.assertEqual(pc['memory']['source'], 'none')
        p = parts(v)
        self.assertEqual(p['sd']['status'], 'on')
        self.assertIn('microSD', p['sd']['title'])
        self.assertEqual(p['ssd']['dev'], 'SAMSUNG MZ9L4512HBLU-00BMV · Hearth (C:)')
        self.assertEqual(p['mcu']['dev'], 'ASUSTeK Computer Inc. N-KEY Device')
        self.assertEqual(p['batt']['text'], 'BATTERY · 64%')
        self.assertIn('16 GB', v['sub'], 'installed memory from the memory blocks, without root')
        self.assertEqual(v['name'], 'ROG Ally')

    def test_every_drawing_is_whole(self):
        """Never a broken picture: every part inside the stage, with a title, a status and a known style; every wire to
        a hub that exists."""
        styles = set(re.findall(r'case "([A-Za-z0-9]+)"', (SHELL / 'ResmonBoard.qml').read_text()))
        for name in ('owner-atx', 'dad-fm2', 'laptop', 'rog-ally'):
            pc, v = machine(name)
            ids = {p['id'] for p in v['parts']}
            self.assertEqual(len(ids), len(v['parts']), name + ': ids are unique')
            for p in v['parts']:
                self.assertTrue(0 <= p['x'] and p['x'] + p['w'] <= v['w'] and 0 <= p['y'] and p['y'] + p['h'] <= v['h'], '%s: %s is off the stage' % (name, p['id']))
                self.assertTrue(p['title'] and p['dev'] is not None, name + ': ' + p['id'])
                self.assertIn(p['status'], ('on', 'empty'))
                self.assertIn(p['style'], styles, '%s: %s has a style the board can\'t paint' % (name, p['id']))
                if p['hub']:
                    self.assertTrue(p['hub'] in v['hubs'] or p['hub'] in ids, '%s: %s is wired to %s' % (name, p['id'], p['hub']))
                a = p.get('action')
                if a:
                    self.assertIn(a['kind'], ('tab', 'settings', 'disks', 'devices', 'mixer', 'files'))
            self.assertTrue(v['name'] and v['sub'])


class LayoutChoice(unittest.TestCase):
    def pc(self, **kw):
        base = {'family': 'desktop', 'handheld': None, 'formFactor': 'ATX', 'dmi': {'board_name': 'X'}, 'pci': [{}]}
        base.update(kw)
        return base

    def test_families(self):
        self.assertEqual(rm.layout_choice(self.pc())['template'], 'atx')
        self.assertEqual(rm.layout_choice(self.pc(formFactor='mATX'))['template'], 'matx')
        self.assertEqual(rm.layout_choice(self.pc(formFactor='ITX'))['template'], 'itx')
        self.assertEqual(rm.layout_choice(self.pc(dmi={}))['template'], 'generic', 'a board that says nothing')
        self.assertEqual(rm.layout_choice(self.pc(family='laptop'))['template'], 'laptop')
        self.assertEqual(rm.layout_choice(self.pc(family='handheld', handheld='steam-deck'))['template'], 'handheld:steam-deck')
        self.assertEqual(rm.layout_choice(self.pc(family='handheld'))['template'], 'handheld:generic')

    def test_handhelds_by_dmi_name(self):
        for name, want in (('ROG Ally RC71L_RC71L', 'rog-ally'), ('ROG Ally X RC72LA_RC72LA', 'rog-ally-x'),
                           ('ROG Xbox Ally RC73YA_RC73YA', 'rog-xbox-ally'), ('ROG Xbox Ally X RC73XA_RC73XA', 'rog-xbox-ally-x'),
                           ('Jupiter', 'steam-deck'), ('Galileo', 'steam-deck-oled'), ('83E1', 'legion-go'), ('83L3', 'legion-go-s')):
            self.assertEqual(rm.family_of({'product_name': name}), ('handheld', want), name)
        self.assertEqual(rm.family_of({'product_name': 'Claw A1M'}), ('handheld', None))
        self.assertEqual(rm.family_of({'product_name': 'ROG Flow Z13 GZ302EA', 'chassis_type': '32'}), ('laptop', None))
        self.assertEqual(rm.family_of({'product_name': 'MS-7D98', 'chassis_type': '3'}), ('desktop', None))

    def test_board_names(self):
        for name, chip, ff in (('PRO B760M-A WIFI', 'B760', 'mATX'), ('FM2A88X Extreme6+', 'A88X', 'ATX'), ('ROG STRIX X670E-E GAMING WIFI', 'X670E', 'ATX'),
                               ('ROG STRIX B650-I GAMING WIFI', 'B650', 'ITX'), ('MAG B650 TOMAHAWK WIFI', 'B650', 'ATX'), ('A520M-A PRO', 'A520', 'mATX')):
            self.assertEqual(rm.chipset_of(name), chip, name)
            self.assertEqual(rm.form_factor(name, 4, 2, 'desktop'), ff, name)
        self.assertEqual(rm.form_factor('Unknown', 2, 1, 'desktop'), 'ITX')

    def test_a_failing_drawing_falls_back_to_the_generic_one(self):
        pc, _ = machine('owner-atx')
        pc['drives'] = [None]   # broken data
        v = rm.build_view(pc)
        self.assertEqual(v['layout']['template'], 'generic')
        self.assertTrue(v['parts'])

    def test_memory_channels(self):
        self.assertEqual(rm.channel_of('ChannelB-DIMM1'), 'B')
        self.assertEqual(rm.channel_of('DIMM_A2'), 'A')
        self.assertEqual(rm.channel_of('A1_DIMM0', 'P0 CHANNEL A'), 'A')
        self.assertEqual(rm.channel_of('DIMM 1', 'BANK 1'), 'B')

    def test_tips_only_when_true(self):
        pc, _ = machine('owner-atx')
        self.assertEqual([t['id'] for t in rm.tips_for(pc)], ['m2-empty'])
        for s in pc['memory']['slots']:   # both sticks moved to channel A
            s['empty'] = s['channel'] == 'B'
            s['size'] = 0 if s['empty'] else 17179869184
        self.assertIn('mem-channel', [t['id'] for t in rm.tips_for(pc)])
        pc['sensors']['cmos'] = 2.6
        self.assertIn('cmos-low', [t['id'] for t in rm.tips_for(pc)])
        pc['sensors']['cmos'] = None
        self.assertNotIn('cmos-low', [t['id'] for t in rm.tips_for(pc)], "not readable: no tip")


def smbios(*structs):
    out = b''
    for typ, body, strings in structs:
        formatted = bytes([typ, 4 + len(body)]) + b'\x01\x00' + body
        tail = b''.join(s.encode() + b'\0' for s in strings) + b'\0' if strings else b'\0\0'
        out += formatted + tail
    return out + bytes([127, 4, 0xff, 0xff]) + b'\0\0'


def type17(size_mb, loc, bank, maker, serial, part, speed=5600, typ=0x22, ff=0x09):
    b = bytearray(0x5C - 4)
    def put(off, fmt, v):
        struct.pack_into(fmt, b, off - 4, v)
    put(0x08, '<H', 64); put(0x0A, '<H', 64); put(0x0C, '<H', size_mb)
    b[0x0E - 4] = ff; b[0x10 - 4] = 1; b[0x11 - 4] = 2; b[0x12 - 4] = typ
    put(0x15, '<H', speed); b[0x17 - 4] = 3; b[0x18 - 4] = 4; b[0x19 - 4] = 5; b[0x1A - 4] = 6; b[0x1B - 4] = 1
    put(0x20, '<H', speed); put(0x22, '<H', 1100); put(0x24, '<H', 1100); put(0x26, '<H', 1250)
    return (17, bytes(b), [loc, bank, maker, serial, 'ASSET-TAG-9', part])


class RootHelper(unittest.TestCase):
    def table(self):
        t16 = bytearray(0x17 - 4)
        t16[0] = 0x03; t16[1] = 0x03; t16[2] = 0x03
        struct.pack_into('<I', t16, 0x07 - 4, 192 * 1024 * 1024); struct.pack_into('<H', t16, 0x0D - 4, 4)
        t9 = bytearray(0x11 - 4)
        t9[0] = 1; t9[1] = 0xC3; t9[2] = 0x0D; t9[3] = 0x04
        struct.pack_into('<H', t9, 0x0D - 4, 0); t9[0x0F - 4] = 1; t9[0x10 - 4] = 0
        t8 = bytes([1, 0x22, 2, 0x12, 0x10])
        t1 = bytes([1, 2, 3, 4]) + b'\x11' * 16   # system information with a UUID: never given back
        return smbios((1, t1, ['Maker', 'Product', 'Version', 'SERIAL-SYS-1']), (16, bytes(t16), []), type17(16384, 'DIMM_A2', 'P0 CHANNEL A', 'Kingston', 'SERIAL-1234', 'KF556C40-16'),
                      type17(0, 'DIMM_A1', 'P0 CHANNEL A', 'Unknown', 'SERIAL-0', 'NO DIMM'), (9, bytes(t9), ['PCIEX16_1']), (8, t8, ['JUSB1', 'USB 3.2']))

    def test_only_memory_and_slot_tables(self):
        d = hw.parse(self.table())
        self.assertEqual(set(d), {'ok', 'memory', 'slots', 'ports'})
        self.assertEqual(set(d['memory']), {'arrays', 'devices'})
        dev = d['memory']['devices'][0]
        self.assertEqual((dev['locator'], dev['size'], dev['type'], dev['formFactor'], dev['speed'], dev['manufacturer'], dev['partNumber'], dev['configuredVoltage']),
                         ('DIMM_A2', 16 * 2 ** 30, 'DDR5', 'DIMM', 5600, 'Kingston', 'KF556C40-16', 1.25))
        self.assertEqual(d['memory']['devices'][1]['size'], 0)
        self.assertEqual(d['memory']['devices'][1]['manufacturer'], '', "an empty slot's placeholder names are dropped")
        self.assertEqual(d['memory']['arrays'][0]['maxCapacity'], 192 * 2 ** 30)
        self.assertEqual(d['slots'], [{'designation': 'PCIEX16_1', 'type': 'PCIe 5.0 x16', 'width': 16, 'usage': 'in use', 'address': '0000:01:00.0'}])
        self.assertEqual(d['ports'], [{'internal': 'JUSB1', 'external': 'USB 3.2', 'kind': 'USB'}])
        text = json.dumps(d)
        for secret in ('SERIAL', 'ASSET', 'Product', 'Maker'):
            self.assertNotIn(secret, text, 'never anything but the memory and slot tables')
        allowed = {'locator', 'bank', 'size', 'type', 'formFactor', 'speed', 'configuredSpeed', 'manufacturer', 'partNumber', 'rank', 'voltage', 'maxVoltage', 'configuredVoltage', 'dataWidth'}
        self.assertEqual(set(dev), allowed)

    def test_command_line(self):
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(self.table())
        try:
            if os.geteuid() != 0:
                r = subprocess.run([sys.executable, str(HWINFO), '--file', f.name], capture_output=True, text=True)
                self.assertEqual(r.returncode, 0)
                self.assertTrue(json.loads(r.stdout)['ok'])
            r = subprocess.run([sys.executable, str(HWINFO), '--anything'], capture_output=True, text=True)
            self.assertEqual(json.loads(r.stdout)['ok'], False, 'other arguments are refused')
        finally:
            os.unlink(f.name)
        self.assertTrue(os.access(HWINFO, os.X_OK))

    def test_broken_tables_dont_crash(self):
        for data in (b'', b'\x11\x02', b'\x11\x40' + b'\0' * 10, self.table()[:37]):
            self.assertTrue(hw.parse(data)['ok'])

    def test_polkit_rule(self):
        rule = (ROOT / 'os/system_files/usr/share/polkit-1/rules.d/49-hearth-hwinfo.rules').read_text()
        self.assertIn('action.lookup("program") == "/usr/libexec/nimbus/hearth-hwinfo"', rule)
        self.assertIn('subject.local && subject.active', rule)
        self.assertEqual(rule.count('polkit.Result.YES'), 1)
        wf = (ROOT / '.github/workflows/base-peppermint.yml').read_text()
        self.assertIn('49-hearth-hwinfo.rules', wf, "the image check looks for the rule")


class Sampler(unittest.TestCase):
    def test_tags(self):
        P = rm.ht.Proc
        procs = {10: P(pid=10, cmd=['/home/u/.steam/steam/ubuntu12_32/steam'], comm='steam', exe='steam'),
                 20: P(pid=20, cmd=['reaper', 'SteamLaunch', 'AppId=1091500', '--', 'proton', 'waitforexitandrun', 'Cyberpunk2077.exe'], comm='reaper', exe='reaper'),
                 30: P(pid=30, cmd=['ollama', 'serve'], comm='ollama', exe='ollama'),
                 40: P(pid=40, cmd=['/usr/bin/pipewire'], comm='pipewire', exe='pipewire'),
                 50: P(pid=50, cmd=['python3', '/usr/share/nimbus/desktop/shell/hearth-pad'], comm='python3', exe='python3.13'),
                 60: P(pid=60, cmd=['firefox'], comm='firefox', exe='firefox')}
        g = lambda pid, **kw: dict({'pids': [pid], 'name': procs[pid].comm, 'kind': 'app', 'protected': False, 'why': '', 'tag': ''}, **kw)
        self.assertEqual(rm.tag_of(g(10), procs), 'Steam')
        self.assertEqual(rm.tag_of(g(20, kind='game', tag='Steam'), procs), 'Game · Proton')
        self.assertEqual(rm.tag_of(g(30), procs), 'Home server')
        self.assertEqual(rm.tag_of(g(40, protected=True, why='Sound (PipeWire)'), procs), 'System')
        self.assertEqual(rm.tag_of(g(50, protected=True, why='Hearth controller and buttons'), procs), 'Hearth')
        self.assertEqual(rm.tag_of(g(60), procs), '')

    def test_parsers(self):
        self.assertEqual(rm.parse_status_extra('RssAnon:\t 100 kB\nRssFile:\t 20 kB\nRssShmem:\t 4 kB\nVmSwap:\t 8 kB\n'), {'shared': 24 * 1024, 'swap': 8 * 1024})
        self.assertEqual(rm.parse_proc_io_split('rchar: 9\nread_bytes: 4096\nwrite_bytes: 512\n'), (4096, 512))
        self.assertEqual(rm.parse_drm_memory('drm-client-id: 3\ndrm-memory-vram:\t131072 KiB\ndrm-memory-gtt: 4 KiB\n'), 131072 * 1024)
        self.assertEqual(rm.hex_addr('0100007F:0050'), '127.0.0.1:80')
        self.assertEqual(rm.hex_addr('0000000000000000FFFF00000A00000A:01BB'), '10.0.0.10:443')
        pm = rm.parse_pmon('# gpu         pid   type     sm    mem    enc    dec    jpg    ofa     fb   ccpm    command\n'
                           '# Idx           #    C/G      %      %      %      %      %      %     MB     MB    name\n'
                           '    0       4821     G     68     40      -      -      -      -   6553      0    Cyberpunk2077.exe\n'
                           '    0       2204     G      3      1      -      -      -      -    410      0    gamescope\n')
        self.assertEqual(pm[4821], {'gpu': 68.0, 'vram': 6553 * 1048576, 'type': 'G'})
        smi = rm.parse_smi_full('0, NVIDIA GeForce RTX 5070, 72, 8090, 12227, 64, 168.20, 250.00, 2610, 14001, 48, 580.65, 5, 16, 00000000:01:00.0, 98.05\n')[0]
        self.assertEqual((smi['busy'], smi['watts'], smi['wattsMax'], smi['mhz'], smi['fan'], smi['linkGen'], smi['pci']), (72, 168.2, 250, 2610, 48, 5, '0000:01:00.0'))

    def test_memory_bar_adds_up(self):
        mi = {'MemTotal': 32 * 2 ** 30, 'MemFree': 6 * 2 ** 30, 'MemAvailable': 20 * 2 ** 30, 'Dirty': 2 ** 28, 'Writeback': 0}
        b = rm.meminfo_breakdown(mi, 32 * 2 ** 30 + 2 ** 29)
        self.assertEqual(b['reserved'], 2 ** 29)
        self.assertEqual(b['inUse'], 12 * 2 ** 30)
        self.assertEqual(b['saving'], 2 ** 28)
        self.assertEqual(b['reserved'] + b['inUse'] + b['saving'] + b['cache'] + b['free'], b['installed'])

    def test_serve_sends_what_the_tab_needs(self):
        r, w = os.pipe()
        os.write(w, b'{"interval": 0.05, "want": ["procs", "mem", "disk", "net", "gpu", "gpuprocs", "temps"]}\n')
        out = io.StringIO()
        rm.serve(r, out, max_samples=2)
        os.close(w); os.close(r)
        last = json.loads(out.getvalue().strip().splitlines()[-1])
        self.assertIn('memBar', last)
        self.assertIn('fans', last)
        g = last['procs'][0]
        for k in ('tag', 'color', 'threads', 'avg', 'shared', 'swap', 'read', 'write', 'conns', 'vram'):
            self.assertIn(k, g)

    def test_brief(self):
        s = rm.ResSampler()
        s.sample(['procs', 'gpu', 'temps'])
        b = rm.brief(s)
        self.assertEqual(set(b), {'gpu', 'cpu', 'mem', 'net', 'game', 'others'})
        self.assertLessEqual(len(b['others']), 4)

    def test_pause_uses_the_home_servers_runtime_flag_only(self):
        calls = []
        def hs(args, timeout=10):
            calls.append(args)
            return {'available': True, 'setupDone': True, 'lightMode': False, 'resumed': False, 'paused': [], 'backups': {'count': 1}} if args == ['quick'] else {}
        self.assertEqual(rm.pause('status', hs)['needsOption'], True, 'Light mode off: the option to turn it on is offered, not done')
        self.assertEqual(calls, [['quick']])
        calls.clear(); rm.pause('on', hs)
        self.assertEqual(calls, [['resume', 'off'], ['quick']])
        calls.clear(); rm.pause('off', hs)
        self.assertEqual(calls, [['resume', 'on'], ['quick']])
        calls.clear(); rm.pause('light-on', hs)
        self.assertEqual(calls, [['quick-set', 'lightMode', 'on'], ['quick']], 'only from its own button')
        with self.assertRaises(rm.Fail):
            rm.pause('lightMode off', hs)


class NoSettingsWritten(unittest.TestCase):
    def test_opening_writes_no_setting(self):
        """`hearth-resmon pc` and a sample write nothing in ~/.config or ~/.local (only a cache in ~/.cache/hearth)."""
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=home + '/.config', XDG_CACHE_HOME=home + '/.cache', XDG_DATA_HOME=home + '/.local/share',
                       XDG_STATE_HOME=home + '/.local/state', XDG_RUNTIME_DIR=home + '/run')
            os.makedirs(home + '/run')
            for args in (['pc', '--no-helper'], ['pc', '--fixture', str(FIX / 'owner-atx.json')], ['sample', 'procs', 'mem', 'temps'], ['sample', 'brief']):
                r = subprocess.run([sys.executable, str(SHELL / 'hearth-resmon')] + args, capture_output=True, text=True, env=env, timeout=60)
                self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
                json.loads(r.stdout)
            written = [str(p.relative_to(home)) for p in pathlib.Path(home).rglob('*') if p.is_file()]
            self.assertEqual([w for w in written if not w.startswith(('.cache/hearth/', 'run/'))], [], written)

    def test_qml_writes_no_setting(self):
        for f in ('ResourceMonitor.qml', 'ResourceMonitorView.qml', 'ResmonBoard.qml'):
            src = (SHELL / f).read_text()
            for word in ('SettingsWriter', 'setText', 'Settings.reduceMotion =', 'setPref', 'hearth_settings_io'):
                self.assertNotIn(word, src, '%s: opening Resource Monitor writes no setting' % f)
        view = (SHELL / 'ResourceMonitorView.qml').read_text()
        self.assertIn('!Settings.reduceMotion', view, 'the flow and bolts hold still with Reduce motion')
        self.assertIn('Loader { source: "ResourceMonitor.qml" }', (SHELL / 'shell.qml').read_text(), 'loaded by name: a problem only loses Resource Monitor')
        self.assertIn('source: "ResourceMonitorView.qml"', (SHELL / 'ResourceMonitor.qml').read_text())

    def test_wired_in(self):
        self.assertIn('"resmon", "open"', (SHELL / 'TaskManagerView.qml').read_text(), 'Task Manager › Open Resource Monitor')
        self.assertIn('"resmon", "open"', (SHELL / 'SetAbout.qml').read_text(), 'Settings › System › About')
        desk = (ROOT / 'os/system_files/usr/share/applications/hearth-resmon.desktop').read_text()
        self.assertIn('Exec=/usr/bin/hearth-resmon', desk)
        self.assertTrue((ROOT / 'branding/icons/apps/hearth-resmon.svg').exists())
        self.assertIn("TAB_WINDOWS = ('Resource Monitor',)", (SHELL / 'hearth-pad').read_text(), 'LB / RB switch its tabs')
        self.assertIn('active.title === "Resource Monitor"', (SHELL / 'Pad.qml').read_text())


if __name__ == '__main__':
    unittest.main()
