"""Stand-in answers for the Desktop Mode test's pictures of 1.14's more-of-KDE settings pages (desktop-test.yml puts a
small sh wrapper per program in /tmp/fakebin that runs this with the program's name). Anything it doesn't know goes to
the real program, so the rest of the desktop is unaffected. Never installed anywhere."""
import getpass
import os
import sys
import time

prog, args = sys.argv[1], sys.argv[2:]
a = ' '.join(args)
ME = getpass.getuser()
BAT = '/org/freedesktop/UPower/devices/battery_BAT0'


def real():
    for d in ('/usr/bin', '/bin', '/usr/sbin'):
        p = os.path.join(d, prog)
        if os.path.exists(p):
            os.execv(p, [p] + args)
    sys.exit(127)


def out(t):
    sys.stdout.write(t.replace(' you ', ' %s ' % ME))
    sys.exit(0)


if prog == 'upower':
    if args == ['-e']:
        out(BAT + '\n/org/freedesktop/UPower/devices/line_power_AC\n')
    if args[:1] == ['-i'] and args[1:] == [BAT]:
        out('  native-path:          BAT0\n  vendor:               SMP\n  model:                5B10W13930\n  power supply:         yes\n'
            '  present:              yes\n  state:                discharging\n  energy-full:          52.3 Wh\n  energy-full-design:   57 Wh\n'
            '  energy-rate:          8.4 W\n  percentage:           73%\n  technology:           lithium-polymer\n  charge-cycles:        212\n')
    real()
