"""Hearth Games (hosts/hearth-gaming: hearth-gamestore, docs/HEARTH-GAMES.md): Hearth's own store app for GOG, Epic
and Battle.net, on hearth-core's store engine. What can be checked without building it: the page only makes calls the
app allows, the image carries the same pinned gogdl and legendary as hearth-core, the app and its menu entry and icon
get into the image, Settings opens it, and the built-in Proton hook is where the launches go through."""
import sys
sys.dont_write_bytecode = True
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
GAMING = ROOT / 'hosts' / 'hearth-gaming'
LIB = (GAMING / 'src' / 'gamestore.rs').read_text()
APP = (GAMING / 'src' / 'apps' / 'gamestore.rs').read_text()
JS = (GAMING / 'ui' / 'games.js').read_text()
HTML = (GAMING / 'ui' / 'games.html').read_text()
TOOLS = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'tools.rs').read_text()
STORES = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'stores.rs').read_text()
BUILD = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
WF = (ROOT / '.github' / 'workflows' / 'base-peppermint.yml').read_text()


def calls():
    block = LIB.split('pub const CALLS', 1)[1].split('];', 1)[0]
    return set(re.findall(r'\("([a-zA-Z]+\.[a-zA-Z]+)"', block))


class Page(unittest.TestCase):
    def test_the_page_only_makes_allowed_calls(self):
        used = set(re.findall(r"call\('([a-zA-Z]+\.[a-zA-Z]+)'", JS)) | set(re.findall(r"data-dl=\"([a-zA-Z]+\.[a-zA-Z]+)\"", JS)) \
            | set(re.findall(r"dlOp\('([a-zA-Z]+\.[a-zA-Z]+)'", JS))
        self.assertTrue(used, 'found the calls')
        self.assertFalse(used - calls(), f'calls the app refuses: {used - calls()}')

    def test_the_window_commands_exist(self):
        handler = re.search(r'generate_handler!\[([^\]]+)\]', APP).group(1)
        have = {h.strip() for h in handler.split(',')}
        used = set(re.findall(r"KIT\.invoke\('([a-z_]+)'", JS))
        self.assertFalse(used - have, f'commands the window lacks: {used - have}')

    def test_never_asks_for_a_password(self):
        # Sign-ins happen on the store's own page (a separate window); Battle.net in Blizzard's own app.
        self.assertNotIn('type="password"', HTML + JS)
        allowed = LIB.split('pub const CALLS', 1)[1].split('];', 1)[0]
        self.assertNotIn('password', allowed)
        for refused in ('login.', 'remote.', 'logs.', 'steam'):
            self.assertNotIn(f'("{refused}', allowed)

    def test_games_start_like_their_steam_shortcuts(self):
        self.assertIn('"--play-now"', APP)
        self.assertIn('setsid', APP, 'closing the window leaves the game running')

    def test_close_while_downloading_asks(self):
        self.assertIn('games://closing', APP)
        self.assertIn('games://closing', JS)


class Image(unittest.TestCase):
    def tool(self, name):
        m = re.search(rf'pub const {name}: Tool = Tool \{{[^}}]*url: "([^"]+)", sha256: "([0-9a-f]+)"', TOOLS)
        self.assertIsNotNone(m, name)
        return m.group(1), m.group(2)

    def test_the_image_carries_the_same_pinned_engines(self):
        for const, file in (('GOGDL', 'gogdl'), ('LEGENDARY', 'legendary')):
            url, sha = self.tool(const)
            self.assertIn(f'hearth_tool {file} {url} {sha}', BUILD, f'{file}: build.sh and tools.rs differ')
        self.assertIn('/usr/lib/hearth/tools/$1', BUILD)
        self.assertIn('pub const IMAGE_DIR: &str = "/usr/lib/hearth/tools";', TOOLS)

    def test_the_app_gets_into_the_image(self):
        self.assertRegex(BUILD, r'for app in [^\n]*hearth-gamestore; do')
        self.assertIn('gaming_entry hearth-gamestore "Hearth Games"', BUILD)
        self.assertRegex(WF, r'for a in [^\n]*hearth-gamestore; do cp')
        self.assertTrue((ROOT / 'branding' / 'icons' / 'apps' / 'hearth-gamestore.svg').is_file())
        for n in (16, 24, 32, 48, 64, 128, 256):
            self.assertTrue((ROOT / 'branding' / 'icons' / 'png' / f'{n}x{n}' / 'hearth-gamestore.png').is_file(), n)
        self.assertIn('name = "hearth-gamestore"', (GAMING / 'Cargo.toml').read_text())

    def test_settings_opens_it(self):
        qml = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'SetGames.qml').read_text()
        self.assertIn('"hearth-gamestore"', qml)


class Proton(unittest.TestCase):
    def test_launches_go_through_the_built_in_proton_hook(self):
        self.assertIn('crate::proton_pick::fill_in(&mut e, store, id);', STORES)
        pick = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'proton_pick.rs').read_text()
        self.assertIn('pub fn auto_for(', pick)
        self.assertIn('"/usr/share/steam/compatibilitytools.d"', pick)


if __name__ == '__main__':
    unittest.main()
