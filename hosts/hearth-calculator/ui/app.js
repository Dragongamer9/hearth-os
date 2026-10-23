// Hearth Calculator's page: Standard (left to right, like Windows' Standard mode), Scientific (with precedence and
// brackets), Programmer (whole numbers, bases, bit operations) and Converter. The maths is engine.js.
'use strict';
const T = window.__TAURI__;
const invoke = T ? T.core.invoke : (c, a) => demoInvoke(c, a || {});
const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const { D, CalcError } = Calc;

// ---------------------------------------------------------------- keypads: [id, label, class]
const SYM = { '+': '+', '-': '−', '*': '×', '/': '÷', 'mod': 'mod', '^': '^', 'yroot': 'yroot', 'logy': 'log base', 'E': 'e', 'AND': 'AND', 'OR': 'OR', 'XOR': 'XOR', 'NAND': 'NAND', 'NOR': 'NOR', '<<': '<<', '>>': '>>', 'rol': 'RoL', 'ror': 'RoR', '%': 'mod' };
const PADS = {
  standard: { cols: 4, keys: [
    ['pct', '%', 'fn'], ['CE', 'CE', 'fn'], ['C', 'C', 'fn'], ['back', '⌫', 'fn'],
    ['inv', '¹⁄ₓ', 'fn'], ['sqr', 'x²', 'fn'], ['sqrt', '√x', 'fn'], ['/', '÷', 'op'],
    ['7', '7', 'num'], ['8', '8', 'num'], ['9', '9', 'num'], ['*', '×', 'op'],
    ['4', '4', 'num'], ['5', '5', 'num'], ['6', '6', 'num'], ['-', '−', 'op'],
    ['1', '1', 'num'], ['2', '2', 'num'], ['3', '3', 'num'], ['+', '+', 'op'],
    ['neg', '±', 'num'], ['0', '0', 'num'], ['dot', '.', 'num'], ['eq', '=', 'eq']] },
  scientific: { cols: 5, keys: [
    ['2nd', '2nd', 'fn'], ['pi', 'π', 'fn'], ['e', 'e', 'fn'], ['C', 'C', 'fn'], ['back', '⌫', 'fn'],
    ['sin', 'sin', 'fn', 'asin', 'sin⁻¹'], ['cos', 'cos', 'fn', 'acos', 'cos⁻¹'], ['tan', 'tan', 'fn', 'atan', 'tan⁻¹'], ['CE', 'CE', 'fn'], ['pct', '%', 'fn'],
    ['sqr', 'x²', 'fn', 'cube', 'x³'], ['inv', '¹⁄ₓ', 'fn'], ['abs', '|x|', 'fn', 'floor', '⌊x⌋'], ['E', 'exp', 'fn'], ['mod', 'mod', 'fn'],
    ['sqrt', '√x', 'fn', 'cbrt', '∛x'], ['(', '(', 'fn'], [')', ')', 'fn'], ['fact', 'n!', 'fn'], ['/', '÷', 'op'],
    ['^', 'xʸ', 'fn', 'yroot', 'ʸ√x'], ['7', '7', 'num'], ['8', '8', 'num'], ['9', '9', 'num'], ['*', '×', 'op'],
    ['pow10', '10ˣ', 'fn', 'pow2', '2ˣ'], ['4', '4', 'num'], ['5', '5', 'num'], ['6', '6', 'num'], ['-', '−', 'op'],
    ['log', 'log', 'fn', 'logy', 'logᵧx'], ['1', '1', 'num'], ['2', '2', 'num'], ['3', '3', 'num'], ['+', '+', 'op'],
    ['ln', 'ln', 'fn', 'exp', 'eˣ'], ['neg', '±', 'num'], ['0', '0', 'num'], ['dot', '.', 'num'], ['eq', '=', 'eq']] },
  programmer: { cols: 6, keys: [
    ['AND', 'AND', 'fn'], ['OR', 'OR', 'fn'], ['XOR', 'XOR', 'fn'], ['NOT', 'NOT', 'fn'], ['NAND', 'NAND', 'fn'], ['back', '⌫', 'fn'],
    ['NOR', 'NOR', 'fn'], ['rol', 'RoL', 'fn'], ['ror', 'RoR', 'fn'], ['<<', '<<', 'fn'], ['>>', '>>', 'fn'], ['C', 'AC', 'fn'],
    ['A', 'A', 'num'], ['B', 'B', 'num'], ['7', '7', 'num'], ['8', '8', 'num'], ['9', '9', 'num'], ['/', '÷', 'op'],
    ['hC', 'C', 'num'], ['D', 'D', 'num'], ['4', '4', 'num'], ['5', '5', 'num'], ['6', '6', 'num'], ['*', '×', 'op'],
    ['hE', 'E', 'num'], ['F', 'F', 'num'], ['1', '1', 'num'], ['2', '2', 'num'], ['3', '3', 'num'], ['-', '−', 'op'],
    ['(', '(', 'fn'], [')', ')', 'fn'], ['neg', '±', 'num'], ['0', '0', 'num'], ['%', 'mod', 'fn'], ['+', '+', 'op'],
    ['eq', '=', 'eq wide']] },
  convert: { cols: 4, keys: [
    ['7', '7', 'num'], ['8', '8', 'num'], ['9', '9', 'num'], ['back', '⌫', 'fn'],
    ['4', '4', 'num'], ['5', '5', 'num'], ['6', '6', 'num'], ['CE', 'CE', 'fn'],
    ['1', '1', 'num'], ['2', '2', 'num'], ['3', '3', 'num'], ['swap', '⇅', 'fn'],
    ['neg', '±', 'num'], ['0', '0', 'num'], ['dot', '.', 'num'], ['copy', 'Copy', 'fn']] },
};
const UNARY_LABEL = { inv: '1/', sqr: 'sqr', cube: 'cube', sqrt: '√', cbrt: '∛', abs: 'abs', floor: 'floor', fact: 'fact', pow10: '10^', pow2: '2^', exp: 'e^', ln: 'ln', log: 'log', sin: 'sin', cos: 'cos', tan: 'tan', asin: 'sin⁻¹', acos: 'cos⁻¹', atan: 'tan⁻¹', neg: 'negate', NOT: 'NOT' };
const DECIMAL_BINARY = ['+', '-', '*', '/', 'mod', '^', 'yroot', 'logy', 'E'];
const PROG_BINARY = ['+', '-', '*', '/', '%', 'AND', 'OR', 'XOR', 'NAND', 'NOR', '<<', '>>', 'rol', 'ror'];

