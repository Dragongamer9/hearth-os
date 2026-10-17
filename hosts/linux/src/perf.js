'use strict';
// Live performance numbers for Quick Access › Performance, from the kernel:
//   CPU   /proc/stat (busy share since the last sample)
//   RAM   /proc/meminfo
//   GPU   /sys/class/drm/card*/device/gpu_busy_percent and mem_info_vram_used (AMD, incl. ROG Ally and Steam Deck;
//         other GPUs report null and the UI shows a dash)
// Same fields as the Windows host's perf.stats.

const fs = require('node:fs');
const path = require('node:path');

let last = null; // previous /proc/stat totals

function cpuTimes(text) {
  const f = text.split('\n')[0].trim().split(/\s+/).slice(1).map(Number);
  const idle = (f[3] || 0) + (f[4] || 0);           // idle + iowait
  const total = f.reduce((a, b) => a + (b || 0), 0);
  return { idle, total };
}

function cpuPercent(statText) {
  const now = cpuTimes(statText);
  const prev = last;
  last = now;
  if (!prev || now.total <= prev.total) return 0;
  return Math.round(100 * (1 - (now.idle - prev.idle) / (now.total - prev.total)));
}

function memory(text) {
  const kb = k => { const m = new RegExp(`^${k}:\\s+(\\d+)`, 'm').exec(text); return m ? +m[1] : 0; };
  const total = kb('MemTotal'), avail = kb('MemAvailable');
  const gb = n => Math.round((n / 1024 / 1024) * 10) / 10;
  return { ramUsed: gb(total - avail), ramTotal: gb(total) };
}

function gpu(root = '/sys/class/drm') {
  let names = [];
  try { names = fs.readdirSync(root).filter(n => /^card\d+$/.test(n)); } catch {}
  for (const n of names) {
    const dev = path.join(root, n, 'device');
    try {
      const busy = +fs.readFileSync(path.join(dev, 'gpu_busy_percent'), 'utf8');
      let vram = null;
      try { vram = Math.round((+fs.readFileSync(path.join(dev, 'mem_info_vram_used'), 'utf8') / 1024 ** 3) * 10) / 10; } catch {}
      return { gpu: Number.isFinite(busy) ? busy : null, vramUsed: vram };
    } catch { /* not an AMD GPU, or no access */ }
  }
  return { gpu: null, vramUsed: null };
}

async function sample() {
  const [stat, mem] = await Promise.all([fs.promises.readFile('/proc/stat', 'utf8'), fs.promises.readFile('/proc/meminfo', 'utf8')]);
  return { cpu: cpuPercent(stat), ...memory(mem), ...gpu() };
}

module.exports = { sample, cpuPercent, memory, gpu };
