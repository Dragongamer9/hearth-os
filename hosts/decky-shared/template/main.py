"""Hearth NAME's backend (a Hearth Decky plugin; docs/PLUGIN-UPDATES.md, "Adding a new plugin").

Runs as you inside Decky Loader. Keep the Hearth OS check and the load mark exactly as below: off Hearth OS nothing
starts, and on Hearth OS the mark (hearth-loaded.json after 30 s) tells plugin updates this version loaded; without it
a downloaded copy is taken away again.
"""
import asyncio
import os
import sys

try:
    import decky
except ImportError:  # tests
    decky = None

# Hearth OS only, and the load mark for plugin updates (hosts/decky-shared/hearth_only.py): next to main.py when packed,
# in hosts/decky-shared in the repo.
_here = os.path.dirname(os.path.abspath(__file__))
for _p in (os.path.join(_here, '..', 'decky-shared'), _here):
    if _p not in sys.path:
        sys.path.insert(0, _p)
import hearth_only  # noqa: E402

NAME = 'Hearth NAME'


def log(msg):
    if decky:
        decky.logger.info(msg)


class Plugin:
    async def hearth_os(self):
        """Whether this is Hearth OS (the frontend shows the Hearth OS panel instead of the plugin when not)."""
        return hearth_only.status(log, NAME)

    async def _main(self):
        if not hearth_only.status(log, NAME)['ok']:
            return   # not Hearth OS: nothing starts (the panel says why)
        log('%s ready' % NAME)
        self._loaded = asyncio.get_running_loop().create_task(hearth_only.mark_loaded_later(decky))

    async def _unload(self):
        pass
