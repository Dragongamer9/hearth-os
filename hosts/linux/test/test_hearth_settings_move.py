"""Hearth's old Game Mode is hidden, and its settings live in Mantel's Settings or in apps of their own
(docs/SETTINGS-MOVE.md): the new pages are listed, every call they make is one Hearth's core answers for Settings
(hearth-core settings_cli.rs), every page they open in the Hearth app's window is one it opens (bridge.rs start_page and
ui/index.html openStartPage), and nothing people see offers the old Game Mode."""
import sys
sys.dont_write_bytecode = True
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
CORE = ROOT / 'hosts' / 'hearth-core' / 'src'
NEW = {'remote': 'SetRemote.qml', 'games': 'SetGames.qml', 'storage': 'SetStorage.qml', 'steam': 'SetSteam.qml'}


def rust_list(text, name):
    """The names in a Rust `const NAME: &[...] = &[ ... ];` (the first string of each tuple, or each string)."""
    m = re.search(r'const %s:[^=]*=\s*&\[(.*?)\];' % name, text, re.S)
    assert m, name
    body = m.group(1)
    tuples = re.findall(r'\(\s*"([^"]+)"', body)
    return set(tuples) if tuples else set(re.findall(r'"([^"]+)"', body))


def qml_strings(text):
    """The string literals in QML outside // comments."""
    out = []
    for line in text.splitlines():
        code = re.sub(r'(^|\s)//.*$', '', line)
        out += re.findall(r'"((?:[^"\\]|\\.)*)"', code)
    return out


class Pages(unittest.TestCase):
    def test_settings_lists_the_moved_pages(self):
        s = (SHELL / 'SettingsApp.qml').read_text()
        for page, f in NEW.items():
            self.assertRegex(s, r'\{ id: "%s", icon: "[a-z-]+", label: "[^"]+" \}' % page)
            self.assertIn('%s: "%s"' % (page, f), s)
            self.assertTrue((SHELL / f).exists(), f)

    def test_every_call_is_one_settings_may_make(self):
        allowed = rust_list((CORE / 'settings_cli.rs').read_text(), 'CALLS')
        used = set()
        for f in list(NEW.values()) + ['SetAbout.qml']:
            used |= set(re.findall(r'core\.call\("([\w.]+)"', (SHELL / f).read_text()))
        self.assertTrue(used)
        self.assertEqual(used - allowed, set())
        # Long jobs stay in the Hearth app: never asked of a one-shot call.
        self.assertFalse({'diskspace.move', 'diskspace.scan', 'game.install', 'game.uninstall'} & allowed)

    def test_every_page_opened_in_the_app_is_one_it_opens(self):
        pages = rust_list((CORE / 'bridge.rs').read_text(), 'PAGES')
        ui = (ROOT / 'ui' / 'index.html').read_text()
        opened = set()
        for f in SHELL.glob('*.qml'):
            t = f.read_text()
            opened |= set(re.findall(r'openPage\("(\w+)"', t))
            opened |= set(re.findall(r'"--windowed", "--page", "(\w+)"', t))
        self.assertTrue({'sources', 'diskspace', 'library', 'storage', 'system'} <= opened, opened)
        self.assertEqual(opened - pages, set())
        m = re.search(r'function openStartPage\(p\)\{(.*?)\n\}', ui, re.S)
        self.assertTrue(m)
        for p in opened - {'hhd', 'shop', 'settings'}:
            self.assertIn("'%s'" % p, m.group(1), p)

    def test_calls_go_through_hearths_core(self):
        hc = (SHELL / 'HearthCall.qml').read_text()
        self.assertIn('["nimbus", "--settings-call", j.method, j.args]', hc)
        main = (ROOT / 'hosts' / 'hearth-rs' / 'src' / 'main.rs').read_text()
        self.assertIn('if has("--settings-call") { std::process::exit(hearth_core::settings_cli::run(&args)); }', main)
        # Before the app-update handoff and the renderer pick: answered by this program, without a window.
        self.assertLess(main.index('--settings-call'), main.index('appchannel::handoff(&args)'))

    def test_store_sign_ins_and_apps(self):
        g = (SHELL / 'SetGames.qml').read_text()
        for store in ('epic', 'gog', 'xbox'):
            self.assertIn('page.signIn("%s"' % store, g)
        self.assertIn('"open", store]', g)
        for app in ('hearth-mods', 'hearth-saves', 'hearth-controller-lab', 'hearth-theater'):
            self.assertIn('["%s"]' % app, g)

    def test_going_back_a_version_is_in_updates(self):
        u = (SHELL / 'SetUpdates.qml').read_text()
        self.assertIn('page.start("rollback")', u)
        self.assertIn('"Go back to " + page.ver(', u)
        self.assertIn('rollbackQueued', u)


class Hidden(unittest.TestCase):
    def test_the_hearth_app_is_not_in_the_menus(self):
        b = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        m = re.search(r'cat > /usr/share/applications/nimbus\.desktop <<DESKTOP\n(.*?)\nDESKTOP', b, re.S)
        self.assertTrue(m)
        self.assertIn('NoDisplay=true', m.group(1).splitlines())
        launcher = (SHELL / 'Launcher.qml').read_text()
        self.assertIn('Quickshell.env("HEARTH_LEGACY_GAME_MODE") === "1"', launcher)
        self.assertIn('(root.legacyGame || !/^nimbus(\\.desktop)?$/.test(a.id || ""))', launcher)

    def test_mantel_never_offers_the_old_game_mode(self):
        bad = re.compile(r"Hearth'?s? (own )?Game Mode|Game Mode's (Friends|Settings|Library)|Picked in Game Mode|in Game Mode \(Settings")
        for f in sorted(SHELL.glob('*.qml')):
            for s in qml_strings(f.read_text()):
                self.assertIsNone(bad.search(s), '%s: "%s"' % (f.name, s))

    def test_the_docs_say_it_is_archived(self):
        d = (ROOT / 'tools' / 'build-docs.py').read_text()
        for gone in ("Hearth's Game Mode is still here", 'Switch to Hearth Game Mode', "Setting multi-seat up stays in Hearth's Game Mode"):
            self.assertNotIn(gone, d)
        doc = (ROOT / 'docs' / 'SETTINGS-MOVE.md').read_text()
        for page in ('Remote play', 'Games & stores', 'Storage', 'Steam Game Mode', 'Interim', 'Obsolete', 'HEARTH_LEGACY_GAME_MODE'):
            self.assertIn(page, doc)


if __name__ == '__main__':
    unittest.main()
