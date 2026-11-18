/* hearth-gamepad.js: how Hearth's apps read the controller (docs/CONTROLLER-FOCUS.md). One copy here; tools/pad-sync.py
 * copies it next to every app page that reads the controller (hosts/hearth-<app>/ui/hearth-gamepad.js).
 *
 *   HearthGamepad.loop(fn)   run fn(time) every frame for as long as the page lives (time: performance.now(), as
 *                            requestAnimationFrame gives it). An error in fn is logged and the next
 *                            frame comes anyway (a loop that rescheduled itself at its end died with the first error:
 *                            "the controller only works sometimes"). Frames come from requestAnimationFrame, and every
 *                            100 ms from a timer when the page stops getting frames (WebKitGTK pauses them).
 *   HearthGamepad.pads()     the connected controllers (navigator.getGamepads(), the empty slots left out), with two
 *                            fixes for desktop windows:
 *                            - while the window isn't focused (or is hidden) none: WebKitGTK stops updating the state
 *                              then, and a button that was down when the focus went stayed "down" (a D-pad held while a
 *                              notification or Quick Access came up kept moving the selection);
 *                            - when the focus comes back, a button or stick still held from before counts as let go
 *                              until it really is (the A that closed Quick Access or picked in the Hearth menu doesn't
 *                              press the button the app has selected too).
 *   HearthGamepad.active()   whether the window has the focus (as far as the page can tell).
 *   HearthGamepad.hint       false: never show the "press any button" note (see below).
 *
 * WebKitGTK only shows a page the controllers after a button was pressed on one while the page was open (the Gamepad
 * API's rule). Where a controller was used in this app before and none shows a moment after the window opened, a
 * small note says so once ("Press any button on your controller"); it goes away with the first press.
 * Desktop Mode's controller helper (hearth-pad) stands aside while one of these apps is in front: it sends no pointer
 * or keys then, so presses don't land twice (desktop/shell/controller-apps.json lists the apps).
 */
