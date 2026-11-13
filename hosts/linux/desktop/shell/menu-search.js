// How the app menus' search ranks what you type (Launcher.qml: every menu style and its search page). Pure, so
// hosts/linux/test/test_menu_search.py runs it in node. Better than "contains": a name that is the query or starts
// with it first, then a word in the name, the initials ("tm" → Task Manager), the program's own name (konsole, dolphin),
// Windows' names for the same thing (explorer, cmd, notepad, control panel, taskmgr), one typo for longer words,
// accents ignored, and what you open often a little higher.
.pragma library

// Windows (and other systems') names → words that find Hearth's app for it.
var ALIASES = {
    "explorer": "files", "file explorer": "files", "finder": "files", "dolphin": "files", "nautilus": "files",
    "cmd": "terminal", "command prompt": "terminal", "powershell": "terminal", "konsole": "terminal", "console": "terminal", "shell": "terminal", "bash": "terminal",
    "notepad": "text", "wordpad": "write", "word": "write", "excel": "sheets", "powerpoint": "slides",
    "calc": "calculator", "control panel": "settings", "control": "settings", "preferences": "settings", "system preferences": "settings",
    "taskmgr": "task manager", "task mgr": "task manager", "activity monitor": "task manager", "processes": "task manager",
    "resmon": "resource monitor", "perfmon": "resource monitor", "msconfig": "settings",
    "paint": "paint", "mspaint": "paint", "snipping tool": "capture", "snip": "capture", "screenshot": "capture",
    "photos": "photos", "pictures": "photos", "media player": "music", "groove": "music", "movies": "video",
    "edge": "browser", "chrome": "browser", "internet": "browser", "web": "browser",
    "store": "store", "microsoft store": "store", "app store": "store", "software": "store",
    "disk management": "disks", "diskmgmt": "disks", "device manager": "devices", "devmgmt": "devices",
    "event viewer": "logs", "eventvwr": "logs", "regedit": "settings", "services": "services",
    "pdf": "reader", "acrobat": "reader", "zip": "archive", "7zip": "archive", "winrar": "archive"
};

