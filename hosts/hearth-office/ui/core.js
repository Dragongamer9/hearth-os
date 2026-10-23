// Hearth Office: what the three apps share. The window (menus, toolbar, status bar, start screen, dialogs),
// files (open, save, save as, recent, recovery copies, the unsaved-changes guard), printing and PDF export,
// keyboard shortcuts and the controller. Each app (write.js, sheets.js, slides.js) registers itself with
// H.register() and does its own document work.
// Outside the app (a plain browser, the tests) there is no Rust side: a stand-in (H.mock) keeps files in memory.
'use strict';

const H = window.H = {
  apps: {}, app: null, info: {}, settings: {}, fonts: [],
  doc: { path: null, name: 'Untitled', ext: null, dirty: false, rid: null, used: false },
};

// ------------------------------------------------------------------ the Rust side (or the stand-in)
(function bridge() {
  const T = window.__TAURI__;
  if (T && T.core) {
    H.invoke = (cmd, args) => T.core.invoke(cmd, args || {});
    H.listen = (name, fn) => T.event.listen(name, (e) => fn(e.payload));
    H.native = true;
    return;
  }
  // The stand-in: files live in H.mock.files (path -> base64). Tests set pickOpen/pickSave to answer the pickers.
  const params = new URLSearchParams(location.search);
  const listeners = {};
  const M = H.mock = {
    files: {}, recent: [], recovery: {}, settings: {}, dirty: false, quit: false, printed: 0, converted: null,
    pickOpen: null, pickSave: null, opened: [], log: [],
    emit(name, payload) { (listeners[name] || []).forEach((f) => f(payload)); },
  };
  const app = params.get('app') || 'write';
  const conv = params.get('convert');
  const cmds = {
    init: () => ({
      app, platform: 'linux', version: 'dev', home: '/home/you', documents: '/home/you/Documents', settings: M.settings,
      launch: { app, file: params.get('file'), convert: conv ? conv.split('|') : null, fullscreen: false, present: params.has('present') },
      recent: M.recent.filter((r) => r.app === app), recovery: Object.values(M.recovery).map((r) => r.meta).filter((m) => m.app === app),
    }),
    read_file: ({ path }) => { if (!(path in M.files)) throw `Couldn't open ${path}: not found`; return { data: M.files[path], size: 0, modified: 0 }; },
    write_file: ({ path, data }) => { M.files[path] = data; },
    pick_open: () => M.pickOpen, pick_save: ({ name }) => M.pickSave || ('/home/you/Documents/' + name),
    recent_add: ({ path }) => { M.recent = [{ path, name: path.split('/').pop(), app, exists: true }, ...M.recent.filter((r) => r.path !== path)]; return M.recent; },
    recent_remove: ({ path }) => { M.recent = M.recent.filter((r) => r.path !== path); return M.recent; },
    recent_list: () => M.recent,
    recovery_save: ({ id, meta, body }) => { M.recovery[id] = { meta: { ...meta, id, time: Date.now() / 1000 | 0 }, body }; },
    recovery_clear: ({ id }) => { delete M.recovery[id]; },
    recovery_load: ({ id }) => M.recovery[id].body,
    settings_save: ({ value }) => { M.settings = value; },
    set_dirty: ({ dirty }) => { M.dirty = dirty; }, set_title: ({ title }) => { document.title = title; },
    fullscreen: () => {}, open_url: ({ url }) => { M.opened.push(url); }, open_window: (a) => { M.opened.push(a); },
    fonts: () => [], print: () => { M.printed++; },
    export_pdf: ({ path }) => { setTimeout(() => M.emit('office://pdf', { ok: true, path }), 50); return 'started'; },
    convert_done: (a) => { M.converted = a; }, log: ({ message }) => { M.log.push(message); }, quit: () => { M.quit = true; },
  };
  H.invoke = async (cmd, args) => { if (!cmds[cmd]) throw 'no command ' + cmd; return cmds[cmd](args || {}); };
  H.listen = (name, fn) => { (listeners[name] = listeners[name] || []).push(fn); };
  H.native = false;
})();

// ------------------------------------------------------------------ small helpers
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
H.$ = $; H.$$ = $$;
H.esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
H.el = (tag, attrs, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (k === 'class') e.className = v;
    else if (k === 'html') e.innerHTML = v;
    else if (k === 'text') e.textContent = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k === 'style' && typeof v === 'object') Object.assign(e.style, v);
    else e.setAttribute(k, v === true ? '' : v);
  }
  for (const k of kids.flat()) if (k != null && k !== false) e.append(k.nodeType ? k : document.createTextNode(k));
  return e;
};
H.extOf = (p) => { const m = /\.([^./\\]+)$/.exec(p || ''); return m ? m[1].toLowerCase() : ''; };
H.baseName = (p) => (p || '').split(/[\\/]/).pop();
H.stem = (p) => H.baseName(p).replace(/\.[^.]+$/, '');
H.dirOf = (p) => { const i = Math.max((p || '').lastIndexOf('/'), (p || '').lastIndexOf('\\')); return i > 0 ? p.slice(0, i) : null; };
H.debounce = (fn, ms) => { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; };
H.sleep = (ms) => new Promise((r) => setTimeout(r, ms));
H.uid = () => Math.random().toString(36).slice(2, 10) + Date.now().toString(36).slice(-4);

H.b64ToBytes = (b64) => { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u; };
H.bytesToB64 = (bytes) => {
  const u = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let s = ''; const CH = 0x8000;
  for (let i = 0; i < u.length; i += CH) s += String.fromCharCode.apply(null, u.subarray(i, i + CH));
  return btoa(s);
};
H.textToBytes = (t) => new TextEncoder().encode(t);
H.bytesToText = (b) => {
  const u = b instanceof Uint8Array ? b : new Uint8Array(b);
  if (u[0] === 0xff && u[1] === 0xfe) return new TextDecoder('utf-16le').decode(u.subarray(2));
  if (u[0] === 0xfe && u[1] === 0xff) return new TextDecoder('utf-16be').decode(u.subarray(2));
  try { return new TextDecoder('utf-8', { fatal: true }).decode(u).replace(/^﻿/, ''); }
  catch { return new TextDecoder('windows-1252').decode(u); }
};
H.mimeOf = (ext) => ({ png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml', bmp: 'image/bmp', emf: 'image/emf', wmf: 'image/wmf', tif: 'image/tiff', tiff: 'image/tiff' }[ext] || 'application/octet-stream');
H.dataUrl = (bytes, mime) => `data:${mime};base64,${H.bytesToB64(bytes)}`;
H.dataUrlBytes = (url) => { const m = /^data:([^;,]+)?(;base64)?,(.*)$/s.exec(url || ''); if (!m) return null; return { mime: m[1] || 'application/octet-stream', bytes: m[2] ? H.b64ToBytes(m[3]) : H.textToBytes(decodeURIComponent(m[3])) }; };
// An image as PNG (for formats that can't hold SVG or WebP), with its natural size.
H.toPng = (src) => new Promise((res) => {
  const img = new Image();
  img.onload = () => {
    const w = img.naturalWidth || 300, h = img.naturalHeight || 150;
    const c = document.createElement('canvas'); c.width = w; c.height = h;
    c.getContext('2d').drawImage(img, 0, 0, w, h);
    res({ url: c.toDataURL('image/png'), w, h });
  };
  img.onerror = () => res(null);
  img.src = src;
});
H.imageSize = (src) => new Promise((res) => { const i = new Image(); i.onload = () => res({ w: i.naturalWidth, h: i.naturalHeight }); i.onerror = () => res({ w: 0, h: 0 }); i.src = src; });

// Colours: any CSS colour to RRGGBB (no #), or null.
H.hex = (c) => {
  if (!c) return null;
  c = String(c).trim();
  let m = /^#?([0-9a-f]{6})$/i.exec(c); if (m) return m[1].toUpperCase();
  m = /^#?([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(c); if (m) return (m[1] + m[1] + m[2] + m[2] + m[3] + m[3]).toUpperCase();
  m = /^rgba?\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)(?:[\s,/]+([\d.]+%?))?\s*\)$/i.exec(c);
  if (m) { if (m[4] !== undefined && parseFloat(m[4]) === 0) return null; return [m[1], m[2], m[3]].map((x) => (+x).toString(16).padStart(2, '0')).join('').toUpperCase(); }
  if (c === 'transparent' || c === 'inherit' || c === 'initial' || c === 'currentcolor') return null;
  const ctx = H._cctx || (H._cctx = document.createElement('canvas').getContext('2d'));
  ctx.fillStyle = '#010203'; ctx.fillStyle = c;
  const v = ctx.fillStyle;
  if (v === '#010203' && c.toLowerCase() !== '#010203') return null;
  return H.hex(v);
};