// ---------------------------------------------------------------- state
const st = {
  mode: 'standard', entry: '0', typing: false, tokens: [], texts: [], opText: null, lastOp: null, lastB: null, error: null,
  second: false, angle: 'deg', base: 10, word: 'QWORD', afterParen: false, eqText: '', history: [], memory: [], side: 'history',
  conv: { cat: 'Length', from: 'in', to: 'cm' }, pad: false,
};
const prog = () => st.mode === 'programmer';
const bits = () => Calc.WORD[st.word];
const precedence = () => st.mode !== 'standard';

function entryValue() {
  // A result keeps all its digits (40), not just the 32 shown: 1 ÷ 3 × 3 is 1.
  if (!st.typing && st.val != null && st.valFor === st.entry && st.valMode === st.mode) return st.val;
  if (prog()) return Calc.parseBase(st.entry, st.base);
  return D.parse(st.entry === '-' || st.entry === '' ? '0' : st.entry);
}
function show(v) {
  if (prog()) return Calc.toBase(v, st.base, bits());
  return v.format();
}
function setResult(v) { st.entry = show(v); st.typing = false; st.val = v; st.valFor = st.entry; st.valMode = st.mode; }
function fail(e) {
  if (!(e instanceof CalcError)) console.error(e);
  st.error = e instanceof CalcError ? e.message : 'Something went wrong';
  st.tokens = []; st.texts = []; st.typing = false; st.opText = null; st.afterParen = false;
}
function clearAll() { st.entry = '0'; st.typing = false; st.tokens = []; st.texts = []; st.opText = null; st.error = null; st.afterParen = false; st.eqText = ''; st.lastOp = null; }
function operandText() { return st.opText || (prog() ? st.entry : fmtTyped(st.entry)); }

