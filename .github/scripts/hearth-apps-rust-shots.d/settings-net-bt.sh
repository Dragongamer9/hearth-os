# Hearth Settings › Network & internet and Bluetooth & devices (hosts/hearth-ui/apps/settings/src/cats/net*.rs, bt*.rs):
# a picture of every page. CI has no NetworkManager, BlueZ, CUPS, KDE Connect or KWin, so first a few pictures of what
# the pages say then, and then every page with a pretend PC: fake nmcli and bluetoothctl on PATH, and a copy of Mantel's
# shell folder where the helpers the pages ask (hearth-network, hearth-bt, hearth-system, hearth-phone, hearth-printq…)
# answer from the fixture below. Nothing here touches the real helpers; opening a page writes nothing.
# Sourced by hearth-apps-rust-test.sh inside its headless screen: run NAME APP_ID REST BINARY ARGS...

run settings-nb-00-wifi-none hearth-settings 0 "$BIN/hearth-settings-app" --page wifi
run settings-nb-01-bluetooth-none hearth-settings 0 "$BIN/hearth-settings-app" --page bluetooth
run settings-nb-02-printers-none hearth-settings 0 "$BIN/hearth-settings-app" --page printers

NBFX=/tmp/netbt-fx
rm -rf "$NBFX"; mkdir -p "$NBFX/bin" "$NBFX/shell" "$NBFX/run"
for f in "$ROOT"/hosts/linux/desktop/shell/*; do ln -s "$f" "$NBFX/shell/$(basename "$f")"; done

cat > "$NBFX/fx.py" <<'PY'
# The pretend PC's answers, as Mantel's helpers and nmcli / bluetoothctl give them.
import json, os, sys

RUN = os.environ.get('NBFX_RUN', '/tmp/netbt-fx/run')
name = os.path.basename(sys.argv[1])
args = sys.argv[2:]
a0 = args[0] if args else ''


def out(v):
    print(json.dumps(v))
    sys.exit(0)


def picture(path, w, h, seed):
    try:
        from PIL import Image, ImageDraw
    except ImportError:
        return
    im = Image.new('RGB', (w, h), (250, 250, 248))
    d = ImageDraw.Draw(im)
    d.rectangle([w * 0.08, h * 0.06, w * 0.92, h * 0.14], fill=(40, 40, 46))
    for i in range(14):
        y = h * 0.2 + i * h * 0.05
        d.rectangle([w * 0.08, y, w * (0.5 + ((i * 7 + seed) % 5) * 0.08), y + h * 0.018], fill=(120, 120, 128))
    d.ellipse([w * 0.6, h * 0.72, w * 0.88, h * 0.9], outline=(220, 90, 40), width=4)
    im.save(path)


def qr_rows(n=29):
    rows = []
    for y in range(n):
        r = ''
        for x in range(n):
            finder = any(x0 <= x < x0 + 7 and y0 <= y < y0 + 7 for x0, y0 in ((0, 0), (n - 7, 0), (0, n - 7)))
            if finder:
                fx, fy = x % (n - 7) if x >= n - 7 else x, y % (n - 7) if y >= n - 7 else y
                ring = fx in (0, 6) or fy in (0, 6) or (2 <= fx <= 4 and 2 <= fy <= 4)
                r += '1' if ring else '0'
            else:
                r += '1' if (x * 7 + y * 13 + x * y) % 5 < 2 else '0'
        rows.append(r)
    return rows


if name == 'nmcli':
    a = ' '.join(args)
    if a == 'radio wifi':
        print('enabled')
    elif a.startswith('-t -f UUID,TYPE,NAME connection show'):
        print('1f2e-home:802-11-wireless:Home-5G\n2a3b-cafe:802-11-wireless:Corner Cafe\n3c4d-wired:802-3-ethernet:Wired connection 1')
    elif '802-11-wireless.ssid connection show uuid 1f2e-home' in a or '802-11-wireless.ssid connection show id Home-5G' in a:
        print('Home-5G')
    elif '802-11-wireless.ssid connection show uuid 2a3b-cafe' in a:
        print('Corner Cafe')
    elif a.startswith('-t -f IN-USE,SSID,SIGNAL,SECURITY,FREQ device wifi list'):
        print('*:Home-5G:86:WPA2 WPA3:5180 MHz\n :Home-5G:58:WPA2 WPA3:2437 MHz\n :Corner Cafe:64:WPA2:2412 MHz\n :Neighbours_Fiber:49:WPA2:5240 MHz\n'
              ' :Library Guest:38:--:2462 MHz\n :DIRECT-Printer:31:WPA2:2437 MHz\n :Hidden Lab:22:WPA2:5745 MHz\n :Apartment 4B:18:WPA2:2412 MHz')
    elif a == '-t -f DEVICE,TYPE device':
        print('wlan0:wifi\nenp3s0:ethernet\nlo:loopback')
    elif a == '-g IP4.ADDRESS device show wlan0':
        print('192.168.1.20/24')
    elif a == '-g GENERAL.STATE device show wlan0':
        print('100 (connected)')
    elif a == '-g GENERAL.CONNECTION device show wlan0':
        print('Home-5G')
    elif a.startswith('-t -f TYPE,STATE,CONNECTION dev'):
        print('wifi:connected:Home-5G\nethernet:unavailable:\nloopback:connected (externally):lo')
    elif a.startswith('-t -f IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS,GENERAL.HWADDR device show'):
        print('GENERAL.HWADDR:3C\\:A9\\:F4\\:12\\:8B\\:07\nIP4.ADDRESS[1]:192.168.1.20/24\nIP4.GATEWAY:192.168.1.1\nIP4.DNS[1]:192.168.1.1\nIP6.ADDRESS[1]:fe80::3ea9:f4ff:fe12:8b07/64')
    sys.exit(0)

if name == 'bluetoothctl':
    a = ' '.join(x for x in args if not x.startswith('--timeout') and not x.isdigit())
    if a == 'show':
        print('Controller 5C:F3:70:9A:11:02 (public)\n\tName: chris-hearth\n\tPowered: yes\n\tDiscoverable: no')
    elif a == 'devices Paired':
        print('Device 98:B6:E9:01:22:33 Xbox Wireless Controller\nDevice 38:18:4C:AA:BB:CC WH-1000XM5\nDevice F4:7B:09:12:34:56 Pixel 8')
    elif a == 'devices Connected':
        print('Device 98:B6:E9:01:22:33 Xbox Wireless Controller')
    elif a == 'devices':
        print('Device 98:B6:E9:01:22:33 Xbox Wireless Controller\nDevice 38:18:4C:AA:BB:CC WH-1000XM5\nDevice F4:7B:09:12:34:56 Pixel 8\n'
              'Device 7C:2F:80:44:55:66 JBL Flip 6\nDevice 11-22-33-44-55-66 11-22-33-44-55-66')
    sys.exit(0)

if name == 'hearth-network':
    conns = [
        {'name': 'Home-5G', 'uuid': '1f2e-home', 'type': '802-11-wireless', 'kind': 'wifi', 'device': 'wlan0', 'active': True, 'autoconnect': True},
        {'name': 'Corner Cafe', 'uuid': '2a3b-cafe', 'type': '802-11-wireless', 'kind': 'wifi', 'device': '', 'active': False, 'autoconnect': True},
        {'name': 'Wired connection 1', 'uuid': '3c4d-wired', 'type': '802-3-ethernet', 'kind': 'ethernet', 'device': '', 'active': False, 'autoconnect': True},
        {'name': 'Home VPN', 'uuid': '4e5f-wg', 'type': 'wireguard', 'kind': 'vpn', 'device': 'wg0', 'active': True, 'autoconnect': False, 'vpnType': 'WireGuard'},
        {'name': 'Work (OpenVPN)', 'uuid': '5a6b-ovpn', 'type': 'vpn', 'kind': 'vpn', 'device': '', 'active': False, 'autoconnect': False, 'vpnType': 'OpenVPN'},
    ]
    if a0 == 'status':
        types = [('ipsec', 'IPsec (IKEv2)', True, 'strongswan'), ('l2tp', 'L2TP/IPsec', True, 'l2tp'), ('pptp', 'PPTP', False, ''),
                 ('openconnect', 'OpenConnect (Cisco AnyConnect, GlobalProtect, Pulse)', True, 'openconnect'), ('sstp', 'SSTP', False, ''),
                 ('fortisslvpn', 'Fortinet SSL VPN', False, '')]
        out({'connections': conns, 'vpns': [c for c in conns if c['kind'] == 'vpn'], 'wifiDevice': 'wlan0',
             'plugins': {'wireguard': True, 'openvpn': True}, 'devices': [{'device': 'wlan0', 'type': 'wifi', 'state': 'connected'}, {'device': 'enp3s0', 'type': 'ethernet', 'state': 'unavailable'}],
             'vpnTypes': [{'id': i, 'label': l, 'installed': ins, 'plugin': pl, 'package': 'network-manager-' + i} for i, l, ins, pl in types],
             'hasModem': False, 'hasEthernet': True, 'qr': True})
    if a0 == 'details':
        out({'uuid': '1f2e-home', 'name': 'Home-5G', 'type': '802-11-wireless', 'kind': 'wifi', 'autoconnect': True, 'metered': 'unknown',
             'ipv4': {'method': 'auto', 'address': '', 'gateway': ''}, 'dns': [], 'autoDns': True, 'ipv6': 'auto', 'mac': 'stable', 'macFixed': '',
             'now': {'device': 'wlan0', 'address': '192.168.1.20/24', 'gateway': '192.168.1.1', 'dns': ['192.168.1.1'], 'address6': 'fe80::3ea9:f4ff:fe12:8b07/64'},
             'mtu': 0, 'enterprise': None, 'mobile': None, 'ssid': 'Home-5G', 'security': 'WPA'})
    if a0 == 'wifi-qr':
        rows = qr_rows()
        out({'ssid': 'Home-5G', 'security': 'WPA', 'password': 'ember-glow-2026', 'size': len(rows), 'rows': rows})
    if a0 == 'hotspot':
        out({'on': False, 'ssid': 'chris-hearth hotspot', 'band': 'a', 'uuid': '9e8d-hs', 'device': 'wlan0', 'canHotspot': True})
    if a0 == 'proxy':
        out({'mode': 'manual', 'url': '', 'http': 'proxy.home.lan:3128', 'https': 'proxy.home.lan:3128', 'socks': '', 'ignore': 'localhost, 127.0.0.1, .home.lan'})

if name == 'hearth-home-server':
    if a0 == 'status':
        out({'host': 'chris-hearth', 'wired': [{'iface': 'enp3s0', 'mac': '3c:a9:f4:12:8b:08'}], 'applied': {'ok': True},
             'settings': {'tsOnly': True, 'tsBoot': True, 'exitNode': False, 'taildropReceive': True, 'wakeFromAway': False},
             'tailscale': {'installed': True, 'running': True, 'state': 'Running', 'login': 'chris@example.com', 'tailnet': 'tail1234.ts.net',
                           'dns': 'chris-hearth.tail1234.ts.net', 'name': 'chris-hearth', 'ips': ['100.101.12.7'],
                           'peers': [{'name': 'pixel-8', 'os': 'android', 'ips': ['100.88.4.21'], 'online': True},
                                     {'name': 'rog-ally', 'os': 'linux', 'ips': ['100.92.31.2'], 'online': True},
                                     {'name': 'work-laptop', 'os': 'windows', 'ips': ['100.70.8.44'], 'online': False}]}})
    if a0 == 'ts-ping':
        out({'pings': {'pixel-8': {'ms': 18, 'via': 'direct'}, 'rog-ally': {'ms': 4, 'via': 'direct'}}})

if name == 'hearth-bt':
    if a0 == 'devices':
        out({'devices': [
            {'mac': '98:B6:E9:01:22:33', 'name': 'Xbox Wireless Controller', 'icon': 'input-gaming', 'connected': True, 'trusted': True, 'blocked': False, 'battery': 80, 'canSend': False},
            {'mac': '38:18:4C:AA:BB:CC', 'name': 'WH-1000XM5', 'icon': 'audio-headset', 'connected': False, 'trusted': True, 'blocked': False, 'battery': 55, 'canSend': False},
            {'mac': 'F4:7B:09:12:34:56', 'name': 'Pixel 8', 'icon': 'phone', 'connected': False, 'trusted': False, 'blocked': False, 'battery': None, 'canSend': True},
            {'mac': '00:1A:7D:DA:71:13', 'name': 'Old Keyboard', 'icon': 'input-keyboard', 'connected': False, 'trusted': False, 'blocked': True, 'battery': None, 'canSend': False}]})

if name == 'hearth-bt-receive':
    out({'present': True, 'powered': True, 'name': 'Living room PC', 'systemName': 'chris-hearth', 'address': '5C:F3:70:9A:11:02'})

if name == 'hearth-system':
    if a0 == 'keyboard':
        out({'layouts': [{'layout': 'us', 'variant': '', 'name': 'English (US)'}, {'layout': 'de', 'variant': '', 'name': 'German'}],
             'switch': 'grp:alt_shift_toggle', 'delay': 600, 'rate': 25, 'kwin': True, 'numlock': 'on', 'compose': 'compose:ralt', 'switchMode': None})
    if a0 == 'keyboard-layouts':
        out({'layouts': [{'layout': 'us', 'variant': '', 'name': 'English (US)'}, {'layout': 'gb', 'variant': '', 'name': 'English (UK)'},
                         {'layout': 'de', 'variant': '', 'name': 'German'}, {'layout': 'fr', 'variant': '', 'name': 'French'}]})
    if a0 == 'shortcuts':
        out({'shortcuts': [{'keys': 'Super', 'action': 'Open the start menu'}, {'keys': 'Super+E', 'action': 'Open Files'},
                           {'keys': 'Super+W', 'action': 'Overview of windows'}, {'keys': 'Ctrl+Alt+T', 'action': 'Open the terminal'}]})
    if a0 == 'pointer':
        out({'mouse': {'speed': 0.2, 'natural': False, 'leftHanded': False, 'accel': 'flat', 'scrollSpeed': 1.4, 'middle': False},
             'touchpad': {'speed': 0.0, 'tap': True, 'natural': True, 'scroll': 'twofinger', 'scrollSpeed': 1.0, 'accel': 'adaptive', 'tapDrag': True, 'click': 'fingers', 'dwt': True},
             'hasTouchpad': True})
    if a0 == 'printers':
        out({'cups': True, 'canAdmin': True, 'configTool': False, 'printers': [
            {'name': 'HP_LaserJet_M404', 'info': 'HP LaserJet Pro M404', 'uri': 'ipp://192.168.1.40/ipp/print', 'state': 'idle', 'reason': '', 'location': 'Office', 'default': True},
            {'name': 'Canon_TS6350', 'info': 'Canon PIXMA TS6350', 'uri': 'usb://Canon/TS6300', 'state': 'printing', 'reason': 'Printing page 2', 'location': '', 'default': False}]})

if name == 'hearth-kwin-bridge':
    out({'count': 3})

if name == 'hearth-kwin-settings':
    if a0 == 'status':
        out({'live': True, 'running': True, 'preview': False, 'version': '6.5.2', 'wm': 'kwin'})
    if a0 == 'get':
        out({'files': {'kcminputrc': {'ButtonRebinds][Mouse': {'ExtraButton1': 'Key,Ctrl+Z', 'ExtraButton4': 'Key,Meta+W'}},
                       'kwinrc': {'Wayland': {'EnablePrimarySelection': 'true'}}}, 'mine': {}, 'bad': {}})
    if a0 == 'input':
        out({'devices': [
            {'sysName': 'event7', 'name': 'Wacom Intuos S Pen', 'tabletTool': True, 'mapToWorkspace': False, 'outputName': 'DP-1', 'supportsOutputArea': True,
             'outputArea': [0, 0, 1, 1], 'supportsInputArea': True, 'inputArea': [0.25, 0.25, 0.5, 0.5], 'supportsLeftHanded': True, 'leftHanded': False,
             'pressureCurve': '0,0.35;0.4,1;', 'supportsPressureRange': True, 'pressureRangeMin': 0.05, 'pressureRangeMax': 0.9},
            {'sysName': 'event8', 'name': 'Wacom Intuos S Pad', 'tabletPad': True, 'tabletPadButtonCount': 4},
            {'sysName': 'event9', 'name': 'ELAN Touchscreen', 'touch': True, 'supportsDisableEvents': True, 'enabled': True, 'outputName': 'eDP-1', 'orientationDBus': 0,
             'supportsCalibrationMatrix': True, 'calibrationMatrix': '1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1', 'defaultCalibrationMatrix': '1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1'}]})
    if a0 == 'outputs':
        out({'outputs': [{'name': 'DP-1', 'model': 'LG 27GP850'}, {'name': 'eDP-1', 'model': 'Built-in screen'}]})

if name == 'hearth-phone':
    if a0 == 'state':
        out({'installed': True, 'running': True, 'state': {
            'name': 'chris-hearth', 'folder': os.path.expanduser('~/Downloads/Phone'), 'tcpPort': 1716,
            'commands': [{'key': 'c1', 'name': 'Lock the screen', 'command': 'loginctl lock-session'}],
            'notifyApps': ['Discord', 'Steam'], 'notifyOut': {'body': True, 'persistentOnly': False, 'urgency': 0, 'blocked': []},
            'texts': [{'device': 'p8', 'url': 'https://hearthos.fun', 'text': '', 'from': 'Pixel 8', 'at': 1763480000000}],
            'devices': [
                {'id': 'p8', 'name': 'Pixel 8', 'paired': True, 'reachable': True, 'remoteInput': True, 'keyboard': False,
                 'battery': {'charge': 76, 'charging': False}, 'plugins': {'signal': {'0': {'networkType': '5G', 'signalStrength': 3}}, 'sms': {'threads': 12}},
                 'options': {'notifications': True, 'clipboard': True, 'input': True, 'pauseOnCall': True, 'commands': True, 'volume': True},
                 'players': ['Spotify'], 'playing': {'Spotify': {'title': 'Concerning Hobbits', 'artist': 'Howard Shore', 'isPlaying': True}},
                 'notifications': [{'id': 'n1', 'appName': 'Messages', 'title': 'Dad', 'text': 'Trains 2010 is running great on the new PC!', 'replyId': 'r1', 'actions': ['Mark as read']},
                                   {'id': 'n2', 'appName': 'Calendar', 'title': 'Work at 10 PM', 'text': 'Overnight shift', 'actions': []}]},
                {'id': 'tab', 'name': 'Galaxy Tab A9', 'paired': False, 'reachable': True, 'pair': {'state': 'asking', 'key': '4F 9A 21'}}]}})
    if a0 == 'call' and len(args) > 1 and args[1] == 'sms_threads':
        out({'list': [{'thread': 1, 'names': ['Dad'], 'addresses': ['+15550100'], 'last': 'Trains 2010 is running great on the new PC!', 'sent': False, 'date': 1763480000000, 'unread': 1},
                      {'thread': 2, 'names': ['Work'], 'addresses': ['+15550123'], 'last': 'See you at 10', 'sent': True, 'date': 1763390000000, 'unread': 0}]})
    if a0 == 'call' and len(args) > 1 and args[1] == 'sms_thread':
        out({'list': [{'body': 'Did the new Hearth update install?', 'sent': False, 'date': 1763470000000},
                      {'body': 'Yes, all good. Try the train game now', 'sent': True, 'date': 1763471000000},
                      {'body': 'Trains 2010 is running great on the new PC!', 'sent': False, 'date': 1763480000000}]})
    out({'ok': True})

if name == 'hearth-printq':
    if a0 == 'jobs':
        out({'printers': [{'name': 'HP_LaserJet_M404', 'state': 'idle', 'reason': ''}, {'name': 'Canon_TS6350', 'state': 'printing', 'reason': ''}],
             'jobs': [{'id': 'Canon_TS6350-41', 'num': 41, 'printer': 'Canon_TS6350', 'title': 'Boarding pass.pdf', 'user': 'chris', 'size': 184320, 'when': '9:41 PM', 'state': 'printing', 'mine': True},
                      {'id': 'Canon_TS6350-42', 'num': 42, 'printer': 'Canon_TS6350', 'title': 'Recipe — Lembas.docx', 'user': 'chris', 'size': 40960, 'when': '9:42 PM', 'state': 'held', 'mine': True}],
             'done': [{'printer': 'HP_LaserJet_M404', 'num': 40, 'user': 'chris', 'size': 2621440, 'when': 'Yesterday 6:02 PM'}]})
    if a0 == 'scanners':
        out({'scanners': [{'name': 'Canon PIXMA TS6350 (network)', 'device': 'airscan:e0:Canon TS6350'}, {'name': 'HP ScanJet 2400', 'device': 'hpaio:/usb/ScanJet'}]})
    if a0 == 'scan-test':
        f = os.path.join(RUN, 'test.png')
        picture(f, 425, 550, 1)
        out({'file': f})

if name == 'hearth-scan':
    if a0 == 'options':
        out({'modes': ['Color', 'Gray', 'Lineart'], 'sources': ['Flatbed', 'ADF'], 'resolutions': [75, 150, 300, 600]})
    if a0 == 'pages':
        pages = []
        for i in range(3):
            f = os.path.join(RUN, 'page-%d.jpg' % i)
            picture(f, 260, 340, i)
            pages.append({'path': f})
        out({'pages': pages})

if name == 'hearth-ink':
    out({'printers': [
        {'name': 'Canon_TS6350', 'markers': [{'name': 'Black', 'level': 62, 'low': 15, 'color': '000000'}, {'name': 'Cyan', 'level': 34, 'low': 15, 'color': '00a0e0'},
                                             {'name': 'Magenta', 'level': 9, 'low': 15, 'color': 'e0007a'}, {'name': 'Yellow', 'level': 71, 'low': 15, 'color': 'f5d000'}]},
        {'name': 'HP_LaserJet_M404', 'markers': [{'name': 'Black toner', 'level': -1}]}]})

if name == 'hearth-usbdev':
    out({'usbguard': {'running': True}, 'devices': [
        {'name': 'Logitech G502', 'kind': 'Mouse', 'maker': 'Logitech', 'speed': '12 Mb/s', 'vendor': '046d', 'product': 'c08b', 'authorized': True, 'guard': {'id': '3', 'target': 'allow'}},
        {'name': 'SanDisk Ultra Fit', 'kind': 'Storage', 'maker': 'SanDisk', 'speed': '5 Gb/s', 'vendor': '0781', 'product': '5583', 'authorized': False, 'guard': {'id': '7', 'target': 'block'}},
        {'name': 'Xbox Wireless Adapter', 'kind': 'Controller or input', 'maker': 'Microsoft', 'speed': '480 Mb/s', 'vendor': '045e', 'product': '02fe', 'authorized': True, 'guard': {'id': '4', 'target': 'allow'}}]})

if name == 'hearth-thunderbolt':
    out({'available': True, 'security': 'user', 'devices': [
        {'uuid': 'd1', 'vendor': 'CalDigit', 'name': 'TS4 Dock', 'connected': True, 'authorized': True, 'stored': True, 'generation': 'Thunderbolt 4'},
        {'uuid': 'd2', 'vendor': 'Samsung', 'name': 'T7 Shield', 'connected': True, 'authorized': False, 'stored': False, 'generation': 'USB4'}]})

out({})
PY

# The helpers' stand-ins (Python, as the pages run `python3 HELPER ARGS`), and nmcli / bluetoothctl on PATH.
for h in hearth-network hearth-home-server hearth-bt hearth-bt-receive hearth-system hearth-kwin-bridge hearth-kwin-settings hearth-phone hearth-printq hearth-scan hearth-ink hearth-usbdev hearth-thunderbolt; do
  rm -f "$NBFX/shell/$h"
  printf 'import sys\nsys.argv.insert(1, "%s")\nexec(open("%s/fx.py").read())\n' "$h" "$NBFX" > "$NBFX/shell/$h"
done
for b in nmcli bluetoothctl; do
  printf '#!/bin/sh\nexec python3 %s/fx.py %s "$@"\n' "$NBFX" "$b" > "$NBFX/bin/$b"
  chmod +x "$NBFX/bin/$b"
done
chmod -R a+rwX "$NBFX"

nb() { # nb NAME PAGE: a picture of PAGE on the pretend PC
  run "settings-nb-$1" hearth-settings 0 env PATH="$NBFX/bin:$PATH" HEARTH_SHELL_DIR="$NBFX/shell" NBFX_RUN="$NBFX/run" "$BIN/hearth-settings-app" --page "$2"
}
nb 10-wifi wifi
nb 11-wifi-properties wifi:props
nb 12-wifi-hidden wifi:hidden
nb 13-tailscale hs-tailscale
nb 14-vpn vpn
nb 15-vpn-by-hand vpn:add-l2tp
nb 16-hotspot hotspot
nb 17-proxy proxy
nb 18-net-details net-details
nb 19-net-details-qr net-details:qr
nb 20-net-details-enterprise net-details:add-enterprise
nb 30-category bt
nb 31-bluetooth bluetooth
nb 32-bt-devices bluetooth-devices:first
nb 33-bt-receive bluetooth-receive
nb 34-keyboard keyboard
nb 35-keyboard-osk keyboard:osk
nb 36-mouse mouse
nb 37-mouse-buttons mouse-buttons
nb 38-mouse-buttons-press mouse-buttons:press
nb 39-phone phone
nb 40-phone-messages phone-messages
nb 41-phone-thread phone-messages:1
nb 42-tablet kwin-tablet
nb 43-printers printers
nb 44-print-queue print-queue
nb 45-scanners scanners:test
nb 46-scan-pdf scan-pdf
nb 47-ink-levels ink-levels
nb 48-usb-devices usb-devices
nb 49-thunderbolt thunderbolt
