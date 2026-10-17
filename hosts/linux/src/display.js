'use strict';
// Resolution, refresh rate and scale on Linux. Which tool does the work depends on where Nimbus is running:
//   gamescope  (Game Mode)        modes from the kernel; applied as gamescope options when Game Mode restarts;
//                                 scale is Nimbus's own interface zoom
//   kde        (Plasma desktop)   kscreen-doctor; Plasma remembers the settings itself
//   wlr        (labwc, Sway…)     wlr-randr; saved to ~/.config/nimbus/display.sh, which Desktop Mode runs at start
//   xrandr     (X11 desktops)     xrandr; scale is Nimbus's interface zoom
// Every backend answers get() with the same shape, so the UI doesn't care which one it is.

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { run, has, readText } = require('./util');

const CONF_DIR = path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'nimbus');
const SCALES = [100, 125, 150, 175, 200];
// gamescope can't list refresh rates; these are the usual ones (it falls back to the nearest the screen supports).
const COMMON_RATES = [30, 40, 50, 60, 75, 90, 120, 144, 165];

const round2 = n => Math.round(n * 100) / 100;
const rateLabel = r => (Math.abs(r - Math.round(r)) < 0.05 ? String(Math.round(r)) : r.toFixed(2));

/** Groups a flat list of {w,h,rate} into resolutions, biggest first, each with its refresh rates, fastest first. */
function group(modes) {
  const map = new Map();
  for (const m of modes) {
    const k = `${m.w}x${m.h}`;
    if (!map.has(k)) map.set(k, { w: m.w, h: m.h, rates: [] });
    const g = map.get(k);
    if (!g.rates.some(r => Math.abs(r - m.rate) < 0.01)) g.rates.push(m.rate);
  }
  const out = [...map.values()].sort((a, b) => b.w * b.h - a.w * a.h || b.w - a.w);
  for (const g of out) g.rates.sort((a, b) => b - a);
  return out;
}

function detect(env = process.env) {
  if (env.GAMESCOPE_WAYLAND_DISPLAY) return 'gamescope';
  if (/KDE/i.test(env.XDG_CURRENT_DESKTOP || '') && has('kscreen-doctor')) return 'kde';
  if (env.WAYLAND_DISPLAY && has('wlr-randr') && !/GNOME/i.test(env.XDG_CURRENT_DESKTOP || '')) return 'wlr';
  if (env.DISPLAY && !env.WAYLAND_DISPLAY && has('xrandr')) return 'xrandr';
  return null;
}

// ------------------------------------------------------------------ parsers (exported for tests)

function parseKscreen(json) {
  const outs = (JSON.parse(json).outputs || []).filter(o => o.enabled && o.connected !== false);
  const o = outs.find(x => x.priority === 1) || outs[0];
  if (!o) return null;
  const modes = (o.modes || []).map(m => ({ id: String(m.id), w: m.size.width, h: m.size.height, rate: round2(m.refreshRate) }));
  const cur = modes.find(m => m.id === String(o.currentModeId)) || modes[0];
  return { output: o.name, modes, current: cur ? { w: cur.w, h: cur.h, rate: cur.rate } : null, scale: Math.round((o.scale || 1) * 100) };
}

function parseWlr(json) {
  const outs = JSON.parse(json).filter(o => o.enabled);
  const o = outs[0];
  if (!o) return null;
  const modes = (o.modes || []).map(m => ({ w: m.width, h: m.height, rate: round2(m.refresh), exact: m.refresh }));
  const cur = (o.modes || []).find(m => m.current);
  const pref = (o.modes || []).find(m => m.preferred);
  return { output: o.name, modes, current: cur ? { w: cur.width, h: cur.height, rate: round2(cur.refresh) } : null,
    preferred: pref ? { w: pref.width, h: pref.height } : null, scale: Math.round((o.scale || 1) * 100) };
}

