"""1.14 round 2 for Mantel (KDE Plasma's features written for Hearth, on KWin and labwc): notifications (the history
across restarts, automatic Do Not Disturb, job progress, Unity launcher badges, actions kept, sounds, the OSDs, Hearth's
own warnings never blocked) and widgets and panels (the bar's own items in any order, more panels, binary and fuzzy
clocks, other calendars and holidays, web page / calculator / colour picker / keyboard / pager / window list widgets,
the tray's printers, disks, camera / mic and background apps, a light / dark tile).
The JavaScript rules run with node; hearth-notifyd's readers run here; the QML wiring is checked as text. The real
thing runs in .github/workflows/notify-widgets.yml (labwc and KWin, with pictures)."""
import importlib.machinery
import importlib.util
import json
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest
import wave

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent.parent
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
SHELL = DESK / 'shell'
NODE = shutil.which('node')


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def run_js(lib, body):
    src = (SHELL / lib).read_text().replace('.pragma library', '')
    r = subprocess.run([NODE, '-e', src + '\n;' + body], capture_output=True, text=True, timeout=30)
    if r.returncode != 0:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout.strip().splitlines()[-1])


def code(text):
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


def read(name):
    return (SHELL / name).read_text()


@unittest.skipUnless(NODE, 'node is needed for the calendars')
class Calendars(unittest.TestCase):
    def test_known_dates(self):
        r = run_js('calendars.js', '''const d = (y, m, x) => new Date(y, m - 1, x);
            console.log(JSON.stringify([convert(d(2016, 4, 11), "persian").text, convert(d(2026, 3, 21), "persian").text,
              convert(d(2026, 9, 12), "hebrew").text, convert(d(2025, 4, 13), "hebrew").text, convert(d(2024, 3, 11), "hebrew").text,
              convert(d(2026, 2, 18), "islamic").text, convert(d(2026, 1, 7), "julian").text,
              convert(d(2026, 3, 22), "indian").text, convert(d(2026, 11, 16), "indian").text, convert(d(2026, 1, 1), "") ]));''')
        self.assertEqual(r[0], '23 Farvardin 1395')
        self.assertEqual(r[1], '1 Farvardin 1405')         # Nowruz 2026
        self.assertEqual(r[2], '1 Tishri 5787')            # Rosh Hashanah 2026
        self.assertEqual(r[3], '15 Nisan 5785')            # Passover 2025
        self.assertEqual(r[4], '1 Adar II 5784')           # a leap year's second Adar
        self.assertEqual(r[5], '1 Ramadan 1447')
        self.assertEqual(r[6], '25 December 2025')         # Orthodox Christmas
        self.assertEqual(r[7], '1 Chaitra 1948')
        self.assertEqual(r[8], '25 Kartika 1948')
        self.assertIsNone(r[9])

    def test_holidays(self):
        r = run_js('calendars.js', '''console.log(JSON.stringify([holidays("us", 2026), holidays("gb", 2026), holidays("de", 2026),
            holidays("jewish", 2026), holidays("islamic", 2026), holidays("nl", 2025), easter(2025), holidays("", 2026)]));''')
        us = {h['name']: h['date'] for h in r[0]}
        self.assertEqual(us['Thanksgiving'], '2026-11-26')
        self.assertEqual(us['Memorial Day'], '2026-05-25')
        self.assertEqual(us['Labor Day'], '2026-09-07')
        gb = {h['name']: h['date'] for h in r[1]}
        self.assertEqual(gb['Good Friday'], '2026-04-03')
        self.assertEqual(gb['Summer bank holiday'], '2026-08-31')
        de = {h['name']: h['date'] for h in r[2]}
        self.assertEqual(de['Christi Himmelfahrt'], '2026-05-14')
        jw = {h['name']: h['date'] for h in r[3]}
        self.assertEqual(jw['Rosh Hashanah'], '2026-09-12')
        self.assertEqual(jw['Passover begins'], '2026-04-02')
        self.assertEqual(jw['Hanukkah begins'], '2026-12-05')
        isl = {h['name']: h['date'] for h in r[4]}
        self.assertEqual(isl['Eid al-Fitr'], '2026-03-20')
        nl = {h['name']: h['date'] for h in r[5]}
        self.assertEqual(nl['Koningsdag'], '2025-04-26')     # 27 April 2025 was a Sunday
        self.assertEqual(r[6], [4, 20])
        self.assertEqual(r[7], [])
        self.assertEqual([h['date'] for h in r[0]], sorted(h['date'] for h in r[0]))


