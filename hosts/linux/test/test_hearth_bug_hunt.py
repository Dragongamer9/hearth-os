"""Bug hunt after 1.7 (device logs #77-#81 and the owner's reports): each fix checked where it can be without building.
The Rust parts also have their own tests (hearth-gaming launch.rs / vfs.rs / wabbajack.rs, run by gaming.yml)."""
import sys
sys.dont_write_bytecode = True
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
GAMING = ROOT / 'hosts' / 'hearth-gaming' / 'src'
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'


def read(*parts):
    return (ROOT.joinpath(*parts)).read_text()


class Mods(unittest.TestCase):
    """Log #77 (Nvidia desktop): Play through the organizer started SKSE, then never took the mods away ("The game is
    running through the organizer: close it first." at every later Play, from Game Mode too)."""

    def test_waiting_for_the_game_skips_the_overlays_own_process(self):
        launch = (GAMING / 'launch.rs').read_text()
        body = launch.split('pub fn processes_in', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('not_the_game(&comm, &cmdline)', body)
        helper = launch.split('pub fn not_the_game', 1)[1].split('\n}\n', 1)[0]
        for name in ('fuse-overlayfs', 'fusermount3', 'hearth-mods'):
            self.assertIn(name, helper)

    def test_a_deploy_from_before_a_restart_is_not_running(self):
        vfs = (GAMING / 'organizer' / 'vfs.rs').read_text()
        self.assertIn('pub boot: String', vfs)
        self.assertIn('boot: boot_id()', vfs)
        org = (GAMING / 'organizer' / 'mod.rs').read_text()
        self.assertNotIn('vfs::alive(a.pid)', org + vfs.replace('alive(a.pid)\n}', ''))

    def test_wabbajack_prefix_uses_hearths_umu_and_the_same_proton_as_play(self):
        wj = (GAMING / 'wabbajack.rs').read_text()
        setup = wj.split('fn set_up(', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('crate::launch::umu()', setup)
        self.assertNotIn('has("umu-run")', setup)
        self.assertIn('crate::launch::umu_env(', setup)
        self.assertNotIn('PROTONPATH=GE-Proton umu-run', wj)

    def test_proton_choice_uses_the_detector_and_keeps_a_prefixs_proton(self):
        launch = (GAMING / 'launch.rs').read_text()
        self.assertIn('hearth_core::proton_pick::auto_for("steam"', launch)
        self.assertIn('hearth_core::proton_pick::named(', launch)
        self.assertIn('PROTON_NOTE', launch)
        self.assertIn('pub fn named(', read('hosts', 'hearth-core', 'src', 'proton_pick.rs'))

    def test_ci_runs_the_overlay_test(self):
        wf = read('.github', 'workflows', 'gaming.yml')
        self.assertIn('fuse-overlayfs', wf)
        tests = (GAMING / 'organizer' / 'tests.rs').read_text()
        self.assertIn('crate::launch::processes_in(&fx.game).is_empty()', tests)


class Logs(unittest.TestCase):
    def test_send_logs_needs_no_root_and_no_rpm_ostree(self):
        # Log #81: "This command must be executed as the root user" and "rpm-ostree: not found" in every report.
        logs = read('hosts', 'hearth-core', 'src', 'logs.rs')
        cmd = logs.split('pub const SYSTEM_VERSION_SH', 1)[1].split('";', 1)[0]
        self.assertIn('id -u', cmd.split('bootc status', 1)[0], 'bootc status only as root')
        self.assertIn('command -v rpm-ostree', cmd.split('rpm-ostree status', 1)[0])
        self.assertIn('ostree admin status', cmd)
        self.assertNotIn('|| rpm-ostree status', logs)
        self.assertIn('BOOT_FALLBACK_SH', logs)


class HandheldDaemon(unittest.TestCase):
    def test_no_asking_hhd_where_it_isnt(self):
        # Log #81 (laptop): "Handheld Daemon's overlay app is away and HHD couldn't be asked to start it" repeatedly.
        gm = read('hosts', 'hearth-rs', 'src', 'gamemode.rs')
        keeper = gm.split('pub fn start_overlay_keeper', 1)[1].split('\n    }\n', 1)[0]
        first = keeper.index('hhd::socket_present()')
        self.assertLess(first, keeper.index('start_overlay_app()'))
        self.assertIn('hhd_missing', gm)
        self.assertIn('pub fn socket_present()', read('hosts', 'hearth-core', 'src', 'hhd.rs'))


class KeyboardFiles(unittest.TestCase):
    def test_missing_files_are_quiet_but_real_errors_are_not(self):
        # Log #81: "Read of ~/.config/nimbus/osk.json failed: File does not exist." at every start.
        for name, count in (('Keyboard.qml', 3), ('OskThemes.qml', 1)):
            q = (SHELL / name).read_text()
            self.assertEqual(q.count('printErrors: false'), count, name)
            self.assertEqual(q.count('error !== FileViewError.FileNotFound'), count, name)
            self.assertIn('/nimbus/osk.json', q)
        # The bundled word lists must be there: their read errors still print.
        words = (SHELL / 'Keyboard.qml').read_text().split('id: wordsFile', 1)[1].split('FileView {', 1)[0]
        self.assertNotIn('printErrors: false', words)


class AiService(unittest.TestCase):
    def test_the_ai_service_does_not_restart_forever(self):
        unit = read('os', 'system_files', 'usr', 'lib', 'systemd', 'system', 'hearth-ollama.service')
        unit_part = unit.split('[Service]', 1)[0]
        self.assertIn('StartLimitIntervalSec=', unit_part)
        self.assertIn('StartLimitBurst=', unit_part)
        self.assertIn('Restart=on-failure', unit)
        helper = read('os', 'system_files', 'usr', 'libexec', 'nimbus', 'hearth-home-server-helper')
        svc = helper.split('def svc(action, name):', 1)[1].split('\ndef ', 1)[0]
        self.assertLess(svc.index("'reset-failed'"), svc.index("['systemctl', action"))


class HiddenIcons(unittest.TestCase):
    def test_a_hidden_icon_loads_nothing(self):
        # Logs #77-#81: hundreds of "Icon.qml: Cannot open .../icons/<app id>-light.svg" from Icons kept hidden
        # next to an app's own picture.
        icon = (SHELL / 'Icon.qml').read_text()
        self.assertIn('source: !root.visible ? ""', icon)


class Repair(unittest.TestCase):
    def test_repair_is_not_a_call_that_waits(self):
        wj = (GAMING / 'wabbajack.rs').read_text()
        body = wj.split('pub fn repair(', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('std::thread::spawn', body)
        self.assertIn('j.repair == "running"', body)
        mods = (GAMING / 'apps' / 'mods.rs').read_text()
        self.assertIn('j["repair"] == "running"', mods.split('fn busy()', 1)[1].split('\n}', 1)[0], 'the worker stays while it runs')
        js = (ROOT / 'hosts' / 'hearth-gaming' / 'ui' / 'mods.js').read_text()
        self.assertIn("j.repair === 'running'", js)


if __name__ == '__main__':
    unittest.main()
