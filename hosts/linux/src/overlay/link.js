'use strict';
// The overlay app's line to the running Hearth app: hearth-core's overlay socket (hosts/hearth-core/src/overlay_link.rs),
// one JSON message per line each way. Up: the page's messages (the same ones it posts in Hearth's own windows) and
// HHD's open requests. Down: replies and events for the page, as Hearth sends its overlay window. The socket is in
// the user's runtime folder (only this user can reach it; Hearth also checks who connected), never on the network.
// Hearth may start after the overlay, or restart: tried again every second.

const net = require('node:net');
const fs = require('node:fs');
const path = require('node:path');
const { EventEmitter } = require('node:events');

const MAX_LINE = 16 * 1024 * 1024;

/** Where Hearth listens. HHD starts the overlay without XDG_RUNTIME_DIR, so /run/user/<uid> comes first (Hearth
 *  looks there first too). */
function socketPath(env = process.env, uid = process.getuid(), exists = fs.existsSync) {
  const run = `/run/user/${uid}`;
  const dir = exists(run) ? run : (env.XDG_RUNTIME_DIR || run);
  return path.join(dir, 'hearth', 'overlay.sock');
}

/** Splits a stream into lines: returns [complete lines, the unfinished rest]. */
function splitLines(buf) {
  const parts = buf.split('\n');
  const rest = parts.pop();
  return [parts, rest];
}

class HearthLink extends EventEmitter {
  constructor(file, { retryMs = 1000, connect = p => net.createConnection(p) } = {}) {
    super();
    this.file = file;
    this.retryMs = retryMs;
    this.connectFn = connect;
    this.sock = null;
    this.connected = false;
    this.stopped = false;
    this.timer = null;
  }
  start() { this.stopped = false; this.attempt(); return this; }
  stop() { this.stopped = true; if (this.timer) clearTimeout(this.timer); if (this.sock) this.sock.destroy(); }

  attempt() {
    if (this.stopped) return;
    let rest = '';
    const s = this.connectFn(this.file);
    this.sock = s;
    s.setEncoding('utf8');
    s.on('connect', () => { this.connected = true; this.emit('connect'); });
    s.on('data', chunk => {
      rest += chunk;
      if (rest.length > MAX_LINE) { s.destroy(); return; }
      const [lines, more] = splitLines(rest);
      rest = more;
      for (const l of lines) {
        if (!l.trim()) continue;
        let msg; try { msg = JSON.parse(l); } catch { continue; }
        this.emit('message', msg);
      }
    });
    s.on('error', () => {});
    s.on('close', () => {
      const was = this.connected;
      this.connected = false;
      if (this.sock === s) this.sock = null;
      if (was) this.emit('disconnect');
      if (!this.stopped) this.timer = setTimeout(() => this.attempt(), this.retryMs);
    });
  }

  /** Sends one message; false when Hearth isn't there. */
  send(msg) {
    if (!this.connected || !this.sock) return false;
    try { this.sock.write(JSON.stringify(msg) + '\n'); return true; } catch { return false; }
  }
}

module.exports = { HearthLink, socketPath, splitLines };