if prog == 'busctl':
    if 'ListInhibitors' in args:
        out('{"type":"a(ssssuu)","data":[[["sleep:idle","Firefox","Playing video","block",1000,4242],'
            '["idle","Steam","A game is running","block",1000,5151],["sleep","Hearth downloads","Downloading an update","delay",1000,6161],'
            '["shutdown:sleep","NetworkManager","NetworkManager needs to turn off networks","delay",0,10]]]}\n')
    if 'GetHistory' in args:
        now = int(time.time())
        kind = args[args.index('GetHistory') + 2]
        span = int(args[args.index('GetHistory') + 3])
        pts = []
        for i in range(0, 121):
            t = now - span + span * i // 120
            if kind == 'charge':
                v = 95 - (i % 60) * 1.3 if (i // 60) % 2 == 0 else 20 + (i % 60) * 1.25
            else:
                v = 6 + 4 * ((i * 7) % 11) / 10.0
            pts.append('[%d,%.1f,%d]' % (t, v, 2 if kind == 'charge' else 2))
        out('{"type":"a(udu)","data":[[%s]]}\n' % ','.join(pts))
    if 'CanHibernate' in args:
        out('s "na"\n')
    real()
if prog == 'flatpak':
    if args[:2] == ['list', '--app']:
        out('org.mozilla.firefox\tFirefox\tflathub\ncom.discordapp.Discord\tDiscord\tflathub\ncom.spotify.Client\tSpotify\tflathub\n')
    if args[:2] == ['info', '-M']:
        out('[Application]\nname=%s\n\n[Context]\nshared=network;ipc;\nsockets=x11;wayland;pulseaudio;\ndevices=dri;\n'
            'filesystems=xdg-download;xdg-pictures:ro;\nfeatures=bluetooth;\n' % args[2])
    if args[:3] == ['override', '--user', '--show']:
        out('[Context]\nfilesystems=home;\nsockets=!x11;\n' if args[3] == 'org.mozilla.firefox' else '')
    if args[:4] == ['config', '--user', '--get', 'extra-languages']:
        out('de\n')
    if args[:3] == ['info', '--show-ref', 'org.fcitx.Fcitx5']:
        sys.exit(1)
    real()
if prog == 'bluetoothctl':
    if 'show' in args:
        out('Controller 00:1A:7D:DA:71:13 (public)\n\tName: hearth\n\tAlias: Living room PC\n\tPowered: yes\n\tDiscoverable: no\n')
    if 'devices' in args and 'Paired' in args:
        out('Device AA:BB:CC:11:22:33 Pixel 8\nDevice AA:BB:CC:44:55:66 Xbox Wireless Controller\nDevice AA:BB:CC:77:88:99 WH-1000XM5\n')
    if 'info' in args:
        mac = args[-1]
        d = {'AA:BB:CC:11:22:33': ('Pixel 8', 'phone', 'yes', 'no', 'no', '0x52 (82)', True),
             'AA:BB:CC:44:55:66': ('Xbox Wireless Controller', 'input-gaming', 'yes', 'yes', 'no', '0x3c (60)', False),
             'AA:BB:CC:77:88:99': ('WH-1000XM5', 'audio-headset', 'no', 'yes', 'no', '0x5f (95)', False)}.get(mac)
        if d:
            out('Device %s (public)\n\tName: %s\n\tAlias: %s\n\tIcon: %s\n\tPaired: yes\n\tTrusted: %s\n\tBlocked: %s\n\tConnected: %s\n%s\tBattery Percentage: %s\n'
                % (mac, d[0], d[0], d[1], d[3], d[4], d[2], '\tUUID: OBEX Object Push          (00001105-0000-1000-8000-00805f9b34fb)\n' if d[6] else '', d[5]))
    real()
if prog == 'lpstat':
    if args == ['-e']:
        out('HP_LaserJet_M209\nBrother_DCP_L2550\n')
    if args == ['-p']:
        out('printer HP_LaserJet_M209 now printing HP_LaserJet_M209-41.  enabled since Mon 16 Nov 2026 09:00:00 PM\n'
            'printer Brother_DCP_L2550 is idle.  enabled since Mon 16 Nov 2026 08:00:00 PM\n')
    if args[:2] == ['-W', 'not-completed'] and '-l' not in args:
        out('HP_LaserJet_M209-41     you     482304   Mon 16 Nov 2026 10:02:03 PM\nHP_LaserJet_M209-42     you     10240    Mon 16 Nov 2026 10:02:40 PM\n'
            'HP_LaserJet_M209-43     sam     2048000  Mon 16 Nov 2026 10:03:10 PM\n')
    if args[:2] == ['-W', 'completed']:
        out('Brother_DCP_L2550-38    you     88000    Mon 16 Nov 2026 07:41:00 PM\nHP_LaserJet_M209-40     you     120000   Mon 16 Nov 2026 09:58:00 PM\n')
    if '-l' in args:
        out('HP_LaserJet_M209-42     you     10240    Mon 16 Nov 2026 10:02:40 PM\n\tStatus: job-hold-until-specified\n')
    real()
if prog == 'lpq':
    out('HP_LaserJet_M209 is ready and printing\nRank    Owner   Job     File(s)                         Total Size\n'
        'active  you     41      Tax return 2026.pdf             482304 bytes\n1st     you     42      Boarding pass.pdf               10240 bytes\n'
        '2nd     sam     43      Recipes.odt                     2048000 bytes\n')
if prog == 'scanimage':
    here = os.path.dirname(os.path.abspath(__file__))
    if '-A' in args:
        out('Options specific to device `%s\':\n  Scan mode:\n    --mode Lineart|Gray|Color [Color]\n    --resolution 75|150|300|600dpi [150]\n'
            '    --source Flatbed|ADF [Flatbed]\n' % args[args.index('-d') + 1])
    if '-o' in args:
        import shutil
        n = sum(1 for f in os.listdir(os.path.dirname(args[args.index('-o') + 1])) if f.endswith('.jpg'))
        shutil.copy(os.path.join(here, 'scan-page-%d.jpg' % (n % 2 + 1)), args[args.index('-o') + 1])
        sys.exit(0)
    if args == ['-L']:
        out("device `airscan:e0:HP OfficeJet Pro 9010' is a eSCL HP OfficeJet Pro 9010 ip=192.168.1.20\n"
            "device `brother5:bus2;dev1' is a Brother DCP-L2550DW USB scanner\n")
    real()
if prog == 'fwupdmgr':
    if args[:1] == ['get-history']:
        out('{"Devices":[{"Name":"Unifying Receiver","Vendor":"Logitech","Version":"RQR12.10","UpdateState":2,"Modified":1763300000,'
            '"Releases":[{"Version":"RQR12.11","Summary":"Firmware for the Logitech Unifying receiver"}]},'
            '{"Name":"Samsung SSD 980 PRO 1TB","Vendor":"Samsung","Version":"5B2QGXA7","UpdateState":3,"Modified":1762800000,'
            '"UpdateError":"The drive did not answer after the update; it kept its old firmware","Releases":[{"Version":"5B2QGXA8"}]},'
            '{"Name":"System Firmware","Vendor":"Framework","Version":"3.03","UpdateState":2,"Modified":1755000000,"Releases":[{"Version":"3.05","Summary":"Framework Laptop 13 BIOS"}]}]}\n')
    sys.exit(2)
if prog == 'sensors':
    import json
    out(json.dumps({'k10temp-pci-00c3': {'Adapter': 'PCI adapter', 'Tctl': {'temp1_input': 61.4}, 'Tccd1': {'temp3_input': 55.1}},
                    'amdgpu-pci-0300': {'Adapter': 'PCI adapter', 'edge': {'temp1_input': 48.0, 'temp1_crit': 100.0},
                                        'junction': {'temp2_input': 84.0, 'temp2_crit': 110.0}, 'fan1': {'fan1_input': 1450.0},
                                        'PPT': {'power1_average': 142.0}},
                    'nvme-pci-0100': {'Adapter': 'PCI adapter', 'Composite': {'temp1_input': 39.9, 'temp1_max': 81.8, 'temp1_crit': 84.8}},
                    'nct6775-isa-0290': {'Adapter': 'ISA adapter', 'fan1': {'fan1_input': 980.0}, 'fan2': {'fan2_input': 1220.0},
                                         'SYSTIN': {'temp1_input': 33.0}}}) + '\n')
if prog == 'usbguard':
    sys.exit(1)
real()
