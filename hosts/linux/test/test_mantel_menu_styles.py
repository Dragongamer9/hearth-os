"""The right-click menu's four looks (Settings › Personalization › Bar and dock › Right-click menu; docs/MANTEL.md):
Today (PopMenu.qml's own menu, the default), Windows 11 style, Compact list and Tiles and groups (MenuStyles.qml).
The setting is only written when you pick one, nothing chosen is Today, a look that can't load falls back to Today's
menu (in the desktop's menus and in Hearth Files), every caller keeps menu.popup(screen, x, y, opts, items), and the
new item fields (quick, shortcut, group) are optional."""
import sys
sys.dont_write_bytecode = True
import pathlib
import re
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
LOOKS = ['today', 'windows', 'compact', 'tiles']


def read(name):
    return (SHELL / name).read_text()


def code(text):
    """QML / JS without // comments."""
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


def block(text, start):
    i = text.index(start)
    j = text.index('{', i)
    depth = 0
    for k in range(j, len(text)):
        depth += {'{': 1, '}': -1}.get(text[k], 0)
        if depth == 0:
            return text[i:k + 1]
    raise AssertionError('unbalanced: ' + start)


class TheSettingDefaultsToToday(unittest.TestCase):
    def test_saved_only_when_changed_and_empty_by_default(self):
        s = read('Settings.qml')
        self.assertRegex(s, r'property string menuStyle: ""(?=\s)')
        keys = re.search(r'readonly property var keys: (.*?\]\))\s*\n\s*on', s, re.S).group(1)
        self.assertIn('"menuStyle"', keys)
        self.assertIn('onMenuStyleChanged: save("menuStyle")', s)
        self.assertIn('if (!loaded || applying) return;', s)

    def test_theme_reads_today_when_nothing_is_chosen(self):
        t = code(read('Theme.qml'))
        self.assertIn('readonly property var menuStyles: ["today", "windows", "compact", "tiles"]', t)
        self.assertIn('readonly property string menuStyle: menuStyles.indexOf(Settings.menuStyle) >= 0 ? Settings.menuStyle : "today"', t)

    def test_only_the_picker_writes_it(self):
        for f in sorted(SHELL.glob('*.qml')):
            hits = re.findall(r'\bSettings\.menuStyle\s*=(?!=)', code(f.read_text()))
            if f.name == 'MenuStylePicker.qml':
                self.assertEqual(len(hits), 1, 'the picker writes it once, in choose()')
                fn = block(code(f.read_text()), 'function choose(')
                self.assertIn('Settings.menuStyle = v', fn)
            else:
                self.assertFalse(hits, f.name + ' writes Settings.menuStyle')

    def test_the_picker_has_the_four_looks_and_writes_only_on_a_pick(self):
        p = code(read('MenuStylePicker.qml'))
        for v in LOOKS:
            self.assertIn('value: "%s"' % v, p)
        self.assertIn('onActivated: msp.choose(v)', p)
        self.assertNotIn('Component.onCompleted', p)
        self.assertIn('readonly property bool lit: Theme.menuStyle === v', p)

    def test_settings_page_loads_the_picker_by_name_and_search_finds_it(self):
        b = read('SetBarDock.qml')
        self.assertIn('source: "MenuStylePicker.qml"', b)
        self.assertIn('title: "Right-click menu"', b)
        idx = read('settings-index.js')
        self.assertRegex(idx, r'"t": "Right-click menu",\n"p": "bar-dock"')
        r = subprocess.run([sys.executable, str(ROOT / 'tools' / 'build-settings-index.py'), '--check'])
        self.assertEqual(r.returncode, 0, 'settings-index.js is out of date: run tools/build-settings-index.py')


