'use strict';
// Hearth's Quick Access as Handheld Daemon's overlay app: HHD's line protocol, and how it maps to Hearth and to the
// Quick Access page. No Electron in here, so it can be tested on its own (test/overlay-protocol.test.js).
//
// HHD → here (stdin), see hhd/plugins/overlay/base.py and controllers.py (OverlayWriter):
//   cmd:open_qam, cmd:open_qam_if_closed (the first press of a double press), cmd:open_expanded, cmd:open_notification,
//   cmd:close, cmd:close_now, cmd:mute (HHD reads the controllers for us: we never read them ourselves)
//   action:<a|b|x|y|lb|rb|select|mode|up|down|left|right|x_up>  the controller, only while HHD holds it (grab)
// here → HHD (stdout):
//   stat:closed|qam|expanded|notification  HHD shows this window over the game (or hides it) when this changes
//   grab:enable|disable                    HHD keeps the controllers from the game; a grab lapses after 5 seconds,
//                                          so it's sent again every 2 seconds while open (as hhd-ui does)
// Hearth decides what a press opens (its own screen when Hearth is in front, Quick Access over a game otherwise):
// an open goes to Hearth as {type:'hhd', action:'open'}, and the page opens when Hearth answers overlay.shown;
// overlay.hidden closes it. A close goes to Hearth as the page's own close ({type:'overlay', action:'close'}).
// A press while Hearth isn't connected (HHD just restarted this app, or Hearth is restarting) isn't lost: it goes to
// Hearth when it connects, if that's within PENDING_MS, marked late (Hearth ignores one it caused itself by asking
// HHD to start this app).

const OPEN = new Set(['qam', 'expanded']);
const STATES = new Set(['closed', 'qam', 'expanded', 'notification']);
// HHD's action names → the page's controller names (the same ones Hearth's controller reader sends).
const ACTIONS = { a: 'a', b: 'b', x: 'x', y: 'y', lb: 'lb', rb: 'rb', select: 'select', mode: 'guide',
                  up: 'up', down: 'down', left: 'left', right: 'right' };
const GRAB_EVERY = 2000;
// A second "open" this soon after opening doesn't close it again (Guide + B: Guide opens it through Hearth, and HHD's
// Xbox + B shortcut asks for it too).
const TOGGLE_GUARD = 600;
const OSD_MS = 1800;
const PENDING_MS = 10000;

const ev = (name, data) => ({ type: 'event', name, data });

/** The page's controller name for an action: line from HHD, or null for anything it doesn't use. */
function padFor(action) { return Object.prototype.hasOwnProperty.call(ACTIONS, action) ? ACTIONS[action] : null; }

class OverlayProtocol {
  /**
   * out(line)       a line for HHD (without the newline)
   * toHearth(msg)   a message for Hearth (through its overlay socket)
   * toPage(msg)     a message for the Quick Access page
   * onHost(msg)     Hearth's instructions for the window itself ({type:'overlay-host', ...})
   * connected()     whether Hearth is there to ask
   */
  constructor({ out, toHearth, toPage, onHost = () => {}, connected = () => true, now = Date.now,
                setTimer = setTimeout, clearTimer = clearTimeout, every = setInterval, stopEvery = clearInterval }) {
    Object.assign(this, { out, toHearth, toPage, onHost, connected, now, setTimer, clearTimer, every, stopEvery });
    this.state = 'closed';
    this.want = 'qam';
    this.preopened = false;
    this.openedAt = 0;
    this.grabTimer = null;
    this.osdTimer = null;
    this.peeking = false;
    this.pending = null;   // {kind, at}: an open asked for while Hearth wasn't connected
  }

  get isOpen() { return OPEN.has(this.state); }

  /** A line from HHD (stdin). */
  line(raw) {
    const l = String(raw || '').trim();
    if (l.startsWith('cmd:')) return this.cmd(l.slice(4));
    if (l.startsWith('action:')) return this.action(l.slice(7));
  }

  cmd(c) {
    switch (c) {
      case 'open_qam_if_closed':
        // First press of a double press: open now to save the wait; the press that follows isn't a second toggle.
        if (!this.isOpen) { this.preopened = true; this.open('qam'); }
        return;
      case 'open_qam':
        if (this.preopened) { this.preopened = false; return; }
        return this.isOpen ? this.toggleClose() : this.open('qam');
      case 'open_expanded':
      case 'open_overlay':
        this.preopened = false;
        return this.isOpen ? this.toggleClose() : this.open('expanded');
      case 'close':
        this.preopened = false;
        return this.close();
      case 'close_now':
        this.preopened = false;
        this.setState('closed');
        return this.close();
      default:
        // open_notification (Hearth shows its own), mute/unmute (nothing to turn off: the page never reads controllers here)
        return;
    }
  }

