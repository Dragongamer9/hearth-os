"""Every Mantel app made of blocks (AppBlocks.qml, AppBlocksFlow.qml, app-blocks.js).

Each of Mantel's own app windows (Files, Settings, Task Manager, Capture, Resource Monitor, Reliability history,
System Restore) names its parts as blocks and offers Edit layout and Describe it right where you are; what's kept goes
only through hearth-layout app-keep (the Rust apps' file and format); nothing customised, nothing moves; and nothing
runs at rest (no timer that repeats or starts by itself, no polling). The rules themselves (app-blocks.js) are run in
node when it's here.
"""
import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SHELL = os.path.join(ROOT, "hosts", "linux", "desktop", "shell")
APP_RS = os.path.join(ROOT, "hosts", "hearth-layout", "src", "app.rs")


def read(name):
    with open(os.path.join(SHELL, name), encoding="utf-8") as f:
        return f.read()


# app window file → (app id, the blocks it must name)
APPS = {
    "FileManager.qml": ("files", ["tabs", "address", "toolbar", "sidebar", "folder", "status", "split", "preview"]),
    "SettingsFrame.qml": ("settings", ["side", "page", "hints"]),
    "TaskManagerView.qml": ("task-manager", ["titlebar", "rail", "head", "body", "tiles", "detail"]),
    "Capture.qml": ("capture", ["head", "options", "recent", "editor", "list"]),
    "ResourceMonitorView.qml": ("resmon", ["title", "tabs", "page", "hints"]),
    "ReliabilityView.qml": ("reliability", ["title", "header", "chart", "events"]),
    "RestoreView.qml": ("system-restore", ["title", "header", "body", "footer"]),
}


def block_ids(src):
    return set(re.findall(r'\{ id: "([a-z0-9-]+)", title: ', src))


class AppsHaveBlocks(unittest.TestCase):
    def test_every_app_window_has_blocks_with_its_parts(self):
        for f, (app, ids) in APPS.items():
            src = read(f)
            self.assertIn("AppBlocks {", src, f)
            self.assertIn('app: "%s"' % app, src, f)
            named = block_ids(src)
            for b in ids:
                self.assertIn(b, named, "%s doesn't name the block %s" % (f, b))
            # its parts are given to the blocks by id
            m = re.search(r"parts:\s*(\(\{.*?\}\)|\{.*?\n\s{8}\}|cap\.page.*?\})\n", src, re.S)
            self.assertTrue(m, "%s gives its parts to the blocks" % f)

    def test_the_app_ids_are_hearth_layouts(self):
        rs = open(APP_RS, encoding="utf-8").read()
        known = set(re.findall(r'app: "([a-z-]+)",', rs)) | set(re.findall(r'window_app!\(\w+, "([a-z-]+)"', rs))
        for f, (app, _) in APPS.items():
            self.assertIn(app, known, "hearth-layout has no Describe it for %s (%s)" % (app, f))

    def test_block_ids_and_grid_check(self):
        # hearth-layout's check: ids lowercase-dashes; on the 24 x 24 grid
        for f in APPS:
            src = read(f)
            for m in re.finditer(r'\{ id: "([^"]+)", title: "[^"]*", x: (\d+), y: (\d+), w: (\d+), h: (\d+)', src):
                bid, x, y, w, h = m.group(1), *map(int, m.groups()[1:])
                self.assertRegex(bid, r"^[a-z0-9-]{1,48}$", f)
                self.assertTrue(w >= 1 and h >= 1 and x + w <= 24 and y + h <= 24, "%s: %s is off the grid" % (f, bid))


