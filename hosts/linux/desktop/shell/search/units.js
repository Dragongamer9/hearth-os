.pragma library
// Search plugin: unit conversion (like KRunner's). "10 km to mi", "72 f in c", "5 ft 11 in cm"? (no: one unit),
// "3.5 kg = lb", "500 mb to gb", "60 mph to km/h", "2 cups in ml", "1 acre to m2", "90 min to h".
// answer(q) → {value, text, from, to} or null.

// Each dimension: unit → how many of the base unit one is. Names are matched folded (lower case, no dots).
var DIMS = {
    length: { base: "m", u: { mm: 0.001, cm: 0.01, m: 1, km: 1000, in: 0.0254, inch: 0.0254, inches: 0.0254, '"': 0.0254,
        ft: 0.3048, foot: 0.3048, feet: 0.3048, "'": 0.3048, yd: 0.9144, yard: 0.9144, yards: 0.9144, mi: 1609.344, mile: 1609.344, miles: 1609.344,
        nmi: 1852, meter: 1, meters: 1, metre: 1, metres: 1, kilometer: 1000, kilometers: 1000, kilometre: 1000, kilometres: 1000,
        centimeter: 0.01, centimeters: 0.01, millimeter: 0.001, millimeters: 0.001, um: 1e-6, µm: 1e-6 } },
    mass: { base: "kg", u: { mg: 1e-6, g: 0.001, gram: 0.001, grams: 0.001, kg: 1, kilo: 1, kilos: 1, kilogram: 1, kilograms: 1,
        t: 1000, tonne: 1000, tonnes: 1000, lb: 0.45359237, lbs: 0.45359237, pound: 0.45359237, pounds: 0.45359237,
        oz: 0.028349523125, ounce: 0.028349523125, ounces: 0.028349523125, st: 6.35029318, stone: 6.35029318, ton: 907.18474, tons: 907.18474 } },
    volume: { base: "l", u: { ml: 0.001, cl: 0.01, dl: 0.1, l: 1, liter: 1, liters: 1, litre: 1, litres: 1, m3: 1000,
        gal: 3.785411784, gallon: 3.785411784, gallons: 3.785411784, qt: 0.946352946, quart: 0.946352946, quarts: 0.946352946,
        pt: 0.473176473, pint: 0.473176473, pints: 0.473176473, cup: 0.2365882365, cups: 0.2365882365,
        floz: 0.0295735295625, "fl oz": 0.0295735295625, tbsp: 0.01478676478125, tsp: 0.00492892159375 } },
    area: { base: "m2", u: { mm2: 1e-6, cm2: 1e-4, m2: 1, "sq m": 1, km2: 1e6, ha: 1e4, hectare: 1e4, hectares: 1e4,
        acre: 4046.8564224, acres: 4046.8564224, ft2: 0.09290304, "sq ft": 0.09290304, sqft: 0.09290304, in2: 0.00064516, mi2: 2589988.110336, "sq mi": 2589988.110336 } },
    speed: { base: "m/s", u: { "m/s": 1, mps: 1, "km/h": 1 / 3.6, kmh: 1 / 3.6, kph: 1 / 3.6, mph: 0.44704, "mi/h": 0.44704, kn: 0.514444, knot: 0.514444, knots: 0.514444, "ft/s": 0.3048 } },
    time: { base: "s", u: { ms: 0.001, s: 1, sec: 1, secs: 1, second: 1, seconds: 1, min: 60, mins: 60, minute: 60, minutes: 60,
        h: 3600, hr: 3600, hrs: 3600, hour: 3600, hours: 3600, d: 86400, day: 86400, days: 86400, wk: 604800, week: 604800, weeks: 604800,
        yr: 31557600, year: 31557600, years: 31557600 } },
    data: { base: "B", u: { b: 1, byte: 1, bytes: 1, kb: 1e3, mb: 1e6, gb: 1e9, tb: 1e12, pb: 1e15,
        kib: 1024, mib: 1048576, gib: 1073741824, tib: 1099511627776, bit: 0.125, bits: 0.125, kbit: 125, mbit: 125000, gbit: 1.25e8 } },
    energy: { base: "J", u: { j: 1, joule: 1, joules: 1, kj: 1000, cal: 4.184, kcal: 4184, wh: 3600, kwh: 3.6e6, btu: 1055.06 } },
    power: { base: "W", u: { w: 1, watt: 1, watts: 1, kw: 1000, hp: 745.699872, mw: 1e6 } },
    pressure: { base: "Pa", u: { pa: 1, kpa: 1000, bar: 1e5, psi: 6894.757293168, atm: 101325, mmhg: 133.322387415 } },
    temperature: { base: "C", u: { c: 1, "°c": 1, celsius: 1, f: 1, "°f": 1, fahrenheit: 1, k: 1, kelvin: 1 } }
};
// What to show a unit as.
var SHOW = { inch: "in", inches: "in", '"': "in", foot: "ft", feet: "ft", "'": "ft", mile: "mi", miles: "mi", meter: "m", meters: "m", metre: "m",
    metres: "m", kilometer: "km", kilometers: "km", kilometre: "km", kilometres: "km", pound: "lb", pounds: "lb", lbs: "lb",
    gram: "g", grams: "g", kilogram: "kg", kilograms: "kg", kilo: "kg", kilos: "kg", ounce: "oz", ounces: "oz", liter: "L", liters: "L", litre: "L", litres: "L", l: "L",
    c: "°C", "°c": "°C", celsius: "°C", f: "°F", "°f": "°F", fahrenheit: "°F", k: "K", kelvin: "K", kmh: "km/h", kph: "km/h",
    b: "B", byte: "B", bytes: "B", kb: "KB", mb: "MB", gb: "GB", tb: "TB", pb: "PB", kib: "KiB", mib: "MiB", gib: "GiB", tib: "TiB" };

