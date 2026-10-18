'use strict';
// The on-screen performance overlay in Game Mode (Quick Access › Performance › Overlay), like SteamOS's: the
// session starts gamescope with --mangoapp and points MANGOHUD_CONFIGFILE at a file in the runtime folder;
// mangoapp watches that file, so writing a new layout to it changes what's shown over the game at once.

const fs = require('node:fs');

const LEVELS = ['Off', 'FPS', 'Simple', 'Detailed', 'Everything'];
const PRESETS = [
  'no_display',
  'legacy_layout=false\nfps_only\nfont_size=20\nbackground_alpha=0.4\nposition=top-left\nhud_no_margin',
  'legacy_layout=false\nhorizontal\nhud_compact\nfps\nframetime=0\ncpu_stats\ngpu_stats\nbattery\nfont_size=20\nbackground_alpha=0.4\nposition=top-center\ntable_columns=14',
  'legacy_layout=false\nfps\nframe_timing=1\ncpu_stats\ncpu_temp\ngpu_stats\ngpu_temp\nram\nvram\nbattery\nbattery_time\nfont_size=20\nbackground_alpha=0.5\nposition=top-left',
  'legacy_layout=false\nfps\nframe_timing=1\nframetime\ncpu_stats\ncpu_temp\ncpu_power\ncpu_mhz\ncore_load\ngpu_stats\ngpu_temp\ngpu_power\ngpu_core_clock\ngpu_mem_clock\nram\nvram\nswap\nbattery\nbattery_watt\nbattery_time\nfan\nresolution\nengine_version\nwine\ngamemode\nfont_size=18\nbackground_alpha=0.5\nposition=top-left',
];

/** Where mangoapp reads its layout from (set by the session), or null outside Game Mode / without mangoapp. */
function configFile(env = process.env) { return env.MANGOHUD_CONFIGFILE || null; }
function supported(env = process.env) { const f = configFile(env); return !!f && fs.existsSync(f); }

function text(level) { const n = Math.max(0, Math.min(LEVELS.length - 1, Math.round(+level || 0))); return PRESETS[n] + '\n'; }

/** Shows the overlay at this level (0 = off). */
function set(level, env = process.env) {
  const f = configFile(env);
  if (!f) throw new Error('The performance overlay works in Game Mode.');
  const t = f + '.tmp';
  fs.writeFileSync(t, text(level));
  fs.renameSync(t, f);
  return Math.max(0, Math.min(LEVELS.length - 1, Math.round(+level || 0)));
}

module.exports = { LEVELS, PRESETS, text, set, supported, configFile };