// ---------------------------------------------------------------- input
function digit(d) {
  if (st.error) clearAll();
  if (st.afterParen) { pushOp('*'); }
  if (st.eqText && !st.typing) st.eqText = '';
  const max = prog() ? bits() : 32;
  if (!st.typing) { st.entry = d === '.' ? '0.' : d; st.typing = true; st.opText = null; return; }
  if (d === '.' && (prog() || st.entry.includes('.'))) return;
  const digitsNow = st.entry.replace(/[^0-9A-F]/gi, '').length;
  if (d !== '.' && digitsNow >= max) return;
  if (st.entry === '0' && d !== '.') st.entry = d;
  else if (st.entry === '-0' && d !== '.') st.entry = '-' + d;
  else st.entry += d;
  // Programmer mode: no more digits than the word holds.
  if (prog()) { try { if (Calc.parseBase(st.entry, st.base) >= (1n << BigInt(bits()))) st.entry = st.entry.slice(0, -1) || '0'; } catch (_) {} }
}
const lastToken = () => st.tokens[st.tokens.length - 1];
const isOp = t => typeof t === 'string' && t !== '(' && t !== ')';
function pushOperand() {
  st.tokens.push(entryValue()); st.texts.push(operandText());
}
function pushOp(o) {
  if (st.error) return;
  if (st.afterParen) { st.afterParen = false; }
  else if (!st.typing && st.opText === null && isOp(lastToken()) && !st.eqJust) { st.tokens[st.tokens.length - 1] = o; st.texts[st.texts.length - 1] = SYM[o] || o; render(); return; }
  else pushOperand();
  st.eqJust = false;
  st.tokens.push(o); st.texts.push(SYM[o] || o);
  st.typing = false; st.opText = null; st.eqText = '';
  // Standard mode shows the running total (left to right).
  if (!precedence()) {
    try { setResult(Calc.evaluate(st.tokens.slice(0, -1), { precedence: false })); } catch (e) { fail(e); }
  }
}
function openParen() {
  if (st.error) clearAll();
  if (st.afterParen || (st.tokens.length && !isOp(lastToken()) && lastToken() !== '(')) pushOp('*');
  st.tokens.push('('); st.texts.push('(');
  st.typing = false; st.opText = null; st.entry = '0'; st.eqText = '';
}
function closeParen() {
  const open = st.tokens.filter(t => t === '(').length - st.tokens.filter(t => t === ')').length;
  if (open <= 0 || st.error) return;
  if (!st.afterParen) pushOperand();
  st.tokens.push(')'); st.texts.push(')');
  st.afterParen = true; st.typing = false; st.opText = null;
  // Show the bracket's value.
  try {
    let i = st.tokens.length - 1, depth = 0;
    for (; i >= 0; i--) { if (st.tokens[i] === ')') depth++; if (st.tokens[i] === '(') { depth--; if (depth === 0) break; } }
    const v = prog() ? Calc.pevaluate(st.tokens.slice(i + 1, -1), bits()) : Calc.evaluate(st.tokens.slice(i + 1, -1));
    st.entry = show(v);
  } catch (e) { fail(e); }
}
function evalTokens(tokens) { return prog() ? Calc.pevaluate(tokens, bits()) : Calc.evaluate(tokens, { precedence: precedence() }); }
function equals() {
  if (st.error) { clearAll(); return; }
  let text, result;
  try {
    if (!st.tokens.length) {
      if (st.lastOp != null) {
        const a = entryValue();
        result = prog() ? Calc.pbinary(st.lastOp, a, st.lastB, bits()) : Calc.binary(st.lastOp, a, st.lastB);
        text = `${operandText()} ${SYM[st.lastOp] || st.lastOp} ${show(st.lastB)} =`;
      } else { result = entryValue(); text = `${operandText()} =`; }
    } else {
      if (!st.afterParen) pushOperand();
      let open = st.tokens.filter(t => t === '(').length - st.tokens.filter(t => t === ')').length;
      while (open-- > 0) { st.tokens.push(')'); st.texts.push(')'); }
      // Repeating = repeats the last operation (5 + 3 = = gives 11).
      const n = st.tokens.length;
      if (n >= 2 && isOp(st.tokens[n - 2]) && st.tokens[n - 1] !== ')') { st.lastOp = st.tokens[n - 2]; st.lastB = st.tokens[n - 1]; } else st.lastOp = null;
      result = evalTokens(st.tokens);
      text = `${st.texts.join(' ')} =`;
    }
  } catch (e) { fail(e); render(); return; }
  setResult(result);
  st.tokens = []; st.texts = []; st.opText = null; st.afterParen = false; st.eqText = text; st.eqJust = true;
  addHistory(text, st.entry);
}
function unary(fn) {
  if (st.error) return;
  try {
    const v = st.afterParen ? (st.afterParen = false, popParenGroup()) : entryValue();
    let r;
    if (prog()) r = fn === 'NOT' ? Calc.wrap(~v, bits()) : Calc.wrap(-v, bits());
    else r = Calc.unary(fn, v, st.angle);
    const inner = st.parenText || operandText(); st.parenText = null;
    st.opText = fn === 'inv' ? `1/(${inner})` : `${UNARY_LABEL[fn] || fn}(${inner})`;
    setResult(r);
    if (st.eqText) st.eqText = '';
  } catch (e) { fail(e); }
}
// A bracket just closed, then a function: the function applies to the whole bracket.
function popParenGroup() {
  let i = st.tokens.length - 1, depth = 0;
  for (; i >= 0; i--) { if (st.tokens[i] === ')') depth++; if (st.tokens[i] === '(') { depth--; if (depth === 0) break; } }
  const group = st.tokens.slice(i + 1, -1);
  st.parenText = st.texts.slice(i + 1, -1).join(' ');
  st.tokens = st.tokens.slice(0, i); st.texts = st.texts.slice(0, i);
  return prog() ? Calc.pevaluate(group, bits()) : Calc.evaluate(group, { precedence: precedence() });
}
function percent() {
  if (st.error || prog()) return;
  try {
    const v = entryValue(); const op = lastToken();
    let r;
    if (st.tokens.length >= 2 && (op === '+' || op === '-')) r = Calc.evaluate(st.tokens.slice(0, -1), { precedence: precedence() }).mul(v).div(100);
    else r = v.div(100);
    st.opText = r.format(); setResult(r);
  } catch (e) { fail(e); }
}
function negate() {
  if (st.error) return;
  if (st.typing) { st.entry = st.entry.startsWith('-') ? st.entry.slice(1) : '-' + st.entry; return; }
  if (st.mode === 'convert') { st.entry = st.entry.startsWith('-') ? st.entry.slice(1) : (st.entry === '0' ? '0' : '-' + st.entry); return; }
  unary('neg');
}
function backspace() {
  if (st.error) { clearAll(); return; }
  if (st.typing) { st.entry = st.entry.slice(0, -1); if (st.entry === '' || st.entry === '-') { st.entry = '0'; st.typing = st.mode === 'convert'; } return; }
  if (st.mode === 'convert') { st.entry = st.entry.length > 1 ? st.entry.slice(0, -1) : '0'; st.typing = true; return; }
  if (st.eqText) st.eqText = '';
}
function clearEntry() { st.entry = '0'; st.typing = false; st.opText = null; st.error = null; if (st.eqJust) { st.eqText = ''; } }
function constant(c) { if (st.error) clearAll(); if (st.afterParen) pushOp('*'); setResult(c === 'pi' ? Calc.PI.round(32) : Calc.E.round(32)); st.opText = c === 'pi' ? 'π' : 'e'; st.eqText = ''; }

