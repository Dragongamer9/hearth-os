'use strict';
// Hearth Contacts' page. Keyboard first (arrows, Enter, Esc, Ctrl+N/F/I/E/S, Delete, E to edit) and fully usable
// with a controller (D-pad/stick moves, A chooses, B goes back, X new, Y search, Menu for import/export/accounts),
// with an on-screen keyboard for typing. Opened in a plain browser it shows sample contacts (demoInvoke below).
const T = window.__TAURI__;
const invoke = T ? T.core.invoke : demoInvoke;
const $ = (s, el = document) => el.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

const KINDS = { phone: ['cell', 'home', 'work', 'main', 'fax', 'other'], email: ['home', 'work', 'other'], adr: ['home', 'work', 'other'], url: ['home', 'work', 'other'] };
const KIND_NAME = { cell: 'Mobile', home: 'Home', work: 'Work', main: 'Main', fax: 'Fax', pager: 'Pager', other: 'Other' };
const S = { books: [], contacts: [], accounts: [], presets: [], book: null, group: null, q: '', sel: null, edit: null, pad: false, syncing: false };
const keyOf = e => `${e.book}/${e.contact.uid}`;

// ---------------------------------------------------------------- data
async function load() {
  try {
    const r = await invoke('state');
    Object.assign(S, { books: r.books, contacts: r.contacts, accounts: r.accounts, presets: r.presets });
    if (S.book && !S.books.find(b => b.id === S.book)) S.book = null;
    render();
  } catch (e) { toast(String(e), true); }
}

