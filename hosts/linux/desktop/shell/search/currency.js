.pragma library
// Search plugin: currency conversion (like KRunner's), offline-safe. "20 usd to eur", "€50 in $", "100 pounds to yen".
// Rates are per 1 euro. BUILTIN (below, approximate, dated) is used until hearth-search's "rates" command has saved
// the European Central Bank's daily rates (~/.cache/hearth/search/rates.json, refreshed when online, at most daily);
// the result always says which rates it used and how old they are, so an offline answer is never passed off as today's.
// answer(q, rates) → {value, text, sub} or null. rates: {date, source, rates: {USD: 1.1, ...}} or null.

var BUILTIN = { date: "2026-06-01", source: "built-in", rates: {
    EUR: 1, USD: 1.13, GBP: 0.84, JPY: 163, CHF: 0.94, CAD: 1.55, AUD: 1.73, NZD: 1.89, CNY: 8.15, HKD: 8.85,
    SEK: 10.9, NOK: 11.5, DKK: 7.46, PLN: 4.26, CZK: 24.9, HUF: 400, RON: 5.0, BGN: 1.956, TRY: 44.5, INR: 96.5,
    KRW: 1560, SGD: 1.46, MXN: 21.8, BRL: 6.3, ZAR: 20.3, ILS: 4.1, THB: 37, PHP: 63.5, IDR: 18400, MYR: 4.8, ISK: 144 } };

var NAMES = {
    "$": "USD", "us$": "USD", "dollar": "USD", "dollars": "USD", "usd": "USD", "bucks": "USD",
    "€": "EUR", "euro": "EUR", "euros": "EUR", "eur": "EUR",
    "£": "GBP", "pound": "GBP", "pounds": "GBP", "quid": "GBP", "gbp": "GBP", "sterling": "GBP",
    "¥": "JPY", "yen": "JPY", "jpy": "JPY", "yuan": "CNY", "rmb": "CNY", "renminbi": "CNY",
    "franc": "CHF", "francs": "CHF", "c$": "CAD", "a$": "AUD", "nz$": "NZD", "rupee": "INR", "rupees": "INR", "₹": "INR",
    "won": "KRW", "₩": "KRW", "zloty": "PLN", "złoty": "PLN", "lira": "TRY", "₺": "TRY", "real": "BRL", "reais": "BRL", "r$": "BRL",
    "rand": "ZAR", "peso": "MXN", "pesos": "MXN", "shekel": "ILS", "₪": "ILS", "baht": "THB", "krona": "SEK", "kronor": "SEK", "krone": "NOK"
};

function code(s, rates) {
    var t = String(s || "").trim().toLowerCase();
    if (NAMES[t]) return NAMES[t];
    var up = t.toUpperCase();
    return /^[A-Z]{3}$/.test(up) && rates.rates[up] !== undefined ? up : null;
}

function usable(r) { return r && r.rates && typeof r.rates === "object" && r.rates.EUR !== undefined ? r : null; }

// "20 usd to eur", "usd 20 to eur", "$20 in €", "20$ = €"
var RE = /^\s*([^\d\s.,-]{0,4}?)\s*(\d+(?:[.,]\d+)?)\s*([^\d\s][^=>]*?)?\s+(?:to|in|into|as|=|>|->)\s+(.+?)\s*$/i;

function answer(qRaw, ratesIn) {
    var rates = usable(ratesIn) || BUILTIN;
    var m = RE.exec(String(qRaw || ""));
    if (!m) return null;
    var fromWord = (m[1] || "").trim() || (m[3] || "").trim();
    if ((m[1] || "").trim() && (m[3] || "").trim()) return null;
    var a = code(fromWord, rates), b = code(m[4], rates);
    if (!a || !b) return null;
    var ra = rates.rates[a], rb = rates.rates[b];
    if (!ra || !rb) return null;
    var v = parseFloat(m[2].replace(",", ".")) / ra * rb;
    var text = (Math.round(v * 100) / 100).toFixed(2) + " " + b;
    var src = rates.source === "built-in" ? "Built-in rates from " + rates.date + " (offline; may be out of date)"
                                         : "ECB rates from " + rates.date;
    return { value: v, text: text, sub: m[2] + " " + a + " = " + text + " · " + src, code: b };
}

function results(q, rates) {
    var a = answer(q, rates);
    if (!a) return [];
    return [{ plugin: "currency", group: "Convert", title: a.text, sub: a.sub, icon: "sync", score: 1890, top: true,
              action: { type: "copy", value: a.text.replace(/ .*/, "") } }];
}