class WaysIn(unittest.TestCase):
    def test_part_menu_and_menu_items(self):
        ab = read("AppBlocks.qml")
        for label in ("Edit layout", "Customize…", "Move this", "Hide this", "Describe it…", "Back to the original"):
            self.assertIn('"%s"' % label, ab)
        # the app's own menus' LAYOUT items
        self.assertIn('{ id: "ab-edit", label: "Edit layout"', ab)
        self.assertIn('{ id: "ab-describe", label: "Describe it…"', ab)
        # editing in place (drag and drop board 3): the draft, Undo and Redo, Keep this, Try it, Cancel, the keys
        for s in ("function change(", "function undo(", "function redo(", "function keepThis(", "function tryIt(", "function goBack(",
                  "function cancel(", "Qt.Key_Escape", "Qt.Key_Z", "Qt.Key_Y", "Qt.Key_Delete", "Qt.Key_Menu", "Qt.Key_Tab"):
            self.assertIn(s, ab, s)
        ed = read("AppBlocksEditor.qml")
        for s in ("function grabPart(", "function movePart(", "function letGoPart(", "L.sizePx(", "L.movePx(", "L.dropped(",
                  "function grabLib(", "function grabBlock(", "function moveDrag(", "function endDrag(", 'label: "Keep this"'):
            self.assertIn(s, ed, s)

    def test_files_view_menu_has_layout_and_describe_it(self):
        fm = read("FileManager.qml")
        view = fm[fm.index("function viewMenu("):fm.index("function runMenu(")]
        self.assertIn('menuKind = "view"', view)
        self.assertEqual(view.count("layoutItems()"), 4, "every View menu has LAYOUT")
        self.assertIn('heading("LAYOUT")', fm)
        # at the bottom of the empty space's right-click menu
        ctx = fm[fm.index("function openMenu("):fm.index("// ================================================ This PC and Installed apps")]
        self.assertIn("for (const it of layoutItems()) items.push(it);", ctx)
        self.assertIn('if (id.startsWith("ab-")) { if (blocks) blocks.runMenu(id); return; }', fm)
        # the menus' items in order or left out (Edit layout › Menus…)
        self.assertIn('blocks.arrange(menuKind, items)', fm)
        self.assertIn('{ id: "view", title: "View"', fm)
        self.assertIn('{ id: "context", title: "Right-click (empty space)"', fm)

    def test_always_findable_edit_layout(self):
        for f in ("SettingsFrame.qml", "TaskManagerView.qml", "Capture.qml", "ResourceMonitorView.qml", "ReliabilityView.qml", "RestoreView.qml"):
            self.assertIn("EditLayoutButton {", read(f), f)

    def test_describe_it_opens_for_an_app(self):
        d = read("DescribeIt.qml")
        self.assertIn("function openApp(", d)
        self.assertIn('"describe-app", appId', d)
        self.assertIn('"app-keep", appId', d)
        self.assertIn('"app-undo", appId', d)
        sh = read("shell.qml")
        self.assertIn("function onDescribeAsked(a, n, c) { describe.openApp(a, n, c); }", sh)
        self.assertIn('target: "appblocks"', sh)

    def test_settings_pages_groups_are_blocks(self):
        sp = read("SetPage.qml")
        self.assertIn('source: "AppBlocksFlow.qml"', sp)
        self.assertIn("isSetHeader: true", read("SetHeader.qml"))
        fl = read("AppBlocksFlow.qml")
        for s in ('"Edit this page"', '"Move up"', '"Move down"', '"Hide this group"', '"Describe it…"', '"Back to the original"', "ctl.keepFor("):
            self.assertIn(s, fl, s)


class SettingsRedFlag(unittest.TestCase):
    def test_kept_only_through_hearth_layout(self):
        for f in ("AppBlocks.qml", "AppBlocksFlow.qml", "AppBlocksEditor.qml", "AppBlockCustomize.qml", "AppBlock.qml"):
            src = read(f)
            self.assertNotIn("setText(", src, f)
        for f in ("AppBlocksFlow.qml", "AppBlocksEditor.qml", "AppBlockCustomize.qml", "AppBlock.qml"):
            self.assertNotIn("SettingsWriter", read(f), f)
            self.assertNotIn('"hearth-layout"', read(f), f)
        ab = read("AppBlocks.qml")
        # the parts' looks and places (app-looks/<app>.json) have one writer: AppBlocks, through SettingsWriter (merged)
        self.assertEqual(ab.count("SettingsWriter {"), 1)
        self.assertIn('SettingsWriter { id: looksWriter; path: ab.app !== "" ? ab.looksFile : "" }', ab)
        self.assertIn('"/nimbus/app-looks/" + app + ".json"', ab)
        self.assertEqual(len(re.findall(r"looksWriter\.merge\(", ab)), 4, "written only by Keep this / Try it, Go back, and the part menu's Back to the original and its Undo")
        self.assertIn('"hearth-layout", "app-keep", app', ab)
        # a file that couldn't be read is never written over
        self.assertIn("if (unreadable) {", ab)
        self.assertIn("unreadable = true", ab)

    def test_nothing_runs_at_rest(self):
        for f in ("AppBlocks.qml", "AppBlocksFlow.qml", "AppBlocksHub.qml", "EditLayoutButton.qml", "AppBlocksEditor.qml", "AppBlockCustomize.qml", "AppBlock.qml"):
            src = read(f)
            for t in re.finditer(r"Timer \{([^}]*)\}", src):
                body = t.group(1)
                # (a block showing the clock ticks while it's on screen: only then)
                if 'running: bk.shows === "clock" && bk.visible' in body:
                    continue
                self.assertNotIn("repeat: true", body, f)
                self.assertNotIn("running: true", body, f)
            self.assertNotIn("running: true", src.replace("keeper.running = true", ""), f)

    def test_nothing_customised_moves_nothing(self):
        ab = read("AppBlocks.qml")
        # in "own" every part goes (or stays) home
        self.assertIn('want.set(it, mode === "own" ? "home"', ab)
        fl = read("AppBlocksFlow.qml")
        self.assertIn('if (saved === undefined && !everApplied && !editing) return;', fl)


