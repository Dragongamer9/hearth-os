.pragma library
// Search plugin: calendar events (like KRunner's calendar runner): what's coming up in Hearth Calendar (your own events,
// subscribed calendars and Online accounts' calendars), from `hearth-time --events` (repeats included) through
// hearth-search-sources pim. By title or place, the soonest first; "today", "tomorrow" and "this week" list them.
// Enter opens Hearth Calendar.

var DAY = 86400000;

function when(e, now) {
    var d = new Date(e.start), today = new Date(now); today.setHours(0, 0, 0, 0);
    var days = Math.floor((new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() - today.getTime()) / DAY);
    var dayName = days === 0 ? "Today" : days === 1 ? "Tomorrow" : ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][d.getDay()] + " " + e.date;
    if (e.allDay) return dayName + " · all day";
    var h = d.getHours(), m = d.getMinutes();
    return dayName + " · " + (h < 10 ? "0" : "") + h + ":" + (m < 10 ? "0" : "") + m;
}

function results(qRaw, events, nowMs) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 3 || !events || !events.length) return [];
    var now = typeof nowMs === "number" ? nowMs : Date.now();
    var t0 = new Date(now); t0.setHours(0, 0, 0, 0);
    var range = q === "today" ? [t0.getTime(), t0.getTime() + DAY] : q === "tomorrow" ? [t0.getTime() + DAY, t0.getTime() + 2 * DAY]
              : (q === "this week" || q === "week") ? [t0.getTime(), t0.getTime() + 7 * DAY] : null;
    var list = events.slice().sort(function (a, b) { return a.start - b.start; }), out = [];
    for (var i = 0; i < list.length && out.length < 4; i++) {
        var e = list[i], end = e.end || e.start;
        if (end < now && !(range && e.allDay && end > range[0])) continue;
        var hit = range ? e.start < range[1] && end > range[0]
                        : String(e.title || "").toLowerCase().indexOf(q) >= 0 || (q.length >= 4 && String(e.location || "").toLowerCase().indexOf(q) >= 0);
        if (!hit) continue;
        out.push({ plugin: "events", group: "Calendar", title: e.title || "(no title)", sub: when(e, now) + (e.location ? " · " + e.location : ""), icon: "calendar",
                   score: (range ? 1500 : 660) - out.length, top: !!range && out.length === 0, action: { type: "exec", value: ["hearth-time", "--app", "calendar"] } });
    }
    return out;
}
