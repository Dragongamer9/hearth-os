// Hearth Tools: what every tool's page shares. Calls to the Rust side (H.call, H.job), the header, tabs and
// controller hints, dialogs, a password prompt with an on-screen keyboard, toasts, and moving around with the
// keyboard or a controller (D-pad/stick moves, A chooses, B goes back, X the page's second action, Y refreshes,
// LB/RB switch tabs). Opened in a plain browser (no Tauri), the page runs on its own sample data (H.demo).
'use strict';
const H = (() => {
  const T = window.__TAURI__;
  const boot = window.__HEARTH__ || {};
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const human = (b, digits = 1) => { if (b == null || isNaN(b)) return ''; const u = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']; let v = Number(b), i = 0; while (Math.abs(v) >= 1000 && i < u.length - 1) { v /= 1000; i++; } return (i ? v.toFixed(v >= 100 ? 0 : digits) : Math.round(v)) + ' ' + u[i]; };
  const when = ms => { if (!ms) return ''; const d = new Date(ms); return d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }); };
  const ago = ms => { if (!ms) return ''; const s = (Date.now() - ms) / 1000; if (s < 60) return 'just now'; if (s < 3600) return Math.round(s / 60) + ' min ago'; if (s < 86400) return Math.round(s / 3600) + ' h ago'; const d = Math.round(s / 86400); return d < 60 ? d + ' days ago' : Math.round(d / 30) + ' months ago'; };
  const icons = {
    hearth: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 2c1.5 3.2 5.5 5.6 5.5 10.2A5.5 5.5 0 0 1 12 18a5.5 5.5 0 0 1-5.5-5.8c0-2.4 1.3-3.9 2.4-5 .2 1.7 1 2.8 2 3.3C10.6 7.6 11.2 4.6 12 2zM5 20h14v2H5z"/></svg>',
  };

  // ---------------------------------------------------------------- calls
  const demo = { call: null, job: null };
  function call(method, args = {}) {
    if (T) return T.core.invoke('call', { method, args });
    if (demo.call) { try { return Promise.resolve(demo.call(method, args)); } catch (e) { return Promise.reject(String(e)); } }
    return Promise.reject('This is a preview: open the app to use it.');
  }
  const jobs = new Map();
  let listening = null;
  function job(name, args = {}, onStep = () => {}) {
    if (!T) {
      if (!demo.job) return Promise.reject('This is a preview.');
      return new Promise((res, rej) => { setTimeout(() => { try { res(demo.job(name, args, onStep)); } catch (e) { rej(String(e)); } }, 250); });
    }
    if (!listening) listening = T.event.listen('tools://job', ev => {
      const m = ev.payload, j = jobs.get(m.id);
      if (!j) return;
      if (m.kind === 'step') j.onStep(m.data);
      else { jobs.delete(m.id); m.ok ? j.res(m.result) : j.rej(m.error); }
    });
    let id = null;
    const p = listening.then(() => T.core.invoke('start', { job: name, args })).then(i => { id = i; return new Promise((res, rej) => jobs.set(i, { res, rej, onStep })); });
    p.cancel = () => { if (id != null) T.core.invoke('cancel', { id }); };
    return p;
  }

  // ---------------------------------------------------------------- the frame: header, tabs, footer
  const S = { tabs: [], tab: null, onTab: null, pad: false, back: null, alt: null, refresh: null };
  function shell({ title, sub = '', tabs = [], tab = null, onTab = null, hints = [] }) {
    document.title = title;
    document.body.innerHTML = `<div class="app">
      <header class="top"><span class="logo">${icons.hearth}</span><h1>${esc(title)}</h1><span class="sub" id="sub">${esc(sub)}</span><span class="spacer"></span><span id="actions" class="row"></span></header>
      <nav class="tabs" id="tabs"></nav>
      <main class="body" id="main"></main>
      <footer class="hints" id="hints"></footer></div><div id="layer"></div>`;
    S.tabs = tabs; S.onTab = onTab; S.tab = tab || (tabs[0] && tabs[0].id);
    drawTabs();
    setHints(hints);
  }
  function drawTabs() {
    const el = $('#tabs'); if (!el) return;
    el.innerHTML = S.tabs.map(t => `<button class="tab f${t.id === S.tab ? ' on' : ''}" data-tab="${esc(t.id)}">${esc(t.label)}</button>`).join('');
    $$('[data-tab]', el).forEach(b => b.onclick = () => setTab(b.dataset.tab));
  }
  function setTab(id, focus = false) {
    if (!S.tabs.some(t => t.id === id)) return;
    S.tab = id; drawTabs();
    if (S.onTab) S.onTab(id);
    if (focus) setTimeout(() => ($(`[data-tab="${CSS.escape(id)}"]`) || firstFocus())?.focus(), 0);
  }
  function setHints(list) {
    const base = [['A', 'Choose'], ['B', 'Back']];
    const all = base.concat(list || []);
    if (S.tabs.length > 1) all.push(['LB RB', 'Switch tabs']);
    const el = $('#hints'); if (el) el.innerHTML = all.map(([k, v]) => `<span><b>${esc(k)}</b>${esc(v)}</span>`).join('');
  }
  const sub = t => { const el = $('#sub'); if (el) el.textContent = t || ''; };
  const actions = html => { const el = $('#actions'); if (el) el.innerHTML = html; return el; };

  // ---------------------------------------------------------------- toasts and dialogs
  let toastTimer = 0;
  function toast(text, bad = false) {
    $('.toast')?.remove();
    const t = document.createElement('div'); t.className = 'toast' + (bad ? ' bad' : ''); t.textContent = text; document.body.appendChild(t);
    clearTimeout(toastTimer); toastTimer = setTimeout(() => t.remove(), bad ? 6000 : 3200);
  }
  const fail = e => toast(String(e && e.message || e), true);
  let dlgDone = null, lastFocus = null;
  // A dialog: {title, html, buttons: [{label, value, primary, danger}]} -> the chosen value (null for back).
  function dialog({ title, html = '', buttons = [{ label: 'OK', value: true, primary: true }], onOpen = null, wide = false }) {
    closeDialog(null);
    lastFocus = document.activeElement;
    return new Promise(res => {
      dlgDone = res;
      $('#layer').innerHTML = `<div class="veil"><div class="dlg" role="dialog" style="${wide ? 'width:min(60em,100%)' : ''}"><h2>${esc(title)}</h2><div class="dlg-body">${html}</div>
        <div class="row end">${buttons.map((b, i) => `<button class="btn f${b.primary ? ' primary' : ''}${b.danger ? ' danger' : ''}" data-dbtn="${i}">${esc(b.label)}</button>`).join('')}</div></div></div>`;
      $$('[data-dbtn]').forEach(b => b.onclick = () => closeDialog(buttons[+b.dataset.dbtn].value));
      if (onOpen) onOpen($('.dlg'));
      setTimeout(() => ($('.dlg input') || $('.dlg .btn.primary') || $('.dlg .f'))?.focus(), 0);
    });
  }
  function closeDialog(v) {
    if (!$('#layer') || !$('#layer').firstChild) return;
    $('#layer').innerHTML = ''; oskTarget = null;
    const d = dlgDone; dlgDone = null;
    if (lastFocus && document.contains(lastFocus)) setTimeout(() => lastFocus.focus(), 0);
    if (d) d(v);
  }
  const confirmDlg = (title, html, label = 'OK', danger = false) => dialog({ title, html, buttons: [{ label: 'Cancel', value: false }, { label, value: true, primary: true, danger }] });
  // Asks for your account password (to show a saved password). The page clears it from memory after use.
  async function askPassword(why) {
    let pw = null;
    const v = await dialog({ title: 'Your password', html: `<p class="muted">${esc(why)}</p><input class="inp f" type="password" id="pw" data-osk="1" autocomplete="off" placeholder="Your account password">`,
      buttons: [{ label: 'Cancel', value: false }, { label: 'Show', value: true, primary: true }],
      onOpen: d => { const i = $('#pw', d); i.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); pw = i.value; closeDialog(true); } }); i.addEventListener('input', () => { pw = i.value; }); } });
    if (!v) return null;
    return pw || null;
  }
  // A text prompt (the controller gets the on-screen keyboard).
  async function prompt(title, label, value = '', placeholder = '') {
    let text = value;
    const v = await dialog({ title, html: `<p class="muted">${esc(label)}</p><input class="inp f" id="pr" data-osk="1" value="${esc(value)}" placeholder="${esc(placeholder)}">`,
      buttons: [{ label: 'Cancel', value: false }, { label: 'OK', value: true, primary: true }],
      onOpen: d => { const i = $('#pr', d); i.addEventListener('input', () => { text = i.value; }); i.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); text = i.value; closeDialog(true); } }); } });
    return v ? text : null;
  }

  // On-screen keyboard (controller): opens under the field when A is pressed on it.
  let oskTarget = null, shift = false;
  function openOsk(input) {
    oskTarget = input;
    const host = input.closest('.dlg') || input.parentElement;
    $('.osk')?.remove();
    const rows = ['1234567890', 'qwertyuiop', 'asdfghjkl-', 'zxcvbnm.@_'];
    const k = document.createElement('div'); k.className = 'osk';
    k.innerHTML = rows.map(r => [...r].map(c => `<button class="f" data-k="${esc(c)}">${esc(shift ? c.toUpperCase() : c)}</button>`).join('')).join('')
      + '<button class="f wide" data-k="shift">⇧ Shift</button><button class="f wider" data-k=" ">Space</button><button class="f wide" data-k="back">⌫</button><button class="f wide" data-k="done">Done</button>';
    host.appendChild(k);
    $$('[data-k]', k).forEach(b => b.onclick = () => {
      const c = b.dataset.k;
      if (c === 'done') { k.remove(); const t = oskTarget; oskTarget = null; t.focus(); t.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return; }
      if (c === 'shift') { shift = !shift; const keep = b.dataset.k; openOsk(input); $(`.osk [data-k="${keep}"]`)?.focus(); return; }
      if (c === 'back') oskTarget.value = oskTarget.value.slice(0, -1); else oskTarget.value += shift ? c.toUpperCase() : c;
      oskTarget.dispatchEvent(new Event('input', { bubbles: true }));
    });
    setTimeout(() => $('[data-k="q"]', k)?.focus(), 0);
  }

  // ---------------------------------------------------------------- moving around
  const visible = el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden'; };
  function focusables() {
    const root = ($('#layer') && $('#layer').firstChild) || document;
    return $$('.f, input.inp, select.inp, textarea.inp', root).filter(el => !el.disabled && visible(el));
  }
  const firstFocus = () => focusables()[0];
  function move(dir) {
    const all = focusables();
    const cur = document.activeElement;
    if (!cur || cur === document.body || !all.includes(cur)) { all[0]?.focus(); return; }
    const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of all) {
      if (el === cur) continue;
      const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
      const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
      if (!ok) continue;
      const dx = bx - ax, dy = by - ay;
      const main = dir === 'left' || dir === 'right' ? Math.abs(dx) : Math.abs(dy);
      const cross = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
      const score = main + cross * 2.2;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  }
  function choose() {
    const el = document.activeElement;
    if (!el || el === document.body) { firstFocus()?.focus(); return; }
    if (el.dataset.osk && S.pad) { if (!oskTarget) openOsk(el); return; }
    if (el.tagName === 'SELECT') { const o = el.options; el.selectedIndex = (el.selectedIndex + 1) % o.length; el.dispatchEvent(new Event('change', { bubbles: true })); return; }
    el.click();
  }
  function back() {
    if ($('.osk')) { const t = oskTarget; $('.osk').remove(); oskTarget = null; t?.focus(); return; }
    if ($('#layer') && $('#layer').firstChild) { closeDialog(null); return; }
    if (S.back && S.back()) return;
  }
  document.addEventListener('keydown', e => {
    S.pad = false;
    const typing = (e.target.tagName === 'INPUT' && !['range', 'checkbox'].includes(e.target.type)) || e.target.tagName === 'TEXTAREA';
    const k = e.key;
    if (k === 'Escape' || (k === 'Backspace' && !typing && !e.target.isContentEditable)) { e.preventDefault(); back(); return; }
    if (typing && (k === 'ArrowLeft' || k === 'ArrowRight' || k === 'Home' || k === 'End')) return;
    if (e.target.tagName === 'SELECT' && (k === 'ArrowUp' || k === 'ArrowDown')) return;
    const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    if (dirs[k]) { e.preventDefault(); move(dirs[k]); return; }
    if (k === 'F5' || (k === 'r' && e.ctrlKey)) { e.preventDefault(); S.refresh && S.refresh(); return; }
    if (k === 'PageDown' && e.ctrlKey || (k === 'Tab' && e.ctrlKey && !e.shiftKey)) { e.preventDefault(); cycleTab(1); return; }
    if (k === 'PageUp' && e.ctrlKey || (k === 'Tab' && e.ctrlKey && e.shiftKey)) { e.preventDefault(); cycleTab(-1); return; }
    if (k === 'Enter' && !typing && e.target.tagName !== 'BUTTON' && e.target.classList?.contains('f')) { e.preventDefault(); choose(); }
  });
  function cycleTab(d) {
    if (S.tabs.length < 2 || ($('#layer') && $('#layer').firstChild)) return;
    const i = S.tabs.findIndex(t => t.id === S.tab);
    setTab(S.tabs[(i + d + S.tabs.length) % S.tabs.length].id, true);
  }
  // Controllers (the standard mapping).
  const held = {}; let lastMove = 0;
  function pad() {
    const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
    const now = performance.now();
    const st = { up: false, down: false, left: false, right: false, a: false, b: false, x: false, y: false, lb: false, rb: false };
    for (const g of pads) {
      const bt = i => g.buttons[i]?.pressed; const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
      st.up ||= bt(12) || ay < -.6; st.down ||= bt(13) || ay > .6; st.left ||= bt(14) || ax < -.6; st.right ||= bt(15) || ax > .6;
      st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.lb ||= bt(4); st.rb ||= bt(5);
    }
    for (const [k, on] of Object.entries(st)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const repeat = held[k] && now - held[k] > 380 && now - lastMove > 110 && ['up', 'down', 'left', 'right'].includes(k);
      if (first) held[k] = now;
      if (!first && !repeat) continue;
      S.pad = true; lastMove = now;
      if (k === 'a') choose(); else if (k === 'b') back(); else if (k === 'x') S.alt && S.alt(); else if (k === 'y') S.refresh && S.refresh();
      else if (k === 'lb') cycleTab(-1); else if (k === 'rb') cycleTab(1); else move(k);
    }
    requestAnimationFrame(pad);
  }
  requestAnimationFrame(pad);

  async function copy(text) {
    try { await navigator.clipboard.writeText(text); toast('Copied'); return; } catch (e) { /* older WebKit: below */ }
    const t = document.createElement('textarea'); t.value = text; t.style.position = 'fixed'; t.style.opacity = '0'; document.body.appendChild(t); t.select();
    try { document.execCommand('copy'); toast('Copied'); } catch (e) { toast('Couldn\'t copy', true); }
    t.remove();
  }

  return { T, boot, $, $$, esc, human, when, ago, call, job, demo, shell, sub, actions, setTab, setHints, S, toast, fail, dialog, closeDialog,
    confirm: confirmDlg, askPassword, prompt, focusables, firstFocus, copy, icons, preview: !T };
})();
