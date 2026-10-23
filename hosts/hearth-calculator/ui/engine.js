// Hearth Calculator's arithmetic. Numbers are decimals (a BigInt and a power of ten), not binary floats, so
// 0.1 + 0.2 is 0.3 and money adds up. + − × are exact; ÷ and √ keep 40 significant digits; sin, ln and the
// other functions come from the platform's maths (about 16 digits) and are rounded to 15 so sin 30° is 0.5.
// Programmer mode uses whole numbers of a chosen width (8 to 64 bits, two's complement). Used by app.js;
// tests/engine.test.js runs it in Node.
'use strict';
(function (glob) {
  const PREC = 40;           // significant digits kept for ÷, √ and friends
  const SHOW = 32;           // significant digits shown
  const B10 = 10n;
  const pow10 = n => B10 ** BigInt(n);
  const babs = x => (x < 0n ? -x : x);
  const digits = x => (x === 0n ? 1 : babs(x).toString().length);

  class CalcError extends Error {}

  // value = m × 10^e
  class D {
    constructor(m, e = 0) { this.m = BigInt(m); this.e = e; }
    static zero() { return new D(0n, 0); }
    static of(x) {
      if (x instanceof D) return x;
      if (typeof x === 'bigint') return new D(x, 0);
      if (typeof x === 'number') return D.fromNumber(x);
      return D.parse(String(x));
    }
    static parse(s) {
      s = String(s).trim().replace(/[,_\s]/g, '').replace(/^\+/, '');
      const m = /^(-?)(\d*)(?:\.(\d*))?(?:e([+-]?\d+))?$/i.exec(s);
      if (!m || (m[2] === '' && (m[3] || '') === '')) throw new CalcError('Not a number');
      const int = m[2] || '0', frac = m[3] || '';
      let mant = BigInt(int + frac);
      if (m[1]) mant = -mant;
      return new D(mant, -frac.length + (m[4] ? parseInt(m[4], 10) : 0)).norm();
    }
    // A float from the platform's maths, rounded to 15 significant digits (hides 0.49999999999999994).
    static fromNumber(x, sig = 15) {
      if (!isFinite(x)) throw new CalcError(isNaN(x) ? 'Result is undefined' : 'Overflow');
      if (x === 0) return D.zero();
      return D.parse(x.toPrecision(sig));
    }
    norm() {
      if (this.m === 0n) return new D(0n, 0);
      let m = this.m, e = this.e;
      while (m % B10 === 0n) { m /= B10; e++; }
      return new D(m, e);
    }
    isZero() { return this.m === 0n; }
    sign() { return this.m < 0n ? -1 : this.m > 0n ? 1 : 0; }
    neg() { return new D(-this.m, this.e); }
    abs() { return new D(babs(this.m), this.e); }
    isInt() { return this.norm().e >= 0; }
    // Rounded to `sig` significant digits (half to even).
    round(sig = PREC) {
      const n = digits(this.m);
      if (n <= sig) return this.norm();
      const drop = n - sig;
      return new D(divRound(this.m, pow10(drop)), this.e + drop).norm();
    }
    // Rounded to `places` decimal places.
    toFixedPlaces(places) {
      if (-this.e <= places) return this.norm();
      const drop = -this.e - places;
      return new D(divRound(this.m, pow10(drop)), -places).norm();
    }
    add(b) { b = D.of(b); const e = Math.min(this.e, b.e); return new D(this.m * pow10(this.e - e) + b.m * pow10(b.e - e), e).norm(); }
    sub(b) { return this.add(D.of(b).neg()); }
    mul(b) { b = D.of(b); return new D(this.m * b.m, this.e + b.e).norm(); }
    div(b, prec = PREC) {
      b = D.of(b);
      if (b.isZero()) throw new CalcError(this.isZero() ? 'Result is undefined' : 'Cannot divide by zero');
      if (this.isZero()) return D.zero();
      // Scale so the quotient has prec+2 digits, then round.
      const shift = Math.max(0, prec + 2 - digits(this.m) + digits(b.m));
      const q = divRound(this.m * pow10(shift), b.m);
      return new D(q, this.e - b.e - shift).round(prec);
    }
    cmp(b) { const d = this.sub(b); return d.sign(); }
    eq(b) { return this.cmp(b) === 0; }
    toInt() { // truncated toward zero
      const n = this.norm();
      return n.e >= 0 ? n.m * pow10(n.e) : n.m / pow10(-n.e);
    }
    toNumber() { return Number(this.m) * Math.pow(10, this.e) || parseFloat(`${this.m}e${this.e}`); }
    // For display: up to SHOW significant digits, plain when it fits, else 1.2345e+40.
    format({ sig = SHOW, group = false, maxPlain = 21 } = {}) {
      const r = this.round(sig);
      if (r.isZero()) return '0';
      const neg = r.m < 0n;
      const ds = babs(r.m).toString();
      const exp10 = ds.length - 1 + r.e; // position of the first digit
      let out;
      if (exp10 >= maxPlain || exp10 < -8) {
        const mant = ds.length > 1 ? `${ds[0]}.${ds.slice(1)}` : ds;
        out = `${mant}e${exp10 >= 0 ? '+' : ''}${exp10}`;
      } else if (r.e >= 0) {
        out = ds + '0'.repeat(r.e);
        if (group) out = groupInt(out);
      } else {
        const pt = ds.length + r.e;
        let int, frac;
        if (pt > 0) { int = ds.slice(0, pt); frac = ds.slice(pt); }
        else { int = '0'; frac = '0'.repeat(-pt) + ds; }
        out = `${group ? groupInt(int) : int}.${frac}`;
      }
      return (neg ? '-' : '') + out;
    }
    toString() { return this.format(); }
  }
  function divRound(a, b) { // round half to even
    let q = a / b, r = a % b;
    if (r === 0n) return q;
    const twice = babs(r) * 2n, bb = babs(b);
    const away = (a < 0n) !== (b < 0n) ? -1n : 1n;
    if (twice > bb || (twice === bb && q % 2n !== 0n)) q += away;
    return q;
  }
  const groupInt = s => s.replace(/\B(?=(\d{3})+(?!\d))/g, ',');

  // ---------------------------------------------------------------- functions
  const PI = D.parse('3.14159265358979323846264338327950288419716939937510');
  const E = D.parse('2.71828182845904523536028747135266249775724709369995');
  function sqrt(x) {
    x = D.of(x);
    if (x.sign() < 0) throw new CalcError('Invalid input');
    if (x.isZero()) return D.zero();
    // Integer square root of m × 10^(2k) for enough digits.
    let m = x.m, e = x.e;
    if ((e % 2 + 2) % 2 !== 0) { m *= B10; e -= 1; }
    const want = PREC + 2;
    let extra = Math.max(0, want * 2 - digits(m));
    if (extra % 2) extra++;
    m *= pow10(extra); e -= extra;
    let r = BigInt(Math.floor(Math.sqrt(Number(m)))) || 1n;
    if (digits(m) > 300) r = 1n << BigInt(Math.ceil(m.toString(2).length / 2));
    for (;;) { const n = (r + m / r) >> 1n; if (babs(n - r) <= 1n) { r = n; break; } r = n; }
    while (r * r > m) r--;
    while ((r + 1n) * (r + 1n) <= m) r++;
    return new D(r, e / 2).round(PREC);
  }
  function powInt(x, n) {
    x = D.of(x);
    if (n < 0n) return D.of(1).div(powInt(x, -n));
    if (n > 100000n) throw new CalcError('Overflow');
    let r = D.of(1), b = x, k = n;
    while (k > 0n) { if (k & 1n) r = r.mul(b).round(PREC + 20); b = b.mul(b).round(PREC + 20); k >>= 1n; }
    return r.round(PREC + 10);
  }
  function pow(x, y) {
    x = D.of(x); y = D.of(y);
    if (y.isInt()) {
      if (x.isZero() && y.sign() < 0) throw new CalcError('Cannot divide by zero');
      return powInt(x, y.toInt());
    }
    if (x.sign() < 0) {
      // Odd roots of negatives: (-8)^(1/3) = -2.
      const inv = D.of(1).div(y);
      if (inv.round(12).isInt() && inv.round(12).toInt() % 2n !== 0n) return pow(x.neg(), y).neg();
      throw new CalcError('Invalid input');
    }
    return D.fromNumber(Math.pow(x.toNumber(), y.toNumber()));
  }
  function root(x, n) { // n-th root
    x = D.of(x); n = D.of(n);
    if (n.isZero()) throw new CalcError('Invalid input');
    if (n.eq(2)) return sqrt(x);
    if (x.sign() < 0) {
      if (n.isInt() && n.toInt() % 2n !== 0n) return root(x.neg(), n).neg();
      throw new CalcError('Invalid input');
    }
    const r = D.fromNumber(Math.pow(x.toNumber(), 1 / n.toNumber()));
    // Exact when it should be (∛27 = 3).
    const rr = r.round(12);
    if (n.isInt() && powInt(rr, n.toInt()).eq(x)) return rr;
    return r;
  }
  function factorial(x) {
    x = D.of(x);
    if (x.isInt()) {
      const n = x.toInt();
      if (n < 0n) throw new CalcError('Invalid input');
      if (n > 3000n) throw new CalcError('Overflow');
      let r = 1n; for (let i = 2n; i <= n; i++) r *= i;
      return new D(r, 0).round(PREC + 10);
    }
    return D.fromNumber(gamma(x.toNumber() + 1));
  }
  function gamma(z) {
    if (z < 0.5) return Math.PI / (Math.sin(Math.PI * z) * gamma(1 - z));
    const g = 7, c = [0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059, 12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];
    z -= 1; let x = c[0];
    for (let i = 1; i < g + 2; i++) x += c[i] / (z + i);
    const t = z + g + 0.5;
    return Math.sqrt(2 * Math.PI) * Math.pow(t, z + 0.5) * Math.exp(-t) * x;
  }
  const ANGLE = { deg: Math.PI / 180, rad: 1, grad: Math.PI / 200 };
  function trig(fn, x, unit = 'deg') {
    x = D.of(x);
    if (['sin', 'cos', 'tan'].includes(fn) && unit !== 'rad') {
      // Exact values at the usual angles (and tan 90° is undefined, not 16331239353195370).
      const full = unit === 'deg' ? 360n : 400n;
      const q = unit === 'deg' ? 90n : 100n;
      const xm = x.isInt() ? ((x.toInt() % full) + full) % full : null;
      if (xm !== null && xm % q === 0n) {
        const k = Number(xm / q);
        const sin = [0, 1, 0, -1][k], cos = [1, 0, -1, 0][k];
        if (fn === 'sin') return D.of(sin);
        if (fn === 'cos') return D.of(cos);
        if (cos === 0) throw new CalcError('Invalid input');
        return D.of(sin / cos);
      }
    }
    const v = x.toNumber();
    let r;
    switch (fn) {
      case 'sin': r = Math.sin(v * ANGLE[unit]); break;
      case 'cos': r = Math.cos(v * ANGLE[unit]); break;
      case 'tan': r = Math.tan(v * ANGLE[unit]); break;
      case 'asin': if (Math.abs(v) > 1) throw new CalcError('Invalid input'); r = Math.asin(v) / ANGLE[unit]; break;
      case 'acos': if (Math.abs(v) > 1) throw new CalcError('Invalid input'); r = Math.acos(v) / ANGLE[unit]; break;
      case 'atan': r = Math.atan(v) / ANGLE[unit]; break;
      case 'sinh': r = Math.sinh(v); break;
      case 'cosh': r = Math.cosh(v); break;
      case 'tanh': r = Math.tanh(v); break;
      default: throw new CalcError('Unknown function');
    }
    if (Math.abs(r) < 1e-15) r = 0;
    return D.fromNumber(r);
  }
  function unary(fn, x, unit) {
    x = D.of(x);
    switch (fn) {
      case 'neg': return x.neg();
      case 'sqr': return x.mul(x);
      case 'cube': return x.mul(x).mul(x);
      case 'sqrt': return sqrt(x);
      case 'cbrt': return root(x, 3);
      case 'inv': return D.of(1).div(x);
      case 'abs': return x.abs();
      case 'fact': return factorial(x);
      case 'pow10': return x.isInt() ? powInt(D.of(10), x.toInt()) : D.fromNumber(Math.pow(10, x.toNumber()));
      case 'pow2': return x.isInt() ? powInt(D.of(2), x.toInt()) : D.fromNumber(Math.pow(2, x.toNumber()));
      case 'exp': return D.fromNumber(Math.exp(x.toNumber()));
      case 'ln': if (x.sign() <= 0) throw new CalcError('Invalid input'); return D.fromNumber(Math.log(x.toNumber()));
      case 'log': {
        if (x.sign() <= 0) throw new CalcError('Invalid input');
        // log of a power of ten is exact.
        const n = x.norm(); if (n.m === 1n) return D.of(n.e);
        return D.fromNumber(Math.log10(x.toNumber()));
      }
      case 'log2': if (x.sign() <= 0) throw new CalcError('Invalid input'); return D.fromNumber(Math.log2(x.toNumber()));
      case 'floor': return D.of(floorBig(x));
      case 'ceil': return D.of(-floorBig(x.neg()));
      case 'percent': return x.div(100);
      default: return trig(fn, x, unit);
    }
  }
  function floorBig(x) { const t = x.toInt(); return x.sign() < 0 && !x.isInt() ? t - 1n : t; }
  function binary(op, a, b) {
    a = D.of(a); b = D.of(b);
    switch (op) {
      case '+': return a.add(b);
      case '-': return a.sub(b);
      case '*': return a.mul(b);
      case '/': return a.div(b);
      case 'mod': { if (b.isZero()) throw new CalcError('Cannot divide by zero'); const q = floorBig(a.div(b, PREC + 20)); return a.sub(b.mul(q)); }
      case '^': return pow(a, b);
      case 'yroot': return root(a, b);
      case 'logy': { if (a.sign() <= 0 || b.sign() <= 0 || b.eq(1)) throw new CalcError('Invalid input'); return D.fromNumber(Math.log(a.toNumber()) / Math.log(b.toNumber())); }
      case 'E': return a.mul(powInt(D.of(10), b.toInt()));
      default: throw new CalcError('Unknown operation');
    }
  }
  const PRECEDENCE = { '+': 1, '-': 1, '*': 2, '/': 2, 'mod': 2, '^': 3, 'yroot': 3, 'logy': 3, 'E': 4 };
  const RIGHT = { '^': true, 'yroot': true };
  // Evaluates tokens: numbers (D), operator strings, '(' and ')'. `precedence` false: left to right (Standard mode).
  function evaluate(tokens, { precedence = true } = {}) {
    const out = [], ops = [];
    const apply = () => { const op = ops.pop(); const b = out.pop(), a = out.pop(); if (a === undefined || b === undefined) throw new CalcError('Incomplete'); out.push(binary(op, a, b)); };
    const prec = op => (precedence ? PRECEDENCE[op] : 1);
    for (const t of tokens) {
      if (t instanceof D) out.push(t);
      else if (t === '(') ops.push(t);
      else if (t === ')') { while (ops.length && ops[ops.length - 1] !== '(') apply(); if (ops.pop() !== '(') throw new CalcError('Unbalanced brackets'); }
      else {
        while (ops.length && ops[ops.length - 1] !== '(' && (prec(ops[ops.length - 1]) > prec(t) || (prec(ops[ops.length - 1]) === prec(t) && !(precedence && RIGHT[t])))) apply();
        ops.push(t);
      }
    }
    while (ops.length) { if (ops[ops.length - 1] === '(') { ops.pop(); continue; } apply(); }
    if (out.length !== 1) throw new CalcError('Incomplete');
    return out[0];
  }

  // ---------------------------------------------------------------- programmer mode
  const WORD = { QWORD: 64, DWORD: 32, WORD: 16, BYTE: 8 };
  function wrap(v, bits, signed = true) {
    const mod = 1n << BigInt(bits);
    let u = ((v % mod) + mod) % mod;
    if (signed && u >= mod >> 1n) u -= mod;
    return u;
  }
  const unsigned = (v, bits) => wrap(v, bits, false);
  function toBase(v, base, bits) {
    if (base === 10) return wrap(v, bits).toString(10);
    return unsigned(v, bits).toString(base).toUpperCase();
  }
  function groupBase(s, base) {
    if (base === 2) return s.replace(/\B(?=([01]{4})+(?![01]))/g, ' ');
    if (base === 16) return s.replace(/\B(?=([0-9A-F]{4})+(?![0-9A-F]))/g, ' ');
    if (base === 8) return s.replace(/\B(?=([0-7]{3})+(?![0-7]))/g, ' ');
    return s.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  }
  function parseBase(s, base) {
    s = String(s).replace(/[\s,_]/g, '');
    if (!s) return 0n;
    const neg = s.startsWith('-'); if (neg) s = s.slice(1);
    const prefix = { 16: '0x', 8: '0o', 2: '0b' }[base];
    if (base === 10) { if (!/^\d+$/.test(s)) throw new CalcError('Not a number'); const v = BigInt(s); return neg ? -v : v; }
    if (!new RegExp(`^[${'0123456789ABCDEF'.slice(0, base)}]+$`, 'i').test(s)) throw new CalcError('Not a number');
    const v = BigInt(prefix + s);
    return neg ? -v : v;
  }
  const PPREC = { '*': 6, '/': 6, '%': 6, '+': 5, '-': 5, '<<': 4, '>>': 4, '>>>': 4, 'rol': 4, 'ror': 4, 'AND': 3, 'NAND': 3, 'XOR': 2, 'OR': 1, 'NOR': 1 };
  function pbinary(op, a, b, bits) {
    const B = BigInt(bits), u = x => unsigned(x, bits);
    let r;
    switch (op) {
      case '+': r = a + b; break;
      case '-': r = a - b; break;
      case '*': r = a * b; break;
      case '/': if (b === 0n) throw new CalcError('Cannot divide by zero'); r = a / b; break;
      case '%': if (b === 0n) throw new CalcError('Cannot divide by zero'); r = a % b; break;
      case 'AND': r = a & b; break;
      case 'OR': r = a | b; break;
      case 'XOR': r = a ^ b; break;
      case 'NAND': r = ~(a & b); break;
      case 'NOR': r = ~(a | b); break;
      case '<<': r = a << (b % (B + 1n)); break;
      case '>>': r = a >> (b % (B + 1n)); break;           // arithmetic (keeps the sign)
      case '>>>': r = u(a) >> (b % (B + 1n)); break;        // logical
      case 'rol': { const n = ((b % B) + B) % B; r = ((u(a) << n) | (u(a) >> (B - n))); break; }
      case 'ror': { const n = ((b % B) + B) % B; r = ((u(a) >> n) | (u(a) << (B - n))); break; }
      default: throw new CalcError('Unknown operation');
    }
    return wrap(r, bits);
  }
  function pevaluate(tokens, bits) {
    const out = [], ops = [];
    const apply = () => { const op = ops.pop(); const b = out.pop(), a = out.pop(); if (a === undefined || b === undefined) throw new CalcError('Incomplete'); out.push(pbinary(op, a, b, bits)); };
    for (const t of tokens) {
      if (typeof t === 'bigint') out.push(wrap(t, bits));
      else if (t === '(') ops.push(t);
      else if (t === ')') { while (ops.length && ops[ops.length - 1] !== '(') apply(); if (ops.pop() !== '(') throw new CalcError('Unbalanced brackets'); }
      else { while (ops.length && ops[ops.length - 1] !== '(' && PPREC[ops[ops.length - 1]] >= PPREC[t]) apply(); ops.push(t); }
    }
    while (ops.length) { if (ops[ops.length - 1] === '(') { ops.pop(); continue; } apply(); }
    if (out.length !== 1) throw new CalcError('Incomplete');
    return out[0];
  }

  // ---------------------------------------------------------------- unit conversion
  // Each unit: factor to the category's base unit (as an exact decimal string), or a function pair (temperature).
  const UNITS = {
    Length: { base: 'm', units: [
      ['nm', 'Nanometres', '1e-9'], ['µm', 'Micrometres', '1e-6'], ['mm', 'Millimetres', '0.001'], ['cm', 'Centimetres', '0.01'], ['m', 'Metres', '1'], ['km', 'Kilometres', '1000'],
      ['in', 'Inches', '0.0254'], ['ft', 'Feet', '0.3048'], ['yd', 'Yards', '0.9144'], ['mi', 'Miles', '1609.344'], ['nmi', 'Nautical miles', '1852']] },
    Mass: { base: 'kg', units: [
      ['mg', 'Milligrams', '0.000001'], ['g', 'Grams', '0.001'], ['kg', 'Kilograms', '1'], ['t', 'Tonnes', '1000'],
      ['oz', 'Ounces', '0.028349523125'], ['lb', 'Pounds', '0.45359237'], ['st', 'Stone', '6.35029318'], ['ton', 'Short tons (US)', '907.18474'], ['lt', 'Long tons (UK)', '1016.0469088']] },
    Temperature: { base: 'C', units: [['C', 'Celsius'], ['F', 'Fahrenheit'], ['K', 'Kelvin']] },
    'Data size': { base: 'B', units: [
      ['bit', 'Bits', '0.125'], ['B', 'Bytes', '1'], ['kB', 'Kilobytes (1000)', '1000'], ['MB', 'Megabytes (1000²)', '1000000'], ['GB', 'Gigabytes (1000³)', '1000000000'], ['TB', 'Terabytes (1000⁴)', '1000000000000'],
      ['KiB', 'Kibibytes (1024)', '1024'], ['MiB', 'Mebibytes (1024²)', '1048576'], ['GiB', 'Gibibytes (1024³)', '1073741824'], ['TiB', 'Tebibytes (1024⁴)', '1099511627776'],
      ['kbit', 'Kilobits', '125'], ['Mbit', 'Megabits', '125000'], ['Gbit', 'Gigabits', '125000000']] },
    Time: { base: 's', units: [
      ['µs', 'Microseconds', '0.000001'], ['ms', 'Milliseconds', '0.001'], ['s', 'Seconds', '1'], ['min', 'Minutes', '60'], ['h', 'Hours', '3600'], ['d', 'Days', '86400'], ['wk', 'Weeks', '604800'], ['yr', 'Years (365 days)', '31536000']] },
    Area: { base: 'm2', units: [
      ['cm2', 'Square centimetres', '0.0001'], ['m2', 'Square metres', '1'], ['ha', 'Hectares', '10000'], ['km2', 'Square kilometres', '1000000'],
      ['in2', 'Square inches', '0.00064516'], ['ft2', 'Square feet', '0.09290304'], ['ac', 'Acres', '4046.8564224'], ['mi2', 'Square miles', '2589988.110336']] },
    Volume: { base: 'L', units: [
      ['mL', 'Millilitres', '0.001'], ['L', 'Litres', '1'], ['m3', 'Cubic metres', '1000'], ['tsp', 'Teaspoons (US)', '0.00492892159375'], ['tbsp', 'Tablespoons (US)', '0.01478676478125'],
      ['floz', 'Fluid ounces (US)', '0.0295735295625'], ['cup', 'Cups (US)', '0.2365882365'], ['pt', 'Pints (US)', '0.473176473'], ['gal', 'Gallons (US)', '3.785411784'], ['ukgal', 'Gallons (UK)', '4.54609']] },
    Speed: { base: 'm/s', units: [['m/s', 'Metres per second', '1'], ['km/h', 'Kilometres per hour', '0.27777777777777777777777777777777777778'], ['mph', 'Miles per hour', '0.44704'], ['kn', 'Knots', '0.51444444444444444444444444444444444444'], ['ft/s', 'Feet per second', '0.3048']] },
  };
  function convert(cat, value, from, to) {
    const v = D.of(value);
    if (cat === 'Temperature') {
      const toC = { C: x => x, F: x => x.sub(32).mul(5).div(9), K: x => x.sub('273.15') }[from];
      const fromC = { C: x => x, F: x => x.mul(9).div(5).add(32), K: x => x.add('273.15') }[to];
      return fromC(toC(v)).round(PREC - 6);
    }
    const u = UNITS[cat].units;
    const f = D.parse(u.find(x => x[0] === from)[2]), t = D.parse(u.find(x => x[0] === to)[2]);
    return v.mul(f).div(t).round(PREC - 6);
  }

  const api = { D, CalcError, PI, E, sqrt, pow, root, factorial, trig, unary, binary, evaluate, PRECEDENCE, WORD, wrap, unsigned, toBase, groupBase, parseBase, pbinary, pevaluate, UNITS, convert, SHOW };
  if (typeof module !== 'undefined' && module.exports) module.exports = api; else glob.Calc = api;
})(typeof window !== 'undefined' ? window : globalThis);
