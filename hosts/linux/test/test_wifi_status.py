"""Settings › Wi-Fi: the network you're on is found even on a mesh, and backed by NetworkManager's active link."""
import pathlib, unittest

QML = pathlib.Path(__file__).resolve().parents[1] / "desktop/shell/SetWifi.qml"


class WifiStatus(unittest.TestCase):
    def setUp(self):
        self.s = QML.read_text()

    def test_mesh_rows_keep_the_one_in_use(self):
        self.assertIn("else if (n.inUse && !out[seen[n.ssid]].inUse) out[seen[n.ssid]] = n;", self.s)
        self.assertNotIn("seen[f[1]] = true", self.s)

    def test_active_connection_backs_up_the_scan(self):
        self.assertIn("property string activeSsid", self.s)
        self.assertIn("GENERAL.CONNECTION", self.s)
        self.assertIn("page.activeSsid = l[2]", self.s)
        self.assertIn("nets.find(n => n.inUse) || (activeSsid", self.s)


if __name__ == "__main__":
    unittest.main()