// Scripts are loaded when an app first needs them (the big libraries aren't needed at start).
const loaded = {};
H.need = (...files) => Promise.all(files.map((f) => loaded[f] || (loaded[f] = new Promise((res, rej) => {
  const s = document.createElement('script');
  s.src = f; s.onload = res; s.onerror = () => rej(new Error('Couldn\'t load ' + f));
  document.head.append(s);
}))));

// HTML that came from a file or the clipboard: no scripts, no frames, no forms, no event handlers.
H.clean = (html) => window.DOMPurify ? DOMPurify.sanitize(html, {
  FORBID_TAGS: ['style', 'script', 'iframe', 'frame', 'object', 'embed', 'form', 'input', 'button', 'select', 'textarea', 'link', 'meta', 'base', 'svg', 'math'],
  FORBID_ATTR: ['id', 'contenteditable', 'srcset'],
  ALLOW_DATA_ATTR: false,
  ADD_DATA_URI_TAGS: ['img'],
  ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel):|data:image\/(?:png|jpe?g|gif|webp|bmp|svg\+xml);|#|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i,
}) : '';

// ------------------------------------------------------------------ toast, dialogs, menus
H.toast = (msg, ms = 2600) => {
  const t = $('#toast'); t.textContent = msg; t.classList.add('show');
  clearTimeout(H._toastT); H._toastT = setTimeout(() => t.classList.remove('show'), ms);
};

// A dialog. body: HTML or an element. buttons: [{label, value, primary, danger}]. Resolves with the button's
// value (or null for Escape / the close button). `init(box)` runs once it's on screen; `check(value, box)` can
// refuse a button by returning a message.
H.dialog = (o) => new Promise((resolve) => {
  const prevFocus = document.activeElement;
  const box = H.el('div', { class: 'dlg' + (o.wide ? ' wide' : ''), role: 'dialog', 'aria-modal': 'true' });
  const head = H.el('div', { class: 'dlg-h' }, H.el('h3', { text: o.title || '' }), H.el('button', { class: 'ibtn f', 'data-f': true, title: 'Close', html: '<svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg>', onclick: () => done(null) }));
  const body = H.el('div', { class: 'dlg-b' });
  if (typeof o.body === 'string') body.innerHTML = o.body; else if (o.body) body.append(o.body);
  const foot = H.el('div', { class: 'dlg-f' });
  for (const b of o.buttons || [{ label: 'OK', value: true, primary: true }]) {
    foot.append(H.el('button', { class: 'btn f' + (b.primary ? ' primary' : '') + (b.danger ? ' danger' : ''), 'data-f': true, 'data-v': String(b.value), text: b.label, onclick: () => press(b.value) }));
  }
  box.append(head, body, foot);
  const shade = H.el('div', { class: 'shade' }, box);
  shade.addEventListener('mousedown', (e) => { if (e.target === shade && !o.modal) done(null); });
  const press = (v) => {
    if (o.check && v !== null && v !== false) { const why = o.check(v, box); if (why) { H.toast(why); return; } }
    done(v);
  };
  function done(v) {
    shade.remove(); H._dialogs = H._dialogs.filter((d) => d !== shade);
    if (prevFocus && prevFocus.focus && document.contains(prevFocus)) prevFocus.focus();
    resolve(o.result && v !== null && v !== false ? o.result(v, box) : v);
  }
  shade.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); done(null); }
    else if (e.key === 'Enter' && !e.shiftKey && e.target.tagName !== 'TEXTAREA' && e.target.tagName !== 'BUTTON' && !e.target.isContentEditable) {
      const p = (o.buttons || [{ value: true, primary: true }]).find((b) => b.primary);
      if (p) { e.preventDefault(); press(p.value); }
    }
  });
  H._dialogs = (H._dialogs || []).concat(shade);
  $('#layer').append(shade);
  if (o.init) o.init(box);
  const first = box.querySelector('[autofocus]') || box.querySelector('input,select,textarea') || foot.querySelector('.primary') || foot.querySelector('button');
  setTimeout(() => first && first.focus(), 0);
});
H.alert = (title, text) => H.dialog({ title, body: H.el('p', { text }), buttons: [{ label: 'OK', value: true, primary: true }] });
H.confirm = (title, text, yes = 'OK', danger = false) => H.dialog({ title, body: H.el('p', { text }), buttons: [{ label: 'Cancel', value: false }, { label: yes, value: true, primary: true, danger }] });
H.prompt = async (title, label, value = '', opts = {}) => {
  const id = 'p' + H.uid();
  const v = await H.dialog({
    title, body: `<label class="fld"><span>${H.esc(label)}</span><input id="${id}" type="${opts.type || 'text'}" value="${H.esc(value)}" ${opts.placeholder ? `placeholder="${H.esc(opts.placeholder)}"` : ''}></label>`,
    buttons: [{ label: 'Cancel', value: null }, { label: opts.ok || 'OK', value: true, primary: true }],
    init: (b) => { const i = b.querySelector('input'); setTimeout(() => { i.focus(); i.select(); }, 0); },
    result: (_, b) => b.querySelector('input').value,
  });
  return v;
};
// A form from fields: [{id, label, type: text|number|select|check|color|textarea, value, options:[[v,label]]}].
H.form = (title, fields, ok = 'OK', extra = {}) => H.dialog({
  title, wide: extra.wide,
  body: H.el('div', { class: 'form' }, fields.map((f) => {
    if (f.type === 'heading') return H.el('h4', { text: f.label });
    if (f.type === 'note') return H.el('p', { class: 'note', text: f.label });
    let input;
    if (f.type === 'select') { input = H.el('select', { 'data-id': f.id }, f.options.map(([v, l]) => H.el('option', { value: v, selected: String(v) === String(f.value), text: l }))); }
    else if (f.type === 'check') { input = H.el('input', { type: 'checkbox', 'data-id': f.id, checked: !!f.value }); return H.el('label', { class: 'chk' }, input, H.el('span', { text: f.label })); }
    else if (f.type === 'textarea') { input = H.el('textarea', { 'data-id': f.id, rows: f.rows || 3 }); input.value = f.value ?? ''; }
    else { input = H.el('input', { type: f.type || 'text', 'data-id': f.id, value: f.value ?? '', min: f.min, max: f.max, step: f.step, placeholder: f.placeholder }); }
    return H.el('label', { class: 'fld' + (f.half ? ' half' : '') }, H.el('span', { text: f.label }), input);
  })),
  buttons: [{ label: 'Cancel', value: null }, { label: ok, value: true, primary: true }],
  check: extra.check,
  result: (_, box) => {
    const out = {};
    $$('[data-id]', box).forEach((i) => { out[i.dataset.id] = i.type === 'checkbox' ? i.checked : (i.type === 'number' ? parseFloat(i.value) : i.value); });
    return out;
  },
});

