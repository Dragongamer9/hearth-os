// The Run box (Super+R): what something typed opens, and the suggestions while typing. Pure functions over the map in
// win-commands.json, used by RunView.qml and checked by hosts/linux/test/test_win_tools.py (with node).
//
// env: { home: "/home/me",
//        installed: name => true/false        (a program from a command's "needs"),
//        fromWindows: text => {path} / {error} / null    (drive-paths.js with This PC's letters),
//        apps: [{ id: "firefox.desktop", name: "Firefox" }, …] }
// resolve() gives one of:
//   { kind: "tasks" | "devices" | "settings" | "files" | "exec" | "terminal" | "keyboard" | "screenshot", …do, label }
//   { kind: "url", url }   { kind: "path", path }   { kind: "app", id }   { kind: "command", argv }
//   { kind: "missing", text }   { kind: "error", text }
.pragma library

function lower(s) { return String(s || "").trim().toLowerCase(); }

function isUrl(t) {
    return /^(https?|ftp):\/\/\S+$/i.test(t) || /^www\.[^\s\/]+\.[a-z]{2,}(\/\S*)?$/i.test(t);
}

// "taskmgr", "taskmgr.exe", "TASKMGR", "perfmon /rel": the command it names, or null.
function findCommand(t, data) {
    const key = lower(t).replace(/\s+/g, " ");
    const bare = key.replace(/\.exe$/, "");
    for (const c of (data && data.commands) || []) {
        for (const n of c.names || []) {
            if (key === n || bare === n) return c;
        }
    }
    return null;
}

// %appdata%\discord → ~/.config/discord. { text } with the variables put in, or { error } for one Hearth doesn't know.
function expandVars(t, data) {
    let unknown = "";
    const out = t.replace(/%([^%\s]+)%/g, (m, name) => {
        const v = ((data && data.vars) || {})[name.toLowerCase()];
        if (!v) { unknown = unknown || m; return m; }
        return v.path;
    });
    if (unknown) return { error: "Hearth doesn't know " + unknown + ". Try %appdata%, %localappdata%, %temp% or %userprofile%." };
    return { text: /^[~\/]/.test(out) ? out.replace(/\\/g, "/") : out };
}

function homePath(t, home) {
    if (t === "~") return home;
    if (t.startsWith("~/")) return home.replace(/\/+$/, "") + t.slice(1);
    return t;
}

function findApp(t, apps) {
    const q = lower(t);
    if (!q) return null;
    const list = apps || [];
    const idOf = a => lower(String(a.id || "").replace(/\.desktop$/, ""));
    return list.find(a => lower(a.name) === q) || list.find(a => idOf(a) === q)
        || list.find(a => idOf(a).split(".").pop() === q) || null;
}

function resolve(text, data, env) {
    const t = String(text || "").trim();
    if (!t) return null;
    const e = env || {};
    const home = e.home || "/";
    // ms-settings:display → Settings › Display (a page Hearth doesn't have opens Settings' start).
    const ms = /^ms-settings:([a-z0-9-]*)$/i.exec(t);
    if (ms) {
        const page = ((data && data.msSettings) || {})[ms[1].toLowerCase()] || "";
        return { kind: "settings", page: page, label: "Settings" };
    }
    if (isUrl(t)) return { kind: "url", url: /^www\./i.test(t) ? "https://" + t : t, label: t };
    const c = findCommand(t, data);
    if (c) {
        if (c.needs && !(e.installed && e.installed(c.needs))) return { kind: "missing", text: c.opens + " isn't installed on this PC." };
        return Object.assign({ label: c.opens }, c.do);
    }
    let s = t;
    if (/%[^%\s]+%/.test(s)) {
        const x = expandVars(s, data);
        if (x.error) return { kind: "error", text: x.error };
        s = x.text;
    }
    // \\pc\share: a shared folder on the network, opened in Files.
    const unc = /^\\\\([^\\\/]+)(?:[\\\/]+([^\\\/]+))?[\\\/]*$/.exec(s);
    if (unc) return { kind: "path", path: "smb://" + unc[1] + "/" + (unc[2] || ""), label: s };
    // C:\Users\me\Documents, D:\Games, d:/Games: This PC's drive letters (drive-paths.js).
    if (/^[A-Za-z]:([\\\/]|$)/.test(s)) {
        const r = e.fromWindows ? e.fromWindows(s) : null;
        if (!r) return { kind: "error", text: "Hearth can't open " + s + " yet. Try again in a moment." };
        if (r.error) return { kind: "error", text: r.error + "." };
        return { kind: "path", path: r.path, label: s };
    }
    if (s === "~" || s.startsWith("~/") || s.startsWith("/")) return { kind: "path", path: homePath(s, home), label: s };
    const a = findApp(s, e.apps);
    if (a) return { kind: "app", id: a.id, label: a.name };
    // A program by its name (firefox, htop --tree): run if it's installed (RunView checks first).
    if (/^[A-Za-z0-9._+-]+$/.test(s.split(/\s+/)[0])) return { kind: "command", argv: s.split(/\s+/), label: s };
    return { kind: "error", text: "Hearth can't find “" + t + "”. Check the name, then try again." };
}

