// Hearth Theater's viewer, from the couch: while a site is open (electron/main.js shows it over this page), this
// page reads the controller and drives a real pointer (the core's uinput or XTest mouse), so the site never gets
// any script of Theater's and every click is the system's own: inside frames from other sites, video players,
// popups and bot checks alike.
//   Left stick: pointer   A: click (hold to drag)   B: back a page (or leave the page's full screen)
//   Right stick, D-pad up/down: scroll   D-pad left/right: video −/+ 10 s   X: play/pause   Y: video full screen
//   LB/RB: back/forward   RT: on-screen keyboard   Menu: these hints   View: back to Theater
// Without theaterHost (a plain browser), it does nothing. Tests set window.__theaterFakePad ({a: true, lx: 1...}).
'use strict';
(() => {
  const host = window.theaterHost;
  if (!host) return;
  const DEAD = 0.18;
  const curve = v => { const a = Math.abs(v); if (a < DEAD) return 0; const n = (a - DEAD) / (1 - DEAD); return Math.sign(v) * n * n; };
  const BTN = { a: 0, b: 1, x: 2, y: 3, lb: 4, rb: 5, lt: 6, rt: 7, select: 8, start: 9, up: 12, down: 13, left: 14, right: 15 };
  let open = false, timer = 0, last = 0, fx = 0, fy = 0, wx = 0, wy = 0, oskOn = false;
  const held = {};

  function read() {
    const s = {}; let lx = 0, ly = 0, rx = 0, ry = 0;
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    for (const g of pads) {
      for (const [k, i] of Object.entries(BTN)) { const b = g.buttons[i]; if (b && (b.pressed || ((k === 'lt' || k === 'rt') && b.value > 0.5))) s[k] = true; }
      lx += curve(g.axes[0] || 0); ly += curve(g.axes[1] || 0); rx += curve(g.axes[2] || 0); ry += curve(g.axes[3] || 0);
    }
    const f = window.__theaterFakePad;
    if (f) { for (const k of Object.keys(BTN)) if (f[k]) s[k] = true; lx += f.lx || 0; ly += f.ly || 0; rx += f.rx || 0; ry += f.ry || 0; }
    return { s, lx, ly, rx, ry };
  }

  function press(k) {
    switch (k) {
      case 'a': host.ptr({ op: 'button', button: 'left', down: true }); break;
      case 'b': host.viewer('back'); break;
      case 'x': host.viewer('media', { do: 'toggle' }); break;
      case 'y': host.viewer('media', { do: 'fill' }); break;
      case 'left': host.viewer('media', { do: 'seek', s: -10 }); break;
      case 'right': host.viewer('media', { do: 'seek', s: 10 }); break;
      case 'up': host.ptr({ op: 'wheel', dx: 0, dy: -360 }); break;
      case 'down': host.ptr({ op: 'wheel', dx: 0, dy: 360 }); break;
      case 'lb': host.viewer('back'); break;
      case 'rb': host.viewer('forward'); break;
      case 'rt': keyboard(); break;
      case 'start': host.viewer('hints'); break;
      case 'select': host.viewer('close'); break;
    }
  }
  function release(k) { if (k === 'a') host.ptr({ op: 'button', button: 'left', down: false }); }

  function tick() {
    if (!open) return;
    const now = performance.now(), dt = Math.min(50, now - last); last = now;
    const { s, lx, ly, rx, ry } = read();
    // The on-screen keyboard reads the controller itself while it's open; buttons held then wait to be let go.
    if (oskOn && window.HearthOSK && HearthOSK.isOpen()) { for (const k of Object.keys(BTN)) held[k] = s[k] ? -1 : 0; return; }
    if (oskOn) closedKeyboard();
    if (lx || ly) {
      const speed = 1.15 * Math.max(screen.width || innerWidth, 800) / 1000;   // about a screen's width in under a second
      fx += lx * speed * dt; fy += ly * speed * dt;
      const dx = Math.trunc(fx), dy = Math.trunc(fy);
      if (dx || dy) { fx -= dx; fy -= dy; host.ptr({ op: 'move', dx, dy }); }
    }
    if (rx || ry) {
      // In 1/120 of a wheel notch: up to about 20 notches a second.
      wx += rx * dt * 2.4; wy += ry * dt * 2.4;
      const dx = Math.trunc(wx), dy = Math.trunc(wy);
      if (dx || dy) { wx -= dx; wy -= dy; host.ptr({ op: 'wheel', dx, dy }); }
    }
    for (const k of Object.keys(BTN)) {
      if (!s[k]) { if (held[k] > 0) release(k); held[k] = 0; continue; }
      if (held[k] === -1) continue;                       // held since before the viewer opened (the A that opened it)
      const dir = k === 'up' || k === 'down';
      if (!held[k]) { held[k] = now; press(k); }
      else if (dir && now - held[k] > 400) { held[k] = now - 300; press(k); }
    }
  }

  // ---- the on-screen keyboard (Hearth's, osk/hearth-osk.js), docked under the site
  function keyboard() {
    if (!window.HearthOSK) { host.viewer('toast', { text: "The on-screen keyboard isn't here" }); return; }
    if (oskOn) { HearthOSK.close(); return; }
    const keys = { enter: 'enter', left: 'left', right: 'right', home: 'home', end: 'end' };
    HearthOSK.open({
      mode: 'text', enterLabel: 'Enter',
      adapter: {
        context: () => '',
        insert: text => host.viewer('type', { text }),
        deleteBack: n => host.ptr({ op: 'key', key: 'backspace', n: Math.max(1, n || 1) }),
        key: name => { if (keys[name]) host.ptr({ op: 'key', key: keys[name] }); },
      },
      onClose: () => closedKeyboard(),
    });
    oskOn = true;
    const dock = document.querySelector('.hosk-dock');
    // Its height (the dock slides in, so where it is right now isn't where it ends up).
    const h = dock ? dock.getBoundingClientRect().height : innerHeight * 0.4;
    const atTop = !!(dock && dock.classList.contains('at-top'));
    host.viewer('osk', { on: true, top: atTop ? 0 : innerHeight - h, h });
  }
  function closedKeyboard() {
    if (!oskOn) return;
    oskOn = false;
    if (window.HearthOSK && HearthOSK.isOpen()) HearthOSK.close();
    host.viewer('osk', { on: false });
  }

  function start() {
    if (open) return;
    open = true; last = performance.now(); fx = fy = wx = wy = 0;
    if (typeof H !== 'undefined') H.viewing = true;
    const { s } = read();
    for (const k of Object.keys(BTN)) held[k] = s[k] ? -1 : 0;
    // A timer, not animation frames: this page is under the site while it shows.
    timer = setInterval(tick, 16);
  }
  function stop() {
    if (!open) return;
    if (held.a > 0) release('a');
    open = false; clearInterval(timer);
    closedKeyboard();
    if (typeof H !== 'undefined') H.viewing = false;
  }
  host.on(m => { if (m && m.type === 'open') start(); else if (m && m.type === 'closed') stop(); });
  window.TheaterViewerPad = { start, stop, isOpen: () => open };
})();