@unittest.skipUnless(NODE, 'node is needed for the widget rules')
class WidgetTools(unittest.TestCase):
    def test_calculator_is_safe_and_right(self):
        r = run_js('widget-tools.js', '''const e = ["1+2*3", "(1+2)*3", "2^3^2", "50%", "10/0", "sin(90)", "0.1+0.2", "2(3+4)", "alert(1)", "1+", "",
            "this.constructor", "sqrt(16)+2pi"];
            console.log(JSON.stringify(e.map(x => { const c = calc(x, true); return c.error !== undefined ? "!" : fmt(c.value); })));''')
        self.assertEqual(r, ['7', '9', '512', '0.5', '!', '1', '0.3', '14', '!', '!', '', '!', '10.2831853072'])
        self.assertNotIn('eval(', code(read('widget-tools.js')))
        self.assertNotIn('Function(', code(read('widget-tools.js')))

    def test_clocks(self):
        r = run_js('widget-tools.js', '''console.log(JSON.stringify([fuzzy(15, 7, 1), fuzzy(15, 52, 1), fuzzy(23, 58, 1), fuzzy(11, 58, 2), fuzzy(15, 0, 3),
            binaryColumns(13, 47, 9, true), binaryColumns(9, 5, 0, false)]))''')
        self.assertEqual(r[:5], ['Five past three', 'Ten to four', 'Midnight', 'Noon', 'Afternoon'])
        self.assertEqual(r[5], [[0, 1], [0, 0, 1, 1], [1, 0, 0], [0, 1, 1, 1], [0, 0, 0], [1, 0, 0, 1]])
        self.assertEqual(len(r[6]), 4)

    def test_colours(self):
        r = run_js('widget-tools.js', 'console.log(JSON.stringify([formats("#ff8a3d"), argbToHex(0xff66c0f4), parseHex("abc"), parseHex("nope")]))')
        self.assertEqual(r[0][:3], ['#FF8A3D', 'rgb(255, 138, 61)', 'hsl(24, 100%, 62%)'])
        self.assertEqual(r[1], '#66c0f4')
        self.assertEqual(r[2], {'r': 170, 'g': 187, 'b': 204})
        self.assertIsNone(r[3])

    def test_bar_items_every_one_once_flame_kept_new_ones_in_place(self):
        r = run_js('widget-tools.js', '''const d = barLayout(null);
            console.log(JSON.stringify([d, barLayout({start: ["window", "menu", "menu"], end: ["notifications", "volume"], hidden: ["tray", "zzz", "menu"]}),
              barMove(null, "volume", -1), barMove(null, "network", 0, "start"), barMove(null, "menu", 0, "hidden"), isDefaultLayout(null),
              BAR_ITEMS.map(b => b.id)]));''')
        d, odd, up, moved, hid, isdef, ids = r
        self.assertEqual(sorted(d['start'] + d['end']), sorted(ids))
        self.assertEqual(d['start'][0], 'menu')
        self.assertEqual(d['end'][-1], 'notifications')
        self.assertEqual(d['hidden'], [])
        self.assertEqual(odd['start'][:2], ['window', 'menu'])              # your order is kept
        self.assertEqual(odd['hidden'], ['tray'])                           # the flame can't be hidden, unknown ids go
        self.assertEqual(sorted(odd['start'] + odd['end'] + odd['hidden']), sorted(ids))
        self.assertLess(up['end'].index('volume'), up['end'].index('bluetooth'))
        self.assertIn('network', moved['start'])
        self.assertNotIn('network', moved['end'])
        self.assertEqual(hid['hidden'], [])
        self.assertTrue(isdef)

    def test_panels(self):
        r = run_js('widget-tools.js', '''console.log(JSON.stringify([panelOf("a", {edge: "left", items: ["clock", "nope", "pager"], size: 500}), panelOf("b", "x"),
            panelClash([], "HDMI-1", "top", "HDMI-1", "top", true), panelClash([], "HDMI-1", "bottom", "HDMI-1", "top", true),
            panelClash([{screen: "HDMI-1", edge: "left"}], "HDMI-1", "left", "HDMI-1", "top", false), panelClash([], "DP-2", "top", "HDMI-1", "top", true),
            cloneMain(null)]))''')
        self.assertEqual(r[0]['items'], ['clock', 'pager'])
        self.assertEqual(r[0]['size'], 44)
        self.assertIsNone(r[1])
        self.assertEqual(r[2], 'The main bar is there')
        self.assertEqual(r[3], 'The dock is there')
        self.assertEqual(r[4], "There's a panel there already")
        self.assertEqual(r[5], '')
        self.assertIn('gamemode', r[6])


