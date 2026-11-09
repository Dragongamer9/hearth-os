// Hearth Files: paths in drive letters, the Windows way (docs/HEARTH-FILES.md "Drive letters"). Pure functions,
// used by FileManager.qml and checked by hosts/linux/test/test_hearth_this_pc.py (with node).
//
// ctx: { home: "/home/me", user: "me",
//        drives: [{ letter: "C", path: "/", label: "Hearth (C:)" }, { letter: "D", path: "/run/media/me/Games", label: "Games (D:)" }, …],
//        folders: [{ label: "Documents", path: "/home/me/Documents" }, …] }
// C: is always the system: C:\ is /, C:\Users\<you> is your home folder. Other letters are their drive's mount place.
//
// Hearth (C:) with Windows folder names (docs/HEARTH-FILES.md "Hearth (C:) the Windows way"), when ctx.c.on: a view,
// nothing on disk moves. ctx.c: { on: true, users: "/var/home" (the folder home folders are in),
//        apps: [{ name: "Firefox", path: "/var/lib/flatpak/app/org.mozilla.firefox", x86: false }, …] }
//   C:\                          the view's top (hearth:c): Program Files, Program Files (x86), ProgramData, Users, Windows
//   C:\Program Files[\<app>]     a view of every installed app (hearth:c/Program Files); <app> is its real folder
//   C:\Program Files (x86)[\<app>] Windows apps and Steam, the same way
//   C:\ProgramData               /var/lib (look only)
//   C:\Windows                   /usr (look only)
//   C:\Users                     the folder the home folders are in (/var/home); C:\Users\<you> is your home folder
//   C:\Users\<you>\AppData       a view (hearth:c/Users/<you>/AppData): Roaming = ~/.config, Local = ~/.local/share,
//                                Local\Temp = ~/.cache
//   C:\etc and the rest           the real top-level folder (/etc)
// Without ctx.c (or with ctx.c.on false) everything is exactly as before 1.10. Windows apps never see any of this:
// inside a Wine prefix C: stays the prefix's own drive.
.pragma library

var C_ROOT = "hearth:c";