  /** The controller (only while open: HHD only sends it while it holds the controllers for us). */
  action(a) {
    if (!this.isOpen) return;
    const b = padFor(a);
    if (b) this.toPage(ev('pad', { b }));
  }

  open(kind) {
    this.want = kind;
    if (this.connected()) { this.pending = null; this.toHearth({ type: 'hhd', action: 'open', how: kind }); }
    else this.pending = { kind, at: this.now() };
  }
  /** Hearth (re)connected: an open asked for meanwhile goes to it now (late). */
  hearthConnected() {
    const p = this.pending;
    this.pending = null;
    if (!p || this.now() - p.at > PENDING_MS || !this.connected()) return;
    this.want = p.kind;
    this.toHearth({ type: 'hhd', action: 'open', how: p.kind, late: true });
  }
  toggleClose() {
    if (this.now() - this.openedAt < TOGGLE_GUARD) return;
    this.close();
  }
  close() {
    this.pending = null;
    // Without Hearth there's no one to answer overlay.hidden: close here.
    if (!this.connected()) return this.setState('closed');
    this.toHearth({ type: 'overlay', action: 'close' });
  }

  /** Out of sight for a moment while Quick Access stays open (Take a screenshot: a picture of just the game). */
  peek(hidden) {
    if (hidden && this.isOpen && !this.peeking) {
      this.peeking = true;
      this.stopGrab();
      this.out('stat:closed');
      this.out('grab:disable');
    } else if (!hidden && this.peeking) {
      this.peeking = false;
      if (this.isOpen) { this.out(`stat:${this.state}`); this.startGrab(); }
    }
  }
  startGrab() {
    this.out('grab:enable');
    if (!this.grabTimer) this.grabTimer = this.every(() => this.out('grab:enable'), GRAB_EVERY);
  }
  stopGrab() { if (this.grabTimer) { this.stopEvery(this.grabTimer); this.grabTimer = null; } }

  /** Hearth went away (or restarted): nothing stays open over the game, and HHD gives the controller back. */
  disconnected() { this.preopened = false; this.setState('closed'); }

  /** A message from Hearth, passed on to the page (opening and closing are seen here on the way). */
  fromHearth(msg) {
    if (!msg || typeof msg !== 'object') return null;
    if (msg.type === 'overlay-host') { this.onHost(msg); return null; }
    if (msg.type === 'event') {
      switch (msg.name) {
        case 'overlay.shown':
          this.openedAt = this.now();
          this.setState(OPEN.has(this.want) ? this.want : 'qam');
          this.want = 'qam';   // the next open not asked for through HHD (Guide) is the plain one
          this.toPage(msg);
          // HHD hands us the controller as action: lines: the page mustn't also read it through the browser.
          this.toPage(ev('padHost', { connected: 1 }));
          return null;
        case 'overlay.hidden':
          this.setState('closed');
          break;
        case 'padHost':
          this.toPage(ev('padHost', { connected: 1 }));
          return null;
        case 'osd': {
          // The volume bar over the game: shown without taking the controller.
          if (this.state === 'closed' || this.state === 'notification') {
            this.setState('notification');
            if (this.osdTimer) this.clearTimer(this.osdTimer);
            const ms = (msg.data && Number(msg.data.ms)) || OSD_MS;
            this.osdTimer = this.setTimer(() => { this.osdTimer = null; if (this.state === 'notification') this.setState('closed'); }, ms + 300);
          }
          break;
        }
      }
    }
    this.toPage(msg);
    return null;
  }

  setState(s) {
    if (!STATES.has(s) || s === this.state) return;
    const was = this.state;
    const peeked = this.peeking;
    this.state = s;
    this.peeking = false;
    if (s !== 'notification' && this.osdTimer) { this.clearTimer(this.osdTimer); this.osdTimer = null; }
    this.out(`stat:${s}`);
    if (OPEN.has(s) && (!OPEN.has(was) || peeked)) this.startGrab();
    else if (!OPEN.has(s) && OPEN.has(was)) { this.stopGrab(); this.out('grab:disable'); }
  }
}

module.exports = { OverlayProtocol, padFor, ACTIONS, GRAB_EVERY, TOGGLE_GUARD, PENDING_MS };
