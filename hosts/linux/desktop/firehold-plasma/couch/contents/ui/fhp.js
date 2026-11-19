// Firehold on Plasma's own rules for the Couch home (mockups docs/mockups/firehold-plasma 01-03), on top of
// firehold.js (Mantel's Firehold's, shared). Pure functions: the checks run them in node (test_firehold_plasma.py).
.pragma library

// The approved mockups' colours (hearth-workspace hearth/mockups/firehold-plasma-more/firehold.html :root).
var P = { ac: "#ff7a2f", ac2: "#ffb36b", tx: "#f2ede8", mut: "#a39d96", dim: "#6f6a66", ok: "#5fd38d", warn: "#ffc65c",
          bad: "#ff5d5d", blue: "#5bb8ff", tab: "#9a928b", card: "#dc1e1a18", cardLine: "#12ffffff", pill: "#12ffffff",
          nb: "#17ffffff", btn: "#14ffffff", note: "#231f1d", ink: "#1d0d03" };
var KEYS = { A: "#5fd38d", B: "#ff6b6b", X: "#5bb8ff", Y: "#ffd35c" };
function keyColor(k) { return KEYS[k] || "#bfb6ad"; }

var DAY = 86400000;

// "41 min ago", "1 h ago", "Yesterday", "3 days", "Last week", "Last month".
function agoShort(t, now) {
    if (!t) return "";
    var s = Math.max(0, now - t) / 1000;
    if (s < 60) return "Now";
    if (s < 3600) return Math.floor(s / 60) + " min ago";
    if (s < 86400) return Math.floor(s / 3600) + " h ago";
    if (s < 2 * 86400) return "Yesterday";
    if (s < 7 * 86400) return Math.floor(s / 86400) + " days";
    if (s < 14 * 86400) return "Last week";
    return "Last month";
}
// "today", "Mon", "2 days", "last month": when it came onto this PC.
function addedWhen(t, now) {
    if (!t) return "";
    var d = Math.floor((now - t) / DAY);
    if (d < 1) return "today";
    if (d < 7) return ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][new Date(t).getDay()];
    if (d < 30) return Math.floor(d / 7) + (d < 14 ? " week ago" : " weeks ago");
    return "last month";
}
function hm(min) {
    var m = Math.max(0, Math.round(min || 0));
    if (m < 60) return m + " min";
    return Math.floor(m / 60) + " h" + (m % 60 ? " " + (m % 60) + " min" : "");
}

// The hero's chapter line: "Angmar campaign · Chapter 5 of 8: The Fall of Arthedain"; "" for a game that doesn't say.
function chapterLine(p) {
    if (!p) return "";
    var parts = [];
    if (p.campaign) parts.push(/campaign$/i.test(p.campaign) ? p.campaign : p.campaign + " campaign");
    if (p.chapter) parts.push("Chapter " + p.chapter + (p.of ? " of " + p.of : "") + (p.name ? ": " + p.name : ""));
    else if (p.name) parts.push(p.name);
    return parts.join(" · ");
}
// The hero's details line: mod, launcher, where it's saved, how it runs.
function detailsLine(g, p, storage) {
    if (!g) return "";
    var parts = [];
    p = p || {};
    if (p.details) parts.push(p.details);
    if (p.mod) parts.push(p.mod);
    if (p.launcher) parts.push("through " + p.launcher);
    parts.push(g.cloudSaves ? "saved on this PC and in the cloud" : (g.savesBackedUp && storage) ? "saved on this PC and on Hearth Storage"
               : g.savesBackedUp ? "saved on this PC and backed up" : "saved on this PC");
    parts.push("runs natively, no Game Mode");
    return parts.join(" · ");
}
function hours(g, p) {
    if (p && p.hours) return p.hours;
    return g && g.minutes ? Math.round(g.minutes / 60) : 0;
}