function cOn(ctx) { return !!(ctx && ctx.c && ctx.c.on); }
function usersDir(ctx) {
    if (ctx && ctx.c && ctx.c.users) return ctx.c.users.replace(/\/+$/, "");
    const h = ((ctx && ctx.home) || "/home/you").replace(/\/+$/, "");
    return h.slice(0, h.lastIndexOf("/")) || "/";
}
function joinPath(root, segs) {
    const r = root.replace(/\/+$/, "");
    const rest = segs.filter(s => s).join("/");
    return rest ? (r === "" ? "/" + rest : r + "/" + rest) : (r || "/");
}
// The view's pages: "root" (C:\), "pf" (Program Files), "x86" (Program Files (x86)), "appdata" (Users\<you>\AppData);
// "" for anything else (a hearth:c path that stands for a real folder, or not a hearth:c path at all).
function cPage(p, ctx) {
    if (!p || p.indexOf(C_ROOT) !== 0) return "";
    const segs = p.slice(C_ROOT.length).split("/").filter(s => s).map(s => s.toLowerCase());
    if (!segs.length) return "root";
    if (segs.length === 1 && segs[0] === "program files") return "pf";
    if (segs.length === 1 && segs[0] === "program files (x86)") return "x86";
    if (segs.length === 3 && segs[0] === "users" && segs[2] === "appdata"
            && segs[1] === String((ctx && ctx.user) || "").toLowerCase()) return "appdata";
    return "";
}
function isCPath(p) { return !!p && p.indexOf(C_ROOT) === 0; }
// What a C:\ form path (as segments after "C:") stands for with the view on: { path } (a real path or a view page) or
// { error }.
function cSegsToPath(segs, ctx) {
    if (!segs.length) return { path: C_ROOT };
    const low = segs.map(s => s.toLowerCase());
    const c = ctx.c || {};
    const home = (ctx.home || "").replace(/\/+$/, "");
    if (low[0] === "program files" || low[0] === "program files (x86)") {
        const x86 = low[0] === "program files (x86)";
        const page = C_ROOT + "/" + (x86 ? "Program Files (x86)" : "Program Files");
        if (segs.length === 1) return { path: page };
        const a = (c.apps || []).find(x => x && x.path && !!x.x86 === x86 && String(x.name).toLowerCase() === low[1]);
        if (!a) return { error: "There's no “" + segs[1] + "” in " + (x86 ? "Program Files (x86)" : "Program Files") };
        return { path: joinPath(a.path, segs.slice(2)) };
    }
    if (low[0] === "programdata") return { path: joinPath("/var/lib", segs.slice(1)) };
    if (low[0] === "windows") return { path: joinPath("/usr", segs.slice(1)) };
    if (low[0] === "users") {
        if (segs.length === 1) return { path: usersDir(ctx) };
        if (home && low[1] === String(ctx.user || "").toLowerCase()) {
            const r = segs.slice(2), rl = low.slice(2);
            if (rl[0] === "appdata") {
                if (r.length === 1) return { path: C_ROOT + "/Users/" + ctx.user + "/AppData" };
                if (rl[1] === "roaming") return { path: joinPath(home + "/.config", r.slice(2)) };
                if (rl[1] === "local" && rl[2] === "temp") return { path: joinPath(home + "/.cache", r.slice(3)) };
                if (rl[1] === "local") return { path: joinPath(home + "/.local/share", r.slice(2)) };
                return { error: "AppData has Roaming and Local (Local\\Temp too)" };
            }
            return { path: joinPath(home, r) };
        }
        return { path: joinPath(usersDir(ctx), segs.slice(1)) };
    }
    return { path: "/" + segs.join("/") };
}
// The real place a hearth:c path stands for ({ path }; a view page answers with itself), or { error }.
function cResolve(p, ctx) {
    if (!isCPath(p)) return { path: p };
    const segs = p.slice(C_ROOT.length).split("/").filter(s => s);
    return cSegsToPath(segs, Object.assign({}, ctx, { c: Object.assign({ apps: [] }, (ctx && ctx.c) || {}) }));
}
// The real places the view maps, longest first: [{ path, win }]. Apps first (they're inside the others).
function cMap(ctx) {
    const out = [];
    const c = ctx.c || {};
    for (const a of c.apps || []) {
        if (a && a.path && a.path.indexOf("/") === 0 && a.name && !a.noMap)
            out.push({ path: a.path.replace(/\/+$/, ""), win: "C:\\" + (a.x86 ? "Program Files (x86)" : "Program Files") + "\\" + a.name });
    }
    const home = (ctx.home || "").replace(/\/+$/, "");
    if (home) {
        const u = ctx.user || "you";
        out.push({ path: home + "/.config", win: "C:\\Users\\" + u + "\\AppData\\Roaming" });
        out.push({ path: home + "/.local/share", win: "C:\\Users\\" + u + "\\AppData\\Local" });
        out.push({ path: home + "/.cache", win: "C:\\Users\\" + u + "\\AppData\\Local\\Temp" });
        out.push({ path: home, win: "C:\\Users\\" + u });
    }
    out.push({ path: usersDir(ctx), win: "C:\\Users" });
    out.push({ path: "/var/lib", win: "C:\\ProgramData" });
    out.push({ path: "/usr", win: "C:\\Windows" });
    out.sort((a, b) => b.path.length - a.path.length);
    return out;
}
function cWindows(p, ctx) {
    for (const m of cMap(ctx)) {
        if (p === m.path || p.startsWith(m.path + "/")) {
            const r = p.slice(m.path.length).split("/").filter(x => x).join("\\");
            return m.win + (r ? "\\" + r : "");
        }
    }
    return "C:\\" + p.split("/").filter(x => x).join("\\");
}
// Does the view name this real place (not just the plain C:\etc fallback)? For crumbs and the address bar.
function cMapped(p, ctx) {
    if (!cOn(ctx) || !p || p.indexOf("/") !== 0) return false;
    return cMap(ctx).some(m => p === m.path || p.startsWith(m.path + "/"));
}

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
    if (isCPath(p)) return cCrumbs(toWindows(p, ctx), ctx, thisPc);
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
    if (cOn(ctx)) {
        // The view's names: This PC › Hearth (C:) › Program Files › Firefox, … › ProgramData › …, … › Users › … ›
        // AppData › Roaming. Your own folders and the rest of your home folder keep their steps.
        if (ctx.home && under(p, ctx.home)) {
            const m = cMapFor(p, ctx);
            if (!m || m.length <= ctx.home.replace(/\/+$/, "").length) return null;
        }
        return cCrumbs(cWindows(p, ctx), ctx, thisPc);
    }
    if (ctx && ctx.home && under(p, ctx.home)) return null;
    const c = ((ctx && ctx.drives) || []).find(x => x && x.letter === "C");
    return parts("/", c ? c.label : "Hearth (C:)");
}
// The mapped place a path is in (the longest), or "".
function cMapFor(p, ctx) {
    for (const m of cMap(ctx)) if (p === m.path || p.startsWith(m.path + "/")) return m.path;
    return "";
}
// The steps of a C:\ form: each step is where typing it would go.
function cCrumbs(win, ctx, thisPc) {
    ctx = Object.assign({}, ctx, { c: Object.assign({ apps: [] }, (ctx && ctx.c) || {}) });
    const c = ((ctx && ctx.drives) || []).find(x => x && x.letter === "C");
    const out = [{ label: "This PC", path: thisPc }, { label: c ? c.label : "Hearth (C:)", path: C_ROOT }];
    const segs = win.replace(/^[Cc]:/, "").split("\\").filter(x => x);
    for (let i = 0; i < segs.length; i++) {
        const r = cSegsToPath(segs.slice(0, i + 1), ctx);
        out.push({ label: segs[i], path: r.path || C_ROOT });
    }
    return out;
}