function fold(s) {
    s = String(s || "").toLowerCase();
    try { s = s.normalize("NFD").replace(/[̀-ͯ]/g, ""); } catch (e) {}
    return s.replace(/[’']/g, "").replace(/\s+/g, " ").trim();
}

function words(s) { return fold(s).split(/[\s()\[\]._\-\/:,+&]+/).filter(function (w) { return w.length > 0; }); }

// Initials of the words, and of CamelCase parts ("TaskManager" → "tm").
function initials(s) {
    var spaced = String(s || "").replace(/([a-z])([A-Z])/g, "$1 $2");
    return words(spaced).map(function (w) { return w[0]; }).join("");
}

// At most one edit (a letter added, missing, changed, or two swapped) between a and b?
function oneEdit(a, b) {
    if (a === b) return true;
    var la = a.length, lb = b.length;
    if (Math.abs(la - lb) > 1) return false;
    var i = 0;
    while (i < la && i < lb && a[i] === b[i]) i++;
    if (la === lb) {
        if (a.slice(i + 1) === b.slice(i + 1)) return true;                                   // changed
        return i + 1 < la && a[i] === b[i + 1] && a[i + 1] === b[i] && a.slice(i + 2) === b.slice(i + 2);   // swapped
    }
    return la > lb ? a.slice(i + 1) === b.slice(i) : a.slice(i) === b.slice(i + 1);         // added / missing
}

// The query's letters in order inside the text ("tskmgr" in "task manager"), with no more than a few skipped.
function inOrder(q, t) {
    var j = 0, gaps = 0, last = -1;
    for (var i = 0; i < t.length && j < q.length; i++) {
        if (t[i] === q[j]) { if (last >= 0 && i - last > 1) gaps++; last = i; j++; }
    }
    return j === q.length && gaps <= 3;
}

// How well text (a name) fits q: 0 = not at all.
function nameScore(q, text) {
    var t = fold(text);
    if (!t || !q) return 0;
    if (t === q) return 1000;
    if (t.indexOf(q) === 0) return 900 - Math.min(80, t.length - q.length);
    var tc = t.replace(/[^a-z0-9]/g, ""), qc = q.replace(/[^a-z0-9]/g, "");   // "wifi" for "Wi-Fi"
    if (qc && tc === qc) return 980;
    if (qc.length >= 2 && tc.indexOf(qc) === 0) return 880 - Math.min(80, tc.length - qc.length);
    var ws = words(t);
    var qs = q.split(" ");
    for (var k = 0; k < ws.length; k++) if (ws[k].indexOf(q) === 0) return 760 - k * 10;
    // every typed word starts a word of the name, in any order ("man task")
    if (qs.length > 1 && qs.every(function (x) { return ws.some(function (w) { return w.indexOf(x) === 0; }); })) return 700;
    var ini = initials(text);
    if (q.length >= 2 && q.indexOf(" ") < 0 && ini.indexOf(q) === 0) return 650;
    if (t.indexOf(q) > 0) return 560;
    if (q.length >= 4) {
        for (var m = 0; m < ws.length; m++) {
            if (oneEdit(q, ws[m]) || (ws[m].length > q.length && oneEdit(q, ws[m].slice(0, q.length)))) return 420;
        }
    }
    if (q.length >= 3 && q.indexOf(" ") < 0 && inOrder(q, t.replace(/ /g, ""))) return 300;
    return 0;
}

// Lower scores for the other things an app says about itself.
function textScore(q, text, scale) {
    var s = nameScore(q, text);
    if (s >= 560) return Math.round(s * scale);
    var t = fold(text);
    if (t && q.length >= 2 && t.indexOf(q) >= 0) return Math.round(450 * scale);
    return s >= 420 ? Math.round(s * scale * 0.8) : 0;
}

function program(exec) {
    var first = String(exec || "").trim().split(/\s+/);
    var p = first[0] === "env" ? first.filter(function (x) { return x.indexOf("=") < 0; })[1] || "" : first[0] || "";
    if (/^(flatpak)$/.test(p)) { var id = String(exec).match(/run\s+(?:--\S+\s+)*([\w.-]+)/); p = id ? id[1] : p; }
    return fold(p.replace(/^.*\//, ""));
}

// An app: {name, genericName, comment, keywords, id, execString}. uses: {id: times opened}.
function appScore(qRaw, a, uses) {
    var q = fold(qRaw);
    if (!q || !a) return 0;
    var best = Math.max(
        nameScore(q, a.name),
        textScore(q, a.genericName, 0.62),
        textScore(q, Array.from(a.keywords || []).join(" "), 0.6),
        textScore(q, a.comment, 0.45),
        textScore(q, program(a.execString || a.command || ""), 0.7),
        textScore(q, String(a.id || "").replace(/\.desktop$/, "").replace(/^.*\./, ""), 0.55));
    var al = ALIASES[q];
    if (al && al !== q) best = Math.max(best, Math.round(appScore(al, a, null) * 0.85));
    if (best <= 0) return 0;
    var n = uses ? (uses[a.id] || 0) : 0;
    return best + Math.min(60, n * 6);
}

// A game title or a Settings page (name + words).
function itemScore(qRaw, title, extra) {
    var q = fold(qRaw);
    if (!q) return 0;
    return Math.max(nameScore(q, title), textScore(q, extra || "", 0.5));
}

// [items] sorted best first, only the ones that fit; score(item) → number.
function rank(items, score) {
    var out = [];
    for (var i = 0; i < items.length; i++) {
        var s = score(items[i]);
        if (s > 0) out.push({ s: s, i: i, v: items[i] });
    }
    out.sort(function (a, b) { return b.s - a.s || a.i - b.i; });
    return out.map(function (x) { return x.v; });
}