class NotifyHelper(unittest.TestCase):
    def setUp(self):
        self.d = load(SHELL / 'hearth-notifyd', 'hearth_notifyd')

    def test_jobs_v1_and_v3(self):
        d = self.d
        j = {'id': 1, 'app': 'Dolphin', 'caps': 3}
        for m, a in (('setInfoMessage', ('Copying',)), ('setDescriptionField', (0, 'Source', '~/a.mkv')),
                     ('setTotalAmount', (1000, 'bytes')), ('setProcessedAmount', (250, 'bytes')), ('setSpeed', (50,))):
            self.assertTrue(d.job_apply_v2(j, m, a))
        v = d.job_view(j)
        self.assertEqual((v['percent'], v['processed'], v['total'], v['title'], v['killable'], v['suspendable']), (25, 250, 1000, 'Source', True, True))
        d.job_apply_v2(j, 'terminate', ('',))
        self.assertEqual(d.job_view(j)['state'], 'done')
        k = {'id': 2, 'caps': 0}
        d.job_apply_v3(k, {'title': 'Copying 214 files', 'totalBytes': 400, 'processedBytes': 100, 'totalFiles': 214, 'processedFiles': 50,
                           'descriptionLabel1': 'Destination', 'descriptionValue1': '/media/Backup'})
        v = d.job_view(k)
        self.assertEqual((v['title'], v['percent'], v['unit'], v['killable']), ('Copying 214 files', 25, 'bytes', False))
        d.job_terminate_v3(k, 1, '')
        self.assertEqual(k['state'], 'cancelled')
        d.job_terminate_v3(k, 117, 'Disk full')
        self.assertEqual((k['state'], k['error']), ('failed', 'Disk full'))

    def test_tray_readers(self):
        d = self.d
        lsblk = {'blockdevices': [
            {'name': 'nvme0n1', 'path': '/dev/nvme0n1', 'rm': False, 'hotplug': False, 'type': 'disk', 'children': [
                {'name': 'nvme0n1p2', 'path': '/dev/nvme0n1p2', 'rm': False, 'type': 'part', 'fstype': 'ext4', 'mountpoints': ['/sysroot']}]},
            {'name': 'sda', 'path': '/dev/sda', 'model': 'SanDisk Ultra ', 'rm': True, 'hotplug': True, 'type': 'disk', 'tran': 'usb', 'children': [
                {'name': 'sda1', 'path': '/dev/sda1', 'label': 'USB STICK', 'size': '29.8G', 'rm': True, 'type': 'part', 'fstype': 'vfat', 'mountpoints': ['/run/media/me/USB STICK']}]},
            {'name': 'sdb', 'path': '/dev/sdb', 'model': 'Card', 'rm': True, 'type': 'disk', 'children': [
                {'name': 'sdb1', 'path': '/dev/sdb1', 'rm': True, 'type': 'part', 'fstype': 'exfat', 'mountpoints': [None]}]}]}
        out = d.parse_lsblk(json.dumps(lsblk))
        self.assertEqual([x['path'] for x in out], ['/dev/sda1', '/dev/sdb1'])
        self.assertEqual(out[0]['name'], 'USB STICK')
        self.assertEqual(out[0]['mount'], '/run/media/me/USB STICK')
        self.assertEqual(out[0]['drive'], '/dev/sda')
        self.assertEqual(out[1]['mount'], '')
        self.assertEqual(d.parse_lsblk('not json'), [])
        jobs = d.parse_lpstat('Office-12 me 1024 Mon 16 Nov 2026 10:00:00\nOffice-13 me 2048 Mon 16 Nov\nnonsense\n')
        self.assertEqual([(j['job'], j['printer'], j['size']) for j in jobs], [('Office-12', 'Office', '1024'), ('Office-13', 'Office', '2048')])

    def test_never_writes_a_setting_and_asks_safely(self):
        t = (SHELL / 'hearth-notifyd').read_text()
        self.assertNotRegex(t, r"open\([^)]*['\"]w['\"]")   # only its log (append) under the runtime folder
        self.assertNotIn('shell=True', t)
        self.assertIn("if not path.startswith('/dev/'):", t)
        self.assertIn("Gio.BusNameOwnerFlags.NONE", t)       # never takes the job server from a real plasmashell
        self.assertIn('watch_senders', t)                     # a job whose app quit ends instead of staying forever