// Menus: [{label, key, run, sub, disabled(), checked(), sep}]. The same list is used for the menu bar, the
// right-click menus and the keyboard shortcuts.
H.closeMenus = () => { $$('.menu').forEach((m) => m.remove()); $$('#menubar .mb.open').forEach((b) => b.classList.remove('open')); H._menuOpen = null; };
H.showMenu = (items, x, y, opts = {}) => {
  if (!opts.keep) H.closeMenus();
  const m = H.el('div', { class: 'menu', role: 'menu' });
  const list = typeof items === 'function' ? items() : items;
  for (const it of list) {
    if (!it) continue;
    if (it.sep) { m.append(H.el('div', { class: 'msep' })); continue; }
    if (it.heading) { m.append(H.el('div', { class: 'mhead', text: it.heading })); continue; }
    const dis = it.disabled && it.disabled();
    const chk = it.checked && it.checked();
    const b = H.el('button', { class: 'mi' + (dis ? ' dis' : '') + (chk ? ' on' : ''), role: 'menuitem', tabindex: -1, disabled: dis },
      H.el('span', { class: 'mc', text: chk ? '✓' : '' }), H.el('span', { class: 'ml', text: it.label }),
      H.el('span', { class: 'mk', text: it.sub ? '›' : (it.key || '') }));
    if (it.style) b.querySelector('.ml').setAttribute('style', it.style);
    if (it.sub) {
      const openSub = () => {
        $$('.menu', $('#layer')).filter((x) => x._level > (m._level || 0)).forEach((x) => x.remove());
        const r = b.getBoundingClientRect();
        const sm = H.showMenu(it.sub, r.right - 4, r.top - 6, { keep: true, level: (m._level || 0) + 1 });
        return sm;
      };
      b.addEventListener('mouseenter', openSub);
      b.addEventListener('click', (e) => { e.stopPropagation(); const sm = openSub(); const f = sm.querySelector('.mi:not(.dis)'); f && f.focus(); });
      b._openSub = openSub;
    } else {
      b.addEventListener('mouseenter', () => { $$('.menu', $('#layer')).filter((x) => x._level > (m._level || 0)).forEach((x) => x.remove()); });
      b.addEventListener('click', (e) => { e.stopPropagation(); if (dis) return; H.closeMenus(); setTimeout(() => it.run && it.run(), 0); });
    }
    m.append(b);
  }
  m._level = opts.level || 0;
  m.addEventListener('mousedown', (e) => e.preventDefault()); // keep the document's selection
  $('#layer').append(m);
  const r = m.getBoundingClientRect();
  const vw = innerWidth, vh = innerHeight;
  m.style.left = Math.max(4, Math.min(x, vw - r.width - 4)) + 'px';
  m.style.top = Math.max(4, Math.min(y, vh - r.height - 4)) + 'px';
  if (opts.focus) { const f = m.querySelector('.mi:not(.dis)'); f && f.focus(); }
  return m;
};
document.addEventListener('mousedown', (e) => { if (!e.target.closest('.menu') && !e.target.closest('#menubar')) H.closeMenus(); }, true);
document.addEventListener('keydown', (e) => {
  const m = e.target.closest && e.target.closest('.menu');
  if (!m) return;
  const items = $$('.mi:not(.dis)', m);
  const i = items.indexOf(e.target);
  if (e.key === 'ArrowDown') { e.preventDefault(); (items[i + 1] || items[0]).focus(); }
  else if (e.key === 'ArrowUp') { e.preventDefault(); (items[i - 1] || items[items.length - 1]).focus(); }
  else if (e.key === 'ArrowRight' && e.target._openSub) { e.preventDefault(); const sm = e.target._openSub(); const f = sm.querySelector('.mi:not(.dis)'); f && f.focus(); }
  else if (e.key === 'ArrowLeft' && m._level > 0) { e.preventDefault(); m.remove(); const up = $$('.menu').find((x) => x._level === m._level - 1); up && (up.querySelector('.mi:focus') || up.querySelector('.mi')).focus(); }
  else if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && H._menuOpen != null) { e.preventDefault(); H.openBarMenu(H._menuOpen + (e.key === 'ArrowRight' ? 1 : -1), true); }
  else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); const back = H._menuOpen; H.closeMenus(); if (back != null) { const b = $$('#menubar .mb')[back]; b && b.focus(); } else H.app && H.app.focus && H.app.focus(); }
  else if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.target.click(); }
}, true);

H.buildMenubar = () => {
  const bar = $('#menubar'); bar.innerHTML = '';
  H.menus = H.app.menus();
  H.menus.forEach((m, i) => {
    const b = H.el('button', { class: 'mb f', 'data-f': true, text: m.label, role: 'menuitem' });
    b.addEventListener('mousedown', (e) => { e.preventDefault(); if (H._menuOpen === i) H.closeMenus(); else H.openBarMenu(i, false); });
    b.addEventListener('mouseenter', () => { if (H._menuOpen != null && H._menuOpen !== i) H.openBarMenu(i, false); });
    b.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') { e.preventDefault(); H.openBarMenu(i, true); } else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') { e.preventDefault(); const bs = $$('#menubar .mb'); bs[(i + (e.key === 'ArrowRight' ? 1 : bs.length - 1)) % bs.length].focus(); } });
    bar.append(b);
  });
  H.collectKeys();
};
H.openBarMenu = (i, focus) => {
  const n = H.menus.length; i = (i + n) % n;
  H.closeMenus();
  const b = $$('#menubar .mb')[i];
  b.classList.add('open');
  const r = b.getBoundingClientRect();
  H.showMenu(H.menus[i].items, r.left, r.bottom + 2, { keep: true, focus });
  H._menuOpen = i;
};

