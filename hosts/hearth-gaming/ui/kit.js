// What Controller Lab, Saves and Mods share: talking to the Rust side, moving around with the D-pad / stick /
// arrow keys (A chooses, B goes back, LB/RB switch tabs), dialogs, a folder and file browser, the on-screen
// keyboard, and toasts. Opened in a plain browser (no Tauri), each page uses its own sample data (KIT.demo).
'use strict';
const KIT = (() => {
  const T = window.__TAURI__;
  const K = {
    live: !!T, pad: false, demo: null,
    capture: null,   // a function: while set, the controller doesn't move around (Controller Lab's test); it gets every frame's pads
    onBack: null, onTab: null, onX: null, onY: null, onStart: null,
  };
  K.invoke = (cmd, args = {}) => T ? T.core.invoke(cmd, args) : (K.demo ? Promise.resolve().then(() => K.demo(cmd, args)) : Promise.reject('preview'));
  K.listen = (ev, fn) => { if (T) T.event.listen(ev, fn); };
  K.$ = (s, el = document) => el.querySelector(s);
  K.esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  K.human = b => { if (b == null) return ''; const u = ['bytes', 'KB', 'MB', 'GB', 'TB']; let v = b, i = 0; while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
    return i === 0 ? `${b} bytes` : `${v >= 100 ? v.toFixed(0) : v.toFixed(1).replace(/\.0$/, '')} ${u[i]}`; };
  K.when = ms => { if (!ms) return 'never'; const d = new Date(ms); const now = new Date(); const same = d.toDateString() === now.toDateString();
    const y = new Date(now - 864e5).toDateString() === d.toDateString();
    const t = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    return same ? `today ${t}` : y ? `yesterday ${t}` : `${d.toLocaleDateString([], { day: 'numeric', month: 'short', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' })} ${t}`; };
  K.err = e => (e && e.message) || String(e || 'Something went wrong.');

  // ---------------------------------------------------------------- toasts
  let toastTimer = 0;
  K.toast = text => {
    let el = K.$('#kit-toast');
    if (!el) { el = document.createElement('div'); el.id = 'kit-toast'; el.className = 'toast'; document.body.appendChild(el); }
    el.textContent = text; el.hidden = false;
    clearTimeout(toastTimer); toastTimer = setTimeout(() => { el.hidden = true; }, 3800);
  };

  // ---------------------------------------------------------------- dialogs
  const layer = () => { let l = K.$('#kit-layer'); if (!l) { l = document.createElement('div'); l.id = 'kit-layer'; document.body.appendChild(l); } return l; };
  K.dialogOpen = () => !!layer().firstChild;
  let backFocus = null, dlgBack = null;
  K.dialog = (html, { locked = false, wide = false, onBack = null } = {}) => {
    if (!K.dialogOpen()) backFocus = document.activeElement;
    layer().innerHTML = `<div class="veil"><div class="dlg ${wide ? 'wide' : ''}" role="dialog">${html}</div></div>`;
    layer().firstChild.dataset.locked = locked ? '1' : '';
    dlgBack = onBack;
    const first = K.$('#kit-layer [data-autofocus]') || K.$('#kit-layer .f');
    first?.focus();
    return layer().firstChild;
  };
  K.close = () => { if (K.sharedOsk() && HearthOSK.isOpen()) HearthOSK.close(); layer().innerHTML = ''; dlgBack = null; oskTarget = null; const b = backFocus; backFocus = null; setTimeout(() => { if (b && document.contains(b)) b.focus(); else K.focusFirst(); }, 0); };
  K.progress = (title, text = 'Starting') => K.dialog(`<h2>${K.esc(title)}</h2><div class="prog spin" id="kit-pg"><i></i></div><div class="msg" id="kit-pgt">${K.esc(text)}</div>`, { locked: true });
  K.setProgress = (text, pct) => { const t = K.$('#kit-pgt'), p = K.$('#kit-pg'); if (t && text) t.textContent = text; if (p && pct != null) { p.classList.remove('spin'); p.firstChild.style.width = `${Math.round(pct * 100)}%`; } };
  K.alert = (title, text, ok = true) => new Promise(res => {
    K.dialog(`<h2 class="${ok ? '' : 'bad'}">${K.esc(title)}</h2>${text ? `<p>${text}</p>` : ''}<div class="row"><button class="btn primary f" data-kit="ok" data-autofocus>OK</button></div>`);
    layer().onclick = e => { if (e.target.closest('[data-kit="ok"]')) { layer().onclick = null; K.close(); res(); } };
  });
  K.confirm = ({ title, text = '', ok = 'OK', danger = false }) => new Promise(res => {
    K.dialog(`<h2>${K.esc(title)}</h2>${text ? `<div>${text}</div>` : ''}<div class="row"><button class="btn f" data-kit="no" ${danger ? 'data-autofocus' : ''}>Cancel</button><button class="btn primary f ${danger ? 'danger' : ''}" data-kit="yes" ${danger ? '' : 'data-autofocus'}>${K.esc(ok)}</button></div>`,
      { onBack: () => { layer().onclick = null; K.close(); res(false); } });
    layer().onclick = e => { const b = e.target.closest('[data-kit]'); if (!b) return; layer().onclick = null; K.close(); res(b.dataset.kit === 'yes'); };
  });
  /** A choice from a list: [{id, label, sub}] → the id, or null. */
  K.choose = (title, items, text = '') => new Promise(res => {
    K.dialog(`<h2>${K.esc(title)}</h2>${text ? `<p class="msg">${text}</p>` : ''}<div class="list">${items.map((it, i) => `<button class="item f" data-kit-i="${i}" ${i === 0 ? 'data-autofocus' : ''} style="grid-template-columns:1fr auto"><span class="n">${K.esc(it.label)}</span><span class="end">${K.esc(it.end || '')}</span>${it.sub ? `<span class="m" style="grid-column:1/-1">${K.esc(it.sub)}</span>` : ''}</button>`).join('')}</div><div class="row"><button class="btn f" data-kit="no">Cancel</button></div>`,
      { onBack: () => { layer().onclick = null; K.close(); res(null); } });
    layer().onclick = e => { const b = e.target.closest('[data-kit-i],[data-kit]'); if (!b) return; layer().onclick = null; K.close(); res(b.dataset.kitI != null ? items[+b.dataset.kitI].id : null); };
  });

  // ---------------------------------------------------------------- folder and file browser
  /** opts: {title, start, files (show files), archives (only archives), folder (a folder can be chosen), note} → path or null. */
  K.pickPath = opts => new Promise(async res => {
    const o = Object.assign({ title: 'Choose a folder', files: false, archives: false, folder: true }, opts);
    let places = [];
    try { places = await K.invoke('places'); } catch (e) { places = []; }
    let cur = o.start || places[0]?.path || '/';
    const done = v => { layer().onclick = null; K.close(); res(v); };
    const draw = async (focusKey) => {
      let d;
      try { d = await K.invoke('browse', { path: cur, files: o.files, archives: o.archives }); }
      catch (e) { K.toast(K.err(e)); if (cur !== places[0]?.path) { cur = places[0]?.path || '/'; return draw(); } d = { path: cur, parent: null, entries: [] }; }
      const rows = d.entries.map((x, i) => `<button class="item f" data-kit-p="${i}" style="grid-template-columns:auto 1fr auto"><span class="ico" style="grid-row:auto;width:1.9em;height:1.9em">${x.dir ? '📁' : x.archive ? '🗜' : '📄'}</span><span class="n">${K.esc(x.name)}</span><span class="end">${x.dir ? '›' : K.esc(K.human(x.size))}</span></button>`).join('');
      K.dialog(`<h2>${K.esc(o.title)}</h2>${o.note ? `<p class="msg small" style="margin-top:-.3em">${K.esc(o.note)}</p>` : ''}
        <div class="crumbs">${places.map((p, i) => `<button class="chip f ${cur === p.path || cur.startsWith(p.path + '/') ? 'on' : ''}" data-kit-place="${i}">${K.esc(p.label)}</button>`).join('')}</div>
        <div class="path" style="margin-bottom:.4em">${K.esc(d.path)}</div>
        <div class="picker">${d.parent ? `<button class="item f" data-kit-up style="grid-template-columns:auto 1fr"><span class="ico" style="grid-row:auto;width:1.9em;height:1.9em">↑</span><span class="n">Up one folder</span></button>` : ''}${rows || '<div class="empty">Nothing here.</div>'}</div>
        <div class="row"><button class="btn f" data-kit="cancel">Cancel</button>${o.folder ? `<button class="btn primary f" data-kit="here">Choose this folder</button>` : ''}</div>`, { wide: true, onBack: () => { if (d.parent) { cur = d.parent; draw(); } else done(null); } });
      const fk = focusKey && K.$(`#kit-layer [data-kit-p="${focusKey}"]`); if (fk) fk.focus();
      layer().onclick = e => {
        const b = e.target.closest('button'); if (!b) return;
        if (b.dataset.kit === 'cancel') return done(null);
        if (b.dataset.kit === 'here') return done(d.path);
        if (b.dataset.kitUp != null) { cur = d.parent; return draw(); }
        if (b.dataset.kitPlace != null) { cur = places[+b.dataset.kitPlace].path; return draw(); }
        if (b.dataset.kitP != null) { const x = d.entries[+b.dataset.kitP]; if (x.dir) { cur = x.path; draw(); } else done(x.path); }
      };
    };
    draw();
  });

  // ---------------------------------------------------------------- on-screen keyboard (typing with a controller)
  let oskTarget = null, lower = false;
  const OSK = 'QWERTYUIOP ASDFGHJKL- ZXCVBNM_. 1234567890'.split(' ');
  // Hearth's shared keyboard (ui/osk, docs/OSK.md) when it's there; the small one below is only a fallback.
  K.sharedOsk = () => typeof HearthOSK !== 'undefined' && HearthOSK.version >= 1;
  K.oskOpen = () => (K.sharedOsk() && HearthOSK.isOpen()) || !!oskTarget;
  K.openOsk = input => {
    if (K.sharedOsk()) {
      HearthOSK.open({ target: input, enterLabel: 'Done', pad: true, onEnter: () => { HearthOSK.close(); input.focus(); }, onClose: () => { if (document.contains(input)) input.focus(); } });
      return;
    }
    oskTarget = input;
    const rows = OSK.map(r => [...r].map(c => `<button class="f" data-osk-key="${c}">${lower ? c.toLowerCase() : c}</button>`).join('')).join('');
    input.insertAdjacentHTML('afterend', `<div class="osk" id="kit-osk">${rows}<button class="f wide" data-osk-key="space">Space</button><button class="f wide" data-osk-key="back">⌫</button><button class="f wide" data-osk-key="lower">abc</button><button class="f wide" data-osk-key="done">Done</button></div>`);
    K.$('#kit-osk .f').focus();
  };
  const closeOsk = () => { const t = oskTarget; K.$('#kit-osk')?.remove(); oskTarget = null; t?.focus(); };
  document.addEventListener('click', e => {
    const b = e.target.closest('[data-osk-key]'); if (!b || !oskTarget) return;
    const k = b.dataset.oskKey, t = oskTarget;
    if (k === 'done') return closeOsk();
    if (k === 'lower') { lower = !lower; document.querySelectorAll('#kit-osk [data-osk-key]').forEach(x => { if (x.dataset.oskKey.length === 1) x.textContent = lower ? x.dataset.oskKey.toLowerCase() : x.dataset.oskKey; }); return; }
    if (k === 'back') t.value = t.value.slice(0, -1);
    else if (t.value.length < (t.maxLength > 0 ? t.maxLength : 120)) t.value += k === 'space' ? ' ' : (lower ? k.toLowerCase() : k);
    t.dispatchEvent(new Event('input', { bubbles: true }));
  });

  // ---------------------------------------------------------------- moving around
  const focusables = () => {
    const root = layer().firstChild || document;
    return [...root.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null && !el.closest('#kit-layer:empty'));
  };
  K.focusFirst = sel => { const el = (sel && document.querySelector(sel)) || focusables()[0]; el?.focus(); return el; };
  K.move = dir => {
    const cur = document.activeElement;
    if (cur?.type === 'range' && (dir === 'left' || dir === 'right')) { const st = Number(cur.step) || 1; cur.value = Number(cur.value) + (dir === 'right' ? st : -st); cur.dispatchEvent(new Event('input', { bubbles: true })); cur.dispatchEvent(new Event('change', { bubbles: true })); return; }
    const all = focusables(); if (!all.length) return;
    if (!cur || !all.includes(cur)) { all[0].focus(); return; }
    const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
    let best = null, bestScore = Infinity;
    for (const el of all) {
      if (el === cur) continue;
      const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
      const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
      if (!ok) continue;
      const main = dir === 'left' || dir === 'right' ? Math.abs(bx - ax) : Math.abs(by - ay);
      const cross = dir === 'left' || dir === 'right' ? Math.abs(by - ay) : Math.abs(bx - ax);
      const score = main + cross * 2.2;
      if (score < bestScore) { bestScore = score; best = el; }
    }
    if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
  };
  K.choose_ = () => {
    const el = document.activeElement;
    if (!el || el === document.body) { focusables()[0]?.focus(); return; }
    if (el.dataset.osk != null && K.pad) { if (!K.oskOpen()) K.openOsk(el); return; }
    if (el.tagName === 'BUTTON' || el.tagName === 'A') el.click();
  };
  K.back = () => {
    if (oskTarget) { closeOsk(); return; }
    if (K.dialogOpen()) { const l = layer().firstChild; if (dlgBack) dlgBack(); else if (!l.dataset.locked) K.close(); return; }
    if (K.onBack) K.onBack();
  };
  document.addEventListener('keydown', e => {
    K.pad = false;
    if (K.capture && !K.dialogOpen()) { if (e.key === 'Escape') { e.preventDefault(); K.onBack?.(); } return; }
    const typing = e.target.tagName === 'INPUT' && e.target.type !== 'range';
    const k = e.key;
    if (k === 'Escape' || (k === 'Backspace' && !typing)) { e.preventDefault(); K.back(); return; }
    if (typing && (k === 'ArrowLeft' || k === 'ArrowRight')) return;
    const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
    if (dirs[k]) { e.preventDefault(); K.move(dirs[k]); return; }
    if (k === 'Enter' && typing) { e.preventDefault(); K.move('down'); return; }
    if (!typing && !K.dialogOpen()) {
      if (k === 'PageUp' || k === '[') { e.preventDefault(); K.onTab?.(-1); }
      if (k === 'PageDown' || k === ']') { e.preventDefault(); K.onTab?.(1); }
    }
  });

  // Controllers (the standard mapping): D-pad or left stick moves, A chooses, B back, X and Y the page's own, LB/RB tabs.
  const held = {}; let lastMove = 0;
  K.rawPads = () => (navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : []);
  function frame() {
    const pads = K.rawPads();
    const now = performance.now();
    if (K.capture && !K.dialogOpen()) { try { K.capture(pads, now); } catch (e) { console.error(e); } requestAnimationFrame(frame); return; }
    // The shared keyboard reads the controller itself while it's open.
    if (K.sharedOsk() && HearthOSK.isOpen()) { for (const k in held) held[k] = now; requestAnimationFrame(frame); return; }
    const st = { up: false, down: false, left: false, right: false, a: false, b: false, x: false, y: false, lb: false, rb: false, start: false };
    for (const g of pads) {
      const bt = i => g.buttons[i]?.pressed;
      const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
      st.up ||= bt(12) || ay < -.6; st.down ||= bt(13) || ay > .6; st.left ||= bt(14) || ax < -.6; st.right ||= bt(15) || ax > .6;
      st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.lb ||= bt(4); st.rb ||= bt(5); st.start ||= bt(9);
    }
    for (const [k, on] of Object.entries(st)) {
      if (!on) { held[k] = 0; continue; }
      const first = !held[k];
      const dir = ['up', 'down', 'left', 'right'].includes(k);
      const repeat = held[k] && now - held[k] > 380 && now - lastMove > 110;
      if (first) held[k] = now;
      if (!(first || (repeat && dir))) continue;
      K.pad = true; lastMove = now;
      if (k === 'a') K.choose_(); else if (k === 'b') K.back();
      else if (k === 'x') { if (!K.dialogOpen()) K.onX?.(); }
      else if (k === 'y') { if (!K.dialogOpen()) K.onY?.(); }
      else if (k === 'lb') { if (!K.dialogOpen()) K.onTab?.(-1); }
      else if (k === 'rb') { if (!K.dialogOpen()) K.onTab?.(1); }
      else if (k === 'start') { if (!K.dialogOpen()) K.onStart?.(); }
      else K.move(k);
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  /** Whether the controller is the last thing used (then focus follows choices into the details). */
  K.usingPad = () => K.pad;
  return K;
})();
