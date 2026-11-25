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
        for label in ("Edit layout", "Move this", "Hide this", "Describe it…", "Back to the original"):
            self.assertIn('"%s"' % label, ab)
        # the app's own menus' LAYOUT items
        self.assertIn('{ id: "ab-edit", label: "Edit layout"', ab)
        self.assertIn('{ id: "ab-describe", label: "Describe it…"', ab)
        # editing in place: drag, sizing edges, hide, the tray, Undo, Back to the original, Done, Esc and Ctrl+Z, the controller
        for s in ("function grab(", "function dragTo(", "function release(", "L.sized(", "L.dropped(", "Hidden:", "Qt.Key_Escape",
                  "Qt.Key_Z", "Qt.Key_PageDown", "Qt.Key_PageUp", "Qt.Key_Menu", 'label: "Undo"', 'label: "Done"', 'model: ["corners", "see", "accent"]'):
            self.assertIn(s, ab, s)

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
        for f in ("AppBlocks.qml", "AppBlocksFlow.qml"):
            src = read(f)
            self.assertNotIn("setText(", src, f)
            self.assertNotIn("SettingsWriter", src, f)
        ab = read("AppBlocks.qml")
        self.assertIn('"hearth-layout", "app-keep", app', ab)
        # a file that couldn't be read is never written over
        self.assertIn("if (unreadable) {", ab)
        self.assertIn("unreadable = true", ab)

    def test_nothing_runs_at_rest(self):
        for f in ("AppBlocks.qml", "AppBlocksFlow.qml", "AppBlocksHub.qml", "EditLayoutButton.qml"):
            src = read(f)
            for t in re.finditer(r"Timer \{([^}]*)\}", src):
                body = t.group(1)
                self.assertNotIn("repeat: true", body, f)
                self.assertNotIn("running: true", body, f)
            self.assertNotIn("running: true", src.replace("keeper.running = true", ""), f)

    def test_nothing_customised_moves_nothing(self):
        ab = read("AppBlocks.qml")
        # in "own" every part goes (or stays) home
        self.assertIn('want.set(it, mode === "own" ? "home"', ab)
        fl = read("AppBlocksFlow.qml")
        self.assertIn('if (saved === undefined && !everApplied && !editing) return;', fl)


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


if __name__ == "__main__":
    unittest.main()
