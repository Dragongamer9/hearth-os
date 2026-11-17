"""1.14 (wave 3) shell bits: the keyboard's RGB lights following the accent (hearth-kbd-lights, KbdLights.qml: off
by default, never writes while off), Settings › Startup's "Confirm before logging out" and "Default leave option"
(Leave.qml, LeaveConfirm.qml, PowerMenu.qml), the emoji picker's sizes and two skin tones (hosts/hearth-emoji), and
the on-screen keyboard's accents by a long touch (slide onto one, lift) as well as by holding A."""
import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LIGHTS = SHELL / 'hearth-kbd-lights'
EMOJI = ROOT / 'hosts' / 'hearth-emoji'
NODE = shutil.which('node')


def read(p):
    return Path(p).read_text(encoding='utf-8')


class KeyboardLights(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.home = os.path.join(self.tmp, 'home')
        self.leds = os.path.join(self.tmp, 'leds')
        self.led = os.path.join(self.leds, 'rgb:kbd_backlight')
        os.makedirs(self.led)
        os.makedirs(os.path.join(self.home, '.local', 'share', 'nimbus'))
        for name, text in (('multi_intensity', '1 2 3\n'), ('multi_index', 'green red blue\n'), ('max_brightness', '100\n'), ('brightness', '0\n')):
            with open(os.path.join(self.led, name), 'w') as f:
                f.write(text)
        os.makedirs(os.path.join(self.leds, 'input3::capslock'))   # not a keyboard backlight: never touched
        self.bin = os.path.join(self.tmp, 'bin')   # a PATH with python3 and no openrgb
        os.makedirs(self.bin)
        os.symlink(shutil.which('python3'), os.path.join(self.bin, 'python3'))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_it(self, *args, on=None):
        if on is not None:
            with open(os.path.join(self.home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
                json.dump({'kbdLights': on, 'look': 'steam'}, f)
        env = {'HOME': self.home, 'HEARTH_LEDS_DIR': self.leds, 'PATH': self.bin}
        return subprocess.run([shutil.which('python3'), str(LIGHTS)] + list(args), capture_output=True, text=True, env=env, timeout=20)

    def intensity(self):
        return read(os.path.join(self.led, 'multi_intensity')).strip()

    def test_off_by_default_writes_nothing(self):
        r = self.run_it('apply', '#ff0000')   # no desktop.json at all
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(r.stdout)['wrote'], 0)
        self.assertEqual(self.intensity(), '1 2 3')
        r = self.run_it('apply', '#ff0000', on=False)
        self.assertEqual(json.loads(r.stdout)['skipped'], 'off')
        self.assertEqual(self.intensity(), '1 2 3')

    def test_damaged_settings_count_as_off(self):
        with open(os.path.join(self.home, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
            f.write('{"kbdLights": tr')
        self.run_it('apply', '#ff0000')
        self.assertEqual(self.intensity(), '1 2 3')

    def test_on_colours_in_the_kernels_order_and_scale(self):
        r = self.run_it('apply', '#FF8000', on=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(json.loads(r.stdout), {'ok': True, 'way': 'kernel', 'wrote': 1})
        self.assertEqual(self.intensity(), '50 100 0')    # green red blue, out of 100
        self.assertEqual(read(os.path.join(self.led, 'brightness')).strip(), '0')   # how bright: left alone

    def test_bad_colour_and_status(self):
        self.assertNotEqual(self.run_it('apply', 'red', on=True).returncode, 0)
        self.assertEqual(self.intensity(), '1 2 3')
        st = json.loads(self.run_it('status', on=True).stdout)
        self.assertEqual(st['way'], 'kernel')
        self.assertTrue(st['on'])
        self.assertEqual([l['name'] for l in st['leds']], ['rgb:kbd_backlight'])

    def test_openrgb_only_keyboards(self):
        listed = '0: Corsair K70 RGB\n  Type:           Keyboard\n  Description:    x\n1: Logitech G502\n  Type:           Mouse\n2: Razer Huntsman\n  Type:           Keyboard\n'
        fake = os.path.join(self.bin, 'openrgb')
        log = os.path.join(self.tmp, 'openrgb.log')
        with open(fake, 'w') as f:
            f.write('#!/bin/sh\necho "$*" >> %s\nif [ "$1" = --list-devices ]; then cat <<"X"\n%sX\nfi\n' % (log, listed))
        os.chmod(fake, 0o755)
        os.symlink('/bin/sh', os.path.join(self.bin, 'sh'))
        os.symlink(shutil.which('cat'), os.path.join(self.bin, 'cat'))
        r = self.run_it('apply', '#1a9fff', on=True)
        self.assertEqual(json.loads(r.stdout), {'ok': True, 'way': 'openrgb', 'wrote': 2}, r.stderr)
        calls = read(log).splitlines()
        self.assertEqual(calls, ['--list-devices', '--device 0 --mode static --color 1a9fff', '--device 2 --mode static --color 1a9fff'])
        self.assertEqual(self.intensity(), '1 2 3')   # OpenRGB did it: the kernel's file isn't touched too

    def test_mantel_runs_it_only_when_on(self):
        q = read(SHELL / 'KbdLights.qml')
        self.assertIn('readonly property bool on: Settings.loaded && Settings.kbdLights', q)
        self.assertIn('if (!on) return;', q)
        self.assertRegex(q, r'onColorChanged: if \(on\) later\.restart\(\)')
        s = read(SHELL / 'Settings.qml')
        self.assertIn('property bool kbdLights: false', s)
        self.assertIn('onKbdLightsChanged: save("kbdLights")', s)
        self.assertIn('"kbdLights"', s)
        self.assertIn('Loader { id: kbdLights; source: "KbdLights.qml"', read(SHELL / 'shell.qml'))
        a = read(SHELL / 'SetAppearance.qml')
        self.assertIn('title: "Keyboard lights follow the accent"', a)
        self.assertIn('kbdJob.run(["status"]', a)            # opening the page only asks
        self.assertNotIn('"apply"', a)
        rules = read(ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'udev' / 'rules.d' / '70-hearth-backlight.rules')
        self.assertIn('TEST=="multi_intensity", RUN+="/bin/chgrp video /sys/class/leds/%k/multi_intensity"', rules)


class LeaveOptions(unittest.TestCase):
    def test_settings_keys(self):
        s = read(SHELL / 'Settings.qml')
        self.assertIn('property bool confirmLogout: false', s)
        self.assertIn('property string leaveDefault: ""', s)
        for k in ('confirmLogout', 'leaveDefault'):
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), s)
        self.assertIn('.concat(["kbdLights", "confirmLogout", "leaveDefault"])', s)

    def test_rows_on_startup(self):
        st = read(SHELL / 'SetStartup.qml')
        self.assertIn('title: "Confirm before logging out"', st)
        self.assertIn('onToggled: v => Settings.confirmLogout = v', st)
        self.assertIn('title: "Default leave option"', st)
        self.assertIn('onChosen: v => Settings.leaveDefault = v', st)
        # each choice is a power menu entry's label (that's how the menu finds it)
        pm = read(SHELL / 'PowerMenu.qml')
        for label in re.findall(r'\{ value: "([^"]+)", label: "\1" \}', st):
            self.assertIn('label: "%s"' % label, pm + ' label: "Log out"', label)
        self.assertIn('items.findIndex(x => x.label === Settings.leaveDefault)', pm)

    def test_every_menu_logs_out_through_leave(self):
        self.assertIn('if (Settings.confirmLogout && canAsk) { confirmRequested(); return; }', read(SHELL / 'Leave.qml'))
        self.assertIn('Quickshell.execDetached(["nimbus-mode", "logout"])', read(SHELL / 'Leave.qml'))
        self.assertIn('else if (kind === "logout") Leave.logout();', read(SHELL / 'Launcher.qml'))
        self.assertIn('if (ab.command.join(" ") === "nimbus-mode logout") Leave.logout();', read(SHELL / 'Launcher.qml'))
        self.assertIn('Leave.logout(); return; }', read(SHELL / 'WinTools.qml'))
        self.assertIn('if (cmd === "nimbus-mode logout") Leave.logout();', read(SHELL / 'DesktopMenu.qml'))
        c = read(SHELL / 'LeaveConfirm.qml')
        self.assertIn('Component.onCompleted: Leave.canAsk = true', c)
        self.assertIn('if (i === 0) Leave.now();', c)
        self.assertIn('Loader { source: "LeaveConfirm.qml"', read(SHELL / 'shell.qml'))


class EmojiPicker(unittest.TestCase):
    def test_size_is_saved_by_itself(self):
        lib = read(EMOJI / 'src' / 'lib.rs')
        self.assertIn('pub size: u8,', lib)
        self.assertIn('v["size"] = serde_json::json!(size.min(SIZE_MAX));', lib)
        main = read(EMOJI / 'src' / 'main.rs')
        self.assertIn('generate_handler![init, set_tone, set_size, pick, close]', main)
        self.assertIn('"size": s.size', main)
        page = read(EMOJI / 'ui' / 'index.html')
        self.assertIn("invoke('set_size', { size: n })", page)
        self.assertIn('.app.size-l {', page)

    @unittest.skipUnless(NODE, 'node is not installed')
    def test_mixed_tones_are_unicodes(self):
        page = read(EMOJI / 'ui' / 'index.html')
        js = page[page.index('const TONES = '):page.index('const TONE_COLORS')]
        script = 'const DATA = [];\n' + js + '\nconsole.log(JSON.stringify(Object.fromEntries(Object.entries(MIX).map(([k, f]) => [k, [f(2, 5), f(3, 3)]]))));'
        out = json.loads(subprocess.run([NODE, '-e', script], capture_output=True, text=True, check=True, timeout=20).stdout)
        T = ['', '\U0001F3FB', '\U0001F3FC', '\U0001F3FD', '\U0001F3FE', '\U0001F3FF']
        Z, H = '‍', '❤️'
        want = {
            '🤝': ['🫱' + T[2] + Z + '🫲' + T[5], '🤝' + T[3]],
            '🧑' + Z + '🤝' + Z + '🧑': ['🧑' + T[2] + Z + '🤝' + Z + '🧑' + T[5], '🧑' + T[3] + Z + '🤝' + Z + '🧑' + T[3]],
            '👭': ['👩' + T[2] + Z + '🤝' + Z + '👩' + T[5], '👭' + T[3]],
            '👫': ['👩' + T[2] + Z + '🤝' + Z + '👨' + T[5], '👫' + T[3]],
            '👬': ['👨' + T[2] + Z + '🤝' + Z + '👨' + T[5], '👬' + T[3]],
            '💏': ['🧑' + T[2] + Z + H + Z + '💋' + Z + '🧑' + T[5], '💏' + T[3]],
            '💑': ['🧑' + T[2] + Z + H + Z + '🧑' + T[5], '💑' + T[3]],
            '👩' + Z + H + Z + '💋' + Z + '👨': ['👩' + T[2] + Z + H + Z + '💋' + Z + '👨' + T[5], '👩' + T[3] + Z + H + Z + '💋' + Z + '👨' + T[3]],
            '👩' + Z + H + Z + '👩': ['👩' + T[2] + Z + H + Z + '👩' + T[5], '👩' + T[3] + Z + H + Z + '👩' + T[3]],
        }
        for k, v in want.items():
            self.assertEqual(out.get(k), v, k)
        self.assertEqual(len(out), 13)
        # every one is an emoji in the picker's list
        data = read(EMOJI / 'ui' / 'emoji-data.js')
        listed = {d['e'] for d in json.loads(data[data.index('['):data.rindex(']') + 1])}
        self.assertTrue(set(out) <= listed, set(out) - listed)


class OskAccents(unittest.TestCase):
    @unittest.skipUnless(NODE, 'node is not installed')
    def test_long_touch_slide_and_lift_and_holding_a(self):
        script = r'''
const fs = require('fs'), vm = require('vm');
const ctx = {}; vm.createContext(ctx);
vm.runInContext(fs.readFileSync(process.argv[1], 'utf8') + '\nthis.HearthOSK = HearthOSK;', ctx);
const O = ctx.HearthOSK; let text = '';
const mk = () => O.createEngine({ host: 'mantel', adapter: { insert: s => { text += s; }, deleteBack: n => { text = text.slice(0, -n); }, key: () => {} } });
const find = (e, ch) => { let at = null; e.view().rows.forEach((row, r) => row.forEach((k, c) => { if (k.t === 'ch' && k.v === ch) at = [r, c]; })); return at; };
const out = {};
let e = mk(); const at = find(e, 'e');
out.opened = e.longPress(at[0], at[1]); out.alts = e.view().popup.alts;
e.hoverAlt(2); out.lit = e.view().popup.i; e.hoverAlt(99); out.stillLit = e.view().popup.i;
e.pickAlt(e.view().popup.i); out.typed = text; out.closed = e.view().popup === null;
// the controller: A held on "a" opens its accents, Right then A picks the second
text = ''; e = mk(); const a = find(e, 'a'); e.state.sel = { r: a[0], c: a[1] };
e.handle('a', true, 1000); e.tick(1700); out.padOpened = !!e.view().popup;
e.handle('a', false, 1750); e.handle('right', 'press'); e.handle('a', 'press'); out.padTyped = text;
console.log(JSON.stringify(out));
'''
        r = subprocess.run([NODE, '-e', script, str(ROOT / 'ui' / 'osk' / 'hearth-osk.js')], capture_output=True, text=True, timeout=20)
        self.assertEqual(r.returncode, 0, r.stderr)
        o = json.loads(r.stdout)
        self.assertTrue(o['opened'])
        self.assertEqual(o['alts'][2], 'ê')
        self.assertEqual((o['lit'], o['stillLit']), (2, 2))
        self.assertEqual(o['typed'], 'ê')
        self.assertTrue(o['closed'])
        self.assertTrue(o['padOpened'])
        self.assertEqual(o['padTyped'], 'á')

    def test_touch_slides_in_mantel_and_on_web_pages(self):
        key = read(SHELL / 'OskKey.qml')
        self.assertIn('onPositionChanged: m => { if (held) { const p = onBoard(m); owner.slideTo(owner.altAt(p.x, p.y)); } }', key)
        self.assertIn('if (held) { const p = onBoard(m); owner.liftOn(owner.altAt(p.x, p.y)); }', key)
        board = read(SHELL / 'OskBoard.qml')
        self.assertIn('signal altHovered(int i)', board)
        self.assertIn('readonly property int altIndex: index', board)
        self.assertIn('onAltHovered: i => { if (root.engine && root.engine.hoverAlt) root.engine.hoverAlt(i); }', read(SHELL / 'Keyboard.qml'))
        js = read(ROOT / 'ui' / 'osk' / 'hearth-osk.js')
        self.assertIn("root.addEventListener('pointermove', slide);", js)
        self.assertIn('doc.elementFromPoint(e.clientX, e.clientY)', js)
        self.assertIn('.hosk-key{ touch-action: none; }', read(ROOT / 'ui' / 'osk' / 'hearth-osk.css'))
        # the Qt renderer's copies are the same
        for f in ('OskBoard.qml', 'OskKey.qml'):
            self.assertEqual(read(SHELL / f), read(ROOT / 'hosts' / 'hearth-qml' / 'qml' / f), f)


if __name__ == '__main__':
    unittest.main()