// ------------------------------------------------------------------ keyboard shortcuts
H.keyOf = (e) => {
  let k = e.key;
  if (k === ' ') k = 'Space';
  if (e.shiftKey && /^Digit\d$/.test(e.code || '')) k = e.code.slice(5);
  if (k.length === 1) k = k.toUpperCase();
  const shift = e.shiftKey && (k.length > 1 || /^[A-Z0-9]$/.test(k));
  return (e.ctrlKey || e.metaKey ? 'Ctrl+' : '') + (e.altKey ? 'Alt+' : '') + (shift ? 'Shift+' : '') + k;
};
H.collectKeys = () => {
  H.keys = {};
  const walk = (items) => (typeof items === 'function' ? items() : items).forEach((it) => {
    if (!it || it.sep) return;
    if (it.key && it.run) for (const k of String(it.key).split(' / ')) H.keys[k.replace(/\s/g, '')] = it;
    if (it.sub && typeof it.sub !== 'function') walk(it.sub);
  });
  H.menus.forEach((m) => walk(m.items));
  Object.assign(H.keys, H.app.extraKeys ? H.app.extraKeys() : {});
};
const NATIVE_IN_FIELDS = new Set(['Ctrl+Z', 'Ctrl+Y', 'Ctrl+Shift+Z', 'Ctrl+A', 'Ctrl+C', 'Ctrl+V', 'Ctrl+X', 'Ctrl+B', 'Ctrl+I', 'Ctrl+U', 'Delete', 'Backspace']);
document.addEventListener('keydown', (e) => {
  if (H._dialogs && H._dialogs.length) return;
  if (e.target.closest && e.target.closest('.menu')) return;
  const k = H.keyOf(e);
  if (k === 'F10' || k === 'Alt+F') { e.preventDefault(); H.openBarMenu(0, true); return; }
  if (!H.keys) return;
  const it = H.keys[k];
  if (!it) return;
  const t = e.target;
  const field = (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT') && !t.classList.contains('app-key');
  if (field && NATIVE_IN_FIELDS.has(k)) return;
  if (it.disabled && it.disabled()) return;
  e.preventDefault(); e.stopPropagation();
  it.run(e);
}, true);

// ------------------------------------------------------------------ the document: state, title, recovery
H.setTitle = () => {
  const d = H.doc;
  $('#docName').textContent = d.name;
  $('#docState').textContent = d.dirty ? ' • edited' : (d.path ? ' • saved' : '');
  H.invoke('set_title', { title: `${d.dirty ? '• ' : ''}${d.name} — ${H.app.name}` }).catch(() => {});
};
// The app calls this after every change to the document.
H.changed = () => {
  H.doc.used = true;
  H._recoveryDue = true;
  if (!H.doc.dirty) { H.doc.dirty = true; H.invoke('set_dirty', { dirty: true }); H.setTitle(); }
};
H.markSaved = () => {
  H.doc.dirty = false; H._recoveryDue = false;
  H.invoke('set_dirty', { dirty: false });
  if (H.doc.rid) H.invoke('recovery_clear', { id: H.doc.rid });
  H.setTitle();
};
H.newRid = () => { H.doc.rid = H.app.id + '-' + H.uid(); };
// Every 15 seconds: a recovery copy of a changed document (so a crash or power cut loses little). With
// AutoSave on (File menu), a document that has a file in a format that keeps everything is saved to it as well.
H.autosaveTick = async () => {
  if (!H.app || !H.doc.dirty || !H._recoveryDue || H._busy) return;
  H._recoveryDue = false;
  try {
    if (!H.doc.rid) H.newRid();
    await H.invoke('recovery_save', { id: H.doc.rid, meta: { app: H.app.id, title: H.doc.name, path: H.doc.path, ext: H.doc.ext }, body: H.app.snapshot() });
    if (H.settings.autosave && H.doc.path && H.app.canWrite(H.doc.ext) && !H.app.lossy(H.doc.ext)) await H.save({ quiet: true });
  } catch (e) { H.invoke('log', { message: 'recovery: ' + e }); }
};
setInterval(() => H.autosaveTick(), 15000);

H.saveSettings = () => H.invoke('settings_save', { value: H.settings }).catch(() => {});

// ------------------------------------------------------------------ open
// Whether this window's document is in use (then another file opens in a new window).
H.inUse = () => H.doc.dirty || H.doc.path || H.doc.used;

H.openDialog = async () => {
  const p = await H.invoke('pick_open', { kind: H.app.id });
  if (p) await H.openPath(p);
};
H.openPath = async (path, opts = {}) => {
  const ext = H.extOf(path);
  const owner = Object.values(H.apps).find((a) => a.openExts.includes(ext));
  if (!H.app.openExts.includes(ext)) {
    if (owner && owner !== H.app) { await H.invoke('open_window', { app: owner.id, file: path }); H.toast(`Opening ${H.baseName(path)} in ${owner.name}`); return; }
    if (!opts.anyway) { await H.alert('Can\'t open this file', `${H.app.name} doesn't open .${ext || '?'} files.`); return; }
  }
  if (H.inUse() && !opts.here) { await H.invoke('open_window', { app: H.app.id, file: path }); H.toast(`Opening ${H.baseName(path)} in a new window`); return; }
  H.busy(`Opening ${H.baseName(path)}…`);
  try {
    const r = await H.invoke('read_file', { path });
    const bytes = H.b64ToBytes(r.data);
    await H.app.ready;
    const notes = await H.app.load(bytes, ext, H.baseName(path));
    H.doc = { path, name: H.baseName(path), ext, dirty: false, rid: null, used: true };
    H.newRid();
    H.invoke('set_dirty', { dirty: false });
    H.setTitle();
    H.hideStart();
    H.recentAdd(path);
    if (notes && notes.length) H.toast(notes.join(' '), 6000);
    return true;
  } catch (e) {
    console.error(e);
    H.invoke('log', { message: `open ${path}: ${e && e.stack || e}` });
    if (!opts.quiet) await H.alert('Couldn\'t open the file', `${H.baseName(path)}: ${e && e.message || e}`);
    if (opts.throw) throw e;
    return false;
  } finally { H.busy(null); }
};
H.recentAdd = async (path) => { try { H.info.recent = await H.invoke('recent_add', { path }); } catch {} };

// ------------------------------------------------------------------ save
H.formatFor = (ext) => H.app.formats.find((f) => f.ext === ext);
H.save = async (opts = {}) => {
  const d = H.doc;
  if (!d.path || !H.app.canWrite(d.ext)) return H.saveAs(null, opts);
  // A format that keeps less than the app shows: say so once per document, and offer the full format.
  const warn = H.app.lossy(d.ext);
  if (warn && !d.lossOk && !opts.quiet) {
    const full = H.app.formats[0];
    const v = await H.dialog({
      title: `Keep using .${d.ext}?`, body: H.el('p', { text: warn }),
      buttons: [{ label: 'Cancel', value: null }, { label: `Save as ${full.label} (.${full.ext})`, value: 'full' }, { label: `Keep .${d.ext}`, value: 'keep', primary: true }],
    });
    if (!v) return false;
    if (v === 'full') return H.saveAs(full.ext, opts);
    d.lossOk = true;
  }
  return H.writeTo(d.path, d.ext, opts);
};
H.saveAs = async (ext, opts = {}) => {
  ext = ext || (H.app.canWrite(H.doc.ext) ? H.doc.ext : H.app.formats[0].ext);
  const f = H.formatFor(ext) || H.app.formats[0];
  const name = H.stem(H.doc.name || 'Untitled') + '.' + f.ext;
  const p = await H.invoke('pick_save', { name, label: f.label, exts: [f.ext], dir: H.dirOf(H.doc.path) });
  if (!p) return false;
  const e2 = H.extOf(p) || f.ext;
  if (!H.app.canWrite(e2)) { await H.alert('Can\'t save that type', `${H.app.name} can't save .${e2} files.`); return false; }
  const ok = await H.writeTo(p, e2, opts);
  if (ok) { H.doc.path = p; H.doc.name = H.baseName(p); H.doc.ext = e2; H.doc.lossOk = !!H.app.lossy(e2); H.setTitle(); H.recentAdd(p); }
  return ok;
};
H.writeTo = async (path, ext, opts = {}) => {
  H._busy = true;
  if (!opts.quiet) H.busy('Saving…');
  try {
    const bytes = await H.app.save(ext);
    await H.invoke('write_file', { path, data: H.bytesToB64(bytes) });
    H.markSaved();
    if (!opts.quiet) H.toast(`Saved ${H.baseName(path)}`);
    return true;
  } catch (e) {
    console.error(e);
    H.invoke('log', { message: `save ${path}: ${e && e.stack || e}` });
    await H.alert('Couldn\'t save', String(e && e.message || e));
    return false;
  } finally { H._busy = false; H.busy(null); }
};
// Save a copy in another format without changing which file this window is working on.
H.exportAs = async (ext) => {
  const f = H.formatFor(ext);
  const p = await H.invoke('pick_save', { name: H.stem(H.doc.name || 'Untitled') + '.' + ext, label: f ? f.label : ext, exts: [ext], dir: H.dirOf(H.doc.path) });
  if (!p) return;
  H.busy('Exporting…');
  try { const bytes = await H.app.save(H.extOf(p) || ext); await H.invoke('write_file', { path: p, data: H.bytesToB64(bytes) }); H.toast(`Exported ${H.baseName(p)}`); }
  catch (e) { await H.alert('Couldn\'t export', String(e && e.message || e)); }
  finally { H.busy(null); }
};

// ------------------------------------------------------------------ print and PDF
H.preparePrint = async () => {
  const box = $('#print'); box.innerHTML = '';
  const dims = await H.app.printLayout(box);
  let st = $('#printPage');
  if (!st) { st = H.el('style', { id: 'printPage' }); document.head.append(st); }
  st.textContent = `@page { size: ${dims.w}mm ${dims.h}mm; margin: 0; }`;
  // Pictures must be loaded before the page is printed.
  await Promise.all($$('img', box).map((i) => i.complete ? null : new Promise((r) => { i.onload = i.onerror = r; })));
  if (document.fonts && document.fonts.ready) await document.fonts.ready;
  return dims;
};
H.print = async () => { await H.preparePrint(); await H.invoke('print'); };
H.exportPdf = async (path) => {
  if (!path) path = await H.invoke('pick_save', { name: H.stem(H.doc.name || 'Untitled') + '.pdf', label: 'PDF', exts: ['pdf'], dir: H.dirOf(H.doc.path) });
  if (!path) return false;
  H.busy('Making the PDF…');
  try {
    const dims = await H.preparePrint();
    const done = new Promise((res) => { H._pdfDone = res; });
    const how = await H.invoke('export_pdf', { path, widthMm: dims.w, heightMm: dims.h });
    if (how === 'dialog') { H.toast('Pick "Microsoft Print to PDF" in the print dialog'); return true; }
    const r = await Promise.race([done, H.sleep(60000).then(() => ({ ok: false, error: 'it took too long' }))]);
    if (!r.ok) throw new Error(r.error || 'the PDF wasn\'t made');
    H.toast(`Exported ${H.baseName(path)}`);
    return true;
  } catch (e) { await H.alert('Couldn\'t make the PDF', String(e && e.message || e)); return false; }
  finally { H.busy(null); }
};
H.listen('office://pdf', (p) => { if (H._pdfDone) { H._pdfDone(p); H._pdfDone = null; } });

H.busy = (msg) => {
  let b = $('#busy');
  if (!msg) { if (b) b.remove(); return; }
  if (!b) { b = H.el('div', { id: 'busy' }, H.el('div', { class: 'spin' }), H.el('span')); document.body.append(b); }
  b.querySelector('span').textContent = msg;
};

// ------------------------------------------------------------------ closing with unsaved changes
H.askSave = async () => {
  if (!H.doc.dirty) return true;
  const v = await H.dialog({
    title: `Save changes to "${H.doc.name}"?`, body: H.el('p', { text: 'Your changes will be lost if you don\'t save them.' }), modal: true,
    buttons: [{ label: 'Cancel', value: null }, { label: 'Don\'t save', value: 'discard', danger: true }, { label: 'Save', value: 'save', primary: true }],
  });
  if (v === 'save') return await H.save();
  if (v === 'discard') { if (H.doc.rid) await H.invoke('recovery_clear', { id: H.doc.rid }); return true; }
  return false;
};
H.listen('office://close', async () => { if (await H.askSave()) H.invoke('quit'); });
H.quit = async () => { if (await H.askSave()) H.invoke('quit'); };
H.newWindow = (app) => H.invoke('open_window', { app: app || H.app.id });

// The File menu every app shares (each adds its own Save as / export formats).
H.fileMenu = (extra = []) => [
  { label: 'New', key: 'Ctrl+N', run: () => (H.inUse() ? H.newWindow() : H.showStart()) },
  { label: 'New from template…', run: () => H.showStart() },
  { label: 'Open…', key: 'Ctrl+O', run: () => H.openDialog() },
  { label: 'Open recent', sub: () => { const r = (H.info.recent || []).slice(0, 12); return r.length ? r.map((x) => ({ label: x.name + (x.exists === false ? ' (missing)' : ''), disabled: () => x.exists === false, run: () => H.openPath(x.path) })) : [{ label: 'No recent files', disabled: () => true }]; } },
  { sep: true },
  { label: 'Save', key: 'Ctrl+S', run: () => H.save() },
  { label: 'Save as', sub: H.app.formats.filter((f) => f.write !== false).map((f) => ({ label: `${f.label} (.${f.ext})`, run: () => H.saveAs(f.ext) })) },
  { label: 'Save as…', key: 'Ctrl+Shift+S', run: () => H.saveAs() },
  { label: 'AutoSave', checked: () => !!H.settings.autosave, run: () => { H.settings.autosave = !H.settings.autosave; H.saveSettings(); H.toast(H.settings.autosave ? 'AutoSave is on: changes are saved to the file every 15 seconds (when its format keeps everything).' : 'AutoSave is off. Recovery copies are still kept.', 5000); } },
  ...extra,
  { sep: true },
  { label: 'Export as PDF…', key: 'Ctrl+Shift+E', run: () => H.exportPdf() },
  { label: 'Print…', key: 'Ctrl+P', run: () => H.print() },
  { sep: true },
  { label: 'New Write document', run: () => H.newWindow('write') },
  { label: 'New spreadsheet', run: () => H.newWindow('sheets') },
  { label: 'New presentation', run: () => H.newWindow('slides') },
  { sep: true },
  { label: 'Close', key: 'Ctrl+W', run: () => H.quit() },
];
H.helpMenu = (extra = []) => [
  ...extra,
  { label: 'Keyboard shortcuts', key: 'F1', run: () => H.showKeys() },
  { label: `About ${H.app.name}`, run: () => H.alert(H.app.name, `${H.app.name} ${H.info.version || ''}: part of Hearth Office, Hearth OS's own office apps. Free software (GPL-3.0). Uses ${H.app.credits}.`) },
];
H.showKeys = () => {
  const rows = Object.entries(H.keys || {}).filter(([, it]) => it.label).sort((a, b) => a[1].label.localeCompare(b[1].label));
  const t = H.el('div', { class: 'keys' }, rows.map(([k, it]) => H.el('div', { class: 'krow' }, H.el('span', { text: it.label }), H.el('kbd', { text: k }))));
  H.dialog({ title: 'Keyboard shortcuts', body: t, wide: true });
};

// ------------------------------------------------------------------ the start screen
const APP_LIST = [['write', 'Write', 'Documents'], ['sheets', 'Sheets', 'Spreadsheets'], ['slides', 'Slides', 'Presentations']];
H.appIcon = (id) => `<span class="appico ${id}">${{ write: '<svg viewBox="0 0 24 24"><path d="M7 4h7l4 4v12H7z"/><path d="M14 4v4h4M9.5 12h6M9.5 15h6M9.5 18h4"/></svg>', sheets: '<svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 9.5h16M4 14.5h16M10 4v16"/></svg>', slides: '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="12" rx="2"/><path d="M12 17v3M8.5 20h7M8 9.5h8M8 12.5h5"/></svg>' }[id]}</span>`;
H.timeAgo = (t) => {
  if (!t) return '';
  const s = Date.now() / 1000 - t;
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  return new Date(t * 1000).toLocaleDateString();
};
H.showStart = async () => {
  const s = $('#start');
  $('#startIcon').innerHTML = H.appIcon(H.app.id);
  $('#startTitle').textContent = H.app.name;
  $('#startSub').textContent = H.app.tagline;
  $('#startApps').innerHTML = '';
  for (const [id, short] of APP_LIST) if (id !== H.app.id) $('#startApps').append(H.el('button', { class: 'chip f', 'data-f': true, html: `${H.appIcon(id)}<span>New ${short === 'Write' ? 'document' : short === 'Sheets' ? 'spreadsheet' : 'presentation'}</span>`, onclick: () => H.newWindow(id) }));
  const tiles = $('#tiles'); tiles.innerHTML = '';
  for (const t of H.app.templates) {
    tiles.append(H.el('button', { class: 'tile f', 'data-f': true, onclick: () => H.startNew(t.id) },
      H.el('div', { class: 'thumb', html: t.thumb || '' }), H.el('div', { class: 'tn', text: t.name }), t.desc ? H.el('div', { class: 'td', text: t.desc }) : null));
  }
  // Recovered work.
  const rb = $('#recoverBox'); rb.innerHTML = '';
  let rec = [];
  try { rec = (H.info.recovery || []).filter((r) => r.id !== H.doc.rid); } catch {}
  if (rec.length) {
    rb.append(H.el('h2', { text: 'Recovered' }), H.el('p', { class: 'note', text: 'These weren\'t saved when their window closed. Open one to keep working on it, or discard it.' }));
    for (const r of rec.slice(0, 6)) {
      rb.append(H.el('div', { class: 'rrow' },
        H.el('div', { class: 'rn' }, H.el('b', { text: r.title || 'Untitled' }), H.el('span', { text: ` ${r.path ? '— ' + r.path : '(never saved)'} · ${H.timeAgo(r.time)}` })),
        H.el('button', { class: 'btn f', 'data-f': true, text: 'Discard', onclick: async (e) => { await H.invoke('recovery_clear', { id: r.id }); H.info.recovery = H.info.recovery.filter((x) => x.id !== r.id); e.target.closest('.rrow').remove(); if (!$('.rrow', rb)) rb.hidden = true; } }),
        H.el('button', { class: 'btn primary f', 'data-f': true, text: 'Open', onclick: () => H.recover(r) })));
    }
    rb.hidden = false;
  } else rb.hidden = true;
  // Recent files.
  const rl = $('#recentList'); rl.innerHTML = '';
  try { H.info.recent = await H.invoke('recent_list'); } catch {}
  const recent = (H.info.recent || []).filter((r) => r.app === H.app.id || !r.app);
  if (!recent.length) rl.append(H.el('p', { class: 'note', text: 'Files you open or save show up here.' }));
  for (const r of recent.slice(0, 20)) {
    const row = H.el('div', { class: 'recent-row' + (r.exists === false ? ' gone' : '') },
      H.el('button', { class: 'rr f', 'data-f': true, onclick: () => r.exists === false ? H.toast('That file isn\'t there anymore') : H.openPath(r.path) },
        H.el('span', { class: 'rext', text: (H.extOf(r.path) || '?').toUpperCase() }),
        H.el('span', { class: 'rname', text: r.name }), H.el('span', { class: 'rpath', text: H.dirOf(r.path) || '' }), H.el('span', { class: 'rtime', text: r.exists === false ? 'missing' : H.timeAgo(r.time) })),
      H.el('button', { class: 'ibtn f', 'data-f': true, title: 'Remove from the list', html: '<svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18"/></svg>', onclick: async () => { H.info.recent = await H.invoke('recent_remove', { path: r.path }); row.remove(); } }));
    rl.append(row);
  }
  s.hidden = false;
  document.body.classList.add('starting');
  setTimeout(() => { const f = $('.tile', s); f && f.focus(); }, 30);
};
H.hideStart = () => { $('#start').hidden = true; document.body.classList.remove('starting'); };
H.startNew = async (tpl) => {
  await H.app.ready;
  if (H.inUse()) { H.invoke('open_window', { app: H.app.id }); return; }
  H.app.newDoc(tpl);
  H.doc = { path: null, name: H.app.untitled, ext: null, dirty: false, rid: null, used: false };
  H.newRid();
  H.setTitle(); H.hideStart();
  H.app.focus && H.app.focus();
};
H.recover = async (r) => {
  await H.app.ready;
  if (H.inUse()) { H.toast('Close this document first, or open the recovered one from a new window'); return; }
  try {
    const body = await H.invoke('recovery_load', { id: r.id });
    await H.app.restore(body);
    H.doc = { path: r.path || null, name: r.title || H.app.untitled, ext: r.ext || null, dirty: false, rid: r.id, used: true };
    H.changed();
    H.hideStart();
    H.toast('Recovered. Save it to keep it.');
  } catch (e) { H.alert('Couldn\'t recover it', String(e)); }
};
$('#startOpen') && $('#startOpen').addEventListener('click', () => H.openDialog());

// ------------------------------------------------------------------ the controller
// A standard-layout pad (Xbox-style): the d-pad and left stick move between buttons on the start screen, in
// dialogs and menus (A presses, B goes back); in a document the sticks scroll. Each app can take buttons first
// (Slides uses them for presenting, Sheets for moving between cells).
H.pad = { prev: [], held: {}, axesT: 0 };
const BTN = { 0: 'A', 1: 'B', 2: 'X', 3: 'Y', 4: 'LB', 5: 'RB', 6: 'LT', 7: 'RT', 8: 'Back', 9: 'Start', 12: 'Up', 13: 'Down', 14: 'Left', 15: 'Right' };
H.spatial = (dir) => {
  const scope = (H._dialogs && H._dialogs.length && H._dialogs[H._dialogs.length - 1]) || $$('.menu').pop() || (!$('#start').hidden && $('#start')) || document.body;
  const items = $$('[data-f],.mi:not(.dis),.f', scope).filter((e) => e.offsetParent !== null && !e.disabled);
  const cur = document.activeElement;
  if (!items.length) return;
  if (!items.includes(cur)) { items[0].focus(); return; }
  const r0 = cur.getBoundingClientRect(); const c0 = { x: r0.left + r0.width / 2, y: r0.top + r0.height / 2 };
  let best = null, bd = Infinity;
  for (const e of items) {
    if (e === cur) continue;
    const r = e.getBoundingClientRect(); const c = { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    const dx = c.x - c0.x, dy = c.y - c0.y;
    const ok = dir === 'Left' ? dx < -4 : dir === 'Right' ? dx > 4 : dir === 'Up' ? dy < -4 : dy > 4;
    if (!ok) continue;
    const main = (dir === 'Left' || dir === 'Right') ? Math.abs(dx) : Math.abs(dy);
    const side = (dir === 'Left' || dir === 'Right') ? Math.abs(dy) : Math.abs(dx);
    const d = main + side * 2.5;
    if (d < bd) { bd = d; best = e; }
  }
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest' }); }
};
H.padPress = (b) => {
  if (H.app && H.app.pad && H.app.pad(b)) return;
  const overlay = (H._dialogs && H._dialogs.length) || $$('.menu').length || !$('#start').hidden;
  if (['Up', 'Down', 'Left', 'Right'].includes(b)) {
    if (overlay) H.spatial(b);
    else { const s = H.app.scroller && H.app.scroller(); if (s) s.scrollBy({ top: b === 'Up' ? -200 : b === 'Down' ? 200 : 0, left: b === 'Left' ? -200 : b === 'Right' ? 200 : 0, behavior: 'smooth' }); }
  } else if (b === 'A') { const a = document.activeElement; if (a && a !== document.body && (overlay || a.matches('button'))) a.click(); }
  else if (b === 'B') { if ($$('.menu').length) H.closeMenus(); else if (H._dialogs && H._dialogs.length) H._dialogs[H._dialogs.length - 1].dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); else if (!$('#start').hidden && H.doc.used) H.hideStart(); }
  else if (b === 'Start') { if ($('#start').hidden) H.showStart(); else if (H.doc.used) H.hideStart(); }
  else if (b === 'Back') H.openBarMenu(0, true);
};
function padLoop() {
  const pads = navigator.getGamepads ? Array.from(navigator.getGamepads()).filter(Boolean) : [];
  if (!pads.length) { H.pad.running = false; return; }
  const now = performance.now();
  const p = pads[0];
  for (const [i, name] of Object.entries(BTN)) {
    const down = p.buttons[i] && p.buttons[i].pressed;
    const was = H.pad.prev[i];
    if (down && !was) { H.pad.held[i] = now + 420; H.padPress(name); }
    else if (down && ['Up', 'Down', 'Left', 'Right'].includes(name) && now > H.pad.held[i]) { H.pad.held[i] = now + 110; H.padPress(name); }
    H.pad.prev[i] = down;
  }
  // Left stick: like the d-pad. Right stick: scrolls.
  const [ax, ay, rx, ry] = [p.axes[0] || 0, p.axes[1] || 0, p.axes[2] || 0, p.axes[3] || 0];
  if (Math.hypot(ax, ay) > 0.6 && now > H.pad.axesT) { H.padPress(Math.abs(ax) > Math.abs(ay) ? (ax > 0 ? 'Right' : 'Left') : (ay > 0 ? 'Down' : 'Up')); H.pad.axesT = now + 180; }
  if (Math.hypot(rx, ry) > 0.2) { const s = H.app && H.app.scroller && H.app.scroller(); if (s) s.scrollBy(rx * 24, ry * 24); }
  requestAnimationFrame(padLoop);
}
window.addEventListener('gamepadconnected', () => { if (!H.pad.running) { H.pad.running = true; requestAnimationFrame(padLoop); } });

// ------------------------------------------------------------------ toolbar helpers the apps use
H.tbtn = (icon, title, run, opts = {}) => {
  const b = H.el('button', { class: 'tb f' + (opts.wide ? ' wide' : ''), title, 'data-f': true, 'data-cmd': opts.cmd || null, html: icon });
  b.addEventListener('mousedown', (e) => e.preventDefault()); // keep the selection in the document
  b.addEventListener('click', (e) => run(e, b));
  return b;
};
H.tsel = (options, title, onchange, opts = {}) => {
  const s = H.el('select', { class: 'ts f' + (opts.cls ? ' ' + opts.cls : ''), title, 'data-f': true }, options.map(([v, l]) => H.el('option', { value: v, text: l ?? v })));
  s.addEventListener('change', () => onchange(s.value, s));
  return s;
};
H.tsep = () => H.el('span', { class: 'tsep' });
// A colour button: the swatch shows the colour; the arrow opens a palette.
H.PALETTE = ['#000000', '#434343', '#666666', '#999999', '#cccccc', '#ffffff', '#c00000', '#ff0000', '#ffc000', '#ffff00', '#92d050', '#00b050', '#00b0f0', '#0070c0', '#002060', '#7030a0',
  '#f4cccc', '#fce5cd', '#fff2cc', '#d9ead3', '#d0e0e3', '#cfe2f3', '#d9d2e9', '#ead1dc', '#e06666', '#f6b26b', '#ffd966', '#93c47d', '#76a5af', '#6fa8dc', '#8e7cc3', '#c27ba0', '#ff8a3d', '#2a1206'];
H.colorBtn = (icon, title, initial, apply, opts = {}) => {
  const wrap = H.el('span', { class: 'cbtn' });
  let cur = initial;
  const main = H.tbtn(icon + '<i class="bar"></i>', title, () => apply(cur));
  const bar = main.querySelector('.bar'); bar.style.background = cur || 'transparent';
  const more = H.tbtn('<svg viewBox="0 0 24 24" class="small"><path d="M7 10l5 5 5-5"/></svg>', title + ': pick a colour', () => {
    const r = more.getBoundingClientRect();
    H.closeMenus();
    const m = H.el('div', { class: 'menu palette' });
    if (opts.none) m.append(H.el('button', { class: 'mi wide', text: opts.none, onclick: () => { H.closeMenus(); apply(null); } }));
    const grid = H.el('div', { class: 'swatches' });
    for (const c of H.PALETTE) grid.append(H.el('button', { class: 'sw', title: c, style: { background: c }, onclick: () => { cur = c; bar.style.background = c; H.closeMenus(); apply(c); } }));
    m.append(grid);
    const custom = H.el('input', { type: 'color', value: cur && cur.startsWith('#') ? cur : '#ff8a3d' });
    custom.addEventListener('change', () => { cur = custom.value; bar.style.background = cur; H.closeMenus(); apply(cur); });
    m.append(H.el('label', { class: 'mi wide' }, H.el('span', { text: 'Custom… ' }), custom));
    m.addEventListener('mousedown', (e) => { if (e.target.tagName !== 'INPUT') e.preventDefault(); });
    $('#layer').append(m);
    m.style.left = Math.min(r.left - 30, innerWidth - 240) + 'px'; m.style.top = r.bottom + 4 + 'px';
  });
  more.classList.add('narrow');
  wrap.append(main, more);
  return wrap;
};
H.ICON = {
  undo: '<svg viewBox="0 0 24 24"><path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 0 12h-3"/></svg>',
  redo: '<svg viewBox="0 0 24 24"><path d="M15 14l5-5-5-5"/><path d="M20 9H10a6 6 0 0 0 0 12h3"/></svg>',
  bold: '<svg viewBox="0 0 24 24"><path d="M7 5h6a3.5 3.5 0 0 1 0 7H7zM7 12h7a3.5 3.5 0 0 1 0 7H7z" stroke-width="2.4"/></svg>',
  italic: '<svg viewBox="0 0 24 24"><path d="M14 5h-4M14 19h-4M14.5 5l-5 14" stroke-width="2"/></svg>',
  underline: '<svg viewBox="0 0 24 24"><path d="M7 4v7a5 5 0 0 0 10 0V4M5 20h14" stroke-width="2"/></svg>',
  strike: '<svg viewBox="0 0 24 24"><path d="M4 12h16M16 6.5C15 5 13.6 4.5 12 4.5c-2.5 0-4 1.3-4 3.2 0 1.3.8 2.2 2.3 2.8M8 17.5c1 1.4 2.4 2 4.2 2 2.6 0 4.3-1.4 4.3-3.3"/></svg>',
  color: '<svg viewBox="0 0 24 24"><path d="M6 17L12 4l6 13M8.3 12.5h7.4"/></svg>',
  fill: '<svg viewBox="0 0 24 24"><path d="M5 12l7-7 6 6-7 7z"/><path d="M19 15s2 2.3 2 3.5a2 2 0 0 1-4 0c0-1.2 2-3.5 2-3.5z"/></svg>',
  highlight: '<svg viewBox="0 0 24 24"><path d="M9 15l-3 3h5l1-1M9 15l7-9 3 3-7 9z"/></svg>',
  left: '<svg viewBox="0 0 24 24"><path d="M4 6h16M4 10h10M4 14h16M4 18h10"/></svg>',
  center: '<svg viewBox="0 0 24 24"><path d="M4 6h16M7 10h10M4 14h16M7 18h10"/></svg>',
  right: '<svg viewBox="0 0 24 24"><path d="M4 6h16M10 10h10M4 14h16M10 18h10"/></svg>',
  justify: '<svg viewBox="0 0 24 24"><path d="M4 6h16M4 10h16M4 14h16M4 18h16"/></svg>',
  bullets: '<svg viewBox="0 0 24 24"><circle cx="5" cy="7" r="1.2"/><circle cx="5" cy="12" r="1.2"/><circle cx="5" cy="17" r="1.2"/><path d="M9 7h11M9 12h11M9 17h11"/></svg>',
  numbers: '<svg viewBox="0 0 24 24"><path d="M10 7h10M10 12h10M10 17h10M4 5.5l1.5-1V9M3.8 14.2c.3-.6.9-.9 1.5-.8.9.1 1.3 1 .7 1.7L3.8 18h2.6"/></svg>',
  indent: '<svg viewBox="0 0 24 24"><path d="M4 6h16M11 10h9M11 14h9M4 18h16M4 9.5l3 2.5-3 2.5"/></svg>',
  outdent: '<svg viewBox="0 0 24 24"><path d="M4 6h16M11 10h9M11 14h9M4 18h16M7 9.5L4 12l3 2.5"/></svg>',
  link: '<svg viewBox="0 0 24 24"><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/></svg>',
  image: '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="M21 16l-5-5-9 8"/></svg>',
  table: '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="1.5"/><path d="M3 10h18M3 14.5h18M9 5v14M15 5v14"/></svg>',
  clear: '<svg viewBox="0 0 24 24"><path d="M6 5h12M12 5l-3 14M15 13l5 5M20 13l-5 5"/></svg>',
  find: '<svg viewBox="0 0 24 24"><circle cx="10.5" cy="10.5" r="5.5"/><path d="M15 15l5 5"/></svg>',
  print: '<svg viewBox="0 0 24 24"><path d="M7 9V4h10v5M7 17H5a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-2"/><rect x="7" y="14" width="10" height="6"/></svg>',
  save: '<svg viewBox="0 0 24 24"><path d="M5 4h11l3 3v13H5z"/><path d="M8 4v5h7V4M8 20v-6h8v6"/></svg>',
  open: '<svg viewBox="0 0 24 24"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>',
  spell: '<svg viewBox="0 0 24 24"><path d="M4 15l3.5-9L11 15M5.3 12h4.4M14 10l2.5 3L21 6"/><path d="M4 19c2-1 3 1 5 0s3 1 5 0 3 1 5 0" stroke-width="1.4"/></svg>',
  pagebreak: '<svg viewBox="0 0 24 24"><path d="M6 3v5h12V3M6 21v-5h12v5M3 12h3M9 12h2M13 12h2M18 12h3"/></svg>',
  shape: '<svg viewBox="0 0 24 24"><rect x="3" y="10" width="10" height="10" rx="1"/><circle cx="16" cy="8" r="5"/></svg>',
  text: '<svg viewBox="0 0 24 24"><path d="M5 6V4h14v2M12 4v16M9 20h6"/></svg>',
  play: '<svg viewBox="0 0 24 24"><path d="M7 4.5v15l12-7.5z"/></svg>',
  plus: '<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>',
  trash: '<svg viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/></svg>',
  merge: '<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="1.5"/><path d="M8 12h8M10 10l-2 2 2 2M14 10l2 2-2 2"/></svg>',
  wrap: '<svg viewBox="0 0 24 24"><path d="M4 6h16M4 12h13a3 3 0 0 1 0 6h-4M15 16l-2 2 2 2M4 18h6"/></svg>',
  borders: '<svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" stroke-dasharray="2 2"/><path d="M4 20h16" stroke-width="2.4"/></svg>',
  sum: '<svg viewBox="0 0 24 24"><path d="M17 5H7l6 7-6 7h10"/></svg>',
  sortAZ: '<svg viewBox="0 0 24 24"><path d="M4 15l2.5-8L9 15M4.8 12.5h3.4M13 7h6l-6 8h6M17 18l2 2 2-2"/></svg>',
  filter: '<svg viewBox="0 0 24 24"><path d="M4 5h16l-6 7v6l-4 2v-8z"/></svg>',
  chart: '<svg viewBox="0 0 24 24"><path d="M4 20h16M7 17v-6M12 17V6M17 17v-9"/></svg>',
  freeze: '<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="1.5"/><path d="M3 9h18M9 4v16" stroke-width="2.4"/></svg>',
  percent: '<svg viewBox="0 0 24 24"><path d="M19 5L5 19"/><circle cx="7" cy="7" r="2.2"/><circle cx="17" cy="17" r="2.2"/></svg>',
  currency: '<svg viewBox="0 0 24 24"><path d="M12 3v18M16.5 7.5c-.8-1.3-2.5-2-4.5-2-2.5 0-4 1.2-4 3 0 4.5 9 2.5 9 7 0 1.8-1.8 3-4.5 3-2 0-3.8-.8-4.6-2.2"/></svg>',
  decUp: '<svg viewBox="0 0 24 24"><path d="M3 17h1M7 17a2 2.5 0 1 0 0-.01M14 7l3-3 3 3M17 4v9"/><text x="10" y="20" font-size="7" stroke="none" fill="currentColor">.00</text></svg>',
  decDown: '<svg viewBox="0 0 24 24"><path d="M14 10l3 3 3-3M17 4v9"/><text x="3" y="20" font-size="7" stroke="none" fill="currentColor">.0</text></svg>',
  notes: '<svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8 9h8M8 13h8M8 17h5"/></svg>',
  front: '<svg viewBox="0 0 24 24"><rect x="9" y="9" width="11" height="11" rx="1" fill="currentColor" fill-opacity=".3"/><path d="M4 15V4h11"/></svg>',
  back: '<svg viewBox="0 0 24 24"><rect x="4" y="4" width="11" height="11" rx="1" fill="currentColor" fill-opacity=".3"/><path d="M9 20h11V9"/></svg>',
  theme: '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8.5"/><circle cx="8.5" cy="10" r="1.3"/><circle cx="12" cy="7.5" r="1.3"/><circle cx="15.5" cy="10" r="1.3"/><path d="M12 20.5a2.5 2.5 0 0 1 0-5h1.5"/></svg>',
  comment: '<svg viewBox="0 0 24 24"><path d="M4 5h16v11H9l-5 4z"/></svg>',
};

// ------------------------------------------------------------------ start
H.register = (app) => { H.apps[app.id] = app; };
H.applyTheme = () => {
  document.documentElement.classList.toggle('light', H.settings.theme === 'light');
};
H.boot = async () => {
  let info;
  try { info = await H.invoke('init'); } catch (e) { info = { app: 'write', launch: {} }; }
  H.info = info;
  H.settings = info.settings || {};
  H.applyTheme();
  const app = H.apps[info.app] || H.apps.write;
  H.app = app;
  document.body.dataset.app = app.id;
  $('#brandIcon').innerHTML = H.appIcon(app.id);
  $('#brand').title = app.name;
  H.doc.name = app.untitled;
  H.fonts = await H.invoke('fonts').catch(() => []);
  app.ready = app.mount($('#main'), $('#toolbar'), $('#status'));
  try { await app.ready; } catch (e) { H.invoke('log', { message: 'mount: ' + (e && e.stack || e) }); await H.alert('Something went wrong', String(e && e.message || e)); }
  H.buildMenubar();
  $('#btnHome').addEventListener('click', () => ($('#start').hidden ? H.showStart() : (H.doc.used || H.doc.path ? H.hideStart() : null)));
  const L = info.launch || {};
  if (L.convert) return H.convert(L.convert[0], L.convert[1]);
  H.app.newDoc(null);
  H.newRid();
  H.setTitle();
  if (L.file) { const ok = await H.openPath(L.file, { here: true }); if (!ok) H.showStart(); else if (L.present && app.present) app.present(0); }
  else H.showStart();
  H.invoke('log', { message: `${app.name} ready` });
};
// --convert IN OUT: open, save as OUT's format (PDF too), quit with how it went.
H.convert = async (inp, out) => {
  try {
    const r = await H.invoke('read_file', { path: inp });
    await H.app.load(H.b64ToBytes(r.data), H.extOf(inp), H.baseName(inp));
    H.doc = { path: inp, name: H.baseName(inp), ext: H.extOf(inp), used: true };
    const ext = H.extOf(out);
    if (ext === 'pdf') {
      const dims = await H.preparePrint();
      const done = new Promise((res) => { H._pdfDone = res; });
      await H.invoke('export_pdf', { path: out, widthMm: dims.w, heightMm: dims.h });
      const p = await Promise.race([done, H.sleep(60000).then(() => ({ ok: false, error: 'timed out' }))]);
      if (!p.ok) throw new Error(p.error);
    } else {
      if (!H.app.canWrite(ext)) throw new Error(`can't save .${ext}`);
      const bytes = await H.app.save(ext);
      await H.invoke('write_file', { path: out, data: H.bytesToB64(bytes) });
    }
    await H.invoke('convert_done', { ok: true, message: `saved ${out}` });
  } catch (e) {
    await H.invoke('convert_done', { ok: false, message: `${inp} → ${out}: ${e && e.stack || e}` });
  }
};
window.addEventListener('error', (e) => { H.invoke('log', { message: `error: ${e.message} at ${e.filename}:${e.lineno}` }).catch(() => {}); });
window.addEventListener('unhandledrejection', (e) => { H.invoke('log', { message: `unhandled: ${e.reason && e.reason.stack || e.reason}` }).catch(() => {}); });
// Dropped files open (a Hearth file opens in its own app).
window.addEventListener('dragover', (e) => { if (e.dataTransfer && Array.from(e.dataTransfer.types).includes('Files')) e.preventDefault(); });
document.addEventListener('DOMContentLoaded', () => {
  // The apps' scripts come after this one.
  Promise.all(['write.js', 'sheets.js', 'slides.js'].map((f) => H.need(f))).then(H.boot, (e) => { document.body.textContent = String(e); });
});
