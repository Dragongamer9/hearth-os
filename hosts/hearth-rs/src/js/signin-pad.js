// Runs in a store's sign-in window (Epic, GOG, Microsoft) so it works with just a controller, like on Windows:
//   left or right stick / d-pad   move the pointer          A        click
//   right stick (with LB held)    scroll                     Y        on-screen keyboard
//   B                             close the keyboard, or cancel sign-in
// In the keyboard: d-pad / left stick pick a key, A types it, X backspace, LB shift, Menu (☰) Enter, Y or B close.
// The pointer and keyboard are drawn on top of the store's page (in a shadow root, so the page's styles can't
// touch them). Clicks are real input events. Typing goes straight into the text box you picked (and stays
// there, even if the page moves focus), the way a paste would, so sites accept it. Picking a text box opens
// the keyboard by itself.

(() => {
if (window.__hearthPad) return;
if (window.__hearthPadWanted === false) return;   // the Xbox Cloud Gaming window, on a page that isn't a sign-in
window.__hearthPad = true;
// The real controllers: the sign-in page itself is shown none (signin-nogamepad.js), but this script needs them.
const readPads = () => { const g = window.__hearthGamepads || (navigator.getGamepads && navigator.getGamepads.bind(navigator)); return g ? [...g()].filter(Boolean) : []; };
// In the Rust host there's no Electron to send real input events: clicks, typing and scrolling happen in the
// page itself, and B leaves through an address the app catches.
const ipcRenderer = { send(what, p) {
  if (what === 'signin:cancel') { if (window.__hearthPadCancel) window.__hearthPadCancel(); else location.href = 'hearth-signin://cancel'; return; }
  if (what === 'signin:click') {
    const el = document.elementFromPoint(p.x, p.y);
    if (!el) return;
    const o = { bubbles: true, cancelable: true, clientX: p.x, clientY: p.y, view: window, button: 0 };
    for (const t of ['pointerdown', 'mousedown']) el.dispatchEvent(new (t.startsWith('pointer') ? PointerEvent : MouseEvent)(t, o));
    if (typeof el.focus === 'function') el.focus();
    for (const t of ['pointerup', 'mouseup']) el.dispatchEvent(new (t.startsWith('pointer') ? PointerEvent : MouseEvent)(t, o));
    el.click();
    return;
  }
  if (what === 'signin:wheel') { window.scrollBy(0, p.dy); return; }
  if (what === 'signin:type') { document.execCommand('insertText', false, p); return; }
  if (what === 'signin:key') {
    const el = document.activeElement;
    if (p === 'Return') { const f = el && el.form; if (f) { if (f.requestSubmit) f.requestSubmit(); else f.submit(); } else if (el) el.click(); }
    else if (p === 'Backspace') document.execCommand('delete', false);
    else if (p === 'Tab') { const all = [...document.querySelectorAll('input,button,a,select,textarea')].filter(e => !e.disabled); const i = all.indexOf(el); if (all[i + 1]) all[i + 1].focus(); }
  }
} };

const DEAD = 0.18;
const ROWS = [
  ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
  ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
  ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', '@'],
  ['z', 'x', 'c', 'v', 'b', 'n', 'm', '.', '-', '_'],
  ['shift', 'space', 'bksp', 'tab', 'enter'],
];
const SHIFTED = { 1: '!', 2: '#', 3: '$', 4: '%', 5: '&', 6: '*', 7: '+', 8: '?', 9: '(', 0: ')', '.': ',', '-': '/', '_': '=', '@': '~' };
const LABEL = { shift: '⇧ Shift', space: 'Space', bksp: '⌫', tab: 'Tab ⇥', enter: 'Enter ↵' };

let ui, cursor, kb, hint;
let x = innerWidth / 2, y = innerHeight / 2;
let kbOpen = false, shift = false, row = 1, col = 0;
let field = null;   // the text box you're typing into

const editable = el => !!el && (el.isContentEditable || el.tagName === 'TEXTAREA' ||
  (el.tagName === 'INPUT' && !/^(button|submit|reset|checkbox|radio|file|image|range|color|hidden)$/i.test(el.type || 'text')));
document.addEventListener('focusin', e => { if (editable(e.target)) field = e.target; }, true);
const visible = el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden'; };
function target() {
  const a = document.activeElement;
  if (editable(a)) return a;
  if (field && document.contains(field) && visible(field)) return field;
  // The page swapped the box for a new one (Microsoft's sign-in does this between the email and password
  // steps): use the box that's on screen now, rather than typing into whatever else has focus.
  const box = [...document.querySelectorAll('input,textarea,[contenteditable="true"]')].find(el => editable(el) && visible(el) && !el.disabled && !el.readOnly);
  if (box) { field = box; return box; }
  return null;
}
/** Types into the text box itself; falls back to key events if there isn't one. */
function typeText(t) {
  const el = target();
  // No text box: don't type into the page at all (a space or Enter on a focused link would open it).
  if (!el) return flashTip();
  if (document.activeElement !== el) el.focus();
  if (!document.execCommand('insertText', false, t)) sendType(t);
}
function backspace() {
  const el = target();
  if (!el) return flashTip();
  if (document.activeElement !== el) el.focus();
  if (!document.execCommand('delete', false)) sendKey('Backspace');
}
function enter() {
  const el = target();
  if (!el) return flashTip();
  if (document.activeElement !== el) el.focus();
  sendKey('Return');
}
function flashTip() { if (kb) { renderKb(); kb.querySelector('.tip').style.color = '#ffb4a8'; } }
const prev = {};
const repeatAt = {};

// Steam (running in the background) and some handheld drivers also turn controller buttons into keys for the
// focused window: A becomes Enter, B Escape. On a store's page that Enter "clicks" whatever link has focus, on
// top of what A already did here. Key presses that arrive while a controller button is held, and that this
// script didn't send, are dropped; a real keyboard still works.
let ownKeyAt = 0;
const sendKey = key => { ownKeyAt = performance.now(); ipcRenderer.send('signin:key', key); };
const sendType = t => { ownKeyAt = performance.now(); ipcRenderer.send('signin:type', t); };
let padHeldAt = 0;
function padHeld() {
  const pads = readPads();
  if (pads.some(p => p.buttons.some(b => b && b.pressed))) padHeldAt = performance.now();
  return performance.now() - padHeldAt < 300;
}
// The controller was used in the last few seconds (Steam and some drivers turn its buttons into mouse clicks and
// keys a moment later, sometimes after the button is already up).
let padUsedAt = 0;
const padActive = () => padHeld() || performance.now() - padUsedAt < 3000;
for (const type of ['keydown', 'keypress', 'keyup']) {
  window.addEventListener(type, e => {
    if (!e.isTrusted || performance.now() - ownKeyAt < 250) return;
    // While the on-screen keyboard is open, or a controller button is held, keys that aren't typing (Enter,
    // Escape, Space, Tab, arrows) come from the controller, not you: they'd press whatever link has focus.
    const nav = /^(Enter|Escape| |Spacebar|Tab|Arrow\w+)$/.test(e.key);
    if (padHeld() || (nav && (kbOpen || padActive()))) { e.preventDefault(); e.stopImmediatePropagation(); }
  }, true);
}
// The same for clicks: while a controller button is held, the only clicks that reach
// the page are the ones this script makes (Steam's desktop controls also turn A and the triggers into mouse
// clicks, which clicked whatever link was under the pointer while you typed).
let ownClickAt = 0;
// Clicks this script makes aren't "trusted" (they come from the page); real ones are. While you're using the
// controller (or typing on the on-screen keyboard), real mouse clicks are the controller's buttons turned into
// clicks by Steam or a driver, landing on whatever is under the system pointer: drop them. Touching the screen
// still works.
for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click', 'auxclick', 'dblclick', 'contextmenu']) {
  window.addEventListener(type, e => {
    if (!e.isTrusted) return;
    if (e.pointerType === 'touch' || e.pointerType === 'pen' || (e.sourceCapabilities && e.sourceCapabilities.firesTouchEvents)) return;
    if (performance.now() - lastTouchAt < 800) return;   // the mouse events a tap makes afterwards
    if (kbOpen || padActive()) { e.preventDefault(); e.stopImmediatePropagation(); }
  }, true);
}
let lastTouchAt = 0;
window.addEventListener('touchstart', () => { lastTouchAt = performance.now(); }, true);

function build() {
  const host = document.createElement('div');
  host.style.cssText = 'position:fixed;inset:0;pointer-events:none;z-index:2147483647';
  const root = host.attachShadow({ mode: 'closed' });
  root.innerHTML = `<style>
    .cur{position:fixed;width:22px;height:22px;margin:-11px 0 0 -11px;border-radius:50%;background:rgba(63,208,184,.35);
      border:2px solid #3fd0b8;box-shadow:0 0 0 2px rgba(0,0,0,.5);display:none}
    .hint{position:fixed;left:50%;bottom:14px;transform:translateX(-50%);background:rgba(18,24,33,.92);color:#e9eef4;
      font:600 13px system-ui,sans-serif;padding:8px 14px;border-radius:10px;border:1px solid #2a3443;white-space:nowrap}
    .kb{position:fixed;left:50%;bottom:0;transform:translateX(-50%);width:min(980px,100vw);box-sizing:border-box;padding:14px;
      background:#121821;border:1px solid #2a3443;border-bottom:0;border-radius:16px 16px 0 0;display:none;
      font:600 18px system-ui,sans-serif;color:#e9eef4}
    .r{display:flex;gap:8px;margin:0 0 8px}.k{flex:1;height:52px;display:flex;align-items:center;justify-content:center;
      background:#1d2531;border-radius:10px;border:2px solid transparent}.k.wide{flex:2.2}.k.on{border-color:#3fd0b8;background:#273140}
    .k.lit{background:#3fd0b8;color:#06261f}.tip{font:500 12px system-ui,sans-serif;color:#7f8b99;text-align:center}
  </style><div class="cur"></div><div class="hint"></div><div class="kb"></div>`;
  document.documentElement.appendChild(host);
  cursor = root.querySelector('.cur');
  hint = root.querySelector('.hint');
  kb = root.querySelector('.kb');
  renderHint();
}

function renderHint() {
  hint.textContent = kbOpen
    ? 'A type   X ⌫   LB shift   ☰ Enter   Y/B close keyboard'
    : 'Sticks move   A click   LB + right stick scroll   Y keyboard   B cancel';
}

function renderKb() {
  kb.innerHTML = ROWS.map((r, ri) => `<div class="r">${r.map((k, ci) => {
    const label = LABEL[k] || (shift ? SHIFTED[k] || k.toUpperCase() : k);
    const cls = ['k', LABEL[k] ? 'wide' : '', ri === row && ci === col ? 'on' : '', k === 'shift' && shift ? 'lit' : ''].join(' ');
    return `<div class="${cls}">${label}</div>`;
  }).join('')}</div>`).join('') + '<div class="tip">' + (target() ? 'Typing into the box you picked. B closes the keyboard.' : 'Pick a text box first: close the keyboard (B), point at the box and press A.') + '</div>';
}

function setKb(open) {
  kbOpen = open;
  kb.style.display = open ? 'block' : 'none';
  // Keep the field you're typing in visible: the keyboard goes to the other half of the screen.
  const top = y > innerHeight * 0.55;
  Object.assign(kb.style, top ? { top: '0', bottom: 'auto', borderRadius: '0 0 16px 16px' } : { top: 'auto', bottom: '0', borderRadius: '16px 16px 0 0' });
  Object.assign(hint.style, open && !top ? { top: '14px', bottom: 'auto' } : { top: 'auto', bottom: '14px' });
  if (open) renderKb();
  renderHint();
}

function pressKey() {
  const k = ROWS[row][col];
  if (k === 'shift') { shift = !shift; return renderKb(); }
  if (k === 'space') return typeText(' ');
  if (k === 'bksp') return backspace();
  if (k === 'tab') { const el = target(); if (el) el.focus(); return sendKey('Tab'); }
  if (k === 'enter') return enter();
  typeText(shift ? SHIFTED[k] || k.toUpperCase() : k);
  if (shift) { shift = false; renderKb(); }
}

function moveKey(dr, dc) {
  if (dr) {
    const from = ROWS[row].length - 1;
    row = Math.max(0, Math.min(ROWS.length - 1, row + dr));
    // Keep roughly the same horizontal spot when moving between rows of different lengths.
    col = Math.round((from ? col / from : 0) * (ROWS[row].length - 1));
  } else col = Math.max(0, Math.min(ROWS[row].length - 1, col + dc));
  renderKb();
}

// Edge-triggered buttons, with key repeat for directions.
function pressed(name, down, now, repeat) {
  const was = prev[name];
  prev[name] = down;
  if (!down) { delete repeatAt[name]; return false; }
  if (!was) { repeatAt[name] = now + 400; return true; }
  if (repeat && now >= repeatAt[name]) { repeatAt[name] = now + 110; return true; }
  return false;
}

let last = performance.now();
function poll(now) {
  const dt = Math.min(50, now - last) / 16.7;
  last = now;
  const pads = readPads();
  const st = { a: false, b: false, x: false, y: false, lb: false, start: false, up: false, down: false, left: false, right: false };
  let lx = 0, ly = 0, rx = 0, ry = 0;
  for (const p of pads) {
    const btn = i => !!(p.buttons[i] && p.buttons[i].pressed);
    st.a ||= btn(0); st.b ||= btn(1); st.x ||= btn(2); st.y ||= btn(3); st.lb ||= btn(4); st.start ||= btn(9);
    st.up ||= btn(12); st.down ||= btn(13); st.left ||= btn(14); st.right ||= btn(15);
    const ax = i => (Math.abs(p.axes[i] || 0) > DEAD ? p.axes[i] : 0);
    lx += ax(0); ly += ax(1); rx += ax(2); ry += ax(3);
  }
  if (Object.values(st).some(Boolean) || Math.abs(lx) + Math.abs(ly) + Math.abs(rx) + Math.abs(ry) > 0) padUsedAt = now;
  if (pads.length) {
    cursor.style.display = kbOpen ? 'none' : 'block';
    hint.style.display = 'block';
  }

  if (pressed('y', st.y, now)) setKb(!kbOpen);
  if (kbOpen) {
    if (pressed('up', st.up || ly < -0.6, now, true)) moveKey(-1, 0);
    if (pressed('down', st.down || ly > 0.6, now, true)) moveKey(1, 0);
    if (pressed('left', st.left || lx < -0.6, now, true)) moveKey(0, -1);
    if (pressed('right', st.right || lx > 0.6, now, true)) moveKey(0, 1);
    if (pressed('a', st.a, now, true)) pressKey();
    if (pressed('x', st.x, now, true)) backspace();
    if (pressed('lb', st.lb, now)) { shift = !shift; renderKb(); }
    if (pressed('start', st.start, now)) enter();
    if (pressed('b', st.b, now)) setKb(false);
  } else {
    // Pointer: either stick, faster the further it's pushed; d-pad nudges.
    const speed = 14 * dt;
    let mx = lx, my = ly;
    if (!st.lb) { mx += rx; my += ry; }
    x += Math.sign(mx) * mx * mx * speed * 1.6;
    y += Math.sign(my) * my * my * speed * 1.6;
    if (pressed('left', st.left, now, true)) x -= 12;
    if (pressed('right', st.right, now, true)) x += 12;
    if (pressed('up', st.up, now, true)) y -= 12;
    if (pressed('down', st.down, now, true)) y += 12;
    x = Math.max(0, Math.min(innerWidth - 1, x));
    y = Math.max(0, Math.min(innerHeight - 1, y));
    cursor.style.left = x + 'px';
    cursor.style.top = y + 'px';
    if (st.lb && Math.abs(ry) > 0) ipcRenderer.send('signin:wheel', { x, y, dy: -ry * 40 * dt });
    if (pressed('a', st.a, now)) {
      ownClickAt = performance.now();
      ipcRenderer.send('signin:click', { x, y });
      // Clicked into a text box: open the keyboard for it.
      setTimeout(() => { if (!kbOpen && editable(document.activeElement)) { field = document.activeElement; setKb(true); } }, 250);
    }
    if (pressed('start', st.start, now)) sendKey('Return');
    if (pressed('b', st.b, now)) ipcRenderer.send('signin:cancel');
  }
  requestAnimationFrame(poll);
}

if (document.readyState === 'loading') window.addEventListener('DOMContentLoaded', () => { build(); requestAnimationFrame(poll); });
else { build(); requestAnimationFrame(poll); }
})();