// Memory keys, as on Windows: MS stores, MR recalls, M+ and M− change the latest, MC clears.
function memory(k) {
  if (prog() || st.mode === 'convert') return;
  let v; try { v = entryValue(); } catch (_) { return; }
  if (st.error) return;
  if (k === 'MS') st.memory.unshift(v.format());
  else if (k === 'MR') { if (st.memory.length) { if (st.afterParen) pushOp('*'); st.entry = st.memory[0]; st.typing = false; st.opText = null; } }
  else if (k === 'M+' || k === 'M-') { const cur = st.memory.length ? D.parse(st.memory[0]) : D.zero(); const nv = k === 'M+' ? cur.add(v) : cur.sub(v); if (st.memory.length) st.memory[0] = nv.format(); else st.memory.unshift(nv.format()); }
  else if (k === 'MC') st.memory = [];
  st.typing = false;
  save(); renderSide();
}
function addHistory(expr, result) {
  st.history.unshift({ expr, result, mode: st.mode, base: st.base });
  if (st.history.length > 200) st.history.length = 200;
  save(); renderSide();
}

// ---------------------------------------------------------------- converter
function convRender() {
  const c = st.conv, cat = Calc.UNITS[c.cat];
  const name = id => cat.units.find(u => u[0] === id)?.[1] || id;
  let out = '';
  try { out = Calc.convert(c.cat, st.entry === '-' ? '0' : st.entry, c.from, c.to).format({ group: true }); } catch (e) { out = e.message; }
  $('#conv').innerHTML = `<button class="cat f" data-pick="cat">${esc(c.cat)} ▾</button>
    <div class="line"><span class="val on">${esc(fmtTyped(st.entry))}</span><button class="cat f" data-pick="from">${esc(name(c.from))} ▾</button></div>
    <div class="line"><span class="val" id="convOut">${esc(out)}</span><button class="cat f" data-pick="to">${esc(name(c.to))} ▾</button></div>`;
}
function pickList(title, items, cur, done) {
  const v = document.createElement('div');
  v.className = 'picker';
  v.style.cssText = 'position:fixed;inset:0;background:rgba(8,5,4,.7);display:grid;place-items:center;z-index:9';
  v.innerHTML = `<div style="background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:.8em;width:min(24em,92vw);max-height:86vh;overflow:auto"><h3 style="margin:.1em .3em .5em">${esc(title)}</h3>${items.map(([id, label]) => `<button class="f hi" style="display:block;width:100%;text-align:left" data-id="${esc(id)}">${id === cur ? '● ' : ''}${esc(label)}</button>`).join('')}</div>`;
  document.body.appendChild(v);
  const close = () => { v.remove(); $('#conv .cat')?.focus(); };
  v.addEventListener('click', e => { const b = e.target.closest('[data-id]'); if (b) { done(b.dataset.id); close(); render(); } else if (e.target === v) close(); });
  v.closePicker = close;
  ($(`[data-id="${CSS.escape(cur)}"]`, v) || $('.f', v)).focus();
}
$('#conv').addEventListener('click', e => {
  const b = e.target.closest('[data-pick]'); if (!b) return;
  const c = st.conv, cat = Calc.UNITS[c.cat];
  if (b.dataset.pick === 'cat') pickList('Convert', Object.keys(Calc.UNITS).map(k => [k, k]), c.cat, k => { c.cat = k; const u = Calc.UNITS[k].units; c.from = u[0][0]; c.to = u[Math.min(1, u.length - 1)][0]; save(); });
  else pickList(b.dataset.pick === 'from' ? 'From' : 'To', cat.units.map(u => [u[0], `${u[1]} (${u[0]})`]), c[b.dataset.pick], id => { c[b.dataset.pick] = id; save(); });
});

