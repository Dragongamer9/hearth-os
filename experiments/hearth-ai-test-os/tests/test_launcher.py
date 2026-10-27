"""Exercise optional per-user launcher in a temporary home, never system paths."""
import os
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "install-desktop-launcher.sh"

class DesktopLauncherTests(unittest.TestCase):
    def test_install_and_remove_with_spaces(self):
        with tempfile.TemporaryDirectory(prefix="hearth launcher test ") as temp:
            root = pathlib.Path(temp)
            appdir = root / "user data" / "applications"
            env = dict(os.environ, XDG_DATA_HOME=str(root / "user data"))
            result = subprocess.run(["sh", str(INSTALLER), "install"], env=env,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            desktop = appdir / "hearth-ai-experimental.desktop"
            entry = desktop.read_text(encoding="utf-8")
            self.assertIn("Type=Application", entry)
            self.assertIn("Terminal=true", entry)
            self.assertIn('Exec="', entry)
            self.assertIn("launch.sh", entry)
            self.assertIn("Icon=", entry)
            self.assertIn("hearth-ai.svg", entry)
            self.assertTrue((ROOT / "hearth-ai.svg").is_file())
            result = subprocess.run(["sh", str(INSTALLER), "--remove"], env=env,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(desktop.exists())

if __name__ == "__main__":
    unittest.main()