// Continue playing: what you pinned first, then what was played last; the hero's game left out.
function continueRow(games, pins, skipId, n) {
    var list = (games || []).filter(function (g) { return g.id !== skipId && g.installed; });
    var pinned = (pins || []).map(function (id) { return list.filter(function (g) { return g.id === id; })[0]; }).filter(Boolean);
    var played = list.filter(function (g) { return g.lastPlayed > 0 && pinned.indexOf(g) < 0; })
                     .sort(function (a, b) { return b.lastPlayed - a.lastPlayed; });
    return pinned.concat(played).slice(0, n);
}
// A Continue playing tile's line: who played it (when it wasn't you) or where you're up to, then when.
function continueSub(g, p, now, me) {
    p = p || {};
    var when = agoShort(g.lastPlayed, now);
    if (p.who && p.who !== me) return p.who + " played " + (/^\d/.test(when) ? when : when.toLowerCase());
    var days = g.lastPlayed ? (now - g.lastPlayed) / DAY : 0;
    if (p.build) return p.build + " · " + (days >= 2 && days < 7 ? "last " + ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"][new Date(g.lastPlayed).getDay()] : when.toLowerCase());
    if (p.campaign) return p.campaign + (/campaign$/i.test(p.campaign) ? "" : " campaign") + " · " + when.toLowerCase();
    if (g.minutes && /^(Yesterday|\d+ h ago|\d+ min ago)$/.test(when)) return when + " · " + hm(g.minutes);
    return when;
}

// Recently added: the newest games on this PC, from every store.
function addedRow(games, added, now, n) {
    var a = added || {};
    var list = (games || []).filter(function (g) { return (a[g.id] || g.added) && (now - (a[g.id] || g.added)) < 45 * DAY; });
    list.sort(function (x, y) { return (a[y.id] || y.added) - (a[x.id] || x.added); });
    return list.slice(0, n);
}
function addedSub(g, p, t, now) {
    p = p || {};
    if (g.devBuild) return "Dev build" + (p.launcher ? " from " + p.launcher : "");
    if (p.version) return "Version " + p.version + (p.launcher ? " · " + p.launcher : "");
    if (p.note) return p.note;
    var w = addedWhen(t, now);
    return g.store + " · " + (w === "today" ? "installed today" : "added " + w);
}
// A tile's corner badge (mockups 01/02): NEW, DEV, UPDATE READY, UPDATING 68%, CLOUD SAVES, the download size.
function badge(g, dl, added, now) {
    var d = dl ? dl[g.id] : null;
    if (d && d.state === "running") return { text: "UPDATING " + d.percent + "%", color: "#ffb36b" };
    if (g.update) return { text: "UPDATE READY", color: "#ffb36b" };
    var t = added ? added[g.id] : 0;
    if (g.devBuild && t && now - t < 14 * DAY) return { text: "DEV", color: "#b9a8ff" };
    if (t && now - t < 7 * DAY) return { text: "NEW", color: "#ffb36b" };
    if (!g.installed && g.size) return { text: g.size, color: "#cccccc" };
    if (g.cloudSaves) return { text: "CLOUD SAVES", color: "#9fd8ff" };
    return null;
}

// The downloads card's lines.
function downloadsSummary(list) {
    var r = 0, w = 0;
    (list || []).forEach(function (d) { if (d.state === "running") r++; else w++; });
    return [r ? r + " running" : "", w ? w + " waiting" : ""].filter(Boolean).join(" · ");
}
function downloadsMap(list) {
    var m = {};
    (list || []).forEach(function (d) { m[d.id] = d; });
    return m;
}

// The controllers pill and card: "72% · 18%"; "You · 72%", "Player 2 · 18% charge soon", a headset's "60%".
function padsPill(list) {
    return (list || []).filter(function (c) { return c.kind === "pad"; }).map(function (c) { return c.percent + "%"; }).join(" · ");
}
function deviceLabel(c) {
    if (c.kind === "pad") return (c.player === 1 ? "You" : "Player " + c.player) + " · " + c.percent + "%" + (c.low ? " charge soon" : "");
    return c.percent + "%";
}
function lowPad(list) {
    var l = (list || []).filter(function (c) { return c.kind === "pad" && c.percent <= 15 && !c.charging; });
    return l.length ? l[0] : null;
}

// From your Hearth: each device's line and its button.
function deviceCard(d, now) {
    var name = d.name || "Device";
    var mins = d.lastSeen ? Math.max(0, Math.round((now - d.lastSeen) / 60000)) : 0;
    if (d.online && d.playing) {
        return { title: name + " · in " + d.playing, sub: "Online " + hm(d.uptime / 60).replace(/ \d+ min$/, "") + " · Hearth Link" + (d.family ? ", family" : ""),
                 action: d.family ? "help" : "", button: d.family ? (/dad|mom|mum/i.test(name) ? (/dad/i.test(name) ? "Help him" : "Help her") : "Help") : "" };
    }
    if (d.online) {
        var asleep = d.kind === "handheld" && d.mode === "sleep";
        return { title: name + (asleep ? " · asleep" : " · on"), sub: [d.savesAt ? "Saves sent " + agoShort(d.savesAt, now).toLowerCase() : "Hearth Link" + (d.family ? ", family" : ""),
                                                                       d.battery !== null && d.battery !== undefined ? d.battery + "%" : ""].filter(Boolean).join(" · "),
                 action: d.family ? "help" : "", button: d.family ? "Help" : "" };
    }
    var off = d.kind === "handheld" ? "asleep" : "off";
    return { title: name + " · " + off,
             sub: d.backupAt ? "Backed up " + (now - d.backupAt < 36 * 3600000 && new Date(d.backupAt).getHours() >= 18 ? "last night" : agoShort(d.backupAt, now).toLowerCase())
                  : d.savesAt ? "Saves sent " + agoShort(d.savesAt, now).toLowerCase() + (d.battery !== null && d.battery !== undefined ? " · " + d.battery + "%" : "")
                  : mins ? "Last seen " + agoShort(d.lastSeen, now).toLowerCase() : "Hearth Link",
             action: d.wake ? "wake" : "", button: d.wake ? "Wake" : "" };
}

// Watch and listen: the media apps you have, in the mockup's order; a web app says it opens in Firefox.
var MEDIA = [["jellyfin", "Jellyfin", "#2a3a6a", "#7a4aa0"], ["youtube", "YouTube", "#5a1a1a", "#a03a2a"], ["plex", "Plex", "#3a3010", "#a08020"],
             ["spotify", "Spotify", "#123a24", "#2a8a4a"], ["twitch", "Twitch", "#2a1a4a", "#6a3aa0"], ["hearth-photos", "Hearth Photos", "#4a2a18", "#c06a2a"],
             ["moonlight", "Moonlight", "#1a2a3a", "#3a6a8a"], ["hearth-tv", "Hearth TV", "#3a1a10", "#ff7a2f"], ["kodi", "Kodi", "#14283a", "#2a7aa0"],
             ["vlc", "VLC", "#3a2410", "#c0701a"], ["netflix", "Netflix", "#3a0a0a", "#a01a1a"], ["haruna", "Haruna", "#2a1a3a", "#6a4a9a"]];
function mediaApps(apps) {
    var out = [];
    MEDIA.forEach(function (m) {
        var a = (apps || []).filter(function (x) {
            var id = String(x.id || "").toLowerCase(), n = String(x.name || "").toLowerCase();
            return id.indexOf(m[0]) >= 0 || n === m[1].toLowerCase() || n.replace(/[^a-z]/g, "") === m[0].replace(/-/g, "");
        })[0];
        if (a) out.push({ key: m[0], name: m[1], c1: m[2], c2: m[3], app: a });
    });
    return out;
}
function mediaSub(m, player, extras) {
    if (player && player.app && (player.app.toLowerCase().indexOf(m.key.replace("-", "")) >= 0 || player.identity.toLowerCase() === m.name.toLowerCase())) {
        return (player.playing ? "" : "Paused · ") + (player.short || player.title);
    }
    if (extras && extras[m.key]) return extras[m.key];
    if (m.app.web) return "in Firefox";
    if (m.key === "moonlight") return "Stream from the PC";
    if (m.key === "hearth-tv") return "The TV app";
    return m.app.comment || "";
}

// "23:10 / 58:40" from microseconds.
function clock(us) {
    var s = Math.max(0, Math.floor((us || 0) / 1e6));
    var h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, x = s % 60;
    var mm = (h ? (m < 10 ? "0" : "") : "") + m;
    return (h ? h + ":" : "") + mm + ":" + (x < 10 ? "0" : "") + x;
}

// This screen: a TV or a monitor (Qt's screen name, its model and its size).
function isTv(name, model, diagMm) {
    if (/tv/i.test(String(model || ""))) return true;
    return /hdmi/i.test(String(name || "")) && diagMm > 900;
}
