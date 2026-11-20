// Firehold on Plasma's colours and words for the shared Quick settings and notifications (style "firehold";
// mockups docs/mockups/firehold-plasma/07-10): the same ones Firehold on Plasma's own pieces use (its accent
// #ff7a2f, the Desk's blue #5bb8ff, its text #f6ede6), on its dark brown panels.
.pragma library

var panel = "#211b18";
var line = "#3a302a";
var tile = "#2b2421";
var tileOn = "#4a2c1d";
var dot = "#3b332f";
var seg = "#2c2522";
var accent = "#ff7a2f";
var desk = "#5bb8ff";
var text = "#f6ede6";
var muted = "#a99c93";
var warn = "#f5b842";
var good = "#3ecf8e";
var danger = "#ff5d5d";
var scrim = "#99000000";

function font(fams) { return fams.indexOf("Inter") >= 0 ? "Inter" : "Noto Sans"; }

function modeName(m) { return m === "couch" ? "Couch" : m === "away" ? "Away" : "Desk"; }

// "now", "2 min ago", "1 h", "Wed 11:40 PM": a notification's time, as the cards and the list say it.
function ago(t, short) {
    var d = t instanceof Date ? t : new Date(Number(t) || 0);
    var s = Math.max(0, (Date.now() - d.getTime()) / 1000);
    if (s < 60) return "now";
    if (s < 3600) return Math.floor(s / 60) + (short ? " min" : " min ago");
    if (s < 86400 && sameDay(d, new Date())) return Math.floor(s / 3600) + (short ? " h" : " h ago");
    return Qt.formatDateTime(d, "ddd h:mm AP");
}
function sameDay(a, b) { return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate(); }
function dayOf(t) {
    var d = t instanceof Date ? t : new Date(Number(t) || 0), now = new Date();
    if (sameDay(d, now)) return "Today";
    var y = new Date(now.getTime() - 86400000);
    if (sameDay(d, y)) return "Yesterday";
    return Qt.formatDate(d, "dddd, MMMM d");
}
function initial(s) { var t = String(s || "").trim(); return t ? t.charAt(0).toUpperCase() : "·"; }
// An app's tile colour (the letter tiles in the cards and the list): the same app keeps its colour.
function tint(app) {
    var cs = ["#2d4a35", "#3a2a24", "#4a2426", "#2b2f5a", "#2c3a46", "#3e3524", "#3a2a4a"];
    var h = 0, s = String(app || "");
    for (var i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 9973;
    return cs[h % cs.length];
}