// ---------------------------------------------------------------- drawing
function fmtTyped(s) {
  if (st.mode === 'programmer') return s;
  const m = /^(-?)(\d*)(\.?)(\d*)(e.*)?$/.exec(s);
  if (!m || m[5]) return s;
  return `${m[1]}${m[2].replace(/\B(?=(\d{3})+(?!\d))/g, ',')}${m[3]}${m[4]}`;
}
function entryText() {
  if (st.error) return st.error;
  if (prog()) return Calc.groupBase(st.entry, st.base);
  if (st.typing) return fmtTyped(st.entry);
  try { return D.parse(st.entry).format({ group: true }); } catch (_) { return st.entry; }
}
function render() {
  document.body.dataset.calc = st.mode;
  for (const b of $$('[data-mode]')) b.classList.toggle('on', b.dataset.mode === st.mode);
  const e = $('#entry');
  const text = entryText();
  e.textContent = text;
  e.className = 'entry' + (st.error ? ' err' : text.length > 26 ? ' tiny' : text.length > 16 ? ' small' : '');
  $('#expr').textContent = st.mode === 'convert' ? '' : st.eqText || [...st.texts, ...(st.opText && !st.typing ? [st.opText] : [])].join(' ');
  $('#expr').hidden = st.mode === 'convert';
  e.hidden = st.mode === 'convert';
  $('#bases').hidden = !prog();
  $('#conv').hidden = st.mode !== 'convert';
  $('#angleRow').hidden = st.mode !== 'scientific';
  $('#wordRow').hidden = !prog();
  $('#angleBtn').textContent = st.angle.toUpperCase();
  $('#wordBtn').textContent = st.word;
  for (const m of $$('#memrow [data-k^="M"]')) { m.hidden = prog() || st.mode === 'convert'; m.disabled = (m.dataset.k === 'MC' || m.dataset.k === 'MR') && !st.memory.length; }
  if (prog()) {
    let v = 0n; try { v = Calc.parseBase(st.entry, st.base); } catch (_) {}
    $('#bases').innerHTML = [[16, 'HEX'], [10, 'DEC'], [8, 'OCT'], [2, 'BIN']].map(([b, n]) => `<button class="f ${b === st.base ? 'on' : ''}" data-base="${b}"><span class="bn">${n}</span><span class="bv">${esc(st.error ? '' : Calc.groupBase(Calc.toBase(v, b, bits()), b))}</span></button>`).join('');
    // Digits the base doesn't have are greyed out.
    for (const k of $$('#pad [data-key]')) {
      const d = keyDigit(k.dataset.key);
      if (d != null) k.disabled = parseInt(d, 16) >= st.base;
    }
  }
  if (st.mode === 'convert') convRender();
  if (st.mode === 'scientific') for (const k of $$('#pad [data-key]')) { const def = PADS.scientific.keys.find(x => x[0] === k.dataset.key); if (def && def[3]) k.innerHTML = st.second ? def[4] : def[1]; if (k.dataset.key === '2nd') k.classList.toggle('pressed', st.second); }
}
function keyDigit(id) { if (/^[0-9]$/.test(id)) return id; if (/^[ABDF]$/.test(id)) return id; if (id === 'hC') return 'C'; if (id === 'hE') return 'E'; return null; }
function buildPad() {
  const p = PADS[st.mode];
  const pad = $('#pad');
  pad.style.gridTemplateColumns = `repeat(${p.cols}, 1fr)`;
  pad.innerHTML = p.keys.map(([id, label, cls]) => `<button class="k f ${cls}" data-key="${esc(id)}" ${cls.includes('wide') ? `style="grid-column:span ${p.cols}"` : ''}>${label}</button>`).join('');
}
function renderSide() {
  for (const c of $$('[data-side]')) c.classList.toggle('on', c.dataset.side === st.side);
  const l = $('#sidelist');
  if (st.side === 'history') {
    l.innerHTML = st.history.length ? st.history.map((h, i) => `<button class="hi f" data-hist="${i}"><div class="he">${esc(h.expr)}</div><div class="hr">${esc(h.mode === 'programmer' ? h.result : fmtTyped(h.result))}</div></button>`).join('') : '<div class="empty">There\'s no history yet.</div>';
  } else {
    l.innerHTML = st.memory.length ? st.memory.map((m, i) => `<div class="memitem"><button class="mv f" data-mem="${i}" title="Use this number">${esc(fmtTyped(m))}</button><button class="mk f" data-memop="MC" data-i="${i}">MC</button><button class="mk f" data-memop="M+" data-i="${i}">M+</button><button class="mk f" data-memop="M-" data-i="${i}">M−</button></div>`).join('') : '<div class="empty">There\'s nothing saved in memory.</div>';
  }
}
function setMode(m) {
  if (m === st.mode) return;
  const keep = (() => { try { return st.mode === 'programmer' ? D.of(Calc.parseBase(st.entry, st.base)) : st.mode === 'convert' ? D.parse(st.entry) : D.parse(st.entry); } catch (_) { return D.zero(); } })();
  st.mode = m; st.second = false;
  clearAll();
  if (m === 'programmer') { st.entry = Calc.toBase(Calc.wrap(keep.toInt(), bits()), st.base, bits()); }
  else st.entry = keep.format();
  buildPad(); render(); save();
}
function setBase(b) {
  let v = 0n; try { v = Calc.parseBase(st.entry, st.base); } catch (_) {}
  st.base = b; st.entry = Calc.toBase(v, b, bits()); st.typing = false;
  // Numbers already in the expression keep their values; only how they're shown changes.
  st.texts = st.tokens.map((t, i) => (typeof t === 'bigint' ? Calc.toBase(t, b, bits()) : st.texts[i]));
  render(); save();
}

