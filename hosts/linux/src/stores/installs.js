'use strict';
// Epic and GOG installs that Nimbus runs itself (through legendary and gogdl), one at a time like a store's
// download queue. Port of the Windows host's Stores/InstallQueue.cs, including its progress-line parsing.

const { EventEmitter } = require('node:events');
const { log } = require('../util');

const PERCENT = /= Progress: ([\d.]+)%?(?: \((\d+)\/(\d+)\)| (\d+)\/(\d+))?/;
const SPEED = /\+ Download\s+- ([\d.]+) MiB\/s/;
const DL_SIZE = /Download size: ([\d.]+) MiB/;

class InstallJob {
  constructor({ gameId, title, work }) {
    Object.assign(this, { gameId, title, work });
    this.state = 'queued';            // queued | downloading | installing
    this.progress = null; this.bytesDone = null; this.bytesTotal = null; this.speed = 0;
    this.abort = new AbortController();
  }

  /** One output line. bytesInLine: the x/y numbers are bytes (gogdl), not chunks (legendary). */
  readLine(line, bytesInLine) {
    let m;
    if ((m = PERCENT.exec(line))) {
      this.state = 'downloading';
      this.progress = Math.min(1, Math.max(0, parseFloat(m[1]) / 100));
      if (bytesInLine && m[4] != null) { this.bytesDone = +m[4]; this.bytesTotal = +m[5]; }
      else if (this.bytesTotal) this.bytesDone = Math.round(this.bytesTotal * this.progress);
    } else if ((m = SPEED.exec(line))) this.speed = Math.round(parseFloat(m[1]) * 1024 * 1024);
    else if (!bytesInLine && (m = DL_SIZE.exec(line))) this.bytesTotal = Math.round(parseFloat(m[1]) * 1024 * 1024);
  }
}

class InstallQueue extends EventEmitter {
  constructor() { super(); this.jobs = []; this.running = false; }

  get(id) { return this.jobs.find(j => j.gameId === id) || null; }
  active() { return [...this.jobs]; }

  enqueue(opts) {
    if (this.get(opts.gameId)) throw new Error(`${opts.title} is already in the download queue.`);
    const job = new InstallJob(opts);
    this.jobs.push(job);
    if (!this.running) this.pump();
    return job;
  }

  cancel(id) {
    const job = this.get(id);
    if (!job) return false;
    if (job.state === 'queued') {
      this.jobs = this.jobs.filter(j => j !== job);
      this.emit('finished', id, 'cancelled');
    }
    job.abort.abort();
    return true;
  }

  async pump() {
    this.running = true;
    for (;;) {
      const job = this.jobs[0];
      if (!job) { this.running = false; return; }
      let error = null;
      try {
        job.state = 'downloading';
        await job.work(job, job.abort.signal);
      } catch (e) {
        error = e.cancelled || job.abort.signal.aborted ? 'cancelled' : e.message;
        if (error !== 'cancelled') log(`install ${job.gameId} FAILED ${e.message}`);
      }
      this.jobs = this.jobs.filter(j => j !== job);
      log(`install ${job.gameId} finished: ${error || 'ok'}`);
      this.emit('finished', job.gameId, error);
    }
  }
}

module.exports = { InstallQueue, InstallJob };