function shown(c) {
  if (c.display && c.display.trim()) return c.display.trim();
  const n = c.name || {};
  const parts = [n.prefix, n.given, n.additional, n.family, n.suffix].map(s => (s || '').trim()).filter(Boolean);
  if (parts.length) return parts.join(' ');
  return (c.org || '').trim() || c.emails?.find(e => e.value)?.value || c.phones?.find(p => p.value)?.value || 'No name';
}
function initials(c) {
  const n = shown(c).replace(/[^\p{L}\p{N} ]/gu, '').trim().split(/\s+/);
  return ((n[0]?.[0] || '') + (n.length > 1 ? n[n.length - 1][0] : '')).toUpperCase() || '?';
}
function avatar(c, big) {
  return `<span class="av ${big ? 'big' : ''}">${c.photo ? `<img alt="" src="${esc(c.photo)}">` : esc(initials(c))}</span>`;
}
function groups() {
  const g = new Map();
  for (const e of S.contacts) if (!S.book || e.book === S.book) for (const x of e.contact.groups || []) g.set(x, (g.get(x) || 0) + 1);
  return [...g.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}
function visible() {
  const q = S.q.trim().toLowerCase(), digits = q.replace(/[^0-9]/g, '');
  return S.contacts.filter(e => {
    if (S.book && e.book !== S.book) return false;
    const c = e.contact;
    if (S.group && !(c.groups || []).includes(S.group)) return false;
    if (!q) return true;
    const hay = [shown(c), c.nickname, c.org, c.title, c.note, ...(c.emails || []).map(x => x.value), ...(c.addresses || []).map(a => [a.street, a.locality, a.country].join(' ')), ...(c.groups || [])].join(' ').toLowerCase();
    if (hay.includes(q)) return true;
    return digits.length >= 3 && (c.phones || []).some(p => p.value.replace(/[^0-9]/g, '').includes(digits));
  });
}
const current = () => S.contacts.find(e => keyOf(e) === S.sel);
const bookName = id => S.books.find(b => b.id === id)?.name || id;

// ---------------------------------------------------------------- drawing
function render() {
  const focusKey = document.activeElement?.dataset?.key;
  $('#sub').textContent = `${S.contacts.length} contact${S.contacts.length === 1 ? '' : 's'}`;
  renderSide(); renderList();
  if (!S.edit) renderDetail();
  const back = focusKey && document.querySelector(`[data-key="${CSS.escape(focusKey)}"]`);
  if (back) back.focus({ preventScroll: true });
}

function renderSide() {
  const books = S.books.map(b => {
    const a = S.accounts.find(x => x.id === b.id);
    return `<button class="nav f ${S.book === b.id ? 'sel' : ''}" data-key="book:${esc(b.id)}" data-book="${esc(b.id)}"><span>${esc(b.name)}${a?.lastError ? ' <span class="err" title="' + esc(a.lastError) + '">!</span>' : ''}</span><span>${b.count}</span></button>`;
  }).join('');
  const gs = groups().map(([g, n]) => `<button class="nav f ${S.group === g ? 'sel' : ''}" data-key="group:${esc(g)}" data-group="${esc(g)}"><span>${esc(g)}</span><span>${n}</span></button>`).join('');
  $('#side').innerHTML = `<h3>Address books</h3><button class="nav f ${!S.book ? 'sel' : ''}" data-key="book:*" data-book=""><span>All contacts</span><span>${S.contacts.length}</span></button>${books}`
    + (gs ? `<h3>Groups</h3><button class="nav f ${!S.group ? 'sel' : ''}" data-key="group:*" data-group=""><span>Everyone</span><span></span></button>${gs}` : '');
}

function renderList() {
  const list = visible();
  let last = '', html = '';
  for (const e of list) {
    const c = e.contact, name = shown(c);
    const L = (name[0] || '#').toLocaleUpperCase();
    const letter = /\p{L}/u.test(L) ? L : '#';
    if (letter !== last && !S.q) { html += `<div class="letter">${esc(letter)}</div>`; last = letter; }
    const sub = c.phones?.find(p => p.value)?.value || c.emails?.find(x => x.value)?.value || c.org || '';
    html += `<button class="row f ${keyOf(e) === S.sel ? 'sel' : ''}" data-key="c:${esc(keyOf(e))}" data-sel="${esc(keyOf(e))}">${avatar(c)}<span class="n">${esc(name)}</span><span class="m">${esc(sub)}</span></button>`;
  }
  $('#list').innerHTML = html || `<div class="empty">${S.q ? 'Nobody matches that.' : S.contacts.length ? 'No contacts here.' : 'No contacts yet. Press New, or import a .vcf file.'}</div>`;
}

function birthdayText(b) {
  if (!b) return '';
  const m = /^(\d{4}|-)-?(\d\d)-(\d\d)$/.exec(b.replace(/^--/, '--'));
  const md = b.startsWith('--') ? b.slice(2) : b.slice(5);
  const [mm, dd] = md.split('-').map(Number);
  if (!mm || !dd) return b;
  const d = new Date(2000, mm - 1, dd);
  let s = d.toLocaleDateString(undefined, { day: 'numeric', month: 'long' });
  if (!b.startsWith('--')) {
    const y = Number(b.slice(0, 4)), now = new Date();
    let age = now.getFullYear() - y; if (now.getMonth() + 1 < mm || (now.getMonth() + 1 === mm && now.getDate() < dd)) age--;
    s = `${s} ${y} (${age})`;
  }
  void m;
  return s;
}
const adrText = a => [a.pobox && `PO Box ${a.pobox}`, a.street, a.ext, [a.postcode, a.locality].filter(Boolean).join(' '), a.region, a.country].filter(x => x && x.trim()).join('\n');

function renderDetail() {
  const el = $('#detail');
  const e = current();
  if (!e) { el.innerHTML = `<div class="empty">${S.contacts.length ? 'Choose a contact.' : ''}</div>`; return; }
  const c = e.contact;
  const items = (title, rows) => rows.length ? `<div class="card"><h3>${title}</h3>${rows.join('')}</div>` : '';
  const phones = (c.phones || []).filter(p => p.value).map((p, i) => `<div class="item"><span class="k">${esc(KIND_NAME[p.kind] || p.kind)}${p.pref ? ' ★' : ''}</span><span class="v">${esc(p.value)}</span><span class="acts"><button class="btn small f" data-key="call:${i}" data-uri="tel:${esc(p.value.replace(/[^0-9+*#]/g, ''))}">Call</button><button class="btn small f" data-key="cp:p${i}" data-copy="${esc(p.value)}">Copy</button></span></div>`);
  const emails = (c.emails || []).filter(x => x.value).map((x, i) => `<div class="item"><span class="k">${esc(KIND_NAME[x.kind] || x.kind)}${x.pref ? ' ★' : ''}</span><span class="v">${esc(x.value)}</span><span class="acts"><button class="btn small f" data-key="mail:${i}" data-uri="mailto:${esc(x.value)}">Email</button><button class="btn small f" data-key="cp:e${i}" data-copy="${esc(x.value)}">Copy</button></span></div>`);
  const adrs = (c.addresses || []).map((a, i) => ({ a, t: adrText(a) })).filter(x => x.t).map(({ a, t }, i) => `<div class="item"><span class="k">${esc(KIND_NAME[a.kind] || a.kind)}</span><span class="v">${esc(t)}</span><span class="acts"><button class="btn small f" data-key="map:${i}" data-map="${esc(t.replace(/\n/g, ', '))}">Map</button><button class="btn small f" data-key="cp:a${i}" data-copy="${esc(t)}">Copy</button></span></div>`);
  const urls = (c.urls || []).filter(x => x.value).map((x, i) => `<div class="item"><span class="k">${esc(KIND_NAME[x.kind] || 'Web')}</span><span class="v">${esc(x.value)}</span><span class="acts"><button class="btn small f" data-key="url:${i}" data-uri="${esc(/^https?:\/\//i.test(x.value) ? x.value : 'https://' + x.value)}">Open</button></span></div>`);
  const more = [];
  if (c.birthday) more.push(`<div class="item"><span class="k">Birthday</span><span class="v">${esc(birthdayText(c.birthday))}</span><span></span></div>`);
  if (c.nickname) more.push(`<div class="item"><span class="k">Nickname</span><span class="v">${esc(c.nickname)}</span><span></span></div>`);
  el.innerHTML = `
    <div class="dhead">${avatar(c, true)}<div><h2>${esc(shown(c))}</h2><div class="sub">${esc([c.title, c.org].filter(Boolean).join(' · '))}</div><div class="sub">${esc(bookName(e.book))}</div></div></div>
    <div class="acts"><button class="btn primary f" data-key="d:edit" data-act="edit">Edit</button><button class="btn f" data-key="d:share" data-act="export-one">Save as .vcf</button>${S.books.length > 1 ? `<button class="btn f" data-key="d:move" data-act="move">Move to…</button>` : ''}<button class="btn danger f" data-key="d:del" data-act="delete">Delete</button></div>
    ${items('Phone', phones)}${items('Email', emails)}${items('Address', adrs)}${items('Web', urls)}${items('More', more)}
    ${c.note ? `<div class="card"><h3>Notes</h3><div class="v" style="white-space:pre-wrap;user-select:text">${esc(c.note)}</div></div>` : ''}
    ${(c.groups || []).length ? `<div class="card"><h3>Groups</h3><div class="chips">${c.groups.map(g => `<span class="chip">${esc(g)}</span>`).join('')}</div></div>` : ''}`;
}

// ---------------------------------------------------------------- editing
const blank = () => ({ uid: '', display: '', name: { family: '', given: '', additional: '', prefix: '', suffix: '' }, nickname: '', org: '', title: '', phones: [], emails: [], addresses: [], urls: [], birthday: '', photo: '', note: '', groups: [], rev: '', extra: [] });

function startEdit(isNew) {
  const e = current();
  const book = isNew ? (S.book || 'local') : e.book;
  const c = isNew ? blank() : JSON.parse(JSON.stringify(e.contact));
  if (isNew) { c.phones.push({ kind: 'cell', value: '', pref: false }); c.emails.push({ kind: 'home', value: '', pref: false }); if (S.group) c.groups.push(S.group); }
  // A display name that's just the parts of the name is kept in step with them.
  S.edit = { book, c, isNew, autoName: isNew || c.display === [c.name.prefix, c.name.given, c.name.additional, c.name.family, c.name.suffix].filter(Boolean).join(' ') };
  document.body.classList.remove('list-mode');
  renderEditor();
  setTimeout(() => $('#detail [data-f="name.given"]')?.focus(), 0);
}

function kindBtn(list, i, kind) { return `<button class="btn small f kind" data-key="k:${list}:${i}" data-kind="${list}:${i}" title="Change the kind">${esc(KIND_NAME[kind] || kind)}</button>`; }

function renderEditor() {
  const { c, book, isNew } = S.edit;
  const inp = (f, label, val, extra = '') => `<div class="field"><label>${label}</label><input class="inp f" data-key="f:${f}" data-f="${f}" data-osk="1" value="${esc(val)}" spellcheck="false" autocomplete="off" ${extra}></div>`;
  const multi = (list, rows, placeholder, type) => rows.map((x, i) => `<div class="multi">${kindBtn(list, i, x.kind)}<input class="inp f" type="${type}" data-key="f:${list}.${i}" data-f="${list}.${i}.value" data-osk="1" value="${esc(x.value)}" placeholder="${placeholder}" spellcheck="false" autocomplete="off"><button class="btn small f" data-key="rm:${list}:${i}" data-rm="${list}:${i}" title="Remove">✕</button></div>`).join('');
  const adr = c.addresses.map((a, i) => `<div class="multi adr">${kindBtn('addresses', i, a.kind)}<div class="adrf">
      <input class="inp f wide" data-key="f:a${i}s" data-f="addresses.${i}.street" data-osk="1" value="${esc(a.street)}" placeholder="Street">
      <input class="inp f" data-key="f:a${i}p" data-f="addresses.${i}.postcode" data-osk="1" value="${esc(a.postcode)}" placeholder="Postcode">
      <input class="inp f" data-key="f:a${i}l" data-f="addresses.${i}.locality" data-osk="1" value="${esc(a.locality)}" placeholder="Town or city">
      <input class="inp f" data-key="f:a${i}r" data-f="addresses.${i}.region" data-osk="1" value="${esc(a.region)}" placeholder="State or region">
      <input class="inp f" data-key="f:a${i}c" data-f="addresses.${i}.country" data-osk="1" value="${esc(a.country)}" placeholder="Country"></div>
      <button class="btn small f" data-key="rm:addresses:${i}" data-rm="addresses:${i}" title="Remove">✕</button></div>`).join('');
  const allGroups = [...new Set([...groups().map(g => g[0]), ...c.groups])].sort();
  const bdayY = c.birthday && !c.birthday.startsWith('--') ? c.birthday.slice(0, 4) : '';
  const bdayMD = c.birthday ? (c.birthday.startsWith('--') ? c.birthday.slice(2) : c.birthday.slice(5)) : '';
  $('#detail').innerHTML = `<div class="form">
    <div class="dhead">${avatar(c, true)}<div class="acts"><button class="btn f" data-key="e:photo" data-act="photo">${c.photo ? 'Change photo' : 'Add photo'}</button>${c.photo ? '<button class="btn f" data-key="e:nophoto" data-act="nophoto">Remove photo</button>' : ''}</div></div>
    <div class="grid2">${inp('name.given', 'First name', c.name.given)}${inp('name.family', 'Last name', c.name.family)}${inp('display', 'Shown as', c.display, 'placeholder="From the name"')}${inp('nickname', 'Nickname', c.nickname)}${inp('org', 'Company', c.org)}${inp('title', 'Job title', c.title)}${inp('name.prefix', 'Title (Dr., Ms.)', c.name.prefix)}${inp('name.additional', 'Middle name', c.name.additional)}</div>
    <div class="card"><h3>Phone</h3>${multi('phones', c.phones, 'Number', 'tel')}<button class="btn small f" data-key="add:phones" data-add="phones">Add a number</button></div>
    <div class="card"><h3>Email</h3>${multi('emails', c.emails, 'name@example.com', 'email')}<button class="btn small f" data-key="add:emails" data-add="emails">Add an email</button></div>
    <div class="card"><h3>Address</h3>${adr}<button class="btn small f" data-key="add:addresses" data-add="addresses">Add an address</button></div>
    <div class="card"><h3>Web</h3>${multi('urls', c.urls, 'https://', 'url')}<button class="btn small f" data-key="add:urls" data-add="urls">Add a web address</button></div>
    <div class="card"><h3>Birthday</h3><div class="grid2"><div class="field"><label>Day and month</label><input class="inp f" type="text" data-key="f:bmd" id="bmd" data-osk="1" placeholder="MM-DD" value="${esc(bdayMD)}" maxlength="5"></div><div class="field"><label>Year (if known)</label><input class="inp f" type="text" data-key="f:by" id="by" data-osk="1" placeholder="YYYY" value="${esc(bdayY)}" maxlength="4"></div></div></div>
    <div class="card"><h3>Groups</h3><div class="chips">${allGroups.map(g => `<button class="chip f ${c.groups.includes(g) ? 'on' : ''}" data-key="g:${esc(g)}" data-tgroup="${esc(g)}">${esc(g)}</button>`).join('')}<input class="inp f" style="width:12em" data-key="f:newgroup" id="newgroup" data-osk="1" placeholder="New group, then Enter"></div></div>
    <div class="field"><label>Notes</label><textarea class="inp f" data-key="f:note" data-f="note" data-osk="1">${esc(c.note)}</textarea></div>
    ${isNew && S.books.length > 1 ? `<div class="field"><label>Save in</label><div class="chips">${S.books.map(b => `<button class="chip f ${b.id === book ? 'on' : ''}" data-key="sb:${esc(b.id)}" data-savebook="${esc(b.id)}">${esc(b.name)}</button>`).join('')}</div></div>` : ''}
    <div class="acts"><button class="btn primary f" data-key="e:save" data-act="save">Save</button><button class="btn f" data-key="e:cancel" data-act="cancel">Cancel</button></div>
  </div>`;
}

function setPath(obj, path, val) {
  const p = path.split('.'); let o = obj;
  for (let i = 0; i < p.length - 1; i++) o = o[p[i]];
  o[p[p.length - 1]] = val;
}
function readBirthday() {
  const md = ($('#bmd')?.value || '').trim(), y = ($('#by')?.value || '').trim();
  if (!md) return '';
  const m = /^(\d{1,2})[-./ ](\d{1,2})$/.exec(md);
  if (!m) throw new Error('Write the birthday as month and day, like 07-04.');
  const mm = m[1].padStart(2, '0'), dd = m[2].padStart(2, '0');
  if (+mm < 1 || +mm > 12 || +dd < 1 || +dd > 31) throw new Error('That birthday isn\'t a real date.');
  if (y && !/^\d{4}$/.test(y)) throw new Error('The birth year needs four digits.');
  return y ? `${y}-${mm}-${dd}` : `--${mm}-${dd}`;
}
async function saveEdit() {
  const ed = S.edit; const c = ed.c;
  try { c.birthday = readBirthday(); } catch (e) { toast(e.message, true); return; }
  c.phones = c.phones.filter(p => p.value.trim()); c.emails = c.emails.filter(p => p.value.trim()); c.urls = c.urls.filter(p => p.value.trim());
  c.addresses = c.addresses.filter(a => [a.street, a.locality, a.postcode, a.region, a.country, a.pobox, a.ext].some(x => (x || '').trim()));
  if (ed.autoName) c.display = '';
  try {
    const saved = await invoke('save', { book: ed.book, contact: c });
    S.edit = null; S.sel = `${ed.book}/${saved.uid}`;
    await load();
    toast('Saved');
    syncSoon(ed.book);
    $(`[data-key="d:edit"]`)?.focus();
  } catch (e) { toast(String(e), true); }
}
function cancelEdit() { S.edit = null; renderDetail(); (document.querySelector('.row.sel') || $('#q')).focus(); }

async function choosePhoto() {
  try {
    const path = await invoke('plugin:dialog|open', { options: { title: 'Choose a photo', multiple: false, directory: false, filters: [{ name: 'Pictures', extensions: ['jpg', 'jpeg', 'png', 'webp', 'gif', 'bmp'] }] } });
    if (!path) return;
    const url = await invoke('read_image', { path });
    S.edit.c.photo = await shrink(url);
    renderEditor(); $('[data-key="e:photo"]')?.focus();
  } catch (e) { toast(String(e), true); }
}
// Photos are kept small (256 × 256 JPEG): that's what phones and servers take without trouble.
function shrink(url) {
  return new Promise((ok, bad) => {
    const img = new Image();
    img.onload = () => {
      const s = 256, cv = document.createElement('canvas'); cv.width = s; cv.height = s;
      const side = Math.min(img.width, img.height), sx = (img.width - side) / 2, sy = (img.height - side) / 2;
      cv.getContext('2d').drawImage(img, sx, sy, side, side, 0, 0, s, s);
      ok(cv.toDataURL('image/jpeg', 0.85));
    };
    img.onerror = () => bad('That picture couldn\'t be opened.');
    img.src = url;
  });
}

// ---------------------------------------------------------------- dialogs
let lastFocus = null;
function showDialog(html) {
  if (!$('#layer').firstChild) lastFocus = document.activeElement?.dataset?.key || null;
  $('#layer').innerHTML = `<div class="veil"><div class="dlg" role="dialog">${html}</div></div>`;
  setTimeout(() => ($('#layer [data-autofocus]') || $('#layer .f'))?.focus(), 0);
}
function closeDialog() {
  $('#layer').innerHTML = ''; oskTarget = null; if (window.HearthOSK) HearthOSK.close(true);
  const k = lastFocus;
  setTimeout(() => (k && document.querySelector(`[data-key="${CSS.escape(k)}"]`) || document.querySelector('.row.sel') || $('#q'))?.focus(), 0);
}
let toastTimer;
function toast(text, bad) {
  $('.toast')?.remove();
  document.body.insertAdjacentHTML('beforeend', `<div class="toast ${bad ? 'bad' : ''}">${esc(text)}</div>`);
  clearTimeout(toastTimer); toastTimer = setTimeout(() => $('.toast')?.remove(), bad ? 6000 : 2500);
}

function askDelete() {
  const e = current(); if (!e) return;
  const acct = S.accounts.find(a => a.id === e.book);
  showDialog(`<h2>Delete ${esc(shown(e.contact))}?</h2><p class="help">${acct ? `It's also deleted from ${esc(acct.name)} at the next sync.` : 'This can\'t be undone.'}</p>
    <div class="drow"><button class="btn f" data-dlg="close" data-autofocus>Cancel</button><button class="btn primary f" style="background:var(--danger)" data-dlg="delete">Delete</button></div>`);
}
async function doDelete() {
  const e = current(); if (!e) return closeDialog();
  try {
    await invoke('delete', { book: e.book, uid: e.contact.uid });
    const list = visible(); const i = list.findIndex(x => keyOf(x) === S.sel);
    S.sel = list[i + 1] ? keyOf(list[i + 1]) : list[i - 1] ? keyOf(list[i - 1]) : null;
    closeDialog(); await load(); syncSoon(e.book); toast('Deleted');
  } catch (err) { toast(String(err), true); }
}

function askMove() {
  const e = current(); if (!e) return;
  showDialog(`<h2>Move ${esc(shown(e.contact))} to</h2><div class="chips">${S.books.filter(b => b.id !== e.book).map((b, i) => `<button class="chip f" data-moveto="${esc(b.id)}" ${i ? '' : 'data-autofocus'}>${esc(b.name)}</button>`).join('')}</div><div class="drow"><button class="btn f" data-dlg="close">Cancel</button></div>`);
}

// Import: pick a .vcf, see what's in it, choose the address book.
async function startImport(path) {
  try {
    if (!path) path = await invoke('plugin:dialog|open', { options: { title: 'Add contacts from a file', multiple: false, directory: false, filters: [{ name: 'Contacts (vCard)', extensions: ['vcf', 'vcard'] }] } });
    if (!path) return;
    const p = await invoke('preview_file', { path });
    S.importPath = path; S.importBook = S.book || 'local';
    const books = S.books.map(b => `<button class="chip f ${b.id === S.importBook ? 'on' : ''}" data-ibook="${esc(b.id)}">${esc(b.name)}</button>`).join('');
    showDialog(`<h2>Add ${p.count} contact${p.count === 1 ? '' : 's'}?</h2><p class="help">${esc(p.names.join(', '))}${p.count > p.names.length ? ', …' : ''}</p>
      <p class="help">From ${esc(path)}. Contacts already here (the same card) are updated rather than doubled.</p>
      <div class="field"><label>Add to</label><div class="chips">${books}</div></div>
      <div class="drow"><button class="btn f" data-dlg="close">Cancel</button><button class="btn primary f" data-dlg="import" data-autofocus>Add</button></div>`);
  } catch (e) { toast(String(e), true); }
}
async function doImport() {
  try {
    const r = await invoke('import_file', { path: S.importPath, book: S.importBook });
    closeDialog(); await load(); syncSoon(S.importBook);
    toast(`${r.added} added${r.updated ? `, ${r.updated} updated` : ''}${r.skipped ? `, ${r.skipped} empty skipped` : ''}`);
  } catch (e) { toast(String(e), true); }
}

function startExport(onlyCurrent) {
  const e = current();
  const opts = [];
  if (onlyCurrent && e) opts.push(['one', `Only ${shown(e.contact)}`]);
  opts.push(['shown', `The ${visible().length} shown now`]);
  opts.push(['all', `All ${S.contacts.length} contacts`]);
  S.exportWhich = opts[0][0];
  showDialog(`<h2>Save contacts as a .vcf file</h2><p class="help">A .vcf file opens in any contacts app or phone.</p>
    <div class="chips">${opts.map(([k, t], i) => `<button class="chip f ${i ? '' : 'on'}" data-xwhich="${k}" ${i ? '' : 'data-autofocus'}>${esc(t)}</button>`).join('')}</div>
    <div class="drow"><button class="btn f" data-dlg="close">Cancel</button><button class="btn primary f" data-dlg="export">Choose where…</button></div>`);
}
async function doExport() {
  const e = current();
  const which = S.exportWhich === 'one' && e ? [[e.book, e.contact.uid]] : S.exportWhich === 'shown' ? visible().map(x => [x.book, x.contact.uid]) : [];
  const name = S.exportWhich === 'one' && e ? shown(e.contact) : S.group || 'Contacts';
  try {
    const path = await invoke('plugin:dialog|save', { options: { title: 'Save contacts', defaultPath: `${name.replace(/[\\/:*?"<>|]/g, '_')}.vcf`, filters: [{ name: 'Contacts (vCard)', extensions: ['vcf'] }] } });
    if (!path) return;
    if (S.exportWhich === 'shown' && !which.length) { toast('Nothing is shown to save.', true); return; }
    const n = await invoke('export_file', { path, which });
    closeDialog(); toast(`${n} contact${n === 1 ? '' : 's'} saved to ${path}`);
  } catch (err) { toast(String(err), true); }
}

// Accounts (CardDAV).
function showAccounts() {
  const rows = S.accounts.map(a => `<div class="acct"><div><b>${esc(a.name)}</b> <span class="muted">${esc(a.bookName)}</span><small>${esc(a.username)} · ${esc(a.server)}</small>
      <small class="${a.lastError ? 'bad' : ''}">${a.lastError ? esc(a.lastError) : a.lastSync ? 'Last synced ' + esc(revText(a.lastSync)) : 'Not synced yet'}</small></div>
      <div class="acts"><button class="btn small f" data-syncone="${esc(a.id)}">Sync now</button>${a.lastError && /password|keyring/i.test(a.lastError) ? `<button class="btn small f" data-repass="${esc(a.id)}">Sign in again</button>` : ''}<button class="btn small f" data-rmacct="${esc(a.id)}">Remove</button></div></div>`).join('');
  showDialog(`<h2>Accounts</h2><p class="help">Keep an address book in step with Nextcloud, iCloud, Fastmail or any CardDAV server. Passwords are kept in the system keyring.</p>
    ${rows || '<p class="muted">No accounts yet. Contacts are kept on this computer only.</p>'}
    <div class="drow">${S.accounts.length ? '<button class="btn f" data-syncall="1">Sync all</button>' : ''}<button class="btn primary f" data-dlg="addacct" data-autofocus>Add an account</button><button class="btn f" data-dlg="close">Close</button></div>`);
}
function revText(r) {
  const m = /^(\d{4})(\d\d)(\d\d)T(\d\d)(\d\d)(\d\d)Z$/.exec(r || '');
  return m ? new Date(Date.UTC(+m[1], m[2] - 1, +m[3], +m[4], +m[5], +m[6])).toLocaleString() : r;
}
function addAccount(preset) {
  const p = S.presets.find(x => x.id === preset) || S.presets[0];
  S.newAcct = { preset: p.id, books: null };
  showDialog(`<h2>Add an account</h2>
    <div class="chips">${S.presets.map(x => `<button class="chip f ${x.id === p.id ? 'on' : ''}" data-preset="${x.id}">${esc(x.name)}</button>`).join('')}</div>
    <p class="help">${esc(p.help)}</p>
    <div class="field"><label>Server</label><input class="inp f" id="a-server" data-osk="1" value="${esc(p.server)}" spellcheck="false" autocomplete="off"></div>
    <div class="field"><label>${esc(p.userHint)}</label><input class="inp f" id="a-user" data-osk="1" spellcheck="false" autocomplete="off" ${p.server.length > 8 ? 'data-autofocus' : ''}></div>
    <div class="field"><label>Password${p.id === 'icloud' || p.id === 'fastmail' ? ' (an app password)' : ''}</label><input class="inp f" id="a-pass" type="password" data-osk="1" autocomplete="off"></div>
    <div id="a-books"></div>
    <div class="drow"><button class="btn f" data-dlg="accounts">Back</button><button class="btn primary f" id="a-go" data-dlg="discover">Sign in</button></div>`);
  if (p.server.length <= 8) setTimeout(() => $('#a-server')?.focus(), 0);
}
async function discover() {
  const server = $('#a-server').value.trim(), username = $('#a-user').value.trim(), password = $('#a-pass').value;
  if (!server || !username || !password) { toast('Fill in the server, user name and password.', true); return; }
  $('#a-books').innerHTML = '<p class="muted"><span class="spin"></span> Looking for your address books…</p>';
  try {
    const books = await invoke('discover', { server, username, password });
    S.newAcct.books = books; S.newAcct.book = 0;
    $('#a-books').innerHTML = `<div class="field"><label>Address book</label><div class="chips">${books.map((b, i) => `<button class="chip f ${i ? '' : 'on'}" data-abook="${i}">${esc(b.name)}</button>`).join('')}</div></div>`;
    const go = $('#a-go'); go.textContent = 'Add'; go.dataset.dlg = 'addnow'; go.focus();
  } catch (e) { $('#a-books').innerHTML = `<p class="bad">${esc(e)}</p>`; }
}
async function addNow() {
  const n = S.newAcct, b = n.books[n.book], p = S.presets.find(x => x.id === n.preset);
  const go = $('#a-go'); go.disabled = true;
  $('#a-books').insertAdjacentHTML('beforeend', '<p class="muted"><span class="spin"></span> Copying the address book…</p>');
  try {
    const r = await invoke('add_account', { name: p.id === 'other' ? new URL(/:\/\//.test($('#a-server').value) ? $('#a-server').value : 'https://' + $('#a-server').value).hostname : p.name, provider: p.id,
      server: $('#a-server').value.trim(), username: $('#a-user').value.trim(), password: $('#a-pass').value, bookUrl: b.url, bookName: b.name });
    await load(); showAccounts();
    if (r.error) toast(r.error, true); else toast(`${r.report.downloaded} contacts copied from ${p.name}`);
  } catch (e) { go.disabled = false; toast(String(e), true); }
}
function askRemoveAccount(id) {
  const a = S.accounts.find(x => x.id === id); if (!a) return;
  S.rmAcct = id;
  showDialog(`<h2>Remove ${esc(a.name)}?</h2><p class="help">Nothing on the server is touched. Its password is taken out of the keyring.</p>
    <div class="drow"><button class="btn f" data-dlg="accounts" data-autofocus>Cancel</button><button class="btn f" data-rmnow="keep">Remove, keep a copy here</button><button class="btn primary f" data-rmnow="drop">Remove with its contacts</button></div>`);
}
function rePass(id) {
  const a = S.accounts.find(x => x.id === id); if (!a) return;
  S.rePass = id;
  showDialog(`<h2>Sign in to ${esc(a.name)} again</h2><p class="help">${esc(a.username)} · ${esc(a.server)}</p>
    <div class="field"><label>Password</label><input class="inp f" id="re-pass" type="password" data-osk="1" autocomplete="off" data-autofocus></div>
    <div class="drow"><button class="btn f" data-dlg="accounts">Cancel</button><button class="btn primary f" data-dlg="repass">Save and sync</button></div>`);
}

let syncTimer = null;
function syncSoon(book) { if (S.accounts.some(a => a.id === book)) { clearTimeout(syncTimer); syncTimer = setTimeout(() => runSync(book), 1500); } }
async function runSync(id = '', loud = false) {
  if (S.syncing || !S.accounts.length) return;
  S.syncing = true; $('#hint').textContent = 'Syncing…';
  try {
    const rs = await invoke('sync', { id });
    const bad = rs.filter(r => r.error);
    const moved = rs.reduce((n, r) => n + (r.report ? r.report.downloaded + r.report.uploaded + r.report.deletedHere + r.report.deletedThere : 0), 0);
    const conflicts = rs.reduce((n, r) => n + (r.report?.conflicts || 0), 0);
    $('#hint').textContent = bad.length ? `Sync: ${bad[0].name}: ${bad[0].error}` : `Synced ${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
    if (loud || bad.length) toast(bad.length ? `${bad[0].name}: ${bad[0].error}` : `Synced${moved ? ` (${moved} change${moved === 1 ? '' : 's'})` : ''}`, !!bad.length);
    if (conflicts) toast(`${conflicts} contact${conflicts === 1 ? ' was' : 's were'} changed here and on the server: both versions are kept.`);
    if (moved || bad.length || loud) { if (!S.edit) await load(); else { const r = await invoke('state'); Object.assign(S, { books: r.books, contacts: r.contacts, accounts: r.accounts }); renderSide(); renderList(); } }
    if ($('#layer').firstChild && $('#layer h2')?.textContent === 'Accounts') showAccounts();
  } catch (e) { $('#hint').textContent = String(e); }
  S.syncing = false;
}

// ---------------------------------------------------------------- clicks and typing
document.addEventListener('click', async e => {
  const b = e.target.closest('button'); if (!b) return;
  const d = b.dataset;
  if (d.book !== undefined) { S.book = d.book || null; S.group = null; render(); return; }
  if (d.group !== undefined) { S.group = d.group || null; render(); return; }
  if (d.sel) { if (S.edit) return; S.sel = d.sel; document.body.classList.remove('list-mode'); renderList(); renderDetail(); const el = $(`[data-key="c:${CSS.escape(d.sel)}"]`); el?.focus({ preventScroll: true }); if (S.pad || e.detail === 0) $('#detail .f')?.focus(); return; }
  if (d.copy !== undefined) { copy(d.copy); return; }
  if (d.uri) { invoke('open_uri', { uri: d.uri }).catch(err => toast(String(err), true)); return; }
  if (d.map) { invoke('open_map', { query: d.map }).catch(err => toast(String(err), true)); return; }
  if (d.kind) { const [list, i] = d.kind.split(':'); const x = S.edit.c[list][+i]; const ks = KINDS[{ phones: 'phone', emails: 'email', addresses: 'adr', urls: 'url' }[list]]; x.kind = ks[(ks.indexOf(x.kind) + 1) % ks.length]; b.textContent = KIND_NAME[x.kind]; return; }
  if (d.rm) { const [list, i] = d.rm.split(':'); S.edit.c[list].splice(+i, 1); renderEditor(); ($(`[data-key="add:${list}"]`))?.focus(); return; }
  if (d.add) { const l = d.add; S.edit.c[l].push(l === 'addresses' ? { kind: 'home', pobox: '', ext: '', street: '', locality: '', region: '', postcode: '', country: '' } : { kind: l === 'phones' ? 'cell' : 'home', value: '', pref: false }); renderEditor(); const n = S.edit.c[l].length - 1; ($(`[data-key="f:${l}.${n}"]`) || $(`[data-key="f:a${n}s"]`))?.focus(); return; }
  if (d.tgroup) { const g = S.edit.c.groups, i = g.indexOf(d.tgroup); if (i < 0) g.push(d.tgroup); else g.splice(i, 1); b.classList.toggle('on', i < 0); return; }
  if (d.savebook) { S.edit.book = d.savebook; document.querySelectorAll('[data-savebook]').forEach(x => x.classList.toggle('on', x === b)); return; }
  if (d.ibook) { S.importBook = d.ibook; document.querySelectorAll('[data-ibook]').forEach(x => x.classList.toggle('on', x === b)); return; }
  if (d.xwhich) { S.exportWhich = d.xwhich; document.querySelectorAll('[data-xwhich]').forEach(x => x.classList.toggle('on', x === b)); return; }
  if (d.moveto) { const c = current(); try { await invoke('move_to', { from: c.book, uid: c.contact.uid, to: d.moveto }); S.sel = `${d.moveto}/${c.contact.uid}`; closeDialog(); await load(); syncSoon(d.moveto); syncSoon(c.book); toast(`Moved to ${bookName(d.moveto)}`); } catch (err) { toast(String(err), true); } return; }
  if (d.preset) { addAccount(d.preset); return; }
  if (d.abook !== undefined) { S.newAcct.book = +d.abook; document.querySelectorAll('[data-abook]').forEach(x => x.classList.toggle('on', x === b)); return; }
  if (d.syncone) { await runSync(d.syncone, true); return; }
  if (d.syncall) { await runSync('', true); return; }
  if (d.rmacct) { askRemoveAccount(d.rmacct); return; }
  if (d.repass) { rePass(d.repass); return; }
  if (d.rmnow) { try { await invoke('remove_account', { id: S.rmAcct, keepCopy: d.rmnow === 'keep' }); S.book = null; await load(); showAccounts(); } catch (err) { toast(String(err), true); } return; }
  if (d.key && b.closest('#osk')) { oskPress(d.key); return; }
  switch (d.dlg) {
    case 'close': closeDialog(); return;
    case 'delete': doDelete(); return;
    case 'import': doImport(); return;
    case 'export': doExport(); return;
    case 'accounts': showAccounts(); return;
    case 'addacct': addAccount(S.presets[0]?.id); return;
    case 'discover': discover(); return;
    case 'addnow': addNow(); return;
    case 'repass': try { await invoke('set_password', { id: S.rePass, password: $('#re-pass').value }); showAccounts(); runSync(S.rePass, true); } catch (err) { toast(String(err), true); } return;
  }
  switch (d.act) {
    case 'new': if (!S.edit) startEdit(true); return;
    case 'edit': startEdit(false); return;
    case 'save': saveEdit(); return;
    case 'cancel': cancelEdit(); return;
    case 'delete': askDelete(); return;
    case 'move': askMove(); return;
    case 'import': startImport(); return;
    case 'export': startExport(false); return;
    case 'export-one': startExport(true); return;
    case 'accounts': showAccounts(); return;
    case 'photo': choosePhoto(); return;
    case 'nophoto': S.edit.c.photo = ''; renderEditor(); $('[data-key="e:photo"]')?.focus(); return;
  }
});

document.addEventListener('input', e => {
  const t = e.target;
  if (t.id === 'q') { S.q = t.value; renderList(); return; }
  if (S.edit && t.dataset.f) {
    setPath(S.edit.c, t.dataset.f, t.value);
    if (t.dataset.f === 'display') S.edit.autoName = !t.value.trim();
  }
});

function copy(text) {
  const done = () => toast('Copied');
  if (navigator.clipboard?.writeText) navigator.clipboard.writeText(text).then(done, () => fallbackCopy(text));
  else fallbackCopy(text);
  function fallbackCopy(t) { const ta = document.createElement('textarea'); ta.value = t; document.body.appendChild(ta); ta.select(); document.execCommand('copy'); ta.remove(); done(); }
}

function back() {
  if (oskTarget) { closeOsk(); return; }
  if ($('#layer').firstChild) { closeDialog(); return; }
  if (S.edit) { cancelEdit(); return; }
  const a = document.activeElement;
  if (a && a.closest('#detail')) { document.body.classList.add('list-mode'); (document.querySelector('.row.sel') || document.querySelector('.row'))?.focus(); return; }
  if (a && a.closest('.listcol') && S.q) { S.q = ''; $('#q').value = ''; renderList(); document.querySelector('.row.sel')?.focus(); return; }
  if (a && a.closest('.listcol')) { (document.querySelector('.nav.sel') || $('#q')).focus(); return; }
}

document.addEventListener('keydown', e => {
  S.pad = false;
  const t = e.target;
  const typing = (t.tagName === 'INPUT' && t.type !== 'range') || t.tagName === 'TEXTAREA';
  const k = e.key, ctrl = e.ctrlKey || e.metaKey;
  if (k === 'Escape') { e.preventDefault(); if (t.id === 'q' && S.q) { S.q = ''; t.value = ''; renderList(); return; } back(); return; }
  if (ctrl && k.toLowerCase() === 's' && S.edit) { e.preventDefault(); saveEdit(); return; }
  if (ctrl && k.toLowerCase() === 'n') { e.preventDefault(); if (!S.edit && !$('#layer').firstChild) startEdit(true); return; }
  if (ctrl && k.toLowerCase() === 'f') { e.preventDefault(); if (!$('#layer').firstChild && !S.edit) { $('#q').focus(); $('#q').select(); } return; }
  if (ctrl && k.toLowerCase() === 'i') { e.preventDefault(); if (!S.edit) startImport(); return; }
  if (ctrl && k.toLowerCase() === 'e') { e.preventDefault(); if (!S.edit) startExport(!!current()); return; }
  if (k === 'F5') { e.preventDefault(); runSync('', true); return; }
  if (t.id === 'newgroup' && k === 'Enter') {
    e.preventDefault(); const g = t.value.trim();
    if (g && !S.edit.c.groups.includes(g)) { S.edit.c.groups.push(g); renderEditor(); $('#newgroup')?.focus(); }
    return;
  }
  if (t.id === 'q' && (k === 'Enter' || k === 'ArrowDown')) { e.preventDefault(); const first = document.querySelector('.row'); if (first) { if (k === 'Enter') first.click(); else first.focus(); } return; }
  if (typing) {
    if (t.tagName === 'TEXTAREA') return;
    if (k === 'ArrowLeft' || k === 'ArrowRight') return;
    if (k === 'Enter') { e.preventDefault(); if ($('#a-pass') === t) { $('#a-go')?.click(); return; } if ($('#re-pass') === t) { $('[data-dlg="repass"]')?.click(); return; } move('down'); return; }
  }
  if (!typing && !$('#layer').firstChild && !S.edit) {
    if (k === '/' ) { e.preventDefault(); $('#q').focus(); return; }
    if ((k === 'e' || k === 'E') && current()) { e.preventDefault(); startEdit(false); return; }
    if (k === 'Delete' && current()) { e.preventDefault(); askDelete(); return; }
  }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k]) { e.preventDefault(); move(dirs[k]); }
});

// ---------------------------------------------------------------- moving around: D-pad, stick, arrow keys
function focusables() {
  const root = $('#layer').firstChild || document;
  return [...root.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null);
}
function move(dir) {
  const cur = document.activeElement;
  const all = focusables(); if (!all.length) return;
  if (!cur || !all.includes(cur)) { (document.querySelector('.row.sel') || all[0]).focus(); return; }
  // In a list, up and down go to the next item in it, even across letter headings.
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
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
}
document.addEventListener('focusin', e => {
  // Moving through the list with the D-pad or arrows shows each contact.
  const el = e.target;
  if (el.dataset?.sel && !S.edit && el.dataset.sel !== S.sel) { S.sel = el.dataset.sel; document.querySelectorAll('.row.sel').forEach(r => r.classList.remove('sel')); el.classList.add('sel'); renderDetail(); }
});
function choose() {
  const el = document.activeElement;
  if (!el || el === document.body) { focusables()[0]?.focus(); return; }
  if (el.dataset.osk && S.pad) { if (!oskTarget) openOsk(el); return; }
  if (el.tagName === 'BUTTON') el.click();
}

// The on-screen keyboard, for typing with a controller: Hearth's shared keyboard (osk/, docs/OSK.md); the small one
// below only if its files are missing.
let oskTarget = null, shift = false;
const OSK = ['1234567890', 'qwertyuiop', 'asdfghjkl\'', 'zxcvbnm,.@', '+-_/:#&()!'];
function openOsk(input) {
  oskTarget = input;
  if (window.HearthOSK) {
    const enter = () => { input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })); HearthOSK.close(); };
    HearthOSK.open({ target: input, mode: input.dataset.osk === 'search' ? 'search' : undefined, enterLabel: input.tagName === 'TEXTAREA' ? '' : 'Done', onEnter: enter, onClose: () => { oskTarget = null; } });
    return;
  }
  const rows = OSK.map(r => [...r].map(c => `<button class="f" data-key="${esc(c)}">${esc(c)}</button>`).join('')).join('');
  input.insertAdjacentHTML('afterend', `<div class="osk" id="osk">${rows}<button class="f wide" data-key="shift">Shift</button><button class="f wide" data-key="space">Space</button><button class="f wide" data-key="back">⌫</button><button class="f wide" data-key="clear">Clear</button><button class="f wide" data-key="done">Done</button></div>`);
  $('#osk [data-key="q"]').focus();
}
function closeOsk() { if (window.HearthOSK && HearthOSK.isOpen()) { HearthOSK.close(); return; } const t = oskTarget; $('#osk')?.remove(); oskTarget = null; t?.focus(); }
function oskPress(k) {
  const t = oskTarget; if (!t) return;
  if (k === 'done') return closeOsk();
  if (k === 'shift') { shift = !shift; document.querySelectorAll('#osk [data-key]').forEach(b => { if (b.dataset.key.length === 1) b.textContent = shift ? b.dataset.key.toUpperCase() : b.dataset.key; }); return; }
  if (k === 'back') t.value = t.value.slice(0, -1);
  else if (k === 'clear') t.value = '';
  else t.value += k === 'space' ? ' ' : (shift ? k.toUpperCase() : k);
  t.dispatchEvent(new Event('input', { bubbles: true }));
}

// Controllers (the standard mapping): D-pad or left stick moves, A chooses, B goes back, X new, Y search, Menu (Start) the main actions.
const held = {}; let lastPadMove = 0;
function pad() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const now = performance.now();
  const st = { up: false, down: false, left: false, right: false, a: false, b: false, x: false, y: false, menu: false };
  for (const g of pads) {
    const bt = i => g.buttons[i]?.pressed;
    const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
    st.up ||= bt(12) || ay < -.6; st.down ||= bt(13) || ay > .6; st.left ||= bt(14) || ax < -.6; st.right ||= bt(15) || ax > .6;
    st.a ||= bt(0); st.b ||= bt(1); st.x ||= bt(2); st.y ||= bt(3); st.menu ||= bt(9);
  }
  // The on-screen keyboard reads the controller itself while it's open (held buttons stay held for after).
  if (window.HearthOSK && HearthOSK.isOpen()) { for (const [k, on] of Object.entries(st)) held[k] = on ? (held[k] || now) : 0; requestAnimationFrame(pad); return; }
  for (const [k, on] of Object.entries(st)) {
    if (!on) { held[k] = 0; continue; }
    const first = !held[k];
    const repeat = held[k] && now - held[k] > 380 && now - lastPadMove > 110;
    if (first) held[k] = now;
    if (!(first || (repeat && ['up', 'down', 'left', 'right'].includes(k)))) continue;
    S.pad = true; lastPadMove = now;
    const free = !$('#layer').firstChild && !S.edit;
    if (k === 'a') choose(); else if (k === 'b') back();
    else if (k === 'x') { if (free) startEdit(true); }
    else if (k === 'y') { if (free) { $('#q').focus(); openOsk($('#q')); } }
    else if (k === 'menu') { if (free) showDialog(`<h2>Contacts</h2><div class="chips"><button class="chip f" data-act="import" data-autofocus>Import a .vcf file</button><button class="chip f" data-act="export">Export</button><button class="chip f" data-act="accounts">Accounts and sync</button></div><div class="drow"><button class="btn f" data-dlg="close">Close</button></div>`); }
    else move(k);
  }
  requestAnimationFrame(pad);
}
requestAnimationFrame(pad);
window.addEventListener('gamepadconnected', () => { $('#hint').textContent = 'Controller connected'; });

// ---------------------------------------------------------------- start
(async () => {
  await load();
  if (S.contacts.length && !S.sel) { S.sel = keyOf(visible()[0] || S.contacts[0]); renderList(); renderDetail(); }
  (document.querySelector('.row.sel') || $('#q')).focus();
  const file = await invoke('launch_file').catch(() => null);
  if (file) startImport(file);
  if (S.accounts.length) runSync('');
  // Sync every 15 minutes while open.
  setInterval(() => { if (!S.edit) runSync(''); }, 15 * 60 * 1000);
})();

// ---------------------------------------------------------------- without the app (opening the page in a browser): sample contacts
function demoInvoke(cmd, args) {
  const P = (given, family, extra) => Object.assign(blank(), { uid: `${given}-${family}`.toLowerCase(), name: { family, given, additional: '', prefix: '', suffix: '' } }, extra);
  demoInvoke.db ||= [
    { book: 'local', contact: P('Ada', 'Lovelace', { org: 'Analytical Engines', title: 'Programmer', phones: [{ kind: 'cell', value: '+44 20 7946 0018', pref: true }], emails: [{ kind: 'work', value: 'ada@example.org', pref: false }], birthday: '1815-12-10', groups: ['Pioneers'], addresses: [{ kind: 'home', pobox: '', ext: '', street: '12 St James\'s Square', locality: 'London', region: '', postcode: 'SW1Y 4JH', country: 'United Kingdom' }], note: 'Wrote the first program.' }) },
    { book: 'local', contact: P('Grace', 'Hopper', { phones: [{ kind: 'work', value: '+1 202 555 0147', pref: false }], groups: ['Pioneers'] }) },
    { book: 'dav-demo', contact: P('Linus', 'Torvalds', { emails: [{ kind: 'home', value: 'linus@example.fi', pref: false }] }) },
    { book: 'dav-demo', contact: P('Margaret', 'Hamilton', { org: 'MIT', phones: [{ kind: 'cell', value: '+1 617 555 0199', pref: false }], groups: ['Pioneers', 'Apollo'] }) },
  ];
  const db = demoInvoke.db;
  const books = [{ id: 'local', name: 'On this computer', kind: 'local', count: db.filter(e => e.book === 'local').length }, { id: 'dav-demo', name: 'Nextcloud · Contacts', kind: 'carddav', count: db.filter(e => e.book === 'dav-demo').length }];
  switch (cmd) {
    case 'state': return Promise.resolve({ books, contacts: [...db].sort((a, b) => shown(a.contact).localeCompare(shown(b.contact))), accounts: [{ id: 'dav-demo', name: 'Nextcloud', provider: 'nextcloud', server: 'https://cloud.example.com', username: 'you', bookName: 'Contacts', lastSync: '20261022T101010Z', lastError: '' }],
      presets: [{ id: 'nextcloud', name: 'Nextcloud', server: 'https://', userHint: 'User name', help: 'Your Nextcloud address.' }, { id: 'other', name: 'Other CardDAV server', server: 'https://', userHint: 'User name', help: '' }] });
    case 'save': { const c = args.contact; if (!c.uid) c.uid = String(Math.random()).slice(2); const i = db.findIndex(e => e.contact.uid === c.uid); if (i >= 0) db[i] = { book: args.book, contact: c }; else db.push({ book: args.book, contact: c }); return Promise.resolve(c); }
    case 'delete': demoInvoke.db = db.filter(e => e.contact.uid !== args.uid); return Promise.resolve();
    case 'launch_file': return Promise.resolve(null);
    case 'sync': return Promise.resolve([{ id: 'dav-demo', name: 'Nextcloud', report: { downloaded: 0, uploaded: 0, deletedHere: 0, deletedThere: 0, conflicts: 0 }, error: null }]);
    default: return Promise.reject('This is a preview: open Contacts to do that.');
  }
}