// ---------------------------------------------------------------- keys and buttons
function press(id) {
  st.error && !['C', 'CE', 'back'].includes(id) && /^[0-9A-F.]$|^(dot|hC|hE)$/.test(id) && clearAll();
  const k2 = st.mode === 'scientific' && st.second ? (PADS.scientific.keys.find(x => x[0] === id)?.[3] || id) : id;
  if (st.mode === 'scientific' && st.second && k2 !== id) st.second = false;
  const d = prog() ? keyDigit(k2) : (/^[0-9]$/.test(k2) ? k2 : null);
  if (d != null) { if (!prog() || parseInt(d, 16) < st.base) digit(d); }
  else if (k2 === 'dot') digit('.');
  else if ((prog() ? PROG_BINARY : DECIMAL_BINARY).includes(k2)) pushOp(k2);
  else switch (k2) {
    case 'eq': if (st.mode !== 'convert') equals(); break;
    case 'C': clearAll(); break;
    case 'CE': clearEntry(); break;
    case 'back': backspace(); break;
    case 'neg': negate(); break;
    case 'pct': percent(); break;
    case '(': openParen(); break;
    case ')': closeParen(); break;
    case 'pi': case 'e': constant(k2); break;
    case '2nd': st.second = !st.second; break;
    case 'swap': { const c = st.conv; [c.from, c.to] = [c.to, c.from]; save(); break; }
    case 'copy': copy(); break;
    default: if (UNARY_LABEL[k2]) unary(k2);
  }
  render();
}
document.addEventListener('click', e => {
  const k = e.target.closest('[data-key]'); if (k) { flash(k); press(k.dataset.key); return; }
  const m = e.target.closest('[data-mode]'); if (m) { setMode(m.dataset.mode); return; }
  const mk = e.target.closest('#memrow [data-k]');
  if (mk) {
    if (mk.dataset.k === 'angle') { st.angle = { deg: 'rad', rad: 'grad', grad: 'deg' }[st.angle]; save(); render(); return; }
    if (mk.dataset.k === 'word') { const w = ['QWORD', 'DWORD', 'WORD', 'BYTE']; st.word = w[(w.indexOf(st.word) + 1) % 4]; let v = 0n; try { v = Calc.parseBase(st.entry, st.base); } catch (_) {} st.entry = Calc.toBase(Calc.wrap(v, bits()), st.base, bits()); save(); render(); return; }
    memory(mk.dataset.k); render(); return;
  }
  const b = e.target.closest('[data-base]'); if (b) { setBase(+b.dataset.base); return; }
  const s = e.target.closest('[data-side]'); if (s) { st.side = s.dataset.side; renderSide(); return; }
  const h = e.target.closest('[data-hist]');
  if (h) { const it = st.history[+h.dataset.hist]; if (it.mode !== st.mode) setMode(it.mode); if (prog() && it.base !== st.base) setBase(it.base); st.entry = it.result; st.typing = false; st.opText = null; st.eqText = it.expr; render(); return; }
  const mv = e.target.closest('[data-mem]'); if (mv) { if (prog() || st.mode === 'convert') return; st.entry = st.memory[+mv.dataset.mem]; st.typing = false; st.opText = null; render(); return; }
  const mo = e.target.closest('[data-memop]');
  if (mo) {
    const i = +mo.dataset.i;
    if (mo.dataset.memop === 'MC') st.memory.splice(i, 1);
    else { try { const v = entryValue(); const cur = D.parse(st.memory[i]); st.memory[i] = (mo.dataset.memop === 'M+' ? cur.add(v) : cur.sub(v)).format(); } catch (_) {} }
    save(); renderSide(); render(); return;
  }
  const a = e.target.closest('[data-act]');
  if (a?.dataset.act === 'side') toggleSide();
  if (a?.dataset.act === 'clearSide') { if (st.side === 'history') st.history = []; else st.memory = []; save(); renderSide(); render(); }
});
function toggleSide() {
  const small = window.matchMedia('(max-width: 720px)').matches;
  if (small) document.body.classList.toggle('side-open'); else document.body.classList.toggle('side-closed');
  $('#histBtn').classList.toggle('on', small ? document.body.classList.contains('side-open') : !document.body.classList.contains('side-closed'));
  st.sideClosed = document.body.classList.contains('side-closed'); save();
}
function flash(k) { k.classList.add('pressed'); setTimeout(() => k.classList.remove('pressed'), 120); }
function flashKey(id) { const k = $(`#pad [data-key="${CSS.escape(id)}"]`); if (k) flash(k); }
let toastTimer;
function toast(t) { const el = $('#toast'); el.textContent = t; el.hidden = false; clearTimeout(toastTimer); toastTimer = setTimeout(() => { el.hidden = true; }, 1800); }
async function copy() {
  let text = st.mode === 'convert' ? ($('#convOut')?.textContent || '').replace(/,/g, '') : st.error ? '' : st.entry;
  if (!text) return;
  try { await invoke('clipboard_write', { text }); toast('Copied'); } catch (e) { toast(String(e)); }
}
async function paste() {
  let text = '';
  try { text = await invoke('clipboard_read'); } catch (_) { return; }
  text = String(text || '').trim().replace(/[\s,_]/g, '').replace(/^\+/, '');
  try {
    if (prog()) { const v = Calc.parseBase(text.replace(/^0[xob]/i, ''), st.base); st.entry = Calc.toBase(v, st.base, bits()); }
    else { st.entry = D.parse(text).format(); }
    st.typing = st.mode === 'convert'; st.opText = null; st.error = null; if (st.afterParen) st.afterParen = false;
    render();
  } catch (_) { toast('That isn\'t a number'); }
}
document.addEventListener('keydown', e => {
  st.pad = false; document.body.classList.remove('pad');
  if ($('.picker')) { if (e.key === 'Escape') { $('.picker').closePicker(); e.preventDefault(); } else if (/^Arrow/.test(e.key)) { e.preventDefault(); move(e.key.slice(5).toLowerCase()); } return; }
  const k = e.key, ctrl = e.ctrlKey || e.metaKey;
  if (ctrl) {
    const map = { c: copy, v: paste, h: toggleSide, m: () => memory('MS'), r: () => memory('MR'), p: () => memory('M+'), q: () => memory('M-'), l: () => memory('MC'), d: () => { st.history = []; save(); renderSide(); } };
    const f = map[k.toLowerCase()]; if (f) { e.preventDefault(); f(); render(); }
    return;
  }
  if (e.altKey && /^[1-4]$/.test(k)) { e.preventDefault(); setMode(['standard', 'scientific', 'programmer', 'convert'][+k - 1]); return; }
  const focus = document.activeElement;
  if (k === 'Enter' && focus && focus.matches('button.f') && !focus.closest('#pad') && st.pad) return;
  let id = null;
  if (/^[0-9]$/.test(k)) id = k;
  else if (prog() && /^[a-f]$/i.test(k) && !(k.toLowerCase() === 'c' && false)) id = { c: 'hC', e: 'hE' }[k.toLowerCase()] || k.toUpperCase();
  else if (k === '.' || k === ',') id = 'dot';
  else if (k === '+') id = '+';
  else if (k === '-') id = '-';
  else if (k === '*' || k === 'x' || k === 'X') id = '*';
  else if (k === '/') id = '/';
  else if (k === 'Enter' || k === '=') id = 'eq';
  else if (k === 'Backspace') id = 'back';
  else if (k === 'Escape') id = 'C';
  else if (k === 'Delete') id = 'CE';
  else if (k === 'F9') id = 'neg';
  else if (k === '(' || k === ')') id = k;
  else if (k === '%') id = prog() ? '%' : 'pct';
  else if (!prog()) id = { r: 'inv', q: 'sqr', '@': 'sqrt', '^': '^', '!': 'fact', s: 'sin', o: 'cos', t: 'tan', l: 'log', n: 'ln', p: 'pi', m: 'mod', y: '^' }[k] || null;
  else id = { '&': 'AND', '|': 'OR', '^': 'XOR', '~': 'NOT', '<': '<<', '>': '>>' }[k] || null;
  if (st.mode !== 'scientific' && ['sin', 'cos', 'tan', 'log', 'ln', 'pi', 'mod', '^', 'fact', '(', ')'].includes(id) && !prog()) id = null;
  if (st.mode === 'convert' && id && !/^[0-9]$|^(dot|back|CE|neg|C)$/.test(id)) id = id === 'C' ? 'CE' : null;
  if (id === 'C' && st.mode === 'convert') id = 'CE';
  if (id) { e.preventDefault(); flashKey(id); press(id); return; }
  const dirs = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
  if (dirs[k]) { e.preventDefault(); move(dirs[k]); }
});

