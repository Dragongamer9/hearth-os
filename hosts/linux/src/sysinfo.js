'use strict';
// What Settings › About and › System show: OS, device, processor, graphics, memory, storage, uptime.
// Read from /etc/os-release, /sys and /proc; anything missing is left out. Same shape as the Windows host.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const read = (f, root = '') => { try { return fs.readFileSync(path.join(root, f), 'utf8').trim(); } catch { return ''; } };

function osRelease(root = '') {
  const out = {};
  for (const line of read('/etc/os-release', root).split('\n')) {
    const m = line.match(/^([A-Z_]+)=(.*)$/);
    if (m) out[m[1]] = m[2].replace(/^["']|["']$/g, '');
  }
  return out;
}

function cpuName(root = '') {
  const m = read('/proc/cpuinfo', root).match(/^model name\s*:\s*(.+)$/m);
  return m ? m[1].replace(/\s+/g, ' ').trim() : (os.cpus()[0] || {}).model || '';
}

const GPU_VENDORS = { '0x1002': 'AMD', '0x10de': 'NVIDIA', '0x8086': 'Intel' };
/** Graphics: the driver's product name when the kernel offers one, else the vendor. */
function gpus(root = '') {
  const out = [];
  let cards = [];
  try { cards = fs.readdirSync(path.join(root, '/sys/class/drm')).filter(n => /^card\d+$/.test(n)); } catch {}
  for (const c of cards) {
    const dev = `/sys/class/drm/${c}/device`;
    const vendor = read(`${dev}/vendor`, root);
    if (!vendor) continue;
    const name = read(`${dev}/product_name`, root) || GPU_VENDORS[vendor] || vendor;
    const vram = parseInt(read(`${dev}/mem_info_vram_total`, root), 10);
    const label = name + (vram > 0 ? ` (${Math.round(vram / 2 ** 30)} GB)` : '');
    if (!out.includes(label)) out.push(label);
  }
  return out;
}

function storage(dir = os.homedir()) {
  try {
    const s = fs.statfsSync(dir);
    return { path: dir, totalBytes: s.blocks * s.bsize, freeBytes: s.bavail * s.bsize };
  } catch { return null; }
}

function info({ root = '' } = {}) {
  const rel = osRelease(root);
  const vendor = read('/sys/class/dmi/id/sys_vendor', root);
  const product = read('/sys/class/dmi/id/product_name', root);
  return {
    os: rel.NAME || 'Linux',
    osVersion: rel.VERSION || rel.VERSION_ID || '',
    osPretty: rel.PRETTY_NAME || '',
    device: [vendor, product].filter(Boolean).join(' ').replace(/^(ASUSTeK COMPUTER INC\.)/, 'ASUS'),
    cpu: cpuName(root),
    cores: os.cpus().length,
    gpu: gpus(root).join(', '),
    memoryBytes: os.totalmem(),
    storage: storage(),
    uptimeSec: Math.round(os.uptime()),
    kernel: os.release(),
    hostname: os.hostname(),
  };
}

module.exports = { info, osRelease, cpuName, gpus };
