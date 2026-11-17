"""The website's look picker matches Hearth OS's looks (tools/site-looks.py, from theme/palettes.json)."""
import pathlib, subprocess, sys, unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


class SiteLooks(unittest.TestCase):
    def test_generated_files_are_current(self):
        r = subprocess.run([sys.executable, str(ROOT / "tools/site-looks.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_pages_load_the_picker(self):
        index = (ROOT / "site/index.html").read_text()
        self.assertRegex(index, r'href="looks\.css(\?v=[0-9a-f]+)?"')
        self.assertRegex(index, r'src="looks\.js(\?v=[0-9a-f]+)?"')
        self.assertIn('class="looks"', index)
        docs = (ROOT / "tools/build-docs.py").read_text()
        self.assertIn('../looks.css', docs)
        self.assertIn('../looks.js', docs)
        self.assertIn("hearth.look", docs)

    def test_gold_look_with_its_own_song_and_background(self):
        """Gold (black and gold, the Server look) is in the picker, with its own song, its glow and its beat effect."""
        import json, re
        js = (ROOT / "site/looks.js").read_text()
        looks = json.loads(re.search(r"var LOOKS = (\[.*?\]), DEF", js).group(1))
        gold = [l for l in looks if l["id"] == "gold"]
        self.assertEqual(len(gold), 1, "Gold is missing from the site's looks")
        self.assertEqual(gold[0]["name"], "Gold")
        self.assertEqual(gold[0]["desc"], "Black and gold, the Server look")
        self.assertEqual(gold[0]["wall"], "gold")
        self.assertIn("gold: function", js)
        self.assertTrue((ROOT / "site/img/looks/gold-glow.png").is_file())
        css = (ROOT / "site/looks.css").read_text()
        self.assertIn(':root[data-look="gold"]', css)
        self.assertIn("25E2", css)                          # the ◢ labels
        self.assertIn('html[data-frame="gold"]', css)        # the corner marks
        index = (ROOT / "site/index.html").read_text()
        track = re.search(r"LOOK_TRACK = \{ gold: '([a-z0-9-]+)' \}", index)
        self.assertTrue(track, "Gold has no song of its own")
        self.assertEqual(track.group(1), "hearth-story")
        self.assertIn("'" + track.group(1) + "']", index)   # it's in the soundtrack list
        self.assertTrue((ROOT / "site/music" / (track.group(1) + ".mp3")).is_file())
        self.assertIn("gold: 'gold'", index)                # the background moves to the beat
        # Steam-style stays the site's default look.
        self.assertIn('DEF = "steam"', js)

    def test_gold_light_mode_is_readable(self):
        """Gold's light mode keeps text and links at 4.5:1 or better."""
        import importlib.util
        spec = importlib.util.spec_from_file_location("site_looks", ROOT / "tools/site-looks.py")
        m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

        def lum(h):
            c = [int(h.lstrip("#")[i:i + 2], 16) / 255 for i in (0, 2, 4)]
            c = [x / 12.92 if x <= .03928 else ((x + .055) / 1.055) ** 2.4 for x in c]
            return .2126 * c[0] + .7152 * c[1] + .0722 * c[2]

        def ratio(a, b):
            la, lb = sorted([lum(a), lum(b)], reverse=True); return (la + .05) / (lb + .05)

        for mode in ("light", "dark"):
            base, acc = m.vars_for(m.SITE_LOOKS["gold"][mode], mode)
            for bg in ("--deep", "--page", "--panel", "--raise"):
                for fg in ("--hi", "--text", "--muted"):
                    self.assertGreaterEqual(ratio(base[fg], base[bg]), 4.5, f"{mode} {fg} on {bg}")
                self.assertGreaterEqual(ratio(acc["--acc"], base[bg]), 4.5, f"{mode} accent on {bg}")
            self.assertGreaterEqual(ratio(acc["--onacc"], acc["--acc"]), 4.5, f"{mode} button text")


if __name__ == "__main__":
    unittest.main()