class Board3(unittest.TestCase):
    """Drag and drop board 3 (docs/mockups/drag-drop/drag-drop-3.png; checklist part 3) in Mantel's apps."""

    def test_nothing_is_written_until_keep_this(self):
        ab = read("AppBlocks.qml")
        body = ab[ab.index("    function change(d) {"):ab.index("    function undo() {")]
        self.assertNotIn("keepChange", body)
        self.assertNotIn("merge(", body)
        for fn in ("function setLook(", "function hide(id) {", "function startFrom(", "function raise("):
            seg = ab[ab.index(fn):ab.index(fn) + 1400]
            seg = seg[:seg.index("\n    function ", 10)]
            if fn == "function hide(id) {":
                seg = seg[:seg.index("// not editing")]
            self.assertNotIn("keepChange", seg, fn)
            self.assertNotIn("looksWriter", seg, fn)
        # the only writes: write() (Keep this, Try it), goBack() and the part menu's own (outside Edit layout)
        w = ab[ab.index("    function write(d) {"):ab.index("    function keepThis() {")]
        self.assertIn("keepChange(ch);", w)
        self.assertIn("looksWriter.merge(m);", w)
        kt = ab[ab.index("    function keepThis() {"):ab.index("    function done() {")]
        self.assertIn("!sameDraft(draft, start)", kt, "Keep this with nothing changed writes nothing")
        ed = read("AppBlocksEditor.qml")
        cu = read("AppBlockCustomize.qml")
        for src in (ed, cu):
            self.assertNotIn("keepChange", src)
            self.assertNotIn("looksWriter", src)

    def test_the_editor_is_its_own_file_behind_a_loader(self):
        ab = read("AppBlocks.qml")
        self.assertIn('setSource("AppBlocksEditor.qml", { ab: ab })', ab)
        self.assertIn("active: ab.editing || ab.trying", ab)
        self.assertIn('Qt.createComponent("AppBlock.qml")', ab)
        ed = read("AppBlocksEditor.qml")
        self.assertIn('setSource("AppBlockCustomize.qml", { ab: ed.ab })', ed)

    def test_the_board(self):
        ed = read("AppBlocksEditor.qml")
        for s in ('text: "EDITING"', "L.partLabel(", "\"' blocks\" : \"'s blocks\"", '"drag into " + ed.ab.shortAppName', '"Find a block ("',
                  '["all", "All"], ["buttons", "Buttons"], ["panes", "Panes"], ["info", "Info"], ["own", "Your own"]',
                  'Section { title: "BUTTONS"', 'Section { title: "PANES"', '"MAKE YOUR OWN"', 'label: "Button · runs anything"', 'label: "Count · words · spacer"',
                  'label: "↶ Undo"', 'label: "↷ Redo"', '"Snap to grid" + (ed.snapOn ? " · " + ed.grid + " px" : "")', 'label: "┼ Guides"', 'label: "◫ Layers"',
                  '"Start from: " + ed.startName', 'label: "Menus…"', 'label: "Cancel"', 'label: "Try it"', 'label: "Keep this"',
                  '"THE SAME IN EVERY HEARTH APP · SETTINGS · TASK MANAGER · CAPTURE · NOTES · STORE … · RIGHT-CLICK ANY PART › CUSTOMIZE"',
                  "rotation: -2.5", "L.dropNote(", 'color: "#5ec8ff"', "setLineDash([8, 5])", '"Customize…"', '"Copy its look"', '"Paste a look"',
                  '"Duplicate"', '"Move to ›  "', '"Hide (keep it here)"', '"Take it out"', 'label: "Back to editing"', 'label: "Go back"'):
            self.assertIn(s, ed, s)

    def test_the_side_panel(self):
        cu = read("AppBlockCustomize.qml")
        for s in ('[["look", "Look"], ["shows", "Shows"], ["does", "Does"], ["place", "Place"], ["when", "When"], ["motion", "Motion"]]',
                  'label: "Reset"', 'Head { text: "SIZE AND PLACE" }', 'label: "Width"', '[["fit", "Fit"], ["fixed", "Fixed"], ["fill", "Fill"]]',
                  'label: "Height"', 'label: "Space around it"', 'label: "Space inside"', 'Head { text: "BACKGROUND AND EDGE" }', 'label: "Fill"',
                  'label: "See-through"', 'label: "Blur behind it"', 'label: "Corners"', '[["none", "None"], ["line", "Line"], ["glow", "Glow"]]',
                  '[["none", "None"], ["soft", "Soft"], ["deep", "Deep"]]', 'Head { text: "ICON AND WORDS" }', 'label: "Icon"', 'label: "Words"',
                  'label: "Font"', '[["light", "Light"], ["medium", "Medium"], ["bold", "Bold"]]', '[["icon", "Icon"], ["words", "Words"], ["both", "Both"]]',
                  "Head { text: \"WHEN IT'S POINTED AT · PRESSED\" }", '[["same", "Same"], ["lighter", "Lighter"], ["accent", "Accent"], ["own", "Own…"]]',
                  '[["same", "Same"], ["darker", "Darker"], ["own", "Own…"]]', 'Head { text: "THE OTHER TABS" }',
                  '[["press", "Press"], ["right", "Right-click"], ["middle", "Middle-click"], ["scroll", "Scroll"], ["hover", "Pointed at"]]',
                  '[["", "Its own"], ["nothing", "Nothing"], ["open", "Open"], ["run", "Run"], ["panel", "Show a panel"], ["action", "An action"]]',
                  '[["always", "Always"], ["hover", "Only when pointed at"], ["something", "When there\'s something to show"], ["pages", "Only on some pages"]]',
                  '[["none", "At once"], ["fade", "Fade"], ["slide", "Slide up"], ["pop", "Pop"]]', 'label: "Bounces when something new comes"',
                  'label: "Save as my own block…"', 'label: "Copy its look"', '"Apply to every bar item"'):
            self.assertIn(s, cu, s)

    def test_every_setting_is_drawn(self):
        ab = read("AppBlocks.qml")
        for s in ("L.sizedByLook(", "hd.look.shadow", 'hd.look.edge === "glow"', "hd.look.blur", "hd.look.see", "hd.look.fill", "hd.look.corners",
                  "hd.look.edgeW", "hd.look.icon", "L.fillWords(hd.look.words", "hd.look.font", "hd.look.fontSize", "hd.look.weight", "hd.look.show",
                  'l.hover === "lighter"', 'l.pressed === "darker"', "hd.look.does.press", "hd.look.does.right", "hd.look.does.middle",
                  "hd.look.does.scroll", "hd.look.does.hover", "L.showsNow(look", 'look.appear === "slide"', "bounceAnim.restart()", "hd.m[3]", "hd.look.pad"):
            self.assertIn(s, ab, s)
        bk = read("AppBlock.qml")
        for s in ("look.wMode", "look.hPx", "bk.look.shadow", 'bk.look.edge === "glow"', "bk.look.fill", "l.see", "bk.look.corners", "look.icon",
                  "bk.look.iconColor", "bk.look.iconSize", "look.words", "bk.look.font", "look.fontSize", "bk.look.weight", "look.show", "l.hover", "l.pressed",
                  "look.does", "L.showsNow(look", "look.appear", "look.bounce", "Layout.fillWidth"):
            self.assertIn(s, bk, s)

    def test_every_app_has_its_own_blocks(self):
        for f in APPS:
            src = read(f)
            self.assertRegex(src, r"\n\s+actions: ", f)
            self.assertRegex(src, r"\n\s+panes: ", f)
        fm = read("FileManager.qml")
        for s in ('label: "Sort my Downloads"', 'label: "Share"', 'label: "Open in terminal"', 'label: "Compress"',
                  '{ id: "preview", label: "Preview"', '{ id: "details", label: "Details"', '{ id: "tree", label: "Folder tree"', '{ id: "second", label: "Second folder"',
                  'marks: ({ "Search": searchBox })', 'label: "Sidebar"'):
            self.assertIn(s, fm, s)

    def test_no_fractional_font_sizes(self):
        # a 12.5 once made FileManager fail to load and took the whole shell down
        for f in ("AppBlocks.qml", "AppBlocksEditor.qml", "AppBlockCustomize.qml", "AppBlock.qml", "FileManager.qml"):
            self.assertNotRegex(read(f), r"pixelSize:\s*\d+\.\d", f)

    def test_ipc_for_the_pictures(self):
        sh = read("shell.qml")
        for s in ("function hold(app: string, id: string, x: real, y: real): string", "function letGo(app: string)", "function size(app: string, id: string, edge: string, dx: real, dy: real): string",
                  "function customize(app: string, id: string, tab: string)", "function keep(app: string)", "function tryIt(app: string)", "function goBack(app: string)"):
            self.assertIn(s, sh, s)


