.pragma library
// Search plugin: the calculator (like KRunner's). "2+2", "=sqrt(2)", "15% of 80", "(3+4)*2^3", "0xff + 1", "pi*2".
// A small parser of its own (never eval): + - * / ^ (or **), % (percent, or "a % b" mod with spaces), parentheses,
// sqrt cbrt abs round floor ceil ln log log2 exp sin cos tan asin acos atan (degrees with "deg"), pi, e, tau.
// answer(q) → {value, text} or null. Enter copies the answer.

var FUNCS = {
    sqrt: Math.sqrt, cbrt: Math.cbrt, abs: Math.abs, round: Math.round, floor: Math.floor, ceil: Math.ceil,
    ln: Math.log, log: Math.log10, log10: Math.log10, log2: Math.log2, exp: Math.exp,
    sin: Math.sin, cos: Math.cos, tan: Math.tan, asin: Math.asin, acos: Math.acos, atan: Math.atan
};
var CONSTS = { pi: Math.PI, "π": Math.PI, e: Math.E, tau: 2 * Math.PI };

function tokens(s) {
    var out = [], i = 0;
    while (i < s.length) {
        var c = s[i];
        if (c === " ") { i++; continue; }
        var m = /^(0x[0-9a-f]+|0b[01]+|\d+(?:[.,]\d+)?(?:e[+-]?\d+)?|\.\d+)/i.exec(s.slice(i));
        if (m && !/^e/i.test(m[0])) {
            var t = m[0], v;
            if (/^0x/i.test(t)) v = parseInt(t.slice(2), 16);
            else if (/^0b/i.test(t)) v = parseInt(t.slice(2), 2);
            else v = parseFloat(t.replace(",", "."));
            out.push({ t: "n", v: v }); i += t.length; continue;
        }
        m = /^[a-zπ]+/i.exec(s.slice(i));
        if (m) {
            var w = m[0].toLowerCase(), prev = out[out.length - 1];
            if (w === "x" && prev && prev.t === "n") out.push({ t: "op", v: "*" });     // "3 x 4"
            else out.push({ t: "id", v: w });
            i += m[0].length; continue;
        }
        if (s.substr(i, 2) === "**") { out.push({ t: "op", v: "^" }); i += 2; continue; }
        if ("+-*/^%()×÷−".indexOf(c) >= 0) {
            var op = c === "×" ? "*" : c === "÷" ? "/" : c === "−" ? "-" : c;
            // "a % b" with spaces round it is modulo; "15%" is percent
            if (c === "%" && s[i - 1] === " " && s[i + 1] === " ") op = "mod";
            out.push({ t: "op", v: op }); i++; continue;
        }
        return null;
    }
    return out;
}

function parse(toks) {
    var p = 0;
    function peek() { return toks[p]; }
    function isOp(v) { var t = toks[p]; return t && t.t === "op" && t.v === v; }
    function expr() {
        var v = term();
        while (isOp("+") || isOp("-")) { var o = toks[p++].v; var r = term(); v = o === "+" ? v + r : v - r; }
        return v;
    }
    function term() {
        var v = unary();
        for (;;) {
            if (isOp("*") || isOp("/") || isOp("mod")) {
                var o = toks[p++].v, r = unary();
                v = o === "*" ? v * r : o === "/" ? v / r : v % r;
            } else if (peek() && (peek().t === "n" || peek().t === "id" || isOp("("))) {
                v = v * unary();                        // "2pi", "3(4+1)"
            } else if (peek() && peek().t === "id" && peek().v === "of") {
                p++; v = v * unary();
            } else break;
        }
        return v;
    }
    function unary() {
        if (isOp("-")) { p++; return -unary(); }
        if (isOp("+")) { p++; return unary(); }
        return power();
    }
    function power() {
        var b = postfix();
        if (isOp("^")) { p++; return Math.pow(b, unary()); }
        return b;
    }
    function postfix() {
        var v = atom();
        while (isOp("%")) { p++; v = v / 100; }
        return v;
    }
    function atom() {
        var t = toks[p++];
        if (!t) throw "end";
        if (t.t === "n") return t.v;
        if (t.t === "op" && t.v === "(") { var v = expr(); if (!isOp(")")) throw "paren"; p++; return v; }
        if (t.t === "id") {
            if (t.v === "of") return atom();
            if (CONSTS[t.v] !== undefined) return CONSTS[t.v];
            if (FUNCS[t.v]) {
                var deg = false;
                var a = isOp("(") ? (p++, (function () { var x = expr(); if (isOp(")")) p++; else throw "paren"; return x; })()) : unary();
                if (peek() && peek().t === "id" && (peek().v === "deg" || peek().v === "degrees")) { p++; deg = true; }
                if (deg && /^(sin|cos|tan)$/.test(t.v)) a = a * Math.PI / 180;
                return FUNCS[t.v](a);
            }
        }
        throw "bad";
    }
    var v = expr();
    if (p !== toks.length) throw "left";
    return v;
}

function format(v) {
    if (!isFinite(v)) return null;
    if (Math.abs(v) >= 1e15 || (Math.abs(v) < 1e-6 && v !== 0)) return v.toExponential(6).replace(/\.?0+e/, "e");
    var r = Math.round(v * 1e10) / 1e10;
    return String(r);
}

// The answer for a query, or null when it isn't a sum.
function answer(qRaw) {
    var q = String(qRaw || "").trim();
    var forced = q.charAt(0) === "=";
    if (forced) q = q.slice(1).trim();
    if (!q) return null;
    var toks = tokens(q);
    if (!toks || !toks.length) return null;
    // Without "=", it must look like a sum: an operator or a function, not just a number or a word.
    var hasOp = toks.some(function (t) { return t.t === "op" && t.v !== "(" && t.v !== ")"; });
    var hasFn = toks.some(function (t) { return t.t === "id" && FUNCS[t.v]; });
    var onlyWords = toks.every(function (t) { return t.t === "id"; });
    if (!forced && (!(hasOp || hasFn) || onlyWords)) return null;
    if (!forced && toks.filter(function (t) { return t.t === "n"; }).length === 0 && !hasFn) return null;
    var v;
    try { v = parse(toks); } catch (e) { return null; }
    var text = format(v);
    if (text === null) return null;
    return { value: v, text: text };
}

function results(q) {
    var a = answer(q);
    if (!a) return [];
    return [{ plugin: "calc", group: "Calculator", title: a.text, sub: String(q).replace(/^=\s*/, "") + " = " + a.text + " · Enter copies it",
              icon: "plus", score: 2000, top: true, action: { type: "copy", value: a.text } }];
}
