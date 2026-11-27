"""Red flag 13 (the owner, Nov 22): every app Hearth makes, Mantel included, can be updated from the Hearth Store. Every
pure-Rust app's program and Mantel are choices in Actions › Quick update (quick-update.yml), and Hearth's shortcuts never
call Mantel only at its built-in folder (a Store copy runs from the home folder: hearth-mantel-ipc)."""
import pathlib, re, unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
# helpers that are part of another app's quick update, not apps of their own
# (hearth-server-more-app only shows the Home server helper's five more pages on their own for CI's pictures; the pages
# themselves are built into hearth-home-server-app and update with it; it never goes into Hearth OS.)
NOT_APPS = {"hearth-g15", "hearth-server-more-app"}


class StoreParts(unittest.TestCase):
    def test_every_app_can_have_a_quick_update(self):
        q = (ROOT / ".github/workflows/quick-update.yml").read_text()
        m = re.search(r"options: \[([^\]]*)\]", q, re.S)
        opts = {x.strip() for x in m.group(1).replace("\n", " ").split(",")}
        self.assertIn("mantel", opts)
        for cargo in sorted((ROOT / "hosts/hearth-ui/apps").glob("*/Cargo.toml")):
            for name in re.findall(r'\[\[bin\]\]\s*name = "([^"]+)"', cargo.read_text()):
                if name not in NOT_APPS:
                    self.assertIn(name, opts, f"{name} ({cargo.parent.name}) can't get a quick update: add it to quick-update.yml")

    def test_shortcuts_find_the_mantel_thats_running(self):
        rc = (ROOT / "hosts/linux/desktop/labwc/rc.xml").read_text()
        self.assertNotIn("quickshell ipc -p $NIMBUS_SHARE/desktop/shell", rc)
        self.assertTrue((ROOT / "os/system_files/usr/bin/hearth-mantel-ipc").is_file())
        self.assertTrue((ROOT / "hosts/linux/desktop/shell/hearth-mantel-answer").is_file())


if __name__ == "__main__":
    unittest.main()
