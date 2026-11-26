"""Every Hearth app made of its real parts (the owner, Nov 24: "Modular for all Hearth apps is required as well"), and
Edit layout that feels natural (hosts/hearth-ui/src/arrange.rs).

For Capture, the G15 app, Hearth Ideas, Hearth Notes, Reliability history, Resource Monitor, Server mode, System
Restore and Task Manager: every page the app has names its blocks (hearth_ui::blocks PageDef), each block is on the
12 x 12 grid with a unique id, every block is one of the page's real parts (the page's own view hands it over by that id
and puts it back together the app's own way), and every app has an always-findable "Edit layout". Edit layout itself
is direct: a right-click menu on any block, drag to move, edges and a corner to size, saved as you go (no Keep step),
Undo and Back to the original, Esc / Ctrl+Z and the controller.

Read from the sources (the apps are built and tested in GitHub Actions only)."""
import pathlib
import re
import unittest

UI = pathlib.Path(__file__).resolve().parents[2] / 'hearth-ui'
APPS = UI / 'apps'

# app -> every page it has (its BLOCKS)
PAGES = {
    'task-manager': ['window', 'processes', 'performance', 'history', 'startup', 'users', 'details', 'services', 'tasks', 'settings'],
    'system-restore': ['main'],
    'reliability': ['main'],
    'resmon': ['window', 'pc', 'overview', 'cpu', 'memory', 'disk', 'network', 'gpu'],
    'server': ['dashboard'],
    'capture': ['window', 'take', 'edit', 'recent'],
    'g15': ['window', 'overview', 'gkeys', 'screen', 'lighting', 'editor'],
    'ideas': ['window', 'list', 'idea', 'built'],
    'notes': ['window', 'files', 'search', 'today', 'board', 'graph', 'modes', 'sync', 'conflict', 'settings'],
    'lights': ['main'],
    'remap': ['main'],
    'mouse': ['main'],
    'shield': ['window', 'today', 'panel', 'devices', 'device', 'log', 'lists', 'allow', 'block', 'family', 'settings'],
    # the open-source apps (docs/mockups/open-source-apps)
    'listen': ['window', 'side', 'recording', 'how'],
    'imagine': ['window', 'type', 'graph', 'results'],
    'passwords': ['window', 'side', 'list', 'item'],
    'cloud': ['window', 'side', 'home'],
    'home': ['window', 'board'],
}

BLOCK = re.compile(r'\bbls?\(\s*"([^"]+)"\s*,\s*"([^"]*)"\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)')


def sources(app):
    return {p: p.read_text() for p in sorted((APPS / app).rglob('*.rs'))}


def braced(text, start):
    """The text from `start` (a '{' or '[') to its matching close."""
    open_c = text[start]
    close_c = {'{': '}', '[': ']'}[open_c]
    depth = 0
    for i in range(start, len(text)):
        if text[i] == open_c:
            depth += 1
        elif text[i] == close_c:
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    raise AssertionError('unbalanced')


def pages_of(app):
    """{page id: [(block id, title, x, y, w, h)]} from the app's PageDefs (a shared `const NAME: &[BlockDef]` too)."""
    out = {}
    for text in sources(app).values():
        consts = {}
        for m in re.finditer(r'const\s+([A-Z_]+)\s*:\s*&\[\s*(?:hearth_ui::blocks::)?BlockDef\s*\]\s*=\s*&', text):
            consts[m.group(1)] = braced(text, m.end())
        for m in re.finditer(r'PageDef\s*\{', text):
            body = braced(text, m.end() - 1)
            pid = re.search(r'id:\s*"([^"]+)"', body).group(1)
            blocks_src = re.search(r'blocks:\s*(&\[|[A-Z_]+)', body)
            if blocks_src.group(1) == '&[':
                src = braced(body, blocks_src.end() - 1)
            else:
                src = consts[blocks_src.group(1)]
            out[pid] = [(b[0], b[1], int(b[2]), int(b[3]), int(b[4]), int(b[5])) for b in BLOCK.findall(src)]
    return out


