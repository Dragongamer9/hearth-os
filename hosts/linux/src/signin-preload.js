'use strict';
// Runs in a store's sign-in window (Epic, GOG, Microsoft) so it works with just a controller, like on Windows:
//   left or right stick / d-pad   move the pointer          A        click
//   right stick (with LB held)    scroll                     Y        on-screen keyboard
//   B                             close the keyboard, or cancel sign-in
// In the keyboard: d-pad / left stick pick a key, A types it, X backspace, LB shift, Menu (☰) Enter, Y or B close.
// The pointer and keyboard are drawn on top of the store's page (in a shadow root, so the page's styles can't
// touch them). Clicks are real input events. Typing goes straight into the text box you picked (and stays
// there, even if the page moves focus), the way a paste would, so sites accept it. Picking a text box opens
// the keyboard by itself.

const { ipcRenderer } = require('electron');

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
function target() {
  const a = document.activeElement;
  if (editable(a)) return a;
  if (field && document.contains(field)) return field;
  return null;
}
/** Types into the text box itself; falls back to key events if there isn't one. */
function typeText(t) {
  const el = target();
  if (!el) return ipcRenderer.send('signin:type', t);
  if (document.activeElement !== el) el.focus();
  if (!document.execCommand('insertText', false, t)) ipcRenderer.send('signin:type', t);
}
function backspace() {
  const el = target();
  if (!el) return ipcRenderer.send('signin:key', 'Backspace');
  if (document.activeElement !== el) el.focus();
  if (!document.execCommand('delete', false)) ipcRenderer.send('signin:key', 'Backspace');
}
function enter() {
  const el = target();
  if (el && document.activeElement !== el) el.focus();
  ipcRenderer.send('signin:key', 'Return');
}
const prev = {};
const repeatAt = {};

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
  if (k === 'tab') { const el = target(); if (el) el.focus(); return ipcRenderer.send('signin:key', 'Tab'); }
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
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const st = { a: false, b: false, x: false, y: false, lb: false, start: false, up: false, down: false, left: false, right: false };
  let lx = 0, ly = 0, rx = 0, ry = 0;
  for (const p of pads) {
    const btn = i => !!(p.buttons[i] && p.buttons[i].pressed);
    st.a ||= btn(0); st.b ||= btn(1); st.x ||= btn(2); st.y ||= btn(3); st.lb ||= btn(4); st.start ||= btn(9);
    st.up ||= btn(12); st.down ||= btn(13); st.left ||= btn(14); st.right ||= btn(15);
    const ax = i => (Math.abs(p.axes[i] || 0) > DEAD ? p.axes[i] : 0);
    lx += ax(0); ly += ax(1); rx += ax(2); ry += ax(3);
  }
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
      ipcRenderer.send('signin:click', { x, y });
      // Clicked into a text box: open the keyboard for it.
      setTimeout(() => { if (!kbOpen && editable(document.activeElement)) { field = document.activeElement; setKb(true); } }, 250);
    }
    if (pressed('start', st.start, now)) ipcRenderer.send('signin:key', 'Return');
    if (pressed('b', st.b, now)) ipcRenderer.send('signin:cancel');
  }
  requestAnimationFrame(poll);
}

window.addEventListener('DOMContentLoaded', () => { build(); requestAnimationFrame(poll); });