(function (root) {
  'use strict';
  if (root.HearthGamepad) return;
  const doc = root.document;
  const nav = root.navigator;
  const now = () => (root.performance && root.performance.now ? root.performance.now() : Date.now());
  const G = { hint: true };
  const loops = [];
  let blurred = false;          // a blur came and no focus since
  let wasActive = true;
  let guard = null;             // pad index → { b: Set(button indices), a: Set(axis indices) } held from before
  let raf = 0, last = 0, timer = 0, started = 0, seenPad = false, hintEl = null, hintShown = false;

  function hasFocus() { try { return !doc || typeof doc.hasFocus !== 'function' || doc.hasFocus(); } catch (e) { return true; } }
  /** The window has the focus: not hidden, and not blurred (a page that never saw a blur counts as focused, so a
   *  WebKit build whose hasFocus() is wrong can't lock the controller out). */
  G.active = function () {
    if (doc && doc.hidden) return false;
    if (hasFocus()) { blurred = false; return true; }
    return !blurred;
  };
  function raw() {
    let list = [];
    try { list = nav && nav.getGamepads ? Array.from(nav.getGamepads() || []) : []; } catch (e) { list = []; }
    return list.filter(g => g && g.connected !== false);
  }
  function snapshot(pads) {
    guard = {};
    for (const g of pads) {
      const b = new Set(), a = new Set();
      (g.buttons || []).forEach((x, i) => { if (x && (x.pressed || x.value > 0.5)) b.add(i); });
      (g.axes || []).forEach((v, i) => { if (Math.abs(v || 0) > 0.5) a.add(i); });
      if (b.size || a.size) guard[g.index] = { b, a };
    }
    if (!Object.keys(guard).length) guard = null;
  }
  function filtered(pads) {
    if (!guard) return pads;
    let any = false;
    const out = pads.map(g => {
      const h = guard[g.index];
      if (!h) return g;
      for (const i of [...h.b]) { const x = g.buttons[i]; if (!x || !(x.pressed || x.value > 0.5)) h.b.delete(i); }
      for (const i of [...h.a]) { if (Math.abs(g.axes[i] || 0) < 0.3) h.a.delete(i); }
      if (!h.b.size && !h.a.size) { delete guard[g.index]; return g; }
      any = true;
      const up = { pressed: false, touched: false, value: 0 };
      return {
        id: g.id, index: g.index, mapping: g.mapping, connected: g.connected, timestamp: g.timestamp,
        vibrationActuator: g.vibrationActuator, hapticActuators: g.hapticActuators,
        buttons: Array.from(g.buttons, (x, i) => (h.b.has(i) ? up : x)),
        axes: Array.from(g.axes, (v, i) => (h.a.has(i) ? 0 : v)),
      };
    });
    if (!any && !Object.keys(guard).length) guard = null;
    return out;
  }
  G.pads = function () {
    const active = G.active();
    if (!active) { wasActive = false; return []; }
    const pads = raw();
    if (!wasActive) { wasActive = true; snapshot(pads); }
    if (pads.length) noteSeen(pads);
    return filtered(pads);
  };

  // ---- "Press any button on your controller", once, where a controller was used in this app before.
  function store(k, v) { try { if (v === undefined) return root.localStorage.getItem(k); root.localStorage.setItem(k, v); } catch (e) { /* no storage */ } return null; }
  function noteSeen(pads) {
    if (!seenPad && pads.some(g => (g.buttons || []).some(x => x && x.pressed))) { seenPad = true; store('hearth-pad-seen', '1'); }
    if (hintEl) { hintEl.remove(); hintEl = null; }
  }
  function maybeHint() {
    if (!G.hint || hintShown || seenPad || !doc || !doc.body || !G.active() || now() - started < 1500) return;
    if (store('hearth-pad-seen') !== '1' || raw().length) return;
    hintShown = true;
    hintEl = doc.createElement('div');
    hintEl.className = 'hearth-pad-hint';
    hintEl.setAttribute('role', 'status');
    hintEl.textContent = 'Press any button on your controller to use it here';
    hintEl.style.cssText = 'position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:2147483646;padding:10px 18px;' +
      'border-radius:999px;background:rgba(20,22,28,.92);color:#fff;font:500 14px/1.3 system-ui,sans-serif;pointer-events:none;' +
      'box-shadow:0 6px 24px rgba(0,0,0,.35)';
    doc.body.appendChild(hintEl);
    setTimeout(() => { if (hintEl) { hintEl.remove(); hintEl = null; } }, 8000);
  }

  // ---- the loop
  function tick() {
    raf = 0;
    last = now();
    for (const fn of loops.slice()) {
      try { fn(last); } catch (e) { if (root.console) root.console.error('controller:', e); }
    }
    maybeHint();
    schedule();
  }
  function schedule() {
    if (raf || !loops.length) return;
    raf = root.requestAnimationFrame ? root.requestAnimationFrame(tick) : root.setTimeout(tick, 16);
  }
  function watchdog() {
    if (!loops.length) return;
    if (now() - last > 250) {
      if (raf && root.cancelAnimationFrame) root.cancelAnimationFrame(raf);
      raf = 0;
      tick();
    }
  }
  G.loop = function (fn) {
    if (typeof fn !== 'function' || loops.includes(fn)) return;
    loops.push(fn);
    if (!started) started = now();
    if (!timer) timer = root.setInterval(watchdog, 100);
    schedule();
  };
  G.stop = function (fn) { const i = loops.indexOf(fn); if (i >= 0) loops.splice(i, 1); };

  function lost() { blurred = true; wasActive = false; }
  function back() { blurred = false; schedule(); }
  if (root.addEventListener) {
    root.addEventListener('blur', lost);
    root.addEventListener('focus', back);
    root.addEventListener('pageshow', back);
    root.addEventListener('gamepadconnected', back);
    root.addEventListener('gamepaddisconnected', () => schedule());
  }
  if (doc && doc.addEventListener) doc.addEventListener('visibilitychange', () => { if (doc.hidden) wasActive = false; else schedule(); });

  root.HearthGamepad = G;
})(typeof window !== 'undefined' ? window : globalThis);
