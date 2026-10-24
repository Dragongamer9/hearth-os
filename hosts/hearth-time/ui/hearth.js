// What every Hearth Time page shares (Clock, Calendar, Notes, Weather and the ringing window): talking to the Rust
// side, moving around with arrow keys, a D-pad or a stick, dialogs, toasts, an on-screen keyboard for typing with a
// controller, number spinners, and icons. Opened in a plain browser (no Tauri), each page answers its own calls with
// sample data (window.DEMO), so it can be looked at and checked without the app.
'use strict';
const H = (() => {
  const T = window.__TAURI__;
  const invoke = (cmd, args = {}) => T ? T.core.invoke(cmd, args) : Promise.resolve().then(() => window.DEMO ? window.DEMO(cmd, args) : Promise.reject('preview'));
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const pad2 = n => String(n).padStart(2, '0');

  // ------------------------------------------------------------------------------------------- icons (24x24, stroked)
  const P = {
    plus: 'M12 5v14M5 12h14', minus: 'M5 12h14', x: 'M6 6l12 12M18 6L6 18', check: 'M5 12.5l4.5 4.5L19 7',
    back: 'M15 5l-7 7 7 7', fwd: 'M9 5l7 7-7 7', up: 'M5 15l7-7 7 7', down: 'M5 9l7 7 7-7',
    trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13', edit: 'M4 20h4L19 9l-4-4L4 16zM13 7l4 4', pin: 'M9 4h6l-1 6 3 3H7l3-3zM12 13v7',
    search: 'M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13zM15.5 15.5L20 20', refresh: 'M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6',
    alarm: 'M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM12 9v4l2.5 2.5M4 5l3-2M20 5l-3-2', timer: 'M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM12 13V9M10 2h4M18.5 6.5l1.5-1.5',
    stopwatch: 'M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM12 13l3-3M10 2h4', globe: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3 12h18M12 3c3 3.5 3 14.5 0 18M12 3c-3 3.5-3 14.5 0 18',
    play: 'M8 5v14l11-7z', pause: 'M8 5v14M16 5v14', reset: 'M4 12a8 8 0 1 0 2.3-5.7M4 4v5h5', flag: 'M5 21V4h11l-2 4 2 4H5',
    cal: 'M4 6h16v14H4zM4 10h16M8 3v4M16 3v4', list: 'M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01', menu: 'M4 7h16M4 12h16M4 17h16',
    import: 'M12 4v11M7 10l5 5 5-5M5 20h14', export: 'M12 16V5M7 9l5-5 5 5M5 20h14', link: 'M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1',
    bell: 'M6 16V11a6 6 0 1 1 12 0v5l2 2H4zM10 20a2 2 0 0 0 4 0', repeat: 'M17 2l3 3-3 3M4 11V9a4 4 0 0 1 4-4h12M7 22l-3-3 3-3M20 13v2a4 4 0 0 1-4 4H4',
    place: 'M12 21s7-6.2 7-12a7 7 0 1 0-14 0c0 5.8 7 12 7 12zM12 11.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z', note: 'M6 3h9l4 4v14H6zM14 3v5h5M9 13h7M9 17h5',
    checklist: 'M4 6l1.5 1.5L8 5M4 12l1.5 1.5L8 11M4 18l1.5 1.5L8 17M11 6h9M11 12h9M11 18h9', bold: 'M7 4h6a4 4 0 0 1 0 8H7zM7 12h7a4 4 0 0 1 0 8H7z',
    heading: 'M6 4v16M18 4v16M6 12h12', bullet: 'M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01', folder: 'M3 6h6l2 2h10v11H3z',
    eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', keyboard: 'M3 6h18v12H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10',
    snooze: 'M12 21a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM9.5 10h5l-5 6h5M4 5l3-2M20 5l-3-2', image: 'M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M15.5 9.5h.01',
  };
  const icon = (name, cls = '') => `<svg class="${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${P[name] || ''}"/></svg>`;

  // ------------------------------------------------------------------------------------------- dialogs and toasts
  let lastFocus = null;
  const layer = () => $('#layer') || document.body.appendChild(Object.assign(document.createElement('div'), { id: 'layer' }));
  function dialog(html, opts = {}) {
    if (!layer().firstChild) lastFocus = document.activeElement;
    layer().innerHTML = `<div class="veil"><div class="dlg ${opts.wide ? 'wide' : ''}" role="dialog">${html}</div></div>`;
    const d = $('.dlg', layer());
    (d.querySelector('[data-autofocus]') || d.querySelector('.f'))?.focus();
    if (!opts.sticky) $('.veil', layer()).addEventListener('click', e => { if (e.target.classList.contains('veil')) closeDialog(); });
    return d;
  }
  function closeDialog() { closeOsk(); layer().innerHTML = ''; lastFocus?.focus?.({ preventScroll: true }); lastFocus = null; }
  const dialogOpen = () => !!layer().firstChild;
  function ask(title, text, yes = 'OK', danger = false) {
    return new Promise(res => {
      const d = dialog(`<h2>${esc(title)}</h2>${text ? `<p class="muted">${esc(text)}</p>` : ''}<div class="row end"><button class="btn f" data-a="no" ${danger ? 'data-autofocus' : ''}>Cancel</button><button class="btn primary f ${danger ? 'danger' : ''}" data-a="yes" ${danger ? '' : 'data-autofocus'}>${esc(yes)}</button></div>`);
      d.addEventListener('click', e => { const b = e.target.closest('[data-a]'); if (!b) return; closeDialog(); res(b.dataset.a === 'yes'); });
      H.onDialogBack = () => { closeDialog(); res(false); };
    });
  }
  let toastTimer = 0;
  function toast(text, ms = 3200) {
    let t = $('#toast');
    if (!t) { t = document.createElement('div'); t.id = 'toast'; t.className = 'toast'; document.body.appendChild(t); }
    t.textContent = text; t.classList.add('on');
    clearTimeout(toastTimer); toastTimer = setTimeout(() => t.classList.remove('on'), ms);
  }

  // ------------------------------------------------------------------------------------------- spinners
  // <div class="spin f" tabindex="0" data-spin="hour" data-min="0" data-max="23" data-v="7" data-wrap="1" data-padw="2">
  function spin(name, v, min, max, opts = {}) {
    return `<div class="spin f" tabindex="0" data-spin="${name}" data-min="${min}" data-max="${max}" data-v="${v}" data-wrap="${opts.wrap === false ? 0 : 1}" data-step="${opts.step || 1}" data-padw="${opts.pad ?? 2}" role="spinbutton" aria-label="${esc(opts.cap || name)}">
      <span class="u" data-spinstep="1">${icon('up')}</span><span class="v">${fmtSpin(v, opts.pad ?? 2, opts.names)}</span><span class="d" data-spinstep="-1">${icon('down')}</span>${opts.cap ? `<span class="cap">${esc(opts.cap)}</span>` : ''}</div>`;
  }
  const spinNames = {};
  function fmtSpin(v, padw, names) { return names ? names[v] ?? v : padw ? String(v).padStart(padw, '0') : String(v); }
  function setSpin(el, v) {
    const min = +el.dataset.min, max = +el.dataset.max, wrap = el.dataset.wrap === '1';
    if (v > max) v = wrap ? min + (v - max - 1) : max;
    if (v < min) v = wrap ? max - (min - v - 1) : min;
    el.dataset.v = v;
    $('.v', el).textContent = fmtSpin(v, +el.dataset.padw, spinNames[el.dataset.spin]);
    el.dispatchEvent(new CustomEvent('spin', { bubbles: true, detail: { name: el.dataset.spin, value: v } }));
  }
  const stepSpin = (el, dir) => setSpin(el, +el.dataset.v + dir * (+el.dataset.step || 1));
  document.addEventListener('click', e => { const s = e.target.closest('[data-spinstep]'); if (s) { const el = s.closest('.spin'); el.focus(); stepSpin(el, +s.dataset.spinstep); } });
  document.addEventListener('wheel', e => { const el = e.target.closest('.spin'); if (el) { e.preventDefault(); stepSpin(el, e.deltaY < 0 ? 1 : -1); } }, { passive: false });
  let typed = '', typedAt = 0;
  function spinKey(el, k) {
    if (!/^[0-9]$/.test(k)) return false;
    const now = Date.now();
    typed = (now - typedAt < 1200 && typed.length < 4 ? typed : '') + k; typedAt = now;
    let v = +typed;
    if (v > +el.dataset.max) { typed = k; v = +k; }
    setSpin(el, Math.max(+el.dataset.min, v));
    return true;
  }

  // ------------------------------------------------------------------------------------------- moving around
  function focusables(root) {
    root = root || (dialogOpen() ? layer() : document);
    return $$('.f', root).filter(el => !el.disabled && el.offsetParent !== null && !el.closest('[hidden]'));
  }
  function move(dir) {
    const cur = document.activeElement;
    if (cur?.classList.contains('spin') && (dir === 'up' || dir === 'down')) { stepSpin(cur, dir === 'up' ? 1 : -1); return; }
    if (H.onMove && H.onMove(dir, cur) === true) return;
    const all = focusables(); if (!all.length) return;
    if (!cur || !all.includes(cur)) { all[0].focus(); return; }
    const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of all) {
      if (el === cur) continue;
      const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
      const dx = bx - ax, dy = by - ay;
      const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
      if (!ok) continue;
      const main = dir === 'left' || dir === 'right' ? Math.abs(dx) : Math.abs(dy);
      const cross = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
      const score = main + cross * 2.2;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (best) { best.focus({ preventScroll: true }); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  }
  function choose() {
    const el = document.activeElement;
    if (!el || el === document.body) { focusables()[0]?.focus(); return; }
    if (H.pad && (el.tagName === 'INPUT' && el.type !== 'checkbox' || el.tagName === 'TEXTAREA') && !oskTarget) { openOsk(el); return; }
    if (el.tagName === 'INPUT' && el.type === 'checkbox') { el.click(); return; }
    if (el.tagName === 'BUTTON' || el.classList.contains('f')) el.click();
  }
  function back() {
    if (oskTarget) { closeOsk(); return; }
    if (dialogOpen()) { if (H.onDialogBack) { const f = H.onDialogBack; H.onDialogBack = null; f(); } else closeDialog(); return; }
    if (H.onBack) H.onBack();
  }
  const typing = el => el && (el.tagName === 'TEXTAREA' || (el.tagName === 'INPUT' && !['checkbox', 'range', 'button'].includes(el.type)) || el.isContentEditable);
  document.addEventListener('keydown', e => {
    H.pad = false; document.body.classList.remove('pad');
    if (layer().contains(e.target) && e.target !== layer()) return;
    const k = e.key, el = e.target;
    if (k === 'Escape') { e.preventDefault(); if (typing(el) && !dialogOpen() && H.onEscapeTyping) { H.onEscapeTyping(el); return; } back(); return; }
    if (typing(el)) {
      if (k === 'Enter' && el.tagName === 'INPUT' && !e.shiftKey) { if (el.dataset.enter) return; e.preventDefault(); move('down'); }
      if ((k === 'ArrowDown' || k === 'ArrowUp') && el.tagName === 'INPUT') { e.preventDefault(); move(k === 'ArrowDown' ? 'down' : 'up'); }
      return;
    }
    if (el?.classList?.contains('spin') && spinKey(el, k)) { e.preventDefault(); return; }
    if (H.onKey && H.onKey(e) === true) { e.preventDefault(); return; }
    const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    if (dirs[k]) { e.preventDefault(); move(dirs[k]); return; }
    if ((k === 'Enter' || k === ' ') && el?.classList?.contains('f') && el.tagName !== 'BUTTON') { e.preventDefault(); el.click(); }
    if (k === 'Backspace') { e.preventDefault(); back(); }
  });
  // Keys inside a dialog (or the on-screen keyboard in it) are handled here, not by the page.
  layer();
  layer().addEventListener('keydown', e => {
    const k = e.key, el = e.target;
    if (k === 'Escape') { e.preventDefault(); back(); return; }
    if (typing(el)) {
      if (k === 'Enter' && el.tagName === 'INPUT') { if (el.dataset.enter) return; e.preventDefault(); move('down'); }
      if ((k === 'ArrowDown' || k === 'ArrowUp') && el.tagName === 'INPUT') { e.preventDefault(); move(k === 'ArrowDown' ? 'down' : 'up'); }
      return;
    }
    if (el?.classList?.contains('spin') && spinKey(el, k)) { e.preventDefault(); return; }
    const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    if (dirs[k]) { e.preventDefault(); move(dirs[k]); return; }
    if ((k === 'Enter' || k === ' ') && el?.classList?.contains('f') && el.tagName !== 'BUTTON') { e.preventDefault(); el.click(); }
  });

  // ------------------------------------------------------------------------------------------- controllers
  // The standard mapping: D-pad or left stick moves, A chooses, B goes back; X, Y, LB, RB, Start: each page's own.
  const held = {}; let lastMove = 0;
  function poll() {
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    const now = performance.now();
    const s = { up: 0, down: 0, left: 0, right: 0, a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0, start: 0, select: 0 };
    for (const g of pads) {
      const bt = i => g.buttons[i]?.pressed;
      const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
      s.up ||= bt(12) || ay < -.6; s.down ||= bt(13) || ay > .6; s.left ||= bt(14) || ax < -.6; s.right ||= bt(15) || ax > .6;
      s.a ||= bt(0); s.b ||= bt(1); s.x ||= bt(2); s.y ||= bt(3); s.lb ||= bt(4); s.rb ||= bt(5); s.select ||= bt(8); s.start ||= bt(9);
    }
    // The on-screen keyboard reads the controller itself while it's open (held buttons stay held for after).
    if (window.HearthOSK && HearthOSK.isOpen()) { for (const [k, on] of Object.entries(s)) held[k] = on ? (held[k] || now) : 0; requestAnimationFrame(poll); return; }
    for (const [k, on] of Object.entries(s)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const dir = ['up', 'down', 'left', 'right'].includes(k);
      const repeat = held[k] && dir && now - held[k] > 380 && now - lastMove > 110;
      if (first) held[k] = now;
      if (!first && !repeat) continue;
      H.pad = true; document.body.classList.add('pad'); lastMove = now;
      if (k === 'a') choose(); else if (k === 'b') back();
      else if (dir) { if (oskTarget || !H.onPadDir || H.onPadDir(k) !== true) move(k); }
      else if (H.buttons[k]) H.buttons[k]();
    }
    requestAnimationFrame(poll);
  }
  requestAnimationFrame(poll);

  // ------------------------------------------------------------------------------------------- on-screen keyboard
  // Hearth's shared keyboard (osk/, docs/OSK.md); the small one below only if its files are missing.
  let oskTarget = null, oskLower = true, oskSym = false;
  const ROWS = { abc: ['qwertyuiop', 'asdfghjkl\'', 'zxcvbnm,.?'], sym: ['1234567890', '-/:;()&@"#', '!*+=_%[]<>'] };
  function openOsk(input) {
    closeOsk();
    oskTarget = input;
    if (window.HearthOSK) {
      HearthOSK.open({ target: input, enterLabel: input.tagName === 'TEXTAREA' ? '' : 'Done', onEnter: () => { HearthOSK.close(); move('down'); }, onClose: () => { oskTarget = null; } });
      return;
    }
    const rows = (oskSym ? ROWS.sym : ROWS.abc).map(r => [...r].map(c => `<button class="f" data-osk="${esc(c)}">${esc(oskLower || oskSym ? c : c.toUpperCase())}</button>`).join('')).join('');
    input.insertAdjacentHTML('afterend', `<div class="osk" id="osk">${rows}<button class="f wide" data-osk="shift">${oskSym ? 'abc' : '⇧'}</button><button class="f wide" data-osk="sym">${oskSym ? '' : '123'}</button><button class="f wider" data-osk="space">Space</button><button class="f wide" data-osk="back">⌫</button>${input.tagName === 'TEXTAREA' ? '<button class="f wide" data-osk="enter">↵</button>' : ''}<button class="f wide" data-osk="done">Done</button></div>`);
    $('#osk .f').focus();
  }
  function closeOsk() { if (window.HearthOSK && HearthOSK.isOpen()) { HearthOSK.close(); return; } const t = oskTarget; $('#osk')?.remove(); oskTarget = null; t?.focus({ preventScroll: true }); }
  document.addEventListener('click', e => {
    const b = e.target.closest('[data-osk]'); if (!b || !oskTarget) return;
    const k = b.dataset.osk, t = oskTarget;
    if (k === 'done') return closeOsk();
    if (k === 'shift') { if (oskSym) oskSym = false; else oskLower = !oskLower; openOsk(t); return; }
    if (k === 'sym') { oskSym = true; openOsk(t); return; }
    const s = t.selectionStart ?? t.value.length, en = t.selectionEnd ?? s;
    let ins = k === 'space' ? ' ' : k === 'enter' ? '\n' : k === 'back' ? '' : (oskLower || oskSym ? k : k.toUpperCase());
    if (t.maxLength > 0 && t.value.length - (en - s) + ins.length > t.maxLength) return;
    const from = k === 'back' && s === en ? Math.max(0, s - 1) : s;
    t.value = t.value.slice(0, from) + ins + t.value.slice(en);
    const at = from + ins.length;
    try { t.setSelectionRange(at, at); } catch { }
    t.dispatchEvent(new Event('input', { bubbles: true }));
    if (!oskLower && !oskSym && ins.length === 1 && /[a-z]/i.test(ins)) { oskLower = true; const f = document.activeElement?.dataset?.osk; openOsk(t); if (f) $(`#osk [data-osk="${CSS.escape(f.toLowerCase())}"]`)?.focus(); }
  });

  // ------------------------------------------------------------------------------------------- time words
  let clock24 = !(new Intl.DateTimeFormat(undefined, { hour: 'numeric' }).resolvedOptions().hour12);
  const fmtTime = (d, opts = {}) => {
    d = d instanceof Date ? d : new Date(d);
    const h = d.getHours(), m = pad2(d.getMinutes());
    if (H.clock24) return `${pad2(h)}:${m}${opts.sec ? ':' + pad2(d.getSeconds()) : ''}`;
    const ap = h < 12 ? 'AM' : 'PM';
    return `${h % 12 || 12}:${m}${opts.sec ? ':' + pad2(d.getSeconds()) : ''}${opts.noAp ? '' : ' ' + ap}`;
  };
  const DAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
  const ymd = d => `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
  const parseYmd = s => { const [y, m, d] = String(s).slice(0, 10).split('-').map(Number); return new Date(y, (m || 1) - 1, d || 1); };
  const fmtDate = (d, opts = {}) => {
    d = d instanceof Date ? d : new Date(d);
    const s = `${opts.noDay ? '' : (opts.short ? DAYS[d.getDay()].slice(0, 3) : DAYS[d.getDay()]) + ', '}${opts.short ? MONTHS[d.getMonth()].slice(0, 3) : MONTHS[d.getMonth()]} ${d.getDate()}`;
    return opts.year || d.getFullYear() !== new Date().getFullYear() ? `${s}, ${d.getFullYear()}` : s;
  };
  const ago = ms => {
    const s = Math.round((Date.now() - ms) / 1000);
    if (s < 60) return 'just now'; if (s < 3600) return `${Math.floor(s / 60)} min ago`;
    if (s < 86400) return `${Math.floor(s / 3600)} h ago`; return `${Math.floor(s / 86400)} days ago`;
  };
  const dur = (ms, opts = {}) => {
    const neg = ms < 0; ms = Math.abs(ms);
    const t = opts.ceil ? Math.ceil(ms / 1000) : Math.floor(ms / 1000);
    const h = Math.floor(t / 3600), m = Math.floor(t % 3600 / 60), s = t % 60;
    const body = h ? `${h}:${pad2(m)}:${pad2(s)}` : `${pad2(m)}:${pad2(s)}`;
    return (neg ? '−' : '') + body + (opts.cs ? `.${pad2(Math.floor(ms % 1000 / 10))}` : '');
  };
  const words = ms => {
    const m = Math.round(ms / 60000);
    if (m < 1) return 'less than a minute'; if (m < 60) return `${m} min`;
    const h = Math.floor(m / 60), r = m % 60;
    if (h < 24) return r ? `${h} h ${r} min` : `${h} h`;
    const d = Math.floor(h / 24), rh = h % 24;
    return rh ? `${d} d ${rh} h` : `${d} d`;
  };

  const H = {
    T, invoke, $, $$, esc, pad2, icon, dialog, closeDialog, dialogOpen, ask, toast, spin, setSpin, spinNames, focusables, move, choose, back,
    openOsk, closeOsk, fmtTime, fmtDate, ymd, parseYmd, ago, dur, words, DAYS, MONTHS,
    pad: false, buttons: {}, onBack: null, onMove: null, onKey: null, onPadDir: null, onDialogBack: null, onEscapeTyping: null, clock24,
  };
  return H;
})();
