"""The shortcuts reach the Mantel that's running (the owner, Nov 22: "i updated mantel and nonw windows button wont open
app menu"): with a quick update from the Hearth Store, Mantel runs from your home folder, so nothing may call the
built-in folder directly; hearth-mantel-ipc follows $XDG_RUNTIME_DIR/hearth-mantel-shell, written by labwc's autostart."""
import os, pathlib, subprocess, tempfile, unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
IPC = ROOT / "os/system_files/usr/bin/hearth-mantel-ipc"


class MantelIpc(unittest.TestCase):
    def test_no_shortcut_calls_the_built_in_folder(self):
        for f in ["hosts/linux/desktop/labwc/rc.xml", "hosts/linux/desktop/labwc/menu.xml", "hosts/linux/desktop/open.sh"]:
            t = (ROOT / f).read_text()
            self.assertNotIn("quickshell ipc -p $NIMBUS_SHARE/desktop/shell", t, f)
            self.assertNotIn('quickshell ipc -p "$NIMBUS_SHARE/desktop/shell"', t, f)
            self.assertNotIn("quickshell ipc -p &quot;$NIMBUS_SHARE", t, f)
        rc = (ROOT / "hosts/linux/desktop/labwc/rc.xml").read_text()
        self.assertIn("""<keybind key="Super_L" onRelease="yes"><action name="Execute" command="sh -c 'hearth-mantel-ipc call launcher toggle'" />""", rc)

    def test_autostart_says_where_mantel_runs(self):
        a = (ROOT / "hosts/linux/desktop/labwc/autostart").read_text()
        i = a.index('printf \'%s\' "$pick" > "${XDG_RUNTIME_DIR:-/tmp}/hearth-mantel-shell"')
        self.assertLess(i, a.index('quickshell -p "$pick"'))

    def run_ipc(self, run, share):
        with tempfile.TemporaryDirectory() as b:
            q = pathlib.Path(b, "quickshell")
            q.write_text('#!/bin/sh\necho "$@"\n')
            q.chmod(0o755)
            env = dict(os.environ, PATH=f"{b}:{os.environ['PATH']}", XDG_RUNTIME_DIR=run, NIMBUS_SHARE=share)
            return subprocess.run(["sh", str(IPC), "call", "launcher", "toggle"], env=env, capture_output=True, text=True).stdout.strip()

    def test_it_follows_the_store_copy_and_falls_back(self):
        with tempfile.TemporaryDirectory() as run, tempfile.TemporaryDirectory() as store:
            self.assertEqual(self.run_ipc(run, "/usr/share/nimbus"), "ipc -p /usr/share/nimbus/desktop/shell call launcher toggle")
            pathlib.Path(store, "shell.qml").write_text("")
            pathlib.Path(run, "hearth-mantel-shell").write_text(store)
            self.assertEqual(self.run_ipc(run, "/usr/share/nimbus"), f"ipc -p {store} call launcher toggle")
            pathlib.Path(run, "hearth-mantel-shell").write_text("/nowhere")
            self.assertEqual(self.run_ipc(run, "/x"), "ipc -p /x/desktop/shell call launcher toggle", "a folder without Mantel: the built-in one")


if __name__ == "__main__":
    unittest.main()
