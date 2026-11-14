"""Low battery (1.11): warnings at 10% and 5%, then at 3% hibernate only where a hibernation came back before on this
PC, otherwise UPower's clean shutdown at 2%. Checks the shell's watch, the resume hook's proof and the image's UPower
levels."""
import os
import re
import subprocess
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
QML = open(os.path.join(ROOT, 'hosts/linux/desktop/shell/BatteryWatch.qml')).read()


class Watch(unittest.TestCase):
    def test_levels(self):
        # your levels (Task Manager › Battery, 1.12): 10% and 5% unless you chose others; acting at the last one
        self.assertIn('String(c["battery.warn"] || "10,5")', QML)
        self.assertIn('lv !== last && low <= lv && warned > lv', QML)
        self.assertIn('if (low <= last && !acting)', QML)

    def test_hibernates_only_where_proven(self):
        self.assertIn('if (at5 === "hibernate" && proven && canHib)', QML)
        self.assertIn('/var/lib/hearth/hibernate-proven', QML)
        # and only if it's still on battery after the warning
        self.assertRegex(QML, r'sleep 15; \[ .*Discharging \] && systemctl hibernate')

    def test_loaded_by_the_shell(self):
        self.assertIn('Loader { source: "BatteryWatch.qml" }', open(os.path.join(ROOT, 'hosts/linux/desktop/shell/shell.qml')).read())


class Proof(unittest.TestCase):
    def test_resume_hook_writes_the_proof_only_after_hibernation(self):
        hook = open(os.path.join(ROOT, 'os/system_files/usr/lib/systemd/system-sleep/hearth-resumed')).read()
        part = hook[hook.index('hibernate|hybrid-sleep|suspend-then-hibernate)'):]
        self.assertIn('/var/lib/hearth/hibernate-proven', part.split(';;')[0])
        self.assertNotIn('hibernate-proven', hook[:hook.index('hibernate|hybrid-sleep|suspend-then-hibernate)')])
        self.assertEqual(subprocess.run(['sh', '-n', os.path.join(ROOT, 'os/system_files/usr/lib/systemd/system-sleep/hearth-resumed')]).returncode, 0)


class UPower(unittest.TestCase):
    def test_the_image_sets_the_levels(self):
        b = open(os.path.join(ROOT, 'os-bases/peppermint/build.sh')).read()
        i = b.index('# Low battery (1.11)')
        block = b[i:b.index('\nfi\n', i)]
        script = re.search(r"sed -i -E '([^']+)' /etc/UPower/UPower.conf", block).group(1)
        conf = '[UPower]\n#UsePercentageForPolicy=true\nPercentageLow=20.0\nPercentageCritical=5.0\nPercentageAction=2.0\nCriticalPowerAction=HybridSleep\n'
        with tempfile.NamedTemporaryFile('w', delete=False) as f:
            f.write(conf)
        subprocess.run(['sed', '-i', '-E', script, f.name], check=True)
        out = open(f.name).read()
        os.unlink(f.name)
        for line in ('UsePercentageForPolicy=true', 'PercentageLow=10.0', 'PercentageCritical=5.0', 'PercentageAction=2.0', 'CriticalPowerAction=PowerOff'):
            self.assertIn(line + '\n', out)


if __name__ == '__main__':
    unittest.main()
