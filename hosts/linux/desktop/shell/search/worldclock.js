.pragma library
// Search plugin: time in other cities and time-zone conversion (like KRunner's date/time runner).
//   "time in tokyo", "tokyo time", "time utc"          the time there now
//   "3pm london to tokyo", "15:30 utc in new york"     a time somewhere, somewhere else
//   "9am in tokyo"                                     a time here, there
// zones: {offsets: {zone: minutes from UTC now}, local: zone, localOffset: minutes} from hearth-search-sources zones
// (Python's zoneinfo: summer time right). Without it, the standard offsets below are used and the row says so.
// Enter copies the time.

// [name, IANA zone, standard offset in minutes]
var CITIES = [
    ["UTC", "UTC", 0], ["GMT", "UTC", 0], ["London", "Europe/London", 0], ["Dublin", "Europe/Dublin", 0], ["Lisbon", "Europe/Lisbon", 0],
    ["Paris", "Europe/Paris", 60], ["Berlin", "Europe/Berlin", 60], ["Madrid", "Europe/Madrid", 60], ["Rome", "Europe/Rome", 60],
    ["Amsterdam", "Europe/Amsterdam", 60], ["Stockholm", "Europe/Stockholm", 60], ["Warsaw", "Europe/Warsaw", 60], ["Vienna", "Europe/Vienna", 60],
    ["Athens", "Europe/Athens", 120], ["Helsinki", "Europe/Helsinki", 120], ["Kyiv", "Europe/Kyiv", 120], ["Cairo", "Africa/Cairo", 120],
    ["Johannesburg", "Africa/Johannesburg", 120], ["Istanbul", "Europe/Istanbul", 180], ["Moscow", "Europe/Moscow", 180],
    ["Nairobi", "Africa/Nairobi", 180], ["Dubai", "Asia/Dubai", 240], ["Karachi", "Asia/Karachi", 300], ["Delhi", "Asia/Kolkata", 330],
    ["Mumbai", "Asia/Kolkata", 330], ["Dhaka", "Asia/Dhaka", 360], ["Bangkok", "Asia/Bangkok", 420], ["Jakarta", "Asia/Jakarta", 420],
    ["Singapore", "Asia/Singapore", 480], ["Hong Kong", "Asia/Hong_Kong", 480], ["Beijing", "Asia/Shanghai", 480], ["Shanghai", "Asia/Shanghai", 480],
    ["Manila", "Asia/Manila", 480], ["Perth", "Australia/Perth", 480], ["Tokyo", "Asia/Tokyo", 540], ["Seoul", "Asia/Seoul", 540],
    ["Brisbane", "Australia/Brisbane", 600], ["Sydney", "Australia/Sydney", 600], ["Melbourne", "Australia/Melbourne", 600],
    ["Auckland", "Pacific/Auckland", 720], ["Honolulu", "Pacific/Honolulu", -600], ["Anchorage", "America/Anchorage", -540],
    ["Los Angeles", "America/Los_Angeles", -480], ["San Francisco", "America/Los_Angeles", -480], ["Seattle", "America/Los_Angeles", -480],
    ["Vancouver", "America/Vancouver", -480], ["Denver", "America/Denver", -420], ["Phoenix", "America/Phoenix", -420],
    ["Chicago", "America/Chicago", -360], ["Dallas", "America/Chicago", -360], ["Houston", "America/Chicago", -360],
    ["Mexico City", "America/Mexico_City", -360], ["New York", "America/New_York", -300], ["Toronto", "America/Toronto", -300],
    ["Miami", "America/New_York", -300], ["Washington", "America/New_York", -300], ["Boston", "America/New_York", -300],
    ["Bogota", "America/Bogota", -300], ["Lima", "America/Lima", -300], ["Santiago", "America/Santiago", -240],
    ["Halifax", "America/Halifax", -240], ["Buenos Aires", "America/Argentina/Buenos_Aires", -180], ["Sao Paulo", "America/Sao_Paulo", -180],
    ["Rio de Janeiro", "America/Sao_Paulo", -180], ["Reykjavik", "Atlantic/Reykjavik", 0],
    ["EST", "America/New_York", -300], ["EDT", "America/New_York", -300], ["CST", "America/Chicago", -360], ["CDT", "America/Chicago", -360],
    ["MST", "America/Denver", -420], ["PST", "America/Los_Angeles", -480], ["PDT", "America/Los_Angeles", -480],
    ["CET", "Europe/Paris", 60], ["CEST", "Europe/Paris", 60], ["BST", "Europe/London", 0], ["JST", "Asia/Tokyo", 540], ["IST", "Asia/Kolkata", 330],
    ["AEST", "Australia/Sydney", 600]
];