// D:\SteamLibrary\steamapps for a path on a lettered drive; C:\Users\<you>\… in the home folder; C:\usr\… elsewhere
// (with the view on: C:\Windows\…, C:\ProgramData\…, C:\Program Files\<app>\…, …\AppData\Roaming\…).
function toWindows(p, ctx) {
    if (isCPath(p)) return "C:\\" + p.slice(C_ROOT.length).split("/").filter(x => x).join("\\");
    if (!p || p.indexOf("/") !== 0) return "";
    const bs = s => s.split("/").filter(x => x).join("\\");
    const d = driveFor(p, ctx);
    if (d) return d.letter + ":\\" + bs(rest(p, d.path));
    if (cOn(ctx)) return cWindows(p, ctx);
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
    if (letter === "C" && cOn(ctx)) return cSegsToPath(segs, ctx);
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

// ---- The view's pages (FilesCDrive.qml draws them; FileManager.qml's `entries`)
// data: { apps: hearth-files installed-apps' list (with each app's real `path` / `dir`), hearth: Hearth's own apps' folder,
//         steam: Steam's folder ("" if none), linux: the real top-level folder names, showLinux: the switch,
//         flatpak: [the Flatpak app folders] }
// Each entry: { name, path, dir: true, cv: { type, size, linux, win, real: [lines], note, lock, look, dim, group, app,
//              places: [{ name, path, size }] (Properties), kind } }. Pure: checked by test_hearth_cdrive.py with node.
function tilde(p, ctx) {
    const h = ((ctx && ctx.home) || "").replace(/\/+$/, "");
    return h && (p === h || p.startsWith(h + "/")) ? "~" + p.slice(h.length) : p;
}
function cName(n) { return String(n || "?").replace(/[\\/]/g, "-").trim() || "?"; }
// Names Windows-style: each app once; a second app with the same name gets its id after it.
function cAppList(data, ctx) {
    const out = [];
    const seen = {};
    const add = (name, a, x86, path) => {
        let n = cName(name);
        if (seen[(x86 ? "x:" : "p:") + n.toLowerCase()]) n = n + " (" + cName(a.appId || a.id || "2") + ")";
        seen[(x86 ? "x:" : "p:") + n.toLowerCase()] = true;
        out.push({ name: n, path: path, x86: x86, app: a });
    };
    const apps = (data && data.apps) || [];
    const sorted = apps.slice().sort((a, b) => String(a.name || "").toLowerCase() < String(b.name || "").toLowerCase() ? -1 : 1);
    for (const a of sorted) {
        if (a.kind === "flatpak" && a.path) add(a.name, a, false, a.path);
        else if (a.kind === "recomp" && a.path) add(a.name, a, false, a.path);
        else if (a.kind === "winapp" && (a.dir || a.path)) add(a.name, a, true, a.dir || a.path);
    }
    const g = hearthGames(data);
    // Hearth Games over several stores' folders opens at the folder they're in, but doesn't rename that folder (~/Games
    // stays C:\Users\<you>\Games): noMap.
    if (g) out.push({ name: "Hearth Games", path: g.path, x86: false, app: null, noMap: g.dirs.length > 1 });
    if (data && data.hearth) out.push({ name: "Hearth", path: data.hearth, x86: false, app: null });
    if (data && data.steam) out.push({ name: "Steam", path: data.steam, x86: true, app: null });
    return out;
}
function commonDir(paths) {
    if (!paths.length) return "";
    let parts = paths[0].split("/");
    for (const p of paths.slice(1)) {
        const q = p.split("/");
        let i = 0;
        while (i < parts.length && i < q.length && parts[i] === q[i]) i++;
        parts = parts.slice(0, i);
    }
    return parts.join("/") || "/";
}
function parentDir(p) { const s = p.replace(/\/+$/, ""); return s.slice(0, s.lastIndexOf("/")) || "/"; }
// Hearth Games' stores (GOG, Epic…): one folder, at the folder their games are in.
function hearthGames(data) {
    const gs = ((data && data.apps) || []).filter(a => a.kind === "games" && a.path && a.path.indexOf("/") === 0);
    if (!gs.length) return null;
    const dirs = [];
    for (const a of gs) { const d = parentDir(a.path); if (dirs.indexOf(d) < 0) dirs.push(d); }
    const common = dirs.length === 1 ? dirs[0] : commonDir(dirs);
    const stores = [];
    for (const a of gs) if (a.publisher && stores.indexOf(a.publisher) < 0) stores.push(a.publisher);
    return { path: common === "/" ? dirs[0] : common, dirs: dirs, stores: stores, size: gs.reduce((n, a) => n + (a.size || 0), 0), count: gs.length };
}
function andList(xs) { return xs.length <= 1 ? (xs[0] || "") : xs.slice(0, -1).join(", ") + " and " + xs[xs.length - 1]; }
function cEntries(page, ctx, data) {
    data = data || {};
    const home = ((ctx && ctx.home) || "").replace(/\/+$/, "");
    const user = (ctx && ctx.user) || "you";
    const users = usersDir(ctx);
    const T = p => tilde(p, ctx);
    const out = [];
    const add = (name, path, cv) => out.push({ name: name, path: path, dir: true, size: cv.size || 0, mtime: 0, cv: cv });
    const apps = (data.apps || []);
    const flatDirs = (data.flatpak && data.flatpak.length) ? data.flatpak : ["/var/lib/flatpak/app"];
    const winDirs = [];
    for (const a of apps) if (a.kind === "winapp" && (a.dir || a.path)) { const d = parentDir(a.dir || a.path); if (winDirs.indexOf(d) < 0) winDirs.push(d); }
    if (!winDirs.length && home) winDirs.push(home + "/Games/Windows apps");
    const recomps = apps.filter(a => a.kind === "recomp" && a.path);
    const recompDirs = [];
    for (const a of recomps) { const d = parentDir(a.path); if (recompDirs.indexOf(d) < 0) recompDirs.push(d); }
    const hg = hearthGames(data);
    const sum = l => l.reduce((n, a) => n + (a.size || 0), 0);
    const steamSize = sum(apps.filter(a => a.kind === "steam" && (!a.drive || a.drive === "/")));
    if (page === "root") {
        const gameDirs = (hg ? hg.dirs : []).concat(recompDirs);
        add("Program Files", C_ROOT + "/Program Files", { type: "Folder · your apps", linux: "several places (see Details)", look: false, view: true,
            win: "C:\\Program Files", note: "Every app you have installed, one folder each, wherever it really lives. Opening one goes to its real folder.",
            real: [].concat(data.hearth ? ["Hearth apps: " + T(data.hearth)] : [], ["Flatpaks: " + flatDirs.map(T).join(", ")],
                            gameDirs.length ? ["Games: " + gameDirs.map(T).join(", ")] : []) });
        add("Program Files (x86)", C_ROOT + "/Program Files (x86)", { type: "Folder · Windows apps", view: true,
            linux: winDirs.map(T).join(", ") + (data.steam ? " + Steam" : ""), win: "C:\\Program Files (x86)",
            note: "Your Windows apps and games, and Steam, like on a Windows PC.",
            real: winDirs.map(T).concat(data.steam ? [T(data.steam)] : []) });
        add("ProgramData", "/var/lib", { type: "Folder · look only", linux: "/var/lib", win: "C:\\ProgramData", real: ["/var/lib"], lock: true, look: true,
            note: "What apps keep for everyone on this PC. You can look, but Hearth keeps changes out." });
        add("Users", users, { type: "Folder", linux: T(users) === users ? users : T(users), win: "C:\\Users", real: [users],
            note: "Everyone’s own folder. Yours is C:\\Users\\" + user + "." });
        add("Windows", "/usr", { type: "Hearth itself · look only", linux: "/usr", win: "C:\\Windows", real: ["/usr"], lock: true, look: true,
            note: "Hearth’s own system files. Each update replaces them all at once, so this folder is look only, exactly like now." });
        if (data.showLinux) {
            const kinds = { etc: "Linux folder · settings", usr: "Linux folder · Hearth itself" };
            let first = true;
            for (const n of (data.linux || []).slice().sort()) {
                add(n, "/" + n, { type: kinds[n] || "Linux folder", linux: "/" + n, win: "C:\\" + n, real: ["/" + n], dim: true, linuxFolder: true,
                    group: first ? "THE LINUX FOLDERS" : "",
                    note: "A real Linux folder, shown as it is. Turn off “Show the Linux folders too” to hide these." });
                first = false;
            }
        }
    } else if (page === "pf" || page === "x86") {
        const x86 = page === "x86";
        const list = cAppList(data, ctx).filter(e => e.x86 === x86);
        const base = x86 ? "C:\\Program Files (x86)\\" : "C:\\Program Files\\";
        for (const e of list) {
            const a = e.app;
            let cv;
            if (e.name === "Hearth" && !a) {
                cv = { type: "Part of Hearth · look only", linux: T(e.path), real: [T(e.path)], lock: true, look: true, kind: "hearth",
                       note: "Hearth’s built-in apps: Files, Settings, Task Manager and the rest. Look only." };
            } else if (e.name === "Hearth Games" && !a) {
                cv = { type: "Folder · " + (andList(hg.stores) || "Hearth Games"), size: hg.size, linux: hg.dirs.map(T).join(", "), real: hg.dirs.map(T), kind: "games",
                       note: "Games installed from Hearth Games’ stores." };
            } else if (e.name === "Steam" && !a) {
                cv = { type: "Folder · Steam", size: steamSize, linux: T(e.path), real: [T(e.path)], kind: "steam",
                       note: "Steam and its library: steamapps\\common holds your games, like on Windows." };
            } else if (a.kind === "flatpak") {
                cv = { type: "App · Flatpak", size: a.size || 0, linux: e.path, real: [e.path], app: a, kind: "flatpak",
                       note: "A Flatpak app. Uninstall and Modify work like in Installed apps." };
            } else if (a.kind === "recomp") {
                cv = { type: "Game · Recomp", size: a.size || 0, linux: T(e.path), real: [T(e.path)], app: a, kind: "recomp",
                       note: "A Recomp. Play, Update and Uninstall work like in Hearth Games." };
            } else {
                cv = { type: "Windows app", size: a.size || 0, linux: T(e.path), real: [T(e.path)], app: a, kind: "winapp",
                       note: "A Windows app. Inside the app itself, it still sees its own C: drive." };
            }
            cv.win = base + e.name;
            add(e.name, e.path, cv);
        }
        // Hearth first, then by name (as the mockup).
        out.sort((a, b) => (a.name === "Hearth" ? -1 : b.name === "Hearth" ? 1 : 0) || (a.name.toLowerCase() < b.name.toLowerCase() ? -1 : a.name.toLowerCase() > b.name.toLowerCase() ? 1 : 0));
    } else if (page === "appdata" && home) {
        const w = "C:\\Users\\" + user + "\\AppData\\";
        add("Local", home + "/.local/share", { type: "Folder · app data", linux: "~/.local/share", win: w + "Local", real: [home + "/.local/share"],
            note: "Where apps keep their data on this PC. Local\\Temp inside it is ~/.cache." });
        add("Roaming", home + "/.config", { type: "Folder · app settings", linux: "~/.config", win: w + "Roaming", real: [home + "/.config"],
            note: "Where apps keep their settings." });
    }
    return out;
}
// The extra entries a real folder gets with the view on: AppData in your home folder, Temp in AppData\Local. They
// stand for other folders (hearth:c paths), so nothing that changes files can reach them.
function cExtras(p, ctx, have) {
    if (!cOn(ctx) || !p || !ctx.home) return [];
    const home = ctx.home.replace(/\/+$/, "");
    const names = (have || []).map(n => String(n).toLowerCase());
    const u = ctx.user || "you";
    if (p === home && names.indexOf("appdata") < 0)
        return [{ name: "AppData", path: C_ROOT + "/Users/" + u + "/AppData", dir: true, size: 0, mtime: 0, hidden: false,
                  cv: { type: "Folder · app settings", linux: "~/.config, ~/.local/share", win: "C:\\Users\\" + u + "\\AppData", view: true,
                        real: ["Roaming: ~/.config", "Local: ~/.local/share", "Local\\Temp: ~/.cache"],
                        note: "Where apps keep their settings and data. Roaming and Local inside it are your Linux settings folders." } }];
    if (p === home + "/.local/share" && names.indexOf("temp") < 0)
        return [{ name: "Temp", path: C_ROOT + "/Users/" + u + "/AppData/Local/Temp", dir: true, size: 0, mtime: 0, hidden: false,
                  cv: { type: "Folder · temporary files", linux: "~/.cache", win: "C:\\Users\\" + u + "\\AppData\\Local\\Temp", real: [home + "/.cache"],
                        note: "Temporary files and caches (~/.cache)." } }];
    return [];
}
// Why a drop or paste into this place is refused ("" when it isn't): the view's own folders hold nothing of their own,
// and Windows and ProgramData are look only. Inside real folders reached through the view, everything works as always.
function cRefusal(p, ctx) {
    if (isCPath(p)) {
        const pg = cPage(p, ctx);
        if (pg === "pf" || pg === "x86") return "You can’t put files straight into " + (pg === "x86" ? "Program Files (x86)" : "Program Files")
            + ". To add an app, install it, and it shows up here by itself.";
        if (pg === "appdata") return "You can’t put files straight into AppData: open Roaming or Local first.";
        if (pg === "root") return "You can’t put files straight into Hearth (C:). Open a folder in it first (your own folder is in Users).";
        return "That’s a name for another folder: open it first.";
    }
    if (p === "/usr" || (p && p.indexOf("/usr/") === 0)) return "Windows (C:\\Windows) is Hearth itself: look only. Each update replaces it all at once.";
    if (p === "/var/lib" || (p && p.indexOf("/var/lib/") === 0)) return "ProgramData (C:\\ProgramData) is look only: what apps keep there is changed by the apps themselves.";
    return "";
}
