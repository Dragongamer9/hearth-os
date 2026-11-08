// Hearth Files: paths in drive letters, the Windows way (docs/HEARTH-FILES.md "Drive letters"). Pure functions,
// used by FileManager.qml and checked by hosts/linux/test/test_hearth_this_pc.py (with node).
//
// ctx: { home: "/home/me", user: "me",
//        drives: [{ letter: "C", path: "/", label: "Hearth (C:)" }, { letter: "D", path: "/run/media/me/Games", label: "Games (D:)" }, …],
//        folders: [{ label: "Documents", path: "/home/me/Documents" }, …] }
// C: is always the system: C:\ is /, C:\Users\<you> is your home folder. Other letters are their drive's mount place.
.pragma library

function under(p, root) {
    if (!p || !root) return false;
    const r = root.replace(/\/+$/, "");
    return p === root || p === r || p.startsWith(r + "/") || r === "";
}

function rest(p, root) {
    const r = root.replace(/\/+$/, "");
    return p.length > r.length ? p.slice(r.length + 1) : "";
}

// The lettered drive a path is on (not C:), longest mount place first.
function driveFor(p, ctx) {
    let best = null;
    for (const d of (ctx && ctx.drives) || []) {
        if (!d || !d.letter || !d.path || d.letter === "C" || d.path === "/") continue;
        if (under(p, d.path) && (!best || d.path.length > best.path.length)) best = d;
    }
    return best;
}

function folderFor(p, ctx) {
    let best = null;
    for (const f of (ctx && ctx.folders) || []) {
        if (f && f.path && under(p, f.path) && (!best || f.path.length > best.path.length)) best = f;
    }
    return best;
}

// The address bar's steps for a path: This PC › Games (D:) › SteamLibrary, This PC › Documents › …, or
// This PC › Hearth (C:) › usr › … for the rest of the system. null where Files' own steps fit better (the rest of the
// home folder, the Trash, Hearth's pages, Hearth Storage's hearth:// folders).
function crumbsFor(p, ctx, thisPc) {
    if (!p || p.indexOf("/") !== 0) return null;
    const top = { label: "This PC", path: thisPc };
    const parts = (root, label) => {
        const out = [top, { label: label, path: root }];
        let at = root.replace(/\/+$/, "");
        for (const s of rest(p, root).split("/")) {
            if (!s) continue;
            at += "/" + s;
            out.push({ label: s, path: at });
        }
        return out;
    };
    const d = driveFor(p, ctx);
    if (d) return parts(d.path, d.label || d.letter + ":");
    const f = folderFor(p, ctx);
    if (f) return parts(f.path, f.label);
    if (ctx && ctx.home && under(p, ctx.home)) return null;
    const c = ((ctx && ctx.drives) || []).find(x => x && x.letter === "C");
    return parts("/", c ? c.label : "Hearth (C:)");
}

// D:\SteamLibrary\steamapps for a path on a lettered drive; C:\Users\<you>\… in the home folder; C:\usr\… elsewhere.
function toWindows(p, ctx) {
    if (!p || p.indexOf("/") !== 0) return "";
    const bs = s => s.split("/").filter(x => x).join("\\");
    const d = driveFor(p, ctx);
    if (d) return d.letter + ":\\" + bs(rest(p, d.path));
    if (ctx && ctx.home && under(p, ctx.home)) {
        const r = bs(rest(p, ctx.home));
        return "C:\\Users\\" + (ctx.user || "you") + (r ? "\\" + r : "");
    }
    return "C:\\" + bs(p);
}

// Typed in the address bar: "D:\Games", "d:/Games", "D:" → { path } (the Linux path), { error } for a letter no
// drive has, or null when it isn't a drive-letter path at all.
function fromWindows(text, ctx) {
    const m = /^\s*([A-Za-z]):([\\/].*)?\s*$/.exec(text || "");
    if (!m) return null;
    const letter = m[1].toUpperCase();
    const segs = [];
    for (const s of (m[2] || "").split(/[\\/]+/)) {
        if (!s || s === ".") continue;
        if (s === "..") { segs.pop(); continue; }
        segs.push(s);
    }
    if (letter === "C") {
        if (segs.length >= 2 && segs[0].toLowerCase() === "users" && ctx && ctx.home
                && segs[1].toLowerCase() === String(ctx.user || "").toLowerCase())
            return { path: [ctx.home.replace(/\/+$/, "")].concat(segs.slice(2)).join("/") };
        return { path: "/" + segs.join("/") };
    }
    const d = ((ctx && ctx.drives) || []).find(x => x && x.letter === letter && x.path);
    if (!d) return { error: "There's no drive " + letter + ": on this PC" };
    return { path: [d.path.replace(/\/+$/, "")].concat(segs).join("/") };
}