class TodayIsTheFallback(unittest.TestCase):
    def test_popmenu_keeps_its_api_and_loads_the_other_looks_by_name(self):
        p = code(read('PopMenu.qml'))
        self.assertIn('function popup(scr, x, y, opts, list)', p)
        self.assertIn('source: "MenuStyles.qml"', p)
        self.assertIn('active: Theme.menuStyle !== "today"', p)
        self.assertRegex(p, r'readonly property bool styled: Theme\.menuStyle !== "today" && styledView\.status === Loader\.Ready')
        # Today's own menu is drawn whenever the other look isn't there.
        card = block(p, 'Rectangle {\n        id: card')
        self.assertIn('visible: !root.styled', card)
        self.assertIn('width: 268', card)
        self.assertIn('implicitHeight: sep ? 9 : 40', card)
        self.assertIn('Loader.Error', p)

    def test_files_falls_back_to_its_own_menu(self):
        f = code(read('FileManager.qml'))
        self.assertIn('source: "MenuStyles.qml"', f)
        self.assertIn('visible: app.overlay === "menu" && !styledMenu.styled', f)
        self.assertIn('if (app.overlay === "menu" && styledMenu.styled)', f)
        self.assertIn('function onChosen(it) { app.runMenu(it.id); }', f)

    def test_files_marks_the_quick_actions(self):
        f = code(read('FileManager.qml'))
        for ident in ['cut', 'copy', 'rename', 'trash']:
            self.assertRegex(f, r'\{ id: "%s", [^\n]*quick: true' % ident, ident)

    def test_callers_still_pass_plain_items(self):
        for name in ['Bar.qml', 'Desktop.qml', 'Islands.qml', 'Launcher.qml', 'TaskStrip.qml', 'shell.qml', 'Dock.qml']:
            src = code(read(name))
            self.assertRegex(src, r'\b(menu|popMenu)\.popup\(', name)
            for m in re.finditer(r'\b(?:menu|popMenu)\.popup\(([^;]*)\);', src):
                self.assertGreaterEqual(m.group(1).count(','), 4, name + ': ' + m.group(0)[:80])


class TheLooks(unittest.TestCase):
    def test_each_look_and_its_sizes(self):
        m = code(read('MenuStyles.qml'))
        self.assertIn('readonly property real cardWidth: win ? 240 : compact ? 212 : 250', m)
        self.assertIn('readonly property real rowH: win ? 30 : compact ? 27 : 29', m)
        self.assertIn('readonly property real iconSize: compact ? 14 : 16', m)
        self.assertIn('columns: 4', m)                            # Tiles and groups' tiles
        self.assertIn('readonly property bool flying: win && stack.length > 1', m)   # Windows 11's flyout
        self.assertIn('readonly property bool flip: mv.spaceRight < width + 8', m)

    def test_items_fields_are_optional(self):
        m = code(read('MenuStyles.qml'))
        self.assertIn('it.quick === true', m)
        self.assertIn('it.shortcut || it.hint || it.key', m)          # PopMenu's hint, Files' key
        self.assertIn('it.separator === true || it.sep === true', m)  # PopMenu's separator, Files' sep
        self.assertIn('title: g || "More"', m)                         # ungrouped items

    def test_keyboard_and_controller(self):
        m = code(read('MenuStyles.qml'))
        k = block(m, 'function key(event)')
        for key in ['Key_Up', 'Key_Down', 'Key_Left', 'Key_Right', 'Key_Escape', 'Key_Return', 'Key_Enter', 'Key_Menu']:
            self.assertIn(key, k)
        self.assertIn('MenuHints', m)
        self.assertIn('{ b: "A", t: "Choose" }, { b: "B", t: "Back" }', m)

    def test_follows_the_theme(self):
        m = code(read('MenuStyles.qml'))
        self.assertNotRegex(m, r'"#[0-9a-fA-F]{6}"', 'no fixed colours: Theme only')
        for t in ['Theme.accent', 'Theme.accentInk', 'Theme.glass', 'Theme.line', 'Theme.text', 'Theme.muted', 'Theme.danger']:
            self.assertIn(t, m)


if __name__ == '__main__':
    unittest.main()
