"""Sharper graphics (vkBasalt; hearth-core vkbasalt.rs, Hearth Games' ui/vkbasalt.js): what can be checked without
building it. The Fedora image installs vkBasalt without failing when it's missing, the switch is off unless turned on,
the page's calls are ones Hearth Games allows and hearth-core answers, every start that applies a game's options gets
the environment, and Steam's own files are never touched."""
import sys
sys.dont_write_bytecode = True
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
CORE = ROOT / 'hosts' / 'hearth-core' / 'src'
GAMING = ROOT / 'hosts' / 'hearth-gaming'
VKB = (CORE / 'vkbasalt.rs').read_text()
JS = (GAMING / 'ui' / 'vkbasalt.js').read_text()
GAMES = (GAMING / 'ui' / 'games.js').read_text()
HTML = (GAMING / 'ui' / 'games.html').read_text()
LIB = (GAMING / 'src' / 'gamestore.rs').read_text()
BRIDGE = (CORE / 'bridge.rs').read_text()


class Image(unittest.TestCase):
    def test_fedora_installs_it_and_never_fails_without_it(self):
        build = (ROOT / 'os-bases' / 'anatase' / 'build.sh').read_text()
        line = next(l for l in build.splitlines() if 'install' in l and 'vkBasalt' in l and l.startswith('dnf5'))
        self.assertIn('--skip-unavailable', line)
        self.assertRegex(line, r'\|\| echo')


class Switch(unittest.TestCase):
    def test_off_by_default(self):
        self.assertIn('json!({"on": false, "strength": 40, "smaa": false, "vibrance": false, "key": "Home"})', VKB)
        self.assertIn('"vkbasalt": crate::vkbasalt::defaults()', (CORE / 'game_options.rs').read_text())

    def test_the_page_only_makes_allowed_calls_and_hearth_core_answers_them(self):
        block = LIB.split('pub const CALLS', 1)[1].split('];', 1)[0]
        allowed = set(re.findall(r'\("([a-zA-Z]+\.[a-zA-Z]+)"', block))
        used = set(re.findall(r"call\('([a-zA-Z]+\.[a-zA-Z]+)'", JS))
        self.assertEqual(used, {'game.vkbasalt', 'game.setVkbasalt', 'vkbasalt.info', 'vkbasalt.shared'})
        self.assertFalse(used - allowed, used - allowed)
        for c in used:
            self.assertIn(f'"{c}" =>', BRIDGE, c)

    def test_on_the_game_page_and_the_stores_tab(self):
        self.assertIn('<script src="vkbasalt.js"></script>', HTML)
        self.assertLess(HTML.index('vkbasalt.js'), HTML.index('games.js'))
        self.assertIn('VKB.mount(g, d)', GAMES)
        self.assertIn('VKB.stores(', GAMES)

    def test_the_toggle_key_is_written_and_shown(self):
        self.assertIn('toggleKey = {key}', VKB)
        self.assertIn('to turn the effects off and on', JS)

    def test_every_start_with_the_games_options_gets_it(self):
        manual = (CORE / 'manual.rs').read_text()
        apply = manual.split('pub fn apply', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('crate::vkbasalt::launch_env(opts)', apply)
        # The starts that apply a game's options all go through manual::apply (play-now included: library.launch_with).
        for f in ('stores.rs', 'steam_engine.rs'):
            self.assertIn('crate::manual::apply(', (CORE / f).read_text(), f)

    def test_steam_is_only_a_launch_option_to_copy(self):
        self.assertIn('pub const STEAM_OPTION: &str = "ENABLE_VKBASALT=1 %command%";', VKB)
        for bad in ('localconfig.vdf', 'steam_ctl', 'userdata'):
            self.assertNotIn(bad, VKB + JS)
        self.assertIn('Faugus Launcher has its own field', JS)

    def test_settings_go_through_the_helpers(self):
        self.assertIn('crate::settings_file::write_text', VKB)
        self.assertNotIn('std::fs::write(', VKB.split('#[cfg(test)]', 1)[0])


if __name__ == '__main__':
    unittest.main()