class EveryPageNamesItsBlocks(unittest.TestCase):
    def test_every_page_of_every_app(self):
        for app, want in PAGES.items():
            with self.subTest(app=app):
                got = pages_of(app)
                self.assertEqual(sorted(got), sorted(want), f'{app}: its pages')

    def test_blocks_are_whole_unique_and_on_the_grid(self):
        for app in PAGES:
            for pid, blocks in pages_of(app).items():
                with self.subTest(app=app, page=pid):
                    ids = [b[0] for b in blocks]
                    self.assertGreaterEqual(len(ids), 2, f'{app} {pid}: made of its parts, not one big block')
                    self.assertEqual(len(ids), len(set(ids)), f'{app} {pid}: ids unique')
                    for bid, title, x, y, w, h in blocks:
                        self.assertTrue(title.strip(), f'{app} {pid} {bid}: a name for Edit layout')
                        self.assertTrue(w >= 1 and h >= 1 and x + w <= 12 and y + h <= 12, f'{app} {pid} {bid}: on the grid')

    def test_finer_than_before(self):
        # the coarse cut these apps had (1-4 big blocks on one page) is gone: more pages and more parts
        total = {app: sum(len(b) for b in pages_of(app).values()) for app in PAGES}
        for app, n in total.items():
            with self.subTest(app=app):
                self.assertGreaterEqual(n, 6, f'{app}: {n} blocks in all')

    def test_every_block_is_a_real_part_of_its_page(self):
        """Each block id is handed over by the page's view (`("id", …)` / `.add("id", …)`) and put back the app's own
        way (`ps.take("id")`, or a loop over the ids it handed over)."""
        for app in PAGES:
            text = '\n'.join(sources(app).values())
            for pid, blocks in pages_of(app).items():
                for bid, *_ in blocks:
                    with self.subTest(app=app, page=pid, block=bid):
                        handed = re.search(r'\(\s*"%s"\s*,' % re.escape(bid), text) or re.search(r'\.add\(\s*"%s"' % re.escape(bid), text)
                        self.assertTrue(handed, f'{app} {pid}: {bid} is handed over by the view')
                        taken = f'ps.take("{bid}")' in text or 'ps.take(id)' in text
                        self.assertTrue(taken, f'{app} {pid}: {bid} is put back the app\'s own way')

    def test_every_page_is_drawn_through_its_blocks(self):
        for app in PAGES:
            text = '\n'.join(sources(app).values())
            for pid in pages_of(app):
                with self.subTest(app=app, page=pid):
                    direct = re.search(r'arrange::parts\(\s*&a\.describe,\s*&(?:a\.)?p(?:al)?,\s*"%s"' % re.escape(pid), text)
                    via = ('a.tab.id()' in text and app == 'resmon') or ('a.page.id()' in text and app == 'task-manager') or re.search(r'head_and\(a,\s*"%s"' % re.escape(pid), text)
                    self.assertTrue(direct or via, f'{app}: page {pid} goes through hearth_ui::arrange::parts')

    def test_every_app_has_edit_layout_to_hand(self):
        for app in PAGES:
            with self.subTest(app=app):
                text = '\n'.join(sources(app).values())
                self.assertTrue('arrange::edit_layout()' in text or 'arrange::edit_page(' in text, f'{app}: an Edit layout in its menu, side or title')


class EditLayoutIsDirect(unittest.TestCase):
    A = (UI / 'src' / 'arrange.rs').read_text()
    B = (UI / 'src' / 'blocks.rs').read_text()
    D = (UI / 'src' / 'describe.rs').read_text()

    # The Nov 24 Edit layout (saved as you go, grips on the grid) was replaced by drag-drop board 3 (the owner, Nov 26:
    # "i like drag dop 1, need to expand on it in every app"): studio.rs, nothing kept until Keep this.
    S = (UI / 'src' / 'studio.rs').read_text()

    def test_board_3_keeps_nothing_until_keep_this(self):
        for s in ('"Keep this"', '"Try it"', 'SMsg::Cancel =>', 'N::Escape'):
            self.assertIn(s, self.S)
        self.assertIn('app-keep', self.S, 'kept through hearth-layout app-keep (the safe writer)')

    def test_board_3_customize_from_the_right_click(self):
        self.assertIn('"Customize…"', self.S)

    def test_board_3_is_light_at_rest(self):
        self.assertNotIn('iced::time::every', self.S)

    def test_describe_it_stays_beside_it(self):
        self.assertIn('"Edit layout"', self.D)
        self.assertIn('DMsg::Edit(crate::arrange::EMsg::Start)', self.D)
        self.assertIn('Edit(crate::arrange::EMsg)', self.D)

    def test_the_old_api_still_works(self):
        # the other apps' block lists and Arrange's messages are unchanged
        for x in ('pub fn parts<', 'pub fn parts_in<', 'pub fn page<', 'pub fn page_in<', 'pub fn take(&mut self, id: &str)'):
            self.assertIn(x, self.A)
        for m in ('ArrUndo', 'ArrReset', 'ArrKeep', 'Grab(&\'static str, blocks::Grip)', 'Cell(u8, u8)'):
            self.assertIn(m, self.D)



if __name__ == '__main__':
    unittest.main()
