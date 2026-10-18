'use strict';
/**
 * The one place that decides what's in front in Game Mode (Hearth, the game, an app, Steam, the overlay).
 *
 * Before, several bits of code moved focus on their own timers: the overlay handing the screen back after
 * closing, a game search retrying for 30 seconds, a delayed stream focus. When two of them overlapped, the
 * older one won a moment later and the screen jumped back (the overlay "stuck", the old app came back).
 *
 * Now every change goes through run(): changes happen one at a time, in order, and the newest request wins.
 * A request that's waiting (or a game search that's still retrying) is dropped as soon as something newer is
 * asked for. "Soft" requests, like the hand-back after closing the overlay, never cancel anything: they only
 * happen if nothing else was asked for meanwhile.
 */
class FocusOwner {
  constructor({ log = () => {} } = {}) {
    this.gen = 0;
    this.queue = Promise.resolve();
    this.log = log;
  }

  /** A new choice was made: anything asked for before it is dropped. Returns the choice's number. */
  claim() { return ++this.gen; }

  /**
   * Runs fn(still) after the changes already queued, unless something newer was asked for in the meantime.
   * still() says whether this is still the newest request (long-running changes check it to stop early).
   * Resolves true if it ran, false if it was dropped (or failed).
   */
  run(label, fn, { delay = 0, soft = false } = {}) {
    const n = soft ? this.gen : this.claim();
    const still = () => this.gen === n;
    const go = async () => {
      if (!still()) { this.log(`focus: ${label} dropped (something newer)`); return false; }
      try { await fn(still); return true; }
      catch (e) { this.log(`focus: ${label} failed: ${e.message}`); return false; }
    };
    const start = delay ? new Promise(r => setTimeout(r, delay)) : Promise.resolve();
    const p = start.then(() => { const q = this.queue.then(go, go); this.queue = q.catch(() => {}); return q; });
    return p;
  }
}

module.exports = { FocusOwner };