function city(sRaw) {
    var s = String(sRaw || "").trim().toLowerCase().replace(/\s+/g, " ");
    if (s.length < 2) return null;
    for (var i = 0; i < CITIES.length; i++) if (CITIES[i][0].toLowerCase() === s || CITIES[i][1].toLowerCase() === s) return CITIES[i];
    if (s.length >= 4) for (var j = 0; j < CITIES.length; j++) if (CITIES[j][0].toLowerCase().indexOf(s) === 0) return CITIES[j];
    return null;
}

function offsetOf(c, zones) {
    var o = zones && zones.offsets ? zones.offsets[c[1]] : undefined;
    return typeof o === "number" ? { m: o, real: true } : { m: c[2], real: false };
}

function pad(n) { return (n < 10 ? "0" : "") + n; }
function hm(mins) { mins = ((mins % 1440) + 1440) % 1440; return pad(Math.floor(mins / 60)) + ":" + pad(mins % 60); }
function dayNote(mins) { return mins < 0 ? " (the day before)" : mins >= 1440 ? " (the next day)" : ""; }

// "3pm", "3:30 pm", "15:30", "noon" → minutes after midnight, or -1
function parseTime(sRaw) {
    var s = String(sRaw || "").trim().toLowerCase();
    if (s === "noon") return 720;
    if (s === "midnight") return 0;
    var m = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?$/.exec(s);
    if (!m) return -1;
    var h = parseInt(m[1], 10), mi = m[2] ? parseInt(m[2], 10) : 0;
    if (mi > 59) return -1;
    if (m[3]) { if (h < 1 || h > 12) return -1; h = h % 12 + (m[3][0] === "p" ? 12 : 0); }
    else if (!m[2] || h > 23) return -1;
    return h * 60 + mi;
}

function localOffset(zones, now) {
    if (zones && typeof zones.localOffset === "number") return zones.localOffset;
    return -new Date(now).getTimezoneOffset();
}

function row(title, sub, copy, real) {
    return { plugin: "worldclock", group: "World clock", title: title, sub: sub + (real ? "" : " · standard time (summer time not counted)"),
             icon: "clock", score: 1880, top: true, action: { type: "copy", value: copy } };
}

function results(qRaw, zones, nowMs) {
    var q = String(qRaw || "").trim().replace(/\s+/g, " ");
    if (q.length < 3) return [];
    var now = typeof nowMs === "number" ? nowMs : Date.now();
    var utcMin = Math.floor(now / 60000) % 1440;
    var m;
    // a time from one place to another
    if ((m = /^(.+?)\s+(?:in\s+)?(.+?)\s+(?:to|in|as)\s+(.+)$/i.exec(q))) {
        var t = parseTime(m[1]), a = city(m[2]), b = city(m[3]);
        if (t >= 0 && a && b) {
            var oa = offsetOf(a, zones), ob = offsetOf(b, zones), r = t - oa.m + ob.m;
            return [row(hm(r) + " in " + b[0] + dayNote(r), hm(t) + " in " + a[0] + " → " + b[0], hm(r), oa.real && ob.real)];
        }
    }
    // a time here, somewhere else
    if ((m = /^(.+?)\s+(?:to|in)\s+(.+)$/i.exec(q))) {
        var t2 = parseTime(m[1]), c2 = city(m[2]);
        if (t2 >= 0 && c2) {
            var lo = localOffset(zones, now), o2 = offsetOf(c2, zones), r2 = t2 - lo + o2.m;
            return [row(hm(r2) + " in " + c2[0] + dayNote(r2), hm(t2) + " here → " + c2[0], hm(r2), o2.real)];
        }
    }
    // the time there now
    var place = null;
    if ((m = /^(?:what\s+)?time\s+(?:is\s+it\s+)?(?:in\s+)?(.+)$/i.exec(q))) place = m[1];
    else if ((m = /^(.+?)\s+time$/i.exec(q))) place = m[1];
    var c = place ? city(place) : null;
    if (!c) return [];
    var o = offsetOf(c, zones), there = utcMin + o.m;
    var diff = o.m - localOffset(zones, now), dh = diff / 60;
    var rel = diff === 0 ? "the same as here" : (Math.abs(dh) + (Math.abs(dh) === 1 ? " hour " : " hours ") + (diff > 0 ? "ahead of" : "behind") + " you");
    return [row(hm(there) + " in " + c[0] + dayNote(there), (c[1] === "UTC" ? "UTC" : c[1].replace(/_/g, " ")) + " · " + rel, hm(there), o.real)];
}
