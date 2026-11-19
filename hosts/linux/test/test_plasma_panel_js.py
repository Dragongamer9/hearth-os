"""Settings › Personalization › Bar on KDE Plasma (the Fedora edition; hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs).
The owner on the Ally (Nov 19): switching the app menu put it on the right side, and panel transparency did nothing.
  * The launcher swap (SWAP_LAUNCHER_JS) runs in node against a stand-in panel: the new launcher takes the old one's
    place in AppletOrder (also when AppletOrder wasn't written yet), the old one goes, nothing else moves.
  * Transparency writes the panel's own plasmashellrc line (panelOpacity) for every panel, so it survives signing
    out, and uses the scripting's opacity live where Plasma has it."""
import pathlib
import re
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
DESK = ROOT / 'hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs'

FAKE = r"""
function Panel(types, order) {
  var self = this; this.cfg = {AppletOrder: order}; this.ws = []; this.next = 100; this.reloaded = 0;
  types.forEach(function (t, i) { self.ws.push(self.mk(t, i + 1)); });
}
Panel.prototype.mk = function (t, id) { var self = this; return {type: t, id: id, remove: function () { self.ws = self.ws.filter(function (w) { return w.id !== id; }); }}; };
Panel.prototype.widgets = function () { return this.ws.slice(); };
Panel.prototype.readConfig = function (k, d) { return this.cfg[k] === undefined ? d : this.cfg[k]; };
Panel.prototype.writeConfig = function (k, v) { this.cfg[k] = v; };
Panel.prototype.reloadConfig = function () { this.reloaded++; };
Panel.prototype.addWidget = function (t) { var w = this.mk(t, this.next++); this.ws.push(w); return w; };
function run(types, order) {
  var p = new Panel(types, order);
  (function (p) { __JS__ })(p);
  var byId = {}; p.ws.forEach(function (w) { byId[w.id] = w.type; });
  return {order: String(p.cfg.AppletOrder).split(';').map(function (x) { return byId[x] || ('?' + x); }), reloaded: p.reloaded};
}
var out = [];
out.push(run(['org.kde.plasma.kickoff', 'org.kde.plasma.icontasks', 'org.kde.plasma.systemtray', 'org.kde.plasma.digitalclock'], '1;2;3;4'));
out.push(run(['org.kde.plasma.kickoff', 'org.kde.plasma.icontasks', 'org.kde.plasma.systemtray'], ''));
out.push(run(['org.kde.plasma.pager', 'org.kde.plasma.kicker', 'org.kde.plasma.icontasks'], '1;2;3'));
out.push(run(['org.kde.plasma.icontasks', 'org.kde.plasma.systemtray'], '1;2'));
console.log(JSON.stringify(out));
"""


def swap_js():
    m = re.search(r'pub const SWAP_LAUNCHER_JS: &str = r#"(.*?)"#;', DESK.read_text(), re.S)
    return m.group(1)


@unittest.skipUnless(shutil.which('node'), 'node runs the panel script')
class Swap(unittest.TestCase):
    def test_the_new_launcher_takes_the_old_ones_place(self):
        js = swap_js().replace('__NEW__', '"org.kde.plasma.kickerdash"')
        out = subprocess.run(['node', '-e', FAKE.replace('__JS__', js)], capture_output=True, text=True, timeout=20)
        self.assertEqual(out.returncode, 0, out.stderr)
        import json
        r = json.loads(out.stdout)
        self.assertEqual(r[0]['order'], ['org.kde.plasma.kickerdash', 'org.kde.plasma.icontasks', 'org.kde.plasma.systemtray', 'org.kde.plasma.digitalclock'])
        self.assertEqual(r[1]['order'], ['org.kde.plasma.kickerdash', 'org.kde.plasma.icontasks', 'org.kde.plasma.systemtray'],
                         'no AppletOrder written yet: still in the old one\'s place, not at the end')
        self.assertEqual(r[2]['order'], ['org.kde.plasma.pager', 'org.kde.plasma.kickerdash', 'org.kde.plasma.icontasks'])
        self.assertEqual(r[3]['order'], ['org.kde.plasma.kickerdash', 'org.kde.plasma.icontasks', 'org.kde.plasma.systemtray'],
                         'no launcher before: it goes first')
        self.assertTrue(all(x['reloaded'] == 1 for x in r))


class Opacity(unittest.TestCase):
    def test_written_to_plasmashellrc_and_live(self):
        src = DESK.read_text()
        fn = src[src.index('pub fn opacity_cmd'):src.index('pub fn glass')]
        self.assertIn('kwriteconfig6 --file plasmashellrc --group PlasmaViews --group \\"Panel $id\\" --key panelOpacity {n}', fn)
        self.assertIn('"opaque" => 1, "translucent" => 2, _ => 0', fn)
        self.assertIn('typeof ps[i].opacity!==', fn)
        self.assertIn('plasma-plasmashell.service', fn)
        self.assertIn('t.push(kde(opacity_cmd(o)));', src)

    def test_the_look_sync_never_writes_panel_opacity(self):
        looks = (ROOT / 'os-bases/anatase/system_files/usr/libexec/nimbus/hearth-plasma-looks').read_text()
        self.assertNotIn('panelOpacity', looks)
        self.assertNotIn('--resetLayout', looks)


if __name__ == '__main__':
    unittest.main()