NODE = shutil.which("node")


@unittest.skipUnless(NODE, "node isn't here")
class Rules(unittest.TestCase):
    def run_js(self, body):
        src = read("app-blocks.js").replace(".pragma library", "")
        with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as f:
            f.write(src + "\nconst out = (function () {" + body + "})();\nconsole.log(JSON.stringify(out));\n")
            p = f.name
        try:
            r = subprocess.run([NODE, p], capture_output=True, text=True, timeout=30)
        finally:
            os.unlink(p)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout.strip().splitlines()[-1])

    DEFS = 'var D = [{id:"side",title:"Side",x:0,y:0,w:4,h:24},{id:"list",title:"List",x:4,y:0,w:14,h:24},{id:"pre",title:"Pre",x:18,y:0,w:6,h:24}];'

    def test_nothing_kept_is_the_apps_own(self):
        o = self.run_js(self.DEFS + 'return [resolve(D, undefined, {}).mode, resolve(D, {}, {}).mode, resolve(D, "x", {}).mode, resolve(D, [1], {}).mode];')
        self.assertEqual(o, ["own"] * 4)

    def test_what_doesnt_check_is_the_original(self):
        o = self.run_js(self.DEFS + '''return [
            {list:{x:20,w:10}}, {list:{w:0}}, {list:{x:-1}}, {list:{corners:99}}, {list:{seeThrough:100}}, {list:{font:"Wingdings"}},
            {list:{accent:"red"}}, {list:{hidden:"yes"}}, {list:{x:0, run:"rm -rf ~"}}, {list:"big"},
            {side:{hidden:true},list:{hidden:true},pre:{hidden:true}}].map(s => resolve(D, s, {}).mode);''')
        self.assertEqual(o, ["own"] * 11)

    def test_hidden_only_is_natural_and_places_are_the_grid(self):
        o = self.run_js(self.DEFS + '''
            var n = resolve(D, {side:{hidden:true}}, {});
            var k = resolve(D, {}, {pre:"off"});
            var g = resolve(D, {list:{x:0,y:0,w:18,h:24,corners:16}}, {});
            return [n.mode, n.placed[0].hidden, k.mode, k.placed[2].hidden, g.mode, g.placed[1].corners, g.placed[0].x];''')
        self.assertEqual(o, ["natural", True, "natural", True, "grid", 16, 0])

    def test_kept_reads_back_the_same(self):
        o = self.run_js(self.DEFS + '''
            var p = D.map(own); p[1].x = 5; p[1].w = 13; p[0].hidden = true; p[2] = nextLook(p[2], "corners");
            var j = toJson(p, false);
            var r = resolve(D, j, {});
            return [r.mode, JSON.stringify(r.placed) === JSON.stringify(p), "corners" in j.side, pageJson(D.map(own), D.map(own), "own")];''')
        self.assertEqual(o, ["grid", True, False, {}])

    def test_edit_layout_keeps_only_what_changed(self):
        o = self.run_js(self.DEFS + '''
            var s = D.map(own), d = D.map(own); d[0].hidden = true;
            var moved = D.map(own); moved[1].w = 10;
            return [pageJson(d, s, "own"), Object.keys(pageJson(moved, s, "own").list)];''')
        self.assertEqual(o[0], {"side": {"hidden": True}, "list": {}, "pre": {}})
        self.assertEqual(o[1], ["x", "y", "w", "h"])

    def test_a_part_with_nothing_to_show_gives_its_place(self):
        o = self.run_js(self.DEFS + '''
            var p = D.map(own);
            var a = reflow(p, {pre: false});
            p[0].hidden = true;
            var b = reflow(p, {});
            return [a[1].w, b[1].x, b[1].w];''')
        self.assertEqual(o, [20, 0, 18])

    def test_a_place_already_covered_isnt_given_again(self):
        # the pane under the folder (kept over it) has nothing to show: its place isn't handed to the one beside it
        o = self.run_js('''
            var p = [{id:"folder",x:0,y:0,w:18,h:24},{id:"split",x:12,y:0,w:6,h:24},{id:"side",x:18,y:0,w:6,h:24}];
            var r = reflow(p, {split: false});
            return [r[2].x, r[2].w, r[0].w];''')
        self.assertEqual(o, [18, 6, 18])

    def test_sizing_pulls_the_neighbour_and_moving_swaps(self):
        o = self.run_js(self.DEFS + '''
            var p = D.map(own);
            var s = sized(p, "side", {r:1}, 3, 0, 24, 24);
            var m = p.map(x => Object.assign({}, x)); m[0] = moved(p[0], 18, 0, 24, 24);
            var sw = dropped(m, "side", p);
            return [s[0].w, s[1].x, s[1].w, sw[0].x, sw[0].w, sw[2].x, sw[2].w, moved(p[1], 50, 50, 24, 24).x];''')
        self.assertEqual(o, [7, 7, 11, 18, 6, 0, 4, 10])

    def test_inside_and_beside_give_way(self):
        o = self.run_js('''
            var D = [{id:"folder",title:"F",x:4,y:6,w:20,h:18},{id:"status",title:"S",x:4,y:22,w:20,h:2},
                     {id:"pre",title:"P",x:19,y:6,w:5,h:18,beside:["folder","status"],size:5}];
            var c = carve(D.map(own).slice(0, 2));
            var b = besides(D.map(own), D, {folder:true, status:true, pre:false});
            return [c[0].h, b[2].x, b[2].w, b[0].w, b[1].w];''')
        self.assertEqual(o, [16, 19, 5, 15, 15])

    def test_menus_in_order_and_left_out(self):
        o = self.run_js('''
            var items = [{id:"a"},{id:"b"},{sep:true},{id:"c"},{id:"ab-edit"}];
            var r = arrangeMenu(items, ["c", "-a", "-ab-edit"]);
            return [r.map(i => i.sep ? "|" : i.id), menuZone(["c","b"], {a:true}), arrangeMenu(items, []).length];''')
        self.assertEqual(o, [["c", "|", "b", "|", "ab-edit"], ["c", "b", "-a"], 5])

    def test_catalog_is_what_hearth_layout_checks(self):
        o = self.run_js(self.DEFS + 'return catalog([{id:"mantel-window",title:"W",blocks:D}]);')
        self.assertEqual(o["cols"], 24)
        self.assertEqual(o["pages"][0]["blocks"][2], {"id": "pre", "title": "Pre", "x": 18, "y": 0, "w": 6, "h": 24})

    # ---- board 3
    def test_a_look_is_checked_one_setting_at_a_time(self):
        o = self.run_js('''return [checkLook({fill: "#112233", see: 200, bogus: 1, edge: "glow", edgeW: 99, m: [1, 2, 3, 4], does: {press: {k: "run", a: "x"}}}),
                                checkLook("x"), checkLook({does: {kick: {k: "run"}}}), checkLook({words: "a\\u0001b", font: "Inter", weight: "heavy"})];''')
        self.assertEqual(o[0], {"fill": "#112233", "edge": "glow", "m": [1, 2, 3, 4], "does": {"press": {"k": "run", "a": "x"}}})
        self.assertIsNone(o[1])
        self.assertEqual(o[2], {})
        self.assertEqual(o[3], {"font": "Inter"})

    def test_what_is_kept_beside_the_cells_is_read_safely(self):
        o = self.run_js('''return [readExtras({v: 1, rects: {a: {x: 0, y: 0, w: 0.2, h: 1}, zz: {x: 0, y: 0, w: 1, h: 1}, b: {x: 0.9, y: 0, w: 0.5, h: 1}},
                                    looks: {a: {fill: "#112233"}, nope: {fill: "#000000"}},
                                    added: [{id: "b-1", kind: "action", ref: "share", in: "toolbar", at: 2}, {id: "bad", kind: "action"},
                                            {id: "b-2", kind: "button", in: "", x: 0.5, y: 0.5}], z: ["a", "q", "b-1"]}, ["a", "b"]),
                         readExtras("broken", ["a"]), extrasJson({rects: {}, looks: {a: {}}, z: [], added: []})];''')
        self.assertEqual(o[0]["rects"], {"a": {"x": 0, "y": 0, "w": 0.2, "h": 1}})
        self.assertEqual(o[0]["looks"], {"a": {"fill": "#112233"}})
        self.assertEqual([b["id"] for b in o[0]["added"]], ["b-1"])
        self.assertEqual(o[0]["z"], ["a", "b-1"])
        self.assertEqual(o[1], {"rects": {}, "looks": {}, "z": [], "added": []})
        self.assertIsNone(o[2])

    def test_labels_name_each_part_and_its_size(self):
        o = self.run_js('''return [partLabel("The command bar", {x: 0, y: 0, w: 1500, h: 48}, 1600, 900),
                                partLabel("Sidebar", {x: 0, y: 0, w: 216, h: 800}, 1600, 900),
                                partLabel("The folder", {x: 216, y: 0, w: 900, h: 700}, 1600, 900)];''')
        self.assertEqual(o, ["COMMAND BAR · 48 px", "SIDEBAR · 216 px · DRAG ITS EDGE", "FOLDER · 900 × 700"])

    def test_a_pane_sized_by_its_edge_lines_up_and_says_so(self):
        o = self.run_js('''var L = [{id: "side", title: "Sidebar", x: 0, y: 0, w: 216, h: 800}, {id: "folder", title: "The folder", x: 216, y: 0, w: 884, h: 800},
                                     {id: "preview", title: "Preview", x: 1100, y: 0, w: 500, h: 800}];
                            var a = sizePx(L, "preview", {l: 1}, -16, 0, 1600, 900, {grid: 8, guides: true, marks: [{name: "Search", x: 1086, y: 10, w: 400, h: 30}]});
                            var b = sizePx(L, "side", {r: 1}, 13, 0, 1600, 900, {grid: 8, guides: false});
                            return [a.note, a.list[2].x, a.list[2].w, a.list[1].w, a.guides, b.list[0].w, b.list[1].x, b.note];''')
        self.assertEqual(o, ["Preview: 514 px · lined up with Search", 1086, 514, 870, [{"x": 1086}], 232, 232, "Sidebar: 232 px"])

    def test_parts_go_anywhere_snapped_and_lined_up(self):
        o = self.run_js('''var L = [{id: "a", name: "the sidebar", x: 0, y: 100, w: 200, h: 700}, {id: "b", x: 400, y: 0, w: 300, h: 300}];
                            var m = movePx(L, "b", 333, 397, 1600, 900, {grid: 8, guides: true});
                            var f = movePx(L, "b", 5000, 5000, 1600, 900, {});
                            var g = movePx(L, "b", -197, 103, 1600, 900, {grid: 8, guides: true});
                            return [m.list[1].x, m.list[1].y, f.list[1].x, f.list[1].y, g.list[1].x, g.list[1].y, g.note];''')
        self.assertEqual(o, [736, 400, 1300, 600, 200, 100, "Lined up with the sidebar"])

    def test_blocks_go_in_among_a_bars_own_items(self):
        o = self.run_js('''return [barOrder(3, [{id: "x", at: 2}, {id: "y", at: 0}, {id: "z", at: 9}]), barOrder(0, [{id: "a", at: 4}]),
                                insertAt([10, 50, 90], 60), dropNote("The command bar", "Sort", false), dropNote("", "", true)];''')
        self.assertEqual(o, [["y", 0, 1, "x", 2, "z"], ["a"], 2, "Drop: command bar, after Sort", "Drop: on the window, anywhere"])

    def test_the_apps_blocks_window(self):
        o = self.run_js('''var l = library([{id: "sortdl", label: "Sort my Downloads"}, {id: "share", label: "Share"}], [{id: "preview", label: "Preview"}],
                                         [{id: "items", label: "Items here"}], [{id: "b-1", kind: "button", label: "Backup"}, {id: "b-2", kind: "count"}]);
                            return [l.map(b => b.id), find(l, "buttons", "").length, find(l, "all", "sort").map(b => b.name), find(l, "own", "").length,
                                    newBlock("make:button", "b-9"), newBlock("action:share", "b-8").kind, newBlock("make:count", "b-7").kind, newBlock("x:y", "b-6")];''')
        self.assertEqual(o[0], ["action:sortdl", "action:share", "pane:preview", "count:items", "own:b-1"])
        self.assertEqual(o[1:4], [2, ["Sort my Downloads"], 1])
        self.assertEqual(o[4], {"id": "b-9", "kind": "button", "label": "My button", "run": ""})
        self.assertEqual(o[5:], ["action", "count", None])

    def test_looks_size_fill_and_when(self):
        o = self.run_js('''var r = {x: 100, y: 100, w: 200, h: 100};
                            return [sizedByLook(r, {wMode: "fixed", wPx: 320}, null, [], 1600, 900).w,
                                    sizedByLook(r, {wMode: "fill"}, null, [{x: 900, y: 50, w: 100, h: 400}], 1600, 900).w,
                                    sizedByLook(r, {wMode: "fit", hMode: "fit"}, {w: 150, h: 40}, [], 1600, 900),
                                    showsNow({when: "hover"}, false, true, "p"), showsNow({when: "hover"}, true, true, "p"), showsNow({when: "something"}, false, false, "p"),
                                    showsNow({when: "pages", pages: ["a"]}, false, true, "b"), showsNow({}, false, false, "b"),
                                    fillWords("Downloads · {count} {nope}", {count: 3}), pasteLook({words: "w", fill: "#000000", does: {press: {k: "run"}}}, {fill: "#ffffff", words: "x"})];''')
        self.assertEqual(o[0:2], [320, 800])
        self.assertEqual(o[2], {"x": 100, "y": 100, "w": 150, "h": 40})
        self.assertEqual(o[3:8], [False, True, False, False, True])
        self.assertEqual(o[8], "Downloads · 3 {nope}")
        self.assertEqual(o[9], {"words": "w", "does": {"press": {"k": "run"}}, "fill": "#ffffff"})

    def test_cells_and_pixels_and_start_from(self):
        o = self.run_js('''var p = [{id: "bar", x: 0, y: 0, w: 1600, h: 48}, {id: "side", x: 0, y: 48, w: 300, h: 852}, {id: "main", x: 300, y: 48, w: 1300, h: 852}];
                            var ps = presets(p, 1600, 900);
                            var foc = ps.find(x => x.id === "focus").parts;
                            return [pxCell(cellPx({x: 3, y: 4, w: 5, h: 6}, 1600, 900), 1600, 900), toFrac({x: 400, y: 0, w: 400, h: 450}, 1600, 900),
                                    ps.map(x => x.id), ps[1].parts[1].x, foc[1].hidden, foc[2].x, foc[2].w, ps[3].parts[0].y,
                                    reflowPx([{id: "a", x: 0, y: 0, w: 200, h: 800}, {id: "b", x: 201, y: 0, w: 899, h: 800, hidden: true}], {})[0].w];''')
        self.assertEqual(o[0], {"x": 3, "y": 4, "w": 5, "h": 6})
        self.assertEqual(o[1], {"x": 0.25, "y": 0, "w": 0.25, "h": 0.5})
        self.assertEqual(o[2], ["hearth", "mirror", "focus", "bars-below"])
        self.assertEqual(o[3:], [1300, True, 0, 1600, 852, 1100])


if __name__ == "__main__":
    unittest.main()
