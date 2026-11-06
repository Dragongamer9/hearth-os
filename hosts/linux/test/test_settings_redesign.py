"""The redesigned Hearth Settings (docs/MANTEL.md, "Settings"): 12 categories plus Home, every old page with one home and
its old id still opening, the search index made from the pages' cards, the frame that can't stop Settings from opening,
the controller's Y / LB / RB / ☰, and settings that never change by themselves (opening a page writes nothing; the new
installs' Sky accent is a default worked out when read, never written)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DESK = ROOT / 'hosts' / 'linux' / 'desktop'
SHELL = DESK / 'shell'
THEME = DESK / 'theme'
REG = (SHELL / 'settings-pages.js').read_text()

# Every page id Hearth Settings had before the redesign (SettingsApp.qml's list): each must still open.
OLD_IDS = ['install', 'wifi', 'bluetooth', 'link', 'remote', 'hs-overview', 'hs-shares', 'hs-files', 'hs-drives', 'hs-tailscale',
           'hs-services', 'hs-gaming', 'hs-family', 'hs-ai', 'hs-backups', 'hs-safety', 'hs-people', 'hs-alerts', 'hs-setup', 'sound',
           'notifications', 'display', 'appearance', 'boot', 'power', 'controller', 'handheld', 'keyboard', 'mouse', 'printers',
           'datetime', 'accessibility', 'defaults', 'apps', 'games', 'storage', 'steam', 'clipboard', 'users', 'signin', 'browser',
           'claude', 'updates', 'about']
CATEGORIES = ['home', 'system', 'net', 'bt', 'link', 'pers', 'apps', 'gaming', 'accounts', 'hs', 'privacy', 'a11y', 'updates']


def pages():
    body = REG[REG.index('var PAGES = ['):REG.index('\n];', REG.index('var PAGES = ['))]
    out = {}
    for line in body.splitlines():
        m = re.search(r'\{ id: "([\w-]+)", file: "([\w.]+)", cat: "(\w+)",', line)
        if m:
            p = {'id': m.group(1), 'file': m.group(2), 'cat': m.group(3), 'label': re.search(r'\blabel: "([^"]+)"', line).group(1), 'line': line}
            for k in ('parent', 'old'):
                v = re.search(r'\b%s: "([^"]+)"' % k, line)
                p[k] = v.group(1) if v else None
            part = re.search(r'props: \{ part: "(\w+)" \}', line)
            p['part'] = part.group(1) if part else None
            out[p['id']] = p
    return out


def categories():
    body = REG[REG.index('var CATEGORIES = ['):REG.index('];', REG.index('var CATEGORIES = ['))]
    return [dict(zip(('id', 'icon', 'label', 'landing'), m)) for m in
            re.findall(r'\{ id: "(\w+)", icon: "([\w-]+)", label: "([^"]+)", landing: "([\w-]+)" \}', body)]


def code(text):
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in text.splitlines())


class Registry(unittest.TestCase):
    def test_twelve_categories_plus_home(self):
        cats = categories()
        self.assertEqual([c['id'] for c in cats], CATEGORIES)
        self.assertEqual([c['label'] for c in cats], ['Home', 'System', 'Network & internet', 'Bluetooth & devices', 'Hearth Link',
                                                       'Personalization', 'Apps', 'Gaming', 'Accounts', 'Home server',
                                                       'Privacy & security', 'Accessibility', 'Updates & recovery'])
        ps = pages()
        for c in cats:
            self.assertIn(c['landing'], ps, c)
            self.assertEqual(ps[c['landing']]['cat'], c['id'], c)
            self.assertIn('landing: true', ps[c['landing']]['line'], c)

    def test_every_old_page_has_a_home_and_its_id_still_opens(self):
        ps = pages()
        for pid in OLD_IDS:
            self.assertIn(pid, ps, pid)
            self.assertTrue((SHELL / ps[pid]['file']).exists(), ps[pid]['file'])
        # Every Set*.qml page file is in the registry (nothing gets lost), except the parts other pages use.
        parts = {'SetBase.qml', 'SetRow.qml', 'SetSwitch.qml', 'SetChoice.qml', 'SetSlider.qml', 'SetField.qml', 'SetHeader.qml',
                 'SetPage.qml', 'SetJob.qml', 'SetBootCard.qml', 'SetBootFrames.qml', 'SetBootPreview.qml', 'SetBootVideo.qml',
                 'SetCard.qml', 'SetCardGroup.qml', 'SetExpander.qml', 'SetChip.qml', 'SetBanner.qml', 'SetLinkCard.qml', 'SetButton.qml',
                 'SetStyle.qml'}
        files = {p['file'] for p in ps.values()}
        for f in SHELL.glob('Set*.qml'):
            if f.name not in parts and not f.name.startswith('Settings'):
                self.assertIn(f.name, files, f.name + ' has no place in Settings')

    def test_ids_are_unique_and_parents_and_links_exist(self):
        ids = re.findall(r'\{ id: "([\w-]+)", file:', REG)
        self.assertEqual(len(ids), len(set(ids)))
        ps = pages()
        for p in ps.values():
            self.assertIn(p['cat'], CATEGORIES, p['id'])
            if p['parent']:
                self.assertIn(p['parent'], ps)
                self.assertEqual(ps[p['parent']]['cat'], p['cat'])
        for target in re.findall(r'\{ page: "([\w-]+)"', REG):
            self.assertIn(target, ps)
        for alias, target in re.findall(r'"([\w:-]+)": "([\w-]+)"', REG[REG.index('var ALIASES'):REG.index('\n', REG.index('var ALIASES'))]):
            self.assertIn(target, ps, alias)

    def test_the_owners_ia(self):
        ps = pages()
        where = {pid: ps[pid]['cat'] for pid in ps}
        want = {'wifi': 'net', 'hs-tailscale': 'net', 'bluetooth': 'bt', 'keyboard': 'bt', 'mouse': 'bt', 'printers': 'bt',
                'link': 'link', 'appearance': 'pers', 'bar-dock': 'pers', 'apps': 'apps', 'defaults': 'apps', 'browser': 'apps',
                'claude': 'apps', 'steam': 'gaming', 'games': 'gaming', 'remote': 'gaming', 'controller': 'gaming',
                'handheld': 'gaming', 'startup-animation': 'gaming', 'users': 'accounts', 'signin': 'accounts', 'display': 'system',
                'sound': 'system', 'notifications': 'system', 'power': 'system', 'storage': 'system', 'boot': 'system',
                'datetime': 'system', 'clipboard': 'system', 'about': 'system', 'install': 'system', 'accessibility': 'a11y',
                'updates': 'updates', 'backup': 'updates', 'player-count': 'privacy'}
        for pid, cat in want.items():
            self.assertEqual(where[pid], cat, pid)
        # Home server is always its own category, with every hs- page but Tailscale; AI chat sits under Services.
        for pid in ps:
            if pid.startswith('hs-') and pid != 'hs-tailscale':
                self.assertEqual(where[pid], 'hs', pid)
        self.assertEqual(ps['hs-ai']['parent'], 'hs-services')
        self.assertEqual(ps['hs-ai']['label'], 'AI chat')
        self.assertIn('live: true', ps['install']['line'])
        # One file, two homes: About's and Boot & startup's parts.
        self.assertEqual((ps['boot']['part'], ps['startup-animation']['part']), ('splash', 'animation'))
        self.assertEqual((ps['about']['part'], ps['player-count']['part'], ps['backup']['part']), ('about', 'privacy', 'backup'))
        # Settings that show up in two places have link cards (one real home).
        for cat, target in (('hs', 'hs-tailscale'), ('gaming', 'storage'), ('privacy', 'clipboard'), ('privacy', 'hs-safety'), ('privacy', 'signin')):
            self.assertRegex(REG, r'\n    %s: \[[^\]]*\{ page: "%s"' % (cat, target))
        # The old names are kept for the search.
        self.assertEqual(ps['appearance']['old'], 'Appearance')
        self.assertEqual(ps['signin']['old'], 'Sign-in')
        self.assertEqual(ps['hs-overview']['old'], 'Home server › Overview')

    def test_parts_are_marked_in_their_files(self):
        about = (SHELL / 'SetAbout.qml').read_text()
        for part in ('about', 'privacy', 'backup'):
            self.assertIn('// @part ' + part, about)
            self.assertIn('page.inPart("%s")' % part, about)
        boot = (SHELL / 'SetBoot.qml').read_text()
        for part in ('splash', 'animation'):
            self.assertIn('// @part ' + part, boot)
            self.assertIn('page.inPart("%s")' % part, boot)
        self.assertIn('function inPart(p) { return part === "" || part === p; }', (SHELL / 'SetPage.qml').read_text())


class Search(unittest.TestCase):
    def test_index_is_up_to_date(self):
        r = subprocess.run([sys.executable, str(ROOT / 'tools' / 'build-settings-index.py'), '--check'])
        self.assertEqual(r.returncode, 0, 'shell/settings-index.js is out of date: run tools/build-settings-index.py')

    def test_it_finds_single_settings_with_their_page(self):
        s = (SHELL / 'settings-index.js').read_text()
        entries = json.loads(s[s.index('['):s.rindex(']') + 1])
        ps = pages()
        for e in entries:
            self.assertIn(e['p'], ps, e)
        titles = {(e['t'], e['p']) for e in entries}
        for t, p in (('Go to sleep after', 'power'), ('Ask for my password after sleep', 'signin'), ('Night light', 'display'),
                     ('Reduce motion', 'accessibility'), ('Count me in the player count', 'player-count'), ('Start in', 'gaming'),
                     ('Sleep at night, wake in the morning', 'hs-alerts'), ('Word suggestions', 'keyboard')):
            self.assertIn((t, p), titles)
        # Pages are found by their old names too (the frame searches label, old name and words).
        frame = (SHELL / 'SettingsFrame.qml').read_text()
        self.assertIn('p.label + " " + (p.old || "") + " " + (p.words || "")', frame)
        self.assertIn('Index.ENTRIES', frame)


class Frame(unittest.TestCase):
    def test_settings_always_opens(self):
        app = (SHELL / 'SettingsApp.qml').read_text()
        self.assertIn('setSource(Qt.resolvedUrl("SettingsFrame.qml"), { app: app })', app)
        self.assertIn('setSource(Qt.resolvedUrl("SettingsSimple.qml"), { app: app })', app)
        self.assertIn('status === Loader.Error && !fellBack', app)
        self.assertIn('title: "Hearth Settings"', app)          # Pad.qml gives this window the controller's keys
        self.assertNotRegex(code(app), r'\bSettingsFrame\s*\{')   # only through the Loader
        self.assertIn('SettingsApp { id: settings }', (SHELL / 'shell.qml').read_text())
        simple = code((SHELL / 'SettingsSimple.qml').read_text())
        self.assertNotRegex(simple, r'\b(SetCard|SetExpander|SetBanner|SettingsFrame)\b')

    def test_opens_on_home_and_old_ids_and_parts_open(self):
        app = (SHELL / 'SettingsApp.qml').read_text()
        self.assertIn('property string page: "home"', app)
        self.assertIn('page = e && (!e.live || live) ? id : (shown ? page : "home");', app)
        self.assertIn('SettingsNav.pageAnchor = m[2]', app)
        self.assertIn('"appearance:bar": "bar-dock"', REG)
        # The launcher's search keeps working (Launcher.qml reads pages and keywords).
        self.assertIn('readonly property var pages:', app)
        self.assertIn('readonly property var keywords:', app)

    def test_breadcrumbs_back_hints_and_the_controller(self):
        f = (SHELL / 'SettingsFrame.qml').read_text()
        for want in ('Reg.crumbs(entry.id)', 'function back()', 'Ctrl+PgDown', 'Ctrl+PgUp', '"Ctrl+F"', '"Ctrl+W"',
                     'label: "Select"', 'label: "Back"', 'label: "Search"', 'label: "Previous / next category"', 'label: "Close"'):
            self.assertIn(want, f)
        self.assertIn('case Qt.Key_Escape: SettingsNav.back(); break;', (SHELL / 'SetBase.qml').read_text())
        pad = (SHELL / 'hearth-pad').read_text()
        self.assertIn("SETTINGS_KEYS = {'y': ('KEY_LEFTCTRL', 'KEY_F'), 'lb': ('KEY_LEFTCTRL', 'KEY_PAGEUP'), 'rb': ('KEY_LEFTCTRL', 'KEY_PAGEDOWN'),", pad)
        self.assertIn("self.keys_why.endswith('Hearth Settings')", pad)

    def test_hearth_pad_sends_settings_keys_only_to_settings(self):
        loader = importlib.machinery.SourceFileLoader('hearth_pad_settings_test', str(SHELL / 'hearth-pad'))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        hp = importlib.util.module_from_spec(spec)
        loader.exec_module(hp)

        class Out:
            def __init__(self): self.sent, self.taps = [], []
            def send(self, m): self.sent.append(m)
            def press(self, k, down): self.taps.append((k, down))
            def __getattr__(self, n): return lambda *a, **k: None
        pad = hp.Pad.__new__(hp.Pad)
        pad.out = Out()
        pad.plasma = False
        pad.keys_ctx = True
        pad.keys_down = set()
        pad.mouse_src = {'BTN_LEFT': set(), 'BTN_RIGHT': set(), 'BTN_MIDDLE': set()}
        tapped = []
        pad.tap = lambda *keys: tapped.append(keys)
        pad.keys_why = 'window: org.quickshell Hearth Settings'
        for b in ('y', 'lb', 'rb', 'menu'):
            pad.keys_button(b, True, 0.0)
        self.assertEqual(tapped, [('KEY_LEFTCTRL', 'KEY_F'), ('KEY_LEFTCTRL', 'KEY_PAGEUP'), ('KEY_LEFTCTRL', 'KEY_PAGEDOWN'), ('KEY_LEFTCTRL', 'KEY_W')])
        self.assertIn({'t': 'osk', 'open': True}, pad.out.sent)            # Y: the keyboard to type the search with
        tapped.clear(); pad.out.sent.clear()
        pad.keys_why = 'window: org.kde.dolphin Documents – Files'
        pad.keys_button('y', True, 0.0); pad.keys_button('menu', True, 0.0); pad.keys_button('lb', True, 0.0)
        self.assertEqual(tapped, [])
        self.assertEqual(pad.out.sent, [{'t': 'osk', 'open': True}, {'t': 'launcher'}])


class NeverChangeByThemselves(unittest.TestCase):
    NEW = ['SettingsFrame.qml', 'SettingsSimple.qml', 'SetStyle.qml', 'SetCategory.qml', 'SetCard.qml', 'SetCardGroup.qml',
           'SetExpander.qml', 'SetChip.qml', 'SetBanner.qml', 'SetLinkCard.qml', 'SetButton.qml', 'SetPage.qml', 'SetBase.qml']

    def test_the_frame_and_the_cards_only_draw(self):
        for f in self.NEW:
            t = code((SHELL / f).read_text())
            self.assertEqual(re.findall(r'\bSettings\.\w+\s*=(?!=)', t), [], f)
            self.assertNotIn('writer.merge', t, f)
            self.assertNotIn('"set",', t, f)

    def test_new_pages_write_only_when_you_press(self):
        # Home, Gaming and Bar and dock change a setting only from a press (onToggled / onChosen / onClicked).
        for f in ('SetHome.qml', 'SetGaming.qml', 'SetBarDock.qml', 'StartInChoice.qml'):
            t = code((SHELL / f).read_text())
            for line in t.splitlines():
                if re.search(r'\bSettings\.\w+\s*=(?!=)|"set", "lightMode"|setStartMode\(v\)|DeskState\.setDnd\(', line):
                    self.assertRegex(line, r'onToggled|onChosen|onClicked|function setStartMode', (f, line.strip()))
        # Start in has one writer (it moved from Sign-in), the same session.conf line as before.
        self.assertNotIn('NIMBUS_START_MODE=$1', (SHELL / 'SetSignin.qml').read_text())
        self.assertIn('NIMBUS_START_MODE=$1', (SHELL / 'StartInChoice.qml').read_text())

    def test_moved_switches_keep_their_keys(self):
        self.assertIn('checked: Settings.nightLight; onToggled: v => Settings.nightLight = v', (SHELL / 'SetDisplay.qml').read_text())
        self.assertIn('checked: Settings.reduceMotion; onToggled: v => Settings.reduceMotion = v', (SHELL / 'SetAccessibility.qml').read_text())
        bar = (SHELL / 'SetBarDock.qml').read_text()
        self.assertIn('onChosen: v => Settings.barStyle = v', bar)
        self.assertIn('onChosen: v => Settings.dockMode = v', bar)
        a = (SHELL / 'SetAppearance.qml').read_text()
        for gone in ('Settings.nightLight = v', 'Settings.reduceMotion = v', 'Settings.barStyle = v', 'Settings.dockMode = v'):
            self.assertNotIn(gone, a)

    def test_the_sky_default_is_only_for_new_installs_and_never_written(self):
        s = (SHELL / 'Settings.qml').read_text()
        self.assertIn('root.defaultsGen = typeof o.defaults === "number" ? o.defaults : 1;', s)
        keys = re.search(r'readonly property var keys: (.*?\]\))', s, re.S).group(1)
        self.assertNotIn('"defaults"', keys)
        self.assertNotRegex(code(s), r'save\("defaults"\)|defaultsGen\s*=(?!=)(?!.*o\.defaults)')
        self.assertIn('property string look: "steam"', s)
        t = (SHELL / 'Theme.qml').read_text()
        self.assertIn('readonly property bool skyDefault: steam && Settings.defaultsGen >= 2', t)
        self.assertIn('readonly property color lookAccent: skyDefault ? "#66c0f4" : ember', t)
        skel = json.loads((ROOT / 'os' / 'system_files' / 'etc' / 'skel' / '.local' / 'share' / 'nimbus' / 'desktop.json').read_text())
        self.assertEqual(skel, {'defaults': 2})
        # hearth-theme (labwc, Qt, GTK) works it out the same way, only when the file says so.
        for desktop, accent in (({'defaults': 2}, '#66c0f4'), ({}, '#1a9fff'), ({'defaults': 2, 'accent': '#ff5c8a'}, '#ff5c8a'),
                                ({'defaults': 2, 'look': 'hearth'}, '#ff8a3d'), ({'defaults': True}, '#1a9fff')):
            with tempfile.TemporaryDirectory() as d:
                os.makedirs(os.path.join(d, '.local', 'share', 'nimbus'))
                with open(os.path.join(d, '.local', 'share', 'nimbus', 'desktop.json'), 'w') as f:
                    json.dump(desktop, f)
                before = open(os.path.join(d, '.local', 'share', 'nimbus', 'desktop.json')).read()
                env = dict(os.environ, HOME=d, XDG_RUNTIME_DIR=os.path.join(d, 'rt'))
                for k in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'LABWC_PID'):
                    env.pop(k, None)
                subprocess.run([sys.executable, str(THEME / 'hearth-theme')], env=env, capture_output=True, text=True, check=True)
                gtk = pathlib.Path(d, '.config', 'nimbus', 'theme', 'gtk4.css').read_text()
                self.assertIn('@define-color accent_bg_color %s;' % accent, gtk, desktop)
                self.assertEqual(open(os.path.join(d, '.local', 'share', 'nimbus', 'desktop.json')).read(), before, 'never written')

    def test_desktop_test_checks_that_opening_every_page_writes_nothing(self):
        y = (ROOT / '.github' / 'workflows' / 'desktop-test.yml').read_text()
        self.assertIn('settings-redesign', y)
        self.assertIn('opening every Settings page changed', y)


if __name__ == '__main__':
    unittest.main()
