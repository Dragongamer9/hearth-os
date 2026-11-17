// Other calendars and holidays for the calendar widget and the clock's panel (1.14; KDE Plasma's "alternate
// calendars" and "holidays" plugins, written for Hearth). Pure arithmetic, no Qt and no internet, so
// test_notify_widgets_114.py checks it with node against known dates.
//   CALENDARS: Islamic (the tabular civil calendar: real months follow the moon sighting, so it can be a day off),
//              Hebrew, Persian (Solar Hijri, the 33-year rule), Julian, Indian national (Saka).
//   HOLIDAY SETS: public holidays of a few countries (fixed days, Easter-based days, "the nth Monday"), and the main
//              Islamic, Jewish and Persian days (from the calendars above).
.pragma library

function div(a, b) { return ~~(a / b); }
function mod(a, b) { return a - ~~(a / b) * b; }
function isLeapG(y) { return (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0; }

// Julian day number (at noon) of a Gregorian date, and back.
function g2jdn(y, m, d) {
    var a = Math.floor((14 - m) / 12), yy = y + 4800 - a, mm = m + 12 * a - 3;
    return d + Math.floor((153 * mm + 2) / 5) + 365 * yy + Math.floor(yy / 4) - Math.floor(yy / 100) + Math.floor(yy / 400) - 32045;
}
function jdn2g(j) {
    var a = j + 32044, b = Math.floor((4 * a + 3) / 146097), c = a - Math.floor(146097 * b / 4);
    var d = Math.floor((4 * c + 3) / 1461), e = c - Math.floor(1461 * d / 4), m = Math.floor((5 * e + 2) / 153);
    return { y: 100 * b + d - 4800 + Math.floor(m / 10), m: m + 3 - 12 * Math.floor(m / 10), d: e - Math.floor((153 * m + 2) / 5) + 1 };
}
function jdnOf(date) { return g2jdn(date.getFullYear(), date.getMonth() + 1, date.getDate()); }

// ---------------------------------------------------------------- Islamic (tabular, civil epoch 16 July 622)
var ISLAMIC_EPOCH = 1948440;
function islamicToJdn(y, m, d) { return d + Math.ceil(29.5 * (m - 1)) + (y - 1) * 354 + Math.floor((3 + 11 * y) / 30) + ISLAMIC_EPOCH - 1; }
function jdnToIslamic(j) {
    var y = Math.floor((30 * (j - ISLAMIC_EPOCH) + 10646) / 10631);
    var m = Math.min(12, Math.ceil((j - (29 + islamicToJdn(y, 1, 1))) / 29.5) + 1);
    return { y: y, m: m, d: j - islamicToJdn(y, m, 1) + 1 };
}
var ISLAMIC_MONTHS = ["Muharram", "Safar", "Rabi' I", "Rabi' II", "Jumada I", "Jumada II", "Rajab", "Sha'ban", "Ramadan", "Shawwal", "Dhu al-Qi'dah", "Dhu al-Hijjah"];

// ---------------------------------------------------------------- Hebrew
var HEBREW_EPOCH = 347998;   // so that hNewYear is the Julian day number of 1 Tishri (Fourmilab's 347995.5 + 1, at noon)
function hLeap(y) { return mod(7 * y + 1, 19) < 7; }
function hMonths(y) { return hLeap(y) ? 13 : 12; }
function hDelay1(y) {
    var months = Math.floor((235 * y - 234) / 19), parts = 12084 + 13753 * months;
    var day = months * 29 + Math.floor(parts / 25920);
    if (mod(3 * (day + 1), 7) < 3) day++;
    return day;
}
function hDelay2(y) {
    var last = hDelay1(y - 1), present = hDelay1(y), next = hDelay1(y + 1);
    return next - present === 356 ? 2 : present - last === 382 ? 1 : 0;
}
function hNewYear(y) { return HEBREW_EPOCH + hDelay1(y) + hDelay2(y); }
function hYearDays(y) { return hNewYear(y + 1) - hNewYear(y); }
function hMonthDays(y, m) {
    if (m === 2 || m === 4 || m === 6 || m === 10 || m === 13) return 29;
    if (m === 12 && !hLeap(y)) return 29;
    if (m === 8 && mod(hYearDays(y), 10) !== 5) return 29;
    if (m === 9 && mod(hYearDays(y), 10) === 3) return 29;
    return 30;
}
// Months numbered from Nisan (1) as the Bible does; the year starts at Tishri (7). 12 = Adar (Adar I in a leap year), 13 = Adar II.
function hebrewToJdn(y, m, d) {
    var j = hNewYear(y) + d - 1, mon;
    if (m < 7) {
        for (mon = 7; mon <= hMonths(y); mon++) j += hMonthDays(y, mon);
        for (mon = 1; mon < m; mon++) j += hMonthDays(y, mon);
    } else {
        for (mon = 7; mon < m; mon++) j += hMonthDays(y, mon);
    }
    return j;
}
function jdnToHebrew(j) {
    var y = Math.floor((j - HEBREW_EPOCH) / 365.2468) + 1;
    while (hNewYear(y + 1) <= j) y++;
    while (hNewYear(y) > j) y--;
    var m = j < hebrewToJdn(y, 1, 1) ? 7 : 1;
    while (j > hebrewToJdn(y, m, hMonthDays(y, m))) m++;
    return { y: y, m: m, d: j - hebrewToJdn(y, m, 1) + 1 };
}
function hebrewMonthName(y, m) {
    var names = ["Nisan", "Iyar", "Sivan", "Tammuz", "Av", "Elul", "Tishri", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar", "Adar II"];
    return m === 12 && hLeap(y) ? "Adar I" : names[m - 1];
}

// ---------------------------------------------------------------- Persian (Solar Hijri; the 33-year rule)
var JBREAKS = [-61, 9, 38, 199, 426, 686, 756, 818, 1111, 1181, 1210, 1635, 2060, 2097, 2192, 2262, 2324, 2394, 2456, 3178];
function jalCal(jy) {
    var gy = jy + 621, leapJ = -14, jp = JBREAKS[0], jm, jump = 0, i;
    for (i = 1; i < JBREAKS.length; i++) {
        jm = JBREAKS[i]; jump = jm - jp;
        if (jy < jm) break;
        leapJ = leapJ + div(jump, 33) * 8 + div(mod(jump, 33), 4);
        jp = jm;
    }
    var n = jy - jp;
    leapJ = leapJ + div(n, 33) * 8 + div(mod(n, 33) + 3, 4);
    if (mod(jump, 33) === 4 && jump - n === 4) leapJ += 1;
    var leapG = div(gy, 4) - div((div(gy, 100) + 1) * 3, 4) - 150;
    var march = 20 + leapJ - leapG;
    if (jump - n < 6) n = n - jump + div(jump + 4, 33) * 33;
    var leap = mod(mod(n + 1, 33) - 1, 4);
    if (leap === -1) leap = 4;
    return { leap: leap, gy: gy, march: march };
}
function persianToJdn(jy, jm, jd) {
    var r = jalCal(jy);
    return g2jdn(r.gy, 3, r.march) + (jm - 1) * 31 - div(jm, 7) * (jm - 7) + jd - 1;
}
function jdnToPersian(j) {
    var gy = jdn2g(j).y, jy = gy - 621, r = jalCal(jy), k = j - g2jdn(gy, 3, r.march);
    if (k >= 0) {
        if (k <= 185) return { y: jy, m: 1 + div(k, 31), d: mod(k, 31) + 1 };
        k -= 186;
    } else {
        jy -= 1; k += 179;
        if (r.leap === 1) k += 1;
    }
    return { y: jy, m: 7 + div(k, 30), d: mod(k, 30) + 1 };
}
var PERSIAN_MONTHS = ["Farvardin", "Ordibehesht", "Khordad", "Tir", "Mordad", "Shahrivar", "Mehr", "Aban", "Azar", "Dey", "Bahman", "Esfand"];

// ---------------------------------------------------------------- Julian
function jdnToJulian(j) {
    var c = j + 32082, d = Math.floor((4 * c + 3) / 1461), e = c - Math.floor(1461 * d / 4), m = Math.floor((5 * e + 2) / 153);
    return { y: d - 4800 + Math.floor(m / 10), m: m + 3 - 12 * Math.floor(m / 10), d: e - Math.floor((153 * m + 2) / 5) + 1 };
}
var GREG_MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

// ---------------------------------------------------------------- Indian national (Saka)
var SAKA_MONTHS = ["Chaitra", "Vaisakha", "Jyaistha", "Asadha", "Sravana", "Bhadra", "Asvina", "Kartika", "Agrahayana", "Pausa", "Magha", "Phalguna"];
function jdnToSaka(j) {
    var g = jdn2g(j), y = g.y, leap = isLeapG(y);
    var start = g2jdn(y, 3, leap ? 21 : 22);
    var sy = y - 78;
    if (j < start) { sy -= 1; leap = isLeapG(y - 1); start = g2jdn(y - 1, 3, leap ? 21 : 22); }
    var k = j - start, lens = [leap ? 31 : 30, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30], m = 0;
    while (k >= lens[m]) { k -= lens[m]; m++; }
    return { y: sy, m: m + 1, d: k + 1 };
}

// ---------------------------------------------------------------- the list the settings show
var CALENDARS = [
    { id: "", name: "None" },
    { id: "islamic", name: "Islamic (Hijri)" },
    { id: "hebrew", name: "Hebrew" },
    { id: "persian", name: "Persian (Solar Hijri)" },
    { id: "julian", name: "Julian" },
    { id: "indian", name: "Indian national (Saka)" }
];

// {y, m, d, month: "Ramadan", short: "1 Ram"} for a JavaScript Date, or null.
function convert(date, system) {
    var j = jdnOf(date), r, name;
    if (system === "islamic") { r = jdnToIslamic(j); name = ISLAMIC_MONTHS[r.m - 1]; }
    else if (system === "hebrew") { r = jdnToHebrew(j); name = hebrewMonthName(r.y, r.m); }
    else if (system === "persian") { r = jdnToPersian(j); name = PERSIAN_MONTHS[r.m - 1]; }
    else if (system === "julian") { r = jdnToJulian(j); name = GREG_MONTHS[r.m - 1]; }
    else if (system === "indian") { r = jdnToSaka(j); name = SAKA_MONTHS[r.m - 1]; }
    else return null;
    return { y: r.y, m: r.m, d: r.d, month: name, text: r.d + " " + name + " " + r.y, short: r.d === 1 ? r.d + " " + name.slice(0, 3) : String(r.d) };
}

// ---------------------------------------------------------------- holidays
function easter(y) {   // the anonymous Gregorian algorithm: Easter Sunday as [month, day]
    var a = y % 19, b = Math.floor(y / 100), c = y % 100, d = Math.floor(b / 4), e = b % 4, f = Math.floor((b + 8) / 25);
    var g = Math.floor((b - f + 1) / 3), h = (19 * a + b - d - g + 15) % 30, i = Math.floor(c / 4), k = c % 4;
    var l = (32 + 2 * e + 2 * i - h - k) % 7, m = Math.floor((a + 11 * h + 22 * l) / 451);
    return [Math.floor((h + l - 7 * m + 114) / 31), ((h + l - 7 * m + 114) % 31) + 1];
}
function pad(n) { return (n < 10 ? "0" : "") + n; }
function ymdOf(j) { var g = jdn2g(j); return g.y + "-" + pad(g.m) + "-" + pad(g.d); }
// The nth weekday (0 = Sunday) of a month; n = -1 is the last one.
function nth(y, m, wd, n) {
    if (n > 0) { var first = g2jdn(y, m, 1), w = (first + 1) % 7; return first + ((wd - w + 7) % 7) + (n - 1) * 7; }
    var last = m === 12 ? g2jdn(y, 12, 31) : g2jdn(y, m + 1, 1) - 1;
    var lw = (last + 1) % 7;
    return last - ((lw - wd + 7) % 7);
}
var HOLIDAY_SETS = [
    { id: "", name: "None" },
    { id: "us", name: "United States" }, { id: "gb", name: "United Kingdom (England)" }, { id: "ie", name: "Ireland" },
    { id: "ca", name: "Canada" }, { id: "au", name: "Australia" }, { id: "de", name: "Germany" }, { id: "fr", name: "France" },
    { id: "nl", name: "Netherlands" }, { id: "es", name: "Spain" }, { id: "it", name: "Italy" },
    { id: "islamic", name: "Islamic days" }, { id: "jewish", name: "Jewish days" }, { id: "persian", name: "Persian days" }
];
// The holidays of SET in Gregorian year Y: [{date: "YYYY-MM-DD", name}], in date order.
function holidays(set, y) {
    var out = [], E = easter(y), e = g2jdn(y, E[0], E[1]);
    function fix(m, d, name) { out.push({ j: g2jdn(y, m, d), name: name }); }
    function at(j, name) { out.push({ j: j, name: name }); }
    switch (set) {
    case "us":
        fix(1, 1, "New Year's Day"); at(nth(y, 1, 1, 3), "Martin Luther King Jr. Day"); at(nth(y, 2, 1, 3), "Presidents' Day");
        at(nth(y, 5, 1, -1), "Memorial Day"); fix(6, 19, "Juneteenth"); fix(7, 4, "Independence Day"); at(nth(y, 9, 1, 1), "Labor Day");
        at(nth(y, 10, 1, 2), "Columbus Day"); fix(11, 11, "Veterans Day"); at(nth(y, 11, 4, 4), "Thanksgiving"); fix(12, 25, "Christmas Day");
        break;
    case "gb":
        fix(1, 1, "New Year's Day"); at(e - 2, "Good Friday"); at(e + 1, "Easter Monday"); at(nth(y, 5, 1, 1), "Early May bank holiday");
        at(nth(y, 5, 1, -1), "Spring bank holiday"); at(nth(y, 8, 1, -1), "Summer bank holiday"); fix(12, 25, "Christmas Day"); fix(12, 26, "Boxing Day");
        break;
    case "ie":
        fix(1, 1, "New Year's Day"); at(nth(y, 2, 1, 1), "St Brigid's Day"); fix(3, 17, "St Patrick's Day"); at(e + 1, "Easter Monday");
        at(nth(y, 5, 1, 1), "May bank holiday"); at(nth(y, 6, 1, 1), "June bank holiday"); at(nth(y, 8, 1, 1), "August bank holiday");
        at(nth(y, 10, 1, -1), "October bank holiday"); fix(12, 25, "Christmas Day"); fix(12, 26, "St Stephen's Day");
        break;
    case "ca": {
        fix(1, 1, "New Year's Day"); at(e - 2, "Good Friday");
        var v = g2jdn(y, 5, 24); at(v - ((v + 1) % 7 + 6) % 7, "Victoria Day");
        fix(7, 1, "Canada Day"); at(nth(y, 9, 1, 1), "Labour Day"); fix(9, 30, "National Day for Truth and Reconciliation");
        at(nth(y, 10, 1, 2), "Thanksgiving"); fix(11, 11, "Remembrance Day"); fix(12, 25, "Christmas Day"); fix(12, 26, "Boxing Day");
        break;
    }
    case "au":
        fix(1, 1, "New Year's Day"); fix(1, 26, "Australia Day"); at(e - 2, "Good Friday"); at(e + 1, "Easter Monday"); fix(4, 25, "Anzac Day");
        at(nth(y, 6, 1, 2), "King's Birthday"); fix(12, 25, "Christmas Day"); fix(12, 26, "Boxing Day");
        break;
    case "de":
        fix(1, 1, "Neujahr"); at(e - 2, "Karfreitag"); at(e + 1, "Ostermontag"); fix(5, 1, "Tag der Arbeit"); at(e + 39, "Christi Himmelfahrt");
        at(e + 50, "Pfingstmontag"); fix(10, 3, "Tag der Deutschen Einheit"); fix(12, 25, "1. Weihnachtstag"); fix(12, 26, "2. Weihnachtstag");
        break;
    case "fr":
        fix(1, 1, "Jour de l'an"); at(e + 1, "Lundi de Pâques"); fix(5, 1, "Fête du Travail"); fix(5, 8, "Victoire 1945"); at(e + 39, "Ascension");
        at(e + 50, "Lundi de Pentecôte"); fix(7, 14, "Fête nationale"); fix(8, 15, "Assomption"); fix(11, 1, "Toussaint"); fix(11, 11, "Armistice");
        fix(12, 25, "Noël");
        break;
    case "nl": {
        fix(1, 1, "Nieuwjaarsdag"); at(e - 2, "Goede Vrijdag"); at(e, "Eerste Paasdag"); at(e + 1, "Tweede Paasdag");
        var kd = g2jdn(y, 4, 27); at((kd + 1) % 7 === 0 ? kd - 1 : kd, "Koningsdag"); fix(5, 5, "Bevrijdingsdag"); at(e + 39, "Hemelvaartsdag");
        at(e + 49, "Eerste Pinksterdag"); at(e + 50, "Tweede Pinksterdag"); fix(12, 25, "Eerste Kerstdag"); fix(12, 26, "Tweede Kerstdag");
        break;
    }
    case "es":
        fix(1, 1, "Año Nuevo"); fix(1, 6, "Epifanía del Señor"); at(e - 2, "Viernes Santo"); fix(5, 1, "Fiesta del Trabajo"); fix(8, 15, "Asunción de la Virgen");
        fix(10, 12, "Fiesta Nacional de España"); fix(11, 1, "Todos los Santos"); fix(12, 6, "Día de la Constitución"); fix(12, 8, "Inmaculada Concepción");
        fix(12, 25, "Navidad");
        break;
    case "it":
        fix(1, 1, "Capodanno"); fix(1, 6, "Epifania"); at(e, "Pasqua"); at(e + 1, "Lunedì dell'Angelo"); fix(4, 25, "Festa della Liberazione");
        fix(5, 1, "Festa del Lavoro"); fix(6, 2, "Festa della Repubblica"); fix(8, 15, "Ferragosto"); fix(11, 1, "Ognissanti");
        fix(12, 8, "Immacolata Concezione"); fix(12, 25, "Natale"); fix(12, 26, "Santo Stefano");
        break;
    case "islamic": {
        var a = jdnToIslamic(g2jdn(y, 1, 1)).y;
        for (var iy = a; iy <= a + 1; iy++) {
            at(islamicToJdn(iy, 1, 1), "Islamic New Year"); at(islamicToJdn(iy, 1, 10), "Ashura"); at(islamicToJdn(iy, 3, 12), "Mawlid");
            at(islamicToJdn(iy, 9, 1), "Ramadan begins"); at(islamicToJdn(iy, 10, 1), "Eid al-Fitr"); at(islamicToJdn(iy, 12, 10), "Eid al-Adha");
        }
        break;
    }
    case "jewish": {
        var hy = jdnToHebrew(g2jdn(y, 1, 1)).y;
        for (var h = hy; h <= hy + 1; h++) {
            at(hebrewToJdn(h, 7, 1), "Rosh Hashanah"); at(hebrewToJdn(h, 7, 10), "Yom Kippur"); at(hebrewToJdn(h, 7, 15), "Sukkot");
            at(hebrewToJdn(h, 9, 25), "Hanukkah begins"); at(hebrewToJdn(h, hLeap(h) ? 13 : 12, 14), "Purim");
            at(hebrewToJdn(h, 1, 15), "Passover begins"); at(hebrewToJdn(h, 3, 6), "Shavuot");
        }
        break;
    }
    case "persian": {
        var py = jdnToPersian(g2jdn(y, 1, 1)).y;
        for (var p = py; p <= py + 1; p++) { at(persianToJdn(p, 1, 1), "Nowruz"); at(persianToJdn(p, 1, 13), "Sizdah Bedar"); at(persianToJdn(p, 10, 1) - 1, "Yalda Night"); }
        break;
    }
    }
    var lo = g2jdn(y, 1, 1), hi = g2jdn(y, 12, 31), seen = {};
    return out.filter(function (x) { return x.j >= lo && x.j <= hi; })
        .sort(function (a, b) { return a.j - b.j; })
        .filter(function (x) { var k = x.j + x.name; if (seen[k]) return false; seen[k] = true; return true; })
        .map(function (x) { return { date: ymdOf(x.j), name: x.name }; });
}
// {"YYYY-MM-DD": "name · name"} for the years a month grid shows.
function holidayMap(set, years) {
    var m = {};
    if (!set) return m;
    for (var i = 0; i < years.length; i++) {
        var list = holidays(set, years[i]);
        for (var k = 0; k < list.length; k++) m[list[k].date] = m[list[k].date] ? m[list[k].date] + " · " + list[k].name : list[k].name;
    }
    return m;
}
