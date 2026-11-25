"""Every page of Settings, the Store, Files and Family and friends made of its real parts as blocks (the owner, Nov 24:
every Hearth app modular, and editing a layout natural: a right-click on a part, an Edit layout entry, Esc and Ctrl+Z).

What must hold: each page names its blocks, every block a page names is one of its real parts (its view takes it by
that id), the original look is the page's own composition of those same parts, each app can open Edit layout right
there, and the CI pictures show every page untouched and with Edit layout open. Settings' pages are made of their
sections (secs.rs): each heading starts one, and a page's column is a Secs."""
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
UI = ROOT / 'hosts' / 'hearth-ui'
APPS = UI / 'apps'
SHOTS = ROOT / '.github' / 'scripts' / 'hearth-apps-rust-shots.d' / 'app-blocks-a.sh'


def read(p):
    return pathlib.Path(p).read_text(encoding='utf-8')


def page_defs(src):
    """{page id: [block ids]} from PageDef { id: "…", …, blocks: &[bl("…", …), …] } (or a const blocks list)."""
    out = {}
    for m in re.finditer(r'PageDef\s*\{\s*id:\s*"([a-z0-9-]+)",\s*title:\s*"[^"]*",\s*blocks:\s*(&\[.*?\]|[A-Z_]+),?\s*\}', src, re.S):
        body = m.group(2)
        if not body.startswith('&['):
            c = re.search(r'const %s: &\[hearth_ui::blocks::BlockDef\] = &\[(.*?)\];' % body, src, re.S)
            body = c.group(1) if c else ''
        out[m.group(1)] = re.findall(r'\bbls?\("([a-z0-9-]+)"', body)
    return out


def sources(app):
    return '\n'.join(read(p) for p in sorted((APPS / app / 'src').rglob('*.rs')))


class EachPageNamesItsParts(unittest.TestCase):
    def check(self, app, want, src_files=None):
        src = sources(app)
        defs = {}
        for f in (src_files or [p for p in (APPS / app / 'src').rglob('*.rs')]):
            defs.update(page_defs(read(f)))
        for page in want:
            self.assertIn(page, defs, f'{app} has no page {page}')
            ids = defs[page]
            self.assertTrue(ids, f'{app}:{page} names no blocks')
            self.assertEqual(len(ids), len(set(ids)), f'{app}:{page} names a block twice')
            for b in ids:
                self.assertRegex(b, r'^[a-z0-9-]{1,48}$', 'hearth-layout checks ids this way')
                self.assertIn(f'take("{b}")', src, f'{app}:{page}: block {b} is no part its view takes')
        return defs

    def test_store(self):
        d = self.check('store', ['window', 'home', 'apps', 'library', 'app', 'parts', 'system', 'tests', 'settings'])
        w11 = read(APPS / 'store/src/w11.rs')
        for page in ['home', 'apps', 'library', 'app', 'parts', 'system', 'tests', 'settings']:
            self.assertIn(f'view::page_parts(a, "{page}"', w11, f'the Store draws {page} from its parts')
        self.assertIn('view::page_parts_fill(a, "window"', w11)
        self.assertEqual(d['window'][0], 'page', "1.18's kept window (the page alone) opens under the rest")
        self.assertGreaterEqual(len(d['home']), 4)

    def test_files(self):
        d = self.check('files', ['window', 'chooser', 'bars'])
        self.assertEqual(d['bars'], ['tabs', 'address', 'toolbar', 'panes', 'status'])
        self.assertEqual(d['window'], ['sidebar', 'folder', 'preview'], "1.18's ids kept")
        menus = read(APPS / 'files/src/menus.rs')
        self.assertGreaterEqual(menus.count('MenuItem::new("edit-layout", "Edit layout")'), 2, 'in the View menu and the right-click menu')
        self.assertIn('"edit-layout" =>', menus)

    def test_family(self):
        d = self.check('family', ['friends', 'ask', 'notice'])
        self.assertIn('people', d['friends'])
        self.assertIn('add', d['friends'])

    def test_settings_window_and_home(self):
        frame = read(APPS / 'settings/src/frame.rs')
        d = page_defs(frame)
        self.assertEqual(d['window'], ['brand', 'search', 'cats', 'title', 'page'])
        for b in d['window']:
            self.assertIn(f'take("{b}")', frame)
        self.assertIn('STATIC_BLOCKS: &[hearth_ui::blocks::PageDef] = &[frame::WINDOW, home::PAGE]', read(APPS / 'settings/src/main.rs'))


