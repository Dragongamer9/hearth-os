.pragma library
// Search plugin: virtual desktops and activities by name (like KRunner's desktops and activities runners).
// data: {desktops: [{n, name, active}] (KWin's, WinList.desktops), activities: [{id, name, current}] (Activities.qml)}.
// "desktop 2", "work", "activity games": Enter switches to it. The one you're on says so and isn't offered.

function results(qRaw, data) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 2 || !data) return [];
    var out = [];
    var kindD = /^(desktop|desk)\s*/.exec(q), kindA = /^activit(y|ies)\s*/.exec(q);
    var wd = kindD ? q.slice(kindD[0].length) : q, wa = kindA ? q.slice(kindA[0].length) : q;
    var ds = data.desktops || [], as = data.activities || [];
    if (!kindA) for (var i = 0; i < ds.length; i++) {
        var d = ds[i], name = String(d.name || ("Desktop " + d.n)), l = name.toLowerCase();
        var hit = kindD ? (wd === "" || l.indexOf(wd) >= 0 || String(d.n) === wd) : l.indexOf(wd) === 0 || (wd.length >= 3 && l.indexOf(wd) > 0);
        if (!hit) continue;
        out.push({ plugin: "desktops", group: "Virtual desktops", title: d.active ? name + " (you're on it)" : "Switch to " + name,
                   sub: "Virtual desktop " + d.n, icon: "desktops", score: (kindD ? 1500 : 760) - i,
                   top: !!kindD && !d.active && out.length === 0, action: d.active ? null : { type: "desktop", value: d.n } });
    }
    if (!kindD) for (var j = 0; j < as.length; j++) {
        var a = as[j], an = String(a.name || a.id), al = an.toLowerCase();
        var ahit = kindA ? (wa === "" || al.indexOf(wa) >= 0) : al.indexOf(wa) === 0 || (wa.length >= 3 && al.indexOf(wa) > 0);
        if (!ahit) continue;
        out.push({ plugin: "desktops", group: "Activities", title: a.current ? an + " (you're on it)" : "Switch to the " + an + " activity",
                   sub: "Activity · its own windows, wallpaper and pinned apps", icon: "grid", score: (kindA ? 1500 : 750) - j,
                   top: !!kindA && !a.current && out.length === 0, action: a.current ? null : { type: "activity", value: a.id } });
    }
    return out.slice(0, 6);
}