class Notifications(unittest.TestCase):
    def test_hearths_own_warnings_are_never_blocked(self):
        q = code(read('Notifications.qml'))
        dec = q[q.index('function decide(n)'):q.index('NotificationServer {')]
        self.assertIn('if (!rule.allow && !warn) { n.dismiss(); return; }', dec)
        # the warning wins before every rule that keeps a notification quiet
        self.assertLess(dec.index('if (warn) pop = true;'), dec.index('rule.priority === "low"'))
        self.assertLess(dec.index('if (warn) pop = true;'), dec.index('DeskState.isMuted'))
        self.assertLess(dec.index('if (warn) pop = true;'), dec.index('DeskState.dnd && !urgent'))
        self.assertLess(dec.index('if (warn) pop = true;'), dec.index('NotifyHub.autoReason'))
        r = read('NotifyRules.qml')
        self.assertIn('function isHearthWarning(n)', r)
        self.assertIn('return n.urgency === 2 ||', r)
        ds = read('DeskState.qml')
        self.assertIn('if (on && NotifyRules.isHearth(app)) { SettingsNav.toast("Hearth\'s own warnings always show"); return; }', ds)
        self.assertIn('!NotifyRules.isHearth(card.app)', q)          # no mute bell on Hearth's own pop-ups

    def test_history_kept_atomically_and_buttons_kept(self):
        ds = read('DeskState.qml')
        self.assertIn('SettingsWriter { id: histWriter; path: root.historyPath }', ds)
        self.assertIn('/nimbus/notifications.json', ds)
        self.assertIn('o["n:" + h.key] = serial(h);', ds)            # one key per notification, merged
        self.assertIn('histWriter.repair()', ds)                     # a damaged file is put back, never overwritten
        self.assertNotIn('setText(', code(ds))
        self.assertIn('n.closed.connect(() => root.gone(key));', ds)
        self.assertIn('a.invoke();', ds)
        self.assertIn('if (n.transient) return item;', ds)
        q = read('Notifications.qml')
        self.assertIn('n.tracked = true;', q)                       # alive while in the history (its buttons work)
        self.assertIn('onTriggered: DeskState.unpop(card.modelData)', q)   # a pop-up's timeout only hides it
        cp = read('ClockPanel.qml')
        self.assertIn('DeskState.invoke(card.modelData.key, parent.modelData.identifier)', cp)

    def test_automatic_quiet_never_touches_your_switch(self):
        h = code(read('NotifyHub.qml'))
        self.assertNotRegex(h, r'DeskState\.dnd\s*=(?!=)')
        self.assertNotIn('setDnd(', h)
        self.assertIn('readonly property bool quiet: DeskState.dnd || autoReason !== ""', h)
        for k in ('autoFullscreen: true', 'autoSharing: true', 'schedule: false', 'keepHistory: true'):
            self.assertIn(k, h)
        self.assertIn('SettingsWriter { id: writer; path: hub.prefsPath; history: true }', h)
        self.assertIn('if (pref(k) === v) return;', h)               # nothing written unless it changed
        self.assertIn('function onQuietEnded(count, why)', read('Notifications.qml'))
        n = read('SetNotifications.qml')
        for t in ('While a full-screen app is in front', 'While you share or record your screen', 'On a schedule', 'Keep the list after a restart',
                  "Hearth's own warnings always show"):
            self.assertIn(t, n)

    def test_dnd_shortcut_on_both(self):
        rc = (DESK / 'labwc' / 'rc.xml').read_text()
        self.assertIn('<keybind key="W-A-n"><action name="Execute" command="sh -c \'quickshell ipc -p $NIMBUS_SHARE/desktop/shell call notifications dnd toggle\'" /></keybind>', rc)
        sc = json.loads((DESK / 'kwin' / 'windows' / 'shortcuts.json').read_text())
        dnd = [s for s in sc['shortcuts'] if s['id'] == 'dnd']
        self.assertEqual(len(dnd), 1)
        self.assertEqual((dnd[0]['keys'], dnd[0]['labwc']), ('Meta+Alt+N', 'W-A-n'))

    def test_badges_and_jobs_wired(self):
        d = read('Dock.qml')
        self.assertIn('info: NotifyHub.badgeFor(task.modelData.id || (task.wins.length ? task.wins[0].appId : ""))', d)
        i = read('Islands.qml')
        for comp in ('cJobs', 'cPrinters', 'cDisks', 'cPrivacy', 'cBackground', 'cLayout'):
            self.assertIn('id: ' + comp, i)
        self.assertIn('Repeater { model: NotifyHub.jobs', read('ClockPanel.qml').replace('\n                    model', ' model'))


