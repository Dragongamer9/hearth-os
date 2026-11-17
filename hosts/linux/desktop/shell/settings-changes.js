// The change history's words (1.14, the new Settings: Home › Changed lately, All changes, the help panel's "last
// changed"): a setting's name, its values in words, and "2 min ago". Keys of desktop.json (Settings.qml); anything
// else shows its key.
.pragma library

var NAMES = {
    nightLight: "Night light", wallpaper: "Wallpaper", reduceMotion: "Reduce motion", pointerSpeed: "Pointer speed",
    lockOnWake: "Lock when it wakes", clock24: "24-hour clock", clipboardHistory: "Clipboard history",
    screenOffMinutes: "Screen off after", sleepMinutes: "Sleep after", look: "Look", mode: "Mode", accent: "Accent colour",
    countMe: "Count me in the player count", barStyle: "Bar style", dockMode: "Dock", deskClock: "Desktop clock",
    panelGlass: "Panel transparency", sunset: "Sunset mode", sunsetLight: "Light from", sunsetDark: "Dark from",
    barLayout: "Bar layout", barEdge: "Main bar's edge", dockFloat: "Floating dock", appMenu: "App menu",
    shareStatus: "Share this device's status", menuStyle: "Right-click menu", subTheme: "Sub theme", wallMatch: "Match sub theme",
    nightTemp: "Night light warmth", nightMode: "Night light on", nightFrom: "Night light from", nightTo: "Night light until"
};
// Keys that change with another one and mean nothing on their own: never listed.
var HIDDEN = { accentFrom: true, wallSwatches: true, menuTiles: true };

function name(key) { return NAMES[key] || key; }
function shown(c) { return !HIDDEN[c.key]; }

function minutes(v) { return v === 0 ? "Never" : v >= 60 && v % 60 === 0 ? (v / 60) + (v === 60 ? " hour" : " hours") : v + " min"; }

function value(key, v, accents) {
    if (v === null || v === undefined || v === "") {
        if (key === "wallpaper") return "The look's own";
        if (key === "accent" || key === "subTheme" || key === "barStyle" || key === "dockMode" || key === "deskClock" || key === "panelGlass") return "The look's own";
        if (key === "appMenu" || key === "menuStyle" || key === "barLayout" || key === "barEdge" || key === "dockFloat") return "Default";
        return "Off";
    }
    if (typeof v === "boolean") return v ? "On" : "Off";
    if (key === "screenOffMinutes" || key === "sleepMinutes") return minutes(Number(v));
    if (key === "pointerSpeed") return Math.round(Number(v) * 100) + "%";
    if (key === "nightTemp") return Number(v) + " K";
    if (key === "nightMode") return v === "always" ? "All the time" : "Evenings";
    if (key === "accent") {
        const a = (accents || []).find(x => String(x.color).toLowerCase() === String(v).toLowerCase());
        return a ? a.name : String(v);
    }
    if (key === "wallpaper") {
        const s = String(v);
        if (s.indexOf("builtin:") === 0 || s.indexOf("anim:") === 0) return s.split(":")[1].replace(/[-_]/g, " ").replace(/^./, c => c.toUpperCase());
        return s.split("/").pop();
    }
    if (typeof v === "string") return v.replace(/^./, c => c.toUpperCase());
    return String(v);
}

function ago(at, nowSec) {
    const d = Math.max(0, (nowSec || Math.floor(Date.now() / 1000)) - at);
    if (d < 60) return "just now";
    if (d < 3600) return Math.floor(d / 60) + " min ago";
    const then = new Date(at * 1000), now = new Date((nowSec || Math.floor(Date.now() / 1000)) * 1000);
    const days = Math.round((new Date(now.getFullYear(), now.getMonth(), now.getDate()) - new Date(then.getFullYear(), then.getMonth(), then.getDate())) / 86400000);
    if (days === 0) return Math.floor(d / 3600) + (d < 7200 ? " hour ago" : " hours ago");
    if (days === 1) return "yesterday";
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    return months[then.getMonth()] + " " + then.getDate();
}

// "Accent colour → Ember orange"
function line(c, accents) { return name(c.key) + " → " + value(c.key, c.to, accents); }

// X Default in the controller look: what each one goes back to (Settings.qml's own defaults; "" is the look's own).
var DEFAULTS = {
    nightLight: false, wallpaper: "", reduceMotion: false, pointerSpeed: 1.0, lockOnWake: false, clock24: false,
    clipboardHistory: true, screenOffMinutes: 10, sleepMinutes: 0, mode: "dark", accent: "", countMe: true,
    barStyle: "", dockMode: "", deskClock: "", panelGlass: "", sunset: false, sunsetLight: "07:00", sunsetDark: "19:30",
    barLayout: "", barEdge: "", dockFloat: "", appMenu: "", shareStatus: true, menuStyle: "", subTheme: "", wallMatch: true,
    nightTemp: 3700, nightMode: "times", nightFrom: "19:30", nightTo: "07:00"
};