function parseXrandr(text) {
  const outs = [];
  let cur = null;
  for (const line of text.split('\n')) {
    const head = /^(\S+) connected( primary)?/.exec(line);
    if (head) { cur = { output: head[1], primary: !!head[2], modes: [], current: null }; outs.push(cur); continue; }
    if (/^\S/.test(line)) { cur = null; continue; }
    const m = cur && /^\s+(\d+)x(\d+)\S*\s+(.*)$/.exec(line);
    if (!m) continue;
    for (const tok of m[3].trim().split(/\s+/)) {
      const r = parseFloat(tok);
      if (!Number.isFinite(r)) continue;
      const mode = { w: +m[1], h: +m[2], rate: round2(r) };
      cur.modes.push(mode);
      if (tok.includes('*')) cur.current = mode;
      if (tok.includes('+')) cur.preferred = { w: mode.w, h: mode.h };
    }
  }
  const o = outs.find(x => x.primary && x.modes.length) || outs.find(x => x.modes.length);
  return o ? { output: o.output, modes: o.modes, current: o.current, preferred: o.preferred || null, scale: 100 } : null;
}

/** Connected screen's resolutions from the kernel (/sys/class/drm/*\/modes). */
function drmModes(root = '/sys/class/drm') {
  let names = [];
  try { names = fs.readdirSync(root).filter(n => /^card\d+-/.test(n)); } catch {}
  for (const n of names) {
    if ((readText(path.join(root, n, 'status')) || '').trim() !== 'connected') continue;
    const list = (readText(path.join(root, n, 'modes')) || '').split('\n')
      .map(l => /^(\d+)x(\d+)/.exec(l)).filter(Boolean).map(m => ({ w: +m[1], h: +m[2] }));
    if (list.length) return { output: n.replace(/^card\d+-/, ''), list };
  }
  return null;
}

/** Reads and writes NIMBUS_GAMESCOPE_ARGS in ~/.config/nimbus/session.conf, keeping everything else. */
function gamescopeArgs(set) {
  const file = path.join(CONF_DIR, 'session.conf');
  const text = readText(file) || '';
  const m = /^NIMBUS_GAMESCOPE_ARGS="([^"]*)"/m.exec(text);
  const args = m ? m[1] : '';
  if (!set) return args;
  // Drop our own options, keep the rest, then add the new values.
  const kept = args.replace(/(^|\s)-(W|H|r)\s+\S+/g, '').replace(/(^|\s)--(output-width|output-height|nested-refresh)[= ]\S+/g, '').trim();
  const next = [kept, set].filter(Boolean).join(' ');
  const line = `NIMBUS_GAMESCOPE_ARGS="${next}"`;
  fs.mkdirSync(CONF_DIR, { recursive: true });
  fs.writeFileSync(file, m ? text.replace(/^NIMBUS_GAMESCOPE_ARGS="[^"]*"/m, line) : (text && !text.endsWith('\n') ? text + '\n' : text) + line + '\n');
  return next;
}

// ------------------------------------------------------------------ the service

class DisplayModes {
  /**
   * @param {object} o
   * @param {() => {w:number,h:number,rate:number}} o.screen   what Electron sees (fallback for current mode)
   * @param {() => number} o.getZoom    Nimbus interface zoom, percent
   * @param {(pct:number) => void} o.setZoom
   * @param {() => void} o.restartGameMode   ends Game Mode so the session starts it again with new options
   */
  constructor({ screen, getZoom, setZoom, restartGameMode, env = process.env }) {
    Object.assign(this, { screen, getZoom, setZoom, restartGameMode, env });
    this.backend = detect(env);
  }

  async read() {
    switch (this.backend) {
      case 'kde': return parseKscreen(await run('kscreen-doctor', ['-j']));
      case 'wlr': return parseWlr(await run('wlr-randr', ['--json']));
      case 'xrandr': return parseXrandr(await run('xrandr', ['--query']));
      case 'gamescope': {
        const drm = drmModes();
        const s = this.screen();
        const args = gamescopeArgs();
        const rate = +(/-r\s+(\d+)/.exec(args) || [])[1] || Math.round(s.rate) || 60;
        const w = +(/-W\s+(\d+)/.exec(args) || [])[1] || s.w, h = +(/-H\s+(\d+)/.exec(args) || [])[1] || s.h;
        const sizes = drm ? drm.list : [{ w, h }];
        const modes = sizes.flatMap(m => COMMON_RATES.map(r => ({ w: m.w, h: m.h, rate: r })));
        // The kernel lists the screen's preferred (native) mode first.
        return { output: drm ? drm.output : 'screen', modes, current: { w, h, rate }, preferred: sizes[0], scale: this.getZoom() };
      }
      default: return null;
    }
  }