// ---------------------------------------------------------------- controller and arrow keys
function focusables() { const root = $('.picker') || document; return [...root.querySelectorAll('.f')].filter(el => !el.disabled && el.offsetParent !== null); }
function move(dir) {
  const all = focusables(); if (!all.length) return;
  const cur = document.activeElement;
  if (!cur || !all.includes(cur)) { ($('#pad [data-key="5"]') || all[0]).focus(); return; }
  const a = cur.getBoundingClientRect(); const ax = a.left + a.width / 2, ay = a.top + a.height / 2;
  let best = null, bestScore = Infinity;
  for (const el of all) {
    if (el === cur) continue;
    const b = el.getBoundingClientRect(); const bx = b.left + b.width / 2, by = b.top + b.height / 2;
    const ok = dir === 'left' ? b.right <= a.left + 4 : dir === 'right' ? b.left >= a.right - 4 : dir === 'up' ? b.bottom <= a.top + 4 : b.top >= a.bottom - 4;
    if (!ok) continue;
    const main = dir === 'left' || dir === 'right' ? Math.abs(bx - ax) : Math.abs(by - ay);
    const cross = dir === 'left' || dir === 'right' ? Math.abs(by - ay) : Math.abs(bx - ax);
    const s = main + cross * 2.2;
    if (s < bestScore) { bestScore = s; best = el; }
  }
  if (best) { best.focus(); best.scrollIntoView({ block: 'nearest' }); }
}
// D-pad/stick moves, A presses, B deletes, X clears, Y equals, LB/RB switch mode, View shows history, Menu copies.
const held = {}; let lastPadMove = 0;
function pad() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const now = performance.now();
  const s = { up: 0, down: 0, left: 0, right: 0, a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0, select: 0, start: 0 };
  for (const g of pads) {
    const bt = i => g.buttons[i]?.pressed; const ax = g.axes[0] || 0, ay = g.axes[1] || 0;
    s.up ||= bt(12) || ay < -.6; s.down ||= bt(13) || ay > .6; s.left ||= bt(14) || ax < -.6; s.right ||= bt(15) || ax > .6;
    s.a ||= bt(0); s.b ||= bt(1); s.x ||= bt(2); s.y ||= bt(3); s.lb ||= bt(4); s.rb ||= bt(5); s.select ||= bt(8); s.start ||= bt(9);
  }
  for (const [k, on] of Object.entries(s)) {
    if (!on) { held[k] = 0; continue; }
    const first = !held[k];
    const rep = held[k] && now - held[k] > 380 && now - lastPadMove > 100 && ['up', 'down', 'left', 'right', 'b'].includes(k);
    if (first) held[k] = now;
    if (!first && !rep) continue;
    lastPadMove = now; st.pad = true; document.body.classList.add('pad');
    if (['up', 'down', 'left', 'right'].includes(k)) move(k);
    else if (k === 'a') { const el = document.activeElement; if (el && el.matches('button')) el.click(); else ($('#pad [data-key="5"]'))?.focus(); }
    else if (k === 'b') { if ($('.picker')) $('.picker').closePicker(); else { flashKey('back'); press('back'); } }
    else if (k === 'x') { const id = st.mode === 'convert' ? 'CE' : 'C'; flashKey(id); press(id); }
    else if (k === 'y') { flashKey('eq'); press('eq'); }
    else if (k === 'lb' || k === 'rb') { const m = ['standard', 'scientific', 'programmer', 'convert']; setMode(m[(m.indexOf(st.mode) + (k === 'rb' ? 1 : -1) + 4) % 4]); $('#pad [data-key="5"]')?.focus(); }
    else if (k === 'select') toggleSide();
    else if (k === 'start') copy();
  }
  requestAnimationFrame(pad);
}