// What "Open:" suggests while typing (at most `max`): earlier entries first, then Windows commands, %folders% and apps.
// [{ cmd, opens }]; empty text: the recent entries.
function suggest(text, data, env, history, max) {
    const limit = max || 6;
    const q = lower(text);
    const e = env || {};
    const out = [];
    const seen = {};
    const push = (cmd, opens) => {
        const k = lower(cmd);
        if (!k || seen[k] || out.length >= limit) return;
        seen[k] = true;
        out.push({ cmd: cmd, opens: opens });
    };
    const describe = h => {
        const r = resolve(h, data, e);
        if (!r) return "";
        if (r.kind === "error" || r.kind === "missing") return "";
        if (r.kind === "path") return "Folder or file in Files";
        if (r.kind === "url") return "Website in your browser";
        if (r.kind === "command") return "Program";
        return r.label || "";
    };
    for (const h of history || []) {
        if (!q || lower(h).startsWith(q)) { const d = describe(h); if (d) push(h, d); }
    }
    if (!q) return out;
    for (const c of (data && data.commands) || []) {
        if (c.needs && !(e.installed && e.installed(c.needs))) continue;
        const n = (c.names || []).find(x => x.startsWith(q) && x.indexOf(":") < 0);
        if (n) push(n, c.opens);
    }
    const vars = (data && data.vars) || {};
    for (const k in vars) {
        const v = "%" + k + "%";
        if (v.startsWith(q) || (q.length >= 3 && k.startsWith(q))) push(v, vars[k].opens);
    }
    const apps = e.apps || [];
    for (const a of apps) if (lower(a.name).startsWith(q)) push(a.name, "App");
    if (q.length >= 3) for (const a of apps) if (lower(a.name).indexOf(q) > 0) push(a.name, "App");
    return out;
}

// The recent list after running `entry`: newest first, no repeats (ignoring case), at most 12.
function remember(history, entry, max) {
    const t = String(entry || "").trim();
    const list = (history || []).filter(h => typeof h === "string" && lower(h) !== lower(t));
    return t ? [t].concat(list).slice(0, max || 12) : list.slice(0, max || 12);
}

// The Super+X menu's entries whose program is installed (separators kept, never two in a row or at an end).
function winxItems(data, installed) {
    const out = [];
    for (const it of (data && data.winx) || []) {
        if (it.sep) { if (out.length && !out[out.length - 1].sep) out.push(it); continue; }
        if (it.needs && !(installed && installed(it.needs))) continue;
        out.push(it);
    }
    while (out.length && out[out.length - 1].sep) out.pop();
    return out;
}

// Every program a command or an entry needs (checked once when the Run box or the menu opens).
function needed(data) {
    const s = {};
    for (const c of ((data && data.commands) || []).concat((data && data.winx) || [])) if (c.needs) s[c.needs] = true;
    return Object.keys(s).sort();
}