class SettingsPagesAreTheirSections(unittest.TestCase):
    def test_a_heading_starts_a_section(self):
        w = read(APPS / 'settings/src/widgets.rs')
        head = w[w.index('pub fn head<'):]
        head = head[:head.index('\n}\n')]
        self.assertIn('crate::secs::heading(&s)', head)

    def test_the_pages_columns_are_secs(self):
        files = sorted((APPS / 'settings/src/cats').rglob('*.rs'))
        sites = []
        for f in files:
            src = read(f)
            for m in re.finditer(r'let mut (c|col) = crate::widgets::Secs::on\(a, [a-z_.0-9]+\)', src):
                sites.append((f.name, m.group(1)))
        self.assertGreaterEqual(len(sites), 40, sites)
        mods = {f for f, _ in sites}
        for want in ['system_sound.rs', 'system_power.rs', 'system_lang.rs', 'system_display.rs', 'accounts.rs', 'a11y.rs', 'bt.rs', 'net.rs', 'fonts.rs']:
            self.assertIn(want, mods)

    def test_the_modules_with_secs_hand_arrange_to_describe(self):
        main = read(APPS / 'settings/src/main.rs')
        for m, v in [('system', 'System'), ('net', 'Net'), ('bt', 'Bt'), ('couch', 'Couch'), ('pers', 'Pers'), ('accounts', 'Accounts'), ('a11y', 'A11y')]:
            src = read(APPS / f'settings/src/cats/{m}.rs')
            self.assertIn('Arrange(hearth_ui::describe::DMsg),', src, m)
            self.assertIn('impl From<hearth_ui::describe::DMsg> for Msg', src, m)
            self.assertIn('Msg::Arrange(_) => return Task::none(),', src, m)
            self.assertIn(f'Msg::{v}({m}::Msg::Arrange(dm))', main, m)

    def test_nothing_customised_is_the_same_column(self):
        s = read(APPS / 'settings/src/secs.rs')
        # the original: every section's column in one column with the page's own spacing; a skip (from:N) as before
        self.assertIn('Column::with_children(all.into_iter().skip(skip)).spacing(spacing)', s)
        self.assertIn('let mut c = Column::new().spacing(spacing);', s)
        self.assertIn('Column::with_children(els).spacing(spacing)', s)
        # worked out once per new section, never per frame; at most the grid's 12 rows of blocks
        self.assertIn('if list.len() >= 12', s)


class EditLayoutIsRightThere(unittest.TestCase):
    def test_helpers_are_additive(self):
        a = read(UI / 'src/arrange.rs')
        for f in ['pub fn edit_at(', 'pub fn right_edit<', 'pub fn arrange_key(']:
            self.assertIn(f, a)
        self.assertIn('Key::Named(Named::Escape) => Some(DMsg::ArrKeep)', a)
        self.assertIn('Some(DMsg::ArrUndo)', a)

    def test_each_app_uses_them(self):
        for app in ['store', 'files', 'family', 'settings']:
            src = sources(app)
            self.assertIn('hearth_ui::arrange::arrange_key(', src, f'{app}: Esc and Ctrl+Z while arranging')
            self.assertIn('hearth_ui::arrange::right_edit(', src, f'{app}: a right-click on a part')
            self.assertIn('hearth_ui::arrange::edit_at(', src, f'{app}: Edit layout right there')
        self.assertIn('Some(Msg::EditAt("window", None))', read(APPS / 'store/src/w11.rs'), "Store settings' Edit layout")


class ThePictures(unittest.TestCase):
    def test_every_page_untouched_and_with_edit_layout_open(self):
        s = read(SHOTS)
        for pg in ['home', 'apps', 'library', 'parts', 'system', 'tests', 'settings']:
            self.assertIn(f'"{pg}:', s)
        self.assertIn('HEARTH_ARRANGE_OPEN="store:$id"', s)
        for o in ['store:app', 'store:window', 'files:bars', 'files:window', 'family:friends', 'family:ask', 'family:notice', 'settings:window', 'settings:home']:
            self.assertIn(f'HEARTH_ARRANGE_OPEN={o}', s)
        self.assertIn('HEARTH_ARRANGE_OPEN="settings:$page"', s)
        self.assertIn('kept-layout', s)


if __name__ == '__main__':
    unittest.main()
