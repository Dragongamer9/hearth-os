"""The website's look picker matches Hearth OS's looks (tools/site-looks.py, from theme/palettes.json)."""
import pathlib, subprocess, sys, unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


class SiteLooks(unittest.TestCase):
    def test_generated_files_are_current(self):
        r = subprocess.run([sys.executable, str(ROOT / "tools/site-looks.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_pages_load_the_picker(self):
        index = (ROOT / "site/index.html").read_text()
        self.assertIn('href="looks.css"', index)
        self.assertIn('src="looks.js"', index)
        self.assertIn('class="looks"', index)
        docs = (ROOT / "tools/build-docs.py").read_text()
        self.assertIn('../looks.css', docs)
        self.assertIn('../looks.js', docs)
        self.assertIn("hearth.look", docs)


if __name__ == "__main__":
    unittest.main()