function fold(s) { return String(s || "").toLowerCase().replace(/\./g, "").replace(/\s+/g, " ").trim(); }

function find(unit) {
    var u = fold(unit);
    // "MB" vs "mb": the same here (bytes); "Mb"/"mbit" are bits only when written out.
    for (var d in DIMS) if (DIMS[d].u[u] !== undefined) return { dim: d, key: u, f: DIMS[d].u[u] };
    if (u.length > 3 && u.charAt(u.length - 1) === "s") return find(u.slice(0, -1));
    return null;
}

function toC(v, k) { return k.indexOf("f") >= 0 ? (v - 32) * 5 / 9 : (k === "k" || k === "kelvin") ? v - 273.15 : v; }
function fromC(v, k) { return k.indexOf("f") >= 0 ? v * 9 / 5 + 32 : (k === "k" || k === "kelvin") ? v + 273.15 : v; }

function fmt(v) {
    if (!isFinite(v)) return null;
    var a = Math.abs(v);
    if (a !== 0 && (a >= 1e12 || a < 1e-4)) return v.toExponential(4).replace(/\.?0+e/, "e");
    var r = Math.round(v * 1e4) / 1e4;
    if (a >= 100) r = Math.round(v * 100) / 100;
    return String(r);
}

var RE = /^\s*(-?\d+(?:[.,]\d+)?)\s*([^\d\s][^=]*?)\s+(?:to|in|into|as|=|>|->)\s+(.+?)\s*$/i;

function answer(qRaw) {
    var m = RE.exec(String(qRaw || ""));
    if (!m) return null;
    var v = parseFloat(m[1].replace(",", "."));
    var a = find(m[2]), b = find(m[3]);
    if (!a || !b || a.dim !== b.dim) return null;
    var out = a.dim === "temperature" ? fromC(toC(v, a.key), b.key) : v * a.f / b.f;
    var text = fmt(out);
    if (text === null) return null;
    var sa = SHOW[a.key] || m[2].trim(), sb = SHOW[b.key] || m[3].trim();
    return { value: out, text: text + " " + sb, from: m[1] + " " + sa, to: sb, dim: a.dim };
}

function results(q) {
    var a = answer(q);
    if (!a) return [];
    return [{ plugin: "units", group: "Convert", title: a.text, sub: a.from + " = " + a.text + " · Enter copies it",
              icon: "sync", score: 1900, top: true, action: { type: "copy", value: a.text.replace(/ .*/, "") } }];
}
