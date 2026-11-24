"""The Debian edition (Peppermint) is archived (the owner, Nov 19, 2026: "Only Fedora, Archive Debian"; docs/ARCHIVED.md):
nothing new is published for it. base-peppermint.yml may only run by hand (Actions › Run workflow), with publish off
unless someone ticks it, so a push to main or any branch never puts a new image on hearth-peppermint:testing, where
Debian devices look for updates. It never touches hearth-os (Fedora's :stable, :latest or test tags) either.
os-bases/peppermint stays in place: the Fedora image copies its system_files (os-bases/anatase/Containerfile)."""
import os
import re
import unittest

try:
    import yaml
except ImportError:          # the runner's Python without PyYAML: these checks need it
    yaml = None

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..')
WF = os.path.join(ROOT, '.github', 'workflows', 'base-peppermint.yml')


def load():
    text = open(WF, encoding='utf-8').read()
    data = yaml.safe_load(text)
    # PyYAML reads the bare key `on` as True.
    return text, data, data.get('on', data.get(True))


@unittest.skipIf(yaml is None, 'PyYAML is needed to read the workflow')
class DebianImageArchived(unittest.TestCase):
    def test_only_by_hand(self):
        _, _, on = load()
        self.assertIsInstance(on, dict, 'base-peppermint.yml: "on:" should list its triggers')
        self.assertEqual(set(on), {'workflow_dispatch'},
                         'base-peppermint.yml runs only by hand (no push, pull_request, schedule or workflow_run): '
                         'the Debian edition is archived and a push must never publish hearth-peppermint:testing')

    def test_publish_is_off_unless_ticked(self):
        _, _, on = load()
        pub = ((on['workflow_dispatch'] or {}).get('inputs') or {}).get('publish')
        self.assertIsNotNone(pub, 'Run workflow keeps its publish switch (off: build and check only)')
        self.assertEqual(pub.get('type'), 'boolean')
        self.assertIs(pub.get('default'), False, 'publish must be off by default')

    def test_publish_step_needs_the_switch(self):
        text, data, _ = load()
        envs = [j.get('env', {}).get('PUBLISH') for j in data['jobs'].values() if 'PUBLISH' in (j.get('env') or {})]
        self.assertTrue(envs, 'the image job sets PUBLISH')
        for e in envs:
            self.assertIn("github.event_name == 'workflow_dispatch'", e)
            self.assertIn('inputs.publish', e)
        pushes = [(name, s) for name, j in data['jobs'].items() for s in j.get('steps', [])
                  if re.search(r'podman push|skopeo copy[^\n]*docker://', s.get('run', '') or '')]
        self.assertTrue(pushes, 'expected the (hand-run) publish step')
        # 1.18 (Debian again): the image goes up only when publish was ticked, or to an owner-approved test tag
        # (test-<version>, and its -candidate while the checks run: the owner, Nov 24).
        for name, s in pushes:
            cond = str(s.get('if', '')).strip()
            self.assertTrue(cond == "env.PUBLISH == 'true'" or "inputs.test_tag != ''" in cond,
                            '%s: a step that uploads the image must only run when publish was ticked or a test tag given' % name)

    def test_never_touches_fedora_or_latest(self):
        text, data, _ = load()
        self.assertEqual(data['env']['IMAGE'], 'ghcr.io/dragongamer9/hearth-peppermint')
        self.assertNotIn('ghcr.io/dragongamer9/hearth-os', text, 'the Debian workflow never publishes Fedora\'s image')
        self.assertNotRegex(text, r'\$IMAGE:latest|IMAGE}:latest', 'never :latest')


class DebianImageArchivedText(unittest.TestCase):
    def test_no_push_trigger_text(self):
        text = open(WF, encoding='utf-8').read()
        on = re.search(r'^on:\n(.*?)^\S', text, re.S | re.M).group(1)
        for t in ('push:', 'pull_request', 'schedule:', 'workflow_run:'):
            self.assertNotRegex(on, r'(?m)^  ' + t, 'base-peppermint.yml: no %s trigger (archived; hand-run only)' % t)

    def test_debian_folders_stay_for_fedora(self):
        self.assertTrue(os.path.isdir(os.path.join(ROOT, 'os-bases', 'peppermint', 'system_files')))
        cf = open(os.path.join(ROOT, 'os-bases', 'anatase', 'Containerfile'), encoding='utf-8').read()
        self.assertIn('os-bases/peppermint/system_files', cf)


if __name__ == '__main__':
    unittest.main()
