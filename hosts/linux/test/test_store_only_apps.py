"""Store apps aren't built in (the owner, Nov 27: "Forge isn't a built in app also it shows it's installed when it's not";
"Remember just because we add apps don't make them built in or installed by default").

Only what was in the Hearth OS 1.18 image is built in. Every app made since is a Store app: not in the image, listed
in the Store as "kind": "app" with "inImage": false, and got from the Hearth Store into the home folder. This fails when
a new app is put in the image or listed as built in without being on the lists of built-in apps below (adding one there
is the owner's call), and when the image build, quick-update.yml and the Store's list disagree on which apps are Store
apps."""
import json
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]

# Hearth's pure-Rust apps in the Hearth OS 1.18 image (v1.18.0's os-bases/peppermint/build.sh), and the ones installed
# by their own steps there (G15, Family and friends).
BUILT_IN_PROGRAMS = {
    "hearth-task-manager", "hearth-system-restore", "hearth-reliability-history", "hearth-settings-app", "hearth-files-app",
    "hearth-file-chooser", "hearth-capture-app", "hearth-pad-host", "hearth-store-app", "hearth-server-app", "hearth-resmon-app",
    "hearth-notes-app", "hearth-ideas-app", "hearth-layout", "hearth-g15", "hearth-g15-app", "hearth-family-app",
}
# The built-in entries in v1.18.0's own Store list (os/system_files/usr/share/hearth/store/catalog.json).
BUILT_IN_IDS = {
    "g15", "hearth-notes", "beacon", "usb-maker", "mantel", "files", "settings", "task-manager", "capture", "family", "server",
    "system-restore", "reliability", "resmon", "store", "controller", "file-chooser", "mantel-rust", "desktop-layouts", "looks",
    "sounds", "game-mode", "decky-hearth-sync", "decky-hearth-link", "decky-hearth-music", "home-server-helper", "hearth-link",
    "ai-engines", "storage-helper", "ai-share-helper", "hearth-core", "recovery", "ideas", "controller-lab", "saves", "mods",
    "gamestore", "colour-filter", "steam-engine",
}


def words(s):
    return set(s.split())


class StoreOnlyApps(unittest.TestCase):
    def setUp(self):
        self.build = (ROOT / "os-bases/peppermint/build.sh").read_text()
        self.qu = (ROOT / ".github/workflows/quick-update.yml").read_text()
        self.cat = json.loads((ROOT / "os/system_files/usr/share/hearth/store/catalog.json").read_text())
        m = re.search(r'^STORE_ONLY_APPS="([^"]+)"', self.build, re.M)
        self.assertIsNotNone(m, "build.sh names its Store apps (STORE_ONLY_APPS)")
        self.store_only = words(m.group(1))

    def test_the_image_has_only_the_built_in_apps(self):
        loop = re.search(r'^STORE_ONLY_APPS=.*\nfor b in ([^;]+); do', self.build, re.M)
        self.assertIsNotNone(loop, "the pure-Rust apps' loop follows STORE_ONLY_APPS")
        put_in = words(loop.group(1))
        extra = sorted(put_in - BUILT_IN_PROGRAMS)
        self.assertFalse(extra, f"{extra} would be built into Hearth OS: they're Store apps (STORE_ONLY_APPS), unless the owner says to build them in")
        self.assertFalse(put_in & self.store_only)

    def test_every_hearth_app_is_built_in_or_a_store_app(self):
        for cargo in sorted((ROOT / "hosts/hearth-ui/apps").glob("*/Cargo.toml")):
            for name in re.findall(r'\[\[bin\]\]\s*name = "([^"]+)"', cargo.read_text()):
                self.assertTrue(name in BUILT_IN_PROGRAMS or name in self.store_only,
                                f"{name} ({cargo.parent.name}) is new: add it to STORE_ONLY_APPS (build.sh) and store_apps (quick-update.yml); it isn't built in")

    def test_quick_update_writes_them_as_store_apps(self):
        m = re.search(r"store_apps = \(([^)]*)\)", self.qu, re.S)
        self.assertIsNotNone(m)
        self.assertEqual(set(re.findall(r'"([^"]+)"', m.group(1))), self.store_only, "quick-update.yml's store_apps is build.sh's STORE_ONLY_APPS")
        self.assertIn('e["kind"] = "app"', self.qu)
        self.assertIn('e["inImage"] = False', self.qu)

    def test_the_list_never_calls_them_built_in(self):
        for a in self.cat["apps"]:
            prog = a.get("partId") or (a.get("open") or "").split(" ")[0]
            if prog in self.store_only:
                self.assertEqual(a.get("kind"), "app", f"{a['id']} is a Store app, not built in")
                self.assertIs(a.get("inImage"), False, a["id"])
            if a.get("kind") in ("builtin", "built-in"):
                self.assertIn(a["id"], BUILT_IN_IDS, f"{a['id']} is listed as built into Hearth OS: only what 1.18 had is (make it \"kind\": \"app\")")

    def test_their_menu_entries_arent_in_the_image(self):
        self.assertIn("for b in $STORE_ONLY_APPS; do", self.build)
        self.assertIn('share/applications', self.qu, "a Store app's menu entry comes with its Store copy")


if __name__ == "__main__":
    unittest.main()
