"""Hearth Games (hosts/hearth-gaming: hearth-gamestore, docs/HEARTH-GAMES.md): Hearth's own store app for GOG, Epic
and Battle.net, on hearth-core's store engine. What can be checked without building it: the page only makes calls the
app allows, the image carries the same pinned gogdl and legendary as hearth-core, the app and its menu entry and icon
get into the image, Settings opens it, the built-in Proton hook is where the launches go through, and its Recomps tab
uses Hearth Recomps' engine (hearth-core recomps.rs) without changing how ports reach Steam Game Mode."""
import sys
sys.dont_write_bytecode = True
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
GAMING = ROOT / 'hosts' / 'hearth-gaming'
LIB = (GAMING / 'src' / 'gamestore.rs').read_text()
APP = (GAMING / 'src' / 'apps' / 'gamestore.rs').read_text()
RECOMPS_JS = (GAMING / 'ui' / 'recomps.js').read_text()
JS = (GAMING / 'ui' / 'games.js').read_text() + RECOMPS_JS
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


class Recomps(unittest.TestCase):
    """Hearth Games › Recomps (docs/RECOMPS.md): a window on hearth-core's recomps engine, not a second engine."""
    CORE = ROOT / 'hosts' / 'hearth-core' / 'src'

    def test_the_tab_is_there_and_reached_with_lb_rb(self):
        self.assertIn('data-tab="recomps"', HTML)
        self.assertIn('<script src="recomps.js"></script>', HTML)
        self.assertLess(HTML.index('recomps.js'), HTML.index('games.js'))
        self.assertIn("const TABS = ['library', 'downloads', 'stores', 'recomps'];", JS)
        self.assertIn('pub const TABS: &[&str] = &["library", "downloads", "stores", "recomps"];', LIB)

    def test_every_recomps_call_is_one_hearth_cores_bridge_answers(self):
        bridge = (self.CORE / 'bridge.rs').read_text()
        mine = {c for c in calls() if c.startswith('recomps.')}
        self.assertTrue({'recomps.list', 'recomps.details', 'recomps.install', 'recomps.cancel', 'recomps.uninstall',
                         'recomps.places', 'recomps.pickGame', 'recomps.useGame'} <= mine, mine)
        for c in mine:
            self.assertIn(f'"{c}" =>', bridge, c)
        # The page uses them all, and the engine isn't copied into hearth-gaming.
        used = set(re.findall(r"call\('(recomps\.[a-zA-Z]+)'", RECOMPS_JS))
        self.assertEqual(used, mine)
        for name, t in (('gamestore.rs', LIB), ('apps/gamestore.rs', APP), ('recomps.js', RECOMPS_JS)):
            for engine in ('fn check_file(', 'fn pick_asset(', 'fn parse_releases(', 'api.github.com', 'curl'):
                self.assertNotIn(engine, t, f'{name}: {engine}')

    def test_the_file_picker_and_use_it_anyway(self):
        self.assertIn('b.pick_file = Arc::new(', APP)
        self.assertIn('tauri_plugin_dialog::init()', APP)
        self.assertIn('gamestore::use_game_ok(', APP, 'Use it anyway only for the file the engine just checked')
        self.assertIn("call('recomps.useGame', { id, path: j.file, anyway: true })", RECOMPS_JS)
        self.assertIn('Use it anyway', RECOMPS_JS)

    def test_uninstall_offers_a_backup_and_never_touches_your_copy(self):
        self.assertIn("call('recomps.uninstall', { id: e.id, keep: pickd === 'keep' })", RECOMPS_JS)
        self.assertIn('Your own copy of the game is never deleted.', RECOMPS_JS)

    def test_hearth_never_links_to_games(self):
        # docs/RECOMPS.md: the person brings their own copy; the page has no addresses at all (icons come from the
        # catalogue, which recomps.rs's tests keep to the projects' own repositories).
        self.assertNotRegex(RECOMPS_JS, r'https?://')

    def test_play_and_steam_game_mode_unchanged(self):
        # Installed ports stay Hearth library entries recomp:<id> (source "recomps"), which Hearth Sync puts in Steam's
        # Game Mode; Play here is the same `nimbus --play-now recomp:<id>` start as their Steam shortcuts.
        rec = (self.CORE / 'recomps.rs').read_text()
        self.assertIn('id: format!("recomp:{id}"), title, source: "recomps"', rec)
        self.assertIn("play({ id: `recomp:${id}`, title: e.title })", RECOMPS_JS)
        self.assertIn('gamestore::playable(&id)', APP)
        sync = (ROOT / 'hosts' / 'decky-hearth-sync' / 'main.py').read_text()
        self.assertIn("'recomps': ('recomps', 'Recomps')", sync)

    def test_settings_opens_hearth_games_on_recomps(self):
        qml = (ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'SetGames.qml').read_text()
        self.assertIn('page.app(["hearth-gamestore", "--tab", "recomps"]', qml)
        self.assertNotIn('core.openPage("recomps"', qml)
        self.assertIn('"--tab"', LIB)
        self.assertIn('games://open', APP)
        self.assertIn('games://open', JS)


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
