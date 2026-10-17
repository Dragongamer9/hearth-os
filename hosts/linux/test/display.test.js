'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'nimbus-display-'));
process.env.XDG_CONFIG_HOME = tmp;
process.env.XDG_DATA_HOME = path.join(tmp, 'data');
const d = require('../src/display');

test('kscreen-doctor JSON', () => {
  const r = d.parseKscreen(JSON.stringify({ outputs: [
    { name: 'HDMI-A-1', enabled: true, connected: true, priority: 2, currentModeId: '9', modes: [], scale: 1 },
    { name: 'eDP-1', enabled: true, connected: true, priority: 1, currentModeId: '2', scale: 1.25, modes: [
      { id: '1', size: { width: 1920, height: 1080 }, refreshRate: 144.0 },
      { id: '2', size: { width: 1920, height: 1080 }, refreshRate: 60.0 },
      { id: '3', size: { width: 1280, height: 720 }, refreshRate: 59.94 }] }] }));
  assert.equal(r.output, 'eDP-1');
  assert.deepEqual(r.current, { w: 1920, h: 1080, rate: 60 });
  assert.equal(r.scale, 125);
  assert.deepEqual(d.group(r.modes).map(g => [g.w, g.rates]), [[1920, [144, 60]], [1280, [59.94]]]);
});

test('wlr-randr JSON', () => {
  const r = d.parseWlr(JSON.stringify([{ name: 'DP-1', enabled: true, scale: 1.5, modes: [
    { width: 2560, height: 1440, refresh: 164.958, preferred: true, current: true },
    { width: 2560, height: 1440, refresh: 59.951, current: false }] }]));
  assert.deepEqual(r.current, { w: 2560, h: 1440, rate: 164.96 });
  assert.equal(r.modes[1].exact, 59.951);
  assert.equal(r.scale, 150);
});

test('xrandr text, primary output', () => {
  const r = d.parseXrandr(`Screen 0: minimum 8 x 8, current 1920 x 1080
HDMI-1 connected 1280x720+1920+0 (normal) 0mm x 0mm
   1280x720      60.00*+
DP-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 527mm x 296mm
   1920x1080     60.00 +  144.00*  120.00
   1280x720      60.00
VGA-1 disconnected (normal left inverted right x axis y axis)
`);
  assert.equal(r.output, 'DP-1');
  assert.deepEqual(r.current, { w: 1920, h: 1080, rate: 144 });
  assert.equal(r.modes.length, 4);
});

test('gamescope options are rewritten in session.conf, other settings kept', () => {
  const conf = path.join(tmp, 'nimbus', 'session.conf');
  fs.mkdirSync(path.dirname(conf), { recursive: true });
  fs.writeFileSync(conf, 'NIMBUS_START_MODE=desktop\nNIMBUS_GAMESCOPE_ARGS="--adaptive-sync -W 1280 -H 800 -r 60"\n');
  d.gamescopeArgs('-W 1920 -H 1080 -r 120');
  assert.equal(fs.readFileSync(conf, 'utf8'), 'NIMBUS_START_MODE=desktop\nNIMBUS_GAMESCOPE_ARGS="--adaptive-sync -W 1920 -H 1080 -r 120"\n');
  assert.equal(d.gamescopeArgs(), '--adaptive-sync -W 1920 -H 1080 -r 120');
});

test('picks the backend from the environment', () => {
  assert.equal(d.detect({ GAMESCOPE_WAYLAND_DISPLAY: 'gamescope-0', WAYLAND_DISPLAY: 'x' }), 'gamescope');
  assert.equal(d.detect({}), null);
});

test('Desktop Mode keeps wlr-randr changes in display.sh', () => {
  const m = new d.DisplayModes({ screen: () => ({ w: 0, h: 0, rate: 60 }), getZoom: () => 100, setZoom() {}, restartGameMode() {}, env: {} });
  m.saveWlr('DP-1', { mode: '2560x1440@164.958Hz' });
  m.saveWlr('DP-1', { scale: '1.25' });
  const sh = fs.readFileSync(path.join(tmp, 'nimbus', 'display.sh'), 'utf8');
  assert.match(sh, /wlr-randr --output 'DP-1' --mode '2560x1440@164.958Hz' --scale 1.25/);
});