// ---------------------------------------------------------------- saved state (history, memory, mode)
let saveTimer;
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke('store_set', { value: { mode: st.mode, angle: st.angle, base: st.base, word: st.word, history: st.history, memory: st.memory, conv: st.conv, sideClosed: !!st.sideClosed } }).catch(() => {}), 300);
}
(async function start() {
  let saved = null;
  try { saved = await invoke('store_get'); } catch (_) {}
  if (saved && typeof saved === 'object') {
    st.angle = saved.angle || 'deg'; st.base = saved.base || 10; st.word = saved.word || 'QWORD';
    st.history = Array.isArray(saved.history) ? saved.history : []; st.memory = Array.isArray(saved.memory) ? saved.memory : [];
    if (saved.conv && Calc.UNITS[saved.conv.cat]) st.conv = saved.conv;
    st.mode = PADS[saved.mode] ? saved.mode : 'standard';
    if (saved.sideClosed) { document.body.classList.add('side-closed'); st.sideClosed = true; }
  }
  $('#histBtn').classList.toggle('on', !st.sideClosed && !window.matchMedia('(max-width: 720px)').matches);
  buildPad(); render(); renderSide();
  requestAnimationFrame(pad);
  document.body.dataset.ready = '1';
})();

function demoInvoke(cmd, a) {
  const s = (window.__demoStore ||= {});
  if (cmd === 'store_get') return Promise.resolve(s.calc || null);
  if (cmd === 'store_set') { s.calc = a.value; return Promise.resolve(null); }
  if (cmd === 'clipboard_write') { window.__clip = a.text; return Promise.resolve(null); }
  if (cmd === 'clipboard_read') return Promise.resolve(window.__clip || '');
  return Promise.resolve(null);
}
