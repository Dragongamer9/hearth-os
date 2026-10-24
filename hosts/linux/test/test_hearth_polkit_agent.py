"""Desktop Mode (labwc/autostart) and Game Mode (session/nimbus-watchdog) start the same polkit agent, so pkexec's
password prompts (Hearth Text's Edit as administrator) work in both; Peppermint's image installs one Debian 13 has."""
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


def agents(text):
    m = re.search(r'for agent in (.*?); do', text, re.S)
    return [a for a in re.split(r'[\s\\]+', m.group(1)) if a] if m else []


class Agent(unittest.TestCase):
    def test_same_list_in_both_modes(self):
        desk = agents((ROOT / 'hosts/linux/desktop/labwc/autostart').read_text())
        game = agents((ROOT / 'hosts/linux/session/nimbus-watchdog').read_text())
        self.assertTrue(desk)
        self.assertEqual(desk, game)
        self.assertIn('/usr/libexec/polkit-mate-authentication-agent-1', desk)

    def test_game_mode_agent_on_gamescopes_display_and_stopped(self):
        wd = (ROOT / 'hosts/linux/session/nimbus-watchdog').read_text()
        self.assertIn('GDK_BACKEND=x11', wd)
        self.assertIn("trap 'kill $AGENT_PID 2>/dev/null' EXIT", wd)
        self.assertIn('[ -z "$HEARTH_SEAT" ]', wd)

    def test_peppermint_installs_a_trixie_agent(self):
        build = (ROOT / 'os-bases/peppermint/build.sh').read_text()
        self.assertIn('optional mate-polkit', build)


if __name__ == '__main__':
    unittest.main()