class SoundsAndOsd(unittest.TestCase):
    def test_every_event_has_hearths_own_sound_and_the_licences_are_written_down(self):
        s = read('Sounds.qml')
        ids = re.findall(r'\{ id: "([a-z-]+)", label:', s)
        self.assertGreaterEqual(len(ids), 10)
        for i in ids:
            f = DESK / 'sounds' / 'hearth' / 'stereo' / (i + '.wav')
            self.assertTrue(f.exists(), i)
            with wave.open(str(f)) as w:
                self.assertLess(w.getnframes() / w.getframerate(), 1.5, i)      # short
        lic = (DESK / 'sounds' / 'LICENSES.md').read_text()
        self.assertIn('GPL-3.0', lic)
        self.assertIn('sound-theme-freedesktop', lic)
        self.assertIn('Inherits=freedesktop', (DESK / 'sounds' / 'hearth' / 'index.theme').read_text())
        mk = load(ROOT / 'tools' / 'make-hearth-sounds.py', 'make_hearth_sounds')
        self.assertEqual(sorted(set(ids) - set(mk.SOUNDS)), [])
        self.assertIn('SettingsWriter { id: writer; path: snd.path; history: true }', s)
        self.assertIn('/usr/share/sounds/hearth', (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text())

    def test_the_sounds_are_made_the_same_again(self):
        mk = load(ROOT / 'tools' / 'make-hearth-sounds.py', 'make_hearth_sounds2')
        with tempfile.TemporaryDirectory() as d:
            mk.main(['x', d])
            for name in ('message-new-instant', 'complete'):
                self.assertEqual((pathlib.Path(d) / (name + '.wav')).read_bytes(),
                                 (DESK / 'sounds' / 'hearth' / 'stereo' / (name + '.wav')).read_bytes(), name)

    def test_osds_and_new_parts_load_by_file_name(self):
        sh = read('shell.qml')
        for f in ('SystemOsd.qml', 'BarArrange.qml', 'ExtraPanels.qml'):
            self.assertIn('Loader { source: "%s"' % f, sh)
            self.assertNotRegex(code(sh), r'\b%s\s*\{' % f[:-4])
        o = read('SystemOsd.qml')
        for t in ('Caps Lock on', 'Num Lock on', 'Microphone muted', 'The microphone is muted', 'Keyboard layout'):
            self.assertIn(t, o)
        self.assertIn('mask: Region {}', o)
        self.assertIn('WlrLayershell.keyboardFocus: WlrKeyboardFocus.None', o)


class WidgetsAndPanels(unittest.TestCase):
    def test_every_bar_item_has_a_component_and_game_mode_stays(self):
        i = read('Islands.qml')
        ids = run_js('widget-tools.js', 'console.log(JSON.stringify(BAR_ITEMS.map(b => b.id)))') if NODE else []
        fn = i[i.index('function compFor(id)'):i.index('component BarSlot')]
        for b in ids:
            self.assertIn('case "%s": return c' % b, fn, b)
        self.assertIn('Repeater { model: isl.zoneItems("start"); delegate: BarSlot {} }', i)
        self.assertIn('Repeater { model: isl.zoneItems("end"); delegate: BarSlot {} }', i)
        self.assertEqual(len(re.findall(r'\bGameModeButton \{', i)), 1)

    def test_bar_and_panels_saved_only_when_you_change_them(self):
        b = code(read('BarLayout.qml'))
        self.assertIn('SettingsWriter { id: writer; path: bl.path; history: true }', b)
        self.assertIn('if (JSON.stringify(T.barLayout(l)) === JSON.stringify(layout)) return;', b)
        self.assertIn('writer.repair()', b)
        self.assertNotIn('setText(', b)
        self.assertIn('o["p:" + id]', b)                               # one key per panel

    def test_new_widgets_files_and_settings(self):
        for f in ('WidgetWeb.qml', 'WidgetCalc.qml', 'WidgetColour.qml', 'WidgetKeys.qml', 'WidgetPager.qml', 'WidgetWindowlist.qml'):
            t = read(f)
            self.assertIn('WidgetBase {', t, f)
            self.assertNotIn('setText(', code(t), f)
        c = read('WidgetClock.qml')
        self.assertIn('w.set("style", "binary")', c)
        self.assertIn('w.set("style", "fuzzy")', c)
        cal = read('WidgetCalendar.qml')
        self.assertIn('import "calendars.js" as Cal', cal)
        self.assertIn('w.set("altCalendar", id)', cal)
        self.assertIn('w.set("holidays", id)', cal)
        hw = (SHELL / 'hearth-widgets').read_text()
        self.assertIn("'--profile', prof", hw)                          # never the person's own browser profile
        self.assertIn("WEB_OK = re.compile(r'^(https?://", hw)

    def test_light_dark_tile(self):
        q = read('QuickAccess.qml')
        self.assertIn('Tile { navId: "theme"', q)
        self.assertIn('case "theme": {', q)
        blk = q[q.index('case "theme": {'):q.index('break;', q.index('case "theme": {'))]
        self.assertEqual(sorted(re.findall(r'Settings\.(\w+)\s*=(?!=)', blk)), ['mode', 'sunset'])
        self.assertIn('Auto light and dark is off now', blk)         # said, and one step to turn back on


if __name__ == '__main__':
    unittest.main()