  /**
   * What the UI shows, in the same shape as the Windows host: the current mode, every resolution
   * (native first) with its refresh rates in whole hertz, and scale choices.
   */
  async get() {
    const d = await this.read().catch(() => null);
    if (!d) return {};
    const scr = this.screen();
    const cur = d.current || { w: scr.w, h: scr.h, rate: scr.rate };
    const zoomScale = this.backend === 'gamescope' || this.backend === 'xrandr';
    const groups = group(d.modes);
    // Native (the screen's preferred mode) first, like Windows.
    const pref = d.preferred && groups.findIndex(g => g.w === d.preferred.w && g.h === d.preferred.h);
    if (pref > 0) groups.unshift(groups.splice(pref, 1)[0]);
    const pct = zoomScale ? this.getZoom() : d.scale;
    return {
      resolution: `${cur.w} × ${cur.h}`,
      scale: pct + '%',
      width: cur.w, height: cur.h, refreshRate: Math.round(cur.rate),
      modes: groups.map(g => ({ width: g.w, height: g.h, rates: [...new Set(g.rates.map(Math.round))] })),
      scalePercent: pct, scaleRecommended: 100, scales: SCALES,
      // Linux extras the UI understands:
      backend: this.backend,
      scaleIsInterface: zoomScale,
      appliesOnRestart: this.backend === 'gamescope',
    };
  }

  async setMode(w, h, rate) {   // rate in Hz; the nearest the screen offers is used
    const d = await this.read();
    if (!d) throw new Error('Changing the resolution isn\'t supported on this desktop');
    // Pick the exact mode the tool reported (refresh rates like 59.94 must match).
    const m = d.modes.filter(x => x.w === w && x.h === h).sort((a, b) => Math.abs(a.rate - rate) - Math.abs(b.rate - rate))[0];
    if (!m) throw new Error(`${w} × ${h} isn't available on this screen`);
    switch (this.backend) {
      case 'kde': await run('kscreen-doctor', [`output.${d.output}.mode.${m.id}`]); break;
      case 'wlr':
        await run('wlr-randr', ['--output', d.output, '--mode', `${m.w}x${m.h}@${m.exact}Hz`]);
        this.saveWlr(d.output, { mode: `${m.w}x${m.h}@${m.exact}Hz` });
        break;
      case 'xrandr': await run('xrandr', ['--output', d.output, '--mode', `${m.w}x${m.h}`, '--rate', String(m.rate)]); break;
      case 'gamescope':
        gamescopeArgs(`-W ${m.w} -H ${m.h} -r ${Math.round(m.rate)}`);
        setTimeout(() => this.restartGameMode(), 600); // let the UI show the toast first
        return { ...(await this.get()), restarting: true };
    }
    return this.get();
  }

  async setScale(pct) {
    if (!SCALES.includes(pct)) throw new Error('Unknown scale');
    const d = await this.read();
    if (this.backend === 'kde') await run('kscreen-doctor', [`output.${d.output}.scale.${pct / 100}`]);
    else if (this.backend === 'wlr') {
      await run('wlr-randr', ['--output', d.output, '--scale', String(pct / 100)]);
      this.saveWlr(d.output, { scale: String(pct / 100) });
    } else this.setZoom(pct);
    return this.get();
  }

  /** Desktop Mode (labwc) forgets wlr-randr changes on restart; keep them in a script its autostart runs. */
  saveWlr(output, change) {
    const file = path.join(CONF_DIR, 'display.json');
    let saved = {};
    try { saved = JSON.parse(fs.readFileSync(file, 'utf8')); } catch {}
    saved[output] = { ...(saved[output] || {}), ...change };
    fs.mkdirSync(CONF_DIR, { recursive: true });
    fs.writeFileSync(file, JSON.stringify(saved, null, 2));
    const lines = Object.entries(saved).map(([o, c]) =>
      `wlr-randr --output '${o}'${c.mode ? ` --mode '${c.mode}'` : ''}${c.scale ? ` --scale ${c.scale}` : ''}`);
    fs.writeFileSync(path.join(CONF_DIR, 'display.sh'), `#!/bin/sh\n# Written by Nimbus › Settings › Display\n${lines.join('\n')}\n`, { mode: 0o755 });
  }
}

module.exports = { DisplayModes, detect, group, parseKscreen, parseWlr, parseXrandr, gamescopeArgs, drmModes, SCALES };
